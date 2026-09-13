# Windows保護bytes境界

C2内容履歴とC7資格保管が共有するOS接続の前提単位。WindowsのDPAPI CurrentUserだけを使い、用途contextを追加entropyへ結合する。保管file・Permission・Approval・Audit・Recovery・IPC・UIを所有せず、呼出しだけでそれらの権限を発生させない。製品の保管/閲覧接続は未完了のrelease_blocker。

平文は1～65536bytes、暗号文は1～131072bytes、用途は1～1024bytes。用途には呼出し側がGUI-Shell用途・対象識別子・版を正本化して与える。外部入力だけから用途を決定してはならない。入力と出力の長さ、成功時null、OS失敗を検査する。エラーは分類とOS数値だけで、内容を出力しない。独自暗号は使わない。

## OS接続の明示レビュー

LANGUAGE_POLICYのunsafe例外レビューとして、native/windows_protection/src/lib.rsのみにFFIを隔離する。既存Rust helperのforbid(unsafe_code)は維持する。これはOSに安全なRust関数がない境界の実装であり、失敗を隠すalternate実行経路ではない。範囲はWindows bytes変換のみで、process/network/filesystem/credential列挙やGUI操作を追加しない。

入力sliceは呼出し中生存し、APIは変更しない。上限によりu32変換の切捨てを防ぐ。DPAPI出力は一つのRAII所有者が保持し、複写後・エラー帰還時にvolatile消去してLocalFreeする。復号結果はDebug/Clone/SerializeなしのSecretで保持し、drop時に所有bytesをvolatile消去する。呼出し側が作る複写、OS内部、pagefile、crash dump、process侵害までの消去保証はない。

UI抑止flagを固定し、machine共有flagとpromptを指定しない。外部bindingはMicrosoftのwindows-sys 0.61.2を固定使用し、対象featureはFoundationとSecurity Cryptographyに限る。licenseはMIT OR Apache-2.0、GUI-Shell側はApache-2.0。binding依存のunsafeを含むため、言語だけで安全を主張しない。

一次資料: [CryptProtectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata)。このAPIは同じWindowsユーザーの復号を防ぐ境界ではなく、オフライン署名やadministrator耐性を代替しない。

## 試験と残存境界

Rust helperのWindows integration testから実APIを呼ぶ。合成dataだけを使用し、正常・別用途・改変・切断・同入力の再暗号化・上限を検査する。これはOS接続試験であり保管製品の完成ではない。別ユーザー、別端末、実際の保管/削除、承認失効、監査書込失敗、snapshot漏洩、非Windowsの安全保管はrelease_blocker。非WindowsでDPAPI代替を成功扱いしない。
