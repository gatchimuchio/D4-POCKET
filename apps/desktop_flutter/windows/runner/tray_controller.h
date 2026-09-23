#ifndef RUNNER_TRAY_CONTROLLER_H_
#define RUNNER_TRAY_CONTROLLER_H_

#include <flutter/binary_messenger.h>
#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>

#include <memory>
#include <string>

#include <windows.h>

class TrayController {
 public:
  TrayController(HWND window, flutter::BinaryMessenger* messenger);
  ~TrayController();

  TrayController(const TrayController&) = delete;
  TrayController& operator=(const TrayController&) = delete;

  bool Initialize();
  bool HandleMessage(UINT message, WPARAM wparam, LPARAM lparam);
  bool ExitRequested() const { return exit_requested_; }

 private:
  using Channel = flutter::MethodChannel<flutter::EncodableValue>;

  void HandleMethodCall(
      const flutter::MethodCall<flutter::EncodableValue>& call,
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result);
  void ShowMenu();
  void NotifyAction(const std::string& action);
  void OpenWindow();
  void RequestExit();
  bool AddIcon();
  void RemoveIcon();
  void UpdateMenuProjection(const flutter::EncodableValue& arguments);

  HWND window_ = nullptr;
  std::unique_ptr<Channel> channel_;
  bool icon_added_ = false;
  bool exit_requested_ = false;
  std::string runtime_status_ = "unknown";
  std::string pending_approval_count_ = "unknown";
  std::string critical_notification_count_ = "unknown";
  bool stop_request_supported_ = false;
};

#endif  // RUNNER_TRAY_CONTROLLER_H_
