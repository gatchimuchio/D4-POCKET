#include "linux_status_notifier.h"

#include <unistd.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <utility>
#include <vector>

namespace {

constexpr char kWatcherName[] = "org.freedesktop.StatusNotifierWatcher";
constexpr char kWatcherPath[] = "/StatusNotifierWatcher";
constexpr char kWatcherInterface[] = "org.freedesktop.StatusNotifierWatcher";
constexpr char kItemPath[] = "/StatusNotifierItem";
constexpr char kItemInterface[] = "org.freedesktop.StatusNotifierItem";
constexpr char kMenuPath[] = "/Menu";
constexpr char kMenuInterface[] = "com.canonical.dbusmenu";
constexpr char kBusName[] = "org.freedesktop.DBus";
constexpr char kBusPath[] = "/org/freedesktop/DBus";
constexpr char kBusInterface[] = "org.freedesktop.DBus";
constexpr char kPropertiesInterface[] = "org.freedesktop.DBus.Properties";
constexpr char kUnknown[] = "不明";
constexpr gint kOpenItem = 1;
constexpr gint kRuntimeItem = 2;
constexpr gint kApprovalItem = 3;
constexpr gint kNotificationItem = 4;
constexpr gint kSeparatorOne = 5;
constexpr gint kStopItem = 6;
constexpr gint kSeparatorTwo = 7;
constexpr gint kExitItem = 8;

const char kItemXml[] = R"XML(
<node>
  <interface name="org.freedesktop.StatusNotifierItem">
    <method name="ContextMenu"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
    <method name="Activate"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
    <method name="SecondaryActivate"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
    <method name="Scroll"><arg type="i" direction="in"/><arg type="s" direction="in"/></method>
    <signal name="NewTitle"/>
    <signal name="NewIcon"/>
    <signal name="NewAttentionIcon"/>
    <signal name="NewOverlayIcon"/>
    <signal name="NewToolTip"/>
    <signal name="NewStatus"><arg type="s"/></signal>
    <property name="Category" type="s" access="read"/>
    <property name="Id" type="s" access="read"/>
    <property name="Title" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="WindowId" type="u" access="read"/>
    <property name="IconName" type="s" access="read"/>
    <property name="IconPixmap" type="a(iiay)" access="read"/>
    <property name="OverlayIconName" type="s" access="read"/>
    <property name="OverlayIconPixmap" type="a(iiay)" access="read"/>
    <property name="AttentionIconName" type="s" access="read"/>
    <property name="AttentionIconPixmap" type="a(iiay)" access="read"/>
    <property name="AttentionMovieName" type="s" access="read"/>
    <property name="ToolTip" type="(sa(iiay)ss)" access="read"/>
    <property name="ItemIsMenu" type="b" access="read"/>
    <property name="Menu" type="o" access="read"/>
  </interface>
</node>)XML";

const char kMenuXml[] = R"XML(
<node>
  <interface name="com.canonical.dbusmenu">
    <method name="GetLayout"><arg type="i" direction="in"/><arg type="i" direction="in"/><arg type="as" direction="in"/><arg type="u" direction="out"/><arg type="(ia{sv}av)" direction="out"/></method>
    <method name="GetGroupProperties"><arg type="ai" direction="in"/><arg type="as" direction="in"/><arg type="a(ia{sv})" direction="out"/></method>
    <method name="GetProperty"><arg type="i" direction="in"/><arg type="s" direction="in"/><arg type="v" direction="out"/></method>
    <method name="Event"><arg type="i" direction="in"/><arg type="s" direction="in"/><arg type="v" direction="in"/><arg type="u" direction="in"/></method>
    <method name="EventGroup"><arg type="a(isvu)" direction="in"/><arg type="ai" direction="out"/></method>
    <method name="AboutToShow"><arg type="i" direction="in"/><arg type="b" direction="out"/></method>
    <method name="AboutToShowGroup"><arg type="ai" direction="in"/><arg type="ai" direction="out"/><arg type="ai" direction="out"/></method>
    <signal name="ItemsPropertiesUpdated"><arg type="a(ia{sv})"/><arg type="a(ias)"/></signal>
    <signal name="LayoutUpdated"><arg type="u"/><arg type="i"/></signal>
    <signal name="ItemActivationRequested"><arg type="i"/><arg type="u"/></signal>
    <property name="Version" type="u" access="read"/>
    <property name="TextDirection" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="IconThemePath" type="as" access="read"/>
  </interface>
