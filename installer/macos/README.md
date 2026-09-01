# macOS installer

v1後に計画する製品 installer 作業。

classification: known_limitation
reason: macOS は未検証の portability 計画対象であり、host 検証なしに対応済みと主張してはならない。
blocks_release: no

目標:

```text
Installer complete -> app launch -> experience start
```

通常の利用者に、terminal、WSL、npm、Git、port設定、runtime root 検出の手動操作を必須にしてはならない。
