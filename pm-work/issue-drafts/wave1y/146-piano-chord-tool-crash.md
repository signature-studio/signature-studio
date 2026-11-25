---
draft: "146"
title: "Piano roll のコード進行ツールの Start creating で止まる: FL 内蔵の Python(python312.dll)を real module として読み込めるようにする(段階 1: LoadLibrary が成功し、止まらない)"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["resources", "verify"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の実機で、Piano roll の Tools メニューの「コード進行ツール」を開き、"Start creating" を押した瞬間に、FL が落ちる(engine が止まる)。この Issue は段階 1 で、次の 2 つを目標にする。
1. Start creating を押しても engine が止まらず、FL 内蔵の Python の実行環境(`Python\Python312.dll`)の読み込み(`LoadLibraryW`)が成功すること。
2. 起動の時間(`elapsedMs`)と、起動前に取る量(`preBootBytes`)を増やさないこと。

段階 2(Start creating で Piano roll にノートが作られること)は、別の Issue(下書き 151。題名「コード進行ツールの Start creating で、Piano roll にノートが作られる(FL の Python のスクリプトの実行)」)で扱う。段階 1 だけでは、ノートは作られない。

## 背景

- 依頼者の報告(原文): 「コード進行ツールは "Start creating" を押下した瞬間クラッシュします」。
- 事実 1(配信の一覧 `inputs/fl-installer-files.tsv`): コード進行ツールの実体は、FL 内蔵の Python のスクリプトに見える。`System\Tools\Chord progression\` に 59 件(`cpt.pyscript`・`stack.pyscript`・Python の bytecode の `.pyc` など)。大きさは、この一覧に無い(dll・exe 以外は空)。
- 事実 2(同じ一覧): Python の実行環境は `Shared\Python\`(37 件。`python312.dll` 7,003,928 B、`python312.zip`、`python312._pth`、`_*.pyd` など)。インストーラに在る。
- 事実 3(配信の範囲): 配信の接頭辞 `ci/resources-prefixes.txt` には、`System\Tools\Chord progression\` と `Shared\Python\` を足した済み(main の fc686522)。実物は作業ツリーの `lift/out-app/host-extra/resources/` に置かれる。一方、起動の seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)には、`Shared\Python\` と `System\Tools\` の行が 0 件。seed に無い物は VFS に載らない。`ci/extract-dlls.tsv` にも `python312.dll` の行は無い。
- 事実 4(読み込みの経路 `lift/runtime/fle_load.c`・`lift/runtime/fle_wall_ia2.c` の `fle_ia2_path_absent`): 「Python\Python312.dll」の `LoadLibraryW` は、常に「見つからない(126)」と答える。この答えは、記録の上で選んだ choice(`fle_wall_ia2.c` の注と、`lift/harness/seeds/ia2-imports.txt` の W21 の行)。その注は「インストーラに Python312.dll が無い」と書くが、事実 2 の一覧には `Shared\Python\python312.dll` が在る。食い違い。この choice は、チャンネルラックの設定アイコン(ia2-W21)の要求にも効いている。
- 事実 5(別の試行の記録 `pm-work/findings/blocks-samples-1366.md` の 4.3): Welcome の Default の行を開いた直後に、`[err] missing-path: LoadLibraryW 126 "Python\Python312.dll"` と `[err] fle: RaiseException: exception code 0xeedfade ...` が出て、engine が exit code 134 で止まった(測定の記録)。

## 前回の試行で分かったこと(4 回目。run 45566401846。failed)

数値は runner の値(nproc 4、loadavg 1.03 1.13 1.16)。

- 判定は差し戻し。受け入れ未達(道具の `stopped` が true、`engine` が exited。DPR 1 と乱数の DPR)。`pythonLoaded` は true。作業の差分は work.diff に保存された(作業者の commit は作業者の環境の物で、main に無い)。次の試行は、`.prior-work.md` の取り込みで差分を載せてから続ける。記録は `pm-work/findings/chord-stop-1386.md` の「7」(差分に在る)。
- 道具の JSON(最後の版。DPR 1): `{"tool":"chord-progression","dpr":1,"error":null,"engine":"exited","stopped":true,"presentedDelta":0,"pythonLoaded":true,"elapsedMs":11329,"preBootBytes":313241213}`。直前の版(DPR 1 と 2.25): `elapsedMs` 11,593 と 11,449、`preBootBytes` 313,240,871。
- 流れ A〜D: 4 本とも条件を満たす(`elapsedMs` 11947・11744・12107・11750、中央値 約 11,849。`preBootBytes` 313,241,213)。
- 単体: `npm test --prefix app` は 71 files・1,410 件が通る(10 件はスキップ)。`cargo test -p fe-build` は 23 件が通る(fixture `lift/crates/fe-build/tests/plan/engine.txt` を現在の plan の出力に合わせた。2 行の差)。`seh_rt`(fe-lift の試験)は 17 件が通る。`guard/scan.sh` は 0 件、`guard/flags.sh HEAD` は指摘なし。
- 着地の RSP は正しい(候補 a は否定)。`fle: seh landing: target frame 0x63ae33, R13 0x4, the landing 0x63ae4e, the landing RSP 0x3e186120, RBP 0x3e186120 ... (establisher 0x3e186120)`。
- 停止の記録(前回と同じ値): `fle: stop record: faulting address 0x5ee8604d8d490043; ... R1=0x5df6e7 ... R13=0x5df6e7 R14=0x5ee8604d8d48ffe3 R15=0`。
- 停止の場所: 2 回目の着地 `0x63ae4e`(関数 `0x63ade0` の中)の後、`call [0x16b85b0]`(IAT)を経て Python の枠に入る。停止時のスタックで、Python の枠より上で最初に現れる FLEngine の戻り番地は `0xc3948f`(IAT 呼び出しの直後)。
- Python 側の停止の命令は、`0x9b79c`(関数 `0x9b6e4`。`mov r13, r8` の後に検査し、`mov r14,[r13+0x10]` で読む)か `0x27ee7b`(関数 `0x27ee20`。`mov r13, rcx`)。どちらかは未確定。停止時の戻り番地の並びは、どちらの枠とも合わない。`0x9b6e4` の枠の戻り番地 `0x3e887` は `call 0x18003e960` の後で、`0x9b6e4` を直接呼んでいない。
- 引数: `0x5df6e7`(FLEngine の戻り番地)が、Python の関数の引数に入っている。Python の検査 `0x1800f60f0` は、この値を「読める」と返す。なぜ入ったかは未確定。
- 例外: `0xEEDFADE` が 2 回送出される。2 回目の送出の戻り番地 `0x5df6e7` は `0x3e13a088` に在り、着地 `0x63ade0` の RSP `0x3e13a120`(RBP)とアンワインド規則は整合する。例外のクラス名は、この試行では読んでいない。
- 試して外した: 直近の間接呼び出しの記録(`fle_dispatch` で、戻り番地・`RCX`・`R13` を残す)。Python のモジュールでは `R` が別の値になり、記録が全て 0 だった。入れ直さない。
- lazy: manifest の `lazy` は 131 件(外部 3 件を除く)で、上限 128 を超え、bundle が `/lazy: must have <= 128 items` で落ちた。段階 1 に要らない `sqlite3`・`_sqlite3`・`_lzma` の 3 件を外して 128 件にした。上限は main で 256 になった(#1414)。戻し方は「やること」③。
- 生成物: `lift/out-app/plugins/pack.args` と `lift/out-app/plugins/vfs-seed-idle1.tsv` は、engine の段階で `lift/harness/` の表から作られる。作業ツリーの表だけを直すと、生成物は古いままになる(前回は `_lzma.pyd` の行が残り、起動の前に止まった)。表を直したら、engine と bundle を作り直す。
- `lift/out-app/gen` が残っていると、`fe-build engine` が生成を二重に走らせて落ちる(この試行で 1 回起きた)。受け入れの `rm -rf lift/out-app/b2d-src lift/out-app/gen` は、engine の前に置く。
- 抽出の木に `python.python312.dll` が無いと、`fe-build engine` が `lift/out-app/host/lift/out-plugins/python.python312.dll` の欠落で止まる。受け入れの写しの段で補う。

## 1 つ前の試行で分かったこと(run 45540991873。3 回目)

- 判定は差し戻し。受け入れコマンドが失敗した(判定の段で、道具の `stopped` が true、`engine` が exited。DPR 1 と 2.5 の両方)。作業は work.diff に保存された。作業者の commit(`c8000daf`・`a6f146d4` ほか)は main に無く、次の試行は、差分を main の上に取り込んで続ける。runner の値。nproc 4、loadavg 1.08〜1.96。
- 測定(`measure.mjs --tool chord-progression`。DPR 1 と 2.25): `pythonLoaded` true、`stopped` true、`elapsedMs` 10,610 と 10,643、`preBootBytes` 312,235,140(2 回目の 312,215,074 から +20,066 B。内訳は未測定。上限 314,368,711 まで約 2.1 MB)。
- 流れ A〜D(`welcome-flow.mjs`): 4 本とも条件を満たす。`elapsedMs` は 10,985・10,801・10,725・10,731(中央値 10,766)。
- 単体: `npm test --prefix app` は 1,406 件が通る(10 件はスキップ)。`cargo test -p fe-build` は 22 件が通る。`guard/scan.sh`・`guard/flags.sh` は 0 件。
- `pythonLoaded` の判定を直した。engine のログの行は `realmod: loaded python.python312.dll`(キーの接頭辞付き)なので、道具は接頭辞に依らず判定する。`LoadLibraryW` の Python の欠落は 0 件。残る 4 件の欠落(WavpackDLL など)は Python と無関係。
- lazy の上限: bundle は `/lazy: must have <= 128 items` で落ち、134 件だった。段階 1 に要らない 6 件(`_msi.pyd`・`_wmi.pyd`・`_overlapped.pyd`・`winsound.pyd`・`_multiprocessing.pyd`・`pythonw.exe`)を、`pack.args`・seed 表・生成物の 3 箇所から外し、128 件にした。上限を上げるのは #{{171}} の作業。
- 抽出の木が python312 の plugin の無い main の版から作られていたため、`python.python312.dll` が出力に無かった。前の試行と同じく、`host-extra/resources/Shared/Python/python312.dll`(SHA-256 `0cc4e557…`)を `lift/out-app/host/` の 2 箇所へ写して補った。`ci/` は作業者が変えない(PM が直す)。
- 停止の場所(測定済み):
  - 停止の番地 `0x5ee8604d8d490043`(8 バイトの load)は `R14 + 0x60`。`R14` = `0x5ee8604d8d48ffe3` は `[R13 + 0x10]` から読まれた。`R13` = `0x5df6e7` は、`R1`・`R6` とスタック上の値と同じ。
  - `0x5df6f7` は FLEngine の `.text`(RVA `0x1df6f7`)の中。そのバイト列 `e3 ff 48 8d 4d 60 e8 5e` は命令。つまり、コードのバイト列を番地として読んでいる。
  - `0x5df6e7` は、`f_5df610`(例外ハンドラ付き)の `call 0x411e80`(`0x5df6e2`)の戻り番地。
  - `[R13 + 0x10]` で `R14` を読む命令は `python312.dll` に 2 箇所(RVA `0x9b79c`・`0x27ee7b`)。どちらが停止の RIP かは未確定。
- 例外の経路(`fle: seh`。今回の記録。常に出る): Delphi の例外 `0xEEDFADE` が 2 回送出される(`f_411d20` → `RaiseException`)。1 回目は `f_5df610` 内の着地 `0x5df673`。2 回目は `0x63ae33` で受けられ、着地 `0x63ae4e`。送出と着地の計 4 つの記録の全てで、`R13 = 0x4`。
- アンワインダは、`UWOP_PUSH_NONVOL`・`UWOP_SAVE_NONVOL`・`UWOP_SAVE_NONVOL_FAR` で保存レジスタを復元する(`lift/runtime/fle_seh.c` の `unwind_one`)。2 回目のアンワインドで `R13` の復元の記録は 0 件。つまり、途中のフレームに `R13` の保存が無く、アンワインドは `R13` を変えていない。前の読み(「復元しない」)は誤り。
- `R13 = 0x5df6e7` が入ったのは、2 回目の着地の後(`f_63ae4e` 以降)。`f_63ae4e`・`f_41a770`・`f_411fc0`・`f_469080` に `R[13]` への代入は無い。
- 候補(未確定): (a) 着地の時の RSP が 1 スロット(または 1 フレーム分)ずれ、後の `pop` が戻り番地 `0x5df6e7` を読んだ(PM の読み)。(b) `python312.dll` の `0x27ee20`(RVA)は `mov r13, rcx` で始まり、停止時の `R1`(rcx)も `0x5df6e7`。(b) なら、戻り番地が第 1 引数として渡されたことになる。
- 見立て: 停止の原因は import の欠けではない。UCRT の (a)〜(h) は、この試行では実装していない。
- 原文(比較用。3 回目の記録の抜粋。直した後も、同じ形の行を出す):

```
{"tool":"chord-progression","dpr":1,"error":null,"engine":"exited","stopped":true,"presentedDelta":0,"pythonLoaded":true,"elapsedMs":10610,"preBootBytes":312235140}
fle: stop record: faulting address 0x5ee8604d8d490043; registers R0=0x3e1384d0 R1=0x5df6e7 R2=0xc815470 R3=0x3e1384d0 R4=0x3e1382b0 R5=0x3e138309 R6=0x5df6e7 R7=0x3e138540 R8=0 R9=0x3e1383b8 R10=0 R11=0 R12=0 R13=0x5df6e7 R14=0x5ee8604d8d48ffe3 R15=0
fle: seh raise: code 0xeedfade, R13 0x4, R4 0x3e139ec8, the return address 0x411e4d
fle: seh landing: target frame 0x63ae33, R13 0x4, the landing 0x63ae4e
```

## 2 回前の試行で分かったこと(run 45505988410)

- 判定は差し戻し(段階 1 の受け入れは未達。`stopped` が true、`engine` が exited)。流れ A〜D と単体は通る。作業は work.diff に保存された。作業者の commit は `f8414a6b` を含む数本(作業者の環境。push していない)。runner の値。nproc 4、loadavg 1.59〜1.83。
- 前回の差分(実装): UCRT の import を 66 件、`lift/runtime/fle_crt.c` に新設(stdio・FILE・書式付き出力・環境変数・時刻・変換・文字列・`_wstat64i32`・`localeconv`・PathCch・BCryptGenRandom・exit・abort・signal)。import の件数は、実装 262(既存 114 + ship7 の 148)・未対応 96・序数 3(合計 361)。`lift/harness/seeds/ship7-imports.txt` の末尾に 66 行を足した(既存の行の位置は変えない)。
- `lift/harness/hyb/c-imports-engine.txt` に 115 件を足した(行のある python の import 116 件から、`WaitForSingleObjectEx` を除く)。`ReleaseSemaphore`・`_errno`・`__acrt_iob_func`・`_isatty` などを含む。この表が無いと、python の関数の多くが C に下ろされず(`not modelled`)、呼び出しが素通りする。
- `Shared\Python\` の 35 件を lazy に(`lift/harness/seeds/vfs-seed-idle1.tsv` と `lift/harness/web/pack.args`)。`python312.dll` は plugin の行が配る。`Shared\Python\python.exe` は `guard/scan.sh` の python の規則に掛かるので、外した。
- 止まり方の変化: 前回の `__stdio_common_vswprintf` の未実装は直った。今回は `fle_addr_fault`(`lift/runtime/fle_host.c`)で止まる。値 `0x5ee8604d8d490043` を番地として読んでいる。DPR 1 と DPR 2.55 の 2 点とも同じ値。直前は `realmod: loaded python312.dll`(`LoadLibraryW` の成功。`pythonLoaded` true)。止まる直前は `Python\Python312.dll` の探索の後で、ファイルは開かれていない。RIP は未特定。
- 道具の値(最後の build。`measure.mjs --tool chord-progression`): DPR 1 は `elapsedMs` 11,198・`preBootBytes` 312,215,074・`stopped` true。DPR 2.55 は `elapsedMs` 11,399・`preBootBytes` 312,215,074・`stopped` true。
- 流れ A〜D(同じ build。`welcome-flow.mjs`): 4 本とも `engine-idle`・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true、B と D は `welcomeClosed` true。`elapsedMs` は 11,265・11,544・11,202・11,211(中央値 11,238。上限 12,500)。
- `preBootBytes`: 直す前 311,109,738 → 前回の差分 313,128,824 → 今回 312,215,074(直す前より +1,105,336 B。増えた内訳は未測定。上限 314,368,711 に対し、残り約 2.1 MB)。
- 値の解釈(未確定): `0x5ee8604d8d490043` は、`EncodePointer` の符号化の形ではない(`DecodePointer` で復号しても小さなポインタにならない)。BCryptGenRandom の出力の列にも現れない。未設定の TLS スロットの読み(`TlsGetValue`)も、原因ではなさそう。
- 値のバイト列は、命令のバイト列に似る。メモリの並び(リトルエンディアン)では `43 00 49 8d 4d 60 e8 5e` で、`49 8d 4d 60` は `lea rcx,[r13+0x60]` の符号化に読める。PM の見立て: コードのバイト列を番地として読んでいる(やること 8)。
- 単体: `npm test --prefix app` 1,401 件、`cargo test -p fe-build` 20 件が通る。`guard/flags.sh` が、`fle_imp_getenv`・`fle_imp__wgetenv` の名前を、`getenv(` の規則で新しい旗として検出したため、`fle_imp_crt_env_lookup`・`fle_imp_crt_wenv_lookup` に改名した。
- このジョブの抽出キャッシュは `python312` の plugin.tsv より前の鍵で作られ、`lift/out-app/host/lift/out-plugins/python312.dll` と `lift/out-app/host/lift/out-idle1/inputs/install/Shared/python312.dll` が無かった。作業者は `host-extra/resources/Shared/Python/python312.dll`(SHA-256 `0cc4e557…`)を写して補った(`lift/out-app/` は commit されない)。次の試行でも同じ前提が要る。
- 未測定: ia2-W21(チャンネルラックの設定アイコン)。`fle_ia2_path_absent` の choice は外したまま。

## その前の試行で分かったこと

2 回の試行の結果。作業は、どちらも work.diff に保存されている。commit は、この main に無い(1 回目の作業は main の 05577cf9 に道具と記録だけが入っている)。次の試行は、差分を main の上に取り込んで続ける。

- 1 回目(run 45367751383。failed)の再現: 道具 `tools/piano-roll-tools/measure.mjs` で Start creating を押すと、`missing-path: LoadLibraryW 126 "Python\Python312.dll"` の後に、`fle: guest address 0x520000077c (8-byte load) has bits above 32 set` で engine が exit code 134 で止まった。原因: seed 表に `Shared\Python\` の行が無く、VFS に `Python\Python312.dll` が無い。直す前の値: `elapsedMs` 6745、`preBootBytes` 311,109,738。
- 2 回目(run 45452762437。failed)の結果:
  - `realmod: loaded python312.dll`(`LoadLibraryW` の成功)を観測した。`pythonLoaded` は、DPR 1 と別の DPR(乱数)の 2 点で true。
  - その後、`fle: unsupported instruction at 0xc5a95a8`(RVA 0x295a8)で止まる。これは `api-ms-win-crt-stdio-l1-1-0.dll!__stdio_common_vswprintf` の呼び出し。runtime に UCRT の import が無いため。
  - 道具の JSON(DPR 1): `{"tool":"chord-progression","dpr":1,"error":null,"engine":"exited","stopped":true,"presentedDelta":0,"notes":null,"pythonLoaded":true,"elapsedMs":8933,"preBootBytes":313128824,...}`。別の DPR でも同じ止まり方(`elapsedMs` 9112、`preBootBytes` 313,128,824)。
  - 流れ A〜D(`tools/smoke/welcome-flow.mjs`): 4 本とも `engine-idle`、standIn が false、`elapsedMs` の中央値 9,287.5、`preBootBytes` の最大 313,128,824(上限 314,368,711)。
  - 単体試験: `npm test --prefix app` が 1374 件通った。`cargo test -p fe-build` は 18 件通った。
  - import: 361 件(序数 3 件を含む)。内訳は、既存の表(fixed・ship6)の実装 114、差分の `ship7-imports.txt` の新しい行 82、未対応 162、序数 3(WS2_32)。全件の表は `pm-work/findings/python312-imports.md`(差分に在る)。
  - 差分の中身: `lift/runtime/fle_crt.c`(CRT・VCRUNTIME・math の 80 関数。DllMain の初期化に要る範囲)、`lift/runtime/fle_sem.c`(semaphore の 3 関数。`CreateSemaphoreA` で止まったため)、`ship7-imports.txt`(ship6 の後ろに 82 行を追記。FLEngine と既存の fixed モジュールは、この行を使わない)、`fle_ia2_path_absent` の choice を外す(ia2-W21 は未測定)、`plain-units.txt` に python の unit(m11 の 6 件)、道具に `pythonLoaded`。
  - 注意 1: `EXTRA_IMPORTS` を変えたら、release の `fe-build` を作り直す。`cargo test` は試験用のバイナリしか作らないので、古い `fe-build` で `engine` を作ると、変更が入らない。
  - 注意 2: `WaitForSingleObjectEx` は `lift/harness/two/wrp1_vst3_imp.c` に在る。semaphore 側の同じ名前と衝突するので、`fle_imp_sem_WaitForSingleObjectEx` にした。
  - `preBootBytes` は 311,109,738 → 313,128,824(+2,019,086 B)。増えた内訳は測っていない。上限まで約 1.24 MB。
  - 起動が `state running` で止まったのは、1 回目の試行の 2 回(起動の前に viewport を変えた起動)。2 回目の試行では、流れ A〜D と道具の試行で止まらなかった(0 回)。#{{149}}(起動の間欠の停止。完了 22:28Z)で直ったとされる。この試行の後の出来事で、この Issue では未確認。
- 未確認: ia2-W21(チャンネルラックの設定アイコン)の画面は測っていない。`LoadLibraryW` の引数の形(絶対の Shared の path と推定)は、成功の行(`realmod: loaded`)で確かめられた。
- 仮説の結果: 1 回目の仮説(126 の後に不正な値を読んで止まる)は、seed の行を足すと、止まる場所が `realmod` の読み込みの後の CRT の import へ移ったことで確かめられた。

## 読む物

- `pm-work/findings/chord-tool-crash.md`(前回の記録。停止の全文と道具の値)
- `pm-work/findings/python312-imports.md`(import の全件の表。差分に在る)
- `tools/piano-roll-tools/measure.mjs`(道具。`pythonLoaded` を足した)
- `inputs/fl-installer-files.tsv`(`System\Tools\Chord progression\`・`Shared\Python\`)
- `ci/resources-prefixes.txt`・`ci/extract-dlls.tsv`・`lift/harness/seeds/vfs-seed-idle1.tsv`
- `lift/runtime/fle_load.c`(`LoadLibraryW`。334 行付近で `fle_realmod_find_path` を呼ぶ)・`lift/runtime/fle_realmod.c`(real module の探索)・`lift/runtime/fle_wall_ia2.c`(`fle_ia2_path_absent` と、その注)・`lift/harness/seeds/ia2-imports.txt`(W21 の行)
- `lift/runtime/fle_crt.c`・`lift/runtime/fle_sem.c`(前回の差分)・`lift/harness/seeds/ship6-imports.txt`・`lift/harness/seeds/ship7-imports.txt`
- `lift/plugins/` の仕組み(#{{125}}。1 プラグイン 1 ディレクトリ。`app/scripts/extract-fl.mjs` の `--plugins lift/plugins`)・`lift/plugins/python312/plugin.tsv`
- `lift/runtime/fle_vfs.c`(`missing-path:` の行)・`lift/runtime/fle_proc.c`(`RaiseException`)
- `tools/smoke/welcome-flow.mjs`・`tools/boot-check/boot-bundle.mjs`(`bootBundle` と `afterIdle`)
- 注: 前回の記録の一部は、CRT の実装の出所として `lift/harness/web/mt1/plugin-worker/vst_crt.c` を挙げていた。この main には、このファイルも、他のどの ref にも無い。在るものとして探さない。

## 先に行う(PM の作業は済み)

- 配信の接頭辞の追加は、main の fc686522 で済み(`System\Tools\Chord progression\` 59 件・`Shared\Python\` 37 件)。作業者は `ci/` を変えない。`ci/extract-dlls.tsv` に行が要ると分かったら、変えずに、報告に書いて終える(PM が足す)。
- 道具の `pythonLoaded` の項目と、`lift/plugins/python312/plugin.tsv`(real module の取り出し)は、前回の差分に在る。

## 実物の置き場

- ラベル `resources` の extract が、`ci/resources-prefixes.txt` の接頭辞で取り出し、作業ツリーの `lift/out-app/host-extra/resources/` に置く(`Shared\Python\` 37 件を含む)。
- 受け入れの先頭で `mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/` を行い、ビルドの入力の場所へ写す(`fe-build bundle` の `pack-seed.mjs` は `lift/out-app/host/resources/` を読む)。
- 受け入れの `python312.dll` の 2 箇所への写し(`fe-build engine` の前)は、抽出の木に python の plugin の DLL が無いときの手当て(前回の試行で必要だった)。この Issue の変更が main に入り(`lift/plugins/python312/plugin.tsv` が main に在る)、抽出が `lift/out-app/host/lift/out-plugins/python.python312.dll` を作るようになれば、要らない。
- 起動の seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)に行が無い物は、VFS に載らない。足りない実物の行は、作業者が seed 表に足してよい。host の欄を `@lazy-file:` で始めると、起動の後に取る(#{{105}} の方式)。段階 1 の lazy は 128 件(上限ちょうど。3 回目の試行で、段階 1 に要らない 6 件を外した)。`System\Tools\Chord progression\` の 59 件は段階 2 で扱い、上限を超えるなら報告に書いて終える。

## やること(段階 1)

3 回目の試行の後の順序(PM の決定)。①〜⑤を先に行い、残りは後ろの「持ち越し」に回す。

1. ①(実測で確かめる): 着地ごとに、着地の直前と直後の RSP、ハンドラを持つ関数の、その `call` の時点の RSP(入口の RSP から、プロローグの `sub rsp`・`push` の量で求める)を、`fle: seh landing` の行に足す(常に出る。環境変数・`#ifdef` は作らない)。`0x27ee20` の入口で `R1`(rcx)と戻り番地を記録し、(b) を確かめる。結果で、(a)(b) のどちらかに決める。
2. ②(直す): (a) なら、着地の RSP を UNWIND_INFO の規則で求め直す(x64 の規則: 着地の RSP は、ハンドラを持つ関数の `call` の時点の値)。(b) なら、`R13` を書いた呼び出し元を、①の記録で特定して直す。直す前に、①で見つかったずれを再現する試験を `cargo test -p fe-lift --test seh_rt` に先に足し、red を確かめる。保存レジスタの復元だけの試験は、今の `fle_seh.c`(保存レジスタを復元する)で通るので、red にならない。この直しは、全ての Delphi の例外の着地に効く(#{{166}} の RaiseException も同じ経路)。
3. ③(lazy): 1 つ前の試行(run 45540991873)で外した 6 件と、前回(run 45566401846)で外した `sqlite3`・`_sqlite3`・`_lzma` の 3 件の、計 9 件を、`lift/harness/web/pack.args` と `lift/harness/seeds/vfs-seed-idle1.tsv` の行に、main の版と同じ位置で戻す(PM の決定)。上限は、`app/src/bundle/manifest.ts` の `lazy` の `maxItems`(#1414 で 256)。`lift/harness/vfs_lazy.h` の `VL_REG_MAX`(1024)も足りる。
4. ④(測る): `preBootBytes` は 314,368,711 以下(今 312,235,140)、`elapsedMs` は 12,500 以下(今 10,610〜10,985)、流れ A〜D を保つ。直す前と後の道具の JSON の行を報告に書く。
5. ⑤(受け入れ): 受け入れは元のまま(下の「受け入れ」)。

持ち越し(①〜⑤の後。前の試行からの項目):
- UCRT の import(未対応は 2 回目の試行で 96 件。3 回目では数えていない)。依頼者の決定どおり、次の順で実装する。
  - (a) 書式と stdio: `__stdio_common_vswprintf`・`__stdio_common_vfprintf`・`__stdio_common_vsscanf`・`_wfopen` など。書式は UCRT と同じ解釈にする(`%s`・`%ls`・`%d`・`%lld`・`%f`・`%g`・`%p`・`%x`・幅・精度・`%%`)。
  - (b) ファイルと fd: `_get_osfhandle`・`_open_osfhandle`・`_wstat64` など。VFS の上の FILE・fd の model で作る。
  - (c) 時刻: `_time64`・`localtime_s` など。(d) ロケール: `setlocale` など。(e) ヒープ: `malloc`・`free`・`realloc` など。
  - (f) 変換: `strtod`・`_wcstod` など。(g) 環境: `getenv`・`_wgetenv`・`_putenv` など。(h) 終了: `exit`・`abort`・`_exit` など。
  - 実装の場所は `lift/runtime/fle_crt.c`(前回の差分に在る)。依頼者の記録が挙げた `vst_crt.c` は、この main に無い。Vital の作業(#{{029}})が同じ領域を変える。衝突は、両方を残す形で解く。
  - 未対応の物は、呼ばれた時に止める(既存の方式)。stub を作らない。実装の範囲と件数を、`pm-work/findings/python312-imports.md` に反映する。
- `fle_ia2_path_absent`(`Python\Python312.dll` を「見つからない」と答える choice)は、前の差分で外してある。ia2-W21 は未測定。測ってから、戻すか外したままにするかを決める。オプトインは作らない。
- 停止の RIP: `python312.dll` の 2 箇所(RVA `0x9b79c`・`0x27ee7b`)のどちらかを、①の記録で確かめる。
- 停止の記録(`fle: stop record`)と SEH の記録(`fle: seh`)は消さない。常に出す(環境変数・`#ifdef` を足さない)。#{{154}} の作業者も同じ診断を入れる。両方の差分が衝突したら、両方を残す。
- 名前の解決(`fle_realmod_find_path`)と、起動の後の取得(`@lazy-file:`)は、前の試行で動いた。

## 次の一手(PM の決定。4 回目の後)

着地の RSP と、Python の入り口の FLEngine の番地は分かった。停止の原因は、まだ分かっていない。次の試行は、次の記録を先に出し(常に出す。環境変数・`#ifdef` を足さない)、その結果で Python の入りを直す。

- ⓐ 2 回送出される `0xEEDFADE` の例外オブジェクトの、クラス名(VMT の ClassName)を、`fle: seh raise` の行にも出す。クラスが `EOSError`(DLL の読み込みの失敗)・`EInOutError`(ファイル)・`EPythonError` などなら、その直前の失敗(欠けた DLL・ファイル)が、本当の原因の可能性が高い。その失敗を、停止の記録の前に出す。
- ⓑ Python に入る経路: 停止時のスタックの FLEngine の戻り番地 `0xc3948f`(IAT 呼び出しの直後)から、呼んでいる Python の export を、IAT の名前(`0x16b85b0` の項目の名前)で特定する(`Py_Initialize`・`Py_InitializeEx`・`PyRun_SimpleString` など)。
- ⓒ Python の初期化が読む物(`PYTHONHOME`・`PYTHONPATH`・`python312._pth`・`Lib\`・`encodings`)を、VFS とレジストリの読みで記録する。`Py_Initialize` が引数の少ない型で呼ばれている(呼び出し規約の不一致)と分かったら、停止の記録に、その IAT の名前と R1・R2・R8・R9 を出す。
- ⓓ UCRT の import(`getenv`・`_wgetenv`・`fopen`・`_wfopen`・`GetModuleFileNameW`・`GetFullPathNameW`・`LoadLibraryExW`)の呼び出しを、`Py_Initialize` の前後で、名前と結果を記録する。
- ④ 受け入れは、写しの段を除いて元のまま。この Issue は 1 回の試行で終わらない可能性が高い。次の試行は、ⓐ〜ⓓ の記録を先に出し、その結果を報告に書いて終える(commit は毎回する)。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && mkdir -p lift/out-app/host/lift/out-plugins lift/out-app/host/lift/out-idle1/inputs/install/Shared && cp lift/out-app/host-extra/resources/Shared/Python/python312.dll lift/out-app/host/lift/out-plugins/python.python312.dll && cp lift/out-app/host-extra/resources/Shared/Python/python312.dll lift/out-app/host/lift/out-idle1/inputs/install/Shared/python.python312.dll && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && D=$(node -e 'process.stdout.write((require("crypto").randomInt(101,301)/100).toFixed(2))') && node tools/piano-roll-tools/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --tool chord-progression --dprs 1,$D > "$RUNNER_TEMP/chord.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.stopped===false&&r.engine==="engine-idle"&&r.presentedDelta>=1&&r.pythonLoaded===true&&!r.missingPaths.some((l)=>l.startsWith("missing-path: LoadLibraryW")&&l.includes("Python"))&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===2&&L[0].dpr===1&&L[1].dpr>=1.01&&L[1].dpr<=3&&L.every(ok)?0:1)' "$RUNNER_TEMP/chord.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const ms=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b);const med=(ms[1]+ms[2])/2;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&med<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
```

判定の基準(段階 1):
- 道具の 2 行(DPR 1 と乱数の DPR): `error` が null、`stopped` が false、`engine` が engine-idle、`presentedDelta` が 1 以上、`pythonLoaded` が true、`LoadLibraryW` の行(Python を含む)が `missingPaths` に 0 件、`elapsedMs` が 12,500 以下、`preBootBytes` が 314,368,711 以下。`GetFileAttributesW` の探索の行は判定に入れない(成功の後にも出うる)。
- `tools/smoke/welcome-flow.mjs` の流れ A〜D(4 行): 各行が、`error` が null、`state` が engine-idle、`standIn` が false、`trapped` が false、`hostErrors` が 0、`opened` が true。流れ B と D は `welcomeClosed` が true。4 流れの `elapsedMs` の中央値が 12,500 以下、`preBootBytes` の最大が 314,368,711 以下。
- 単体テスト(`npm test --prefix app`)が通る。
- 段階 2 の「ノートが 1 以上」は、この Issue の受け入れに入れない。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「直った」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 直す前と後の道具の JSON の行、流れ A〜D の 4 行、`elapsedMs` と `preBootBytes` の値(増えた内訳を含む)、`pythonLoaded` の根拠の行、import の一覧の件数(実装・stub・未対応)、止まった場所(あれば)の RVA と import 名、ia2-W21 の判断。

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。オプトインを作らない。
- 製品のコード(`app/`・`lift/`・`ci/`)に Python を入れない。FL に内蔵の Python の DLL を、ゲストとして動かすのは FL の機能なので、よい。`ci/` は作業者が変えない。
- ベンダー名と Cloudflare の識別子を書かない。ぼやけた・拡大した画面を出さない。拡大率や DPI の固定の値を、定数・表として埋め込まない(乱数の DPR で測る)。
- OCR を使わない。FL の画面の文字は、ゲストのメモリの走査か、エンジンの記録で読む。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- 製品の作業は Actions で行う。この機械で製品のビルドや測定をしない。
- 禁止されているのは、AI のベンダー名と Cloudflare の識別子だけ。FL の製品名・インストール先の名前・変数名は、書いてよい。
- 名前の扱い(依頼者の訂正): ファイルに書いてはならないのは、AI のベンダー名と、Cloudflare の識別子だけ(`guard/scan.sh` が検査する)。FL の製品名(FL Studio など)・`Image-Line` のインストール先の名前・`%FLStudioFactoryData%` などの変数名は、書いてよい。

## 作業の進め方

- engine の作り直しは約 35 分。ビルドはフォアグラウンドで、シェルツールの `timeout` 引数に 5400000 を渡して待つ(背景で起動して待たない)。
- 作業の開始時刻を `date -u` で記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit して報告を書いて終える。
- 判定は commit の後に同じ job で、もう一度全部走る(約 60 分)。job の上限は 5 時間 50 分。3 時間 30 分を過ぎたら、新しいビルドを始めない。
- この Issue は 1 回の試行で終わらない可能性が高い。次の試行は、その commit から続く(`.prior-work.md` を読む)。
- blend2d の取得(GitHub)が 403 で落ちたら一時的。受け入れのコマンドは `lift/out-app/b2d-src` を消してから作り直すので、そのまま再試行する。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 他のタスクとの関係

- blocked_by: #{{125}}(閉じた。プラグインを 1 プラグイン 1 ディレクトリ `lift/plugins/<名前>/` の追加だけで足す仕組みは main に在る)。
- #{{147}}(Riff machine)は、道具 `tools/piano-roll-tools/measure.mjs` だけを使う。この Issue を待たない。
- 段階 2(下書き 151)は、この Issue が閉じるまで起動しない。
- `ci/` の変更は PM が行う(この Issue の作業者は変えない)。
- #{{029}}(Vital)が、CRT の実装の同じ領域(`lift/runtime/`)を変える。衝突は両方を残して解く。
