# Approval の表示境界（Approval Visibility Boundary）

Approval UI は、Runtime が露出していない内容をユーザーが承認できるかのように示してはならない。

## Full payload の保存境界

`full_payload` は、hashing、revalidation、Audit correlation、または後の full review のため、Approval の保存領域に存在し得る。

有効な Approval Contract が `content_visibility=full` でない限り、UI projection は決して `full_payload` を露出してはならない。

`content_visibility` が `none`、`hash_only`、`summary`、または `redacted` の場合、UI は許可された projection だけを描画し、label、tooltip、log、preview、search index、accessibility text、debug view を通じて full payload の値を漏出してはならない。

## 編集可能 field の制約

- 編集可能な field は Runtime が明示的に宣言しなければならない。
- authority field は決して編集できない。
- hidden field は決して編集できない。
- sealed field は決して編集できない。
- sacred-domain field は決して編集できない。
- Runtime identity、Permission identity、Audit identity、および payload hash は、決して直接編集できない。

編集後は、次を行う。

- payload を再 hash 化する。
- Approval を再検証する。
- 必要な場合、Approval status を validation required にする。
- 編集 event を Audit に記録する。
