import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';

const Map<String, String> _surfaceSemanticsIds = {
  'Dashboard': 'gui_shell.surface.dashboard',
  'NavigationRail': 'gui_shell.surface.navigation_rail',
  'Runtime Status': 'gui_shell.surface.runtime_status',
  'Invariant Status': 'gui_shell.surface.invariant_status',
};

String surfaceSemanticsIdentifier(String label) {
  final mapped = _surfaceSemanticsIds[label];
  if (mapped != null) {
    return mapped;
  }
  final normalized = label
      .trim()
      .toLowerCase()
      .replaceAll(RegExp(r'[^a-z0-9]+'), '_')
      .replaceAll(RegExp(r'^_+|_+$'), '');
  return 'gui_shell.surface.$normalized';
}

class SurfaceSemanticsRegistry {
  SurfaceSemanticsRegistry._();

  static final Map<String, String> _observed = <String, String>{};

  static Map<String, String> get observed => Map.unmodifiable(_observed);

  static void record(String label) {
    _observed[label] = surfaceSemanticsIdentifier(label);
  }

  static void resetForTest() {
    _observed.clear();
  }
}

class SurfaceSemantics extends StatelessWidget {
  const SurfaceSemantics({
    super.key,
    required this.label,
    required this.child,
    this.value,
    this.header = false,
    this.explicitChildNodes = false,
    this.excludeSemantics = false,
  });

  final String label;
  final String? value;
  final bool header;
  final bool explicitChildNodes;
  final bool excludeSemantics;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    SurfaceSemanticsRegistry.record(label);
    return Semantics(
      identifier: surfaceSemanticsIdentifier(label),
      label: label,
      value: value,
      header: header,
      container: true,
      explicitChildNodes: explicitChildNodes,
      excludeSemantics: excludeSemantics,
      child: child,
    );
  }
}

class ShellPage extends StatelessWidget {
  const ShellPage({super.key, required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SurfaceSemantics(
              label: title,
              header: true,
              excludeSemantics: true,
              child:
                  Text(title, style: Theme.of(context).textTheme.headlineSmall),
            ),
            const SizedBox(height: 16),
            ...children.map((child) => Padding(
                padding: const EdgeInsets.only(bottom: 16), child: child)),
          ],
        ),
      ),
    );
  }
}

class MetricItem {
  const MetricItem({required this.label, required this.value});

  final String label;
  final String value;
}

class MetricRow extends StatelessWidget {
  const MetricRow({super.key, required this.items});

