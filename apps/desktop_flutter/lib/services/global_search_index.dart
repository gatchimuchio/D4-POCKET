import '../models/generated_contracts.dart';
import 'broker_client.dart';
import 'host_registration_client.dart';
import 'workspace_client.dart';

enum GlobalSearchTarget { host, workspace, grantedAction, navigation }

class GlobalSearchResult {
  const GlobalSearchResult({
    required this.target,
    required this.category,
    required this.title,
    required this.detail,
    required this.recordId,
    required this.pageIndex,
  });

  final GlobalSearchTarget target;
  final String category;
  final String title;
  final String detail;
  final String recordId;
  final int pageIndex;
  String get evidenceSource => 'INTERNAL_STATE';
  String get sourceLabel => target == GlobalSearchTarget.navigation
      ? '固定画面案内'
      : evidenceSource;

  bool matches(String query) {
    final normalized = query.trim().toLowerCase();
    return normalized.isEmpty ||
        '$category $title $detail $recordId'.toLowerCase().contains(normalized);
  }
}

/// Search results are rebuilt from current Broker lists each time a surface opens.
/// They contain only Host, Workspace, and currently valid Workspace read grants.
class GlobalSearchIndex {
  static const maxEntries = 96; // 64 Hosts + 16 Workspaces + 16 grants.
  static const maxQueryLength = 128;
  static const maxResults = 30;

  static Future<List<GlobalSearchResult>> loadCurrent(
    BrokerTransport broker, {
    DateTime? now,
  }) async {
    final hostClient = HostRegistrationClient(broker);
    final workspaceClient = WorkspaceClient(broker);
    final current = await Future.wait<Object>([
      hostClient.refresh(),
      workspaceClient.list(),
    ]);
    return build(
      hosts: current[0] as List<HostRegistryRecord>,
      workspaces: current[1] as List<WorkspaceRegistration>,
      now: now ?? DateTime.now(),
    );
  }

  /// The inputs must come from the validating HostRegistrationClient and
  /// WorkspaceClient. UI production callers use [loadCurrent].
  static List<GlobalSearchResult> build({
    required List<HostRegistryRecord> hosts,
    required List<WorkspaceRegistration> workspaces,
    required DateTime now,
  }) {
    final results = <GlobalSearchResult>[];
    for (final host in hosts.take(64)) {
      final hostId = _safeLabel(host.hostId, 128);
      if (hostId.isEmpty) continue;
      final displayName = _safeLabel(host.displayName, 128);
      final platform = _safeLabel(host.platform, 32);
      final connection = _safeLabel(host.connectionState, 48);
      final trust = _safeLabel(host.trustState, 48);
      results.add(GlobalSearchResult(
        target: GlobalSearchTarget.host,
        category: 'Host',
        title: displayName.isEmpty ? hostId : displayName,
        detail: [
          if (platform.isNotEmpty) platform,
          if (connection.isNotEmpty) '接続=$connection',
          if (trust.isNotEmpty) 'Trust=$trust',
          'metadata-only',
        ].join(' / '),
        recordId: hostId,
        pageIndex: 19,
      ));
    }

    for (final workspace in workspaces.take(16)) {
      results.add(GlobalSearchResult(
        target: GlobalSearchTarget.workspace,
        category: 'Workspace',
        title: workspace.id,
        detail: [
          'Runtime=${workspace.runtime}',
          '読取範囲=${workspace.visibility}',
          if (workspace.current(now)) '承認中',
          if (workspace.approval == null) '読取未承認',
        ].join(' / '),
        recordId: workspace.id,
        pageIndex: 5,
      ));

      if (workspace.current(now) && workspace.visibility != 'none') {
        results.add(GlobalSearchResult(
          target: GlobalSearchTarget.grantedAction,
          category: '許可済み操作',
          title: 'Workspace読取',
          detail: [
            workspace.id,
            'Runtime=${workspace.runtime}',
            '表示範囲=${workspace.visibility}',
            '期限=${DateTime.fromMillisecondsSinceEpoch(workspace.expires! * 1000, isUtc: true).toIso8601String()}',
            '選択時は画面移動のみ',
          ].join(' / '),
          recordId: workspace.id,
          pageIndex: 5,
        ));
      }
    }

    return List.unmodifiable(results.take(maxEntries));
  }

  static List<GlobalSearchResult> search(
    List<GlobalSearchResult> entries,
    String query,
  ) {
    if (query.length > maxQueryLength) return const [];
    return entries
        .where((result) => result.matches(query))
        .take(maxResults)
        .toList(growable: false);
  }
}

String _safeLabel(String value, int limit) => String.fromCharCodes(
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
