---
draft: "179"
title: "Mehran Abbasi の曲を開くと、Elastique の間接呼び出し(`0xb2844c0`)で止まる"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["154"]
labels: ["verify", "resources"]
size: M
priority: High
project: signature-studio/3
---

## 目的

demo の曲 Mehran Abbasi - With You を File > Open で開くと、途中で engine が止まる(終了コード 134)。止まる理由は、時間伸縮のライブラリ Elastique の中の間接呼び出し(`0xb2844c0`)の飛び先を、lift が「既知の関数の入口ではない」と判定したこと。#1403 の報告で確かめた。この Issue は、この間接呼び出しを認識できるようにして、Mehran の曲を開けるようにする。

この Issue の目標は次の 3 つ。
1. Mehran Abbasi - With You を File > Open で開くと、engine が止まらず、`engine-idle` になる(`error` が null)。
2. Mehran を開く途中の停止の理由に、`not a known function entry` が無い。
3. 認識の範囲を広げても、既存の表の判定が変わらない(退行 0)。

## 背景

- 事実 1(#1403 の報告。run 45566707815。runner の値): Mehran を開く途中で、`fle: indirect call to 0xb2844c0 is not a known function entry` の行が出て、engine が exit code 134 で止まる。素材の配信(#1403 の 27 行)を外した対照の bundle でも、同じ停止が出た。素材とは無関係。
- 事実 2(#1413): MSVC の switch 表の飛び先(ガード `cmp`・`ja` が無い形)を、lift が認識するように直した(閉じている)。認識の規則は `pm-work/findings/switch-tables.md` と `tools/switch-tables/check.mjs` にある。`0xb2844c0` が、その形かどうかは未確認。
- 事実 3(配布の仕組み): Elastique は、FL の DLL の中の像。その DLL が、lift の対象(`lift/plugins/<名前>/plugin.tsv` に載る)か、real module として配る物かは未確認。
- 関連: #1400(Sytrus を real module として配る。同じ型の直し)・#1403(素材の配信。Mehran の素材は、この停止より前に読まれる)。

## 前回の試行で分かったこと(run 45593569814。`failed`)

- 前回: run 45593569814 の作業 #1420 のジョブ。判定は差し戻し(`failed`)。作業は差分として `agent-1420` の `work.diff` に保存された(作業者の最後の commit `11bee9ef` は runner 上のもので、main には無い)。次の試行は、その差分を `.prior-work.md` と共に作業ツリーで受け取る。
- 直した(受理): `lift/harness/seeds/elastique-seeds-all3.txt` に 5 行を足した(754 → 759 行): `0xb2844c0`・`0xb070220`・`0xb2c1520`・`0xb2c5120`・`0xb2c5130`。生成 C に `fle_dispatch(fle_ld(UINT64_C(0xb4b30b0), 8))` が出て、入口の表に `{ 0xb2844c0, f_b2844c0 }` が入った。`cargo test -p fe-decode` は通った。switch 表の判定は前後で同じ(FLEngine 20/3/0/17/0、Sytrus 17/3/0/14/0、Balance 15/2/0/13/0。退行 0)。
- 形と像(実測): `1802844c0: 48 ff 25 e9 eb 22 00`(`jmp qword ptr [rip+disp]` の thunk。slot は `0x1804b30b0`、engine では `0xb4b30b0`)。switch 表の形ではない。直接の `call`・`jmp`・`jcc` の参照は 0 件(バイト列の走査)。DIR64 の再配置の slot にも無い(既存の adjustor の規則の対象外)。像は `elastique_x64.dll`。既に lift の対象(`lift/harness/b2d2/modules.tsv`)で、real module にする必要は無かった。
- 停止の場所は変わった。`0xb2844c0` の停止は消えた。Mehran を開く測定(`measure.mjs` の写しで、開いた後の入力を変えた 4 通り):

  | 測定 | 開いた後の入力 | 結果 |
  |---|---|---|
  | 1 | Space | exited・exit 134・`indirect call to 0` |
  | 2 | Enter(窓を閉じる)→ Space | 同じ |
  | 3 | Enter のみ | 同じ |
  | 4 | 何も送らない | engine-idle のまま。`not a known function entry` は無い |

- 読み: 開くだけなら止まらない(測定 4)。「Problems loading the project」(TMissingPluginsForm)の窓を閉じると止まる(測定 3)。止まりは再生の経路ではなく、この窓の経路の null の呼び出しと見える(測定 1 は、窓が既定のボタンとして効けば同じ。未確認)。呼び出し元は未確認。
- 起動確認(DPR 1): engine-idle・error null。`elapsedMs` は 12,565(1 回目。上限 12,500 を 65 ms 超えた)・12,274・12,294。流れ A〜D: 4 件とも通った(12,020〜12,340 ms。preBootBytes 312,223,030)。`npm test --prefix app` は 1,432 件通った。
- 受け入れの `cp -a lift/out-app/host-extra/resources/. …` が落ちる欠陥(#1400 と同じ構造): この Issue に `resources` ラベルが無く、`host-extra/resources` が作られなかった。PM が `resources` ラベルを付けた(下の 1)。作業者は、`host/resources` に Mehran の物が既に在ったので、この段を省いて残りを手で実行した。
- 測定 1 の出力(`measure.mjs` のまま。runner の値): `error: null`・`state: engine-idle`。`projectDialogs` は `TMissingPluginsForm "Problems loading the project"`(missing-path 134・dll 66)。Space は guest に届いた(`reachedGuest` true)。その後 `end.state: exited`・`fruity: exit code 134`・`[err] fle: indirect call to 0 is not a known function entry`。
- `0xb2844c0` の停止は、この試行の全ての測定で出なかった(新しい停止は `indirect call to 0`。飛び先の 0 は、thunk の slot が 0 なのか、窓の経路の別の null なのか未確認)。seeds の 5 行は、この停止を消したので残す。
- 診断の現状: main の `lift/runtime/fle_dispatch.h` の `fle_dispatch_miss` は、`fle: indirect call to %#llx is not a known function entry` の 1 行だけを出し、呼び出し元は出さない。呼び出し元を特定するには、#1416 の診断が要る。
- 未確認(作業者の報告より): seeds の 4 件(`0xb070220`・`0xb2c1520`・`0xb2c5120`・`0xb2c5130`)の、Mehran での効果。他の DLL(oggio・dsp_ippv2・Blend2D など)に、同じ種類の入口の不足が在るかの走査は、していない。
- 作業者の判断: 報告のとおり、`lift/out-app/host-extra/resources` が無いことは、この Issue に `resources` ラベルが無いことの結果。ラベルを付けた今は、ジョブの `cp` の段が通る見込み(未確認)。
- 判定の要点: 落ちたのは `not a known function entry` の 1 点(受け入れの Mehran の行)だけ。起動確認の `elapsedMs` は、3 回の測定のうち 1 回目だけが上限 12,500 を超えた(12,565。残りは 12,274・12,294)。他(fe-decode の試験・流れ A〜D・`npm test`・switch 表の退行 0)は通った。
- 直し方の理由(作業者の報告): 既存の仕組みは、`.pdata` の外で `lea` で取られる関数の入口を、seeds の行で認識する(`lift/harness/vst/seeds_scan.mjs` の注釈)。fe-decode の adjustor の規則(0051・0100)は、DIR64 で参照される thunk だけが対象。この thunk は対象外で、規則を広げると全ての DLL の閉包が変わるため、fe-decode は変えなかった。
- 生成結果: `lift/out-app/gen/src/m7-03.c` に `fle_dispatch(fle_ld(UINT64_C(0xb4b30b0), 8)); return;`。入口の表に `{ 0xb2844c0, f_b2844c0 }`。
- 測定 2〜4 は、`tools/p3-audio-probe/measure.mjs` の写し(一時のファイル)で入力を変えた。道具本体は変えていない。
- ビルド: engine のビルドは 1 回(37 分。前景で実行)で足りた。測定は、そのビルドの bundle で行った(作業者の報告)。
- 測定の値は runner の値(この機械とは別枠。nproc 4、loadavg 1.1〜2.4)。
- 次の試行: 形・像・配り方・直し方(やること 1〜4)は済んでいる。残るのは、#{{154}} が閉じた後の確認(やること 5)だけ。

## 次の試行の決まり(PM の決定。run 45593569814 の後)

1. `blocked_by` に #{{154}}(#1395。ILSvg の欠落)を足した。#{{154}} が閉じるまで、この Issue は起動しない。`resources` ラベルを付けた。#{{154}} が閉じると、この Issue は自動で起動する。
2. #{{154}} が閉じた後、Mehran の `not a known function entry` の呼び出し元(`caller 0x…`)を確かめる。呼び出し元を出す診断は、main には無い(#1416 の範囲で入る)。#1416 の呼び出し元 `0x66dd61`(ILSvg の LoadLibraryW の後)と同じなら、#{{154}} で直る。その時は、この Issue の変更(seeds の 5 行)を残し、#{{154}} の効果を確かめて閉じる。別の呼び出し元なら、その原因を直す。
3. 受け入れは元のまま(Mehran の出力に `not a known function entry` が無い)。起動確認の `elapsedMs` は 1 回の値で見る。前回は 1 回目が 12,565 だったので、この試行の結果を PM が見る。

## 読む物

- `pm-work/findings/switch-tables.md`・`tools/switch-tables/check.mjs`・`lift/crates/fe-decode`(認識の規則と、テスト)
- `pm-work/findings/`(#1403 の記録。`0xb2844c0` の停止の記録)・`pm-work/findings/sytrus-1400.md`(real module の型)
- `lift/plugins/`(`plugin.tsv` の形)・`lift/crates/fe-build/src/plugins.rs`(real module の扱い)
- `tools/p3-audio-probe/measure.mjs`(`<bundle> <flp>` の形で Mehran を開く)・`tools/smoke/welcome-flow.mjs`
- `objdump -d`(配布の DLL を逆アセンブルする。`lift/out-app/host/resources` の下の物)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **形の特定**: `0xb2844c0` の前後を逆アセンブルし、間接呼び出しの形(`jmp reg`・`call reg`・RIP 相対の表・ガードの形・表の幅)を特定する。どの像(DLL)の中かを、ロードの記録で特定する。報告に、形と像の名前を書く。
2. **配り方の確認**: その DLL が lift の対象か(`lift/plugins/<名前>/plugin.tsv` に載るか)を確かめる。載らないなら、real module として配る必要があるかを判断する(#1400 と同じ型)。
3. **直す**(既定の動作として。オプトインを作らない)。次のどちらかを選び、理由を報告に書く。
   - 認識の範囲を広げる(`lift/crates/fe-decode`)。広げた後、`tools/switch-tables/check.mjs` の既存の表の判定が変わらないことを確かめる。
   - その DLL を real module として配る(`plugin.tsv` と seed の行を足す)。
4. 報告に、形・像の名前・選んだ方法・Mehran を開いた後の `error` と `state`・`engine-idle` まで行ったかを書く。
5. **#{{154}} が閉じた後**(PM の決定。上の「次の試行の決まり」の 2): Mehran の `not a known function entry` の呼び出し元(`caller 0x…`)を確かめる。`0x66dd61` なら、#{{154}} の効果で直ったかを確かめて閉じる(seeds の 5 行は残す)。別の呼び出し元なら、その原因を直す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && cargo test --locked --manifest-path lift/Cargo.toml -p fe-decode && mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && mkdir -p .tmp-179 && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/Mehran Abbasi - With You.flp" .tmp-179/mehran.flp && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-179/mehran.flp" > "$RUNNER_TEMP/open-mehran.json" && node -e 'const fs=require("fs");const t=fs.readFileSync(process.argv[1],"utf8");const r=JSON.parse(t.trim());process.exit(r.error===null&&r.state==="engine-idle"&&!t.includes("not a known function entry")?0:1)' "$RUNNER_TEMP/open-mehran.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&Math.max(...L.map((r)=>r.elapsedMs))<=12500&&Math.max(...L.map((r)=>r.preBootBytes))<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(全て満たす):
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- `cargo test -p fe-decode` が通る(認識の試験を足す場合は、その試験を含む)。
- Mehran Abbasi - With You を開いた結果(`open-mehran.json`)が、`error` が null・`state` が engine-idle。出力のどこにも `not a known function entry` が無い。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `tools/switch-tables/check.mjs` の既存の表の判定が、変更の前と同じ(退行 0)。報告に、変更の前後の件数を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 間接呼び出しの形(逆アセンブルの要点)、像(DLL)の名前、選んだ方法と理由、既存の表の退行の確認、Mehran を開いた後の `error`・`state`、流れ A〜D の 4 行。
- 未確認の点は、そのまま書く(例: 停止の理由が Mehran の別の箇所に移った場合は、その行を書く)。

## 制約

- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 素材(demo の曲)を、空の代替(空ファイル・0 埋め)で置かない。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- エンジンのビルドは 1 回約 32〜46 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。`sleep`・`timeout` をコマンドの先頭に書かない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by は無い。
- #1403(demo 2 件の素材の配信)の Mehran の対照の結果は、この停止が素材と無関係であることを示す。#1403 の受け入れは、この Issue を待たない。
- #1413(switch 表の認識)と、`lift/crates/fe-decode` で衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- #1400(Sytrus)と #1414(Sytrus の Data の配信)は、real module と seed の行を変える。衝突したら両方を残す(削除しない)。
- #{{154}}(ILSvg_x64.dll を real module にする。`resources` の配信)が `blocked_by`。#{{154}} が閉じるまで、この Issue は起動しない。#1416(null の関数の呼び出し。呼び出し元の診断は、この Issue の対象外)。