  final List<MetricItem> items;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 12,
      runSpacing: 12,
      children: [
        for (final item in items)
          SurfaceSemantics(
            label: item.label,
            value: item.value,
            excludeSemantics: true,
            child: SizedBox(
              width: 150,
              child: BorderedPanel(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(item.value,
                        style: Theme.of(context).textTheme.headlineMedium),
                    Text(item.label),
                  ],
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class BorderedPanel extends StatelessWidget {
  const BorderedPanel({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: Theme.of(context).colorScheme.outlineVariant),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Padding(padding: const EdgeInsets.all(12), child: child),
    );
  }
}

class StatusPill extends StatelessWidget {
  const StatusPill({super.key, required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Chip(
      label: Text('$label: $value'),
      visualDensity: VisualDensity.compact,
    );
  }
}

class ShellStatusBar extends StatelessWidget {
  const ShellStatusBar({super.key, required this.snapshot});

  final ShellSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    final operation = snapshot.operationStatus;
    final ageLabel = snapshotAgeLabel(snapshot);
    final staleLabel = snapshotIsStale(snapshot) ? 'stale' : 'fresh';
    return Material(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      child: SizedBox(
        height: 44,
        child: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
          child: Row(
            children: [
              const StatusPill(label: 'Phase', value: 'B owner-use'),
              const SizedBox(width: 8),
              StatusPill(label: 'Runtime', value: operation.runtimeStatus),
              const SizedBox(width: 8),
              StatusPill(
                  label: 'Trust/Invariants',
                  value:
                      '${operation.trustStatus}/${operation.invariantStatus}'),
              const SizedBox(width: 8),
              StatusPill(
                  label: 'Approvals',
                  value: '${operation.pendingApprovalsCount} pending'),
              const SizedBox(width: 8),
              StatusPill(label: 'Audit', value: operation.auditChainStatus),
              const SizedBox(width: 8),
              StatusPill(
                  label: 'Problems', value: '${operation.problemsCount}'),
              const SizedBox(width: 8),
              StatusPill(label: 'Release', value: operation.releaseState),
              const SizedBox(width: 8),
              StatusPill(label: 'Snapshot', value: snapshot.snapshotSource),
              const SizedBox(width: 8),
              StatusPill(label: 'Age', value: ageLabel),
              const SizedBox(width: 8),
              StatusPill(label: 'Freshness', value: staleLabel),
            ],
          ),
        ),
      ),
    );
  }
}

class PhaseBanner extends StatelessWidget {
  const PhaseBanner({super.key, required this.snapshot});

  final ShellSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    final stale = snapshotIsStale(snapshot);
    final strictBlocked =
        snapshot.evidenceSummary.missingMeasuredWindowsEvidence ||
            snapshot.evidenceSummary.missingSetupDoctorEvidence ||
            snapshot.evidenceSummary.ownerGo != 'recorded';
    final brokerUnavailable = snapshotIsBrokerUnavailable(snapshot);
    final colorScheme = Theme.of(context).colorScheme;
    final background = brokerUnavailable || strictBlocked || stale
        ? colorScheme.tertiaryContainer
        : colorScheme.secondaryContainer;
    final foreground = brokerUnavailable || strictBlocked || stale
        ? colorScheme.onTertiaryContainer
        : colorScheme.onSecondaryContainer;
    final String text;
    if (brokerUnavailable) {
      text =
          'Rust broker IPC is unavailable or rejected. Authority actions are suspended and local snapshots are not authority.';
    } else if (stale) {
      text =
          'Fallback or stale diagnostic snapshot is active. Phase B owner-use can continue; refresh diagnostics when current state matters.';
    } else if (strictBlocked) {
      text =
          'Owner-use is OK. Strict release remains blocked by Phase D evidence / owner GO.';
    } else {
      text =
          'Owner-use state is current. Completed product release is still not claimed.';
    }
    return Material(
      color: background,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          children: [
            Icon(Icons.info_outline, color: foreground, size: 20),
            const SizedBox(width: 8),
            Expanded(
              child: Text(text, style: TextStyle(color: foreground)),
            ),
          ],
        ),
      ),
    );
  }
}

class EmptyStatePanel extends StatelessWidget {
  const EmptyStatePanel({
    super.key,
    required this.title,
    required this.meaning,
    required this.phaseBBlocked,
    required this.nextAction,
  });

  final String title;
  final String meaning;
  final bool phaseBBlocked;
  final String nextAction;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                phaseBBlocked
                    ? Icons.report_problem_outlined
                    : Icons.check_circle_outline,
                size: 20,
              ),
              const SizedBox(width: 8),
              Text(title, style: Theme.of(context).textTheme.titleMedium),
            ],
          ),
          const SizedBox(height: 8),
          Text('Meaning: $meaning'),
          Text('Phase B owner-use blocked: ${phaseBBlocked ? 'yes' : 'no'}'),
          Text('Next: $nextAction'),
        ],
      ),
    );
  }
}

class SnapshotInfoPanel extends StatelessWidget {
  const SnapshotInfoPanel({super.key, required this.snapshot});

