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

  [string]$VisibleSurfacesJson = "",

  [string]$VisibleSurfacesOutputPath = "",

  [int]$VisibleSurfaceWaitSeconds = 8,

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

function Write-JsonEvidence {
  param(
    [Parameter(Mandatory = $true)]
    $Value,
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [int]$Depth = 10
  )

  $json = $Value | ConvertTo-Json -Depth $Depth
  $encoding = New-Object System.Text.UTF8Encoding $false
  [System.IO.File]::WriteAllText($Path, ($json + [Environment]::NewLine), $encoding)
}

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

function Collect-VisibleSurfaces {
  param(
    [System.Diagnostics.Process]$Process,
    [string]$OutputPath,
    [int]$WaitSeconds
  )

  Add-Type -AssemblyName UIAutomationClient
  Add-Type -AssemblyName UIAutomationTypes
  $expected = @("Dashboard", "NavigationRail", "Runtime Status", "Invariant Status")
  $aggregatePhrase = "GUI Shell Dashboard NavigationRail Runtime Status Invariant Status"

  function Normalize-SurfaceText {
    param([string]$Text)
    return (($Text -replace "\s+", " ").Trim())
  }

  function Test-SurfaceTextContains {
    param(
      [string]$Text,
      [string]$Label
    )
    if ($null -eq $Text -or $Text.Trim() -eq "") {
      return $false
    }
    return ($Text -match [regex]::Escape($Label))
  }

  function Get-ElementString {
    param(
      $Element,
      [string]$PropertyName
    )
    try {
      $value = $Element.Current.$PropertyName
      if ($null -eq $value) {
        return ""
      }
      return $value.ToString().Trim()
    } catch {
      return ""
    }
  }

  function Get-ControlTypeName {
    param($Element)
    try {
      $value = $Element.Current.ControlType.ProgrammaticName
      if ($null -eq $value) {
        return ""
      }
      return $value.ToString().Trim()
    } catch {
      return ""
    }
  }

  function New-ObservedElement {
    param(
      $Element,
      [string]$ElementKey,
      [bool]$IsRoot
    )
    $name = Get-ElementString -Element $Element -PropertyName "Name"
    $automationId = Get-ElementString -Element $Element -PropertyName "AutomationId"
    $className = Get-ElementString -Element $Element -PropertyName "ClassName"
    $frameworkId = Get-ElementString -Element $Element -PropertyName "FrameworkId"
    $controlType = Get-ControlTypeName -Element $Element
    $searchText = Normalize-SurfaceText -Text "$name $automationId"
    $surfacesPresent = @()
    foreach ($label in $expected) {
      if (Test-SurfaceTextContains -Text $searchText -Label $label) {
        $surfacesPresent += $label
      }
    }
    $isNativeContainer = (
      $IsRoot -or
      $controlType -in @("ControlType.Window", "ControlType.Pane") -or
      $className -match "(?i)(Flutter|Window)"
    )
    return [pscustomobject][ordered]@{
      element_key = $ElementKey
      name = $name
      automation_id = $automationId
      control_type = $controlType
      class_name = $className
      framework_id = $frameworkId
      is_root = $IsRoot
      is_native_container = [bool]$isNativeContainer
      surfaces_present = @($surfacesPresent)
      surface_count = $surfacesPresent.Count
      contains_all_required_surfaces = [bool]($surfacesPresent.Count -eq $expected.Count)
    }
  }

  $window = $null
  $deadline = (Get-Date).AddSeconds($WaitSeconds)
  $condition = New-Object System.Windows.Automation.PropertyCondition `
    -ArgumentList ([System.Windows.Automation.AutomationElement]::ProcessIdProperty), ([int]$Process.Id)
  while ((Get-Date) -lt $deadline -and $null -eq $window) {
    $Process.Refresh()
    if ($Process.HasExited) {
      break
    }
    $window = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
      [System.Windows.Automation.TreeScope]::Children,
      $condition
    )
    if ($null -eq $window) {
      Start-Sleep -Milliseconds 250
    }
  }

  $names = New-Object System.Collections.Generic.List[string]
  $observedElements = New-Object System.Collections.Generic.List[object]
  if ($null -ne $window) {
    $observedElements.Add((New-ObservedElement -Element $window -ElementKey "root" -IsRoot $true))
    $elements = $window.FindAll(
      [System.Windows.Automation.TreeScope]::Descendants,
      [System.Windows.Automation.Condition]::TrueCondition
    )
    for ($index = 0; $index -lt $elements.Count; $index += 1) {
      $element = $elements.Item($index)
      $observedElements.Add(
        (New-ObservedElement -Element $element -ElementKey "descendant:$index" -IsRoot $false)
      )
      foreach ($value in @(
          $element.Current.Name,
          $element.Current.AutomationId,
          $element.Current.ControlType.ProgrammaticName
        )) {
        if ($null -ne $value -and $value.ToString().Trim() -ne "") {
          $names.Add($value.ToString().Trim())
        }
      }
    }
  }

  $aggregateSurfaceShortcutDetected = $false
  foreach ($observed in $observedElements) {
    $aggregateText = Normalize-SurfaceText -Text "$($observed.name) $($observed.automation_id)"
    if ($observed.contains_all_required_surfaces) {
      $aggregateSurfaceShortcutDetected = $true
    }
    if ($aggregateText -match [regex]::Escape($aggregatePhrase)) {
      $aggregateSurfaceShortcutDetected = $true
    }
  }

  $surfaceMatches = [ordered]@{}
  $visible = @()
  foreach ($label in $expected) {
    $candidates = @($observedElements | Where-Object { $_.surfaces_present -contains $label })
    $preferred = $null
    if ($candidates.Count -gt 0) {
      $preferred = @(
        $candidates |
          Where-Object { $_.surface_count -eq 1 -and !$_.is_root } |
          Select-Object -First 1
      )
      if ($preferred.Count -eq 0) {
        $preferred = @(
          $candidates |
            Where-Object { !$_.contains_all_required_surfaces -and !$_.is_root } |
            Select-Object -First 1
        )
      }
      if ($preferred.Count -eq 0) {
        $preferred = @($candidates | Select-Object -First 1)
      }
      $preferred = $preferred[0]
      $visible += $label
      $surfaceMatches[$label] = [ordered]@{
        matched = $true
        name = $preferred.name
        automation_id = $preferred.automation_id
        control_type = $preferred.control_type
        class_name = $preferred.class_name
        framework_id = $preferred.framework_id
        element_key = $preferred.element_key
        is_root = $preferred.is_root
        is_native_container = $preferred.is_native_container
        surfaces_present = @($preferred.surfaces_present)
      }
    } else {
      $surfaceMatches[$label] = [ordered]@{
        matched = $false
        name = ""
        automation_id = ""
        control_type = ""
        class_name = ""
        framework_id = ""
        element_key = ""
        is_root = $false
        is_native_container = $false
        surfaces_present = @()
      }
    }
  }
  $matchedElementKeys = @()
  foreach ($label in $expected) {
    $match = $surfaceMatches[$label]
    if ($match["matched"] -eq $true) {
      $matchedElementKeys += $match["element_key"]
    }
  }
  $singleAggregateElement = $false
  if ($matchedElementKeys.Count -eq $expected.Count) {
    $uniqueMatchedElementKeys = @($matchedElementKeys | Select-Object -Unique)
    $singleAggregateElement = ($uniqueMatchedElementKeys.Count -eq 1)
  }
  if ($singleAggregateElement) {
    $aggregateSurfaceShortcutDetected = $true
  }
  $surfaceMatchRequirementsMet = (
    $visible.Count -eq $expected.Count -and
    !$aggregateSurfaceShortcutDetected -and
    !$singleAggregateElement
  )
  $capture = [ordered]@{
    source = "uiautomation"
    path = $OutputPath
    captured_at = (Get-Date).ToUniversalTime().ToString("o")
    process_id = $Process.Id
    window_found = ($null -ne $window)
    window_title = $(if ($null -ne $window) { $window.Current.Name } else { "" })
    expected_surfaces = $expected
    visible_surfaces = @($visible)
    surface_matches = $surfaceMatches
    aggregate_surface_shortcut_detected = [bool]$aggregateSurfaceShortcutDetected
    surface_match_requirements_met = [bool]$surfaceMatchRequirementsMet
    automation_names = @($names | Select-Object -Unique | Select-Object -First 200)
  }
  $output = New-Item -ItemType File -Force -Path $OutputPath
  Write-JsonEvidence -Value $capture -Path $output.FullName -Depth 8
  return $capture
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
if ($VisibleSurfacesJson -ne "") {
  $visibleSurfacesPath = Resolve-Path $VisibleSurfacesJson
  $visibleSurfaceEvidence = Get-Content -Raw -Path $visibleSurfacesPath.Path | ConvertFrom-Json
} else {
  if ($VisibleSurfacesOutputPath -eq "") {
    $outputDirectory = Split-Path -Parent $OutputPath
    if ($outputDirectory -eq "") {
      $outputDirectory = "."
    }
    $VisibleSurfacesOutputPath = Join-Path $outputDirectory "visible_surfaces_collected.json"
  }
  $visibleSurfaceEvidence = Collect-VisibleSurfaces `
    -Process $process `
    -OutputPath $VisibleSurfacesOutputPath `
    -WaitSeconds $VisibleSurfaceWaitSeconds
}
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
  $auditWriteProbe.read = ((Get-Content -Raw -Path $probePath).Trim() -eq "ok")
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

