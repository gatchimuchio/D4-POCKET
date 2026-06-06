class RuntimeRecord {
  const RuntimeRecord({
    required this.runtimeId,
    required this.name,
    required this.status,
    required this.adapterId,
    required this.diagnosticSummary,
  });

  final String runtimeId;
  final String name;
  final String status;
  final String adapterId;
  final String diagnosticSummary;

  factory RuntimeRecord.fromJson(Map<String, Object?> json) {
    return RuntimeRecord(
      runtimeId: json['runtime_id'] as String? ?? '',
      name: json['name'] as String? ?? '',
      status: json['status'] as String? ?? '',
      adapterId: json['adapter_id'] as String? ?? '',
      diagnosticSummary: json['diagnostic_summary'] as String? ?? '',
    );
  }
}

class PermissionRecord {
  const PermissionRecord({
    required this.permissionId,
    required this.capabilityId,
    required this.decision,
    required this.source,
    this.expiresAt,
  });

  final String permissionId;
  final String capabilityId;
  final String decision;
  final String source;
  final String? expiresAt;

  factory PermissionRecord.fromJson(Map<String, Object?> json) {
    return PermissionRecord(
      permissionId: json['permission_id'] as String? ?? '',
      capabilityId: json['capability_id'] as String? ?? '',
      decision: json['decision'] as String? ?? '',
      source: json['source'] as String? ?? '',
      expiresAt: json['expires_at'] as String?,
    );
  }
}

class ApprovalRecord {
  const ApprovalRecord({
    required this.approvalId,
    required this.operation,
    required this.status,
    required this.contentVisibility,
    required this.projectedContent,
    required this.editableFields,
    required this.protectedFields,
  });

  final String approvalId;
  final String operation;
  final String status;
  final String contentVisibility;
  final Map<String, Object?> projectedContent;
  final List<String> editableFields;
  final List<String> protectedFields;

  factory ApprovalRecord.fromJson(Map<String, Object?> json) {
    return ApprovalRecord(
      approvalId: json['approval_id'] as String? ?? '',
      operation: json['operation'] as String? ?? '',
      status: json['status'] as String? ?? '',
      contentVisibility: json['content_visibility'] as String? ?? 'redacted',
      projectedContent:
          Map<String, Object?>.from(json['projected_content'] as Map? ?? {}),
      editableFields: _stringList(json['editable_fields']),
      protectedFields: _stringList(json['protected_fields']),
    );
  }
}

class AuditRecord {
  const AuditRecord({
    required this.eventId,
    required this.action,
    required this.result,
    required this.payloadHash,
    this.previousEventHash,
  });

  final String eventId;
  final String action;
  final String result;
  final String payloadHash;
  final String? previousEventHash;

  factory AuditRecord.fromJson(Map<String, Object?> json) {
    return AuditRecord(
      eventId: json['event_id'] as String? ?? '',
      action: json['action'] as String? ?? '',
      result: json['result'] as String? ?? '',
      payloadHash: json['payload_hash'] as String? ?? '',
      previousEventHash: json['previous_event_hash'] as String?,
    );
  }
}

class RecoveryRecord {
  const RecoveryRecord({
    required this.recoveryId,
    required this.severity,
    required this.message,
    required this.safeToRetry,
  });

  final String recoveryId;
  final String severity;
  final String message;
  final bool safeToRetry;

  factory RecoveryRecord.fromJson(Map<String, Object?> json) {
    return RecoveryRecord(
      recoveryId: json['recovery_id'] as String? ?? '',
      severity: json['severity'] as String? ?? '',
      message: json['message'] as String? ?? '',
      safeToRetry: json['safe_to_retry'] as bool? ?? false,
    );
  }
}

class AgentSessionRecord {
  const AgentSessionRecord({
    required this.sessionId,
    required this.workspace,
    required this.task,
    required this.changedFiles,
    required this.toolCalls,
    required this.shellCommands,
    required this.testStatus,
    required this.diffSummary,
    required this.pendingApprovalCount,
    required this.rollbackCandidate,
    required this.auditEventId,
  });

