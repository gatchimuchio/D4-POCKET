#include "broker_process_channel.h"

#include <gio/gio.h>
#include <glib.h>

#include <algorithm>
#include <atomic>
#include <cstring>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

namespace {

constexpr size_t kRequestLimit = 64 * 1024;
constexpr size_t kResponseLimit = 4 * 1024 * 1024;
constexpr unsigned int kMaxQueuedRequests = 16;

class BrokerContext {
 public:
  BrokerContext() : helper_path_(resolve_helper_path()) {}

  ~BrokerContext() {
    std::lock_guard<std::mutex> lock(mutex_);
    close_input();
    g_clear_object(&stdout_pipe_);
    g_clear_object(&process_);
  }

  bool ReserveRequest() {
    const unsigned int current = in_flight_.fetch_add(1);
    if (current >= kMaxQueuedRequests) {
      in_flight_.fetch_sub(1);
      return false;
    }
    return true;
  }

  void ReleaseRequest() { in_flight_.fetch_sub(1); }

  bool Request(const std::string& request, std::string* response) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (poisoned_ || request.empty() || request.size() > kRequestLimit ||
        request.find('\n') != std::string::npos ||
        request.find('\r') != std::string::npos) {
      return false;
    }
    if (!ensure_process()) {
      return false;
    }

    GError* error = nullptr;
    gsize written = 0;
    const std::string frame = request + "\n";
    if (!g_output_stream_write_all(stdin_pipe_, frame.data(), frame.size(),
                                   &written, nullptr, &error) ||
        written != frame.size()) {
      g_clear_error(&error);
      poison();
      return false;
    }

    std::string buffer;
    char chunk[4096];
    for (;;) {
      const gssize read = g_input_stream_read(stdout_pipe_, chunk, sizeof(chunk),
                                              nullptr, &error);
      if (read <= 0) {
        g_clear_error(&error);
        poison();
        return false;
      }
      const char* newline = static_cast<const char*>(
          std::memchr(chunk, '\n', static_cast<size_t>(read)));
      const size_t content_size = newline == nullptr
                                      ? static_cast<size_t>(read)
                                      : static_cast<size_t>(newline - chunk);
      if (content_size > kResponseLimit - buffer.size() ||
          std::memchr(chunk, '\r', content_size) != nullptr) {
        poison();
        return false;
      }
      buffer.append(chunk, content_size);
      if (newline != nullptr) {
        if (content_size + 1 != static_cast<size_t>(read) || buffer.empty() ||
            !g_utf8_validate(buffer.data(), buffer.size(), nullptr)) {
          poison();
          return false;
        }
        *response = std::move(buffer);
        return true;
      }
    }
  }

 private:
  static std::string resolve_helper_path() {
    GError* error = nullptr;
    gchar* executable = g_file_read_link("/proc/self/exe", &error);
    if (executable == nullptr) {
      g_clear_error(&error);
      return {};
    }
    gchar* directory = g_path_get_dirname(executable);
    gchar* helper = g_build_filename(directory, "libexec",
                                     "gui_shell_linux_broker", nullptr);
    std::string result(helper);
    g_free(helper);
    g_free(directory);
    g_free(executable);
    return result;
  }

  bool ensure_process() {
    if (process_ != nullptr) {
      return stdin_pipe_ != nullptr && stdout_pipe_ != nullptr;
    }
    if (helper_path_.empty() ||
        !g_file_test(helper_path_.c_str(), G_FILE_TEST_IS_EXECUTABLE)) {
      return false;
    }

    GSubprocessLauncher* launcher = g_subprocess_launcher_new(
        static_cast<GSubprocessFlags>(G_SUBPROCESS_FLAGS_STDIN_PIPE |
                                      G_SUBPROCESS_FLAGS_STDOUT_PIPE |
                                      G_SUBPROCESS_FLAGS_STDERR_SILENCE));
    static const char* const allowed_environment[] = {
        "HOME",          "XDG_RUNTIME_DIR", "XDG_STATE_HOME",
        "DISPLAY",       "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS",
        "XDG_SESSION_TYPE", "GDK_BACKEND",  "LANG",
        "LC_ALL",         "LC_CTYPE",       nullptr};
    gchar** environment = g_new0(gchar*, G_N_ELEMENTS(allowed_environment));
    size_t output_index = 0;
    for (size_t index = 0; allowed_environment[index] != nullptr; ++index) {
      const gchar* value = g_getenv(allowed_environment[index]);
      if (value != nullptr) {
        environment[output_index++] =
            g_strdup_printf("%s=%s", allowed_environment[index], value);
      }
    }
    g_subprocess_launcher_set_environ(launcher, environment);
    g_strfreev(environment);

    GError* error = nullptr;
    process_ = g_subprocess_launcher_spawn(launcher, &error,
                                          helper_path_.c_str(), nullptr);
    g_object_unref(launcher);
    if (process_ == nullptr) {
      g_clear_error(&error);
      return false;
    }
    stdin_pipe_ = G_OUTPUT_STREAM(
        g_object_ref(g_subprocess_get_stdin_pipe(process_)));
    stdout_pipe_ = G_INPUT_STREAM(
        g_object_ref(g_subprocess_get_stdout_pipe(process_)));
    return stdin_pipe_ != nullptr && stdout_pipe_ != nullptr;
  }

