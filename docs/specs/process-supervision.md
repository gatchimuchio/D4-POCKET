# Windows process群監督契約

本契約は、Windows上でBrokerが起動したchild processとその子孫の終了責任を定める。process群監督はOS Job Objectを使う。Permission、Approval、Audit、Recoveryの意味は変更せず、Agent Taskの実行権限を作らない。

## 起動と停止

processは初期threadを停止した状態で生成し、実行を再開する前に専用Job Objectへ割り当てる。Jobには`KILL_ON_JOB_CLOSE`を設定する。割当てまたはthread再開を確認できない場合は起動失敗として閉じ、未監督processを残さない。

取消、期限超過、通信異常時はJob内全processの終了を要求し、上限時間内にactive process数が0になったことを確認する。Root processの終了後もJob内に残存processがあれば停止する。終了確認の失敗は操作成功や取消成功へ昇格せず、通信失敗にする。Broker異常終了ではJob handle closeをOSによる最終停止機構とする。

## unsafeの限定例外

`docs/LANGUAGE_POLICY.md`の「unsafe使用は原則禁止、例外は明示レビュー」に基づき、Win32 unsafe呼出しを`native/process_supervision/src/windows_job.rs`へ限定する。Rust Broker crateの`#![forbid(unsafe_code)]`を維持し、新crateには`#![deny(unsafe_op_in_unsafe_fn)]`を適用する。各unsafe blockには直前に日本語の`SAFETY`根拠を置く。

安全な高水準APIだけでは、childが一命令も実行する前のJob割当てと、Job handle close時の子孫終了を同時に構成できないため、専用crateのWin32境界を採用する。process handleは`Child`からの借用、Job・thread・snapshot handleはそれぞれ単独所有として扱い、closeを重複させない。process handleやJob handleをIPC／Flutter／Adapter metadataへ露出しない。別ファイルへのunsafe追加、Job breakaway、未監督fallbackは禁止する。

## 適用範囲と証拠

Codex Adapterのread-only対話、固定`sandbox=workspace-write`を使うAgent Task実装、およびWindowsのMCP stdio childは、このJob Object経路でprocess群を監督する。Agent Task実装はBrokerへ接続済みだが、CLI実Task、Workspace隔離、失敗・取消時cleanupのLIVE_RUNTIME証拠が揃うまでAdapter metadataを`task_execution=unsupported`に保ち、Brokerから起動できない。MCP childのfake protocol fixtureは実Windows process群の起動・停止に対する`LIVE_RUNTIME`証拠であり、実外部MCP Serverやinstalled Brokerの証拠ではない。Job Objectはprocess群の終了管理であり、filesystem／network sandbox、Workspace書込隔離、Runtime trust、Permission、Approvalの証拠ではない。Windows以外のMCP process群停止は本単位で検証していない。

Windows実process fixtureによるowner強制終了・child停止、およびAdapter接続後のchild取消・回収試験は、このprocess群監督機構に対する`LIVE_RUNTIME`証拠である。実Codex CLIのversion／help確認はinterface証拠に限り、モデル実行、Broker Task、Task Workspace隔離・scratch cleanup、Broker異常終了後のcleanup、Agent間隔離の証拠ではない。固定`sandbox=workspace-write`ではWorkspace内の`.env`等の秘密fileをAgentから読めないことも証明されず、Broker crash／強制終了／電源断後のscratch回収機構もない。process群の停止確認をこの2つのfilesystem保証へ転用してはならない。

## Linux Broker crash後のprocess群停止

Linux Agent CLI／MCP childの起動では、既存の同梱`gui_shell_linux_broker`を内部guardian modeで先に起動し、そのPIDをleaderとする専用process groupへchildを配置する。Brokerだけがguardianのstdin pipe writerを保持する。正常終了・取消・deadline・IPC failureではBrokerが現在のguardian保有中にprocess groupを停止・waitし、group消失まで確認する。Brokerがcrash／強制終了してpipeがEOFになった場合、guardianが自身の専用groupへ`SIGKILL`を送り、childとgroup内descendantを停止する。

Guardianは`PID == PGID`を確認できない場合、signal前に終了する。Broker稼働中にguardian消失を検知した場合はTaskを成功扱いせず、追跡rootだけを停止する。guardian終了後は数値PGIDの再利用を避けるためgroup signalを試みず、cleanup未確定を返す。内部mode以外の起動argumentはBroker helperが拒否する。

保証範囲は専用process groupに残るprocess群に限る。childが`setsid`等でgroupを離れた場合の停止、filesystem／network sandbox、外部副作用は保証しない。単独WSL pipe-EOF fixtureだけではscratch journalのcrash RecoveryやAudit durabilityを証明しない。2026-10-20の統合testはWSL上で登録済みAdapter factory、persistent Broker journal、実Codex形状の合成child、別Broker起動、Workspace再登録を連結し、group内descendant停止、Task中断、binding一致scratch回収とRecovery Auditの耐久読戻しを確認する。ただし、Linux native installed Broker／GTK GUI／実Codex・MCP consumer一体受入れではない。

2026-10-20のUbuntu 24.04.5 WSL／Rust 1.95.0で、pipe EOF後のdescendant group停止、guardian消失時のfail-closed、既存group停止を含むprocess tree試験3 passed／0 failed／1 ignored、全Adapter試験29 passed／0 failed／2 ignored、Linux `cargo check --all-targets`を確認した。Windows Job Object試験2 passed／0 failedも確認した。Ubuntu 24.04.5手動Actions run #16（commit `57ad1d0b6364d6ddfe09f383d5cee424ed19a22b`）でもLinux guardian process-tree試験3 passed／0 failed／1 ignored、Rust all-target compile、限定Linux product tests、temporary target cleanupがPASSした。Actions run URLは[run #16](https://github.com/gatchimuchio/D4-POCKET/actions/runs/39379032564)。実Codex sandbox LIVE_RUNTIMEだけはhosted runnerで`bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted`のためNOT_RUN。これらはLinux installed product supportのrelease acceptanceを成立させない。
