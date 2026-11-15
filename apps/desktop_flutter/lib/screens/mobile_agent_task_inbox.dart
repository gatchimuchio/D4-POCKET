import 'dart:async';

import 'package:agent_orchestration/agent_orchestration.dart'
    show
        AgentTaskClient,
        AgentTaskDeclaredResult,
        AgentTaskPreflight,
        AgentTaskRecord,
        AgentTaskResultProjection,
        MobileAgentTaskClient,
        MobileAgentTaskDraft;
import 'package:flutter/material.dart';

import '../services/broker_client.dart'
    show BrokerClientException, BrokerTransport;
import 'shared.dart' show BorderedPanel;

class MobileAgentTaskInbox extends StatefulWidget {
  const MobileAgentTaskInbox({super.key, required this.transport});

  final BrokerTransport transport;

  @override
  State<MobileAgentTaskInbox> createState() => _MobileAgentTaskInboxState();
}

class _MobileAgentTaskInboxState extends State<MobileAgentTaskInbox> {
  List<MobileAgentTaskDraft> _requests = const [];
  bool _busy = false;
  String? _message;
  int _generation = 0;

  @override
  void didUpdateWidget(covariant MobileAgentTaskInbox oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (identical(oldWidget.transport, widget.transport)) return;

    final generation = ++_generation;
    _requests = const [];
    _busy = false;
    _message = null;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || generation != _generation) return;
      unawaited(_reload());
    });
  }

  bool _isCurrent(int generation, BrokerTransport transport) =>
      mounted &&
      generation == _generation &&
      identical(widget.transport, transport);

  Future<void> _reload() async {
    if (_busy) return;
    final transport = widget.transport;
    final generation = _generation;
    setState(() {
      _busy = true;
      _message = null;
    });
    try {
      final client = MobileAgentTaskClient(
        (operation, payload) => transport.request(operation, payload: payload),
      );
      final requests = await client.desktopInbox();
      if (!_isCurrent(generation, transport)) return;
      setState(() {
        _requests = requests;
        _message = requests.isEmpty ? '有効なDevice Link申請はありません。' : null;
      });
    } catch (_) {
      if (_isCurrent(generation, transport)) {
        setState(() => _message = 'Device Link申請をBrokerから取得できません。');
      }
    } finally {
      if (_isCurrent(generation, transport)) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) => BorderedPanel(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Mobile Agent Task受信箱',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 6),
            const Text(
              '端末から届いた依頼をBroker内の期限付き下書きとして確認します。受信だけではPermission／Approval／Taskを作りません。本文確認後も、PermissionとTask固有ApprovalはDesktopの既存Broker・native Owner確認で別々に発行します。端末、履歴、Task結果から権限を継承しません。',
            ),
            const SizedBox(height: 8),
            OutlinedButton.icon(
              key: const ValueKey('mobile-agent-task-inbox-refresh'),
              onPressed: _busy ? null : _reload,
              icon: const Icon(Icons.refresh),
              label: Text(_busy ? 'Broker確認中…' : '受信箱を更新'),
            ),
            if (_message != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(_message!),
              ),
            for (final draft in _requests)
              _MobileAgentTaskInboxEntry(
                key: ValueKey('desktop-mobile-task-${draft.requestId}'),
                draft: draft,
                transport: widget.transport,
                onChanged: _reload,
              ),
          ],
        ),
      );
}

class _MobileAgentTaskInboxEntry extends StatefulWidget {
  const _MobileAgentTaskInboxEntry({
    super.key,
    required this.draft,
    required this.transport,
    required this.onChanged,
  });

  final MobileAgentTaskDraft draft;
  final BrokerTransport transport;
  final Future<void> Function() onChanged;

  @override
  State<_MobileAgentTaskInboxEntry> createState() =>
      _MobileAgentTaskInboxEntryState();
}

