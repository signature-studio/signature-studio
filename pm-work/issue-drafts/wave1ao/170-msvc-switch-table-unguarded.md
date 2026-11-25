---
draft: "170"
title: "MSVC の switch 表の飛び先(ガード `cmp`・`ja` が無い形)を、lift が認識する: 『indirect call to … is not a known function entry』で止まらない"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "022"
blocked_by: ["157"]
labels: ["plugin-builtin", "resources"]
size: L
priority: High
project: signature-studio/3
---

## 目的

内蔵の音源 Sytrus(`Sytrus_x64.dll`)の関数の途中に在る MSVC の switch 表の飛び先を、lift が認識しない。このため、Sytrus を使う demo を開くと、「indirect call to 0xc9cf273 is not a known function entry」で engine が止まる。この Issue は、この形の表を lift が認識できるようにし、止まりを無くす。

目標:
1. `0x44f273`(Sytrus の基底からの RVA)を含む、Sytrus の認識できない `jmp reg` の全件の形を、`pm-work/findings/switch-tables.md` に形ごとに書く。(前回の試行で済み。findings に在る)
2. 認識できない表の形(前回の試行の F1〜F3)を、誤認識(飛び先が関数の範囲の外、または命令の途中)を 0 件にして認識する。(前回の試行で済み)
3. Sytrus・FLEngine・Balance で、認識できない表の形が 0 件であることを、判定の道具で確かめる。OTT は入力が無いので、判定に入れない。

## 前回の試行で分かったこと(この Issue の 1 回目。run 45557815540。runner の値。nproc 4)

- 依頼の前提の訂正: `0x44f273` の表は、ガードの無い形ではない。`cmp DWORD PTR [rsp+0x88],0x1e`(RVA `0x44f24a`)・`ja`(`0x44f252`)が在る。ただし比べる値はメモリの slot にあり、`movsxd rax,DWORD PTR [rsp+0x88]`(`0x44f258`)で同じ slot を読み直す。表は RVA `0x4520d0`・N = `0x1e`・31 項で、項 0 が `0x44f273`。
- 直す前の基準値(`fe-decode jmp-reg` で数えた。表の件数):

| DLL | 間接の `jmp reg` | 解決した表 | 表の形で認識できない | 表ではない | 誤認識 |
|---|---|---|---|---|---|
| FLEngine_x64.dll | 20 | 3(注釈) | 0 | 17 | 0 |
| Sytrus_x64.dll | 17 | 0 | 3 | 14 | 0 |
| Balance | 15 | 0 | 2 | 13 | 0 |

- 認識できない表の形は 3 つ(VA は ImageBase `0x400000` を足した値):
  - F1 メモリ上のガード: Sytrus `0x84f271`。旧道具の診断は `N not derived`。
  - F2 表相対の `movsxd`、ガードが直前: Sytrus `0x55f666`・Balance `0x54a9b6`。診断は出ず、無言で `Ok(None)` だった。FLEngine の 3 件も同じ形で、注釈で手により解決済み。
  - F3 表相対の `movsxd`、ガードと idiom の間に 2 命令: Sytrus `0x563298`・Balance `0x54e5e8`。基底の r15 は、保存と復帰の `mov` の後の値。
- 表ではない `jmp reg`(14・13・17 件)は、`call <stub>` の後に引数を戻して `jmp rax` する遅延読み込みの戻り。表ではないので、判定に入れない(JSON に出す)。
- 前回の差分の中身(work.diff): (a) F1: ガードの slot と読み直しの slot が同じ時だけ N を取る。違えば `N not derived` のまま。(b) F2・F3: 表相対の読み方(標準と scheduled の両方)。(c) 基底の証明: `mov B,S` の複写を辿る。保存・復帰の循環は、組が証明の途中に在る時だけ仮定する。深さは 8。飛び先の検査は既存の `resolve_msvc` と同じ。
- 誤認識は 0 件(`jmp-reg` で、飛び先が関数の範囲の中・命令の先頭・他の命令の中ではない、を数えた)。
- decision の番号: 前回は 0110 を使ったが、0110 は VFS の別件で使用済み。0430 に移した。この repo に決定ログは無い。登録はしない(理由は、コードの注記と findings に書く)。
- 判定の道具の測定(`tools/switch-tables/check.mjs` の出力。直した後): FLEngine: resolved 3・表の形 0・misrecognized 0。Sytrus: resolved 3(基準 0)・表の形 0(基準 3)・misrecognized 0。Balance: resolved 2(基準 0)・表の形 0(基準 2)・misrecognized 0。OTT: not measured(入力が無い)。
- デモ(`Synth_dfr - Overcome - Sytrus.flp`): `not a known function entry` と `0xc9cf273` は 0 件。残りの停止は `fle: CreateFileW: empty path is not defined`(`Sytrus\Data\Default.dat` の不在)で、#{{171}} の範囲。デモはまだ開かない(timeout 180000 ms)。
- 起動(boot-check): error null・engine-idle・standIn false・elapsedMs 8,218。流れ A〜D: elapsedMs 7,683〜7,845、preBootBytes 312,178,232、4 行とも合格。B と D は welcomeClosed true。`npm test --prefix app`: 1406 passed。
- 受け入れを止めたのは `cargo test -p fe-decode -p fe-lift` の段。fe-decode は全て通る。fe-lift は 446 件が落ちる。HEAD でも同じ 446 件が落ちる(失敗名の集合が一致)。原因は試験の入力と link: `auth117/mk_signed_pe.py` が git に無い、C の試験の link に `fle_clip_host_write` の定義が無い、RCPPS の host の比較。fe-lift の switch 表の試験(`msvc_switch` 25・`jumptab` 5・`ldst_switch` 4)は通る。
- 時間: エンジンの作り直しは 25 分、bundle は 13 分。
- 前回の差分は work.diff として保存された(commit は main に無い)。次の試行は、その差分が載った作業ツリーから始まる。

