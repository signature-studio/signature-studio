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

#{{121}} の段階 1(LockResource の停止)を直すと、この停止が見えるようになった。Exoplanet の停止は、LockResource とは別の原因。P3 の測定(#{{004}}・#1335)の入力を広げるため High。

## 前回までの試行で分かったこと

run 45315619741(#{{121}} の作業。この停止は、その作業の中で調べた。段階 1 の判定を通すため、この Issue に分けた)。調べるための一時の診断の出力は、調査の後に全部除去した(作業の差分には入っていない)。

1. 再現(実測): LockResource の修正の後の engine で Exoplanet を File > Open で開くと、ログ末尾が `[err] image-read: fruity_balance_x64.6800000.wasm requested=1 14060.4` の後に `[err] fle: ExitProcess: the program asked to end the process (exit code 0); stopped here, not exited` になる。測定の `error` は `state exited while exoplanet was opened`、タイトルは Song info のまま。
2. ExitProcess の戻り先(実測。一時の診断): `0x680e3d2`(Fruity Balance の像。基準 0x6800000 の中)。Balance の DllMain(reason 1)は戻らず、`load` の行は出ない。
3. 経路(objdump。推定の材料): `0x680e3d2` は、CRT の終了処理 `0x680e270` が ExitProcess の thunk `0x6804bf0` を呼んだ先。`0x680e270` は Balance の entry point(AddressOfEntryPoint `0x2c13a0`。基準を足すと `0x6ac13a0`)の末尾 `0x6ac13ff` から呼ばれる。`exit(code)` は `0x680e400`。thunk の対応: `0x6804bc0` = GetCurrentThreadId、`0x6804c50` = FreeLibrary、`0x6804bf0` = ExitProcess。
4. 推定: CRT の状態 `[0x6b14c80]` が 0 のとき、終了処理が ExitProcess に進む。この状態は `0x680db20`(DllMain の CRT の部分)の入口で `rcx` から作られる。`0x68189c0` は `rcx` をそのまま `0x680db20` へ渡すので、初期値は非 0 のはず。0 のまま終わる理由は、未確認。
5. 関係の記録: commit `212eaf59`(#{{102}})の記録に「Balance を要求した後に SIGABRT(134)で止まる。分けない対照でも止まる。原因は未確認」とある。プラグインを分けたことが原因ではない(推定)。
6. ログに、スレッドの開始・終了の記録は無い。DllMain の reason 2・3 を送る経路は `lift/runtime/fle_thread.c` にある。どれが ExitProcess に至るかは、未確認。
7. 次に確かめること: `0x68189c0`・`0x680db20`・`0x680e130` の引数と戻り値を、runtime の入力(DllMain の引数・GetCurrentThreadId・FreeLibrary の戻り値)と照らす。`0x680e130`(構造体の復元)の経路が、終了処理に入るかを決める(推定)。
8. 前回の作業は 5 時間弱。原因は未特定。ExitProcess を出す呼び出し元は、既存の道具(ゲストの呼び出し経路の記録)が無いため、一時の診断で調べた。

## 読む物

- `lift/runtime/fle_realmod.c`・`fle_realmod.h`・`fle_res.c`(#{{121}} の差分で入る。`fle_realmod_image_mapped`)
- `lift/runtime/fle_thread.c`(DllMain の reason 2・3 を送る経路)
- ExitProcess の応答: `grep -rn "ExitProcess" lift/runtime lift/harness | head`
- `tools/p3-audio-probe/measure.mjs`(開く手順。失敗時のログ末尾 `end.tail`)
- `git show 212eaf59 --stat`、`git log --oneline -- lift/runtime lift/harness app/src/engine`(近い変更)

## やること

1. 作業の最初に `date -u` を記録する。`git log --oneline -5` で、#{{121}} の段階 1 が main に入っているかを確かめる。入っていなければ、受け入れを実行せず、「前提未達」と報告に書いて終える。
2. 再現する: Exoplanet を、道具で File > Open で開く。`end.tail` で、ExitProcess の直前のログを読む。
3. 特定する: 上の 4 の推定を、事実にする(ExitProcess を呼ぶ直前の関数と、CRT の状態 `[0x6b14c80]` を 0 にする書き込み)。道具は、既存の仕組み(ログ・`engineLog()`)か、一時の診断を使う。一時の診断は、最後に必ず除去する。結果を、確かめた事実と推定に分けて報告に書く。
4. 直す: Windows と同じ結果になるように、runtime の応答を直す。既定の動作として直す(オプトインを作らない)。
5. 確かめる: 受け入れを通す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && mkdir -p .tmp-128 && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/Eric Kauffmann & NrgMind - Exoplanet.flp" .tmp-128/exoplanet.flp && cp "lift/out-app/host/resources/Data/Demo projects/Demo songs/DEDZII - NIGHT TERROR.flp" .tmp-128/dedzii.flp && for f in exoplanet dedzii; do node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-128/$f.flp" > "$RUNNER_TEMP/open-$f.json"; done && node -e 'const fs=require("fs");const L=["exoplanet","dedzii"].map((f)=>JSON.parse(fs.readFileSync(process.env.RUNNER_TEMP+"/open-"+f+".json","utf8").trim()));process.exit(L.every((r)=>r.error===null&&r.state==="engine-idle"&&r.quantaWhilePlaying>0)?0:1)'
```

判定の基準: Exoplanet と DEDZII の 2 件とも、`error` が null、`state` が engine-idle、再生の quantum が 1 以上。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 停止の再現(Exoplanet の JSON の行と、ログ末尾の行)、特定の経路(事実と推定)、直した方式と理由、変えたパス(`app/`・`lift/` を含む。`app/` と `lift/` を変えると main の deploy が動く)。

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

- #{{121}}(LockResource の段階 1)が前提。同じ `lift/runtime/fle_res.c`・`fle_realmod.c` を読む。
- #{{004}}・#1335(P3 の測定の入力)。
- #{{102}}・#{{019}}(Fruity Balance と Fruity Filter の、起動後の取得)が、同じ経路を変えうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