  final String sessionId;
  final String workspace;
  final String task;
  final List<String> changedFiles;
  final List<String> toolCalls;
  final List<String> shellCommands;
  final String testStatus;
  final String diffSummary;
  final int pendingApprovalCount;
  final String rollbackCandidate;
  final String auditEventId;

  factory AgentSessionRecord.fromJson(Map<String, Object?> json) {
    return AgentSessionRecord(
      sessionId: json['session_id'] as String? ?? '',
      workspace: json['workspace'] as String? ?? '',
      task: json['task'] as String? ?? '',
      changedFiles: _stringList(json['changed_files']),
      toolCalls: _stringList(json['tool_calls']),
      shellCommands: _stringList(json['shell_commands']),
      testStatus: json['test_status'] as String? ?? '',
      diffSummary: json['diff_summary'] as String? ?? '',
      pendingApprovalCount: json['pending_approval_count'] as int? ?? 0,
      rollbackCandidate: json['rollback_candidate'] as String? ?? '',
      auditEventId: json['audit_event_id'] as String? ?? '',
    );
  }
}

class SetupDoctorCheckRecord {
  const SetupDoctorCheckRecord({
    required this.checkId,
    required this.status,
    required this.message,
    required this.recoveryInstruction,
    required this.grantsAuthority,
  });

  final String checkId;
  final String status;
  final String message;
  final String? recoveryInstruction;
  final bool grantsAuthority;

  factory SetupDoctorCheckRecord.fromJson(Map<String, Object?> json) {
    return SetupDoctorCheckRecord(
      checkId: json['check_id'] as String? ?? '',
      status: json['status'] as String? ?? '',
      message: json['message'] as String? ?? '',
      recoveryInstruction: json['recovery_instruction'] as String?,
      grantsAuthority: json['grants_authority'] as bool? ?? false,
    );
  }
}

class TrustRecord {
  const TrustRecord({
    required this.scope,
    required this.state,
    required this.source,
    required this.expiresAt,
    required this.blockedOperations,
  });

  final String scope;
  final String state;
  final String source;
  final String? expiresAt;
  final List<String> blockedOperations;

  factory TrustRecord.fromJson(Map<String, Object?> json) {
    return TrustRecord(
      scope: json['scope'] as String? ?? '',
      state: json['state'] as String? ?? 'unknown',
      source: json['source'] as String? ?? '',
      expiresAt: json['expires_at'] as String?,
      blockedOperations: _stringList(json['blocked_operations']),
    );
  }
}

class AuthorityMapRecord {
  const AuthorityMapRecord({
    required this.runtimeId,
    required this.capabilityId,
    required this.permissionId,
    required this.approvalId,
    required this.auditEventId,
    required this.recoveryId,
    required this.dangerous,
    required this.warning,
  });

  final String runtimeId;
  final String capabilityId;
  final String permissionId;
  final String approvalId;
  final String auditEventId;
  final String recoveryId;
  final bool dangerous;
  final String warning;

  factory AuthorityMapRecord.fromJson(Map<String, Object?> json) {
    return AuthorityMapRecord(
      runtimeId: json['runtime_id'] as String? ?? '',
      capabilityId: json['capability_id'] as String? ?? '',
      permissionId: json['permission_id'] as String? ?? '',
      approvalId: json['approval_id'] as String? ?? '',
      auditEventId: json['audit_event_id'] as String? ?? '',
      recoveryId: json['recovery_id'] as String? ?? '',
      dangerous: json['dangerous'] as bool? ?? false,
      warning: json['warning'] as String? ?? '',
    );
  }
}

class AdapterCatalogRecord {
  const AdapterCatalogRecord({
    required this.adapterId,
    required this.runtimeId,
    required this.publisher,
    required this.version,
    required this.signature,
    required this.hash,
    required this.requestedCapabilities,
    required this.grantedCapabilities,
    required this.deniedCapabilities,
    required this.trustStatus,
    required this.lastVerified,
    required this.updateAvailable,
    required this.knownRisks,
  });

