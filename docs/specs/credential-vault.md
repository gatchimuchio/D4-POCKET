# 資格情報保管庫

## 対象

資格情報保管庫は、Runtime、Tool、MCP、A2Aなどの接続に使う秘密値を、D4 Pocket / GUI-ShellのBrokerが管理するための境界である。資格情報の存在や参照はPermissionを生成せず、資格情報ID、接続対象、種類、保管方式だけを通常画面へ投影する。

この契約で接続済みの範囲は、owner control経路からの新規追加、通常IPCからの安全なmetadata一覧、Windows MCP stdio接続への限定利用、およびnative Owner確認付き論理失効である。Runtime一般、Agent、Tool、A2Aへの注入、更新、暗号文の物理削除、接続先変更、物理削除Recoveryは未接続であり、未完成のまま保管庫完成とは扱わない。

P13のMac登録・Keychain保管・論理失効の現在範囲は`macos-credential-vault.md`のCLOSED受入れを正本とする。本書のC9当時のWindows限定・Keychain未接続は履歴である。Mac MCPへの追加consumerは`macos-mcp-credential-binding.md`（IMPLEMENTING）へ限定し、公開ID参照・別個native確認・現在recordの再照合から対象stdio環境一つへ結合する。Provider／Agent／A2A注入や正式配布へ成立を拡大しない。

## 登録境界

新規追加は次の経路に限る。

```text
owner資格
  ↓
Brokerの資格情報登録操作
  ↓
永続Audit受信
  ↓
Windows ProtectedStore / DPAPI (Purpose::Credential)
  ↓
公開metadataと暗号文hashだけをAuditへ確定
```

normal IPC、Flutter、Adapter metadata、Profile、History、MCP metadata、A2A Agent Cardから資格情報を追加してはならない。登録payloadのCapability、Permission、Approval、Authority、Audit identityをcallerから受け付けず、Brokerがowner channel、永続Audit、登録済み保管先を独立に確認する。

同じ資格情報IDの再登録は拒否する。暗号文は`create_new`で作成し、既存fileを上書きしない。作成後のAudit確定に失敗した場合は新規暗号文を削除して成功へ昇格させず、削除も失敗した場合は保管復旧が必要な状態として停止する。

## 内容露出

秘密値はBroker response、通常IPC、Flutter snapshot、Audit reason、error、log、trace、test artifactへ投影しない。通常一覧の証拠源はBrokerの検証済み内部Auditと実保管fileのmetadata照合であり、file欠落・改変・link・共有競合は部分一覧へ変換せず拒否する。

公開projectionの`公開範囲`は`metadata_only`に固定し、公開可能な項目は次に限定する。

```text
資格情報ID
用途
接続対象
種類
保管方式
状態
作成時刻
最終使用時刻(null可)
失効時刻(null可)
暗号文hash
作成監査ID
公開範囲
証拠種別
```

暗号文hashは整合照合用のhash_only情報であり、権限、復号鍵、接続許可、Approvalを意味しない。

## 実装済み範囲と未接続範囲

実装済み範囲は、Windows DPAPIへ秘密値を暗号化して保存するowner登録、秘密値を含まない公開receipt、通常IPCの検証付き一覧、normal channelからのowner操作拒否、保管先未登録・永続Audit未使用・秘密値混入をfail-closedで拒否する経路である。

資格情報の論理失効は、Rust Desktopのnative Owner確認を通過した単一Credential IDだけに許可する。Brokerは確認後に永続Auditを再検証し、登録receiptのID・作成監査ID・暗号文hashへ結び付いた失効eventを追加する。資格情報状態は登録・失効・MCP使用Auditを時系列に検証して復元し、失効後のCredential使用event、重複失効、孤立した失効receiptはAudit不整合としてfail-closedにする。失効後はMCP注入を拒否し、一覧には`状態=失効`と失効時刻をmetadata-onlyで表示する。暗号文fileはこの操作では削除せず保持する。失効は取消できず、再利用には別IDの再登録が必要である。暗号文の物理削除とそのRecoveryは別操作であり未接続。

