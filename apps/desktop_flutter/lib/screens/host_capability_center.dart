import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class HostCapabilityCenter extends StatelessWidget {
  const HostCapabilityCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final hosts = client.getSnapshot().hostCapabilities;
    return ShellPage(
      title: 'D4 Pocket ホスト能力',
      children: [
        const BorderedPanel(
          child: Text(
            '実行場所と利用可能な機能をRust Brokerから読み取って表示します。'
            'ここからPermissionやApprovalは生成しません。',
          ),
        ),
        if (hosts.isEmpty)
          const BorderedPanel(child: Text('現在のホスト能力を取得できません。')),
        for (final host in hosts) _HostCapabilityPanel(host: host),
      ],
    );
  }
}

class _HostCapabilityPanel extends StatelessWidget {
  const _HostCapabilityPanel({required this.host});

  final HostCapabilityRecord host;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(host.displayName, style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          Text('ホストID: ${host.hostId}'),
          Text('プラットフォーム: ${host.platform} / 状態: ${host.status}'),
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
                '${_hostCapabilityStatusLabel(capability.status)} / '
                '${_hostCapabilityEvidenceLabel(capability.evidenceSource)}\n'
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
