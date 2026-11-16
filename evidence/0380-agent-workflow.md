# 0380 の記録: Actions の作業用サブエージェントの骨組み(2026-11-16、この機械)

決定: `docs/decisions/0380-agent-workflow.md`。以下は、テストの実出力の写し。stub の opencode を使ったテストは、配線のテストであり、実エンジンの実行ではない。

## 1. plan.sh(一時リポジトリ。擬似の入力。ワークフローの plan ジョブと同じ呼び出し)

```
$ bash ci/agents/plan.sh <c1> <c2>          # a.md 変更、c-1.md 追加、notes.txt(.md でない)変更
plan: 2 task(s) to run, from 2 changed
tasks=["a","c-1"]
count=2

$ bash ci/agents/plan.sh 0000000000000000000000000000000000000000 <c2>     # 初回(before が 0): 全件
plan: 3 task(s) to run, from 3 changed
tasks=["a","b","c-1"]
count=3

$ bash ci/agents/plan.sh <c2> <c2>          # 変更なし
plan: 0 task(s) to run, from 0 changed
tasks=[]
count=0

$ bash ci/agents/plan.sh <c4> <c5>          # 61 件を追加
::warning::タスクが 61 件。先頭 60 件だけを実行し、残り 1 件は実行しない(実行するには、そのファイルを変更して push し直す)
plan: 60 task(s) to run, from 61 changed
(rc 0。先頭 60 件に t01 と t60 を含み、t61 を含まない。tasks= の JSON は jq -e で解析できた: 長さ 60)

$ bash ci/agents/plan.sh <c2> <c3>          # ファイル名 -bad.md(先頭が -)
::error::タスクのファイル名が使えない形式(英数字・ . _ - のみ。先頭は英数字。63 文字まで): -bad
(rc 1。GITHUB_OUTPUT に何も書かれない)

$ env -u GITHUB_OUTPUT bash ci/agents/plan.sh 0000000000000000000000000000000000000000 <c2>   # GITHUB_OUTPUT 無し
tasks=["a","b","c-1"]
count=3
```

## 2. check-credentials.sh(HAS_* は真偽だけ)

```
HAS_REGION=false(他は false)  → ::error::Secret AWS_REGION が登録されていない(…) / rc=1
HAS_REGION=true のみ          → ::error::モデルの認証情報が登録されていない。…  / rc=1
方式 A(HAS_BEARER=true)       → モデルの認証: 方式 A(API キー) / rc=0
方式 B のキー ID のみ          → ::error::モデルの認証情報が登録されていない。…  / rc=1
方式 B(キー ID と秘密)        → モデルの認証: 方式 B(アクセスキー) / rc=0
```

## 3. runner-config.sh と check-agents.sh(CI と同じ形の一時ツリー: 追跡されている opencode.jsonc と .opencode/plugins/、runner の設定を .opencode/opencode.jsonc に置く。global の設定は読まれ得る)

```
$ bash ci/agents/runner-config.sh <一時の path>
runner config written to <一時の path> (model settings of 5 agents; the permissions come from opencode.jsonc)
rc=0    (jq -e . で妥当)

$ bash ci/agents/check-agents.sh            # 上の一時ツリーで
モデル: build・worker・reviewer・title・summary の 5 件が Bedrock の Haiku 5.5(variant なし)
worker: 既定のエージェント(primary)として動ける
worker の拒否の規則: wrangler・cf・gh secret・gh variable・gh api(secrets/variables)・force push・git clean の 8 件を確認
rc=0

$ bash ci/agents/check-agents.sh            # runner の設定を置かない(リポジトリの設定だけ)
::error::モデルが Bedrock の Haiku 5.5 になっていない: build reviewer summary title worker
rc=1
```

注: 負のテストで「拒否の規則を 1 件消す」は、この機械では通った(rc 0)。同じ規則が、リポジトリの外の設定の層にもあると考えられる(未確認)。CI では、その層は無い見込み。規則の有無の論理は、`jq` の式で別に確かめた(存在しない規則 → 偽、存在する規則 → 真)。

## 4. install-opencode.sh(OPENCODE_INSTALL_DIR を一時ディレクトリに向けて実行)

```
opencode 2.2.6 (x64) installed in <一時のディレクトリ>, sha512 matches
opencode v2.2.6
rc=0
cmp <導入した binary> /home/ubuntu/.opencode/bin/opencode → 同一
```

## 5. run-task.sh(一時リポジトリ。stub の opencode は配線テスト用)

```
$ bash ci/agents/run-task.sh t1          # 配線の stub
run-task rc=0
プロンプトの先頭行: <!-- worker-rules v1 -->
CI の注記: 1 件、タスクの見出し: 1 件、AGENTS.md の見出し: 2 件(grep -c の出力)
meta.txt: task: t1 / rc: 0 / started / ended / opencode: opencode 2.2.6 (stub) / commit: <短い SHA> / changed files / report bytes
RUNNER_TEMP のプロンプトは残らない(0 件)

秘密の形の文字列を出力に入れた場合(stub の STUB_LEAK):
  scan-forbidden.sh ci/agents/out/t1 → secret 1 件以上 → rc 1(出力は成果物・サマリーに出ない)
  比較用の一時ファイル(AKIA + 大文字 16 字以上の行 1 行): scan-forbidden: 1 finding(s) → secret、rc 1
```

## 6. GitHub Actions(agent.yml)

```
$ gh run list --workflow agent.yml --limit 3
completed  success  Merge remote-tracking branch 'origin/main'  agent  main  push  43785370242  9s  2026-11-16T01:17:17Z

plan ジョブのログ(gh run view --job <id> --log):
plan: 0 task(s) to run, from 0 changed
run ジョブ: skipped(matrix を作らない)
```
