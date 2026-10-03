#include "tray_controller.h"

#include <shellapi.h>
#include <flutter/standard_method_codec.h>

#include <algorithm>
#include <cstdint>
#include <cwchar>
#include <string>

#include "resource.h"

namespace {

constexpr UINT kTrayCallbackMessage = WM_APP + 41;
constexpr UINT kTrayOpenCommand = 41001;
constexpr UINT kTrayStopCommand = 41002;
constexpr UINT kTrayExitCommand = 41003;
constexpr UINT kTrayIconId = 41004;
constexpr wchar_t kTooltip[] = L"D4 Pocket";
constexpr char kUnknown[] = "\xE4\xB8\x8D\xE6\x98\x8E";

std::wstring Utf8ToWide(const std::string& value) {
  if (value.empty()) return L"";
  const int length = MultiByteToWideChar(
      CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
      static_cast<int>(value.size()), nullptr, 0);
  if (length <= 0) return L"";
  std::wstring result(static_cast<size_t>(length), L'\0');
  if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
                          static_cast<int>(value.size()), result.data(),
                          length) != length) {
    return L"";
  }
  return result;
}

std::string StringValue(const flutter::EncodableMap& map,
                        const char* key,
                        const std::string& fallback) {
  const auto found = map.find(flutter::EncodableValue(key));
  if (found == map.end()) return fallback;
  const auto* value = std::get_if<std::string>(&found->second);
  if (value == nullptr || value->empty() || value->size() > 64) {
    return fallback;
  }
  return *value;
}

bool BoolValue(const flutter::EncodableMap& map, const char* key) {
  const auto found = map.find(flutter::EncodableValue(key));
  if (found == map.end()) return false;
  const auto* value = std::get_if<bool>(&found->second);
  return value != nullptr && *value;
}

std::string CountValue(const flutter::EncodableMap& map, const char* key) {
  const auto found = map.find(flutter::EncodableValue(key));
  if (found == map.end()) return kUnknown;
  if (const auto* value = std::get_if<std::string>(&found->second)) {
    return value->empty() || value->size() > 64 ? kUnknown : *value;
  }
  if (const auto* value = std::get_if<int32_t>(&found->second)) {
    return *value >= 0 && *value <= 256 ? std::to_string(*value) : kUnknown;
  }
  if (const auto* value = std::get_if<int64_t>(&found->second)) {
    return *value >= 0 && *value <= 256 ? std::to_string(*value) : kUnknown;
  }
  return kUnknown;
}

}  // namespace

TrayController::TrayController(HWND window, flutter::BinaryMessenger* messenger)
    : window_(window),
      channel_(std::make_unique<Channel>(
          messenger, "gui_shell/tray",
          &flutter::StandardMethodCodec::GetInstance())) {
  channel_->SetMethodCallHandler(
      [this](const flutter::MethodCall<flutter::EncodableValue>& call,
             std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>
                 result) { HandleMethodCall(call, std::move(result)); });
}

TrayController::~TrayController() {
  RemoveIcon();
  if (channel_) channel_->SetMethodCallHandler(nullptr);
}

bool TrayController::Initialize() {
  return AddIcon();
}

void TrayController::HandleWindowVisibility(bool visible) {
  const auto unknown = std::string(kUnknown);
  if (!visible) {
    runtime_status_ = unknown;
    pending_approval_count_ = unknown;
    critical_notification_count_ = unknown;
  }
  if (window_visibility_known_ && last_window_visible_ == visible) return;
  window_visibility_known_ = true;
  last_window_visible_ = visible;
  if (channel_) {
    channel_->InvokeMethod(
        "onWindowVisibilityChanged",
        std::make_unique<flutter::EncodableValue>(visible));
  }
}

bool TrayController::AddIcon() {
  if (icon_added_ || window_ == nullptr) return icon_added_;
  NOTIFYICONDATAW data{};
  data.cbSize = sizeof(data);
  data.hWnd = window_;
  data.uID = kTrayIconId;
  data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
  data.uCallbackMessage = kTrayCallbackMessage;
  data.hIcon = LoadIconW(GetModuleHandleW(nullptr),
                         MAKEINTRESOURCEW(IDI_APP_ICON));
  wcsncpy_s(data.szTip, ARRAYSIZE(data.szTip), kTooltip, _TRUNCATE);
  if (!Shell_NotifyIconW(NIM_ADD, &data)) return false;
  data.uVersion = NOTIFYICON_VERSION_4;
  Shell_NotifyIconW(NIM_SETVERSION, &data);
  icon_added_ = true;
  return true;
}

