param(
  [Parameter(Mandatory = $true)][string]$CollectorPath,
  [Parameter(Mandatory = $true)][string]$CasesJson
)
$ErrorActionPreference = "Stop"
# 実collectorの純粋判定関数だけを実行し、appやbrokerを起動しない。
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseInput([IO.File]::ReadAllText($CollectorPath, [Text.Encoding]::UTF8), [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -ne 0) { throw "collectorの構文が不正" }
$functions = @($ast.FindAll({ param($node)
  $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq "Test-ObservedSurfaceVisible"
}, $true))
if ($functions.Count -ne 1) { throw "可視判定関数が一意でない" }
. ([scriptblock]::Create($functions[0].Extent.Text))
$cases = Get-Content -Raw -Encoding UTF8 -LiteralPath $CasesJson | ConvertFrom-Json
foreach ($case in $cases) {
  $actual = Test-ObservedSurfaceVisible -Element $case.nodes[1] -ObservedElements $case.nodes
  if ($actual -ne $case.expected) { throw "可視判定が期待と不一致: $($case.name)" }
}
Write-Output "collector可視判定PASS: $($cases.Count)件"