  void close_input() {
    if (stdin_pipe_ != nullptr) {
      GError* error = nullptr;
      g_output_stream_close(stdin_pipe_, nullptr, &error);
      g_clear_error(&error);
      g_clear_object(&stdin_pipe_);
    }
  }

  void poison() {
    poisoned_ = true;
    close_input();
  }

  const std::string helper_path_;
  std::mutex mutex_;
  std::atomic<unsigned int> in_flight_{0};
  GSubprocess* process_ = nullptr;
  GOutputStream* stdin_pipe_ = nullptr;
  GInputStream* stdout_pipe_ = nullptr;
  bool poisoned_ = false;
};

struct RequestTask {
  RequestTask(std::shared_ptr<BrokerContext> context, std::string request)
      : context(std::move(context)), request(std::move(request)) {}
  ~RequestTask() { context->ReleaseRequest(); }

  std::shared_ptr<BrokerContext> context;
  std::string request;
};

void request_task_free(gpointer data) {
  delete static_cast<RequestTask*>(data);
}

void request_worker(GTask* task, gpointer, gpointer task_data,
                    GCancellable*) {
  auto* request = static_cast<RequestTask*>(task_data);
  auto* response = new std::string();
  if (!request->context->Request(request->request, response)) {
    delete response;
    g_task_return_new_error(task, G_IO_ERROR, G_IO_ERROR_FAILED,
                            "Linux安全Brokerとの通信に失敗しました");
    return;
  }
  g_task_return_pointer(task, response,
                        [](gpointer value) { delete static_cast<std::string*>(value); });
}

void request_complete(GObject* source_object, GAsyncResult* result, gpointer) {
  auto* method_call = FL_METHOD_CALL(source_object);
  GError* error = nullptr;
  auto* response = static_cast<std::string*>(
      g_task_propagate_pointer(G_TASK(result), &error));
  if (response == nullptr) {
    fl_method_call_respond_error(method_call, "broker_unavailable",
                                 "安全Brokerとの通信に失敗しました", nullptr,
                                 nullptr);
    g_clear_error(&error);
    return;
  }

  FlValue* value = fl_value_new_string(response->c_str());
  GError* respond_error = nullptr;
  fl_method_call_respond_success(method_call, value, &respond_error);
  g_clear_error(&respond_error);
  fl_value_unref(value);
  delete response;
}

void request_method_call(FlMethodChannel*, FlMethodCall* method_call,
                         gpointer user_data) {
  const gchar* method = fl_method_call_get_name(method_call);
  if (g_strcmp0(method, "request") != 0) {
    fl_method_call_respond_not_implemented(method_call, nullptr);
    return;
  }

  FlValue* arguments = fl_method_call_get_args(method_call);
  if (arguments == nullptr || fl_value_get_type(arguments) != FL_VALUE_TYPE_STRING) {
    fl_method_call_respond_error(method_call, "invalid_request",
                                 "Broker要求の形式が不正です", nullptr, nullptr);
    return;
  }
  const gchar* request = fl_value_get_string(arguments);
  if (request == nullptr || !g_utf8_validate(request, -1, nullptr)) {
    fl_method_call_respond_error(method_call, "invalid_request",
                                 "Broker要求の形式が不正です", nullptr, nullptr);
    return;
  }

  auto* context_holder = static_cast<std::shared_ptr<BrokerContext>*>(user_data);
  std::shared_ptr<BrokerContext> context = *context_holder;
  const size_t request_size = std::strlen(request);
  if (request_size == 0 || request_size > kRequestLimit ||
      !context->ReserveRequest()) {
    fl_method_call_respond_error(method_call, "broker_busy",
                                 "Broker要求を処理できません。時間をおいて再試行してください",
                                 nullptr, nullptr);
    return;
  }

  GTask* task = g_task_new(G_OBJECT(method_call), nullptr, request_complete,
                           nullptr);
  g_task_set_task_data(task, new RequestTask(std::move(context), request),
                       request_task_free);
  g_task_run_in_thread(task, request_worker);
  g_object_unref(task);
}

void shared_context_free(gpointer data) {
  delete static_cast<std::shared_ptr<BrokerContext>*>(data);
}

}  // namespace

void broker_process_channel_register(FlPluginRegistry* registry) {
  FlPluginRegistrar* registrar = fl_plugin_registry_get_registrar_for_plugin(
      registry, "D4PocketBrokerProcessChannel");
  FlBinaryMessenger* messenger = fl_plugin_registrar_get_messenger(registrar);
  FlStandardMethodCodec* codec = fl_standard_method_codec_new();
  FlMethodChannel* channel = fl_method_channel_new(
      messenger, "gui_shell/broker", FL_METHOD_CODEC(codec));
  g_object_unref(codec);

  auto* context = new std::shared_ptr<BrokerContext>(
      std::make_shared<BrokerContext>());
  fl_method_channel_set_method_call_handler(channel, request_method_call,
                                            context, shared_context_free);
  g_object_set_data_full(G_OBJECT(registry), "d4-pocket-linux-broker-channel",
                         channel, g_object_unref);
}
