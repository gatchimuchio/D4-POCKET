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

Rust helper は、明示的な IPC または FFI 境界を通じて呼び出せる状態を保つ。

権限に敏感な runtime 所有権は Flutter、Python、FFI へ委譲しない。broker は Rust Security Broker への移行経路だが、製品 cutover は完了していない。

- 実際の外部 command dispatch は無効である。
- Flutter 製品経路は broker IPC を使うが、インストール済み Windows 製品証拠は依然として別の release blocker である。
- Python Shell Core は cutover 証拠が成立するまで、移行 oracle、tooling 経路、parity 比較源として残る。
- health は `boundary_role=rust_security_broker_candidate` と `authority_cutover_status=not_active` を報告する。
- 明示的な `dev-stdin-smoke` mode は memory 上の state を使い、`broker-server` mode は `--store-dir` 利用時に永続 audit/replay/session state を使う。
- persistent-state-required mode は永続 store 未接続時に休止または拒否する。

これらの未完了項目は、完成製品 release に対する `release_blocker` であり、release-ready 証拠ではない。
