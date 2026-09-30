# 更新センター

状態: C12 download実行経路をBroker/native Owner確認とWindows期限付きDNS解決へ接続（install／process／rollbackはsuspended）

更新センターは、更新候補の表示、Broker所有信頼設定によるEd25519署名検査、更新適用の要求、延期、rollback要求を扱う。更新候補自身の公開鍵、MCP metadata、Profile、履歴、UI stateは信頼源ではない。

## Broker所有の配布元設定

`update_trust.json`版1は後方互換の署名検査専用形式として読み取る。版2は署名公開鍵・fingerprintに加えて、Broker所有の`package_sources`を持てる。各要素は`channel`と`base_url`だけであり、channelは`stable`、`beta`、`nightly`のいずれか、登録数は最大3、同じchannelを複数登録しない。この一意性はJSON SchemaとBroker起動時検証の両方で強制する。新規初期設定は未構成の版2で、公開鍵未設定時は配布元も空でなければならない。版1に配布元fieldを追加することはできない。

`base_url`は`https://`、小文字ASCIIの複数label DNS host、明示portなし、ASCII unreserved path segmentからなる固定形式だけを受け入れる。userinfo、query、fragment、backslash、percent-encoding、IP literal、`localhost`、空／末尾slash／`.`／`..` path segment、大文字hostは拒否する。設定は重複JSON fieldを拒否して8 KiB以内で読み、Broker起動時に再検証する。

取得先は、現在のBroker trustで署名・候補hashを再検証できた版2候補に限り、署名済み`channel`と一致するBroker所有sourceを一つ選んで`base_url + "/" + update_id + ".pkg"`から決定する。Brokerは更新一覧の各候補へ`取得元`を射影し、配布元がある場合だけ`状態=configured`と導出URLを返す。該当配布元がない場合は`unconfigured`、候補が旧版・未検証・trust不整合の場合は`ineligible`とし、いずれもURLを返さない。Flutterまたは候補からURL／pathを受け取らず、署名済みchannelは配布元選択値に限りAuthorityを与えない。この一覧情報は`INTERNAL_STATE`の表示専用であり、Capability、Permission、Approval、ネットワーク作用を生成しない。

## Broker経路

通常認証済みBroker IPCから`更新一覧`、`更新署名検査`、`更新確認`、`更新延期`、`更新download要求`、`更新適用要求`、`更新rollback要求`をRust Update Centerへ送る。新規候補はContract版2とし、候補metadataに配布package全体の小文字hex SHA-256と正確なbyte長（1〜4 GiB）を必須化する。`更新確認`は候補metadataから決定的な署名対象byteを再構成し、package SHA-256・byte長を含む同じbyte列をBroker所有`update_trust.json`のEd25519公開鍵とfingerprintで検査する。署名対象byteと候補fieldが一致した候補だけを`updates.json`へatomic writeする。package位置や実行commandを候補から受け取らない。

信頼設定がない、署名対象byteが一致しない、署名者fingerprintが不一致、署名が不正な候補は、利用可能な更新として保存しない。更新一覧の`署名信頼設定`は`configured`または`unconfigured`を返し、取得不能を0へ置換しない。

版1の既存保存recordは起動時に保持するが、一覧projectionでは`legacy_unbound`へ降格する。これは旧署名のmetadata検証をpackage真正性と誤認させないためであり、旧recordを新候補として再受理せず、download／適用／rollback要求にも使わせない。版2候補のpackage署名bindingだけで取得fileを信頼せず、Broker workerが実byte長とSHA-256を照合した後に限り`downloaded`へ遷移する。

永続recordの`verified`は保存時の結果であり、現在も有効なtrustや実行権限を意味しない。一覧・延期receiptではBrokerが現在保持するtrustでcandidateの署名を再検証し、candidate全体から再計算したhashもrecordと一致するときだけ`verified`を射影する。trust未設定・鍵変更・署名不一致・候補内容または保存hashの不整合は`verification_stale`へ降格する。download／適用／rollback要求の直前にも同じ署名・hash再検証を行い、一致しなければ拒否して実行しない。過去の`verified`記録、履歴、候補hashの呼出元提示は再検証を代替しない。

## download実行境界

`更新download要求`だけを、Rust Desktop起動器のnative Owner確認を経るBroker内部allowlistへ追加した。起動器は通常認証Brokerの現在の`更新一覧`から、更新版、channel、署名済みpackage SHA-256・正確なbyte長、内容概要、導出URLのhostを取得して確認画面に表示する。画面表示用URLをFlutter入力から受け取らない。Owner確認時に得たcandidate hash・source URL・package metadataはprocess内の一回限りcontextとしてBrokerへ渡し、Brokerは実行直前に現在trust・署名・candidate hash・source URL・表示項目をすべて再照合する。不一致や時間切れは拒否する。normal IPC、Owner credentialだけの経路、UI／候補metadataはdownload権限を作れない。

