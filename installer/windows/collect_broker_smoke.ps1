param(
  [Parameter(Mandatory = $true)]
  [string]$BrokerHelperExe,

  [string]$OutputPath = "release_evidence/windows_broker_smoke.json",

  [string]$StoreDir = "",

  [string]$SessionFile = "",

  [int]$MaxRequestBytes = 65536
)

$ErrorActionPreference = "Stop"

$helper = Resolve-Path $BrokerHelperExe
$root = Split-Path -Parent $helper.Path
if ($StoreDir -eq "") {
  $StoreDir = Join-Path $root "store"
}
if ($SessionFile -eq "") {
  $SessionFile = Join-Path $root "broker_session.json"
}

New-Item -ItemType Directory -Force -Path $StoreDir | Out-Null
if (Test-Path $SessionFile) {
  Remove-Item -Force -Path $SessionFile
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

function New-IssuedAt {
  return (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
}

function New-BrokerRequest {
  param(
    [string]$RequestId,
    [string]$Operation,
    [string]$Nonce,
    [string]$SessionId = ""
  )

  $request = [ordered]@{
    request_id = $RequestId
    operation = $Operation
    payload_hash = "sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b"
    nonce = $Nonce
    issued_at = New-IssuedAt
    metadata = [ordered]@{
      client = "windows_installed_smoke"
    }
  }
  if ($SessionId -ne "") {
    $request.session_id = $SessionId
  }
  return $request
}

function Start-Broker {
  param([string]$HelperPath)

  return Start-Process -FilePath $HelperPath -ArgumentList @(
    "broker-server",
    "--store-dir",
    $StoreDir,
    "--session-file",
    $SessionFile,
    "--max-request-bytes",
    "$MaxRequestBytes"
  ) -WindowStyle Hidden -PassThru
}

function Wait-BrokerEndpoint {
  param([string]$Path, [System.Diagnostics.Process]$Process)

  for ($index = 0; $index -lt 100; $index += 1) {
    if (Test-Path $Path) {
      return Get-Content -Raw -Path $Path | ConvertFrom-Json
    }
    $Process.Refresh()
    if ($Process.HasExited) {
      throw "Rust broker が endpoint 準備前に終了しました: $($Process.ExitCode)"
    }
    Start-Sleep -Milliseconds 50
  }
  throw "Rust broker endpoint ファイルが作成されませんでした: $Path"
}

function Invoke-BrokerRequest {
  param($Endpoint, $Request)

  $client = [System.Net.Sockets.TcpClient]::new()
  $client.Connect([string]$Endpoint.host, [int]$Endpoint.port)
  try {
    $stream = $client.GetStream()
    $encoding = [System.Text.UTF8Encoding]::new($false)
    $writer = [System.IO.StreamWriter]::new($stream, $encoding)
    $writer.NewLine = "`n"
    $writer.WriteLine([string]$Endpoint.session_secret)
    $writer.WriteLine(($Request | ConvertTo-Json -Compress -Depth 10))
    $writer.Flush()
    $client.Client.Shutdown([System.Net.Sockets.SocketShutdown]::Send)
    $reader = [System.IO.StreamReader]::new($stream, $encoding)
    $raw = $reader.ReadToEnd().Trim()
    return $raw | ConvertFrom-Json
  } finally {
    $client.Close()
  }
}

function Stop-Broker {
  param([System.Diagnostics.Process]$Process, $Endpoint)

  if ($null -eq $Process) {
    return
  }
  $Process.Refresh()
  if ($Process.HasExited) {
    return
  }
  try {
    $shutdown = New-BrokerRequest `
      -RequestId "windows-installed-shutdown" `
      -Operation "shutdown" `
      -Nonce "windows-installed-shutdown-$($Endpoint.session_id)" `
      -SessionId $Endpoint.session_id
    Invoke-BrokerRequest -Endpoint $Endpoint -Request $shutdown | Out-Null
    $Process.WaitForExit(3000) | Out-Null
  } catch {
  }
  $Process.Refresh()
  if (!$Process.HasExited) {
    Stop-Process -Id $Process.Id -Force
  }
}

$errors = New-Object System.Collections.Generic.List[string]
$broker = $null
$endpoint = $null
$restartBroker = $null
$restartEndpoint = $null
$health = $null
$replay = $null
$freshAfterRestart = $null
$crashFailClosed = $false
$replayNonce = "windows-installed-replay-nonce-$([guid]::NewGuid().ToString('N'))"
$freshNonce = "windows-installed-fresh-nonce-$([guid]::NewGuid().ToString('N'))"

try {
  $broker = Start-Broker -HelperPath $helper.Path
  $endpoint = Wait-BrokerEndpoint -Path $SessionFile -Process $broker
  $healthRequest = New-BrokerRequest `
    -RequestId "windows-installed-health-1" `
    -Operation "health" `
    -Nonce $replayNonce
  $health = Invoke-BrokerRequest -Endpoint $endpoint -Request $healthRequest
  if ($health.status -ne "accepted") {
    $errors.Add("最初の broker health が受理されませんでした")
  }
  if ($health.health.persistence_ready -ne $true) {
    $errors.Add("broker の durable store が準備できていませんでした")
  }

  Stop-Broker -Process $broker -Endpoint $endpoint
  $broker = $null

  if (Test-Path $SessionFile) {
    Remove-Item -Force -Path $SessionFile
  }
  $restartBroker = Start-Broker -HelperPath $helper.Path
  $restartEndpoint = Wait-BrokerEndpoint -Path $SessionFile -Process $restartBroker
  $replay = Invoke-BrokerRequest -Endpoint $restartEndpoint -Request $healthRequest
  if ($replay.status -ne "rejected" -or $replay.error.code -ne "broker_replay_detected") {
    $errors.Add("broker restart 後に再使用した nonce が拒否されませんでした")
  }
  $freshRequest = New-BrokerRequest `
    -RequestId "windows-installed-health-2" `
    -Operation "health" `
    -Nonce $freshNonce
  $freshAfterRestart = Invoke-BrokerRequest -Endpoint $restartEndpoint -Request $freshRequest
  if ($freshAfterRestart.status -ne "accepted") {
    $errors.Add("restart 後の新規 broker health が受理されませんでした")
  }

  Stop-Process -Id $restartBroker.Id -Force
  $restartBroker.WaitForExit(3000) | Out-Null
  try {
    Invoke-BrokerRequest -Endpoint $restartEndpoint -Request (New-BrokerRequest `
      -RequestId "windows-installed-after-crash" `
      -Operation "health" `
      -Nonce "windows-installed-after-crash-nonce") | Out-Null
  } catch {
    $crashFailClosed = $true
  }
  if (!$crashFailClosed) {
    $errors.Add("強制停止後も broker が IPC を受理しました")
  }
} catch {
  $errors.Add($_.Exception.Message)
} finally {
  Stop-Broker -Process $broker -Endpoint $endpoint
  Stop-Broker -Process $restartBroker -Endpoint $restartEndpoint
}

$result = [ordered]@{
  status = $(if ($errors.Count -eq 0) { "passed" } else { "failed" })
  collected_at = (Get-Date).ToUniversalTime().ToString("o")
  evidence_source = [ordered]@{
    collector = "installer/windows/collect_broker_smoke.ps1"
    collector_version = "2"
    synthetic = $false
    command = "powershell -ExecutionPolicy Bypass -File installer\windows\collect_broker_smoke.ps1 -BrokerHelperExe `"$($helper.Path)`""
  }
  helper_exe_path = $helper.Path
  helper_exe_exists = $true
  session_file = $SessionFile
  session_file_created = (Test-Path $SessionFile)
  store_dir = $StoreDir
  endpoint_host = $endpoint.host
  endpoint_port = $endpoint.port
  restricted_loopback_bind = ($endpoint.host -eq "127.0.0.1" -and $restartEndpoint.host -eq "127.0.0.1")
  authenticated_ipc_connection = ($health.status -eq "accepted")
  durable_store_ready = ($health.health.persistence_ready -eq $true)
  replay_nonce = $replayNonce
  restart_replay_rejected = ($replay.status -eq "rejected" -and $replay.error.code -eq "broker_replay_detected")
  replay_error_code = $replay.error.code
  fresh_health_after_restart = ($freshAfterRestart.status -eq "accepted")
  crash_fail_closed = $crashFailClosed
  field_provenance = [ordered]@{
    helper_exe_exists = [ordered]@{ source_type = "directly_measured"; evidence_class = "EXTERNAL_EVIDENCE" }
    session_file_created = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME" }
    restricted_loopback_bind = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME" }
    authenticated_ipc_connection = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME" }
    durable_store_ready = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME" }
    restart_replay_rejected = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME" }
    fresh_health_after_restart = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME" }
    crash_fail_closed = [ordered]@{ source_type = "directly_measured"; evidence_class = "LIVE_RUNTIME" }
  }
  unmeasured_declarations = [ordered]@{
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
  errors = @($errors)
}

$output = New-Item -ItemType File -Force -Path $OutputPath
Write-JsonEvidence -Value $result -Path $output.FullName -Depth 10
Write-Host "書き出しました: $($output.FullName)"
if ($errors.Count -ne 0) {
  exit 1
}
