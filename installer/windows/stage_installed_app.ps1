param(
  [Parameter(Mandatory = $true)]
  [string]$FlutterReleaseDir,

  [Parameter(Mandatory = $true)]
  [string]$BrokerHelperExe,

  [Parameter(Mandatory = $true)]
  [string]$DesktopLauncherExe,

  [string]$InstallRoot = "",

  [string]$RunId = "",

  [string]$GitRoot = "",

  [string]$BuildCommand = "flutter build windows --release; cargo build --release",

  [string]$BuildTimestamp = "",

  [string]$ProductManifestJson = "",

  [switch]$AllowExistingInstallRoot
)

$ErrorActionPreference = "Stop"

function Get-TaggedSha256 {
  param([Parameter(Mandatory = $true)][string]$Path)
  return "sha256:$((Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant())"
}

function Get-TaggedStringSha256 {
  param([Parameter(Mandatory = $true)][string]$Text)
  $encoding = [System.Text.UTF8Encoding]::new($false)
  $bytes = $encoding.GetBytes($Text)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    return "sha256:$(([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant())"
  } finally {
    $sha.Dispose()
  }
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
$desktopLauncher = Resolve-Path $DesktopLauncherExe

$launcherBytes = [System.IO.File]::ReadAllBytes($desktopLauncher.Path)
$launcherText = [System.Text.Encoding]::ASCII.GetString($launcherBytes)
$embeddedAppIds = @([regex]::Matches($launcherText, 'd4-pocket-app-[0-9a-f]{32}') | ForEach-Object { $_.Value } | Sort-Object -Unique)
$embeddedAuditStoreIds = @([regex]::Matches($launcherText, 'audit-store-[0-9a-f]{32}') | ForEach-Object { $_.Value } | Sort-Object -Unique)
$productAppId = $null
$productAuditStoreId = $null
$productManifestPath = $null
$productManifestSha256 = $null
$launcherRuntimeIdentityKind = "gui_shell"

