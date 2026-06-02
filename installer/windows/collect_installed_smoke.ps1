param(
  [Parameter(Mandatory = $true)]
  [string]$InstalledExe,

  [string]$OutputPath = "release_evidence/windows_installed_smoke.json",
  [Parameter(Mandatory = $true)]
  [string]$SetupDoctorJson,

  [Parameter(Mandatory = $true)]
  [string]$ConfigPath,

  [Parameter(Mandatory = $true)]
  [string]$AuditDir,

  [Parameter(Mandatory = $true)]
  [string]$VisibleSurfacesJson,

  [string]$BrokerEvidenceJson = "",

  [string]$RuntimeAssertionsJson = "",

  [string]$ScreenshotPath = ""
)

$ErrorActionPreference = "Stop"

$exe = Resolve-Path $InstalledExe
$hash = (Get-FileHash -Algorithm SHA256 -Path $exe).Hash.ToLowerInvariant()
$setupDoctorPath = Resolve-Path $SetupDoctorJson
$visibleSurfacesPath = Resolve-Path $VisibleSurfacesJson

$process = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 3
$process.Refresh()

$setupDoctor = Get-Content -Raw -Path $setupDoctorPath | ConvertFrom-Json
$visibleSurfaceEvidence = Get-Content -Raw -Path $visibleSurfacesPath | ConvertFrom-Json
$brokerEvidence = $null
if ($BrokerEvidenceJson -ne "") {
  $brokerEvidencePath = Resolve-Path $BrokerEvidenceJson
  $brokerEvidence = Get-Content -Raw -Path $brokerEvidencePath | ConvertFrom-Json
}
$runtimeAssertions = $null
if ($RuntimeAssertionsJson -ne "") {
  $runtimeAssertionsPath = Resolve-Path $RuntimeAssertionsJson
  $runtimeAssertions = Get-Content -Raw -Path $runtimeAssertionsPath | ConvertFrom-Json
}

$mainWindowHandle = 0
$windowTitle = ""
if (!$process.HasExited) {
  $mainWindowHandle = $process.MainWindowHandle
  $windowTitle = $process.MainWindowTitle
}

$resolvedConfigPath = Resolve-Path $ConfigPath -ErrorAction SilentlyContinue
$configJsonValid = $false
if ($null -ne $resolvedConfigPath) {
  try {
    Get-Content -Raw -Path $resolvedConfigPath | ConvertFrom-Json | Out-Null
    $configJsonValid = $true
  } catch {
    $configJsonValid = $false
  }
}

$resolvedAuditDir = Resolve-Path $AuditDir -ErrorAction SilentlyContinue
$auditWriteProbe = [ordered]@{
  attempted = $false
  write = $false
  read = $false
  delete = $false
  probe_path = $null
}
if ($null -ne $resolvedAuditDir) {
  $probePath = Join-Path $resolvedAuditDir ".gui-shell-write-probe"
  $auditWriteProbe.attempted = $true
  $auditWriteProbe.probe_path = $probePath
  Set-Content -Encoding UTF8 -Path $probePath -Value "ok"
  $auditWriteProbe.write = Test-Path $probePath
  $auditWriteProbe.read = ((Get-Content -Raw -Path $probePath) -eq "ok")
  Remove-Item -Force -Path $probePath
  $auditWriteProbe.delete = !(Test-Path $probePath)
}

$firstWindowVisible = (!$process.HasExited -and $mainWindowHandle -ne 0)
$configCreated = ($null -ne $resolvedConfigPath -and $configJsonValid)
$auditDirWritable = (
  $auditWriteProbe.attempted -and
  $auditWriteProbe.write -and
  $auditWriteProbe.read -and
  $auditWriteProbe.delete
)

$evidence = [ordered]@{
  platform = "windows"
  collected_at = (Get-Date).ToUniversalTime().ToString("o")
  evidence_source = [ordered]@{
    collector = "installer/windows/collect_installed_smoke.ps1"
    collector_version = "2"
    manual_confirmation = $false
    screenshot_path = $(if ($ScreenshotPath -ne "") { $ScreenshotPath } else { $null })
  }
  artifact = [ordered]@{
    installed_exe_path = $exe.Path
    installed_exe_exists = $true
    sha256 = "sha256:$hash"
  }
  first_run = [ordered]@{
    status = $(if ($firstWindowVisible -and $configCreated -and $auditDirWritable) { "passed" } else { "failed" })
    command = "& `"$($exe.Path)`""
    launched_from_installed_path = $true
    process_id = $process.Id
    process_running_after_launch = !$process.HasExited
    main_window_handle = $mainWindowHandle
    window_title = $windowTitle
    first_window_visible = $firstWindowVisible
    visible_surfaces = @($visibleSurfaceEvidence.visible_surfaces)
    visible_surfaces_evidence = [ordered]@{
      source = $visibleSurfaceEvidence.source
      path = $visibleSurfaceEvidence.path
      captured_at = $visibleSurfaceEvidence.captured_at
    }
    config_path = $(if ($null -ne $resolvedConfigPath) { $resolvedConfigPath.Path } else { $ConfigPath })
    config_created = $configCreated
    config_json_valid = $configJsonValid
    audit_dir = $(if ($null -ne $resolvedAuditDir) { $resolvedAuditDir.Path } else { $AuditDir })
    audit_dir_writable = $auditDirWritable
    audit_write_probe = $auditWriteProbe
    installer_grants_authority = $false
    installer_silently_approves_permissions = $false
  }
  setup_doctor = $setupDoctor
  broker = $(if ($null -ne $brokerEvidence) {
      $brokerEvidence
    } else {
      [ordered]@{
        status = "missing"
        evidence_source = [ordered]@{
          collector = "installer/windows/collect_broker_smoke.ps1"
          collector_version = "missing"
          synthetic = $true
          command = $null
        }
        authenticated_ipc_connection = $false
        durable_store_ready = $false
        restart_replay_rejected = $false
        crash_fail_closed = $false
        python_runtime_required_for_authority = $true
        flutter_rust_ffi_authority_bridge = $true
      }
    })
  release_runtime_assertions = $runtimeAssertions
}

$output = New-Item -ItemType File -Force -Path $OutputPath
$evidence | ConvertTo-Json -Depth 10 | Set-Content -Encoding UTF8 -Path $output.FullName

if (!$process.HasExited) {
  Stop-Process -Id $process.Id
}

Write-Host "wrote $($output.FullName)"
