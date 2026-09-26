# GUI Shell Windows書出しの意味正本

## 目的

GUI Shell Windows書出しは、D4 Pocketの構成から将来の独立App用Manifest fileを生成する。現行成立範囲は、Brokerが新規App identityと将来用Audit store identityを割り当て、設定、Runtime／Adapter構成、Capability requirement、配布metadata、Module計画をJSON fileとして固定Export directoryへ保存するところまでである。これは実行可能App、物理Audit store、独立Runtime、Installerの生成ではない。

## Broker経路

`GUI Shell書出し`要求をRust Brokerで再検証する。要求modeは`manifest_file`、対象platformはWindowsに固定する。任意画面の選択はGUI入力にすぎず、Brokerが機械可読なModule一覧を照合し、必須Moduleと依存閉包を再計算する。任意の保存pathは要求payloadから受け取らない。Rust起動器が検査して開いた`%LOCALAPPDATA%\\GUI-Shell\\broker\\desktop\\exports` directory handleをBrokerへ渡し、Brokerは128-bit新規App IDからfile名を導出する。fileは上限64 KiB、symlink追跡なしの一時fileへ書き、flush後にhard-linkで新規確定する。既存fileを置換しない。Receiptはfile名、絶対path、SHA-256、byte長、Manifest内容を返す。

Brokerはfile生成前に要求受理Auditを記録し、file確定後にManifest file hashをpayload hashとして完了Auditへ結ぶ。一時fileの後片付けに失敗しても確定済みManifest fileを失敗扱いせず、Receipt／Auditへ`cleanup_pending`と一時file名、hash照合後にOwnerが確認・削除するRecoveryActionを記録する。完了Auditに失敗した場合は成功Receiptを返さず`Suspended`とし、既に生成された可能性があるfileのpath／hashと照合RecoveryActionをエラーへ示す。自動上書き・削除・再試行は行わない。Flutterはfileやdirectoryへ直接アクセスしない。

Desktopの`BrokerClient`は通常資格だけを利用する。Ownerが設定画面で書出しを開始すると、Rust起動器が要求の構造、`desktop_flutter` metadata、Broker session、現在時刻、payload hash、Compose Manifest、Module計画を確認し、hashを含むWindowsネイティブ確認を表示する。表示値は検証済みManifestから射影し、任意payloadやCredentialは表示しない。拒否／未確認は通常Broker経路へ戻してOwner不足として監査し、許可だけがcapacity-1 process内Rust channelを通ってBroker所有threadへ届く。Brokerは同じ要求のsession、鮮度、nonce/replay、payload hash、Authority metadataを再検証する。共有channelの許可operationはこのExportと、別contractで定義する`回帰Case削除`／`回帰Case削除中断確認`の固定allowlistに限り、他のOwner操作へ一般化しない。

Owner資格、privileged IPC、Owner role flagをFlutterへ渡さず、Rust起動器は資格fileも生成しない。確認はWindows session上の明示的な人間操作記録であり、Windows accountの再認証・本人性証明ではない。確認要求はBrokerの300秒鮮度期限を超えると失効する。ネイティブ確認の応答待ちは通常IPCより長いが有限のtransport期限を持つ。

## 継承禁止

書出し元のAuthority、Permission、Approval、Credential、Audit chainは継承しない。Manifest内のApp identityとAudit store identityは新規で、`authority_strip=true`、各`*_inherited=false`、`inheritance_policy`の各値`none`を返す。これらは識別子の割当てであり、実行可能Appや物理Audit storeの生成を意味しない。Capability requirementは必要機能の説明であり、Permissionではない。

Brokerが生成するApp IDは`d4-pocket-app-`に小文字hex 32桁を続けた形式、Audit store IDは`audit-store-`に小文字hex 32桁を続けた形式とする。ReceiptとManifest fileのSchemaはこの形式を検査する。識別子は新規構成を対応付ける値であり、信頼、Permission、Approval、Credentialを表さない。

## 未成立範囲

`manifest_file_status=written`は設定Manifest fileの作成だけを表す。一時file状態は`removed`または`cleanup_pending`として復旧情報と組で明示する。Broker Receiptの`build_status=not_started`、`artifact_status=not_built`、installer未開始、署名なしを固定する。Module計画の`binary_pruning_status=not_applied`も固定し、Manifest上の除外をbinaryからの削除へ読み替えない。Rust起動器にはcompile-time identityで製品別runtime／Audit store pathを選ぶ実装を追加した。別途、開発者が明示実行するWindows bundle build toolを追加したが、Broker Receiptの状態は変更せず、実行可能な独立製品・正式配布を主張しない。clean pushed sourceからのbuild実行と実出力は、別の検証記録に結び付ける。

## 開発専用Windows bundle build

`tooling/export_windows_product.py`はWindows専用の開発用build pathである。clean `main`を要求し、GitHub `origin/main`の読取専用照会で同一commitを確認してから、そのcommitを`git archive`でRepository／OneDrive外の短い一時pathへ展開する。展開器はroot外path、Windows上のcase衝突、file／directory衝突、symlink等を拒否する。Flutter／PubとCargoへ渡す環境変数はallowlist化し、Pub cache、Cargo home、Cargo targetを一時領域へ分離する。

入力ReceiptとManifest fileはそれぞれ現行Schemaに照合し、Export ID、内容、App ID、Audit store ID、Manifest file名、byte長、SHA-256の一致を再検査する。Receiptの出所・署名・Owner操作の真正性は検証しない。Manifest内の生成済みApp ID／Audit store IDはRustのcompile-time build環境へ渡し、Cargo target directoryは両IDの組から作るSHA-256値で短く一意に分離する。MSVC linker出力pathの上限を240 UTF-16文字として構築前に検査し、超過時はCargo実行を停止する。選択Moduleは既存Catalogと依存閉包からFlutter compile-time defineへ写像する。出力は`gui_shell_desktop_launcher.exe`、Flutter Release一式、`broker/gui_shell_rust_helper.exe`、入力Manifestのbyte-for-byte copy、およびbuild evidenceからなるunsigned portable directoryで、既存outputを上書きせず同一volume上で一括公開する。Artifact inventoryはbuild evidence自身を除く全fileのpath、size、SHA-256を記録し、公開直前に再検証する。

build evidenceは`INTERNAL_STATE`で、Owner／source authority未検証、Credential値の明示投入なし、Credential artifact scan未実施、runtime Manifest未消費、製品起動未検証、binary pruning未検証、未署名、Installer未開始、正式配布主張なしを固定する。したがってこのbuild pathはGUI-Shell approval、runtime manifest consumption、Credential不存在、独立Runtime／Audit store、実製品安全Core保持、性能、installed `LIVE_RUNTIME`を証明しない。OwnerのYes操作、署名鍵、formal app identity、Installer／更新／rollback経路は行わない。実desktop起動、別Windows profileでのformal evidence、Credential／authority内容監査、pruning positive／negative testと計測、正式配布は`release_blocker`として継続する。
