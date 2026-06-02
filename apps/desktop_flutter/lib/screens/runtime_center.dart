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
      title: 'Runtime Center',
      children: [
        if (snapshot.runtimes.isEmpty)
          const EmptyStatePanel(
            title: 'No runtime connected',
            meaning: 'The current snapshot has no runtime records.',
            phaseBBlocked: false,
            nextAction:
                'Reconnect the broker product path or refresh diagnostics after runtime discovery completes.',
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
                  children: [
                    table,
                    const SizedBox(height: 12),
                    detail,
                  ],
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
            title: 'Adapter Catalog: ${adapter.adapterId}',
            rows: [
              'runtime: ${adapter.runtimeId}',
              'publisher: ${adapter.publisher}',
              'version: ${adapter.version}',
              'signature: ${adapter.signature}',
              'hash: ${adapter.hash}',
              'trust: ${adapter.trustStatus}',
              'requested: ${adapter.requestedCapabilities.join(', ')}',
              'granted: ${adapter.grantedCapabilities.join(', ')}',
              'denied: ${adapter.deniedCapabilities.join(', ')}',
              'risks: ${adapter.knownRisks.join(', ')}',
            ],
          ),
        for (final diff in snapshot.permissionDiffs)
          SectionList(
            title: 'Permission Diff: ${diff.subject}',
            rows: [
              for (final item in diff.added) '+ $item',
              for (final item in diff.removed) '- $item',
              for (final item in diff.changed) '~ $item',
              for (final item in diff.dangerous) '! $item',
            ],
          ),
        SectionList(
          title: 'Runtime Authority Flow',
          rows: [
            for (final item in snapshot.authorityMap)
              '${item.runtimeId} -> ${item.capabilityId} -> ${item.permissionId} -> ${item.approvalId} -> ${item.auditEventId} -> ${item.recoveryId}',
          ],
        ),
        const SectionList(
          title: 'Authority Boundary',
          rows: [
            'Runtime capability, permission, approval, audit, and recovery decisions remain Shell Core owned.',
            'Flutter displays broker-mediated authority state or diagnostic-only local data and does not grant, approve, or mutate authority.',
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
          DataColumn(label: Text('Runtime')),
          DataColumn(label: Text('Status')),
          DataColumn(label: Text('Adapter')),
          DataColumn(label: Text('Diagnostics')),
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
        .where((problem) =>
            problem.target.contains(runtime.runtimeId) ||
            authority.any((item) => item.recoveryId == problem.recoveryId))
        .toList();
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Runtime Detail',
              style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Text('runtime_id: ${runtime.runtimeId}'),
          Text('status: ${runtime.status}'),
          Text('adapter: ${runtime.adapterId}'),
          Text('snapshot_source: ${snapshot.snapshotSource}'),
          Text('network_exposure: ${snapshot.networkExposure}'),
          const Divider(),
          Text(
              'capabilities: ${authority.map((item) => item.capabilityId).join(', ')}'),
          Text(
              'permissions: ${authority.map((item) => item.permissionId).join(', ')}'),
          Text(
              'approvals: ${authority.map((item) => item.approvalId).join(', ')}'),
          Text(
              'last_audit: ${authority.map((item) => item.auditEventId).join(', ')}'),
          Text(
              'related_recovery: ${authority.map((item) => item.recoveryId).join(', ')}'),
          if (adapter != null) ...[
            const Divider(),
            Text('adapter_trust: ${adapter.trustStatus}'),
            Text('requested: ${adapter.requestedCapabilities.join(', ')}'),
            Text('granted: ${adapter.grantedCapabilities.join(', ')}'),
            Text('denied: ${adapter.deniedCapabilities.join(', ')}'),
          ],
          const Divider(),
          if (relatedProblems.isEmpty)
            const Text('related_problems: none')
          else
            for (final problem in relatedProblems)
              Text('related_problem: ${problem.item} -> ${problem.recoveryId}'),
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