  final String adapterId;
  final String runtimeId;
  final String publisher;
  final String version;
  final String signature;
  final String hash;
  final List<String> requestedCapabilities;
  final List<String> grantedCapabilities;
  final List<String> deniedCapabilities;
  final String trustStatus;
  final String lastVerified;
  final bool updateAvailable;
  final List<String> knownRisks;

  factory AdapterCatalogRecord.fromJson(Map<String, Object?> json) {
    return AdapterCatalogRecord(
      adapterId: json['adapter_id'] as String? ?? '',
      runtimeId: json['runtime_id'] as String? ?? '',
      publisher: json['publisher'] as String? ?? '',
      version: json['version'] as String? ?? '',
      signature: json['signature'] as String? ?? '',
      hash: json['hash'] as String? ?? '',
      requestedCapabilities: _stringList(json['requested_capabilities']),
      grantedCapabilities: _stringList(json['granted_capabilities']),
      deniedCapabilities: _stringList(json['denied_capabilities']),
      trustStatus: json['trust_status'] as String? ?? 'unknown',
      lastVerified: json['last_verified'] as String? ?? '',
      updateAvailable: json['update_available'] as bool? ?? false,
      knownRisks: _stringList(json['known_risks']),
    );
  }
}

class PermissionDiffRecord {
  const PermissionDiffRecord({
    required this.subject,
    required this.added,
    required this.removed,
    required this.changed,
    required this.dangerous,
  });

  final String subject;
  final List<String> added;
  final List<String> removed;
  final List<String> changed;
  final List<String> dangerous;

  factory PermissionDiffRecord.fromJson(Map<String, Object?> json) {
    return PermissionDiffRecord(
      subject: json['subject'] as String? ?? '',
      added: _stringList(json['added']),
      removed: _stringList(json['removed']),
      changed: _stringList(json['changed']),
      dangerous: _stringList(json['dangerous']),
    );
  }
}

class ProblemRecord {
  const ProblemRecord({
    required this.problemId,
    required this.severity,
    required this.category,
    required this.message,
    required this.target,
    required this.recoveryId,
    this.item = '',
    this.classification = 'required_for_v1',
    this.reason = '',
    this.requiredAction = '',
    this.blocksRelease = false,
    this.safeToIgnoreForPhaseB = false,
    this.blocksOwnerUse = false,
    this.blocksCompletedProductRelease = false,
  });

  final String problemId;
  final String severity;
  final String category;
  final String message;
  final String target;
  final String recoveryId;
  final String item;
  final String classification;
  final String reason;
  final String requiredAction;
  final bool blocksRelease;
  final bool safeToIgnoreForPhaseB;
  final bool blocksOwnerUse;
  final bool blocksCompletedProductRelease;

  factory ProblemRecord.fromJson(Map<String, Object?> json) {
    return ProblemRecord(
      problemId: json['problem_id'] as String? ?? '',
      severity: json['severity'] as String? ?? 'warning',
      category: json['category'] as String? ?? '',
      message: json['message'] as String? ?? '',
      target: json['target'] as String? ?? '',
      recoveryId: json['recovery_id'] as String? ?? '',
      item: json['item'] as String? ?? json['message'] as String? ?? '',
      classification: json['classification'] as String? ?? 'required_for_v1',
      reason: json['reason'] as String? ?? '',
      requiredAction: json['required_action'] as String? ?? '',
      blocksRelease: _boolFromJson(json['blocks_release']),
      safeToIgnoreForPhaseB: _boolFromJson(json['safe_to_ignore_for_phase_b']),
      blocksOwnerUse: _boolFromJson(json['blocks_owner_use']),
      blocksCompletedProductRelease:
          _boolFromJson(json['blocks_completed_product_release']),
    );
  }
}

class EvidenceRecord {
  const EvidenceRecord({
    required this.evidenceId,
    required this.kind,
    required this.status,
    required this.path,
    required this.hash,
    required this.exportable,
  });

  final String evidenceId;
  final String kind;
  final String status;
  final String path;
  final String hash;
  final bool exportable;

