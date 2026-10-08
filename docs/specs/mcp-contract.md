# MCP外部概念射影契約

## 対象

この契約は、MCP Serverから観測した外部概念をGUI-Shellの境界付きmetadataへ射影する。C8では実Serverへの接続、Tool実行、Credential取得、Permission付与を行わない。

射影対象は次のとおりである。

```text
Server
Tool
Resource
Prompt
Transport
Credential ref
Trust
Capability diff
```

## 信頼境界

Server metadata、Tool description、Resource URI、Prompt description、Transport情報、Credential ref、Trust状態、Capability diffは、観測または構成の説明であり、Authorityではない。

```text
MCP metadata ≠ Authority
Tool description ≠ Permission
Trust ≠ Approval
Capability diff ≠ Permission grant
Credential ref ≠ Credential value
```

契約の`権限生成`は常に`なし`である。Permission、Approval、Audit identity、secret value、実Credential値を入力へ含めず、公開範囲は`metadata_only`に固定する。`Credential ref`は保管庫のID、用途、接続対象、必要性、状態を持ち、MCP stdio限定で環境変数名を追加できる。環境変数名は接続先processへの値の渡し先を指定するmetadataであり、Credentialの許可やTrustを意味しない。

## Transportと証拠

Transportは`stdio`、`streamable_http`、`oauth`の分類と、対応状態、接続先hash、理由を持つ。接続先の実値、authorization header、OAuth token、process argvはこの契約へ保存しない。

Serverの`origin`と各`証拠種別`は、観測範囲を表す。`server_metadata`や`LIVE_RUNTIME`が`verified`のTrust状態を自動生成したり、Capability diffの追加を自動承認したりしてはならない。追加・変更Capabilityは`requires_operator_review=true`でなければならない。

## Tool inputSchema検証

MCP Toolの`inputSchema`はJSON Schema Draft 2020-12としてmeta-schema検証し、Broker内でvalidatorを構築できた場合だけcatalogへ受け入れる。`$schema`省略時もDraft 2020-12として扱い、明示dialectは既知の2020-12 URI表記だけを受け付ける。`$ref`と`$dynamicRef`は同一文書内fragment参照だけを許可し、HTTP／fileを含む外部参照は取得しない。入力Schemaは128 KiB、4096 JSON node、深さ64を上限とし、超過・不正・未対応dialectは接続をfail-closedにする。

Schema検証済みの`status=supported`はToolの実行可能性、Permission、Approvalを意味しない。操作者向けTool実行はRust Desktopのnative Owner確認、現在Catalog再照合、一回限りPermission、Auditを経た独立操作だけであり、Agentへの結果引渡しやTool結果本文の公開はしない。

## Tool引数の検査と呼出し

`McpCatalog.validate_tool_identity_and_call`は、現在catalog内のTool IDと名前の一致を確認し、引数がobjectであること、JSON化後32 KiB以下・2048 node以下・深さ32以下であること、大小文字を正本化してPermission／Approval／Authority／Credential／secret fieldを再帰的に含まないことを検査する。その後、catalogに保持した当該Toolの`inputSchema`をJSON Schema Draft 2020-12として適用し、適合しない引数を固定error codeで拒否する。外部参照はSchema受入時点で禁止し、引数内容やSchemaのvalidation error詳細をAuditへ出さない。

現行Rust実装のTool IDは`tool-`に続く142桁の小文字16進数である。呼出し要求はこの形だけを受け付け、Brokerが現在Catalog内のIDと名前の完全一致を再検査する。

Rust Brokerは、Windows Desktopまたは`macos-mcp-center.md`に定めるMac Desktopのdefault No native Owner確認を通過した`MCP Tool実行`だけを受け付ける。通常IPC、Owner資格だけの要求、Agent／LLM要求から直接実行しない。確認対象は現在接続中のServer ID、Catalog内のTool ID／名前、引数objectの件数とhash、Broker要求hashである。引数本文はDesktop画面で操作者が確認し、native確認では表示しない。Tool危険度は`unknown`として毎回確認する。

