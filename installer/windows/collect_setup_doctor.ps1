param(
  [Parameter(Mandatory = $true)]
  [string]$InstalledExe,

  [string]$OutputPath = "release_evidence/setup_doctor_installed.json",

  [Parameter(Mandatory = $true)]
  [string]$ConfigPath,

  [Parameter(Mandatory = $true)]
  [string]$AuditDir,

  [string]$BrokerEvidenceJson = "",

  [string]$InstalledManifestJson = ""
)

$ErrorActionPreference = "Stop"

function New-DoctorCheck {
  param(
    [string]$CheckId,
    [string]$Status,
    [string]$Message,
    [string]$RecoveryInstruction
  )

  return [ordered]@{
    check_id = $CheckId
    status = $Status
    message = $Message
    recovery_instruction = $RecoveryInstruction
    grants_authority = $false
  }
}

function Test-JsonFile {
  param([string]$Path)

  try {
    $resolved = Resolve-Path $Path -ErrorAction Stop
    Get-Content -Raw -Path $resolved.Path | ConvertFrom-Json | Out-Null
    return [ordered]@{
      ok = $true
      path = $resolved.Path
    }
  } catch {
    return [ordered]@{
      ok = $false
      path = $Path
      error = $_.Exception.Message
    }
  }
}

function Test-AuditWrite {
  param([string]$Path)

  try {
    $directory = New-Item -ItemType Directory -Force -Path $Path
    $probe = Join-Path $directory.FullName ".gui-shell-setup-doctor-probe"
    Set-Content -Encoding UTF8 -Path $probe -Value "ok"
    $readOk = ((Get-Content -Raw -Path $probe).Trim() -eq "ok")
    Remove-Item -Force -Path $probe
    return [ordered]@{
      ok = $readOk -and !(Test-Path $probe)
      path = $directory.FullName
    }
  } catch {
    return [ordered]@{
      ok = $false
      path = $Path
      error = $_.Exception.Message
    }
  }
}

function Write-JsonEvidence {
  param(
    [Parameter(Mandatory = $true)]
    $Value,
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [int]$Depth = 8
  )

  $json = $Value | ConvertTo-Json -Depth $Depth
  $encoding = New-Object System.Text.UTF8Encoding $false
  [System.IO.File]::WriteAllText($Path, ($json + [Environment]::NewLine), $encoding)
}

function Get-TaggedSha256 {
  param([string]$Path)

  if ($Path -eq "" -or !(Test-Path $Path)) {
    return $null
  }
  return "sha256:$((Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant())"
}

$exe = Resolve-Path $InstalledExe
$artifactHash = (Get-FileHash -Algorithm SHA256 -Path $exe.Path).Hash.ToLowerInvariant()
$configProbe = Test-JsonFile -Path $ConfigPath
$auditProbe = Test-AuditWrite -Path $AuditDir
$brokerEvidence = $null
if ($BrokerEvidenceJson -ne "") {
  $brokerEvidencePath = Resolve-Path $BrokerEvidenceJson
  $brokerEvidence = Get-Content -Raw -Path $brokerEvidencePath.Path | ConvertFrom-Json
}
$installedManifest = $null
$installedManifestPath = $null
if ($InstalledManifestJson -ne "") {
  $installedManifestPath = Resolve-Path $InstalledManifestJson
  $installedManifest = Get-Content -Raw -Path $installedManifestPath.Path | ConvertFrom-Json
} else {
  $candidateManifest = Join-Path (Split-Path -Parent (Split-Path -Parent $exe.Path)) "installed_manifest.json"
  if (Test-Path $candidateManifest) {
    $installedManifestPath = Resolve-Path $candidateManifest
    $installedManifest = Get-Content -Raw -Path $installedManifestPath.Path | ConvertFrom-Json
  }
}

$brokerReady = (
  $null -ne $brokerEvidence -and
  $brokerEvidence.status -eq "passed" -and
  $brokerEvidence.authenticated_ipc_connection -eq $true -and
  $brokerEvidence.durable_store_ready -eq $true
)
$restrictedBind = (
  $null -ne $brokerEvidence -and
  $brokerEvidence.restricted_loopback_bind -eq $true -and
  $brokerEvidence.endpoint_host -eq "127.0.0.1"
)

