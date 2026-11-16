import 'package:broker_contract/broker_transport.dart' show BrokerTransport;
import 'package:flutter/material.dart';

import '../services/mcp_connection_client.dart';
import 'shared.dart';

class ProviderCredentialMetadataPanel extends StatefulWidget {
  const ProviderCredentialMetadataPanel({
    super.key,
    required this.transport,
    required this.active,
  });

  final BrokerTransport? transport;
  final bool active;

  @override
  State<ProviderCredentialMetadataPanel> createState() =>
      _ProviderCredentialMetadataPanelState();
}

class _ProviderCredentialMetadataPanelState
    extends State<ProviderCredentialMetadataPanel> {
  Future<List<McpCredentialSummary>>? _credentials;

  @override
  void initState() {
    super.initState();
    _credentials = _requestForCurrentState();
  }

  @override
  void didUpdateWidget(covariant ProviderCredentialMetadataPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.transport != widget.transport ||
        oldWidget.active != widget.active) {
      _credentials = _requestForCurrentState();
    }
  }

  Future<List<McpCredentialSummary>>? _requestForCurrentState() {
    final transport = widget.transport;
    if (!widget.active || transport == null) return null;
    return McpConnectionClient(transport).listProviderCredentials();
  }

  @override
  Widget build(BuildContext context) {
    final transportAvailable = widget.transport != null;
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Provider資格情報', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          const Text(
            'Brokerが公開する有効Credential metadataだけを表示します。秘密値は取得せず、外部Providerでの認証成功も確認しません。native入力による登録はAgent CenterのProvider欄から行います。',
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            key: const ValueKey('provider-credential-metadata-refresh'),
            onPressed: widget.active && transportAvailable
                ? () => setState(() => _credentials = _requestForCurrentState())
                : null,
            icon: const Icon(Icons.refresh),
            label: const Text('有効Credential metadataを更新'),
          ),
          if (!transportAvailable)
            const Text('Broker接続がないためCredential状態はunknownです。')
          else if (!widget.active)
            const Text('設定を開くとBrokerからmetadataを取得します。')
          else
            FutureBuilder<List<McpCredentialSummary>>(
              future: _credentials,
              builder: (context, snapshot) {
                if (snapshot.connectionState == ConnectionState.waiting) {
                  return const ListTile(
                    contentPadding: EdgeInsets.zero,
                    leading: SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                    title: Text('BrokerからCredential metadataを取得中'),
                  );
                }
                if (snapshot.hasError) {
                  return const Text(
                    'Credential metadataを確認できません。秘密値や認証状態を推測しません。',
                  );
                }
                final credentials = snapshot.data;
                if (credentials == null) {
                  return const Text('Credential metadataは未取得です。');
                }
                if (credentials.isEmpty) {
                  return const Text(
                    '有効なProvider Credential metadataは0件です。失効済み記録や外部認証状態はこの一覧から判定しません。',
                  );
                }
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('有効metadata: ${credentials.length}件'),
                    for (final credential in credentials.take(24))
                      ListTile(
                        key: ValueKey(
                            'provider-credential-${credential.credentialId}'),
                        contentPadding: EdgeInsets.zero,
                        title: Text(
                            '${credential.kind} ・ ${credential.credentialId}'),
                        subtitle: Text(
                          '保管方式=${credential.storage} / 対象=${credential.target} / metadata状態=${credential.status}',
                        ),
                      ),
                    if (credentials.length > 24)
                      Text('残り${credentials.length - 24}件は省略'),
                  ],
                );
              },
            ),
        ],
      ),
    );
  }
}