</node>)XML";

const GDBusInterfaceVTable kItemVTable = {
    LinuxStatusNotifier::ItemMethodCall,
    LinuxStatusNotifier::ItemGetProperty,
    nullptr,
    {nullptr},
};
const GDBusInterfaceVTable kMenuVTable = {
    LinuxStatusNotifier::MenuMethodCall,
    LinuxStatusNotifier::MenuGetProperty,
    nullptr,
    {nullptr},
};

bool contains_name(const gchar* const* values, const char* name) {
  if (values == nullptr) return false;
  for (size_t index = 0; values[index] != nullptr; ++index) {
    if (g_strcmp0(values[index], name) == 0) return true;
  }
  return false;
}

bool is_menu_item(gint item_id) {
  return item_id >= kOpenItem && item_id <= kExitItem;
}

GVariant* empty_icon_pixmaps() {
  GVariantBuilder builder;
  g_variant_builder_init(&builder, G_VARIANT_TYPE("a(iiay)"));
  return g_variant_builder_end(&builder);
}

void add_menu_property(GVariantBuilder* builder, const char* name,
                       GVariant* value) {
  g_variant_builder_add(builder, "{sv}", name, value);
}

}  // namespace

LinuxStatusNotifier::LinuxStatusNotifier() = default;

LinuxStatusNotifier::~LinuxStatusNotifier() { Stop(); }

bool LinuxStatusNotifier::Start(GDBusConnection* connection,
                                ActionHandler action_handler,
                                AvailabilityHandler availability_handler) {
  if (connection_ != nullptr || connection == nullptr) return false;
  connection_ = G_DBUS_CONNECTION(g_object_ref(connection));
  action_handler_ = std::move(action_handler);
  availability_handler_ = std::move(availability_handler);
  if (!ExportObjects()) {
    Stop();
    return false;
  }

  name_owner_subscription_id_ = g_dbus_connection_signal_subscribe(
      connection_, kBusName, kBusInterface, "NameOwnerChanged", kBusPath,
      kWatcherName, G_DBUS_SIGNAL_FLAGS_NONE, NameOwnerChanged, this, nullptr);
  host_registered_subscription_id_ = g_dbus_connection_signal_subscribe(
      connection_, kWatcherName, kWatcherInterface,
      "StatusNotifierHostRegistered", kWatcherPath, nullptr,
      G_DBUS_SIGNAL_FLAGS_NONE, HostSignal, this, nullptr);
  host_unregistered_subscription_id_ = g_dbus_connection_signal_subscribe(
      connection_, kWatcherName, kWatcherInterface,
      "StatusNotifierHostUnregistered", kWatcherPath, nullptr,
      G_DBUS_SIGNAL_FLAGS_NONE, HostSignal, this, nullptr);
  properties_subscription_id_ = g_dbus_connection_signal_subscribe(
      connection_, kWatcherName, kPropertiesInterface, "PropertiesChanged",
      kWatcherPath, kWatcherInterface, G_DBUS_SIGNAL_FLAGS_NONE, HostSignal,
      this, nullptr);

  const std::string requested_name = "org.freedesktop.StatusNotifierItem-" +
                                     std::to_string(getpid()) + "-1";
  g_autoptr(GError) error = nullptr;
  g_autoptr(GVariant) reply = g_dbus_connection_call_sync(
      connection_, kBusName, kBusPath, kBusInterface, "RequestName",
      g_variant_new("(su)", requested_name.c_str(), 0u),
      G_VARIANT_TYPE("(u)"), G_DBUS_CALL_FLAGS_NONE, 1000, nullptr, &error);
  if (reply == nullptr) {
    Stop();
    return false;
  }
  guint32 request_result = 0;
  g_variant_get(reply, "(u)", &request_result);
  if (request_result != 1 && request_result != 4) {
    Stop();
    return false;
  }
  service_name_ = requested_name;

  TryRegisterWithWatcher();
  return true;
}

