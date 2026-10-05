# Windows製品Package

状態: P11 Windows Productizationのpackage format／reader contract。Installer、installed product、Update／Rollback、正式配布の成立を意味しない。

## 1. 責任境界

`D4PKG01`はWindows製品payloadを固定順に格納する無圧縮package形式である。これを導入・更新へ接続するときの配布元署名・digest検証はRust Brokerが担う。この形式reader、package作成tool、Product Manifest自身は配布元trustを生成しない。

Developer専用の`tooling/package_windows_product.py`は、commit／build receiptに結合されたportable Export bundleを梱包する。通常利用者へPython、Rust、Flutter、Git、terminalを要求する経路ではない。

Rust `product_package` readerは、期待するApp ID／Audit Store ID／製品版を呼出し元から固定で受け、packageの構造、path、file数、size、各file hash、同梱`product_manifest.json`の新規identityとAuthority非継承を検査する。展開先は未存在の新規stage directoryに限定する。検査失敗時はreaderが作成したstageだけを除去する。

Broker consumer向けAPIは、capability directoryから開いた入力file handle、正確なbyte長、署名済み候補のpackage SHA-256に加え、出力先parent directory capabilityと単一componentのstage名を受け取る。同一入力handleから読んだ全byteを検査し、stageとpayload内directoryをcapability相対・no-followで作成／展開する。stage作成後は開いたdirectory handleとfile identityを照合し、既存stageを上書きしない。検査失敗時はidentity照合済みの当該stage handleだけを除去し、identityを確立できない場合は別directory誤削除を避けて残置する。Ambient pathを出力先として再openするBroker consumer APIではない。このAPIはreader unit testで検証済みだが、現行Install／Updateのproduction consumerには未接続である。reader自身は署名検証・Owner確認・install root公開を行わない。

package reader自体はBrokerの署名検証、Owner確認、Permission、Approval、Audit、製品配置を実施しない。package内SHA-256は自己整合性検査であり、配布元の真正性を証明しない。今後の導入・更新consumerはBrokerのtrust、native Owner確認、Permission／Approval、durable Audit、Recovery経路へ接続しなければならない。これらが未接続のpackageを正式配布用として扱ってはならない。

## 2. 導入・更新consumerの必須境界

現行package readerは製品root、Start Menu、registry、processへ作用しない。standalone Setupからこれらを直接変更する経路は設けない。導入・削除・更新を実装するときは、既存Brokerを唯一の権限依存作用主体とし、操作ごとにCapability、現在条件に束縛したPermissionとnative Owner Approval、durable AuditEvent、失敗時RecoveryActionを接続する。Flutter／manifest／Setup UI／package metadataは権限を生成しない。

## 3. 製品版表示

portable Exportは、現行`apps/desktop_flutter/pubspec.yaml`の製品版をbuild時にFlutterへ渡す。Settings表示とpackage metadataの製品版は同じsource値を使う。通常の開発buildでは`開発版`と表示する。この表示はbuild identityの投影であり、package署名、installed root、更新・rollbackの証拠ではない。

## 4. 固定形式

```text
8 bytes   D4PKG01\n
4 bytes   manifest UTF-8 JSON length (unsigned little-endian)
N bytes   specs/d4_pocket_product_package_manifest.schema.jsonに従うmanifest
...       manifest順に連結したfile bytes
```

manifestのfile一覧はASCII Windows-safe relative pathのcase-insensitive昇順で、duplicate pathを持たない。各entryのbyte lengthとSHA-256をpayloadと照合し、package末尾の余剰byteを拒否する。現在の上限はpackage 4 GiB、package manifest 1 MiB、file数20,000、同梱Product Manifest 64 KiB。

許可rootは`app/`、`broker/`、`gui_shell_desktop_launcher.exe`、`product_manifest.json`。必須のDesktop／Flutter／Broker／launcher／Product Manifest fileを欠くpackageは拒否する。

## 5. 適合確認

packagerのPython ConformanceとRust readerのunit testは、形式・hash・path containment・製品identity・Authority非継承を検査する。product version表示のWidget／build argument testはversion source結合だけを検査する。これらはinstalled別user profile、署名検証、package配布元trust、Installer、Update／Rollback、正式Releaseの証拠ではない。

## 6. 製品導入先の固定規則

P11の導入先は、管理者権限を要求しない現在利用者単位とする。Rust BrokerはWindows Known Folder APIで取得した現在利用者の`LocalAppData`を起点に、次の固定rootを導出する。

```text
%LOCALAPPDATA%/Programs/D4 Pocket/<App ID>/versions/<製品版>-<package SHA-256>/
```

`App ID`、製品版、package SHA-256は検証済みpackage／現在のBroker trustから得る。Flutter、Setup UI、update候補は導入先pathを指定しない。異なるApp ID、製品版、package digestは別のversion directoryになる。導入先が同一volumeであることを要求し、既存version directoryを上書きしない。

実package展開後のversion directory切替、Start Menu登録、Permission、native Owner Approval、Audit、RecoveryはBroker consumerの責任であり、固定path計画だけでは実作用を意味しない。D4 Pocket runtime／Audit storeは既存のidentity別`%LOCALAPPDATA%/D4Pocket/apps/<App ID>/stores/<Audit Store ID>`に残し、製品payloadの削除・更新と混同しない。Machine-wide registry、elevation、利用者指定の任意install pathはこのP11基本経路に含めない。
