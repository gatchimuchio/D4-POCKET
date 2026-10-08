import 'dart:convert';
import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

import '../services/mcp_connection_client.dart';
import 'shared.dart';

class McpConnectionCenterPanel extends StatefulWidget {
  const McpConnectionCenterPanel({
    super.key,
    required this.transport,
    this.focusRequest = 0,
  });

  final BrokerTransport? transport;
  final int focusRequest;

  @override
  State<McpConnectionCenterPanel> createState() =>
      _McpConnectionCenterPanelState();
}

class _McpConnectionCenterPanelState extends State<McpConnectionCenterPanel> {
  final GlobalKey _titleKey = GlobalKey();
  final _serverIdController = TextEditingController();
  final _executableController = TextEditingController();
  final _workspaceController = TextEditingController();
  final _argumentsController = TextEditingController();
  final _credentialEnvironmentController = TextEditingController();
  late final McpConnectionClient? _client;
  List<McpConnectionSummary>? _connections;
  List<McpCredentialSummary>? _credentials;
  String? _credentialTargetServerId;
  String? _selectedCredentialId;
  String? _revokingCredentialId;
  bool _loading = false;
  bool _connecting = false;
  String? _disconnectingServerId;
  String? _callingToolServerId;
  String? _message;

  @override
  void initState() {
    super.initState();
    final transport = widget.transport;
    _client = transport == null ? null : McpConnectionClient(transport);
  }