void LinuxStatusNotifier::Stop() {
  SetAvailable(false);
  if (connection_ != nullptr) {
    if (name_owner_subscription_id_ != 0) {
      g_dbus_connection_signal_unsubscribe(connection_,
                                           name_owner_subscription_id_);
      name_owner_subscription_id_ = 0;
    }
    if (host_registered_subscription_id_ != 0) {
      g_dbus_connection_signal_unsubscribe(connection_,
                                           host_registered_subscription_id_);
      host_registered_subscription_id_ = 0;
    }
    if (host_unregistered_subscription_id_ != 0) {
      g_dbus_connection_signal_unsubscribe(connection_,
                                           host_unregistered_subscription_id_);
      host_unregistered_subscription_id_ = 0;
    }
    if (properties_subscription_id_ != 0) {
      g_dbus_connection_signal_unsubscribe(connection_,
                                           properties_subscription_id_);
      properties_subscription_id_ = 0;
    }
    if (item_registration_id_ != 0) {
      g_dbus_connection_unregister_object(connection_, item_registration_id_);
      item_registration_id_ = 0;
    }
    if (menu_registration_id_ != 0) {
      g_dbus_connection_unregister_object(connection_, menu_registration_id_);
      menu_registration_id_ = 0;
    }
    if (!service_name_.empty() && !g_dbus_connection_is_closed(connection_)) {
      g_dbus_connection_call_sync(
          connection_, kBusName, kBusPath, kBusInterface, "ReleaseName",
          g_variant_new("(s)", service_name_.c_str()),
          G_VARIANT_TYPE("(u)"), G_DBUS_CALL_FLAGS_NONE, 250, nullptr,
          nullptr);
    }
    g_clear_object(&connection_);
  }
  if (item_node_ != nullptr) {
    g_dbus_node_info_unref(item_node_);
    item_node_ = nullptr;
  }
  if (menu_node_ != nullptr) {
    g_dbus_node_info_unref(menu_node_);
    menu_node_ = nullptr;
  }
  watcher_registered_ = false;
  host_registered_ = false;
  service_name_.clear();
  action_handler_ = nullptr;
  availability_handler_ = nullptr;
}

void LinuxStatusNotifier::SetProjection(
    std::string runtime_status, std::string pending_approval_count,
    std::string critical_notification_count, bool stop_request_supported) {
  auto bounded = [](std::string value) {
    if (value.empty() || value.size() > 64 ||
        !g_utf8_validate(value.data(), static_cast<gssize>(value.size()),
                         nullptr)) {
      return std::string(kUnknown);
    }
    value.erase(std::remove_if(value.begin(), value.end(), [](unsigned char c) {
                  return c < 0x20 || c == 0x7f;
                }),
                value.end());
    return value.empty() ? std::string(kUnknown) : value;
  };
  runtime_status_ = bounded(std::move(runtime_status));
  pending_approval_count_ = bounded(std::move(pending_approval_count));
  critical_notification_count_ =
      bounded(std::move(critical_notification_count));
  stop_request_supported_ = stop_request_supported;
  ++menu_revision_;
  EmitMenuLayoutChanged();
  EmitItemSignal("NewToolTip", nullptr);
}

bool LinuxStatusNotifier::ExportObjects() {
  g_autoptr(GError) error = nullptr;
  item_node_ = g_dbus_node_info_new_for_xml(kItemXml, &error);
  if (item_node_ == nullptr) return false;
  menu_node_ = g_dbus_node_info_new_for_xml(kMenuXml, &error);
  if (menu_node_ == nullptr) return false;

  item_registration_id_ = g_dbus_connection_register_object(
      connection_, kItemPath, item_node_->interfaces[0], &kItemVTable, this,
      nullptr, &error);
  if (item_registration_id_ == 0) return false;
  menu_registration_id_ = g_dbus_connection_register_object(
      connection_, kMenuPath, menu_node_->interfaces[0], &kMenuVTable, this,
      nullptr, &error);
  return menu_registration_id_ != 0;
}

