#include "broker_pipe_controller.h"

#include <flutter/standard_method_codec.h>

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <mutex>
#include <thread>
#include <utility>
#include <vector>

namespace {

constexpr UINT kBrokerPipeResponseMessage = WM_APP + 42;
constexpr size_t kMaxRequestBytes = 64 * 1024;
constexpr size_t kMaxResponseBytes = 4 * 1024 * 1024;
constexpr size_t kMaxConcurrentRequests = 4;
constexpr DWORD kDefaultPipeTimeoutMs = 5000;
constexpr DWORD kOwnerConfirmationPipeTimeoutMs = 310000;
constexpr wchar_t kPipePrefix[] = L"\\\\.\\pipe\\D4PocketBroker-";
// UTF-8 operation values mirrored from broker_client.dart.
constexpr const char* kOwnerConfirmationOperationValues[] = {
    "\x47\x55\x49\x20\x53\x68\x65\x6C\x6C\xE6\x9B\xB8\xE5\x87\xBA\xE3\x81\x97",
    "\xE5\x9B\x9E\xE5\xB8\xB0\x43\x61\x73\x65\xE7\x99\xBB\xE9\x8C\xB2",
    "\xE5\x9B\x9E\xE5\xB8\xB0\x43\x61\x73\x65\xE5\x89\x8A\xE9\x99\xA4",
    "\xE5\x9B\x9E\xE5\xB8\xB0\x43\x61\x73\x65\xE5\x89\x8A\xE9\x99\xA4\xE4\xB8\xAD\xE6\x96\xAD\xE7\xA2\xBA\xE8\xAA\x8D",
    "\xE8\xB3\x87\xE6\xA0\xBC\xE6\x83\x85\xE5\xA0\xB1\xE5\xA4\xB1\xE5\x8A\xB9",
    "\x4D\x43\x50\xE6\x8E\xA5\xE7\xB6\x9A",
    "\x4D\x43\x50\xE5\x88\x87\xE6\x96\xAD",
    "\x4D\x43\x50\x20\x54\x6F\x6F\x6C\xE5\xAE\x9F\xE8\xA1\x8C",
    "\xE6\x9B\xB4\xE6\x96\xB0\x64\x6F\x77\x6E\x6C\x6F\x61\x64\xE8\xA6\x81\xE6\xB1\x82",
    "\x41\x67\x65\x6E\x74\x54\x61\x73\x6B\x57\x6F\x72\x6B\x73\x70\x61\x63\x65\x50\x65\x72\x6D\x69\x73\x73\x69\x6F\x6E\x47\x72\x61\x6E\x74",
    "\x41\x67\x65\x6E\x74\x54\x61\x73\x6B\x4F\x77\x6E\x65\x72\x41\x70\x70\x72\x6F\x76\x61\x6C\x47\x72\x61\x6E\x74",
    "\x41\x67\x65\x6E\x74\x43\x4C\x49\xE5\xAE\x9F\xE8\xA1\x8C\xE7\xB3\xBB\xE4\xBD\x9C\xE6\xA5\xAD\xE9\xA0\x98\xE5\x9F\x9F\xE7\x99\xBB\xE9\x8C\xB2",
};

bool RequiresOwnerConfirmationPipeTimeout(const std::string& request) {
  const auto operation_key = request.find("\"operation\":");
  if (operation_key == std::string::npos) return false;
  const size_t operation_value = operation_key + sizeof("\"operation\":") - 1;
  for (const char* operation : kOwnerConfirmationOperationValues) {
    const std::string encoded_value = std::string("\"") + operation + "\"";
    if (request.compare(operation_value, encoded_value.size(), encoded_value) == 0) {
      return true;
    }
  }
  return false;
}

struct PipeResult {
  bool ok = false;
  std::string response;
};

struct Completion {
  std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result;
  PipeResult pipe_result;
};

class UniqueHandle {
 public:
  explicit UniqueHandle(HANDLE handle = INVALID_HANDLE_VALUE) : handle_(handle) {}
  ~UniqueHandle() {
    if (handle_ != INVALID_HANDLE_VALUE && handle_ != nullptr) CloseHandle(handle_);
  }
  UniqueHandle(const UniqueHandle&) = delete;
  UniqueHandle& operator=(const UniqueHandle&) = delete;
  HANDLE get() const { return handle_; }

