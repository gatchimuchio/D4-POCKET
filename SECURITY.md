# GUI Shell セキュリティ

## セキュリティ姿勢

GUI Shell は control plane である。セキュリティ判断は Flutter widget ではなく、schema、conformance、Shell Core、Adapter Contract、および範囲を限定した Native Helper の各面で行わなければならない。

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