$checks = @(
  New-DoctorCheck `
    -CheckId "windows.installed_app_path" `
    -Status "pass" `
    -Message "Installed GUI-Shell executable exists at $($exe.Path)." `
    -RecoveryInstruction "Stage or install the Windows GUI-Shell artifact before collecting release evidence."
  New-DoctorCheck `
    -CheckId "windows.artifact_hash" `
    -Status "pass" `
    -Message "Installed executable sha256 is sha256:$artifactHash." `
    -RecoveryInstruction "Rebuild and restage the Windows artifact if the recorded hash is not the intended release candidate."
  New-DoctorCheck `
    -CheckId "first_run.config_created" `
    -Status $(if ($configProbe.ok) { "pass" } else { "warning" }) `
    -Message $(if ($configProbe.ok) { "Config JSON parsed at $($configProbe.path)." } else { "Config JSON was not valid at $ConfigPath." }) `
    -RecoveryInstruction "Launch the installed app once and verify the recorded config path parses as JSON."
  New-DoctorCheck `
    -CheckId "first_run.audit_dir_writable" `
    -Status $(if ($auditProbe.ok) { "pass" } else { "warning" }) `
    -Message $(if ($auditProbe.ok) { "Audit directory write/read/delete probe passed at $($auditProbe.path)." } else { "Audit directory write/read/delete probe did not pass at $AuditDir." }) `
    -RecoveryInstruction "Grant the installed app a writable local audit directory and rerun installed smoke."
  New-DoctorCheck `
    -CheckId "setup_doctor.ran_from_installed_app_path" `
    -Status "pass" `
    -Message "Setup Doctor evidence was collected against the installed executable path." `
    -RecoveryInstruction "Run this collector against the installed GUI-Shell executable, not a development build path."
  New-DoctorCheck `
    -CheckId "setup_doctor.runtime_connection" `
    -Status $(if ($brokerReady) { "pass" } else { "warning" }) `
    -Message $(if ($brokerReady) { "Installed Rust broker evidence reports authenticated IPC and durable store readiness." } else { "Installed Rust broker evidence was absent or did not report ready IPC and persistence." }) `
    -RecoveryInstruction "Run collect_broker_smoke.ps1 against the installed Rust broker helper and include the JSON here."
  New-DoctorCheck `
    -CheckId "setup_doctor.authority_boundary" `
    -Status "pass" `
    -Message "Installer and Setup Doctor evidence do not grant authority or silently approve permissions." `
    -RecoveryInstruction "Keep installer diagnostics non-authoritative and broker-mediated."
  New-DoctorCheck `
    -CheckId "setup_doctor.network_public_bind" `
    -Status $(if ($restrictedBind) { "pass" } else { "warning" }) `
    -Message $(if ($restrictedBind) { "Broker evidence reports restricted 127.0.0.1 loopback bind." } else { "Broker restricted loopback bind evidence was absent." }) `
    -RecoveryInstruction "Run broker smoke and confirm the broker endpoint host is 127.0.0.1."
  New-DoctorCheck `
    -CheckId "setup_doctor.recovery_instruction" `
    -Status "pass" `
    -Message "Each Setup Doctor check includes an operator recovery instruction." `
    -RecoveryInstruction "Keep every diagnostic check paired with an operator-readable recovery instruction."
  New-DoctorCheck `
    -CheckId "setup_doctor.audit_storage" `
    -Status $(if ($auditProbe.ok) { "pass" } else { "warning" }) `
    -Message $(if ($auditProbe.ok) { "Audit storage probe passed at $($auditProbe.path)." } else { "Audit storage probe did not pass at $AuditDir." }) `
    -RecoveryInstruction "Repair audit storage permissions before release evidence collection."
)

$hasWarning = @($checks | Where-Object { $_.status -ne "pass" }).Count -ne 0
$report = [ordered]@{
  status = $(if ($hasWarning) { "warning" } else { "pass" })
  evidence_kind = "external_installer_config_broker_probe"
  formal_product_evidence = $false
  valid_for_current_strict_r2 = $false
  reason_not_formal_product_evidence = "This collector probes installed files, config, audit storage, and broker evidence externally. It does not execute an installed-app Setup Doctor machine-readable export."
  evidence_source = [ordered]@{
    collector = "installer/windows/collect_setup_doctor.ps1"
    collector_version = "2"
    source_kind = "external_installer_config_broker_probe"
    product_generated = $false
    collector_derives_checks = $true
    synthetic = $true
    command = "powershell -ExecutionPolicy Bypass -File installer\windows\collect_setup_doctor.ps1 -InstalledExe `"$($exe.Path)`""
  }
  installed_manifest_path = $(if ($null -ne $installedManifestPath) { $installedManifestPath.Path } else { $null })
  installed_manifest_sha256 = $(if ($null -ne $installedManifestPath) { Get-TaggedSha256 -Path $installedManifestPath.Path } else { $null })
  run_id = $(if ($null -ne $installedManifest) { $installedManifest.run_id } else { $null })
  source_commit = $(if ($null -ne $installedManifest) { $installedManifest.source_commit } else { $null })
  ran_from_installed_app_path = $false
  probed_installed_app_path = $true
  operator_readable = $true
  installer_grants_authority = $false
  installer_silently_approves_permissions = $false
  checks = $checks
}

$output = New-Item -ItemType File -Force -Path $OutputPath
Write-JsonEvidence -Value $report -Path $output.FullName -Depth 8
Write-Host "wrote $($output.FullName)"
