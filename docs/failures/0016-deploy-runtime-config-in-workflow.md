# 0016: deploy.yml が、実行時に本番の bindings を読んで配信の設定を作る段を直書きし、設定をツリーに置かなかった

- 日付: 2026-11-16
- 誰が: 後任・メインセッション(エージェント。deploy.yml は aba137b で作られた)
- 依頼者の発言: 本日(2026-11-16)。発言 ID は未確認(PM の一覧の最新は 17:23 JST まで。main 経由で届いた原文)。「(deploy.yml の 20〜57 行を指して)これは何ですか?指示違反です。デプロイ用のwrangler.jsonなどというものは存在せず、全てのデプロイに必要な記述はツリーに含めることができます。」

## 何が起きたか
- `git show HEAD:.github/workflows/deploy.yml` の 20〜57 行(38 行。依頼文は 37 行)は、id `read` の段。実行時に Cloudflare API から 100% の版の bindings を読み、`app/wrangler.jsonc` を元に `app/wrangler.deploy.json` を node で書き出す(47〜56 行)。
- 配信の 3 箇所が `--config wrangler.deploy.json` を使う(66・81・100 行)。ツリーの `app/wrangler.jsonc` は、配信で読まれていなかった。

## 原因(その時の判断)
- AGENTS §8 の「Cloudflare の識別子(アカウント ID・ホスト名)をコミットに書かない」を、依頼者の意向を確かめずに、設定の全体をツリーに置かない根拠にした。
- 1 つの段で行数(100 行)に収めるため、設定の読み取り・検査・生成・版の戻しを直書きで持った。迂回。
- 同じ生成を担う `ci/lib/wrangler_config.py`(97 行)が既にあったが、どの workflow からも呼ばれていなかった(`git grep` で呼び出し 0 件)。作成時にそれを確かめたかは記録に無い(未確認)。

## 影響
- 依頼者の指摘(指示違反)。依頼者の求める「wrangler deploy 1 回」から遠ざかり、配信の経路が増えた。
- 依頼者の時間(同じ型の指摘の繰り返し)。

## 是正
- 依頼者の指示(本日): 設定は `app/wrangler.jsonc` に置き、配信は wrangler deploy 1 回。決定 0401 を改訂した(本記録と同じ commit)。設定の置き場は決定 0403(別の作業が作成)が定めた。
- deploy.yml の書き換えは別の worker が実施中(未確認)。

## 再発防止
- 強制・検査の案(未実施): deploy.yml に `writeFileSync`・`wrangler.deploy.json`・`node <<` を含めない検査(`git grep`)。deploy.yml の行数 100 行・行長 80 文字の検査(0400 の制約)。
- 文書: 決定の文に「依頼者の指示との照合」の行を置く(0401 の改訂で実施)。AGENTS の規則と依頼者の発言が食い違うときは、決定の文に両方を書き、main に返す。

## 同じ型の探索
- 範囲: `git ls-files` のうち `ci/` と `.github/`(archive 系を除く)29 件。2026-11-16 時点。
- (a) 実行時に設定を作る処理: 2 件。deploy.yml 47〜56 行(bindings を読み、`app/wrangler.deploy.json` を書く)。`ci/lib/wrangler_config.py`(97 行。設定を書き出す。呼び出し 0 件)。
- (b) 過剰な異常系の検査(`exit`・`die`・`miss`・`中止`・`失敗`・`return 1` を含む行。`grep -c`): 計 110 行(15 ファイル)。多い順: `ci/extract-cache.sh` 27・ci/stage-cache.sh(削除済み) 24・ci/test-stage-cache.sh(削除済み) 15・ci/stage-keys.sh(削除済み) 9・ci/test-stage-keys.sh(削除済み) 8・ci/agents/check-agents.sh(削除済み) 7。deploy.yml の検査は 4 行(40・43・50・51)と、確かめの `jq -e` 1 行(90)。
- (c) 単純なコマンドを包むシェル: 呼び出しは 15 行(`run: bash ci/…`)。内訳: `.github/actions/extract-layer/action.yml` 6・.github/actions/stage-cache/action.yml(削除済み) 2・`.github/workflows/agent.yml` 7。包まれる側: `ci/agents/summary.sh` 18 行(cat と head を包む)、`ci/setup-toolchain.sh` 27 行(rustup と emsdk の各 1 コマンドを包む)、`ci/agents/install-opencode.sh` 21 行(curl・sha512 の照合・tar・install)。
- (d) workflow の中のインラインの node・python: 1 件(`.github/workflows/deploy.yml` 33 行の `node <<'JS'`)。

## 未確認
- 依頼者の発言 ID(本日分)。
- `ci/lib/wrangler_config.py` を作ったときの判断(作成者の記録は無い)。
- 0401 の改訂後の deploy.yml が、実際に配信できるか(wrangler の既定のアカウント選択、assets の扱い)。
