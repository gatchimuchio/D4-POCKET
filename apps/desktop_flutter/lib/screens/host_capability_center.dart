import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class HostCapabilityCenter extends StatelessWidget {
  const HostCapabilityCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final hosts = snapshot.hostCapabilities;
    final brokerObserved = snapshot.snapshotSource == 'broker';
    return ShellPage(
      title: 'D4 Pocket ホスト能力',
      children: [
        BorderedPanel(
          child: Text(
            brokerObserved
                ? 'この能力一覧は、認証済みRust Brokerの現在の実行Host（ローカル）に対する観測です。'
                    '別Hostの能力や接続状態へ流用せず、degraded状態も表示情報に限りPermissionやApprovalは生成しません。'
                : 'Broker由来の受理済みsnapshotがないため、能力値は試験・診断用の表示です。'
                    '実Hostの能力、local／remote区分、degraded状態の証拠として扱わず、PermissionやApprovalは生成しません。',
          ),
        ),
        if (hosts.isEmpty)
          const BorderedPanel(child: Text('現在のホスト能力を取得できません。')),
        for (final host in hosts)
          _HostCapabilityPanel(host: host, brokerObserved: brokerObserved),
      ],
    );
  }
}

class _HostCapabilityPanel extends StatelessWidget {
  const _HostCapabilityPanel({
    required this.host,
    required this.brokerObserved,
  });

  final HostCapabilityRecord host;
  final bool brokerObserved;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(host.displayName,
              style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          Text('ホストID: ${host.hostId}'),
          Text('所在: ${brokerObserved ? '現在のBroker実行Host（ローカル）' : '未確認'}'),
          Text(
            'プラットフォーム: ${host.platform} / '
            '${brokerObserved ? 'degradedを含む観測状態' : '表示用状態（未観測）'}: ${host.status}',
          ),
          const SizedBox(height: 12),
          for (final capability in host.capabilities)
            ListTile(
              dense: true,
              contentPadding: EdgeInsets.zero,
              leading: Icon(
                capability.status == 'ready'
                    ? Icons.check_circle_outline
                    : Icons.info_outline,
              ),
              title: Text(capability.capabilityId),
              subtitle: Text(
                brokerObserved
                    ? '${_hostCapabilityStatusLabel(capability.status)} / '
                        '${_hostCapabilityEvidenceLabel(capability.evidenceSource)}\n'
                        '${capability.reason}'
                    : '未観測（表示値: ${_hostCapabilityStatusLabel(capability.status)}） / '
                        '${capability.evidenceSource == 'LIVE_RUNTIME' ? 'fixture／診断値をBroker実測へ昇格しない' : 'Broker証拠なし'}\n'
                        '${capability.reason}',
              ),
            ),
        ],
      ),
    );
  }
}

String _hostCapabilityStatusLabel(String status) => switch (status) {
      'ready' => '利用可能',
      'degraded' => '制限付き',
      'unavailable' => '利用不可',
      'suspended' => '停止中',
      'failed' => '失敗',
      _ => '不明',
    };

String _hostCapabilityEvidenceLabel(String source) => switch (source) {
      'CONFIG' => '設定由来',
      'INTERNAL_STATE' => '内部状態',
      'LIVE_RUNTIME' => '実行中の観測',
      'EXTERNAL_EVIDENCE' => '外部証拠',
      'FIXTURE' => '試験fixture',
      _ => '証拠種別不明',
    };
