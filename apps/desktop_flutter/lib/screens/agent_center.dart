import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/agent_task_client.dart';
import '../services/broker_client.dart' show BrokerClientException;
import '../services/runtime_dialogue_client.dart' show RuntimeDialogueClient;
import '../services/shell_core_client.dart';
import '../services/agent_coordination.dart';
import 'shared.dart';
import 'workspace_inspector.dart';

class AgentCenter extends StatefulWidget {
  const AgentCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<AgentCenter> createState() => _AgentCenterState();
}

class _AgentTaskUiState {
  _AgentTaskUiState({
    required this.request,
    required AgentTaskPreflight preflight,
  })  : permissionStatus = preflight.permissionStatus,
        approvalStatus = preflight.approvalStatus;

  AgentTaskRequest? request;
  String permissionStatus;
  String approvalStatus;
  AgentTaskRecord? record;
}

class _AgentCenterState extends State<AgentCenter> {
  bool _registrationPending = false;
  bool _sessionPending = false;
  String? _taskPreflightSession;
  String? _registrationStatus;
  final Map<String, String> _taskPreflightStatus = {};
  final Map<String, _AgentTaskUiState> _agentTasks = {};
  late List<AgentSessionRecord> _sessions;
  _CodexRegistrationInput? _registered;

  @override
  void initState() {
    super.initState();
    final snapshot = widget.client.getSnapshot();
    _sessions = widget.client.mode == 'broker'
        ? List<AgentSessionRecord>.of(snapshot.agentSessions)
        : <AgentSessionRecord>[];
  }

  @override
  void dispose() {
    _agentTasks.clear();
    _taskPreflightStatus.clear();
    super.dispose();
  }

  Future<void> _registerCodexRuntime() async {
    final input = await showDialog<_CodexRegistrationInput>(
      context: context,
      builder: (context) => const _CodexRegistrationDialog(),
    );
    if (!mounted || input == null) return;
    final transport = widget.client.brokerTransport;
    if (transport == null) {
      setState(() => _registrationStatus = 'Broker接続がないため登録を停止しました。');
      return;
    }
    setState(() {
      _registrationPending = true;
      _registrationStatus = null;
    });
    try {
      final response = await transport.request(
        'AgentCLI実行系作業領域登録',
        payload: input.toPayload(),
      );
      if (!mounted) return;
      if (response['status'] == 'accepted') {
        final body = response['body'];
        const expectedKeys = {
          'runtime_id',
          'workspace_id',
          'registration_lifetime',
          'task_execution',
          'permission_generated',
          'approval_generated',
          'credential_value_accepted',
        };
        if (response['operation'] != 'AgentCLI実行系作業領域登録' ||
            response['evidence_source'] != 'LIVE_RUNTIME' ||
            response['audit_event_id'] is! String ||
            (response['audit_event_id'] as String).isEmpty ||
            (response['audit_event_id'] as String).length > 256 ||
            body is! Map ||
            body.length != expectedKeys.length ||
            body.keys.any((key) => !expectedKeys.contains(key)) ||
            body['runtime_id'] != input.runtimeId ||
            body['workspace_id'] != input.workspaceId ||
            body['registration_lifetime'] != 'broker_process' ||
            !const {'supported', 'unsupported', 'unknown'}
                .contains(body['task_execution']) ||
            body['permission_generated'] != false ||
            body['approval_generated'] != false ||
            body['credential_value_accepted'] != false) {
          setState(() => _registrationStatus = 'Broker登録応答を検証できないため停止しました。');
          return;
        }
        setState(() {
          _registered = input.withTaskExecutionStatus(
            body['task_execution']! as String,
          );
          _registrationStatus =
              'Broker起動中だけ登録しました。Task実行能力: ${body['task_execution']}。';
        });
      } else {
        final error = response['error'];
        final message = error is Map ? error['message']?.toString() : null;
        setState(() {
          _registrationStatus =
              message == null || message.isEmpty ? '登録は受理されませんでした。' : message;
        });
      }
    } catch (_) {
      if (mounted) {
        setState(() => _registrationStatus = '安全Brokerへの登録に失敗しました。');
      }
    } finally {
      if (mounted) setState(() => _registrationPending = false);
    }
  }