if ($ProductManifestJson -ne "") {
  $productManifestPath = (Resolve-Path -LiteralPath $ProductManifestJson).Path
  $productManifestItem = Get-Item -LiteralPath $productManifestPath -Force
  if ($productManifestItem.PSIsContainer -or
      ($productManifestItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 -or
      $productManifestItem.Length -lt 1 -or $productManifestItem.Length -gt 65536) {
    throw "D4 Pocket Product Manifestは通常fileかつ64 KiB以下でなければなりません。"
  }
  try {
    $productManifestText = [System.Text.UTF8Encoding]::new($false, $true).GetString([System.IO.File]::ReadAllBytes($productManifestPath))
    $productManifest = ConvertFrom-Json -InputObject $productManifestText -ErrorAction Stop
  } catch {
    throw "D4 Pocket Product ManifestをUTF-8 JSONとして読み取れません。"
  }
  $productAppId = [string]$productManifest.manifest.app_identity.app_id
  $productAuditStoreId = [string]$productManifest.manifest.audit_store.store_id
  if ($productManifest.version -ne 1 -or
      $productManifest.product -ne "D4 Pocket" -or
      [string]$productManifest.export_id -notmatch '^[A-Za-z0-9._-]{1,64}$' -or
      $productAppId -notmatch '^d4-pocket-app-[0-9a-f]{32}$' -or
      $productAuditStoreId -notmatch '^audit-store-[0-9a-f]{32}$') {
    throw "D4 Pocket Product Manifestの製品identityが不正です。"
  }
  if ($embeddedAppIds.Count -ne 1 -or $embeddedAppIds[0] -cne $productAppId -or
      $embeddedAuditStoreIds.Count -ne 1 -or $embeddedAuditStoreIds[0] -cne $productAuditStoreId) {
    throw "Rust Desktop起動器のcompile-time identityがProduct Manifestと一致しません。"
  }
  $launcherRuntimeIdentityKind = "d4_pocket_product"
  $productManifestSha256 = Get-TaggedSha256 -Path $productManifestPath
} elseif ($embeddedAppIds.Count -ne 0 -or $embeddedAuditStoreIds.Count -ne 0) {
  throw "製品identityをcompile-time埋込した起動器には、対応するProduct Manifestが必要です。"
}

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
if ([string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
  throw "LOCALAPPDATA を確認できないため、Desktop起動器の保存先宣言を作成できません。"
}
if ($launcherRuntimeIdentityKind -eq "d4_pocket_product") {
  $launcherRuntimeDir = Join-Path $env:LOCALAPPDATA (Join-Path "D4Pocket\apps" (Join-Path $productAppId (Join-Path "stores" $productAuditStoreId)))
} else {
  $launcherRuntimeDir = Join-Path $env:LOCALAPPDATA "GUI-Shell\broker\desktop"
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
$stagingUserIdentitySalt = [guid]::NewGuid().ToString("N")
$stagingUserSid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$stagingUserIdentityHash = Get-TaggedStringSha256 -Text "$stagingUserIdentitySalt|$stagingUserSid"

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
Copy-Item -Force -Path $desktopLauncher.Path -Destination (Join-Path $installRootPath.FullName "gui_shell_desktop_launcher.exe")
if ($null -ne $productManifestPath) {
  Copy-Item -LiteralPath $productManifestPath -Destination (Join-Path $installRootPath.FullName "product_manifest.json")
}

$appExe = Join-Path $appDir.FullName "gui_shell_desktop.exe"
$brokerExe = Join-Path $brokerDir.FullName "gui_shell_rust_helper.exe"
$launcherExe = Join-Path $installRootPath.FullName "gui_shell_desktop_launcher.exe"
$configPath = Join-Path $configDir.FullName "gui_shell.json"
# Broker単体smoke専用scratch endpoint。実製品runtime endpointはlauncher_runtime.session_fileで別に宣言する。
$sessionFile = Join-Path $runtimeDir.FullName "broker_session.json"
$appArtifactSha256 = Get-TaggedSha256 -Path $appExe
$brokerArtifactSha256 = Get-TaggedSha256 -Path $brokerExe
$launcherArtifactSha256 = Get-TaggedSha256 -Path $launcherExe

$manifest = [ordered]@{
  manifest_version = 3
  run_id = $RunId
  staged_at = (Get-Date).ToUniversalTime().ToString("o")
  source_commit = $sourceCommit
  source_worktree_clean = $sourceWorktreeClean
  source_status_porcelain = $(if ($null -ne $sourceStatus) { $sourceStatus } else { "" })
  staging_user_identity = [ordered]@{
    hash_algorithm = "SHA-256"
    per_run_salt = $stagingUserIdentitySalt
    salted_hash = $stagingUserIdentityHash
  }
  build_command = $BuildCommand
  build_timestamp = $BuildTimestamp
  install_root = $installRootPath.FullName
  app_exe = $appExe
  broker_exe = $brokerExe
  launcher_exe = $launcherExe
  launcher_runtime = [ordered]@{
    root = $launcherRuntimeDir
    store_dir = (Join-Path $launcherRuntimeDir "store")
    session_file = (Join-Path $launcherRuntimeDir "broker_session.json")
    identity_kind = $launcherRuntimeIdentityKind
    app_id = $productAppId
    audit_store_id = $productAuditStoreId
    scope = "per_user"
    isolated = $false
    evidence_class = "CONFIG"
    formal_runtime_proof = $false
  }
  product_manifest = $(if ($null -ne $productManifestPath) {
      [ordered]@{
        path = (Join-Path $installRootPath.FullName "product_manifest.json")
        sha256 = $productManifestSha256
        runtime_manifest_consumed_by_launcher = $false
      }
    } else { $null })
  runtime_dir = $runtimeDir.FullName
  store_dir = $storeDir.FullName
  config_dir = $configDir.FullName
  config_path = $configPath
  audit_dir = $auditDir.FullName
  evidence_dir = $evidenceDir.FullName
  broker_session_file = $sessionFile
  broker_mediated = $true
  app_artifact_sha256 = $appArtifactSha256
  broker_artifact_sha256 = $brokerArtifactSha256
  launcher_artifact_sha256 = $launcherArtifactSha256
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
Write-Host "通常起動器: $launcherExe"
Write-Host "manifest $manifestPath"
