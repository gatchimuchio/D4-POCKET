import 'package:agent_orchestration/agent_orchestration.dart'
    show
        AgentTaskRequest,
        MobileAgentTaskClient,
        MobileAgentMcpCapabilitySnapshot,
        MobileAgentTaskDraft,
        MobileAgentTaskStatus,
        MobileAgentTaskSubmissionRejected;
import 'package:flutter/material.dart';

import '../services/device_link_controller.dart';

class MobileAgentTaskScreen extends StatefulWidget {
  const MobileAgentTaskScreen({
    super.key,
    required this.controller,
    required this.connected,
    required this.active,
  });

  final DeviceLinkController controller;
  final bool connected;
  final bool active;

  @override
  State<MobileAgentTaskScreen> createState() => _MobileAgentTaskScreenState();
}

class _MobileAgentTaskScreenState extends State<MobileAgentTaskScreen> {
  final _instruction = TextEditingController();
  int _connectionGeneration = 0;
  int _operationSequence = 0;
  int? _activeOperation;
  bool _activeOperationReadOnly = false;
  List<String> _runtimes = const [];
  Map<String, List<String>> _workspaces = const {};
  List<MobileAgentTaskDraft> _requests = const [];
  final Map<String, MobileAgentTaskStatus> _statuses = {};
  String? _runtime;
  String? _workspace;
  String? _message;
  bool _loading = false;
  bool _busy = false;
  bool _includeMcpCapabilities = false;

  MobileAgentTaskClient _clientFor(DeviceLinkController controller) =>
      MobileAgentTaskClient(
        (operation, payload) => controller.request(operation, payload: payload),
      );

  @override
  void initState() {
    super.initState();
    if (widget.connected && widget.active) _reload();
  }

  @override
  void didUpdateWidget(covariant MobileAgentTaskScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    final controllerChanged = !identical(
      oldWidget.controller,
      widget.controller,
    );
    if (controllerChanged ||
        oldWidget.connected != widget.connected ||
        oldWidget.active != widget.active) {
      _connectionGeneration++;
      _loading = false;
      _runtimes = const [];
      _workspaces = const {};
      _requests = const [];
      _statuses.clear();
      _runtime = null;
      _workspace = null;
      _message = null;
      if (controllerChanged || _activeOperationReadOnly) {
        // A stale read cannot keep the recovered connection's controls busy.
        _activeOperation = null;
        _activeOperationReadOnly = false;
        _busy = false;
      }
      if (controllerChanged) {
        // A prompt/session selected for one paired device must not silently
        // become a request to another Broker.
        _instruction.clear();
      }
      if (widget.connected && widget.active) {
        _reload();
      }
    }
  }

  bool _isCurrentConnection(int generation, DeviceLinkController controller) =>
      mounted &&
      generation == _connectionGeneration &&
      identical(widget.controller, controller) &&
      widget.connected &&
      widget.active;

  @override
  void dispose() {
    _instruction.dispose();
    super.dispose();
  }

  Future<void> _reload() async {
    if (_loading || !widget.connected || !widget.active) return;
    final generation = _connectionGeneration;
    final controller = widget.controller;
    setState(() {
      _loading = true;
      _message = null;
    });
    try {
      final dialogue = controller.dialogue;
      final runtimes = await dialogue.runtimes();
      if (!_isCurrentConnection(generation, controller)) return;
      final workspaces = await dialogue.workspaceIdsByRuntime();
      if (!_isCurrentConnection(generation, controller)) return;
      final requests = await _clientFor(controller).ownRequests();
      if (!_isCurrentConnection(generation, controller)) return;
      setState(() {
        _runtimes = runtimes;
        _workspaces = workspaces;
        _runtime = runtimes.contains(_runtime)
            ? _runtime
            : (runtimes.isEmpty ? null : runtimes.first);
        final available = _workspaces[_runtime] ?? const <String>[];
        _workspace = available.contains(_workspace)
            ? _workspace
            : (available.isEmpty ? null : available.first);
        _requests = requests;
        _statuses.removeWhere(
          (id, _) => !requests.any((request) => request.requestId == id),
        );
      });
    } catch (_) {
      if (_isCurrentConnection(generation, controller))
        setState(() => _message = '登録済みRuntime／Workspaceまたは端末Taskを取得できません。');
    } finally {
      if (mounted &&
          generation == _connectionGeneration &&
          identical(widget.controller, controller)) {
        setState(() => _loading = false);
      }
    }
  }

