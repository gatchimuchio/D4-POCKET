#ifndef RUNNER_BROKER_PIPE_CONTROLLER_H_
#define RUNNER_BROKER_PIPE_CONTROLLER_H_

#include <flutter/binary_messenger.h>
#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>

#include <memory>
#include <string>

#include <windows.h>

class BrokerPipeController {
 public:
  BrokerPipeController(HWND window, flutter::BinaryMessenger* messenger);
  ~BrokerPipeController();

  BrokerPipeController(const BrokerPipeController&) = delete;
  BrokerPipeController& operator=(const BrokerPipeController&) = delete;

  bool Initialize();
  bool HandleMessage(UINT message, LPARAM data);

 private:
  using Channel = flutter::MethodChannel<flutter::EncodableValue>;

  struct State;
  void HandleMethodCall(
      const flutter::MethodCall<flutter::EncodableValue>& call,
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result);

  HWND window_ = nullptr;
  std::wstring pipe_name_;
  std::shared_ptr<State> state_;
  std::unique_ptr<Channel> channel_;
};

#endif  // RUNNER_BROKER_PIPE_CONTROLLER_H_