C9のMCP利用はWindows stdioだけに限定する。Flutterは通常IPCからmetadata-only一覧を取得し、用途`mcp_transport`・接続対象Server ID・有効状態が一致するCredential IDを選ぶ。接続要求とdefault No native Owner確認へ渡すのはCredential ID、用途、対象、環境変数名だけであり、秘密値は渡さない。Brokerは永続Audit、現在の登録Audit、ID、用途、対象、状態、保管file hashを照合してからDPAPIで読み出し、Rust内の短命値を対象stdio childの指定環境変数へ渡す。OS必須環境変数や`GUI_SHELL_*`は上書きできない。Credential使用は別AuditEventへID、対象、環境変数名、時刻だけを確定し、一覧の最終使用時刻をその検証済みeventから導出する。

native確認では相手Server processとその子孫が値を読取り・外部送信できること、Job Objectはprocess寿命管理であってsandboxではないことを明示する。MCP stdio Serverは未信頼実行物であり、この機能はCredentialの安全な委譲先であることを証明しない。protocol version negotiationで既定のlegacy fallbackが必要な場合、同じ実行file／引数へCredentialを渡したchildを再起動する可能性も確認画面に示す。Credential実値はBroker response、通常IPC、Flutter、Audit reason、error、log、traceへ複写しない。

P6のCodex CLI Provider結合は、`provider_model_selection.authentication_source=broker_credential_vault`とmetadata上のCredential IDだけを登録面で受け取る。Brokerは実行系へ固定された`openai_codex_cli` ProviderとIDを取得し、Agent Taskまたは対話の現在要求hash、Permission／Owner Approvalの有効性、資格情報の用途`provider_api_key`・接続対象・種類・失効状態、永続Auditを照合する。許可された要求について使用Auditを確定した後だけDPAPIから短命復号し、zeroizing bufferでCodex Adapterへ渡す。拒否・失効・Audit障害は実行前にfail-closedとする。CLI起動時は`env_clear`後の固定environment allowlistに加え、実Provider要求に限って`OPENAI_API_KEY`をCodex CLI親processだけへ設定する。Codexの`shell_environment_policy`から`OPENAI_API_KEY`、`CODEX_API_KEY`、`CODEX_ACCESS_TOKEN`を除外し、tool childへの継承を拒否する。登録要求、Owner確認表示、Agent metadata、Task record、Audit、通常Broker responseへ秘密値を複写しない。Codex CLI管理認証modeではD4保管庫を読まず、両mode間のfallbackはしない。この結合は通常Releaseの`task_execution=unsupported`を変更しない。

Product Buildでは合成Credentialを偽Codex CLIへ渡すfixtureで、親process環境への受渡し、引数・結果への非露出、tool shell除外設定を検査する。Broker単体ではcurrent Permission／Approvalの検証後だけcredential resolverへ進むことを検査する。実Codex CLI 0.160.0を合成loopback Responses APIへ接続するWindows `LIVE_RUNTIME`試験は、API request前にMxCの`CreateProcessSecurityEnvironment`／HRESULT `0x80070003`で停止したため、実CLIでの認証・tool child除外・Provider接続は未確認である。Provider healthとmodel availabilityは`unknown`のままとする。この環境制約はProduct Buildのfixture成立を取り消さず、実CLI経路はFinal QA／release evidenceで確認する。

未接続範囲は、Codex CLI Provider以外のRuntime／Tool／A2A使用、GUI上でのCredential登録・更新・物理削除・接続先変更・物理削除Recovery操作、Windows installed productでの登録／注入／失効証拠、macOS/iOS KeychainおよびAndroid Keystoreである。これらは`release_blocker`として残し、限定Codex CLI結合、登録・論理失効またはMCP用への注入成功を製品完成やrelease readinessへ昇格させない。
