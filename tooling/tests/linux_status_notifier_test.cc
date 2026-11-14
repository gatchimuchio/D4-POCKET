#include "../../apps/desktop_flutter/linux/runner/linux_status_notifier.h"

#include <unistd.h>

#include <chrono>
#include <condition_variable>
#include <cstring>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

namespace {

constexpr char kWatcherName[] = "org.freedesktop.StatusNotifierWatcher";
constexpr char kWatcherPath[] = "/StatusNotifierWatcher";
constexpr char kWatcherInterface[] = "org.freedesktop.StatusNotifierWatcher";
constexpr char kItemInterface[] = "org.freedesktop.StatusNotifierItem";
constexpr char kItemPath[] = "/StatusNotifierItem";
constexpr char kMenuInterface[] = "com.canonical.dbusmenu";
constexpr char kMenuPath[] = "/Menu";
constexpr char kPropertiesInterface[] = "org.freedesktop.DBus.Properties";

const char kWatcherXml[] = R"XML(
<node>
  <interface name="org.freedesktop.StatusNotifierWatcher">
    <method name="RegisterStatusNotifierItem"><arg type="s" direction="in"/></method>
    <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
    <signal name="StatusNotifierHostRegistered"/>
    <signal name="StatusNotifierHostUnregistered"/>
  </interface>
</node>)XML";

struct WatcherState {
  std::string address;
  GMainContext* context = nullptr;
  GMainLoop* loop = nullptr;
  GDBusConnection* connection = nullptr;
  GDBusNodeInfo* node = nullptr;
  guint registration_id = 0;
  guint32 request_name_result = 0;
  std::thread thread;
  std::mutex mutex;
  std::condition_variable ready;
  bool initialized = false;
  bool host_registered = true;
  std::string registered_item;
};

void watcher_method_call(GDBusConnection*, const gchar*, const gchar*,
                         const gchar*, const gchar* method,
                         GVariant* parameters,
                         GDBusMethodInvocation* invocation, gpointer data) {
  auto* state = static_cast<WatcherState*>(data);
  if (g_strcmp0(method, "RegisterStatusNotifierItem") != 0) {
    g_dbus_method_invocation_return_error(
        invocation, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_METHOD,
        "Unknown watcher method");
    return;
  }
  const gchar* service = nullptr;
  g_variant_get(parameters, "(&s)", &service);
  {
    std::lock_guard<std::mutex> lock(state->mutex);
    state->registered_item = service;
  }
  state->ready.notify_all();
  g_dbus_method_invocation_return_value(invocation, nullptr);
}

GVariant* watcher_get_property(GDBusConnection*, const gchar*, const gchar*,
                               const gchar*, const gchar* property,
                               GError** error, gpointer data) {
  auto* state = static_cast<WatcherState*>(data);
  if (g_strcmp0(property, "IsStatusNotifierHostRegistered") != 0) {
    g_set_error(error, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_PROPERTY,
                "Unknown watcher property");
    return nullptr;
  }
  std::lock_guard<std::mutex> lock(state->mutex);
  return g_variant_new_boolean(state->host_registered);
}

const GDBusInterfaceVTable kWatcherVTable = {
    watcher_method_call,
    watcher_get_property,
    nullptr,
    {nullptr},
};