## 背景

- 事実(#{{157}} の 2 回目の試行。run 45518902053): Sytrus の wasm を起動の後に読み込んだ後、synth と olbaid の demo が `indirect call to 0xc9cf273 is not a known function entry` で止まる。ゲストの VA `0xc9cf273` は、Sytrus の基底 `0xc580000`(wasm の名前 `sytrus_x64.c580000` の番号)に対する RVA `0x44f273`。関数 `0x44f130` の途中で、関数の先頭ではない。
- 飛びの形: `mov eax,[rcx+rax*4+0x4520d0]; add rax,rcx; jmp rax`。`mov eT,[B+I*4+T2]; add rT,B; jmp rT` の形。直前に `cmp`・`ja` のガードが無いと最初は考えたが、前回の試行で、ガードは在ると分かった(上の節)。
- 認識の仕組み: `lift/crates/fe-decode/src/jumptab.rs` の先頭の注釈(decision 0089・0094・0095・0109)。`recognize_msvc` は `[cmp idx,N; ja D;]` の形から N を決める。`recognize_msvc_scheduled` は、ガードが離れている形を、その入り口(`derive_scheduled_guard`)から求める。N が決まらない形は表を採らず、`Unresolved` のまま残り、`jumptab_rejected` に記録される。
- 停止の経路: 残った間接の飛びは、実行時に既知の関数の入口だけへ飛ぶ。入口でない飛び先では `is not a known function entry` で止まる。
- 参考(報告。main には無い): 別の `jmp reg` の表は、73 件のうち 40 件だけ列挙できた(ILSvg。#{{154}} の作業)。33 件は表の形を認識できない。
- 既存の試験: `lift/crates/fe-decode/tests/msvc_switch.rs`・`lift/crates/fe-decode/tests/jumptab.rs`・`lift/crates/fe-lift/tests/msvc_switch.rs`・`lift/crates/fe-lift/tests/ldst_switch.rs`。実物の DLL を読む試験は、`FLENGINE_PE`(無ければ `binaries/win64/FLEngine_x64.dll`)を使う(`lift/crates/fe-decode/tests/adjustor.rs` が手本)。
- Sytrus の DLL は、取り出し後に `lift/out-plugins/sytrus_x64.dll`(6,269,440 B。sha256 `e1395e97…`)。

## 読む物

- `lift/crates/fe-decode/src/jumptab.rs`(先頭の注釈と、`recognize_msvc`・`recognize_msvc_scheduled`・`derive_scheduled_guard`・`resolve_msvc`・`verify_msvc`)
- `lift/crates/fe-decode/src/indirect.rs`・`lift/crates/fe-decode/src/lib.rs`(`jumptab_rejected` と `kept unresolved` の記録。625 行付近)
- `lift/crates/fe-lift/src/jumptab.rs`・`lift/crates/fe-lift/src/indirect.rs`
- 試験: `lift/crates/fe-decode/tests/msvc_switch.rs`・`lift/crates/fe-decode/tests/jumptab.rs`・`lift/crates/fe-lift/tests/msvc_switch.rs`
- `inputs/plugin-classes.tsv`・`inputs/plugin-hooks.tsv`(Sytrus の行)
- `objdump -d`(関数 `0x44f130` から `0x44f273` までの命令列と、表の位置)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 前回の差分が作業ツリーに載っているかを、`git status` と `git diff --stat` で確かめる。載っている物は取り除かず、残りを行う。載っていなければ、前回の差分の内容から作り直す(違いは報告に書く)。載っているべき物: `lift/crates/fe-decode/src/`(`jumptab.rs`・`lib.rs`・`main.rs`・`survey.rs`)、`lift/crates/fe-decode/tests/`(`msvc_slot.rs`・`msvc_switch.rs`・`switch_tables.rs`)、`pm-work/findings/switch-tables.md`、`tools/switch-tables/check.mjs`。
2. 認識の変更(F1〜F3 と基底の証明)は済んでいる。新しい認識の変更は足さない。
3. 判定の道具 `tools/switch-tables/check.mjs` を、下の「判定の基準」に合わせて直す。FLEngine・Sytrus・Balance の全てで、`unrecognized_table_shape` が 0、`misrecognized` が 0、`resolved_tables` が FLEngine 3 以上・Sytrus 3 以上・Balance 2 以上であることを判定する。基準の表との比較は、判定に使わない。`unrecognized_other`(表ではない `jmp reg`)は判定に入れず、行に出すだけ。OTT は判定に入れず、`not measured` の行を出す。
4. decision 0430 の注記(`lift/crates/fe-decode/src/jumptab.rs` など)に、この repo に決定ログが無いことを書く。登録はしない。
5. 基底の証明の (c)(保存・復帰の循環の仮定)は、findings に書く。PM が後で読む。受け入れには入れない。
6. 受け入れを通す(下の「受け入れ」)。通らなければ、作業ツリーを commit して終える。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && cargo test --locked --manifest-path lift/Cargo.toml -p fe-decode && cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift --test msvc_switch --test jumptab --test ldst_switch --test seh_rt && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/switch-tables/check.mjs && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && mkdir -p .tmp-sytrus && cp "lift/out-app/host/resources/Data/Demo projects/Product demos/Synth_dfr - Overcome - Sytrus.flp" .tmp-sytrus/synth.flp && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" "$PWD/.tmp-sytrus/synth.flp" > "$RUNNER_TEMP/synth.json" && node -e 'const s=require("fs").readFileSync(process.argv[1],"utf8");const r=JSON.parse(s.trim().split("\n").pop());process.exit(r&&typeof r==="object"&&!s.includes("not a known function entry")?0:1)' "$RUNNER_TEMP/synth.json" && npm test --prefix app
```

判定の基準:
- `cargo test -p fe-decode` が全て通る(`switch_tables`・`msvc_slot`・`msvc_switch` を含む)。`cargo test -p fe-lift --test msvc_switch --test jumptab --test ldst_switch --test seh_rt` が通る。`cargo test -p fe-lift` の全体は判定に使わない(HEAD でも 446 件が、試験の入力と link の問題で落ちる。この変更と無関係)。
- `tools/switch-tables/check.mjs` が exit 0。FLEngine・Sytrus・Balance の全てで、`unrecognized_table_shape`(表の形の認識できない `jmp reg`)が 0、`misrecognized`(誤認識)が 0、`resolved_tables` が FLEngine 3 以上・Sytrus 3 以上・Balance 2 以上。`unrecognized_other`(表ではない `jmp reg`)は判定しない(行に出すだけ)。OTT は判定しない(`not measured` と出る)。
- デモ(`Synth_dfr - Overcome - Sytrus.flp`)の `tools/p3-audio-probe/measure.mjs` の JSON に、`not a known function entry` が無い(0 件)。`CreateFileW: empty path`(#{{171}} の範囲)は判定しない。停止の理由は JSON の `error` と `end` に出す。
- 起動 1 回で、engine-idle・standIn が false・error が null。
- 流れ A〜D(`welcome-flow.mjs`)の 4 行: 各行で `error` が null・`state` が engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: `0x44f273` の形(命令の列)、基準値の表(形ごとの件数)、広げた認識の件数と、誤認識の件数、`check.mjs` の出力の行(`unrecognized_other` と OTT の行を含む)、デモの JSON の `error`(停止の理由)、流れ A〜D の 4 行、`elapsedMs`・`preBootBytes`、`lift/`・`app/` を変えたこと(push すると `deploy.yml` が動く)。
- 基底の証明の (c) の仮定は findings に書く(受け入れには入れない)。decision 0430 は決定ログに登録しない(この repo に決定ログは無い)。

## 制約

- 製品に、新しい環境変数(ビルド・実行を切り替える旗)・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 認識を広げるのは既定の動作にする。オプトインを作らない。
- 誤認識を隠すための握りつぶし(飛び先の検査を外す・停止を飲み込む)をしない。
- 名前の規則: 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。(`guard/scan.sh` が AI のベンダー名を検査する。Cloudflare の資源の名前・アカウント ID・ホスト名は書かない。)
- Python と OCR を使わない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。
- エンジンのビルドは約 35 分。フォアグラウンドで直接実行し、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。`sleep`・`timeout` をコマンドの先頭に書かない。

## 他のタスクとの関係

- blocked_by: #{{157}}(Sytrus の wasm を、起動の後に読み込めるようにする。段階 1)。閉じた後に起動する。
- 後続: #{{147}}(Riff machine の窓)は、この Issue と #{{171}} が閉じた後に起動する。
- #{{171}}(Sytrus の Data)とは独立。どちらも demo が最後まで開くために要る。
