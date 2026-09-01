import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class AuditViewer extends StatefulWidget {
  const AuditViewer({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<AuditViewer> createState() => _AuditViewerState();
}

class _AuditViewerState extends State<AuditViewer> {
  final TextEditingController _searchController = TextEditingController();
  String _category = 'all';
  String _result = 'all';

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final auditEvents = snapshot.auditEvents.where(_matchesFilters).toList();
    return ShellPage(
      title: '監査ビューアー',
      children: [
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('監査時系列の絞込み', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              TextField(
                controller: _searchController,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.search),
                  labelText: '監査事象を検索',
                ),
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  for (final category in [
                    'all',
                    'runtime',
                    'approval',
                    'recovery',
                    'evidence',
                    'setup_doctor',
                  ])
                    ChoiceChip(
                      label: Text(_categoryLabel(category)),
                      selected: _category == category,
                      onSelected: (_) => setState(() => _category = category),
                    ),
                  const SizedBox(width: 8),
                  for (final result in ['all', 'success', 'warning', 'blocked'])
                    FilterChip(
                      label: Text(_resultLabel(result)),
                      selected: _result == result,
                      onSelected: (_) => setState(() => _result = result),
                    ),
                ],
              ),
            ],
          ),
        ),
        SectionList(
          title: '監査鎖状態',
          rows: [
            '監査鎖状態: ${snapshot.auditChainStatus}',
            'ハッシュ鎖状態: ${snapshot.auditChainStatus}',
            '操作: 事象をコピー／JSONLを書き出し／鎖を検証／関連する承認・実行系・アダプターを確認',
          ],
        ),
        if (auditEvents.isEmpty)
          const EmptyStatePanel(
            title: '一致する監査事象なし',
            meaning: '現在の絞込み条件では全監査事象が非表示です。',
            phaseBBlocked: false,
            nextAction: '絞込みを解除するか診断を更新してください。',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('事象')),
                DataColumn(label: Text('作用')),
                DataColumn(label: Text('結果')),
                DataColumn(label: Text('内容ハッシュ')),
                DataColumn(label: Text('直前')),
                DataColumn(label: Text('関連')),
                DataColumn(label: Text('コピー')),
              ],
              rows: [
                for (final event in auditEvents)
                  DataRow(
                    cells: [
                      DataCell(Text(event.eventId)),
                      DataCell(Text(event.action)),
                      DataCell(Text(event.result)),
                      DataCell(Text(event.payloadHash)),
                      DataCell(Text(event.previousEventHash ?? 'root')),
                      DataCell(Text(_relatedText(snapshot, event))),
                      DataCell(
                        IconButton(
                          tooltip: '監査事象JSONをコピー',
                          icon: const Icon(Icons.copy, size: 18),
                          onPressed: () => Clipboard.setData(
                            ClipboardData(text: _eventJson(event)),
                          ),
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ),
      ],
    );
  }

  bool _matchesFilters(AuditRecord event) {
    final searchable =
        '${event.eventId} ${event.action} ${event.result} ${event.payloadHash} ${event.previousEventHash ?? ''}'
            .toLowerCase();
    final query = _searchController.text.trim().toLowerCase();
    if (query.isNotEmpty && !searchable.contains(query)) {
      return false;
    }
    if (_category != 'all' && !searchable.contains(_category)) {
      return false;
    }
    if (_result != 'all' && !event.result.toLowerCase().contains(_result)) {
      return false;
    }
    return true;
  }
}

String _categoryLabel(String value) {
  return switch (value) {
    'all' => 'すべて',
    'runtime' => '実行系',
    'approval' => '承認',
    'recovery' => '復旧',
    'evidence' => '証拠',
    'setup_doctor' => '環境診断',
    _ => value,
  };
}

String _resultLabel(String value) {
  return switch (value) {
    'all' => 'すべて',
    'success' => '成功',
    'warning' => '警告',
    'blocked' => '遮断',
    _ => value,
  };
}

String _relatedText(ShellSnapshot snapshot, AuditRecord event) {
  final relatedAuthority = snapshot.authorityMap
      .where((item) => item.auditEventId == event.eventId)
      .toList();
  if (relatedAuthority.isEmpty) {
    return 'なし';
  }
  return relatedAuthority
      .map(
        (item) => '${item.runtimeId}/${item.capabilityId}/${item.recoveryId}',
      )
      .join('\n');
}

String _eventJson(AuditRecord event) {
  return const JsonEncoder.withIndent('  ').convert({
    'event_id': event.eventId,
    'action': event.action,
    'result': event.result,
    'payload_hash': event.payloadHash,
    'previous_event_hash': event.previousEventHash,
  });
}
