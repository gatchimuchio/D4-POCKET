import 'dart:async';
import 'dart:collection';

import 'agent_mcp_task_controller.dart';
import 'agent_task_client.dart';
import 'mcp_connection_client.dart';
import 'package:broker_contract/broker_transport.dart'
    show BrokerClientException, BrokerTransport;
import 'runtime_platform.dart';

typedef AgentTaskExecutionUpdateHandler =
    FutureOr<void> Function(AgentTaskExecutionUpdate update);

/// A view-neutral consumer for completed Agent Tasks and MCP continuation.
///
/// The application creates one coordinator above its screens. Views may observe
/// updates, but disposing or switching a view does not own or stop the run.
/// Permission, Approval, native Owner confirmation, Tool execution and Audit
/// remain Broker operations.
class AgentTaskExecutionCoordinator {
  AgentTaskExecutionCoordinator(
    BrokerTransport transport, {
    this.platform = AgentRuntimePlatform.other,
  }) : _transport = transport;

  static const _maximumRememberedTaskIds = 2048;
  static const _maximumMonitorDuration = Duration(minutes: 16);

  final BrokerTransport _transport;
  final AgentRuntimePlatform platform;
  final LinkedHashSet<String> _rememberedTaskIds = LinkedHashSet<String>();
  final Map<String, _TaskRun> _runs = {};
  final Map<String, String> _currentTaskBySession = {};
  bool _disposed = false;

  /// Broker Taskを監視し、Agentの完了結果が要求したMCP Toolを実行する。
  /// 結果依存の後続Taskは毎回新しいBroker Taskとして開始し、その完了結果が
  /// 次のToolを要求した場合も同じ処理を続ける。
  /// 戻り値のFutureはTask終端時または継続開始後に完了する。
  /// 後続Taskは独立して監視する。
  Future<void> observeStartedTask({
    required AgentTaskRequest request,
    required AgentTaskRecord task,
    required AgentTaskExecutionUpdateHandler onUpdate,
    bool allowMcpContinuation = true,
  }) async {
    if (_disposed || !_remember(task.taskId)) return;
    final priorTaskId = _currentTaskBySession[request.sessionId];
    if (priorTaskId != null && priorTaskId != task.taskId) {
      _runs[priorTaskId]?.stop();
    }
    _currentTaskBySession[request.sessionId] = task.taskId;
    final run = _TaskRun(
      request: request,
      initialTask: task,
      allowMcpContinuation: allowMcpContinuation,
      onUpdate: onUpdate,
    );
    _runs[task.taskId] = run;
    try {
      await _monitor(run);
    } finally {
      if (identical(_runs[task.taskId], run)) _runs.remove(task.taskId);
    }
  }

