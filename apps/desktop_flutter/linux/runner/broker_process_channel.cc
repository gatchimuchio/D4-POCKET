#include "broker_process_channel.h"

#include <gio/gio.h>
#include <glib.h>

#include <algorithm>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstring>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

namespace {

constexpr size_t kRequestLimit = 64 * 1024;
constexpr size_t kResponseLimit = 4 * 1024 * 1024;
constexpr size_t kOwnerSummaryLimit = 16 * 1024;
constexpr size_t kOwnerPromptFrameLimit = kOwnerSummaryLimit * 2 + 64;
constexpr size_t kOwnerChallengeLength = 32;
constexpr unsigned int kMaxQueuedRequests = 16;
constexpr auto kOwnerDialogTimeout = std::chrono::seconds(300);
constexpr auto kCredentialDialogTimeout = std::chrono::seconds(300);
constexpr auto kWorkspaceDialogTimeout = std::chrono::seconds(300);
constexpr size_t kWorkspaceReplyLimit = 2 * 1024 + 128;

bool is_lower_hex(char value) {
  return (value >= '0' && value <= '9') || (value >= 'a' && value <= 'f');
}

bool has_owner_protocol_prefix(const std::string& frame) {
  return frame.compare(0, 8, "D4OWNER1") == 0;
}

bool has_credential_protocol_prefix(const std::string& frame) {
  return frame.compare(0, 7, "D4CRED1") == 0;
}

bool has_workspace_picker_prefix(const std::string& frame) {
  return frame.compare(0, 7, "D4PICK1") == 0;
}

void secure_clear(std::string* value) {
  volatile unsigned char* bytes =
      reinterpret_cast<volatile unsigned char*>(value->data());
  for (size_t index = 0; index < value->size(); ++index) bytes[index] = 0;
  value->clear();
}

bool decode_control_prompt(const std::string& frame, const char* prefix,
                           std::string* challenge, std::string* summary) {
  const std::string expected = std::string(prefix) + " ";
  if (frame.compare(0, expected.size(), expected) != 0 ||
      frame.size() > kOwnerPromptFrameLimit) {
    return false;
  }
  const size_t separator = frame.find(' ', expected.size());
  if (separator == std::string::npos ||
      separator - expected.size() != kOwnerChallengeLength) {
    return false;
  }
  const std::string candidate =
      frame.substr(expected.size(), separator - expected.size());
  if (!std::all_of(candidate.begin(), candidate.end(), is_lower_hex)) {
    return false;
  }
  const std::string encoded = frame.substr(separator + 1);
  if (encoded.empty() || encoded.size() % 2 != 0 ||
      encoded.size() > kOwnerSummaryLimit * 2) {
    return false;
  }
  std::string decoded;
  decoded.reserve(encoded.size() / 2);
  for (size_t index = 0; index < encoded.size(); index += 2) {
    const int high = g_ascii_xdigit_value(encoded[index]);
    const int low = g_ascii_xdigit_value(encoded[index + 1]);
    if (high < 0 || low < 0 ||
        (encoded[index] >= 'A' && encoded[index] <= 'F') ||
        (encoded[index + 1] >= 'A' && encoded[index + 1] <= 'F')) {
      return false;
    }
    const char byte = static_cast<char>((high << 4) | low);
    if (byte == '\0' || byte == '\r' ||
        (static_cast<unsigned char>(byte) < 0x20 && byte != '\n' &&
         byte != '\t')) {
      return false;
    }
    decoded.push_back(byte);
  }
  if (decoded.empty() ||
      !g_utf8_validate(decoded.data(), decoded.size(), nullptr)) {
    return false;
  }
  *challenge = candidate;
  *summary = std::move(decoded);
  return true;
}

bool decode_owner_prompt(const std::string& frame, std::string* challenge,
                         std::string* summary) {
  return decode_control_prompt(frame, "D4OWNER1", challenge, summary);
}

bool decode_credential_prompt(const std::string& frame,
                              std::string* challenge,
                              std::string* summary) {
  return decode_control_prompt(frame, "D4CRED1", challenge, summary);
}

bool decode_workspace_picker_prompt(const std::string& frame,
                                    std::string* challenge) {
  constexpr char kPrefix[] = "D4PICK1 ";
  if (frame.size() != sizeof(kPrefix) - 1 + kOwnerChallengeLength ||
      frame.compare(0, sizeof(kPrefix) - 1, kPrefix) != 0) {
    return false;
  }
  const std::string candidate = frame.substr(sizeof(kPrefix) - 1);
  if (!std::all_of(candidate.begin(), candidate.end(), is_lower_hex)) {
    return false;
  }
  *challenge = candidate;
  return true;
}

struct OwnerDialogState {
  explicit OwnerDialogState(GtkWindow* owner_parent, std::string text)
      : parent(owner_parent), summary(std::move(text)) {}
  ~OwnerDialogState() {
    if (parent != nullptr) g_object_unref(parent);
  }

