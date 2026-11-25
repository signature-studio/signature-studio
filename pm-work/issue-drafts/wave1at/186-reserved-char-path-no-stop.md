---
draft: "186"
title: "パスの予約文字(コロン(NTFS のストリーム)・<・>・|・二重引用符など)を含む呼び出しで、engine を止めない: Windows の返り値に従う"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: ["154"]
labels: ["verify"]
size: S
priority: High
project: signature-studio/3
---

## 目的

crash-hunt の 2 回目の走査で、Settings の 6 つ目の頁(経路 `settings > tab 6 of 16`。DPR 1.7)が、`fle: GetFileAttributesW: reserved character in a path component (streams are not implemented)` で engine を止めた(SIGABRT)。パスの予約文字(`:`・`<`・`>`・`|`・二重引用符・1〜31 の制御文字)を含む呼び出しで、engine を止めず、Windows の失敗の返り値を返す。

この Issue の目標は次の 3 つ。
1. 予約文字を含むパスの呼び出しが止まらず、Windows の返り値(資料と Wine の試験で確かめた値)を返すこと。
2. `:` を含む名前(NTFS のストリーム)を、この仮想のボリュームがストリームを持たないことに従って答えること。
3. 経路 `settings > tab 6 of 16`(DPR 1.7)が、crash-hunt で `ok` になること。

## 背景

