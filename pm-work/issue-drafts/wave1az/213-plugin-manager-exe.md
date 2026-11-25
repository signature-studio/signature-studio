---
draft: "213"
title: "FL の Plugin Manager(外部の PluginManager.exe)を、VM の中で動かす: 実行ファイルの起動の要求を、同じ engine の中の、もう 1 つのゲストに結ぶ"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["198"]
labels: ["delivery", "plugin-builtin", "verify"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

#{{198}}(Plugin Database の作業)で分かった通り、FL の Options メニューの 6 番目の項目(Plugin Manager)は、engine の中の窓を開かず、外部の実行ファイル `System\Tools\Plugin Manager\PluginManager.exe` を起動する。VM はこの実行ファイルを持たないので、窓は出ず、engine は止まらない(`engine-idle` のまま)。

この Issue の目標は次の 3 つ。
1. FL が `PluginManager.exe` の起動を求めたとき、その実行ファイルを VM の中で動かし、Plugin Manager の窓を窓の表に出すこと。
2. その間も engine が止まらないこと(10 秒 `engine-idle` のまま)。
3. 起動の要求が、別のプロセスではなく、同じ engine の中の、新しいゲストのスレッドで走ること。

## 背景

- 実測(#{{198}} の作業): Plugin database の一覧には、`lift/plugins/` の 2 本(`fruity-balance`・`sytrus`)の分が出る。Plugin Manager の窓は出ない。VM の log では、`System\Tools\Plugin Manager\PluginManager.exe` を `GetFileAttributesW` で探し、無い(`missing-path`)として扱う。engine は止まらない。
- インストーラの一覧に、この実行ファイルが在る(`inputs/fl-installer-files.tsv` の `System\Tools\Plugin Manager\PluginManager.exe`。大きさ・ハッシュ・依存の DLL の一覧は、同じ行に在る)。依存の DLL は、winmm・shlwapi・comdlg32・comctl32・shell32・user32・version・oleaut32・advapi32・imagehlp・kernel32 など。
- FL は `-flpath` つきで起動する(推定。`FLEngine_x64.dll` の文字列から。確かめる)。
- `ShellExecuteW` は、`lift/runtime/fle_wall_ia2.c` で「関連付け無し」を返し(SE_ERR_NOASSOC 31。決定 `S-ia2-shellexecute-noassoc`)、起動しない。実行ファイルが在っても、この経路では別の窓は出ない。`ShellExecuteA`・`ShellExecuteExW` も同じ(`lift/runtime/fle_api2_shell.c`)。
- 窓の表の実測では、`TPlugListForm`(題名 `PlugList`)と `TPluginMonitorForm`(題名 `Plugin performance monitor`)が出ている(`pm-work/findings/crash-hunt-2026-11-25.json`)。Plugin Manager の窓の型は、この Issue で実行ファイルを動かした後に確かめる。
- 内蔵プラグインの `lift/plugins/<名前>/plugin.tsv` は DLL の形(本体・補助、hook・args・omit)。実行ファイルは DLL ではないので、この形に合うかは、やること 3 で決める。
- インストーラの資源の木は `lift/out-app/host/resources/`(build の結果。`file-open-demo` の E2E が、この木から DEMO を読む)。

## 読む物

- `inputs/fl-installer-files.tsv`(`System\Tools\Plugin Manager\PluginManager.exe` の行)
- `lift/crates/fe-pe/src/`(PE32+ の読み取り。道具は `fe-pe pe-info`・`pe-exports`・`pe-pdata-info`・`pe-init-table`・`rebase`)
- `lift/crates/fe-build/src/plugins.rs`(`lift/plugins/` の取り込み)・`lift/crates/fe-build/src/main.rs`(`engine`・`bundle` の段)
- `lift/plugins/*/plugin.tsv`・`inputs/plugin-hooks.tsv`・`inputs/plugin-classes.tsv`
- `lift/runtime/fle_wall_ia2.c`(`fle_imp_ShellExecuteW`)・`lift/runtime/fle_api2_shell.c`(`fle_imp_ShellExecuteA`・`fle_imp_ShellExecuteExW`)・`lift/runtime/fle_api2.h`
- `lift/harness/`(VFS の seed と、資源の仕組み)・`lift/harness/seeds/vfs-seed-installed.tsv`
- #1478 の `data.tsv` の仕組み(commit c3d1f693 の題名は「抽出の木の鍵に lift/plugins/*/data.tsv を足す」。その実体は、作業の最初に `git grep` で確かめる)
- `pm-work/findings/missing-paths.md`・`pm-work/findings/crash-hunt-2026-11-25.json`(窓の表)
- `tools/e2e/`(E2E の道具。`lib/`・`scenarios/smoke/`)・`tools/crash-hunt/hunt.mjs`
- #{{198}} の本文と、その作業の差分(作業ツリーに在る前回の差分)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. **PE を読む**(`fe-pe` の道具。表は `pm-work/findings/plugin-manager-exe.md` に書く)。exe は `lift/out-app/host/resources/` の下に在る(build の結果。無ければ `tools/fl-installer-extract.mjs` と `tools/fl-installer-files.mjs` の手順で取り出す。取り出し先は、読んで確かめる)。確かめる物: Machine が x86-64 か・ImageBase・サブシステム(GUI か)・エントリの RVA・再配置の表(`.reloc` の有無)・import(同じ行の DLL の一覧と一致するか)・Delphi の RTL の目印。x86-64 でなければ、この製品の対象外なので、理由を報告に書いて終える。
2. **実行ファイルを VFS に置く**: `System\Tools\Plugin Manager\PluginManager.exe`(FL の木の中の場所)を、lazy の資源として VFS に置く。Data の木が在れば、同じく置く。置き方は、`lift/plugins/*/data.tsv`(#1478 の仕組み)か、`resources` の仕組みのどちらかを読んで決め、理由を書く。大きさとハッシュは、`inputs/fl-installer-files.tsv` の行の値を使う。
3. **lift して 1 つの wasm にする**: 内蔵プラグインの `plugin.tsv` の形(本体・hook・args・omit)が、実行ファイルに使えるかを確かめる。使えなければ、最小の拡張を設計する。拡張は、この 1 本の実行ファイルを既存の仕組みに足す形にし、共有のファイルの変更は最小にする。
4. **起動の要求を、同じ engine の中に結ぶ**: FL が `ShellExecuteW`・`ShellExecuteExW`・`CreateProcessW` のいずれかで、この実行ファイルを指したとき、別のプロセスではなく、同じ engine の中で、新しいゲストのスレッドとして、実行ファイルのエントリを走らせる。次を決め、理由を報告に書く: `GetModuleHandle(NULL)` が実行ファイルの基底を返すこと・コマンドラインとカレントディレクトリ・実行ファイルの終了(`ExitProcess`)をゲストのスレッドの終了として扱い、engine は止めないこと・親が待つ `WaitForSingleObject`・`GetExitCodeProcess` の扱い・`ShellExecuteExW` の `hProcess`。FL がどの呼び出しを使うかは、実測で決める。決定 `S-ia2-shellexecute-noassoc` の扱いは、この実行ファイルの起動だけを変え、他の URL・ファイル種類は関連付け無しのままにする。
5. **窓を確かめる**: Plugin Manager の窓が窓の表に出る。クラスと題名を報告に書く。engine は `engine-idle` のまま 10 秒。
6. **E2E**: `plugin-manager-opens`(#{{198}} が作る。`tools/e2e/scenarios/smoke/plugin-manager-opens.mjs`)の `expectFail` の印を外す(`expectFail: null` にする)。この Issue の受け入れで、ok になることを確かめる。
7. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.some((s)=>s.name==="plugin-manager-opens"&&s.ok===true&&s.expectFail===null&&s.engineStops===0))?0:1)' "$RUNNER_TEMP/e2e.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2` が終了コード 0。
- 両方の表示の行で、`plugin-manager-opens` が `ok` が真・`expectFail` が null・`engineStops` が 0。
- 報告に、Plugin Manager の窓のクラスと題名、窓の表の行を書く。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: (a) PE の表(Machine・ImageBase・サブシステム・エントリの RVA・再配置の有無・import の一覧・Delphi の目印)、(b) VFS の置き場所と、資源の方式の理由、(c) lift の結果(wasm の大きさ・仕組みの拡張の有無と理由)、(d) 起動の要求の経路(どの呼び出しで、どのゲストのスレッドが走ったか。終了と待ちの扱い)、(e) 窓の表の窓(クラスと題名)、(f) 流れ A〜D の 4 行、(g) `plugin-manager-opens` の行。
- 段ごとに、やること 1〜6 のどこまで進んだか(完了・途中・未着手)を書く。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- 実行ファイルの名前と置き場所は、`inputs/` と `lift/` の一覧から作る。製品のコードに書き並べない。
- 決定 `S-ia2-shellexecute-noassoc` の扱いは、この実行ファイルの起動だけを変える。他の URL・ファイル種類は、今まで通り関連付け無しのままにする。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 実行時の値(窓の位置・DPR・時刻)を、期待値の定数に書かない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- 1 回で終わらない前提で、やること 1〜6 の段ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #{{198}}(Plugin Database の作業)。その Issue が閉じるまで起動しない。
- #{{198}} が作る `plugin-manager-opens`(`expectFail` の印付き)の印を、この Issue の作業で外す。
- 第 2 波の内蔵プラグインの束は、`lift/plugins/` の共有の仕組みを触る。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
