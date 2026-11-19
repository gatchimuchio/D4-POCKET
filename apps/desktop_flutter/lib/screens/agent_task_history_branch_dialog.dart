import 'package:broker_contract/broker_request_envelope.dart'
    show brokerTextHash;
import 'package:flutter/foundation.dart' show ValueListenable;
import 'package:flutter/material.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';

import '../services/agent_task_client.dart'
    show AgentTaskClient, AgentTaskPreflight, AgentTaskRecord, AgentTaskRequest;

class AgentTaskHistoryBranchDialog extends StatefulWidget {
  const AgentTaskHistoryBranchDialog({
    super.key,
    required this.client,
    required this.grant,
    required this.source,
    required this.sourceGeneration,
    required this.generationAtOpen,
    this.replay = false,
  });

  final HistoryClient client;
  final HistoryGrant grant;
  final AgentTaskHistoryEntry source;
  final ValueListenable<int> sourceGeneration;
  final int generationAtOpen;
  final bool replay;

  @override
  State<AgentTaskHistoryBranchDialog> createState() =>
      _AgentTaskHistoryBranchDialogState();
}

class _AgentTaskHistoryBranchDialogState
    extends State<AgentTaskHistoryBranchDialog>
    with WidgetsBindingObserver {
  final _instruction = TextEditingController();
  late final HistoryClient _historyClient;
  late final AgentTaskClient _tasks;
  Map<String, Object?>? _branch;
  AgentTaskRequest? _request;
  AgentTaskPreflight? _preflight;
  AgentTaskRecord? _task;
  bool _busy = false;
  bool _active = true;
  bool _invalidated = false;
  bool _branchAttempted = false;
  bool _permissionAttempted = false;
  bool _permissionGranted = false;
  bool _approvalAttempted = false;
  bool _approvalGranted = false;
  bool _startAttempted = false;
  late String _status = widget.replay
      ? '元Taskと同じ指示を入力してください。Brokerが履歴の指示hashと照合します。'
      : '元Taskの本文・結果本文・権限は引き継ぎません。新しい指示を入力してください。';

  String get _historyAction => widget.replay ? '再実行' : '分岐';

  String get _historyAuditKey =>
      widget.replay ? 'replay_audit_event_id' : 'branch_audit_event_id';

  bool get _scopeCurrent =>
      mounted &&
      !_invalidated &&
      widget.sourceGeneration.value == widget.generationAtOpen;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    final transport = _BranchTransport(
      widget.client.transport,
      () => _scopeCurrent && _active,
    );
    _historyClient = HistoryClient(transport);
    _tasks = AgentTaskClient(transport);
    widget.sourceGeneration.addListener(_sourceChanged);
    _sourceChanged();
  }

  void _sourceChanged() {
    if (widget.sourceGeneration.value != widget.generationAtOpen) {
      _invalidate();
    }
  }

  void _invalidate() {
    if (_invalidated) return;
    _invalidated = true;
    _branch = null;
    _request = null;
    _preflight = null;
    _task = null;
    _busy = false;
    _permissionGranted = false;
    _approvalGranted = false;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      _instruction.clear();
      setState(() {});
    });
    WidgetsBinding.instance.ensureVisualUpdate();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.hidden ||
        state == AppLifecycleState.paused ||
        state == AppLifecycleState.detached) {
      _invalidate();
      return;
    }
    final active = state == AppLifecycleState.resumed;
    if (!mounted || active == _active) return;
    setState(() {
      _active = active;
      if (!active) {
        _instruction.clear();
        _status = '画面が非アクティブです。再開後にBroker状態を確認してください。';
      }
    });
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    widget.sourceGeneration.removeListener(_sourceChanged);
    _instruction.dispose();
    super.dispose();
  }

  Future<void> _run(
    String pending,
    String failure,
    Future<void> Function() action,
  ) async {
    if (_busy || !_active || !_scopeCurrent) return;
    setState(() {
      _busy = true;
      _status = pending;
    });
    try {
      await action();
    } on Object {
      if (_scopeCurrent) setState(() => _status = failure);
    } finally {
      if (_scopeCurrent) setState(() => _busy = false);
    }
  }

  Future<void> _createBranch() async {
    final text = _instruction.text;
    if (_busy || !_active || _branchAttempted) return;
    if (text.trim().isEmpty || text.runes.length > 32768) {
      setState(() => _status = 'Task指示は1〜32768文字で入力してください。');
      return;
    }
    if (widget.replay &&
        brokerTextHash(text) != widget.source.instructionHash) {
      setState(() => _status = '元Taskの指示hashと一致しません。入力を確認してください。');
      return;
    }
    _branchAttempted = true;
    await _run(
      '現在の履歴承認を確認し、新しいSessionと${_historyAction}Auditを作成しています。',
      '$_historyActionの準備結果を確認できません。再送せず、履歴・Audit・Recoveryを確認してください。',
      () async {
        final branch = await (widget.replay
            ? _historyClient.replayAgentTask(widget.grant, widget.source, text)
            : _historyClient.branchAgentTask(
                widget.grant,
                widget.source,
                text,
              ));
        if (!_scopeCurrent) return;
        final request = AgentTaskRequest(
          runtimeId: branch['実行系ID']! as String,
          sessionId: branch['対話セッションID']! as String,
          workspaceId: branch['作業領域ID']! as String,
          instruction: text,
          requireUnusedTaskSession: true,
        );
        setState(() {
          _branch = branch;
          _request = request;
          _instruction.clear();
          _status = '新Sessionと${_historyAction}Auditを作成しました。新Taskを事前検査します。';
        });
      },
    );
    if (_scopeCurrent && _branch != null) await _inspectCreatedBranch();
  }

  Future<void> _inspectCreatedBranch() async {
    final branch = _branch;
    final request = _request;
    if (_busy || !_active || branch == null || request == null) return;
    await _run(
      'Brokerが新TaskのRuntime／Workspace／指示を事前検査しています。',
      'Sessionと${_historyAction}Auditは作成済みです。Taskを開始せず、事前検査を再取得してください。',
      () async {
        final preflight = await _tasks.inspect(request);
        if (!_scopeCurrent) return;
        if (preflight.instructionHash != branch['要求hash'] ||
            preflight.permissionStatus != '未付与' ||
            preflight.approvalStatus != '未取得') {
          setState(() {
            _preflight = null;
            _status = '新Taskのhashまたは未付与状態を確認できません。Permission／開始を停止しました。';
          });
          return;
        }
        setState(() {
          _preflight = preflight;
          _status = 'Broker事前検査済み。PermissionとOwner Approvalは新規発行が必要です。';
        });
      },
    );
  }

  Future<void> _grantPermission() async {
    final request = _request;
    if (_permissionAttempted || request == null || _preflight == null) return;
    _permissionAttempted = true;
    await _run(
      '現在のWorkspaceに一回限りのTask Permissionを要求しています。',
      'Permission結果を確認できません。重ねて要求せず、Broker状態とAuditを確認してください。',
      () async {
        await _tasks.grantWorkspacePermission(request);
        if (!_scopeCurrent) return;
        setState(() {
          _permissionGranted = true;
          _status =
              '新TaskのWorkspace Permissionを確認しました。次に別個のOwner Approvalが必要です。';
        });
      },
    );
  }

  Future<void> _grantApproval() async {
    final request = _request;
    if (!_permissionGranted || _approvalAttempted || request == null) return;
    _approvalAttempted = true;
    await _run(
      '新しいTask指示hashに対する一回限りのOwner Approvalを要求しています。',
      'Owner Approval結果を確認できません。重ねて要求せず、Broker状態とAuditを確認してください。',
      () async {
        await _tasks.grantOwnerApproval(request);
        if (!_scopeCurrent) return;
        setState(() {
          _approvalGranted = true;
          _status = '新TaskのPermissionとOwner Approvalを個別に確認しました。';
        });
      },
    );
  }

  Future<void> _startTask() async {
    final request = _request;
    final preflight = _preflight;
    if (!_permissionGranted ||
        !_approvalGranted ||
        _startAttempted ||
        request == null ||
        preflight == null) {
      return;
    }
    _startAttempted = true;
    await _run(
      '新しいAgent TaskをBrokerから開始しています。',
      '開始結果が不明です。再送せず、Task履歴とRecoveryを確認してください。',
      () async {
        final task = await _tasks.start(request);
        if (!_scopeCurrent) return;
        if (task.taskId == widget.source.taskId ||
            !task.isBoundTo(
              request,
              expectedInstructionHash: preflight.instructionHash,
            )) {
          setState(
            () => _status =
                '開始応答のTask／Session／Workspace／hashが一致しません。結果を再送せず隔離して確認してください。',
          );
          return;
        }
        setState(() {
          _task = task;
          _status =
              '新しいTaskを開始しました。閉じてもBrokerの実行は停止しません。本文は別のExposure承認なしに表示しません。';
        });
      },
    );
  }

  Future<void> _refreshTask() async {
    final request = _request;
    final preflight = _preflight;
    final task = _task;
    if (request == null || preflight == null || task == null) return;
    await _run(
      'BrokerからTask状態を再取得しています。',
      'Task状態を確認できません。Task IDとAuditを使ってRecoveryを確認してください。',
      () async {
        final updated = await _tasks.state(task.taskId);
        if (!updated.isBoundTo(
          request,
          expectedInstructionHash: preflight.instructionHash,
          expectedTaskId: task.taskId,
        )) {
          throw StateError('Task binding mismatch');
        }
        if (_scopeCurrent) setState(() => _task = updated);
      },
    );
  }

  Future<void> _cancelTask() async {
    final request = _request;
    final preflight = _preflight;
    final task = _task;
    if (request == null ||
        preflight == null ||
        task == null ||
        !const {'pending', 'running'}.contains(task.status)) {
      return;
    }
    await _run(
      'BrokerへこのTaskの停止を要求しています。',
      '停止結果を確認できません。Task状態とRecoveryを再取得してください。',
      () async {
        final updated = await _tasks.cancel(task.taskId);
        if (!updated.isBoundTo(
          request,
          expectedInstructionHash: preflight.instructionHash,
          expectedTaskId: task.taskId,
        )) {
          throw StateError('Task binding mismatch');
        }
        if (_scopeCurrent) setState(() => _task = updated);
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    if (!_scopeCurrent) {
      return AlertDialog(
        title: const Text('分岐画面の接続が失効しました'),
        content: const Text('現在の履歴から開き直してください。送信済みの操作は履歴と監査で確認してください。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('閉じる'),
          ),
        ],
      );
    }
    final branch = _branch;
    final preflight = _preflight;
    final task = _task;
    final terminal = const {
      'completed',
      'failed',
      'cancelled',
      'suspended',
    }.contains(widget.source.status);
    return AlertDialog(
      title: Text(
        widget.replay ? 'Agent Taskを同じ指示で再実行' : 'Agent Task履歴から新Taskへ分岐',
      ),
      scrollable: true,
      content: SizedBox(
        width: 620,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '元Task: ${widget.source.taskId} ／ ${widget.source.status}\n'
              'Session: ${widget.source.sessionId}\n'
              '元の本文・結果本文・Permission・Approval・Credential・Authorityは引き継ぎません。',
            ),
            const SizedBox(height: 12),
            TextField(
              key: const ValueKey('agent-task-history-branch-input'),
              controller: _instruction,
              enabled: _active && !_busy && !_branchAttempted && terminal,
              maxLength: 32768,
              minLines: 3,
              maxLines: 7,
              decoration: InputDecoration(
                labelText: widget.replay ? '元Taskと同じ指示' : '新しいTask指示',
              ),
            ),
            if (!terminal) Text('実行中または不明なTaskは$_historyActionできません。'),
            if (branch == null)
              FilledButton(
                onPressed: !_active || _busy || _branchAttempted || !terminal
                    ? null
                    : _createBranch,
                child: const Text('新しいSessionを作成して事前検査'),
              ),
            if (branch != null) ...[
              SelectableText(
                '${_historyAction}Audit: ${branch[_historyAuditKey]}\n'
                '新Session: ${branch['対話セッションID']}\n'
                '新指示hash: ${branch['要求hash']}\n'
                'Workspace: ${branch['作業領域ID']}',
              ),
              if (preflight == null)
                OutlinedButton(
                  onPressed: !_active || _busy ? null : _inspectCreatedBranch,
                  child: const Text('新Taskの事前検査を再取得'),
                ),
              if (preflight != null) ...[
                Text(
                  '事前検査hash: ${preflight.instructionHash}\n'
                  'Permission: ${preflight.permissionStatus} ／ '
                  'Approval: ${preflight.approvalStatus}',
                ),
                OutlinedButton(
                  onPressed:
                      !_active ||
                          _busy ||
                          _permissionAttempted ||
                          _permissionGranted
                      ? null
                      : _grantPermission,
                  child: const Text('新しいWorkspace Permissionを発行'),
                ),
                OutlinedButton(
                  onPressed:
                      !_active ||
                          _busy ||
                          !_permissionGranted ||
                          _approvalAttempted ||
                          _approvalGranted
                      ? null
                      : _grantApproval,
                  child: const Text('新しいOwner Approvalを発行'),
                ),
                FilledButton(
                  onPressed:
                      !_active ||
                          _busy ||
                          !_permissionGranted ||
                          !_approvalGranted ||
                          _startAttempted
                      ? null
                      : _startTask,
                  child: const Text('新しいAgent Taskを開始'),
                ),
              ],
              if (task != null) ...[
                SelectableText(
                  '新Task: ${task.taskId} ／ ${task.status}\n'
                  'Task Audit: ${task.auditEventId}\n'
                  '結果hash: ${task.resultHash ?? '未記録'}\n'
                  '結果本文はここでは取得していません。',
                ),
                Wrap(
                  spacing: 8,
                  children: [
                    OutlinedButton(
                      onPressed: !_active || _busy ? null : _refreshTask,
                      child: const Text('Task状態を再取得'),
                    ),
                    if (const {'pending', 'running'}.contains(task.status))
                      OutlinedButton(
                        onPressed: !_active || _busy ? null : _cancelTask,
                        child: const Text('Taskを停止'),
                      ),
                  ],
                ),
              ],
            ],
            const SizedBox(height: 8),
            if (_busy) const LinearProgressIndicator(),
            Text(_status),
            if (_branchAttempted && branch == null)
              const Text('結果不明時は同じ操作を再送しません。履歴／Audit／Recoveryを確認してください。'),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('閉じる'),
        ),
      ],
    );
  }
}

/// 分岐の前後に行う閲覧承認照会も、直前に同じ接続条件を確認する。
class _BranchTransport implements BrokerTransport {
  const _BranchTransport(this._transport, this._maySend);

  final BrokerTransport _transport;
  final bool Function() _maySend;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) {
    if (!_maySend()) {
      throw const BrokerClientException('分岐画面の接続が失効しました');
    }
    return _transport.request(operation, payload: payload);
  }
}
