[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$SetupPath,
  [Parameter(Mandatory = $true)][string]$RunRoot,
  [Parameter(Mandatory = $true)][string]$EvidencePath,
  [Parameter(Mandatory = $true)][string]$AppId,
  [Parameter(Mandatory = $true)][string]$AuditStoreId,
  [Parameter(Mandatory = $true)][string]$InitialVersion,
  [Parameter(Mandatory = $true)][string]$UpdateVersion,
  [Parameter(Mandatory = $true)][string]$SourceCommit,
  [Parameter(Mandatory = $true)][string]$InitialPackageSha256,
  [Parameter(Mandatory = $true)][string]$UpdatePackageSha256
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Windows.Forms
if ($null -eq ('D4PocketSetupLifecycleWin32' -as [type])) {
  Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class D4PocketSetupLifecycleWin32 {
  [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern IntPtr FindWindow(string className, string windowName);
  [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern int GetClassName(IntPtr window, StringBuilder className, int maxCount);
  [DllImport("user32.dll", SetLastError = true)]
  public static extern bool SetForegroundWindow(IntPtr window);
  [DllImport("user32.dll")]
  public static extern bool IsWindow(IntPtr window);
}
'@
}

$stage = 'validate scoped Windows runner paths'
$failure = $null
$result = 'failed'
$setupProcess = $null
$testIdentityCleanup = $false
$setupTempCleanup = $false
$script:startMenuFolderExistedBeforeTest = $false
$productRootAbsentAfterUninstall = $false
$startMenuAbsentAfterUninstall = $false
$userDataPreservedAfterUninstall = $false
$auditPreservedAfterUninstall = $false
$auditBytes = 0
$auditEventCount = 0
$auditSha256 = $null
$ownerDecisions = [System.Collections.Generic.List[object]]::new()
$evidence = [ordered]@{
  version = 1
  result = 'failed'
  source_commit = $SourceCommit
  app_id = $AppId
  audit_store_id = $AuditStoreId
  runner_image = "$env:ImageOS/$env:ImageVersion"
  evidence_class = 'LIVE_RUNTIME'
  authority_scope = 'isolated Developer test identity; real_owner_authority=false'
  real_owner_authority = $false
  setup_executed = $false
  setup_first_launch = $false
  setup_process_exited_after_handoff = $false
  initial_install = $false
  update = $false
  repair = $false
  rollback = $false
  uninstall = $false
  staged_state_not_reported_installed = $false
  initial_version = $InitialVersion
  update_version = $UpdateVersion
  initial_package_sha256 = $InitialPackageSha256
  update_package_sha256 = $UpdatePackageSha256
  product_root_absent_before_test_cleanup = $false
  start_menu_shortcut_absent_before_test_cleanup = $false
  product_root_absent_after_uninstall = $false
  start_menu_shortcut_absent_after_uninstall = $false
  user_data_preserved_after_uninstall = $false
  audit_store_preserved_after_uninstall = $false
  audit_event_count_after_uninstall = 0
  audit_sha256_after_uninstall = $null
  owner_decisions = @()
  test_identity_cleanup = $false
  setup_temp_and_process_cleanup = $false
  cleanup_errors = @()
  failure_stage = $null
}

function Get-UiDescendants {
  param([Parameter(Mandatory = $true)]$Root, [int]$Limit = 12000)
  $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
  $pending = [System.Collections.Generic.Stack[object]]::new()
  $items = [System.Collections.Generic.List[object]]::new()
  try {
    $child = $walker.GetFirstChild($Root)
    while ($null -ne $child -and $pending.Count -lt $Limit) {
      $pending.Push($child)
      $child = $walker.GetNextSibling($child)
    }
  } catch {
    return @()
  }
  while ($pending.Count -gt 0 -and $items.Count -lt $Limit) {
    $item = $pending.Pop()
    $items.Add($item)
    try {
      $child = $walker.GetFirstChild($item)
      $childCount = 0
      while ($null -ne $child -and $pending.Count + $items.Count -lt $Limit -and $childCount -lt 2048) {
        $pending.Push($child)
        $child = $walker.GetNextSibling($child)
        $childCount += 1
      }
    } catch {
      # UIA nodes can disappear while the Flutter tree updates.
    }
  }
  return $items.ToArray()
}

function Get-ProcessByImagePath {
  param([Parameter(Mandatory = $true)][string]$ImagePath)
  $expected = [IO.Path]::GetFullPath($ImagePath)
  @(Get-CimInstance -ClassName Win32_Process | Where-Object {
    -not [string]::IsNullOrWhiteSpace([string]$_.ExecutablePath) -and
    [string]::Equals([IO.Path]::GetFullPath([string]$_.ExecutablePath), $expected, [StringComparison]::OrdinalIgnoreCase)
  })
}

function Stop-OwnedProcessAtPath {
  param([Parameter(Mandatory = $true)][string]$ImagePath)
  $expected = [IO.Path]::GetFullPath($ImagePath)
  foreach ($owner in @(Get-ProcessByImagePath -ImagePath $expected)) {
    $processId = [int]$owner.ProcessId
    $process = Get-Process -Id $processId -ErrorAction Stop
    [void]$process.CloseMainWindow()
    if (-not $process.WaitForExit(15000)) {
      $stillOwned = @(Get-ProcessByImagePath -ImagePath $expected | Where-Object {
        [int]$_.ProcessId -eq $processId -and
        [string]::Equals([IO.Path]::GetFullPath([string]$_.ExecutablePath), $expected, [StringComparison]::OrdinalIgnoreCase)
      })
      if ($stillOwned.Count -eq 1) {
        Stop-Process -Id $processId -Force -ErrorAction Stop
        [void]$process.WaitForExit(15000)
      }
    }
    $remaining = @(Get-ProcessByImagePath -ImagePath $expected | Where-Object {
      [int]$_.ProcessId -eq $processId
    })
    if ($remaining.Count -gt 0) {
      throw 'A task-owned process could not be stopped from its exact generated image path.'
    }
  }
}

function Wait-ProcessByImagePath {
  param([string]$ImagePath, [int]$TimeoutSeconds = 120)
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ((Get-Date) -lt $deadline) {
    $matches = @(Get-ProcessByImagePath -ImagePath $ImagePath)
    if ($matches.Count -gt 0) { return $matches[0] }
    Start-Sleep -Milliseconds 250
  }
  throw 'Expected Product process did not appear before the bounded deadline.'
}

function Wait-WindowForProcess {
  param([Parameter(Mandatory = $true)][int]$ProcessId, [int]$TimeoutSeconds = 90)
  $desktop = [System.Windows.Automation.AutomationElement]::RootElement
  $condition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $ProcessId
  )
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ((Get-Date) -lt $deadline) {
    try {
      $window = $desktop.FindFirst([System.Windows.Automation.TreeScope]::Children, $condition)
      if ($null -ne $window -and $window.Current.NativeWindowHandle -ne 0) {
        [void][D4PocketSetupLifecycleWin32]::SetForegroundWindow([IntPtr]$window.Current.NativeWindowHandle)
        Start-Sleep -Milliseconds 250
        $window.SetFocus()
        return $window
      }
    } catch {
      # The Flutter window may be recreated during a fixed-root handoff.
    }
    Start-Sleep -Milliseconds 250
  }
  throw 'Product main window was not available to UI Automation before the bounded deadline.'
}

function Get-Name {
  param($Element)
  try { return [string]$Element.Current.Name } catch { return '' }
}

function Get-ControlTypeName {
  param($Element)
  try { return [string]$Element.Current.ControlType.ProgrammaticName } catch { return '' }
}

function Invoke-UiElement {
  param([Parameter(Mandatory = $true)]$Element)
  try {
    $pattern = $Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
    return
  } catch {
    try {
      $pattern = $Element.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)
      $pattern.Select()
      return
    } catch {
      throw 'UI Automation element did not expose a supported invocation pattern.'
    }
  }
}

function Find-Action {
  param($Window, [string[]]$Names, [int]$TimeoutSeconds = 25)
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ((Get-Date) -lt $deadline) {
    foreach ($item in @(Get-UiDescendants -Root $Window)) {
      $name = Get-Name $item
      $typeName = Get-ControlTypeName $item
      if ($Names -contains $name -and
          $typeName -in @('ControlType.Button', 'ControlType.RadioButton', 'ControlType.TabItem') -and
          $item.Current.IsEnabled -and -not $item.Current.IsOffscreen) {
        return $item
      }
    }
    Start-Sleep -Milliseconds 300
  }
  throw 'Expected enabled UI action was not exposed before the bounded deadline.'
}

function Find-ActionIfPresent {
  param($Window, [string[]]$Names, [int]$TimeoutSeconds = 2)
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ((Get-Date) -lt $deadline) {
    foreach ($item in @(Get-UiDescendants -Root $Window)) {
      if ($Names -contains (Get-Name $item) -and
          (Get-ControlTypeName $item) -in @('ControlType.Button', 'ControlType.RadioButton', 'ControlType.TabItem') -and
          -not $item.Current.IsOffscreen) {
        return $item
      }
    }
    Start-Sleep -Milliseconds 200
  }
  return $null
}

function Wait-NameElement {
  param($Window, [string]$Name, [int]$TimeoutSeconds = 30, [switch]$Contains)
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ((Get-Date) -lt $deadline) {
    foreach ($item in @(Get-UiDescendants -Root $Window)) {
      $candidate = Get-Name $item
      if (($Contains -and $candidate.Contains($Name, [StringComparison]::OrdinalIgnoreCase)) -or
          (-not $Contains -and $candidate -ceq $Name)) {
        return $item
      }
    }
    Start-Sleep -Milliseconds 250
  }
  throw 'Expected Product UI text was not observed before the bounded deadline.'
}

function Scroll-ToUpdateCenter {
  param($Window)
  for ($attempt = 0; $attempt -lt 16; $attempt++) {
    foreach ($item in @(Get-UiDescendants -Root $Window)) {
      if ((Get-Name $item) -ceq '更新センター') {
        try {
          $pattern = $item.GetCurrentPattern([System.Windows.Automation.ScrollItemPattern]::Pattern)
          $pattern.ScrollIntoView()
        } catch {
          # The title may already be within the visible viewport.
        }
        Start-Sleep -Milliseconds 250
        return
      }
    }
    $scrolled = $false
    foreach ($item in @(Get-UiDescendants -Root $Window)) {
      try {
        $pattern = $item.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
        if ($pattern.Current.VerticallyScrollable) {
          $pattern.Scroll([System.Windows.Automation.ScrollAmount]::LargeIncrement,
            [System.Windows.Automation.ScrollAmount]::NoAmount)
          $scrolled = $true
          break
        }
      } catch {
        # Try the next container.
      }
    }
    if (-not $scrolled) {
      [System.Windows.Forms.SendKeys]::SendWait('{PGDN}')
    }
    Start-Sleep -Milliseconds 350
  }
  throw 'Update Center was not reachable through the Settings UI.'
}

function Open-UpdateCenter {
  param([Parameter(Mandatory = $true)]$Window)
  $settings = Find-Action -Window $Window -Names @('設定')
  Invoke-UiElement $settings
  Scroll-ToUpdateCenter -Window $Window
  [void](Wait-NameElement -Window $Window -Name '更新センター' -TimeoutSeconds 15)
}

function Find-CandidateTitle {
  param($Window, [string]$UpdateId)
  $needle = "($UpdateId)"
  $items = @(Get-UiDescendants -Root $Window)
  $matches = @($items | Where-Object { (Get-Name $_).Contains($needle, [StringComparison]::Ordinal) })
  if ($matches.Count -eq 0) { return $null }
  $title = $matches | Sort-Object { $_.Current.BoundingRectangle.Top } | Select-Object -First 1
  try {
    $pattern = $title.GetCurrentPattern([System.Windows.Automation.ScrollItemPattern]::Pattern)
    $pattern.ScrollIntoView()
  } catch {
    # Continue with the visible tree and bounded geometric row selection.
  }
  return $title
}

function Find-CandidateButton {
  param($Window, [string]$UpdateId, [string]$ButtonName, [switch]$AllowDisabled)
  $title = Find-CandidateTitle -Window $Window -UpdateId $UpdateId
  if ($null -eq $title) { throw 'Signed update candidate is not visible in Update Center.' }
  $titleTop = [double]$title.Current.BoundingRectangle.Top
  $all = @(Get-UiDescendants -Root $Window)
  $nextTitleTop = [double]::PositiveInfinity
  foreach ($candidateId in @('windows-e04-first-install', 'windows-e04-update')) {
    if ($candidateId -eq $UpdateId) { continue }
    $other = @($all | Where-Object { (Get-Name $_).Contains("($candidateId)", [StringComparison]::Ordinal) }) |
      Sort-Object { $_.Current.BoundingRectangle.Top } | Select-Object -First 1
    if ($null -ne $other) {
      $otherTop = [double]$other.Current.BoundingRectangle.Top
      if ($otherTop -gt $titleTop -and $otherTop -lt $nextTitleTop) { $nextTitleTop = $otherTop }
    }
  }
  $buttons = @($all | Where-Object {
    (Get-Name $_) -ceq $ButtonName -and
    (Get-ControlTypeName $_) -eq 'ControlType.Button' -and
    [double]$_.Current.BoundingRectangle.Top -ge ($titleTop - 12) -and
    [double]$_.Current.BoundingRectangle.Top -lt $nextTitleTop
  } | Sort-Object { $_.Current.BoundingRectangle.Top }, { $_.Current.BoundingRectangle.Left })
  if ($buttons.Count -eq 0) { throw 'Candidate-specific UI action was not found in its visible row.' }
  $button = $buttons[0]
  if (-not $AllowDisabled -and -not $button.Current.IsEnabled) {
    throw 'Candidate action is disabled in the current Broker state.'
  }
  return $button
}

function Wait-OwnerDialog {
  param([string[]]$Markers, [int]$TimeoutSeconds = 35)
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ((Get-Date) -lt $deadline) {
    $handle = [D4PocketSetupLifecycleWin32]::FindWindow('#32770', 'D4 Pocket Owner確認')
    if ($handle -ne [IntPtr]::Zero) {
      $className = [Text.StringBuilder]::new(128)
      [void][D4PocketSetupLifecycleWin32]::GetClassName($handle, $className, $className.Capacity)
      if ($className.ToString() -cne '#32770') {
        throw 'Owner confirmation HWND is not a native dialog.'
      }
      $dialog = [System.Windows.Automation.AutomationElement]::FromHandle($handle)
      $pid = [int]$dialog.Current.ProcessId
      $allowedLauncherPaths = @(
        (Join-Path $script:bootstrapRoot 'gui_shell_desktop_launcher.exe'),
        (Join-Path $script:productRoot 'gui_shell_desktop_launcher.exe')
      )
      $ownerProcess = @(Get-CimInstance -ClassName Win32_Process | Where-Object {
        [int]$_.ProcessId -eq $pid -and
        -not [string]::IsNullOrWhiteSpace([string]$_.ExecutablePath) -and
        $allowedLauncherPaths -contains [IO.Path]::GetFullPath([string]$_.ExecutablePath)
      })
      if ($ownerProcess.Count -ne 1) {
        throw 'Owner dialog is not owned by the Export or installed Product Rust launcher.'
      }
      $labels = @(Get-UiDescendants -Root $dialog | ForEach-Object { Get-Name $_ })
      $dialogText = ($labels -join "`n")
      if (-not $dialogText.Contains('payload hash', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Owner confirmation does not expose the request payload hash.'
      }
      foreach ($marker in $Markers) {
        if (-not $dialogText.Contains($marker, [StringComparison]::OrdinalIgnoreCase)) {
          throw 'Owner confirmation content does not match the current test operation.'
        }
      }
      return [pscustomobject]@{ Handle = $handle; Element = $dialog; ProcessId = $pid }
    }
    Start-Sleep -Milliseconds 200
  }
  throw 'Native Product Owner confirmation was not displayed before the bounded deadline.'
}

function Resolve-OwnerDialog {
  param($OwnerDialog, [ValidateSet('approve', 'decline')][string]$Decision, [string]$Operation)
  $names = if ($Decision -eq 'approve') { @('はい', 'Yes') } else { @('いいえ', 'No') }
  $button = $null
  foreach ($item in @(Get-UiDescendants -Root $OwnerDialog.Element)) {
    if ((Get-ControlTypeName $item) -eq 'ControlType.Button' -and
        $names -contains (Get-Name $item) -and $item.Current.IsEnabled) {
      $button = $item
      break
    }
  }
  if ($null -eq $button) { throw 'Native Owner dialog did not expose its expected localized decision button.' }
  Invoke-UiElement $button
  $deadline = (Get-Date).AddSeconds(10)
  while ((Get-Date) -lt $deadline -and [D4PocketSetupLifecycleWin32]::IsWindow($OwnerDialog.Handle)) {
    Start-Sleep -Milliseconds 100
  }
  if ([D4PocketSetupLifecycleWin32]::IsWindow($OwnerDialog.Handle)) {
    throw 'Native Owner dialog remained open after the test decision.'
  }
  $ownerDecisions.Add([pscustomobject]@{ operation = $Operation; decision = $Decision; test_identity = $true })
}

function Invoke-CandidateAction {
  param(
    $Window,
    [string]$UpdateId,
    [string]$ButtonName,
    [string[]]$OwnerMarkers,
    [string]$Operation,
    [switch]$DeclineThenApprove
  )
  $button = Find-CandidateButton -Window $Window -UpdateId $UpdateId -ButtonName $ButtonName
  Invoke-UiElement $button
  if ($DeclineThenApprove) {
    $decline = Wait-OwnerDialog -Markers $OwnerMarkers
    Resolve-OwnerDialog -OwnerDialog $decline -Decision decline -Operation "$Operation`_decline"
    $windowRefreshed = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId -TimeoutSeconds 20
    $stageAction = Find-CandidateButton -Window $windowRefreshed -UpdateId $UpdateId -ButtonName '未起動版へ展開' -AllowDisabled
    if ($stageAction.Current.IsEnabled) {
      throw 'staged action became enabled after the declined package download.'
    }
    $downloadAgain = Find-CandidateButton -Window $windowRefreshed -UpdateId $UpdateId -ButtonName $ButtonName
    Invoke-UiElement $downloadAgain
  }
  $approved = Wait-OwnerDialog -Markers $OwnerMarkers
  Resolve-OwnerDialog -OwnerDialog $approved -Decision approve -Operation $Operation
  Start-Sleep -Milliseconds 500
}

function Get-SetupWindowCandidates {
  param([int]$SetupProcessId)
  $candidates = [System.Collections.Generic.List[object]]::new()
  $seenHandles = [System.Collections.Generic.HashSet[int]]::new()

  try {
    $process = Get-Process -Id $SetupProcessId -ErrorAction Stop
    $mainHandle = [int]$process.MainWindowHandle
    if ($mainHandle -ne 0) {
      $mainWindow = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$mainHandle)
      $nativeHandle = [int]$mainWindow.Current.NativeWindowHandle
      if ($nativeHandle -eq 0) { $nativeHandle = $mainHandle }
      if ($seenHandles.Add($nativeHandle)) { $candidates.Add($mainWindow) }
    }
  } catch {
    # The process main window may not be published before the Setup wizard appears.
  }

  try {
    $condition = [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $SetupProcessId
    )
    $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
      [System.Windows.Automation.TreeScope]::Children, $condition
    )
    foreach ($window in $windows) {
      $nativeHandle = 0
      try { $nativeHandle = [int]$window.Current.NativeWindowHandle } catch {}
      if ($nativeHandle -eq 0 -or $seenHandles.Add($nativeHandle)) { $candidates.Add($window) }
    }
  } catch {
    # Keep any process MainWindowHandle candidate and collect it in the timeout diagnostic.
  }

  return $candidates.ToArray()
}