  Future<void> _startRegisteredSession() async {
    final registration = _registered;
    final transport = widget.client.brokerTransport;
    if (registration == null || transport == null) return;
    setState(() {
      _sessionPending = true;
      _registrationStatus = null;
    });
    try {
      final sessionId = await RuntimeDialogueClient(transport).start(
        registration.runtimeId,
        workspaceId: registration.workspaceId,
      );
      final sessions = await widget.client.refreshAgentSessions();
      final created =
          sessions.where((session) => session.sessionId == sessionId);
      if (created.length != 1 ||
          created.single.agentRuntimeId != registration.runtimeId ||
          created.single.workspace != registration.workspaceId ||
          created.single.status != '利用中') {
        throw const BrokerClientException('Brokerが新規Agent Sessionを照合できません');
      }
      if (!mounted) return;
      setState(() {
        _sessions = sessions;
        _registrationStatus = 'BrokerがSessionとWorkspaceの結合を監査しました。Taskは未実行です。';
      });
    } on Object {
      if (mounted) {
        setState(() => _registrationStatus = 'Broker Session開始または一覧照合に失敗しました。');
      }
    } finally {
      if (mounted) setState(() => _sessionPending = false);
    }
  }

  Future<void> _inspectTask(AgentSessionRecord session) async {
    final transport = widget.client.brokerTransport;
    final registration = _registered;
    if (transport == null ||
        registration == null ||
        session.agentRuntimeId != registration.runtimeId ||
        session.workspace != registration.workspaceId ||
        session.status != '利用中') {
      return;
    }
    final instruction = await showDialog<String>(
      context: context,
      builder: (context) => const _AgentTaskInstructionDialog(),
    );
    if (!mounted || instruction == null) return;
    final request = AgentTaskRequest(
      runtimeId: session.agentRuntimeId,
      sessionId: session.sessionId,
      workspaceId: session.workspace,
      instruction: instruction,
    );
    setState(() {
      _taskPreflightSession = session.sessionId;
      _agentTasks.remove(session.sessionId);
      _taskPreflightStatus.remove(session.sessionId);
    });
    try {
      final result = await AgentTaskClient(transport).inspect(request);
      if (!mounted) return;
      setState(() {
        _agentTasks[session.sessionId] =
            _AgentTaskUiState(request: request, preflight: result);
        _taskPreflightStatus[session.sessionId] =
            'Broker事前検査済み。Permission=${result.permissionStatus}、Approval=${result.approvalStatus}。';
      });
    } on Object catch (error) {
      if (!mounted) return;
      final message =
          error is BrokerClientException ? error.message : 'Broker事前検査に失敗しました。';
      setState(() => _taskPreflightStatus[session.sessionId] = message);
    } finally {
      if (mounted) setState(() => _taskPreflightSession = null);
    }
  }

  bool _sessionStillMatchesRegistration(AgentSessionRecord session) {
    final registration = _registered;
    return registration != null &&
        session.status == '利用中' &&
        session.agentRuntimeId == registration.runtimeId &&
        session.workspace == registration.workspaceId;
  }

  bool _taskSessionStillMatches(
    AgentSessionRecord session,
    AgentTaskRequest request,
  ) =>
      _sessionStillMatchesRegistration(session) &&
      request.runtimeId == session.agentRuntimeId &&
      request.sessionId == session.sessionId &&
      request.workspaceId == session.workspace;