void TrayController::RemoveIcon() {
  if (!icon_added_ || window_ == nullptr) return;
  NOTIFYICONDATAW data{};
  data.cbSize = sizeof(data);
  data.hWnd = window_;
  data.uID = kTrayIconId;
  Shell_NotifyIconW(NIM_DELETE, &data);
  icon_added_ = false;
}

void TrayController::HandleMethodCall(
    const flutter::MethodCall<flutter::EncodableValue>& call,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  if (call.method_name() == "initialize") {
    result->Success(
        flutter::EncodableValue(window_ != nullptr && IsWindowVisible(window_)));
    return;
  }
  if (call.method_name() == "isWindowVisible") {
    result->Success(
        flutter::EncodableValue(window_ != nullptr && IsWindowVisible(window_)));
    return;
  }
  if (call.method_name() == "publish") {
    if (call.arguments() != nullptr) {
      UpdateMenuProjection(*call.arguments());
    }
    result->Success();
    return;
  }
  result->NotImplemented();
}

void TrayController::UpdateMenuProjection(
    const flutter::EncodableValue& arguments) {
  const auto* map = std::get_if<flutter::EncodableMap>(&arguments);
  if (map == nullptr) return;
  stop_request_supported_ = BoolValue(*map, "stop_request_supported");
  if (window_ == nullptr || !IsWindowVisible(window_)) {
    HandleWindowVisibility(false);
    return;
  }
  runtime_status_ = StringValue(*map, "runtime_status", kUnknown);
  pending_approval_count_ = CountValue(*map, "pending_approval_count");
  critical_notification_count_ =
      CountValue(*map, "critical_notification_count");
}

void TrayController::NotifyAction(const std::string& action) {
  if (!channel_) return;
  channel_->InvokeMethod(
      "onTrayAction",
      std::make_unique<flutter::EncodableValue>(action));
}

void TrayController::OpenWindow() {
  if (window_ == nullptr) return;
  ShowWindow(window_, SW_RESTORE);
  SetForegroundWindow(window_);
  NotifyAction("open");
}

void TrayController::RequestExit() {
  if (window_ == nullptr) return;
  exit_requested_ = true;
  PostMessageW(window_, WM_CLOSE, 0, 0);
}

void TrayController::ShowMenu() {
  if (window_ == nullptr) return;
  HMENU menu = CreatePopupMenu();
  if (menu == nullptr) return;
  const std::wstring status =
      L"\u5b9f\u884c\u7cfb\u72b6\u614b: " + Utf8ToWide(runtime_status_);
  const std::wstring approvals =
      L"\u4fdd\u7559\u627f\u8a8d: " + Utf8ToWide(pending_approval_count_);
  const std::wstring notifications =
      L"\u91cd\u5927\u901a\u77e5: " + Utf8ToWide(critical_notification_count_);
  AppendMenuW(menu, MF_STRING, kTrayOpenCommand,
              L"D4 Pocket\u3092\u958b\u304f");
  AppendMenuW(menu, MF_STRING | MF_GRAYED, kTrayOpenCommand + 10,
              status.c_str());
  AppendMenuW(menu, MF_STRING | MF_GRAYED, kTrayOpenCommand + 11,
              approvals.c_str());
  AppendMenuW(menu, MF_STRING | MF_GRAYED, kTrayOpenCommand + 12,
              notifications.c_str());
  AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
  AppendMenuW(menu, stop_request_supported_ ? MF_STRING : MF_STRING | MF_GRAYED,
              kTrayStopCommand,
              L"\u5168Runtime\u505c\u6b62\u3092\u8981\u6c42");
  AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
  AppendMenuW(menu, MF_STRING, kTrayExitCommand, L"\u7d42\u4e86");

  POINT cursor{};
  GetCursorPos(&cursor);
  SetForegroundWindow(window_);
  const UINT command = TrackPopupMenu(
      menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON, cursor.x,
      cursor.y, 0, window_, nullptr);
  DestroyMenu(menu);
  if (command == kTrayOpenCommand) OpenWindow();
  if (command == kTrayStopCommand) NotifyAction("stop_request");
  if (command == kTrayExitCommand) RequestExit();
}

bool TrayController::HandleMessage(UINT message,
                                   WPARAM /*wparam*/,
                                   LPARAM lparam) {
  if (message != kTrayCallbackMessage) return false;
  if (lparam == WM_LBUTTONUP || lparam == WM_LBUTTONDBLCLK) {
    OpenWindow();
    return true;
  }
  if (lparam == WM_RBUTTONUP) {
    ShowMenu();
    return true;
  }
  return true;
}
