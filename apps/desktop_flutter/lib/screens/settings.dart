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
      title: 'Settings',
      children: [
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('Search / Filter',
                  style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              TextField(
                controller: _searchController,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.search),
                  labelText: 'Search settings',
                ),
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  FilterChip(
                    label: const Text('modified'),
                    selected: _modifiedOnly,
                    onSelected: (value) =>
                        setState(() => _modifiedOnly = value),
                  ),
                  FilterChip(
                    label: const Text('authority'),
                    selected: _authorityOnly,
                    onSelected: (value) =>
                        setState(() => _authorityOnly = value),
                  ),
                  FilterChip(
                    label: const Text('dangerous'),
                    selected: _dangerousOnly,
                    onSelected: (value) =>
                        setState(() => _dangerousOnly = value),
                  ),
                  FilterChip(
                    label: const Text('phase/release'),
                    selected: _phaseReleaseOnly,
                    onSelected: (value) =>
                        setState(() => _phaseReleaseOnly = value),
                  ),
                ],
              ),
            ],
          ),
        ),
        const SectionList(title: 'Authority Boundary', rows: [
          'Settings are display-only in Phase B polish.',
          'Reset, export, and mutating changes must go through Shell Core approval paths.',
          'Dangerous or authority-related settings are flagged for operator review.',
        ]),
        if (filtered.isEmpty)
          const EmptyStatePanel(
            title: 'No matching settings',
            meaning:
                'The current filter did not match any setting projection in the snapshot.',
            phaseBBlocked: false,
            nextAction:
                'Clear filters or refresh diagnostics if settings changed.',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('Setting')),
                DataColumn(label: Text('Current')),
                DataColumn(label: Text('Source')),
                DataColumn(label: Text('Effect / Notes')),
                DataColumn(label: Text('Flags')),
              ],
              rows: [
                for (final setting in filtered) _settingRow(setting),
              ],
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
    return DataRow(cells: [
      DataCell(Text('${setting.key}\n${setting.group}')),
      DataCell(Text(setting.currentValue)),
      DataCell(Text(setting.source)),
      DataCell(Text(
        'default: ${setting.defaultValue}\neffective: ${setting.effectiveValue}',
      )),
      DataCell(Text(_flags(setting))),
    ]);
  }

  String _flags(SettingRecord setting) {
    final flags = [
      if (setting.modified) 'modified',
      if (setting.dangerous) 'dangerous',
      if (setting.authorityRelated) 'authority',
    ];
    return flags.isEmpty ? 'none' : flags.join(', ');
  }
}
