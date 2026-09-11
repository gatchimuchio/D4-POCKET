# オフライン署名済み監査チェックポイント

この経路はbuild / release専用であり、通常brokerから到達しない。秘密鍵を読み込むAPIを持たない。Ed25519公開鍵32byteのSHA-256をfingerprintとし、Repositoryの `config/audit_signing_trust.json` にownerが固定する。未設定なら失敗する。

署名入力は固定順序JSONのUTF-8 byte、空白・BOM・末尾改行なし。field順はformat, version, audit_chain_head, audit_log_sha256, audit_anchor_sha256, source_commit, installed_artifact_sha256, generated_at, sequence, previous_checkpoint_hash。formatはgui-shell-audit-checkpoint、versionは1、時刻はUNIX秒、hashは小文字sha256:、source commitは40桁hex。parse後の再serializeが元byteと完全一致しない入力を拒否する。未知field・重複fieldも拒否する。

署名済みcheckpoint hashはcanonical byteと64byte detached signatureの連結のSHA-256。固定型の境界なので曖昧性はない。証拠束はcheckpoint.json、signature.bin、public-key.derを含む。公開鍵はRFC8410 Ed25519 SubjectPublicKeyInfo DERの固定44byteを受理する。任意署名者は受理しない。

継続性はowner管理の公開記録 `trusted-head.json`（version, sequence, signed_checkpoint_hash）を別に入力する。初回はsequence=0 / hash=null。以後、既に受理した最新署名済みhashを保管する。初回以外は直前checkpointと署名を検証し、candidate.previous_checkpoint_hashと一致させる。最新記録と同一sequenceなら署名済みhashも完全一致を必須とする。新規candidateは最新記録+1とそのhashを参照する。欠番・後退・同一番号の別署名は拒否する。

この公開継続性記録は秘密ではないが、検証対象と共に巻き戻してはならない。ownerが外部媒体上で維持し、信頼済み検証環境へ渡す。全履歴と信頼記録を一緒に巻き戻したlocal状態だけから最新性は判定できない。署名成功時に受理記録を自動更新せず、ownerが検証結果のsequence/hashを外部媒体へ保存する。

実測は停止したbroker storeを対象とする。audit.jsonlの全eventを既存BrokerAuditLogで再検証し、anchorのhead/countを照合する。署名で両fileのbyteを結合し、現在のGit source commitと実installed exe hashも照合する。採取前後のbyteが変化したら失敗し、稼働中の不整合を成功にしない。秘密のaudit_anchor.keyは読まない。

CollectorはRepository固定公開鍵fingerprint、ownerの継続性記録、現在のsource、installed exe、storeをRust verifierへ渡す。release検証器も同じ実検証を再実行し、過去のverification resultやverified=trueを信頼しない。署名証拠だけを作った状態では合格しない。

主張範囲は、通常稼働中の同一ユーザーまたはGUI-Shell processが、オフライン秘密鍵なしに改変監査を正規証拠として再署名できないこと。ownerの意図的再署名、媒体の物理侵害、検証器・公開鍵固定点・外部継続性記録そのものを置換できる攻撃はこの証拠で防げると主張しない。administrator_root_resistance_claimed=false。

- item: 実owner鍵と継続性記録
  classification: release_blocker
  reason: 実運用公開鍵が未固定であり、owner署名は未取得。
  required_action: ownerが外部媒体で鍵生成・署名し、公開鍵と署名証拠のみを渡す。
  blocks_release: yes