- 事実(crash-hunt の 2 回目。#1417 完了。`pm-work/findings/crash-hunt-2026-11-25.md` と `.json`): probe `settings tab 5`(経路 `settings > tab 6 of 16`・DPR 1.7)は `stopped`・`exited`。停止の指紋は `fle: GetFileAttributesW: reserved character in a path component (streams are not implemented)`。止めた場所は `lift/runtime/fle_vfs.c` の `check_component`(520 行付近。予約文字の検査は 527 行付近)。
- どのパスか: 停止の記録は、止めた呼び出しのパスを出さない。同じ probe の `logTail` の `GetFileAttributesW` の行は、検査を通った名前の記録(`missing-path`)だけである。テーマの画面(page: Theme)の `orange.flstheme`(記録では `<path>` の形で 4 回)と、`C:\Program Files\Image-Line\FL Studio 21\Artwork\Themes\WitchBG.png` の 2 種類。予約文字を含む名前は検査で止まり、`missing-path` の行を出さないので、止めたパスはこの中に無い。止めたパスは、直す前に一時の診断(commit しない)で出して特定し、報告に書く。
- 同じ停止の箇所は、#{{154}}(英語以外の言語。作業中)が、`?` を含むパス(`OpenSans-Thông thu?ng.ttf`)で踏んだ。#{{154}} の本文は、同じ `check_component` の `?`・`*` の停止を、Windows の返り値に直す方針(API ごとに返り値を決める)である。この Issue は、同じ関数の予約文字の停止を直す。
- 既存の仕組み: `fle_warn`(`lift/runtime/fle_env.c`・`fle_env.h`。#1418 の暫定の答えの警告)は、API ごとに 1 回だけ、`fle: warn: <api>: not modelled, returned <value> (caller 0x...)` を出す。この Issue の警告は、パスごとに 1 回なので、`fle_warn` の挙動は変えずに、同じ形の行を出す仕組みを足す。
- 分類: `pm-work/findings/runtime-stop-sites-classified.tsv` の `fle_vfs.c` の `check_component` の行(分類 (b)・返り値の規約 BOOL・状態 未了)。
- Windows の規則(資料): Microsoft の「Naming Files, Paths, and Namespaces」は、予約文字として `< > : " / \ | ? *` を挙げ、1 から 31 の文字は、ストリームの名前を除いて使えないとする。「File Streams (Local File Systems)」は、NTFS のストリームの名前を `ファイル名:ストリーム名:型` の形と書く。どちらの資料も、関数の返り値は書いていない。返り値は、Wine の試験(`dlls/kernel32/tests/file.c`・`path.c`)で確かめる。資料は題名で書き、URL は本文に書かない。
- 想定(まだ確かめていない): `< > " |` と制御文字を含む名前は `ERROR_INVALID_NAME`(123)。`:` を含む名前は、ストリームが無いとして `ERROR_FILE_NOT_FOUND`(2)。想定と確かめた値が違えば、確かめた値に従い、報告に書く。

## 読む物

- `lift/runtime/fle_vfs.c`: `check_component`(520 行付近)・`parse_path`(537 行付近)と、その呼び出し元(`grep -n 'parse_path(' lift/runtime/fle_vfs.c`)・`fle_vfs_note_missing`(131 行付近)・`put_name_ok`(2912 行付近。ホスト側の名前の検査。止めない。変えない)。#{{154}} が入れた返り値の仕組み(`git log --oneline -- lift/runtime/fle_vfs.c` で、main に入った変更を確かめる)。
- `lift/runtime/fle_env.c`・`lift/runtime/fle_env.h`(`fle_warn`)。
- `lift/crates/fe-lift/tests/vfs.rs`(`undefined_input_stops_the_run` の 920 行付近。`stream` の行 `C:\f.txt:s` は、この Issue で答えが変わる)・`lift/crates/fe-lift/tests/fsx_common/mod.rs`(試験の補助)・`lift/crates/fe-lift/tests/known-failures.txt`・`tools/fe-lift-test.mjs`。
- `pm-work/findings/crash-hunt-2026-11-25.json`(probe `settings tab 5`。`logTail`・`stop`)・`pm-work/findings/runtime-stop-sites-classified.tsv`。
- `tools/crash-hunt/hunt.mjs`(`randomDpr` の 147 行付近。Settings の頁の probe は `extraTasks` の 419 行付近。probe の id は `settings tab 5`)。

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば最初に読む。#{{154}} の変更が main に在ることを、`git log` で確かめる。
1. Windows の返り値を資料と Wine の試験で確かめ、表(API・入力・返り値・根拠)を作る。`parse_path` を呼ぶ API ごとに確かめる(`CreateFileW`・`GetFileAttributesW`・`GetFileAttributesExW`・`SetFileAttributesW`・`DeleteFileW`・`MoveFileExW`・`CopyFileW`・`CreateDirectoryW`・`RemoveDirectoryW`・`FindFirstFileW`・`GetFullPathNameW` など)。特に確かめること:
   - (a) `< > " |` と制御文字の返り値。
   - (b) `:` の返り値。ストリームが無い時・基のファイルが無い時・フォルダの名に `:` が在る時・作成の要求(`CREATE_NEW`・`OPEN_ALWAYS`・`CREATE_ALWAYS`・`TRUNCATE_EXISTING`)の時。
   - (c) `GetFullPathNameW` が名前を検査するか(文字列の処理なら、予約文字で失敗しない可能性がある)。
   - (d) `FindFirstFileW` のパターンで `<`・`>`・`"` が、従来の DOS のワイルドカードとして扱われるか。
   - 確かめられない点は、報告に「確認できない」と書く。その API は、止めたまま残し、報告に書く。
2. 停止したパスを特定する。直す前に、一時の診断(commit しない)で `check_component` の入力を出し、probe `settings tab 5` のパスを報告に書く。
3. 試験を先に red で書く。`lift/crates/fe-lift/tests/vfs.rs` に、予約文字の名前(`< > " |`・制御文字・`:` のストリーム)の試験を足す。`undefined_input_stops_the_run` の `stream` の行(`C:\f.txt:s`)は、この試験に移す(その行は、この Issue で答えが変わる)。`node tools/fe-lift-test.mjs --test vfs` で red を確かめる。
4. 直す。`check_component` の予約文字の停止を、#{{154}} が入れた返り値の仕組みで置き換える(新しい仕組みを作らない)。
   - `:` は、ストリームの名前として扱い、この仮想のボリュームはストリームを持たないので、そのストリームは無い、と答える(返り値は (b) で確かめた値)。
   - 制御文字は、ストリームの名前の部分では許されている(資料)。その扱いは資料に従う。
   - どの API も、確かめた返り値を返す。
   - 警告: パスごとに 1 回、`fle: warn: <API>: reserved character in "<パス>" (caller 0x…)` を標準エラーに出す。caller は `fle_warn` と同じ方法で取る。`fle_warn` 自体の挙動は変えない。
   - `check_component` の他の停止(255 字を超える・末尾の空白やピリオド・デバイスの名前)は、この Issue では変えない。
5. 試験を green にする。`node tools/fe-lift-test.mjs` の全件で、新しい失敗が 0 であることを確かめる。直った既知の失敗があれば、`lift/crates/fe-lift/tests/known-failures.txt` の行を消す。
6. 経路を再現する。`tools/crash-hunt/run.sh` の `--only`(`tools/crash-hunt/hunt.mjs` に無ければ足す。#{{172}} が足す形と同じにする。#{{172}} が main に入っていれば使い、二重に足さない)で、probe `settings tab 5` を DPR 1.7(`--seed 20251125`)で走らせ、`ok` を確かめる。別の指紋に移ったら、その指紋を報告に書く。
7. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/fe-lift-test.mjs && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && mkdir -p "$RUNNER_TEMP/hunt-186" && bash tools/crash-hunt/run.sh --budget 15m --seed 20251125 --out "$RUNNER_TEMP/hunt-186" --only 'settings tab 5' --bundle "$PWD/lift/out-app/bundle/raw/fl-Oz" && node -e 'const fs=require("fs");const P=JSON.parse(fs.readFileSync(process.argv[1]+"/crash-hunt-state.json","utf8")).probes.filter((p)=>p.id==="settings tab 5"&&p.dpr===1.7);process.exit(P.length>=1&&P.every((p)=>p.result==="ok")?0:1)' "$RUNNER_TEMP/hunt-186"
```

判定の基準(全て満たす):
- 起動確認(`boot.json`)が engine-idle・`standIn` が false・`error` が null・`elapsedMs` が 12,500 以下。
- `node tools/fe-lift-test.mjs`(全件)で、新しい失敗が 0。red から green にした新しい試験を含む。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`)の 4 行の判定(`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true)。
- `npm test --prefix app` が通る。
- 経路の再現: probe `settings tab 5` の DPR 1.7 の行が、全て `ok`。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 停止したパス(特定の方法を含む)、Windows の返り値の表(API・入力・返り値・根拠の資料か Wine の試験)、確かめられなかった点、red と green の出力、直した API の数と名前、警告の行の 1 例、probe `settings tab 5` の結果(`ok` か、移った先の指紋)、流れ A〜D の 4 行。
- 変えたパス(`lift/` を変えると main の deploy が動く)。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python を使わない。OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 資料は、題名と年で書く。URL を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000 にする。背景で起動して待たない。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #{{154}}。同じ関数(`check_component`)を触るので、その後に始める。
- `--only`: #{{172}} が `tools/crash-hunt/hunt.mjs` に足す。main に在れば使い、無ければ同じ形で足す(二重に足さない)。
- 暫定で止めている API を Windows の返り値で続けるようにする別の Issue(題名に「暫定で止めている API」を含む)と、同じ行を触りうる。衝突したら、両方の変更を残す。
- 作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
