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

  [string]$AuditAnchorEvidenceJson = "",

  [string]$ScreenshotPath = "",

  [string]$InstalledManifestJson = "",

  [switch]$DiagnosticOnly
)

$ErrorActionPreference = "Stop"

$exe = Resolve-Path $InstalledExe
$hash = (Get-FileHash -Algorithm SHA256 -Path $exe).Hash.ToLowerInvariant()
$brokerProcess = $null
$brokerEndpoint = $null
$brokerEndpointFile = $null
$brokerMediatedLaunch = $false
$previousBrokerEndpointEnv = [Environment]::GetEnvironmentVariable("GUI_SHELL_BROKER_ENDPOINT_JSON", "Process")
$previousBrokerRuntimeDirEnv = [Environment]::GetEnvironmentVariable("GUI_SHELL_BROKER_RUNTIME_DIR", "Process")
$previousSetupDoctorExportEnv = [Environment]::GetEnvironmentVariable("GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON", "Process")
$previousSetupDoctorContextEnv = [Environment]::GetEnvironmentVariable("GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON", "Process")
$previousSurfaceSemanticsExportEnv = [Environment]::GetEnvironmentVariable("GUI_SHELL_SURFACE_SEMANTICS_EXPORT_JSON", "Process")
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

function Get-TaggedSha256 {
  param([string]$Path)

  if ($Path -eq "" -or !(Test-Path $Path)) {
    return $null
  }
  return "sha256:$((Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant())"
}

function Get-TaggedStringSha256 {
  param([string]$Text)

  $encoding = New-Object System.Text.UTF8Encoding $false
  $bytes = $encoding.GetBytes($Text)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    return "sha256:$(([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant())"
  } finally {
    $sha.Dispose()
  }
}

function Resolve-InputOrOutputPath {
  param([string]$Path)

  if (Test-Path $Path) {
    return (Resolve-Path $Path).Path
  }
  $fullPath = [System.IO.Path]::GetFullPath($Path)
  $directory = Split-Path -Parent $fullPath
  if ($directory -ne "") {
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
  }
  return $fullPath
}

function Write-SetupDoctorProductContext {
  param(
    [string]$Path,
    [string]$InstalledAppPath,
    [string]$AppArtifactSha256,
    [string]$ConfigPathValue,
    [string]$AuditDirValue,
    [bool]$BrokerMediated,
    [string]$BrokerEndpointFileValue,
    $BrokerEndpointValue
  )

  $endpointHost = $null
  if ($null -ne $BrokerEndpointValue) {
    $endpointHost = Get-EvidenceValue -Object $BrokerEndpointValue -Name "host"
    if ($null -eq $endpointHost) {
      $endpointHost = Get-EvidenceValue -Object $BrokerEndpointValue -Name "endpoint_host"
    }
  }
  $context = [ordered]@{
    context_kind = "installed_app_setup_doctor_context"
    context_version = 1
    installed_app_path = $InstalledAppPath
    installed_app_path_confirmed = $true
    app_artifact_sha256 = $AppArtifactSha256
    config_path = [System.IO.Path]::GetFullPath($ConfigPathValue)
    audit_dir = [System.IO.Path]::GetFullPath($AuditDirValue)
    broker_mediated_launch = $BrokerMediated
    broker_endpoint_file = $BrokerEndpointFileValue
    restricted_loopback_bind = ($endpointHost -eq "127.0.0.1")
  }
  Write-JsonEvidence -Value $context -Path $Path -Depth 8
}

function New-EvidenceFileRecord {
  param(
    [string]$Kind,
    [string]$Path
  )

  if ($Path -eq "") {
    return $null
  }
  $resolved = Resolve-Path $Path -ErrorAction SilentlyContinue
  if ($null -eq $resolved) {
    return [ordered]@{
      kind = $Kind
      path = $Path
      exists = $false
      sha256 = $null
    }
  }
  return [ordered]@{
    kind = $Kind
    path = $resolved.Path
    exists = $true
    sha256 = Get-TaggedSha256 -Path $resolved.Path
  }
}