  GtkWindow* parent;
  const std::string summary;
  std::mutex mutex;
  std::condition_variable ready;
  std::atomic<bool> expired{false};
  bool complete = false;
  bool confirmed = false;
};

struct OwnerDialogTimeout {
  GtkDialog* dialog;
  bool fired = false;
};

gboolean reject_expired_owner_dialog(gpointer data) {
  auto* timeout = static_cast<OwnerDialogTimeout*>(data);
  timeout->fired = true;
  gtk_dialog_response(timeout->dialog, GTK_RESPONSE_CANCEL);
  return G_SOURCE_REMOVE;
}

void finish_owner_dialog(const std::shared_ptr<OwnerDialogState>& state,
                         bool confirmed) {
  {
    std::lock_guard<std::mutex> lock(state->mutex);
    state->confirmed = confirmed && !state->expired.load();
    state->complete = true;
  }
  state->ready.notify_one();
}

gboolean show_owner_dialog_on_gtk_thread(gpointer data) {
  auto state = *static_cast<std::shared_ptr<OwnerDialogState>*>(data);
  if (state->expired.load()) {
    finish_owner_dialog(state, false);
    return G_SOURCE_REMOVE;
  }

  GtkWidget* dialog = gtk_dialog_new_with_buttons(
      "D4 Pocket Owner確認", state->parent,
      static_cast<GtkDialogFlags>(GTK_DIALOG_MODAL |
                                  GTK_DIALOG_DESTROY_WITH_PARENT),
      "拒否", GTK_RESPONSE_NO, "許可", GTK_RESPONSE_YES, nullptr);
  gtk_window_set_position(GTK_WINDOW(dialog), GTK_WIN_POS_CENTER_ON_PARENT);
  gtk_window_set_default_size(GTK_WINDOW(dialog), 680, 480);
  gtk_dialog_set_default_response(GTK_DIALOG(dialog), GTK_RESPONSE_NO);
  GtkWidget* content = gtk_dialog_get_content_area(GTK_DIALOG(dialog));
  GtkWidget* scroller = gtk_scrolled_window_new(nullptr, nullptr);
  gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(scroller),
                                 GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
  GtkWidget* label = gtk_label_new(state->summary.c_str());
  gtk_label_set_line_wrap(GTK_LABEL(label), TRUE);
  gtk_label_set_line_wrap_mode(GTK_LABEL(label), PANGO_WRAP_WORD_CHAR);
  gtk_label_set_xalign(GTK_LABEL(label), 0.0f);
  gtk_label_set_yalign(GTK_LABEL(label), 0.0f);
  gtk_container_add(GTK_CONTAINER(scroller), label);
  gtk_container_add(GTK_CONTAINER(content), scroller);
  if (state->expired.load()) {
    gtk_widget_destroy(dialog);
    finish_owner_dialog(state, false);
    return G_SOURCE_REMOVE;
  }
  gtk_widget_show_all(dialog);

  OwnerDialogTimeout timeout{GTK_DIALOG(dialog), false};
  const guint timeout_id = g_timeout_add_seconds(
      static_cast<guint>(kOwnerDialogTimeout.count()),
      reject_expired_owner_dialog, &timeout);
  const gint response = gtk_dialog_run(GTK_DIALOG(dialog));
  if (!timeout.fired) g_source_remove(timeout_id);
  gtk_widget_destroy(dialog);
  finish_owner_dialog(state, response == GTK_RESPONSE_YES);
  return G_SOURCE_REMOVE;
}

void destroy_owner_dialog_callback(gpointer data) {
  delete static_cast<std::shared_ptr<OwnerDialogState>*>(data);
}

struct CredentialDialogState {
  CredentialDialogState(GtkWindow* owner_parent, std::string prompt_challenge,
                        std::string prompt_summary)
      : parent(owner_parent), challenge(std::move(prompt_challenge)),
        summary(std::move(prompt_summary)) {}
  ~CredentialDialogState() {
    secure_clear(&reply);
    if (parent != nullptr) g_object_unref(parent);
  }

