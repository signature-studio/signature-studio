---
draft: "161"
title: "実機のビーコンの停止(stopped・exit・error・停滞)を、理由ごとに集計し、上位の原因を特定する"
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

依頼者の指示(2026-11-25): 「まだ非常にクラッシュしやすい状態が続いています。積極的に是正してください。」依頼者が踏む crash の全体像が分からない。実機のビーコンに残った停止を理由ごとに数え、件数の多い理由から直せるようにする。

この Issue の目標は次の 3 つ。
1. 実機のビーコンの集計に、停止したセッション(engine が止まった・エラーで止まった・最初の idle に届かなかった)の表を、理由ごとに足す(`summary.mjs --stops`)。
2. 件数の上位 5 件の理由について、原因の推定を、コードの読みから書く(実測と分けて書く)。
3. 直す Issue の案を、理由ごとに 1 行ずつ書く。起票は PM が行う。

## 背景

- 版 `e29a730b932d` 以降のビーコンには、停止の時に `engine.exit.stop`(`lastFle`: engine の最後の `fle: ` の行。`stopLine`: `== stop:` の行。`abortKind`・`memoryBytes`)と、`engine.exit.evidence`(画面の最初の非背景の行)が入る(#1380)。`files.engine.answeredUnderDemo`・`files.engine.answeredFlp` は、開く窓の答えの種類(件数だけ。名前・パスは無い)。
- 既存の道具: `summary.mjs` の `--exits`(48 時間。終了・エラーのセッションを 1 行ずつ)と `--hangs`(最初の idle に届かなかったセッションを、版と DPR で数える。件数だけ)。
- 既知の記録: `pm-work/findings/real-user-exits-2026-11-24.md`・`pm-work/findings/real-user-hangs-2026-11-24.md`。
- 理由の綴り: `engine.exit.reason` は `stopped`(code 134 は abort)・`error`・`finished`(正常)。

## 読む物

- `tools/perf-beacons/summary.mjs`(`--exits`・`--hangs` の実装。個人の値を出さない方針)
- `app/src/perf/beacon.ts`(`engine.exit` の項目。`stop`・`evidence`・`files` の項目)
- `app/src/perf/exit-message.ts`(`lastFle`・`stopLine`・`abortKind`・`memoryBytes` の作り方と、個人の値の除き方)
- `app/src/files/usage.ts`(`answeredUnderDemo`・`answeredFlp` の定義)
- `app/src/main.ts` と `app/src/ui/stop-message.ts`(`engine.exit.stop` を作る箇所)
- `pm-work/findings/real-user-exits-2026-11-24.md`・`pm-work/findings/real-user-hangs-2026-11-24.md`
- `lift/runtime/`(`fle: ` の行を出す箇所。`grep -rn 'fle_stop\|fle: '`)。`lift/out-app/gen/wasm-func-names.tsv` が在る時だけ使う。
- `.github/workflows/agent.yml` の、実機のビーコンを取り出す段(読むだけ)

## やること

1. `tools/perf-beacons/summary.mjs` に `--stops` を足す(既存の表と出力は残す。`--exits`・`--hangs` と同じく、個人の値を出さない)。対象は、直近 72 時間(UTC)の記録のうち、次のいずれかのセッション。
   - `engine.exit.reason` が `stopped` または `error` の記録がある。
   - `error` の記録がある。
   - 最初の idle に届かなかった(`--hangs` と同じ定義)。

   セッションごとに理由の指紋を作る。指紋は、次の 5 つをこの順に ` | ` でつないだ文字列(先頭 240 字まで): `engine.exit.stop.lastFle` の先頭 80 字、`engine.exit.stop.stopLine`、`engine.exit.stop.abortKind`、`error.what`、`error.where`。数字と 16 進の番地は `#` に置き換え、`X:\` で始まるパスは `<path>` に置き換える。

   表は版ごとに 1 つ。列は、指紋・件数(記録)・セッション数・最初と最後の時刻(UTC)・DPR の区分・`files.engine.answeredUnderDemo` と `answeredFlp` の有無(あり・なし・記録無し)・起動からの時間の区分(`pageMs` が 10 秒未満・60 秒未満・60 秒以上)。件数の多い順に並べる。データが 0 件なら、版ごとの表は出さず、期間と件数だけを出す。終了コードは 0。

2. 集計を `pm-work/findings/real-user-stops-<UTC の日付>.md` に書く。1 の表と、件数の上位 5 件の指紋ごとに次を書く。
   - 件数・版・DPR の区分・起動からの時間の区分。
   - 原因の推定。`lift/runtime/` で、指紋の `fle: ` の文字列を `grep` して、どの関数・どの理由で止めたかを特定する。戻り番地が引けるときは、その番地を書く。
   - 推定の確からしさ(確定・推定・不明)。記録の値(実測)と、コードの読み(推定)を分けて書く。
   - 直す Issue の案(1 行)。

   記録が 0 件のときは、「記録が無い」と書き、原因の推定は書かない。記録が無いことを、停止が無いことの証拠にしない。

3. 報告に、表の総件数、上位 5 件の指紋・件数・版、各々の原因の推定(確定か推定か)、直す Issue の案を、1 行ずつ書く。

## 受け入れ

```accept
node tools/perf-beacons/summary.mjs lift/out-app/host-extra/perf --stops >/dev/null && ls pm-work/findings/real-user-stops-*.md >/dev/null && ! grep -qF -f <(ls -R lift/out-app/host-extra/perf 2>/dev/null | grep -oE '^[0-9a-f]{12}' | sort -u) pm-work/findings/real-user-stops-*.md && ! grep -qE 'C:\\|/home/|/Users/|@[A-Za-z0-9.-]+\.[a-z]{2,}|[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+' pm-work/findings/real-user-stops-*.md && bash guard/scan.sh pm-work/findings/real-user-stops-*.md
```

判定の基準:
- 受け入れの 1 行が通る(表の出力の終了コードが 0。findings に、取り出した sid の値・パス・メールアドレスらしい値・IP らしい値が無い。`guard/scan.sh` が 0 件)。
- 表が出る(0 件でも、期間と件数が出る)。
- findings に、指紋ごとの件数、時刻の範囲、版、上位 5 件の原因の推定と案がある(0 件なら「記録が無い」と書いてある)。
- 「原因は確定」「直った」とは書かない。

## 報告の規則

- 1 行目は「実記録: <何を実行し、どの出力を観測したか>」で始める。
- 報告に書く: 表の総件数、上位 5 件の指紋・件数・版、各々の原因の推定(確定か推定か)、直す Issue の案(1 行ずつ)。
- 個人を特定できる値(sid・IP・パス・ユーザー名・ユーザーエージェント)は、表・findings・報告のどこにも書かない。

## 制約

- 製品(`app/`・`lift/`・`ci/`・`.github/`)は変えない(読むだけ)。道具と findings の作業だけ。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- ベンダー名と Cloudflare の識別子を書かない。ストレージの名前・アカウント ID・ホスト名を書かない。
- Python は使わない。OCR は使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- この作業はビルドを要さない。engine を作らない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 道具と文書の作業なので、長いコマンドは無い。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: なし。
- この Issue は、停止を数えて原因の候補を書くところまで。直す作業は、結果を見た PM が、停止の種類ごとに Issue にする。この Issue は、停止を直さない。
- `#1380`(停止の理由の項目)と `#1377`(評価用プロジェクトの終了の特定)の後の記録を読む。