  /// Stops this application's observation. It does not issue a Task cancel;
  /// Broker process supervision owns execution shutdown and recovery.
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    for (final run in _runs.values) {
      run.stop();
    }
    _runs.clear();
    _currentTaskBySession.clear();
  }

  bool _remember(String taskId) {
    if (_rememberedTaskIds.contains(taskId)) return false;
    if (_rememberedTaskIds.length >= _maximumRememberedTaskIds) {
      _rememberedTaskIds.remove(_rememberedTaskIds.first);
    }
    _rememberedTaskIds.add(taskId);
    return true;
  }

  bool _isCurrent(_TaskRun run) =>
      !_disposed &&
      run.active &&
      _currentTaskBySession[run.request.sessionId] == run.initialTask.taskId &&
      identical(_runs[run.initialTask.taskId], run);

  Future<void> _monitor(_TaskRun run) async {
    var current = run.initialTask;
    _emit(run, task: current);
    if (!_isCurrent(run)) return;

    if (const {'pending', 'running'}.contains(current.status)) {
      final started = DateTime.now();
      var delay = const Duration(seconds: 2);
      while (_isCurrent(run) &&
          DateTime.now().difference(started) < _maximumMonitorDuration &&
          const {'pending', 'running'}.contains(current.status)) {
        await run.wait(delay);
        if (!_isCurrent(run)) return;
        try {
          current = await AgentTaskClient(_transport).state(current.taskId);
          _emit(run, task: current);
        } on Object catch (error) {
          _emit(run, message: _errorMessage(error));
        }
        if (!_isCurrent(run)) return;
        if (const {'pending', 'running'}.contains(current.status)) {
          final elapsed = DateTime.now().difference(started);
          delay = elapsed < const Duration(seconds: 12)
              ? const Duration(seconds: 2)
              : elapsed < const Duration(minutes: 1)
              ? const Duration(seconds: 5)
              : const Duration(seconds: 15);
        }
      }
      if (const {'pending', 'running'}.contains(current.status)) {
        _emit(run, error: '自動監視の上限時間を過ぎました。Broker Taskは保持されています。');
        return;
      }
    }

    if (current.status != 'completed' ||
        !run.allowMcpContinuation ||
        run.request.mcpCapabilitySnapshotHash == null ||
        !_isCurrent(run)) {
      return;
    }

    await _continueAfterTask(run, current);
  }

  Future<void> _continueAfterTask(
    _TaskRun run,
    AgentTaskRecord sourceTask,
  ) async {
    try {
      final outcome =
          await AgentMcpTaskController(
            tasks: AgentTaskClient(_transport),
            mcp: McpConnectionClient(_transport, platform: platform),
          ).continueAfter(
            sourceRequest: run.request,
            sourceTask: sourceTask,
            isCurrent: () => _isCurrent(run),
            onUpdate: (update) => _emit(run, mcpUpdate: update),
            onSessionNotice: (message) => _emitSessionNotice(run, message),
          );
      if (!_isCurrent(run)) return;
      final followup = outcome.followupTask;
      final followupRequest = outcome.followupRequest;
      if (followup == null || followupRequest == null) return;
      unawaited(
        observeStartedTask(
          request: followupRequest,
          task: followup,
          allowMcpContinuation: true,
          onUpdate: run.onUpdate,
        ),
      );
    } on Object catch (error) {
      _emit(run, error: _errorMessage(error));
    }
  }

  Future<void> _emitSessionNotice(_TaskRun run, String message) async {
    if (_disposed) return;
    try {
      await run.onUpdate(
        AgentTaskExecutionUpdate(sessionRecoveryNotice: message),
      );
    } on Object {
      // Recovery notices are informational and never own execution progression.
    }
  }

  void _emit(
    _TaskRun run, {
    AgentTaskRecord? task,
    AgentMcpTaskUpdate? mcpUpdate,
    String? message,
    String? error,
  }) {
    if (!_isCurrent(run)) return;
    try {
      unawaited(
        Future<void>.sync(
          () => run.onUpdate(
            AgentTaskExecutionUpdate(
              task: task,
              mcpUpdate: mcpUpdate,
              message: message,
              error: error,
            ),
          ),
        ).catchError((Object _) {}),
      );
    } on Object {
      // A failed view observer must not own execution progression.
    }
  }

  static String _errorMessage(Object error) => error is BrokerClientException
      ? error.message
      : 'Broker Taskの自動処理に失敗しました。安全のため自動再送はしていません。';
}

class AgentTaskExecutionUpdate {
  const AgentTaskExecutionUpdate({
    this.task,
    this.mcpUpdate,
    this.message,
    this.error,
    this.sessionRecoveryNotice,
  });

  final AgentTaskRecord? task;
  final AgentMcpTaskUpdate? mcpUpdate;
  final String? message;
  final String? error;
  final String? sessionRecoveryNotice;
}

class _TaskRun {
  _TaskRun({
    required this.request,
    required this.initialTask,
    required this.allowMcpContinuation,
    required this.onUpdate,
  });

  final AgentTaskRequest request;
  final AgentTaskRecord initialTask;
  final bool allowMcpContinuation;
  final AgentTaskExecutionUpdateHandler onUpdate;
  bool active = true;
  Timer? _pollTimer;
  Completer<void>? _pollWaiter;

  Future<void> wait(Duration delay) {
    if (!active) return Future<void>.value();
    final waiter = Completer<void>();
    late final Timer timer;
    timer = Timer(delay, () {
      if (identical(_pollWaiter, waiter)) {
        _pollTimer = null;
        _pollWaiter = null;
      }
      if (!waiter.isCompleted) waiter.complete();
    });
    _pollTimer = timer;
    _pollWaiter = waiter;
    return waiter.future;
  }

  void stop() {
    if (!active) return;
    active = false;
    _pollTimer?.cancel();
    _pollTimer = null;
    final waiter = _pollWaiter;
    _pollWaiter = null;
    if (waiter != null && !waiter.isCompleted) waiter.complete();
  }
}
