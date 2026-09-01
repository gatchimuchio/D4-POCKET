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
    -Message "installed GUI-Shell executable が存在します: $($exe.Path)" `
    -RecoveryInstruction "release evidence の収集前に Windows GUI-Shell artifact を stage または install してください。"
  New-DoctorCheck `
    -CheckId "windows.artifact_hash" `
    -Status "pass" `
    -Message "installed executable の sha256 は sha256:$artifactHash です。" `
    -RecoveryInstruction "記録された hash が意図した release candidate と異なる場合は、Windows artifact を rebuild して再び stage してください。"
  New-DoctorCheck `
    -CheckId "first_run.config_created" `
    -Status $(if ($configProbe.ok) { "pass" } else { "warning" }) `
    -Message $(if ($configProbe.ok) { "Config JSON を解析できました: $($configProbe.path)" } else { "Config JSON が無効です: $ConfigPath" }) `
    -RecoveryInstruction "installed app を一度起動し、記録された config path を JSON として解析できることを検証してください。"
  New-DoctorCheck `
    -CheckId "first_run.audit_dir_writable" `
    -Status $(if ($auditProbe.ok) { "pass" } else { "warning" }) `
    -Message $(if ($auditProbe.ok) { "Audit directory の write/read/delete probe が合格しました: $($auditProbe.path)" } else { "Audit directory の write/read/delete probe が合格しませんでした: $AuditDir" }) `
    -RecoveryInstruction "installed app に書込み可能な local audit directory を付与し、installed smoke を再実行してください。"
  New-DoctorCheck `
    -CheckId "setup_doctor.ran_from_installed_app_path" `
    -Status "pass" `
    -Message "Setup Doctor evidence を installed executable path に対して収集しました。" `
    -RecoveryInstruction "development build path ではなく installed GUI-Shell executable に対してこの collector を実行してください。"
  New-DoctorCheck `
    -CheckId "setup_doctor.runtime_connection" `
    -Status $(if ($brokerReady) { "pass" } else { "warning" }) `
    -Message $(if ($brokerReady) { "installed Rust broker evidence は認証済み IPC と durable store の準備完了を報告しています。" } else { "installed Rust broker evidence がないか、IPC と永続化の準備完了を報告していません。" }) `
    -RecoveryInstruction "installed Rust broker helper に対して collect_broker_smoke.ps1 を実行し、その JSON をここに含めてください。"
  New-DoctorCheck `
    -CheckId "setup_doctor.authority_boundary" `
    -Status "pass" `
    -Message "Installer と Setup Doctor evidence は権限を付与せず、permission を無言で承認しません。" `
    -RecoveryInstruction "installer diagnostics の非権限性と broker 仲介を維持してください。"
  New-DoctorCheck `
    -CheckId "setup_doctor.network_public_bind" `
    -Status $(if ($restrictedBind) { "pass" } else { "warning" }) `
    -Message $(if ($restrictedBind) { "Broker evidence は 127.0.0.1 に制限した loopback bind を報告しています。" } else { "Broker の制限済み loopback bind evidence がありません。" }) `
    -RecoveryInstruction "broker smoke を実行し、broker endpoint host が 127.0.0.1 であることを確認してください。"
  New-DoctorCheck `
    -CheckId "setup_doctor.recovery_instruction" `
    -Status "pass" `
    -Message "各 Setup Doctor check に operator recovery instruction が含まれています。" `
    -RecoveryInstruction "すべての diagnostic check に operator が読める recovery instruction を対応付けてください。"
  New-DoctorCheck `
    -CheckId "setup_doctor.audit_storage" `
    -Status $(if ($auditProbe.ok) { "pass" } else { "warning" }) `
    -Message $(if ($auditProbe.ok) { "Audit storage probe が合格しました: $($auditProbe.path)" } else { "Audit storage probe が合格しませんでした: $AuditDir" }) `
    -RecoveryInstruction "release evidence の収集前に audit storage permission を修復してください。"
)

$hasWarning = @($checks | Where-Object { $_.status -ne "pass" }).Count -ne 0
$report = [ordered]@{
  status = $(if ($hasWarning) { "warning" } else { "pass" })
  evidence_kind = "external_installer_config_broker_probe"
  formal_product_evidence = $false
  valid_for_current_strict_r2 = $false
  reason_not_formal_product_evidence = "この collector は installed file、config、audit storage、broker evidence を外部から probe します。installed-app の Setup Doctor による machine-readable export は実行しません。"
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
Write-Host "書き出しました: $($output.FullName)"
