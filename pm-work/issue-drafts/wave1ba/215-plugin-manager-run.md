---
draft: "215"
title: "FL の Plugin Manager を VM の中で動かす(段 B): 起動の要求を、同じ engine の中のゲストのスレッドに結び、窓を出す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: ["213"]
labels: ["delivery", "plugin-builtin", "verify"]
size: XL
priority: High
project: signature-studio/3
---

## 目的

段 A(#1499)が終わると、`PluginManager.exe` は VFS に lazy の資源として置かれ、lift した wasm として bundle に入っている。この段(段 B)の目標は 3 つ。
1. FL が `ShellExecuteW`・`ShellExecuteExW`・`CreateProcessW` のいずれかで、この実行ファイルを指したとき、別のプロセスではなく、同じ engine の中で、新しいゲストのスレッドとして、実行ファイルのエントリを走らせること。
2. Plugin Manager の窓が、窓の表に出ること。
3. その間も engine が止まらないこと(10 秒 `engine-idle` のまま)。

## 背景

- 観測(E2E の `plugin-manager-opens`。段 A の前の実測): Options の 6 番目を選ぶと、FL は `C:\Program Files\Image-Line\FL Studio 21\System\Tools\Plugin Manager\PluginManager.exe` を `GetFileAttributesW` で調べ、無いので止まる。窓は出ず、engine は `engine-idle` のまま。段 A で exe が VFS に在るようになれば、この先の起動の要求が engine のログに現れる。
- `ShellExecuteW` は `lift/runtime/fle_wall_ia2.c`(`fle_imp_ShellExecuteW`)で「関連付け無し」(31)を返す(決定 `S-ia2-shellexecute-noassoc`)。`ShellExecuteA`・`ShellExecuteExW` は `lift/runtime/fle_api2_shell.c` で同じ。
- `GetModuleHandle(NULL)` は engine の像を返す(`lift/runtime/fle_env.c`)。`ExitProcess` は `fle_stop` で engine を止める(`lift/runtime/fle_proc.c`)。
- 使える部品: 協調スレッド(`lift/runtime/fle_thread.c`・`fle_thread.h` の `fle_thread_spawn`。1 つのホストのスレッドの上に、ゲストごとのスタック)、realmod の読み込み(`fle_realmod_find_path`・`fle_realmod_load`・`fle_realmod_getproc`。`lift/runtime/fle_realmod.h`)、thread handle への `WaitForSingleObject`(`lift/runtime/fle_event.c`)。
- `fle_realmod` は `AddressOfEntryPoint` を DllMain として呼ぶ(`lift/runtime/fle_realmod.c`)。exe の入口は、引数の無いスレッドの本体として呼ぶ。この違いの扱いは、段 A の lift の方式に合わせる。
- 窓の表の実測(`pm-work/findings/crash-hunt-2026-11-25.json`): `TPlugListForm`(題名 `PlugList`)と `TPluginMonitorForm`(題名 `Plugin performance monitor`)。Plugin Manager の窓の型は、まだ確かめていない。
- 設計の論点と案は `pm-work/findings/plugin-manager-exe.md` の 4 節にある。決めること: `GetModuleHandle(NULL)` の扱い(exe のスレッドからは exe の基底)・`ExitProcess` の扱い(exe のスレッドからは、そのスレッドの終了。engine は止めない)・`hProcess` と `GetExitCodeProcess`・コマンドラインとカレントディレクトリ。FL がどの呼び出しを使うかは、実測で決める。

## 読む物

- `pm-work/findings/plugin-manager-exe.md`(4 節と 5 節)。段 A の報告と成果(lift の方式と、bundle の lazy の名前)
- `lift/runtime/fle_wall_ia2.c`(`fle_imp_ShellExecuteW`)・`lift/runtime/fle_api2_shell.c`(`fle_imp_ShellExecuteA`・`fle_imp_ShellExecuteExW`)・`lift/runtime/fle_api2.h`
- `lift/runtime/fle_thread.c`・`lift/runtime/fle_thread.h`・`lift/runtime/fle_realmod.c`・`lift/runtime/fle_realmod.h`・`lift/runtime/fle_event.c`・`lift/runtime/fle_env.c`(`GetModuleHandle`)・`lift/runtime/fle_proc.c`(`ExitProcess`)
- `tools/e2e/scenarios/smoke/plugin-manager-opens.mjs`・`tools/e2e/run.mjs`・`tools/e2e/judge.jq`
- `pm-work/findings/crash-hunt-2026-11-25.json`(窓の表)

## やること

0. 作業の最初に `date -u` を記録する。`.prior-work.md` が在れば、最初に読む。
1. 段 A の成果を確かめる: bundle の manifest(`lift/out-app/bundle/raw/fl-Oz/manifest.json`)の `lazy` に `PluginManager.exe` と、その wasm が在る。無ければ、この段は始められないので、報告に書いて終える。
2. 起動の要求を実測する: `plugin-manager-opens` の経路で、Options の 6 番目が、どの呼び出し(`ShellExecuteW`・`ShellExecuteExW`・`CreateProcessW`)を使うかを engine のログで確かめる。報告に、その行を貼る。
3. 同じ engine の中に結ぶ: 上の論点を決め、理由を報告に書く。`lpFile` が exe の完全なパスに一致する時だけ、新しいゲストのスレッドで exe のエントリを走らせる。戻り値は成功(33 以上)。それ以外の URL・ファイル種類は、今まで通り関連付け無し(31)のまま。決定 `S-ia2-shellexecute-noassoc` の変更の範囲は、この exe の起動だけ。
4. 窓: Plugin Manager の窓が窓の表に出る。クラスと題名を報告に書く。engine は 10 秒 `engine-idle` のまま。
5. E2E: `tools/e2e/scenarios/smoke/plugin-manager-opens.mjs` の `expectFail: '#1499'` を `expectFail: null` にする(印を外す)。
6. 受け入れを通す。通らなければ、作業ツリーを commit して、報告を書いて終える。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false&&r.elapsedMs<=12500?0:1)' "$RUNNER_TEMP/boot.json" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app && (cd tools/e2e && npm ci && npx playwright install chromium) && node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2 > "$RUNNER_TEMP/e2e.jsonl" && jq -e -s -f tools/e2e/judge.jq "$RUNNER_TEMP/e2e.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));process.exit(L.length===2&&L.every((r)=>r.error===null&&r.servedHeadersMatch===true&&r.scenarios.some((s)=>s.name==="plugin-manager-opens"&&s.ok===true&&s.expectFail===null&&s.engineStops===0))?0:1)' "$RUNNER_TEMP/e2e.jsonl" && bash guard/scan.sh
```

判定の基準:
- 起動確認(`boot.json`)が engine-idle・standIn が false・elapsedMs が 12,500 以下。
- 流れ A〜D の 4 行が、`error` が null・engine-idle・`standIn` が false・`trapped` が false・`hostErrors` が 0・`opened` が true。B と D は `welcomeClosed` が true。
- `npm test --prefix app` が通る。
- `node tools/e2e/run.mjs --scenarios smoke --displays 4k-dpr1,4k-dpr2` が終了コード 0。`tools/e2e/judge.jq` が通る(印を外したので、`plugin-manager-opens` も印の無い行として判定される)。
- 両方の表示の行で、`plugin-manager-opens` が `ok` が真・`expectFail` が null・`engineStops` が 0。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 「達成」「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: (a) 起動の要求の経路(FL が使った呼び出しの名前と、そのログの行。どのゲストのスレッドが走ったか。終了と待ちの扱い)、(b) 決めた論点ごとの理由(`GetModuleHandle(NULL)`・`ExitProcess`・`hProcess`・コマンドライン・カレントディレクトリ)、(c) 窓の表の窓(クラスと題名)、(d) 流れ A〜D の 4 行、(e) `plugin-manager-opens` の行(両表示)、(f) やること 1〜6 のどこまで進んだか(完了・途中・未着手)。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。テストのための分岐を製品に入れない。
- 決定 `S-ia2-shellexecute-noassoc` の変更は、この exe の起動だけ。他の URL・ファイル種類は、今まで通り関連付け無し(31)のままにする。
- 実行ファイルの名前と置き場所は、`inputs/` と `lift/` の一覧から作る。製品のコードに、一覧に無い名前を書き並べない。
- Python を使わない(調査と検証にだけ使う)。OCR を使わない。
- 実行時の値(窓の位置・DPR・時刻)を、期待値の定数に書かない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- sleep・timeout のコマンド・nohup・末尾の `&` は使わない。待つ時は条件で待つ。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。

## 作業の進め方

- 作業の最初に `date -u` を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- ビルドは約 35 分。フォアグラウンドで直接、シェルツールの `timeout` 引数は最大 5400000(90 分)にする。背景で起動して待たない。
- やること 1〜6 の段ごとに commit する。再試行は前回の差分の上で続け、作業ツリーを消さない。
- 受け入れが通らなくても、作業の最後に作業ツリーを commit する。未 commit の変更が残ると、判定は失敗する。

## 他のタスクとの関係

- blocked_by: #1499(段 A: exe を VFS に置き、PE の表を作り、lift して wasm にする)。その Issue が閉じるまで起動しない。
- #1484(Plugin Database の作業。`plugin-manager-opens` の E2E を作り、印を付けた)。
- 第 2 波の内蔵プラグインの束は、`lift/plugins/` の共有の仕組みを触る。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
