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
    final brokerMode = widget.client.mode == 'broker';
    final brokerUnavailableMode = widget.client.mode == 'broker_unavailable';
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
            '監査一覧取得: ${brokerMode ? 'Broker経路未接続' : brokerUnavailableMode ? 'Broker接続不可' : '診断fixtureのみ'}',
            '監査鎖状態: 未検証',
            'ハッシュ鎖状態: 未検証',
            '操作: 表示中の診断fixtureをコピーできます。永続監査記録と鎖の検証結果は未取得です。',
          ],
        ),
        if (auditEvents.isEmpty)
          EmptyStatePanel(
            title: brokerMode
                ? 'Broker監査一覧は未接続'
                : brokerUnavailableMode
                    ? 'Broker監査一覧を取得できません'
                    : '一致する診断fixtureなし',
            meaning: brokerMode || brokerUnavailableMode
                ? '永続監査記録はBrokerから取得していません。起動時応答の監査参照IDだけから一覧を作りません。'
                : '現在の絞込み条件では診断用fixtureが非表示です。',
            phaseBBlocked: false,
            nextAction: brokerMode
                ? 'Brokerの監査一覧読取経路が接続されるまで、実監査記録は表示できません。'
                : brokerUnavailableMode
                    ? 'Brokerへの接続を復旧してから、監査一覧の読取状態を確認してください。'
                    : '絞込みを解除して診断fixtureを再表示してください。',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('診断fixture事象')),
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
                          tooltip: '診断fixture JSONをコピー',
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