void LinuxStatusNotifier::TryRegisterWithWatcher() {
  if (connection_ == nullptr || service_name_.empty()) return;
  g_autoptr(GError) error = nullptr;
  g_autoptr(GVariant) reply = g_dbus_connection_call_sync(
      connection_, kWatcherName, kWatcherPath, kWatcherInterface,
      "RegisterStatusNotifierItem", g_variant_new("(s)", service_name_.c_str()),
      G_VARIANT_TYPE("()"), G_DBUS_CALL_FLAGS_NONE, 500, nullptr, &error);
  watcher_registered_ = reply != nullptr;
  if (!watcher_registered_) {
    host_registered_ = false;
    SetAvailable(false);
    return;
  }
  RefreshHostState();
}

void LinuxStatusNotifier::RefreshHostState() {
  if (connection_ == nullptr || !watcher_registered_) {
    host_registered_ = false;
    SetAvailable(false);
    return;
  }
  g_autoptr(GError) error = nullptr;
  g_autoptr(GVariant) reply = g_dbus_connection_call_sync(
      connection_, kWatcherName, kWatcherPath, kPropertiesInterface, "Get",
      g_variant_new("(ss)", kWatcherInterface,
                    "IsStatusNotifierHostRegistered"),
      G_VARIANT_TYPE("(v)"), G_DBUS_CALL_FLAGS_NONE, 500, nullptr, &error);
  if (reply == nullptr) {
    host_registered_ = false;
    SetAvailable(false);
    return;
  }
  g_autoptr(GVariant) boxed = nullptr;
  g_variant_get(reply, "(@v)", &boxed);
  g_autoptr(GVariant) value = g_variant_get_variant(boxed);
  host_registered_ = g_variant_is_of_type(value, G_VARIANT_TYPE_BOOLEAN) &&
                     g_variant_get_boolean(value);
  SetAvailable(host_registered_);
}

void LinuxStatusNotifier::SetAvailable(bool available) {
  if (available_ == available) return;
  available_ = available;
  if (availability_handler_) availability_handler_(available_);
}

void LinuxStatusNotifier::EmitMenuLayoutChanged() {
  if (connection_ == nullptr || menu_registration_id_ == 0) return;
  g_dbus_connection_emit_signal(connection_, nullptr, kMenuPath,
                                kMenuInterface, "LayoutUpdated",
                                g_variant_new("(ui)", menu_revision_, 0),
                                nullptr);
}

void LinuxStatusNotifier::EmitItemSignal(const char* signal_name,
                                         GVariant* parameters) {
  if (connection_ == nullptr || item_registration_id_ == 0) return;
  g_dbus_connection_emit_signal(connection_, nullptr, kItemPath,
                                kItemInterface, signal_name, parameters,
                                nullptr);
}

void LinuxStatusNotifier::DispatchAction(int item_id) {
  if (!action_handler_) return;
  if (item_id == kOpenItem) {
    action_handler_("open");
  } else if (item_id == kStopItem && stop_request_supported_) {
    action_handler_("stop_request");
  } else if (item_id == kExitItem) {
    action_handler_("exit");
  }
}

GVariant* LinuxStatusNotifier::MenuProperties(
    int item_id, const gchar* const* requested) const {
  GVariantBuilder properties;
  g_variant_builder_init(&properties, G_VARIANT_TYPE("a{sv}"));
  auto wants = [requested](const char* name) {
    return requested == nullptr || requested[0] == nullptr ||
           contains_name(requested, name);
  };
  if (wants("label")) {
    add_menu_property(&properties, "label",
                      g_variant_new_string(MenuLabel(item_id).c_str()));
  }
  if (wants("enabled")) {
    add_menu_property(&properties, "enabled",
                      g_variant_new_boolean(MenuEnabled(item_id)));
  }
  if (wants("visible")) {
    add_menu_property(&properties, "visible", g_variant_new_boolean(true));
  }
  if (item_id == kSeparatorOne || item_id == kSeparatorTwo) {
    if (wants("type")) {
      add_menu_property(&properties, "type", g_variant_new_string("separator"));
    }
  }
  return g_variant_builder_end(&properties);
}

