# 更新センター

状態: C11 現行単位実装済み（配布先はBroker導出表示、実行系はsuspended）

更新センターは、更新候補の表示、Broker所有信頼設定によるEd25519署名検査、更新適用の要求、延期、rollback要求を扱う。更新候補自身の公開鍵、MCP metadata、Profile、履歴、UI stateは信頼源ではない。

## Broker所有の配布元設定

`update_trust.json`版1は後方互換の署名検査専用形式として読み取る。版2は署名公開鍵・fingerprintに加えて、Broker所有の`package_sources`を持てる。各要素は`channel`と`base_url`だけであり、channelは`stable`、`beta`、`nightly`のいずれか、登録数は最大3、同じchannelを複数登録しない。この一意性はJSON SchemaとBroker起動時検証の両方で強制する。新規初期設定は未構成の版2で、公開鍵未設定時は配布元も空でなければならない。版1に配布元fieldを追加することはできない。

`base_url`は`https://`、小文字ASCIIの複数label DNS host、明示portなし、ASCII unreserved path segmentからなる固定形式だけを受け入れる。userinfo、query、fragment、backslash、percent-encoding、IP literal、`localhost`、空／末尾slash／`.`／`..` path segment、大文字hostは拒否する。設定は重複JSON fieldを拒否して8 KiB以内で読み、Broker起動時に再検証する。

取得先は、現在のBroker trustで署名・候補hashを再検証できた版2候補に限り、署名済み`channel`と一致するBroker所有sourceを一つ選んで`base_url + "/" + update_id + ".pkg"`から決定する。Brokerは更新一覧の各候補へ`取得元`を射影し、配布元がある場合だけ`状態=configured`と導出URLを返す。該当配布元がない場合は`unconfigured`、候補が旧版・未検証・trust不整合の場合は`ineligible`とし、いずれもURLを返さない。Flutterまたは候補からURL／pathを受け取らず、署名済みchannelは配布元選択値に限りAuthorityを与えない。この一覧情報は`INTERNAL_STATE`の表示専用であり、Capability、Permission、Approval、ネットワーク作用、download可能状態を生成しない。redirect、DNS解決先、TLS接続、取得byte照合、展開、同一volume install／rollbackは未実装である。download／install／rollbackは引き続き`suspended`であり、版2設定や配布先表示だけで実行可能にはならない。

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
- 更新一覧の取得元projectionが現在trustで検証済みの候補とBroker所有sourceだけから導出され、未設定・未適格候補にURLを返さないこと
- 信頼設定未構成、署名不正、現在trust変更、永続候補content／hash改変、未知field、malformed stateのfail-closed
- 更新候補の永続化・再読込、延期のAudit、実行要求のsuspended
- Broker所有update trust版1互換、版2の配布元構造・起動時検証、設定読込の上限・重複field拒否
- Desktop設定画面の更新一覧と要求操作

実ダウンロード後のfile照合、インストール、rollback適用、Windows installed productでの更新実証、owner固定公開鍵の本番provisioningは未成立であり、正式releaseの`release_blocker`として保持する。metadata署名検査の成功だけでpackage downloadや実行の真正性を主張しない。