  Future<void> _submit() async {
    final runtime = _runtime;
    final workspace = _workspace;
    final instruction = _instruction.text;
    if (_busy ||
        !widget.connected ||
        !widget.active ||
        runtime == null ||
        workspace == null ||
        instruction.trim().isEmpty) {
      return;
    }
    final generation = _connectionGeneration;
    final controller = widget.controller;
    final operation = ++_operationSequence;
    _activeOperation = operation;
    setState(() {
      _busy = true;
      _message = null;
    });
    String? sessionId;
    var submissionMayHaveReachedBroker = false;
    var mcpCapabilityReadPending = false;
    try {
      MobileAgentMcpCapabilitySnapshot? mcpCapabilities;
      if (_includeMcpCapabilities) {
        mcpCapabilityReadPending = true;
        mcpCapabilities = await _clientFor(controller).mcpCapabilitySnapshot();
        mcpCapabilityReadPending = false;
        if (!_isCurrentConnection(generation, controller)) return;
        if (!mcpCapabilities.usable) {
          setState(
            () => _message = '利用可能なMCP Tool能力を確認できないため、Task申請を送信しませんでした。',
          );
          return;
        }
      }
      sessionId = await controller.dialogue.start(
        runtime,
        workspaceId: workspace,
      );
      if (!_isCurrentConnection(generation, controller)) {
        try {
          await controller.dialogue.close(sessionId);
        } catch (_) {
          // A newly created empty Session is closed only when submission did
          // not reach the Broker; close failure remains an unknown outcome.
        }
        return;
      }
      final request = AgentTaskRequest(
        runtimeId: runtime,
        sessionId: sessionId,
        workspaceId: workspace,
        instruction: instruction,
        mcpCapabilitySnapshotHash: mcpCapabilities?.hash,
      );
      request.validate();
      submissionMayHaveReachedBroker = true;
      final draft = await _clientFor(controller).submit(request);
      if (!_isCurrentConnection(generation, controller)) return;
      setState(() {
        _requests = [
          draft,
          ..._requests.where((item) => item.requestId != draft.requestId),
        ];
        _instruction.clear();
        _message =
            '申請をBrokerへ送信しました。Desktop側のAgent Centerで内容確認と新規Permission／Approvalが必要です。';
      });
    } on MobileAgentTaskSubmissionRejected {
      var sessionClosed = false;
      if (sessionId != null) {
        try {
          await controller.dialogue.close(sessionId);
          sessionClosed = true;
        } catch (_) {
          // 明示拒否後の空Sessionだけを閉じる。
        }
      }
      if (_isCurrentConnection(generation, controller)) {
        setState(
          () => _message = sessionClosed
              ? 'Brokerが申請を拒否しました。Taskは受理されず、Sessionを終了しました。'
              : 'Brokerが申請を拒否しました。Taskは受理されていません。Session終了は未確認です。',
        );
      }
    } catch (_) {
      var sessionClosed = false;
      if (sessionId != null && !submissionMayHaveReachedBroker) {
        try {
          await controller.dialogue.close(sessionId);
          sessionClosed = true;
        } catch (_) {
          // 申請前に失敗した空Sessionだけを閉じる。
        }
      }
      if (_isCurrentConnection(generation, controller)) {
        setState(
          () => _message = submissionMayHaveReachedBroker
              ? '申請結果が不明です。Sessionを維持しました。再送せず、一覧を更新してください。'
              : mcpCapabilityReadPending
              ? '現在のMCP能力を確認できません。Task申請は送信していません。'
              : sessionId == null
              ? 'Session開始を確認できません。申請は再送せず、状態を更新してください。'
              : sessionClosed
              ? '申請前に停止しました。空のSessionを終了しました。'
              : '申請前に停止しました。Session終了は未確認です。',
        );
      }
    } finally {
      if (mounted && _activeOperation == operation) {
        _activeOperation = null;
        setState(() => _busy = false);
      }
      if (submissionMayHaveReachedBroker &&
          generation != _connectionGeneration &&
          mounted &&
          identical(widget.controller, controller) &&
          widget.connected &&
          widget.active) {
        _reload();
      }
    }
  }

  Future<void> _refreshStatus(MobileAgentTaskDraft draft) async {
    await _perform(draft, (controller, generation) async {
      final status = await _clientFor(controller).status(draft);
      if (!_isCurrentConnection(generation, controller) ||
          !_requests.any((request) => request.requestId == draft.requestId)) {
        return;
      }
      setState(() {
        _statuses[draft.requestId] = status;
        _message = '申請状態をBrokerで更新しました。';
      });
    }, readOnly: true);
  }