  GtkWindow* parent;
  const std::string challenge;
  const std::string summary;
  std::mutex mutex;
  std::condition_variable ready;
  std::atomic<bool> expired{false};
  bool complete = false;
  std::string reply;
};

struct CredentialDialogTimeout {
  GtkDialog* dialog;
  bool fired = false;
};

gboolean cancel_expired_credential_dialog(gpointer data) {
  auto* timeout = static_cast<CredentialDialogTimeout*>(data);
  timeout->fired = true;
  gtk_dialog_response(timeout->dialog, GTK_RESPONSE_CANCEL);
  return G_SOURCE_REMOVE;
}

std::string hex_encode(const unsigned char* bytes, size_t size) {
  static constexpr char alphabet[] = "0123456789abcdef";
  std::string encoded;
  encoded.reserve(size * 2);
  for (size_t index = 0; index < size; ++index) {
    encoded.push_back(alphabet[bytes[index] >> 4]);
    encoded.push_back(alphabet[bytes[index] & 0x0f]);
  }
  return encoded;
}

void finish_credential_dialog(
    const std::shared_ptr<CredentialDialogState>& state,
    std::string reply) {
  {
    std::lock_guard<std::mutex> lock(state->mutex);
    state->reply = state->expired.load() ? std::string() : std::move(reply);
    state->complete = true;
  }
  state->ready.notify_one();
}

gboolean show_credential_dialog_on_gtk_thread(gpointer data) {
  auto state = *static_cast<std::shared_ptr<CredentialDialogState>*>(data);
  if (state->expired.load()) {
    finish_credential_dialog(state, {});
    return G_SOURCE_REMOVE;
  }
  if (state->parent == nullptr || !GTK_IS_WINDOW(state->parent)) {
    finish_credential_dialog(state, {});
    return G_SOURCE_REMOVE;
  }

  GtkWidget* dialog = gtk_dialog_new_with_buttons(
      "D4 Pocket 資格情報入力", state->parent,
      static_cast<GtkDialogFlags>(GTK_DIALOG_MODAL |
                                  GTK_DIALOG_DESTROY_WITH_PARENT),
      "取消", GTK_RESPONSE_CANCEL, "入力して続行", GTK_RESPONSE_ACCEPT,
      nullptr);
  gtk_window_set_position(GTK_WINDOW(dialog), GTK_WIN_POS_CENTER_ON_PARENT);
  gtk_window_set_default_size(GTK_WINDOW(dialog), 640, 360);
  gtk_dialog_set_default_response(GTK_DIALOG(dialog), GTK_RESPONSE_CANCEL);
  GtkWidget* content = gtk_dialog_get_content_area(GTK_DIALOG(dialog));
  GtkWidget* label = gtk_label_new(state->summary.c_str());
  gtk_label_set_line_wrap(GTK_LABEL(label), TRUE);
  gtk_label_set_line_wrap_mode(GTK_LABEL(label), PANGO_WRAP_WORD_CHAR);
  gtk_label_set_xalign(GTK_LABEL(label), 0.0f);
  gtk_label_set_yalign(GTK_LABEL(label), 0.0f);
  gtk_box_pack_start(GTK_BOX(content), label, TRUE, TRUE, 12);
  GtkWidget* entry = gtk_entry_new();
  gtk_entry_set_visibility(GTK_ENTRY(entry), FALSE);
  gtk_entry_set_max_length(GTK_ENTRY(entry), 16384);
  gtk_entry_set_activates_default(GTK_ENTRY(entry), FALSE);
  gtk_widget_set_tooltip_text(entry, "秘密値は表示されません");
  gtk_box_pack_start(GTK_BOX(content), entry, FALSE, FALSE, 12);
  if (state->expired.load()) {
    gtk_widget_destroy(dialog);
    finish_credential_dialog(state, {});
    return G_SOURCE_REMOVE;
  }
  gtk_widget_show_all(dialog);
  gtk_widget_grab_focus(entry);

  CredentialDialogTimeout timeout{GTK_DIALOG(dialog), false};
  const guint timeout_id = g_timeout_add_seconds(
      static_cast<guint>(kCredentialDialogTimeout.count()),
      cancel_expired_credential_dialog, &timeout);
  const gint response = gtk_dialog_run(GTK_DIALOG(dialog));
  if (!timeout.fired) g_source_remove(timeout_id);

  std::string reply = "D4CRED1 " + state->challenge + " CANCELLED";
  if (response == GTK_RESPONSE_ACCEPT) {
    const gchar* value = gtk_entry_get_text(GTK_ENTRY(entry));
    const size_t length = value == nullptr ? 0 : std::strlen(value);
    bool valid = length > 0 && length <= 16384 &&
                 g_utf8_validate(value, static_cast<gssize>(length), nullptr);
    for (const gchar* cursor = value; valid && cursor != nullptr && *cursor;
         cursor = g_utf8_next_char(cursor)) {
      if (g_unichar_iscntrl(g_utf8_get_char(cursor))) valid = false;
    }
    if (valid) {
      std::string encoded = hex_encode(
          reinterpret_cast<const unsigned char*>(value), length);
      reply = "D4CRED1 " + state->challenge + " ENTERED " + encoded;
      secure_clear(&encoded);
    } else {
      reply = "D4CRED1 " + state->challenge + " INVALID";
    }
  }
  gtk_entry_set_text(GTK_ENTRY(entry), "");
  gtk_widget_destroy(dialog);
  finish_credential_dialog(state, std::move(reply));
  return G_SOURCE_REMOVE;
}

void destroy_credential_dialog_callback(gpointer data) {
  delete static_cast<std::shared_ptr<CredentialDialogState>*>(data);
}

bool request_credential_input(GWeakRef* owner_parent,
                              const std::string& challenge,
                              const std::string& summary,
                              std::string* reply) {
  gpointer parent_ref = g_weak_ref_get(owner_parent);
  if (parent_ref == nullptr) return false;
  GObject* parent = G_OBJECT(parent_ref);
  if (!GTK_IS_WINDOW(parent)) {
    g_object_unref(parent);
    return false;
  }
  auto state = std::make_shared<CredentialDialogState>(
      GTK_WINDOW(parent), challenge, summary);
  auto* callback_state = new std::shared_ptr<CredentialDialogState>(state);
  g_idle_add_full(G_PRIORITY_DEFAULT, show_credential_dialog_on_gtk_thread,
                  callback_state, destroy_credential_dialog_callback);
  std::unique_lock<std::mutex> lock(state->mutex);
  if (!state->ready.wait_for(lock, kCredentialDialogTimeout +
                                      std::chrono::seconds(5),
                             [&state] { return state->complete; })) {
    state->expired.store(true);
    return false;
  }
  if (state->reply.empty()) return false;
  *reply = std::move(state->reply);
  return true;
}

struct WorkspaceSelectionState {
  WorkspaceSelectionState(GtkWindow* owner_parent, std::string picker_challenge)
      : parent(owner_parent), challenge(std::move(picker_challenge)) {}
  ~WorkspaceSelectionState() {
    if (parent != nullptr) g_object_unref(parent);
  }