 private:
  HANDLE handle_;
};

bool IsValidPipeName(const std::wstring& value) {
  const std::wstring prefix(kPipePrefix);
  if (value.size() != prefix.size() + 32 || value.compare(0, prefix.size(), prefix) != 0) {
    return false;
  }
  return std::all_of(value.begin() + prefix.size(), value.end(), [](wchar_t ch) {
    return (ch >= L'0' && ch <= L'9') || (ch >= L'a' && ch <= L'f');
  });
}

std::wstring ReadPipeNameFromEnvironment() {
  constexpr wchar_t kVariable[] = L"GUI_SHELL_BROKER_CHANNEL_PIPE";
  wchar_t buffer[128]{};
  const DWORD length = GetEnvironmentVariableW(kVariable, buffer, ARRAYSIZE(buffer));
  if (length == 0 || length >= ARRAYSIZE(buffer)) return L"";
  std::wstring value(buffer, length);
  return IsValidPipeName(value) ? value : L"";
}

DWORD WaitForIo(HANDLE pipe, OVERLAPPED* overlapped,
                std::chrono::steady_clock::time_point deadline,
                DWORD* transferred) {
  const auto now = std::chrono::steady_clock::now();
  if (now >= deadline) return WAIT_TIMEOUT;
  const auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(deadline - now);
  const DWORD wait_ms = static_cast<DWORD>(std::max<int64_t>(1, remaining.count()));
  const DWORD waited = WaitForSingleObject(overlapped->hEvent, wait_ms);
  if (waited != WAIT_OBJECT_0) return waited;
  return GetOverlappedResult(pipe, overlapped, transferred, FALSE) ? WAIT_OBJECT_0 : WAIT_FAILED;
}

DWORD RemainingMilliseconds(std::chrono::steady_clock::time_point deadline) {
  const auto now = std::chrono::steady_clock::now();
  if (now >= deadline) return 0;
  const auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(deadline - now);
  return static_cast<DWORD>(std::max<int64_t>(1, remaining.count() + 1));
}

bool WriteAll(HANDLE pipe, const std::string& bytes,
              std::chrono::steady_clock::time_point deadline) {
  size_t offset = 0;
  while (offset < bytes.size()) {
    UniqueHandle event(CreateEventW(nullptr, TRUE, FALSE, nullptr));
    if (event.get() == nullptr) return false;
    OVERLAPPED overlapped{};
    overlapped.hEvent = event.get();
    DWORD transferred = 0;
    BOOL written = WriteFile(pipe, bytes.data() + offset,
                             static_cast<DWORD>(bytes.size() - offset),
                             &transferred, &overlapped);
    if (!written && GetLastError() != ERROR_IO_PENDING) return false;
    if (!written && WaitForIo(pipe, &overlapped, deadline, &transferred) != WAIT_OBJECT_0) {
      CancelIoEx(pipe, &overlapped);
      WaitForSingleObject(event.get(), INFINITE);
      return false;
    }
    if (transferred == 0) return false;
    offset += transferred;
  }
  return true;
}

bool ReadLine(HANDLE pipe, std::string* response,
              std::chrono::steady_clock::time_point deadline) {
  std::vector<char> chunk(8192);
  while (response->size() <= kMaxResponseBytes) {
    UniqueHandle event(CreateEventW(nullptr, TRUE, FALSE, nullptr));
    if (event.get() == nullptr) return false;
    OVERLAPPED overlapped{};
    overlapped.hEvent = event.get();
    DWORD transferred = 0;
    BOOL read = ReadFile(pipe, chunk.data(), static_cast<DWORD>(chunk.size()),
                         &transferred, &overlapped);
    if (!read && GetLastError() != ERROR_IO_PENDING) return false;
    if (!read && WaitForIo(pipe, &overlapped, deadline, &transferred) != WAIT_OBJECT_0) {
      CancelIoEx(pipe, &overlapped);
      WaitForSingleObject(event.get(), INFINITE);
      return false;
    }
    if (transferred == 0) return false;
    const auto newline = std::find(chunk.begin(), chunk.begin() + transferred, '\n');
    const size_t count = static_cast<size_t>(newline - chunk.begin());
    if (response->size() + count > kMaxResponseBytes) return false;
    response->append(chunk.data(), count);
    if (newline != chunk.begin() + transferred) {
      if (newline + 1 != chunk.begin() + transferred) return false;
      if (!response->empty() && response->back() == '\r') response->pop_back();
      return !response->empty();
    }
  }
  return false;
}

PipeResult Exchange(const std::wstring& pipe_name, const std::string& request) {
  // This only classifies the transport timeout; Rust retains all authority decisions.
  const DWORD timeout_ms = RequiresOwnerConfirmationPipeTimeout(request)
                               ? kOwnerConfirmationPipeTimeoutMs
                               : kDefaultPipeTimeoutMs;
  const auto deadline = std::chrono::steady_clock::now() +
                        std::chrono::milliseconds(timeout_ms);
  if (!IsValidPipeName(pipe_name) || request.empty() || request.size() > kMaxRequestBytes) {
    return {};
  }
  const DWORD wait_ms = RemainingMilliseconds(deadline);
  if (wait_ms == 0 || !WaitNamedPipeW(pipe_name.c_str(), wait_ms)) return {};
  UniqueHandle pipe(CreateFileW(pipe_name.c_str(), GENERIC_READ | GENERIC_WRITE, 0,
                                nullptr, OPEN_EXISTING, FILE_FLAG_OVERLAPPED,
                                nullptr));
  if (pipe.get() == INVALID_HANDLE_VALUE) return {};
  std::string framed(request);
  framed.push_back('\n');
  if (!WriteAll(pipe.get(), framed, deadline)) return {};
  PipeResult result;
  result.ok = ReadLine(pipe.get(), &result.response, deadline);
  return result;
}

}  // namespace

