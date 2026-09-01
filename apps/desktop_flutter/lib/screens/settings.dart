import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
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

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
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
