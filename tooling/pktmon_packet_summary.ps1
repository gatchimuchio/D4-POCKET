[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$InputPath,
    [Parameter(Mandatory = $true)]
    [string]$SummaryPath,
    [ValidateRange(0, 65535)]
    [int]$ExpectedLoopbackPort = 0
)

$ErrorActionPreference = 'Stop'
if (!(Test-Path -LiteralPath $InputPath -PathType Leaf)) {
    throw "Pktmon formatted textが存在しない: $InputPath"
}
if (!(Test-Path -LiteralPath $SummaryPath -PathType Leaf)) {
    throw "GitHub step summaryが存在しない: $SummaryPath"
}

$recordPattern = [regex]::new(
    '\bPktGroupId\s+(?<group>\d+)\s*,\s*PktNumber\s+(?<number>\d+)\s*,\s*Appearance\s+(?<appearance>\d+)\s*,\s*Direction\s+(?<direction>Rx|Tx)\b',
    [System.Text.RegularExpressions.RegexOptions]::IgnoreCase -bor [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
)
$tuplePattern = [regex]::new(
    '(?<source>(?:\d{1,3}\.){3}\d{1,3})[.:]\d{1,5}\s*>\s*(?<destination>(?:\d{1,3}\.){3}\d{1,3})[.:]\d{1,5}\s*:\s*TCP\b',
    [System.Text.RegularExpressions.RegexOptions]::IgnoreCase -bor [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
)
$lines = @(Get-Content -LiteralPath $InputPath -ErrorAction Stop)
$snapshots = [System.Collections.Generic.List[object]]::new()
$unparsedHeaders = [System.Collections.Generic.List[object]]::new()
$headerShapes = [System.Collections.Generic.List[object]]::new()
$rawRecordMarkerCount = 0
$rawPacketNumberMarkerCount = 0
$rawAppearanceMarkerCount = 0
$rawDirectionMarkerCount = 0
$rawIpv4LiteralCount = 0
$rawTcpMarkerLineCount = 0
$rawRstMarkerLineCount = 0
$recognizedRecordCount = 0
$packetHeaderLineCount = 0
$decodedIpv4TcpTupleCount = 0

for ($i = 0; $i -lt $lines.Count; $i++) {
    $currentLine = $lines[$i]
    if ($currentLine -match '\bPktGroupId\b') { $rawRecordMarkerCount++ }
    if ($currentLine -match '\bPktNumber\b') { $rawPacketNumberMarkerCount++ }
    if ($currentLine -match '\bAppearance\b') { $rawAppearanceMarkerCount++ }
    if ($currentLine -match '\bDirection\s+(?:Rx|Tx)\b') { $rawDirectionMarkerCount++ }
    $rawIpv4LiteralCount += [regex]::Matches(
        $currentLine,
        '(?<!\d)(?:\d{1,3}\.){3}\d{1,3}(?!\d)'
    ).Count
    if ([regex]::IsMatch($currentLine, '\bTCP\b', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)) {
        $rawTcpMarkerLineCount++
    }
    if ([regex]::IsMatch(
        $currentLine,
        '\bRST\b|\bflags\s*\[[^\]]*R[^\]]*\]',
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
    )) {
        $rawRstMarkerLineCount++
    }

    $recordMatch = $recordPattern.Match($lines[$i])
    if (!$recordMatch.Success) { continue }
    $recognizedRecordCount++
    $recordNumber = $recognizedRecordCount
    $direction = $recordMatch.Groups['direction'].Value.ToUpperInvariant()

    $packetLine = $null
    for ($j = $i + 1; $j -lt $lines.Count; $j++) {
        if ([string]::IsNullOrWhiteSpace($lines[$j])) { continue }
        if ($recordPattern.IsMatch($lines[$j])) { break }
        $packetLine = $lines[$j]
        break
    }
    if ($null -eq $packetLine) {
        $shape = [pscustomobject]@{
            Record = $recordNumber
            Direction = $direction
            Reason = 'header_missing'
            Ipv4Literals = 0
            Tcp = $false
            RstFlag = $false
            Separator = 'none'
            LineLength = 0
        }
        $unparsedHeaders.Add($shape)
        $headerShapes.Add($shape)
        continue
    }
    $packetHeaderLineCount++

    $tupleMatch = $tuplePattern.Match($packetLine)
    if (!$tupleMatch.Success) {
        $ipv4Literals = [regex]::Matches($packetLine, '(?<!\d)(?:\d{1,3}\.){3}\d{1,3}(?!\d)').Count
        $hasTcp = [regex]::IsMatch($packetLine, '\bTCP\b', [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
        $hasRstFlag = [regex]::IsMatch(
            $packetLine,
            '\bflags\s*\[[^\]]*R[^\]]*\]|\bTCPFlags\s*=\s*RST\b',
            [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
        )
        $separator = 'none'
        if ($packetLine -match '\s>\s') {
            $separator = 'greater_than'
        } elseif ($packetLine -match '->') {
            $separator = 'arrow'
        } elseif ($packetLine -match '\bto\b') {
            $separator = 'to_word'
        }
        $shape = [pscustomobject]@{
            Record = $recordNumber
            Direction = $direction
            Reason = 'tuple_unrecognized'
            Ipv4Literals = $ipv4Literals
            Tcp = $hasTcp
            RstFlag = $hasRstFlag
            Separator = $separator
            LineLength = $packetLine.Trim().Length
        }
        $unparsedHeaders.Add($shape)
        $headerShapes.Add($shape)
        continue
    }
    $decodedIpv4TcpTupleCount++
    $sourceIsLoopback = $tupleMatch.Groups['source'].Value -eq '127.0.0.1'
    $destinationIsLoopback = $tupleMatch.Groups['destination'].Value -eq '127.0.0.1'
    if (!$sourceIsLoopback -and !$destinationIsLoopback) { continue }

    $shapeSeparator = 'none'
    if ($packetLine -match '\s>\s') {
        $shapeSeparator = 'greater_than'
    } elseif ($packetLine -match '->') {
        $shapeSeparator = 'arrow'
    } elseif ($packetLine -match '\bto\b') {
        $shapeSeparator = 'to_word'
    }
    $headerShapes.Add([pscustomobject]@{
        Record = $recordNumber
        Direction = $direction
        Reason = 'tuple_recognized'
        Ipv4Literals = [regex]::Matches($packetLine, '(?<!\d)(?:\d{1,3}\.){3}\d{1,3}(?!\d)').Count
        Tcp = $true
        RstFlag = [regex]::IsMatch(
            $packetLine,
            '\bflags\s*\[[^\]]*R[^\]]*\]|\bTCPFlags\s*=\s*RST\b',
            [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
        )
        Separator = $shapeSeparator
        LineLength = $packetLine.Trim().Length
    })
    $snapshots.Add([pscustomobject]@{
        RecordIndex = $recordNumber
        Direction = $direction
        Record = $lines[$i].Trim()
        Packet = $packetLine.Trim()
    })
}

$probeSnapshots = @()
if ($ExpectedLoopbackPort -gt 0) {
    $escapedPort = [regex]::Escape([string]$ExpectedLoopbackPort)
    $probePortPattern = [regex]::new(
        "127\.0\.0\.1[.:]\d{1,5}\s*>\s*127\.0\.0\.1[.:]$escapedPort\b|127\.0\.0\.1[.:]$escapedPort\s*>\s*127\.0\.0\.1[.:]\d{1,5}\b",
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase -bor [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    $probeRstPattern = [regex]::new(
        '\bTCP\b.*\bflags\s+\[[^\]]*R[^\]]*\]',
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase -bor [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    $probeSnapshots = @($snapshots | Where-Object {
        $probePortPattern.IsMatch($_.Packet) -and $probeRstPattern.IsMatch($_.Packet)
    })
}
$probeMissing = $ExpectedLoopbackPort -gt 0 -and $probeSnapshots.Count -eq 0

"Pktmon IPv4 loopback TCP snapshots (capture filter: RST): $($snapshots.Count)" | Out-File -FilePath $SummaryPath -Append -Encoding utf8
'Pktmon読取り件数: PktGroupId記録={0}; 向き認識記録={1}; 後続header行={2}; IPv4/TCP tuple={3}; loopback TCP tuple={4}。' -f $rawRecordMarkerCount, $recognizedRecordCount, $packetHeaderLineCount, $decodedIpv4TcpTupleCount, $snapshots.Count | Out-File -FilePath $SummaryPath -Append -Encoding utf8
'Pktmonテキスト形状: 行数={0}; PktGroupId行={1}; PktNumber行={2}; Appearance行={3}; Direction行={4}; IPv4表記数={5}; TCP標識行={6}; RST標識行={7}。' -f $lines.Count, $rawRecordMarkerCount, $rawPacketNumberMarkerCount, $rawAppearanceMarkerCount, $rawDirectionMarkerCount, $rawIpv4LiteralCount, $rawTcpMarkerLineCount, $rawRstMarkerLineCount | Out-File -FilePath $SummaryPath -Append -Encoding utf8
'Only PktGroupId/PktNumber/Appearance records followed by a decoded IPv4 TCP tuple are counted; filter and component catalogs are excluded.' | Out-File -FilePath $SummaryPath -Append -Encoding utf8
'Snapshots may repeat one packet across stack components; correlate PktGroupId and PktNumber. The capture filter selects TCP RST.' | Out-File -FilePath $SummaryPath -Append -Encoding utf8
if ($ExpectedLoopbackPort -gt 0) {
    if ($probeSnapshots.Count -gt 0) {
        "自己接続RST試験の待受け先番号 $ExpectedLoopbackPort を持つRST headerを$($probeSnapshots.Count)件のtuple snapshotで確認しました。" | Out-File -FilePath $SummaryPath -Append -Encoding utf8
    } else {
        "自己接続RST試験の待受け先番号 $ExpectedLoopbackPort を持つRST headerは未検出です。" | Out-File -FilePath $SummaryPath -Append -Encoding utf8
    }
}

foreach ($snapshot in ($snapshots | Select-Object -First 20)) {
    if (!$probeMissing) {
        $snapshot.Record | Out-File -FilePath $SummaryPath -Append -Encoding utf8
        $snapshot.Packet | Out-File -FilePath $SummaryPath -Append -Encoding utf8
    }
}
if ($snapshots.Count -gt 20) {
    "Pktmon summary truncated to the first 20 of $($snapshots.Count) snapshots." | Out-File -FilePath $SummaryPath -Append -Encoding utf8
}
if ($snapshots.Count -eq 0) {
    'No decoded loopback TCP tuple rows matched; filter/component metadata alone is not packet evidence.' | Out-File -FilePath $SummaryPath -Append -Encoding utf8
}
if ($probeMissing) {
    'probe RST headerが未検出のため、header形状だけを記録します（最大20件。IP／port／生header／packet本文は出力しません）:' | Out-File -FilePath $SummaryPath -Append -Encoding utf8
    foreach ($header in ($headerShapes | Select-Object -First 20)) {
        "記録=$($header.Record), 向き=$($header.Direction), 理由=$($header.Reason), IPv4表記数=$($header.Ipv4Literals), TCP=$($header.Tcp), RSTflag=$($header.RstFlag), 区切り=$($header.Separator), 行文字数=$($header.LineLength)" | Out-File -FilePath $SummaryPath -Append -Encoding utf8
    }
    if ($headerShapes.Count -gt 20) {
        "header形状診断は先頭20件のみ表示（全$($headerShapes.Count)件）。" | Out-File -FilePath $SummaryPath -Append -Encoding utf8
    }
} elseif ($unparsedHeaders.Count -gt 0) {
    '未解釈headerの形状（最大20件。生headerやpacket本文は出力しません）:' | Out-File -FilePath $SummaryPath -Append -Encoding utf8
    foreach ($header in ($unparsedHeaders | Select-Object -First 20)) {
        "記録=$($header.Record), 向き=$($header.Direction), 理由=$($header.Reason), IPv4表記数=$($header.Ipv4Literals), TCP=$($header.Tcp), RSTflag=$($header.RstFlag), 区切り=$($header.Separator), 行文字数=$($header.LineLength)" | Out-File -FilePath $SummaryPath -Append -Encoding utf8
    }
    if ($unparsedHeaders.Count -gt 20) {
        "未解釈header形状は先頭20件のみ表示（全$($unparsedHeaders.Count)件）。" | Out-File -FilePath $SummaryPath -Append -Encoding utf8
    }
}
if ($probeMissing) {
    throw "Pktmon解析結果に自己接続RST試験の待受け先番号を持つRST headerがありません: $ExpectedLoopbackPort"
}
