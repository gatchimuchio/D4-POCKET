# 作業領域インスペクタの契約

対象は総合機能拡張rev1のC1。現在の実装単位は差分生成器であり、インスペクタ全体の完成を意味しない。

## 責任と接続

Rustの差分生成器は、統治済み取得経路が渡す変更前・変更後のbytesから決定論的な統合差分と左右比較行を生成する。filesystem、process、network、資格、Permission、Approvalにはaccessしない。入力を取得したこと自体を権限としない。出力は内部の内容を含み得るため、Brokerが現在のContent Exposure Boundaryを適用する前にUI・log・監査本文へ渡してはならない。全文はfullだけ、binaryと上限超過はfullでもmetadataだけとする。

後続の取得経路は、ownerが登録したRuntime・workspace・secret除外範囲・基準点に結合する。UIから任意root、任意commit、任意実行commandを渡して境界を広げない。workspace外、secret path、linkによる逸脱を取得前に拒否し、取得中の対象変更も検出する。相対pathの検査だけを実filesystem境界の証拠にしない。Tool・shell・test履歴は実行元と現在セッションに結合した記録を取得し、外部報告とBroker実測を区別する。履歴に存在するApprovalやPermissionは権限を再生成しない。

## 差分データ

`workspace_diff.schema.json`はRust差分生成結果の構造を定義する。変更前後それぞれの存在、byte数、SHA-256を区別し、存在しないfileと空fileを同一視しない。入力が同一ならunchanged、UTF-8でないかNULを含むならbinary、本文上限超過ならoversized、それ以外はtextとする。binary・oversized・unchangedはunified=nullかつrows=[]とし、部分本文を返さない。

本文生成上限は片側65,536 bytes、片側2,000行、LCS表1,000,000セル。上限判定前に二乗表を確保しない。上限超過は差分なしでも成功でもなく、metadataだけを取得した状態として表示する。metadataのhashは受領した全bytesへ結合するが、巨大fileの取得自体をこの生成器へ委ねない。取得経路には別途streamingと読取上限が必要である。

textは全体を1つのhunkとして統合差分を生成する。固定label a/file、b/fileを使用し、外部pathや制御文字をpatch headerへ挿入しない。追加・削除は/dev/nullを使用する。空fileの追加・削除はGitの存在差header（`new file mode` / `deleted file mode`）で存在差を示す。100644は表示用の標準modeであり、元のfilesystemの実行権限や復旧操作のmodeを保証しない。改行なしは標準の終端改行なしmarker（`No newline at end of file`）で区別する。左右比較は変更前後の行番号、追加・削除・変更・同一の種類、元の行末改行有無を保持する。CRLFとLF、最終改行の有無も差分である。

## C1の残る接続と試験

- item: 作業領域の統治済み取得・履歴・Broker・UI・巻戻しpreview
  classification: release_blocker
  reason: 差分生成器はC1の内部処理であり、製品IPCとUIはまだ消費していない。secret path・workspace外の実取得拒否やAudit対応も未検証。
  required_action: Capability・Permission・Approval・Audit・Recoveryへ対応づけた取得経路を実装し、ファイルツリー、変更file、両差分形式、Tool・shell・test・Approval・Audit・rollback候補を実データで表示する。rollbackの実変更はRecoveryと新しいApproval経路を必須とする。
  blocks_release: yes

Gitの存在差headerは[Git公式の差分形式](https://git-scm.com/docs/diff-format)に従う外部固定構文である。日本語監査のJBE-008はこの生成器の2つの固定語と行区切りescapeだけを対象とし、説明・診断・file全体を除外しない。
