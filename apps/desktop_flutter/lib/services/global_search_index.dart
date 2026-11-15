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
    for (final host in snapshot.hostCapabilities) {
      final hostId = _safeUntrustedLabel(host.hostId, 128);
      if (hostId.isEmpty) continue;
      final displayName = _safeUntrustedLabel(host.displayName, 128);
      final platform = _safeUntrustedLabel(host.platform, 48);
      final hostStatus = _safeUntrustedLabel(host.status, 48);
      for (final capability in host.capabilities.take(16)) {
        if (results.length >= maxEntries) break;
        final capabilityId = _safeUntrustedLabel(capability.capabilityId, 96);
        if (capabilityId.isEmpty) continue;
        final capabilityStatus = _safeUntrustedLabel(capability.status, 48);
        add(
          category: 'Host能力',
          title: capabilityId,
          detail: [
            if (displayName.isNotEmpty) displayName,
            if (platform.isNotEmpty) platform,
            if (hostStatus.isNotEmpty) 'Host status=$hostStatus',
            if (capabilityStatus.isNotEmpty)
              'capability status=$capabilityStatus',
            '能力metadataはPermission／Authorityではありません',
          ].join(' / '),
          recordId: hostId,
          pageIndex: 19,
        );
      }
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
    for (final adapter in snapshot.agentAdapters) {
      final adapterId = _safeUntrustedLabel(adapter.adapterId, 128);
      final agentId = _safeUntrustedLabel(adapter.agentId, 128);
      final recordId = adapterId.isNotEmpty ? adapterId : agentId;
      if (recordId.isEmpty) continue;
      final provider = _safeUntrustedLabel(adapter.provider, 96);
      final model = _safeUntrustedLabel(adapter.model, 128);
      final version = _safeUntrustedLabel(adapter.version, 64);
      final status = _safeUntrustedLabel(adapter.status, 48);
      final capabilityIds = adapter.capabilities
          .take(16)
          .map((capability) => _safeUntrustedLabel(capability.capabilityId, 96))
          .where((capabilityId) => capabilityId.isNotEmpty)
          .toList(growable: false);
      final title = agentId.isNotEmpty ? agentId : adapterId;
      final providerModel =
          [provider, model].where((value) => value.isNotEmpty);
      add(
        category: 'Agent Adapter',
        title: title,
        detail: [
          ...providerModel,
          if (version.isNotEmpty) 'version=$version',
          if (status.isNotEmpty) 'status=$status',
          if (capabilityIds.isNotEmpty) '宣言能力=${capabilityIds.join(', ')}',
          '能力metadataは権限ではありません',
        ].join(' / '),
        recordId: recordId,
        pageIndex: 5,
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
      ('構成', 'GUI Shell構成', 'Manifest作成・Previewの設定面', 'compose-surface', 11),
      (
        '書出し',
        'GUI Shell独立App書出し',
        'Windows／Linux／macOS向けManifest file作成の設定面',
        'export-surface',
        11
      ),
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
    add(
      category: 'A2A',
      title: 'A2A接続',
      detail: 'loopback Agent CardのBroker接続面',
      recordId: 'a2a-surface',
      pageIndex: 20,
    );
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

String _safeUntrustedLabel(String value, int limit) => String.fromCharCodes(
      value.runes.where((rune) {
        final control = rune < 0x20 || (rune >= 0x7f && rune <= 0x9f);
        final directionalOrInvisible = rune == 0x061c ||
            (rune >= 0x200b && rune <= 0x200f) ||
            rune == 0x2028 ||
            rune == 0x2029 ||
            (rune >= 0x202a && rune <= 0x202e) ||
            (rune >= 0x2066 && rune <= 0x2069) ||
            rune == 0xfeff;
        return !control && !directionalOrInvisible;
      }).take(limit),
    ).trim();
