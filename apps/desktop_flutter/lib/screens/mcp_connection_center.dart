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
  late final McpConnectionClient? _client;
  List<McpConnectionSummary>? _connections;
  bool _loading = false;
  String? _disconnectingServerId;
  String? _message;

  @override
  void initState() {
    super.initState();
    final transport = widget.transport;
    _client = transport == null ? null : McpConnectionClient(transport);
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
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('MCP接続センター', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          const Text(
            'Brokerが保持するstdio接続のmetadataだけを表示します。MCP metadataは信頼・権限ではありません。切断はWindows native Owner確認の後にBrokerが実行します。',
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            onPressed: _loading || _disconnectingServerId != null
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
        child: ListTile(
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
                  onPressed: _loading || _disconnectingServerId != null
                      ? null
                      : () => _disconnect(client, connection),
                  icon: const Icon(Icons.link_off),
                  label: const Text('切断'),
                ),
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