  factory EvidenceRecord.fromJson(Map<String, Object?> json) {
    return EvidenceRecord(
      evidenceId: json['evidence_id'] as String? ?? '',
      kind: json['kind'] as String? ?? '',
      status: json['status'] as String? ?? 'missing',
      path: json['path'] as String? ?? '',
      hash: json['hash'] as String? ?? '',
      exportable: json['exportable'] as bool? ?? false,
    );
  }
}

class SettingRecord {
  const SettingRecord({
    required this.key,
    required this.group,
    required this.defaultValue,
    required this.currentValue,
    required this.effectiveValue,
    required this.source,
    required this.modified,
    required this.dangerous,
    required this.authorityRelated,
  });

  final String key;
  final String group;
  final String defaultValue;
  final String currentValue;
  final String effectiveValue;
  final String source;
  final bool modified;
  final bool dangerous;
  final bool authorityRelated;

  factory SettingRecord.fromJson(Map<String, Object?> json) {
    return SettingRecord(
      key: json['key'] as String? ?? '',
      group: json['group'] as String? ?? '',
      defaultValue: json['default']?.toString() ?? '',
      currentValue: json['current']?.toString() ?? '',
      effectiveValue: json['effective']?.toString() ?? '',
      source: json['source'] as String? ?? '',
      modified: json['modified'] as bool? ?? false,
      dangerous: json['dangerous'] as bool? ?? false,
      authorityRelated: json['authority_related'] as bool? ?? false,
    );
  }
}

class PhaseStatusRecord {
  const PhaseStatusRecord({
    required this.phaseAStatus,
    required this.phaseBStatus,
    required this.phaseCStatus,
    required this.phaseDStatus,
    required this.phaseEStatus,
    required this.phaseFStatus,
    required this.completedProductReleaseClaimed,
  });

  final String phaseAStatus;
  final String phaseBStatus;
  final String phaseCStatus;
  final String phaseDStatus;
  final String phaseEStatus;
  final String phaseFStatus;
  final bool completedProductReleaseClaimed;

  factory PhaseStatusRecord.fromJson(Map<String, Object?> json) {
    return PhaseStatusRecord(
      phaseAStatus: json['phase_a_status'] as String? ?? 'complete',
      phaseBStatus: json['phase_b_status'] as String? ?? 'complete',
      phaseCStatus: json['phase_c_status'] as String? ?? 'next',
      phaseDStatus: json['phase_d_status'] as String? ?? 'later',
      phaseEStatus: json['phase_e_status'] as String? ?? 'later',
      phaseFStatus: json['phase_f_status'] as String? ?? 'later',
      completedProductReleaseClaimed:
          json['completed_product_release_claimed'] as bool? ?? false,
    );
  }
}

class OperationStatusRecord {
  const OperationStatusRecord({
    required this.runtimeStatus,
    required this.invariantStatus,
    required this.trustStatus,
    required this.pendingApprovalsCount,
    required this.auditChainStatus,
    required this.problemsCount,
    required this.releaseState,
  });

  final String runtimeStatus;
  final String invariantStatus;
  final String trustStatus;
  final int pendingApprovalsCount;
  final String auditChainStatus;
  final int problemsCount;
  final String releaseState;

  factory OperationStatusRecord.fromJson(Map<String, Object?> json) {
    return OperationStatusRecord(
      runtimeStatus: json['runtime_status'] as String? ?? 'unknown',
      invariantStatus: json['invariant_status'] as String? ?? 'unknown',
      trustStatus: json['trust_status'] as String? ?? 'unknown',
      pendingApprovalsCount: json['pending_approvals_count'] as int? ?? 0,
      auditChainStatus: json['audit_chain_status'] as String? ?? 'unknown',
      problemsCount: json['problems_count'] as int? ?? 0,
      releaseState: json['release_state'] as String? ?? 'not claimed',
    );
  }

  OperationStatusRecord copyWith({
    String? runtimeStatus,
    String? invariantStatus,
    String? trustStatus,
    int? pendingApprovalsCount,
    String? auditChainStatus,
    int? problemsCount,
    String? releaseState,
  }) {
    return OperationStatusRecord(
      runtimeStatus: runtimeStatus ?? this.runtimeStatus,
      invariantStatus: invariantStatus ?? this.invariantStatus,
      trustStatus: trustStatus ?? this.trustStatus,
      pendingApprovalsCount:
          pendingApprovalsCount ?? this.pendingApprovalsCount,
      auditChainStatus: auditChainStatus ?? this.auditChainStatus,
      problemsCount: problemsCount ?? this.problemsCount,
      releaseState: releaseState ?? this.releaseState,
    );
  }
}