void watcher_thread(WatcherState* state) {
  state->context = g_main_context_new();
  g_main_context_push_thread_default(state->context);
  g_autoptr(GError) error = nullptr;
  state->connection = g_dbus_connection_new_for_address_sync(
      state->address.c_str(),
      static_cast<GDBusConnectionFlags>(
          G_DBUS_CONNECTION_FLAGS_AUTHENTICATION_CLIENT |
          G_DBUS_CONNECTION_FLAGS_MESSAGE_BUS_CONNECTION),
      nullptr, nullptr, &error);
  if (state->connection != nullptr) {
    state->node = g_dbus_node_info_new_for_xml(kWatcherXml, &error);
    if (state->node != nullptr) {
      state->registration_id = g_dbus_connection_register_object(
          state->connection, kWatcherPath, state->node->interfaces[0],
          &kWatcherVTable, state, nullptr, &error);
    }
  }
  if (state->registration_id != 0) {
    g_autoptr(GVariant) reply = g_dbus_connection_call_sync(
        state->connection, "org.freedesktop.DBus", "/org/freedesktop/DBus",
        "org.freedesktop.DBus", "RequestName",
        g_variant_new("(su)", kWatcherName, 0u), G_VARIANT_TYPE("(u)"),
        G_DBUS_CALL_FLAGS_NONE, 1000, nullptr, &error);
    if (reply != nullptr) g_variant_get(reply, "(u)", &state->request_name_result);
  }
  state->loop = g_main_loop_new(state->context, FALSE);
  {
    std::lock_guard<std::mutex> lock(state->mutex);
    state->initialized = state->registration_id != 0 &&
                         (state->request_name_result == 1 ||
                          state->request_name_result == 4);
  }
  state->ready.notify_all();
  if (state->initialized) g_main_loop_run(state->loop);
  if (state->registration_id != 0) {
    g_dbus_connection_unregister_object(state->connection,
                                        state->registration_id);
  }
  if (state->node != nullptr) g_dbus_node_info_unref(state->node);
  if (state->connection != nullptr) g_object_unref(state->connection);
  g_main_loop_unref(state->loop);
  g_main_context_pop_thread_default(state->context);
  g_main_context_unref(state->context);
}

bool start_watcher(WatcherState* state) {
  state->thread = std::thread(watcher_thread, state);
  std::unique_lock<std::mutex> lock(state->mutex);
  return state->ready.wait_for(lock, std::chrono::seconds(3),
                               [state] { return state->initialized; });
}

void stop_watcher(WatcherState* state) {
  if (state->loop != nullptr && state->context != nullptr) {
    g_main_context_invoke(state->context,
                          [](gpointer data) -> gboolean {
                            auto* watcher = static_cast<WatcherState*>(data);
                            g_main_loop_quit(watcher->loop);
                            return G_SOURCE_REMOVE;
                          },
                          state);
  }
  if (state->thread.joinable()) state->thread.join();
}

void set_watcher_host(WatcherState* state, bool registered) {
  g_main_context_invoke(
      state->context,
      [](gpointer data) -> gboolean {
        auto* pair = static_cast<std::pair<WatcherState*, bool>*>(data);
        auto* watcher = pair->first;
        const bool host = pair->second;
        {
          std::lock_guard<std::mutex> lock(watcher->mutex);
          watcher->host_registered = host;
        }
        g_dbus_connection_emit_signal(
            watcher->connection, nullptr, kWatcherPath, kWatcherInterface,
            host ? "StatusNotifierHostRegistered"
                 : "StatusNotifierHostUnregistered",
            nullptr, nullptr);
        delete pair;
        return G_SOURCE_REMOVE;
      },
      new std::pair<WatcherState*, bool>(state, registered));
}

GDBusConnection* new_bus_connection(const std::string& address) {
  g_autoptr(GError) error = nullptr;
  return g_dbus_connection_new_for_address_sync(
      address.c_str(),
      static_cast<GDBusConnectionFlags>(
          G_DBUS_CONNECTION_FLAGS_AUTHENTICATION_CLIENT |
          G_DBUS_CONNECTION_FLAGS_MESSAGE_BUS_CONNECTION),
      nullptr, nullptr, &error);
}

struct AsyncCall {
  bool done = false;
  GVariant* reply = nullptr;
  GError* error = nullptr;
};

void async_call_finished(GObject* source, GAsyncResult* result,
                         gpointer user_data) {
  auto* call = static_cast<AsyncCall*>(user_data);
  call->reply = g_dbus_connection_call_finish(
      G_DBUS_CONNECTION(source), result, &call->error);
  call->done = true;
}

