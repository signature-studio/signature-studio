---
draft: "174"
title: "crash-hunt の続き: 残りの経路・Settings の全タブと言語・テーマの切り替え・Browser の各タブ・上部バーの部品と Mixer のインサートを踏み、停止を集める"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

FL の画面の操作を網羅的に自動で踏み、停止を全て集める。#1405 の道具(`tools/crash-hunt/`)の最初の走査は、メニューとキーと右クリックの一部(234 件)だけを踏んだ。この Issue は、残りの経路と、未実施のグループ(上部バーの部品と Mixer のインサート・Settings・Browser・内蔵プラグイン)を踏み、停止を指紋ごとに集める。

停止が見つかること自体は失敗ではない。直すのは、PM が種類ごとに別の Issue にする。この Issue の成果は、停止の一覧と、道具の完成である。

## 背景

- 最初の走査(#1405): `pm-work/findings/crash-hunt-2026-11-25.md`・`.json`。234 件(DPR 1 と、乱数の DPR 1 つ)。ok 230・stopped 4。50 分の予算で、完了していない(`complete` が false)。残りの経路の数は、`.md` の「残り」と、道具の `remaining` に在る。
- 未実施(`.md` の「残り」): (c) 上部バーの部品(Tempo・Song Position・Pattern Selector など)と Mixer のインサートの右クリック。(d) F10 の Settings の全タブと、Language・Theme の切り替え。(e) Browser の各タブ(Sounds・Plugin database・Packs など)。(f) 内蔵プラグインの instantiate(`inputs/plugin-classes.tsv` の Effects 112・Generators 70)。(f) の道具(#1371)は、このリポジトリに無い。
- 上部バーの部品の名前は、窓の表(`pm-work/findings/crash-hunt-2026-11-25.json` の `View/11/1` の `newWindows`)に在る。`TWPControl` の題名: `SpacePanel`・`FixedSpacePanel`・`PositionToolbar`・`TimePanel`・`PatternPanel`・`MonitorPanel`・`CPUPanel`・`MenuPanel`・`DownloaderBtn`・`VolumePanel`・`PitchPanel`・`MetronomeBtn`・`ViewPLBtn` など。座標の定数は書かず、窓の表から位置を取る。
- 1 回目の停止の指紋(4 件)と、この Issue での扱い:
  1. `File > #9 > #11`: `GetFileVersionInfoSizeW` の名前にディレクトリ部が無い(`#{{172}}` が直す)。
  2. `Add > #14`: `RegOpenKeyExW` の空の要素(`#1411` で直した。この走査の木は直す前なので、再走査で `ok` になること)。
  3. `View > #11 > #1`: `indirect call to 0`(`#{{173}}` が扱う)。
  4. `File > #10`: File の最後の項目 `Exit` で engine が終了する。これは正しい動作で、停止に数えない(期待される終了)。
- `#1395`(英語以外の言語の起動)が、Language の切り替えで止まることがある。言語の切り替えで止まる時は、停止の文言を記録し、期待される停止にはしない。

## 読む物

- `tools/crash-hunt/run.sh`・`tools/crash-hunt/hunt.mjs`(全体。特に walk の定義・`LABELS`・`RIGHT_TARGETS`・`KEYS`・`notStarted` の一覧・`probe` の作り方)
- `tools/crash-hunt/README` が無いので、`hunt.mjs` の先頭の注記を読む
- `pm-work/findings/crash-hunt-2026-11-25.md`・`.json`・`pm-work/findings/crash-hunt-state.json`(続きの状態。`probes` の 234 件が入っている)
- `app/src/perf/exit-message.ts`(停止の指紋の作り方。道具が使う)
- `tools/boot-check/boot-bundle.mjs`・`tools/boot-check/cdp.mjs`

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 道具を続きから走らせる。`bash tools/crash-hunt/run.sh --budget 50m --out pm-work/findings --resume`。1 回の走査は 50 分以内。完了した経路は `crash-hunt-state.json` に残るので、複数回に分けて続けてよい。
2. 期待される終了を、停止に数えないようにする。道具の各 probe に `expected`(true か false)を足し、`expected: true` の probe は、`stops` の数に入れない。対象は、File の最後の項目 `Exit` の終了(`stop` が無く、`state` が exited)と、道具の表に書いた他の正しい終了。`expected` の一覧は道具の中に書く。
3. 未実施の (c)(d)(e)(f) を足す。新しい経路の `group` 名は次のとおり(既存の `a`(メニュー)・`b`(キー)・`c`(右クリック)は変えない)。
   - (c) 上部バーの部品と Mixer のインサート: `top`。上部バーの部品の左クリック・右クリック・ホイール・ドラッグ。Mixer のインサートの右クリックの全項目。
   - (d) F10 の Settings: `settings`。窓の表の帯の頁(MIDI・Audio・General・File・Theme・Project・Info・Debug・Account・About)を切り替え、頁ごとに選択欄・チェック・ボタンを踏む。Language の 7 言語と Theme の 13 を切り替える(切り替えの後は、`engine` の状態を記録する)。
   - (e) Browser の各タブ: `browser`。窓の表の題名から、タブの位置を取る。
   - (f) 内蔵プラグインの instantiate: `plugin`。`#1371` の道具が在れば再利用する。無ければ、Mixer の FX スロットの `Select` のメニューから、Effects を 1 つずつ選ぶ(メニューの経路で踏める)。読み込まれないプラグインは「プラグインの欠落」の窓が出る。その窓が出ることも、記録に残す。
   - 座標の定数を書かない。窓の表と、頁の題名の文字の位置から取る。取れない時は、その旨を probe の `notes` に書く。
4. 走査を回す。1 回 50 分以内で、複数回に分けて続ける。`restartFailures` と `cdpRetries` は、JSON に入る(既存)。
5. 記録。停止は指紋ごとに 1 行の表にする(件数・代表の経路・DPR・停止の文言)。`.md` と `.json` を `pm-work/findings/` に残す(道具の既定の名前 `crash-hunt-<UTC の日付>`)。停止の文言の 80 字の指紋は、道具の既定のままにする。
6. 受け入れを通し、作業ツリーを commit する。受け入れが通らなくても、作業ツリーを commit する。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && bash tools/crash-hunt/run.sh --budget 50m --out pm-work/findings --resume && node -e 'const fs=require("fs");const d="pm-work/findings";const f=fs.readdirSync(d).filter((x)=>/^crash-hunt-\d{4}-\d{2}-\d{2}\.json$/.test(x)).sort().pop();const j=JSON.parse(fs.readFileSync(d+"/"+f,"utf8"));const g=j.probesByGroup||{};process.exit(j.probeCount>=400&&["top","settings","browser","plugin"].every((k)=>(g[k]||0)>=1)&&j.restartFailures===0&&typeof j.cdpRetries==="number"?0:1)' && bash guard/scan.sh
```

判定の基準(全て満たす):
- 道具の走査の終了コードが 0(`run.sh` が 1 を返さない)。
- 最新の `pm-work/findings/crash-hunt-<日付>.json` の `probeCount`(続きを含む全 probe の数)が 400 以上。
- `probesByGroup` の `top`・`settings`・`browser`・`plugin` の件数が、それぞれ 1 以上。
- `restartFailures` が 0。`cdpRetries` の数が、JSON に数として入っている。
- `bash guard/scan.sh` が 0 件。
- 4 件の指紋(上の「背景」の 1〜4)が、再走査で、それぞれの扱いどおりに出ている(1・3 は直すまでは停止。2 は `ok`。4 は期待される終了で、停止に数えない)。報告に、その 4 件の結果を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 走査の回数と、各回の経過時間と、残りの経路の数、グループ別の件数(`probesByGroup`)、停止の指紋ごとの表(件数・代表の経路・DPR・文言)、言語とテーマの切り替えの結果(止まった言語・テーマの名前と文言)、`expected` の一覧、4 件の指紋の再走査の結果、`restartFailures`・`cdpRetries`。

## 制約

- 製品(`app/`・`lift/`・`ci/`)を直さない。停止が見つかっても、この Issue で直さない。道具と、記録(`pm-work/findings/`)を変える。
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。道具も、環境変数で動きを変えない(引数で変える)。
- Python と OCR を使わない。文字は、窓の表と道具の記録から読む。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 走査は、1 回 50 分以内。`--resume` で続ける。1 回の走査が 50 分を超えるように、予算を増やさない。
- 起動の間欠の停止や CDP の timeout で、走査が途中で止まったら、その旨を報告に書いて commit して終える(`crash-hunt-state.json` に残った経路から続く)。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by は無い。
- `#{{172}}`・`#{{173}}` の道具の変更(`--only`)が main に入っていれば、それを使う。まだなら、この Issue では `--only` を足さなくてよい。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- `#{{175}}`(暫定で止めている API の分類)が、この走査の停止の一覧を入力にする。停止の一覧は、`pm-work/findings/` の記録として残す。