class EvidenceSummaryRecord {
  const EvidenceSummaryRecord({
    required this.schemaCheck,
    required this.conformanceCheckCount,
    required this.releaseSmoke,
    required this.releaseGateCheck,
    required this.evidenceBundle,
    required this.validateAll,
    required this.strictWindowsRelease,
    required this.missingMeasuredWindowsEvidence,
    required this.missingSetupDoctorEvidence,
    required this.missingAuditAnchorExternalTamperEvidence,
    required this.ownerGo,
  });

  final String schemaCheck;
  final int conformanceCheckCount;
  final String releaseSmoke;
  final String releaseGateCheck;
  final String evidenceBundle;
  final String validateAll;
  final String strictWindowsRelease;
  final bool missingMeasuredWindowsEvidence;
  final bool missingSetupDoctorEvidence;
  final bool missingAuditAnchorExternalTamperEvidence;
  final String ownerGo;

  factory EvidenceSummaryRecord.fromJson(Map<String, Object?> json) {
    return EvidenceSummaryRecord(
      schemaCheck: json['schema_check'] as String? ?? 'not reported',
      conformanceCheckCount: json['conformance_check_count'] as int? ?? 0,
      releaseSmoke: json['release_smoke'] as String? ?? 'not reported',
      releaseGateCheck: json['release_gate_check'] as String? ?? 'not reported',
      evidenceBundle: json['evidence_bundle'] as String? ?? 'not reported',
      validateAll: json['validate_all'] as String? ?? 'not reported',
      strictWindowsRelease:
          json['strict_windows_release'] as String? ?? 'not reported',
      missingMeasuredWindowsEvidence:
          json['missing_measured_windows_evidence'] as bool? ?? true,
      missingSetupDoctorEvidence:
          json['missing_setup_doctor_evidence'] as bool? ?? true,
      missingAuditAnchorExternalTamperEvidence:
          json['missing_audit_anchor_external_tamper_evidence'] as bool? ??
              true,
      ownerGo: json['owner_go'] as String? ?? 'missing',
    );
  }
}

class RecoveryPlaybookRecord {
  const RecoveryPlaybookRecord({
    required this.item,
    required this.severity,
    required this.classification,
    required this.safeToIgnoreForPhaseB,
    required this.requiredAction,
    required this.blocksCompletedProductRelease,
    this.recoveryId = '',
    this.blocksOwnerUse = false,
    this.command = '',
    this.path = '',
  });

  final String recoveryId;
  final String item;
  final String severity;
  final String classification;
  final bool safeToIgnoreForPhaseB;
  final String requiredAction;
  final bool blocksCompletedProductRelease;
  final bool blocksOwnerUse;
  final String command;
  final String path;

  factory RecoveryPlaybookRecord.fromJson(Map<String, Object?> json) {
    return RecoveryPlaybookRecord(
      recoveryId: json['recovery_id'] as String? ?? '',
      item: json['item'] as String? ?? '',
      severity: json['severity'] as String? ?? 'warning',
      classification: json['classification'] as String? ?? 'required_for_v1',
      safeToIgnoreForPhaseB:
          json['safe_to_ignore_for_phase_b'] as bool? ?? false,
      requiredAction: json['required_action'] as String? ?? '',
      blocksCompletedProductRelease:
          _boolFromJson(json['blocks_completed_product_release']),
      blocksOwnerUse: _boolFromJson(json['blocks_owner_use']),
      command: json['command'] as String? ?? '',
      path: json['path'] as String? ?? '',
    );
  }
}

