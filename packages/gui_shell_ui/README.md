# 共通Flutter表示層

DesktopとMobileで共有する対話画面、応答の安全な型検査、通常broker transport契約。
接続は呼出側から注入する。ネットワーク、資格保管、owner承認、実行系への直接通信を所有しない。

`flutter analyze` と `flutter test` で表示とclient境界を検査する。fixture試験は表示の証拠に限定し、実行系の実動作を証明しない。
