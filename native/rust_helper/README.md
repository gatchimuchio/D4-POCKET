# Rust Helper / Rust Security Broker 骨格

UI code が所有すべきでない操作の native helper 境界。

現行 helper module:

- process
- filesystem
- network
- diagnostics
- update_verification
- audit_hash
- ipc

現行 broker module:

- `src/main.rs`: 明示的な `dev-stdin-smoke` 診断と `broker-server` の独立 process lifecycle。
- `src/broker/protocol.rs`: JSON request 解析、型付き envelope 検証、正規 payload-hash 結合、`issued_at` RFC3339 鮮度拒否、audit/replay/session store 準備報告、永続 state 必須時の利用不能 fail-closed 挙動、古い session の拒否、nonce replay 拒否、NFKC/case/zero-width/camelCase/separator/alias/value-only による権限類似 metadata の拒否、JSON response 直列化、health cutover 状態、権限操作の route、process/credential/update gate 報告付き command-envelope 休止。
- `src/broker/audit.rs`: accepted、rejected、suspended request 用の broker 局所追記専用 audit hash chain。各 event hash に request `payload_hash` を含める。
- `src/broker/store.rs`: `broker-server` mode で使う audit hash-chain、HMAC audit anchor、圧縮 replay nonce state、session state の永続 file store。
- `src/broker/authority.rs`: Rust 権限評価、正規化/quarantine、approval 編集、内容射影、audit-chain 検証、command 適格性評価。
- `src/broker/credential_vault.rs`: C7 owner資格情報登録と通常IPCのmetadata一覧。秘密値はWindows DPAPIへ保管し、C9のnative Owner確認後に対象が一致するWindows MCP stdio childへだけ環境変数として渡す。Runtime／一般Tool／Agent／A2Aへの注入は未接続。

Rust helper は、明示的な IPC または FFI 境界を通じて呼び出せる状態を保つ。

権限に敏感な runtime 所有権は Flutter、Python、FFI へ委譲しない。broker は Rust Security Broker への移行経路だが、製品 cutover は完了していない。

- 汎用 `command_envelope` の外部 command dispatch は無効である。rev2の実行系対話は、下記の専用owner承認経路だけで実行する。
- Flutter 製品経路は broker IPC を使うが、インストール済み Windows 製品証拠は依然として別の release blocker である。
- Python Shell Core は cutover 証拠が成立するまで、移行 oracle、tooling 経路、parity 比較源として残る。
- health は `boundary_role=rust_security_broker_candidate` と `authority_cutover_status=not_active` を報告する。
- 明示的な `dev-stdin-smoke` mode は memory 上の state を使い、`broker-server` mode は `--store-dir` 利用時に永続 audit/replay/session state を使う。
- persistent-state-required mode は永続 store 未接続時に休止または拒否する。

これらの未完了項目は、完成製品 release に対する `release_blocker` であり、release-ready 証拠ではない。

## 実行系対話の起動と承認

`src/broker/dialogue.rs` が要求、セッション、承認、取消、表示射影を所有する。`src/adapters/minidora.rs` は登録先のMINIDORA APIだけを呼び、`src/adapters/codex_cli.rs` はownerが起動時に明示した実物Codex CLIだけを固定read-only実行へ射影する。`src/owner_cli.rs` はownerの明示的な制御操作であり、通常Flutterへ資格を渡さない。

```powershell
cargo build --manifest-path native/rust_helper/Cargo.toml
native/rust_helper/target/debug/gui_shell_rust_helper.exe broker-server --store-dir C:/local/gui-shell/store --session-file C:/local/gui-shell/ui.json --owner-session-file C:/local/gui-shell/owner.json --minidora-runtime local=127.0.0.1:8080
native/rust_helper/target/debug/gui_shell_rust_helper.exe 対話承認操作 --session-file C:/local/gui-shell/owner.json 一覧
native/rust_helper/target/debug/gui_shell_rust_helper.exe 対話承認操作 --session-file C:/local/gui-shell/owner.json 承認 <要求ID> <要求hash> full
native/rust_helper/target/debug/gui_shell_rust_helper.exe 回帰Case登録 --session-file C:/local/gui-shell/owner.json 登録 C:/local/gui-shell/regression-case.json
native/rust_helper/target/debug/gui_shell_rust_helper.exe 資格情報登録 --session-file C:/local/gui-shell/owner.json 登録 C:/local/gui-shell/credential-registration.json
```

