[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$InputPath,
    [Parameter(Mandatory = $true)]
    [string]$SummaryPath
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

for ($i = 0; $i -lt $lines.Count; $i++) {
    $recordMatch = $recordPattern.Match($lines[$i])
    if (!$recordMatch.Success) { continue }

    $packetLine = $null
    for ($j = $i + 1; $j -lt $lines.Count; $j++) {
        if ([string]::IsNullOrWhiteSpace($lines[$j])) { continue }
        if ($recordPattern.IsMatch($lines[$j])) { break }
        $packetLine = $lines[$j]
        break
    }
    if ($null -eq $packetLine) { continue }

    $tupleMatch = $tuplePattern.Match($packetLine)
    if (!$tupleMatch.Success) { continue }
    $sourceIsLoopback = $tupleMatch.Groups['source'].Value -eq '127.0.0.1'
    $destinationIsLoopback = $tupleMatch.Groups['destination'].Value -eq '127.0.0.1'
    if (!$sourceIsLoopback -and !$destinationIsLoopback) { continue }

    $snapshots.Add([pscustomobject]@{
        Record = $lines[$i].Trim()
        Packet = $packetLine.Trim()
    })
}

"Pktmon IPv4 loopback TCP snapshots (capture filter: RST): $($snapshots.Count)" | Out-File -FilePath $SummaryPath -Append -Encoding utf8
'Only PktGroupId/PktNumber/Appearance records followed by a decoded IPv4 TCP tuple are counted; filter and component catalogs are excluded.' | Out-File -FilePath $SummaryPath -Append -Encoding utf8
'Snapshots may repeat one packet across stack components; correlate PktGroupId and PktNumber. The capture filter selects TCP RST.' | Out-File -FilePath $SummaryPath -Append -Encoding utf8

foreach ($snapshot in ($snapshots | Select-Object -First 20)) {
    $snapshot.Record | Out-File -FilePath $SummaryPath -Append -Encoding utf8
    $snapshot.Packet | Out-File -FilePath $SummaryPath -Append -Encoding utf8
}
if ($snapshots.Count -gt 20) {
    "Pktmon summary truncated to the first 20 of $($snapshots.Count) snapshots." | Out-File -FilePath $SummaryPath -Append -Encoding utf8
}
if ($snapshots.Count -eq 0) {
    'No decoded loopback TCP tuple rows matched; filter/component metadata alone is not packet evidence.' | Out-File -FilePath $SummaryPath -Append -Encoding utf8
}
