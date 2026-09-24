# GUI Shell Module選択・除去計画の意味正本

## 目的と適用範囲

独立Appへ含めるDesktop画面Moduleを選択し、構成Manifestへ機械可読に記録する。現行Module一覧は`specs/gui_shell_module_catalog.json`を唯一の一覧正本とし、各画面pathは対応付け候補である。`unprunable_core_ids`は安全保持条件であり、Rust Broker／暗号／監査等が個別linkable moduleへ分割済みという主張ではない。画面一覧もRust Broker内部Module、crate依存、第三者runtime／tool、最終binaryの実測済み除去を意味しない。

## 必須Module

Dashboard、Runtime操作、Agent操作、Authority、Approval、Audit、Recovery、Problem、Evidence、SettingsのModuleは常に含める。必須Moduleは設定やManifestから除去できない。とくにSecurity Broker、Cryptography、Audit finality、Credentials、OS controls、Permission enforcementの安全境界は、製品Module分割時にも削除・迂回してはならない。

Machine-readable `unprunable_core_ids`はSecurity Broker、Cryptography、Audit finality、Credentials、OS controls、Permission enforcement、Approval enforcement、Content Exposureを保持条件として列挙する。これらは選択・除外対象ではなく、すべての将来のbuild／prune実装が維持すべき不変条件である。

## 任意Module選択

`gui_shell_export.schema.json`の`module_selection.optional_module_ids`は任意画面の選択だけを表す。fieldがない旧要求は互換性維持のため任意Moduleをすべて選択したものとして扱う。fieldがある場合は、一覧中の任意Moduleだけを受け付け、重複、未知Module、必須Moduleの指定、一覧不整合は拒否する。

選択されたModuleが他Moduleへ依存するとき、Brokerは依存先を再帰的に含める。一覧内の依存不整合、循環、未知pathは失敗として拒否し、未選択Moduleを自動的に権限やCapabilityへ昇格しない。

## Receiptと実除去の境界

書出しReceiptの`module_plan`は、一覧版、明示／既定選択、必須・包含・除外IDを記録する。`binary_pruning_status`は常に`not_applied`である。これはBroker内部状態とManifest計画の証拠であり、ソース、実行可能file、package、Rust Moduleが除去された証拠ではない。

任意Moduleの選択はauthorityではなくUI構成である。Capability、Permission、Approval、Credential、Audit identity／chainを付与・継承しない。Module実除去へ進むときは、実際のbuild経路へManifestを接続し、安全境界保持のnegative testを追加し、比較対象commit／toolchain／対象artifactを固定してサイズ、cold startup、resourceを計測する。

## 未成立範囲

- Manifest selectionからFlutter／Rustのcompile対象を絞るbuild pathは未接続。
- 現行Desktop Export UIは通常Broker資格を使うが、Export操作はOwner資格を要求するため、GUIからのOwner操作経路が未成立。
- 実binaryのModule除去、差分サイズ、cold startup、resource比較は未測定。
- Installer、署名、実起動、配布は別工程として扱う。

上記は開発自体を止めない`development_blocker`ではなく、該当製品成果とreleaseを止める`release_blocker`である。未成立を隠してPhase 33やreleaseを完了扱いしない。
