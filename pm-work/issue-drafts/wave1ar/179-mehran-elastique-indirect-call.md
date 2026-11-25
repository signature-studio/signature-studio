---
draft: "179"
title: "Mehran Abbasi の曲を開くと、Elastique の間接呼び出し(`0xb2844c0`)で止まる"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["verify"]
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