  Future<void> _runAgentTaskAction(
    AgentSessionRecord session,
    Future<void> Function(AgentTaskClient client, _AgentTaskUiState state)
        action,
  ) async {
    final transport = widget.client.brokerTransport;
    final state = _agentTasks[session.sessionId];
    final request = state?.request;
    if (transport == null ||
        state == null ||
        request == null ||
        _taskPreflightSession != null ||
        !_taskSessionStillMatches(session, request)) {
      return;
    }
    setState(() {
      _taskPreflightSession = session.sessionId;
      _taskPreflightStatus[session.sessionId] = 'Broker応答を確認中です。';
    });
    try {
      await action(AgentTaskClient(transport), state);
      if (!mounted) return;
      setState(() {
        _taskPreflightStatus[session.sessionId] = state.record == null
            ? 'Brokerの承認状態を更新しました。実行状態は未確認です。'
            : 'Broker Task状態: ${state.record!.status}。本文は表示せず、Workspace差分とは別に扱います。';
      });
    } on Object catch (error) {
      if (!mounted) return;
      final message = error is BrokerClientException
          ? error.message
          : 'Broker Task操作に失敗しました。';
      setState(() => _taskPreflightStatus[session.sessionId] = message);
    } finally {
      if (mounted) setState(() => _taskPreflightSession = null);
    }
  }

  Future<void> _grantTaskWorkspacePermission(
    AgentSessionRecord session,
  ) async {
    await _runAgentTaskAction(session, (client, state) async {
      final request = state.request!;
      await client.grantWorkspacePermission(request);
      state.permissionStatus = '有効';
    });
  }

  Future<void> _grantTaskOwnerApproval(AgentSessionRecord session) async {
    await _runAgentTaskAction(session, (client, state) async {
      if (state.permissionStatus != '有効') {
        throw const BrokerClientException(
          'Workspace PermissionがBrokerで確認されるまでOwner Approvalを要求できません',
        );
      }
      await client.grantOwnerApproval(state.request!);
      state.approvalStatus = '有効';
    });
  }

  Future<void> _startAgentTask(AgentSessionRecord session) async {
    await _runAgentTaskAction(session, (client, state) async {
      if (state.permissionStatus != '有効' || state.approvalStatus != '有効') {
        throw const BrokerClientException(
          'Workspace PermissionとTask Owner ApprovalをBrokerで確認できません',
        );
      }
      state.record = await client.start(state.request!);
      state.request = null;
    });
  }

  Future<void> _refreshAgentTask(AgentSessionRecord session) async {
    final transport = widget.client.brokerTransport;
    final state = _agentTasks[session.sessionId];
    final taskId = state?.record?.taskId;
    if (transport == null ||
        state == null ||
        taskId == null ||
        _taskPreflightSession != null ||
        !_sessionStillMatchesRegistration(session)) {
      return;
    }
    setState(() {
      _taskPreflightSession = session.sessionId;
      _taskPreflightStatus[session.sessionId] = 'Broker Task状態を更新中です。';
    });
    try {
      state.record = await AgentTaskClient(transport).state(taskId);
      if (!mounted) return;
      setState(() {
        _taskPreflightStatus[session.sessionId] =
            'Broker Task状態: ${state.record!.status}。';
      });
    } on Object catch (error) {
      if (!mounted) return;
      final message = error is BrokerClientException
          ? error.message
          : 'Broker Task状態を確認できません。';
      setState(() => _taskPreflightStatus[session.sessionId] = message);
    } finally {
      if (mounted) setState(() => _taskPreflightSession = null);
    }
  }

  Future<void> _cancelAgentTask(AgentSessionRecord session) async {
    final transport = widget.client.brokerTransport;
    final state = _agentTasks[session.sessionId];
    final taskId = state?.record?.taskId;
    if (transport == null ||
        state == null ||
        taskId == null ||
        _taskPreflightSession != null ||
        !_sessionStillMatchesRegistration(session) ||
        !const {'pending', 'running'}.contains(state.record!.status)) {
      return;
    }
    setState(() {
      _taskPreflightSession = session.sessionId;
      _taskPreflightStatus[session.sessionId] = '取消要求をBrokerへ送信中です。';
    });
    try {
      state.record = await AgentTaskClient(transport).cancel(taskId);
      if (!mounted) return;
      setState(() {
        _taskPreflightStatus[session.sessionId] =
            '取消要求を記録しました。停止完了はBroker状態の再確認が必要です。';
      });
    } on Object catch (error) {
      if (!mounted) return;
      final message =
          error is BrokerClientException ? error.message : 'Task取消要求に失敗しました。';
      setState(() => _taskPreflightStatus[session.sessionId] = message);
    } finally {
      if (mounted) setState(() => _taskPreflightSession = null);
    }
  }