GVariant* LinuxStatusNotifier::MenuNode(int item_id,
                                       int recursion_depth) const {
  GVariant* properties = MenuProperties(item_id, nullptr);
  GVariantBuilder children;
  g_variant_builder_init(&children, G_VARIANT_TYPE("av"));
  if (item_id == 0 && recursion_depth != 0) {
    for (int child_id = kOpenItem; child_id <= kExitItem; ++child_id) {
      GVariant* child =
          MenuNode(child_id, recursion_depth < 0 ? -1 : recursion_depth - 1);
      g_variant_builder_add_value(&children, g_variant_new_variant(child));
    }
  }
  return g_variant_new("(i@a{sv}@av)", item_id, properties,
                       g_variant_builder_end(&children));
}

std::string LinuxStatusNotifier::MenuLabel(int item_id) const {
  switch (item_id) {
    case 0:
      return "D4 Pocket";
    case kOpenItem:
      return "D4 Pocketを開く";
    case kRuntimeItem:
      return "実行系状態: " + runtime_status_;
    case kApprovalItem:
      return "保留承認: " + pending_approval_count_;
    case kNotificationItem:
      return "重大通知: " + critical_notification_count_;
    case kStopItem:
      return "全Runtime停止を要求";
    case kExitItem:
      return "終了";
    default:
      return "";
  }
}

bool LinuxStatusNotifier::MenuEnabled(int item_id) const {
  if (item_id == kOpenItem || item_id == kExitItem) return true;
  if (item_id == kStopItem) return stop_request_supported_;
  return false;
}

void LinuxStatusNotifier::ItemMethodCall(
    GDBusConnection*, const gchar*, const gchar*, const gchar*,
    const gchar* method_name, GVariant*, GDBusMethodInvocation* invocation,
    gpointer user_data) {
  auto* self = static_cast<LinuxStatusNotifier*>(user_data);
  if (g_strcmp0(method_name, "Activate") == 0 ||
      g_strcmp0(method_name, "SecondaryActivate") == 0) {
    self->DispatchAction(kOpenItem);
  }
  g_dbus_method_invocation_return_value(invocation, nullptr);
}

GVariant* LinuxStatusNotifier::ItemGetProperty(
    GDBusConnection*, const gchar*, const gchar*, const gchar*,
    const gchar* property_name, GError** error, gpointer user_data) {
  auto* self = static_cast<LinuxStatusNotifier*>(user_data);
  if (g_strcmp0(property_name, "Category") == 0)
    return g_variant_new_string("ApplicationStatus");
  if (g_strcmp0(property_name, "Id") == 0)
    return g_variant_new_string("d4-pocket");
  if (g_strcmp0(property_name, "Title") == 0)
    return g_variant_new_string("D4 Pocket");
  if (g_strcmp0(property_name, "Status") == 0)
    return g_variant_new_string("Active");
  if (g_strcmp0(property_name, "WindowId") == 0)
    return g_variant_new_uint32(0);
  if (g_strcmp0(property_name, "IconName") == 0)
    return g_variant_new_string("applications-system");
  if (g_strcmp0(property_name, "IconPixmap") == 0 ||
      g_strcmp0(property_name, "OverlayIconPixmap") == 0 ||
      g_strcmp0(property_name, "AttentionIconPixmap") == 0)
    return empty_icon_pixmaps();
  if (g_strcmp0(property_name, "OverlayIconName") == 0 ||
      g_strcmp0(property_name, "AttentionIconName") == 0 ||
      g_strcmp0(property_name, "AttentionMovieName") == 0)
    return g_variant_new_string("");
  if (g_strcmp0(property_name, "ToolTip") == 0)
    return g_variant_new("(s@a(iiay)ss)", "applications-system",
                         empty_icon_pixmaps(), "D4 Pocket",
                         ("実行系状態: " + self->runtime_status_ +
                          " / 保留承認: " + self->pending_approval_count_ +
                          " / 重大通知: " + self->critical_notification_count_)
                             .c_str());
  if (g_strcmp0(property_name, "ItemIsMenu") == 0)
    return g_variant_new_boolean(false);
  if (g_strcmp0(property_name, "Menu") == 0)
    return g_variant_new_object_path(kMenuPath);
  g_set_error(error, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_PROPERTY,
              "Unknown StatusNotifierItem property");
  return nullptr;
}