function Get-EvidenceValue {
  param(
    $Object,
    [string]$Name
  )
  if ($null -eq $Object) {
    return $null
  }
  if ($Object -is [System.Collections.IDictionary]) {
    return $Object[$Name]
  }
  $property = $Object.PSObject.Properties[$Name]
  if ($null -eq $property) {
    return $null
  }
  return $property.Value
}

$requiredVisibleSurfaces = @("Dashboard", "NavigationRail", "Runtime Status", "Invariant Status")
$visibleSurfaceLabels = @(Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "visible_surfaces")
$surfaceMatchesEvidence = Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "surface_matches"
$aggregateSurfaceShortcutDetected = (
  Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "aggregate_surface_shortcut_detected"
) -eq $true
$surfaceMatchRequirementsMet = (
  Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "surface_match_requirements_met"
) -eq $true
$visibleSurfacesComplete = $true
foreach ($surface in $requiredVisibleSurfaces) {
  if ($visibleSurfaceLabels -notcontains $surface) {
    $visibleSurfacesComplete = $false
  }
  $surfaceMatch = Get-EvidenceValue -Object $surfaceMatchesEvidence -Name $surface
  if ($null -eq $surfaceMatch) {
    $visibleSurfacesComplete = $false
  } elseif ((Get-EvidenceValue -Object $surfaceMatch -Name "matched") -ne $true) {
    $visibleSurfacesComplete = $false
  } else {
    $elementKey = Get-EvidenceValue -Object $surfaceMatch -Name "element_key"
    if ($null -eq $elementKey -or $elementKey -eq "") {
      $visibleSurfacesComplete = $false
    }
  }
}
if ($aggregateSurfaceShortcutDetected -or !$surfaceMatchRequirementsMet) {
  $visibleSurfacesComplete = $false
}

