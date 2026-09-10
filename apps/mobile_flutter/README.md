# モバイルFlutter app

owner rev2指示によりAndroid/iOSの正式projectを追加した。Flutter共通表示層は `../../packages/gui_shell_ui` を使用する。
現時点の既存画面は接続前の試作であり、表示される状態は実行系の観測ではない。端末連携単位で実接続へ置換する。

AndroidはFlutterが指定するSDKとJDK 17を使用する。開発試験は `flutter build apk --debug`、`flutter build appbundle --debug`。開発識別子は `com.example.gui_shell_mobile`。releaseへdebug署名を流用しない。
iOSはmacOSとXcodeでのbuildおよび端末証拠を別途必要とする。

- item: 端末連携とplatform別実動作
  classification: release_blocker
  reason: project生成はpairing・安全保管・通信・実機動作の証拠ではない。
  required_action: 端末資格とDesktop接続を実装し、各platformでbuild・起動・対話・失効を検証する。
  blocks_release: yes
- item: 公開配布の識別子と署名
  classification: release_blocker
  reason: 現在の識別子は開発用であり、所有者の配布署名は未設定。
  required_action: 公開配布前に所有者の正式識別子と署名で検証する。
  blocks_release: yes
