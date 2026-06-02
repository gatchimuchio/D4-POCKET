import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import 'shared.dart';

class Dashboard extends StatelessWidget {
  const Dashboard({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final phase = snapshot.phaseStatus;
    final operation = snapshot.operationStatus;
    final evidence = snapshot.evidenceSummary;
    return ShellPage(
      title: 'Dashboard',
      children: [
        SectionList(
          title: 'Phase Status',
          rows: [
            'Phase A: ${phase.phaseAStatus}',
            'Phase B: ${phase.phaseBStatus}',
            'Phase C: ${phase.phaseCStatus}',
            'Phase D: ${phase.phaseDStatus}',
            'Phase E: ${phase.phaseEStatus}',
            'Phase F: ${phase.phaseFStatus}',
            'Owner-use operation: active',
            'Completed product release: ${phase.completedProductReleaseClaimed ? 'claimed' : 'not claimed'}',
            'Strict Windows installed-path evidence: pending',
          ],
        ),
        MetricRow(items: [
          MetricItem(label: 'Runtime Status', value: operation.runtimeStatus),
          MetricItem(
              label: 'Invariant Status', value: operation.invariantStatus),
          MetricItem(
              label: 'Pending Approvals',
              value: '${operation.pendingApprovalsCount}'),
          MetricItem(label: 'Problems', value: '${operation.problemsCount}'),
          MetricItem(label: 'Evidence', value: evidence.evidenceBundle),
          MetricItem(
              label: 'Recovery',
              value: '${snapshot.recoveryPlaybook.length} items'),
        ]),
        const SectionList(
          title: 'Owner Operation Boundary',
          rows: [
            'Phase B owner-use operation is complete.',
            'Completed product release is not claimed.',
            'Missing release evidence blocks completed product release, not Phase B owner-use operation.',
          ],
        ),
        SnapshotInfoPanel(snapshot: snapshot),
        if (snapshot.snapshotSource == 'fallback')
          const EmptyStatePanel(
            title: 'No local snapshot',
            meaning:
                'GUI-Shell is using the safe fallback projection instead of a local owner snapshot.',
            phaseBBlocked: false,
            nextAction:
                'Refresh the development diagnostic snapshot before local inspection.',
          ),
        SectionList(
          title: 'Trust Status',
          rows: [
            for (final trust in snapshot.trustRecords)
              '${trust.scope}: ${trust.state} (${trust.source})'
          ],
        ),
        SectionList(
          title: 'Problems / Blockers',
          rows: snapshot.problems.isEmpty
              ? [
                  'No problems are present in the current snapshot.',
                  'Phase B owner-use is not blocked.',
                  'Open Evidence Center or refresh diagnostics if local state changed.',
                ]
              : [
                  for (final problem in snapshot.problems)
                    '${problem.item}: ${problem.classification}; blocks_release: ${problem.blocksRelease ? 'yes' : 'no'}'
                ],
        ),
        SectionList(
          title: 'Evidence Summary',
          rows: [
            'schema_check: ${evidence.schemaCheck}',
            'conformance_skeleton: passed, ${evidence.conformanceCheckCount} checks',
            'release_smoke: ${evidence.releaseSmoke}',
            'release_gate_check: ${evidence.releaseGateCheck}',
            'evidence_bundle: ${evidence.evidenceBundle}',
            'validate_all: ${evidence.validateAll}',
            'strict_windows_release: ${evidence.strictWindowsRelease}',
          ],
        ),
        SectionList(
          title: 'Recent Audit Events',
          rows: [
            for (final event in snapshot.auditEvents)
              '${event.eventId}: ${event.action} ${event.result}'
          ],
        ),
        SectionList(
          title: 'Runtime Status',
          rows: [
            for (final runtime in snapshot.runtimes)
              '${runtime.name}  ${runtime.status}  ${runtime.adapterId}'
          ],
        ),
        SectionList(
          title: 'Invariant Status',
          rows: [
            for (final entry in snapshot.invariantFlags.entries)
              '${entry.key}: ${entry.value ? 'violation' : 'ok'}'
          ],
        ),
      ],
    );
  }
}
