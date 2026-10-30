# D4-AUD-F05 remote `codex/*` branch棚卸し（2026-10-30）

状態: `VERIFIED`

## 対象と判定

- 棚卸し基準: `origin/main` = `41e834c8bdf3237c52329aa261f934762771c9bc`。取得後のlocal `main`はcleanで、remote `main`と一致していた。
- 対象: `git ls-remote --heads origin`で確認したremote `codex/*` 3本。3本とも`main`未統合の固有commitを持つ。
- commit authorはGit metadata上いずれも`がっちむち♂`。これは過去のcommit authorであり、現在の担当者・作業者を確定する情報ではない。Codexの会話一覧では他の稼働中D4スレッドを観測しなかったが、外部writerがいないことまでは証明しない。
- mainへの統合・remote branch削除・branch更新は行っていない。各枝の最新HEADと未統合差分をそのまま保持した。

| リモート作業枝 | 先頭記録 / 最新コミット | `main`側のみ / 作業枝側のみ | 差分 | 判定と処置 |
|---|---|---:|---|---|
| `codex/b5-pktmon-safe-log` | `4d0a222fcc594dfbe12462f9a8666c446dbb393c` / 2026-10-19 | 154 / 2 | 6ファイル、+211/-5 | `継続`。B5は`OPEN`で、`Pktmon`の失敗診断と出力形状確認に固有の差分がある。現担当は未確認のため保持し、B5施工時に統合可否を決める。 |
| `codex/linux-installed-workspace-acceptance` | `9659fa63e1b499723635ee4754c56bc930ba6ba2` / 2026-10-22 | 4 / 21 | 9ファイル、+1440/-30 | `継続`。Linuxネイティブ製品は必須対象で、導入済み作業領域の受入れ差分は未統合。Linux受入れ工程で継続し、削除しない。 |
| `codex/r2-inspector-owner-confirmation-20261002` | `fca1d0c9d83f03108ebaaf64ea742ca38079b4fc` / 2026-10-02 | 750 / 1 | 9ファイル、+585/-12 | `要判断`。作業領域の所有者確認と基準点保存に関する固有変更は未統合。現在の作業者・継続予定を確定できず、関連する別作業場所にも未反映変更があるため保持する。 |

各branch tipは`origin/main`の祖先ではなく、`git cherry origin/main <branch>`でもbranch固有commitにpatch相当を確認できなかった。件数は`git rev-list --left-right --count origin/main...<branch>`の左がmainのみ、右がbranchのみ。

## Worktreeとの関係

- `C:\d4-r2-build-696604e`はdetached HEAD `696604e66f19eb0363828891dd2698b9a428e9d7`。このHEADがR2 branchのmerge-baseで、そこからR2 branchに10 commit、worktree側に0 commitの分岐差がある。worktreeには`apps/desktop_flutter/test/broker_client_payload_hash_test.dart`と`apps/desktop_flutter/windows/runner/broker_pipe_controller.cpp`の未commit変更があり、両方ともR2 branchの差分対象と重なる。branch/worktreeどちらも変更せず保持した。
- `C:\d4p-p13-native`にも別のdirty detached worktreeがある。今回の3 branchとの差分pathとは重ならないため、変更せず保持した。
- これらの観測は当該worktreeの存在・差分を示す。別プロセスや別端末にwriterがいないことは保証しない。

## 検証・境界

- 実行した確認: `git fetch --prune origin`、remote/local HEAD照合、remote branch一覧、`git rev-list`、`git merge-base`、`git cherry`、`git diff --stat`／`--name-status`、全worktreeの状態確認。
- branch tip・unique commit・変更path・dirty worktreeは特定した。進行中writerの所有者と将来の統合時期は`UNKNOWN`。
- F05のAcceptanceは枝ごとの根拠付き区分と未統合branchの保全。全条件を満たすため`VERIFIED`とする。source buildや製品testはこの棚卸しの対象外で実行していない。
- 製品release状態は変更していない。通常Release `task_execution=unsupported`、`release_ready=false`を維持する。
- 次工程: `D4-AUD-F06`。
