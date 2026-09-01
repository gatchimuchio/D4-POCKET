import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';

const Map<String, String> _surfaceSemanticsIds = {
  'Dashboard': 'gui_shell.surface.dashboard',
  'NavigationRail': 'gui_shell.surface.navigation_rail',
  'Runtime Status': 'gui_shell.surface.runtime_status',
  'Invariant Status': 'gui_shell.surface.invariant_status',
};

String surfaceSemanticsIdentifier(String evidenceLabel) {
  final mapped = _surfaceSemanticsIds[evidenceLabel];
  if (mapped != null) {
    return mapped;
  }
  final normalized = evidenceLabel
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

  static void record(String evidenceLabel) {
    _observed[evidenceLabel] = surfaceSemanticsIdentifier(evidenceLabel);
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
    this.evidenceLabel,
    this.value,
    this.header = false,
    this.explicitChildNodes = false,
    this.excludeSemantics = false,
  });

  final String label;
  final String? evidenceLabel;
  final String? value;
  final bool header;
  final bool explicitChildNodes;
  final bool excludeSemantics;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final stableEvidenceLabel = evidenceLabel ?? label;
    SurfaceSemanticsRegistry.record(stableEvidenceLabel);
    return Semantics(
      identifier: surfaceSemanticsIdentifier(stableEvidenceLabel),
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
  const ShellPage({
    super.key,
    required this.title,
    required this.children,
    this.evidenceTitle,
  });

  final String title;
  final String? evidenceTitle;
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
              evidenceLabel: evidenceTitle,
              header: true,
              excludeSemantics: true,
              child: Text(
                title,
                style: Theme.of(context).textTheme.headlineSmall,
              ),
            ),
            const SizedBox(height: 16),
            ...children.map(
              (child) => Padding(
                padding: const EdgeInsets.only(bottom: 16),
                child: child,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class MetricItem {
  const MetricItem({
    required this.label,
    required this.value,
    this.evidenceLabel,
  });

  final String label;
  final String value;
  final String? evidenceLabel;
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
            evidenceLabel: item.evidenceLabel,
            value: item.value,
            excludeSemantics: true,
            child: SizedBox(
              width: 150,
              child: BorderedPanel(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      item.value,
                      style: Theme.of(context).textTheme.headlineMedium,
                    ),
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
    final staleLabel = snapshotIsStale(snapshot) ? '期限切れ' : '最新';
    return Material(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      child: SizedBox(
        height: 44,
        child: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
          child: Row(
            children: [
              const StatusPill(label: '段階', value: 'B 所有者利用'),
              const SizedBox(width: 8),
              StatusPill(label: '実行系', value: operation.runtimeStatus),
              const SizedBox(width: 8),
              StatusPill(
                label: '信頼／不変条件',
                value: '${operation.trustStatus}/${operation.invariantStatus}',
              ),
              const SizedBox(width: 8),
              StatusPill(
                label: '承認',
                value: '${operation.pendingApprovalsCount}件保留',
              ),
              const SizedBox(width: 8),
              StatusPill(label: '監査', value: operation.auditChainStatus),
              const SizedBox(width: 8),
              StatusPill(label: '問題', value: '${operation.problemsCount}'),
              const SizedBox(width: 8),
              StatusPill(label: 'リリース', value: operation.releaseState),
              const SizedBox(width: 8),
              StatusPill(label: 'スナップショット', value: snapshot.snapshotSource),
              const SizedBox(width: 8),
              StatusPill(label: '経過時間', value: ageLabel),
              const SizedBox(width: 8),
              StatusPill(label: '鮮度', value: staleLabel),
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
          'RustブローカーIPCを利用できないか、接続が拒否されました。権限作用は停止され、ローカルスナップショットは権限根拠になりません。';
    } else if (stale) {
      text =
          '代替または期限切れの診断スナップショットを表示しています。段階Bの所有者利用は継続できますが、現在状態が必要なときは診断を更新してください。';
    } else if (strictBlocked) {
      text = '所有者利用は可能です。厳格リリースは段階Dの証拠または所有者GOがないため、引き続き遮断されています。';
    } else {
      text = '所有者利用状態は最新です。完成製品としてのリリースはまだ主張しません。';
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
          Text('意味: $meaning'),
          Text('段階Bの所有者利用を遮断: ${phaseBBlocked ? 'はい' : 'いいえ'}'),
          Text('次の対応: $nextAction'),
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
          Text('スナップショットの鮮度', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              StatusPill(label: '出所', value: snapshot.snapshotSource),
              StatusPill(label: '生成日時', value: _shortSnapshotTime(snapshot)),
              StatusPill(label: '経過時間', value: snapshotAgeLabel(snapshot)),
              StatusPill(
                label: '警告',
                value: brokerUnavailable
                    ? 'ブローカー利用不可'
                    : stale
                        ? '期限切れ／代替'
                        : 'なし',
              ),
              StatusPill(
                label: 'リリース',
                value: snapshot.operationStatus.releaseState,
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            'パス: ${snapshot.snapshotPath.isEmpty ? '(記録なし)' : snapshot.snapshotPath}',
          ),
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
              child: Text(row),
            ),
        ],
      ),
    );
  }
}

String snapshotAgeLabel(ShellSnapshot snapshot) {
  final generatedAt = _snapshotDate(snapshot);
  if (generatedAt == null) {
    return snapshot.snapshotFreshness.isEmpty
        ? '不明'
        : _localizedSnapshotMarker(snapshot.snapshotFreshness);
  }
  final age = DateTime.now().toUtc().difference(generatedAt.toUtc());
  if (age.inMinutes < 1) {
    return 'たった今';
  }
  if (age.inHours < 1) {
    return '${age.inMinutes}分';
  }
  if (age.inDays < 1) {
    return '${age.inHours}時間';
  }
  return '${age.inDays}日';
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
    return 'ブローカーを介する権限経路は停止中です。ローカルスナップショットは診断用途に限られます。';
  }
  if (stale) {
    return '所有者利用は継続できますが、現在のローカル状態が必要なときは診断スナップショットを更新してください。';
  }
  return '段階Bの所有者利用表示に必要な鮮度を満たしています。';
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
    return '不明';
  }
  final parsed = DateTime.tryParse(value);
  if (parsed == null) {
    return _localizedSnapshotMarker(value);
  }
  return parsed.toLocal().toIso8601String();
}

String _localizedSnapshotMarker(String value) {
  switch (value.toLowerCase()) {
    case 'missing':
      return '未取得';
    case 'parse failed':
      return '解析失敗';
    case 'unavailable':
      return '利用不可';
    case 'static':
      return '固定値';
    case 'generated':
      return '生成済み';
    case 'unknown':
      return '不明';
    default:
      return value;
  }
}
