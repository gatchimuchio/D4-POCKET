# GUI Shell Windows書出しの意味正本

## 目的

GUI Shell Windows書出しは、D4 Pocketの構成Manifestから独立Appの初期Manifestを生成する。現行単位は、書出し先の新規App identity、新規監査store、設定、Runtime／Adapter構成、Capability requirement、配布metadataをBrokerで生成するManifest-only経路である。

## Broker経路

`GUI Shell書出し`要求をRust Brokerで再検証し、`INTERNAL_STATE`のAuditEventと書出しReceiptを返す。対象platformはWindows、出力modeは`manifest_only`に固定する。任意画面の選択はGUI入力にすぎず、Brokerが機械可読なModule一覧を照合し、必須Moduleと依存閉包を再計算する。現行のproduction pathはManifestを応答Receiptとして返すだけであり、ユーザーfileや配布artifactは作らない。Brokerの永続Auditは記録する。

Desktopの`BrokerClient`は通常資格だけを利用する。Ownerが設定画面で書出しを開始すると、Rust起動器が要求の構造、`desktop_flutter` metadata、Broker session、現在時刻、payload hash、Compose Manifest、Module計画を確認し、hashを含むWindowsネイティブ確認を表示する。表示値は検証済みManifestから射影し、任意payloadやCredentialは表示しない。拒否／未確認は通常Broker経路へ戻してOwner不足として監査し、許可だけが容量1のprocess内Rust channelを通ってBroker所有threadへ届く。Brokerは同じ要求のsession、鮮度、nonce/replay、payload hash、Authority metadataを再検証し、`GUI Shell書出し`以外をこの内部経路で処理しない。

Owner資格、privileged IPC、Owner role flagをFlutterへ渡さず、Rust起動器は資格fileも生成しない。確認はWindows session上の明示的な人間操作記録であり、Windows accountの再認証・本人性証明ではない。確認要求はBrokerの300秒鮮度期限を超えると失効する。ネイティブ確認の応答待ちは通常IPCより長いが有限のtransport期限を持つ。

## 継承禁止

書出し元のAuthority、Permission、Approval、Credential、Audit chainは継承しない。書出し先は新規App identityと新規監査storeを持ち、`authority_strip=true`、各`*_inherited=false`、`inheritance_policy`の各値`none`を返す。Capability requirementは必要機能の説明であり、Permissionではない。

## 未成立範囲

`build_status=not_started`、`artifact_status=not_built`、installer未開始、署名なしを固定する。Module計画の`binary_pruning_status=not_applied`も固定し、Manifest上の除外をExport artifactからの削除へ読み替えない。Owner経路の自動試験は通常資格拒否、模擬Yes／No、hash・session・Authority metadata・期限の負例、永続Broker Auditを確認する。実Desktop上での対話的なWindows確認表示・クリックは別のLIVE_RUNTIME確認が必要である。Developer専用Flutter UIのbaseline／選択build比較とAOT report上のsurface library node有無は確認済みだが、独立Export artifactの生成・pruning、安全Core保持、製品起動・資源比較、Installer、署名、配布、rollback、実際のfilesystem書込みは未成立の`release_blocker`として保持する。Manifest ReceiptやDeveloper UI buildだけで独立App完成や製品releaseを主張しない。
