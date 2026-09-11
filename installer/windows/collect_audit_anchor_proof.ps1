param(
  [Parameter(Mandatory = $true)]
  [string]$InstalledRoot,

  [string]$AuditDir = "",

  [string]$OutputPath = "release_evidence/audit_anchor_external_tamper_evidence.json",

  [string]$ExternalAnchorPath = "",

  [string]$SignedEvidencePath = "",

  [string]$CheckpointBundle = "",
  [string]$TrustedHeadPath = "",
  [string]$PreviousCheckpointBundle = "-"
)

$ErrorActionPreference = "Stop"

function Write-JsonEvidence {
  param(
    [Parameter(Mandatory = $true)]
    $Value,
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [int]$Depth = 10
  )

  $json = $Value | ConvertTo-Json -Depth $Depth
  $encoding = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText($Path, ($json + [Environment]::NewLine), $encoding)
}

function Get-TaggedSha256 {
  param([Parameter(Mandatory = $true)][string]$Path)

  if (!(Test-Path $Path)) {
    return $null
  }
  return "sha256:$((Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant())"
}

function Get-StringSha256 {
  param([Parameter(Mandatory = $true)][string]$Value)

  $encoding = [System.Text.UTF8Encoding]::new($false)
  $bytes = $encoding.GetBytes($Value)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    $hash = $sha.ComputeHash($bytes)
    return "sha256:$([System.BitConverter]::ToString($hash).Replace('-', '').ToLowerInvariant())"
  } finally {
    $sha.Dispose()
  }
}

