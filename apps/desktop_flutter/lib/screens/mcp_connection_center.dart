import 'package:flutter/material.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

import '../services/mcp_connection_client.dart';
import 'shared.dart';

class McpConnectionCenterPanel extends StatefulWidget {
  const McpConnectionCenterPanel({super.key, required this.transport});

  final BrokerTransport? transport;

  @override
  State<McpConnectionCenterPanel> createState() =>
      _McpConnectionCenterPanelState();
}

class _McpConnectionCenterPanelState extends State<McpConnectionCenterPanel> {
  final _serverIdController = TextEditingController();
  final _executableController = TextEditingController();
  final _workspaceController = TextEditingController();
  final _argumentsController = TextEditingController();
  late final McpConnectionClient? _client;
  List<McpConnectionSummary>? _connections;
  bool _loading = false;
  bool _connecting = false;
  String? _disconnectingServerId;
  String? _message;

  @override
  void initState() {
    super.initState();
    final transport = widget.transport;
    _client = transport == null ? null : McpConnectionClient(transport);
  }

  @override
  void dispose() {
    _serverIdController.dispose();
    _executableController.dispose();
    _workspaceController.dispose();
    _argumentsController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final client = _client;
    if (client == null) {
      return const BorderedPanel(
        child: Text('MCP接続センター: Broker接続がないため一覧を取得できません。'),
      );
    }
    return BorderedPanel(
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('MCP接続センター', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 4),
            const Text(
              'Brokerが保持するstdio接続のmetadataだけを表示します。MCP metadataは信頼・権限ではありません。切断はWindows native Owner確認の後にBrokerが実行します。',
            ),
            const SizedBox(height: 12),
            const Text(
              '新しい接続はこのPC上のMCP stdioプロセスを起動します。資格情報の設定とTool実行は未対応です。起動引数へ秘密値を入れず、Windows native確認の前に入力内容を確認してください。',
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _serverIdController,
              enabled: !_connecting && _disconnectingServerId == null,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: 'サーバー識別子',
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _executableController,
              enabled: !_connecting && _disconnectingServerId == null,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: '実行ファイルの絶対パス',
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _workspaceController,
              enabled: !_connecting && _disconnectingServerId == null,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: '作業フォルダーの絶対パス',
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _argumentsController,
              enabled: !_connecting && _disconnectingServerId == null,
              minLines: 2,
              maxLines: 6,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: '起動引数（1行に1項目、最大32項目）',
              ),
            ),
            const SizedBox(height: 8),
            OutlinedButton.icon(
              onPressed:
                  _connecting || _loading || _disconnectingServerId != null
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
              onPressed:
                  _loading || _connecting || _disconnectingServerId != null
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
                              _disconnectingServerId != null
                          ? null
                          : () => _disconnect(client, connection),
                      icon: const Icon(Icons.link_off),
                      label: const Text('切断'),
                    ),
            ),
            ExpansionTile(
              title: Text('Tool一覧：${connection.tools.length}件'),
              subtitle: const Text('metadata_only。実行権や信頼を示しません。'),
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
                    trailing: const Text('危険度: 未評価'),
                  ),
                const Padding(
                  padding: EdgeInsets.fromLTRB(16, 0, 16, 12),
                  child: Text('説明文と入力Schema本文は表示しません。Tool実行は未対応です。'),
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
        setState(() => _message =
            'MCP接続は確定していません。Windows native Owner確認とBroker状態を確認してください。');
      }
    } finally {
      if (mounted) setState(() => _connecting = false);
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
