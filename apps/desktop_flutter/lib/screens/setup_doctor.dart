import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class SetupDoctor extends StatelessWidget {
  const SetupDoctor({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final installedEvidence = snapshot.evidence
        .where((item) => item.kind == 'installed-path')
        .toList(growable: false);
    final checks = snapshot.setupDoctorChecks;

    return ShellPage(
      title: '環境診断',
      children: [
        _ReportSummary(status: snapshot.setupDoctorStatus, checks: checks),
        const SectionList(
          title: 'この診断で分かること',
          rows: [
            'Brokerが取得できた現在の実行状態と、確認が必要な項目を表示します。',
            'この診断はPermissionやApprovalを作らず、製品リリースの完成判定にも使いません。',
          ],
        ),
        SectionList(
          title: '実行状態',
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
              ? const ['Brokerから診断項目を取得できません。']
              : ['${checks.length}項目を確認しました。'],
        ),
        for (final check in checks) _SetupDoctorCheckCard(check: check),
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
