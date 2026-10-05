import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class HostOperationCenter extends StatefulWidget {
  const HostOperationCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<HostOperationCenter> createState() => _HostOperationCenterState();
}

class _HostOperationCenterState extends State<HostOperationCenter> {
  String? _activeHostId;
  String? _candidateHostId;
  bool _busy = false;
  String _message = 'Hostを選択すると、表示コンテキストだけをBroker監査付きで切り替えます。';

  @override
  void initState() {
    super.initState();
    final hosts = widget.client.getSnapshot().hosts;
    if (hosts.isNotEmpty) {
      _activeHostId = hosts.first.hostId;
      _candidateHostId = hosts.first.hostId;
    }
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final hosts = snapshot.hosts;
    final selected = _findHost(hosts, _candidateHostId ?? _activeHostId);
    final active = _findHost(hosts, _activeHostId);
    return ShellPage(
      title: 'D4 Pocket Host操作面',
      evidenceTitle: 'Host Operation Center',
      children: [
        const BorderedPanel(
          child: Text(
            'Host一覧はRust Brokerのmetadata-only projectionです。'
            'Host切替は表示対象を変えるだけで、Permission、Approval、Authority、Credentialを生成・共有しません。'
            'ローカル実測は、認証済みBrokerから受理したsnapshotでHost IDが一致する場合だけに限定します。',
          ),
        ),
        if (hosts.isEmpty)
          const BorderedPanel(
            child: Text('登録済みHostがありません。Runtime／Agent一覧も未観測です。'),
          )
        else
          _HostOperationLayout(
            hosts: hosts,
            selected: selected,
            active: active,
            candidateHostId: _candidateHostId,
            busy: _busy,
            message: _message,
            onSelect: (hostId) => setState(() => _candidateHostId = hostId),
            onSwitch:
                selected == null || _busy || selected.hostId == _activeHostId
                    ? null
                    : () => _switchHost(selected),
            snapshot: snapshot,
          ),
        const BorderedPanel(
          child: Text(
            '観測境界: registryのRuntime／Agent件数は内部状態のsummaryです。'
            '現在のBroker観測Hostと一致しないHostについて、個別のRuntime／Agentを推測表示しません。',
          ),
        ),
      ],
    );
  }

  Future<void> _switchHost(HostRegistryRecord host) async {
    setState(() {
      _busy = true;
      _message = '${host.displayName} へのHost切替をBrokerへ要求しています。';
    });
    try {
      final receipt = await widget.client.selectHost(host.hostId);
      if (!mounted) return;
      setState(() {
        _activeHostId = receipt.hostId;
        _candidateHostId = receipt.hostId;
        _message =
            'Host切替を監査しました。監査ID=${receipt.auditId} / 承認=${receipt.approvalState} / 権限生成=${receipt.authorityGenerated}';
        _busy = false;
      });
    } on Object catch (error) {
      if (!mounted) return;
      setState(() {
        _message = 'Host切替を拒否しました。$error';
        _busy = false;
      });
    }
  }
}

class _HostOperationLayout extends StatelessWidget {
  const _HostOperationLayout({
    required this.hosts,
    required this.selected,
    required this.active,
    required this.candidateHostId,
    required this.busy,
    required this.message,
    required this.onSelect,
    required this.onSwitch,
    required this.snapshot,
  });

  final List<HostRegistryRecord> hosts;
  final HostRegistryRecord? selected;
  final HostRegistryRecord? active;
  final String? candidateHostId;
  final bool busy;
  final String message;
  final ValueChanged<String> onSelect;
  final VoidCallback? onSwitch;
  final ShellSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 300,
          child: BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Host一覧', style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                for (final host in hosts)
                  ListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    selected: host.hostId == candidateHostId,
                    leading: Icon(
                      host.hostId == active?.hostId
                          ? Icons.radio_button_checked
                          : Icons.radio_button_unchecked,
                    ),
                    title: Text(host.displayName),
                    subtitle: Text(
                      '識別子: ${host.hostId}\n基盤: ${host.platform} / 接続: ${host.connectionState}',
                    ),
                    onTap: () => onSelect(host.hostId),
                  ),
              ],
            ),
          ),
        ),
        const SizedBox(width: 16),
        Expanded(
          child: selected == null
              ? const BorderedPanel(child: Text('Hostを選択してください。'))
              : _HostDetail(
                  host: selected!,
                  active: selected!.hostId == active?.hostId,
                  busy: busy,
                  message: message,
                  onSwitch: onSwitch,
                  snapshot: snapshot,
                ),
        ),
      ],
    );
  }
}

