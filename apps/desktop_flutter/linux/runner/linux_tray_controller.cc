#include "linux_tray_controller.h"

#include <algorithm>
#include <cstdint>
#include <cstring>
#include <memory>
#include <string>

#include "linux_status_notifier.h"

namespace {

constexpr char kTrayChannelName[] = "gui_shell/tray";
constexpr char kUnknown[] = "不明";
constexpr char kNotificationBusName[] = "org.freedesktop.Notifications";
constexpr char kNotificationObjectPath[] = "/org/freedesktop/Notifications";
constexpr char kNotificationInterface[] = "org.freedesktop.Notifications";
constexpr char kCriticalNotificationBody[] =
    "新しい重要通知があります。D4 Pocketを開いて通知センターで確認してください。";

struct LinuxNotificationState {
  bool alive = true;
  guint notification_id = 0;
};

struct PendingNotificationCall {
  FlMethodCall* method_call;
  std::shared_ptr<LinuxNotificationState> state;
};

void NotificationCallFinished(GObject* source_object, GAsyncResult* result,
                              gpointer user_data) {
  std::unique_ptr<PendingNotificationCall> pending(
      static_cast<PendingNotificationCall*>(user_data));
  g_autoptr(GError) error = nullptr;
  g_autoptr(GVariant) response = g_dbus_connection_call_finish(
      G_DBUS_CONNECTION(source_object), result, &error);
  bool accepted = false;
  if (response != nullptr && pending->state->alive) {
    guint notification_id = 0;
    g_variant_get(response, "(u)", &notification_id);
    pending->state->notification_id = notification_id;
    accepted = true;
  }
  g_autoptr(FlValue) value = fl_value_new_bool(accepted);
  fl_method_call_respond_success(pending->method_call, value, nullptr);
  g_object_unref(pending->method_call);
}

void NotificationActionInvoked(GDBusConnection*, const gchar*, const gchar*,
                               const gchar*, const gchar*, GVariant*,
                               gpointer);

std::string bounded_string(FlValue* map, const char* key,
                           const std::string& fallback) {
  FlValue* value = fl_value_lookup_string(map, key);
  if (value == nullptr || fl_value_get_type(value) != FL_VALUE_TYPE_STRING) {
    return fallback;
  }
  const gchar* text = fl_value_get_string(value);
  if (text == nullptr || text[0] == '\0' || std::strlen(text) > 64 ||
      !g_utf8_validate(text, -1, nullptr)) {
    return fallback;
  }
  return text;
}

std::string bounded_count(FlValue* map, const char* key) {
  FlValue* value = fl_value_lookup_string(map, key);
  if (value == nullptr) return kUnknown;
  if (fl_value_get_type(value) == FL_VALUE_TYPE_STRING) {
    const gchar* text = fl_value_get_string(value);
    if (text == nullptr || text[0] == '\0' || std::strlen(text) > 64 ||
        !g_utf8_validate(text, -1, nullptr)) {
      return kUnknown;
    }
    return text;
  }
  if (fl_value_get_type(value) == FL_VALUE_TYPE_INT) {
    const gint64 count = fl_value_get_int(value);
    if (count >= 0 && count <= 256) return std::to_string(count);
  }
  return kUnknown;
}

bool bounded_bool(FlValue* map, const char* key) {
  FlValue* value = fl_value_lookup_string(map, key);
  return value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL &&
         fl_value_get_bool(value);
}

}  // namespace

struct LinuxTrayController {
  LinuxTrayController(FlPluginRegistry* registry, GtkWindow* owner_window,
                      GApplication* owner_application)
      : window(owner_window),
        application(G_APPLICATION(g_object_ref(owner_application))) {
    FlPluginRegistrar* registrar = fl_plugin_registry_get_registrar_for_plugin(
        registry, "D4PocketLinuxTrayChannel");
    FlBinaryMessenger* messenger = fl_plugin_registrar_get_messenger(registrar);
    FlStandardMethodCodec* codec = fl_standard_method_codec_new();
    channel = fl_method_channel_new(messenger, kTrayChannelName,
                                    FL_METHOD_CODEC(codec));
    g_object_unref(codec);
    fl_method_channel_set_method_call_handler(channel, MethodCall, this,
                                              nullptr);

    g_signal_connect(window, "delete-event", G_CALLBACK(DeleteEvent), this);
    g_signal_connect(window, "map-event", G_CALLBACK(MapEvent), this);
    g_signal_connect(window, "unmap-event", G_CALLBACK(UnmapEvent), this);
    g_signal_connect(window, "window-state-event",
                     G_CALLBACK(WindowStateEvent), this);

    g_autoptr(GError) error = nullptr;
    notifications_bus = g_bus_get_sync(G_BUS_TYPE_SESSION, nullptr, &error);
    if (notifications_bus != nullptr) {
      notifier.Start(
          notifications_bus,
          [this](const std::string& action) { HandleAction(action); },
          [this](bool available) { HandleAvailability(available); });
      notification_action_subscription = g_dbus_connection_signal_subscribe(
          notifications_bus, kNotificationBusName, kNotificationInterface,
          "ActionInvoked", kNotificationObjectPath, nullptr,
          G_DBUS_SIGNAL_FLAGS_NONE, NotificationActionInvoked, this, nullptr);
    }
    RefreshVisibility();
  }