function Get-SetupActionDiagnostic {
  param([int]$SetupProcessId, [object[]]$Windows)
  $details = [System.Collections.Generic.List[string]]::new()
  $process = Get-Process -Id $SetupProcessId -ErrorAction SilentlyContinue
  if ($null -eq $process) {
    $details.Add('process=exited')
  } else {
    $title = ([string]$process.MainWindowTitle -replace '[\r\n\t]+', ' ')
    if ($title.Length -gt 120) { $title = $title.Substring(0, 120) }
    $details.Add("process=alive main_window_handle=$([int]$process.MainWindowHandle) main_window_title='$title'")
  }

  $index = 0
  foreach ($window in @($Windows | Select-Object -First 8)) {
    $windowName = (Get-Name $window) -replace '[\r\n\t]+', ' '
    if ($windowName.Length -gt 120) { $windowName = $windowName.Substring(0, 120) }
    $windowClass = ''
    $windowHandle = 0
    try { $windowClass = [string]$window.Current.ClassName } catch {}
    try { $windowHandle = [int]$window.Current.NativeWindowHandle } catch {}
    $buttons = @(
      Get-UiDescendants -Root $window -Limit 1024 |
        Where-Object { (Get-ControlTypeName $_) -eq 'ControlType.Button' } |
        ForEach-Object {
          $buttonName = (Get-Name $_) -replace '[\r\n\t]+', ' '
          if ($buttonName.Length -gt 96) { $buttonName = $buttonName.Substring(0, 96) }
          if (-not [string]::IsNullOrWhiteSpace($buttonName)) {
            $enabled = $false
            try { $enabled = [bool]$_.Current.IsEnabled } catch {}
            "$buttonName|enabled=$enabled"
          }
        } | Select-Object -First 16
    )
    $buttonSummary = if ($buttons.Count -eq 0) { '<none>' } else { [string]::Join(',', [string[]]$buttons) }
    $details.Add("window[$index] name='$windowName' class='$windowClass' hwnd=$windowHandle buttons=[$buttonSummary]")
    $index += 1
  }

  $diagnostic = [string]::Join('; ', $details.ToArray())
  if ($diagnostic.Length -gt 2400) { $diagnostic = $diagnostic.Substring(0, 2400) }
  return $diagnostic
}

