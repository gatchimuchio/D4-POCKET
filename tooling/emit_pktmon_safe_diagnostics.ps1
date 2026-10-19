param(
    [Parameter(Mandatory = $true)]
    [string]$SummaryPath
)

$ErrorActionPreference = 'Stop'
if (!(Test-Path -LiteralPath $SummaryPath -PathType Leaf)) {
    throw 'Pktmon要約を読み取れません'
}

$snapshotPattern = [regex]::new(
    '^Pktmon IPv4 loopback TCP snapshots \(capture filter: RST\): (?<count>\d+)$',
    [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
)
$countPattern = [regex]::new(
    '^Pktmon読取り件数: PktGroupId記録=(?<records>\d+); 向き認識記録=(?<directions>\d+); 後続header行=(?<headers>\d+); IPv4/TCP tuple=(?<tuples>\d+); loopback TCP tuple=(?<loopback>\d+)。$',
    [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
)
$missingProbePattern = [regex]::new(
    '^自己接続RST試験の待受け先番号 \d+ を持つRST headerは未検出です。$',
    [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
)
$matchedProbePattern = [regex]::new(
    '^自己接続RST試験の待受け先番号 \d+ を持つRST headerを(?<count>\d+)件のtuple snapshotで確認しました。$',
    [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
)
$shapePattern = [regex]::new(
    '^記録=(?<record>\d+), 向き=(?<direction>RX|TX), 理由=(?<reason>header_missing|tuple_unrecognized|tuple_recognized), IPv4表記数=(?<ipv4>\d+), TCP=(?<tcp>True|False), RSTflag=(?<rst>True|False), 区切り=(?<separator>none|greater_than|arrow|to_word), 行文字数=(?<length>\d+)$',
    [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
)

$diagnostics = [System.Collections.Generic.List[string]]::new()
$hasCounts = $false
$hasProbeResult = $false
foreach ($rawLine in Get-Content -LiteralPath $SummaryPath -ErrorAction Stop) {
    $line = $rawLine.Trim()
    if ($line.Length -gt 256) { continue }

    $match = $snapshotPattern.Match($line)
    if ($match.Success) {
        $diagnostics.Add("Pktmonで確認したループバックTCP RST tuple件数=$($match.Groups['count'].Value)")
        continue
    }

    $match = $countPattern.Match($line)
    if ($match.Success) {
        $diagnostics.Add(
            "Pktmon記録数: records=$($match.Groups['records'].Value); 方向=$($match.Groups['directions'].Value); header=$($match.Groups['headers'].Value); IPv4/TCP tuple=$($match.Groups['tuples'].Value); loopback TCP tuple=$($match.Groups['loopback'].Value)"
        )
        $hasCounts = $true
        continue
    }

    if ($missingProbePattern.IsMatch($line)) {
        $diagnostics.Add('自己接続RST=未検出')
        $hasProbeResult = $true
        continue
    }

    $match = $matchedProbePattern.Match($line)
    if ($match.Success) {
        $diagnostics.Add("自己接続RST=検出 tuple件数=$($match.Groups['count'].Value)")
        $hasProbeResult = $true
        continue
    }

    $match = $shapePattern.Match($line)
    if ($match.Success -and $diagnostics.Count -lt 23) {
        $diagnostics.Add(
            "header形状: 記録=$($match.Groups['record'].Value) 向き=$($match.Groups['direction'].Value) 判定=$($match.Groups['reason'].Value) IPv4表記数=$($match.Groups['ipv4'].Value) TCP=$($match.Groups['tcp'].Value) RSTflag=$($match.Groups['rst'].Value) 区切り=$($match.Groups['separator'].Value) 文字数=$($match.Groups['length'].Value)"
        )
    }
}

if (!$hasCounts -or !$hasProbeResult -or $diagnostics.Count -gt 24) {
    throw 'Pktmon要約に上限付き診断項目がありません'
}

return $diagnostics.ToArray()