  ~LinuxTrayController() {
    if (window != nullptr) {
      g_signal_handlers_disconnect_by_data(window, this);
    }
    notifier.Stop();
    notification_state->alive = false;
    if (notifications_bus != nullptr) {
      if (notification_action_subscription != 0) {
        g_dbus_connection_signal_unsubscribe(
            notifications_bus, notification_action_subscription);
      }
      g_object_unref(notifications_bus);
    }
    if (application_held) g_application_release(application);
    if (channel != nullptr) {
      fl_method_channel_set_method_call_handler(channel, nullptr, nullptr,
                                                nullptr);
      g_object_unref(channel);
    }
    g_object_unref(application);
  }

  static void MethodCall(FlMethodChannel*, FlMethodCall* call,
                         gpointer user_data) {
    auto* self = static_cast<LinuxTrayController*>(user_data);
    const gchar* method = fl_method_call_get_name(call);
    if (g_strcmp0(method, "initialize") == 0) {
      self->channel_initialized = true;
      g_autoptr(FlValue) result = self->notifier.available()
                                      ? fl_value_new_bool(self->WindowVisible())
                                      : fl_value_new_null();
      fl_method_call_respond_success(call, result, nullptr);
      return;
    }
    if (g_strcmp0(method, "isWindowVisible") == 0) {
      if (!self->notifier.available()) {
        g_autoptr(FlValue) result = fl_value_new_null();
        fl_method_call_respond_success(call, result, nullptr);
      } else {
        g_autoptr(FlValue) result = fl_value_new_bool(self->WindowVisible());
        fl_method_call_respond_success(call, result, nullptr);
      }
      return;
    }
    if (g_strcmp0(method, "publish") == 0) {
      FlValue* args = fl_method_call_get_args(call);
      if (args != nullptr && fl_value_get_type(args) == FL_VALUE_TYPE_MAP) {
        self->last_stop_request_supported =
            bounded_bool(args, "stop_request_supported");
        self->notifier.SetProjection(
            bounded_string(args, "runtime_status", kUnknown),
            bounded_count(args, "pending_approval_count"),
            bounded_count(args, "critical_notification_count"),
            self->last_stop_request_supported);
      }
      fl_method_call_respond_success(call, nullptr, nullptr);
      return;
    }
    if (g_strcmp0(method, "notifyCritical") == 0) {
      self->ShowCriticalNotification(call);
      return;
    }
    fl_method_call_respond_not_implemented(call, nullptr);
  }

  void ShowCriticalNotification(FlMethodCall* call) {
    if (notifications_bus == nullptr || !notification_state->alive) {
      g_autoptr(FlValue) result = fl_value_new_bool(false);
      fl_method_call_respond_success(call, result, nullptr);
      return;
    }

    GVariantBuilder actions;
    g_variant_builder_init(&actions, G_VARIANT_TYPE("as"));
    g_variant_builder_add(&actions, "s", "default");
    g_variant_builder_add(&actions, "s", "開く");
    GVariantBuilder hints;
    g_variant_builder_init(&hints, G_VARIANT_TYPE("a{sv}"));
    GVariant* parameters = g_variant_new(
        "(susss@as@a{sv}i)", "D4 Pocket", notification_state->notification_id,
        "dialog-information", "D4 Pocket", kCriticalNotificationBody,
        g_variant_builder_end(&actions), g_variant_builder_end(&hints), 5000);
    auto* pending = new PendingNotificationCall{
        FL_METHOD_CALL(g_object_ref(call)), notification_state};
    g_dbus_connection_call(
        notifications_bus, kNotificationBusName, kNotificationObjectPath,
        kNotificationInterface, "Notify", parameters,
        G_VARIANT_TYPE("(u)"), G_DBUS_CALL_FLAGS_NONE, 3000, nullptr,
        NotificationCallFinished, pending);
  }

  static gboolean DeleteEvent(GtkWidget*, GdkEvent*, gpointer user_data) {
    auto* self = static_cast<LinuxTrayController*>(user_data);
    if (!self->notifier.available() || self->exit_requested) return FALSE;
    gtk_widget_hide(GTK_WIDGET(self->window));
    self->RefreshVisibility();
    return TRUE;
  }

  static gboolean MapEvent(GtkWidget*, GdkEvent*, gpointer user_data) {
    static_cast<LinuxTrayController*>(user_data)->RefreshVisibility();
    return FALSE;
  }

