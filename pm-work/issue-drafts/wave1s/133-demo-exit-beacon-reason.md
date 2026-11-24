---
draft: "133"
title: "実機の評価用プロジェクトを開いて落ちた時の、engine の終了の理由を、ビーコンから読む"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["perf-beacons", "verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の報告(2026-11-24): 評価用プロジェクト(インストーラの demo の曲。`Data\Demo projects\` の下)を開こうとすると、落ちる。

この Issue の目標は次の 3 つ。
1. 実機のビーコンの集計に、engine が終了した(または、エラーで止まった)セッションの表を足す。
2. 評価用プロジェクトを開いた時の終了の理由を、直近の実機の記録から特定する。特定できなければ、その旨と理由を書く。
3. 原因の推定と、次の Issue の候補を、報告に書く。直す Issue は、この結果の後に起票する。

## 背景

- ビーコン(`app/src/perf/beacon.ts`): 送られる理由(`reason`)は first-idle・periodic・exit・error。項目は `engine.state`(例: engine-idle・exited)、`engine.exit`(reason・code・presents・`message`: engine の最後の言葉。400 字以内。個人のパスと名前を除く。`app/src/perf/exit-message.ts`)、`error`(what・where)、`timeline`(起動からの時刻)、`pageMs`(起動からの時間)。`sid` は、ページの読み込みごとの 12 桁の 16 進の乱数(個人の識別子ではないが、表と報告には出さない)。
- 既知(`pm-work/findings/real-user-beacons-2026-11-24.md` の節 1・6): 版 b440e13a623a の 104 件(2026-11-22 18:53Z 〜 11-24 00:40Z)のうち、`engine.exit` は 5 件(`finished`・code 0)、`engine.state` が `exited` の記録は 5 件。落ちた時の記録かどうかは未確認。既存の `summary.mjs` は、版ごとの表の 1 列に、`engine.exit` の理由と code を並べるだけで、セッションごとの表は無い。
- 候補(仮説。確かめる): `HostFilesUnsupported`(ホストのファイルの解放の後の読み。#1357 の系)、`ExitProcess`(exit 0 で止まる。#1372 の Exoplanet の系)、`unreachable`(wasm の trap)、メモリ不足、スタックの上限。#1365(LockResource)・#1366・#1373 との関係を見る。
- 依頼者の実機の DPR は約 1.76(記録の 1.7647 と 1.75)。
- ビーコンの取り出し: `.github/workflows/agent.yml` の「実機のビーコンを R2 から取る」の段が、直近 3 日の記録を `lift/out-app/host-extra/perf/` に置く(commit はしない)。

## 読む物

- `tools/perf-beacons/summary.mjs`(今の表。引数の扱い、`--day`、個人の値を出さない方針)
- `app/src/perf/beacon.ts`・`app/src/perf/exit-message.ts`・`app/src/perf/failure-beacon.ts`(項目の意味と、個人の値の除き方)
- `pm-work/findings/real-user-beacons-2026-11-24.md`(#1363 の集計と判定)
- `.github/workflows/agent.yml` の、ビーコンの取り出しの段(読むだけ)

## やること

1. `tools/perf-beacons/summary.mjs` に `--exits` を足す(既存の表と出力は残す)。直近 48 時間(UTC)の記録のうち、`engine.exit` が在る記録、または `engine.state` が `engine-idle` でない記録を、セッションごとに 1 行にまとめた表にする。列: 時刻(UTC、最初と最後)・版(`bundle.version`)・DPR の区分・記録数・`engine.state`・`engine.exit.reason`・`engine.exit.code`・`engine.exit.message`・`error.what`・`error.where`・`pageMs`(起動からの時間)・`timeline` の最後の印。sid・IP・パス・ユーザー名・ユーザーエージェントの全文は出さない。データが 0 件なら、0 行の表と、期間と件数を出す。終了コードは 0。
2. 集計を書く: `pm-work/findings/real-user-exits-<UTC の日付>.md`。同じ理由(`reason`・`code`)の件数、時刻の範囲、版、DPR の区分。`message` は、個人の値を除いた文字列だけを書く。
3. 評価用プロジェクトを開いた時の終了を特定する。根拠(`message` の内容、`timeline` の印、`error.where`)を書く。特定できない時は、その旨と、取れなかった理由を書く。`#1364` の窓の大きさが記録に在れば含め、無ければ「記録に無い」と書く。
4. 原因の推定を書く(実測と分ける)。次の Issue の候補を、報告に 1〜3 件書く。候補には、再現に要る最小の手順と、必要な計測の追加(ビーコンの項目)を入れる。

この Issue で変えるのは、`tools/perf-beacons/summary.mjs` と `pm-work/findings/real-user-exits-*.md` だけ。`app/`・`lift/`・`ci/`・`.github/`・`guard/` は変えない(読むだけ)。

## 受け入れ

```accept
node tools/perf-beacons/summary.mjs lift/out-app/host-extra/perf --exits >/dev/null && ls pm-work/findings/real-user-exits-*.md >/dev/null && ! grep -qF -f <(ls -R lift/out-app/host-extra/perf 2>/dev/null | grep -oE '^[0-9a-f]{12}' | sort -u) pm-work/findings/real-user-exits-*.md && ! grep -qE '@|C:\\' pm-work/findings/real-user-exits-*.md && bash guard/scan.sh pm-work/findings/real-user-exits-*.md
```

判定の基準:
- 受け入れの 1 行が通る(表の出力の終了コードが 0。findings に、ビーコンの sid の値・`@`・`C:\` が無い。`guard/scan.sh` が 0 件)。
- 表が出る(0 件でも、0 行の表と期間・件数が出る)。
- findings に、終了の理由ごとの件数、時刻の範囲、版が書かれている。
- 原因の推定は、実測と分けて書く。「原因は確定」「直った」とは書かない。

## 報告の規則

- 1 行目は「実記録: <何を実行し、どの出力を観測したか>」で始める。
- 報告に書く: 表の件数、`reason`・`code` の内訳、評価用プロジェクトの終了の特定の結果(根拠と、特定できたかどうか)、原因の推定(確定か推定か)、次の Issue の候補(1〜3 件)。
- 個人を特定できる値(sid・IP・パス・ユーザー名・ユーザーエージェントの全文)は、表・findings・報告の、どこにも書かない。

## 制約

- sid・IP・パス・ユーザー名を書かない。ベンダー名と Cloudflare の識別子を書かない。
- 記録が無い時は、0 件と書いて終える。記録に無いことを書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない(この Issue は、道具と文書の作業)。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは無い。道具と文書の作業なので、長いコマンドは無い。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- 結果は、#1365(LockResource)・#1372(ExitProcess)・#1373(素材の配信)の見直しに使う。この Issue は、それらを直さない。
- `#1363`(集計の道具の初版)の後の記録を読む。
