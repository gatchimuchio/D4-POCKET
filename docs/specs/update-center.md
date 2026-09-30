# 更新センター

状態: C11 現行単位実装済み（実行系はsuspended）

更新センターは、更新候補の表示、Broker所有信頼設定によるEd25519署名検査、更新適用の要求、延期、rollback要求を扱う。更新候補自身の公開鍵、MCP metadata、Profile、履歴、UI stateは信頼源ではない。

## Broker経路

通常認証済みBroker IPCから`更新一覧`、`更新署名検査`、`更新確認`、`更新延期`、`更新download要求`、`更新適用要求`、`更新rollback要求`をRust Update Centerへ送る。新規候補はContract版2とし、候補metadataに配布package全体の小文字hex SHA-256と正確なbyte長（1〜4 GiB）を必須化する。`更新確認`は候補metadataから決定的な署名対象byteを再構成し、package SHA-256・byte長を含む同じbyte列をBroker所有`update_trust.json`のEd25519公開鍵とfingerprintで検査する。署名対象byteと候補fieldが一致した候補だけを`updates.json`へatomic writeする。package位置や実行commandを候補から受け取らない。

信頼設定がない、署名対象byteが一致しない、署名者fingerprintが不一致、署名が不正な候補は、利用可能な更新として保存しない。更新一覧の`署名信頼設定`は`configured`または`unconfigured`を返し、取得不能を0へ置換しない。

版1の既存保存recordは起動時に保持するが、一覧projectionでは`legacy_unbound`へ降格する。これは旧署名のmetadata検証をpackage真正性と誤認させないためであり、旧recordを新候補として再受理せず、download／適用／rollback要求にも使わせない。新規候補の署名がpackageのdigestとsizeへ結合していることだけを保証し、実際に取得したfileが一致することは、今後download consumerがbyte長・SHA-256を照合して初めて確認できる。

永続recordの`verified`は保存時の結果であり、現在も有効なtrustや実行権限を意味しない。一覧・延期receiptではBrokerが現在保持するtrustでcandidateの署名を再検証し、candidate全体から再計算したhashもrecordと一致するときだけ`verified`を射影する。trust未設定・鍵変更・署名不一致・候補内容または保存hashの不整合は`verification_stale`へ降格する。download／適用／rollback要求の直前にも同じ署名・hash再検証を行い、一致しなければ拒否して実行しない。過去の`verified`記録、履歴、候補hashの呼出元提示は再検証を代替しない。

## 実行境界

download、install、process起動、rollbackの外部実行経路は本単位では追加していない。`更新download要求`、`更新適用要求`、`更新rollback要求`は、署名検査済み候補とhashを再照合したうえで、Audit付き`suspended` receiptを返す。これによりUIの要求操作と製品の実行完了を混同しない。

Flutterは一覧表示と要求送信だけを担当し、filesystem、process、network、credential、privileged IPCを直接扱わない。

## 検証対象

- UpdateCandidate版2、UpdateReceipt、UpdateListのSchemaとnegative fixture。版1の保存済みrecordは未結合表示へ降格し、版2でも現在trustとの再照合ができないrecordは`verification_stale`へ降格する。
- Broker所有trustによるEd25519検証、署名対象の正本byte一致、package全体のSHA-256・正確なbyte長への署名結合
- 信頼設定未構成、署名不正、現在trust変更、永続候補content／hash改変、未知field、malformed stateのfail-closed
- 更新候補の永続化・再読込、延期のAudit、実行要求のsuspended
- Desktop設定画面の更新一覧と要求操作

実ダウンロード後のfile照合、インストール、rollback適用、Windows installed productでの更新実証、owner固定公開鍵の本番provisioningは未成立であり、正式releaseの`release_blocker`として保持する。metadata署名検査の成功だけでpackage downloadや実行の真正性を主張しない。