  final ShellSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    final stale = snapshotIsStale(snapshot);
    final brokerUnavailable = snapshotIsBrokerUnavailable(snapshot);
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Snapshot Freshness',
              style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              StatusPill(label: 'Source', value: snapshot.snapshotSource),
              StatusPill(
                  label: 'Generated', value: _shortSnapshotTime(snapshot)),
              StatusPill(label: 'Age', value: snapshotAgeLabel(snapshot)),
              StatusPill(
                  label: 'Warning',
                  value: brokerUnavailable
                      ? 'broker unavailable'
                      : stale
                          ? 'stale/fallback'
                          : 'none'),
              StatusPill(
                  label: 'Release',
                  value: snapshot.operationStatus.releaseState),
            ],
          ),
          const SizedBox(height: 8),
          Text(
              'Path: ${snapshot.snapshotPath.isEmpty ? '(not recorded)' : snapshot.snapshotPath}'),
          Text(_snapshotInfoMessage(brokerUnavailable, stale)),
        ],
      ),
    );
  }
}

class SectionList extends StatelessWidget {
  const SectionList({super.key, required this.title, required this.rows});

  final String title;
  final List<String> rows;

  @override
  Widget build(BuildContext context) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Semantics(
            label: title,
            header: true,
            child: Text(title, style: Theme.of(context).textTheme.titleMedium),
          ),
          const SizedBox(height: 8),
          for (final row in rows)
            Padding(
                padding: const EdgeInsets.symmetric(vertical: 2),
                child: Text(row)),
        ],
      ),
    );
  }
}

String snapshotAgeLabel(ShellSnapshot snapshot) {
  final generatedAt = _snapshotDate(snapshot);
  if (generatedAt == null) {
    return snapshot.snapshotFreshness.isEmpty
        ? 'unknown'
        : snapshot.snapshotFreshness;
  }
  final age = DateTime.now().toUtc().difference(generatedAt.toUtc());
  if (age.inMinutes < 1) {
    return 'just now';
  }
  if (age.inHours < 1) {
    return '${age.inMinutes}m';
  }
  if (age.inDays < 1) {
    return '${age.inHours}h';
  }
  return '${age.inDays}d';
}

bool snapshotIsStale(ShellSnapshot snapshot) {
  final source = snapshot.snapshotSource.toLowerCase();
  final freshness = snapshot.snapshotFreshness.toLowerCase();
  if (source == 'fallback' ||
      source == 'broker_unavailable' ||
      freshness == 'missing' ||
      freshness == 'parse failed' ||
      freshness == 'unavailable' ||
      freshness == 'static') {
    return true;
  }
  final generatedAt = _snapshotDate(snapshot);
  if (generatedAt == null) {
    return true;
  }
  return DateTime.now().toUtc().difference(generatedAt.toUtc()).inHours >= 24;
}

bool snapshotIsBrokerUnavailable(ShellSnapshot snapshot) {
  return snapshot.snapshotSource.toLowerCase() == 'broker_unavailable';
}

String _snapshotInfoMessage(bool brokerUnavailable, bool stale) {
  if (brokerUnavailable) {
    return 'Broker-mediated authority path is suspended; local snapshots are diagnostic-only.';
  }
  if (stale) {
    return 'Owner-use can continue, but refresh the diagnostic snapshot when current local state matters.';
  }
  return 'Snapshot is current enough for Phase B owner-use display.';
}

DateTime? _snapshotDate(ShellSnapshot snapshot) {
  for (final value in [
    snapshot.snapshotGeneratedAt,
    snapshot.snapshotFreshness,
  ]) {
    final parsed = DateTime.tryParse(value);
    if (parsed != null) {
      return parsed;
    }
  }
  return null;
}

String _shortSnapshotTime(ShellSnapshot snapshot) {
  final value = snapshot.snapshotGeneratedAt.isNotEmpty
      ? snapshot.snapshotGeneratedAt
      : snapshot.snapshotFreshness;
  if (value.isEmpty) {
    return 'unknown';
  }
  final parsed = DateTime.tryParse(value);
  if (parsed == null) {
    return value;
  }
  return parsed.toLocal().toIso8601String();
}