  GtkWindow* parent;
  const std::string challenge;
  std::mutex mutex;
  std::condition_variable ready;
  std::atomic<bool> expired{false};
  bool complete = false;
  std::string reply;
};

struct WorkspaceDialogTimeout {
  GtkDialog* dialog;
  bool fired = false;
};

gboolean fail_expired_workspace_dialog(gpointer data) {
  auto* timeout = static_cast<WorkspaceDialogTimeout*>(data);
  timeout->fired = true;
  gtk_dialog_response(timeout->dialog, GTK_RESPONSE_CANCEL);
  return G_SOURCE_REMOVE;
}

void finish_workspace_selection(
    const std::shared_ptr<WorkspaceSelectionState>& state, std::string reply) {
  {
    std::lock_guard<std::mutex> lock(state->mutex);
    state->reply = state->expired.load() ? std::string() : std::move(reply);
    state->complete = true;
  }
  state->ready.notify_one();
}

gboolean show_workspace_selection_on_gtk_thread(gpointer data) {
  auto state = *static_cast<std::shared_ptr<WorkspaceSelectionState>*>(data);
  if (state->expired.load() || state->parent == nullptr ||
      !GTK_IS_WINDOW(state->parent)) {
    finish_workspace_selection(
        state, "D4PICK1 " + state->challenge + " FAILED");
    return G_SOURCE_REMOVE;
  }

  GtkWidget* dialog = gtk_file_chooser_dialog_new(
      "D4 Pocket 作業領域の選択", state->parent,
      GTK_FILE_CHOOSER_ACTION_SELECT_FOLDER, "取消", GTK_RESPONSE_CANCEL,
      "選択", GTK_RESPONSE_ACCEPT, nullptr);
  gtk_window_set_position(GTK_WINDOW(dialog), GTK_WIN_POS_CENTER_ON_PARENT);
  gtk_window_set_default_size(GTK_WINDOW(dialog), 800, 540);
  gtk_dialog_set_default_response(GTK_DIALOG(dialog), GTK_RESPONSE_CANCEL);
  gtk_file_chooser_set_local_only(GTK_FILE_CHOOSER(dialog), TRUE);
  gtk_file_chooser_set_select_multiple(GTK_FILE_CHOOSER(dialog), FALSE);
  gtk_file_chooser_set_create_folders(GTK_FILE_CHOOSER(dialog), FALSE);
  if (state->expired.load()) {
    gtk_widget_destroy(dialog);
    finish_workspace_selection(
        state, "D4PICK1 " + state->challenge + " FAILED");
    return G_SOURCE_REMOVE;
  }
  gtk_widget_show_all(dialog);

  WorkspaceDialogTimeout timeout{GTK_DIALOG(dialog), false};
  const guint timeout_id = g_timeout_add_seconds(
      static_cast<guint>(kWorkspaceDialogTimeout.count()),
      fail_expired_workspace_dialog, &timeout);
  const gint response = gtk_dialog_run(GTK_DIALOG(dialog));
  if (!timeout.fired) g_source_remove(timeout_id);

  std::string reply = "D4PICK1 " + state->challenge + " FAILED";
  if (timeout.fired) {
    reply = "D4PICK1 " + state->challenge + " FAILED";
  } else if (response != GTK_RESPONSE_ACCEPT) {
    reply = "D4PICK1 " + state->challenge + " CANCELLED";
  } else {
    gchar* filename = gtk_file_chooser_get_filename(GTK_FILE_CHOOSER(dialog));
    GError* error = nullptr;
    gsize converted_length = 0;
    gchar* utf8_path = filename == nullptr
                           ? nullptr
                           : g_filename_to_utf8(filename, -1, nullptr,
                                                &converted_length, &error);
    g_clear_error(&error);
    if (utf8_path != nullptr && converted_length > 0 &&
        converted_length <= 1024 &&
        g_path_is_absolute(utf8_path) &&
        g_utf8_validate(utf8_path, static_cast<gssize>(converted_length),
                        nullptr)) {
      bool valid = true;
      for (const gchar* cursor = utf8_path;
           valid && cursor < utf8_path + converted_length;
           cursor = g_utf8_next_char(cursor)) {
        valid = !g_unichar_iscntrl(g_utf8_get_char(cursor));
      }
      if (valid) {
        const std::string path(utf8_path, converted_length);
        const std::string encoded = hex_encode(
            reinterpret_cast<const unsigned char*>(path.data()), path.size());
        reply = "D4PICK1 " + state->challenge + " SELECTED " + encoded;
      }
    }
    g_free(utf8_path);
    g_free(filename);
  }
  gtk_widget_destroy(dialog);
  finish_workspace_selection(state, std::move(reply));
  return G_SOURCE_REMOVE;
}

void destroy_workspace_selection_callback(gpointer data) {
  delete static_cast<std::shared_ptr<WorkspaceSelectionState>*>(data);
}

bool request_workspace_selection(GWeakRef* owner_parent,
                                 const std::string& challenge,
                                 std::string* reply) {
  gpointer parent_ref = g_weak_ref_get(owner_parent);
  if (parent_ref == nullptr) return false;
  GObject* parent = G_OBJECT(parent_ref);
  if (!GTK_IS_WINDOW(parent)) {
    g_object_unref(parent);
    return false;
  }
  auto state = std::make_shared<WorkspaceSelectionState>(
      GTK_WINDOW(parent), challenge);
  auto* callback_state = new std::shared_ptr<WorkspaceSelectionState>(state);
  g_idle_add_full(G_PRIORITY_DEFAULT, show_workspace_selection_on_gtk_thread,
                  callback_state, destroy_workspace_selection_callback);
  std::unique_lock<std::mutex> lock(state->mutex);
  if (!state->ready.wait_for(lock, kWorkspaceDialogTimeout +
                                      std::chrono::seconds(5),
                             [&state] { return state->complete; })) {
    state->expired.store(true);
    return false;
  }
  if (state->reply.empty()) return false;
  *reply = std::move(state->reply);
  return reply->size() <= kWorkspaceReplyLimit;
}

bool confirm_owner_operation(GWeakRef* owner_parent,
                             const std::string& summary) {
  gpointer parent_ref = g_weak_ref_get(owner_parent);
  if (parent_ref == nullptr) return false;
  GObject* parent = G_OBJECT(parent_ref);
  if (!GTK_IS_WINDOW(parent)) {
    g_object_unref(parent);
    return false;
  }
  auto state = std::make_shared<OwnerDialogState>(GTK_WINDOW(parent), summary);
  auto* callback_state = new std::shared_ptr<OwnerDialogState>(state);
  // g_main_context_invoke may run inline on a worker that can acquire the
  // context. An idle source is dispatched by the GTK main loop instead.
  g_idle_add_full(G_PRIORITY_DEFAULT, show_owner_dialog_on_gtk_thread,
                  callback_state, destroy_owner_dialog_callback);

  std::unique_lock<std::mutex> lock(state->mutex);
  if (!state->ready.wait_for(lock, kOwnerDialogTimeout +
                                      std::chrono::seconds(5),
                             [&state] { return state->complete; })) {
    state->expired.store(true);
    return false;
  }
  return state->confirmed;
}

void helper_wait_completed(GObject* source_object, GAsyncResult* result,
                           gpointer) {
  GError* error = nullptr;
  if (!g_subprocess_wait_finish(G_SUBPROCESS(source_object), result, &error)) {
    g_warning("Linux Broker helper exit status could not be collected");
    g_clear_error(&error);
    return;
  }
  GSubprocess* process = G_SUBPROCESS(source_object);
  if (g_subprocess_get_if_exited(process)) {
    g_warning("Linux Broker helper exited with classified code %d",
              g_subprocess_get_exit_status(process));
  } else if (g_subprocess_get_if_signaled(process)) {
    g_warning("Linux Broker helper terminated by signal %d",
              g_subprocess_get_term_sig(process));
  } else {
    g_warning("Linux Broker helper terminated without a classifiable status");
  }
}

class BrokerContext {
 public:
  explicit BrokerContext(GtkWindow* owner_parent)
      : helper_path_(resolve_helper_path()) {
    g_weak_ref_init(&owner_parent_, G_OBJECT(owner_parent));
  }