class ShellSnapshot {
  const ShellSnapshot({
    required this.phaseStatus,
    required this.operationStatus,
    required this.runtimes,
    required this.agentSessions,
    required this.permissions,
    required this.pendingApprovals,
    required this.auditEvents,
    required this.recoveryActions,
    required this.invariantFlags,
    required this.setupDoctorChecks,
    required this.setupDoctorStatus,
    required this.installerGrantsAuthority,
    required this.installerSilentlyApprovesPermissions,
    required this.trustRecords,
    required this.authorityMap,
    required this.adapterCatalog,
    required this.permissionDiffs,
    required this.problems,
    required this.evidence,
    required this.settings,
    required this.auditChainStatus,
    required this.networkExposure,
    required this.releaseBlockerCount,
    required this.evidenceSummary,
    required this.recoveryPlaybook,
    this.snapshotSource = 'fallback',
    this.snapshotPath = '',
    this.snapshotGeneratedAt = '',
    this.snapshotFreshness = 'unknown',
  });

  final PhaseStatusRecord phaseStatus;
  final OperationStatusRecord operationStatus;
  final List<RuntimeRecord> runtimes;
  final List<AgentSessionRecord> agentSessions;
  final List<PermissionRecord> permissions;
  final List<ApprovalRecord> pendingApprovals;
  final List<AuditRecord> auditEvents;
  final List<RecoveryRecord> recoveryActions;
  final Map<String, bool> invariantFlags;
  final List<SetupDoctorCheckRecord> setupDoctorChecks;
  final String setupDoctorStatus;
  final bool installerGrantsAuthority;
  final bool installerSilentlyApprovesPermissions;
  final List<TrustRecord> trustRecords;
  final List<AuthorityMapRecord> authorityMap;
  final List<AdapterCatalogRecord> adapterCatalog;
  final List<PermissionDiffRecord> permissionDiffs;
  final List<ProblemRecord> problems;
  final List<EvidenceRecord> evidence;
  final List<SettingRecord> settings;
  final String auditChainStatus;
  final String networkExposure;
  final int releaseBlockerCount;
  final EvidenceSummaryRecord evidenceSummary;
  final List<RecoveryPlaybookRecord> recoveryPlaybook;
  final String snapshotSource;
  final String snapshotPath;
  final String snapshotGeneratedAt;
  final String snapshotFreshness;

  factory ShellSnapshot.fromJson(Map<String, Object?> json) {
    final runtimes = _records(json['runtimes'], RuntimeRecord.fromJson);
    final approvals =
        _records(json['pending_approvals'], ApprovalRecord.fromJson);
    final invariantFlags =
        Map<String, bool>.from(json['invariant_flags'] as Map? ?? {});
    final trustRecords = _records(json['trust_records'], TrustRecord.fromJson);
    final problems = _records(json['problems'], ProblemRecord.fromJson);
    final auditChainStatus = json['audit_chain_status'] as String? ?? 'unknown';
    return ShellSnapshot(
      phaseStatus: PhaseStatusRecord.fromJson(
          Map<String, Object?>.from(json['phase_status'] as Map? ?? {})),
      operationStatus: OperationStatusRecord.fromJson(
          Map<String, Object?>.from(json['operation_status'] as Map? ??
              {
                'runtime_status':
                    runtimes.isEmpty ? 'unknown' : runtimes.first.status,
                'invariant_status': invariantFlags.values.any((value) => value)
                    ? 'blocked'
                    : 'ok',
                'trust_status':
                    trustRecords.isEmpty ? 'unknown' : trustRecords.first.state,
                'pending_approvals_count': approvals.length,
                'audit_chain_status': auditChainStatus,
                'problems_count': problems.length,
                'release_state': 'not claimed',
              })),
      runtimes: runtimes,
      agentSessions:
          _records(json['agent_sessions'], AgentSessionRecord.fromJson),
      permissions: _records(json['permissions'], PermissionRecord.fromJson),
      pendingApprovals: approvals,
      auditEvents: _records(json['audit_events'], AuditRecord.fromJson),
      recoveryActions:
          _records(json['recovery_actions'], RecoveryRecord.fromJson),
      invariantFlags: invariantFlags,
      setupDoctorChecks: _records(
          json['setup_doctor_checks'], SetupDoctorCheckRecord.fromJson),
      setupDoctorStatus: json['setup_doctor_status'] as String? ?? 'warning',
      installerGrantsAuthority:
          json['installer_grants_authority'] as bool? ?? false,
      installerSilentlyApprovesPermissions:
          json['installer_silently_approves_permissions'] as bool? ?? false,
      trustRecords: trustRecords,
      authorityMap:
          _records(json['authority_map'], AuthorityMapRecord.fromJson),
      adapterCatalog:
          _records(json['adapter_catalog'], AdapterCatalogRecord.fromJson),
      permissionDiffs:
          _records(json['permission_diffs'], PermissionDiffRecord.fromJson),
      problems: problems,
      evidence: _records(json['evidence'], EvidenceRecord.fromJson),
      settings: _records(json['settings'], SettingRecord.fromJson),
      auditChainStatus: auditChainStatus,
      networkExposure: json['network_exposure'] as String? ?? 'unknown',
      releaseBlockerCount: json['release_blocker_count'] as int? ?? 0,
      evidenceSummary: EvidenceSummaryRecord.fromJson(
          Map<String, Object?>.from(json['evidence_summary'] as Map? ?? {})),
      recoveryPlaybook:
          _records(json['recovery_playbook'], RecoveryPlaybookRecord.fromJson),
      snapshotSource: json['snapshot_source'] as String? ?? 'local',
      snapshotPath: json['snapshot_path'] as String? ?? '',
      snapshotGeneratedAt: json['snapshot_generated_at'] as String? ??
          json['generated_at'] as String? ??
          json['snapshot_freshness'] as String? ??
          '',
      snapshotFreshness: json['snapshot_freshness'] as String? ?? 'generated',
    );
  }

