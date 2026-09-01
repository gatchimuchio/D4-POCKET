# デスクトップ Flutter app

予約されたデスクトップ Flutter shell 境界。

classification: required_for_v1
reason: デスクトップ Flutter は交換可能な操作者用 UI 表層であり、UI の責任だけに留める。
blocks_release: no

この app は UI の責任だけに留める。

- 概要画面を描画する
- runtime 状態を表示する
- permission、approval、audit、recovery の各センターを表示する
- 生成済み contract client を呼び出す
- 明示的な境界を通じて adapter/runtime API を呼び出す

権限判定を所有してはならない。