  ~BrokerContext() {
    std::lock_guard<std::mutex> lock(mutex_);
    close_input();
    g_clear_object(&stdout_pipe_);
    g_clear_object(&process_);
    g_weak_ref_clear(&owner_parent_);
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

    for (;;) {
      std::string buffer;
      if (!read_helper_frame(&buffer, &error)) {
        g_clear_error(&error);
        poison();
        g_subprocess_wait_async(process_, nullptr, helper_wait_completed, nullptr);
        return false;
      }
      if (has_workspace_picker_prefix(buffer)) {
        std::string challenge;
        if (!decode_workspace_picker_prompt(buffer, &challenge)) {
          poison();
          return false;
        }
        std::string reply;
        if (!request_workspace_selection(&owner_parent_, challenge, &reply)) {
          reply = "D4PICK1 " + challenge + " FAILED";
        }
        reply.push_back('\n');
        const bool written_ok = g_output_stream_write_all(
                                    stdin_pipe_, reply.data(), reply.size(),
                                    &written, nullptr, &error) &&
                                written == reply.size();
        if (!written_ok ||
            g_output_stream_flush(stdin_pipe_, nullptr, &error) == FALSE) {
          g_clear_error(&error);
          poison();
          return false;
        }
        continue;
      }
      if (!has_owner_protocol_prefix(buffer)) {
        if (!has_credential_protocol_prefix(buffer)) {
          *response = std::move(buffer);
          return true;
        }
        std::string challenge;
        std::string summary;
        if (!decode_credential_prompt(buffer, &challenge, &summary)) {
          poison();
          return false;
        }
        std::string reply;
        if (!request_credential_input(&owner_parent_, challenge, summary,
                                      &reply)) {
          reply = "D4CRED1 " + challenge + " CANCELLED";
        }
        reply.push_back('\n');
        const bool written_ok = g_output_stream_write_all(
                                    stdin_pipe_, reply.data(), reply.size(),
                                    &written, nullptr, &error) &&
                                written == reply.size();
        secure_clear(&reply);
        if (!written_ok) {
          g_clear_error(&error);
          poison();
          return false;
        }
        if (g_output_stream_flush(stdin_pipe_, nullptr, &error) == FALSE) {
          g_clear_error(&error);
          poison();
          return false;
        }
        continue;
      }

      std::string challenge;
      std::string summary;
      if (!decode_owner_prompt(buffer, &challenge, &summary)) {
        poison();
        return false;
      }
      const char* decision =
          confirm_owner_operation(&owner_parent_, summary) ? "YES" : "NO";
      const std::string reply =
          "D4OWNER1 " + challenge + " " + decision + "\n";
      if (!g_output_stream_write_all(stdin_pipe_, reply.data(), reply.size(),
                                     &written, nullptr, &error) ||
          written != reply.size()) {
        g_clear_error(&error);
        poison();
        return false;
      }
      if (g_output_stream_flush(stdin_pipe_, nullptr, &error) == FALSE) {
        g_clear_error(&error);
        poison();
        return false;
      }
    }
  }