function Wait-SetupAction {
  param([int]$SetupProcessId, [string[]]$Names, [int]$TimeoutSeconds = 60)
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  $windows = @()
  while ((Get-Date) -lt $deadline) {
    $windows = @(Get-SetupWindowCandidates -SetupProcessId $SetupProcessId)
    foreach ($window in $windows) {
      foreach ($item in @(Get-UiDescendants -Root $window)) {
        $enabled = $false
        try { $enabled = [bool]$item.Current.IsEnabled } catch {}
        if ($Names -contains (Get-Name $item) -and
            (Get-ControlTypeName $item) -eq 'ControlType.Button' -and $enabled) {
          return [pscustomobject]@{ Window = $window; Button = $item; Name = (Get-Name $item) }
        }
      }
    }
    Start-Sleep -Milliseconds 300
  }
  $diagnostic = Get-SetupActionDiagnostic -SetupProcessId $SetupProcessId -Windows $windows
  throw "Inno Setup wizard did not expose the expected enabled navigation action; setup_ui_diagnostic=$diagnostic"
}

function Get-AuditSummary {
  param([string]$AuditPath)
  if (-not (Test-Path -LiteralPath $AuditPath -PathType Leaf)) {
    throw 'Product Broker Audit file is missing.'
  }
  $raw = [IO.File]::ReadAllBytes($AuditPath)
  $text = [Text.Encoding]::UTF8.GetString($raw)
  $lines = @($text -split "`r?`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
  [pscustomobject]@{
    bytes = $raw.Length
    events = $lines.Count
    sha256 = ([Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($raw))).ToLowerInvariant()
    contains_test_candidate_ids = $text.Contains('windows-e04-first-install') -and $text.Contains('windows-e04-update')
    private_test_key_absent = -not $text.Contains('9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60')
  }
}

function Get-ProductVersionPath {
  param([string]$Version, [string]$PackageHash)
  Join-Path (Join-Path $script:productRoot 'versions') "$Version-$PackageHash"
}

function Open-InstalledProduct {
  param([string]$Version, [string]$PackageHash)
  $versionPath = Get-ProductVersionPath -Version $Version -PackageHash $PackageHash
  $appPath = Join-Path $versionPath 'app\gui_shell_desktop.exe'
  if (-not (Test-Path -LiteralPath $appPath -PathType Leaf)) {
    throw 'Selected active version does not have its expected Flutter executable.'
  }
  $launcherPath = Join-Path $script:productRoot 'gui_shell_desktop_launcher.exe'
  if (-not (Test-Path -LiteralPath $launcherPath -PathType Leaf)) {
    throw 'Fixed installed-root launcher is missing.'
  }
  Start-Process -FilePath $launcherPath -WorkingDirectory $script:productRoot | Out-Null
  $process = Wait-ProcessByImagePath -ImagePath $appPath -TimeoutSeconds 90
  $script:activeFrontendProcessId = [int]$process.ProcessId
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId
  [pscustomobject]@{ Process = $process; Window = $window; AppPath = $appPath; LauncherPath = $launcherPath }
}

function Close-InstalledProduct {
  param([int]$ProcessId, [string]$ExpectedAppPath)
  $matches = @(Get-ProcessByImagePath -ImagePath $ExpectedAppPath | Where-Object { [int]$_.ProcessId -eq $ProcessId })
  if ($matches.Count -ne 1) { throw 'Current Product process identity changed before graceful close.' }
  $process = Get-Process -Id $ProcessId -ErrorAction Stop
  [void]$process.CloseMainWindow()
  if (-not $process.WaitForExit(30000)) {
    throw 'Product UI did not close gracefully before the bounded deadline.'
  }
}

try {
  if ($env:RUNNER_TEMP -eq '') { throw 'RUNNER_TEMP is unavailable.' }
  $runnerTemp = [IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd('\') + '\'
  $script:runRoot = [IO.Path]::GetFullPath($RunRoot)
  $script:evidencePath = [IO.Path]::GetFullPath($EvidencePath)
  $setupResolved = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $SetupPath).Path)
  if (-not $script:runRoot.StartsWith($runnerTemp, [StringComparison]::OrdinalIgnoreCase) -or
      -not $script:evidencePath.StartsWith($runnerTemp, [StringComparison]::OrdinalIgnoreCase) -or
      -not $setupResolved.StartsWith($script:runRoot.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Setup, run root, or evidence path escaped the isolated runner workspace.'
  }
  if ($AppId -notmatch '^d4-pocket-app-[0-9a-f]{32}$' -or
      $AuditStoreId -notmatch '^audit-store-[0-9a-f]{32}$' -or
      $InitialPackageSha256 -notmatch '^[0-9a-f]{64}$' -or
      $UpdatePackageSha256 -notmatch '^[0-9a-f]{64}$') {
    throw 'Export identity or package digest has an invalid shape.'
  }
  if (Test-Path -LiteralPath $script:evidencePath) { throw 'Lifecycle evidence path already exists.' }

  $stage = 'resolve isolated user installation paths'
  $script:localAppData = [IO.Path]::GetFullPath([Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData))
  $script:productRoot = Join-Path $script:localAppData "Programs\D4 Pocket\$AppId"
  $script:runtimeRoot = Join-Path $script:localAppData "D4Pocket\apps\$AppId\stores\$AuditStoreId"
  $script:storeRoot = Join-Path $script:runtimeRoot 'store'
  $script:auditPath = Join-Path $script:storeRoot 'audit.jsonl'
  $startMenuRoot = [IO.Path]::GetFullPath([Environment]::GetFolderPath([Environment+SpecialFolder]::StartMenu))
  $script:startMenuFolder = Join-Path $startMenuRoot 'Programs\D4 Pocket'
  $script:startMenuShortcut = Join-Path $script:startMenuFolder 'D4 Pocket.lnk'
  $script:bootstrapRoot = Join-Path ([IO.Path]::GetFullPath([IO.Path]::GetTempPath())) "D4PocketBootstrap-$AppId"
  $script:userDataSentinel = Join-Path $script:runtimeRoot 'w-e04-developer-preserve-sentinel.txt'
  $script:sentinelText = "isolated-w-e04-$SourceCommit-$AppId`n"

  if ((Test-Path -LiteralPath $script:productRoot) -or
      (Test-Path -LiteralPath $script:startMenuShortcut) -or
      (Test-Path -LiteralPath $script:bootstrapRoot)) {
    throw 'Generated Product installation, Start Menu shortcut, or Setup temporary root already exists.'
  }
  if (-not (Test-Path -LiteralPath $script:runtimeRoot -PathType Container) -or
      -not (Test-Path -LiteralPath $script:storeRoot -PathType Container)) {
    throw 'The per-identity Broker seed state is missing before normal Setup.'
  }
  foreach ($seedDirectory in @($script:runtimeRoot, $script:storeRoot)) {
    $seedMetadata = Get-Item -LiteralPath $seedDirectory -Force -ErrorAction Stop
    if (-not $seedMetadata.PSIsContainer -or
        ($seedMetadata.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw 'The per-identity Broker seed root is not a plain directory.'
    }
  }
  $packageCache = Join-Path $script:storeRoot 'update_packages'
  if (-not (Test-Path -LiteralPath $packageCache -PathType Container)) {
    throw 'The per-identity Broker package cache is missing.'
  }
  $expectedPackages = @(
    @{ Name = "$InitialPackageSha256.pkg"; Hash = $InitialPackageSha256 },
    @{ Name = "$UpdatePackageSha256.pkg"; Hash = $UpdatePackageSha256 }
  )
  foreach ($expectedPackage in $expectedPackages) {
    $cachedPackage = Join-Path $packageCache $expectedPackage.Name
    $cachedMetadata = Get-Item -LiteralPath $cachedPackage -Force -ErrorAction Stop
    if ($cachedMetadata.PSIsContainer -or
        ($cachedMetadata.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
        $cachedMetadata.Length -le 0) {
      throw 'The expected Broker cached package is not a plain nonempty file.'
    }
    $cachedHash = (Get-FileHash -LiteralPath $cachedPackage -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($cachedHash -cne $expectedPackage.Hash) {
      throw 'The Broker cached package bytes differ from the generated D4PKG input.'
    }
  }
  if (Test-Path -LiteralPath $script:startMenuFolder) {
    $script:startMenuFolderExistedBeforeTest = $true
    $startMenuFolderMetadata = Get-Item -LiteralPath $script:startMenuFolder -Force -ErrorAction Stop
    if (-not $startMenuFolderMetadata.PSIsContainer -or
        ($startMenuFolderMetadata.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw 'The existing D4 Pocket Start Menu folder is not a plain directory.'
    }
    $existingShortcuts = @(Get-ChildItem -LiteralPath $script:startMenuFolder -Force -ErrorAction Stop)
    if ($existingShortcuts.Count -gt 0) {
      throw 'Shared D4 Pocket Start Menu folder contains pre-existing user data.'
    }
  }
  if (Test-Path -LiteralPath $script:userDataSentinel) {
    throw 'The isolated per-identity preservation sentinel already exists.'
  }
  foreach ($packageName in @("$InitialPackageSha256.pkg", "$UpdatePackageSha256.pkg")) {
    $expectedCachePath = Join-Path $packageCache $packageName
    if (-not (Test-Path -LiteralPath $expectedCachePath -PathType Leaf)) {
      throw 'The seeded Broker cache does not contain both expected D4PKG packages.'
    }
  }
  $evidence.product_root_absent_before_test_cleanup = $true
  $evidence.start_menu_shortcut_absent_before_test_cleanup = $true
  $evidence.start_menu_folder_existed_before_test = $script:startMenuFolderExistedBeforeTest

  $stage = 'execute normal Inno Setup wizard'
  $setupHash = (Get-FileHash -LiteralPath $setupResolved -Algorithm SHA256).Hash.ToLowerInvariant()
  $setupProcess = Start-Process -FilePath $setupResolved -PassThru
  $evidence.setup_executed = $true
  $next = Wait-SetupAction -SetupProcessId $setupProcess.Id -Names @('次へ(&N) >', '次へ >', '次へ', 'Next >', 'Next')
  Invoke-UiElement $next.Button
  $install = Wait-SetupAction -SetupProcessId $setupProcess.Id -Names @('インストール(&I)', 'インストール', 'Install')
  Invoke-UiElement $install.Button

  $stage = 'observe setup-launched Product window before Broker install'
  $bootstrapLauncher = Join-Path $script:bootstrapRoot 'gui_shell_desktop_launcher.exe'
  $bootstrapApp = Join-Path $script:bootstrapRoot 'app\gui_shell_desktop.exe'
  $initialProcess = Wait-ProcessByImagePath -ImagePath $bootstrapApp -TimeoutSeconds 180
  $script:activeFrontendProcessId = [int]$initialProcess.ProcessId
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId -TimeoutSeconds 120
  $evidence.setup_first_launch = $true
  if ((Test-Path -LiteralPath $script:productRoot) -or (Test-Path -LiteralPath $script:startMenuShortcut)) {
    throw 'Setup bootstrap launch created an Installed marker before Broker activation.'
  }
  $evidence.staged_state_not_reported_installed = $true

  $stage = 'navigate initial Setup-launched Product to Update Center'
  Open-UpdateCenter -Window $window
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId
  $initialDownloadMarkers = @('windows-e04-first-install', $InitialVersion, $InitialPackageSha256, 'updates.example.com')
  Invoke-CandidateAction -Window $window -UpdateId 'windows-e04-first-install' -ButtonName 'download' `
    -OwnerMarkers $initialDownloadMarkers -Operation 'first_install_download' -DeclineThenApprove
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId

  $stage = 'approve initial package staging'
  Invoke-CandidateAction -Window $window -UpdateId 'windows-e04-first-install' -ButtonName '未起動版へ展開' `
    -OwnerMarkers @('windows-e04-first-install', $InitialVersion, $InitialPackageSha256) `
    -Operation 'first_install_stage'
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId
  $stagedUninstallAction = Find-ActionIfPresent -Window $window -Names @('D4 Pocketをアンインストール') -TimeoutSeconds 2
  if ((Test-Path -LiteralPath $script:startMenuShortcut) -or $null -ne $stagedUninstallAction) {
    throw 'Staged package was presented as an installed Product before activation.'
  }

  $stage = 'approve first install activation'
  Invoke-CandidateAction -Window $window -UpdateId 'windows-e04-first-install' -ButtonName 'インストール' `
    -OwnerMarkers @('windows-e04-first-install', $InitialVersion, $InitialPackageSha256) `
    -Operation 'first_install_activation'
  $initialVersionPath = Get-ProductVersionPath -Version $InitialVersion -PackageHash $InitialPackageSha256
  $initialAppPath = Join-Path $initialVersionPath 'app\gui_shell_desktop.exe'
  $installedFirst = Wait-ProcessByImagePath -ImagePath $initialAppPath -TimeoutSeconds 180
  $script:activeFrontendProcessId = [int]$installedFirst.ProcessId
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId -TimeoutSeconds 120
  $evidence.initial_install = $true
  if (-not (Test-Path -LiteralPath (Join-Path $script:productRoot 'gui_shell_desktop_launcher.exe') -PathType Leaf)) {
    throw 'Fixed installed-root launcher was not created by first-install activation.'
  }
  if (-not (Test-Path -LiteralPath $script:startMenuShortcut -PathType Leaf)) {
    throw 'Start Menu shortcut was not registered by first-install activation.'
  }
  $setupFinish = Wait-SetupAction -SetupProcessId $setupProcess.Id -Names @('完了(&F)', '完了', 'Finish') -TimeoutSeconds 45
  Invoke-UiElement $setupFinish.Button
  if (-not $setupProcess.WaitForExit(30000)) {
    throw 'Normal Setup process did not exit after the initial installed-version handoff.'
  }
  $evidence.setup_process_exited_after_handoff = $true

  $stage = 'open installed Product Update Center'
  Open-UpdateCenter -Window $window
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId
  $updateId = 'windows-e04-update'
  $updateMarkers = @($updateId, $UpdateVersion, $UpdatePackageSha256, 'updates.example.com')
  Invoke-CandidateAction -Window $window -UpdateId $updateId -ButtonName 'download' `
    -OwnerMarkers $updateMarkers -Operation 'update_download'
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId
  Invoke-CandidateAction -Window $window -UpdateId $updateId -ButtonName '未起動版へ展開' `
    -OwnerMarkers @($updateId, $UpdateVersion, $UpdatePackageSha256) -Operation 'update_stage'
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId
  Invoke-CandidateAction -Window $window -UpdateId $updateId -ButtonName '更新を有効化' `
    -OwnerMarkers @($updateId, $UpdateVersion, $UpdatePackageSha256) -Operation 'update_activation'
  $evidence.update = $true

  $stage = 'restart through fixed root and verify updated version selection'
  Close-InstalledProduct -ProcessId $script:activeFrontendProcessId -ExpectedAppPath $initialAppPath
  $updateVersionPath = Get-ProductVersionPath -Version $UpdateVersion -PackageHash $UpdatePackageSha256
  $updateAppPath = Join-Path $updateVersionPath 'app\gui_shell_desktop.exe'
  $updated = Open-InstalledProduct -Version $UpdateVersion -PackageHash $UpdatePackageSha256
  $window = $updated.Window
  if ([IO.Path]::GetFullPath($updated.AppPath) -cne [IO.Path]::GetFullPath($updateAppPath)) {
    throw 'Fixed root did not select the newly activated version.'
  }

  $stage = 'rollback through installed Update Center'
  Open-UpdateCenter -Window $window
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId
  Invoke-CandidateAction -Window $window -UpdateId $updateId -ButtonName '直前版へ戻す' `
    -OwnerMarkers @($UpdateVersion, $UpdatePackageSha256, $InitialVersion, $InitialPackageSha256, $AppId, $AuditStoreId) `
    -Operation 'rollback'
  $evidence.rollback = $true
  Close-InstalledProduct -ProcessId $script:activeFrontendProcessId -ExpectedAppPath $updateAppPath
  $rolledBack = Open-InstalledProduct -Version $InitialVersion -PackageHash $InitialPackageSha256
  $window = $rolledBack.Window

  $stage = 'repair missing generated Start Menu entry'
  if (-not (Test-Path -LiteralPath $script:startMenuShortcut -PathType Leaf)) {
    throw 'Start Menu shortcut disappeared before the repair fixture was applied.'
  }
  $shell = New-Object -ComObject WScript.Shell
  $shortcut = $shell.CreateShortcut($script:startMenuShortcut)
  $shortcutTarget = [IO.Path]::GetFullPath([string]$shortcut.TargetPath)
  $fixedLauncher = [IO.Path]::GetFullPath((Join-Path $script:productRoot 'gui_shell_desktop_launcher.exe'))
  if (-not [string]::Equals($shortcutTarget, $fixedLauncher, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Generated Start Menu shortcut does not target this Export identity fixed launcher.'
  }
  Remove-Item -LiteralPath $script:startMenuShortcut -Force
  if (Test-Path -LiteralPath $script:startMenuShortcut) { throw 'Repair fixture failed to remove the generated shortcut.' }
  Open-UpdateCenter -Window $window
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId
  $repairButton = Find-Action -Window $window -Names @('起動項目を修復')
  Invoke-UiElement $repairButton
  $repairDialog = Wait-OwnerDialog -Markers @('起動項目', $AppId, $InitialVersion, $InitialPackageSha256)
  Resolve-OwnerDialog -OwnerDialog $repairDialog -Decision approve -Operation 'repair'
  $deadline = (Get-Date).AddSeconds(45)
  while ((Get-Date) -lt $deadline -and -not (Test-Path -LiteralPath $script:startMenuShortcut -PathType Leaf)) {
    Start-Sleep -Milliseconds 250
  }
  if (-not (Test-Path -LiteralPath $script:startMenuShortcut -PathType Leaf)) {
    throw 'Broker repair did not restore the generated Start Menu shortcut.'
  }
  $shortcut = $shell.CreateShortcut($script:startMenuShortcut)
  if (-not [string]::Equals([IO.Path]::GetFullPath([string]$shortcut.TargetPath), $fixedLauncher, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Broker repair restored a shortcut to the wrong Product identity.'
  }
  $evidence.repair = $true

  $stage = 'write isolated per-identity preservation sentinel'
  if (-not (Test-Path -LiteralPath $script:runtimeRoot -PathType Container)) {
    throw 'Broker runtime identity directory is missing before uninstall.'
  }
  [IO.File]::WriteAllText($script:userDataSentinel, $script:sentinelText, [Text.UTF8Encoding]::new($false))
  $sentinelHash = (Get-FileHash -LiteralPath $script:userDataSentinel -Algorithm SHA256).Hash.ToLowerInvariant()
  $auditBefore = Get-AuditSummary -AuditPath $script:auditPath

  $stage = 'uninstall exact Export Product through the installed UI'
  $window = Wait-WindowForProcess -ProcessId $script:activeFrontendProcessId
  $uninstallButton = Find-Action -Window $window -Names @('D4 Pocketをアンインストール')
  Invoke-UiElement $uninstallButton
  $uninstallDialog = Wait-OwnerDialog -Markers @('アンインストール', $AppId)
  Resolve-OwnerDialog -OwnerDialog $uninstallDialog -Decision approve -Operation 'uninstall'
  $uninstallDeadline = (Get-Date).AddSeconds(90)
  while ((Get-Date) -lt $uninstallDeadline -and
      ((Test-Path -LiteralPath $script:productRoot) -or (Get-ProcessByImagePath -ImagePath $initialAppPath).Count -gt 0)) {
    Start-Sleep -Milliseconds 300
  }
  $evidence.uninstall = -not (Test-Path -LiteralPath $script:productRoot)
  if (-not $evidence.uninstall) { throw 'Product uninstall did not remove its fixed installed root.' }
  $script:activeFrontendProcessId = 0

  $stage = 'verify uninstall preserves per-identity user data and Audit'
  if (-not (Test-Path -LiteralPath $script:userDataSentinel -PathType Leaf)) {
    throw 'Uninstall removed the isolated per-identity user-data sentinel.'
  }
  $sentinelAfterHash = (Get-FileHash -LiteralPath $script:userDataSentinel -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($sentinelAfterHash -cne $sentinelHash) { throw 'Uninstall changed the user-data sentinel bytes.' }
  $auditAfter = Get-AuditSummary -AuditPath $script:auditPath
  if (-not $auditAfter.contains_test_candidate_ids -or -not $auditAfter.private_test_key_absent) {
    throw 'Post-uninstall Audit lost candidate history or contains a private test key marker.'
  }
  if ([int]$auditAfter.events -le [int]$auditBefore.events) {
    throw 'Uninstall did not append a durable Audit event.'
  }
  if (Test-Path -LiteralPath $script:startMenuShortcut) {
    throw 'Uninstall left the Product Start Menu shortcut behind.'
  }
  $userDataPreservedAfterUninstall = $true
  $auditPreservedAfterUninstall = $true
  $productRootAbsentAfterUninstall = $true
  $startMenuAbsentAfterUninstall = $true
  $auditBytes = [int]$auditAfter.bytes
  $auditEventCount = [int]$auditAfter.events
  $auditSha256 = [string]$auditAfter.sha256
  $evidence.user_data_preserved_after_uninstall = $true
  $evidence.audit_store_preserved_after_uninstall = $true
  $evidence.product_root_absent_after_uninstall = $true
  $evidence.start_menu_shortcut_absent_after_uninstall = $true
  $evidence.audit_event_count_after_uninstall = $auditEventCount
  $evidence.audit_sha256_after_uninstall = $auditSha256
  $evidence.sentinel_sha256 = $sentinelHash
  $evidence.audit_before_uninstall = $auditBefore
  $evidence.audit_after_uninstall = $auditAfter

  $stage = 'verify no owned Product process remains'
  $ownedProcessPaths = @(
    (Join-Path $script:bootstrapRoot 'gui_shell_desktop_launcher.exe'),
    (Join-Path $script:bootstrapRoot 'app\gui_shell_desktop.exe'),
    (Join-Path $script:productRoot 'gui_shell_desktop_launcher.exe'),
    (Join-Path $initialVersionPath 'app\gui_shell_desktop.exe'),
    (Join-Path $updateVersionPath 'app\gui_shell_desktop.exe')
  )
  foreach ($ownedPath in $ownedProcessPaths) {
    if (@(Get-ProcessByImagePath -ImagePath $ownedPath).Count -gt 0) {
      throw 'An owned Setup or Product process remains after the lifecycle.'
    }
  }
  $result = 'passed'
} catch {
  $failure = $_
  $evidence.failure_stage = $stage
  $evidence.failure_type = $_.Exception.GetType().Name
  $evidence.failure_message = $_.Exception.Message
} finally {
  $cleanupErrors = [System.Collections.Generic.List[string]]::new()
  try {
    if ($setupProcess -and -not $setupProcess.HasExited) {
      $setupProcess.Refresh()
      if (-not $setupProcess.HasExited) {
        [void]$setupProcess.CloseMainWindow()
        if (-not $setupProcess.WaitForExit(15000)) {
          $cleanupErrors.Add('setup_process_did_not_exit')
        }
      }
    }
  } catch { $cleanupErrors.Add('setup_process_cleanup_error') }

  try {
    $ownedProcessPaths = [System.Collections.Generic.List[string]]::new()
    if ($script:bootstrapRoot) {
      $ownedProcessPaths.Add((Join-Path $script:bootstrapRoot 'gui_shell_desktop_launcher.exe'))
      $ownedProcessPaths.Add((Join-Path $script:bootstrapRoot 'app\gui_shell_desktop.exe'))
    }
    if ($script:productRoot) {
      $ownedProcessPaths.Add((Join-Path $script:productRoot 'gui_shell_desktop_launcher.exe'))
    }
    if ($initialVersionPath) {
      $ownedProcessPaths.Add((Join-Path $initialVersionPath 'app\gui_shell_desktop.exe'))
    }
    if ($updateVersionPath) {
      $ownedProcessPaths.Add((Join-Path $updateVersionPath 'app\gui_shell_desktop.exe'))
    }
    if ($setupResolved) { $ownedProcessPaths.Add($setupResolved) }
    foreach ($ownedPath in @($ownedProcessPaths.ToArray() | Select-Object -Unique)) {
      Stop-OwnedProcessAtPath -ImagePath $ownedPath
    }
  } catch { $cleanupErrors.Add('owned_process_cleanup_error') }

  try {
    if ($script:bootstrapRoot -and (Test-Path -LiteralPath $script:bootstrapRoot)) {
      $bootstrapRootFull = [IO.Path]::GetFullPath($script:bootstrapRoot)
      $tempRootFull = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
      if (-not $bootstrapRootFull.StartsWith($tempRootFull, [StringComparison]::OrdinalIgnoreCase) -or
          -not $bootstrapRootFull.EndsWith("D4PocketBootstrap-$AppId", [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Setup temporary root escaped its generated per-identity path.'
      }
      Remove-Item -LiteralPath $bootstrapRootFull -Recurse -Force
    }
    $setupTempCleanup = -not (Test-Path -LiteralPath $script:bootstrapRoot)
  } catch { $cleanupErrors.Add('setup_temp_cleanup_error') }

  try {
    if ($script:startMenuShortcut -and (Test-Path -LiteralPath $script:startMenuShortcut)) {
      $shellCleanup = New-Object -ComObject WScript.Shell
      $shortcutCleanup = $shellCleanup.CreateShortcut($script:startMenuShortcut)
      $shortcutTargetCleanup = [IO.Path]::GetFullPath([string]$shortcutCleanup.TargetPath)
      $expectedTarget = [IO.Path]::GetFullPath((Join-Path $script:productRoot 'gui_shell_desktop_launcher.exe'))
      if ([string]::Equals($shortcutTargetCleanup, $expectedTarget, [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $script:startMenuShortcut -Force
      } else {
        throw 'Refusing to remove a Start Menu shortcut that is not owned by this generated Export identity.'
      }
    }
    if (-not $script:startMenuFolderExistedBeforeTest -and
        $script:startMenuFolder -and (Test-Path -LiteralPath $script:startMenuFolder)) {
      $folderMetadata = Get-Item -LiteralPath $script:startMenuFolder -Force -ErrorAction Stop
      if (-not $folderMetadata.PSIsContainer -or
          ($folderMetadata.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw 'Refusing to remove a Start Menu path that is not a plain generated folder.'
      }
      $leftovers = @(Get-ChildItem -LiteralPath $script:startMenuFolder -Force -ErrorAction Stop)
      if ($leftovers.Count -gt 0) {
        throw 'Generated Start Menu folder contains unexpected data after uninstall.'
      }
      Remove-Item -LiteralPath $script:startMenuFolder -Force
    }
  } catch { $cleanupErrors.Add('start_menu_cleanup_error') }

  try {
    if ($script:productRoot -and (Test-Path -LiteralPath $script:productRoot)) {
      $programRoot = [IO.Path]::GetFullPath((Join-Path $script:localAppData 'Programs\D4 Pocket')).TrimEnd('\') + '\'
      $productRootFull = [IO.Path]::GetFullPath($script:productRoot)
      if (-not $productRootFull.StartsWith($programRoot, [StringComparison]::OrdinalIgnoreCase) -or
          -not $productRootFull.EndsWith("\$AppId", [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to remove an installed path outside the generated Product identity.'
      }
      Remove-Item -LiteralPath $productRootFull -Recurse -Force
    }
    if ($script:runtimeRoot -and (Test-Path -LiteralPath $script:runtimeRoot)) {
      $appsRoot = [IO.Path]::GetFullPath((Join-Path $script:localAppData 'D4Pocket\apps')).TrimEnd('\') + '\'
      $runtimeRootFull = [IO.Path]::GetFullPath($script:runtimeRoot)
      if (-not $runtimeRootFull.StartsWith($appsRoot, [StringComparison]::OrdinalIgnoreCase) -or
          -not $runtimeRootFull.Contains("\$AppId\stores\$AuditStoreId", [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to remove LocalAppData outside the generated app and Audit identities.'
      }
      Remove-Item -LiteralPath $runtimeRootFull -Recurse -Force
    }
    $testIdentityCleanup = -not (Test-Path -LiteralPath $script:productRoot) -and
      -not (Test-Path -LiteralPath $script:runtimeRoot)
  } catch { $cleanupErrors.Add('test_identity_cleanup_error') }

  $evidence.result = $result
  $evidence.product_root_absent_after_uninstall = $productRootAbsentAfterUninstall
  $evidence.start_menu_shortcut_absent_after_uninstall = $startMenuAbsentAfterUninstall
  $evidence.user_data_preserved_after_uninstall = $userDataPreservedAfterUninstall
  $evidence.audit_store_preserved_after_uninstall = $auditPreservedAfterUninstall
  $evidence.audit_event_count_after_uninstall = $auditEventCount
  $evidence.audit_bytes_after_uninstall = $auditBytes
  $evidence.audit_sha256_after_uninstall = $auditSha256
  $evidence.owner_decisions = @($ownerDecisions.ToArray())
  $evidence.test_identity_cleanup = $testIdentityCleanup
  $evidence.setup_temp_and_process_cleanup = $setupTempCleanup -and $cleanupErrors.Count -eq 0
  $evidence.cleanup_errors = @($cleanupErrors.ToArray())
  $evidence.failure_stage = if ($null -eq $failure) { $null } else { $evidence.failure_stage }
  try {
    $evidenceParent = Split-Path -Parent ([IO.Path]::GetFullPath($EvidencePath))
    if (-not (Test-Path -LiteralPath $evidenceParent -PathType Container)) {
      New-Item -ItemType Directory -Path $evidenceParent | Out-Null
    }
    $evidence | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
  } catch {
    if ($null -eq $failure) { $failure = $_ }
  }
}

if ($null -ne $failure) { throw $failure }
if ($result -ne 'passed' -or -not $testIdentityCleanup -or -not $setupTempCleanup) {
  throw 'W-E04 lifecycle completed without required cleanup.'
}
