import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class SetupDoctor extends StatefulWidget {
  const SetupDoctor({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<SetupDoctor> createState() => _SetupDoctorState();
}

class _SetupDoctorState extends State<SetupDoctor> {
  bool _isRefreshing = false;
  int _requestGeneration = 0;
  String? _refreshedStatus;
  List<SetupDoctorCheckRecord>? _refreshedChecks;
  String? _auditEventId;
  String? _reportId;
  String? _generatedAt;
  String? _refreshFailure;

  @override
  void didUpdateWidget(covariant SetupDoctor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.client, widget.client)) {
      _requestGeneration++;
      _isRefreshing = false;
      _refreshedStatus = null;
      _refreshedChecks = null;
      _auditEventId = null;
      _reportId = null;
      _generatedAt = null;
      _refreshFailure = null;
    }
  }

  Future<void> _refresh() async {
    final client = widget.client;
    final generation = ++_requestGeneration;
    setState(() => _isRefreshing = true);
    try {
      final report = await client.refreshSetupDoctorReport();
      if (!mounted || generation != _requestGeneration) return;
      setState(() {
        _refreshedStatus = report.status;
        _refreshedChecks = report.checks;
        _auditEventId = report.auditEventId;
        _reportId = report.reportId;
        _generatedAt = report.generatedAt;
        _refreshFailure = null;
      });
    } on Object {
      if (!mounted || generation != _requestGeneration) return;
      setState(() {
        _refreshedStatus = 'unknown';
        _refreshedChecks = const <SetupDoctorCheckRecord>[];
        _auditEventId = null;
        _reportId = null;
        _generatedAt = null;
        _refreshFailure = '最新の診断を取得できません。Broker接続を確認して再試行してください。';
      });
    } finally {
      if (mounted && generation == _requestGeneration) {
        setState(() => _isRefreshing = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final installedEvidence = snapshot.evidence
        .where((item) => item.kind == 'installed-path')
        .toList(growable: false);
    final checks = _refreshedChecks ?? snapshot.setupDoctorChecks;
    final status = _refreshedStatus ?? snapshot.setupDoctorStatus;
    final brokerMode = widget.client.mode == 'broker';
    final initialSource = switch (snapshot.snapshotSource) {
      'broker' => '起動時にBrokerから取得したSnapshot（更新操作まで古くなる場合があります）',
      'mock' => '試験用Snapshot（実Brokerの現在状態ではありません）',
      'in_memory_diagnostic' => 'メモリ内診断Snapshot（実Brokerの現在状態ではありません）',
      'fallback' => '固定代替Snapshot（実Brokerの現在状態ではありません）',
      _ => 'Broker Snapshotを確認できません',
    };

    return ShellPage(
      title: '環境診断',
      children: [
        _ReportSummary(status: status, checks: checks),
        Align(
          alignment: Alignment.centerLeft,
          child: OutlinedButton.icon(
            key: const ValueKey('setup-doctor-refresh'),
            onPressed: brokerMode && !_isRefreshing ? _refresh : null,
            icon: _isRefreshing
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.refresh),
            label: Text(_isRefreshing ? '取得中…' : 'Brokerから最新診断を取得'),
          ),
        ),
        SectionList(
          title: '診断データ',
          rows: [
            if (_refreshedStatus == null)
              '表示元: $initialSource'
            else if (_refreshFailure != null)
              '表示元: 最新のBroker取得に失敗（前回の診断結果は破棄しました）'
            else
              '表示元: 認証Brokerから再取得した診断報告',
            if (_reportId != null) '診断報告ID: $_reportId',
            if (_generatedAt != null) 'Broker報告時刻: $_generatedAt',
            if (_auditEventId != null) '監査イベントID: $_auditEventId',
            if (_refreshFailure != null) _refreshFailure!,
            'この再取得はSetup Doctor報告だけを更新します。下の他の状態一覧はSnapshotです。',
            if (!brokerMode) '最新状態の再取得には、製品Brokerへの認証済み接続が必要です。',
          ],
        ),
        const SectionList(
          title: 'この診断で分かること',
          rows: [
            '診断項目は起動時Snapshotまたは再取得したBroker報告を表示します。',
            '証拠種別は各項目の観測元を示し、製品全体の完成判定ではありません。',
            'この診断はPermissionやApprovalを作らず、製品リリースの完成判定にも使いません。',
          ],
        ),
        SectionList(
          title: 'Snapshotの実行状態',
          rows: [
            '取得元: ${_snapshotSourceLabel(snapshot.snapshotSource)}',
            '鮮度: ${_snapshotFreshnessLabel(snapshot.snapshotFreshness)}',
            '通信範囲: ${_networkExposureLabel(snapshot.networkExposure)}',
            'Broker監査鎖: ${_auditChainLabel(snapshot.auditChainStatus)}',
            '製品リリース状態: ${_releaseStateLabel(snapshot.operationStatus.releaseState)}',
          ],
        ),
        SectionList(
          title: '権限境界',
          rows: [
            snapshot.installerGrantsAuthority
                ? 'インストーラーによる権限付与が報告されています。権限依存操作を停止してください。'
                : 'インストーラーは権限を付与しません。',
            snapshot.installerSilentlyApprovesPermissions
                ? 'インストーラーによる自動承認が報告されています。権限依存操作を停止してください。'
                : 'インストーラーはPermissionを自動承認しません。',
          ],
        ),
        SectionList(
          title: '診断結果',
          rows: checks.isEmpty
              ? const ['診断項目を表示できません。上の取得状態と復旧案内を確認してください。']
              : ['${checks.length}項目を確認しました。'],
        ),
        for (final check in checks) _SetupDoctorCheckCard(check: check),
        SectionList(
          title: 'AI実行の要因分類',
          rows: _executionReadinessRows(
            snapshot,
            reportStatus: status,
            checks: checks,
          ),
        ),
        SectionList(
          title: 'インストール先の証拠',
          rows: installedEvidence.isEmpty
              ? const ['導入先を検証した証拠はありません。']
              : [
                  for (final evidence in installedEvidence)
                    '状態: ${_statusLabel(evidence.status)}',
                ],
        ),
        SectionList(
          title: '実行系接続',
          rows: snapshot.runtimes.isEmpty
              ? const ['接続済みの実行系はありません。']
              : [
                  for (final runtime in snapshot.runtimes)
                    '${runtime.runtimeId}: ${_runtimeStatusLabel(runtime.status)}',
                ],
        ),
        SectionList(
          title: '復旧手順',
          rows: snapshot.recoveryActions.isEmpty
              ? const ['現在、Brokerから追加の復旧案内はありません。']
              : [
                  for (final recovery in snapshot.recoveryActions)
                    recovery.message
                ],
        ),
      ],
    );
  }
}

List<String> _executionReadinessRows(
  ShellSnapshot snapshot, {
  required String reportStatus,
  required List<SetupDoctorCheckRecord> checks,
}) {
  final agents = snapshot.agentAdapters;
  final source =
      '${_snapshotSourceLabel(snapshot.snapshotSource)} / ${_snapshotFreshnessLabel(snapshot.snapshotFreshness)}';
  final runtimeUnavailable = agents
      .where((agent) =>
          agent.status == 'unavailable' ||
          agent.status == 'unsupported' ||
          agent.processSpawnStatus == 'unsupported')
      .length;
  final runtimeRow = agents.isEmpty
      ? 'Runtime/Agent: このSnapshotにBroker登録Agentなし。Runtime製品の導入有無は不明です。'
      : runtimeUnavailable > 0
          ? 'Runtime/Agent: $runtimeUnavailable件の登録metadataがunavailable／unsupportedです。実行fileの導入状態は別途未確認です。'
          : 'Runtime/Agent: ${agents.length}件の登録metadataがあります。実行fileの導入と現在の起動可否は未確認です。';

  final unsupportedAuthentication = agents
      .where((agent) => agent.authenticationStatus == 'unsupported')
      .length;
  final unknownAuthentication =
      agents.where((agent) => agent.authenticationStatus == 'unknown').length;
  final supportedAuthentication =
      agents.where((agent) => agent.authenticationStatus == 'supported').length;
  final authenticationDetails = <String>[
    if (unsupportedAuthentication > 0)
      '認証方式unsupported=$unsupportedAuthentication件',
    if (unknownAuthentication > 0) '認証状態unknown=$unknownAuthentication件',
    if (supportedAuthentication > 0)
      '認証方式supported宣言=$supportedAuthentication件（資格値の有効性は未検証）',
  ];
  final authenticationRow = agents.isEmpty
      ? '認証: Agent metadataがないためunknownです。資格情報の不足・失効も判定できません。'
      : '認証: ${authenticationDetails.join(' / ')}。秘密値は表示しません。';

  final deniedPermissions = snapshot.permissions.where((permission) {
    final decision = permission.decision.toLowerCase();
    return decision == 'deny' ||
        decision == 'denied' ||
        decision == 'rejected' ||
        decision == '拒否';
  }).length;
  final permissionRow = !snapshot.permissionInventoryKnown
      ? 'Permission: 現在の一覧が未観測のためunknownです。Task別の拒否状態もこのSnapshotからは分かりません。'
      : deniedPermissions > 0
          ? 'Permission: 現在の一覧に拒否記録が$deniedPermissions件あります。別Taskの許可へ転用しません。'
          : 'Permission: 現在の既知一覧に拒否記録はありません。TaskごとのApproval状態は別途確認が必要です。';

  final unsupportedTools =
      agents.where((agent) => agent.toolSupportStatus == 'unsupported').length;
  final unknownTools =
      agents.where((agent) => agent.toolSupportStatus == 'unknown').length;
  final supportedTools =
      agents.where((agent) => agent.toolSupportStatus == 'supported').length;
  final toolDetails = <String>[
    if (unsupportedTools > 0) 'unsupported宣言=$unsupportedTools件',
    if (unknownTools > 0) 'unknown=$unknownTools件',
    if (supportedTools > 0) 'supported宣言=$supportedTools件',
  ];
  final toolRow = agents.isEmpty
      ? 'Tool: Agent metadataがないため互換性はunknownです。'
      : 'Tool: ${toolDetails.join(' / ')}。現在のTool catalog比較は未実施です。';

  final failedChecks = checks.where((check) => check.status == 'fail').length;
  final productRow = reportStatus == 'fail' || failedChecks > 0
      ? '製品: Setup Doctorが故障候補を$failedChecks件報告しています。復旧案内を確認してください。'
      : reportStatus == 'pass'
          ? '製品: このSetup Doctorの固定診断項目はpassです。製品全体のQA判定ではありません。'
          : '製品: Setup Doctorにwarning／unknownがあります。passや故障なしとは判定しません。';

  return <String>[
    '参照元: $source。snapshotの範囲内で分類し、未観測値はunknownのまま表示します。',
    runtimeRow,
    authenticationRow,
    permissionRow,
    toolRow,
    productRow,
    'この表示はPermission／Approvalを発行せず、Agent metadataからAuthorityを生成しません。',
  ];
}

class _ReportSummary extends StatelessWidget {
  const _ReportSummary({required this.status, required this.checks});

  final String status;
  final List<SetupDoctorCheckRecord> checks;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final tone = _statusTone(status);
    final passed = checks.where((check) => check.status == 'pass').length;
    final needsAttention = checks
        .where(
            (check) => check.status == 'warning' || check.status == 'unknown')
        .length;
    final failed = checks.where((check) => check.status == 'fail').length;
    final summary = status == 'pass'
        ? '確認できる範囲に問題は見つかりませんでした。'
        : status == 'fail'
            ? '問題が見つかりました。該当項目の復旧案内を確認してください。'
            : status == 'warning'
                ? '確認できない項目、または注意が必要な項目があります。'
                : '診断状態を確認できません。Broker接続を確認してください。';

    return Semantics(
      label: '環境診断の概要',
      value: '状態: ${_statusLabel(status)}',
      container: true,
      child: Card(
        color: tone.background(colorScheme),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(tone.icon, color: tone.foreground(colorScheme)),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          '診断状態: ${_statusLabel(status)}',
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                        const SizedBox(height: 4),
                        Text(summary),
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  Chip(label: Text('正常 $passed件')),
                  Chip(label: Text('確認 $needsAttention件')),
                  Chip(label: Text('問題 $failed件')),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SetupDoctorCheckCard extends StatelessWidget {
  const _SetupDoctorCheckCard({required this.check});

  final SetupDoctorCheckRecord check;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final tone = _statusTone(check.status);
    final recovery = check.recoveryInstruction;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: Text(
                    _checkTitle(check.checkId),
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                ),
                const SizedBox(width: 8),
                Chip(
                  avatar: Icon(
                    tone.icon,
                    size: 18,
                    color: tone.foreground(colorScheme),
                  ),
                  label: Text(_statusLabel(check.status)),
                  visualDensity: VisualDensity.compact,
                ),
              ],
            ),
            const SizedBox(height: 8),
            Text(check.message),
            const SizedBox(height: 8),
            Chip(
              visualDensity: VisualDensity.compact,
              label: Text(_evidenceClassLabel(check.evidenceClass)),
            ),
            if (check.status != 'pass' && recovery != null) ...[
              const SizedBox(height: 12),
              Text(
                '次に行うこと',
                style: Theme.of(context).textTheme.labelLarge,
              ),
              const SizedBox(height: 4),
              Text(recovery),
            ],
          ],
        ),
      ),
    );
  }
}

