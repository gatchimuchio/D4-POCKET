param(
  [Parameter(Mandatory = $true)]
  [string]$FlutterReleaseDir,

  [Parameter(Mandatory = $true)]
  [string]$BrokerHelperExe,

  [string]$InstallRoot = "$env:LOCALAPPDATA\GUI-Shell\installed"
)

$ErrorActionPreference = "Stop"

$release = Resolve-Path $FlutterReleaseDir
$helper = Resolve-Path $BrokerHelperExe
$installRootPath = New-Item -ItemType Directory -Force -Path $InstallRoot
$appDir = New-Item -ItemType Directory -Force -Path (Join-Path $installRootPath.FullName "app")
$brokerDir = New-Item -ItemType Directory -Force -Path (Join-Path $installRootPath.FullName "broker")
$runtimeDir = New-Item -ItemType Directory -Force -Path (Join-Path $installRootPath.FullName "runtime")

Copy-Item -Recurse -Force -Path (Join-Path $release.Path "*") -Destination $appDir.FullName
Copy-Item -Force -Path $helper.Path -Destination (Join-Path $brokerDir.FullName "gui_shell_rust_helper.exe")

$launcher = Join-Path $installRootPath.FullName "GUI-Shell.brokered.ps1"
$launcherText = @'
$ErrorActionPreference = "Stop"

$InstallRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$AppExe = Join-Path $InstallRoot "app\gui_shell_desktop.exe"
$BrokerExe = Join-Path $InstallRoot "broker\gui_shell_rust_helper.exe"
$RuntimeDir = Join-Path $InstallRoot "runtime"
$StoreDir = Join-Path $RuntimeDir "broker_store"
$SessionFile = Join-Path $RuntimeDir "broker_session.json"

New-Item -ItemType Directory -Force -Path $StoreDir | Out-Null
if (Test-Path $SessionFile) {
  Remove-Item -Force -Path $SessionFile
}

$Broker = Start-Process -FilePath $BrokerExe -ArgumentList @(
  "broker-server",
  "--store-dir",
  $StoreDir,
  "--session-file",
  $SessionFile
) -PassThru -WindowStyle Hidden

try {
  for ($Index = 0; $Index -lt 100; $Index += 1) {
    if (Test-Path $SessionFile) {
      break
    }
    $Broker.Refresh()
    if ($Broker.HasExited) {
      throw "Rust broker exited before endpoint was ready: $($Broker.ExitCode)"
    }
    Start-Sleep -Milliseconds 50
  }

  if (-not (Test-Path $SessionFile)) {
    throw "Rust broker endpoint file was not created: $SessionFile"
  }

  $env:GUI_SHELL_BROKER_ENDPOINT_JSON = $SessionFile
  $env:GUI_SHELL_BROKER_RUNTIME_DIR = $RuntimeDir
  $App = Start-Process -FilePath $AppExe -PassThru
  $App.WaitForExit()
}
finally {
  if ($null -ne $Broker) {
    $Broker.Refresh()
    if (-not $Broker.HasExited) {
      Stop-Process -Id $Broker.Id -Force
    }
  }
}
'@

Set-Content -Encoding UTF8 -Path $launcher -Value $launcherText

$cmdLauncher = Join-Path $installRootPath.FullName "GUI-Shell.brokered.cmd"
$cmdText = @"
@echo off
powershell -ExecutionPolicy Bypass -File "%~dp0GUI-Shell.brokered.ps1"
"@
Set-Content -Encoding ASCII -Path $cmdLauncher -Value $cmdText

$manifest = [ordered]@{
  install_root = $installRootPath.FullName
  app_exe = Join-Path $appDir.FullName "gui_shell_desktop.exe"
  broker_exe = Join-Path $brokerDir.FullName "gui_shell_rust_helper.exe"
  runtime_dir = $runtimeDir.FullName
  launcher_ps1 = $launcher
  launcher_cmd = $cmdLauncher
  broker_mediated = $true
  python_runtime_required_for_authority = $false
  flutter_rust_ffi_authority_bridge = $false
}

$manifestPath = Join-Path $installRootPath.FullName "installed_manifest.json"
$manifest | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 -Path $manifestPath
Write-Host "staged GUI-Shell installed app at $($installRootPath.FullName)"
Write-Host "manifest $manifestPath"
