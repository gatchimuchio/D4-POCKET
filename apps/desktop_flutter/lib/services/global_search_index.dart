import '../models/generated_contracts.dart';

class GlobalSearchResult {
  const GlobalSearchResult({
    required this.category,
    required this.title,
    required this.detail,
    required this.recordId,
    required this.pageIndex,
    required this.evidenceSource,
  });

  final String category;
  final String title;
  final String detail;
  final String recordId;
  final int pageIndex;
  final String evidenceSource;

  bool matches(String query) {
    final normalized = query.trim().toLowerCase();
    return normalized.isEmpty ||
        '$category $title $detail $recordId'.toLowerCase().contains(normalized);
  }
}

class GlobalSearchIndex {
  static const maxEntries = 512;
  static const maxQueryLength = 128;
  static const maxResults = 30;

  static List<GlobalSearchResult> build(ShellSnapshot snapshot) {
    final results = <GlobalSearchResult>[];
    final evidenceSource =
        snapshot.snapshotSource == 'broker' ? 'INTERNAL_STATE' : '不明';

    void add({
      required String category,
      required String title,
      required String detail,
      required String recordId,
      required int pageIndex,
    }) {
      if (results.length >= maxEntries) return;
      results.add(
        GlobalSearchResult(
          category: _bounded(category, 64),
          title: _bounded(title, 160),
          detail: _bounded(detail, 256),
          recordId: _bounded(recordId, 160),
          pageIndex: pageIndex,
          evidenceSource: evidenceSource,
        ),
      );
    }

    for (final runtime in snapshot.runtimes) {
      add(
        category: '実行系',
        title: runtime.runtimeId,
        detail: '${runtime.name} / ${runtime.status} / ${runtime.adapterId}',
        recordId: runtime.runtimeId,
        pageIndex: 3,
      );
    }
    for (final session in snapshot.agentSessions) {
      add(
        category: '対話セッション',
        title: session.sessionId,
        detail: '対話セッション / 試験状態: ${session.testStatus}',
        recordId: session.sessionId,
        pageIndex: 12,
      );
    }
    for (final permission in snapshot.permissions) {
      add(
        category: '権限',
        title: permission.permissionId,
        detail: '${permission.capabilityId} / ${permission.decision}',
        recordId: permission.permissionId,
        pageIndex: 4,
      );
    }
    for (final approval in snapshot.pendingApprovals) {
      add(
        category: '承認',
        title: approval.approvalId,
        detail:
            '${approval.operation} / ${approval.status} / ${approval.contentVisibility}',
        recordId: approval.approvalId,
        pageIndex: 6,
      );
    }
    for (final audit in snapshot.auditEvents) {
      add(
        category: '監査',
        title: audit.eventId,
        detail: '${audit.action} / ${audit.result} / 監査内容ハッシュ',
        recordId: audit.eventId,
        pageIndex: 7,
      );
    }
    for (final recovery in snapshot.recoveryActions) {
      add(
        category: '復旧',
        title: recovery.recoveryId,
        detail: '${recovery.severity} / ${recovery.message}',
        recordId: recovery.recoveryId,
        pageIndex: 8,
      );
    }
    for (final problem in snapshot.problems) {
      add(
        category: '問題',
        title: problem.item.isEmpty ? problem.problemId : problem.item,
        detail: '${problem.category} / ${problem.classification}',
        recordId: problem.problemId,
        pageIndex: 9,
      );
    }
    for (final evidence in snapshot.evidence) {
      add(
        category: '証拠',
        title: evidence.evidenceId,
        detail: '${evidence.kind} / ${evidence.status}',
        recordId: evidence.evidenceId,
        pageIndex: 10,
      );
    }
    for (final host in snapshot.hosts) {
      add(
        category: '接続先',
        title: host.displayName.isEmpty ? host.hostId : host.displayName,
        detail:
            '${host.platform} / ${host.connectionState} / ${host.trustState}',
        recordId: host.hostId,
        pageIndex: 19,
      );
    }
    for (final adapter in snapshot.adapterCatalog) {
      add(
        category: 'アダプター',
        title: adapter.adapterId,
        detail:
            '${adapter.publisher} / ${adapter.version} / ${adapter.trustStatus}',
        recordId: adapter.adapterId,
        pageIndex: 3,
      );
    }
    for (final setting in snapshot.settings) {
      add(
        category: 'プロファイル',
        title: setting.key,
        detail: setting.group,
        recordId: setting.key,
        pageIndex: 11,
      );
    }
    for (final item in snapshot.authorityMap) {
      add(
        category: '権限対応図',
        title: item.runtimeId,
        detail: '${item.capabilityId} / 表示専用',
        recordId: item.runtimeId,
        pageIndex: 4,
      );
    }

    for (final surface in const [
      ('エージェント', 'エージェントセンター', 'エージェントの表示面', 'agent-surface', 5),
      ('MCP', 'MCP接続', '設定のMCP接続面', 'mcp-surface', 11),
      ('A2A', 'A2A接続', 'エージェントセンターの接続面', 'a2a-surface', 5),
      ('評価', '評価ラボ', '評価の表示面', 'evaluation-surface', 14),
      ('通知', '通知センター', '通知の表示面', 'notification-surface', 16),
    ]) {
      add(
        category: surface.$1,
        title: surface.$2,
        detail: surface.$3,
        recordId: surface.$4,
        pageIndex: surface.$5,
      );
    }
    return List.unmodifiable(results);
  }

  static List<GlobalSearchResult> search(
    ShellSnapshot snapshot,
    String query,
  ) {
    if (query.length > maxQueryLength) return const [];
    return build(snapshot)
        .where((result) => result.matches(query))
        .take(maxResults)
        .toList(growable: false);
  }
}

String _bounded(String value, int limit) {
  if (value.length <= limit) return value;
  return value.substring(0, limit);
}