  ShellSnapshot copyWith({
    OperationStatusRecord? operationStatus,
    List<ProblemRecord>? problems,
    List<EvidenceRecord>? evidence,
    String? snapshotSource,
    String? snapshotPath,
    String? snapshotGeneratedAt,
    String? snapshotFreshness,
  }) {
    return ShellSnapshot(
      phaseStatus: phaseStatus,
      operationStatus: operationStatus ?? this.operationStatus,
      runtimes: runtimes,
      agentSessions: agentSessions,
      permissions: permissions,
      pendingApprovals: pendingApprovals,
      auditEvents: auditEvents,
      recoveryActions: recoveryActions,
      invariantFlags: invariantFlags,
      setupDoctorChecks: setupDoctorChecks,
      setupDoctorStatus: setupDoctorStatus,
      installerGrantsAuthority: installerGrantsAuthority,
      installerSilentlyApprovesPermissions:
          installerSilentlyApprovesPermissions,
      trustRecords: trustRecords,
      authorityMap: authorityMap,
      adapterCatalog: adapterCatalog,
      permissionDiffs: permissionDiffs,
      problems: problems ?? this.problems,
      evidence: evidence ?? this.evidence,
      settings: settings,
      auditChainStatus: auditChainStatus,
      networkExposure: networkExposure,
      releaseBlockerCount: releaseBlockerCount,
      evidenceSummary: evidenceSummary,
      recoveryPlaybook: recoveryPlaybook,
      snapshotSource: snapshotSource ?? this.snapshotSource,
      snapshotPath: snapshotPath ?? this.snapshotPath,
      snapshotGeneratedAt: snapshotGeneratedAt ?? this.snapshotGeneratedAt,
      snapshotFreshness: snapshotFreshness ?? this.snapshotFreshness,
    );
  }
}

bool _boolFromJson(Object? value) {
  if (value is bool) {
    return value;
  }
  if (value is String) {
    return value.toLowerCase() == 'yes' || value.toLowerCase() == 'true';
  }
  return false;
}

List<String> _stringList(Object? value) {
  return (value as List? ?? []).map((item) => item.toString()).toList();
}

List<T> _records<T>(
  Object? value,
  T Function(Map<String, Object?> json) decode,
) {
  return (value as List? ?? [])
      .whereType<Map>()
      .map((item) => decode(Map<String, Object?>.from(item)))
      .toList();
}
