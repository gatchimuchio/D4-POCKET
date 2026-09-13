# 開発専用。合成dataだけを使い、資格・署名鍵・製品storeは開かない。
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Windows実行環境が必要です' }
Add-Type -AssemblyName System.Security
$sample = [System.Text.Encoding]::UTF8.GetBytes('GUI-Shell synthetic DPAPI validation data')
$purpose = [System.Text.Encoding]::UTF8.GetBytes('GUI-Shell:history:test:v1')
$otherPurpose = [System.Text.Encoding]::UTF8.GetBytes('GUI-Shell:credential:test:v1')
$scope = [System.Security.Cryptography.DataProtectionScope]::CurrentUser
$checks = [ordered]@{}
$encrypted = [System.Security.Cryptography.ProtectedData]::Protect($sample, $purpose, $scope)
$checks['暗号化'] = -not [System.Linq.Enumerable]::SequenceEqual[byte]($sample, $encrypted)
$restored = [System.Security.Cryptography.ProtectedData]::Unprotect($encrypted, $purpose, $scope)
$checks['正常復号'] = [System.Linq.Enumerable]::SequenceEqual[byte]($sample, $restored)
$checks['異用途拒否'] = $false
try { $null = [System.Security.Cryptography.ProtectedData]::Unprotect($encrypted, $otherPurpose, $scope) }
catch [System.Security.Cryptography.CryptographicException] { $checks['異用途拒否'] = $true }
$checks['改変拒否'] = $false
$tampered = [byte[]]$encrypted.Clone()
$tampered[$tampered.Length - 1] = $tampered[$tampered.Length - 1] -bxor 1
try { $null = [System.Security.Cryptography.ProtectedData]::Unprotect($tampered, $purpose, $scope) }
catch [System.Security.Cryptography.CryptographicException] { $checks['改変拒否'] = $true }
$checks['切断拒否'] = $false
try { $null = [System.Security.Cryptography.ProtectedData]::Unprotect([byte[]]$encrypted[0..7], $purpose, $scope) }
catch [System.Security.Cryptography.CryptographicException] { $checks['切断拒否'] = $true }
if ($checks.Values -contains $false) { throw 'DPAPI合成data試験が不成立' }
[ordered]@{
    result = 'PASS'
    evidence_source = 'FIXTURE'
    windows_dpapi_executed = $true
    checks = $checks
    product_broker_path_verified = $false
    different_user_verified = $false
    same_user_resistance_claimed = $false
    release_ready = $false
} | ConvertTo-Json -Depth 4