class _StatusTone {
  const _StatusTone(this.icon, this.background, this.foreground);

  final IconData icon;
  final Color Function(ColorScheme) background;
  final Color Function(ColorScheme) foreground;
}

_StatusTone _statusTone(String status) => switch (status) {
      'pass' => _StatusTone(
          Icons.check_circle_outline,
          (colors) => colors.secondaryContainer,
          (colors) => colors.onSecondaryContainer,
        ),
      'fail' => _StatusTone(
          Icons.error_outline,
          (colors) => colors.errorContainer,
          (colors) => colors.onErrorContainer,
        ),
      _ => _StatusTone(
          Icons.warning_amber_outlined,
          (colors) => colors.tertiaryContainer,
          (colors) => colors.onTertiaryContainer,
        ),
    };

String _statusLabel(String status) => switch (status) {
      'pass' => '正常',
      'warning' => '確認が必要',
      'fail' => '問題あり',
      _ => '不明',
    };

String _evidenceClassLabel(String? evidenceClass) => switch (evidenceClass) {
      'CONFIG' => '証拠種別: 設定情報',
      'INTERNAL_STATE' => '証拠種別: Broker内部状態',
      'LIVE_RUNTIME' => '証拠種別: Broker実行時観測',
      _ => '証拠種別: 不明',
    };

