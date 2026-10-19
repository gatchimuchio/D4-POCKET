# Linux製品Package

状態: Linux native必須製品対象のExport build／固定package producer／目録／reader契約。Brokerの署名trust、Owner確認、active-version導入、native installed product受入れを代行しない。

## 1. 役割と互換性

Linux製品payloadも既存の固定container `D4PKG01`を使う。既存Windows packageの公開形式、Schema名、path、legacy manifest bytesは変更しない。Linux packageは専用目録Schema `d4_pocket_linux_product_package_manifest.schema.json`を使用し、manifestへ`target_platform: "linux"`を必須記録する。`target_platform`のない既存manifestはWindows payloadとしてのみ解釈し、Linux packageとして推測しない。

containerは8-byte magic、little-endian 32-bit manifest長、UTF-8 JSON manifest、manifest順のfile bytesで構成する。外側の署名・package digest・配布元trustは既存Brokerが別に検証する。目録内SHA-256はfile byteとの自己整合だけを示し、署名者の真正性を作らない。

## 2. Linux payload境界

許可rootは`data/`、`lib/`、固定`libexec/gui_shell_linux_broker`、固定root executable`gui_shell_desktop`／`gui_shell_desktop_launcher`、`product_manifest.json`だけとする。必須fileはGTK Runner、Linux broker helper、Linux launcher、Flutter ICU data、Flutter GTK shared library、Product Manifestである。Payloadに含まれるfileは通常fileに限り、symlink、特殊file、path traversal、予約された他platform root、case-insensitive duplicateを受け付けない。

各fileはbyte長、SHA-256、およびPOSIX permission modeを目録へ含める。D4所有executable三fileだけ`0755`、その他のfileは`0644`とする。setuid／setgid／sticky、group／other write、任意実行bitを許可しない。展開directoryは固定`0755`とする。これらのmodeはmanifestの自己申告だけで決まらず、Linux Broker readerがpathに対する期待値と照合して展開時に設定し、再検証時も実体を照合する。

## 3. Authorityと導入境界

Readerはpackage structure、製品版、固定identity、Product Manifestの新規identity／Authority非継承、file inventory／hash／modeを検査して、Owner操作を生成せず新規stageへ展開する。通常展開失敗時は今回作成したstageだけをcleanupする。Resumeは既存byte prefixとmodeの一致を必須とし、既存fileを上書き・削除しない。

Linux Brokerの署名候補、compile-time App／Audit identity、固定XDG導入root、native Owner確認、durable Auditを再照合する導入consumerを既存Brokerへ接続する。更新適用はBrokerの現在candidate一覧に束縛したLinux native Owner確認を経て、承認時のみ同じ確認情報を既存Broker queueへ渡す。Brokerは署名済みdigestのdownload fileを再検証してLinux D4PKG01を未起動version directoryへstageし、開始／完了Auditを確定する。通常IPC、確認先差替、所有UID／hardlink／unsafe mode違反、package hash／内容不一致はfail-closedする。

stage後の有効化は別の`更新有効版切替要求`で行う。Linux native Owner確認には現在のBroker candidate、compile-time App／Audit identity、固定version directory、固定XDG desktop entry pathを結合し、Brokerが現在のdata homeと両destinationを再導出して一致しなければ拒否する。成功時はstage全体を署名済みpackageへread-only照合し、既存Broker bootstrapperにactive-version recordを記録する。Brokerのactive-version resolverは現在recordとstage launcherを検証し、固定root launcherはfixed product root内のregular executableとしてowner／mode／link数を検査する。固定root launcherはversion切替後も維持し、per-user `.desktop` entryのExec／TryExecはこの固定root launcherを指す。既存entryは同一内容・owner・mode・link数の場合だけ再利用し、競合・symlinkは拒否する。queued／completed／failed Auditを残し、process起動はしない。通常IPC、stale path、stage／package改変はfail-closedする。

