# D4 Pocket工程と総合機能拡張rev1の対応

## 目的と効力

本書は、D4 Pocket統合開発工程表rev2の`Phase 0`〜`Phase 45`と、先行する総合機能拡張rev1の`C0`〜`C34`との要求範囲の対応を示す。D4工程はrev1の進捗・失敗履歴・証拠を初期化せず、既存成果の再実装を避けて不足分を積み上げる。

表の対応は対象範囲の関係だけを示し、完了・`PASS`・同等性を移転しない。C工程の現況と各時点の証拠は`docs/総合機能拡張_rev1/進捗.md`、rev2での実装履歴は`docs/REV2_PROGRESS.md`、現在の工程と公開条件は`ROADMAP.md`および`release_blockers.registry.json`で個別に判定する。旧文書の失敗を後続工程の合格で書き換えない。

## D4工程から既存C工程への対応

| D4工程 | 対象 | 既存rev1工程との範囲関係 |
|---|---|---|
| `Phase 0` | 基準状態固定 | `C0`の基準と履歴を参照する。現行値は再測定し、過去値は履歴として保持する。 |
| `Phase 1` | 製品境界 | `C31`の文書整備と関連するが、製品名と製品境界はD4独自の追加範囲。 |
| `Phase 2` | 接続先能力の観測 | `C17`の複数`Host`登録と関連する。能力の観測と権限は分離する。 |
| `Phase 3` | `Agent`接続契約 | rev1に同一の独立C工程はなく、D4追加範囲。 |
| `Phase 4` | `Agent`起動器 | rev1に同一の独立C工程はなく、実物interface確認を含むD4追加範囲。 |
| `Phase 5` | `Agent`操作表示盤 | `C1`の`Agent Center`・作業領域表示と関連する。 |
| `Phase 6` | 作業領域検査 | `C1`。既存の取得・差分・復旧境界を再利用し、再実装しない。 |
| `Phase 7` | `Agent`比較・`AgentTask`要求契約・`Session`／`Workspace`結合 | `C1`の作業領域登録、`C30`の比較時混線回帰と関連する。`Broker`登録IDと`Session`の結合、`Adapter`固定`root`と登録`root`の物理識別子照合、Task専用`Workspace Permission`／別`Owner Approval`、実行直前の原子的再検証・一回消費、専用`bounded`状態・取消・開始／`terminal Audit`はBroker接続済み。Codex CLI `0.158.0-alpha.2.1`の実helpを確認し、固定`sandbox=workspace-write`起動・Workspace内TEMP/TMP scratch・Windows Job Object監督・cleanupのAdapter実装を追加したが、能力metadataは`unsupported`のままでBrokerから実行されない。Rust全target 358件の試験は合格。これは実Agent／Broker Taskの`LIVE_RUNTIME`証拠ではなく、Workspace内秘密file読取の拒否、実書込隔離・外部path拒否、crash後scratch回収・結果／`diff`表示・`Content Exposure`・cross-agent contamination・比較／`Handoff`は未成立。過去のWindows Application Control拒否記録は`docs/REV2_PROGRESS.md`に履歴として保持する。 |
| `Phase 8` | `Agent`引継ぎ | `C30`の回帰対応と関連する。成果物の受渡しと権限の非継承は別途検証する。 |
| `Phase 9` | 実行履歴 | `C2`。 |
| `Phase 10` | 再実行・分岐 | `C2`。履歴を承認へ昇格しない。 |
| `Phase 11` | 実行系資源監視 | `C3`。測定不能値は`unknown`として扱う。 |
| `Phase 12` | 実行系ライフサイクル | `C4`。 |
| `Phase 13` | 評価ラボ | `C5`。 |
| `Phase 14` | 回帰事例登録 | `C6`。 |
| `Phase 15` | 接続先・Model設定 | rev1に同一の独立C工程はなく、D4追加範囲。 |
| `Phase 16` | 資格情報保管庫 | `C7`。秘密値の非露出境界を継承する。 |
| `Phase 17` | `MCP`接続契約 | `C8`。 |
| `Phase 18` | `MCP`試験基盤 | `C8`・`C9`の契約・接続負例を補完するD4工程。試験用模擬環境の成功を実接続証拠へ昇格しない。 |
| `Phase 19` | `MCP`接続管理 | `C9`。 |
| `Phase 20` | 運用Profile | `C10`。ProfileをPermissionとして扱わない。 |
| `Phase 21` | 更新管理 | `C11`。 |
| `Phase 22` | 通知管理 | `C12`。通知を承認や権限として扱わない。 |
| `Phase 23` | 稼働観測 | `C13`・`C14`。観測値とAudit確定処理を分離する。 |
| `Phase 24` | `A2A`接続契約 | `C15`。 |
| `Phase 25` | `A2A`接続管理 | `C16`。外部Agent metadataからTrustを生成しない。 |
| `Phase 26` | 複数`Host`管理 | `C17`・`C18`。Host切替後に別Hostの権限を再利用しない。 |
| `Phase 27` | `Adapter`管理 | `C19`。 |
| `Phase 28` | `Windows`操作面 | `C18`・`C20`・`C21`・`C22`・`C23`のDesktop操作面と関連する。 |
| `Phase 29` | GUI-Shell構成作成 | rev1に同一の独立C工程はなく、D4追加範囲。 |
| `Phase 30` | 構成差分確認 | rev1に同一の独立C工程はなく、D4追加範囲。 |
| `Phase 31` | AI編集提案 | rev1に同一の独立C工程はなく、OwnerまたはDeveloperが明示開始するD4追加範囲。 |
| `Phase 32` | `Windows`独立App書出し | rev1に同一の独立C工程はなく、D4追加範囲。 |
| `Phase 33` | 機能単位選択・除去 | rev1に同一の独立C工程はなく、D4追加範囲。安全Core保持と実binary／比較証拠を別々に追跡する。 |
| `Phase 34` | 製品配布 | `C33`・`C34`と関連する。Rust Desktop起動器、Flutter–Broker channel、installed evidenceを含む。manifest-onlyやDeveloper buildを配布完了へ昇格しない。 |
| `Phase 35` | Mobile統合 | `C24`・`C26`と関連する。Android実機凍結や正式配布条件は独立条件として保持する。 |
| `Phase 36` | Apple／Linux補助検証 | `C25`・`C26`の補助検証と関連する。仮想環境を実機証拠へ昇格しない。 |
| `Phase 37` | `MINIDORA`接続口 | rev1に同一の独立C工程はなく、実行系契約と安全境界を分離するD4追加範囲。 |
| `Phase 38` | `MINIDORA`段階統合 | rev1に同一の独立C工程はなく、referenceとの一致試験等を条件とする将来のD4追加範囲。 |
| `Phase 39` | 長時間運用 | `C28`。短時間試験を長時間合格へ昇格しない。 |
| `Phase 40` | 障害注入 | `C29`。 |
| `Phase 41` | 性能評価 | `C27`。開発測定と導入済み製品の性能を区別する。 |
| `Phase 42` | 全数回帰 | `C30`・`C32`。要求対応表だけで実行時挙動の完成を主張しない。 |
| `Phase 43` | 文書・ブランド統合 | `C31`。履歴、既存識別子、公開Contractの変更境界を保持する。 |
| `Phase 44` | `Windows`最大到達 | `C32`・`C33`。導入済み製品の証拠と開発build証拠を分離する。 |
| `Phase 45` | 正式公開 | `C34`。公開阻害条件の解消と明示Owner GOを必須とする。 |