struct BrokerPipeController::State {
  std::mutex mutex;
  HWND window = nullptr;
  bool alive = true;
  std::atomic<size_t> in_flight{0};
};

BrokerPipeController::BrokerPipeController(
    HWND window, flutter::BinaryMessenger* messenger)
    : window_(window),
      pipe_name_(ReadPipeNameFromEnvironment()),
      state_(std::make_shared<State>()),
      channel_(std::make_unique<Channel>(
          messenger, "gui_shell/broker",
          &flutter::StandardMethodCodec::GetInstance())) {
  state_->window = window;
  channel_->SetMethodCallHandler(
      [this](const flutter::MethodCall<flutter::EncodableValue>& call,
             std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
        HandleMethodCall(call, std::move(result));
      });
}

BrokerPipeController::~BrokerPipeController() {
  if (channel_) channel_->SetMethodCallHandler(nullptr);
  {
    std::lock_guard<std::mutex> lock(state_->mutex);
    state_->alive = false;
    state_->window = nullptr;
  }
  MSG message{};
  while (PeekMessageW(&message, window_, kBrokerPipeResponseMessage,
                      kBrokerPipeResponseMessage, PM_REMOVE)) {
    delete reinterpret_cast<Completion*>(message.lParam);
  }
}

bool BrokerPipeController::Initialize() { return !pipe_name_.empty(); }

bool BrokerPipeController::HandleMessage(UINT message, LPARAM data) {
  if (message != kBrokerPipeResponseMessage) return false;
  std::unique_ptr<Completion> completion(reinterpret_cast<Completion*>(data));
  if (!completion || !completion->result) return true;
  if (completion->pipe_result.ok) {
    completion->result->Success(flutter::EncodableValue(
        std::move(completion->pipe_result.response)));
  } else {
    completion->result->Error("broker_unavailable",
                              "Broker unavailable.");
  }
  return true;
}

void BrokerPipeController::HandleMethodCall(
    const flutter::MethodCall<flutter::EncodableValue>& call,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  if (call.method_name() != "request") {
    result->NotImplemented();
    return;
  }
  const auto* request = call.arguments()
                            ? std::get_if<std::string>(call.arguments())
                            : nullptr;
  if (request == nullptr || request->empty() || request->size() > kMaxRequestBytes) {
    result->Error("invalid_request", "Invalid broker request.");
    return;
  }
  if (pipe_name_.empty()) {
    result->Error("broker_unavailable", "Broker unavailable.");
    return;
  }
  const size_t current = state_->in_flight.fetch_add(1, std::memory_order_acq_rel);
  if (current >= kMaxConcurrentRequests) {
    state_->in_flight.fetch_sub(1, std::memory_order_acq_rel);
    result->Error("broker_busy", "Broker request queue is full.");
    return;
  }

  const std::wstring pipe_name = pipe_name_;
  const std::string request_copy = *request;
  const auto state = state_;
  auto result_holder = std::make_shared<
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>>(
      std::move(result));
  try {
    std::thread([pipe_name, request_copy, state, result_holder]() mutable {
      PipeResult pipe_result = Exchange(pipe_name, request_copy);
      auto* completion = new Completion{std::move(*result_holder),
                                        std::move(pipe_result)};
      std::lock_guard<std::mutex> lock(state->mutex);
      state->in_flight.fetch_sub(1, std::memory_order_acq_rel);
      if (!state->alive || state->window == nullptr ||
          !PostMessageW(state->window, kBrokerPipeResponseMessage, 0,
                        reinterpret_cast<LPARAM>(completion))) {
        delete completion;
      }
    }).detach();
  } catch (...) {
    state_->in_flight.fetch_sub(1, std::memory_order_acq_rel);
    if (*result_holder) {
      (*result_holder)->Error("broker_unavailable",
                              "Broker unavailable.");
    }
  }
}
