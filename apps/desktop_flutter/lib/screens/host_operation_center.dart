import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import '../services/host_registration_client.dart';
import 'shared.dart';

class HostOperationCenter extends StatefulWidget {
  const HostOperationCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<HostOperationCenter> createState() => _HostOperationCenterState();
}

class _HostOperationCenterState extends State<HostOperationCenter> {
  String? _activeHostId;
  late List<HostRegistryRecord> _hosts;
  String? _candidateHostId;
  bool _busy = false;
  String _message = 'Hostを選択すると、表示コンテキストだけをBroker監査付きで切り替えます。';

  @override
  void initState() {
    super.initState();
    final hosts = widget.client.getSnapshot().hosts;
    _hosts = hosts;
    if (hosts.isNotEmpty) {
      _activeHostId = hosts.first.hostId;
      _candidateHostId = hosts.first.hostId;
    }
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final hosts = _hosts;
    final selected = _findHost(hosts, _candidateHostId ?? _activeHostId);
    final active = _findHost(hosts, _activeHostId);
    return ShellPage(
      title: 'D4 Pocket Host操作面',
      evidenceTitle: 'Host Operation Center',
      children: [
        if (!kIsWeb && defaultTargetPlatform == TargetPlatform.macOS)
          BorderedPanel(
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                const Text('公開metadataだけを登録します。remote接続やTrust・権限は生成しません。'),
                FilledButton(
                    onPressed: _busy || widget.client.brokerTransport == null
                        ? null
                        : _registerHost,
                    child: const Text('Hostを登録')),
                Text(_message),
              ])),
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
      final receipt =
          await widget.client.selectHost(host.hostId, selectedHost: host);
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

  Future<void> _registerHost() async {
    final input = await showDialog<_HostInput>(
        context: context, builder: (_) => const _HostRegistrationDialog());
    if (input == null || !mounted) return;
    setState(() {
      _busy = true;
      _message = '別個のnative確認後にHost metadataをBrokerへ要求します。';
    });
    try {
      final transport = widget.client.brokerTransport;
      if (transport == null) throw StateError('Broker未接続');
      final client = HostRegistrationClient(transport);
      final auditId = await client.register(
          hostId: input.id,
          displayName: input.name,
          platform: input.platform,
          identityHash: input.hash,
          runtimeCount: input.runtimes,
          agentCount: input.agents);
      final hosts = await client.refresh();
      if (!hosts.any((h) => h.hostId == input.id)) throw StateError('現在一覧で未確認');
      if (!mounted) return;
      setState(() {
        _hosts = hosts;
        _candidateHostId = input.id;
        _message = 'Host metadataを登録しました。未審査のままです。Audit=$auditId';
      });
    } on Object {
      if (mounted) {
        setState(() {
          _message = 'Host登録または一覧更新は未成立です。Brokerの拒否・Auditを確認してください。自動再送しません。';
        });
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }
}

class _HostInput {
  const _HostInput(
      this.id, this.name, this.platform, this.hash, this.runtimes, this.agents);
  final String id, name, platform, hash;
  final int runtimes, agents;
}

class _HostRegistrationDialog extends StatefulWidget {
  const _HostRegistrationDialog();
  @override
  State<_HostRegistrationDialog> createState() =>
      _HostRegistrationDialogState();
}

class _HostRegistrationDialogState extends State<_HostRegistrationDialog> {
  final _form = GlobalKey<FormState>();
  final _fields = <String, TextEditingController>{
    for (final label in [
      'Host識別子',
      'Host表示名',
      '公開identity hash',
      '申告Runtime件数',
      '申告Agent件数'
    ])
      label: TextEditingController(),
  };
  String _platform = 'macos';
  @override
  void dispose() {
    for (final controller in _fields.values) {
      controller.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
        title: const Text('公開Host metadata登録'),
        content: SizedBox(
            width: 480,
            child: SingleChildScrollView(
                child: Form(
                    key: _form,
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Text(
                            '秘密値・証明書実値・endpointは入力しないでください。件数は申告値であり実測ではありません。'),
                        for (final entry in _fields.entries)
                          Padding(
                              padding: const EdgeInsets.only(top: 12),
                              child: TextFormField(
                                controller: entry.value,
                                maxLength: entry.key == 'Host表示名' ? 256 : 128,
                                decoration:
                                    InputDecoration(labelText: entry.key),
                                validator: (value) {
                                  final text = value ?? '';
                                  if (text.trim().isEmpty ||
                                      text.runes
                                          .any((r) => r < 32 || r == 127)) {
                                    return '公開値を入力してください';
                                  }
                                  if (entry.key == 'Host識別子' &&
                                      !RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.:-]*$')
                                          .hasMatch(text)) {
                                    return 'Host IDの形式が不正です';
                                  }
                                  if (entry.key == '公開identity hash' &&
                                      (text.length != 71 ||
                                          !RegExp(r'^sha256:[a-f0-9]{64}$')
                                              .hasMatch(text))) {
                                    return 'sha256:に続く64桁の公開hashを入力してください';
                                  }
                                  if (entry.key.startsWith('申告')) {
                                    final number = int.tryParse(text);
                                    if (number == null ||
                                        number < 0 ||
                                        number > 256) {
                                      return '0〜256の申告件数を明示してください';
                                    }
                                  }
                                  return null;
                                },
                              )),
                        DropdownButtonFormField<String>(
                            initialValue: _platform,
                            decoration: const InputDecoration(
                                labelText: '基盤（Platform）'),
                            items: [
                              for (final value in [
                                'windows',
                                'linux',
                                'macos',
                                'android',
                                'ios',
                                'unknown'
                              ])
                                DropdownMenuItem(
                                    value: value, child: Text(value))
                            ],
                            onChanged: (value) {
                              if (value != null) {
                                setState(() => _platform = value);
                              }
                            }),
                      ],
                    )))),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(context), child: const Text('取消')),
          FilledButton(
              onPressed: () {
                if (!_form.currentState!.validate()) return;
                Navigator.pop(
                    context,
                    _HostInput(
                        _fields['Host識別子']!.text,
                        _fields['Host表示名']!.text,
                        _platform,
                        _fields['公開identity hash']!.text,
                        int.parse(_fields['申告Runtime件数']!.text),
                        int.parse(_fields['申告Agent件数']!.text)));
              },
              child: const Text('Owner確認して登録'))
        ],
      );
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
