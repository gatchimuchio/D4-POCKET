# ownerのオフライン署名操作

秘密鍵生成・署名はownerがローカルterminalで実行する。Codexは実運用秘密鍵を生成・読取り・コピーしない。以下のpassphraseはOpenSSLの対話入力で指定し、引数、環境変数、ログへ書かない。媒体上の秘密鍵は終了後切り離す。

初回は外部媒体を接続し、下記PowerShellをownerが実行する。`Read-Host`には外部媒体上の既存directoryを指定する。秘密鍵ではなく公開鍵fileとfingerprintだけがPCへ出る。既存鍵を上書きしない。

```powershell
$offlineRoot = (Resolve-Path -LiteralPath (Read-Host '外部媒体上のGUI-Shell専用directory')).Path
$signingKeyPath = Join-Path $offlineRoot 'gui-shell-ed25519.pem'
$publicKeyPath = Join-Path $offlineRoot 'public-key.der'
if ((Test-Path -LiteralPath $signingKeyPath) -or (Test-Path -LiteralPath $publicKeyPath)) { throw '既存鍵があります。新規生成を中止します。' }
& 'C:\Program Files\Git\usr\bin\openssl.exe' genpkey -algorithm ED25519 -aes-256-cbc -out $signingKeyPath
if ($LASTEXITCODE -ne 0) { throw '鍵生成失敗' }
& 'C:\Program Files\Git\usr\bin\openssl.exe' pkey -in $signingKeyPath -pubout -outform DER -out $publicKeyPath
if ($LASTEXITCODE -ne 0) { throw '公開鍵出力失敗' }
Copy-Item -LiteralPath $publicKeyPath -Destination 'C:\Users\mzcum\codex-work\GUI-Shell-owner-public-key.der'
& 'C:\Users\mzcum\codex-work\GUI-Shell\native\rust_helper\target\debug\gui_shell_rust_helper.exe' 監査チェックポイント fingerprint $publicKeyPath
```

ここで媒体を切り離し、公開鍵fingerprintと公開鍵だけをレビューする。Repositoryの `config/audit_signing_trust.json` にfingerprintを固定してcommitする。実際の固定値が入るまではcollectorが失敗する。鍵ローテーションは既存履歴との連続性を伴う別owner判断とし、継続性記録を0へ戻すことで回避しない。

その後、公開鍵固定済みのclean commitでappとhelperをbuild・stageし、稼働確認後にbrokerを停止する。初回の外部媒体の公開継続性記録は `{"version":1,"sequence":0,"signed_checkpoint_hash":null}`。既存記録を初期化しない。次回以降は最新の受理済みsequence/hashを保持する。

未署名checkpointの生成（pathは当該runに置き換える）:

```powershell
& $helper 監査チェックポイント prepare $store $installedExe $sourceCommit $trustedHeadPath $checkpointPath
```

生成物を新しい署名bundle directoryへ置き、ownerが内容のsource・artifact・sequence・previous hashを確認する。署名時だけ媒体を接続して次を実行する。`$checkpointPath`は上で生成したcanonical file、`$bundle`は新しい証拠directory、`$signingKeyPath`は外部媒体上の秘密鍵である。

```powershell
if (Test-Path -LiteralPath (Join-Path $bundle 'signature.bin')) { throw '既存署名を上書きしません' }
& 'C:\Program Files\Git\usr\bin\openssl.exe' pkeyutl -sign -rawin -inkey $signingKeyPath -in $checkpointPath -out (Join-Path $bundle 'signature.bin')
if ($LASTEXITCODE -ne 0) { throw '署名失敗' }
Copy-Item -LiteralPath $publicKeyPath -Destination (Join-Path $bundle 'public-key.der')
```

bundleのcheckpoint.jsonはcanonical byteのまま保持する。秘密鍵をbundleへ入れない。媒体を切り離し、公開継続性記録を検証用として指定してCollectorとrelease検証を行う。`GUI_SHELL_AUDIT_TRUSTED_HEAD`は公開記録のpathのみであり秘密鍵ではない。公開記録の最新版はownerが媒体上で保持し、検証対象から渡された過去記録へ差し替えない。

```powershell
& $helper 監査チェックポイント verify $store $installedExe $sourceCommit 'config/audit_signing_trust.json' $trustedHeadPath $bundle $previousBundle
& './installer/windows/collect_audit_anchor_proof.ps1' -InstalledRoot $installedRoot -AuditDir $store -OutputPath $proofPath -CheckpointBundle $bundle -TrustedHeadPath $trustedHeadPath -PreviousCheckpointBundle $previousBundle
$env:GUI_SHELL_AUDIT_TRUSTED_HEAD = $trustedHeadPath
python tooling/windows_release_evidence.py --evidence $installedEvidencePath
```

初回previousBundleは `-`。以後は直前署名bundleを指定する。署名checkpointの時刻は検証時から24時間以内、未来は60秒以内。期限を過ぎた証拠を自動再署名しない。Collectorと統合検証を確認後、ownerが最新の受理済みsequence/signed_checkpoint_hashを外部媒体の公開継続性記録へ保存する。これは秘密鍵の複製ではない。同一ユーザー書換え防護はこの独立した信頼記録とオフライン鍵を前提とする。

標準資料: [ring Ed25519検証](https://docs.rs/ring/latest/ring/signature/static.ED25519.html)、[OpenSSL鍵生成](https://docs.openssl.org/3.0/man1/openssl-genpkey/)、[OpenSSL detached署名](https://docs.openssl.org/3.0/man1/openssl-pkeyutl/)。ローカルOpenSSL 3.2.4の存在とversionを確認した。実owner鍵でのこれらの操作は未実行。

- item: owner鍵生成と署名
  classification: release_blocker
  reason: 実運用fingerprint・owner署名・独立した最新継続性記録は未取得。
  required_action: 上記のowner操作を実施し、公開物だけを検証へ渡す。
  blocks_release: yes
