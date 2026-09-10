# GUI Shell の運用モデル

状態: Phase B の owner-use は完了。completed product release は未主張
参照様式: BLUE-TANUKI の direct-main owner workflow
適用範囲: repository の作業流、安全姿勢、validation、backup、報告

## 1. 中核姿勢

GUI Shell は control plane であり、visual wrapper ではない。

repository は次の順序で進めなければならない。

~~~text
standard
  -> schema
  -> conformance
  -> Shell Core
  -> Runtime Catalog
  -> Agent Runtime Contract
  -> adapter
  -> Rust helper
  -> Shell Core persistence / audit chain
  -> desktop product UI
  -> installer / update
  -> v1.0 release gate
  -> post-v1.0 mobile companion
~~~

後続 Phase は、先行する保証を弱めてはならない。

## 2. 優先順位

1. 安全性
2. 堅牢性
3. operator にとっての明瞭さ／UX
4. 製品機能
5. 利便性

feature coverage と利便性は、authority strip、content exposure、approval、audit、recovery、schema validation を弱める正当な理由にならない。

## 3. 境界モデル

~~~text
Runtime
  -> Adapter
      -> schema validation
      -> authority strip
      -> content exposure policy
  -> Shell Core
      -> permission
      -> approval
      -> audit
      -> recovery
  -> UI
      -> display
      -> operator input
  -> Rust helper
      -> bounded native operation
~~~

UI は action を要求し、state を表示できる。authority を付与することはできない。

Adapter metadata は説明できる。permission を付与することはできない。

memory、cache、previous state は UX の参考にできる。それ自体で authority を付与することはできない。

## 4. repository state を完了させる作業流

GUI Shell は、2世代の direct-main backup flow を使用する。repository state を変更する task は、implementation、validation、commit、push、remote HEAD verification、2世代 backup verification のすべてを閉じるまで完了ではない。

<code>main</code> 上で完了対象の work block に関する file を変更する前に、現在 push 済みの state を確認する。

~~~bash
git fetch --prune origin
git status --short --branch
~~~

<code>main</code> が clean でなく、<code>origin/main</code> とも整合していない場合は、編集前に不整合を解消するか blocker を報告する。

次に local recovery branch を rotate し、現在 push 済みの変更前 state を保存する。

~~~bash
# If codex/backup-main already exists:
git branch -f codex/backup-main-prev codex/backup-main

# Always update the latest backup to the current pushed main:
git branch -f codex/backup-main main
~~~

backup generation は remote branch ではなく remote tag として push する。

~~~bash
git push -f origin \
  codex/backup-main-prev:refs/tags/codex/backup-main-prev \
  codex/backup-main:refs/tags/codex/backup-main
~~~

GitHub は push された backup branch を通常 branch として扱い、pull request candidate として表示し得る。remote tag なら、pull request candidate を作らずに off-machine recovery を提供できる。

その後 implementation と validation を行い、<code>main</code> へ直接 commit し、credential を利用できる場合は <code>main</code> を push する。

push 後は remote state を検証する。

~~~bash
git rev-parse HEAD
git ls-remote origin refs/heads/main
git ls-remote --tags origin codex/backup-main codex/backup-main-prev
git status --short --branch
~~~

remote backup branch が既に存在し、owner が remote backup の保持を明示的に要求していない場合は、<code>main</code> が clean かつ整合した後に削除する。

~~~bash
git push origin --delete codex/backup-main codex/backup-main-prev
~~~

owner がその exact emergency handoff を明示的に要求した場合に限り、backup branch を remote branch として push する。この例外を使った場合、GitHub がそれらを pull request candidate として表示し得ることを報告し、backup branch から pull request を open／merge しない。

repository が保持するのは、正確に2本の local backup branch と2本の remote backup tag である。

~~~text
codex/backup-main
codex/backup-main-prev
refs/tags/codex/backup-main
refs/tags/codex/backup-main-prev
~~~

Phase ごとの backup branch または追加の backup generation を作成しない。

## 5. validation の gate

開発と品質判定はローカル作業ツリーで行う。GitHub は完成した局所成果の記録・共有面とし、各単位で試験、差分監査、commit、push、remote HEAD 確認を完了する。自動 CI と CI 必須 status check は使用しない。

GitHub Actions は `workflow_dispatch` のみの手動補助に限る。ローカルにない OS の build、artifact 生成、重い手動検証の必要性を ROADMAP または phase instruction に記録する。実行時は理由、対象 commit、trigger、結果、artifact、証拠範囲を報告する。Actions 成功は実機動作、製品完成、release readiness、owner GO の代替にならない。

conformance は YAML の構造を解析して `workflow_dispatch` 以外の起動、重複鍵、不正形式を拒否する。解析依存は dev-only の `requirements-dev.txt` に固定し、`python -m pip install -r requirements-dev.txt` で準備する。これは CONFIG 証拠であり、GitHub 側の branch protection や実行権限の監査を証明しない。

validation の基準は owner／Codex が明示的に実行する local validation、smoke、release verification、Windows device evidence である。要求ごとに production path、正常、境界、失敗、security / authority negative case、回帰、実行試験を対応させる。

最低限の validation:

~~~bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
~~~

<code>python</code> が利用できない場合の代替:

~~~bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
~~~

条件付き check:

~~~bash
cd native/rust_helper && cargo test
cd apps/desktop_flutter && flutter analyze
~~~

各 command を passed、failed、not run のいずれかとして報告する。

## 6. 変更報告の形式

完了した各変更報告には、次を含めなければならない。

1. 概要
2. 変更 file
3. risk 分類
4. validation 結果
5. release gate 分類
6. 残存 risk
7. 作業 branch
8. commit hash、または <code>not committed</code>
9. push 結果、または <code>not pushed</code>
10. remote HEAD の確認
11. backup generation の ref と hash
12. rollback の point

## 7. release claim の規則

適用されるすべての validation gate が通過し、owner が release claim を明示的に承認するまでは、release readiness を主張しない。

現在の claim boundary は次に置く。

~~~text
CLAIM.md
~~~
