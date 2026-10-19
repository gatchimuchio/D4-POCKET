# Linux製品Package

状態: Linux native必須製品対象の固定package目録／reader契約。Brokerの署名trust、Owner確認、導入consumer、配布物の受入れを代行しない。

## 1. 役割と互換性

Linux製品payloadも既存の固定container `D4PKG01`を使う。既存Windows packageの公開形式、Schema名、path、legacy manifest bytesは変更しない。Linux packageは専用目録Schema `d4_pocket_linux_product_package_manifest.schema.json`を使用し、manifestへ`target_platform: "linux"`を必須記録する。`target_platform`のない既存manifestはWindows payloadとしてのみ解釈し、Linux packageとして推測しない。

containerは8-byte magic、little-endian 32-bit manifest長、UTF-8 JSON manifest、manifest順のfile bytesで構成する。外側の署名・package digest・配布元trustは既存Brokerが別に検証する。目録内SHA-256はfile byteとの自己整合だけを示し、署名者の真正性を作らない。

## 2. Linux payload境界

許可rootは`data/`、`lib/`、固定`libexec/gui_shell_linux_broker`、固定root executable`gui_shell_desktop`／`gui_shell_desktop_launcher`、`product_manifest.json`だけとする。必須fileはGTK Runner、Linux broker helper、Linux launcher、Flutter ICU data、Flutter GTK shared library、Product Manifestである。Payloadに含まれるfileは通常fileに限り、symlink、特殊file、path traversal、予約された他platform root、case-insensitive duplicateを受け付けない。

各fileはbyte長、SHA-256、およびPOSIX permission modeを目録へ含める。D4所有executable三fileだけ`0755`、その他のfileは`0644`とする。setuid／setgid／sticky、group／other write、任意実行bitを許可しない。展開directoryは固定`0755`とする。これらのmodeはmanifestの自己申告だけで決まらず、Linux Broker readerがpathに対する期待値と照合して展開時に設定し、再検証時も実体を照合する。

## 3. Authorityと導入境界

Readerはpackage structure、製品版、固定identity、Product Manifestの新規identity／Authority非継承、file inventory／hash／modeを検査して、Owner操作を生成せず新規stageへ展開する。通常展開失敗時は今回作成したstageだけをcleanupする。Resumeは既存byte prefixとmodeの一致を必須とし、既存fileを上書き・削除しない。

Linux Brokerの署名候補、compile-time App／Audit identity、固定XDG導入root、native Owner確認、durable Auditが一致して初めてproduct installation consumerへ接続できる。package stagingはactive-version切替、desktop entry、update、rollback、uninstall、installed product受入れを意味しない。これらのconsumerが未接続の間、Linux完全対応や`task_execution=supported`を主張しない。