  List<Widget> _agentTaskControls(AgentSessionRecord session) {
    final taskState = _agentTasks[session.sessionId];
    if (taskState == null) return const [];
    final task = taskState.record;
    final busy = _taskPreflightSession == session.sessionId;
    return [
      SectionList(
        title: 'Broker Task承認状態',
        rows: [
          '作業領域Permission状態: ${taskState.permissionStatus}',
          'Task一回承認状態（Owner Approval）: ${taskState.approvalStatus}',
          if (task != null) 'Task識別子: ${task.taskId}',
          if (task != null) 'Task状態: ${task.status}',
          if (task != null) '指示hash: ${task.instructionHash}',
          if (task?.resultHash != null) '結果hash: ${task!.resultHash}',
          if (task != null) '終了監査参照: ${task.auditEventId}',
        ],
      ),
      if (task == null && taskState.permissionStatus != '有効')
        OutlinedButton(
          onPressed: busy ? null : () => _grantTaskWorkspacePermission(session),
          child: const Text('Workspace PermissionのOwner確認'),
        ),
      if (task == null &&
          taskState.permissionStatus == '有効' &&
          taskState.approvalStatus != '有効')
        OutlinedButton(
          onPressed: busy ? null : () => _grantTaskOwnerApproval(session),
          child: const Text('Task一回ApprovalのOwner確認'),
        ),
      if (task == null &&
          taskState.permissionStatus == '有効' &&
          taskState.approvalStatus == '有効')
        FilledButton(
          onPressed: busy ? null : () => _startAgentTask(session),
          child: Text(busy ? 'Broker応答待ち' : 'Taskを一回実行'),
        ),
      if (task != null)
        OutlinedButton(
          onPressed: busy ? null : () => _refreshAgentTask(session),
          child: const Text('Task状態を更新'),
        ),
      if (task != null && const {'pending', 'running'}.contains(task.status))
        OutlinedButton(
          onPressed: busy ? null : () => _cancelAgentTask(session),
          child: const Text('Task停止を要求'),
        ),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final client = widget.client;
    final snapshot = client.getSnapshot();
    final adapters = snapshot.agentAdapters;
    final sessions =
        client.mode == 'broker' ? _sessions : const <AgentSessionRecord>[];
    final comparison = AgentComparisonProjection.fromSessions(sessions);
    return ShellPage(
      title: 'エージェントセンター',
      children: [
        if (client.mode == 'broker')
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Codex実行系と作業領域',
                    style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                const Text(
                  'Broker起動中だけの登録です。Rust BrokerがOwner確認後にCodex CLIのinterfaceを検査します。これはTask実行・Permission・Approval・Trustを有効にしません。',
                ),
                const SizedBox(height: 8),
                OutlinedButton.icon(
                  onPressed:
                      _registrationPending ? null : _registerCodexRuntime,
                  icon: _registrationPending
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.add),
                  label: Text(_registrationPending ? 'Broker応答待ち' : '登録を開始'),
                ),
                if (_registrationStatus != null) ...[
                  const SizedBox(height: 8),
                  Text(_registrationStatus!),
                ],
                if (_registered != null) ...[
                  const SizedBox(height: 8),
                  SectionList(
                    title: '今回のBroker内登録',
                    rows: [
                      '実行系ID: ${_registered!.runtimeId}',
                      '作業領域ID: ${_registered!.workspaceId}',
                      'Task実行: ${_registered!.taskExecutionStatus}',
                      'Permission／Approval／Credential: 生成なし',
                    ],
                  ),
                  OutlinedButton(
                    onPressed: _sessionPending ? null : _startRegisteredSession,
                    child: Text(_sessionPending
                        ? 'Broker Session開始待ち'
                        : '登録Workspaceで対話Sessionを開始'),
                  ),
                ],
              ],
            ),
          ),
        if (client.workspaceClient != null)
          BorderedPanel(
              child: WorkspaceInspector(client: client.workspaceClient!)),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Agent Adapter状態',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 8),
              if (adapters.isEmpty)
                const SectionList(
                  title: '状態',
                  rows: ['Agent Adapterの実物登録はありません。unknownを利用可能へ昇格しません。'],
                )
              else
                for (final adapter in adapters)
                  _AgentAdapterPanel(adapter: adapter),
            ],
          ),
        ),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                '複数Agent比較',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 8),
              SectionList(
                title: '状態',
                rows: [
                  if (client.mode != 'broker')
                    'Local／mock snapshotはAgent実行結果ではないため比較対象にしません。'
                  else if (comparison.available)
                    'Broker metadata上の識別子重複はありませんが、実Agent比較・実行時隔離は未接続です。'
                  else
                    comparison.statusMessage,
                ],
              ),
              SectionList(
                title: '対象セッション',
                rows: comparison.sessionIds.isEmpty
                    ? ['なし']
                    : comparison.sessionIds,
              ),
              SectionList(
                title: '安全境界',
                rows: [
                  comparison.available
                      ? 'Agent runtime ID／Workspace参照の重複なし（実行時の隔離は未検証）'
                      : '比較条件が成立していません',
                  'Authority・Approval・Credentialは共有しない',
                ],
              ),
            ],
          ),
        ),
        const BorderedPanel(
          child: SectionList(
            title: 'Agent引き継ぎ',
            rows: [
              '未接続です。Task成果・diff・試験結果のBroker経路が成立するまで引き継ぎ概要を生成しません。',
              '接続後もAuthority・Permission・Approval・Credential・hidden contextは引き継がず、target条件で再評価します。',
            ],
          ),
        ),
        if (client.mode != 'broker' && snapshot.agentSessions.isNotEmpty)
          const BorderedPanel(
            child: SectionList(
              title: 'エージェント実行',
              rows: ['ローカルの模擬データは、Broker上のエージェント実行として表示しません。'],
            ),
          ),
        for (final session in sessions)
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  session.sessionId,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                SectionList(
                    title: 'Agent実行系ID', rows: [session.agentRuntimeId]),
                SectionList(title: '対話状態', rows: [session.status]),
                const SectionList(
                  title: '証拠種別',
                  rows: ['INTERNAL_STATE（Broker対話Session一覧）'],
                ),
                SectionList(title: '作成監査ID', rows: [session.auditEventId]),
                if (session.workspace.trim().isEmpty)
                  const Text('Workspace結合は未確認です。')
                else ...[
                  SectionList(
                    title: 'Broker登録Workspace ID',
                    rows: [session.workspace],
                  ),
                  SectionList(
                    title: 'Workspace結合監査ID',
                    rows: [session.workspaceAuditEventId],
                  ),
                  const Text(
                    'Runtime一致はBroker内で検証済みです。実Agentの実行Session・書込み隔離は未検証です。',
                  ),
                ],
                const Text(
                  'この画面のTask操作は事前検査のみです。Brokerがunsupportedを返した場合はPermission／Approval要求へ進みません。Task実行・状態・結果は未接続で、Capability表示だけでは権限を与えません。',
                ),
                if (_registered != null &&
                    session.agentRuntimeId == _registered!.runtimeId &&
                    session.workspace == _registered!.workspaceId) ...[
                  OutlinedButton(
                    onPressed: _taskPreflightSession == session.sessionId
                        ? null
                        : () => _inspectTask(session),
                    child: Text(_taskPreflightSession == session.sessionId
                        ? 'Broker事前検査中'
                        : 'Task実行能力を事前検査（実行なし）'),
                  ),
                  if (_taskPreflightStatus[session.sessionId] != null)
                    Text(_taskPreflightStatus[session.sessionId]!),
                  ..._agentTaskControls(session),
                ],
                const SectionList(
                  title: '未接続の実行情報',
                  rows: [
                    '作業領域: 実行directoryは未確認',
                    'タスク: 事前検査のみ。実行状態は未接続',
                    '変更ファイル: Brokerから未取得',
                    '道具呼出し: Brokerから未取得',
                    'シェルコマンド: Brokerから未取得',
                    '試験状態: Brokerから未取得',
                    '差分概要: Brokerから未取得',
                    '保留中の承認: Brokerから未取得（承認がないことを意味しません）',
                    '巻戻し候補: Brokerから未取得',
                    '監査リンク: 上記の監査ID参照のみ',
                  ],
                ),
              ],
            ),
          ),
      ],
    );
  }
}

