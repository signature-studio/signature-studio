# 0017: 最新の版を調べずに版を書いた。版の選択を、記憶・既存の行・後の自動更新に任せた

- 日付: 2026-11-16
- 誰が: メインセッション(版の指示。PM 経由の情報)と worker(版を書いた commit の作成者は未確認)。対象の commit は aba137b(deploy.yml の 40 桁の SHA)と 6636b8a(v4.1.3 の指定)
- 依頼者の発言: 2026-11-16。発言 ID は未確認(main 経由で届いた原文をそのまま写す)。
  - 「あなたは何度言っても最新のバージョンを使用しないため、私がdependabotを導入しました。dependabotのPRは全てマージしてください。」
  - 「なぜわざわざ古いバージョンを？」「知らなかったわけではないでしょうに」「知ってて、あえて古いバージョンを選択したのですよね？古いバージョンを選択せざるを得ない、強い理由があったのですよね？」「あなたは既に発言しており、あなたの主張は取り消せません。さぁ、あなたの考える『強い理由』とやらを発表してください！」「いえ、ですから、意味不明なのですが」

## 何が起きたか

- (a) aba137b の deploy.yml の cloudflare/wrangler-action は、40 桁の SHA `953926a2e2182532811c01a25e53647d93bf07c0` の固定だった(`git show aba137b:.github/workflows/deploy.yml` の 59・74・93 行)。旧の `aba137b^:.github/workflows/deploy-worker.yml` の 295・334 行も同じ SHA で、行末に `# v4.1.3` のコメントが付いていた。`gh api repos/cloudflare/wrangler-action/commits/v4.1.3` の sha は同じ値。書いた時点で最新を調べた記録は無い(未確認)。結果として、その時点の最新と同じ commit だった。
- (b) PM 経由の情報では、1 行の長さ(81 文字)が問題になったとき、main が worker に「cloudflare/wrangler-action@v3 のタグ指定にして(dependabot が最新に上げる)」と指示した。v3 は最新ではない(`gh api repos/cloudflare/wrangler-action/releases/latest` は v4.1.3)。`git log --all -S 'wrangler-action@v3'` は 0 件で、v3 はどの commit にも残っていない。指示の原文と、81 文字の行の所在は未確認。その数分前に dependabot の PR で actions/upload-artifact(#9・58341ae)・actions/setup-node(#10・5cf6888)・actions/checkout(#11・2a0c2d6)が 4 から 7 に上がり、マージされていた(evidence/R6-24.md の 20〜22 行)。
- (c) worker が最新を調べ、v4.1.3 を指定した(commit 6636b8a「deploy: wrangler-action をタグ v4.1.3 で指定(80 文字以内)」。HEAD の deploy.yml の 44 行)。
- (d) 説明の変遷(「記憶で書いた」→「知っていた・理由なし」→「最新を調べる作業を dependabot に任せた」)は PM 経由の情報。原文は未確認。

## 原因(その時の判断)

- 版を、最新を調べずに決めた。(a) は旧ファイルの行を写した。(b) は指定の版を、最新を調べずに指示した(PM 経由)。後で dependabot が上げることを、版の選択の代わりにした。
- 依頼者が dependabot を入れた目的は、最新の版を使わせることだった(依頼者の発言の 1 文目)。版を後の自動更新に任せる使い方は、その目的に反する。
- 古い版を選ぶ強い理由は、commit・記録のどこにも無い。PM 経由の説明(d)も、理由を示していない。

## 影響

- 依頼者の指摘(指示の是正)。依頼者の時間。依頼者が dependabot を入れた目的から外れた。
- v3 が配信・実行に使われたかは未確認(commit に残っていない)。
- 現在の deploy.yml の wrangler-action は v4.1.3(最新。gh api)。ほかの版の不一致は下の表(古い 7 件)。

## 是正

- 6636b8a で wrangler-action を v4.1.3 に指定した(worker)。
- 本記録で、版の探索の表を作った(下の「同じ型の探索」)。版の更新そのものは、この記録では行わない(別の依頼)。
- 規則(この記録の決定): 版を書くときは、その場で `gh api repos/OWNER/REPO/releases/latest`(または tags)で最新を調べて書く。AGENTS には未反映(未実施)。

## 再発防止

- 文書: 上の規則を AGENTS に入れる(未実施)。
- 検査の案(未実施): 版を書く行(uses:・node-version・npm の版・OPENCODE_VERSION)が変わるとき、gh api の最新と照合する検査。

## 同じ型の探索

- 範囲: HEAD 22f1c0e。ワークツリーには他の作業の未 commit の変更があるため、`git grep` と `git show` で HEAD を読んだ。
  - `.github/` の `uses:` 9 行、`runs-on:` 4 行、`node-version:` 1 行(`git grep -n -E 'uses:|runs-on:|node-version:' 22f1c0e -- .github`)。`uses:` のうちローカル参照 2 行(agent.yml 54・82)は版が無いので表から外した。
  - `app/package.json` の devDependencies 6 件・dependencies 2 件。版は `app/package-lock.json` の `node_modules/<名>` の version、最新は `npm view <名> version`。
  - `ci/agents/install-opencode.sh` の OPENCODE_VERSION(5 行)。最新は `npm view @opencode/cli-linux-x64 version` と `npm view @opencode/cli-linux-x64-baseline version`。
  - `ci/versions.env`: 読めなかった(read が権限で拒否された)。全エントリは未確認。ツールチェイン(EMSDK_VERSION・RUST_TOOLCHAIN)は ci/setup-toolchain.sh が ci/versions.env から読む。値は未確認。binaryen は emsdk に同梱(ci/setup-toolchain.sh の 3 行)。
  - dependabot の対象: `.github/dependabot.yml` の 3 本(github-actions・cargo の /lift・npm の /app)。npm の /app は対象に入っている(11〜14 行)。依頼文の「dependabot の対象外」は当たらない。
  - nodejs/node の `releases/latest` は alpha(v27.0.0-alpha.0.1.0)を返した。安定版は `releases?per_page=60` の prerelease が偽の物から取った。
- 表(判定の定義: 「古い」= 指定または lock の版が最新より前。同じメジャーのタグ指定は古くないとする):

| 対象 | 場所 | 指定 | 最新 | 古いか |
|---|---|---|---|---|
| actions/checkout | agent.yml 29・49・77、deploy.yml 22 | v7 | v7.0.1 | 古くない(同じメジャー) |
| actions/setup-node | deploy.yml 27 | v7 | v7.1.0 | 古くない(同じメジャー) |
| actions/upload-artifact | agent.yml 127 | v7 | v7.0.2 | 古くない(同じメジャー) |
| cloudflare/wrangler-action | deploy.yml 44 | v4.1.3 | v4.1.3 | 同じ |
| node-version | deploy.yml 29 | "24" | 26.13.1(安定版の最新の主版) | 古い |
| runs-on | agent.yml 22・45・68 | ubuntu-latest | 未確認(gh api で対応先が取れない) | 未確認 |
| runs-on | deploy.yml 19 | ubuntu-24.04 | 未確認 | 未確認 |
| wrangler | app/package.json | ^4.156.0(lock 4.156.0) | 4.162.0 | 古い(lock) |
| @cloudflare/workers-types | app/package.json | ^5.20261101.1(lock 同じ) | 5.20261116.1 | 古い(lock) |
| @types/node | app/package.json | ^26.6.6(lock 同じ) | 26.6.6 | 同じ |
| typescript | app/package.json | ^7.0.2(lock 同じ) | 7.0.2 | 同じ |
| vite | app/package.json | ^8.3.5(lock 同じ) | 8.3.8 | 古い(lock) |
| vitest | app/package.json | ^5.0.5(lock 同じ) | 5.1.0 | 古い(lock) |
| fflate | app/package.json | ^0.8.3(lock 同じ) | 0.8.3 | 同じ |
| file-selector | app/package.json | ^5.0.2(lock 同じ) | 5.0.3 | 古い(lock) |
| @opencode/cli-linux-x64・-baseline | ci/agents/install-opencode.sh 5 行 | 2.2.6 | 2.3.0(両方) | 古い |
| EMSDK_VERSION・RUST_TOOLCHAIN | ci/versions.env(ci/setup-toolchain.sh が読む) | 未確認 | 未確認 | 未確認 |
| ci/versions.env の全エントリ | ci/versions.env | 未確認(読めない) | 未確認 | 未確認 |

- 件数: 表の対象は全 21 件(uses の外部 7 行・node-version 1・runs-on 4・npm 8・OPENCODE_VERSION 1)。古い 7 件(node-version 1・npm 5・OPENCODE_VERSION 1)。古くない・同じ 10 件。未確認 4 件(runs-on)。ツールチェイン 2 件と ci/versions.env は未確認(件数も未確認)。

## 未確認

- 依頼者の発言 ID(本日分)。
- aba137b の版を書いた時点の判断(最新を調べたかの記録は無い。SHA は v4.1.3 と一致)。
- v3 の指示の原文と、81 文字の行の所在(commit に v3 が無い)。
- ci/versions.env の全エントリとツールチェインの版(read が権限で拒否された)。
- runs-on の対応先(ubuntu-latest・ubuntu-24.04)の最新。
- 版の更新の影響: ツールチェイン(emsdk・binaryen・rust)の更新は wasm の出力に影響する。更新は出力の同一性の検査とセットにする(別の依頼)。
