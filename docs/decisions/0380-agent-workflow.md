# 0380: Actions で作業用サブエージェント(worker)を並列に動かす骨組み
分類: (b) API 拡充(GitHub Actions で `opencode run` を 1 タスク 1 ジョブで動かす契約の実装。新しい判断の処理は、認証情報が無いときの停止と、出力の検査だけ)
撤回条件: (1) 認証情報を登録した後の最初の実行で、`opencode run` の仕様が本決定と違うと実測されたら、ワークフローを直すか止める。(2) 出力の検査を通った出力に、秘密や禁止語があったと分かったら、検査を直すまで run ジョブを外す。(3) 並列上限 60 が GitHub の仕様と違うと確かめられたら、上限を直す。
日付: 2026-11-16   ROADMAP: 対象外(依頼者の設計 B。ROADMAP に行は無い)   一覧: —   コミット: 本決定と同じ commit 群(`git log --oneline -- .github/workflows/agent.yml`)

## 決定
- 決定: `ci/tasks/<名前>.md`(英数字・`.`・`_`・`-`)を push すると、ファイルごとに 1 ジョブ(matrix・fail-fast 無効・max-parallel 60・timeout 350 分)で、作業用サブエージェントを `opencode run` の 1 回として動かす。根拠: 依頼者の設計 B(GitHub Actions で最大 60 並列)。
- 決定: 1 push の対象は最大 60 件。超えた分は実行せず、警告を出す。根拠: 並列上限。切り捨てを隠さない。
- 決定: 初回の push(before が 0)は `ci/tasks/<名前>.md` の全件を対象にする。根拠: 差分の基準が無い。
- 決定: 認証情報(Secret `OPENCODE_AUTH_JSON`)が無いジョブは、取り込みの手順(`ci/agents/import-login.sh`)で止まる。値は表示しない。根拠: 登録前に run を動かさない(Issue 0005。決定 0383)。
- 決定: モデルは、依頼者の console の BYOK の Haiku 5.5 だけ(提供元は console が返す。既定の参照は `console-aws-bedrock/` の下の Haiku 5.5 で、モデル ID は部品から組み立てる)。runner の設定で、使う 5 つの agent(build・worker・reviewer・title・summary)を同じモデルにする。根拠: 役割表の Haiku 5.5(AGENTS.md §1)。runner は worker だけを起動するので、build に Sonnet を与えない。
- 決定: runner の worker は、既定のエージェント(`default_agent`)として `mode: all` で起動する。根拠: `opencode run` は既定のエージェントで動く。subagent は子セッションでしか動かない(https://opencode.ai/docs/agents/)。
- 決定: 権限は、リポジトリの `opencode.jsonc` だけを出所にする。runner の設定には写さない。根拠: 規則は最後に一致したものが勝つ。runner の設定に allow を足すと、後の拒否を上書きする。写すと、`git clean -n*` のような許可の例外の順序が変わる。確かめは `ci/agents/check-agents.sh`(拒否の規則が有効な設定にあるか)。
- 決定: 出力(報告・stderr・差分・メタ情報)は、`scripts/scan-forbidden.sh` が通ってから、成果物とサマリーに出す。通らなければ、どちらにも出さず失敗する。根拠: Actions のログは公開。
- 決定: runner では push も commit もさせない(checkout は `persist-credentials: false`、プロンプトに注記)。差分は `changes.patch` という提案として保存する。
- 決定: Cloudflare の Secrets・Variables は、このワークフローに渡さない。Secret の名前は `OPENCODE_AUTH_JSON` だけ。根拠: エージェントに Cloudflare の資格情報を渡さない。

## 実測(コマンド・条件・日付・n)
- 2026-11-16 `opencode --version` → `opencode v2.2.6`(この機械)。
- 2026-11-16 `npm view @opencode/cli-linux-x64@2.2.6 dist.integrity` の値の tarball を `curl` で取得し、sha512 を計算 → 一致(n=1)。展開した binary の sha256 は、この機械の `/home/ubuntu/.opencode/bin/opencode` と一致(n=1)。baseline(`cli-linux-x64-baseline`)も同様に一致(n=1)。
- `ci/agents/install-opencode.sh`(`OPENCODE_INSTALL_DIR` を一時ディレクトリに向けて実行)→ `sha512 matches`、`--version` → 2.2.6、導入した binary はこの機械の binary と `cmp` で同一。
- `ci/agents/plan.sh`(一時リポジトリ、擬似の入力。n=7): 差分 1 件の変更と追加 → `["a","c-1"]`・count 2。before が 0 → 全件 `["a","b","c-1"]`・count 3。変更なし → `[]`・count 0。ファイル名 `-bad` → rc 1、出力は書かれない。61 件 → 60 件・警告 1 件。`GITHUB_OUTPUT` 無しでは標準出力に書く。
- `ci/agents/runner-config.sh` → `jq -e .` で妥当。5 つの agent が `amazon-bedrock`。worker の mode は all。variant は無い。
- `ci/agents/check-agents.sh`(CI と同じ形の一時ツリー: 追跡されている `opencode.jsonc` と `.opencode/plugins/`、runner の設定を `.opencode/opencode.jsonc` に置く)→ rc 0。モデル 5 件・worker の primary・拒否の規則 8 件を確認。
- `ci/agents/run-task.sh`(一時リポジトリ。stub の opencode は配線のテスト用で、実エンジンではない)→ プロンプトは規則のマーカーで始まり、CI の注記・AGENTS.md・タスクを含む。`meta.txt` は rc 0。プロンプトの一時ファイルは残らない。秘密の形の文字列を出力に入れると、`scan-forbidden.sh` が `secret` で rc 1。

## 読み(解釈。実行していない)
- `opencode run "<メッセージ>"` の形は公式文書にある(https://opencode.ai/docs/cli/)。フラグは使わない。`opencode run --help` は、この機械の policy で拒否されたため、フラグは確かめていない(未確認)。
- 組み込みのプロバイダー ID は `amazon-bedrock`(文書の例と、バイナリの文字列)。この機械の設定のプロバイダーは別の名前で、CI には無い。
- `.opencode/plugins/model-pin-gate.ts` は、セッションのモデルが agent の pin と違うとツールを止める。runner は top-level の `model` も worker と同じにして一致させる。`worker-rules.ts` は subagent の呼び出しにだけ効くので、runner は規則を prompt に自分で含める。

## 未確認
- 認証情報の登録後の `opencode run` の実動作(Bedrock の呼び出し、モデル ID の解決、`models` の空の項目の扱い、非対話時の permission の扱い)。
- `opencode run` が `mode: all` の worker を既定のエージェントとして受け付けるか。
- 60 並列での Bedrock の上限(quota・レート制限)。
- GitHub の matrix の上限(並列 60 は依頼者の設計の前提。公式文書での確認は R8-5 の行の範囲)。
- この機械で、`opencode debug agents` の結果に設定の変更が遅れて反映される現象があった(共有のサーバーが設定を保持するため)。CI は毎回新しい環境なので影響しない見込み。未確認。
- 拒否の規則を repo から消したときに、CI の検査が落ちること。この機械では、別の層の同じ規則が残って通るため、確かめられなかった。

## 出典
- 旧: `archive/ci-old/agents/install-opencode.sh`(版と sha512 を更新)・`summary.sh`(同一の写し)。`docs/ops/models.md`・`scripts/set-model-profile.sh`(Bedrock の固定の設定)。
- 公式: https://opencode.ai/docs/cli/(`opencode run`)、https://opencode.ai/docs/providers/(Bedrock)、https://opencode.ai/docs/agents/(mode・default_agent・subagent)。
- npm: `@opencode/cli-linux-x64` と `@opencode/cli-linux-x64-baseline` の 2.2.6(`dist.integrity`)。
- 関連: 決定 0379(Worker の配信のワークフロー。別の作業)。
- 注(2026-11-16): 依頼 Issue 4(依頼者に別の認証情報を作らせる内容で誤りだった)は取り下げた。GitHub の Issue 4 は、2026-11-16 の確認の時点で GitHub から消えていた(消した人は未確認)。認証は、依頼者の opencode のログイン情報の取り込み(Secret `OPENCODE_AUTH_JSON`)に置き換えた(決定 0383)。依頼者の指示の出典: 発言 msg_1b854db83001HV82xjrIrfn2e8(2026-11-08 08:25:30。要約: ~/work/handover/setup/S16-intent/catalog.md の 3・14・15 行)。分類: (b) API 拡充(認証情報の取り込みの契約の実装)。旧方式の実測の記録は evidence/0380-agent-workflow.md に残す。
- 注(決定 0384): モデルは Variable `OPENCODE_MODEL` で指定する。上の「モデルは、依頼者の console の BYOK の Haiku 5.5 だけ」の決定は、決定 0384 で置き換えた。