class _CodexRegistrationInput {
  const _CodexRegistrationInput({
    required this.runtimeId,
    required this.codexCliPath,
    required this.workspaceId,
    required this.workspaceRoot,
    required this.secretPaths,
    this.taskExecutionStatus = 'unknown',
  });

  final String runtimeId;
  final String codexCliPath;
  final String workspaceId;
  final String workspaceRoot;
  final List<String> secretPaths;
  final String taskExecutionStatus;

  _CodexRegistrationInput withTaskExecutionStatus(String status) =>
      _CodexRegistrationInput(
        runtimeId: runtimeId,
        codexCliPath: codexCliPath,
        workspaceId: workspaceId,
        workspaceRoot: workspaceRoot,
        secretPaths: secretPaths,
        taskExecutionStatus: status,
      );

  Map<String, Object?> toPayload() => {
        'version': 1,
        'adapter_id': 'codex-cli',
        'runtime_id': runtimeId,
        'cli_path': codexCliPath,
        'workspace_id': workspaceId,
        'workspace_root': workspaceRoot,
        'secret_paths': secretPaths,
      };
}

class _CodexRegistrationDialog extends StatefulWidget {
  const _CodexRegistrationDialog();

  @override
  State<_CodexRegistrationDialog> createState() =>
      _CodexRegistrationDialogState();
}