class _MobileAgentTaskInboxEntryState
    extends State<_MobileAgentTaskInboxEntry> {
  bool _expanded = false;
  bool _reviewed = false;
  bool _busy = false;
  AgentTaskPreflight? _preflight;
  AgentTaskRecord? _task;
  AgentTaskResultProjection? _result;
  String? _error;
  int _generation = 0;
  DialogRoute<void>? _resultDialogRoute;

  bool _sameDraft(MobileAgentTaskDraft left, MobileAgentTaskDraft right) =>
      left.requestId == right.requestId &&
      left.deviceId == right.deviceId &&
      left.runtimeId == right.runtimeId &&
      left.sessionId == right.sessionId &&
      left.workspaceId == right.workspaceId &&
      left.instruction == right.instruction &&
      left.instructionHash == right.instructionHash &&
      left.createdAt == right.createdAt &&
      left.expiresAt == right.expiresAt &&
      left.taskId == right.taskId;

  bool _isCurrent(
    int generation,
    BrokerTransport transport,
    MobileAgentTaskDraft draft,
  ) =>
      mounted &&
      generation == _generation &&
      identical(widget.transport, transport) &&
      _sameDraft(widget.draft, draft);

  void _dismissStaleResultDialog() {
    final route = _resultDialogRoute;
    _resultDialogRoute = null;
    if (route == null) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final navigator = route.navigator;
      if (route.isActive && navigator != null) navigator.removeRoute(route);
    });
  }

  @override
  void didUpdateWidget(covariant _MobileAgentTaskInboxEntry oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (identical(oldWidget.transport, widget.transport) &&
        _sameDraft(oldWidget.draft, widget.draft)) {
      return;
    }

    _generation++;
    _expanded = false;
    _reviewed = false;
    _busy = false;
    _preflight = null;
    _task = null;
    _result = null;
    _error = null;
    _dismissStaleResultDialog();
  }

  @override
  void dispose() {
    _generation++;
    _dismissStaleResultDialog();
    super.dispose();
  }

  Future<void> _perform(
    Future<void> Function(
      AgentTaskClient client,
      MobileAgentTaskDraft draft,
      bool Function() isCurrent,
    ) action,
  ) async {
    if (_busy) return;
    final transport = widget.transport;
    final draft = widget.draft;
    final generation = _generation;
    final client = AgentTaskClient(transport);
    bool isCurrent() => _isCurrent(generation, transport, draft);
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await action(client, draft, isCurrent);
    } on BrokerClientException catch (error) {
      if (isCurrent()) setState(() => _error = error.message);
    } catch (_) {
      if (isCurrent()) {
        setState(() => _error = 'Broker操作の結果を確認できません。状態を更新してください。');
      }
    } finally {
      if (isCurrent()) setState(() => _busy = false);
    }
  }

  Future<void> _refresh() => _perform((client, draft, isCurrent) async {
        final taskId = draft.taskId;
        if (taskId != null) {
          final task = await client.state(taskId);
          if (isCurrent()) setState(() => _task = task);
        } else {
          final preflight = await client.inspect(draft.toAgentTaskRequest());
          if (isCurrent()) setState(() => _preflight = preflight);
        }
      });

  Future<void> _grantPermission() => _perform((client, draft, isCurrent) async {
        if (!_reviewed) throw const BrokerClientException('先に申請本文を確認してください');
        final request = draft.toAgentTaskRequest();
        await client.grantWorkspacePermission(request);
        if (!isCurrent()) return;
        final preflight = await client.inspect(request);
        if (isCurrent()) setState(() => _preflight = preflight);
      });

  Future<void> _grantApproval() => _perform((client, draft, isCurrent) async {
        if (!_reviewed) throw const BrokerClientException('先に申請本文を確認してください');
        final request = draft.toAgentTaskRequest();
        final preflight = await client.inspect(request);
        if (!isCurrent()) return;
        if (preflight.permissionStatus != '有効') {
          setState(() => _preflight = preflight);
          throw const BrokerClientException('Workspace Permissionが有効になっていません');
        }
        await client.grantOwnerApproval(request);
        if (!isCurrent()) return;
        final updated = await client.inspect(request);
        if (isCurrent()) setState(() => _preflight = updated);
      });

  Future<void> _cancel() => _perform((client, draft, isCurrent) async {
        final task = _task;
        if (task == null ||
            task.taskId != draft.taskId ||
            !const {'pending', 'running'}.contains(task.status)) {
          return;
        }
        final updated = await client.cancel(task.taskId);
        if (isCurrent()) setState(() => _task = updated);
      });

  Future<void> _showResult() => _perform((client, draft, isCurrent) async {
        final task = _task;
        if (task == null || task.taskId != draft.taskId) {
          throw const BrokerClientException('Task状態を更新してください');
        }
        final approval = await client.grantResultExposure(task, 'full');
        if (!isCurrent()) return;
        final projection = await client.readResult(approval);
        if (!isCurrent()) return;
        setState(() => _result = projection);
        await _showProjection(projection.declaredResult, task);
      });

  Future<void> _showProjection(
    AgentTaskDeclaredResult? result,
    AgentTaskRecord task,
  ) async {
    final route = DialogRoute<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Agent Task結果（未検証のAgent申告）'),
        content: SizedBox(
          width: 640,
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  'Task ${task.taskId} / result ${task.resultHash ?? 'unknown'}',
                ),
                if (result == null)
                  const Text('定義済みResult projectionへ変換できないため、本文の表示を止めました。')
                else ...[
                  const SizedBox(height: 8),
                  Text('Summary: ${result.summary}'),
                  Text(
                    'Changed files（申告）: ${result.changedFiles.isEmpty ? 'なし' : result.changedFiles.join('、')}',
                  ),
                  Text('Test result（申告）: ${result.testResult}'),
                  const SizedBox(height: 8),
                  const Text('Diff（申告）'),
                  SelectableText(result.diff.isEmpty ? 'なし' : result.diff),
                ],
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('閉じる'),
          ),
        ],
      ),
    );
    _resultDialogRoute = route;
    try {
      await Navigator.of(context, rootNavigator: true).push<void>(route);
    } finally {
      if (identical(_resultDialogRoute, route)) _resultDialogRoute = null;
    }
  }

  @override
  Widget build(BuildContext context) {
    final draft = widget.draft;
    final task = _task;
    final preflight = _preflight;
    return Card(
      child: ExpansionTile(
        key: ValueKey('mobile-task-entry-$_generation'),
        initiallyExpanded: _expanded,
        onExpansionChanged: (value) => setState(() {
          _expanded = value;
          if (!value) _reviewed = false;
        }),
        title: Text('申請 ${draft.requestId} ・ 端末 ${draft.deviceId}'),
        subtitle: Text(
          '${draft.runtimeId} / ${draft.workspaceId} ・ ${draft.linked ? 'Task ${draft.taskId}' : '未開始'}',
        ),
        childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
        children: [
          Align(
            alignment: Alignment.centerLeft,
            child: SelectableText(draft.instruction),
          ),
          const SizedBox(height: 6),
          Text(
            'Session: ${draft.sessionId} / 指示hash: ${draft.instructionHash ?? 'Broker確認前'}',
          ),
          Text(
            '期限（Broker時刻）: ${DateTime.fromMillisecondsSinceEpoch(draft.expiresAt * 1000, isUtc: true).toIso8601String()}',
          ),
          CheckboxListTile(
            contentPadding: EdgeInsets.zero,
            value: _reviewed,
            onChanged: _busy
                ? null
                : (value) => setState(() => _reviewed = value == true),
            title: const Text('依頼本文を確認した'),
          ),
          if (preflight != null)
            Text(
              'Permission: ${preflight.permissionStatus} / Approval: ${preflight.approvalStatus}',
            ),
          if (task != null) ...[
            Text('Task: ${task.status} / ${task.taskId}'),
            Text(
              '結果hash: ${task.resultHash ?? 'unknown'} / 終端Audit: ${task.auditEventId}',
            ),
            if (task.status == 'completed' && task.resultContentAvailable)
              OutlinedButton.icon(
                key: ValueKey('mobile-agent-task-result-${draft.requestId}'),
                onPressed: _busy ? null : _showResult,
                icon: const Icon(Icons.visibility_outlined),
                label: const Text('native Owner確認後に結果／diffを表示'),
              ),
            if (const {'pending', 'running'}.contains(task.status))
              OutlinedButton.icon(
                onPressed: _busy ? null : _cancel,
                icon: const Icon(Icons.cancel_outlined),
                label: const Text('Brokerから取消'),
              ),
          ],
          Wrap(
            spacing: 8,
            runSpacing: 6,
            children: [
              OutlinedButton.icon(
                onPressed: _busy ? null : _refresh,
                icon: const Icon(Icons.sync),
                label: const Text('状態をBrokerから取得'),
              ),
              if (!draft.linked && task == null)
                OutlinedButton.icon(
                  onPressed: _busy || !_reviewed ? null : _grantPermission,
                  icon: const Icon(Icons.folder_open_outlined),
                  label: const Text('Workspace Permissionを個別発行'),
                ),
              if (!draft.linked && task == null)
                FilledButton.icon(
                  onPressed:
                      _busy || !_reviewed || preflight?.permissionStatus != '有効'
                          ? null
                          : _grantApproval,
                  icon: const Icon(Icons.verified_user_outlined),
                  label: const Text('native Owner確認でTask Approval'),
                ),
            ],
          ),
          if (_busy) const LinearProgressIndicator(),
          if (_error != null)
            Align(alignment: Alignment.centerLeft, child: Text(_error!)),
          if (_result != null && _result!.declaredResult == null)
            const Align(
              alignment: Alignment.centerLeft,
              child: Text('Result本文は形式・上限検査に通らなかったため表示していません。'),
            ),
          const Align(
            alignment: Alignment.centerLeft,
            child: Text('端末へのTask結果本文の返送は行いません。TaskはMobile側の同一申請から一回だけ開始できます。'),
          ),
        ],
      ),
    );
  }
}
