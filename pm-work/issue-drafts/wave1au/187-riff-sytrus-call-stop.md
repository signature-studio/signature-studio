---
draft: "187"
title: "リフ マシーンの窓を開くと、Sytrus の中の呼び出し(`indirect call to 0xcccccccccccccccc`)で engine が止まる: 原因を特定して直す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["147"]
labels: ["verify", "resources", "plugin-builtin"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

Riff machine(リフ マシーン)の窓が出た後で、engine が止まらないこと。窓を開くと内蔵の音源 Sytrus の中で、不正な間接呼び出し(`fle: indirect call to 0xcccccccccccccccc is not a known function entry`)が起き、engine が SIGABRT で止まる。この止まりの原因を特定し、直す。

この Issue が閉じると、#{{188}}(Accept でノートを作る確かめ)に進める。

## 背景

- #{{147}}の段階 1 で、窓が出るようになった(run 45592009894)。窓は `TPRScoreCreatorForm`・visible・painted。警告は出ない。
- その後、窓が Sytrus を使い始めると、engine が止まる(終了コード 134)。行は `fle: indirect call to 0xcccccccccccccccc is not a known function entry`(DPR 1)、`... to 0`(DPR 1.32)、`... to 0x475c084fff0bac7`(別の回)。
- Accept を押さない版(一時の写し。製品ではない)でも、同じ位置で止まった。止まりは Accept の操作と無関係で、窓を開いて Sytrus を使い始めた時の物。
- 止まりの直前: `real module table: name=generators.sytrus.sytrus_x64.dll base=0xc580000` と `image-read: generators.sytrus...dll requested=1`。Sytrus のモジュールは読み込まれている。
- 呼び出し元: 戻り番地 `0xc9c04e9`(guest stack の rsp の値。実測)。これは Sytrus の像(base 0xc580000)の中にある。RVA は 0x5404e9 付近(戻り番地から base を引いた値。推定)。
- 止まりを出すのは `lift/runtime/fle_dispatch.h` の `fle_dispatch_miss`。main の版は、呼び先の値の 1 行だけを出す。呼び出し元と R[0..15] は出さない(#1416 が扱う。open)。
- `0xcccccccccccccccc` は、MSVC のコードの隙間の埋め(`int3` = 0xCC が 8 個)。未初期化のスタック・データ、またはコードの隙間を読んだ値と見られる(推定)。
- 推定 (i): Sytrus の中で、未初期化の関数ポインタ(表やスタック上の値)を呼ぶ。推定 (ii): lift が、データの読みを、コードの隙間(`int3` の埋め)に当てている(像のずれ・reloc・pdata の誤り)。どちらかは確かめていない。
- #1400(Sytrus の読み込み・段階 1)・#1413(switch 表の飛び先)・#1414(Data の配信)は閉じた。#1413 は別の番地の止まり(`0xc9cf273`)を直した。この止まりは、その後に出た別の番地。
- Sytrus の DLL の行: `lift/plugins/sytrus/plugin.tsv`。`generators.sytrus.sytrus_x64.dll`・`Plugins\Fruity\Generators\Sytrus\Sytrus_x64.dll`・6,269,440 B・sha256 `e1395e97…`・hook `rva:0x1b930,0x38b80,0x38ba0`・omit `0x48cb70`。

## 前回までの試行で分かったこと

(run 45592009894。job「作業 #1387」。この止まりの元の観測。詳細は #{{147}} の本文の 14〜32)

1. 窓を開いた(DPR 1): `windowNew` に `TPRScoreCreatorForm`(751,275・334x506)。止まりは SIGABRT(終了コード 134)。`elapsedMs` 11,537・`preBootBytes` 312,304,864。
2. DPR 1.32: 窓は (1045,444)・334x506。止まりは `to 0`。
3. 直す前の回(HEAD の seed 表)は窓が無いので、Sytrus のモジュールが読まれない(`real module table` の行は 0 件)。直した後の回で初めて現れた。
4. Accept を押さない版(一時の写し)でも、同じ位置で止まった(`to 0x475c084fff0bac7`)。
5. 止まりの直前のログ: 3754 行目 `image-read: generators.sytrus...dll requested=1` の後、`Fruityplug_x64.dll` の探索が失敗する。配信の一覧に `Fruityplug_x64.dll` は無い(負の探索)。
6. Sytrus の import 欄は、配信の一覧に出ていない。解決できなかった import の記録は、止まりの前に見つからなかった。
7. 戻り番地 `0xc9c04e9`(11:20 の回の guest stack at rsp)。Sytrus の像の中。RVA 0x5404e9 付近(推定)。
8. `Plugin presets\Generators\Sytrus\` の資料 984 件は、配信の一覧に在り、seed 表に無い。`FindFirstFileW 3`(無い)が止まりの前に出る。止まりの原因かは未確認(推定)。
9. Sytrus の Data は「読む時に届く」遅延ファイル。届かない読みは EIO で名前付きに報告される。今回のログには無いので、遅延の Data の欠落は原因ではなさそう(推定)。
10. 呼び先の値は回ごとに違う(`0xcccccccccccccccc`・`0`・`0x475c084fff0bac7`)。未初期化の値を読んでいる見込み(推定)。
11. `fle_dispatch_miss`(`lift/runtime/fle_dispatch.h`)は、呼び先の値の 1 行だけを出す。呼び出し元と R[0..15] は出ない。#1416 が、その診断を扱う(open)。
12. #1400 の記録(`pm-work/findings/sytrus-1400.md`)の止まり(`indirect call to 0xc9cf273`)は、#1413 で直した。今回の止まりは、その後に出た別の番地。
13. #1400 の記録の「次にやること」の 5 つ目: Riff machine(#1387)は、#1413・#1414 が閉じた後に起動する。この順で進めた。
14. 起動の流れ A〜D は、窓を開く前の段階で 4 行とも合格(`elapsedMs` の中央値 11,698 ms)。止まりは、Riff の窓を開いた後にだけ起きる。
15. `npm test --prefix app`: Test Files 73 passed、Tests 1432 passed。
16. 道具 `tools/piano-roll-tools/measure.mjs --tool riff-machine` の JSON の `stopLines`(`fle: indirect call to …`)と `logTail` が、止まりの位置を示す。`engine`・`stopped` も出る。
17. `stopped` は、`== stop: returned from entry` を数えない(正常な終わりでも出るため)。止まりは `fle:` の行と、`fruity: exit code` の行で分かる。
18. 前回の記録(`pm-work/findings/riff-machine-no-response.md`)の 4 が、この止まりの記録。
19. 止まり後の画面は、数えた値ではない(`notes` 0 は未確認)。
20. 作業の順番: engine の作り直し(約 35〜40 分)の後に、窓を開く道具を 1 回走らせる。bundle を作り直す時は、seed 表の版を確かめてから(前回は `tables` の終了コードの確かめが抜けて、古い版で bundle を作った)。
21. 前回の差分の `missingPaths`: 配信の資料の `Riff machine\*.*` は、すべて成功。残りはユーザーの領域(`C:\Users\user\Documents\Image-Line\...`)の 2 行(受け入れは配信の資料の場所に限る)。
22. Sytrus の `plugin.tsv` の omit `0x48cb70` は、guest の setjmp を呼ぶ関数の入口。今回の止まりの RVA(0x5404e9 付近)とは別の番地(推定)。
23. Sytrus の DLL は plugin-builtin の作業で `lift/out-app/host-extra/plugins/` に置かれる。前回の run は、この木が無くても Sytrus のモジュールを読んだ(読み元は未確認)。この Issue では、DLL の実物を `objdump` で読むため、plugin-builtin を付けた。
24. 新しい plugin の最初の run は、木に DLL が無いことがある(#1400 の記録。抽出の鍵が HEAD の鍵のため)。木の有無を、作業の最初に `lift/out-app/host-extra/plugins/MANIFEST.tsv` で確かめる。
25. 止まりの型: 回ごとに呼び先の値が違う(上の 10)。DPR との関係は、確かめていない。
26. 実測と推定の区別: 実測は、止まりの行・戻り番地・直前のログ・窓の表。推定は、RVA・呼び先の出所・`0xcc` の意味。
27. #1400 の記録の 3: 受け入れの `cp -a lift/out-app/host-extra/resources/.` は、`host-extra/resources` が在るときだけ通る(前回の失敗の原因)。木を置くのは `resources` のラベルの作業だけ。この Issue には `resources` を付けた。
28. #1400 の記録の 4: 新しい plugin の最初の run は、木に DLL が無いことがある(抽出の鍵が HEAD の鍵のため)。この Issue の `plugin-builtin` の木は、作業の最初に確かめる(上の 2 の手順)。
29. `lift/plugins/sytrus/plugin.tsv` の hook 点(`rva:0x1b930,0x38b80,0x38ba0`)は、Delphi の 3 点と同じ形で、`inputs/plugin-hooks.tsv` の値。止まりの RVA(0x5404e9 付近)とは別の番地。
30. 止まりの RVA の推定は、戻り番地から base を引いた値。base は 0xc580000(前回のログの `real module table`)。読み直しの時は、同じ base を使う。

## 読む物

- `pm-work/findings/riff-machine-no-response.md`(#1387 の記録。前回の差分で main に入る。無ければ、上の「前回までの試行で分かったこと」を使う)
- `lift/runtime/fle_dispatch.h`(`fle_dispatch_miss`・`fle_jumptab_miss`)
- `lift/plugins/sytrus/plugin.tsv`(Sytrus の行。hook と omit)
- `pm-work/findings/sytrus-1400.md`・`pm-work/findings/switch-tables.md`(#1413 の switch 表の記録。別の止まり)
- `lift/crates/fe-wasmgen/src/lib.rs`(間接呼び出しの生成。`fle_dispatch_miss` を呼ぶ所)・`lift/crates/fe-lift`(間接呼び出しの扱い)
- `lift/out-app/host-extra/plugins/`(plugin-builtin の作業で置かれる木。`MANIFEST.tsv` の sha256 で照合する。Sytrus_x64.dll の実物)
- #1395 の本文(ILSvg の `0x3286b83d0` の調査。GetMem の上位 32 ビットの扱い。比べる材料として読む)
- `tools/piano-roll-tools/measure.mjs`(`--tool riff-machine`。止まりの再現)

## やること

1. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
2. 木の確かめ: `lift/out-app/host-extra/resources` と `lift/out-app/host-extra/plugins/MANIFEST.tsv` が在ることを確かめる。無ければ、受け入れの前提が欠けているので、報告に書いて終える。
3. 再現: `node tools/piano-roll-tools/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --tool riff-machine --dprs 1` を走らせる。止まりの行(`fle: indirect call to …`)と、その直前の 40 行のログを、報告に写す。
4. 呼び出し元を読む: 止まりの戻り番地(guest stack の rsp 付近の値)を取り、Sytrus の像の base を引いて RVA を出す。`objdump -d` で、その RVA の前後 60 命令を読み、呼ぶ先のポインタが、どこから読まれるか(`[rip+disp]`・`[reg+off]`・スタック上)を特定する。
5. 値を取る: その読み元の guest アドレスの値を、止まりの直前に読む(一時の診断。製品には残さない)。値が、(i) コードの隙間(`0xcc` の並び)を指す、(ii) 0(未初期化のデータ)、(iii) 解決されていない import(NULL)、のどれかに分けて、報告に書く。
6. 直す: (i) なら、lift 側のデータの読み(`fe-wasmgen`・`fe-lift` のアドレスの扱い・reloc・pdata)を直す。(ii)・(iii) なら、初期化の経路(DllMain の呼び出し順・`DLL_PROCESS_ATTACH`・遅延 import の解決)を直す。どちらも既定の動作として直す(オプトインを作らない)。
7. 受け入れを通す。
8. 1 回の試行で直せない時: 原因の特定(上の (i)〜(iii) のどれか。番地と読み元の命令)を `pm-work/findings/` に記録し、commit して終える。その時、受け入れは通らない(止まりが残るため)。次の試行は、その記録から続ける。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/piano-roll-tools/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --tool riff-machine --dprs 1 > "$RUNNER_TEMP/riff.jsonl" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n")[0]);const win=!!r.windowNew&&r.windowNew.some((w)=>w.visible===true&&w.painted===true);process.exit(r.error===null&&win&&r.stopped===false&&r.engine==="engine-idle"&&!r.stopLines.some((l)=>l.includes("not a known function entry"))?0:1)' "$RUNNER_TEMP/riff.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle で、`standIn` が false。
- 道具の 1 行(DPR 1): `error` が null。`windowNew` に、`visible` と `painted` が true の窓が 1 件以上。`stopped` が false。`engine` が engine-idle(操作の 3 秒後の道具の値)。`stopLines` に `not a known function entry` の行が無い。
- 流れ A〜D(4 行)。各行が、`error` null・`state` が engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。流れ B と D は `welcomeClosed` が true。`preBootBytes` の最大が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。
- 原因の特定(i)〜(iii)のどれか、と、その根拠(番地・読み元の命令・値)を、報告と `pm-work/findings/` の記録に書く。推定と実測を分ける。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 止まりの行と直前の 40 行、戻り番地と RVA、読み元の命令(`objdump` の行)、読み元の値と (i)〜(iii) の分類、直した方式と理由(既定の動作として)、流れ A〜D の 4 行、`elapsedMs` と `preBootBytes`。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。一時の診断は、報告の後に消す。
- Python を使わない。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の開始時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine の作り直しは約 35〜40 分。ビルドはフォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- bundle を作る前に、seed 表の版(`lift/harness/seeds/vfs-seed-idle1.tsv` と、bundle が読む `lift/out-app/plugins/vfs-seed-idle1.tsv`)を確かめる。前回は、写しが古いまま bundle を作った。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{147}}(リフ マシーンの窓が出る段階 1。その差分の seed 表 105 行と道具が main に入ってから起動する)。
- #{{188}}(Accept の後のノート)は、この Issue が閉じてから起動する。
- #1416(View メニューの null 呼び出し)は、同じ `fle_dispatch_miss` の診断を扱う。衝突したら両方を残す。
- #1395(ILSvg の `0x3286b83d0`。GetMem の上位 32 ビット)と、同じ型の未初期化の値かを、報告に書く(比べるだけ。#1395 は触らない)。