  Future<void> _start(MobileAgentTaskDraft draft) async {
    final current = _statuses[draft.requestId];
    if (current?.canStart != true) return;
    await _perform(draft, (controller, generation) async {
      final task = await _clientFor(controller).start(draft);
      if (!_isCurrentConnection(generation, controller) ||
          !_requests.any((request) => request.requestId == draft.requestId)) {
        return;
      }
      setState(() {
        _statuses[draft.requestId] = MobileAgentTaskStatus(
          requestId: draft.requestId,
          state: 'Task状態',
          task: task,
        );
        _message = 'BrokerがTask開始を受け付けました。Task結果本文はMobileへ送信しません。';
      });
    });
  }

  Future<void> _cancel(MobileAgentTaskDraft draft) async {
    final current = _statuses[draft.requestId];
    final task = current?.task;
    if (task == null || !const {'pending', 'running'}.contains(task.status))
      return;
    await _perform(draft, (controller, generation) async {
      final updated = await _clientFor(controller).cancel(draft);
      if (!_isCurrentConnection(generation, controller) ||
          !_requests.any((request) => request.requestId == draft.requestId)) {
        return;
      }
      setState(() {
        _statuses[draft.requestId] = MobileAgentTaskStatus(
          requestId: draft.requestId,
          state: 'Task状態',
          task: updated,
        );
        _message = 'BrokerへTask取消を要求しました。';
      });
    });
  }

  Future<void> _perform(
    MobileAgentTaskDraft draft,
    Future<void> Function(DeviceLinkController controller, int generation)
    operation, {
    bool readOnly = false,
  }) async {
    if (_busy || !widget.connected || !widget.active) return;
    final generation = _connectionGeneration;
    final controller = widget.controller;
    final operationId = ++_operationSequence;
    _activeOperation = operationId;
    _activeOperationReadOnly = readOnly;
    setState(() {
      _busy = true;
      _message = null;
    });
    try {
      await operation(controller, generation);
    } catch (_) {
      if (_isCurrentConnection(generation, controller) &&
          _requests.any((request) => request.requestId == draft.requestId)) {
        setState(() {
          _statuses.remove(draft.requestId);
          _message = '操作結果を確認できません。自動再送せず、Broker状態を更新してください。';
        });
      }
    } finally {
      if (mounted && _activeOperation == operationId) {
        _activeOperation = null;
        _activeOperationReadOnly = false;
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final workspaces = _workspaces[_runtime] ?? const <String>[];
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text('Agent Task', style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 8),
        const Text(
          'Mobileは依頼の送信・状態確認・取消要求だけを行います。Permission／Approvalを発行せず、Task結果本文やWorkspace内容も取得しません。実行前にDesktop側で受信内容を確認し、現行条件で別途承認します。',
        ),
        const SizedBox(height: 12),
        if (!widget.connected)
          const Card(
            child: Padding(
              padding: EdgeInsets.all(12),
              child: Text('Device Linkの接続確認後に利用できます。'),
            ),
          ),
        DropdownButtonFormField<String>(
          key: const ValueKey('mobile-agent-task-runtime'),
          initialValue: _runtimes.contains(_runtime) ? _runtime : null,
          decoration: const InputDecoration(labelText: 'Runtime'),
          items: [
            for (final id in _runtimes)
              DropdownMenuItem(value: id, child: Text(id)),
          ],
          onChanged: _busy
              ? null
              : (value) => setState(() {
                  _runtime = value;
                  final values = _workspaces[value] ?? const <String>[];
                  _workspace = values.isEmpty ? null : values.first;
                }),
        ),
        const SizedBox(height: 8),
        DropdownButtonFormField<String>(
          key: const ValueKey('mobile-agent-task-workspace'),
          initialValue: workspaces.contains(_workspace) ? _workspace : null,
          decoration: const InputDecoration(labelText: 'Workspace'),
          items: [
            for (final id in workspaces)
              DropdownMenuItem(value: id, child: Text(id)),
          ],
          onChanged: _busy
              ? null
              : (value) => setState(() => _workspace = value),
        ),
        const SizedBox(height: 8),
        TextField(
          key: const ValueKey('mobile-agent-task-instruction'),
          controller: _instruction,
          maxLines: 5,
          maxLength: 32768,
          enabled: !_busy && widget.connected && widget.active,
          onChanged: (_) => setState(() {}),
          decoration: const InputDecoration(
            border: OutlineInputBorder(),
            labelText: 'Task依頼',
            hintText: '実行したい作業を記入',
          ),
        ),
        SwitchListTile.adaptive(
          key: const ValueKey('mobile-agent-task-mcp-capabilities'),
          contentPadding: EdgeInsets.zero,
          value: _includeMcpCapabilities,
          onChanged: _busy || !widget.connected || !widget.active
              ? null
              : (value) => setState(() => _includeMcpCapabilities = value),
          title: const Text('このTaskでMCP Tool能力を使う'),
          subtitle: const Text(
            '現在のBroker能力hashだけを申請へ含めます。Permission／ApprovalはDesktopで別途必要です。Tool一覧や結果本文はMobileへ返りません。',
          ),
        ),
        Wrap(
          spacing: 8,
          children: [
            FilledButton.icon(
              key: const ValueKey('mobile-agent-task-submit'),
              onPressed:
                  _busy ||
                      !widget.connected ||
                      !widget.active ||
                      _runtime == null ||
                      _workspace == null ||
                      _instruction.text.trim().isEmpty
                  ? null
                  : _submit,
              icon: const Icon(Icons.send),
              label: Text(_busy ? 'Broker応答待ち…' : '新規申請を送信'),
            ),
            OutlinedButton.icon(
              onPressed:
                  _loading || _busy || !widget.connected || !widget.active
                  ? null
                  : _reload,
              icon: const Icon(Icons.refresh),
              label: const Text('登録・申請を更新'),
            ),
          ],
        ),
        if (_message != null) ...[const SizedBox(height: 8), Text(_message!)],
        const SizedBox(height: 16),
        Text('自分の申請（最大16件）', style: Theme.of(context).textTheme.titleMedium),
        if (_requests.isEmpty)
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 12),
            child: Text('端末申請はありません。'),
          ),
        for (final draft in _requests)
          _MobileAgentTaskRequestCard(
            draft: draft,
            status: _statuses[draft.requestId],
            busy: _busy || !widget.connected || !widget.active,
            onRefresh: () => _refreshStatus(draft),
            onStart: () => _start(draft),
            onCancel: () => _cancel(draft),
          ),
      ],
    );
  }
}

