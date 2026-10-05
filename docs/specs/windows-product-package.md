# Windows製品Package

状態: P11 Windows Productizationの基盤contract。Installer／Update production pathの成立を意味しない。

## 1. 責任境界

`D4PKG01`はWindows製品payloadを固定順に格納する無圧縮package形式である。package全体の配布元署名・digest検証はRust Brokerの責任であり、この形式reader、package作成tool、Product Manifest自身は配布元trustを生成しない。

Developer専用の`tooling/package_windows_product.py`は、commit／build receiptに結合されたportable Export bundleを梱包する。通常利用者へPython、Rust、Flutter、Git、terminalを要求する経路ではない。

Rust `product_package` readerは、期待するApp ID／Audit Store ID／製品版を呼出し元から固定で受け、packageの構造、path、file数、size、各file hash、同梱`product_manifest.json`の新規identityとAuthority非継承を検査する。展開先は未存在の新規stage directoryに限定する。検査失敗時はreaderが作成したstageだけを除去する。

このmoduleはBrokerの署名検証、Owner確認、Permission、Approval、Audit、filesystem installation、process起動またはrollbackを実施しない。現時点ではInstaller／Updateから未接続であり、P11の製品経路受入れは未成立である。

## 2. 固定形式

```text
8 bytes   D4PKG01\n
4 bytes   manifest UTF-8 JSON length (unsigned little-endian)
N bytes   specs/d4_pocket_product_package_manifest.schema.jsonに従うmanifest
...       manifest順に連結したfile bytes
```

manifestのfile一覧はASCII Windows-safe relative pathのcase-insensitive昇順で、duplicate pathを持たない。各entryのbyte lengthとSHA-256をpayloadと照合し、package末尾の余剰byteを拒否する。現在の上限はpackage 4 GiB、package manifest 1 MiB、file数20,000、同梱Product Manifest 64 KiB。

許可rootは`app/`、`broker/`、`gui_shell_desktop_launcher.exe`、`product_manifest.json`。必須のDesktop／Flutter／Broker／launcher／Product Manifest fileを欠くpackageは拒否する。

## 3. 適合確認

packagerのPython ConformanceとRust readerのunit testは、形式・hash・path containment・製品identity・Authority非継承を検査する。これらは`FIXTURE`／unit evidenceであり、installed product、update、rollback、署名検証、通常Release成立の証拠ではない。