接続時のCredential利用は、Windows stdioに限定した別のOwner確認操作である。Flutterは検証付きnormal IPC一覧から、用途`mcp_transport`・対象Server ID・有効状態が一致するmetadataだけを選び、Credential IDと環境変数名を送る。Brokerは登録AuditとProtectedStoreのhashを照合し、target・purpose・状態の完全一致を再検証した後でのみ秘密値を読み出す。値はRustから対象childの指定環境変数へ渡し、Broker response、snapshot、Audit内容、error、log、traceへ出さない。許可環境変数名は予約OS名・`GUI_SHELL_*`と衝突してはならない。別Serverへの再利用、無効・欠落・改変Credential、環境変数名不正、永続Audit不在はprocess起動前にfail-closedとする。使用AuditにはID、target、変数名、時刻だけを記録する。対象Serverと子孫は秘密値を読取り・送信でき、Windows Job Objectはsandboxを提供しない。legacy protocol fallbackは同じ実行対象へCredentialを再度渡す可能性をOwner確認に表示する。

Brokerは、受信payloadのraw hashを保持し、未知field、接続／Toolの識別不一致、再帰的なAuthority／Credential field、上限超過、現在Catalogの`inputSchema`不適合を拒否する。`McpCatalog.validate_tool_identity_and_call`を送信直前に呼び、現在CatalogのTool ID／名前・Schemaを再照合する。metadata、履歴、Permission ID、Approval IDを要求から受け取らない。native Owner確認に結び付くPermissionは呼出し対象Server／Tool／引数hashだけの一回限りで、Broker内で生成・即時消費し再利用しない。Tool-call payloadや`tools/call`引数へCredential実値を含めない。Windows stdio childの接続時注入は前述の独立したCredential統治経路に限る。

送信は現接続のMCP stdio childに`tools/call`を一度だけ行う。response ID、JSON-RPC形状、結果型、結果上限を検査する。呼出し後のtimeout、応答ID不一致、malformed／unsupported result、結果Audit確定失敗は結果不明として接続を隔離し、自動再送しない。Ownerが結果を照合した後、明示切断・再接続するまで次のTool呼出しを拒否する。

Tool出力本文は、text・image・audio・resource link・embedded resource・structured contentのいずれもBrokerが保存・返却せず、snapshot、Audit reason、error、log、traceへ複写しない。Receiptは結果hash、`isError`相当の成否、content件数／型だけを`hash_only`で返す。したがって、この経路は操作者の一回限りTool実行と実行結果の存在確認までであり、結果本文をAgentへ渡す経路ではない。結果の安全な取得・redaction・Content Exposure承認が成立するまで、AgentによるMCP Tool利用と結果本文表示は未対応の`release_blocker`とする。

## 実装範囲

C8の作業単位はSchema、正常／負例fixture、Conformanceを追加した。C9はstdio discovery、connect、metadata list、Windows owner切断に加え、native Owner確認・一回Permission・Catalog preflight・stdio `tools/call`・hash-only result receiptをRust Broker経路へ接続する。Windows stdioへの限定Credential注入は別Owner確認・保管庫照合・Auditを経て接続する。Tool結果本文をAgentへ渡すContent Exposure経路、MCP以外のCredential注入、Resource／Prompt本文取得、Streamable HTTP、OAuth、外部MCP実物Test Harness、非Windows process群監督は未接続であり、`release_blocker`として保持する。

P13のMac追加範囲は`macos-mcp-center.md`を責任正本とする。Credentialなしの接続／Tool／明示切断とD4所有process group停止だけを追加し、Windowsの全子孫・異常終了保証へ昇格しない。上記C9当時の非Windows未接続は履歴であり、Macの現在範囲は有限Acceptanceの証拠で判定する。Mac Credential注入、Agent引渡し、別group／crash保証はこの単位では未成立のまま保持する。