function Find-InstalledManifestPath {
  param([string]$ExePath)

  if ($InstalledManifestJson -ne "") {
    return (Resolve-Path $InstalledManifestJson).Path
  }
  $candidate = Join-Path (Split-Path -Parent (Split-Path -Parent $ExePath)) "installed_manifest.json"
  if (Test-Path $candidate) {
    return (Resolve-Path $candidate).Path
  }
  return $null
}

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
      throw "Rust broker が endpoint 準備前に終了しました: $($process.ExitCode)"
    }
    Start-Sleep -Milliseconds 50
  }
  throw "Rust broker endpoint ファイルが作成されませんでした: $SessionFile"
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
  if ($null -eq $previousSetupDoctorExportEnv) {
    Remove-Item Env:\GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON -ErrorAction SilentlyContinue
  } else {
    $env:GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON = $previousSetupDoctorExportEnv
  }
  if ($null -eq $previousSetupDoctorContextEnv) {
    Remove-Item Env:\GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON -ErrorAction SilentlyContinue
  } else {
    $env:GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON = $previousSetupDoctorContextEnv
  }
  if ($null -eq $previousSurfaceSemanticsExportEnv) {
    Remove-Item Env:\GUI_SHELL_SURFACE_SEMANTICS_EXPORT_JSON -ErrorAction SilentlyContinue
  } else {
    $env:GUI_SHELL_SURFACE_SEMANTICS_EXPORT_JSON = $previousSurfaceSemanticsExportEnv
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

  function Get-RuntimeIdString {
    param($Element)
    try {
      $runtimeId = $Element.GetRuntimeId()
      if ($null -eq $runtimeId) {
        return ""
      }
      return (($runtimeId | ForEach-Object { $_.ToString() }) -join ".")
    } catch {
      return ""
    }
  }

  function Get-ParentRuntimeIdString {
    param($Element)
    try {
      $parent = [System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($Element)
      if ($null -eq $parent) {
        return ""
      }
      return Get-RuntimeIdString -Element $parent
    } catch {
      return ""
    }
  }

  function Get-SupportedPatternNames {
    param($Element)
    try {
      return @(
        $Element.GetSupportedPatterns() |
          ForEach-Object { $_.ProgrammaticName.ToString() } |
          Where-Object { $_ -ne "" }
      )
    } catch {
      return @()
    }
  }

  function Get-BoundingRectangleEvidence {
    param($Element)
    try {
      $rect = $Element.Current.BoundingRectangle
      return [ordered]@{
        x = $rect.X
        y = $rect.Y
        width = $rect.Width
        height = $rect.Height
      }
    } catch {
      return [ordered]@{
        x = 0
        y = 0
        width = 0
        height = 0
      }
    }
  }

  function Get-ElementBool {
    param(
      $Element,
      [string]$PropertyName
    )
    try {
      return [bool]$Element.Current.$PropertyName
    } catch {
      return $false
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
    $runtimeId = Get-RuntimeIdString -Element $Element
    $parentRuntimeId = $(if ($IsRoot) { "" } else { Get-ParentRuntimeIdString -Element $Element })
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
      runtime_id = $runtimeId
      parent_runtime_id = $parentRuntimeId
      name = $name
      automation_id = $automationId
      control_type = $controlType
      class_name = $className
      framework_id = $frameworkId
      localized_control_type = Get-ElementString -Element $Element -PropertyName "LocalizedControlType"
      help_text = Get-ElementString -Element $Element -PropertyName "HelpText"
      is_offscreen = Get-ElementBool -Element $Element -PropertyName "IsOffscreen"
      bounding_rectangle = Get-BoundingRectangleEvidence -Element $Element
      supported_patterns = @(Get-SupportedPatternNames -Element $Element)
      is_root = $IsRoot
      is_native_container = [bool]$isNativeContainer
      surfaces_present = @($surfacesPresent)
      surface_count = $surfacesPresent.Count
      contains_all_required_surfaces = [bool]($surfacesPresent.Count -eq $expected.Count)
    }
  }

  function Test-RequiredSurfaceLabelsReady {
    param($RootElement)
    if ($null -eq $RootElement) {
      return $false
    }
    try {
      $elements = $RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition
      )
    } catch {
      return $false
    }
    $seen = @{}
    for ($index = 0; $index -lt $elements.Count; $index += 1) {
      $element = $elements.Item($index)
      $name = Get-ElementString -Element $element -PropertyName "Name"
      $automationId = Get-ElementString -Element $element -PropertyName "AutomationId"
      $searchText = Normalize-SurfaceText -Text "$name $automationId"
      foreach ($label in $expected) {
        if (Test-SurfaceTextContains -Text $searchText -Label $label) {
          $seen[$label] = $true
        }
      }
    }
    foreach ($label in $expected) {
      if (!$seen.ContainsKey($label)) {
        return $false
      }
    }
    return $true
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
  $surfaceDeadline = (Get-Date).AddSeconds($WaitSeconds)
  while (
    (Get-Date) -lt $surfaceDeadline -and
    $null -ne $window -and
    !(Test-RequiredSurfaceLabelsReady -RootElement $window)
  ) {
    $Process.Refresh()
    if ($Process.HasExited) {
      break
    }
    Start-Sleep -Milliseconds 250
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
          (Get-ElementString -Element $element -PropertyName "Name"),
          (Get-ElementString -Element $element -PropertyName "AutomationId"),
          (Get-ControlTypeName -Element $element)
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
  $treeEdges = @(
    $observedElements |
      Where-Object { $_.parent_runtime_id -ne "" } |
      ForEach-Object {
        [ordered]@{
          child_runtime_id = $_.runtime_id
          parent_runtime_id = $_.parent_runtime_id
          child_element_key = $_.element_key
        }
      }
  )
  $automationNames = New-Object System.Collections.Generic.List[string]
  foreach ($candidateName in $names) {
    if ($candidateName -eq "") {
      continue
    }
    if (!$automationNames.Contains($candidateName)) {
      $automationNames.Add($candidateName)
    }
    if ($automationNames.Count -ge 200) {
      break
    }
  }
  $rootWindowTitle = ""
  if ($observedElements.Count -gt 0) {
    $rootObservedElement = $observedElements.Item(0)
    if ($null -ne $rootObservedElement.name) {
      $rootWindowTitle = $rootObservedElement.name.ToString()
    }
  }
  $windowFound = [bool]($observedElements.Count -gt 0)
  $expectedSurfaceLabels = @($expected | ForEach-Object { $_.ToString() })
  $visibleSurfaceLabels = @($visible | ForEach-Object { $_.ToString() })
  $automationNameValues = @($automationNames.ToArray())
  $observedElementValues = @($observedElements.ToArray())
  $treeEdgeValues = @($treeEdges)
  $capture = [ordered]@{
    source = "uiautomation"
    path = $OutputPath
    captured_at = (Get-Date).ToUniversalTime().ToString("o")
    process_id = $Process.Id
    window_found = $windowFound
    window_title = $rootWindowTitle
    expected_surfaces = @($expectedSurfaceLabels)
    visible_surfaces = @($visibleSurfaceLabels)
    surface_matches = $surfaceMatches
    aggregate_surface_shortcut_detected = [bool]$aggregateSurfaceShortcutDetected
    surface_match_requirements_met = [bool]$surfaceMatchRequirementsMet
    automation_names = @($automationNameValues)
    diagnostic_tree = [ordered]@{
      mode = "full_uiautomation_tree_projection"
      observed_element_count = $observedElementValues.Count
      observed_elements = @($observedElementValues)
      tree_edges = @($treeEdgeValues)
      capture_limit = "none"
      failure_diagnostic = !$surfaceMatchRequirementsMet
    }
  }
  $output = New-Item -ItemType File -Force -Path $OutputPath
  Write-JsonEvidence -Value $capture -Path $output.FullName -Depth 8
  return $capture
}

trap {
  $failure = $_
  Restore-SmokeEnvironment
  if ($null -ne $process) {
    $process.Refresh()
    if (!$process.HasExited) {
      Stop-Process -Id $process.Id -Force
    }
  }
  Stop-SmokeBroker -Process $brokerProcess
  throw $failure
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

$setupDoctorPath = Resolve-InputOrOutputPath -Path $SetupDoctorJson
$surfaceSemanticsPath = Join-Path (Split-Path -Parent $setupDoctorPath) "surface_semantics_export.json"
$setupDoctorContextPath = Join-Path (Split-Path -Parent $setupDoctorPath) "setup_doctor_context.json"
Write-SetupDoctorProductContext `
  -Path $setupDoctorContextPath `
  -InstalledAppPath $exe.Path `
  -AppArtifactSha256 "sha256:$hash" `
  -ConfigPathValue $ConfigPath `
  -AuditDirValue $AuditDir `
  -BrokerMediated $brokerMediatedLaunch `
  -BrokerEndpointFileValue $brokerEndpointFile `
  -BrokerEndpointValue $brokerEndpoint
$env:GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON = $setupDoctorPath
$env:GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON = $setupDoctorContextPath
$env:GUI_SHELL_SURFACE_SEMANTICS_EXPORT_JSON = $surfaceSemanticsPath

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

if (!(Test-Path $setupDoctorPath)) {
  throw "インストール済み app が環境診断の製品出力を書き出しませんでした: $setupDoctorPath"
}
$setupDoctor = Get-Content -Raw -Path $setupDoctorPath | ConvertFrom-Json
$installedManifestPath = Find-InstalledManifestPath -ExePath $exe.Path
$installedManifest = $null
if ($null -ne $installedManifestPath) {
  $installedManifest = Get-Content -Raw -Path $installedManifestPath | ConvertFrom-Json
}
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
  if (
    (Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "surface_match_requirements_met") -ne $true -and
    (Test-Path $surfaceSemanticsPath)
  ) {
    $visibleSurfaceEvidence = Get-Content -Raw -Path $surfaceSemanticsPath | ConvertFrom-Json
  }
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
$auditAnchorEvidence = $null
if ($AuditAnchorEvidenceJson -ne "") {
  $auditAnchorEvidencePath = Resolve-Path $AuditAnchorEvidenceJson
  $auditAnchorEvidence = Get-Content -Raw -Path $auditAnchorEvidencePath | ConvertFrom-Json
}
$evidenceBundleFiles = New-Object System.Collections.Generic.List[object]
foreach ($record in @(
    (New-EvidenceFileRecord -Kind "setup_doctor" -Path $setupDoctorPath),
    (New-EvidenceFileRecord -Kind "broker_smoke" -Path $BrokerEvidenceJson),
    (New-EvidenceFileRecord -Kind "visible_surfaces" -Path (Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "path")),
    (New-EvidenceFileRecord -Kind "runtime_assertions" -Path $RuntimeAssertionsJson),
    (New-EvidenceFileRecord -Kind "audit_anchor_external_tamper_evidence" -Path $AuditAnchorEvidenceJson),
    (New-EvidenceFileRecord -Kind "screenshot_supporting_material" -Path $ScreenshotPath),
    (New-EvidenceFileRecord -Kind "installed_manifest" -Path $installedManifestPath),
    (New-EvidenceFileRecord -Kind "setup_doctor_context" -Path $setupDoctorContextPath)
  )) {
  if ($null -ne $record) {
    $evidenceBundleFiles.Add($record)
  }
}
$evidenceBundleFileValues = @($evidenceBundleFiles.ToArray())
$bundleText = (@{ files = @($evidenceBundleFileValues) } | ConvertTo-Json -Compress -Depth 10)
$evidenceBundleSha256 = Get-TaggedStringSha256 -Text $bundleText

$mainWindowHandle = 0
$windowTitle = ""
if (!$process.HasExited) {
  $mainWindowHandle = $process.MainWindowHandle.ToInt64()
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
  provenance = [ordered]@{
    evidence_contract_version = 2
    run_id = $(if ($null -ne $installedManifest) { $installedManifest.run_id } else { $null })
    source_commit = $(if ($null -ne $installedManifest) { $installedManifest.source_commit } else { $null })
    source_worktree_clean = $(if ($null -ne $installedManifest) { $installedManifest.source_worktree_clean } else { $false })
    source_status_porcelain = $(if ($null -ne $installedManifest) { $installedManifest.source_status_porcelain } else { $null })
    build_command = $(if ($null -ne $installedManifest) { $installedManifest.build_command } else { $null })
    build_timestamp = $(if ($null -ne $installedManifest) { $installedManifest.build_timestamp } else { $null })
    staged_manifest_path = $installedManifestPath
    installed_manifest_sha256 = $(if ($null -ne $installedManifestPath) { Get-TaggedSha256 -Path $installedManifestPath } else { $null })
    app_artifact_sha256 = "sha256:$hash"
    broker_artifact_sha256 = $(if ($null -ne $installedManifest) { $installedManifest.broker_artifact_sha256 } elseif ($BrokerHelperExe -ne "") { Get-TaggedSha256 -Path (Resolve-Path $BrokerHelperExe).Path } else { $null })
    isolation = [ordered]@{
      uses_shared_fixed_install_root = $(if ($null -ne $installedManifest -and $null -ne $installedManifest.isolation) { $installedManifest.isolation.uses_shared_fixed_install_root } else { $true })
      isolated_install_root = $(if ($null -ne $installedManifest -and $null -ne $installedManifest.isolation) { $installedManifest.isolation.isolated_install_root } else { $null })
      isolated_runtime_dir = $(if ($null -ne $installedManifest -and $null -ne $installedManifest.isolation) { $installedManifest.isolation.isolated_runtime_dir } else { $null })
      isolated_store_dir = $(if ($null -ne $installedManifest -and $null -ne $installedManifest.isolation) { $installedManifest.isolation.isolated_store_dir } else { $BrokerStoreDir })
      isolated_config_dir = $(if ($null -ne $installedManifest -and $null -ne $installedManifest.isolation) { $installedManifest.isolation.isolated_config_dir } else { Split-Path -Parent $ConfigPath })
      isolated_audit_dir = $(if ($null -ne $installedManifest -and $null -ne $installedManifest.isolation) { $installedManifest.isolation.isolated_audit_dir } else { $AuditDir })
    }
    evidence_bundle_files = @($evidenceBundleFileValues)
    evidence_bundle_sha256 = $evidenceBundleSha256
  }
  field_provenance = [ordered]@{
    artifact = [ordered]@{ source_type = "directly_measured"; evidence_class = "EXTERNAL_EVIDENCE"; formal_release_input = $true }
    "first_run.process" = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME"; formal_release_input = $true }
    "first_run.visible_surfaces" = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME"; formal_release_input = $true }
    "first_run.config_audit" = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME"; formal_release_input = $true }
    "first_run.installer_authority_boundary" = [ordered]@{ source_type = "static_assertion"; evidence_class = "CONFIG"; formal_release_input = $true }
    setup_doctor = [ordered]@{
      source_type = $(if ((Get-EvidenceValue -Object $setupDoctor -Name "formal_product_evidence") -eq $true) { "product_export" } else { "external_probe" })
      evidence_class = $(if ((Get-EvidenceValue -Object $setupDoctor -Name "formal_product_evidence") -eq $true) { "LIVE_RUNTIME" } else { "EXTERNAL_EVIDENCE" })
      formal_release_input = $true
    }
    "broker.ipc_restart_crash" = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME"; formal_release_input = $true }
    release_runtime_assertions = [ordered]@{ source_type = "static_assertion"; evidence_class = @("CONFIG", "FIXTURE"); formal_release_input = $true }
    unsupported_claims = @()
  }
  evidence_source = [ordered]@{
    collector = "installer/windows/collect_installed_smoke.ps1"
    collector_version = "7"
    manual_confirmation = $false
    screenshot_path = $(if ($ScreenshotPath -ne "") { $ScreenshotPath } else { $null })
  }
  artifact = [ordered]@{
    installed_exe_path = $exe.Path
    installed_exe_exists = $true
    sha256 = "sha256:$hash"
  }
  first_run = [ordered]@{
    status = $(if ($DiagnosticOnly.IsPresent) { "diagnostic_only" } elseif ($firstWindowVisible -and $configCreated -and $auditDirWritable -and $visibleSurfacesComplete) { "passed" } else { "failed" })
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
      diagnostic_tree = Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "diagnostic_tree"
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
        field_provenance = [ordered]@{}
        unmeasured_declarations = [ordered]@{
          python_runtime_required_for_authority = [ordered]@{ value = $true; source_type = "unsupported_claim"; evidence_class = "CONFIG"; formal_runtime_proof = $false }
          flutter_rust_ffi_authority_bridge = [ordered]@{ value = $true; source_type = "unsupported_claim"; evidence_class = "CONFIG"; formal_runtime_proof = $false }
        }
      }
    })
  release_runtime_assertions = $runtimeAssertions
}

if ($null -ne $auditAnchorEvidence) {
  $evidence["field_provenance"]["audit_anchor.external_tamper_evidence"] = [ordered]@{
    source_type = "directly_measured"
    evidence_class = $(if ((Get-EvidenceValue -Object (Get-EvidenceValue -Object $auditAnchorEvidence -Name "evidence_source") -Name "evidence_class") -eq "EXTERNAL_EVIDENCE") { "EXTERNAL_EVIDENCE" } else { "LIVE_RUNTIME" })
    formal_release_input = $true
  }
  $evidence["audit_anchor_external_tamper_evidence"] = $auditAnchorEvidence
}

$output = New-Item -ItemType File -Force -Path $OutputPath
Write-JsonEvidence -Value $evidence -Path $output.FullName -Depth 10

if ($null -ne $process -and !$process.HasExited) {
  Stop-Process -Id $process.Id
}
Stop-SmokeBroker -Process $brokerProcess

Write-Host "書き出しました: $($output.FullName)"
