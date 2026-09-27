# Windows process群監督契約

本契約は、Windows上でBrokerが起動したchild processとその子孫の終了責任を定める。process群監督はOS Job Objectを使う。Permission、Approval、Audit、Recoveryの意味は変更せず、Agent Taskの実行権限を作らない。

## 起動と停止

processは初期threadを停止した状態で生成し、実行を再開する前に専用Job Objectへ割り当てる。Jobには`KILL_ON_JOB_CLOSE`を設定する。割当てまたはthread再開を確認できない場合は起動失敗として閉じ、未監督processを残さない。

取消、期限超過、通信異常時はJob内全processの終了を要求し、上限時間内にactive process数が0になったことを確認する。Root processの終了後もJob内に残存processがあれば停止する。終了確認の失敗は操作成功や取消成功へ昇格せず、通信失敗にする。Broker異常終了ではJob handle closeをOSによる最終停止機構とする。

## unsafeの限定例外

`docs/LANGUAGE_POLICY.md`の「unsafe使用は原則禁止、例外は明示レビュー」に基づき、Win32 unsafe呼出しを`native/process_supervision/src/windows_job.rs`へ限定する。Rust Broker crateの`#![forbid(unsafe_code)]`を維持し、新crateには`#![deny(unsafe_op_in_unsafe_fn)]`を適用する。各unsafe blockには直前に日本語の`SAFETY`根拠を置く。

安全な高水準APIだけでは、childが一命令も実行する前のJob割当てと、Job handle close時の子孫終了を同時に構成できないため、専用crateのWin32境界を採用する。process handleは`Child`からの借用、Job・thread・snapshot handleはそれぞれ単独所有として扱い、closeを重複させない。process handleやJob handleをIPC／Flutter／Adapter metadataへ露出しない。別ファイルへのunsafe追加、Job breakaway、未監督fallbackは禁止する。

## 適用範囲と証拠

Codex Adapterのread-only対話と、固定`sandbox=workspace-write`を使うAgent Task実装は、どちらもこのJob Object経路でprocess群を監督する。Agent Task実装はBrokerへ接続済みだが、CLI実Task、Workspace隔離、失敗・取消時cleanupのLIVE_RUNTIME証拠が揃うまでAdapter metadataを`task_execution=unsupported`に保ち、Brokerから起動できない。Job Objectはprocess群の終了管理であり、filesystem／network sandbox、Workspace書込隔離、Runtime trust、Permission、Approvalの証拠ではない。

Windows実process fixtureによるowner強制終了・child停止、およびAdapter接続後のchild取消・回収試験は、このprocess群監督機構に対する`LIVE_RUNTIME`証拠である。実Codex CLIのversion／help確認はinterface証拠に限り、モデル実行、Broker Task、Task Workspace隔離・scratch cleanup、Broker異常終了後のcleanup、Agent間隔離の証拠ではない。固定`sandbox=workspace-write`ではWorkspace内の`.env`等の秘密fileをAgentから読めないことも証明されず、Broker crash／強制終了／電源断後のscratch回収機構もない。process群の停止確認をこの2つのfilesystem保証へ転用してはならない。
