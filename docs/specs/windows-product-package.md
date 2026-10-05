# Windows製品Package

状態: P11のpackage format／readerおよびBroker統治の未起動version staging consumer。Installer、active version切替、Start Menu、Rollback、installed product、正式配布の成立を意味しない。

## 1. 責任境界

`D4PKG01`はWindows製品payloadを固定順に格納する無圧縮package形式である。これを導入・更新へ接続するときの配布元署名・digest検証はRust Brokerが担う。この形式reader、package作成tool、Product Manifest自身は配布元trustを生成しない。

Developer専用の`tooling/package_windows_product.py`は、commit／build receiptに結合されたportable Export bundleを梱包する。通常利用者へPython、Rust、Flutter、Git、terminalを要求する経路ではない。

Rust `product_package` readerは、期待するApp ID／Audit Store ID／製品版を呼出し元から固定で受け、packageの構造、path、file数、size、各file hash、同梱`product_manifest.json`の新規identityとAuthority非継承を検査する。通常展開APIは未存在の新規stage directoryだけを受け付け、検査失敗時はreaderが作成したstageだけを除去する。Broker専用の再開APIは、既存fileが署名packageの対応fileと一致するbyte prefixであることを検査し、残りbyteだけを追記できる。既存fileの上書き・削除はしない。

Broker consumer向けAPIは、capability directoryから開いた入力file handle、正確なbyte長、署名済み候補のpackage SHA-256に加え、出力先parent directory capabilityと単一componentのstage名を受け取る。同一入力handleから読んだ全byteを検査し、stageとpayload内directoryをcapability相対・no-followで作成／展開する。stage作成後は開いたdirectory handleとfile identityを照合する。通常展開は既存stageを拒否し、再開APIは既存fileのbyte prefixとpackage内容を照合して残りだけを追記する。stage全体も走査し、package inventoryと一致しないentry、reparse point、内容不一致を拒否する。新規stageの検査失敗時だけ、そのstage handleをidentity照合して除去し、再開stageは失敗時も保持する。Ambient pathを出力先として再openするAPIではない。Windows Brokerの`更新適用要求`consumerはこの再開APIへ接続済みで、実行時にBroker固定download storeから同一file handleをno-followで開き、署名済み全体hash／長さとProduct Manifestを再検証してから、固定`versions` directory capability内へ展開する。既存内容は上書き・削除しない。

package reader自体はBrokerの署名検証、Owner確認、Permission、Approval、Audit、製品配置を実施しない。package内SHA-256は自己整合性検査であり、配布元の真正性を証明しない。現行consumerはBroker現在trustで候補を再検証し、Rust Desktop起動器のnative Owner確認を要求する。Brokerへ埋め込まれた製品App／Audit Store identityとWindows Known Folderから固定導入先を再導出し、展開前のdurable Audit、展開結果Auditを記録する。成功値は`version_staged`であり、有効化は`suspended`のまま。これはBroker test fixture上のconsumer経路と実Win32 Owner dialog表示・選択の検証であり、installed productからの配布・起動を証明しない。

## 2. 導入・更新consumerの必須境界

導入・更新のversion stagingは既存Brokerだけが実行する。通常IPC、Owner資格のみの要求、Flutter／manifest／package metadataは導入権限を生成できず、native Owner確認がない要求は拒否する。native確認対象は現在trustで検証された候補の版、channel、package hash／byte長、App ID、Audit Store ID、Known Folder由来の固定導入先であり、Brokerが実行直前に候補・identity・destinationを再照合する。Brokerはdurable Audit storeが利用可能な場合だけintentを先に確定し、その後でLocalAppData配下にcapability directoryを作成し、packageを展開する。

本consumerが行うのはcontent-addressedな未起動version directoryの作成だけである。Start Menu変更、active version選択、process起動、旧version削除、rollback、Uninstaller、Repair UIは行わない。process crash／電源断の後に同じpackageをOwner確認付きで再適用すると、既存fileがpackage byte列のprefixと一致する範囲だけを再利用し、未完byteを追記して完了できる。完全stageへの再要求も全fileとinventoryを再照合する。相違byte、不正属性、reparse point、package外entryはfail-closedで拒否し、既存stageを変更・削除しない。これはRust Broker fixture上の部分stage／再要求testであり、実installed product process crash／電源断のLIVE_RUNTIME証拠ではない。active version、起動、Rollback、Installer／Uninstaller、Repair UIとinstalled product経路はP11未完了のまま。standalone Setupからfilesystem／registryへ直接作用する経路は設けない。

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

## 7. 有効版記録と固定root Bootstrapper

固定導入rootの`gui_shell_desktop_launcher.exe`は、portable bundleの隣接package起動器と同じbinaryを使い、実行fileがWindows Known Folder由来の`LocalAppData/Programs/D4 Pocket/<App ID>/`直下にある場合だけBootstrapper経路へ入る。compile-time App IDがないgeneric起動器やroot外のportable bundleは従来の固定sibling配置を使う。固定root判定に失敗した場合はinstalled modeへ推測で移行しない。

Bootstrapperはroot直下の`active_version.json`を最大4 KiB、`versions` directoryと選択版をno-follow capabilityで読み、同一volume／file identity／非reparse条件を確認する。記録は`specs/d4_pocket_active_version.schema.json`に従い、version、product、App ID、Audit Store ID、製品版、package hash、version-local launcher hash、Product Manifest hashだけを含む。実行pathは受け取らない。有効版directory名は製品版とpackage hashから固定導出し、起動前にlauncherとProduct Manifestのhashを照合し、通常package layoutも再検証する。

この記録は起動対象の選択情報であり、配布元trust、Permission、Approval、Capabilityを作らない。Bootstrapperは受信引数を転送せず、環境をOS用allowlistへ絞り、`LOCALAPPDATA`はKnown Folder API由来値へ固定してからversion-local launcherを起動する。起動後のBroker lifecycle／Auditはversion-local launcherの現行契約に従う。記録欠損・identity不一致・JSON重複／未知field・version path不正・hash不一致・reparse・required payload欠損は、UIやBrokerを起動せず固定codeで失敗する。

現行実装はこの有効版記録を読むBootstrapper分岐と拒否試験までである。Brokerによる記録生成、固定rootへの初回Bootstrapper配置、有効版切替、Start Menu、Rollbackは未接続であり、今回のreaderとfixture試験をinstalled productの起動成立へ昇格しない。記録生成と切替はstage操作と別のnative Owner確認・Broker Permission・durable Auditを持つ次のP11操作として接続する。
