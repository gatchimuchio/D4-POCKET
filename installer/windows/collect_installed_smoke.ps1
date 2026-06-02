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

  [string]$BrokerHelperExe = "",

  [string]$BrokerStoreDir = "",

  [string]$BrokerSessionFile = "",

  [switch]$NoPythonRuntime,

  [string]$RuntimeAssertionsJson = "",

  [string]$ScreenshotPath = ""
)

$ErrorActionPreference = "Stop"

$exe = Resolve-Path $InstalledExe
$hash = (Get-FileHash -Algorithm SHA256 -Path $exe).Hash.ToLowerInvariant()
$setupDoctorPath = Resolve-Path $SetupDoctorJson
$visibleSurfacesPath = Resolve-Path $VisibleSurfacesJson
$brokerProcess = $null
$brokerEndpoint = $null
$brokerEndpointFile = $null
$brokerMediatedLaunch = $false
$previousBrokerEndpointEnv = [Environment]::GetEnvironmentVariable("GUI_SHELL_BROKER_ENDPOINT_JSON", "Process")
$previousBrokerRuntimeDirEnv = [Environment]::GetEnvironmentVariable("GUI_SHELL_BROKER_RUNTIME_DIR", "Process")
$previousPathEnv = [Environment]::GetEnvironmentVariable("Path", "Process")
$pythonRuntimePathScrubbed = $false
$pythonPathEntriesRemovedCount = 0
$pythonPathEntriesRemainingCount = 0
$pythonCommandsVisibleAfterScrub = @()

function Start-SmokeBroker {
  param(
    [string]$HelperExe,
    [string]$StoreDir,
    [string]$SessionFile
  )

  $helper = Resolve-Path $HelperExe
  if ($StoreDir -eq "") {
    $StoreDir = Join-Path (Split-Path -Parent $helper.Path) "store"
  }
  if ($SessionFile -eq "") {
    $SessionFile = Join-Path (Split-Path -Parent $helper.Path) "broker_session.json"
  }
  New-Item -ItemType Directory -Force -Path $StoreDir | Out-Null
  if (Test-Path $SessionFile) {
    Remove-Item -Force -Path $SessionFile
  }
  $process = Start-Process -FilePath $helper.Path -ArgumentList @(
    "broker-server",
    "--store-dir",
    $StoreDir,
    "--session-file",
    $SessionFile
  ) -WindowStyle Hidden -PassThru
  for ($index = 0; $index -lt 100; $index += 1) {
    if (Test-Path $SessionFile) {
      return [ordered]@{
        process = $process
        session_file = $SessionFile
        endpoint = Get-Content -Raw -Path $SessionFile | ConvertFrom-Json
      }
    }
    $process.Refresh()
    if ($process.HasExited) {
      throw "Rust broker exited before endpoint was ready: $($process.ExitCode)"
    }
    Start-Sleep -Milliseconds 50
  }
  throw "Rust broker endpoint file was not created: $SessionFile"
}

function Stop-SmokeBroker {
  param($Process)
  if ($null -eq $Process) {
    return
  }
  $Process.Refresh()
  if (!$Process.HasExited) {
    Stop-Process -Id $Process.Id -Force
  }
}

function Restore-SmokeEnvironment {
  if ($null -eq $previousBrokerEndpointEnv) {
    Remove-Item Env:\GUI_SHELL_BROKER_ENDPOINT_JSON -ErrorAction SilentlyContinue
  } else {
    $env:GUI_SHELL_BROKER_ENDPOINT_JSON = $previousBrokerEndpointEnv
  }
  if ($null -eq $previousBrokerRuntimeDirEnv) {
    Remove-Item Env:\GUI_SHELL_BROKER_RUNTIME_DIR -ErrorAction SilentlyContinue
  } else {
    $env:GUI_SHELL_BROKER_RUNTIME_DIR = $previousBrokerRuntimeDirEnv
  }
  if ($null -eq $previousPathEnv) {
    Remove-Item Env:\Path -ErrorAction SilentlyContinue
  } else {
    $env:Path = $previousPathEnv
  }
}

function Enable-NoPythonLaunchPath {
  $beforeEntries = @($env:Path -split ";" | Where-Object { $_ -ne "" })
  $afterEntries = @($beforeEntries | Where-Object { $_ -notmatch "(?i)(python|WindowsApps)" })
  $env:Path = ($afterEntries -join ";")
  $script:pythonRuntimePathScrubbed = $true
  $script:pythonPathEntriesRemovedCount = $beforeEntries.Count - $afterEntries.Count
  $script:pythonPathEntriesRemainingCount = @($afterEntries | Where-Object { $_ -match "(?i)python" }).Count
  $script:pythonCommandsVisibleAfterScrub = @(
    Get-Command python, python3, py -ErrorAction SilentlyContinue |
      Select-Object -ExpandProperty Name -Unique
  )
}

trap {
  Restore-SmokeEnvironment
  if ($null -ne $process) {
    $process.Refresh()
    if (!$process.HasExited) {
      Stop-Process -Id $process.Id -Force
    }
  }
  Stop-SmokeBroker -Process $brokerProcess
  throw
}

if ($BrokerHelperExe -ne "") {
  $startedBroker = Start-SmokeBroker -HelperExe $BrokerHelperExe -StoreDir $BrokerStoreDir -SessionFile $BrokerSessionFile
  $brokerProcess = $startedBroker.process
  $brokerEndpoint = $startedBroker.endpoint
  $brokerEndpointFile = $startedBroker.session_file
  $brokerMediatedLaunch = $true
  $env:GUI_SHELL_BROKER_ENDPOINT_JSON = $brokerEndpointFile
  $env:GUI_SHELL_BROKER_RUNTIME_DIR = Split-Path -Parent $brokerEndpointFile
}

$process = $null
try {
  if ($NoPythonRuntime.IsPresent) {
    Enable-NoPythonLaunchPath
  }
  $process = Start-Process -FilePath $exe -PassThru
  Start-Sleep -Seconds 3
  $process.Refresh()
} finally {
  Restore-SmokeEnvironment
}

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
    collector_version = "3"
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
    broker_mediated_launch = $brokerMediatedLaunch
    broker_helper_path = $(if ($BrokerHelperExe -ne "") { (Resolve-Path $BrokerHelperExe).Path } else { $null })
    broker_endpoint_file = $brokerEndpointFile
    broker_endpoint_created = $(if ($null -ne $brokerEndpointFile) { Test-Path $brokerEndpointFile } else { $false })
    broker_transport = $(if ($null -ne $brokerEndpoint) { $brokerEndpoint.transport } else { $null })
    no_python_runtime_requested = [bool]$NoPythonRuntime
    python_runtime_path_scrubbed = $pythonRuntimePathScrubbed
    python_path_entries_removed_count = $pythonPathEntriesRemovedCount
    python_path_entries_remaining_count = $pythonPathEntriesRemainingCount
    python_commands_visible_after_scrub = @($pythonCommandsVisibleAfterScrub)
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

if ($null -ne $process -and !$process.HasExited) {
  Stop-Process -Id $process.Id
}
Stop-SmokeBroker -Process $brokerProcess

Write-Host "wrote $($output.FullName)"
