---
draft: "172"
title: "GetFileVersionInfoSizeW などの、ディレクトリ部が無いファイル名の呼び出しで、engine を止めない: LoadLibrary と同じ探索の順序を実装する"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["verify"]
size: S
priority: Urgent
project: signature-studio/3
---

## 目的

ディレクトリ部の無いファイル名(`ILSvg_x64.dll` のような、名前だけの指定)で版情報の API(`GetFileVersionInfoSizeW`・`GetFileVersionInfoW`)を呼んだ時に、engine を止めない。Windows と同じく、`LoadLibrary` と同じ探索の順序でファイルを探し、見つからない時は Windows と同じ失敗を返す。

この Issue の目標は次の 3 つ。
1. ディレクトリ部の無い名前で、止まらず、探索の順序で最初に見つかったファイルの答えを返すこと。
2. 見つからない時は、Windows と同じ失敗(0 と `GetLastError` = `ERROR_FILE_NOT_FOUND`)を返すこと。
3. `GetFileVersionInfoSizeW`・`GetFileVersionInfoW`・`LoadLibraryW` が、同じ探索の順序を使うこと(別々の探索を持たない)。

## 背景

- 道具の実測(`pm-work/findings/crash-hunt-2026-11-25.json` の `File/9/11`。DPR 1)。経路 `File > #9 > #11` を踏むと、`fle: GetFileVersionInfoSizeW: a file name with no directory part: the LoadLibrary search sequence is not modelled (decision 0098)` で SIGABRT する。開いていた窓は `TQuickPopupMenuWindow`。
- 現状のコード: `lift/runtime/fle_vfs.c` の `fle_imp_GetFileVersionInfoSizeW`(2103 行付近)は、ディレクトリ部(区切り文字か `:`)の無い名前で止める。同じ止めが `fle_imp_GetFileVersionInfoW`(2156 行付近)にもある。
- 決定 0098 の注記(`fle_vfs.c` の 2075 行付近)は、仮想のファイルシステムに無いパスの答えを、そのパスの失敗(`GetFileAttributesW` と同じ 2 か 3)とする。決定 0098 の本文はリポジトリに無い(`docs/` は無い)。注記だけが、その内容を書いている。
- `LoadLibraryW`(`lift/runtime/fle_load.c` の 480 行付近)は、ディレクトリ部の無い名前を、決定 0233 の `search_volume` で探す。`GetFileVersionInfoSize` の説明は、ファイル名の探索に「`LoadLibrary` 関数が指定する検索の順序」を使うと書いている。
- 資料: Microsoft の「Dynamic-Link Library Search Order」(DLL の検索順序の説明)と、Wine の `dlls/version/version.c`(`GetFileVersionInfoSize`)。どちらも、読んだ題名と年を報告に書く(URL は書かない)。

## 読む物

- `lift/runtime/fle_vfs.c`(2075〜2200 行: `GetFileVersionInfoSizeW`・`GetFileVersionInfoW`・`is_engine_path`・`parse_path`・`lookup`・`note_missing`。`VerQueryValueW` は 2290 行付近)
- `lift/runtime/fle_vfs.h`(69〜71 行)・`lift/runtime/fle_env.h`(`FLE_VIRTUAL_*` の定義。40〜53 行)
- `lift/runtime/fle_load.c`(`LoadLibraryW` は 480 行付近。`search_volume`・`not_found`・決定 0233 の注記)
- `lift/crates/fe-lift/tests/wall_e1.rs`(`GetFileVersionInfoSizeW` の既存の試験。191 行・705 行付近)
- `pm-work/findings/crash-hunt-2026-11-25.json`(`File/9/11` の `logTail` と `stop`)
- `tools/crash-hunt/hunt.mjs`・`tools/crash-hunt/run.sh`(道具)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 探索の順序を資料で確かめる(上の 2 件)。`LoadLibraryW` の今の探索(`fle_load.c`)が、どの仮想のフォルダを、どの順で見るかを、コードで確かめる。
2. **試験を先に、red で書く**(`lift/crates/fe-lift/tests/` に足す。既存の `wall_e1.rs` の形を使ってよい)。確かめること: (a) ディレクトリ部の無い名前で、順序の最初の場所に在るファイルの答えが返る、(b) 無い時は 0 と `GetLastError` = 2(`ERROR_FILE_NOT_FOUND`)、(c) どの場合も止まらない。
3. 直す。`GetFileVersionInfoSizeW` と `GetFileVersionInfoW` の、ディレクトリ部の無い名前の停止を、探索の結果に替える。探索は `LoadLibraryW` と同じ関数(または同じ順序の表)を使い、2 つ目の探索を作らない。
   - 見つからない時: 0 と `ERROR_FILE_NOT_FOUND`(2)。`*lpdwHandle` は 0(既存の注記のとおり)。
   - 見つかったファイルが、仮想のファイルの時: 版の資源をこの版はモデルしていない(既存の注記のとおり)。この場合の停止は、ディレクトリ部のある名前と同じに残す。ただし、止める文言に、探索で見つかったパスを入れる。
4. 道具で再現する。`tools/crash-hunt/` に、経路を選ぶ `--only <id>[,<id>...]` を足す(まだ無い時だけ。既定の動作は変えない。経路の id は `File/9/11` の形)。`--only` の仕様は `#{{173}}` と `#{{174}}` も使う。main に入っていれば、それを使い、二重に足さない。`File/9/11` を DPR 1 で踏み、`ok` になることを確かめる。
5. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && mkdir -p "$RUNNER_TEMP/hunt-172" && bash tools/crash-hunt/run.sh --budget 15m --out "$RUNNER_TEMP/hunt-172" --only File/9/11 --bundle "$PWD/lift/out-app/bundle/raw/fl-Oz" && node -e 'const fs=require("fs");const P=JSON.parse(fs.readFileSync(process.argv[1]+"/crash-hunt-state.json","utf8")).probes.filter((p)=>p.id==="File/9/11");process.exit(P.length>=1&&P.every((p)=>p.result==="ok")?0:1)' "$RUNNER_TEMP/hunt-172"
```

判定の基準(全て満たす):
- 起動確認(`boot.json`)が engine-idle・`standIn` が false・`elapsedMs` が 12,500 以下。
- `cargo test -p fe-lift` が全て通る(red から green にした新しい試験を含む)。
- 流れ A〜D の 4 行の判定(`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true)。
- `npm test --prefix app` が通る。
- 道具の再現: `File/9/11` の probe の全てが `ok`。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 探索の順序(資料の題名と年と、採った順序)、見つからない時と見つかった時の答え、試験の red と green の出力、`File/9/11` の probe の結果、変えたパス(`lift/` を変えると main の deploy が動く)、流れ A〜D の 4 行。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- Python と OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 資料は、題名と年で書く。URL を書かない。
- sleep・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 起動の間欠の停止や CDP の timeout で流れ A〜D が落ちたら、その行を報告に書いて commit して終える(再試行で続く)。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by は無い。
- `#{{173}}`(View の経路の null の呼び出し)とは、触る場所が違う。
- `#{{174}}`(crash-hunt の続き)と `tools/crash-hunt/hunt.mjs` を共有する。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- `#{{175}}`(暫定で止めている API の分類)とは、`lift/runtime/fle_vfs.c` の同じ領域を触りうる。衝突の扱いは同じ。
