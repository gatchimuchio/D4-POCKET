import 'package:flutter/material.dart';
import '../services/device_link_controller.dart';
import '../services/mobile_projection_client.dart';
import 'shared.dart';

class MobileAgentStatus extends StatefulWidget {
  const MobileAgentStatus({
    super.key,
    required this.controller,
    required this.connected,
    required this.active,
  });

  final DeviceLinkController controller;
  final bool connected;
  final bool active;

  @override
  State<MobileAgentStatus> createState() => _MobileAgentStatusState();
}

class _MobileAgentStatusState extends State<MobileAgentStatus> {
  Future<MobileAgentStatusList>? _future;

  @override
  void didUpdateWidget(MobileAgentStatus oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!widget.connected ||
        !widget.active ||
        oldWidget.connected != widget.connected ||
        oldWidget.active != widget.active) {
      _future = null;
    }
  }

  Future<MobileAgentStatusList> _load() =>
      MobileProjectionClient(widget.controller).agents();

  void _refresh() {
    _future = widget.connected ? _load() : null;
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    if (!widget.active) return const SizedBox.shrink();
    if (widget.connected && _future == null) _future = _load();
    return MobilePage(
      title: 'Agent状態',
      children: [
        const Text(
          '接続中Desktop BrokerのAgent Adapter metadataを表示します。これは実taskの実行可否、Permission、Approval、Trustを保証しません。資格値、Workspace path、ToolやMCPの実行は扱いません。',
        ),
        if (!widget.connected)
          const StatusTile(
            icon: Icons.help_outline,
            title: '未接続',
            subtitle: 'Agent状態は未観測です。',
          )
        else ...[
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton.icon(
              onPressed: widget.controller.busy ? null : _refresh,
              icon: const Icon(Icons.refresh),
              label: const Text('状態を更新'),
            ),
          ),
          FutureBuilder<MobileAgentStatusList>(
            future: _future,
            builder: (context, snapshot) {
              if (snapshot.connectionState == ConnectionState.waiting) {
                return const LinearProgressIndicator();
              }
              if (snapshot.hasError) {
                return const StatusTile(
                  icon: Icons.error_outline,
                  title: 'Agent状態を取得できません',
                  subtitle: 'Broker応答を確認できないため、未観測として扱います。',
                );
              }
              final result = snapshot.data;
              if (result == null || result.items.isEmpty) {
                return const StatusTile(
                  icon: Icons.help_outline,
                  title: 'Agent metadataなし',
                  subtitle: '利用可能なAgentとは推定しません。',
                );
              }
              return Column(
                children: result.items.map(_agentTile).toList(growable: false),
              );
            },
          ),
        ],
      ],
    );
  }

  Widget _agentTile(MobileAgentSummary agent) {
    final capabilities = agent.capabilities
        .take(8)
        .map((item) => '${item.capabilityId}: ${_supportLabel(item.status)}')
        .join('・');
    final remainder = agent.capabilities.length > 8
        ? '・ほか${agent.capabilities.length - 8}件'
        : '';
    return Column(
      children: [
        StatusTile(
          icon: _statusIcon(agent.status),
          title: agent.agentId,
          subtitle: '${agent.provider} / ${agent.model} / ${agent.version}',
        ),
        StatusTile(
          icon: Icons.info_outline,
          title: 'Broker報告状態',
          subtitle: _statusLabel(agent.status),
        ),
        StatusTile(
          icon: Icons.extension_outlined,
          title: '宣言Capability',
          subtitle: capabilities.isEmpty ? '未観測' : '$capabilities$remainder',
        ),
        const SizedBox(height: 8),
      ],
    );
  }

  IconData _statusIcon(String status) => switch (status) {
    'ready' => Icons.check_circle_outline,
    'degraded' => Icons.warning_amber_outlined,
    'unavailable' => Icons.cloud_off_outlined,
    _ => Icons.help_outline,
  };

  String _statusLabel(String status) => switch (status) {
    'ready' => '準備状態をBroker metadataが報告（実task実行は未確認）',
    'degraded' => '一部制限あり',
    'unavailable' => '利用不可',
    _ => '未対応',
  };

  String _supportLabel(String status) => switch (status) {
    'supported' => '対応と宣言',
    'unsupported' => '未対応',
    _ => '不明',
  };
}