$evidence = [ordered]@{
  platform = "windows"
  collected_at = (Get-Date).ToUniversalTime().ToString("o")
  evidence_source = [ordered]@{
    collector = "installer/windows/collect_installed_smoke.ps1"
    collector_version = "4"
    manual_confirmation = $false
    screenshot_path = $(if ($ScreenshotPath -ne "") { $ScreenshotPath } else { $null })
  }
  artifact = [ordered]@{
    installed_exe_path = $exe.Path
    installed_exe_exists = $true
    sha256 = "sha256:$hash"
  }
  first_run = [ordered]@{
    status = $(if ($firstWindowVisible -and $configCreated -and $auditDirWritable -and $visibleSurfacesComplete) { "passed" } else { "failed" })
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
    visible_surfaces_complete = $visibleSurfacesComplete
    visible_surfaces = @($visibleSurfaceLabels)
    visible_surfaces_evidence = [ordered]@{
      source = Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "source"
      path = Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "path"
      captured_at = Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "captured_at"
      surface_matches = $surfaceMatchesEvidence
      aggregate_surface_shortcut_detected = [bool]$aggregateSurfaceShortcutDetected
      surface_match_requirements_met = [bool]$surfaceMatchRequirementsMet
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
Write-JsonEvidence -Value $evidence -Path $output.FullName -Depth 10

if ($null -ne $process -and !$process.HasExited) {
  Stop-Process -Id $process.Id
}
Stop-SmokeBroker -Process $brokerProcess

Write-Host "wrote $($output.FullName)"
