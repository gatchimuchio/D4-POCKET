# MCP接続センター

P13のMac製品接続は`macos-mcp-center.md`のCLOSED受入れを参照する。追加の公開資格情報ID選択・Keychainからの対象stdio結合は`macos-mcp-credential-binding.md`（IMPLEMENTING）の範囲に限定する。以下のC9 Windows経路は維持し、Macのprocess停止保証をWindows Job Objectと同一視しない。

本書の外部wire仕様固定点は、公式MCP仕様2026-07-28版の[基本protocol](https://modelcontextprotocol.io/specification/2026-07-28/basic)と[Tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)である。外部仕様はwire形式の根拠に限り、Authority、Permission、Approval、TrustはGUI-Shellの正本だけが定める。

## 対象

C9は、owner controlから明示されたMCP stdio ServerをRust Brokerのprocess境界へ接続し、MCPのdiscoveryとTool／Resource／Prompt一覧を検証済みmetadataへ射影する。MCPの外部metadataはC8と同じくAuthority、Permission、Approval、Trustを生成しない。

現行の接続対象は`stdio`だけである。現行`server/discover`を先に試し、各新protocol要求の`_meta`へ版、client情報、機能を宣言しない空の`clientCapabilities`を付ける。

## Tool引数の検査と呼出し

操作者向けTool呼出しはRust Desktop native Owner確認だけがApprovalとなる。OwnerがDesktop画面上のJSON arguments全文を確認した後、Rust起動器はServer ID、Tool ID／名前、top-level field数、arguments hash、request hashだけをdefault No確認へ示す。通常IPC、Owner資格だけ、Agent要求、Adapter metadataからは呼び出せない。

Brokerはcatalogの`inputSchema`適合、32 KiB以下・2048 node以下・深さ32以下、大小文字を問わない再帰的なAuthority／Permission／Approval／Credential／secret field不在を再検査する。Tool名だけでなく現在catalogのTool IDも一致させる。Tool IDは`tool-`に続けて、`sha256:<64桁hex>`表記のUTF-8 byte列をhex化した142桁の小文字hexを付けて生成する。native確認ごとにServer／Tool／arguments hashへ結合した一回限りPermissionをBroker内で生成・即時消費し、ApprovalとともにAuditへ記録する。Credential実値は注入しない。詳細は`mcp-contract.md`を参照する。

同一stdio childへ`tools/call`を一回だけ送信し、ID／JSON-RPC形状／resultType／結果サイズを検証する。`content`は最大128個、`text`／`image`／`audio`／`resource_link`／`resource`だけを受け付ける。`structuredContent`はobjectだけを受け付ける。未知content type、malformed result、input-required／partial result、timeout、ID不一致、結果Audit失敗は結果不明として接続を隔離し、自動再送しない。

Tool結果本文はFlutter、snapshot、Audit内容、error、log、traceへ渡さない。`hash_only` receiptはresult hash、Tool側error flag、content件数／型だけを持つ。Receipt確定後にBrokerの一時応答を破棄する。Agentへの結果引渡しとContent Exposureによる本文表示は未接続の`release_blocker`である。

版2026-07-28の応答では、結果種別`resultType`が`complete`の場合だけを受け付ける。旧版互換として、結果種別の欠落は許容する。`server/discover`がJSON-RPCのmethod未対応（`-32601`）を返すか、対応protocol versionがない場合だけ、旧`initialize`／`notifications/initialized`へ切り替える。

通信期限切れ、応答形式不正、要求ID不一致、parameter不正その他の失敗では、再起動や旧protocolでの再試行を行わない。外部Serverの実装を推測せず、JSON-RPC応答、capability、一覧、重複、pagination、秘密値・権限fieldをBroker側で検証する。Tool `inputSchema`はJSON Schema Draft 2020-12のmeta-schema検証とvalidator構築に成功したものだけをCatalogへ入れる。外部`$ref`を取得せず、128 KiB／4096 node／深さ64の上限を適用する。receiptの公開範囲は`metadata_only`に固定する。

## 実行経路

```text
owner control
  → MCP接続
  → Rust Broker
  → MCP stdio child process
  → server/discover または initialize
  → tools/list・resources/list・prompts/list
  → metadata-only receipt

OwnerがDesktop上でTool入力を確認
  → MCP Tool実行
  → Rust Desktop native Owner確認（default No）
  → Rust Brokerの現在接続・Tool ID・Schema・引数再検査
  → 一回限りPermissionの生成・消費
  → 同一stdio childへtools/callを一度送信
  → 結果hash／件数／型だけのhash-only LIVE_RUNTIME receipt
```

`MCP接続一覧`は通常IPCの読み取り専用経路であり、owner channelからは拒否する。接続開始、受信、受理、一覧返却には永続Auditを要求する。child processは環境変数をallowlistへ制限し、stderrを取り込まず、response timeout、line上限、終了状態をfail-closedで扱う。stdoutの各行は改行到着前から256 KiBを上限として逐次読取り、超過時は残りを無制限に蓄積せず接続を失敗させる。

Desktop設定のMCP接続センターは、サーバー識別子、Windows絶対実行file、Windows絶対workspace、1行1項目のstdio引数を受け取る接続設定面を持つ。Flutterはpath、process、Credential保管fileへ直接アクセスせず、秘密値の入力欄・取得API・snapshot保持を持たない。通常IPCのCredential metadata一覧から用途と対象Serverが一致する有効Credential IDだけを選択し、子processへ渡す環境変数名を指定できる。接続要求の`Credential ref`はID、用途、対象、required、status、環境変数名だけを含み、実値やAuthority fieldを送らない。引数へ秘密値を入力しないよう画面に警告する。

接続要求は既存Windows Rust起動器のBroker channelから厳密に検査され、default Noのnative Owner確認を通る。確認はServer ID、実行file、workspace、引数件数・hash、Credential ID、環境変数名、request payload hashを示すが、引数本文とCredential実値は表示しない。Credentialを選択した場合、対象Server processと子孫processが値を読取り・外部送信でき、Windows Job Objectはsandboxではないことを表示する。protocol fallback時に同じ対象へCredentialを渡したchildを再起動する場合も説明する。操作者はnative確認前のDesktop入力とCredentialの対象を確認する。未知field、非絶対path、制御文字、上限外入力、Credential refの不一致・unsupported state・危険な環境変数名はnative確認候補またはBrokerで拒否する。OwnerのYes後もBrokerが要求を再検証し、登録Audit／対象／用途／状態／ProtectedStore hashを照合してDPAPIから秘密値を読み、指定された環境変数だけを既存Windows Job Object監督下のstdio childへ渡してMCP discoveryを行う。Credential使用と接続を別々にAudit確定し、metadata-only receipt後も実値を返さない。通常IPCとnative Owner確認を通らないplatform経路では接続しない。

一覧更新は利用者の明示操作で行い、Server ID、表示名、stdio種別、Tool／Resource／Prompt件数だけを表示する。Tool欄は展開操作でTool名、Tool ID、入力Schema hashを表示できる。Resource欄は名前、Resource ID、URI template hash、Prompt欄は名前、Prompt ID、引数Schema hashだけを表示できる。各IDとhashは外部metadataであり、実行権や信頼を示さない。Tool description／description_summary、入力Schema本文、Resource URI／本文、Prompt description／引数／本文は表示・取得しない。Tool危険度は`unknown`のまま示す。接続先、実行file、起動引数、Credential refは一覧へ表示しない。Tool呼出しは個別の入力確認とWindows native Owner確認が必要で、一覧取得だけでは呼び出さない。Resource／Prompt本文取得要求も送信しない。一覧の外側と格納済み接続projectionは`INTERNAL_STATE`であり、MCP Serverとの現在接続やTrustの保証へ昇格させない。

Tool入力画面はJSON argumentsを操作者へ表示し、秘密値を入力しないよう警告する。native確認はServer ID、Tool ID／名前、top-level argument数、引数hash、request hashだけを示し、引数本文を表示しない。Ownerが確認を拒否した場合は送信しない。BrokerはWindowsまたは`macos-mcp-center.md`に定めるMac native確認経路以外からのTool呼出しを拒否する。他platformでは未対応として拒否する。

成功・Tool実行errorのどちらも、Brokerは応答本文を保持・公開せず、結果hash、content件数／型、Tool側error flagだけを`hash_only`の`LIVE_RUNTIME` receiptに含める。Agentへの結果引渡し、全文表示、Resourceリンク追跡は行わない。結果本文はMCP Serverの未信頼dataであり、後続の安全redaction・Content Exposure contractが成立するまでBroker内にも保存しない。

送信後にtimeout、malformed／unsupported response、ID不一致、結果監査失敗が発生した場合、Brokerは接続をquarantinedへ遷移させ、自動retryしない。owner切断だけがprocess群を終了・記録解消できる。quarantined接続は一覧で状態表示し、Tool呼出しを拒否する。ownerは外部副作用の有無を別途確認してから、必要なら再接続する。これは対象MCP Serverの外部状態を自動復旧・rollbackする保証ではない。

ownerは`MCP切断`で対象Server IDを明示できる。要求はowner controlだけで受け、未知field、未知Server、通常IPCからの切断は拒否する。Windows Job Objectのprocess群停止確認後に`LIVE_RUNTIME`の結果Auditを永続化し、その後に限ってBrokerの接続記録を消す。停止または結果Auditを確定できない場合は成功を返さず記録を保持する。停止済みだがAuditに失敗した場合は、ownerの明示再試行で停止を再確認して監査確定する。receipt／Auditに実行path、引数、workspace、Credential実値を含めない。切断はPermissionを生成しない。

Desktop切断要求はFlutterから既存Broker channelへ送り、Windows Rust起動器または限定Mac Rust helperが厳密な要求fieldを検査してからdefault Noのnative Owner確認を表示する。OwnerのYes後もBrokerが要求を再検証し、所有する実process群停止・永続Audit・記録解消を行う。FlutterはOwner資格・session file・Approvalを保持しない。native確認処理が実行されない通常IPC・他platformではBrokerが拒否し、fallbackや自動再送を行わない。

WindowsではMCP stdio Serverも既存のRust process群監督経路から起動する。childの初期threadを再開する前に専用Job Objectへ割り当て、Broker異常終了時のhandle closeでrootと子孫を停止する。root終了後に子孫が残っている場合もBrokerは接続一覧を返さず、終了を確認できない停止要求を成功扱いしない。この保証はWindows process群の範囲であり、MCP ServerのTrust、filesystem sandbox、credential安全性は証明しない。

## 境界

- `MCP metadata ≠ Authority`
- `Tool description ≠ Permission`
- `Trust ≠ Approval`
- `Capability diff ≠ Permission grant`
- `Credential ref ≠ Credential value`
- `Credential available ≠ Server trusted`
- Agent metadata、MCP metadata、履歴、Profile、Tool schemaから権限を生成しない。
- Windows stdioを除くCredential実値の注入、AgentへのTool結果引渡し、Resource／Prompt実取得、Streamable HTTP、OAuth、Tool出力redaction／full content approvalは未接続である。
- Windows／Mac以外ではprocess群停止を保証する既存監督がないため、`mcp_process_tree_supervision_unsupported`として`MCP切断`をfail-closedで拒否する。WindowsのJob Object試験を他OSのprocess群停止証拠へ流用しない。
- Windowsのfake MCP stdio child／descendant process fixtureはJob Objectによる起動・停止だけを検証する。外部MCP Serverの適合やinstalled product経路の証拠ではない。Windows以外のprocess群監督は別途検証を要する。
- `server/discover`またはlegacy `initialize`の応答は、Serverが信頼済みまたは承認済みであることを証明しない。

## 失敗と残存範囲

未知Server、応答id不一致、malformed JSON-RPC、authority／secret field、未処理pagination、応答timeout、child終了、必須Credentialの不足は接続または一覧を拒否する。`mcp_server_unavailable`と`mcp_timeout`を成功へ変換しない。

Agentへ結果を渡すMCP Tool実行経路、Windows stdio以外のCredential実値注入、Tool結果のContent Exposure、外部MCP実物Test Harness、非Windows process群監督、installed product上のOwner Credential利用証拠が未成立であることは`release_blocker`である。操作者向けの一回限りWindows呼出しreceiptとstdio Credential injectionだけで、Agent統合、外部MCP全体、D4 Pocket全体または正式releaseの完成を主張しない。

## Macの限定製品経路

P13の現在範囲と有限Acceptanceは`macos-mcp-center.md`へ分離する。Mac絶対pathのCredentialなしstdioを、既存Rust helperの独立native Owner確認後にBrokerへ渡す。UIの資格情報選択は無効、configured参照はnative候補とBroker双方で拒否する。Toolは上記現在Schema・一回Permission・Audit・hash-onlyと同じ責任境界を使う。切断はD4所有POSIX process groupの不存在確認後だけ記録を消す。App Sandboxを継承し、Windows Job Objectの全子孫／Broker crash保証、別groupへ離脱した第三者内部、外部副作用をMacの停止成功へ含めない。追加fault／Formal Evidenceは既存Final QA／platform関門へ残す。
