# 更新センター

状態: C11 現行単位実装済み（実行系はsuspended）

更新センターは、更新候補の表示、Broker所有信頼設定によるEd25519署名検査、更新適用の要求、延期、rollback要求を扱う。更新候補自身の公開鍵、MCP metadata、Profile、履歴、UI stateは信頼源ではない。

## Broker経路

通常認証済みBroker IPCから`更新一覧`、`更新署名検査`、`更新確認`、`更新延期`、`更新download要求`、`更新適用要求`、`更新rollback要求`をRust Update Centerへ送る。`更新確認`は候補metadataから決定的な署名対象byteを再構成し、Broker所有の`update_trust.json`のEd25519公開鍵とfingerprintで検査する。検査済み候補だけを`updates.json`へatomic writeする。

信頼設定がない、署名対象byteが一致しない、署名者fingerprintが不一致、署名が不正な候補は、利用可能な更新として保存しない。更新一覧の`署名信頼設定`は`configured`または`unconfigured`を返し、取得不能を0へ置換しない。

## 実行境界

download、install、process起動、rollbackの外部実行経路は本単位では追加していない。`更新download要求`、`更新適用要求`、`更新rollback要求`は、署名検査済み候補とhashを再照合したうえで、Audit付き`suspended` receiptを返す。これによりUIの要求操作と製品の実行完了を混同しない。

Flutterは一覧表示と要求送信だけを担当し、filesystem、process、network、credential、privileged IPCを直接扱わない。

## 検証対象

- UpdateCandidate、UpdateReceipt、UpdateListのSchemaとnegative fixture
- Broker所有trustによるEd25519検証、署名対象の正本byte一致
- 信頼設定未構成、署名不正、候補hash不一致、未知field、malformed stateのfail-closed
- 更新候補の永続化・再読込、延期のAudit、実行要求のsuspended
- Desktop設定画面の更新一覧と要求操作

実ダウンロード、インストール、rollback適用、Windows installed productでの更新実証、owner固定公開鍵の本番プロビジョニングは未成立であり、正式releaseの`release_blocker`として保持する。
