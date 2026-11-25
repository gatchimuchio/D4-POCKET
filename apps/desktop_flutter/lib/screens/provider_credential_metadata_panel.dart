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
  String? _mutatingCredentialId;
  String? _message;
  int _generation = 0;

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
      _generation++;
      _mutatingCredentialId = null;
      _message = null;
      _credentials = _requestForCurrentState();
    }
  }

  Future<List<McpCredentialSummary>>? _requestForCurrentState() {
    final transport = widget.transport;
    if (!widget.active || transport == null) return null;
    return McpConnectionClient(transport).listProviderCredentialMetadata();
  }

  void _refresh() {
    setState(() {
      _message = null;
      _credentials = _requestForCurrentState();
    });
  }

  Future<void> _mutateCredential(
    McpCredentialSummary credential, {
    required bool delete,
  }) async {
    final transport = widget.transport;
    if (transport == null || !widget.active || _mutatingCredentialId != null) {
      return;
    }
    final generation = _generation;
    final client = McpConnectionClient(transport);
    setState(() {
      _mutatingCredentialId = credential.credentialId;
      _message = null;
    });
    try {
      if (delete) {
        await client.deleteCredential(credential: credential);
      } else {
        await client.revokeCredential(credential: credential);
      }
      final current = await client.listProviderCredentialMetadata();
      if (mounted && generation == _generation) {
        setState(() {
          _credentials = Future.value(current);
          _message = delete
              ? '失効済みCredentialをNative Vaultから削除し、Audit metadataを保持しました。媒体上のbit消去は保証しません。'
              : 'Credentialを失効しました。以後のBroker利用を拒否し、Vault dataは削除操作まで保持します。';
        });
      }
    } on Object {
      if (mounted && generation == _generation) {
        setState(
          () => _message =
              'Credential変更を確定できません。metadataを再取得し、削除中なら新しいnative Owner確認で同じ削除を再開してください。',
        );
        try {
          final current = await client.listProviderCredentialMetadata();
          if (mounted && generation == _generation) {
            setState(() => _credentials = Future.value(current));
          }
        } on Object {
          // The explicit message above preserves the unknown state.
        }
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _mutatingCredentialId = null);
      }
    }
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
            'Brokerが公開するProvider Credential metadataを表示します。秘密値は取得せず、外部Providerの認証状態も確認しません。native入力による登録はAgent Centerから行い、失効とVault削除は別々のnative Owner確認で行います。',
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            key: const ValueKey('provider-credential-metadata-refresh'),
            onPressed:
                widget.active &&
                    transportAvailable &&
                    _mutatingCredentialId == null
                ? _refresh
                : null,
            icon: const Icon(Icons.refresh),
            label: const Text('Credential metadataを更新'),
          ),
          if (_message != null) ...[const SizedBox(height: 6), Text(_message!)],
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
                    'Provider Credential metadataはありません。外部認証状態はこの一覧から判定しません。',
                  );
                }
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Broker記録: ${credentials.length}件'),
                    for (final credential in credentials.take(24))
                      ListTile(
                        key: ValueKey(
                          'provider-credential-${credential.credentialId}',
                        ),
                        contentPadding: EdgeInsets.zero,
                        title: Text(
                          '${credential.kind} ・ ${credential.credentialId}',
                        ),
                        subtitle: Text(
                          '保管方式=${credential.storage} / 対象=${credential.target} / metadata状態=${credential.status}',
                        ),
                        trailing: credential.status == '有効'
                            ? TextButton.icon(
                                key: ValueKey(
                                  'provider-credential-revoke-${credential.credentialId}',
                                ),
                                onPressed: _mutatingCredentialId == null
                                    ? () => _mutateCredential(
                                        credential,
                                        delete: false,
                                      )
                                    : null,
                                icon:
                                    _mutatingCredentialId ==
                                        credential.credentialId
                                    ? const SizedBox(
                                        width: 18,
                                        height: 18,
                                        child: CircularProgressIndicator(
                                          strokeWidth: 2,
                                        ),
                                      )
                                    : const Icon(Icons.key_off_outlined),
                                label: const Text('失効'),
                              )
                            : credential.status == '失効' ||
                                  credential.status == '削除中'
                            ? TextButton.icon(
                                key: ValueKey(
                                  'provider-credential-delete-${credential.credentialId}',
                                ),
                                onPressed: _mutatingCredentialId == null
                                    ? () => _mutateCredential(
                                        credential,
                                        delete: true,
                                      )
                                    : null,
                                icon:
                                    _mutatingCredentialId ==
                                        credential.credentialId
                                    ? const SizedBox(
                                        width: 18,
                                        height: 18,
                                        child: CircularProgressIndicator(
                                          strokeWidth: 2,
                                        ),
                                      )
                                    : const Icon(Icons.delete_outline),
                                label: Text(
                                  credential.status == '削除中'
                                      ? '削除再開'
                                      : 'Vault削除',
                                ),
                              )
                            : const Text('削除済み'),
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