class _MobileAgentTaskRequestCard extends StatelessWidget {
  const _MobileAgentTaskRequestCard({
    required this.draft,
    required this.status,
    required this.busy,
    required this.onRefresh,
    required this.onStart,
    required this.onCancel,
  });

  final MobileAgentTaskDraft draft;
  final MobileAgentTaskStatus? status;
  final bool busy;
  final VoidCallback onRefresh;
  final VoidCallback onStart;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) {
    final task = status?.task;
    final state = status?.state ?? (draft.linked ? '開始済み' : '状態未更新');
    return Card(
      child: ExpansionTile(
        key: ValueKey('mobile-agent-task-${draft.requestId}'),
        title: Text('申請 ${draft.requestId}'),
        subtitle: Text('$state ・ ${draft.runtimeId} / ${draft.workspaceId}'),
        childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
        children: [
          Align(
            alignment: Alignment.centerLeft,
            child: SelectableText(
              draft.instruction.isEmpty
                  ? '申請本文はDesktopのAgent Centerで確認してください。'
                  : draft.instruction,
            ),
          ),
          const SizedBox(height: 8),
          if (draft.instructionHash case final hash?) Text('指示hash: $hash'),
          Text(
            '期限（Broker時刻）: ${DateTime.fromMillisecondsSinceEpoch(draft.expiresAt * 1000, isUtc: true).toIso8601String()}',
          ),
          if (status != null && task == null)
            Text(
              status!.canStart
                  ? 'Brokerの現在確認で開始候補です。開始時にBrokerが再評価します。'
                  : 'Desktop側で依頼確認と新規承認が必要です。',
            ),
          if (task != null) ...[
            Text('Task: ${task.status} / ${task.taskId}'),
            Text('結果hash: ${task.resultHash ?? 'unknown'}'),
            const Text(
              'Task結果本文: Mobileには返しません。Desktop Agent Centerで別途確認してください。',
            ),
          ],
          Wrap(
            spacing: 8,
            children: [
              OutlinedButton.icon(
                onPressed: busy ? null : onRefresh,
                icon: const Icon(Icons.sync),
                label: const Text('Broker状態'),
              ),
              if (status?.canStart == true)
                FilledButton.icon(
                  onPressed: busy ? null : onStart,
                  icon: const Icon(Icons.play_arrow),
                  label: const Text('承認済みTaskを開始'),
                ),
              if (task != null &&
                  const {'pending', 'running'}.contains(task.status))
                OutlinedButton.icon(
                  onPressed: busy ? null : onCancel,
                  icon: const Icon(Icons.cancel_outlined),
                  label: const Text('取消要求'),
                ),
            ],
          ),
        ],
      ),
    );
  }
}