資格fileの親directoryはownerが事前に用意する。資格fileをGitへ追加しない。MINIDORAは別processで起動し、GUI Shellが勝手に起動・変更・配布しない。複数登録は `--minidora-runtime` を実行系ごとに指定する。登録だけでは通信しない。

### 実物Codex CLIの限定登録

Windows上で実際に確認できたCodex CLIだけを、ownerの起動引数から次の形式で登録できる。

```powershell
native/rust_helper/target/debug/gui_shell_rust_helper.exe broker-server `
  --store-dir C:/local/gui-shell/store `
  --session-file C:/local/gui-shell/ui.json `
  --codex-runtime codex=C:/absolute/path/to/codex.exe=C:/absolute/path/to/workspace
```

登録時にexecutableとworkspaceを実在する絶対pathとして検査し、`codex --version`と`codex exec --help`を実際に確認する。実行時は固定した `codex exec --json --ephemeral --ignore-user-config --sandbox read-only --color never --cd <workspace> -` だけを使い、任意のargv、環境変数、workspace、process操作をIPCから受け取らない。環境変数は安全なallowlistだけを子processへ渡し、credential実値やsecret pathを応答・監査へ射影しない。

Codexの応答はJSONLの `thread.started`、`item.completed` の `agent_message`、`turn.completed` を検証してから、既存の `実行系挙`、`対話開始`、`対話送信`、`対話取得`、`対話中止`、`対話終了` とowner承認・監査経路へ接続する。`turn.failed`、不正JSON、出力上限超過、取消、期限超過は成功へ昇格しない。この登録はCodexの実物interfaceと読み取り専用のAgent Launcher基盤を提供するが、write-capable Agent、MCP、複数Agent比較、Handoff、製品releaseを成立させない。

`--codex-runtime` の値は `ID=絶対executable path=絶対workspace path` であり、executable path内の `=` は未対応である（workspace pathの残りは3番目のfieldとして扱う）。Claude、Gemini、その他Vendorの実装は、実物interfaceを確認するまで追加しない。

### C6 回帰Caseのowner登録

`回帰Case登録` はowner資格fileからだけ実行し、`specs/regression_case_registration.schema.json`に従うJSON fileを受け取る。Brokerは完了済み通常対話の要求ID/hash、全文表示承認、結果証跡、終了監査IDを再照合する。元の対話入力・応答本文を自動コピーせず、ownerが`owner_explicit_redacted`として明示した定義だけを`ProtectedStore::Purpose::Regression`へ暗号化する。CLI出力は`regression_case_receipt.schema.json`のhash-only receiptに限定され、private本文・条件・参照・期待経路を表示しない。

### C7 資格情報保管庫のowner登録

`資格情報登録`はowner資格fileからだけ実行し、`specs/credential_registration.schema.json`に従うJSON fileを受け取る。Brokerは秘密値を`ProtectedStore::Purpose::Credential`へWindows DPAPI保管し、CLIには`credential_receipt.schema.json`のmetadata_only receiptだけを表示する。通常IPCの`資格情報一覧`は保管fileのhash照合後にmetadataだけを返し、秘密値、owner資格、Permission、Approval、Authorityを返さない。Windows MCP stdio childへの注入だけは、対象Server一致のCredential metadata選択とnative Owner確認後、Brokerが用途・対象・保管hashを再検証して接続中processへ環境変数として限定注入し、値を通常応答やAuditへ投影せず使用Auditを記録する。Credentialを受け取るServer processは値を読み取り・外部送信でき、Job Objectはsandboxではない。MCP以外への取得・Runtime／一般Tool／Agent／A2A注入、更新、失効、削除、接続先変更、Recovery、GUI管理面は未接続である。

開発検証は `cargo test --manifest-path native/rust_helper/Cargo.toml`。実物参照版との統合は `python tooling/minidora_live_check.py --reference <MINIDORA参照clone>`。このPythonは試験用process管理であり、製品依存ではない。固定commitとclean状態を検査し、基本会話・保留・失敗分離・資格拒否・監査chain再読取を実行する。外部検索や基礎Coreの能力を保証する試験ではない。
