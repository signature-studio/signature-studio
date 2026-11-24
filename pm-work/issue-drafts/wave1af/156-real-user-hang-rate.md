---
draft: "156"
title: "実機のビーコンで、起動が first idle に届かず離脱した割合を、版ごとに集計する(起動の間欠の停止が、本番に出ているかを知る)"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["perf-beacons", "verify"]
size: S
priority: Urgent
project: signature-studio/3
---

## 目的

起動が first idle に届かず、`state running` のまま止まる障害が、本番(実機の利用者)に出ているかを、実機のビーコンで知る。この Issue は、集計の道具を足して、結果を版ごとに出すまで。障害の修正は行わない(#{{149}})。

## 背景

- runner(受け入れを動かす環境)で、起動が間欠に `state running` のまま止まる(first idle に届かない)。#{{149}} で扱う。受け入れの起動確認と流れ A〜D が、この間欠の停止で落ちている。
- #1352 段 A の版(bundle.version `d7a6e6f392f2` 以降)が、本番に出ている。その前の版は `b440e13a623a`。段 A の版で、本番の起動が止まる割合が変わったかは、まだ分からない。
- ビーコン(`app/src/perf/beacon.ts`): 送る理由は `first-idle`・`periodic`・`exit`・`error`。first idle に届いた時に `first-idle` を送る。`engine.state`(例: `engine-idle`・`exited`)と `pageMs`(起動からの時間)を含む。
- 失敗の記録(`app/src/perf/failure-beacon.ts`): 理由 `no-first-idle`(起動から 60 秒を過ぎても first idle が無い)と `left-before-first-idle`(first idle の前にページを離れた)。
- 既存の集計: `tools/perf-beacons/summary.mjs`(版ごとの表。`--exits` で終了と失敗の表)。前回の集計: `pm-work/findings/real-user-exits-2026-11-24.md`・`pm-work/findings/real-user-beacons-2026-11-24.md`。
- 個人を特定できる値(sid・IP・パス・ユーザー名)は、表にも findings にも出さない。sid は、ページの読み込みごとの 12 桁の 16 進の乱数で、個人の識別子ではないが、出さない。

## 読む物

- `tools/perf-beacons/summary.mjs`(引数、版ごとの表、`--exits` の作り方。sid を出さない方針)
- `app/src/perf/beacon.ts`・`app/src/perf/failure-beacon.ts`(理由の一覧、first idle の印、`pageMs`)
- `pm-work/findings/real-user-exits-2026-11-24.md`・`pm-work/findings/real-user-beacons-2026-11-24.md`(前回の表の形)
- `.github/workflows/agent.yml` の「実機のビーコンを R2 から取る」の段(読むだけ。直近 3 日の記録を `lift/out-app/host-extra/perf/` に置く。commit はしない)

## やること

1. `tools/perf-beacons/summary.mjs` に `--hangs` を足す(既存の表と `--exits` の出力は変えない)。セッション(`sid` ごと。表に sid を出さない)を、次のように数える。
   - 届いた: first idle の印が在る(`engine.state` が `engine-idle`、または、理由 `first-idle` の記録が在る)。
   - 届かず離脱: first idle の印が無く、失敗の記録 `left-before-first-idle` が在る、または、最後の記録の `pageMs` が 20 秒以上。
   - うち停滞の疑い: 届かず離脱のうち、first idle の印が無く、最後の記録の `pageMs` が 20 秒以上のもの(別の列で数える)。
   - 判定できない: first idle の印が無く、20 秒未満で記録が終わっていて、離脱の記録も無いもの(別の列で数える)。
   - 割合は、届かず離脱 ÷(届いた + 届かず離脱)。判定できないものは分母に入れない。
2. 版(`bundle.version`)と DPR の区分(1 / 1 超 2 未満 / 2 以上 3 未満 / 3 以上)ごとに、上の 4 つの数と割合を表にする。0 件の区分も行に出し、合計の行を付ける。
3. 結果を `pm-work/findings/real-user-hangs-<UTC の日付>.md` に書く: 期間(取り出した記録の最初と最後の時刻)・記録数・セッション数・上の表・読み。読みには、#1352 段 A の版(`d7a6e6f392f2` 以降)と、その前の版(`b440e13a623a`)の比較を書く。件数が少ないときは、差を言い切らず、件数を書く。「原因は確定」「直った」とは書かない。
4. 記録が 0 件でも通る形にする: 0 件の表と、期間・件数を出し、終了コードは 0。

## 受け入れ

```accept
node tools/perf-beacons/summary.mjs lift/out-app/host-extra/perf --hangs >/dev/null && ls pm-work/findings/real-user-hangs-*.md >/dev/null && ! grep -qF -f <(ls -R lift/out-app/host-extra/perf 2>/dev/null | grep -oE '^[0-9a-f]{12}' | sort -u) pm-work/findings/real-user-hangs-*.md && ! grep -qE '@|C:\\' pm-work/findings/real-user-hangs-*.md && bash guard/scan.sh pm-work/findings/real-user-hangs-*.md
```

判定の基準:
- 受け入れの 1 行が通る。`--hangs` の終了コードが 0。findings に、取り出した記録の sid(ファイル名の先頭の 12 桁)の値が無い。`@`・`C:\` が無い。`guard/scan.sh` が 0 件。
- 上の検査は、sid の値そのものを、取り出した記録から探す形にしている。8 文字以上の英数字の一律の検査は、版の値とフィールド名に当たるので使わない。
- 表が出る(0 件でも、0 行の表と期間・件数が出る)。
- findings に、版と DPR の区分ごとの表、合計、読みが在る。

## 報告の規則

- 1 行目は「実記録: <何を実行し、どの出力を観測したか>」で始める。
- 報告に書く: 期間と件数、版と DPR の区分ごとの 届いた・届かず離脱 の数と割合、停滞の疑いの数、判定できない数、#1352 段 A の版と、その前の版の比較。
- 個人を特定できる値(sid・IP・パス・ユーザー名)は、表・findings・報告のどこにも書かない。

## 制約

- sid・IP・パス・ユーザー名を書かない。ベンダー名と Cloudflare の識別子を書かない。R2・アカウント ID・ホスト名を書かない。
- `app/`・`lift/`・`ci/`・`.github/`・`guard/` は変えない(読むだけ)。変えるのは `tools/perf-beacons/summary.mjs` と `pm-work/findings/real-user-hangs-*.md` だけ。
- Python を製品と本番の経路に使わない。OCR を使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは無い。道具と文書の作業なので、長いコマンドは無い。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- #{{149}} の障害が本番で出ているかを見る Issue。障害は直さない。結果は、#{{149}} の優先度の判断に使う。
- #{{133}}(実機の評価用プロジェクトを開いた時の終了)と同じ道具(`summary.mjs`)と、同じ取り出し(直近 3 日の記録)を使う。
- #{{155}}(ビーコンの fps と遅延。`summary.mjs` に表を足す)と、同じファイルを変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
