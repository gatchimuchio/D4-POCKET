# 資格情報保管庫

## 対象

資格情報保管庫は、Runtime、Tool、MCP、A2Aなどの接続に使う秘密値を、D4 Pocket / GUI-ShellのBrokerが管理するための境界である。資格情報の存在や参照はPermissionを生成せず、資格情報ID、接続対象、種類、保管方式だけを通常画面へ投影する。

この作業単位で接続するのは、owner control経路からの新規追加と、通常IPCからの安全なmetadata一覧である。秘密値の取得、Runtimeへの注入、更新、失効、削除、接続先変更はこの登録契約へ未接続であり、未完成のまま保管庫完成とは扱わない。

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

未接続範囲は、秘密値のRuntime / Tool / MCP / A2A使用、GUI管理面、更新、失効、削除、接続先変更、Recovery操作、Windows実機でのowner登録証拠、macOS/iOS KeychainおよびAndroid Keystoreである。これらは`release_blocker`として残し、登録成功を製品完成やrelease readinessへ昇格させない。