downloadはBrokerのserial IPC loop外の単一worker上で非同期HTTP clientを使って実行し、Brokerは10 msのbounded loopでcompletionを受けてAuditを確定する。接続期限は20秒、HTTP要求全体は最大24時間、bodyの連続読取無通信期限は60秒とし、転送が続く場合に全体60秒で打ち切らない。Flutterは直接network／filesystemを扱わず、既存の`MethodChannel('gui_shell/broker')`を通じてjobを開始し、更新一覧の手動refreshで状態を読む。状態はBroker process内でboundedに保持し、常駐pollingは行わない。Broker再起動後はjob状態が消えるが、再度明示要求した場合に既存packageを固定directory内で全byte再hashして照合する。中断`.part`は次の明示download要求時に、Audit intent／結果を記録してBroker固定directory内だけを削除する。

通信はHTTPSのみ、既定の証明書・hostname検証を有効にし、redirect・system proxy・自動retryを無効化する。WindowsではRust helper内のWindows DNS Client `DnsQueryEx`を使い、A／AAAAを逐次照会する。各照会にはdownload全体期限と最大15秒のDNS期限の早い方を適用し、20 ms間隔でcancel要求を確認する。期限超過またはcancel時は`DnsCancelQuery`を呼び、callback完了まで結果・cancel handle・query contextを保持する。callbackを2秒以内に回収できない、またはcancelに失敗した場合はstatic failure codeで失敗し、未完了照会が残る間は後続照会を拒否する。DNS結果に非global addressが一つでも含まれる場合は全体を拒否し、検査したaddressへ接続先を固定してDNS rebindingを防ぐ。HTTP応答はstatus 200、単一の正確な`Content-Length`、`Transfer-Encoding`なし、`Content-Encoding`なしを要求する。固定64 KiB bufferで実byte数とSHA-256を計算し、署名済みbyte長・digestの両方が一致した場合だけ、capability directory内の`create_new`一時fileをfsyncし、create-only hard linkで`<digest>.pkg`として公開する。失敗・中断では一時fileを除去する。path、応答本文、秘密値はFlutter／Auditへ返さず、Auditには更新ID、候補hash、状態、byte長、static failure codeのみを記録する。

保管は固定Broker directory、同時job一件、完成package一件（最大4 GiB）にboundedとする。別digestのpackageがすでにある場合は上書きせず拒否する。install／process起動／rollbackは未接続のため引き続き`suspended`。download済み状態は権限ではなく、後続installは現在trustを再検証し、保存package全体を再hashしてから別のApproval／Audit／Recovery契約へ進む必要がある。

Flutterは一覧表示と要求送信だけを担当し、filesystem、process、network、credential、privileged IPCを直接扱わない。

## 検証対象

- UpdateCandidate版2、UpdateReceipt、UpdateListのSchemaとnegative fixture。版1の保存済みrecordは未結合表示へ降格し、版2でも現在trustとの再照合ができないrecordは`verification_stale`へ降格する。
- Broker所有trustによるEd25519検証、署名対象の正本byte一致、package全体のSHA-256・正確なbyte長への署名結合
- 更新一覧の取得元projectionが現在trustで検証済みの候補とBroker所有sourceだけから導出され、未設定・未適格候補にURLを返さないこと
- 信頼設定未構成、署名不正、現在trust変更、永続候補content／hash改変、未知field、malformed stateのfail-closed
- 更新候補の永続化・再読込、延期のAudit、実行要求のsuspended
- Windows DNS Clientの実callback、A／AAAA応答、明示cancel、有限deadlineとtimeout後のcancel、およびcancel／期限の事前判定
- Broker所有update trust版1互換、版2の配布元構造・起動時検証、設定読込の上限・重複field拒否
- Desktop設定画面の更新一覧と要求操作

Windows installed productでの実配布元download、製品BrokerからのDNS／TLS失敗注入、破損package repair、保存後tamper検査からinstall／rollbackまでの経路、owner固定公開鍵の本番provisioningは未成立であり、正式releaseの`release_blocker`として保持する。DNS API試験はWindows localの制御loopback fixtureを使用し、installed productや実配布元での挙動を証明しない。Linux／macOS pathは現状`ToSocketAddrs`を使う。この期限付き取消resolverとの機能差は非Windows技術工程R15で扱う`post_v1_scope`であり、Windows 1.0の完了主張へ含めない。metadata署名検査やlocal TLS fixtureの成功だけで実配布元の安全性、installed productでの更新成功、release readinessを主張しない。system proxy必須環境は未対応の`known_limitation`であり、直接HTTPS接続が許可されない環境ではdownloadは失敗する。
