# Windows製品Package

状態: P11のpackage format／readerおよびBroker統治の未起動version staging consumer。Installer、active version切替、Start Menu、Rollback、installed product、正式配布の成立を意味しない。

## 1. 責任境界

`D4PKG01`はWindows製品payloadを固定順に格納する無圧縮package形式である。これを導入・更新へ接続するときの配布元署名・digest検証はRust Brokerが担う。この形式reader、package作成tool、Product Manifest自身は配布元trustを生成しない。

Developer専用の`tooling/package_windows_product.py`は、commit／build receiptに結合されたportable Export bundleを梱包する。通常利用者へPython、Rust、Flutter、Git、terminalを要求する経路ではない。

Rust `product_package` readerは、期待するApp ID／Audit Store ID／製品版を呼出し元から固定で受け、packageの構造、path、file数、size、各file hash、同梱`product_manifest.json`の新規identityとAuthority非継承を検査する。展開先は未存在の新規stage directoryに限定する。検査失敗時はreaderが作成したstageだけを除去する。

Broker consumer向けAPIは、capability directoryから開いた入力file handle、正確なbyte長、署名済み候補のpackage SHA-256に加え、出力先parent directory capabilityと単一componentのstage名を受け取る。同一入力handleから読んだ全byteを検査し、stageとpayload内directoryをcapability相対・no-followで作成／展開する。stage作成後は開いたdirectory handleとfile identityを照合し、既存stageを上書きしない。検査失敗時はidentity照合済みの当該stage handleだけを除去し、identityを確立できない場合は別directory誤削除を避けて残置する。Ambient pathを出力先として再openするAPIではない。Windows Brokerの`更新適用要求`consumerはこのAPIへ接続済みで、実行時にBroker固定download storeから同一file handleをno-followで開き、署名済み全体hash／長さとProduct Manifestを再検証してから、固定`versions` directory capability内へ展開する。既存stageは上書きしない。

package reader自体はBrokerの署名検証、Owner確認、Permission、Approval、Audit、製品配置を実施しない。package内SHA-256は自己整合性検査であり、配布元の真正性を証明しない。現行consumerはBroker現在trustで候補を再検証し、Rust Desktop起動器のnative Owner確認を要求する。Brokerへ埋め込まれた製品App／Audit Store identityとWindows Known Folderから固定導入先を再導出し、展開前のdurable Audit、展開結果Auditを記録する。成功値は`version_staged`であり、有効化は`suspended`のまま。これはBroker test fixture上のconsumer経路と実Win32 Owner dialog表示・選択の検証であり、installed productからの配布・起動を証明しない。

## 2. 導入・更新consumerの必須境界

導入・更新のversion stagingは既存Brokerだけが実行する。通常IPC、Owner資格のみの要求、Flutter／manifest／package metadataは導入権限を生成できず、native Owner確認がない要求は拒否する。native確認対象は現在trustで検証された候補の版、channel、package hash／byte長、App ID、Audit Store ID、Known Folder由来の固定導入先であり、Brokerが実行直前に候補・identity・destinationを再照合する。Brokerはdurable Audit storeが利用可能な場合だけintentを先に確定し、その後でLocalAppData配下にcapability directoryを作成し、packageを展開する。

本consumerが行うのはcontent-addressedな未起動version directoryの作成だけである。Start Menu変更、active version選択、process起動、旧version削除、rollback、Uninstaller、Repair UIは行わない。通常失敗は自身のstageだけをreaderがidentity照合してcleanupする。process crash／電源断で展開途中のstageが残った場合も起動・有効化せずfail-closedとなるが、現状は残存stageの自動Recovery／再試行を実装していないため、同じdestinationへの再適用は拒否される。このRecoveryとinstalled product経路はP11の未完了条件として残す。standalone Setupからfilesystem／registryへ直接作用する経路は設けない。

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

packagerのPython ConformanceとRust readerのunit testは、形式・hash・path containment・製品identity・Authority非継承を検査する。Broker apply fixtureはnative Owner由来の一回限り確認context、現在candidate／製品identity／destination再照合、package tamper拒否、durable intent／completion Audit、version staging、再適用時の非上書きを検査する。Win32 Owner UI試験はRust起動器が作る確認文と実MessageBoxの表示・No／Yes結果を検査する。これらは別user profileからのinstalled product、実配布元経由のdownload、Installer、active version／Start Menu切替、crash後stage Recovery、Rollback、正式Releaseの証拠ではない。

## 6. 製品導入先の固定規則

P11の導入先は、管理者権限を要求しない現在利用者単位とする。Rust BrokerはWindows Known Folder APIで取得した現在利用者の`LocalAppData`を起点に、次の固定rootを導出する。

```text
%LOCALAPPDATA%/Programs/D4 Pocket/<App ID>/versions/<製品版>-<package SHA-256>/
```

`App ID`、製品版、package SHA-256は検証済みpackage／現在のBroker trustから得る。Flutter、Setup UI、update候補は導入先pathを指定しない。異なるApp ID、製品版、package digestは別のversion directoryになる。導入先が同一volumeであることを要求し、既存version directoryを上書きしない。

Windows Broker consumerはKnown Folder APIの現在利用者`LocalAppData`から固定`versions` directoryをcapabilityで開き、同一volumeと非reparse directory identityを検証して、署名済みpackageのversion directoryを新規作成する。展開前intent Auditが失敗した場合はinstall directoryを作成しない。package全体と内包fileの検証が成功した場合だけ`version_staged`を返し、永続Auditへ完了記録する。Audit storeは既存のidentity別`%LOCALAPPDATA%/D4Pocket/apps/<App ID>/stores/<Audit Store ID>`に残し、製品payloadの削除・更新と混同しない。Machine-wide registry、elevation、利用者指定の任意install pathはこのP11基本経路に含めない。なお、固定path／fixture上のconsumer接続はinstalled productの起動、別user profile隔離、process cleanup、起動失敗Recovery、Rollbackを意味しない。
