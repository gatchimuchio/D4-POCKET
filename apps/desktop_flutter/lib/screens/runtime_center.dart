import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class RuntimeCenter extends StatefulWidget {
  const RuntimeCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<RuntimeCenter> createState() => _RuntimeCenterState();
}

class _RuntimeCenterState extends State<RuntimeCenter> {
  String? _selectedRuntimeId;

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final selectedRuntime = _selectedRuntime(snapshot);
    return ShellPage(
      title: '実行系センター',
      children: [
        if (snapshot.runtimes.isEmpty)
          const EmptyStatePanel(
            title: '接続中の実行系なし',
            meaning: '現在のスナップショットに実行系の記録がありません。',
            phaseBBlocked: false,
            nextAction: '実行系の検出完了後に、ブローカーの製品経路へ再接続するか診断を更新してください。',
          )
        else
          LayoutBuilder(
            builder: (context, constraints) {
              final table = _RuntimeTable(
                snapshot: snapshot,
                selectedRuntimeId: selectedRuntime?.runtimeId ??
                    snapshot.runtimes.first.runtimeId,
                onSelected: (runtimeId) =>
                    setState(() => _selectedRuntimeId = runtimeId),
              );
              final detail = _RuntimeDetailPanel(
                snapshot: snapshot,
                runtime: selectedRuntime ?? snapshot.runtimes.first,
              );
              if (constraints.maxWidth < 980) {
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [table, const SizedBox(height: 12), detail],
                );
              }
              return Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(flex: 3, child: table),
                  const SizedBox(width: 12),
                  Expanded(flex: 2, child: detail),
                ],
              );
            },
          ),
        for (final adapter in snapshot.adapterCatalog)
          SectionList(
            title: 'アダプター台帳: ${adapter.adapterId}',
            rows: [
              '実行系: ${adapter.runtimeId}',
              '発行者: ${adapter.publisher}',
              '版: ${adapter.version}',
              '署名: ${adapter.signature}',
              'ハッシュ: ${adapter.hash}',
              '信頼: ${adapter.trustStatus}',
              '要求能力: ${adapter.requestedCapabilities.join(', ')}',
              '付与能力: ${adapter.grantedCapabilities.join(', ')}',
              '拒否能力: ${adapter.deniedCapabilities.join(', ')}',
              '既知の危険: ${adapter.knownRisks.join(', ')}',
            ],
          ),
        for (final diff in snapshot.permissionDiffs)
          SectionList(
            title: '許可差分: ${diff.subject}',
            rows: [
              for (final item in diff.added) '+ $item',
              for (final item in diff.removed) '- $item',
              for (final item in diff.changed) '~ $item',
              for (final item in diff.dangerous) '! $item',
            ],
          ),
        SectionList(
          title: '実行系の権限経路',
          rows: [
            for (final item in snapshot.authorityMap)
              '${item.runtimeId} -> ${item.capabilityId} -> ${item.permissionId} -> ${item.approvalId} -> ${item.auditEventId} -> ${item.recoveryId}',
          ],
        ),
        const SectionList(
          title: '権限境界',
          rows: [
            '実行系の能力、許可、承認、監査、復旧の判断はShell Coreが保持します。',
            'Flutterはブローカー経由の権限状態または診断専用のローカルデータを表示するだけで、権限の付与、承認、変更を行いません。',
          ],
        ),
      ],
    );
  }

  RuntimeRecord? _selectedRuntime(ShellSnapshot snapshot) {
    if (snapshot.runtimes.isEmpty) {
      return null;
    }
    return snapshot.runtimes.firstWhere(
      (runtime) => runtime.runtimeId == _selectedRuntimeId,
      orElse: () => snapshot.runtimes.first,
    );
  }
}

class _RuntimeTable extends StatelessWidget {
  const _RuntimeTable({
    required this.snapshot,
    required this.selectedRuntimeId,
    required this.onSelected,
  });

  final ShellSnapshot snapshot;
  final String selectedRuntimeId;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: DataTable(
        columns: const [
          DataColumn(label: Text('実行系')),
          DataColumn(label: Text('状態')),
          DataColumn(label: Text('アダプター')),
          DataColumn(label: Text('診断')),
        ],
        rows: [
          for (final runtime in snapshot.runtimes)
            DataRow(
              selected: runtime.runtimeId == selectedRuntimeId,
              onSelectChanged: (_) => onSelected(runtime.runtimeId),
              cells: [
                DataCell(Text(runtime.name)),
                DataCell(Text(runtime.status)),
                DataCell(Text(runtime.adapterId)),
                DataCell(Text(runtime.diagnosticSummary)),
              ],
            ),
        ],
      ),
    );
  }
}

class _RuntimeDetailPanel extends StatelessWidget {
  const _RuntimeDetailPanel({required this.snapshot, required this.runtime});

  final ShellSnapshot snapshot;
  final RuntimeRecord runtime;

  @override
  Widget build(BuildContext context) {
    final adapter = _adapterForRuntime(snapshot, runtime.runtimeId);
    final authority = snapshot.authorityMap
        .where((item) => item.runtimeId == runtime.runtimeId)
        .toList();
    final relatedProblems = snapshot.problems
        .where(
          (problem) =>
              problem.target.contains(runtime.runtimeId) ||
              authority.any((item) => item.recoveryId == problem.recoveryId),
        )
        .toList();
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('実行系の詳細', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Text('実行系ID: ${runtime.runtimeId}'),
          Text('状態: ${runtime.status}'),
          Text('アダプター: ${runtime.adapterId}'),
          Text('スナップショット出所: ${snapshot.snapshotSource}'),
          Text('ネットワーク公開範囲: ${snapshot.networkExposure}'),
          const Divider(),
          Text('能力: ${authority.map((item) => item.capabilityId).join(', ')}'),
          Text('許可: ${authority.map((item) => item.permissionId).join(', ')}'),
          Text('承認: ${authority.map((item) => item.approvalId).join(', ')}'),
          Text(
            '直近監査: ${authority.map((item) => item.auditEventId).join(', ')}',
          ),
          Text('関連復旧: ${authority.map((item) => item.recoveryId).join(', ')}'),
          if (adapter != null) ...[
            const Divider(),
            Text('アダプター信頼: ${adapter.trustStatus}'),
            Text('要求能力: ${adapter.requestedCapabilities.join(', ')}'),
            Text('付与能力: ${adapter.grantedCapabilities.join(', ')}'),
            Text('拒否能力: ${adapter.deniedCapabilities.join(', ')}'),
          ],
          const Divider(),
          if (relatedProblems.isEmpty)
            const Text('関連問題: なし')
          else
            for (final problem in relatedProblems)
              Text('関連問題: ${problem.item} → ${problem.recoveryId}'),
        ],
      ),
    );
  }

  AdapterCatalogRecord? _adapterForRuntime(
    ShellSnapshot snapshot,
    String runtimeId,
  ) {
    for (final adapter in snapshot.adapterCatalog) {
      if (adapter.runtimeId == runtimeId) {
        return adapter;
      }
    }
    return null;
  }
}
