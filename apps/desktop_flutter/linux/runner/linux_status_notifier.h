#ifndef RUNNER_LINUX_STATUS_NOTIFIER_H_
#define RUNNER_LINUX_STATUS_NOTIFIER_H_

#include <gio/gio.h>

#include <functional>
#include <string>

class LinuxStatusNotifier {
 public:
  using ActionHandler = std::function<void(const std::string&)>;
  using AvailabilityHandler = std::function<void(bool)>;

  LinuxStatusNotifier();
  ~LinuxStatusNotifier();

  LinuxStatusNotifier(const LinuxStatusNotifier&) = delete;
  LinuxStatusNotifier& operator=(const LinuxStatusNotifier&) = delete;

  // Returns false only when the session bus or the fixed D-Bus objects cannot
  // be established. A missing desktop watcher/host is reported as unavailable.
  bool Start(GDBusConnection* connection, ActionHandler action_handler,
             AvailabilityHandler availability_handler);
  void Stop();
  bool available() const { return available_; }
  void SetProjection(std::string runtime_status,
                     std::string pending_approval_count,
                     std::string critical_notification_count,
                     bool stop_request_supported);

  // GDBus callbacks are public only so the static interface vtables can point
  // at them; applications should use Start/SetProjection/Stop.
  static void ItemMethodCall(GDBusConnection*, const gchar*, const gchar*,
                             const gchar*, const gchar*, GVariant*,
                             GDBusMethodInvocation*, gpointer);
  static GVariant* ItemGetProperty(GDBusConnection*, const gchar*,
                                   const gchar*, const gchar*, const gchar*,
                                   GError**, gpointer);
  static void MenuMethodCall(GDBusConnection*, const gchar*, const gchar*,
                             const gchar*, const gchar*, GVariant*,
                             GDBusMethodInvocation*, gpointer);
  static GVariant* MenuGetProperty(GDBusConnection*, const gchar*,
                                   const gchar*, const gchar*, const gchar*,
                                   GError**, gpointer);
  static void NameOwnerChanged(GDBusConnection*, const gchar*, const gchar*,
                               const gchar*, const gchar*, GVariant*,
                               gpointer);
  static void HostSignal(GDBusConnection*, const gchar*, const gchar*,
                         const gchar*, const gchar*, GVariant*, gpointer);

 private:
  bool ExportObjects();
  void TryRegisterWithWatcher();
  void RefreshHostState();
  void SetAvailable(bool available);
  void EmitMenuLayoutChanged();
  void EmitItemSignal(const char* signal_name, GVariant* parameters);
  void DispatchAction(int item_id);
  GVariant* MenuNode(int item_id, int recursion_depth) const;
  GVariant* MenuProperties(int item_id,
                           const gchar* const* requested) const;
  std::string MenuLabel(int item_id) const;
  bool MenuEnabled(int item_id) const;

  GDBusConnection* connection_ = nullptr;
  GDBusNodeInfo* item_node_ = nullptr;
  GDBusNodeInfo* menu_node_ = nullptr;
  guint item_registration_id_ = 0;
  guint menu_registration_id_ = 0;
  guint name_owner_subscription_id_ = 0;
  guint host_registered_subscription_id_ = 0;
  guint host_unregistered_subscription_id_ = 0;
  guint properties_subscription_id_ = 0;
  std::string service_name_;
  ActionHandler action_handler_;
  AvailabilityHandler availability_handler_;
  std::string runtime_status_ = "不明";
  std::string pending_approval_count_ = "不明";
  std::string critical_notification_count_ = "不明";
  bool stop_request_supported_ = false;
  bool watcher_registered_ = false;
  bool host_registered_ = false;
  bool available_ = false;
  guint menu_revision_ = 1;
};

#endif  // RUNNER_LINUX_STATUS_NOTIFIER_H_