void LinuxStatusNotifier::MenuMethodCall(
    GDBusConnection*, const gchar*, const gchar*, const gchar*,
    const gchar* method_name, GVariant* parameters,
    GDBusMethodInvocation* invocation, gpointer user_data) {
  auto* self = static_cast<LinuxStatusNotifier*>(user_data);
  if (g_strcmp0(method_name, "GetLayout") == 0) {
    gint parent_id = 0;
    gint recursion_depth = 0;
    gchar** requested = nullptr;
    g_variant_get(parameters, "(ii^as)", &parent_id, &recursion_depth,
                  &requested);
    g_strfreev(requested);
    if (parent_id != 0 && !is_menu_item(parent_id)) {
      g_dbus_method_invocation_return_error(
          invocation, G_DBUS_ERROR, G_DBUS_ERROR_INVALID_ARGS,
          "Unknown menu item");
      return;
    }
    GVariant* node = self->MenuNode(parent_id, recursion_depth);
    g_dbus_method_invocation_return_value(
        invocation, g_variant_new("(u@(ia{sv}av))", self->menu_revision_,
                                  node));
    return;
  }
  if (g_strcmp0(method_name, "GetGroupProperties") == 0) {
    GVariant* ids = nullptr;
    gchar** requested = nullptr;
    g_variant_get(parameters, "(@ai^as)", &ids, &requested);
    GVariantBuilder result;
    g_variant_builder_init(&result, G_VARIANT_TYPE("a(ia{sv})"));
    GVariantIter iter;
    gint item_id = 0;
    g_variant_iter_init(&iter, ids);
    while (g_variant_iter_next(&iter, "i", &item_id)) {
      if (item_id == 0 || is_menu_item(item_id)) {
        GVariant* properties = self->MenuProperties(
            item_id, const_cast<const gchar* const*>(requested));
        g_variant_builder_add(&result, "(i@a{sv})", item_id, properties);
      }
    }
    g_dbus_method_invocation_return_value(
        invocation, g_variant_new("(a(ia{sv}))", &result));
    g_variant_unref(ids);
    g_strfreev(requested);
    return;
  }
  if (g_strcmp0(method_name, "GetProperty") == 0) {
    gint item_id = 0;
    const gchar* property_name = nullptr;
    g_variant_get(parameters, "(i&s)", &item_id, &property_name);
    if (item_id != 0 && !is_menu_item(item_id)) {
      g_dbus_method_invocation_return_error(
          invocation, G_DBUS_ERROR, G_DBUS_ERROR_INVALID_ARGS,
          "Unknown menu item");
      return;
    }
    GVariant* properties = self->MenuProperties(item_id, nullptr);
    GVariant* value = g_variant_lookup_value(properties, property_name, nullptr);
    g_variant_unref(properties);
    if (value == nullptr) {
      g_dbus_method_invocation_return_error(
          invocation, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_PROPERTY,
          "Unknown menu item property");
      return;
    }
    g_dbus_method_invocation_return_value(invocation,
                                          g_variant_new("(v)", value));
    g_variant_unref(value);
    return;
  }
  if (g_strcmp0(method_name, "Event") == 0) {
    gint item_id = 0;
    const gchar* event_name = nullptr;
    GVariant* data = nullptr;
    guint32 timestamp = 0;
    g_variant_get(parameters, "(i&svu)", &item_id, &event_name, &data,
                  &timestamp);
    if (g_strcmp0(event_name, "clicked") == 0 &&
        self->MenuEnabled(item_id)) {
      self->DispatchAction(item_id);
    }
    g_variant_unref(data);
    g_dbus_method_invocation_return_value(invocation, nullptr);
    return;
  }
  if (g_strcmp0(method_name, "EventGroup") == 0) {
    GVariant* events = nullptr;
    g_variant_get(parameters, "(@a(isvu))", &events);
    GVariantIter iter;
    g_variant_iter_init(&iter, events);
    GVariantBuilder errors;
    g_variant_builder_init(&errors, G_VARIANT_TYPE("ai"));
    gint item_id = 0;
    const gchar* event_name = nullptr;
    GVariant* data = nullptr;
    guint32 timestamp = 0;
    while (g_variant_iter_next(&iter, "(i&svu)", &item_id, &event_name,
                               &data, &timestamp)) {
      if (!is_menu_item(item_id)) {
        g_variant_builder_add(&errors, "i", item_id);
      } else if (g_strcmp0(event_name, "clicked") == 0 &&
                 self->MenuEnabled(item_id)) {
        self->DispatchAction(item_id);
      }
      if (data != nullptr) g_variant_unref(data);
    }
    g_dbus_method_invocation_return_value(
        invocation, g_variant_new("(ai)", &errors));
    g_variant_unref(events);
    return;
  }
  if (g_strcmp0(method_name, "AboutToShow") == 0) {
    g_dbus_method_invocation_return_value(invocation,
                                          g_variant_new("(b)", false));
    return;
  }
  if (g_strcmp0(method_name, "AboutToShowGroup") == 0) {
    GVariantBuilder updates;
    GVariantBuilder errors;
    g_variant_builder_init(&updates, G_VARIANT_TYPE("ai"));
    g_variant_builder_init(&errors, G_VARIANT_TYPE("ai"));
    g_dbus_method_invocation_return_value(
        invocation, g_variant_new("(ai@ai)", &updates,
                                  g_variant_builder_end(&errors)));
    return;
  }
  g_dbus_method_invocation_return_error(
      invocation, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_METHOD,
      "Unknown DBusMenu method");
}

