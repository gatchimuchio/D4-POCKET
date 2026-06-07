param(
  [Parameter(Mandatory = $true)]
  [string]$InstalledRoot,

  [string]$AuditDir = "",

  [string]$OutputPath = "release_evidence/audit_anchor_external_tamper_evidence.json",

  [string]$ExternalAnchorPath = "",

  [string]$SignedEvidencePath = ""
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
        $errors.Add("broad write ACL detected on $path")
      }
    } catch {
      $errors.Add("failed to read ACL for $path`: $($_.Exception.Message)")
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
  $errors.Add("audit directory missing: $AuditDir")
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
      $errors.Add("required audit anchor file missing: $path")
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
    $errors.Add("path is outside installed root: $path")
  }
}

$aclEvidence = Get-AclEvidence -Paths @($checkedPaths.ToArray())
$windowsAclVerified = [bool]$aclEvidence.verified
$dpapiVerified = Test-DpapiCurrentUser

$externalAnchorVerified = $false
$externalAnchorSha256 = $null
if ($ExternalAnchorPath -ne "") {
  $externalAnchor = Resolve-Path $ExternalAnchorPath -ErrorAction SilentlyContinue
  if ($null -ne $externalAnchor) {
    $externalAnchorVerified = $true
    $externalAnchorSha256 = Get-TaggedSha256 -Path $externalAnchor.Path
  } else {
    $errors.Add("external anchor path missing: $ExternalAnchorPath")
  }
}

$signedEvidenceVerified = $false
$signedEvidenceSha256 = $null
if ($SignedEvidencePath -ne "") {
  $signedEvidence = Resolve-Path $SignedEvidencePath -ErrorAction SilentlyContinue
  if ($null -ne $signedEvidence) {
    $signature = Get-AuthenticodeSignature -FilePath $signedEvidence.Path
    $signedEvidenceVerified = ($signature.Status -eq "Valid")
    $signedEvidenceSha256 = Get-TaggedSha256 -Path $signedEvidence.Path
    if (!$signedEvidenceVerified) {
      $errors.Add("signed evidence Authenticode status is $($signature.Status): $SignedEvidencePath")
    }
  } else {
    $errors.Add("signed evidence path missing: $SignedEvidencePath")
  }
}

$sameUserMitigated = (
  $installedPathVerified -and
  $windowsAclVerified -and
  ($checkedPaths.Count -ge 5)
)

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
  external_anchor_verified = $externalAnchorVerified
  signed_evidence_verified = $signedEvidenceVerified
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
  external_anchor_verified = $externalAnchorVerified
  signed_evidence_verified = $signedEvidenceVerified
  administrator_root_resistance_claimed = $false
  checked_paths = @($checkedPaths.ToArray())
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
Write-Host "wrote $($output.FullName)"
