$ErrorActionPreference = 'Stop'
$token = [guid]::NewGuid().ToString('N')
$tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$inputPath = Join-Path $tempRoot "d4-pktmon-summary-$token.txt"
$summaryPath = Join-Path $tempRoot "d4-pktmon-summary-$token.md"
$formatter = Join-Path $PSScriptRoot 'pktmon_packet_summary.ps1'

try {
    @(
        '[00]0000.0000::<DateTime> Packet Filter 1: IP-1=127.0.0.1, IP-2=0.0.0.0, Protocol=TCP, Port-1=0, Port-2=0, TCPFlags=RST'
        'Property: Component 39, IP Address = 127.0.0.1'
        'Property: Component 29, IP Address = 127.0.0.1'
        '[30]0000.0001::<DateTime> [Microsoft-Windows-PktMon] PktGroupId 8444249301423149, PktNumber 1, Appearance 1, Direction Tx , Type IP , Component 39, Filter 1, OriginalSize 54, LoggedSize 54'
        '    ip: 127.0.0.1.49152 > 127.0.0.1.443: TCP, flags [R], length 0'
        '[30]0000.0002::<DateTime> [Microsoft-Windows-PktMon] PktGroupId 8444249301423149, PktNumber 1, Appearance 2, Direction Tx , Type IP , Component 29, Filter 1, OriginalSize 54, LoggedSize 54'
        '    ip: 127.0.0.1.49152 > 127.0.0.1.443: TCP, flags [R], length 0'
        '[30]0000.0003::<DateTime> [Microsoft-Windows-PktMon] PktGroupId 8444249301423149, PktNumber 2, Appearance 1, Direction Rx , Type IP , Component 39, Filter 1, OriginalSize 54, LoggedSize 54'
        '    ip: 127.0.0.1.443 > 127.0.0.1.49152: TCP, flags [R], length 0'
        '[30]0000.0004::<DateTime> [Microsoft-Windows-PktMon] PktGroupId 12, PktNumber 1, Appearance 1, Direction Rx , Type IP , Component 8, Filter 1, OriginalSize 54, LoggedSize 54'
        '    ip: 192.0.2.10.443 > 198.51.100.5.49152: TCP, flags [R], length 0'
        '[30]0000.0005::<DateTime> [Microsoft-Windows-PktMon] PktGroupId 13, PktNumber 1, Appearance 1, Direction Rx , Type IP , Component 8, Filter 1, OriginalSize 42, LoggedSize 42'
        '    ip: 127.0.0.1.443 > 127.0.0.1.49152: UDP, length 0'
    ) | Set-Content -LiteralPath $inputPath -Encoding utf8
    Set-Content -LiteralPath $summaryPath -Value '' -Encoding utf8

    & $formatter -InputPath $inputPath -SummaryPath $summaryPath -ExpectedLoopbackPort 443
    $summary = Get-Content -LiteralPath $summaryPath -Raw
    if ($summary -notmatch 'snapshots \(capture filter: RST\): 3') {
        throw 'PktGroupId付きloopback TCP snapshotを正しく数えない'
    }
    if ($summary -match 'Packet Filter 1|Property: Component') {
        throw 'filter／component metadataをpacket snapshotとして出力した'
    }
    if ($summary -notmatch 'PktGroupId 8444249301423149, PktNumber 1, Appearance 1, Direction Tx') {
        throw 'packet metadata／送信方向を保持しない'
    }
    if ($summary -notmatch '127\.0\.0\.1\.49152 > 127\.0\.0\.1\.443: TCP') {
        throw 'decoded TCP tupleを保持しない'
    }
    if ($summary -notmatch 'TCP, flags \[R\]') {
        throw 'RST-filtered packet headerを保持しない'
    }
    if ($summary -notmatch '自己接続RST試験の待受け先番号 443 を持つRST headerを[1-9][0-9]*件のtuple snapshotで確認しました。') {
        throw '期待したsynthetic loopback RST portを確認しない'
    }
    if ($summary -notmatch 'PktNumber 2, Appearance 1, Direction Rx') {
        throw '受信方向snapshotを保持しない'
    }
    if ($summary -match '192\.0\.2\.10|198\.51\.100\.5|UDP') {
        throw 'loopback外または非TCP recordを出力した'
    }

    @(
        '[00]0000.0000::<DateTime> Packet Filter 1: IP-1=127.0.0.1, IP-2=0.0.0.0, Protocol=TCP, Port-1=0, Port-2=0, TCPFlags=RST'
        'Property: Component 39, IP Address = 127.0.0.1'
        'Property: Component 29, IP Address = 127.0.0.1'
    ) | Set-Content -LiteralPath $inputPath -Encoding utf8
    Set-Content -LiteralPath $summaryPath -Value '' -Encoding utf8
    & $formatter -InputPath $inputPath -SummaryPath $summaryPath
    $metadataOnlySummary = Get-Content -LiteralPath $summaryPath -Raw
    if ($metadataOnlySummary -notmatch 'snapshots \(capture filter: RST\): 0') {
        throw 'run #52型filter／component metadataだけの入力を0件にできない'
    }
    if ($metadataOnlySummary -notmatch 'metadata alone is not packet evidence') {
        throw 'packet tupleがない場合の証拠限界を明示しない'
    }
    $missingProbeRejected = $false
    try {
        & $formatter -InputPath $inputPath -SummaryPath $summaryPath -ExpectedLoopbackPort 443
    } catch {
        $missingProbeRejected = $_.Exception.Message -match '自己接続RST試験の待受け先番号を持つRST headerがありません: 443'
    }
    if (!$missingProbeRejected) {
        throw '期待したsynthetic probe packetがない場合に拒否しない'
    }

    @(
        '[30]0000.0000::<DateTime> [Microsoft-Windows-PktMon] PktGroupId 14, PktNumber 1, Appearance 1, Direction Tx , Type IP , Component 39, Filter 1, OriginalSize 54, LoggedSize 54'
        '    ip: 127.0.0.1.49152 > 127.0.0.1.443: TCP, flags [S], length 0'
    ) | Set-Content -LiteralPath $inputPath -Encoding utf8
    Set-Content -LiteralPath $summaryPath -Value '' -Encoding utf8
    $nonRstProbeRejected = $false
    try {
        & $formatter -InputPath $inputPath -SummaryPath $summaryPath -ExpectedLoopbackPort 443
    } catch {
        $nonRstProbeRejected = $_.Exception.Message -match '自己接続RST試験の待受け先番号を持つRST headerがありません: 443'
    }
    if (!$nonRstProbeRejected) {
        throw 'RSTではないpacketをprobe成功として扱う'
    }

    $manySnapshots = [System.Collections.Generic.List[string]]::new()
    for ($i = 1; $i -le 21; $i++) {
        $manySnapshots.Add("[30]0000.$('{0:D4}' -f $i)::<DateTime> [Microsoft-Windows-PktMon] PktGroupId 44, PktNumber $i, Appearance 1, Direction Tx , Type IP , Component 39, Filter 1, OriginalSize 54, LoggedSize 54")
        $manySnapshots.Add('    ip: 127.0.0.1.49152 > 127.0.0.1.443: TCP, flags [R], length 0')
    }
    $manySnapshots | Set-Content -LiteralPath $inputPath -Encoding utf8
    Set-Content -LiteralPath $summaryPath -Value '' -Encoding utf8
    & $formatter -InputPath $inputPath -SummaryPath $summaryPath
    $boundedSummary = Get-Content -LiteralPath $summaryPath -Raw
    $emittedSnapshots = @($boundedSummary -split '\r?\n' | Where-Object { $_ -match '\bPktGroupId\s+\d+,\s*PktNumber\s+\d+,\s*Appearance\s+\d+' }).Count
    if ($boundedSummary -notmatch 'snapshots \(capture filter: RST\): 21' -or $emittedSnapshots -ne 20) {
        $summaryHead = $boundedSummary -split '\r?\n' | Select-Object -First 1
        throw "summary表示上限20件を維持しない (heading=$summaryHead; rows=$emittedSnapshots)"
    }
    if ($boundedSummary -notmatch 'truncated to the first 20 of 21 snapshots') {
        throw 'summary切り詰めを明示しない'
    }

    'Pktmon packet summary parser fixtures: PASS'
}
finally {
    foreach ($path in @($inputPath, $summaryPath)) {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            Remove-Item -LiteralPath $path -Force
        }
    }
}
