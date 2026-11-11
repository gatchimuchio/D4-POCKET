# A2A v1.0 Stream送信

## 位置付けと範囲

C16の一回`SendMessage`および手動一回`GetTask`を変更せず、接続済みAgentへA2A protocol v1.0 `SendStreamingMessage`を一度だけ実行する追加契約である。transportはJSON-RPC 2.0 over HTTP(S)とし、現実装は認証不要のinterfaceだけを対象にする。`SubscribeToTask`、Credential利用、再接続、retry、background pollingは含めない。

native Owner経路があるWindows／Linux Desktopだけを対象とする。macOS、Mobile、remote Hostの支持を主張しない。Agent Cardの`streaming=true`は外部宣言metadataであり、Trust、Permission、Approvalまたは実行権限ではない。Brokerは現在の接続receipt、Agent ID、interface hash、能力宣言、要求hashを作用直前に再検査する。

## 一回作用と失敗

各送信に対して、Rust Brokerはnative Owner確認、永続Audit、一回限りPermissionの発行・消費を行う。Permissionと確認は現在のAgent、interface、Messageへ束縛し、再利用しない。外部送信を一回行った後は、timeout、途中切断、不正event、Audit確定不能、Broker再起動をすべて結果不明として扱う。外部要求を再送・再接続しない。

要求本文はUTF-8 4 KiB以下、1024 Unicode scalar以下。応答は15秒、64 KiB、32 eventまで。超過・不正JSON／SSE／未知variant・不一致ID／Task／Contextはfail-closedとする。受信後のAuditが確定できなければ成功を返さずunknownとし、再送しない。

## A2A v1.0 SSE形状

各SSE `data` eventは、`jsonrpc`が`2.0`で要求IDと一致するJSON-RPC responseの`StreamResponse`でなければならない。v1.0の`StreamResponse` oneofは`task`、`message`、`statusUpdate`、`artifactUpdate`であり、旧版の`kind` discriminatorや`final` booleanを要求しない。送信Messageもv1.0のmember-discriminated JSONを使い、text partは`{"text":"..."}`とする。v0.3系の`kind: message`／`kind: text`は送信しない。

- Message-only応答は、単独の`message` event一つでstreamを終える。これは外部Task完了を意味しない。
- Task streamは最初にTaskを一つ返し、その後は同一Task／Contextに束縛された`statusUpdate`または`artifactUpdate`だけを返す。terminal状態は`completed`、`failed`、`canceled`、`rejected`のいずれかでなければならず、streamも直後に閉じる。
- terminal event後の追加event、Task stream中のMessage、Task／Context不一致、終端なし、空streamは拒否する。

Task／Message／Artifactの識別値、各raw event、結果projectionをhash化する。本文、Artifact parts、外部ID実値、credential、未知metadataはUI、Audit、log、errorへ出さない。Auditには要求hash、Agent ID、interface／Message hash、event数・種類、状態、result hashだけを記録する。Broker再起動時、`started`のまま終端Auditがない要求は一度だけunknownへ隔離する。

## UI projectionと権限境界

A2A接続センターは、対応Desktop上でstreamingを宣言した現在接続Agentにのみ一回送信UIを表示する。native Owner確認を必須とし、受信後は`metadata-only` projectionとしてevent kind、状態、part count、hashだけを投影する。本文をAgent Center Task、D4 Pocket Task、Compare、Handoffへ移送しない。外部Task状態はD4 PocketのTask完了やApprovalにならない。

```text
Agent Card capability ≠ Permission
external Task terminal ≠ D4 Pocket Task completion
external Message / Artifact ≠ Authority
stream event ≠ Approval
```

Protocol reference: [A2A v1.0 specification](https://a2a-protocol.org/v1.0.0/specification/).
