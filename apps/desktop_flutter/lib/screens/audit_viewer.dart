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
  AuditInspectionPage? _page;
  bool _loading = false;
  String? _loadError;
  int _loadGeneration = 0;

  @override
  void dispose() {
    _loadGeneration++;
    _searchController.dispose();
    super.dispose();
  }

  @override
  void didUpdateWidget(covariant AuditViewer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.client, widget.client) ||
        !identical(
            oldWidget.client.brokerTransport, widget.client.brokerTransport)) {
      _loadGeneration++;
      _page = null;
      _loading = false;
      _loadError = null;
    }
  }

  Future<void> _loadPage({String? beforeEventId}) async {
    final generation = ++_loadGeneration;
    setState(() {
      _page = null;
      _loading = true;
      _loadError = null;
    });
    try {
      final page = await widget.client.refreshAuditInspection(
        limit: 100,
        beforeEventId: beforeEventId,
      );
      if (!mounted || generation != _loadGeneration) return;
      setState(() => _page = page);
    } on Object {
      if (!mounted || generation != _loadGeneration) return;
      setState(() => _loadError = 'Broker監査一覧を取得できません。接続と監査状態を確認してください。');
    } finally {
      if (mounted && generation == _loadGeneration) {
        setState(() => _loading = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final brokerMode = widget.client.mode == 'broker';
    final brokerUnavailableMode = widget.client.mode == 'broker_unavailable';
    final brokerPage = brokerMode ? _page : null;
    final sourceEvents = brokerMode
        ? brokerPage?.events ?? const <AuditRecord>[]
        : snapshot.auditEvents;
    final auditEvents = sourceEvents.where(_matchesFilters).toList();
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
                  for (final result in [
                    'all',
                    'success',
                    'warning',
                    'blocked',
                    'recorded',
                  ])
                    FilterChip(
                      label: Text(_resultLabel(result)),
                      selected: _result == result,
                      onSelected: (_) => setState(() => _result = result),
                    ),
                ],
              ),
              if (brokerMode) ...[
                const SizedBox(height: 12),
                OutlinedButton.icon(
                  key: const ValueKey('audit-inspection-refresh'),
                  onPressed: _loading ? null : () => _loadPage(),
                  icon: _loading
                      ? const SizedBox.square(
                          dimension: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.refresh),
                  label: Text(_page == null ? '監査一覧を取得' : '最新の監査一覧へ更新'),
                ),
                if (_loadError != null) ...[
                  const SizedBox(height: 8),
                  Text(_loadError!),
                ],
              ],
            ],
          ),
        ),
        SectionList(
          title: '監査鎖状態',
          rows: [
            '監査一覧取得: ${brokerPage != null ? 'Brokerから取得済み' : _loading ? '取得中' : brokerMode ? '未取得' : brokerUnavailableMode ? 'Broker接続不可' : '診断fixtureのみ'}',
            '監査鎖状態: ${brokerPage != null ? '検証済み' : '未検証'}',
            'ハッシュ鎖状態: ${brokerPage != null ? '検証済み' : '未検証'}',
            brokerPage != null
                ? '全記録 ${brokerPage.verifiedEventCount} 件の保存chainとanchorをBrokerが検証。取得境界: ${brokerPage.snapshotHeadEventId ?? '記録なし'}。表示は最大100件です。'
                : brokerMode
                    ? '明示取得後にBrokerが検証したbounded pageだけを表示します。'
                    : '診断fixtureのコピーだけが利用できます。永続監査記録は取得していません。',
          ],
        ),
        if (auditEvents.isEmpty)
          EmptyStatePanel(
            title: brokerMode
                ? brokerPage != null
                    ? '表示条件に一致する監査eventなし'
                    : _loading
                        ? 'Broker監査一覧を取得中'
                        : _loadError != null
                            ? 'Broker監査一覧は表示できません'
                            : 'Broker監査一覧は未取得'
                : brokerUnavailableMode
                    ? 'Broker監査一覧を取得できません'
                    : '一致する診断fixtureなし',
            meaning: brokerMode
                ? brokerPage != null
                    ? '現在の絞込み条件ではこの検証済みpageに一致するeventがありません。'
                    : _loadError ?? '永続監査記録はBrokerから明示取得した後に表示します。'
                : brokerUnavailableMode
                    ? 'Broker接続を復旧後に一覧を取得してください。'
                    : '現在の絞込み条件では診断用fixtureが非表示です。',
            phaseBBlocked: false,
            nextAction: brokerMode
                ? brokerPage != null
                    ? '絞込みを解除するか、別pageへ移動してください。'
                    : '「監査一覧を取得」を選択してBrokerから検証済みpageを取得してください。'
                : brokerUnavailableMode
                    ? 'Brokerへの接続を復旧してから、監査一覧の読取状態を確認してください。'
                    : '絞込みを解除して診断fixtureを再表示してください。',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('監査event')),
                DataColumn(label: Text('作用')),
                DataColumn(label: Text('結果')),
                DataColumn(label: Text('内容ハッシュ')),
                DataColumn(label: Text('直前')),
                DataColumn(label: Text('event hash')),
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
                      DataCell(Text(event.eventHash ?? '—')),
                      DataCell(Text(_relatedText(snapshot, event))),
                      DataCell(
                        IconButton(
                          tooltip: '監査投影JSONをコピー',
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
        if (brokerPage?.hasOlderEvents == true)
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton.icon(
              key: const ValueKey('audit-inspection-older'),
              onPressed: _loading
                  ? null
                  : () => _loadPage(
                        beforeEventId: brokerPage!.nextBeforeEventId,
                      ),
              icon: const Icon(Icons.history),
              label: const Text('さらに古い監査event'),
            ),
          ),
      ],
    );
  }

  bool _matchesFilters(AuditRecord event) {
    final searchable =
        '${event.eventId} ${event.action} ${event.result} ${event.payloadHash} ${event.previousEventHash ?? ''} ${event.eventHash ?? ''}'
            .toLowerCase();
    final query = _searchController.text.trim().toLowerCase();
    if (query.isNotEmpty && !searchable.contains(query)) {
      return false;
    }
    if (_category != 'all' && !_matchesCategory(event)) {
      return false;
    }
    if (_result != 'all' && !event.result.toLowerCase().contains(_result)) {
      return false;
    }
    return true;
  }

  bool _matchesCategory(AuditRecord event) {
    final action = event.action.toLowerCase();
    return switch (_category) {
      'runtime' => action.contains('runtime') ||
          action.contains('agenttask') ||
          action.contains('実行系'),
      'approval' => action.contains('approval') ||
          action.contains('承認') ||
          action.contains('許可'),
      'recovery' => action.contains('recovery') ||
          action.contains('回復') ||
          action.contains('復旧'),
      'evidence' => action.contains('evidence') ||
          action.contains('証拠') ||
          action.contains('監査'),
      'setup_doctor' =>
        action.contains('setup doctor') || action.contains('環境診断'),
      _ => true,
    };
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
    'recorded' => '記録',
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
    if (event.eventHash != null) 'event_hash': event.eventHash,
  });
}
