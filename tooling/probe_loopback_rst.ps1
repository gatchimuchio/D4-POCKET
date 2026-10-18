[CmdletBinding()]
param(
    [ValidateRange(1, 30)]
    [int]$TimeoutSeconds = 5
)

$ErrorActionPreference = 'Stop'
$loopback = [System.Net.IPAddress]::Loopback
$listener = [System.Net.Sockets.TcpListener]::new($loopback, 0)
$client = $null
$server = $null

try {
    $listener.Start()
    $listenerPort = ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port

    $client = [System.Net.Sockets.TcpClient]::new()
    $client.Connect($loopback, $listenerPort)
    $clientPort = ([System.Net.IPEndPoint]$client.Client.LocalEndPoint).Port
    $server = $listener.AcceptTcpClient()
    $server.Client.ReceiveTimeout = $TimeoutSeconds * 1000

    $client.Client.LingerState = [System.Net.Sockets.LingerOption]::new($true, 0)
    $client.Client.Close(0)
    $client.Dispose()
    $client = $null

    $buffer = [byte[]]::new(1)
    try {
        $received = $server.Client.Receive($buffer, 0, $buffer.Length, [System.Net.Sockets.SocketFlags]::None)
        if ($received -eq 0) {
            throw '強制切断後に通常の切断応答を受け取りました。予期した接続リセットではありません。'
        }
        throw "強制切断後に予期しないデータを受け取りました: $received バイト。"
    } catch [System.Net.Sockets.SocketException] {
        if ($_.Exception.SocketErrorCode -ne [System.Net.Sockets.SocketError]::ConnectionReset) {
            throw
        }
    }

    [pscustomobject]@{
        status = 'reset_observed'
        listener_port = $listenerPort
        client_port = $clientPort
    } | ConvertTo-Json -Compress
} finally {
    if ($null -ne $client) { $client.Close() }
    if ($null -ne $server) { $server.Close() }
    $listener.Stop()
}