  static gboolean UnmapEvent(GtkWidget*, GdkEvent*, gpointer user_data) {
    static_cast<LinuxTrayController*>(user_data)->RefreshVisibility();
    return FALSE;
  }

  static gboolean WindowStateEvent(GtkWidget*, GdkEventWindowState* event,
                                   gpointer user_data) {
    auto* self = static_cast<LinuxTrayController*>(user_data);
    self->window_iconified =
        (event->new_window_state & GDK_WINDOW_STATE_ICONIFIED) != 0;
    self->RefreshVisibility();
    return FALSE;
  }

  bool WindowVisible() const {
    return window != nullptr && !window_iconified &&
           gtk_widget_get_visible(GTK_WIDGET(window)) &&
           gtk_widget_get_mapped(GTK_WIDGET(window));
  }

  void RefreshVisibility() {
    const bool visible = WindowVisible();
    if (visibility_known && last_visible == visible) return;
    visibility_known = true;
    last_visible = visible;
    if (!visible) {
      notifier.SetProjection(kUnknown, kUnknown, kUnknown,
                             last_stop_request_supported);
    }
    if (channel_initialized) {
      g_autoptr(FlValue) argument = fl_value_new_bool(visible);
      fl_method_channel_invoke_method(channel, "onWindowVisibilityChanged",
                                      argument, nullptr, nullptr, nullptr);
    }
  }

  void HandleAvailability(bool available) {
    if (available && !application_held) {
      g_application_hold(application);
      application_held = true;
    }
    if (!available && window != nullptr && !WindowVisible()) {
      gtk_widget_show(GTK_WIDGET(window));
      gtk_window_present(window);
      RefreshVisibility();
    }
    if (!available && application_held) {
      g_application_release(application);
      application_held = false;
    }
    if (channel_initialized) {
      g_autoptr(FlValue) argument = fl_value_new_bool(available);
      fl_method_channel_invoke_method(channel, "onTrayAvailabilityChanged",
                                      argument, nullptr, nullptr, nullptr);
    }
  }

  void HandleAction(const std::string& action) {
    if (action == "open") {
      if (window != nullptr) {
        gtk_widget_show(GTK_WIDGET(window));
        gtk_window_deiconify(window);
        gtk_window_present(window);
        RefreshVisibility();
      }
      NotifyAction(action);
      return;
    }
    if (action == "stop_request") {
      NotifyAction(action);
      return;
    }
    if (action == "exit") {
      exit_requested = true;
      if (application_held) {
        g_application_release(application);
        application_held = false;
      }
      if (window != nullptr) gtk_window_close(window);
    }
  }

  void HandleNotificationAction(guint notification_id,
                                const std::string& action) {
    if (!notification_state->alive ||
        notification_id != notification_state->notification_id ||
        action != "default") {
      return;
    }
    if (window != nullptr) {
      gtk_widget_show(GTK_WIDGET(window));
      gtk_window_deiconify(window);
      gtk_window_present(window);
      RefreshVisibility();
    }
    NotifyAction("open_notifications");
  }

  void NotifyAction(const std::string& action) {
    if (!channel_initialized) return;
    g_autoptr(FlValue) argument = fl_value_new_string(action.c_str());
    fl_method_channel_invoke_method(channel, "onTrayAction", argument, nullptr,
                                    nullptr, nullptr);
  }

  GtkWindow* window;
  GApplication* application;
  FlMethodChannel* channel = nullptr;
  GDBusConnection* notifications_bus = nullptr;
  guint notification_action_subscription = 0;
  std::shared_ptr<LinuxNotificationState> notification_state =
      std::make_shared<LinuxNotificationState>();
  LinuxStatusNotifier notifier;
  bool channel_initialized = false;
  bool application_held = false;
  bool exit_requested = false;
  bool window_iconified = false;
  bool visibility_known = false;
  bool last_visible = false;
  bool last_stop_request_supported = false;
};

namespace {

void NotificationActionInvoked(GDBusConnection*, const gchar*, const gchar*,
                               const gchar*, const gchar*, GVariant* parameters,
                               gpointer user_data) {
  auto* self = static_cast<LinuxTrayController*>(user_data);
  guint notification_id = 0;
  const gchar* action = nullptr;
  g_variant_get(parameters, "(u&s)", &notification_id, &action);
  self->HandleNotificationAction(notification_id,
                                 action == nullptr ? "" : action);
}

}  // namespace

LinuxTrayController* linux_tray_controller_new(FlPluginRegistry* registry,
                                               GtkWindow* window,
                                               GApplication* application) {
  if (registry == nullptr || window == nullptr || application == nullptr) {
    return nullptr;
  }
  return new LinuxTrayController(registry, window, application);
}

void linux_tray_controller_free(gpointer data) {
  delete static_cast<LinuxTrayController*>(data);
}
