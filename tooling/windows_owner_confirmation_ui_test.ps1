param(
  [Parameter(Mandatory = $true)]
  [int]$OwnerProcessId,

  [Parameter(Mandatory = $true)]
  [ValidateSet('Yes', 'No')]
  [string]$Decision,

  [Parameter(Mandatory = $true)]
  [ValidatePattern('^(?:[0-9a-fA-F]{2})+$')]
  [string]$ExpectedTextHex
)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$window = $null
$decisionApplied = $false

function Find-MessageBoxButton {
  param(
    [Parameter(Mandatory = $true)]
    [System.Windows.Automation.AutomationElement]$Dialog,

    [Parameter(Mandatory = $true)]
    [ValidateSet('Yes', 'No')]
    [string]$RequestedDecision
  )

  $buttonCondition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
    [System.Windows.Automation.ControlType]::Button
  )
  $buttons = $Dialog.FindAll(
    [System.Windows.Automation.TreeScope]::Descendants,
    $buttonCondition
  )
  $targetId = if ($RequestedDecision -eq 'Yes') { '6' } else { '7' }
  foreach ($button in $buttons) {
    if ($button.Current.AutomationId -eq $targetId) {
      return $button
    }
  }

  $localizedNames = if ($RequestedDecision -eq 'Yes') {
    @('Yes', 'はい')
  } else {
    @('No', 'いいえ')
  }
  foreach ($button in $buttons) {
    foreach ($localizedName in $localizedNames) {
      if ($button.Current.Name -eq $localizedName) {
        return $button
      }
    }
  }

  return $null
}

function Close-MessageBoxAsDeclined {
  param(
    [Parameter(Mandatory = $true)]
    [System.Windows.Automation.AutomationElement]$Dialog
  )

  $noButton = Find-MessageBoxButton -Dialog $Dialog -RequestedDecision 'No'
  if ($null -ne $noButton) {
    $noButton.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    return
  }

  $Dialog.GetCurrentPattern([System.Windows.Automation.WindowPattern]::Pattern).Close()
}

function Stop-TestHarnessAfterUiTimeout {
  $testProcess = Get-Process -Id $OwnerProcessId -ErrorAction Stop
  if ($testProcess.ProcessName -notmatch '^gui_shell_rust_helper-[0-9a-f]+$') {
    throw 'UI Automation timeout後に停止できる対象がRust test harnessと確認できない'
  }
  Stop-Process -Id $OwnerProcessId -Force -ErrorAction Stop
}

try {
  Add-Type -AssemblyName UIAutomationClient
  Add-Type -AssemblyName UIAutomationTypes

  $hexBytes = [byte[]]::new($ExpectedTextHex.Length / 2)
  for ($index = 0; $index -lt $hexBytes.Length; $index++) {
    $hexBytes[$index] = [Convert]::ToByte($ExpectedTextHex.Substring($index * 2, 2), 16)
  }
  $expectedText = [System.Text.Encoding]::UTF8.GetString($hexBytes)
  $windowTitle = 'D4 Pocket Owner確認'
  $nameCondition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::NameProperty,
    $windowTitle
  )
  $root = [System.Windows.Automation.AutomationElement]::RootElement

  [Console]::Out.WriteLine('READY')
  [Console]::Out.Flush()

  $deadline = [DateTime]::UtcNow.AddSeconds(12)
  while ($null -eq $window -and [DateTime]::UtcNow -lt $deadline) {
    $candidates = $root.FindAll(
      [System.Windows.Automation.TreeScope]::Children,
      $nameCondition
    )
    foreach ($candidate in $candidates) {
      if ($candidate.Current.ProcessId -eq $OwnerProcessId -and
          $candidate.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window) {
        $window = $candidate
        break
      }
    }
    if ($null -eq $window) {
      Start-Sleep -Milliseconds 25
    }
  }
  if ($null -eq $window) {
    Stop-TestHarnessAfterUiTimeout
    throw '指定processのOwner確認dialogがUI Automationから見つからない'
  }

  $textCondition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
    [System.Windows.Automation.ControlType]::Text
  )
  $textElements = $window.FindAll(
    [System.Windows.Automation.TreeScope]::Descendants,
    $textCondition
  )
  $displayedText = $null
  foreach ($element in $textElements) {
    $candidateText = $element.Current.Name
    if ($candidateText.Contains("payload hash:")) {
      $displayedText = $candidateText
      break
    }
  }

  $normalize = { param([string]$Text) $Text.Replace("`r`n", "`n").Replace("`r", "`n") }
  if ($null -eq $displayedText -or
      (& $normalize $displayedText) -cne (& $normalize $expectedText)) {
    Close-MessageBoxAsDeclined -Dialog $window
    $decisionApplied = $true
    throw '実際に表示されたOwner確認文がRust production formatterと一致しない'
  }

  $button = Find-MessageBoxButton -Dialog $window -RequestedDecision $Decision
  if ($null -eq $button) {
    Close-MessageBoxAsDeclined -Dialog $window
    $decisionApplied = $true
    throw 'Owner確認dialogの指定ボタンがUI Automationから見つからない'
  }
  $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
  $decisionApplied = $true

  Write-Output (ConvertTo-Json -InputObject $displayedText -Compress)
  exit 0
}
catch {
  if ($null -ne $window -and -not $decisionApplied) {
    try {
      Close-MessageBoxAsDeclined -Dialog $window
    }
    catch {
      # 失敗時もOwner確認を肯定せず、そのまま診断失敗として返す。
    }
  }
  [Console]::Error.WriteLine($_.Exception.Message)
  exit 1
}
