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

function Get-NormalizedFlutterSemanticsName {
  param([AllowNull()][string]$Name)
  if ($null -eq $Name) { return '' }
  return [regex]::Replace($Name, '\r?\nタブ:\s*\d+/\d+$', '')
}

function Test-UiRectangleWithin {
  param(
    [Parameter(Mandatory = $true)]$OuterRectangle,
    [Parameter(Mandatory = $true)]$InnerRectangle
  )
  if ($OuterRectangle.Width -le 0 -or $OuterRectangle.Height -le 0 -or
      $InnerRectangle.Width -le 0 -or $InnerRectangle.Height -le 0) {
    return $false
  }
  return (
    $InnerRectangle.X -ge $OuterRectangle.X -and
    $InnerRectangle.Y -ge $OuterRectangle.Y -and
    ($InnerRectangle.X + $InnerRectangle.Width) -le ($OuterRectangle.X + $OuterRectangle.Width) + 1 -and
    ($InnerRectangle.Y + $InnerRectangle.Height) -le ($OuterRectangle.Y + $OuterRectangle.Height) + 1
  )
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
  $result = [ordered]@{
    callback_posted = $false
    exit_item_observed = $false
    exit_item_invoked = $false
    popup_window_observed = $false
    keyboard_fallback_attempted = $false
    keyboard_fallback_sent = $false
  }
  $Frontend.Refresh()
  if ($Frontend.HasExited -or $Frontend.MainWindowHandle -eq [IntPtr]::Zero) {
    return [pscustomobject]$result
  }
  Add-Type -AssemblyName UIAutomationClient
  Add-Type -AssemblyName UIAutomationTypes
  if (-not [GuiShellExportSmokeWin32]::PostMessage(
      $Frontend.MainWindowHandle,
      [uint32]0x8029,
      [IntPtr]::Zero,
      [IntPtr]0x0205)) {
    return [pscustomobject]$result
  }
  $result.callback_posted = $true
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
          $result.exit_item_observed = $true
          $pattern = $item.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
          $pattern.Invoke()
          $result.exit_item_invoked = $true
          return [pscustomobject]$result
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
  $popupWindow = [GuiShellExportSmokeWin32]::GetOwnedTrayPopup($Frontend.MainWindowHandle)
  if ($popupWindow -ne [IntPtr]::Zero) {
    $result.popup_window_observed = $true
    $result.keyboard_fallback_attempted = $true
    $result.keyboard_fallback_sent = [GuiShellExportSmokeWin32]::SendEndEnterToOwnedTrayPopup(
      $Frontend.MainWindowHandle
    )
  }
  return [pscustomobject]$result
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
    launcher_argument_refusal_dialog_observed = $false
    unexpected_frontend_spawned = $false
    launcher_started = $false
    direct_child_verified = $false
    main_window_observed = $false
    flutter_runner_uia_framework_id = $null
    flutter_runner_uia_control_type = $null
    flutter_runner_control_node_count = 0
    flutter_view_window_observed = $false
    flutter_view_window_class = $null
    flutter_view_child_window_count = 0
    flutter_view_native_handle = $null
    flutter_view_uia_framework_id = $null
    flutter_view_uia_control_type = $null
    flutter_view_uia_offscreen = $null
    ui_observation_status = 'not_run'
    ui_observation_failure_stage = $null
    ui_observation_control_node_count = 0
    ui_observation_known_labels = @()
    trace_inspector_ui_status = 'not_run'
    trace_inspector_span_count = 0
    trace_inspector_selected_operation_visible = $false
    trace_navigation_pointer_scroll_events = 0
    trace_navigation_target_in_view = $false
    trace_navigation_pointer_click_sent = $false
    trace_navigation_window_activated = $false
    trace_navigation_target_rect = $null
    trace_navigation_last_target_rect = $null
    included_surfaces = @()
    excluded_surfaces_absent = $false
    tray_exit_callback_posted = $false
    tray_exit_item_observed = $false
    tray_exit_item_invoked = $false
    tray_popup_window_observed = $false
    tray_keyboard_fallback_attempted = $false
    tray_keyboard_fallback_sent = $false
    graceful_exit_attempted = $false
    graceful_exit = $false
    forced_exit = $false
  }
  broker = [ordered]@{
    normal_endpoint_observed = $false
    selected_trace_operation_status = $null
    selected_trace_operation_audit_id = $null
    selected_trace_operation_authority_absent = $false
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
    audit_validation = [ordered]@{
      file_observed = $false
      selected_trace_event_id_bound = $false
      excluded_event_id_bound = $false
      excluded_operation_code_bound = $false
      command_event_id_bound = $false
      private_marker_absent = $false
      command_marker_absent = $false
      session_secret_absent = $false
    }
  }
  cleanup = [ordered]@{
    endpoint_removed = $false
    runtime_fixture_removed = $false
    launcher_exit_code = $null
    launcher_forced_exit = $false
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
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class GuiShellExportSmokeWin32 {
  private const uint kWindowOwner = 4;
  private const uint kKeyboardInput = 1;
  private const uint kMouseInput = 0;
  private const uint kKeyUp = 2;
  private const uint kMouseLeftDown = 0x0002;
  private const uint kMouseLeftUp = 0x0004;
  private const uint kMouseWheel = 0x0800;

   private delegate bool EnumThreadWindowsCallback(IntPtr window, IntPtr parameter);
   private delegate bool EnumChildWindowsCallback(IntPtr window, IntPtr parameter);

  [StructLayout(LayoutKind.Sequential)]
  private struct KeyboardInput {
    public ushort virtualKey;
    public ushort scanCode;
    public uint flags;
    public uint time;
    public IntPtr extraInfo;
  }

  [StructLayout(LayoutKind.Sequential)]
  private struct MouseInput {
    public int dx;
    public int dy;
    public uint mouseData;
    public uint flags;
    public uint time;
    public IntPtr extraInfo;
  }

  [StructLayout(LayoutKind.Explicit, Size = 32)]
  private struct InputUnion {
    [FieldOffset(0)] public KeyboardInput keyboard;
    [FieldOffset(0)] public MouseInput mouse;
  }

  [StructLayout(LayoutKind.Sequential)]
  private struct Input {
    public uint type;
    public InputUnion data;
  }

  [StructLayout(LayoutKind.Sequential)]
  private struct Point {
    public int x;
    public int y;
  }

  [DllImport("user32.dll", SetLastError = true)]
  private static extern bool SetCursorPos(int x, int y);

  [DllImport("user32.dll", SetLastError = true)]
  private static extern bool SetForegroundWindow(IntPtr window);

  [DllImport("user32.dll", SetLastError = true)]
  private static extern bool ShowWindow(IntPtr window, int command);

  [DllImport("user32.dll")]
  private static extern IntPtr WindowFromPoint(Point point);

  [DllImport("user32.dll", SetLastError = true)]
  private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

   [DllImport("user32.dll", SetLastError = true)]
   private static extern bool EnumChildWindows(IntPtr parent, EnumChildWindowsCallback callback, IntPtr parameter);

  [DllImport("user32.dll", SetLastError = true)]
  private static extern bool EnumThreadWindows(uint threadId, EnumThreadWindowsCallback callback, IntPtr parameter);

  [DllImport("user32.dll")]
  private static extern IntPtr GetWindow(IntPtr window, uint command);

  [DllImport("user32.dll")]
  private static extern bool IsWindowVisible(IntPtr window);

  [DllImport("user32.dll")]
  private static extern IntPtr GetForegroundWindow();

  [DllImport("user32.dll", SetLastError = true)]
  private static extern uint SendInput(uint inputCount, Input[] inputs, int inputSize);

  [DllImport("user32.dll", SetLastError = true)]
  private static extern bool ScreenToClient(IntPtr window, ref Point point);

  [DllImport("user32.dll", EntryPoint = "GetClassNameW", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
  private static extern int GetClassNameW(IntPtr window, StringBuilder className, int maximumCount);

  public static string GetWindowClassName(IntPtr window) {
    var className = new StringBuilder(256);
    int length = GetClassNameW(window, className, className.Capacity);
    return length > 0 ? className.ToString() : String.Empty;
  }

   public static uint GetWindowProcessId(IntPtr window) {
     uint processId;
     GetWindowThreadProcessId(window, out processId);
     return processId;
   }

   private static bool IsForegroundProcess(uint expectedProcessId) {
     IntPtr foreground = GetForegroundWindow();
     return foreground != IntPtr.Zero && GetWindowProcessId(foreground) == expectedProcessId;
   }

   private static bool PositionPointerForProcess(uint expectedProcessId, int x, int y) {
     if (!SetCursorPos(x, y) || !IsForegroundProcess(expectedProcessId)) return false;
     IntPtr underPointer = WindowFromPoint(new Point { x = x, y = y });
     return underPointer != IntPtr.Zero && GetWindowProcessId(underPointer) == expectedProcessId;
   }

   public static bool ActivateWindowForProcess(IntPtr window, uint expectedProcessId) {
     if (window == IntPtr.Zero || expectedProcessId == 0 ||
         GetWindowProcessId(window) != expectedProcessId ||
         GetWindowClassName(window) != "FLUTTER_RUNNER_WIN32_WINDOW") return false;
     ShowWindow(window, 9);
     SetForegroundWindow(window);
     return IsForegroundProcess(expectedProcessId);
   }

   public static bool SendMouseWheelToForegroundProcess(uint expectedProcessId, int x, int y, int delta) {
     if (expectedProcessId == 0 || delta == 0 ||
         !PositionPointerForProcess(expectedProcessId, x, y)) return false;
     Input input = new Input {
       type = kMouseInput,
       data = new InputUnion {
         mouse = new MouseInput {
           dx = 0,
           dy = 0,
           mouseData = unchecked((uint)delta),
           flags = kMouseWheel,
           time = 0,
           extraInfo = IntPtr.Zero
         }
       }
     };
     return SendInput(1, new Input[] { input }, Marshal.SizeOf(typeof(Input))) == 1;
   }

   public static bool ClickScreenPointInForegroundProcess(uint expectedProcessId, int x, int y) {
     if (expectedProcessId == 0 || !PositionPointerForProcess(expectedProcessId, x, y)) return false;
     Input[] inputs = new Input[] {
       MouseButtonInput(kMouseLeftDown),
       MouseButtonInput(kMouseLeftUp)
     };
     return SendInput((uint)inputs.Length, inputs, Marshal.SizeOf(typeof(Input))) == (uint)inputs.Length;
   }

   private static Input MouseButtonInput(uint flags) {
     return new Input {
       type = kMouseInput,
       data = new InputUnion {
         mouse = new MouseInput {
           dx = 0,
           dy = 0,
           mouseData = 0,
           flags = flags,
           time = 0,
           extraInfo = IntPtr.Zero
         }
       }
     };
   }

   public static IntPtr[] GetDescendantWindowsForProcess(IntPtr parent, uint expectedProcessId, int maximumCount) {
     if (parent == IntPtr.Zero) throw new ArgumentException("parent window handle is zero", "parent");
     if (expectedProcessId == 0) throw new ArgumentOutOfRangeException("expectedProcessId");
     if (maximumCount < 1) throw new ArgumentOutOfRangeException("maximumCount");
     var windows = new List<IntPtr>();
     bool exceeded = false;
     EnumChildWindowsCallback callback = (candidate, parameter) => {
       uint candidateProcessId;
       GetWindowThreadProcessId(candidate, out candidateProcessId);
       if (candidateProcessId == expectedProcessId) {
         if (windows.Count == maximumCount) {
           exceeded = true;
           return false;
         }
         windows.Add(candidate);
       }
       return true;
     };
     bool completed = EnumChildWindows(parent, callback, IntPtr.Zero);
     if (exceeded) throw new InvalidOperationException("descendant window inspection exceeded its bounded limit");
     if (!completed) throw new InvalidOperationException("descendant window inspection failed");
     return windows.ToArray();
   }

  [DllImport("user32.dll", SetLastError = true)]
  public static extern bool PostMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);

  public static IntPtr GetOwnedTrayPopup(IntPtr ownerWindow) {
    if (ownerWindow == IntPtr.Zero) return IntPtr.Zero;
    uint ownerProcessId;
    uint ownerThreadId = GetWindowThreadProcessId(ownerWindow, out ownerProcessId);
    if (ownerThreadId == 0 || ownerProcessId == 0) return IntPtr.Zero;
    IntPtr result = IntPtr.Zero;
    EnumThreadWindowsCallback callback = (candidate, parameter) => {
      uint candidateProcessId;
      GetWindowThreadProcessId(candidate, out candidateProcessId);
      if (candidateProcessId == ownerProcessId &&
          IsWindowVisible(candidate) &&
          GetWindowClassName(candidate) == "#32768" &&
          GetWindow(candidate, kWindowOwner) == ownerWindow) {
        result = candidate;
        return false;
      }
      return true;
    };
    EnumThreadWindows(ownerThreadId, callback, IntPtr.Zero);
    return result;
  }

  public static bool SendEndEnterToOwnedTrayPopup(IntPtr ownerWindow) {
    // Keyboard input is limited to the visible #32768 popup owned by the exact frontend HWND.
    IntPtr popup = GetOwnedTrayPopup(ownerWindow);
    if (popup == IntPtr.Zero) return false;
    IntPtr foreground = GetForegroundWindow();
    if (foreground != popup && foreground != ownerWindow) return false;
    Input[] inputs = new Input[] {
      KeyInput(0x23, false), KeyInput(0x23, true),
      KeyInput(0x0D, false), KeyInput(0x0D, true)
    };
    return SendInput((uint)inputs.Length, inputs, Marshal.SizeOf(typeof(Input))) == (uint)inputs.Length;
  }

  private static Input KeyInput(ushort virtualKey, bool keyUp) {
    return new Input {
      type = kKeyboardInput,
      data = new InputUnion {
        keyboard = new KeyboardInput {
          virtualKey = virtualKey,
          scanCode = 0,
          flags = keyUp ? kKeyUp : 0,
          time = 0,
          extraInfo = IntPtr.Zero
        }
      }
    };
  }
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
  $stage = 'verify launcher refusal dialog class'
  $probeDialogHandle = [IntPtr]$probeDialog.Current.NativeWindowHandle
  if ($probeDialogHandle -eq [IntPtr]::Zero -or
      [GuiShellExportSmokeWin32]::GetWindowClassName($probeDialogHandle) -cne '#32770') {
    throw 'launcher did not present a native refusal dialog'
  }
  $evidence.frontend.launcher_argument_refusal_dialog_observed = $true
  $stage = 'dismiss launcher refusal dialog'
  if (-not [GuiShellExportSmokeWin32]::PostMessage(
      $probeDialogHandle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero
    )) {
    throw 'launcher refusal dialog close request failed'
  }
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

  $stage = 'verify isolated runtime storage before launch'
  $runtimeStore = Join-Path $env:LOCALAPPDATA "D4Pocket\apps\$appId\stores\$auditStoreId"
  $runtimeStore = [IO.Path]::GetFullPath($runtimeStore)
  $localAppDataRoot = [IO.Path]::GetFullPath($env:LOCALAPPDATA).TrimEnd('\') + '\'
  if (-not $runtimeStore.StartsWith($localAppDataRoot, [StringComparison]::OrdinalIgnoreCase) -or
      (Test-Path -LiteralPath $runtimeStore)) {
    throw 'fixture runtime store already exists or escaped the runner LocalAppData root'
  }
  $endpointPath = Join-Path $runtimeStore 'broker_session.json'
  $auditPath = Join-Path (Join-Path $runtimeStore 'store') 'audit.jsonl'

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

  $stage = 'observe Flutter top-level window through UI Automation'
  $elements = @()
  $observedKnownNavigationLabels = @()
  try {
    $window = $null
    $uiDeadline = (Get-Date).AddSeconds(90)
    $condition = [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
      [int]$frontendProcess.Id
    )
    while ((Get-Date) -lt $uiDeadline -and $null -eq $window) {
      $frontendProcess.Refresh()
      if ($frontendProcess.HasExited) { throw 'Flutter child exited before UI observation' }
      $window = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
        [System.Windows.Automation.TreeScope]::Children, $condition
      )
      if ($null -eq $window -or $frontendProcess.MainWindowHandle -eq [IntPtr]::Zero) {
        $window = $null
        Start-Sleep -Milliseconds 250
      }
    }
    if ($null -eq $window) {
      throw 'Flutter main window was not observed through UI Automation'
    }
    $evidence.frontend.main_window_observed = $true
    $stage = 'verify Flutter native main-window handle'
    $frontendProcess.Refresh()
    $mainWindowHandle = [IntPtr]$frontendProcess.MainWindowHandle
    $observedWindowHandle = [IntPtr]$window.Current.NativeWindowHandle
    if ($mainWindowHandle -eq [IntPtr]::Zero -or $observedWindowHandle -ne $mainWindowHandle) {
      throw 'UI Automation process window does not match the verified Flutter main-window handle'
    }
    $stage = 'materialize Flutter runner UI Automation root before its semantics child'
    $flutterRunnerElement = [System.Windows.Automation.AutomationElement]::FromHandle($mainWindowHandle)
    if ($flutterRunnerElement.Current.ProcessId -ne $frontendProcess.Id) {
      throw 'Flutter runner UI Automation root belongs to another process'
    }
    $evidence.frontend.flutter_runner_uia_framework_id = [string]$flutterRunnerElement.Current.FrameworkId
    $evidence.frontend.flutter_runner_uia_control_type = [string]$flutterRunnerElement.Current.ControlType.ProgrammaticName
    $runnerControlElements = @(Get-ControlViewDescendants -RootElement $flutterRunnerElement)
    $evidence.frontend.flutter_runner_control_node_count = $runnerControlElements.Count
    $stage = 'materialize Flutter semantics from the same-process FLUTTERVIEW child window'
    $flutterViewElement = $null
    $flutterViewHandle = [IntPtr]::Zero
    $lastFlutterViewUiaError = $null
    $childWindowDeadline = (Get-Date).AddSeconds(60)
    while ((Get-Date) -lt $childWindowDeadline -and $null -eq $flutterViewElement) {
      $frontendProcess.Refresh()
      if ($frontendProcess.HasExited) { throw 'Flutter child exited before its FLUTTERVIEW semantics became available' }
      $childHandles = [GuiShellExportSmokeWin32]::GetDescendantWindowsForProcess(
        $mainWindowHandle, [uint32]$frontendProcess.Id, 256
      )
      $flutterViewHandles = @($childHandles | Where-Object {
        [GuiShellExportSmokeWin32]::GetWindowClassName($_) -ceq 'FLUTTERVIEW'
      })
      $evidence.frontend.flutter_view_child_window_count = $flutterViewHandles.Count
      if ($flutterViewHandles.Count -gt 1) {
        throw 'multiple same-process FLUTTERVIEW child windows were observed'
      }
      if ($flutterViewHandles.Count -eq 1) {
        $candidateHandle = [IntPtr]$flutterViewHandles[0]
        if ([GuiShellExportSmokeWin32]::GetWindowProcessId($candidateHandle) -ne [uint32]$frontendProcess.Id) {
          throw 'FLUTTERVIEW child window process identity changed during observation'
        }
        try {
          $candidateElement = [System.Windows.Automation.AutomationElement]::FromHandle($candidateHandle)
          $candidateFrameworkId = [string]$candidateElement.Current.FrameworkId
          $candidateControlType = [string]$candidateElement.Current.ControlType.ProgrammaticName
          $candidateOffscreen = [bool]$candidateElement.Current.IsOffscreen
          if ($candidateElement.Current.ProcessId -ne $frontendProcess.Id) {
            throw 'FLUTTERVIEW UI Automation root belongs to another process'
          }
          $flutterViewHandle = $candidateHandle
          $flutterViewElement = $candidateElement
          $evidence.frontend.flutter_view_uia_framework_id = $candidateFrameworkId
          $evidence.frontend.flutter_view_uia_control_type = $candidateControlType
          $evidence.frontend.flutter_view_uia_offscreen = $candidateOffscreen
        } catch {
          $lastFlutterViewUiaError = $_.Exception.GetType().Name
          $flutterViewElement = $null
        }
      }
      if ($null -eq $flutterViewElement) { Start-Sleep -Milliseconds 250 }
    }
    if ($null -eq $flutterViewElement) {
      $failureSuffix = if ($null -eq $lastFlutterViewUiaError) { '' } else { "; last_uia_error=$lastFlutterViewUiaError" }
      throw "same-process FLUTTERVIEW UI Automation root was not materialized before deadline$failureSuffix"
    }
    $evidence.frontend.flutter_view_window_observed = $true
    $evidence.frontend.flutter_view_window_class = [GuiShellExportSmokeWin32]::GetWindowClassName($flutterViewHandle)
    $evidence.frontend.flutter_view_native_handle = '0x{0:X}' -f $flutterViewHandle.ToInt64()
    $knownNavigationLabels = @('観測', '追跡', '診断', '履歴', '評価ラボ', 'ホスト能力', '通知', 'Host操作')
    $observedLabels = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $maximumObservedNodeCount = 0
    $navRoot = @()
    $uiDeadline = (Get-Date).AddSeconds(60)
    while ((Get-Date) -lt $uiDeadline -and $navRoot.Count -eq 0) {
      try {
        $elements = @(Get-ControlViewDescendants -RootElement $flutterViewElement)
      } catch {
        $elements = @()
      }
      if ($elements.Count -gt $maximumObservedNodeCount) {
        $maximumObservedNodeCount = $elements.Count
      }
      $candidateNodes = @($flutterViewElement) + $elements
      foreach ($node in $candidateNodes) {
        try {
          $name = Get-NormalizedFlutterSemanticsName -Name ([string]$node.Current.Name)
          if ($knownNavigationLabels -ccontains $name) { $null = $observedLabels.Add($name) }
        } catch {
          # Continue across nodes that disappear during UIA materialization.
        }
      }
      $navRoot = @($candidateNodes | Where-Object {
        try {
          (Get-NormalizedFlutterSemanticsName -Name ([string]$_.Current.Name)) -ceq 'ナビゲーション' -or
            $_.Current.AutomationId -ceq 'gui_shell.surface.navigation_rail'
        } catch { $false }
      } | Select-Object -First 1)
      if ($navRoot.Count -eq 0) { Start-Sleep -Milliseconds 250 }
    }
    $evidence.frontend.ui_observation_control_node_count = $maximumObservedNodeCount
    $observedKnownNavigationLabels = @($observedLabels | Sort-Object)
    if ($navRoot.Count -ne 1) {
      $evidence.frontend.ui_observation_known_labels = $observedKnownNavigationLabels
      $stage = "locate live NavigationRail Control View semantic root (max_control_nodes=$maximumObservedNodeCount; known_labels=$($observedKnownNavigationLabels -join ','))"
      throw 'NavigationRail semantic root was not observed through its native UIA fragment'
    }
    $stage = 'enumerate live NavigationRail semantic children'
    $navigationElements = @(Get-ControlViewDescendants -RootElement $navRoot[0])
    $navigationNames = @($navigationElements | ForEach-Object {
      try { Get-NormalizedFlutterSemanticsName -Name ([string]$_.Current.Name) } catch { '' }
    } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $includedSurfaces = @('観測', '追跡')
    foreach ($surface in $includedSurfaces) {
      $stage = "verify selected module surface: $surface"
      if ($navigationNames -cnotcontains $surface) { throw "selected module surface missing from live NavigationRail: $surface" }
    }
    $excludedSurfaces = @('診断', '履歴', '評価ラボ', 'ホスト能力', '通知', 'Host操作')
    foreach ($surface in $excludedSurfaces) {
      $stage = "verify excluded module surface absent: $surface"
      if ($navigationNames -ccontains $surface) { throw "excluded module surface remains in live NavigationRail: $surface" }
    }
    $evidence.frontend.included_surfaces = $includedSurfaces
    $evidence.frontend.excluded_surfaces_absent = $true
    $evidence.frontend.ui_observation_known_labels = @($navigationNames | Where-Object {
      @('観測', '追跡', '診断', '履歴', '評価ラボ', 'ホスト能力', '通知', 'Host操作') -ccontains $_
    } | Sort-Object -Unique)
    $evidence.frontend.ui_observation_status = 'passed'
  } catch {
    $evidence.frontend.ui_observation_status = 'not_established'
    $evidence.frontend.ui_observation_failure_stage = $stage
    $evidence.frontend.ui_observation_control_node_count = $elements.Count
    if ($observedKnownNavigationLabels.Count -gt 0) {
      $evidence.frontend.ui_observation_known_labels = $observedKnownNavigationLabels
    }
  }

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

  $stage = 'exercise selected Trace Inspector operation over authenticated loopback IPC'
  $selectedPayload = [ordered]@{ '上限' = 32; '版' = 1 }
  $selectedPayloadJson = ConvertTo-Json -InputObject $selectedPayload -Compress -Depth 8
  $selectedPayloadHash = 'sha256:' + (Get-Sha256Hex -Bytes ([Text.UTF8Encoding]::new($false).GetBytes($selectedPayloadJson)))
  $selectedRequest = [ordered]@{
    request_id = 'export-selected-' + [guid]::NewGuid().ToString('N')
    session_id = [string]$endpoint.session_id
    operation = '観測一覧'
    payload_hash = $selectedPayloadHash
    nonce = 'export-selected-' + [guid]::NewGuid().ToString('N')
    issued_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    metadata = [ordered]@{}
    payload = $selectedPayload
  }
  $selectedJson = ConvertTo-Json -InputObject $selectedRequest -Compress -Depth 8
  $selectedResult = Invoke-AuthenticatedRequest -Endpoint $endpoint -SessionSecret $sessionSecret -RequestJson $selectedJson
  $selectedBody = $selectedResult.Response.body
  if ($selectedResult.Json.Contains($sessionSecret) -or
      $selectedResult.Response.status -cne 'accepted' -or
      $selectedResult.Response.operation -cne '観測一覧' -or
      $null -ne $selectedResult.Response.error -or
      [string]::IsNullOrWhiteSpace([string]$selectedResult.Response.audit_event_id) -or
      $selectedBody.'権限生成' -cne 'なし' -or
      $selectedBody.'証拠種別' -cne 'INTERNAL_STATE' -or
      $selectedBody.'Span一覧' -isnot [System.Array] -or
      $selectedBody.'Trace一覧' -isnot [System.Array] -or
      $selectedBody.'Span一覧'.Count -lt 1 -or
      $selectedBody.'Trace一覧'.Count -lt 1) {
    throw 'selected Trace Inspector Broker operation was not accepted with bounded non-authoritative output'
  }
  $evidence.broker.selected_trace_operation_status = [string]$selectedResult.Response.status
  $evidence.broker.selected_trace_operation_audit_id = [string]$selectedResult.Response.audit_event_id
  $evidence.broker.selected_trace_operation_authority_absent = $selectedBody.'権限生成' -ceq 'なし'

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

  $stage = 'render selected Trace Inspector Broker data through the live Flutter consumer'
  $traceNavItem = $null
  if ($evidence.frontend.ui_observation_status -cne 'passed' -or $navRoot.Count -ne 1) {
    throw 'Trace Inspector navigation is unavailable because its live NavigationRail was not established'
  }
  $stage = 'activate the verified Flutter test window for bounded UI input'
  if (-not [GuiShellExportSmokeWin32]::ActivateWindowForProcess(
    $mainWindowHandle, [uint32]$frontendProcess.Id
  )) {
    throw 'Windows did not activate the verified Flutter test window'
  }
  $evidence.frontend.trace_navigation_window_activated = $true
  $traceNavigationClicked = $false
  $traceNavScrollLimit = 8
  $traceNavDeadline = (Get-Date).AddSeconds(15)
  while ((Get-Date) -lt $traceNavDeadline -and -not $traceNavigationClicked) {
    $flutterViewElement = [System.Windows.Automation.AutomationElement]::FromHandle($flutterViewHandle)
    $currentViewElements = @(Get-ControlViewDescendants -RootElement $flutterViewElement)
    $currentViewNodes = @($flutterViewElement) + $currentViewElements
    $currentNavRoots = @($currentViewNodes | Where-Object {
      try {
        (Get-NormalizedFlutterSemanticsName -Name ([string]$_.Current.Name)) -ceq 'ナビゲーション' -or
          $_.Current.AutomationId -ceq 'gui_shell.surface.navigation_rail'
      } catch { $false }
    } | Select-Object -First 2)
    if ($currentNavRoots.Count -ne 1) {
      throw "live NavigationRail root changed during Trace selection (count=$($currentNavRoots.Count))"
    }
    $traceNavigationElements = @(Get-ControlViewDescendants -RootElement $currentNavRoots[0])
    $traceNavMatches = @($traceNavigationElements | Where-Object {
      try { (Get-NormalizedFlutterSemanticsName -Name ([string]$_.Current.Name)) -ceq '追跡' } catch { $false }
    })
    if ($traceNavMatches.Count -gt 1) {
      throw 'multiple live Trace Inspector navigation labels were observed'
    }
    if ($traceNavMatches.Count -eq 1) {
      $traceNavItem = $traceNavMatches[0]
      if (-not $traceNavItem.Current.IsEnabled) {
        throw 'live Trace Inspector navigation destination is disabled'
      }
      $targetRectangle = $traceNavItem.Current.BoundingRectangle
      $navigationRectangle = $currentNavRoots[0].Current.BoundingRectangle
      $windowRectangle = $window.Current.BoundingRectangle
      $evidence.frontend.trace_navigation_last_target_rect = [ordered]@{
        x = [double]$targetRectangle.X
        y = [double]$targetRectangle.Y
        width = [double]$targetRectangle.Width
        height = [double]$targetRectangle.Height
      }
      $targetInNavigation = Test-UiRectangleWithin -OuterRectangle $navigationRectangle -InnerRectangle $targetRectangle
      $targetInWindow = Test-UiRectangleWithin -OuterRectangle $windowRectangle -InnerRectangle $targetRectangle
      if ($targetInNavigation -and $targetInWindow -and -not $traceNavItem.Current.IsOffscreen) {
        $clickX = [int][Math]::Floor($targetRectangle.X + ($targetRectangle.Width / 2))
        $clickY = [int][Math]::Floor($targetRectangle.Y + ($targetRectangle.Height / 2))
        $stage = 'click the visible Trace Inspector destination in the verified foreground Flutter process'
        if (-not [GuiShellExportSmokeWin32]::ClickScreenPointInForegroundProcess(
          [uint32]$frontendProcess.Id, $clickX, $clickY
        )) {
          throw 'Trace Inspector pointer click was refused because the Flutter process was not foreground or SendInput failed'
        }
        $evidence.frontend.trace_navigation_target_in_view = $true
        $evidence.frontend.trace_navigation_pointer_click_sent = $true
        $evidence.frontend.trace_navigation_target_rect = [ordered]@{
          x = [double]$targetRectangle.X
          y = [double]$targetRectangle.Y
          width = [double]$targetRectangle.Width
          height = [double]$targetRectangle.Height
        }
        $traceNavigationClicked = $true
      } else {
        if ([int]$evidence.frontend.trace_navigation_pointer_scroll_events -ge $traceNavScrollLimit) {
          throw 'Trace Inspector destination did not enter the visible NavigationRail after bounded scrolling'
        }
        $scrollX = [int][Math]::Floor($navigationRectangle.X + ($navigationRectangle.Width / 2))
        $scrollY = [int][Math]::Floor($navigationRectangle.Y + ($navigationRectangle.Height / 2))
        $stage = 'scroll the verified foreground NavigationRail to reveal Trace Inspector'
        if (-not [GuiShellExportSmokeWin32]::SendMouseWheelToForegroundProcess(
          [uint32]$frontendProcess.Id, $scrollX, $scrollY, -120
        )) {
          throw 'NavigationRail pointer scroll was refused because the Flutter process was not foreground or SendInput failed'
        }
        $evidence.frontend.trace_navigation_pointer_scroll_events =
          [int]$evidence.frontend.trace_navigation_pointer_scroll_events + 1
        Start-Sleep -Milliseconds 250
      }
    } else {
      Start-Sleep -Milliseconds 200
    }
  }
  if (-not $traceNavigationClicked) {
    throw 'Trace Inspector navigation destination was not available for a bounded live UI selection'
  }

  $traceUiDeadline = (Get-Date).AddSeconds(30)
  $traceUiNames = @()
  $traceUiText = ''
  $traceSpanCount = 0
  $traceSelectedOperationVisible = $false
  while ((Get-Date) -lt $traceUiDeadline) {
    $traceUiElements = @(Get-ControlViewDescendants -RootElement $flutterViewElement)
    $traceUiNames = @($traceUiElements | ForEach-Object {
      try { Get-NormalizedFlutterSemanticsName -Name ([string]$_.Current.Name) } catch { '' }
    } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $traceUiText = $traceUiNames -join "`n"
    $spanMatch = [regex]::Match($traceUiText, 'Span:\s*([0-9]+)')
    if ($spanMatch.Success) { $traceSpanCount = [int]$spanMatch.Groups[1].Value }
    $traceSelectedOperationVisible = $traceUiText.Contains('Broker / 観測一覧 / accepted')
    if ($traceUiText.Contains('追跡情報') -and $traceSpanCount -gt 0 -and $traceSelectedOperationVisible) {
      break
    }
    Start-Sleep -Milliseconds 250
  }
  if (-not $traceUiText.Contains('追跡情報') -or $traceSpanCount -lt 1 -or -not $traceSelectedOperationVisible) {
    throw 'selected Trace Inspector did not render the positive Broker observation through Flutter'
  }
  $evidence.frontend.trace_inspector_ui_status = 'passed'
  $evidence.frontend.trace_inspector_span_count = $traceSpanCount
  $evidence.frontend.trace_inspector_selected_operation_visible = $traceSelectedOperationVisible

  $stage = 'verify bounded Audit non-exposure and retain only safe evidence fields'
  if (-not (Test-Path -LiteralPath $auditPath -PathType Leaf)) { throw 'Broker Audit file was not created' }
  $evidence.broker.audit_validation.file_observed = $true
  $auditBytes = [IO.File]::ReadAllBytes($auditPath)
  $evidence.broker.audit_sha256 = 'sha256:' + (Get-Sha256Hex -Bytes $auditBytes)
  $evidence.broker.audit_bytes = $auditBytes.Length
  [Array]::Clear($auditBytes, 0, $auditBytes.Length)
  $auditText = Get-Content -LiteralPath $auditPath -Raw -Encoding utf8
  $evidence.broker.audit_validation.selected_trace_event_id_bound = $auditText.Contains([string]$selectedResult.Response.audit_event_id)
  $evidence.broker.audit_validation.excluded_event_id_bound = $auditText.Contains([string]$excludedResult.Response.audit_event_id)
  $evidence.broker.audit_validation.excluded_operation_code_bound = $auditText.Contains('optional_module_not_included')
  $evidence.broker.audit_validation.command_event_id_bound = $auditText.Contains([string]$commandResult.Response.audit_event_id)
  $evidence.broker.audit_validation.private_marker_absent = -not $auditText.Contains($privateMarker)
  $evidence.broker.audit_validation.command_marker_absent = -not $auditText.Contains($commandMarker)
  $evidence.broker.audit_validation.session_secret_absent = -not $auditText.Contains($sessionSecret)
  if (-not $evidence.broker.audit_validation.selected_trace_event_id_bound -or
      -not $evidence.broker.audit_validation.excluded_event_id_bound -or
      -not $evidence.broker.audit_validation.excluded_operation_code_bound -or
      -not $evidence.broker.audit_validation.command_event_id_bound -or
      -not $evidence.broker.audit_validation.private_marker_absent -or
      -not $evidence.broker.audit_validation.command_marker_absent -or
      -not $evidence.broker.audit_validation.session_secret_absent) {
    throw 'Audit does not bind both outcomes or exposed a private marker / session secret'
  }
  $evidence.broker.private_marker_absent_from_responses_and_audit = $true
  $evidence.broker.session_secret_absent_from_responses_and_audit = $true
} catch {
  $runError = $_
  $evidence.failure_stage = $stage
} finally {
  if ($null -ne $frontendProcess) {
    try {
      $frontendProcess.Refresh()
      if (-not $frontendProcess.HasExited) {
        $evidence.frontend.graceful_exit_attempted = $true
        $trayExit = Request-FrontendExit -Frontend $frontendProcess
        $evidence.frontend.tray_exit_callback_posted = [bool]$trayExit.callback_posted
        $evidence.frontend.tray_exit_item_observed = [bool]$trayExit.exit_item_observed
        $evidence.frontend.tray_exit_item_invoked = [bool]$trayExit.exit_item_invoked
        $evidence.frontend.tray_popup_window_observed = [bool]$trayExit.popup_window_observed
        $evidence.frontend.tray_keyboard_fallback_attempted = [bool]$trayExit.keyboard_fallback_attempted
        $evidence.frontend.tray_keyboard_fallback_sent = [bool]$trayExit.keyboard_fallback_sent
        $gracefulRequested = [bool]$trayExit.exit_item_invoked
        if ($trayExit.keyboard_fallback_sent) { $gracefulRequested = $true }
        $frontendExited = $false
        if ($gracefulRequested) { $frontendExited = $frontendProcess.WaitForExit(30000) }
        if ($gracefulRequested -and $frontendExited) {
          $evidence.frontend.graceful_exit = $true
        } elseif ($null -eq $evidence.failure_stage) {
          $evidence.failure_stage = 'native tray exit did not close the launched app within the bounded deadline'
        }
      } elseif ($null -eq $evidence.failure_stage) {
        $evidence.failure_stage = 'frontend exited before native tray exit could be verified'
      }
    } catch {
      if ($null -eq $evidence.failure_stage) {
        $evidence.failure_stage = 'native tray exit request failed'
      }
    }
  }
  if ($null -ne $launcherProcess -and $evidence.frontend.graceful_exit) {
    try {
      if (-not $launcherProcess.WaitForExit(30000) -or $launcherProcess.ExitCode -ne 0) {
        $cleanupPassed = $false
        if ($null -eq $evidence.failure_stage) {
          $evidence.failure_stage = 'portable launcher did not exit successfully after the frontend'
        }
      } else {
        $evidence.cleanup.launcher_exit_code = [int]$launcherProcess.ExitCode
      }
    } catch {
      $cleanupPassed = $false
      if ($null -eq $evidence.failure_stage) {
        $evidence.failure_stage = 'portable launcher exit could not be verified'
      }
    }
  }
  if ($null -ne $launcherProbe) {
    try {
      $launcherProbe.Refresh()
      if (-not $launcherProbe.HasExited) {
        $stopped = Stop-ExactOwnedProcess -Process $launcherProbe -ExpectedPath (Join-Path $bundleRoot 'gui_shell_desktop_launcher.exe') -ExpectedHash ([string]$evidence.artifact_sha256.launcher).Replace('sha256:', '')
        if (-not $stopped) { $cleanupPassed = $false }
      }
    } catch {
      $cleanupPassed = $false
      if ($null -eq $evidence.failure_stage) { $evidence.failure_stage = 'launcher refusal probe cleanup failed' }
    }
  }
  if ($null -ne $frontendProcess) {
    try {
      $frontendProcess.Refresh()
      if (-not $frontendProcess.HasExited) {
        $stopped = Stop-ExactOwnedProcess -Process $frontendProcess -ExpectedPath (Join-Path $bundleRoot 'app\gui_shell_desktop.exe') -ExpectedHash ([string]$evidence.artifact_sha256.flutter_app).Replace('sha256:', '') -ExpectedParentId $frontendParentId
        if (-not $stopped) {
          $cleanupPassed = $false
        } else {
          $evidence.frontend.forced_exit = $true
          if ($null -eq $evidence.failure_stage) { $evidence.failure_stage = 'frontend required exact-process forced cleanup' }
        }
      }
    } catch {
      $cleanupPassed = $false
      if ($null -eq $evidence.failure_stage) { $evidence.failure_stage = 'frontend process cleanup could not be verified' }
    }
  }
  if ($null -ne $launcherProcess) {
    try {
      $launcherProcess.Refresh()
      if (-not $launcherProcess.HasExited) {
        $stopped = Stop-ExactOwnedProcess -Process $launcherProcess -ExpectedPath (Join-Path $bundleRoot 'gui_shell_desktop_launcher.exe') -ExpectedHash ([string]$evidence.artifact_sha256.launcher).Replace('sha256:', '')
        if (-not $stopped) {
          $cleanupPassed = $false
        } else {
          $evidence.cleanup.launcher_forced_exit = $true
          if ($null -eq $evidence.failure_stage) { $evidence.failure_stage = 'portable launcher required exact-process forced cleanup' }
        }
      }
      $launcherProcess.Refresh()
      if ($launcherProcess.HasExited -and $null -eq $evidence.cleanup.launcher_exit_code) {
        $evidence.cleanup.launcher_exit_code = [int]$launcherProcess.ExitCode
      }
    } catch {
      $cleanupPassed = $false
      if ($null -eq $evidence.failure_stage) { $evidence.failure_stage = 'portable launcher process cleanup could not be verified' }
    }
  }
  if ($null -ne $sessionSecret) { $sessionSecret = $null }
  if ($null -ne $endpoint) { $endpoint.session_secret = $null }
  if ($null -ne $auditText) { $auditText = $null }
  if ($null -ne $privateMarker) { $privateMarker = $null }
  if ($null -ne $commandMarker) { $commandMarker = $null }
  if ($null -ne $runtimeStore) {
    try {
      if (Test-Path -LiteralPath $runtimeStore) {
        $storeFullPath = [IO.Path]::GetFullPath($runtimeStore)
        if ($storeFullPath -cne [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA "D4Pocket\apps\$($evidence.fixture.app_id)\stores\$($evidence.fixture.audit_store_id)")) -or
            -not $storeFullPath.StartsWith($localAppDataRoot, [StringComparison]::OrdinalIgnoreCase) -or
            ($null -ne $launcherProcess -and -not $launcherProcess.HasExited)) {
          throw 'runtime fixture ownership or stopped-process check failed'
        }
        Remove-Item -LiteralPath $storeFullPath -Recurse -Force
        $evidence.cleanup.runtime_fixture_removed = -not (Test-Path -LiteralPath $storeFullPath)
        if (-not $evidence.cleanup.runtime_fixture_removed) { $cleanupPassed = $false }
      } else {
        $evidence.cleanup.runtime_fixture_removed = $true
      }
    } catch {
      $cleanupPassed = $false
      if ($null -eq $evidence.failure_stage) { $evidence.failure_stage = 'runtime fixture cleanup failed' }
    }
  } else {
    $evidence.cleanup.runtime_fixture_removed = $true
  }
  if ($null -ne $endpointPath) {
    try {
      $evidence.cleanup.endpoint_removed = -not (Test-Path -LiteralPath $endpointPath)
      if (-not $evidence.cleanup.endpoint_removed) {
        $cleanupPassed = $false
        if ($null -eq $evidence.failure_stage) { $evidence.failure_stage = 'normal Broker endpoint remained after shutdown' }
      }
    } catch {
      $cleanupPassed = $false
      if ($null -eq $evidence.failure_stage) { $evidence.failure_stage = 'normal Broker endpoint cleanup could not be verified' }
    }
  }
}

if ($evidence.frontend.ui_observation_status -ne 'passed' -and $null -eq $evidence.failure_stage) {
  $evidence.failure_stage = 'UI Automation module surface observation was not established after independent Broker and cleanup checks'
}
if (-not $cleanupPassed -or
    -not $evidence.cleanup.endpoint_removed -or
    -not $evidence.cleanup.runtime_fixture_removed -or
    -not $evidence.frontend.graceful_exit -or
    $evidence.frontend.forced_exit -or
    $evidence.cleanup.launcher_forced_exit -or
    $evidence.cleanup.launcher_exit_code -ne 0) {
  $evidence.result = 'failed'
  if ($null -eq $evidence.failure_stage) { $evidence.failure_stage = 'owned process or fixture cleanup' }
} elseif ($null -ne $runError -or $evidence.frontend.ui_observation_status -ne 'passed') {
  $evidence.result = 'failed'
} else {
  $evidence.result = 'passed'
}
$evidence | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $evidenceFullPath -Encoding utf8
if ($null -ne $runError -or $evidence.result -ne 'passed') {
  throw "Windows Export portable launch smoke failed at stage: $($evidence.failure_stage)"
}
