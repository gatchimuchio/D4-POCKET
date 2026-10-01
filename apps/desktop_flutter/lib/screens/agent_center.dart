import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
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

class _AgentCenterState extends State<AgentCenter> {
  bool _registrationPending = false;
  String? _registrationStatus;
  _CodexRegistrationInput? _registered;

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
        setState(() {
          _registered = input;
          _registrationStatus = 'Broker起動中だけ登録しました。Task実行はunsupportedのままです。';
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

  @override
  Widget build(BuildContext context) {
    final client = widget.client;
    final snapshot = client.getSnapshot();
    final adapters = snapshot.agentAdapters;
    final sessions = client.mode == 'broker'
        ? snapshot.agentSessions
        : const <AgentSessionRecord>[];
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
                      'Task実行: unsupported',
                      'Permission／Approval／Credential: 生成なし',
                    ],
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
                  'Task状態・結果・diff表示の製品UI経路は未接続です。BrokerのTask契約が存在しても、Codex task_execution=unsupportedを解除しません。',
                ),
                const SectionList(
                  title: '未接続の実行情報',
                  rows: [
                    '作業領域: 実行directoryは未確認',
                    'タスク: Brokerから未取得',
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
  });

  final String runtimeId;
  final String codexCliPath;
  final String workspaceId;
  final String workspaceRoot;
  final List<String> secretPaths;

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
