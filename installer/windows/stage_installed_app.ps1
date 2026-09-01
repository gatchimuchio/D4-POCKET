param(
  [Parameter(Mandatory = $true)]
  [string]$FlutterReleaseDir,

  [Parameter(Mandatory = $true)]
  [string]$BrokerHelperExe,

  [string]$InstallRoot = "",

  [string]$RunId = "",

  [string]$GitRoot = "",

  [string]$BuildCommand = "flutter build windows --release; cargo build --release",

  [string]$BuildTimestamp = "",

  [switch]$AllowExistingInstallRoot
)

$ErrorActionPreference = "Stop"

function Get-TaggedSha256 {
  param([Parameter(Mandatory = $true)][string]$Path)
  return "sha256:$((Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant())"
}

function Invoke-GitString {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Root,
    [Parameter(Mandatory = $true)]
    [string[]]$Arguments
  )

  try {
    $output = & git -C $Root @Arguments 2>$null
    if ($LASTEXITCODE -ne 0) {
      return $null
    }
    return (($output -join "`n").Trim())
  } catch {
    return $null
  }
}

function Test-LegacyFixedInstallRoot {
  param([string]$Path)
  $normalized = $Path.Replace("/", "\").TrimEnd("\").ToLowerInvariant()
  return $normalized.EndsWith("\gui-shell\installed")
}

function Write-JsonEvidence {
  param(
    [Parameter(Mandatory = $true)]
    $Value,
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [int]$Depth = 8
  )

  $json = $Value | ConvertTo-Json -Depth $Depth
  $encoding = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText($Path, ($json + [Environment]::NewLine), $encoding)
}

$release = Resolve-Path $FlutterReleaseDir
$helper = Resolve-Path $BrokerHelperExe

if ($RunId -eq "") {
  $RunId = "run-$((Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ'))-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
}
if ($InstallRoot -eq "") {
  $InstallRoot = Join-Path $env:LOCALAPPDATA "GUI-Shell\installed-runs\$RunId"
}
if ($BuildTimestamp -eq "") {
  $BuildTimestamp = (Get-Date).ToUniversalTime().ToString("o")
}
if ($GitRoot -eq "") {
  $GitRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
}

if ((Test-Path $InstallRoot) -and !$AllowExistingInstallRoot.IsPresent) {
  throw "InstallRoot はすでに存在します。正式証拠には新規の分離実行 root が必要です: $InstallRoot"
}
if ((Test-LegacyFixedInstallRoot -Path $InstallRoot) -and !$AllowExistingInstallRoot.IsPresent) {
  throw "従来の共有固定 InstallRoot は正式な Windows 証拠として無効です: $InstallRoot"
}

$sourceCommit = Invoke-GitString -Root $GitRoot -Arguments @("rev-parse", "HEAD")
$sourceStatus = Invoke-GitString -Root $GitRoot -Arguments @("status", "--porcelain")
$sourceWorktreeClean = ($null -ne $sourceCommit -and $null -ne $sourceStatus -and $sourceStatus -eq "")

$installRootPath = New-Item -ItemType Directory -Force -Path $InstallRoot
$appDir = New-Item -ItemType Directory -Force -Path (Join-Path $installRootPath.FullName "app")
$brokerDir = New-Item -ItemType Directory -Force -Path (Join-Path $installRootPath.FullName "broker")
$runtimeDir = New-Item -ItemType Directory -Force -Path (Join-Path $installRootPath.FullName "runtime")
$storeDir = New-Item -ItemType Directory -Force -Path (Join-Path $runtimeDir.FullName "broker_store")
$configDir = New-Item -ItemType Directory -Force -Path (Join-Path $runtimeDir.FullName "config")
$auditDir = New-Item -ItemType Directory -Force -Path (Join-Path $runtimeDir.FullName "audit")
$evidenceDir = New-Item -ItemType Directory -Force -Path (Join-Path $runtimeDir.FullName "evidence")

Copy-Item -Recurse -Force -Path (Join-Path $release.Path "*") -Destination $appDir.FullName
Copy-Item -Force -Path $helper.Path -Destination (Join-Path $brokerDir.FullName "gui_shell_rust_helper.exe")

$appExe = Join-Path $appDir.FullName "gui_shell_desktop.exe"
$brokerExe = Join-Path $brokerDir.FullName "gui_shell_rust_helper.exe"
$configPath = Join-Path $configDir.FullName "gui_shell.json"
$sessionFile = Join-Path $runtimeDir.FullName "broker_session.json"
$appArtifactSha256 = Get-TaggedSha256 -Path $appExe
$brokerArtifactSha256 = Get-TaggedSha256 -Path $brokerExe

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
      throw "Rust broker が endpoint 準備前に終了しました: $($Broker.ExitCode)"
    }
    Start-Sleep -Milliseconds 50
  }

  if (-not (Test-Path $SessionFile)) {
    throw "Rust broker endpoint ファイルが作成されませんでした: $SessionFile"
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
  manifest_version = 2
  run_id = $RunId
  staged_at = (Get-Date).ToUniversalTime().ToString("o")
  source_commit = $sourceCommit
  source_worktree_clean = $sourceWorktreeClean
  source_status_porcelain = $(if ($null -ne $sourceStatus) { $sourceStatus } else { "" })
  build_command = $BuildCommand
  build_timestamp = $BuildTimestamp
  install_root = $installRootPath.FullName
  app_exe = $appExe
  broker_exe = $brokerExe
  runtime_dir = $runtimeDir.FullName
  store_dir = $storeDir.FullName
  config_dir = $configDir.FullName
  config_path = $configPath
  audit_dir = $auditDir.FullName
  evidence_dir = $evidenceDir.FullName
  broker_session_file = $sessionFile
  launcher_ps1 = $launcher
  launcher_cmd = $cmdLauncher
  broker_mediated = $true
  app_artifact_sha256 = $appArtifactSha256
  broker_artifact_sha256 = $brokerArtifactSha256
  isolation = [ordered]@{
    uses_shared_fixed_install_root = $false
    isolated_install_root = $installRootPath.FullName
    isolated_runtime_dir = $runtimeDir.FullName
    isolated_store_dir = $storeDir.FullName
    isolated_config_dir = $configDir.FullName
    isolated_audit_dir = $auditDir.FullName
  }
  declarations = [ordered]@{
    python_runtime_required_for_authority = [ordered]@{
      value = $false
      source_type = "static_assertion"
      evidence_class = "CONFIG"
      formal_runtime_proof = $false
    }
    flutter_rust_ffi_authority_bridge = [ordered]@{
      value = $false
      source_type = "static_assertion"
      evidence_class = "CONFIG"
      formal_runtime_proof = $false
    }
  }
}

$manifestPath = Join-Path $installRootPath.FullName "installed_manifest.json"
Write-JsonEvidence -Value $manifest -Path $manifestPath -Depth 8
Write-Host "GUI-Shell のインストール済み app を stage しました: $($installRootPath.FullName)"
Write-Host "manifest $manifestPath"
