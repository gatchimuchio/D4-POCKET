param(
  [Parameter(Mandatory = $true)]
  [string]$InstalledExe,

  [Parameter(Mandatory = $true)]
  [string]$DesktopLauncherExe,

  [string]$OutputPath = "release_evidence/windows_installed_smoke.json",

  [string]$VisibleSurfacesJson = "",

  [string]$VisibleSurfacesOutputPath = "",

  [int]$VisibleSurfaceWaitSeconds = 8,

  [string]$BrokerEvidenceJson = "",

  [switch]$NoPythonRuntime,

  [switch]$UseCurrentWindowsProfile,

  [string]$RuntimeAssertionsJson = "",

  [string]$AuditAnchorEvidenceJson = "",

  [string]$ScreenshotPath = "",

  [string]$InstalledManifestJson = "",

  [switch]$DiagnosticOnly
)

$ErrorActionPreference = "Stop"
$MaximumEvidenceFileBytes = 67108864
$MaximumBrokerAuditBytes = 8388608

$exe = Resolve-Path $InstalledExe
$launcher = Resolve-Path $DesktopLauncherExe
$hash = (Get-FileHash -Algorithm SHA256 -Path $exe).Hash.ToLowerInvariant()
$launcherHash = (Get-FileHash -Algorithm SHA256 -Path $launcher).Hash.ToLowerInvariant()
$process = $null
$launcherProcess = $null
$launcherStartTime = $null
$launcherSessionId = $null
$frontendDirectChildVerified = $false
$frontendImageSha256Verified = $false
$frontendObservedParentProcessId = $null
$brokerEndpoint = $null
$brokerEndpointFile = $null
$brokerMediatedLaunch = $false
$previousPathEnv = [Environment]::GetEnvironmentVariable("Path", "Process")
$previousLocalAppDataEnv = [Environment]::GetEnvironmentVariable("LOCALAPPDATA", "Process")
$smokeEnvironmentVariables = @(
  "GUI_SHELL_BROKER_ENDPOINT_JSON",
  "GUI_SHELL_BROKER_SESSION_JSON",
  "GUI_SHELL_BROKER_RUNTIME_DIR",
  "GUI_SHELL_BROKER_CHANNEL_PIPE",
  "GUI_SHELL_SNAPSHOT_JSON",
  "GUI_SHELL_SURFACE_SEMANTICS_EXPORT_JSON"
)
$previousSmokeEnvironment = [ordered]@{}
foreach ($name in $smokeEnvironmentVariables) {
  $previousSmokeEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
$pythonRuntimePathScrubbed = $false
$pythonPathEntriesRemovedCount = 0
$pythonPathEntriesRemainingCount = 0
$pythonCommandsVisibleAfterScrub = @()
$frontendRunningAfterLaunch = $false
$frontendCloseRequested = $false
$frontendExitMenuInvoked = $false
$frontendForcedToExit = $false
$frontendCleanupError = $false
$launcherExitCode = $null
$launcherExitedAfterFrontend = $false
$sessionFileRemovedAfterShutdown = $false
$startupAuditEventCount = 0
$shutdownAuditEventCount = 0
$startupAuditRecorded = $false
$shutdownAuditRecorded = $false
$normalBrokerHealthEventCount = 0
$normalBrokerHealthFirstEventId = $null
$normalBrokerHealthRequestAccepted = $false
$separateWindowsProfileVerified = $false
$currentUserSid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$runIsolationId = ""
$localAppDataRoot = ""
$brokerRuntimeRoot = ""
$brokerStoreDir = ""

if ($UseCurrentWindowsProfile.IsPresent) {
  $actualProfileLocalAppData = [System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData)
  if ([string]::IsNullOrWhiteSpace($actualProfileLocalAppData) -or
      [string]::Compare((Resolve-Path -LiteralPath $env:LOCALAPPDATA).Path, $actualProfileLocalAppData, $true) -ne 0) {
    throw "現在のWindows user profileとLOCALAPPDATAが一致しません。"
  }
  $localAppDataRoot = $actualProfileLocalAppData
} else {
  if (!$DiagnosticOnly.IsPresent) {
    throw "正式collectorでは別Windows user profileを確認してください。同一profileでの実行は-DiagnosticOnlyに限定します。"
  }
}
if ($VisibleSurfacesJson -ne "" -and !$DiagnosticOnly.IsPresent) {
  throw "正式collectorは現在画面を自身でUIAutomation計測します。外部visible-surface JSONは-DiagnosticOnlyでのみ読めます。"
}

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

function Read-Utf8Json {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $resolved = Resolve-Path $Path
  if ((Get-Item -LiteralPath $resolved.Path).Length -gt $MaximumEvidenceFileBytes) {
    throw "証拠JSONが読み込み上限を超えています。"
  }
  return [System.IO.File]::ReadAllText($resolved.Path, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
}

function Get-TaggedSha256 {
  param([string]$Path)

  if ($Path -eq "" -or !(Test-Path $Path)) {
    return $null
  }
  if ((Get-Item -LiteralPath $Path).Length -gt $MaximumEvidenceFileBytes) {
    throw "証拠fileがhash対象上限を超えています。"
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

function Assert-NormalBrokerEndpoint {
  param($Endpoint)

  if ($null -eq $Endpoint -or
      [string]$Endpoint.credential_role -ne "normal" -or
      [string]$Endpoint.host -ne "127.0.0.1" -or
      [string]$Endpoint.transport -ne "authenticated_loopback_tcp" -or
      [int]$Endpoint.port -lt 1 -or
      [int]$Endpoint.port -gt 65535 -or
      [string]::IsNullOrEmpty([string]$Endpoint.session_id) -or
      -not ([string]$Endpoint.session_secret -match '^[a-f0-9]{64}$') -or
      [int]$Endpoint.max_request_bytes -lt 1) {
    throw "broker通常接続資格が不正"
  }
}

function Restore-SmokeEnvironment {
  foreach ($name in $smokeEnvironmentVariables) {
    $value = $previousSmokeEnvironment[$name]
    if ($null -eq $value) {
      [Environment]::SetEnvironmentVariable($name, $null, "Process")
    } else {
      [Environment]::SetEnvironmentVariable($name, $value, "Process")
    }
  }
  if ($null -eq $previousPathEnv) {
    [Environment]::SetEnvironmentVariable("Path", $null, "Process")
  } else {
    [Environment]::SetEnvironmentVariable("Path", $previousPathEnv, "Process")
  }
  if ($null -eq $previousLocalAppDataEnv) {
    [Environment]::SetEnvironmentVariable("LOCALAPPDATA", $null, "Process")
  } else {
    [Environment]::SetEnvironmentVariable("LOCALAPPDATA", $previousLocalAppDataEnv, "Process")
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

function Get-VerifiedInstalledFrontendChildren {
  param(
    [System.Diagnostics.Process]$LauncherProcess,
    [string]$ExpectedPath,
    [string]$ExpectedSha256
  )

  $expectedName = [System.IO.Path]::GetFileName($ExpectedPath)
  $matches = New-Object 'System.Collections.Generic.List[object]'
  $children = Get-CimInstance -ClassName Win32_Process -Filter "ParentProcessId = $($LauncherProcess.Id)" -ErrorAction SilentlyContinue
  foreach ($child in @($children)) {
    if ([string]::IsNullOrWhiteSpace($child.ExecutablePath) -or
        [string]::Compare([System.IO.Path]::GetFileName($child.ExecutablePath), $expectedName, $true) -ne 0) {
      continue
    }
    try {
      $observedSha256 = (Get-FileHash -LiteralPath $child.ExecutablePath -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
      if ($observedSha256 -ne $ExpectedSha256) {
        continue
      }
      $candidate = Get-Process -Id ([int]$child.ProcessId) -ErrorAction Stop
      $candidate.Refresh()
      if ($candidate.HasExited -or
          $candidate.SessionId -ne $launcherSessionId -or
          $candidate.StartTime -lt $launcherStartTime) {
        continue
      }
      $matches.Add([pscustomobject]@{
        Process = $candidate
        ParentProcessId = [int]$child.ParentProcessId
        ImageSha256 = "sha256:$observedSha256"
      })
    } catch {
      continue
    }
  }
  return $matches.ToArray()
}

function Find-InstalledFrontendProcess {
  param([System.Diagnostics.Process]$LauncherProcess, [string]$ExpectedPath, [string]$ExpectedSha256)

  for ($attempt = 0; $attempt -lt 300; $attempt += 1) {
    $LauncherProcess.Refresh()
    if ($LauncherProcess.HasExited) {
      throw "Rust Desktop起動器がFlutter画面を起動する前に終了しました: $($LauncherProcess.ExitCode)"
    }
    $children = @(Get-VerifiedInstalledFrontendChildren -LauncherProcess $LauncherProcess -ExpectedPath $ExpectedPath -ExpectedSha256 $ExpectedSha256)
    if ($children.Count -eq 1) {
      $script:frontendDirectChildVerified = $true
      $script:frontendImageSha256Verified = $true
      $script:frontendObservedParentProcessId = $children[0].ParentProcessId
      return $children[0].Process
    }
    if ($children.Count -gt 1) {
      throw "Rust Desktop起動器からhash一致するFlutter childが複数起動しました。"
    }
    Start-Sleep -Milliseconds 100
  }
  throw "Rust Desktop起動器の直接childから配置済みFlutter imageをhash照合できませんでした。"
}

function Request-InstalledFrontendExit {
  param([System.Diagnostics.Process]$Frontend)

  $Frontend.Refresh()
  if ($Frontend.HasExited) { return $false }
  $windowHandle = $Frontend.MainWindowHandle
  if ($windowHandle -eq [IntPtr]::Zero) { return $false }

  Add-Type -AssemblyName UIAutomationClient
  Add-Type -AssemblyName UIAutomationTypes
  if ($null -eq ("GuiShellInstalledSmokeWin32" -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class GuiShellInstalledSmokeWin32 {
  [DllImport("user32.dll", SetLastError = true)]
  public static extern bool PostMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
}
"@
  }

  # 通知領域controllerへ右button releaseを渡し、製品の終了menuを開く。
  $posted = [GuiShellInstalledSmokeWin32]::PostMessage(
    $windowHandle,
    [uint32]0x8029,
    [IntPtr]::Zero,
    [IntPtr]0x0205
  )
  if (!$posted) { return $false }

  $desktop = [System.Windows.Automation.AutomationElement]::RootElement
  for ($attempt = 0; $attempt -lt 50; $attempt += 1) {
    Start-Sleep -Milliseconds 200
    $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
    $pending = New-Object 'System.Collections.Generic.Stack[object]'
    try {
      $topLevel = $walker.GetFirstChild($desktop)
      $topLevelCount = 0
      while ($null -ne $topLevel -and $topLevelCount -lt 1024) {
        $topLevelProcessId = -1
        try {
          $topLevelProcessId = [int]$topLevel.Current.ProcessId
        } catch {
          $topLevelProcessId = -1
        }
        if ($topLevelProcessId -eq $Frontend.Id) {
          $pending.Push($topLevel)
        }
        $topLevel = $walker.GetNextSibling($topLevel)
        $topLevelCount += 1
      }
    } catch {
      continue
    }
    $observedCount = 0
    while ($pending.Count -gt 0 -and $observedCount -lt 8192) {
      $item = $pending.Pop()
      $observedCount += 1
      try {
        if ($item.Current.ControlType -eq [System.Windows.Automation.ControlType]::MenuItem -and
            $item.Current.Name -eq "終了" -and
            $item.Current.ProcessId -eq $Frontend.Id -and
            $item.Current.IsEnabled) {
          $pattern = $item.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
          $pattern.Invoke()
          return $true
        }
      } catch {
        continue
      }
      try {
        $child = $walker.GetFirstChild($item)
        $siblingCount = 0
        while ($null -ne $child -and $siblingCount -lt 1024) {
          $pending.Push($child)
          $child = $walker.GetNextSibling($child)
          $siblingCount += 1
        }
      } catch {
        continue
      }
    }
  }
  return $false
}

function Stop-InstalledLauncher {
  $frontends = @()
  if ($null -ne $process) {
    $frontends = @([pscustomobject]@{
      Process = $process
      ParentProcessId = $frontendObservedParentProcessId
      ImageSha256 = "sha256:$hash"
    })
  } elseif ($null -ne $launcherProcess -and $null -ne $launcherStartTime -and $null -ne $launcherSessionId) {
    try {
      $frontends = @(Get-VerifiedInstalledFrontendChildren -LauncherProcess $launcherProcess -ExpectedPath $exe.Path -ExpectedSha256 $hash)
      if ($frontends.Count -eq 1) {
        $script:process = $frontends[0].Process
        $script:frontendDirectChildVerified = $true
        $script:frontendImageSha256Verified = $true
        $script:frontendObservedParentProcessId = $frontends[0].ParentProcessId
      } elseif ($frontends.Count -gt 1) {
        $script:frontendCleanupError = $true
      }
    } catch {
      $script:frontendCleanupError = $true
    }
  }
  foreach ($frontendIdentity in $frontends) {
    $frontend = $frontendIdentity.Process
    $frontend.Refresh()
    if (!$frontend.HasExited) {
      try {
        $exitMenuInvoked = Request-InstalledFrontendExit -Frontend $frontend
        if ($exitMenuInvoked) {
          $script:frontendCloseRequested = $true
          $script:frontendExitMenuInvoked = $true
        }
        if (!$exitMenuInvoked) {
          $script:frontendCleanupError = $true
          $null = $frontend.CloseMainWindow()
          $exitedAfterGracefulRequest = $frontend.WaitForExit(3000)
        } else {
          $exitedAfterGracefulRequest = $frontend.WaitForExit(20000)
        }
        if (!$exitedAfterGracefulRequest) {
          $script:frontendCleanupError = $true
          $script:frontendForcedToExit = $true
          Stop-Process -Id $frontend.Id -Force -ErrorAction SilentlyContinue
          $null = $frontend.WaitForExit(10000)
        }
      } catch {
        $script:frontendCleanupError = $true
        $script:frontendForcedToExit = $true
        Stop-Process -Id $frontend.Id -Force -ErrorAction SilentlyContinue
      }
    }
  }
  if ($null -ne $launcherProcess) {
    $launcherProcess.Refresh()
    if (!$launcherProcess.HasExited) {
      $script:launcherExitedAfterFrontend = $launcherProcess.WaitForExit(15000)
      if (!$script:launcherExitedAfterFrontend) {
        $script:frontendCleanupError = $true
        Stop-Process -Id $launcherProcess.Id -Force -ErrorAction SilentlyContinue
      }
    } else {
      $script:launcherExitedAfterFrontend = $true
    }
    $launcherProcess.Refresh()
    if ($launcherProcess.HasExited) {
      $script:launcherExitCode = $launcherProcess.ExitCode
    }
  }
}

function Test-ObservedSurfaceVisible {
  param($Element, $ObservedElements)
  if ($Element.is_root -ne $false -or $Element.is_native_container -ne $false) { return $false }
  $byId = @{}
  foreach ($node in $ObservedElements) {
    if ([string]::IsNullOrEmpty($node.runtime_id) -or $byId.ContainsKey($node.runtime_id)) { return $false }
    $byId[$node.runtime_id] = $node
  }
  $seen = @{}
  $cursor = $Element
  $intersection = $null
  while ($null -ne $cursor) {
    if ($seen.ContainsKey($cursor.runtime_id) -or $seen.Count -ge 128) { return $false }
    $seen[$cursor.runtime_id] = $true
    if ($cursor.is_offscreen -ne $false) { return $false }
    $rect = $cursor.bounding_rectangle
    if ($null -eq $rect) { return $false }
    foreach ($value in @($rect.x, $rect.y, $rect.width, $rect.height)) {
      if ($null -eq $value -or $value -is [bool] -or $value -is [string]) { return $false }
      if ([double]::IsNaN([double]$value) -or [double]::IsInfinity([double]$value)) { return $false }
    }
    if ($rect.width -le 0 -or $rect.height -le 0) { return $false }
    $bounds = @([double]$rect.x, [double]$rect.y, ([double]$rect.x + $rect.width), ([double]$rect.y + $rect.height))
    if ([double]::IsInfinity($bounds[2]) -or [double]::IsInfinity($bounds[3])) { return $false }
    if ($null -eq $intersection) { $intersection = $bounds }
    else {
      $intersection = @([Math]::Max($intersection[0], $bounds[0]), [Math]::Max($intersection[1], $bounds[1]),
                        [Math]::Min($intersection[2], $bounds[2]), [Math]::Min($intersection[3], $bounds[3]))
    }
    if ($intersection[2] -le $intersection[0] -or $intersection[3] -le $intersection[1]) { return $false }
    if ($cursor.is_root -eq $true) { return ($cursor.parent_runtime_id -eq "") }
    if ([string]::IsNullOrEmpty($cursor.parent_runtime_id) -or !$byId.ContainsKey($cursor.parent_runtime_id)) { return $false }
    $cursor = $byId[$cursor.parent_runtime_id]
  }
  return $false
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
    $surfaceNames = @{
      "Dashboard" = @("概要", "gui_shell.surface.dashboard")
      "NavigationRail" = @("ナビゲーション", "gui_shell.surface.navigation_rail")
      "Runtime Status" = @("実行系状態", "gui_shell.surface.runtime_status")
      "Invariant Status" = @("不変条件状態", "gui_shell.surface.invariant_status")
    }
    $mapped = $surfaceNames[$Label]
    $normalized = Normalize-SurfaceText -Text $Text
    return ($normalized -eq $mapped[0] -or $normalized -eq "$($mapped[0]) $($mapped[0])" -or
      $normalized -eq $mapped[1] -or $Text -match ("(?<![a-z0-9_.])" + [regex]::Escape($Label) + "(?![a-z0-9_.])"))
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

  function Get-ControlViewDescendants {
    param(
      $RootElement,
      [int]$MaximumElements = 10000
    )

    $script:surfaceTreeCaptureLimitReached = $false
    $script:surfaceTreeDuplicateRuntimeIdDetected = $false
    $elements = New-Object System.Collections.Generic.List[object]
    $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
    $pending = New-Object 'System.Collections.Generic.Stack[object]'
    $seenRuntimeIds = @{}
    $child = $walker.GetFirstChild($RootElement)
    while ($null -ne $child -and ($elements.Count + $pending.Count) -lt $MaximumElements) {
      $pending.Push($child)
      $child = $walker.GetNextSibling($child)
    }
    if ($null -ne $child) {
      $script:surfaceTreeCaptureLimitReached = $true
    }

    while ($pending.Count -gt 0) {
      if ($elements.Count -ge $MaximumElements) {
        $script:surfaceTreeCaptureLimitReached = $true
        break
      }
      $element = $pending.Pop()
      $runtimeId = Get-RuntimeIdString -Element $element
      if ($runtimeId -ne "") {
        if ($seenRuntimeIds.ContainsKey($runtimeId)) {
          $script:surfaceTreeDuplicateRuntimeIdDetected = $true
          continue
        }
        $seenRuntimeIds[$runtimeId] = $true
      }
      $null = $elements.Add($element)

      $child = $walker.GetFirstChild($element)
      while ($null -ne $child -and ($elements.Count + $pending.Count) -lt $MaximumElements) {
        $pending.Push($child)
        $child = $walker.GetNextSibling($child)
      }
      if ($null -ne $child) {
        $script:surfaceTreeCaptureLimitReached = $true
      }
    }
    return @($elements.ToArray())
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
      return $null
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
    $surfacesPresent = @()
    foreach ($label in $expected) {
      if ((Test-SurfaceTextContains -Text $name -Label $label) -or (Test-SurfaceTextContains -Text $automationId -Label $label)) {
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
    $elements = @(Get-ControlViewDescendants -RootElement $RootElement)
    if ($script:surfaceTreeCaptureLimitReached -or $script:surfaceTreeDuplicateRuntimeIdDetected) {
      return $false
    }
    $seen = @{}
    foreach ($element in $elements) {
      $name = Get-ElementString -Element $element -PropertyName "Name"
      $automationId = Get-ElementString -Element $element -PropertyName "AutomationId"
      foreach ($label in $expected) {
        if ((Test-SurfaceTextContains -Text $name -Label $label) -or (Test-SurfaceTextContains -Text $automationId -Label $label)) {
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
    $elements = @(Get-ControlViewDescendants -RootElement $window)
    for ($index = 0; $index -lt $elements.Count; $index += 1) {
      $element = $elements[$index]
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
    $candidates = @($observedElements | Where-Object {
      $_.surfaces_present -contains $label -and (Test-ObservedSurfaceVisible -Element $_ -ObservedElements $observedElements)
    })
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
    !$script:surfaceTreeCaptureLimitReached -and
    !$script:surfaceTreeDuplicateRuntimeIdDetected -and
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
      tree_view = "control"
      capture_limit = $(if ($script:surfaceTreeCaptureLimitReached) { "max_10000_elements" } elseif ($script:surfaceTreeDuplicateRuntimeIdDetected) { "duplicate_runtime_id" } else { "none" })
      failure_diagnostic = !$surfaceMatchRequirementsMet
    }
  }
  $output = New-Item -ItemType File -Force -Path $OutputPath
  Write-JsonEvidence -Value $capture -Path $output.FullName -Depth 8
  return $capture
}

trap {
  $failure = $_
  Stop-InstalledLauncher
  Restore-SmokeEnvironment
  throw $failure
}

$installedManifestPath = Find-InstalledManifestPath -ExePath $exe.Path
if ($null -eq $installedManifestPath) {
  throw "起動器・App・Broker artifactの由来を照合するinstalled manifestがありません。"
}
$installedManifest = Read-Utf8Json -Path $installedManifestPath
foreach ($item in @(
    @{ actual = $exe.Path; expected = $installedManifest.app_exe; label = "App" },
    @{ actual = $launcher.Path; expected = $installedManifest.launcher_exe; label = "Rust Desktop起動器" }
  )) {
  if ([string]::Compare([System.IO.Path]::GetFullPath([string]$item.actual), [System.IO.Path]::GetFullPath([string]$item.expected), $true) -ne 0) {
    throw "$($item.label) pathがstaged manifestと一致しません。"
  }
}
if ([string]$installedManifest.app_artifact_sha256 -ne "sha256:$hash" -or
    [string]$installedManifest.launcher_artifact_sha256 -ne "sha256:$launcherHash") {
  throw "AppまたはRust Desktop起動器のartifact hashがstaged manifestと一致しません。"
}
$brokerHelperPath = [string]$installedManifest.broker_exe
$brokerHash = Get-TaggedSha256 -Path $brokerHelperPath
if ($null -eq $brokerHash -or [string]$installedManifest.broker_artifact_sha256 -ne $brokerHash) {
  throw "staged Rust Broker helperのartifact hashを確認できません。"
}
$stagingUserIdentity = $installedManifest.staging_user_identity
$stagingUserIdentitySalt = [string]$stagingUserIdentity.per_run_salt
$stagingUserIdentityHash = [string]$stagingUserIdentity.salted_hash
if ($stagingUserIdentity.hash_algorithm -ne "SHA-256" -or
    $stagingUserIdentitySalt -notmatch '^[a-f0-9]{32}$' -or
    $stagingUserIdentityHash -notmatch '^sha256:[a-f0-9]{64}$') {
  throw "staged manifestに比較可能なper-run user identity digestがありません。現行形式で再stageしてください。"
}
$currentUserIdentityHash = Get-TaggedStringSha256 -Text "$stagingUserIdentitySalt|$currentUserSid"
$separateWindowsProfileVerified = ![string]::Equals($currentUserIdentityHash, $stagingUserIdentityHash, [System.StringComparison]::OrdinalIgnoreCase)
if ($UseCurrentWindowsProfile.IsPresent -and !$separateWindowsProfileVerified) {
  throw "起動器runtimeを分離するため、stage時とは異なるWindows user profileで実行してください。"
}
$runIsolationId = "$($installedManifest.run_id)-smoke-$([guid]::NewGuid().ToString('N'))"
if ($UseCurrentWindowsProfile.IsPresent) {
  $localAppDataRoot = Join-Path ([System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::LocalApplicationData)) ("D4Pocket-installed-smoke\" + $runIsolationId)
  New-Item -ItemType Directory -Path $localAppDataRoot -ErrorAction Stop | Out-Null
} else {
  $newLocalAppDataRoot = Join-Path $env:TEMP ("D4Pocket-installed-smoke-" + $runIsolationId)
  New-Item -ItemType Directory -Path $newLocalAppDataRoot -ErrorAction Stop | Out-Null
  $localAppDataRoot = $newLocalAppDataRoot
}
$brokerRuntimeRoot = Join-Path $localAppDataRoot "GUI-Shell\broker\desktop"
$brokerStoreDir = Join-Path $brokerRuntimeRoot "store"
$brokerEndpointFile = Join-Path $brokerRuntimeRoot "broker_session.json"
$firstRunConfigurationPath = Join-Path $brokerStoreDir "first_run_configuration.json"
$configExistedBeforeLaunch = Test-Path -LiteralPath $firstRunConfigurationPath
if ($configExistedBeforeLaunch) {
  throw "初回設定fileが起動前から存在します。clean installed first-run用の隔離LOCALAPPDATAを確認してください。"
}

try {
  if ($NoPythonRuntime.IsPresent) {
    Enable-NoPythonLaunchPath
  }
  foreach ($name in $smokeEnvironmentVariables) {
    [Environment]::SetEnvironmentVariable($name, $null, "Process")
  }
  [Environment]::SetEnvironmentVariable("LOCALAPPDATA", $localAppDataRoot, "Process")
  $launcherProcess = Start-Process -FilePath $launcher.Path -PassThru
  $launcherProcess.Refresh()
  $launcherStartTime = $launcherProcess.StartTime
  $launcherSessionId = $launcherProcess.SessionId
  $process = Find-InstalledFrontendProcess -LauncherProcess $launcherProcess -ExpectedPath $exe.Path -ExpectedSha256 $hash
  for ($attempt = 0; $attempt -lt 100 -and !(Test-Path -LiteralPath $brokerEndpointFile); $attempt += 1) {
    $launcherProcess.Refresh()
    if ($launcherProcess.HasExited) {
      throw "Broker endpoint準備後にRust Desktop起動器が終了しました: $($launcherProcess.ExitCode)"
    }
    Start-Sleep -Milliseconds 100
  }
  if (!(Test-Path -LiteralPath $brokerEndpointFile)) {
    throw "Rust Desktop起動器が実使用Broker endpointを作成しませんでした。"
  }
  try {
    $brokerEndpoint = Read-Utf8Json -Path $brokerEndpointFile
  } catch {
    throw "Rust Desktop起動器のBroker endpoint JSONを読み取れません。"
  }
  Assert-NormalBrokerEndpoint -Endpoint $brokerEndpoint
  $brokerMediatedLaunch = $true
  Start-Sleep -Seconds 3
  $process.Refresh()
  $frontendRunningAfterLaunch = !$process.HasExited
} finally {
  Restore-SmokeEnvironment
}

$setupDoctorPath = Join-Path $brokerStoreDir "setup_doctor_report.json"
$setupDoctor = [ordered]@{
  status = "unknown"
  formal_product_evidence = $false
  evidence_source = [ordered]@{
    collector = "installer/windows/collect_installed_smoke.ps1"
    source_kind = "product_export_not_observed"
    product_generated = $false
    collector_derives_checks = $false
    synthetic = $false
  }
}
if ($VisibleSurfacesJson -ne "") {
  $visibleSurfacesPath = Resolve-Path $VisibleSurfacesJson
  $visibleSurfaceEvidence = Read-Utf8Json -Path $visibleSurfacesPath.Path
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
$process.Refresh()
$mainWindowHandle = 0
$windowTitle = ""
if (!$process.HasExited) {
  $mainWindowHandle = $process.MainWindowHandle.ToInt64()
  $windowTitle = $process.MainWindowTitle
}
$firstWindowVisible = (!$process.HasExited -and $mainWindowHandle -ne 0)
Stop-InstalledLauncher
$sessionFileRemovedAfterShutdown = !(Test-Path -LiteralPath $brokerEndpointFile)
$brokerAuditPath = Join-Path $brokerStoreDir "audit.jsonl"
$setupDoctorReport = $null
$setupDoctorReportRawBytes = $null
$setupDoctorReportSha256 = $null
$setupDoctorFileObserved = $false
$setupDoctorAuditEvent = $null
$setupDoctorAcceptedAuditMatchCount = 0
$configCreated = $false
$configFileObserved = $false
$configJsonValid = $false
$configSha256 = $null
$configAuditEvent = $null
$configAuditAcceptedMatchCount = 0
if (Test-Path -LiteralPath $setupDoctorPath -PathType Leaf) {
  $setupDoctorItem = Get-Item -LiteralPath $setupDoctorPath -ErrorAction Stop
  if (($setupDoctorItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0 -and
      $setupDoctorItem.Length -gt 0 -and $setupDoctorItem.Length -le 65536) {
    try {
      $setupDoctorReportRawBytes = [System.IO.File]::ReadAllBytes($setupDoctorPath)
      $setupDoctorReport = Read-Utf8Json -Path $setupDoctorPath
      $setupDoctorReportSha256 = Get-TaggedSha256 -Path $setupDoctorPath
      $setupDoctorFileObserved = ($setupDoctorReport -is [System.Management.Automation.PSCustomObject] -and
        $setupDoctorReportSha256 -match '^sha256:[a-f0-9]{64}$')
    } catch {
      $setupDoctorReport = $null
      $setupDoctorReportRawBytes = $null
      $setupDoctorReportSha256 = $null
      $setupDoctorFileObserved = $false
    }
  }
}
$configurationItem = Get-Item -LiteralPath $firstRunConfigurationPath -ErrorAction SilentlyContinue
if ($null -ne $configurationItem -and $configurationItem -is [System.IO.FileInfo] -and
    ($configurationItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0 -and
    $configurationItem.Length -gt 0 -and $configurationItem.Length -le 16384) {
  $configurationStream = $null
  try {
    $configurationStream = [System.IO.File]::Open(
      $firstRunConfigurationPath,
      [System.IO.FileMode]::Open,
      [System.IO.FileAccess]::Read,
      [System.IO.FileShare]::Read
    )
    if ($configurationStream.Length -gt 0 -and $configurationStream.Length -le 16384) {
      $configurationBytes = New-Object byte[] ([int]$configurationStream.Length)
      $configurationOffset = 0
      while ($configurationOffset -lt $configurationBytes.Length) {
        $configurationRead = $configurationStream.Read(
          $configurationBytes,
          $configurationOffset,
          $configurationBytes.Length - $configurationOffset
        )
        if ($configurationRead -le 0) { break }
        $configurationOffset += $configurationRead
      }
      $configurationHasTrailingBytes = $configurationStream.ReadByte() -ne -1
      if ($configurationOffset -eq $configurationBytes.Length -and !$configurationHasTrailingBytes) {
        $strictUtf8 = New-Object System.Text.UTF8Encoding($false, $true)
        $configurationText = $strictUtf8.GetString($configurationBytes)
        $configurationDocument = $configurationText | ConvertFrom-Json
        $rootProperties = @($configurationDocument.PSObject.Properties.Name | Sort-Object)
        $preferenceProperties = @($configurationDocument.ui_preferences.PSObject.Properties.Name | Sort-Object)
        $configJsonValid = (
          $configurationDocument -is [System.Management.Automation.PSCustomObject] -and
          ($rootProperties -join ",") -ceq "product,ui_preferences,version" -and
          (($preferenceProperties -join ",") -ceq "density,locale,theme") -and
          $configurationDocument.version -eq 1 -and
          $configurationDocument.product -ceq "D4 Pocket" -and
          $configurationDocument.ui_preferences.theme -ceq "system" -and
          $configurationDocument.ui_preferences.density -ceq "compact" -and
          $configurationDocument.ui_preferences.locale -ceq "ja-JP"
        )
        if ($configJsonValid) {
          $configHashAlgorithm = [System.Security.Cryptography.SHA256]::Create()
          try {
            $configSha256 = "sha256:$(([System.BitConverter]::ToString($configHashAlgorithm.ComputeHash($configurationBytes))).Replace('-', '').ToLowerInvariant())"
          } finally {
            $configHashAlgorithm.Dispose()
          }
          $configFileObserved = $true
        }
      }
    }
  } catch {
    $configJsonValid = $false
    $configSha256 = $null
    $configFileObserved = $false
  } finally {
    if ($null -ne $configurationStream) {
      $configurationStream.Dispose()
    }
  }
}
$configCreated = (!$configExistedBeforeLaunch -and $configFileObserved)
if (Test-Path -LiteralPath $brokerAuditPath -PathType Leaf) {
  if ((Get-Item -LiteralPath $brokerAuditPath).Length -gt $MaximumBrokerAuditBytes) {
    throw "Broker lifecycle Audit fileが読み込み上限を超えています。"
  }
  foreach ($line in [System.IO.File]::ReadAllLines($brokerAuditPath, [System.Text.Encoding]::UTF8)) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    try {
      $auditEvent = $line | ConvertFrom-Json
    } catch {
      throw "Broker lifecycle Auditに不正なJSON行があります。"
    }
    if ([string]$auditEvent.operation -eq "health" -and
        [string]$auditEvent.decision -eq "accepted" -and
        [string]$auditEvent.reason -eq "health status returned" -and
        [string]$auditEvent.evidence_source -eq "LIVE_RUNTIME" -and
        [string]$auditEvent.request_id -ne "" -and
        [string]$auditEvent.event_id -match '^broker-audit-[1-9][0-9]*$' -and
        [string]$auditEvent.payload_hash -match '^sha256:[a-f0-9]{64}$' -and
        [string]$auditEvent.event_hash -match '^sha256:[a-f0-9]{64}$') {
      $normalBrokerHealthEventCount += 1
      if ($null -eq $normalBrokerHealthFirstEventId) {
        $normalBrokerHealthFirstEventId = [string]$auditEvent.event_id
      }
    }
    if ($setupDoctorFileObserved -and
        [string]$auditEvent.operation -eq "Setup Doctor報告取得" -and
        [string]$auditEvent.decision -eq "accepted" -and
        [string]$auditEvent.reason -eq "setup_doctor_report_exported" -and
        [string]$auditEvent.evidence_source -eq "LIVE_RUNTIME" -and
        [string]$auditEvent.payload_hash -eq $setupDoctorReportSha256 -and
        [string]$auditEvent.event_id -match '^broker-audit-[1-9][0-9]*$' -and
        [string]$auditEvent.event_hash -match '^sha256:[a-f0-9]{64}$') {
      $setupDoctorAcceptedAuditMatchCount += 1
      $setupDoctorAuditEvent = $auditEvent
    }
    if ($configJsonValid -and $null -ne $configSha256 -and
        [string]$auditEvent.operation -eq "初回設定取得" -and
        [string]$auditEvent.decision -eq "accepted" -and
        [string]$auditEvent.reason -eq "初回UI設定を固定Broker storeからprojection。authorityは生成しない" -and
        [string]$auditEvent.evidence_source -eq "LIVE_RUNTIME" -and
        [string]$auditEvent.payload_hash -eq $configSha256 -and
        [string]$auditEvent.request_id -ne "" -and
        [string]$auditEvent.event_id -match '^broker-audit-[1-9][0-9]*$' -and
        [string]$auditEvent.event_hash -match '^sha256:[a-f0-9]{64}$') {
      $configAuditAcceptedMatchCount += 1
      $configAuditEvent = $auditEvent
    }
    if ([string]$auditEvent.operation -eq "D4 Pocket Desktop起動") {
      $startupAuditEventCount += 1
      if ([string]$auditEvent.decision -eq "recorded" -and [string]$auditEvent.evidence_source -eq "LIVE_RUNTIME") {
        $startupAuditRecorded = $true
      }
    }
    if ([string]$auditEvent.operation -eq "D4 Pocket Desktop終了") {
      $shutdownAuditEventCount += 1
      if ([string]$auditEvent.decision -eq "recorded" -and [string]$auditEvent.evidence_source -eq "LIVE_RUNTIME") {
        $shutdownAuditRecorded = $true
      }
    }
  }
}
$normalBrokerHealthRequestAccepted = $normalBrokerHealthEventCount -gt 0
$setupDoctorAuditMatched = ($setupDoctorFileObserved -and $setupDoctorAcceptedAuditMatchCount -eq 1)
$configAuditMatched = ($configFileObserved -and $configJsonValid -and $configAuditAcceptedMatchCount -eq 1)
$setupDoctorStatus = "unknown"
$setupDoctorChecks = @()
if ($setupDoctorAuditMatched) {
  $setupDoctorStatus = [string]$setupDoctorReport.status
  $setupDoctorChecks = @($setupDoctorReport.checks)
}
$setupDoctor = [ordered]@{
  status = $setupDoctorStatus
  formal_product_evidence = [bool]$setupDoctorAuditMatched
  report_path = $(if ($setupDoctorFileObserved) { $setupDoctorPath } else { $null })
  report_sha256 = $setupDoctorReportSha256
  serialized_report_base64 = $(if ($setupDoctorFileObserved) { [System.Convert]::ToBase64String($setupDoctorReportRawBytes) } else { $null })
  product_report = $setupDoctorReport
  accepted_audit_event = $setupDoctorAuditEvent
  ran_from_installed_app_path = ($setupDoctorAuditMatched -and $frontendImageSha256Verified -and
    (@($setupDoctorChecks | Where-Object { $_.check_id -eq "setup_doctor.ran_from_installed_app_path" -and $_.status -eq "pass" }).Count -eq 1))
  operator_readable = $false
  installer_grants_authority = $false
  installer_silently_approves_permissions = $false
  checks = $setupDoctorChecks
  evidence_source = [ordered]@{
    collector = "installer/windows/collect_installed_smoke.ps1"
    source_kind = $(if ($setupDoctorAuditMatched) { "installed_app_machine_readable_export" } else { "product_export_not_verified" })
    product_generated = [bool]$setupDoctorFileObserved
    collector_derives_checks = $false
    synthetic = $false
    command = "通常起動したD4 Pocket UIが認証済みBroker IPCのSetup Doctor報告取得を呼び出し"
    accepted_audit_event_id = $(if ($null -ne $setupDoctorAuditEvent) { $setupDoctorAuditEvent.event_id } else { $null })
    accepted_audit_event_payload_hash = $(if ($null -ne $setupDoctorAuditEvent) { $setupDoctorAuditEvent.payload_hash } else { $null })
  }
}
$brokerEvidence = $null
if ($BrokerEvidenceJson -ne "") {
  $brokerEvidencePath = Resolve-Path $BrokerEvidenceJson
  $brokerEvidence = Read-Utf8Json -Path $brokerEvidencePath.Path
}
$runtimeAssertions = $null
if ($RuntimeAssertionsJson -ne "") {
  $runtimeAssertionsPath = Resolve-Path $RuntimeAssertionsJson
  $runtimeAssertions = Read-Utf8Json -Path $runtimeAssertionsPath.Path
}
$auditAnchorEvidence = $null
if ($AuditAnchorEvidenceJson -ne "") {
  $auditAnchorEvidencePath = Resolve-Path $AuditAnchorEvidenceJson
  $auditAnchorEvidence = Read-Utf8Json -Path $auditAnchorEvidencePath.Path
}
$evidenceBundleFiles = New-Object System.Collections.Generic.List[object]
foreach ($record in @(
    (New-EvidenceFileRecord -Kind "setup_doctor" -Path $setupDoctorPath),
    (New-EvidenceFileRecord -Kind "first_run_configuration" -Path $firstRunConfigurationPath),
    (New-EvidenceFileRecord -Kind "broker_smoke" -Path $BrokerEvidenceJson),
    (New-EvidenceFileRecord -Kind "broker_lifecycle_audit" -Path $brokerAuditPath),
    (New-EvidenceFileRecord -Kind "visible_surfaces" -Path (Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "path")),
    (New-EvidenceFileRecord -Kind "runtime_assertions" -Path $RuntimeAssertionsJson),
    (New-EvidenceFileRecord -Kind "audit_anchor_external_tamper_evidence" -Path $AuditAnchorEvidenceJson),
    (New-EvidenceFileRecord -Kind "screenshot_supporting_material" -Path $ScreenshotPath),
    (New-EvidenceFileRecord -Kind "installed_manifest" -Path $installedManifestPath)
  )) {
  if ($null -ne $record) {
    $evidenceBundleFiles.Add($record)
  }
}
$evidenceBundleFileValues = @($evidenceBundleFiles.ToArray())
$bundleText = (@{ files = @($evidenceBundleFileValues) } | ConvertTo-Json -Compress -Depth 10)
$evidenceBundleSha256 = Get-TaggedStringSha256 -Text $bundleText

$resolvedAuditDir = Resolve-Path $brokerStoreDir -ErrorAction SilentlyContinue
$auditWriteProbe = [ordered]@{
  attempted = $false
  write = $false
  read = $false
  delete = $false
  probe_path = $null
}
if ($null -ne $resolvedAuditDir) {
  $probePath = Join-Path $resolvedAuditDir (".gui-shell-write-probe-" + [guid]::NewGuid().ToString("N"))
  $auditWriteProbe.attempted = $true
  $auditWriteProbe.probe_path = $probePath
  $probeStream = $null
  $probeCreated = $false
  try {
    $probeStream = [System.IO.File]::Open(
      $probePath,
      [System.IO.FileMode]::CreateNew,
      [System.IO.FileAccess]::ReadWrite,
      [System.IO.FileShare]::None
    )
    $probeCreated = $true
    $probeBytes = [System.Text.Encoding]::UTF8.GetBytes("ok")
    $probeStream.Write($probeBytes, 0, $probeBytes.Length)
    $probeStream.Flush()
    $auditWriteProbe.write = $true
    $probeStream.Position = 0
    $probeBuffer = New-Object byte[] $probeBytes.Length
    $probeReadCount = $probeStream.Read($probeBuffer, 0, $probeBuffer.Length)
    $auditWriteProbe.read = ($probeReadCount -eq $probeBytes.Length -and [System.Text.Encoding]::UTF8.GetString($probeBuffer) -eq "ok")
    $probeStream.Dispose()
    $probeStream = $null
    Remove-Item -LiteralPath $probePath -ErrorAction Stop
    $probeCreated = $false
    $auditWriteProbe.delete = !(Test-Path -LiteralPath $probePath)
  } catch {
    $auditWriteProbe.write = $false
  } finally {
    if ($null -ne $probeStream) {
      $probeStream.Dispose()
    }
    if ($probeCreated -and (Test-Path -LiteralPath $probePath)) {
      try {
        Remove-Item -LiteralPath $probePath -ErrorAction Stop
        $auditWriteProbe.delete = !(Test-Path -LiteralPath $probePath)
      } catch {
        $auditWriteProbe.delete = $false
      }
    }
  }
}

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
# build時の登録は、現在の描画・可視性を測定していない。
$surfaceBuildRegistry = (
  (Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "source") -eq "flutter_semantics_runtime_export" -or
  (Get-EvidenceValue -Object (Get-EvidenceValue -Object $visibleSurfaceEvidence -Name "diagnostic_tree") -Name "mode") -eq "flutter_dart_surface_semantics_runtime_export"
)
$visibleSurfacesComplete = !$surfaceBuildRegistry
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
$unsupportedClaims = @()
if (!$configAuditMatched) {
  $unsupportedClaims += "first_run_configuration_product_export"
}
if (!$setupDoctorAuditMatched) {
  $unsupportedClaims += "formal_setup_doctor_product_export"
}
if (!$separateWindowsProfileVerified) {
  $unsupportedClaims += "separate_windows_user_profile"
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
    launcher_artifact_sha256 = "sha256:$launcherHash"
    broker_artifact_sha256 = $brokerHash
    isolation = [ordered]@{
      uses_shared_fixed_install_root = $(if ($null -ne $installedManifest -and $null -ne $installedManifest.isolation) { $installedManifest.isolation.uses_shared_fixed_install_root } else { $true })
      isolated_install_root = $(if ($null -ne $installedManifest -and $null -ne $installedManifest.isolation) { $installedManifest.isolation.isolated_install_root } else { $null })
      isolated_localappdata = $localAppDataRoot
      isolated_runtime_dir = $brokerRuntimeRoot
      isolated_store_dir = $brokerStoreDir
      isolated_config_dir = $null
      isolated_audit_dir = $brokerStoreDir
      run_id = $runIsolationId
      separate_windows_user_profile = $separateWindowsProfileVerified
    }
    evidence_bundle_files = @($evidenceBundleFileValues)
    evidence_bundle_sha256 = $evidenceBundleSha256
  }
  field_provenance = [ordered]@{
    artifact = [ordered]@{ source_type = "directly_measured"; evidence_class = "EXTERNAL_EVIDENCE"; formal_release_input = $true }
    "first_run.process" = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME"; formal_release_input = $true }
    "first_run.visible_surfaces" = [ordered]@{
      source_type = $(if ($surfaceBuildRegistry) { "product_export" } else { "directly_measured" })
      evidence_class = $(if ($surfaceBuildRegistry) { "INTERNAL_STATE" } else { "LIVE_RUNTIME" })
      formal_release_input = !$surfaceBuildRegistry
    }
    "first_run.broker_lifecycle_audit" = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME"; formal_release_input = $true }
    "first_run.broker_health_request" = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME"; formal_release_input = $true }
    "first_run.config_audit" = [ordered]@{
      source_type = $(if ($configAuditMatched) { "directly_measured" } else { "unsupported_claim" })
      evidence_class = $(if ($configAuditMatched) { "LIVE_RUNTIME" } else { "INTERNAL_STATE" })
      formal_release_input = [bool]$configAuditMatched
    }
    "first_run.installer_authority_boundary" = [ordered]@{ source_type = "static_assertion"; evidence_class = "CONFIG"; formal_release_input = $true }
    setup_doctor = [ordered]@{
      source_type = $(if ($setupDoctorAuditMatched) { "product_export" } else { "unsupported_claim" })
      evidence_class = $(if ($setupDoctorAuditMatched) { "LIVE_RUNTIME" } else { "INTERNAL_STATE" })
      formal_release_input = [bool]$setupDoctorAuditMatched
    }
    "broker.ipc_restart_crash" = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME"; formal_release_input = $true }
    release_runtime_assertions = [ordered]@{ source_type = "static_assertion"; evidence_class = @("CONFIG", "FIXTURE"); formal_release_input = $true }
    unsupported_claims = @($unsupportedClaims)
  }
  evidence_source = [ordered]@{
    collector = "installer/windows/collect_installed_smoke.ps1"
    collector_version = "15"
    manual_confirmation = $false
    screenshot_path = $(if ($ScreenshotPath -ne "") { $ScreenshotPath } else { $null })
  }
  artifact = [ordered]@{
    installed_exe_path = $exe.Path
    installed_exe_exists = $true
    sha256 = "sha256:$hash"
    desktop_launcher_path = $launcher.Path
    desktop_launcher_sha256 = "sha256:$launcherHash"
    broker_helper_path = $brokerHelperPath
    broker_helper_sha256 = $brokerHash
  }
  first_run = [ordered]@{
    status = $(if ($DiagnosticOnly.IsPresent) { "diagnostic_only" } elseif ($firstWindowVisible -and $frontendExitMenuInvoked -and !$frontendForcedToExit -and !$frontendCleanupError -and $configCreated -and $configAuditMatched -and $auditDirWritable -and $visibleSurfacesComplete -and $startupAuditRecorded -and $shutdownAuditRecorded -and $sessionFileRemovedAfterShutdown -and $separateWindowsProfileVerified -and $normalBrokerHealthRequestAccepted -and $launcherExitedAfterFrontend -and $launcherExitCode -eq 0) { "passed" } else { "failed" })
    command = "& `"$($launcher.Path)`""
    launched_from_installed_path = $true
    launched_via_rust_desktop_launcher = $true
    launcher_process_id = $launcherProcess.Id
    launcher_exited_after_frontend = $launcherExitedAfterFrontend
    launcher_exit_code = $launcherExitCode
    launcher_runtime_dir = $brokerRuntimeRoot
    profile_identity_isolated_from_staging_user = $separateWindowsProfileVerified
    profile_identity_sid_exposed = $false
    process_id = $process.Id
    process_identity = [ordered]@{
      method = "direct_parent_pid_and_sha256"
      evidence_class = "LIVE_RUNTIME"
      direct_child_observed = $frontendDirectChildVerified
      parent_process_id = $frontendObservedParentProcessId
      same_windows_session = $true
      started_after_launcher = $true
      image_sha256 = "sha256:$hash"
      image_matches_manifest = $frontendImageSha256Verified
    }
    process_running_after_launch = $frontendRunningAfterLaunch
    frontend_close_requested = $frontendCloseRequested
    frontend_exit_menu_invoked = $frontendExitMenuInvoked
    frontend_forced_to_exit = $frontendForcedToExit
    frontend_cleanup_error = $frontendCleanupError
    main_window_handle = $mainWindowHandle
    window_title = $windowTitle
    first_window_visible = $firstWindowVisible
    broker_mediated_launch = $brokerMediatedLaunch
    broker_helper_path = $brokerHelperPath
    broker_endpoint_file = $brokerEndpointFile
    broker_endpoint_created = ($null -ne $brokerEndpoint)
    broker_endpoint_removed_after_shutdown = $sessionFileRemovedAfterShutdown
    broker_transport = $(if ($null -ne $brokerEndpoint) { $brokerEndpoint.transport } else { $null })
    broker_endpoint_credential_role = $(if ($null -ne $brokerEndpoint) { $brokerEndpoint.credential_role } else { $null })
    normal_endpoint_credential_role_verified = $(if ($null -ne $brokerEndpoint) { $brokerEndpoint.credential_role -eq "normal" } else { $false })
    broker_health_request = [ordered]@{
      accepted = $normalBrokerHealthRequestAccepted
      accepted_event_count = $normalBrokerHealthEventCount
      first_audit_event_id = $normalBrokerHealthFirstEventId
      evidence_class = "LIVE_RUNTIME"
      caller_process_attributed = $false
      client_response_receipt_observed = $false
    }
    broker_lifecycle_audit = [ordered]@{
      path = $brokerAuditPath
      sha256 = Get-TaggedSha256 -Path $brokerAuditPath
      startup_event_count = $startupAuditEventCount
      startup_event_recorded = $startupAuditRecorded
      shutdown_event_count = $shutdownAuditEventCount
      shutdown_event_recorded = $shutdownAuditRecorded
      evidence_class = "LIVE_RUNTIME"
    }
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
    config_path = $firstRunConfigurationPath
    config_existed_before_launch = $configExistedBeforeLaunch
    config_created = $configCreated
    config_json_valid = $configJsonValid
    config_audit = [ordered]@{
      evidence_class = $(if ($configAuditMatched) { "LIVE_RUNTIME" } else { "INTERNAL_STATE" })
      accepted_event_count = $configAuditAcceptedMatchCount
      config_sha256 = $configSha256
      accepted_event = $configAuditEvent
    }
    audit_dir = $(if ($null -ne $resolvedAuditDir) { $resolvedAuditDir.Path } else { $brokerStoreDir })
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

Write-Host "書き出しました: $($output.FullName)"