function Test-PathUnderRoot {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Root
  )

  $candidate = [System.IO.Path]::GetFullPath($Path).TrimEnd('\')
  $rootPath = [System.IO.Path]::GetFullPath($Root).TrimEnd('\')
  return $candidate.Equals($rootPath, [System.StringComparison]::OrdinalIgnoreCase) -or
    $candidate.StartsWith($rootPath + "\", [System.StringComparison]::OrdinalIgnoreCase)
}

function Test-DpapiCurrentUser {
  try {
    Add-Type -AssemblyName System.Security
    $plain = [System.Text.UTF8Encoding]::new($false).GetBytes("gui-shell-audit-anchor-dpapi-probe")
    $protected = [System.Security.Cryptography.ProtectedData]::Protect(
      $plain,
      $null,
      [System.Security.Cryptography.DataProtectionScope]::CurrentUser
    )
    $roundTrip = [System.Security.Cryptography.ProtectedData]::Unprotect(
      $protected,
      $null,
      [System.Security.Cryptography.DataProtectionScope]::CurrentUser
    )
    return ([Convert]::ToBase64String($plain) -eq [Convert]::ToBase64String($roundTrip))
  } catch {
    return $false
  }
}

function Test-BroadWriteAce {
  param($AccessRule)

  $identity = [string]$AccessRule.IdentityReference
  $rights = [System.Security.AccessControl.FileSystemRights]$AccessRule.FileSystemRights
  $broadIdentities = @(
    "Everyone",
    "BUILTIN\Users",
    "NT AUTHORITY\Authenticated Users",
    "BUILTIN\Guests"
  )
  $writeRights = @(
    [System.Security.AccessControl.FileSystemRights]::Write,
    [System.Security.AccessControl.FileSystemRights]::Modify,
    [System.Security.AccessControl.FileSystemRights]::FullControl,
    [System.Security.AccessControl.FileSystemRights]::WriteData,
    [System.Security.AccessControl.FileSystemRights]::AppendData,
    [System.Security.AccessControl.FileSystemRights]::CreateFiles,
    [System.Security.AccessControl.FileSystemRights]::CreateDirectories
  )
  $hasWrite = $false
  foreach ($right in $writeRights) {
    if (($rights -band $right) -ne 0) {
      $hasWrite = $true
      break
    }
  }
  return (
    $AccessRule.AccessControlType -eq [System.Security.AccessControl.AccessControlType]::Allow -and
    $hasWrite -and
    ($broadIdentities -contains $identity)
  )
}

function Get-AclEvidence {
  param([Parameter(Mandatory = $true)][string[]]$Paths)

  $reports = New-Object System.Collections.Generic.List[object]
  $errors = New-Object System.Collections.Generic.List[string]
  foreach ($path in $Paths) {
    try {
      $acl = Get-Acl -Path $path
      $broadWriteRules = @()
      foreach ($rule in $acl.Access) {
        if (Test-BroadWriteAce -AccessRule $rule) {
          $broadWriteRules += [ordered]@{
            identity = [string]$rule.IdentityReference
            rights = [string]$rule.FileSystemRights
            inherited = [bool]$rule.IsInherited
          }
        }
      }
      $reports.Add([ordered]@{
        path = $path
        owner = [string]$acl.Owner
        protected = [bool]$acl.AreAccessRulesProtected
        broad_write_rules = @($broadWriteRules)
      })
      if ($broadWriteRules.Count -gt 0) {
        $errors.Add("広範な write ACL を検出しました: $path")
      }
    } catch {
      $errors.Add("ACL の読取りに失敗しました: $path`: $($_.Exception.Message)")
    }
  }
  return [ordered]@{
    reports = @($reports.ToArray())
    errors = @($errors.ToArray())
    verified = ($errors.Count -eq 0 -and $reports.Count -gt 0)
  }
}

$errors = New-Object System.Collections.Generic.List[string]
$root = Resolve-Path $InstalledRoot
if ($AuditDir -eq "") {
  $AuditDir = Join-Path $root.Path "runtime\audit"
}
$auditDirPath = Resolve-Path $AuditDir -ErrorAction SilentlyContinue
if ($null -eq $auditDirPath) {
  $errors.Add("audit directory がありません: $AuditDir")
}

$requiredAuditFiles = @()
if ($null -ne $auditDirPath) {
  $requiredAuditFiles = @(
    (Join-Path $auditDirPath.Path "audit_anchor.key")
    (Join-Path $auditDirPath.Path "audit_anchor.json")
    (Join-Path $auditDirPath.Path "audit.jsonl")
  )
  foreach ($path in $requiredAuditFiles) {
    if (!(Test-Path $path)) {
      $errors.Add("必須の audit anchor file がありません: $path")
    }
  }
}

$checkedPaths = New-Object System.Collections.Generic.List[string]
$checkedPaths.Add($root.Path)
if ($null -ne $auditDirPath) {
  $checkedPaths.Add($auditDirPath.Path)
}
foreach ($path in $requiredAuditFiles) {
  if (Test-Path $path) {
    $checkedPaths.Add((Resolve-Path $path).Path)
  }
}

$installedPathVerified = $true
foreach ($path in $checkedPaths) {
  if (!(Test-PathUnderRoot -Path $path -Root $root.Path)) {
    $installedPathVerified = $false
    $errors.Add("path が installed root の外部です: $path")
  }
}

$aclEvidence = Get-AclEvidence -Paths @($checkedPaths.ToArray())
$windowsAclVerified = [bool]$aclEvidence.verified
$dpapiAvailable = Test-DpapiCurrentUser
# 固定文字列の往復はOS機能の確認だけであり、監査鍵の保護を証明しない。
$dpapiVerified = $false

$externalAnchorVerified = $false
$externalAnchorSha256 = $null
if ($ExternalAnchorPath -ne "") {
  $externalAnchor = Resolve-Path $ExternalAnchorPath -ErrorAction SilentlyContinue
  if ($null -ne $externalAnchor) {
    $errors.Add("外部fileの存在とhashだけでは、監査chainへの結合・独立保管・巻戻し防護を検証できません")
    $externalAnchorSha256 = Get-TaggedSha256 -Path $externalAnchor.Path
  } else {
    $errors.Add("external anchor path がありません: $ExternalAnchorPath")
  }
}

$signedEvidenceVerified = $false
$signedEvidenceSha256 = $null
if ($SignedEvidencePath -ne "") {
  $signedEvidence = Resolve-Path $SignedEvidencePath -ErrorAction SilentlyContinue
  if ($null -ne $signedEvidence) {
    $signature = Get-AuthenticodeSignature -FilePath $signedEvidence.Path
    $signedFileSignatureValid = ($signature.Status -eq "Valid")
    # 任意fileの署名は、この監査chainと信頼済み署名者への結合を証明しない。
    $errors.Add("署名fileと対象監査chain・信頼済み署名者の結合検証がありません")
    $signedEvidenceSha256 = Get-TaggedSha256 -Path $signedEvidence.Path
    if (!$signedFileSignatureValid) {
      $errors.Add("signed evidence の Authenticode status は $($signature.Status) です: $SignedEvidencePath")
    }
  } else {
    $errors.Add("signed evidence path がありません: $SignedEvidencePath")
  }
}

# 広範な主体へのwrite ACEがないことは、所有者自身による三fileの
# 一括書換え・親directory経由の置換を防ぐ証拠ではない。
# ACLだけでは昇格せず、下記の独立した署名checkpoint検証を必須にする。
$sameUserMitigated = $false
$checkpointResult = $null
$checkpointInputs = $null
if ($CheckpointBundle -ne "") {
  try {
    if ($TrustedHeadPath -eq "") { throw "owner管理の継続性記録が必要です" }
    $repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
    $sourceCommit = (& git -C $repositoryRoot rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw "現在source commitを取得できません" }
    $manifestPath = Join-Path $root.Path "installed_manifest.json"
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.source_commit -ne $sourceCommit -or $manifest.source_worktree_clean -ne $true) { throw "installed source commitが現在sourceと一致しません" }
    $artifact = (Resolve-Path -LiteralPath $manifest.app_exe).Path
    if (!(Test-PathUnderRoot -Path $artifact -Root $root.Path)) { throw "artifactがinstalled root外です" }
    if ((Get-TaggedSha256 -Path $artifact) -ne $manifest.app_artifact_sha256) { throw "installed manifestのartifact hash不一致" }
    $verifier = Join-Path $repositoryRoot "native/rust_helper/target/debug/gui_shell_rust_helper.exe"
    $trust = Join-Path $repositoryRoot "config/audit_signing_trust.json"
    $checkpointInputs = [ordered]@{ installed_root=$root.Path; audit_dir=$auditDirPath.Path; bundle=(Resolve-Path -LiteralPath $CheckpointBundle).Path; previous_bundle=$PreviousCheckpointBundle }
    $nativeOutput = & $verifier 監査チェックポイント verify $auditDirPath.Path $root.Path $sourceCommit $trust $TrustedHeadPath $CheckpointBundle $PreviousCheckpointBundle
    if ($LASTEXITCODE -ne 0) { throw "Rustの署名checkpoint検証が失敗しました" }
    $checkpointResult = ($nativeOutput -join "`n") | ConvertFrom-Json
    if ($checkpointResult.status -ne "passed" -or $checkpointResult.verification_kind -ne "offline_ed25519_checkpoint_v2") { throw "Rustの検証結果が不正です" }
    $signedEvidenceVerified = $true
    $sameUserMitigated = $true
  } catch {
    $errors.Add("署名checkpoint検証失敗: $($_.Exception.Message)")
  }
} else {
  $errors.Add("同一ユーザーの書換え防護にはオフライン署名checkpointとowner継続性記録が必要です")
}

$sourceKind = "windows_acl_dpapi_probe"
$evidenceClass = "LIVE_RUNTIME"
if ($signedEvidenceVerified) {
  $sourceKind = "signed_evidence"
  $evidenceClass = "EXTERNAL_EVIDENCE"
} elseif ($externalAnchorVerified) {
  $sourceKind = "external_anchor"
  $evidenceClass = "EXTERNAL_EVIDENCE"
}

$proofMaterial = [ordered]@{
  installed_root = $root.Path
  audit_dir = $(if ($null -ne $auditDirPath) { $auditDirPath.Path } else { $AuditDir })
  checked_paths = @($checkedPaths.ToArray())
  installed_path_verified = $installedPathVerified
  windows_acl_verified = $windowsAclVerified
  dpapi_verified = $dpapiVerified
  dpapi_available = $dpapiAvailable
  external_anchor_verified = $externalAnchorVerified
  signed_evidence_verified = $signedEvidenceVerified
  checkpoint_verification = $checkpointResult
  checkpoint_inputs = $checkpointInputs
  acl_report = $aclEvidence.reports
  errors = @($errors.ToArray())
}
$proofHash = Get-StringSha256 -Value ($proofMaterial | ConvertTo-Json -Compress -Depth 10)
$statusPassed = (
  $errors.Count -eq 0 -and
  $installedPathVerified -and
  $sameUserMitigated -and
  (
    $windowsAclVerified -or
    $dpapiVerified -or
    $externalAnchorVerified -or
    $signedEvidenceVerified
  )
)

$result = [ordered]@{
  status = $(if ($statusPassed) { "passed" } else { "failed" })
  collected_at = (Get-Date).ToUniversalTime().ToString("o")
  installed_root = $root.Path
  audit_dir = $(if ($null -ne $auditDirPath) { $auditDirPath.Path } else { $AuditDir })
  installed_path_verified = $installedPathVerified
  key_anchor_log_same_user_rewrite_mitigated = $sameUserMitigated
  windows_acl_verified = $windowsAclVerified
  dpapi_verified = $dpapiVerified
  dpapi_available = $dpapiAvailable
  external_anchor_verified = $externalAnchorVerified
  signed_evidence_verified = $signedEvidenceVerified
  administrator_root_resistance_claimed = $false
  checked_paths = @($checkedPaths.ToArray())
  checkpoint_verification = $checkpointResult
  checkpoint_inputs = $checkpointInputs
  acl_report = $aclEvidence.reports
  external_anchor_sha256 = $externalAnchorSha256
  signed_evidence_sha256 = $signedEvidenceSha256
  errors = @($errors.ToArray() + $aclEvidence.errors)
  evidence_source = [ordered]@{
    source_kind = $sourceKind
    evidence_class = $evidenceClass
    synthetic = $false
    command = "powershell -ExecutionPolicy Bypass -File installer\windows\collect_audit_anchor_proof.ps1 -InstalledRoot `"$($root.Path)`""
    path = $OutputPath
    sha256 = $proofHash
    sha256_scope = "probe_material_without_self_reference"
  }
}

$outputParent = Split-Path -Parent $OutputPath
if ($outputParent -ne "") {
  New-Item -ItemType Directory -Force -Path $outputParent | Out-Null
}
$output = New-Item -ItemType File -Force -Path $OutputPath
Write-JsonEvidence -Value $result -Path $output.FullName -Depth 10
Write-Host "書き出しました: $($output.FullName)"