GVariant* call_item(GDBusConnection* client, const char* path,
                    const char* interface, const char* method,
                    GVariant* parameters, const GVariantType* reply_type) {
  AsyncCall call;
  const std::string service_name = "org.freedesktop.StatusNotifierItem-" +
                                   std::to_string(getpid()) + "-1";
  g_dbus_connection_call(client, service_name.c_str(),
                         path, interface, method, parameters, reply_type,
                         G_DBUS_CALL_FLAGS_NONE, 2000, nullptr,
                         async_call_finished, &call);
  const auto deadline = std::chrono::steady_clock::now() +
                        std::chrono::seconds(3);
  while (!call.done && std::chrono::steady_clock::now() < deadline) {
    g_main_context_iteration(nullptr, FALSE);
    std::this_thread::sleep_for(std::chrono::milliseconds(1));
  }
  if (!call.done) return nullptr;
  if (call.error != nullptr) g_error_free(call.error);
  return call.reply;
}

void test_status_notifier_round_trip() {
  GTestDBus* test_bus = g_test_dbus_new(G_TEST_DBUS_NONE);
  g_test_dbus_up(test_bus);
  const char* address = g_test_dbus_get_bus_address(test_bus);

  WatcherState watcher;
  watcher.address = address;
  g_assert_true(start_watcher(&watcher));

  g_autoptr(GError) error = nullptr;
  GDBusConnection* item_connection = new_bus_connection(address);
  g_assert_nonnull(item_connection);
  LinuxStatusNotifier item;
  std::vector<std::string> actions;
  std::vector<bool> availability;
  g_assert_true(item.Start(
      item_connection,
      [&actions](const std::string& action) { actions.push_back(action); },
      [&availability](bool value) { availability.push_back(value); }));
  g_assert_true(item.available());
  const std::string expected_service =
      "org.freedesktop.StatusNotifierItem-" + std::to_string(getpid()) + "-1";
  {
    std::lock_guard<std::mutex> lock(watcher.mutex);
    g_assert_cmpstr(watcher.registered_item.c_str(), ==, expected_service.c_str());
  }

  GDBusConnection* client = new_bus_connection(address);
  g_assert_nonnull(client);
  g_autoptr(GVariant) item_status = call_item(
      client, kItemPath, kPropertiesInterface, "Get",
      g_variant_new("(ss)", kItemInterface, "Status"),
      G_VARIANT_TYPE("(v)"));
  g_assert_nonnull(item_status);
  g_autoptr(GVariant) status_box = nullptr;
  g_variant_get(item_status, "(@v)", &status_box);
  g_autoptr(GVariant) status = g_variant_get_variant(status_box);
  g_assert_cmpstr(g_variant_get_string(status, nullptr), ==, "Active");

  g_autoptr(GVariant) layout_reply = call_item(
      client, kMenuPath, kMenuInterface, "GetLayout",
      g_variant_new("(ii@as)", 0, -1, g_variant_new_strv(nullptr, 0)),
      G_VARIANT_TYPE("(u(ia{sv}av))"));
  g_assert_nonnull(layout_reply);
  guint32 revision = 0;
  g_autoptr(GVariant) layout = nullptr;
  g_variant_get(layout_reply, "(u@(ia{sv}av))", &revision, &layout);
  g_autofree gchar* layout_text = g_variant_print(layout, TRUE);
  g_assert_nonnull(strstr(layout_text, "D4 Pocketを開く"));
  g_assert_nonnull(strstr(layout_text, "全Runtime停止を要求"));

  g_autoptr(GVariant) shallow_layout_reply = call_item(
      client, kMenuPath, kMenuInterface, "GetLayout",
      g_variant_new("(ii@as)", 0, 0, g_variant_new_strv(nullptr, 0)),
      G_VARIANT_TYPE("(u(ia{sv}av))"));
  g_assert_nonnull(shallow_layout_reply);
  guint32 shallow_revision = 0;
  g_autoptr(GVariant) shallow_layout = nullptr;
  g_variant_get(shallow_layout_reply, "(u@(ia{sv}av))", &shallow_revision,
                &shallow_layout);
  g_assert_cmpuint(shallow_revision, >, 0);
  g_autoptr(GVariant) shallow_children =
      g_variant_get_child_value(shallow_layout, 2);
  g_assert_cmpuint(g_variant_n_children(shallow_children), ==, 0);

  g_autoptr(GVariant) runtime_label = call_item(
      client, kMenuPath, kMenuInterface, "GetProperty",
      g_variant_new("(is)", 2, "label"), G_VARIANT_TYPE("(v)"));
  g_assert_nonnull(runtime_label);
  g_autoptr(GVariant) runtime_label_box = nullptr;
  g_variant_get(runtime_label, "(@v)", &runtime_label_box);
  g_autoptr(GVariant) runtime_label_value =
      g_variant_get_variant(runtime_label_box);
  g_assert_cmpstr(g_variant_get_string(runtime_label_value, nullptr), ==,
                  "実行系状態: 不明");

  g_autoptr(GVariant) invalid_menu_item = call_item(
      client, kMenuPath, kMenuInterface, "GetProperty",
      g_variant_new("(is)", 999, "label"), G_VARIANT_TYPE("(v)"));
  g_assert_null(invalid_menu_item);
  g_assert_true(actions.empty());

  g_autoptr(GVariant) disabled_stop = call_item(
      client, kMenuPath, kMenuInterface, "Event",
      g_variant_new("(isvu)", 6, "clicked", g_variant_new_int32(0), 0u),
      G_VARIANT_TYPE("()"));
  g_assert_nonnull(disabled_stop);
  g_assert_true(actions.empty());

  item.SetProjection("稼働中", "2", "1", true);
  g_autoptr(GVariant) enabled_stop = call_item(
      client, kMenuPath, kMenuInterface, "Event",
      g_variant_new("(isvu)", 6, "clicked", g_variant_new_int32(0), 0u),
      G_VARIANT_TYPE("()"));
  g_assert_nonnull(enabled_stop);
  g_assert_cmpuint(actions.size(), ==, 1);
  g_assert_cmpstr(actions[0].c_str(), ==, "stop_request");

  g_autoptr(GVariant) activation = call_item(
      client, kItemPath, kItemInterface, "Activate",
      g_variant_new("(ii)", 0, 0), G_VARIANT_TYPE("()"));
  g_assert_nonnull(activation);
  g_assert_cmpuint(actions.size(), ==, 2);
  g_assert_cmpstr(actions[1].c_str(), ==, "open");

  set_watcher_host(&watcher, false);
  const auto unavailable_deadline = std::chrono::steady_clock::now() +
                                    std::chrono::seconds(2);
  while (item.available() &&
         std::chrono::steady_clock::now() < unavailable_deadline) {
    g_main_context_iteration(nullptr, FALSE);
    std::this_thread::sleep_for(std::chrono::milliseconds(1));
  }
  g_assert_false(item.available());
  set_watcher_host(&watcher, true);
  const auto available_deadline = std::chrono::steady_clock::now() +
                                  std::chrono::seconds(2);
  while (!item.available() &&
         std::chrono::steady_clock::now() < available_deadline) {
    g_main_context_iteration(nullptr, FALSE);
    std::this_thread::sleep_for(std::chrono::milliseconds(1));
  }
  g_assert_true(item.available());
  g_assert_cmpuint(availability.size(), >=, 2);

  item.Stop();
  g_object_unref(client);
  g_object_unref(item_connection);
  stop_watcher(&watcher);
  g_test_dbus_down(test_bus);
  g_object_unref(test_bus);
}

}  // namespace

int main(int argc, char** argv) {
  g_test_init(&argc, &argv, nullptr);
  g_test_add_func("/linux/status-notifier/round-trip",
                  test_status_notifier_round_trip);
  return g_test_run();
}
