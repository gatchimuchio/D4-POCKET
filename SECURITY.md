# GUI Shell セキュリティ

## セキュリティ姿勢

GUI Shell は control plane である。セキュリティ判断は Flutter widget ではなく、schema、conformance、Shell Core、Adapter Contract、および範囲を限定した Native Helper の各面で行わなければならない。

## C31現況と検証境界（2026-09-24）

C30のlocal回帰matrixで、Trust、Authority、Permission、Approval、Audit、Recovery、Evidence、Runtime、Agent、Dialogue、Compare、Device Linkに対応する既存のRust／Broker／Flutter／fixture検証を再実行した。RustとBrokerの試験は権限境界と負例を検証するが、installed productや外部サービスの安全性を証明しない。C30のAgent probeはCodex CLIのversion/help interfaceだけを観測し、実task、credential、workspace書込、複数Agent比較、handoffを実行しない。

失敗注入はC29でRuntime停止相当、Broker停止、MCP／A2A timeout、Credential unavailable、書込不能simulation、Audit失敗、malformed stateを確認した。これはlocalhost fixture、temporary store、実Brokerの開発検証であり、外部Runtime／MCP／A2Aや物理ディスク容量枯渇の証拠へ昇格しない。C28の8時間実測は未成立のままである。

残存分類: Windows installed-pathのprovenance／first-run／Setup Doctor／Broker／Audit anchor外部改変証拠、外部Runtime／Agent、実端末、正式署名、owner GO、正式releaseは`release_blocker`。MobileのMCP live一覧をDesktop owner管理面へ限定すること、およびfixtureだけのprojectionは`known_limitation`として扱う。

## 権限規則

- sensitive action は default deny とする。
- Adapter の `metadata` は信頼しない。
- UI input は決して権限ではない。
- memory、cache、previous state は、それ自体では決して権限ではない。
- Runtime の Permission を暗黙に拡大してはならない。
- 全文表示には `content_visibility=full` が必要である。
- `full_payload` は保存領域に存在し得るが、`content_visibility=full` でない限り UI projection に露出してはならない。

## 機密性の高い操作面

次の操作面では、Capability、Permission、Approval、Audit、Recovery を明示的に扱う必要がある。

- filesystem へのアクセス
- process の実行／制御
- network へのアクセス
- credential へのアクセス
- IPC 通信
- update の検証
- Runtime Adapter の action
- Approval payload の編集
- Audit の export / inspection

## セキュリティ問題の報告

Issue、log、screenshot、audit example には、secret、token、private key、未加工の Approval payload、または hidden content の全文を記載してはならない。

脆弱性の非公開報告には、このリポジトリの GitHub Security Advisories を優先して使用する。GitHub が非公開 Advisory の経路を提供せず、公開 Issue が必要な場合は、redacted reproduction だけを記載し、private evidence、secret、token、未加工の Approval payload、または hidden content の全文を添付してはならない。

報告には、影響を受ける境界、期待される invariant、観測された挙動、redacted data を使った再現手順、および実行した検証コマンドを含める。
