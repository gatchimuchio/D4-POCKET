# Installer 状態

## 現行の製品・導入状態（2026-11-14、v5）

現行の製品要求・工程・施工指示は[2026-11-14 v5採用記録](docs/QE_2026-11-14/採用記録.md)と[現行進捗](docs/QE_2026-11-14/進捗.md)を参照する。E0の4出力着工同期は`OPEN`で、五OSの標準環境におけるinstalled製品QAは未成立。各OSの一般ユーザーが正規Installer/Packageを取得・導入・起動してサービスを利用することが製品目標。Windows所有実機の完成版最終確認以外、物理端末試験を要求せず、非Windowsは合法な標準仮想環境で検査する。Linux nativeは必須の製品対象であり、WSLや過去のhosted runを製品全体の代替証拠にしない。Release Registryは採用前観測で18件中16件`unresolved`／2件`resolved`、`release_ready=false`。以下は2026-10-22時点のrev5導入証拠とそれ以前の履歴であり、v5の製品受入れ・出荷状態を示さない。

## v5 Windows Setup source（2026-11-14）

`installer/windows/d4_pocket_bootstrap_setup.iss`と`tooling/build_windows_installer.py`を追加し、Broker Export Receipt・current main commit・Product Manifest identityに束縛したper-user Setup生成経路を実装した。Setupはrun固有temporary rootへ展開してRust Launcherを起動し、固定導入root・Start Menu・AppData・uninstall registryを直接変更しない。製品の導入・更新等は既存Broker経路へ委ねる。

この変更はSource／Build recipeの範囲。実Export bundleからのSetup実生成・Authenticode署名・別profile installed runtime・first-runからAI Task／Update／Repair／Rollback／Uninstallまでの一気通貫確認は未実施であり、未確定のPublisher identityは設定していない。W06/G6-WIN、`windows_installer_first_run_smoke`、`release_ready=false`を維持する。

## rev5時点の製品・導入証拠（履歴、2026-10-22照合）

当時の製品状態は[rev5進捗](docs/REV5_PRODUCT_PROGRESS.md)と、その時点のRelease Registryを参照する。Windows P11／P12の開発受入れ、Windows B4のstaged `r2-e2e`有限CLOSEDと、別SID等の正式installed-path証拠を区別する。B5はOPEN。正式なインストール済み製品の全条件を本書の旧段階B記録で代替しない。

Linuxは完全対応が必須。run #77（source `e7804a24`）はUbuntu 24.04／Xvfb・X11の導入済みGTK製品でtest identityと非実行test packageを使った更新・Rollback／Export／Uninstall等の限定受入れを記録する。#94の資格情報保管と#107の合成MCPは別run・別sourceの証拠であり、#107で導入lifecycleを再実行したわけではない。正式署名／配布、Wayland描画、一般Linux環境と残る製品機能は未受入れで、`linux_native_product_support`はunresolvedのまま。[F03実績文書対応](docs/QE_2026-10-22/F03_実績文書対応.md)に条件を記録する。

以下の旧段階B・v1.0の環境・状態記述は履歴として保持する。引き継いだ安全・復旧・出荷関門を弱めず、当時の通常Release `task_execution=unsupported`／`release_ready=false`、正式Q0〜Q7 `DEFERRED`を記録する。

旧段階Bの状態記録: 当時の所有者利用は完了。Installer 作業はまだ有償/製品 QC として扱わず、完成製品 release には実測済み Windows インストール先証拠が必要とされていた。

## 実装済み領域

- item: `installer/setup_doctor.py`
  classification: required_for_v1
  status: 構造化状態 check あり

- item: 依存関係の recovery 手順
  classification: required_for_v1
  status: あり

- item: `installer_grants_authority=false`
  classification: required_for_v1
  status: あり

- item: `installer_silently_approves_permissions=false`
  classification: required_for_v1
  status: あり

- item: installer 境界文書
  classification: required_for_v1
  status: あり

## リリース遮断要因

- item: Windows インストール先の初回起動証拠が欠落
  classification: release_blocker
  registry_id: windows_installer_first_run_smoke
  reason: Windows 優先の完成製品 release には、分離実行の由来、source commit、artifact hash、evidence bundle hash、UIAutomation 診断 tree を備えたインストール先の実測初回起動証拠が必要である。
  required_action: 可視表層診断 tree、broker 実測 field 由来、config path、audit directory 確認入力、installed manifest を伴う固有の staged 実行から、強化済み Windows installed smoke collector を実行する。
  blocks_release: yes

- item: Windows 環境診断のインストール先証拠が未収集
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: `windows_setup_doctor_smoke`は現行台帳で未解消。旧記録にはFlutter appの機械可読出力生成とあったが、collector注入や外部probeは通常product export経路の証拠へ転用できず、正式Windows installed-path検証は未成立である。
  required_action: インストール済み app が環境診断製品出力証拠を書き出すように、分離 Windows installed smoke を実行し、`python tooling/windows_release_evidence.py` を通過する。
  blocks_release: yes

## 既知の制限

- item: macOS packaged installer は未検証
  classification: known_limitation
  reason: macOSのP13局所受入れとhosted検証はあるが、正式なmacOS installer／署名・notarization・配布の保証は未成立。旧v1.0の環境不在記述を現在の実装不在へ読み替えない。
  required_action: macOS installer 対応を主張する前に macOS 上で検証する。
  blocks_release: no

- item: Linux native packaged installer／installed lifecycle が未完成
  classification: release_blocker
  registry_id: linux_native_product_support
  reason: Linux native Desktopは必須対象。hosted Xvfb／X11の導入済みGTK製品で#77のlifecycle等を限定受入れたが、正式配布・Wayland描画・一般環境・残る製品機能を含む全体受入れは未成立である。
  required_action: 対象distribution／architecture／desktop sessionを固定し、署名・provenanceを含むpackage、installed起動、更新・rollback・uninstall、失敗復旧、X11／WaylandのOwner確認とAuditをnative Linuxで受け入れる。具体的な不足は`release_blockers.registry.json`を正本とする。
  blocks_release: yes

## 旧段階B後のQC計画履歴（現行rev5対象を除外しない）

- item: packaged 失敗用の installer recovery 手順が欠落
  classification: post_v1_scope
  reason: 有償/製品 QC には、段階Bの所有者利用操作よりも広い installer recovery と rollback coverage が必要である。
  required_action: 有償/製品 release の前に、installer 失敗 recovery catalog、rollback note、長時間 packaging smoke を追加する。
  blocks_release: no