class _HostDetail extends StatelessWidget {
  const _HostDetail({
    required this.host,
    required this.active,
    required this.busy,
    required this.message,
    required this.onSwitch,
    required this.snapshot,
  });

  final HostRegistryRecord host;
  final bool active;
  final bool busy;
  final String message;
  final VoidCallback? onSwitch;
  final ShellSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    final brokerSnapshot = snapshot.snapshotSource == 'broker';
    final localObservation = brokerSnapshot &&
        snapshot.hostCapabilities.any(
          (item) => item.hostId == host.hostId,
        );
    final hostLocation = !brokerSnapshot
        ? '未確認（Broker由来の実測証拠がありません）'
        : localObservation
            ? '現在のBroker実行Host（ローカル）'
            : '別登録Host（remote接続・個別状態は未観測）';
    final unobservedMessage = !brokerSnapshot
        ? '未観測（snapshotはBroker確定経路ではありません。summary以外を実状態として扱いません）'
        : '未観測（別Hostへのlive接続がないため、summary以外を表示しません）';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(host.displayName,
                  style: Theme.of(context).textTheme.titleLarge),
              Text('ホスト識別子: ${host.hostId}'),
              Text('基盤: ${host.platform}'),
              Text('Host所在: $hostLocation'),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  StatusPill(label: '登録接続状態', value: host.connectionState),
                  StatusPill(label: '信頼', value: host.trustState),
                  StatusPill(label: '証拠', value: host.evidenceSource),
                  StatusPill(label: '公開範囲', value: host.visibility),
                ],
              ),
              const SizedBox(height: 8),
              Text(
                  'Runtime summary: ${host.runtimeCount}件 / Agent summary: ${host.agentCount}件'),
              Text(active ? '現在の表示Host' : '切替候補'),
              const SizedBox(height: 12),
              FilledButton.icon(
                onPressed: onSwitch,
                icon: busy
                    ? const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.swap_horiz),
                label: Text(active ? '現在のHost' : 'Host切替'),
              ),
              const SizedBox(height: 8),
              Text(message),
            ],
          ),
        ),
        const SizedBox(height: 16),
        _ObservationPanel(
          title: 'Runtime一覧',
          icon: Icons.hub_outlined,
          observed: localObservation,
          count: host.runtimeCount,
          unobservedMessage: unobservedMessage,
          children: localObservation
              ? [
                  for (final runtime in snapshot.runtimes)
                    ListTile(
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      title: Text(runtime.runtimeId),
                      subtitle: Text(
                        '${runtime.name} / ${runtime.status} / Broker観測',
                      ),
                    ),
                ]
              : const [],
        ),
        const SizedBox(height: 16),
        _ObservationPanel(
          title: 'Agent一覧',
          icon: Icons.smart_toy_outlined,
          observed: localObservation,
          count: host.agentCount,
          unobservedMessage: unobservedMessage,
          children: localObservation
              ? [
                  for (final agent in snapshot.agentSessions)
                    ListTile(
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      title: Text(agent.sessionId),
                      subtitle: Text(
                        '${agent.agentRuntimeId} / ${agent.status} / ${agent.evidenceSource}',
                      ),
                    ),
                ]
              : const [],
        ),
        if (host.trustState != 'trusted' || host.connectionState != 'connected')
          const Padding(
            padding: EdgeInsets.only(top: 16),
            child: BorderedPanel(
              child: Text(
                'Host registryのTrust／接続状態は未確認です。Brokerのローカル実測が存在しても、このmetadataからTrustや権限作用は生成しません。',
              ),
            ),
          ),
      ],
    );
  }
}

class _ObservationPanel extends StatelessWidget {
  const _ObservationPanel({
    required this.title,
    required this.icon,
    required this.observed,
    required this.count,
    required this.unobservedMessage,
    required this.children,
  });

  final String title;
  final IconData icon;
  final bool observed;
  final int count;
  final String unobservedMessage;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(icon),
              const SizedBox(width: 8),
              Text(title, style: Theme.of(context).textTheme.titleMedium),
              const Spacer(),
              Text('summary $count件'),
            ],
          ),
          const SizedBox(height: 8),
          if (observed && children.isNotEmpty)
            ...children
          else
            Text(
              observed ? '現在のBrokerから個別項目は観測されていません。' : unobservedMessage,
            ),
        ],
      ),
    );
  }
}

HostRegistryRecord? _findHost(
  List<HostRegistryRecord> hosts,
  String? hostId,
) {
  for (final host in hosts) {
    if (host.hostId == hostId) return host;
  }
  return null;
}