GVariant* LinuxStatusNotifier::MenuGetProperty(
    GDBusConnection*, const gchar*, const gchar*, const gchar*,
    const gchar* property_name, GError** error, gpointer) {
  if (g_strcmp0(property_name, "Version") == 0)
    return g_variant_new_uint32(3);
  if (g_strcmp0(property_name, "TextDirection") == 0)
    return g_variant_new_string("ltr");
  if (g_strcmp0(property_name, "Status") == 0)
    return g_variant_new_string("normal");
  if (g_strcmp0(property_name, "IconThemePath") == 0) {
    GVariantBuilder paths;
    g_variant_builder_init(&paths, G_VARIANT_TYPE("as"));
    return g_variant_builder_end(&paths);
  }
  g_set_error(error, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_PROPERTY,
              "Unknown DBusMenu property");
  return nullptr;
}

void LinuxStatusNotifier::NameOwnerChanged(
    GDBusConnection*, const gchar*, const gchar*, const gchar*,
    const gchar*, GVariant* parameters, gpointer user_data) {
  auto* self = static_cast<LinuxStatusNotifier*>(user_data);
  const gchar* name = nullptr;
  const gchar* old_owner = nullptr;
  const gchar* new_owner = nullptr;
  g_variant_get(parameters, "(&s&s&s)", &name, &old_owner, &new_owner);
  if (g_strcmp0(name, kWatcherName) != 0) return;
  if (new_owner[0] == '\0') {
    self->watcher_registered_ = false;
    self->host_registered_ = false;
    self->SetAvailable(false);
    return;
  }
  if (old_owner[0] == '\0' || g_strcmp0(old_owner, new_owner) != 0) {
    self->watcher_registered_ = false;
    self->TryRegisterWithWatcher();
  }
}

void LinuxStatusNotifier::HostSignal(
    GDBusConnection*, const gchar*, const gchar*, const gchar*,
    const gchar*, GVariant*, gpointer user_data) {
  static_cast<LinuxStatusNotifier*>(user_data)->RefreshHostState();
}
