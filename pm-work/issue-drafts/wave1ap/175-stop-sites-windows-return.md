---
draft: "175"
title: "暫定で止めている API(1,261 箇所)を、Windows の失敗の返り値で続けるように直す: 返り値の規約ごとに分類し、失敗が正規の結果である API から直す"
type: Task
milestone: "M5 内蔵のエフェクトとインストゥルメント"
parent: "018"
blocked_by: []
labels: ["verify"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

`lift/runtime` の `fle_stop`(engine を止める関数)のうち、Windows の挙動が未確認、または未実装のため、暫定で止めている箇所を、Windows の失敗の返り値で続けるように直す。利用者の操作で当たりうる、止めるべきでない API から直す。

依頼者の「まだ非常にクラッシュしやすい」は、この暫定の停止が原因の主な物である。暫定の停止に当たるたびに、engine 全体が止まる。

この Issue は大きいので、段階を分ける。この Issue の段階 1 は、分類の表と、直す API の family の一部である。段階 2 以降は、PM が表を見て family ごとに Issue にする。

## 背景

- `pm-work/findings/runtime-stop-sites.md`(#1411 の作業で作った表)。`lift/runtime` の `fle_stop(` の呼び出しは 1,394 箇所。同じ文言の行をまとめて表にしている。分類は、(a) 133 件(内部の不変条件・資源・ホストの限界。止めてよい)と、(b) 1,261 件(Windows の挙動が未確認、または未実装のため、暫定で止めている)。(b) の行は、この Issue の対象。
- 実機の記録: `pm-work/findings/real-user-stops-2026-11-25.md`(#1404 の集計)。停止の一部は、暫定の停止に当たっている。例: 英語以外の言語の起動で `CompareStringW` の停止が出る(`#1395` の記録)。
- 同じ型の直し: `#1411` は、`RegOpenKeyExW` の空の要素を、Windows の返り値(`ERROR_BAD_PATHNAME`)で続けるように直した(`lift/runtime/fle_reg.c`)。この Issue は、その型を、他の API の family に広げる。
- 方針: 止める代わりに、資料で確かめた Windows の失敗の返り値を返し、警告の 1 行を出す。止めるべき API(メモリ・スレッド・同期・SEH・描画の根幹・ロードの根幹)は、止めたままにする。
- 常に動く軽い記録: 警告の API 名ごとの回数を、ビーコンに載せる。`engine.exit` の項目ではなく、ビーコンの `files`(`app/src/files/usage.ts` の型)の近くに、`warns`(API 名 → 回数。上位 20 件)として載せる。ビーコンの型は `app/src/perf/beacon.ts`。実機のビーコンで、どの API の「未確認」が実際に呼ばれるかが分かる。

## 読む物

- `pm-work/findings/runtime-stop-sites.md`(全体。分類の規則と、表の行の形)
- `pm-work/findings/real-user-stops-2026-11-25.md`(実機の停止の指紋)
- `pm-work/findings/crash-hunt-2026-11-25.md`・`.json`(道具の停止の指紋)
- `lift/runtime/fle_env.c`(`fle_stop` の定義と、警告の出し方)・`lift/runtime/fle_env.h`
- `lift/runtime/fle_reg.c`(`#1411` の直し。`ERROR_BAD_PATHNAME` の返し方と、警告の 1 行の形)
- `lift/crates/fe-lift/tests/reg.rs`(`undefined_input_stops_the_run` の形と、返り値の試験の書き方)
- `app/src/perf/beacon.ts`・`app/src/files/usage.ts`・`app/src/perf/exit-message.ts`(ビーコンの型と、停止の文言の扱い)
- `tools/perf-beacons/summary.mjs`(ビーコンの読み手)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **分類の表を作る**(段階 1 の中心)。`pm-work/findings/runtime-stop-sites.md` の (b) の行を、`pm-work/findings/runtime-stop-sites-classified.tsv` に写し、次の列を足す。
   - `区分`: `失敗が正規の結果`(ファイル・ディレクトリ・レジストリ・ロケール・シェル・バージョン情報・ネットワーク・COM の生成・`GetProcAddress` の NULL・デバイス・クリップボード・プリンタ・IME・UI Automation・WebView2 の一部。利用者の操作で、失敗がある得る物)・`止めるべき`(メモリ・スレッド・同期・SEH・描画の根幹・ロードの根幹)・`判断保留`(どちらか分からない。PM に上げる)。
   - `返り値の規約`: `BOOL`(失敗は FALSE と `SetLastError`)・`HANDLE`(NULL か `INVALID_HANDLE_VALUE`)・`HRESULT`(失敗の `E_*`)・`ERROR`(エラーコードを返す API)・`COUNT`(個数を返す。失敗は 0)・`POINTER`(失敗は NULL)・`VOID`(戻り値の無い API)・`NONE`(止めるべき・判断保留の行)。
   - `状態`: `直した`(止めていない。返り値を返す)・`残した`(止めるべき・判断保留の行)・`未了`(失敗が正規の結果だが、まだ直していない)。
   - 列は、`ファイル`・`行`・`API`・`止める条件`・`分類`(md からそのまま)・`区分`・`返り値の規約`・`状態` の順。1 行目は見出し。
   - 機械で当たる物は機械で当て、判断が要る物は人が決める。判断の根拠を、表の横の注記(`pm-work/findings/runtime-stop-sites-classified.md` の 1 ページ)に書く。
2. **資料で返り値を確かめる**。`区分` が `失敗が正規の結果` の API ごとに、Windows の失敗の返り値と `GetLastError` の値を、公開の資料(Microsoft の説明・Wine のテスト `dlls/*/tests`)で確かめる。確かめた資料は、題名と年で書く。確かめられない物は、その API の最も一般的な失敗(例: ファイルの無い `ERROR_FILE_NOT_FOUND`)か `ERROR_CALL_NOT_IMPLEMENTED`(120)を返し、警告を出し、報告に、確かめられなかったことを書く。
3. **段階 1 の family を直す**。直すのは、次の family のうち、表と実機の記録(`real-user-stops-2026-11-25.md`・道具の `crash-hunt`)に出た API に限る。
   - ファイル名と版情報(`GetFileVersionInfo*`。`#{{172}}` と重なるので、`#{{172}}` が main に在ればそれを使う)。
   - レジストリ(`RegOpenKeyExW` 以外の鍵の API。`#1411` で直した `RegOpenKeyExW` とは別の経路)。
   - `GetProcAddress` の NULL(失敗の返り値の NULL を返す)。
   - ロケール(`CompareStringW` など。`#1395` の記録に停止が出る)。`#1395` が作業中の場合は、同じ行を二重に変えない。
   - シェルのフォルダ(`SHGetFolderPathW` など)。
   - 直した API の 1 つずつに、返り値と `GetLastError` の期待値の試験を足す(**先に red で書く**。`lift/crates/fe-lift/tests/` の既存の試験の形を使う)。
4. **警告の 1 行**。止めずに返り値を返す時は、同じ API の 1 回目だけ、次の形の 1 行を stderr に出す(静的な集合で覚える。2 回目以降は出さない)。
   `fle: warn: <API>: not modelled, returned <値> (caller 0x…)`
   - `fle_stop` と同じ `fle:` の形にし、`warn` の語を入れる。呼び出し元の番地は、取れる時だけ。
5. **ビーコンの回数**。警告の対象になった API の呼び出し回数を、API 名ごとに数え、上位 20 件を、ビーコンの `warns`(API 名 → 回数)として載せる。ビーコンの型(`app/src/perf/beacon.ts`)と、作る所(既存の `files` の作り方の近く)を直す。環境変数・旗・URL パラメータを足さない。既存の読み手(`tools/perf-beacons/summary.mjs`)は、この欄を無視しても壊れない形にする(`warns` が無い古いビーコンも読める)。
6. **止める API は残す**。`区分` が `止めるべき` と `判断保留` の行は、止めたままにする(`状態` は `残した`)。
7. 受け入れを通す。通らなければ、表と、直した family の一覧を commit して終える(段階 1 が終わらなくても、再試行で続く)。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && cargo test --locked --manifest-path lift/Cargo.toml -p fe-lift && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && node -e 'const fs=require("fs");const md=fs.readFileSync("pm-work/findings/runtime-stop-sites.md","utf8").split("\n").filter((l)=>/^\| lift\/runtime\//.test(l));const mdB=md.filter((l)=>/\| \(b\) \|/.test(l)).length;const T=fs.readFileSync("pm-work/findings/runtime-stop-sites-classified.tsv","utf8").replace(/\r/g,"").trim().split("\n");const H=T[0].split("\t");const c=(n)=>H.indexOf(n);const R=T.slice(1).map((l)=>l.split("\t"));const B=R.filter((r)=>r[c("分類")]==="(b)");const AREA=["失敗が正規の結果","止めるべき","判断保留"];const ST=["直した","残した","未了"];const ok=mdB>0&&B.length===mdB&&B.every((r)=>AREA.includes(r[c("区分")])&&(r[c("返り値の規約")]||"").trim()!==""&&ST.includes(r[c("状態")]))&&R.filter((r)=>r[c("状態")]==="直した").every((r)=>r[c("区分")]==="失敗が正規の結果");const fixed=R.filter((r)=>r[c("状態")]==="直した").length;process.exit(ok&&fixed>=1?0:1)' && bash guard/scan.sh
```

判定の基準(全て満たす):
- 起動確認(`boot.json`)が engine-idle・`standIn` が false・`elapsedMs` が 12,500 以下。
- `cargo test -p fe-lift` が全て通る(直した API ごとの新しい試験を含む)。
- 流れ A〜D の 4 行の判定(`#1411` と同じ)。
- `npm test --prefix app` が通る(警告の記録と、ビーコンの `warns` の試験を含む)。
- `pm-work/findings/runtime-stop-sites-classified.tsv` が在り、`runtime-stop-sites.md` の (b) の行の数と一致する(未分類 0)。各行に `区分`・`返り値の規約`・`状態` が入る。
- `状態` が `直した` の行が 1 以上あり、全て `区分` が `失敗が正規の結果` である。
- `bash guard/scan.sh` が 0 件。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 分類の内訳(`区分`ごとの件数と、`返り値の規約` ごとの件数)、直した family と API の一覧、確かめた資料の題名と年、確かめられなかった API とその扱い、試験の red と green の出力、警告の行の出力例(1 API につき 1 回)、`warns` の出力例、流れ A〜D の 4 行、変えたパス(`lift/` と `app/` を変えると main の deploy が動く)。
- 段階 2 以降に残した family を、PM が Issue にできるように、一覧で書く。

## 制約

- 製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。警告とビーコンの `warns` は既定の動作として出す。オプトインを作らない。
- 止めるべき API を、返り値で続けさせない。判断に迷う物は、`判断保留` にして止めたままにする。
- Windows の返り値を、資料で確かめずに書かない。確かめられない時は、その旨を報告に書く。
- ベンダー名・Cloudflare の識別子・URL を書かない。資料は、題名と年で書く。
- Python と OCR を使わない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- 表(分類)を先に作って commit すると、段階 1 が終わらなくても、次の試行が続きから行える。表の commit を先にする。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 起動の間欠の停止や CDP の timeout で流れ A〜D が落ちたら、その行を報告に書いて commit して終える(再試行で続く)。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by は無い。
- `#{{172}}`(版情報の探索)・`#{{173}}`(null の関数の呼び出し)・`#{{174}}`(crash-hunt の続き)と、同じ領域を触りうる。`lift/runtime/fle_vfs.c` と `fle_dispatch.h` で衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
- `#1395`(英語以外の言語の起動)の、ロケールと `ILSvg_x64.dll` の停止とは、同じ API に当たりうる。
- `#1411`(`RegOpenKeyExW` の空の要素)で作った表と、直し方(返り値と警告の 1 行)を、この Issue の基準にする。
