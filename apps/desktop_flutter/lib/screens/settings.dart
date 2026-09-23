import 'dart:convert';

import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/profile_client.dart';
import '../services/shell_core_client.dart';
import '../services/update_client.dart';
import 'shared.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  final TextEditingController _searchController = TextEditingController();
  bool _modifiedOnly = false;
  bool _authorityOnly = false;
  bool _dangerousOnly = false;
  bool _phaseReleaseOnly = false;
  late final ProfileClient? _profileClient;
  late final UpdateClient? _updateClient;
  Future<List<Map<String, Object?>>>? _profilesFuture;
  Future<Map<String, Object?>>? _updatesFuture;
  final TextEditingController _profileIdController =
      TextEditingController(text: 'default.local');
  final TextEditingController _profileNameController =
      TextEditingController(text: '標準ローカル実行');
  String? _profileMessage;
  String? _exportedProfile;
  String? _updateMessage;

  @override
  void dispose() {
    _searchController.dispose();
    _profileIdController.dispose();
    _profileNameController.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    _profileClient = widget.client.brokerTransport == null
        ? null
        : ProfileClient(widget.client.brokerTransport!);
    _updateClient = widget.client.brokerTransport == null
        ? null
        : UpdateClient(widget.client.brokerTransport!);
    _refreshProfiles();
    _refreshUpdates();
  }

  @override
  Widget build(BuildContext context) {
    final settings = widget.client.getSnapshot().settings;
    final filtered = settings.where(_matchesFilters).toList();
    return ShellPage(
      title: '設定',
      children: [
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('検索／絞込み', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              TextField(
                controller: _searchController,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.search),
                  labelText: '設定を検索',
                ),
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  FilterChip(
                    label: const Text('変更済み'),
                    selected: _modifiedOnly,
                    onSelected: (value) =>
                        setState(() => _modifiedOnly = value),
                  ),
                  FilterChip(
                    label: const Text('権限関連'),
                    selected: _authorityOnly,
                    onSelected: (value) =>
                        setState(() => _authorityOnly = value),
                  ),
                  FilterChip(
                    label: const Text('危険'),
                    selected: _dangerousOnly,
                    onSelected: (value) =>
                        setState(() => _dangerousOnly = value),
                  ),
                  FilterChip(
                    label: const Text('段階／リリース'),
                    selected: _phaseReleaseOnly,
                    onSelected: (value) =>
                        setState(() => _phaseReleaseOnly = value),
                  ),
                ],
              ),
            ],
          ),
        ),
        const SectionList(
          title: '権限境界',
          rows: [
            '段階Bの仕上げでは設定を表示専用とします。',
            '初期化、書き出し、変更操作はShell Coreの承認経路を通す必要があります。',
            '危険または権限関連の設定には、操作者確認用の印を付けます。',
          ],
        ),
        _profilePanel(),
        _updatePanel(),
        if (filtered.isEmpty)
          const EmptyStatePanel(
            title: '一致する設定なし',
            meaning: '現在の絞込み条件に一致する設定射影がスナップショット内にありません。',
            phaseBBlocked: false,
            nextAction: '絞込みを解除するか、設定が変わった場合は診断を更新してください。',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('設定')),
                DataColumn(label: Text('現在値')),
                DataColumn(label: Text('出所')),
                DataColumn(label: Text('有効値／注記')),
                DataColumn(label: Text('印')),
              ],
              rows: [for (final setting in filtered) _settingRow(setting)],
            ),
          ),
      ],
    );
  }

  Widget _profilePanel() {
    final client = _profileClient;
    if (client == null) {
      return const BorderedPanel(
        child: Text('運用プロファイル: Broker接続がないため操作できません。ローカルsnapshotは権限源ではありません。'),
      );
    }
    return BorderedPanel(
      child: FutureBuilder<List<Map<String, Object?>>>(
        future: _profilesFuture,
        builder: (context, snapshot) {
          final profiles = snapshot.data ?? const <Map<String, Object?>>[];
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('運用プロファイル', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 4),
              const Text(
                  '再利用可能な要求設定です。適用してもPermission、Approval、Authorityは生成されません。'),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  SizedBox(
                    width: 180,
                    child: TextField(
                      controller: _profileIdController,
                      decoration: const InputDecoration(labelText: 'プロファイル識別子'),
                    ),
                  ),
                  SizedBox(
                    width: 220,
                    child: TextField(
                      controller: _profileNameController,
                      decoration: const InputDecoration(labelText: '表示名'),
                    ),
                  ),
                  FilledButton.icon(
                    onPressed: _runProfileCreate,
                    icon: const Icon(Icons.add),
                    label: const Text('作成'),
                  ),
                  OutlinedButton.icon(
                    onPressed: () => _runProfileImport(client),
                    icon: const Icon(Icons.file_download_outlined),
                    label: const Text('import'),
                  ),
                  OutlinedButton.icon(
                    onPressed: _refreshProfiles,
                    icon: const Icon(Icons.refresh),
                    label: const Text('更新'),
                  ),
                ],
              ),
              if (_profileMessage != null) ...[
                const SizedBox(height: 8),
                Text(_profileMessage!),
              ],
              if (_exportedProfile != null) ...[
                const SizedBox(height: 8),
                SelectableText(_exportedProfile!),
              ],
              const SizedBox(height: 8),
              if (snapshot.connectionState == ConnectionState.waiting)
                const LinearProgressIndicator()
              else if (snapshot.hasError)
                Text('Profile一覧を取得できません: ${snapshot.error}')
              else if (profiles.isEmpty)
                const Text('保存済みProfileはありません。')
              else
                for (final profile in profiles) _profileRow(client, profile),
            ],
          );
        },
      ),
    );
  }

  Widget _updatePanel() {
    final client = _updateClient;
    if (client == null) {
      return const BorderedPanel(
        child: Text('更新センター: Broker接続がないため操作できません。snapshotは更新権限の根拠ではありません。'),
      );
    }
    return BorderedPanel(
      child: FutureBuilder<Map<String, Object?>>(
        future: _updatesFuture,
        builder: (context, snapshot) {
          final body = snapshot.data ?? const <String, Object?>{};
          final updates = body['更新一覧'] is List
              ? (body['更新一覧']! as List)
                  .whereType<Map>()
                  .map((value) => Map<String, Object?>.from(value))
                  .toList()
              : const <Map<String, Object?>>[];
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('更新センター', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 4),
              Text(
                '信頼設定=${body['署名信頼設定'] ?? 'unknown'}。署名検査前の候補は保存せず、download・適用・rollbackは現在suspendedです。',
              ),
              const SizedBox(height: 8),
              OutlinedButton.icon(
                onPressed: _refreshUpdates,
                icon: const Icon(Icons.refresh),
                label: const Text('更新一覧を確認'),
              ),
              if (_updateMessage != null) ...[
                const SizedBox(height: 8),
                Text(_updateMessage!),
              ],
              if (snapshot.connectionState == ConnectionState.waiting)
                const LinearProgressIndicator()
              else if (snapshot.hasError)
                Text('更新一覧を取得できません: ${snapshot.error}')
              else if (updates.isEmpty)
                const Text('署名検査済みの更新候補はありません。')
              else
                for (final update in updates) _updateRow(client, update),
            ],
          );
        },
      ),
    );
  }

  Widget _updateRow(UpdateClient client, Map<String, Object?> update) {
    final updateId = update['更新ID']?.toString() ?? '';
    final candidateHash = update['候補hash']?.toString() ?? '';
    return ListTile(
      contentPadding: EdgeInsets.zero,
      title: Text('${update['提供版'] ?? ''} ($updateId)'),
      subtitle: Text(
        'channel=${update['channel']} / 署名=${update['署名状態']} / rollback=${update['rollback可能']}',
      ),
      trailing: Wrap(
        spacing: 4,
        children: [
          TextButton(
            onPressed: () => _runUpdateRequest(
              () => client.requestDownload(
                updateId: updateId,
                candidateHash: candidateHash,
              ),
              'download要求を記録しました（実行はsuspended）。',
            ),
            child: const Text('download要求'),
          ),
          TextButton(
            onPressed: () => _runUpdateRequest(
              () => client.requestApply(
                updateId: updateId,
                candidateHash: candidateHash,
              ),
              '適用要求を記録しました（実行はsuspended）。',
            ),
            child: const Text('適用要求'),
          ),
          TextButton(
            onPressed: () => _runUpdateRequest(
              () => client.defer(
                updateId: updateId,
                candidateHash: candidateHash,
                deferredUntil: DateTime.now()
                    .toUtc()
                    .add(const Duration(days: 1))
                    .toIso8601String(),
              ),
              '更新を延期しました。',
            ),
            child: const Text('延期'),
          ),
          if (update['rollback可能'] == true)
            TextButton(
              onPressed: () => _runUpdateRequest(
                () => client.requestRollback(
                  updateId: updateId,
                  candidateHash: candidateHash,
                ),
                'rollback要求を記録しました（実行はsuspended）。',
              ),
              child: const Text('rollback要求'),
            ),
        ],
      ),
    );
  }

  void _refreshUpdates() {
    final client = _updateClient;
    if (client == null) return;
    setState(() {
      _updatesFuture = client.list();
    });
  }

  Future<void> _runUpdateRequest(
    Future<Map<String, Object?>> Function() request,
    String success,
  ) async {
    try {
      final body = await request();
      final state = body['実行状態']?.toString();
      _setUpdateMessage(state == 'suspended' ? '$success Brokerは実行を保留しました。' : success);
    } catch (error) {
      _setUpdateMessage('更新操作失敗: $error');
    }
    _refreshUpdates();
  }

  void _setUpdateMessage(String message) {
    if (!mounted) return;
    setState(() => _updateMessage = message);
  }

  Widget _profileRow(ProfileClient client, Map<String, Object?> profile) {
    final id = profile['ProfileID']?.toString() ?? '';
    final name = profile['表示名']?.toString() ?? id;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      title: Text('$name ($id)'),
      subtitle: Text(
        '実行系=${profile['Runtime']} / Adapter=${profile['Adapter']}',
      ),
      trailing: Wrap(
        spacing: 4,
        children: [
          TextButton(
            onPressed: () => _runProfileApply(client, profile),
            child: const Text('適用要求'),
          ),
          TextButton(
            onPressed: () => _runProfileCopy(client, profile),
            child: const Text('複製'),
          ),
          TextButton(
            onPressed: () => _runProfileExport(client, id),
            child: const Text('export'),
          ),
          TextButton(
            onPressed: () => _runProfileDelete(client, id),
            child: const Text('削除'),
          ),
        ],
      ),
    );
  }

  void _refreshProfiles() {
    final client = _profileClient;
    if (client == null) return;
    setState(() {
      _profilesFuture = client.list();
    });
  }

  Future<void> _runProfileCreate() async {
    final client = _profileClient;
    if (client == null) return;
    try {
      await client.create({
        '版': 1,
        'ProfileID': _profileIdController.text.trim(),
        '表示名': _profileNameController.text.trim(),
        'Runtime': 'runtime.local',
        'Adapter': 'adapter.local',
        '要求Capability': ['filesystem.read'],
        'Content Exposure': {'visibility': 'redacted'},
        'network exposure': 'loopback',
        'resource limit': {
          'cpu_millis': 1000,
          'memory_bytes': 1073741824,
          'max_processes': 2,
          'max_output_bytes': 1048576,
        },
        'UI preference': {
          'theme': 'system',
          'density': 'comfortable',
          'locale': 'ja-JP',
        },
      });
      _setProfileMessage('Profileを作成しました。');
    } catch (error) {
      _setProfileMessage('Profile作成失敗: $error');
    }
    _refreshProfiles();
  }

  Future<void> _runProfileApply(
      ProfileClient client, Map<String, Object?> profile) async {
    try {
      await client.apply(
        profileId: profile['ProfileID']!.toString(),
        profileHash: profile['profile_hash']?.toString() ?? '',
      );
      _setProfileMessage('Profile適用要求を記録しました。権限は変更していません。');
    } catch (error) {
      _setProfileMessage('Profile適用要求失敗: $error');
    }
  }

  Future<void> _runProfileCopy(
    ProfileClient client,
    Map<String, Object?> profile,
  ) async {
    final sourceId = profile['ProfileID']?.toString() ?? '';
    try {
      await client.copy(
        sourceProfileId: sourceId,
        profileId: '$sourceId.copy',
        displayName: '${profile['表示名'] ?? sourceId} の複製',
      );
      _setProfileMessage('Profileを複製しました。');
    } catch (error) {
      _setProfileMessage('Profile複製失敗: $error');
    }
    _refreshProfiles();
  }

  Future<void> _runProfileExport(ProfileClient client, String id) async {
    try {
      final receipt = await client.export(id);
      final profile = receipt['Profile'];
      _exportedProfile = profile is Map
          ? profileJson(Map<String, Object?>.from(profile))
          : null;
      _setProfileMessage('Profile設定をexportしました。');
    } catch (error) {
      _setProfileMessage('Profile export失敗: $error');
    }
  }

  Future<void> _runProfileDelete(ProfileClient client, String id) async {
    try {
      await client.delete(profileId: id);
      _setProfileMessage('Profileを削除しました。');
    } catch (error) {
      _setProfileMessage('Profile削除失敗: $error');
    }
    _refreshProfiles();
  }

  Future<void> _runProfileImport(ProfileClient client) async {
    final controller = TextEditingController();
    final raw = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Profile設定をimport'),
        content: TextField(
          controller: controller,
          minLines: 5,
          maxLines: 12,
          decoration: const InputDecoration(
            border: OutlineInputBorder(),
            hintText: 'exportしたProfile JSONを貼り付けてください',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(controller.text),
            child: const Text('import'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (raw == null || raw.trim().isEmpty) return;
    try {
      final decoded = jsonDecode(raw);
      if (decoded is! Map) throw const FormatException('objectではありません');
      await client.importProfile(Map<String, Object?>.from(decoded));
      _setProfileMessage('Profileをimportしました。');
    } catch (error) {
      _setProfileMessage('Profile import失敗: $error');
    }
    _refreshProfiles();
  }

  void _setProfileMessage(String message) {
    if (mounted) setState(() => _profileMessage = message);
  }

  bool _matchesFilters(SettingRecord setting) {
    final query = _searchController.text.trim().toLowerCase();
    final searchable = [
      setting.key,
      setting.group,
      setting.defaultValue,
      setting.currentValue,
      setting.effectiveValue,
      setting.source,
      _flags(setting),
    ].join(' ').toLowerCase();
    if (query.isNotEmpty && !searchable.contains(query)) {
      return false;
    }
    if (_modifiedOnly && !setting.modified) {
      return false;
    }
    if (_authorityOnly && !setting.authorityRelated) {
      return false;
    }
    if (_dangerousOnly && !setting.dangerous) {
      return false;
    }
    if (_phaseReleaseOnly &&
        !searchable.contains('phase') &&
        !searchable.contains('release')) {
      return false;
    }
    return true;
  }

  DataRow _settingRow(SettingRecord setting) {
    return DataRow(
      cells: [
        DataCell(Text('${setting.key}\n${setting.group}')),
        DataCell(Text(setting.currentValue)),
        DataCell(Text(setting.source)),
        DataCell(
          Text('既定値: ${setting.defaultValue}\n有効値: ${setting.effectiveValue}'),
        ),
        DataCell(Text(_flags(setting))),
      ],
    );
  }

  String _flags(SettingRecord setting) {
    final flags = [
      if (setting.modified) '変更済み',
      if (setting.dangerous) '危険',
      if (setting.authorityRelated) '権限関連',
    ];
    return flags.isEmpty ? 'なし' : flags.join(', ');
  }
}