 private:
  bool read_helper_frame(std::string* frame, GError** error) {
    frame->clear();
    char chunk[4096];
    for (;;) {
      const gssize read = g_input_stream_read(stdout_pipe_, chunk, sizeof(chunk),
                                              nullptr, error);
      if (read <= 0) return false;
      const char* newline = static_cast<const char*>(
          std::memchr(chunk, '\n', static_cast<size_t>(read)));
      const size_t content_size = newline == nullptr
                                      ? static_cast<size_t>(read)
                                      : static_cast<size_t>(newline - chunk);
      const size_t limit =
          frame->rfind("D4OWNER1", 0) == 0 ||
                  frame->rfind("D4CRED1", 0) == 0 ||
                  frame->rfind("D4PICK1", 0) == 0
              ? kOwnerPromptFrameLimit
              : kResponseLimit;
      if (content_size > limit - frame->size() ||
          std::memchr(chunk, '\r', content_size) != nullptr) {
        return false;
      }
      frame->append(chunk, content_size);
      if (newline != nullptr) {
        return content_size + 1 == static_cast<size_t>(read) &&
               !frame->empty() &&
               g_utf8_validate(frame->data(), frame->size(), nullptr);
      }
    }
  }

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
        "HOME",          "XDG_DATA_HOME",   "XDG_RUNTIME_DIR",
        "XDG_STATE_HOME",
        "D4P_LAUNCH_NONCE",
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
  GWeakRef owner_parent_;
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

void broker_process_channel_register(FlPluginRegistry* registry,
                                     GtkWindow* owner_parent) {
  FlPluginRegistrar* registrar = fl_plugin_registry_get_registrar_for_plugin(
      registry, "D4PocketBrokerProcessChannel");
  FlBinaryMessenger* messenger = fl_plugin_registrar_get_messenger(registrar);
  FlStandardMethodCodec* codec = fl_standard_method_codec_new();
  FlMethodChannel* channel = fl_method_channel_new(
      messenger, "gui_shell/broker", FL_METHOD_CODEC(codec));
  g_object_unref(codec);

  auto* context = new std::shared_ptr<BrokerContext>(
      std::make_shared<BrokerContext>(owner_parent));
  fl_method_channel_set_method_call_handler(channel, request_method_call,
                                            context, shared_context_free);
  g_object_set_data_full(G_OBJECT(registry), "d4-pocket-linux-broker-channel",
                         channel, g_object_unref);
}