String _checkTitle(String checkId) => switch (checkId) {
      'setup_doctor.ran_from_installed_app_path' => '製品配置',
      'setup_doctor.runtime_connection' => 'Broker接続',
      'setup_doctor.authority_boundary' => '権限境界',
      'setup_doctor.network_public_bind' => '通信範囲',
      'setup_doctor.recovery_instruction' => '復旧案内',
      'setup_doctor.audit_storage' => '監査保存',
      'setup_doctor.config_created' => '初回設定',
      _ => 'その他の診断項目',
    };

String _snapshotSourceLabel(String source) => switch (source) {
      'broker' => 'Rust Broker',
      'broker_unavailable' => 'Broker利用不可',
      'fallback' => '安全な代替値',
      'in_memory_diagnostic' => 'メモリ内診断値',
      'mock' => '試験用データ',
      _ => '不明',
    };

String _snapshotFreshnessLabel(String freshness) => switch (freshness) {
      'verified' => '検証済み',
      'unknown' => '不明',
      'static' => '固定値',
      'missing' => '取得なし',
      'unavailable' => '利用不可',
      'parse failed' => '解析失敗',
      _ => '不明',
    };

String _networkExposureLabel(String exposure) => switch (exposure) {
      'localhost only' => 'loopback限定',
      'unknown' => '不明',
      _ => '要確認',
    };

String _auditChainLabel(String status) => switch (status) {
      'verified' => 'Broker内部で検証済み',
      'unknown' => '不明',
      'tampered' => '改変を検出',
      _ => '要確認',
    };

String _releaseStateLabel(String status) => switch (status) {
      'not claimed' => '未申告',
      'release_ready' => 'release_ready（証拠確認が必要）',
      _ => '不明',
    };

String _runtimeStatusLabel(String status) => switch (status) {
      'ready' => '準備完了',
      'running' => '稼働中',
      'stopped' => '停止中',
      'unknown' => '不明',
      _ => '要確認',
    };