  @override
  void didUpdateWidget(covariant McpConnectionCenterPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.focusRequest != widget.focusRequest) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final targetContext = _titleKey.currentContext;
        if (!mounted || targetContext == null) return;
        Scrollable.ensureVisible(
          targetContext,
          alignment: 0.12,
          duration: const Duration(milliseconds: 220),
          curve: Curves.easeOutCubic,
        );
      });
    }
  }

  @override
  void dispose() {
    _serverIdController.dispose();
    _executableController.dispose();
    _workspaceController.dispose();
    _argumentsController.dispose();
    _credentialEnvironmentController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final client = _client;
    final macOS = defaultTargetPlatform == TargetPlatform.macOS;
    if (client == null) {
      return BorderedPanel(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'MCP接続センター',
              key: _titleKey,
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 4),
            const Text('Broker接続がないため一覧を取得できません。'),
          ],
        ),
      );
    }
    return BorderedPanel(
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'MCP接続センター',
              key: _titleKey,
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 4),
            Text(
              'Brokerが保持するstdio接続のmetadataだけを表示します。MCP metadataは信頼・権限ではありません。切断は${macOS ? 'Mac' : 'Windows'} native Owner確認の後にBrokerが実行します。',
            ),
            const SizedBox(height: 12),
            Text(
              'Tool実行は${macOS ? 'Mac' : 'Windows'} native Owner確認を毎回要求し、結果本文ではなくhash receiptだけを返します。実行前にJSON argumentsを確認してください。秘密値をargumentsへ入力しないでください。Agentへの結果引渡しとResource／Prompt本文取得は未対応です。',
            ),
            const SizedBox(height: 8),
            Text(
              macOS
                  ? 'Mac MCPは資格情報を注入しません。App Sandboxを継承し、明示切断はD4所有process groupだけを停止します。別groupへ離脱したprocessや外部副作用の復旧は保証しません。'
                  : 'MCP CredentialはFlutterへ入力しません。登録済みmetadataから対象Server専用のものを選びます。選択すると値を対象Server processへ渡し、そのprocessは読み取り・外部送信できます。Windows Job Objectはsandboxではありません。',
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _serverIdController,
              onChanged: (_) => setState(() {}),
              enabled: !_connecting &&
                  !_loading &&
                  _disconnectingServerId == null &&
                  _callingToolServerId == null,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: 'サーバー識別子',
              ),
            ),
            const SizedBox(height: 8),
            if (defaultTargetPlatform == TargetPlatform.macOS) ...[
              OutlinedButton.icon(
                key: const ValueKey('macos-native-credential-register'),
                onPressed:
                    _loading || _connecting || _revokingCredentialId != null
                        ? null
                        : () => _registerMacCredential(client),
                icon: const Icon(Icons.lock_outline),
                label: const Text('native入力で資格情報を登録'),
              ),
              const Text(
                  'Mac Keychainへ登録・一覧・論理失効だけを行います。秘密値はFlutterへ渡しません。MCPへの注入は未対応です。'),
              const SizedBox(height: 8),
            ],
            TextField(
              controller: _executableController,
              enabled: !_connecting &&
                  _disconnectingServerId == null &&
                  _callingToolServerId == null,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: '実行ファイルの絶対パス',
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _workspaceController,
              enabled: !_connecting &&
                  _disconnectingServerId == null &&
                  _callingToolServerId == null,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: '作業フォルダーの絶対パス',
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _argumentsController,
              enabled: !_connecting &&
                  _disconnectingServerId == null &&
                  _callingToolServerId == null,
              minLines: 2,
              maxLines: 6,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: '起動引数（1行に1項目、最大32項目）',
              ),
            ),
            const SizedBox(height: 8),
            OutlinedButton.icon(
              key: const ValueKey('mcp-credential-list'),
              onPressed: _connecting ||
                      _loading ||
                      _disconnectingServerId != null ||
                      _callingToolServerId != null
                  ? null
                  : () => _loadCredentials(client),
              icon: const Icon(Icons.key_outlined),
              label: const Text('対象ServerのCredential metadata一覧を取得'),
            ),
            if (_credentials != null &&
                _credentialTargetServerId == _serverIdController.text) ...[
              const SizedBox(height: 8),
              DropdownButtonFormField<String>(
                key: const ValueKey('mcp-credential-selection'),
                initialValue: _credentials!.any(
                        (entry) => entry.credentialId == _selectedCredentialId)
                    ? _selectedCredentialId
                    : null,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  labelText: '対象ServerのCredential（任意）',
                ),
                items: [
                  const DropdownMenuItem<String>(
                    value: null,
                    child: Text('使用しない'),
                  ),
                  for (final credential
                      in _credentials!.where((entry) => entry.status == '有効'))
                    DropdownMenuItem<String>(
                      value: credential.credentialId,
                      child: Text(
                          '${credential.kind} ・ ${credential.credentialId}'),
                    ),
                ],
                onChanged: macOS || _connecting || _loading
                    ? null
                    : (value) => setState(() {
                          _selectedCredentialId = value;
                          if (value == null) {
                            _credentialEnvironmentController.clear();
                          }
                        }),
              ),
            ],
            if (_credentials != null &&
                _credentialTargetServerId == _serverIdController.text) ...[
              const SizedBox(height: 8),
              const Text('このServer向けCredential metadata'),
              for (final credential in _credentials!)
                ListTile(
                  key: ValueKey('mcp-credential-${credential.credentialId}'),
                  title: Text('${credential.kind} ・ ${credential.status}'),
                  subtitle: Text(
                    'ID: ${credential.credentialId}'
                    '${credential.revokedAt == null ? '' : '\n失効時刻: ${credential.revokedAt}'}',
                  ),
                  trailing: credential.status == '有効'
                      ? TextButton.icon(
                          key: ValueKey(
                              'mcp-credential-revoke-${credential.credentialId}'),
                          label: const Text('資格情報を失効'),
                          onPressed: _loading ||
                                  _connecting ||
                                  _revokingCredentialId != null ||
                                  _disconnectingServerId != null ||
                                  _callingToolServerId != null
                              ? null
                              : () => _revokeCredential(client, credential),
                          icon: _revokingCredentialId == credential.credentialId
                              ? const SizedBox(
                                  width: 18,
                                  height: 18,
                                  child:
                                      CircularProgressIndicator(strokeWidth: 2),
                                )
                              : const Icon(Icons.key_off_outlined),
                        )
                      : const Text('失効済み'),
                ),
            ],
            const SizedBox(height: 8),
            TextField(
              key: const ValueKey('mcp-credential-environment-variable'),
              controller: _credentialEnvironmentController,
              enabled: _selectedCredentialId != null &&
                  _credentialTargetServerId == _serverIdController.text &&
                  !_connecting,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: '子processへ渡す環境変数名',
                helperText: '例: MCP_API_KEY。秘密値ではなく名前だけを指定します。',
              ),
            ),
            const SizedBox(height: 8),
            OutlinedButton.icon(
              onPressed: _connecting ||
                      _loading ||
                      _disconnectingServerId != null ||
                      _callingToolServerId != null
                  ? null
                  : () => _connect(client),
              icon: const Icon(Icons.link),
              label: const Text('MCP接続を開始'),
            ),
            if (_connecting) ...[
              const SizedBox(height: 8),
              const LinearProgressIndicator(),
            ],
            const SizedBox(height: 8),
            OutlinedButton.icon(
              key: const ValueKey('mcp-load-connections'),
              onPressed: _loading ||
                      _connecting ||
                      _disconnectingServerId != null ||
                      _callingToolServerId != null
                  ? null
                  : () => _load(client),
              icon: const Icon(Icons.refresh),
              label: const Text('接続一覧を取得'),
            ),
            if (_loading) ...[
              const SizedBox(height: 8),
              const LinearProgressIndicator(),
            ],
            if (_message != null) ...[
              const SizedBox(height: 8),
              Text(_message!),
            ],
            if (_connections != null && _connections!.isEmpty) ...[
              const SizedBox(height: 8),
              const Text('Brokerが保持するMCP接続はありません。'),
            ],
            for (final connection in _connections ?? const [])
              _connectionTile(client, connection),
          ],
        ),
      ),
    );
  }

  Widget _connectionTile(
    McpConnectionClient client,
    McpConnectionSummary connection,
  ) {
    final disconnecting = _disconnectingServerId == connection.serverId;
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: DecoratedBox(
        decoration: BoxDecoration(
          border:
              Border.all(color: Theme.of(context).colorScheme.outlineVariant),
          borderRadius: BorderRadius.circular(8),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ListTile(
              title: Text(
                connection.displayName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              subtitle: Text(
                'サーバーID: ${connection.serverId}\n'
                '通信方式: ${connection.transport} ・ '
                '実行状態: ${connection.executionState} ・ '
                'ツール ${connection.toolCount} / リソース ${connection.resourceCount} / プロンプト ${connection.promptCount}',
              ),
              isThreeLine: true,
              trailing: disconnecting
                  ? const SizedBox(
                      width: 24,
                      height: 24,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : OutlinedButton.icon(
                      onPressed: _loading ||
                              _connecting ||
                              _callingToolServerId != null ||
                              _disconnectingServerId != null
                          ? null
                          : () => _disconnect(client, connection),
                      icon: const Icon(Icons.link_off),
                      label: const Text('切断'),
                    ),
            ),
            ExpansionTile(
              title: Text('Tool一覧：${connection.tools.length}件'),
              subtitle: const Text('metadata_only。毎回のOwner確認後のみ実行。結果本文は返しません。'),
              children: [
                if (connection.tools.isEmpty)
                  const ListTile(title: Text('Toolはありません。')),
                for (final tool in connection.tools)
                  ListTile(
                    title: Text(
                      tool.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    subtitle: Text(
                      '識別子: ${tool.toolId}\n'
                      '入力仕様hash: ${tool.inputSchemaHash}',
                    ),
                    trailing: OutlinedButton(
                      onPressed: connection.executionState != 'ready' ||
                              _loading ||
                              _connecting ||
                              _disconnectingServerId != null ||
                              _callingToolServerId != null
                          ? null
                          : () => _callTool(client, connection, tool),
                      child: Text(_callingToolServerId == connection.serverId
                          ? '実行中…'
                          : '確認して実行'),
                    ),
                  ),
                const Padding(
                  padding: EdgeInsets.fromLTRB(16, 0, 16, 12),
                  child:
                      Text('説明文と入力Schema本文は表示しません。結果本文も画面へ返さず、hashと型だけを記録します。'),
                ),
              ],
            ),
            ExpansionTile(
              title: Text('Resource一覧：${connection.resources.length}件'),
              subtitle: const Text('URIと本文は取得しません。'),
              children: [
                if (connection.resources.isEmpty)
                  const ListTile(title: Text('Resourceはありません。')),
                for (final resource in connection.resources)
                  ListTile(
                    title: Text(
                      resource.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    subtitle: Text(
                      '識別子: ${resource.resourceId}\n'
                      'URIのhash: ${resource.uriTemplateHash}',
                    ),
                  ),
              ],
            ),
            ExpansionTile(
              title: Text('Prompt一覧：${connection.prompts.length}件'),
              subtitle: const Text('説明文・引数・本文は取得しません。'),
              children: [
                if (connection.prompts.isEmpty)
                  const ListTile(title: Text('Promptはありません。')),
                for (final prompt in connection.prompts)
                  ListTile(
                    title: Text(
                      prompt.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    subtitle: Text(
                      '識別子: ${prompt.promptId}\n'
                      '引数仕様hash: ${prompt.argumentSchemaHash}',
                    ),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _load(McpConnectionClient client) async {
    setState(() {
      _loading = true;
      _message = null;
    });
    try {
      final connections = await client.list();
      if (mounted) setState(() => _connections = connections);
    } on Object {
      if (mounted) {
        setState(() {
          _connections = null;
          _message = 'MCP接続一覧を取得できません。Broker状態とAuditを確認してください。';
        });
      }
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _registerMacCredential(McpConnectionClient client) async {
    final target = _serverIdController.text;
    final random = Random.secure();
    // 非秘密の新規record識別子。保存先・権限・Owner資格ではない。
    final id = List.generate(
            16, (_) => random.nextInt(256).toRadixString(16).padLeft(2, '0'))
        .join();
    setState(() {
      _loading = true;
      _message = 'native秘密入力と別個Owner確認を待っています。';
    });
    try {
      await client.registerMacNativeCredential(
          credentialId: id, targetServerId: target);
      final entries = await client.listCredentials(targetServerId: target);
      if (mounted)
        setState(() {
          _credentials = entries;
          _credentialTargetServerId = target;
          _selectedCredentialId = null;
          _message = 'Keychain登録後のmetadataを取得しました。秘密値は取得していません。';
        });
    } on Object {
      if (mounted)
        setState(() {
          _message = '資格情報登録は未成立です。取消・拒否・期限または保管状態を確認してください。自動再送しません。';
        });
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _loadCredentials(McpConnectionClient client) async {
    final targetServerId = _serverIdController.text;
    setState(() {
      _loading = true;
      _credentials = null;
      _credentialTargetServerId = null;
      _selectedCredentialId = null;
      _credentialEnvironmentController.clear();
      _message = null;
    });
    try {
      final credentials = await client.listCredentials(
        targetServerId: targetServerId,
      );
      if (mounted) {
        setState(() {
          _credentials = credentials;
          _credentialTargetServerId = targetServerId;
          _message = credentials.isEmpty
              ? 'このServer向けに利用可能なCredential metadataはありません。値は読み出していません。'
              : '${credentials.length}件の対象Credential metadataを取得しました。秘密値は取得していません。';
        });
      }
    } on Object {
      if (mounted) {
        setState(() {
          _credentials = null;
          _credentialTargetServerId = null;
          _message = 'Credential metadataを取得できません。秘密値は要求していません。';
        });
      }
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _connect(McpConnectionClient client) async {
    setState(() {
      _connecting = true;
      _message = null;
    });
    try {
      final connection = await client.connect(
        serverId: _serverIdController.text,
        executable: _executableController.text,
        workspace: _workspaceController.text,
        argumentsText: _argumentsController.text,
        credentialId: _credentialTargetServerId == _serverIdController.text &&
                _credentials?.any((entry) =>
                        entry.credentialId == _selectedCredentialId &&
                        entry.status == '有効') ==
                    true
            ? _selectedCredentialId
            : null,
        credentialEnvironmentVariable:
            _credentialTargetServerId == _serverIdController.text &&
                    _credentials?.any((entry) =>
                            entry.credentialId == _selectedCredentialId &&
                            entry.status == '有効') ==
                        true
                ? _credentialEnvironmentController.text
                : null,
      );
      if (!mounted) return;
      setState(() {
        _connections = [
          ...?_connections
              ?.where((item) => item.serverId != connection.serverId),
          connection,
        ];
        _message = '接続receiptを受理しました。表示はBroker内部状態であり、TrustやTool実行を示しません。';
      });
    } on Object {
      if (mounted) {
        setState(() =>
            _message = 'MCP接続は確定していません。native Owner確認とBroker状態を確認してください。');
      }
    } finally {
      if (mounted) setState(() => _connecting = false);
    }
  }

  Future<void> _revokeCredential(
    McpConnectionClient client,
    McpCredentialSummary credential,
  ) async {
    setState(() {
      _revokingCredentialId = credential.credentialId;
      _message = null;
    });
    try {
      await client.revokeCredential(credential: credential);
      if (!mounted) return;
      await _loadCredentials(client);
      if (mounted) {
        setState(() => _message =
            'Credentialを論理失効しました。以後のMCP注入を拒否し、暗号文fileは別の物理削除操作まで保持します。');
      }
    } on Object {
      if (mounted) {
        setState(() => _message =
            'Credential失効は確定していません。native Owner確認とBroker Audit状態を確認してください。');
      }
    } finally {
      if (mounted) setState(() => _revokingCredentialId = null);
    }
  }

  Future<void> _callTool(
    McpConnectionClient client,
    McpConnectionSummary connection,
    McpToolSummary tool,
  ) async {
    final controller = TextEditingController(text: '{}');
    try {
      final raw = await showDialog<String>(
        context: context,
        builder: (dialogContext) => AlertDialog(
          title: Text('${tool.name} のarguments'),
          content: SizedBox(
            width: 560,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('秘密値を入力しないでください。入力は保存されません。'),
                const SizedBox(height: 8),
                TextField(
                  controller: controller,
                  minLines: 5,
                  maxLines: 14,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    labelText: 'JSON形式のobject',
                  ),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialogContext),
              child: const Text('キャンセル'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(dialogContext, controller.text),
              child: const Text('入力内容を確認'),
            ),
          ],
        ),
      );
      if (raw == null || !mounted) return;

      final Object? decoded;
      try {
        decoded = jsonDecode(raw);
      } on FormatException {
        setState(() =>
            _message = 'argumentsは正しいJSON objectで入力してください。Toolは送信していません。');
        return;
      }
      if (decoded is! Map || decoded.keys.any((key) => key is! String)) {
        setState(
            () => _message = 'argumentsはJSON objectで入力してください。Toolは送信していません。');
        return;
      }
      final arguments = Map<String, Object?>.from(decoded);
      if (utf8.encode(jsonEncode(arguments)).length > 32 * 1024) {
        setState(() => _message = 'argumentsが32 KiBを超えています。Toolは送信していません。');
        return;
      }
      final preview = const JsonEncoder.withIndent('  ').convert(arguments);
      final confirmed = await showDialog<bool>(
        context: context,
        builder: (dialogContext) => AlertDialog(
          title: const Text('送信する引数を確認'),
          content: SizedBox(
            width: 560,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('接続先MCP識別子: ${connection.serverId}'),
                Text('Tool名・ID: ${tool.name} (${tool.toolId})'),
                const SizedBox(height: 8),
                const Text('以下の全文を確認してください。次にnative Owner確認が表示されます。'),
                const SizedBox(height: 8),
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 300),
                  child: SingleChildScrollView(
                    child: SelectableText(
                      preview,
                      style: const TextStyle(fontFamily: 'monospace'),
                    ),
                  ),
                ),
                const SizedBox(height: 8),
                const Text(
                    'Tool結果本文は返却されず、hashとcontent型だけを表示します。実行後に自動再送はしません。'),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialogContext, false),
              child: const Text('戻る'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(dialogContext, true),
              child: Text(defaultTargetPlatform == TargetPlatform.macOS
                  ? 'Mac確認へ進む'
                  : 'Windows確認へ進む'),
            ),
          ],
        ),
      );
      if (confirmed != true || !mounted) return;

      setState(() {
        _callingToolServerId = connection.serverId;
        _message = null;
      });
      try {
        final receipt = await client.callTool(
          serverId: connection.serverId,
          tool: tool,
          arguments: arguments,
        );
        if (!mounted) return;
        setState(() {
          final executionState =
              receipt.connectionState == 'connected' ? 'ready' : 'quarantined';
          _connections = _connections
              ?.map((item) => item.serverId == connection.serverId
                  ? McpConnectionSummary(
                      serverId: item.serverId,
                      displayName: item.displayName,
                      transport: item.transport,
                      tools: item.tools,
                      resources: item.resources,
                      prompts: item.prompts,
                      connectionState: receipt.connectionState,
                      executionState: executionState,
                    )
                  : item)
              .toList(growable: false);
          _message =
              'Tool応答receiptを受理しました。本文は返却されていません。result hash: ${receipt.resultHash} ・ content: ${receipt.contentTypes.join(', ')} ・ Audit: ${receipt.auditId}${receipt.toolError ? ' ・ Tool側error=true' : ''}${executionState == 'quarantined' ? ' ・ 接続隔離中' : ''}';
        });
      } on Object {
        if (mounted) {
          setState(() {
            _connections = _connections
                ?.map((item) => item.serverId == connection.serverId
                    ? McpConnectionSummary(
                        serverId: item.serverId,
                        displayName: item.displayName,
                        transport: item.transport,
                        tools: item.tools,
                        resources: item.resources,
                        prompts: item.prompts,
                        connectionState: 'quarantined',
                        executionState: 'quarantined',
                      )
                    : item)
                .toList(growable: false);
            _message =
                'Tool結果を確定できません。自動再送していません。外部副作用とBroker状態を確認し、必要なら切断・再接続してください。';
          });
        }
      } finally {
        if (mounted) setState(() => _callingToolServerId = null);
      }
    } finally {
      controller.dispose();
    }
  }

  Future<void> _disconnect(
    McpConnectionClient client,
    McpConnectionSummary connection,
  ) async {
    setState(() {
      _disconnectingServerId = connection.serverId;
      _message = null;
    });
    try {
      await client.disconnect(connection.serverId);
      if (!mounted) return;
      setState(() {
        _connections = _connections
            ?.where((item) => item.serverId != connection.serverId)
            .toList(growable: false);
        _loading = true;
        _message = '切断receiptを受理しました。Brokerの永続Auditを記録済みです。';
      });
      try {
        final connections = await client.list();
        if (mounted) setState(() => _connections = connections);
      } on Object {
        if (mounted) {
          setState(() => _message =
              '切断receiptは受理され、接続記録は解消されました。一覧を再取得できないため更新して確認してください。');
        }
      } finally {
        if (mounted) setState(() => _loading = false);
      }
    } on Object {
      if (mounted) {
        setState(() => _message = '切断は確定していません。Owner確認とBrokerの現在状態を確認してください。');
      }
    } finally {
      if (mounted) setState(() => _disconnectingServerId = null);
    }
  }
}