class _CodexRegistrationDialogState extends State<_CodexRegistrationDialog> {
  final _formKey = GlobalKey<FormState>();
  final _runtimeId = TextEditingController(text: 'codex-local');
  final _cliPath = TextEditingController();
  final _workspaceId = TextEditingController(text: 'workspace-local');
  final _workspaceRoot = TextEditingController();
  final _secretPaths = TextEditingController();

  @override
  void dispose() {
    _runtimeId.dispose();
    _cliPath.dispose();
    _workspaceId.dispose();
    _workspaceRoot.dispose();
    _secretPaths.dispose();
    super.dispose();
  }

  String? _required(String? value) =>
      value == null || value.trim().isEmpty ? '入力してください' : null;

  @override
  Widget build(BuildContext context) => AlertDialog(
        title: const Text('Codex実行系をBrokerへ登録'),
        content: SizedBox(
          width: 560,
          child: SingleChildScrollView(
            child: Form(
              key: _formKey,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Text(
                    '入力値はRust Brokerへ送られ、Owner確認画面に登録範囲として表示されます。CLIは --version と exec --help のみ検査します。Credential値は入力しないでください。',
                  ),
                  const SizedBox(height: 12),
                  TextFormField(
                    controller: _runtimeId,
                    decoration:
                        const InputDecoration(labelText: '実行系ID（Runtime ID）'),
                    validator: _required,
                  ),
                  TextFormField(
                    controller: _cliPath,
                    decoration: const InputDecoration(
                      labelText: 'Codex CLI実行fileの絶対path',
                      hintText: r'C:\Tools\Codex\codex.exe',
                    ),
                    validator: _required,
                  ),
                  TextFormField(
                    controller: _workspaceId,
                    decoration: const InputDecoration(
                        labelText: '作業領域ID（Workspace ID）'),
                    validator: _required,
                  ),
                  TextFormField(
                    controller: _workspaceRoot,
                    decoration: const InputDecoration(
                      labelText: 'Workspace rootの絶対path',
                    ),
                    validator: _required,
                  ),
                  TextFormField(
                    controller: _secretPaths,
                    minLines: 1,
                    maxLines: 4,
                    decoration: const InputDecoration(
                      labelText: '除外する秘密path（相対path、1行に1件）',
                      hintText: '.env\nsecrets',
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('キャンセル'),
          ),
          FilledButton(
            onPressed: () {
              if (!_formKey.currentState!.validate()) return;
              final secrets = _secretPaths.text
                  .split(RegExp(r'[\r\n]+'))
                  .map((path) => path.trim())
                  .where((path) => path.isNotEmpty)
                  .toList(growable: false);
              if (secrets.length > 16) return;
              Navigator.of(context).pop(_CodexRegistrationInput(
                runtimeId: _runtimeId.text.trim(),
                codexCliPath: _cliPath.text.trim(),
                workspaceId: _workspaceId.text.trim(),
                workspaceRoot: _workspaceRoot.text.trim(),
                secretPaths: secrets,
              ));
            },
            child: const Text('native Owner確認へ進む'),
          ),
        ],
      );
}

class _AgentTaskInstructionDialog extends StatefulWidget {
  const _AgentTaskInstructionDialog();

  @override
  State<_AgentTaskInstructionDialog> createState() =>
      _AgentTaskInstructionDialogState();
}

class _AgentTaskInstructionDialogState
    extends State<_AgentTaskInstructionDialog> {
  final _formKey = GlobalKey<FormState>();
  final _instruction = TextEditingController();

  @override
  void dispose() {
    _instruction.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
        title: const Text('Agent Task能力の事前検査'),
        content: SizedBox(
          width: 560,
          child: Form(
            key: _formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Text(
                  '本文はBrokerへ送信されます。API key、password、secretは入力しないでください。'
                  'この操作はCapabilityの事前検査だけで、Permission／Approval／Taskを発行・実行しません。',
                ),
                const SizedBox(height: 12),
                TextFormField(
                  controller: _instruction,
                  minLines: 3,
                  maxLines: 8,
                  maxLength: 32768,
                  decoration: const InputDecoration(labelText: 'Task指示'),
                  validator: (value) => value == null ||
                          value.trim().isEmpty ||
                          value.runes.length > 32768
                      ? '1〜32768文字で入力してください'
                      : null,
                ),
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('キャンセル'),
          ),
          FilledButton(
            onPressed: () {
              if (!_formKey.currentState!.validate()) return;
              Navigator.of(context).pop(_instruction.text);
            },
            child: const Text('Broker事前検査'),
          ),
        ],
      );
}

class _AgentAdapterPanel extends StatelessWidget {
  const _AgentAdapterPanel({required this.adapter});

  final AgentAdapterRecord adapter;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('${adapter.agentId} (${adapter.adapterId})'),
          SectionList(
            title: 'プロバイダー / モデル',
            rows: ['${adapter.provider} / ${adapter.model}'],
          ),
          SectionList(title: '状態', rows: [adapter.status]),
          SectionList(title: 'バージョン', rows: [adapter.version]),
          SectionList(title: '証拠種別', rows: [adapter.evidenceSource]),
          SectionList(title: '理由', rows: [adapter.evidenceReason]),
          SectionList(
            title: '能力',
            rows: adapter.capabilities.isEmpty
                ? ['unknown']
                : [
                    for (final capability in adapter.capabilities)
                      '${capability.capabilityId}: ${capability.status}',
                  ],
          ),
          SectionList(
            title: '境界',
            rows: [
              '作業領域: ${adapter.workspaceBoundary}',
              'プロセス起動: ${adapter.processSpawnStatus}',
              '認証方式: ${adapter.authenticationMethod}',
            ],
          ),
        ],
      ),
    );
  }
}
