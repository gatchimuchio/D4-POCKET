import 'package:flutter/material.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

import '../services/a2a_connection_client.dart';
import 'shared.dart';

class A2aConnectionCenter extends StatefulWidget {
  const A2aConnectionCenter({super.key, required this.transport});

  final BrokerTransport? transport;

  @override
  State<A2aConnectionCenter> createState() => _A2aConnectionCenterState();
}

class _A2aConnectionCenterState extends State<A2aConnectionCenter> {
  final _agentIdController = TextEditingController();
  final _agentCardUriController = TextEditingController();
  late final A2aConnectionClient? _client;
  List<A2aConnectionSummary> _connections = const [];
  bool _loading = false;
  bool _connecting = false;
  String? _message;

  @override
  void initState() {
    super.initState();
    final transport = widget.transport;
    _client = transport == null ? null : A2aConnectionClient(transport);
    if (_client != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _loadConnections();
      });
    }
  }

  @override
  void dispose() {
    _agentIdController.dispose();
    _agentCardUriController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final client = _client;
    return ShellPage(
      title: 'A2A接続センター',
      evidenceTitle: 'A2A Connection Center',
      children: [
        const BorderedPanel(
          child: Text(
            'Agent Cardは未信頼の外部metadataとして扱います。接続先はloopback IPv4のHTTPに限り、'
            'Rust Broker経由で接続ごとにnative Owner確認を行います。Credential値は扱わず、'
            '接続だけでTrust・Permission・Approval・Authorityは生成しません。'
            'Task、Message、Artifact、Streamの実行・取得はこの画面では行いません。',
          ),
        ),
        if (client == null)
          const BorderedPanel(
            child: Text('Broker接続がないためA2A接続と一覧取得は利用できません。'),
          )
        else ...[
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('loopback Agent Card登録',
                    style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                TextField(
                  controller: _agentIdController,
                  enabled: !_loading && !_connecting,
                  maxLength: 128,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    labelText: 'Agent識別子（Agent ID）',
                  ),
                ),
                const SizedBox(height: 8),
                TextField(
                  controller: _agentCardUriController,
                  enabled: !_loading && !_connecting,
                  maxLength: 2048,
                  autocorrect: false,
                  enableSuggestions: false,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    labelText: 'Agent Card接続先URI（http://127.x.x.x:port/...）',
                    helperText: 'HTTPS、外部host、query、fragment、userinfoは受け付けません。',
                  ),
                ),
                const SizedBox(height: 8),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    FilledButton.icon(
                      onPressed: _loading || _connecting ? null : _connect,
                      icon: const Icon(Icons.link),
                      label: Text(_connecting ? '接続中…' : 'Owner確認して接続'),
                    ),
                    OutlinedButton.icon(
                      onPressed:
                          _loading || _connecting ? null : _loadConnections,
                      icon: const Icon(Icons.refresh),
                      label: const Text('一覧を更新'),
                    ),
                  ],
                ),
                if (_message != null) ...[
                  const SizedBox(height: 12),
                  Semantics(liveRegion: true, child: Text(_message!)),
                ],
              ],
            ),
          ),
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text('接続metadata',
                          style: Theme.of(context).textTheme.titleMedium),
                    ),
                    if (_loading)
                      const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                  ],
                ),
                const SizedBox(height: 8),
                if (_connections.isEmpty && !_loading)
                  const Text('Broker一覧にA2A接続metadataはありません。')
                else
                  for (final connection in _connections)
                    _A2aConnectionTile(connection: connection),
              ],
            ),
          ),
        ],
      ],
    );
  }

  Future<void> _loadConnections() async {
    final client = _client;
    if (client == null || _loading || _connecting) return;
    setState(() {
      _loading = true;
      _message = null;
    });
    try {
      final connections = await client.list();
      if (!mounted) return;
      setState(() {
        _connections = connections;
        _message = 'Brokerのmetadata-only一覧を更新しました。';
      });
    } on Object {
      if (!mounted) return;
      setState(() {
        _message = 'Broker一覧を取得できませんでした。接続状態を成功扱いにはしていません。';
      });
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _connect() async {
    final client = _client;
    if (client == null || _loading || _connecting) return;
    setState(() {
      _connecting = true;
      _message = '要求をBrokerへ送信しています。native Owner確認後に接続します。';
    });
    try {
      final receipt = await client.connect(
        agentId: _agentIdController.text,
        agentCardUri: _agentCardUriController.text,
      );
      if (!mounted) return;
      _agentCardUriController.clear();
      setState(() {
        _message =
            '接続metadataを受理しました。Trustは未審査のままです。Audit=${receipt.connectionAuditId}';
      });
      try {
        final connections = await client.list();
        if (!mounted) return;
        setState(() => _connections = connections);
      } on Object {
        if (!mounted) return;
        setState(() {
          _message =
              '接続receiptは受理されましたが一覧を更新できません。Audit=${receipt.connectionAuditId}';
        });
      }
    } on Object {
      if (!mounted) return;
      setState(() {
        _message = 'A2A接続を確認できませんでした。受理receiptがないため成功表示はしていません。';
      });
    } finally {
      if (mounted) setState(() => _connecting = false);
    }
  }
}

class _A2aConnectionTile extends StatelessWidget {
  const _A2aConnectionTile({required this.connection});

  final A2aConnectionSummary connection;

  @override
  Widget build(BuildContext context) {
    final restored = connection.connectionState == 'restored_pending_review';
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('表示名（未信頼metadata）: ${connection.displayName}',
                style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Text('Agent識別子: ${connection.agentId}'),
            Text('説明（未信頼metadata）: ${connection.descriptionSummary}'),
            Text('版: ${connection.version}'),
            Text('接続先hash: ${connection.endpointHash}'),
            Text('接続状態: ${restored ? '再確認が必要' : '接続済み'}'),
            Text(
                '証拠: ${connection.evidenceSource} / 承認: ${connection.approvalState}'),
            Text('信頼状態: 未審査（pending_review）— ${connection.trustReason}'),
            Text('接続Audit: ${connection.connectionAuditId}'),
          ],
        ),
      ),
    );
  }
}
