import 'dart:io';

import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import 'shared.dart';

class SetupDoctor extends StatelessWidget {
  const SetupDoctor({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    return ShellPage(
      title: '環境診断',
      children: [
        const SectionList(
          title: '権限境界',
          rows: [
            'installer_grants_authority=false',
            'installer_silently_approves_permissions=false',
          ],
        ),
        SectionList(
          title: '環境スナップショット',
          rows: [
            'os: ${Platform.operatingSystem} ${Platform.operatingSystemVersion}',
            'dart: ${Platform.version.split('\n').first}',
            'WSL／ネイティブの判定補助: ${Platform.environment.containsKey('WSL_DISTRO_NAME') ? 'wsl' : 'native-or-container'}',
            'Flutterツールチェーン: PATHから取得',
            'Python: tooling/*.pyの検証に必要',
            'Rust helper: validate_all内のcargo testで検証',
            'ネットワーク公開範囲: ${snapshot.networkExposure}',
            '監査鎖状態: ${snapshot.auditChainStatus}',
            '設定／スナップショットのパス: ${snapshot.snapshotPath}',
          ],
        ),
        SectionList(
          title: '診断結果',
          rows: [
            '状態: ${snapshot.setupDoctorStatus}',
            for (final check in snapshot.setupDoctorChecks)
              '${check.checkId}: ${check.status} - ${check.message}${check.recoveryInstruction == null ? '' : ' / ${check.recoveryInstruction}'}',
          ],
        ),
        SectionList(
          title: 'インストール先の証拠',
          rows: [
            for (final evidence in snapshot.evidence.where(
              (item) => item.kind == 'installed-path',
            ))
              '${evidence.evidenceId}: ${evidence.status} ${evidence.path}',
          ],
        ),
        SectionList(
          title: '実行系接続',
          rows: [
            for (final runtime in snapshot.runtimes)
              '${runtime.runtimeId}: ${runtime.status}',
          ],
        ),
        SectionList(
          title: '復旧手順',
          rows: [
            for (final recovery in snapshot.recoveryActions) recovery.message,
          ],
        ),
      ],
    );
  }
}