## C0〜C34の逆引き

| 既存工程 | 主なD4工程 |
|---|---|
| `C0` | `Phase 0` |
| `C1` | `Phase 5`・`Phase 6` |
| `C2` | `Phase 9`・`Phase 10` |
| `C3` | `Phase 11` |
| `C4` | `Phase 12` |
| `C5` | `Phase 13` |
| `C6` | `Phase 14` |
| `C7` | `Phase 16` |
| `C8` | `Phase 17`・`Phase 18` |
| `C9` | `Phase 18`・`Phase 19` |
| `C10` | `Phase 20` |
| `C11` | `Phase 21` |
| `C12` | `Phase 22` |
| `C13` | `Phase 23` |
| `C14` | `Phase 23` |
| `C15` | `Phase 24` |
| `C16` | `Phase 25` |
| `C17` | `Phase 2`・`Phase 26` |
| `C18` | `Phase 26`・`Phase 28` |
| `C19` | `Phase 27` |
| `C20` | `Phase 28` |
| `C21` | `Phase 28` |
| `C22` | `Phase 28` |
| `C23` | `Phase 28` |
| `C24` | `Phase 35` |
| `C25` | `Phase 36` |
| `C26` | `Phase 35`・`Phase 36` |
| `C27` | `Phase 41` |
| `C28` | `Phase 39` |
| `C29` | `Phase 40` |
| `C30` | `Phase 7`・`Phase 8`・`Phase 42` |
| `C31` | `Phase 1`・`Phase 43` |
| `C32` | `Phase 42`・`Phase 44` |
| `C33` | `Phase 34`・`Phase 44` |
| `C34` | `Phase 45` |

Codex Adapterの固定作業pathと同一runtime IDのWorkspace rootを起動時に照合するpreflightに加え、Agent Taskの検査・Permission／Approval発行でAdapter固定rootのdevice/file IDとBroker登録rootの識別子一致を照合する経路を追加した。これは登録実体の一致をfail-closedに検査する内部状態であり、実Agent専用Session、実行directoryの継続的同一性、sandbox、独立Agent比較、cross-agent contamination防止を証明しない。`Phase 7`および`C1`・`C30`の完了状態は変更しない。

## D4追加範囲

次のPhaseはrev1 C0〜C34と同一の独立工程がないD4追加範囲であり、既存Cの状態を転記して完了扱いしない。

`3, 4, 15, 29, 30, 31, 32, 33, 37, 38`

追加範囲も、対応するSchema、実接続経路、正常・負例試験、監査、証拠、公開阻害条件を個別に記録する。
