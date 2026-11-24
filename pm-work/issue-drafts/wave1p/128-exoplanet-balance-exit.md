---
draft: "128"
title: "demo の曲 Exoplanet を File > Open で開くと、Fruity Balance の読み込みの後に engine が exit 0 で止まる(ExitProcess)を直す"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["121"]
labels: ["plugin-builtin"]
size: L
priority: High
project: signature-studio/3
---

## 目的

インストーラの demo の曲 `Eric Kauffmann & NrgMind - Exoplanet.flp`(playlist 4,440 項目)を File > Open で開くと、Fruity Balance(`fruity_balance_x64.6800000.wasm`)の読み込みの後に engine が止まる(`fle: ExitProcess: the program asked to end the process (exit code 0)`)。タイトルは Song info のまま変わらない。

この Issue の目標: Exoplanet を File > Open で開いて、止まらず engine-idle のまま、再生の quantum が進むこと。DEDZII も同じ道具で開けること(回帰の確認)。

#1365 の段階 1(LockResource の停止)を直すと、この停止が見えるようになった。P3 の測定(#1335・#1335)の入力を広げるため High。

## 前回までの試行で分かったこと

- 前回の試行: run 45392098052(job「作業 #1372」)。前提の #1365 段階 1(commit `18a7fded`)は main に在る。開始 12:19:59 UTC、終了 15:04 UTC ごろ。判定は「差し戻し」。直してはいない(push していない)。runner の値。nproc 4、loadavg 0.23 0.84 1.32。
- 以下は前回の実測と、そこから読んだ推定。

1. 再現(実測。4 回目の計測で再現): 測定の JSON は `state` が engine-idle、`error` が "timeout 180000 ms: the title of exoplanet (the title is "Song info")"、`end.state` が exited、`end.title` が Song info。ログ末尾(一時の診断。除去済み):
```
[err] dbg-dllmain enter idx=7 reason=1 base=0x6800000
[err] dbg-L-dispatch top=0 sp=0x1312fba0
[err] dbg-dump exit: landing_top=0
[err] dbg-dump  mod fruity_balance_x64.dll base=0x6800000 state=1 A=0 st=0 B=0
[err] fle: ExitProcess: the program asked to end the process (exit code 0); stopped here, not exited
```
   対照: Fruity Filter の DllMain(reason 1)は戻る(`dbg-dllmain return idx=8 reason=1 r=1`)。
2. 特定(事実): Balance の入口 `0x6ac13a0` の setjmp の続き `0x6ac13c0`(landing L)へ longjmp が来た時、登録の一覧 `fle_landing_top` は空(`top=0`)。
   - 入口は Balance の wasm 側(`lift/out-app/hyb/n0-balance.tsv` に ok。直接 wasm)。続きは C 側(`lift/out-app/gen/src/m8-03.c` の `f_6ac13c0`)。入口と続きが、別の側で lift されている。
   - 登録を作る wrapper(`fle_landing_push`)は、生成物に Balance の分が無い(`gen` の登録は 5 件。いずれも Balance ではない)。
   - Balance は `lift/harness/hyb/hyb-args.tsv` の `balance` 行(`--pdata` だけ。seeds の指定が無い)で lift される。このため、setjmp の続きの seed が無い。
   - ExitProcess の戻り先 `0x680e3d2` は、出口 `0x680e270`(枠 `[0x6b14c80]` が 0 なら ExitProcess)の枝。実測の `A=0`・state 0 と合う。
3. 推定(コードの読みから。実測ではない):
   - longjmp が L に届いた時、登録が無いので、L は普通の関数として黙って呼ばれる。その戻りが出口に続き、枠が 0 なので ExitProcess(0) になる。
   - 原因は lift の分割(入口と続きが別の側)。runtime の応答は結果であり、runtime だけを直しても、Windows と同じ経路にはならないと見られる。
   - 他の exit 経路(`0x4072b0` の 4 か所)は、非 0 のコード(0xe7・0xd2・0xd9)かテーブルの値で、ExitProcess(0) の経路ではない(実測の範囲)。
   - 停止した DllMain は reason 1(プロセスへの接続)で、スレッドの通知(reason 2・3)の経路ではない(`dbg-dllmain enter idx=7 reason=1`)。
4. 直し方の候補(作業者が 1 つ選ぶ。既定の動作として直す。オプトインを作らない):
   - (a) Balance の入口と続きを、同じ側で lift する(hyb の分割。`lift/crates/fe-wasmgen/src/hybrid.rs`・`lift/harness/hyb/hyb-args.tsv` の `balance` 行・`lift/harness/hyb/hybrid.cmd` の `--separate`)。
   - (b) 入口の登録(wrapper)を、分割した側でも生成させる(fe-lift の landing の seed を balance に足す。`--seeds` の行)。
   - 選んだ理由と、変えたパスを報告に書く。`app/`・`lift/` を変えると main の deploy が動く。
5. 阻害(解消済み): `lift/harness/web/pack.args` の OTT の `--lazy-file`(0688430b)は、外部のインストーラが無い作業者の run で `fe-build bundle` を止めていた。main の 4d73666a で、外部の wasm が無い時は省く版になった。この Issue の受け入れは、その上で通る。
6. 前回の作業の保存(`work.diff`): `pm-work/findings/exoplanet-exit-1372.md`(調査の記録。commit `8ca2d33e`)。次の再試行で取り込まれる。作業者は同じパスのファイルを作らない(衝突する)。
7. 未確認: DEDZII の File > Open(回帰)は測っていない。受け入れは通っていない。

## 読む物

- `pm-work/findings/exoplanet-exit-1372.md`(前回の調査の記録。取り込まれていれば)
- `lift/crates/fe-wasmgen/src/hybrid.rs`・`lift/harness/hyb/hyb-args.tsv`・`lift/harness/hyb/hybrid.cmd`
- `git show 212eaf59 --stat`・`git show 212eaf59 -- lift/harness/hyb/hyb-args.tsv lift/harness/hyb/hybrid.cmd`(Balance を分けた時の差分)
- `lift/runtime/fle_dispatch`・`fle_landing_try`・`fle_landing_push` の定義(`grep -rn "fle_landing_try\|fle_landing_push" lift/runtime | head`)
- `lift/runtime/fle_thread.c`(DllMain の reason 2・3 を送る経路)
- `tools/p3-audio-probe/measure.mjs`(開く手順。失敗時のログ末尾 `end.tail`)

## やること

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。前回の調査の記録(`pm-work/findings/exoplanet-exit-1372.md`)を読む。
2. 事実を確かめる: 生成物で、Balance の入口(`0x6ac13a0`)の登録(wrapper)の有無、続きの `f_6ac13c0` の出どころ(C 側)を、`lift/out-app/gen` と `lift/out-app/hyb` で確かめる。
3. 直す: 上の 4 の (a) か (b) の 1 つ。Balance の入口と続きが同じ経路で登録されるようにする。runtime だけを変えて済ませない(推定)。
4. 診断の一時の差分を入れた時は、必ず除去してから commit する(差分に残さない)。
5. 受け入れを通す。通らなければ、作業ツリーを commit して終える。
6. commit する。push はしない。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-128 && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/Eric Kauffmann & NrgMind - Exoplanet.flp" .tmp-128/exoplanet.flp && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp" .tmp-128/dedzii.flp && for f in exoplanet dedzii; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-128/$f.flp" > "$RUNNER_TEMP/open-$f.json"; done && node -e 'const fs=require("fs");const L=["exoplanet","dedzii"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.quantaWhilePlaying>0)?0:1)'
```

判定の基準: Exoplanet と DEDZII の 2 件とも、`error` が null、`state` が engine-idle、再生の quantum が 1 以上。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 停止の再現(Exoplanet の JSON の行と、ログ末尾の行)、特定の経路(事実と推定)、直した方式と理由(上の 4 の (a) か (b))、変えたパス(`app/`・`lift/` を含む。`app/` と `lift/` を変えると main の deploy が動く)、DEDZII の結果。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 展開したプロジェクトは、リポジトリに入れない(`.tmp-*` に置き、消す)。
- 依頼者の個人のプロジェクトは使わない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは 1 回約 35〜46 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。`sleep`・`timeout` をコマンドの先頭に書かない。

## 他のタスクとの関係

- #1365(LockResource の段階 1)が前提。同じ `lift/runtime/fle_res.c`・`fle_realmod.c` を読む。
- #1335・#1335(P3 の測定の入力)。
- #1342・#1319(Fruity Balance と Fruity Filter の、起動後の取得)が、同じ経路を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #1400(Sytrus を、同じ型で 1 プラグイン 1 wasm として届ける)は、この Issue の直しの後に入る(blocked by の関係は `#1400` 側に在る)。