これらはLinux targetのBroker／filesystem試験とWSL上の実装経路証拠であり、native desktop session上のinstalled GUI／desktop entry選択／実launcher起動、Update／Rollback／Uninstaller lifecycleの全体受入れを意味しない。Export builderのsource／toolchain／artifact provenanceは別節で定義する。Linux完全対応と`task_execution=supported`の判定にはinstaller lifecycleとnative acceptanceを含む現行Release blockerの全残件を閉じる必要がある。WSL証拠をnative Linux acceptanceへ転用しない。

## 4. Linux製品package作成tool

tooling/package_linux_product.pyはLinux host専用の開発／build toolであり、既に生成されたLinux bundleを既存D4PKG01へ梱包する。入力には固定root／required file inventory、Linux targetのProduct Manifest、新規App／Audit identity、Authority非継承を要求し、許可外path、symlink、特殊file、hard link、ownerが読めないfile、setuid／setgid／sticky、group／other writeを拒否する。各source fileはnofollowで開き、inventory取得時とpackage作成時に同一device／inode／mode／size／timestampを再照合して、byte length／SHA-256の変化を拒否する。

目録modeはD4所有executable三fileを0755、その他を0644へ固定する。package outputは新規fileとしてbundle／Repository／OneDriveの外へcreate-only公開する。既知Credential pattern scanも実行するが、これは列挙済み形式の検出に限られ、秘密不存在の一般証明ではない。

このproducer自身はLinux binaryをbuildせず、source commit／toolchain provenanceを生成・検証せず、packageへ署名せず、Broker trust／Permission／Approval／導入を行わない。出力されたunsigned D4PKG01のdigestはBrokerの信頼済みcandidate検証へ別途結合する。Linux binary buildは次節のExport builderが担う。

## 5. Linux Export builder

`tooling/export_linux_product.py`はLinux host上でだけ動くDeveloper／build経路である。入力はBroker Export Receiptと同Receiptに束縛されたManifest fileであり、両者のSchema、byte長、SHA-256、Export ID、Manifest内容、Linux target、新規App／Audit identity、Authority非継承、ModulePlanを検査する。合成fixtureはbuilderのtest専用であり、Owner承認や実Broker Exportの証拠ではない。

開始時にsource branch、clean working tree、HEAD、同名origin branchのremote HEAD一致を検査し、commitからGit archiveを作って隔離sourceへ展開する。Flutter／Pub／Cargo／HOME／cache／一時領域を分離し、build子processへ認証情報を含む親環境を継承しない。Flutter Linux Releaseの設定後、CMake install先をstaging bundleへ固定してRunner、launcher、Linux Broker、Flutter engine／assetをbuildする。検証済みManifestのApp ID／Audit store IDはFlutter入力やUI stateでなく、Rust binaryのcompile-time入力へ渡す。更新trustは検証済みManifestからRust build環境へだけ渡し、provenanceへは公開fingerprint／配布元数だけを記録する。

出力rootはRepository／OneDrive外の新規pathに限定し、`bundle/`、unsigned `D4PKG01`、`d4_pocket_linux_export_build.json`を一括公開する。evidenceはsource ref／commit／archive hash、Receipt／Manifest hash、toolchain、実行argument、compile-time identity／update-trust summary、artifact inventory／hash、package hash、限定known-pattern Credential scanを結合する。失敗時に部分出力を残さない。Credential scanは既知patternだけを対象とし、秘密不存在の一般証明ではない。

このbuild evidenceは`INTERNAL_STATE`であり、Manifestのruntime消費、binary pruningの実証、Owner承認、署名、正式配布、インストール、起動、update／rollback／uninstallを主張しない。`.github/workflows/linux-manual-export-build.yml`は`workflow_dispatch`だけで起動するLinux build補助で、Pinned Flutter／Rust toolchain、合成非権限fixture、出力cleanupを使う。Actions成功はLinux source build／bundle／packageの証拠であり、Linux native GUI／Secret Service／権限／installed productの受入れを代替しない。Linuxは任意platformではなくWindows／macOSと同列の必須製品対象であり、installer lifecycleとnative installed acceptanceを含む残件が閉じるまで完全対応扱いにしない。
