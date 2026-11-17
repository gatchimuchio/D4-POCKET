[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$BundlePath,

  [Parameter(Mandatory = $true)]
  [string]$EvidencePath,

  [Parameter(Mandatory = $true)]
  [string]$SourceCommit
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Get-Sha256Hex {
  param([Parameter(Mandatory = $true)][byte[]]$Bytes)
  $algorithm = [System.Security.Cryptography.SHA256]::Create()
  try {
    return [Convert]::ToHexString($algorithm.ComputeHash($Bytes)).ToLowerInvariant()
  } finally {
    $algorithm.Dispose()
  }
}

function Get-ArtifactRecord {
  param(
    [Parameter(Mandatory = $true)]$Receipt,
    [Parameter(Mandatory = $true)][string]$RelativePath
  )
  $matches = @($Receipt.artifact_files | Where-Object { $_.path -ceq $RelativePath })
  if ($matches.Count -ne 1) {
    throw "build receipt artifact inventory mismatch: $RelativePath"
  }
  return $matches[0]
}

function Assert-ArtifactHash {
  param(
    [Parameter(Mandatory = $true)][string]$Root,
    [Parameter(Mandatory = $true)]$Record
  )
  $relative = [string]$Record.path
  $path = Join-Path $Root ($relative.Replace('/', '\'))
  $resolved = [IO.Path]::GetFullPath($path)
  $prefix = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
  if (-not $resolved.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -or
      -not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
    throw "artifact file is missing or outside bundle: $relative"
  }
  $item = Get-Item -LiteralPath $resolved -Force
  if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "artifact file is a reparse point: $relative"
  }
  $actual = (Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -cne [string]$Record.sha256 -or [long]$item.Length -ne [long]$Record.size_bytes) {
    throw "artifact identity does not match build receipt: $relative"
  }
  return $actual
}

function Get-ControlViewDescendants {
  param(
    [Parameter(Mandatory = $true)]$RootElement,
    [int]$MaximumElements = 8192
  )
  $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
  $pending = New-Object 'System.Collections.Generic.Stack[object]'
  $elements = New-Object 'System.Collections.Generic.List[object]'
  $child = $walker.GetFirstChild($RootElement)
  while ($null -ne $child -and $pending.Count -lt $MaximumElements) {
    $pending.Push($child)
    $child = $walker.GetNextSibling($child)
  }
  if ($null -ne $child) { throw 'UI Automation tree exceeded the bounded inspection limit' }
  while ($pending.Count -gt 0) {
    if ($elements.Count -ge $MaximumElements) {
      throw 'UI Automation tree exceeded the bounded inspection limit'
    }
    $element = $pending.Pop()
    $elements.Add($element)
    $child = $walker.GetFirstChild($element)
    while ($null -ne $child -and ($elements.Count + $pending.Count) -lt $MaximumElements) {
      $pending.Push($child)
      $child = $walker.GetNextSibling($child)
    }
    if ($null -ne $child) { throw 'UI Automation tree exceeded the bounded inspection limit' }
  }
  return @($elements.ToArray())
}

function Find-VerifiedFrontend {
  param(
    [Parameter(Mandatory = $true)][System.Diagnostics.Process]$Launcher,
    [Parameter(Mandatory = $true)][string]$ExpectedPath,
    [Parameter(Mandatory = $true)][string]$ExpectedHash,
    [Parameter(Mandatory = $true)][datetime]$LauncherStartTime
  )
  $expectedFullPath = [IO.Path]::GetFullPath($ExpectedPath)
  $children = @(Get-CimInstance -ClassName Win32_Process -Filter "ParentProcessId = $($Launcher.Id)")
  $matches = @()
  foreach ($child in $children) {
    if ([string]::IsNullOrWhiteSpace([string]$child.ExecutablePath) -or
        [string]::Compare([IO.Path]::GetFullPath([string]$child.ExecutablePath), $expectedFullPath, $true) -ne 0) {
      continue
    }
    $hash = (Get-FileHash -LiteralPath $expectedFullPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hash -cne $ExpectedHash) { continue }
    try {
      $process = Get-Process -Id ([int]$child.ProcessId) -ErrorAction Stop
      $process.Refresh()
      if (-not $process.HasExited -and
          $process.SessionId -eq $Launcher.SessionId -and
          $process.StartTime -ge $LauncherStartTime) {
        $matches += [pscustomobject]@{
          Process = $process
          ParentProcessId = [int]$child.ParentProcessId
        }
      }
    } catch {
      continue
    }
  }
  if ($matches.Count -gt 1) { throw 'multiple hash-matched Flutter children belong to the launcher' }
  if ($matches.Count -eq 1) { return $matches[0] }
  return $null
}

function Invoke-AuthenticatedRequest {
  param(
    [Parameter(Mandatory = $true)]$Endpoint,
    [Parameter(Mandatory = $true)][string]$SessionSecret,
    [Parameter(Mandatory = $true)][string]$RequestJson
  )
  $client = [System.Net.Sockets.TcpClient]::new()
  try {
    $client.ReceiveTimeout = 10000
    $client.SendTimeout = 10000
    $client.Connect('127.0.0.1', [int]$Endpoint.port)
    $stream = $client.GetStream()
    $bytes = [Text.UTF8Encoding]::new($false).GetBytes($SessionSecret + "`n" + $RequestJson + "`n")
    $stream.Write($bytes, 0, $bytes.Length)
    $stream.Flush()
    [Array]::Clear($bytes, 0, $bytes.Length)

    $reader = [IO.StreamReader]::new($stream, [Text.UTF8Encoding]::new($false), $false, 1024, $true)
    $responseLine = $reader.ReadLine()
    if ([string]::IsNullOrWhiteSpace($responseLine)) { throw 'Broker returned an empty response' }
    $client.Client.Shutdown([System.Net.Sockets.SocketShutdown]::Send)
    $trailing = $reader.ReadToEnd()
    if (-not [string]::IsNullOrEmpty($trailing)) { throw 'Broker returned trailing response data' }
    return [pscustomobject]@{
      Json = $responseLine
      Response = ($responseLine | ConvertFrom-Json -ErrorAction Stop)
    }
  } finally {
    $client.Dispose()
  }
}

function Request-FrontendExit {
  param([Parameter(Mandatory = $true)][System.Diagnostics.Process]$Frontend)
  $Frontend.Refresh()
  if ($Frontend.HasExited -or $Frontend.MainWindowHandle -eq [IntPtr]::Zero) { return $false }
  Add-Type -AssemblyName UIAutomationClient
  Add-Type -AssemblyName UIAutomationTypes
  if (-not [GuiShellExportSmokeWin32]::PostMessage(
      $Frontend.MainWindowHandle,
      [uint32]0x8029,
      [IntPtr]::Zero,
      [IntPtr]0x0205)) {
    return $false
  }
  $desktop = [System.Windows.Automation.AutomationElement]::RootElement
  $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
  $deadline = (Get-Date).AddSeconds(12)
  while ((Get-Date) -lt $deadline) {
    $pending = New-Object 'System.Collections.Generic.Stack[object]'
    $topLevel = $walker.GetFirstChild($desktop)
    $topLevelCount = 0
    while ($null -ne $topLevel -and $topLevelCount -lt 1024) {
      try {
        if ($topLevel.Current.ProcessId -eq $Frontend.Id) { $pending.Push($topLevel) }
      } catch {
        # A window may disappear while the desktop tree is being observed.
      }
      $topLevel = $walker.GetNextSibling($topLevel)
      $topLevelCount += 1
    }
    $observed = 0
    while ($pending.Count -gt 0 -and $observed -lt 8192) {
      $item = $pending.Pop()
      $observed += 1
      try {
        if ($item.Current.ProcessId -eq $Frontend.Id -and
            $item.Current.ControlType -eq [System.Windows.Automation.ControlType]::MenuItem -and
            $item.Current.Name -eq '終了' -and
            $item.Current.IsEnabled) {
          $pattern = $item.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
          $pattern.Invoke()
          return $true
        }
      } catch {
        # A menu node may disappear during invocation or traversal.
      }
      try {
        $child = $walker.GetFirstChild($item)
        $childCount = 0
        while ($null -ne $child -and ($pending.Count + $observed) -lt 8192 -and $childCount -lt 1024) {
          $pending.Push($child)
          $child = $walker.GetNextSibling($child)
          $childCount += 1
        }
      } catch {
        # Continue with the remaining nodes in this owned window only.
      }
    }
    Start-Sleep -Milliseconds 200
  }
  return $false
}

function Stop-ExactOwnedProcess {
  param(
    [Parameter(Mandatory = $true)][System.Diagnostics.Process]$Process,
    [Parameter(Mandatory = $true)][string]$ExpectedPath,
    [Parameter(Mandatory = $true)][string]$ExpectedHash,
    [int]$ExpectedParentId = 0
  )
  try {
    $Process.Refresh()
    if ($Process.HasExited) { return $true }
    $row = Get-CimInstance -ClassName Win32_Process -Filter "ProcessId = $($Process.Id)" -ErrorAction Stop
    if ($null -eq $row -or
        [string]::Compare([IO.Path]::GetFullPath([string]$row.ExecutablePath), [IO.Path]::GetFullPath($ExpectedPath), $true) -ne 0 -or
        ($ExpectedParentId -gt 0 -and [int]$row.ParentProcessId -ne $ExpectedParentId) -or
        (Get-FileHash -LiteralPath $ExpectedPath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $ExpectedHash) {
      return $false
    }
    Stop-Process -Id $Process.Id -Force -ErrorAction Stop
    return $Process.WaitForExit(10000)
  } catch {
    return $false
  }
}

if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows') {
  throw 'This live launch collector runs only on a GitHub-hosted Windows Actions runner'
}
if ($null -ne $env:GH_TOKEN -or $null -ne $env:GITHUB_TOKEN -or
    $null -ne $env:GIT_CONFIG_VALUE_0 -or $null -ne $env:ACTIONS_RUNTIME_TOKEN -or
    $null -ne $env:ACTIONS_ID_TOKEN_REQUEST_TOKEN) {
  throw 'runner authentication environment was not cleared before launching the fixture app'
}
if ($SourceCommit -notmatch '^[0-9a-f]{40}$' -or $SourceCommit -cne $env:GITHUB_SHA) {
  throw 'source commit does not match the workflow checkout'
}
if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP) -or [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
  throw 'required runner-local roots are unavailable'
}
$runnerRoot = [IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd('\') + '\'
$bundleRoot = [IO.Path]::GetFullPath($BundlePath)
$evidenceFullPath = [IO.Path]::GetFullPath($EvidencePath)
if (-not $bundleRoot.StartsWith($runnerRoot, [StringComparison]::OrdinalIgnoreCase) -or
    -not $evidenceFullPath.StartsWith($runnerRoot, [StringComparison]::OrdinalIgnoreCase) -or
    (Test-Path -LiteralPath $evidenceFullPath)) {
  throw 'bundle or evidence path is outside RUNNER_TEMP or already exists'
}

$evidence = [ordered]@{
  evidence_type = 'd4_pocket_windows_portable_launch_smoke'
  result = 'failed'
  evidence_class = 'LIVE_RUNTIME'
  source_commit = $SourceCommit
  runner_os = $env:RUNNER_OS
  runner_image = "$env:ImageOS/$env:ImageVersion"
  fixture = [ordered]@{
    portable_bundle = $true
    app_id = $null
    audit_store_id = $null
  }
  artifact_sha256 = [ordered]@{ launcher = $null; flutter_app = $null; broker_helper = $null; product_manifest = $null }
  frontend = [ordered]@{
    launcher_argument_rejected = $false
    launcher_argument_error_code = $null
    unexpected_frontend_spawned = $false
    launcher_started = $false
    direct_child_verified = $false
    main_window_observed = $false
    included_surfaces = @()
    excluded_surfaces_absent = $false
    graceful_exit = $false
    forced_exit = $false
  }
  broker = [ordered]@{
    normal_endpoint_observed = $false
    excluded_operation_status = $null
    excluded_operation_code = $null
    excluded_operation_audit_id = $null
    generic_command_status = $null
    generic_command_code = $null
    generic_command_audit_id = $null
    private_marker_absent_from_responses_and_audit = $false
    session_secret_absent_from_responses_and_audit = $false
    audit_sha256 = $null
    audit_bytes = 0
  }
  cleanup = [ordered]@{
    endpoint_removed = $false
    runtime_fixture_removed = $false
    launcher_exit_code = $null
  }
  scope = [ordered]@{
    installed_product_verified = $false
    owner_authorization_verified = $false
    signed = $false
    formal_distribution_claimed = $false
    build_receipt_launch_status_changed = $false
    binary_pruning_verified = $false
  }
  failure_stage = $null
}

$launcherProcess = $null
$launcherProbe = $null
$frontendProcess = $null
$frontendParentId = 0
$runtimeStore = $null
$endpointPath = $null
$auditPath = $null
$sessionSecret = $null
$runError = $null
$cleanupPassed = $true
$stage = 'read bundle receipt and manifest'

try {
  $receiptPath = Join-Path $bundleRoot 'd4_pocket_build_receipt.json'
  $manifestPath = Join-Path $bundleRoot 'product_manifest.json'
  $launcherPath = Join-Path $bundleRoot 'gui_shell_desktop_launcher.exe'
  $appPath = Join-Path $bundleRoot 'app\gui_shell_desktop.exe'
  $helperPath = Join-Path $bundleRoot 'broker\gui_shell_rust_helper.exe'
  foreach ($path in @($receiptPath, $manifestPath, $launcherPath, $appPath, $helperPath)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw 'required portable artifact is missing' }
  }
  $receipt = Get-Content -LiteralPath $receiptPath -Raw -Encoding utf8 | ConvertFrom-Json -ErrorAction Stop
  $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json -ErrorAction Stop
  $stage = 'validate fixture build receipt state'
  if ($receipt.evidence_type -ne 'd4_pocket_windows_export_build' -or
      $receipt.source_commit -cne $SourceCommit -or
      $receipt.evidence_source -ne 'INTERNAL_STATE' -or
      $receipt.portable_bundle_assembled -ne $true -or
      $receipt.standalone_app_verified -ne $false -or
      $receipt.runtime_manifest_consumed -ne $false -or
      $receipt.launch_status -ne 'not_verified' -or
      $receipt.binary_pruning_verified -ne $false -or
      $receipt.formal_distribution_claimed -ne $false -or
      $receipt.signed -ne $false -or
      $receipt.authority_boundary.credential_inherited -ne $false -or
      $receipt.authority_boundary.permission_inherited -ne $false -or
      $receipt.authority_boundary.approval_inherited -ne $false -or
      $receipt.authority_boundary.audit_chain_inherited -ne $false) {
    throw 'build receipt scope is not the expected fixture-only unsigned state'
  }
  $stage = 'validate fixture Manifest identity and authority strip'
  $appId = [string]$manifest.manifest.app_identity.app_id
  $auditStoreId = [string]$manifest.manifest.audit_store.store_id
  if ($appId -cne [string]$receipt.app_id -or
      $auditStoreId -cne [string]$receipt.audit_store_id -or
      $appId -notmatch '^d4-pocket-app-[0-9a-f]{32}$' -or
      $auditStoreId -notmatch '^audit-store-[0-9a-f]{32}$' -or
      $manifest.manifest.app_identity.target_platform -ne 'windows' -or
      $manifest.manifest.inheritance_policy.authority -ne 'none' -or
      $manifest.manifest.inheritance_policy.permission -ne 'none' -or
      $manifest.manifest.inheritance_policy.approval -ne 'none' -or
      $manifest.manifest.inheritance_policy.credential -ne 'none' -or
      $manifest.manifest.inheritance_policy.audit_chain -ne 'none') {
    throw 'fixture manifest identity, target, or inheritance strip is invalid'
  }

  $coreIds = @(
    'core.security_broker', 'core.cryptography', 'core.audit_finality', 'core.credentials',
    'core.os_controls', 'core.permission_enforcement', 'core.approval_enforcement', 'core.content_exposure'
  )
  $requiredIds = @(
    'shell.dashboard', 'shell.runtime_operation', 'shell.agent_operation', 'shell.authority', 'shell.approval',
    'shell.audit', 'shell.recovery', 'shell.problems', 'shell.evidence', 'shell.settings'
  )
  $optionalIds = @(
    'shell.setup_doctor', 'shell.history', 'shell.evaluation_lab', 'shell.host_capabilities',
    'shell.notifications', 'shell.observability', 'shell.trace_inspector', 'shell.host_operations'
  )
  $stage = 'validate selected and excluded modules'
  $includedIds = @($receipt.module_plan.included_module_ids | ForEach-Object { [string]$_ })
  $excludedIds = @($receipt.module_plan.excluded_optional_module_ids | ForEach-Object { [string]$_ })
  foreach ($moduleId in @($coreIds + $requiredIds)) {
    if ($includedIds -cnotcontains $moduleId) { throw "required module missing from build receipt: $moduleId" }
  }
  if ($receipt.module_plan.selection_mode -ne 'explicit_optional' -or
      $includedIds -cnotcontains 'shell.observability' -or
      $includedIds -cnotcontains 'shell.trace_inspector' -or
      $excludedIds.Count -ne 6 -or
      @($excludedIds | Where-Object { $optionalIds -cnotcontains $_ }).Count -gt 0 -or
      @($includedIds | Where-Object { $optionalIds -contains $_ }).Count -ne 2 -or
      @($includedIds | Where-Object { $excludedIds -contains $_ }).Count -gt 0) {
    throw 'fixture module plan does not select Observability/Trace Inspector and exclude the other six optional modules'
  }

  $stage = 'verify receipt-bound artifact hashes'
  $launcherRecord = Get-ArtifactRecord -Receipt $receipt -RelativePath 'gui_shell_desktop_launcher.exe'
  $appRecord = Get-ArtifactRecord -Receipt $receipt -RelativePath 'app/gui_shell_desktop.exe'
  $helperRecord = Get-ArtifactRecord -Receipt $receipt -RelativePath 'broker/gui_shell_rust_helper.exe'
  $manifestRecord = Get-ArtifactRecord -Receipt $receipt -RelativePath 'product_manifest.json'
  $launcherHash = Assert-ArtifactHash -Root $bundleRoot -Record $launcherRecord
  $appHash = Assert-ArtifactHash -Root $bundleRoot -Record $appRecord
  $helperHash = Assert-ArtifactHash -Root $bundleRoot -Record $helperRecord
  $manifestHash = Assert-ArtifactHash -Root $bundleRoot -Record $manifestRecord
  if ($manifestHash -cne [string]$receipt.source_manifest_sha256) {
    throw 'runtime manifest hash does not match the build receipt'
  }
  $evidence.fixture.app_id = $appId
  $evidence.fixture.audit_store_id = $auditStoreId
  $evidence.artifact_sha256.launcher = "sha256:$launcherHash"
  $evidence.artifact_sha256.flutter_app = "sha256:$appHash"
  $evidence.artifact_sha256.broker_helper = "sha256:$helperHash"
  $evidence.artifact_sha256.product_manifest = "sha256:$manifestHash"

  $stage = 'verify the launcher rejects an attempt to enable an excluded module'
  $stage = 'prepare bounded launcher argument refusal observation'
  Add-Type -AssemblyName UIAutomationClient
  Add-Type -AssemblyName UIAutomationTypes
  if ($null -eq ('GuiShellExportSmokeWin32' -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class GuiShellExportSmokeWin32 {
  [DllImport("user32.dll", SetLastError = true)]
  public static extern bool PostMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
}
"@
  }
  $stage = 'start launcher unsupported-argument probe'
  $launcherProbe = Start-Process -FilePath $launcherPath -ArgumentList @('--enable-module=shell.setup_doctor') -WorkingDirectory $bundleRoot -PassThru
  $stage = 'observe launcher refusal dialog'
  $probeDeadline = (Get-Date).AddSeconds(20)
  $probeDialog = $null
  $probeCondition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
    [int]$launcherProbe.Id
  )
  while ((Get-Date) -lt $probeDeadline -and $null -eq $probeDialog) {
    $launcherProbe.Refresh()
    if ($launcherProbe.HasExited) { throw 'launcher did not present the bounded unsupported-argument refusal' }
    $dialogs = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
      [System.Windows.Automation.TreeScope]::Children, $probeCondition
    )
    if ($dialogs.Count -gt 0) { $probeDialog = $dialogs[0] } else { Start-Sleep -Milliseconds 200 }
  }
  if ($null -eq $probeDialog) { throw 'launcher refusal dialog was not observed' }
  $stage = 'verify launcher refusal error code'
  $probeElements = @(Get-ControlViewDescendants -RootElement $probeDialog -MaximumElements 2048)
  $probeNames = @($probeElements | ForEach-Object {
    try { [string]$_.Current.Name } catch { '' }
  } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
  if (@($probeNames | Where-Object { $_.Contains('UNSUPPORTED_ARGUMENTS') }).Count -eq 0) {
    throw 'launcher did not expose the expected unsupported-argument code'
  }
  $probeButton = @($probeElements | Where-Object {
    try {
      $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and
        $_.Current.Name -eq 'OK' -and $_.Current.IsEnabled
    } catch { $false }
  } | Select-Object -First 1)
  if ($probeButton.Count -ne 1) { throw 'launcher refusal dialog OK action was not available' }
  $stage = 'dismiss launcher refusal dialog'
  $probeButton[0].GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
  $stage = 'verify launcher unsupported-argument exit code'
  if (-not $launcherProbe.WaitForExit(10000) -or $launcherProbe.ExitCode -ne 1) {
    throw 'launcher did not exit with the expected unsupported-argument status'
  }
  $stage = 'verify rejected launcher spawned no frontend'
  $unexpectedChildren = @(Get-CimInstance -ClassName Win32_Process -Filter "ParentProcessId = $($launcherProbe.Id)" | Where-Object {
    [string]::Compare([string]$_.ExecutablePath, [IO.Path]::GetFullPath($appPath), $true) -eq 0
  })
  if ($unexpectedChildren.Count -ne 0) {
    $evidence.frontend.unexpected_frontend_spawned = $true
    throw 'launcher spawned the excluded-module app after rejecting an unsupported argument'
  }
  $evidence.frontend.launcher_argument_rejected = $true
  $evidence.frontend.launcher_argument_error_code = 'UNSUPPORTED_ARGUMENTS'

  $stage = 'verify isolated runtime storage before launch'
  $runtimeStore = Join-Path $env:LOCALAPPDATA "D4Pocket\apps\$appId\stores\$auditStoreId"
  $runtimeStore = [IO.Path]::GetFullPath($runtimeStore)
  $localAppDataRoot = [IO.Path]::GetFullPath($env:LOCALAPPDATA).TrimEnd('\') + '\'
  if (-not $runtimeStore.StartsWith($localAppDataRoot, [StringComparison]::OrdinalIgnoreCase) -or
      (Test-Path -LiteralPath $runtimeStore)) {
    throw 'fixture runtime store already exists or escaped the runner LocalAppData root'
  }
  $endpointPath = Join-Path $runtimeStore 'broker_session.json'
  $auditPath = Join-Path $runtimeStore 'audit.jsonl'

  $stage = 'launch exact portable Rust launcher and observe its Flutter child'
  $launcherProcess = Start-Process -FilePath $launcherPath -WorkingDirectory $bundleRoot -PassThru
  $launcherStartTime = $launcherProcess.StartTime
  $evidence.frontend.launcher_started = $true
  $deadline = (Get-Date).AddSeconds(120)
  $verifiedChild = $null
  while ((Get-Date) -lt $deadline) {
    $launcherProcess.Refresh()
    if ($launcherProcess.HasExited) { throw 'portable launcher exited before the Flutter window became ready' }
    $verifiedChild = Find-VerifiedFrontend -Launcher $launcherProcess -ExpectedPath $appPath -ExpectedHash $appHash -LauncherStartTime $launcherStartTime
    if ($null -ne $verifiedChild) { break }
    Start-Sleep -Milliseconds 250
  }
  if ($null -eq $verifiedChild) { throw 'hash-matched direct Flutter child was not observed before deadline' }
  $frontendProcess = $verifiedChild.Process
  $frontendParentId = [int]$verifiedChild.ParentProcessId
  $evidence.frontend.direct_child_verified = $true
  $evidence.frontend.launcher_pid = [int]$launcherProcess.Id
  $evidence.frontend.process_id = [int]$frontendProcess.Id
  $evidence.frontend.parent_process_id = $frontendParentId
  $evidence.frontend.image_sha256 = "sha256:$appHash"
  $evidence.frontend.launcher_sha256 = "sha256:$launcherHash"

  $stage = 'observe bounded UI Automation surfaces from the launched app'
  $window = $null
  $uiDeadline = (Get-Date).AddSeconds(90)
  $condition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
    [int]$frontendProcess.Id
  )
  while ((Get-Date) -lt $uiDeadline -and $null -eq $window) {
    $frontendProcess.Refresh()
    if ($frontendProcess.HasExited) { throw 'Flutter child exited before UI observation' }
    $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
      [System.Windows.Automation.TreeScope]::Children, $condition
    )
    if ($windows.Count -gt 0) { $window = $windows[0] } else { Start-Sleep -Milliseconds 250 }
  }
  if ($null -eq $window -or $frontendProcess.MainWindowHandle -eq [IntPtr]::Zero) {
    throw 'Flutter main window was not observed through UI Automation'
  }
  $evidence.frontend.main_window_observed = $true
  $elements = @(Get-ControlViewDescendants -RootElement $window)
  $navRoot = @($elements | Where-Object {
    try {
      $_.Current.Name -eq 'ナビゲーション' -or $_.Current.AutomationId -eq 'gui_shell.surface.navigation_rail'
    } catch { $false }
  } | Select-Object -First 1)
  if ($navRoot.Count -ne 1) { throw 'NavigationRail semantic root was not observed' }
  $navigationElements = @(Get-ControlViewDescendants -RootElement $navRoot[0])
  $navigationNames = @($navigationElements | ForEach-Object {
    try { [string]$_.Current.Name } catch { '' }
  } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
  $includedSurfaces = @('観測', '追跡')
  foreach ($surface in $includedSurfaces) {
    if ($navigationNames -cnotcontains $surface) { throw "selected module surface missing from live NavigationRail: $surface" }
  }
  $excludedSurfaces = @('診断', '履歴', '評価ラボ', 'ホスト能力', '通知', 'Host操作')
  foreach ($surface in $excludedSurfaces) {
    if ($navigationNames -ccontains $surface) { throw "excluded module surface remains in live NavigationRail: $surface" }
  }
  $evidence.frontend.included_surfaces = $includedSurfaces
  $evidence.frontend.excluded_surfaces_absent = $true

  $stage = 'read and validate the normal authenticated Broker endpoint'
  $endpointDeadline = (Get-Date).AddSeconds(20)
  while ((Get-Date) -lt $endpointDeadline -and -not (Test-Path -LiteralPath $endpointPath -PathType Leaf)) {
    Start-Sleep -Milliseconds 200
  }
  if (-not (Test-Path -LiteralPath $endpointPath -PathType Leaf)) { throw 'fixture Broker endpoint was not created' }
  $endpoint = Get-Content -LiteralPath $endpointPath -Raw -Encoding utf8 | ConvertFrom-Json -ErrorAction Stop
  if ($endpoint.host -cne '127.0.0.1' -or
      [int]$endpoint.port -lt 1 -or [int]$endpoint.port -gt 65535 -or
      $endpoint.credential_role -cne 'normal' -or
      $endpoint.transport -cne 'authenticated_loopback_tcp' -or
      [string]$endpoint.session_id -notmatch '^[A-Za-z0-9-]{1,96}$' -or
      [string]$endpoint.session_secret -notmatch '^[0-9a-f]{64}$' -or
      [int]$endpoint.max_request_bytes -lt 1 -or [int]$endpoint.max_request_bytes -gt 65536) {
    throw 'fixture Broker endpoint does not match the normal bounded loopback contract'
  }
  $sessionSecret = [string]$endpoint.session_secret
  $evidence.broker.normal_endpoint_observed = $true

  $stage = 'exercise excluded Broker operation over authenticated loopback IPC'
  $privateMarker = 'windows-export-private-marker-' + [guid]::NewGuid().ToString('N')
  $payload = [ordered]@{ privateMarker = $privateMarker }
  $payloadJson = ConvertTo-Json -InputObject $payload -Compress -Depth 8
  $payloadHash = 'sha256:' + (Get-Sha256Hex -Bytes ([Text.UTF8Encoding]::new($false).GetBytes($payloadJson)))
  $requestId = 'export-prune-' + [guid]::NewGuid().ToString('N')
  $request = [ordered]@{
    request_id = $requestId
    session_id = [string]$endpoint.session_id
    operation = 'Setup Doctor報告取得'
    payload_hash = $payloadHash
    nonce = 'export-prune-' + [guid]::NewGuid().ToString('N')
    issued_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    metadata = [ordered]@{}
    payload = $payload
  }
  $requestJson = ConvertTo-Json -InputObject $request -Compress -Depth 8
  $excludedResult = Invoke-AuthenticatedRequest -Endpoint $endpoint -SessionSecret $sessionSecret -RequestJson $requestJson
  if ($excludedResult.Json.Contains($privateMarker) -or $excludedResult.Json.Contains($sessionSecret) -or
      $excludedResult.Response.status -cne 'rejected' -or
      $excludedResult.Response.operation -cne 'Setup Doctor報告取得' -or
      $excludedResult.Response.error.code -cne 'optional_module_not_included' -or
      [string]::IsNullOrWhiteSpace([string]$excludedResult.Response.audit_event_id)) {
    throw 'excluded authenticated Broker operation was not rejected with the expected audited response'
  }
  $evidence.broker.excluded_operation_status = [string]$excludedResult.Response.status
  $evidence.broker.excluded_operation_code = [string]$excludedResult.Response.error.code
  $evidence.broker.excluded_operation_audit_id = [string]$excludedResult.Response.audit_event_id

  $stage = 'exercise generic command route and confirm dispatch remains suspended'
  $commandMarker = 'windows-export-command-marker-' + [guid]::NewGuid().ToString('N')
  $commandRequestId = 'export-command-' + [guid]::NewGuid().ToString('N')
  $commandPayloadJson = ConvertTo-Json -InputObject $commandMarker -Compress
  $commandPayloadHash = 'sha256:' + (Get-Sha256Hex -Bytes ([Text.UTF8Encoding]::new($false).GetBytes($commandPayloadJson)))
  $commandRequest = [ordered]@{
    request_id = $commandRequestId
    session_id = [string]$endpoint.session_id
    operation = 'command_envelope'
    payload_hash = $commandPayloadHash
    nonce = 'export-command-' + [guid]::NewGuid().ToString('N')
    issued_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    metadata = [ordered]@{}
    payload = $commandMarker
  }
  $commandJson = ConvertTo-Json -InputObject $commandRequest -Compress -Depth 8
  $commandResult = Invoke-AuthenticatedRequest -Endpoint $endpoint -SessionSecret $sessionSecret -RequestJson $commandJson
  if ($commandResult.Json.Contains($commandMarker) -or $commandResult.Json.Contains($sessionSecret) -or
      $commandResult.Response.status -cne 'suspended' -or
      $commandResult.Response.error.code -cne 'broker_command_dispatch_disabled' -or
      [string]::IsNullOrWhiteSpace([string]$commandResult.Response.audit_event_id)) {
    throw 'generic command route did not remain suspended with an audited response'
  }
  $evidence.broker.generic_command_status = [string]$commandResult.Response.status
  $evidence.broker.generic_command_code = [string]$commandResult.Response.error.code
  $evidence.broker.generic_command_audit_id = [string]$commandResult.Response.audit_event_id

  $stage = 'verify bounded Audit non-exposure and retain only safe evidence fields'
  if (-not (Test-Path -LiteralPath $auditPath -PathType Leaf)) { throw 'Broker Audit file was not created' }
  $auditText = Get-Content -LiteralPath $auditPath -Raw -Encoding utf8
  if ($auditText.Contains($privateMarker) -or $auditText.Contains($commandMarker) -or $auditText.Contains($sessionSecret) -or
      -not $auditText.Contains([string]$excludedResult.Response.audit_event_id) -or
      -not $auditText.Contains('optional_module_not_included') -or
      -not $auditText.Contains([string]$commandResult.Response.audit_event_id)) {
    throw 'Audit does not bind both outcomes or exposed a private marker / session secret'
  }
  $auditBytes = [IO.File]::ReadAllBytes($auditPath)
  $evidence.broker.audit_sha256 = 'sha256:' + (Get-Sha256Hex -Bytes $auditBytes)
  $evidence.broker.audit_bytes = $auditBytes.Length
  [Array]::Clear($auditBytes, 0, $auditBytes.Length)
  $evidence.broker.private_marker_absent_from_responses_and_audit = $true
  $evidence.broker.session_secret_absent_from_responses_and_audit = $true

  $stage = 'close the live app using its native tray exit and verify cleanup'
  $gracefulRequested = Request-FrontendExit -Frontend $frontendProcess
  $frontendExited = $false
  if ($gracefulRequested) { $frontendExited = $frontendProcess.WaitForExit(30000) }
  if (-not $gracefulRequested -or -not $frontendExited) {
    $evidence.frontend.forced_exit = $true
    throw 'native tray exit did not close the launched app within the bounded deadline'
  }
  $evidence.frontend.graceful_exit = $true
  $launcherExited = $launcherProcess.WaitForExit(30000)
  if (-not $launcherExited -or $launcherProcess.ExitCode -ne 0) {
    throw 'portable Rust launcher did not exit successfully after the frontend'
  }
  $evidence.cleanup.launcher_exit_code = [int]$launcherProcess.ExitCode
  $evidence.cleanup.endpoint_removed = -not (Test-Path -LiteralPath $endpointPath)
  if (-not $evidence.cleanup.endpoint_removed) { throw 'normal Broker endpoint remained after graceful shutdown' }
  $evidence.result = 'passed'
} catch {
  $runError = $_
  $evidence.failure_stage = $stage
} finally {
  if ($null -ne $launcherProbe) {
    $launcherProbe.Refresh()
    if (-not $launcherProbe.HasExited) {
      $stopped = Stop-ExactOwnedProcess -Process $launcherProbe -ExpectedPath (Join-Path $bundleRoot 'gui_shell_desktop_launcher.exe') -ExpectedHash ([string]$evidence.artifact_sha256.launcher).Replace('sha256:', '')
      if (-not $stopped) { $cleanupPassed = $false }
    }
  }
  if ($null -ne $frontendProcess) {
    $frontendProcess.Refresh()
    if (-not $frontendProcess.HasExited) {
      $stopped = Stop-ExactOwnedProcess -Process $frontendProcess -ExpectedPath (Join-Path $bundleRoot 'app\gui_shell_desktop.exe') -ExpectedHash ([string]$evidence.artifact_sha256.flutter_app).Replace('sha256:', '') -ExpectedParentId $frontendParentId
      if (-not $stopped) { $cleanupPassed = $false } else { $evidence.frontend.forced_exit = $true }
    }
  }
  if ($null -ne $launcherProcess) {
    $launcherProcess.Refresh()
    if (-not $launcherProcess.HasExited) {
      $stopped = Stop-ExactOwnedProcess -Process $launcherProcess -ExpectedPath (Join-Path $bundleRoot 'gui_shell_desktop_launcher.exe') -ExpectedHash ([string]$evidence.artifact_sha256.launcher).Replace('sha256:', '')
      if (-not $stopped) { $cleanupPassed = $false }
    }
    $launcherProcess.Refresh()
    if ($launcherProcess.HasExited -and $null -eq $evidence.cleanup.launcher_exit_code) {
      $evidence.cleanup.launcher_exit_code = [int]$launcherProcess.ExitCode
    }
  }
  if ($null -ne $sessionSecret) { $sessionSecret = $null }
  if ($null -ne $endpoint) { $endpoint.session_secret = $null }
  if ($null -ne $runtimeStore -and (Test-Path -LiteralPath $runtimeStore)) {
    try {
      $storeFullPath = [IO.Path]::GetFullPath($runtimeStore)
      if ($storeFullPath -cne [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA "D4Pocket\apps\$($evidence.fixture.app_id)\stores\$($evidence.fixture.audit_store_id)")) -or
          -not $storeFullPath.StartsWith($localAppDataRoot, [StringComparison]::OrdinalIgnoreCase) -or
          ($null -ne $launcherProcess -and -not $launcherProcess.HasExited)) {
        throw 'runtime fixture ownership or stopped-process check failed'
      }
      Remove-Item -LiteralPath $storeFullPath -Recurse -Force
      $evidence.cleanup.runtime_fixture_removed = -not (Test-Path -LiteralPath $storeFullPath)
      if (-not $evidence.cleanup.runtime_fixture_removed) { $cleanupPassed = $false }
    } catch {
      $cleanupPassed = $false
    }
  } else {
    $evidence.cleanup.runtime_fixture_removed = $true
  }
  if ($null -ne $endpointPath) {
    $evidence.cleanup.endpoint_removed = -not (Test-Path -LiteralPath $endpointPath)
    if (-not $evidence.cleanup.endpoint_removed) { $cleanupPassed = $false }
  }
}

if (-not $cleanupPassed -or
    -not $evidence.cleanup.endpoint_removed -or
    -not $evidence.cleanup.runtime_fixture_removed -or
    $evidence.frontend.forced_exit) {
  $evidence.result = 'failed'
  if ($null -eq $evidence.failure_stage) { $evidence.failure_stage = 'owned process or fixture cleanup' }
}
$evidence | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $evidenceFullPath -Encoding utf8
if ($null -ne $runError -or $evidence.result -ne 'passed') {
  throw "Windows Export portable launch smoke failed at stage: $($evidence.failure_stage)"
}
