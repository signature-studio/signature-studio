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

## 前回の試行で分かったこと(run 45505988410)

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
- 起動の seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)に行が無い物は、VFS に載らない。足りない実物の行は、作業者が seed 表に足してよい。host の欄を `@lazy-file:` で始めると、起動の後に取る(#{{105}} の方式)。段階 1 の lazy は、今の 75 件に `Shared\Python\` の 37 件を足して 112 件で、上限 128 以内。`System\Tools\Chord progression\` の 59 件は段階 2 で扱い、上限を超えるなら報告に書いて終える。

## やること(段階 1)

0. **済み(前回の差分)**: 道具 `tools/piano-roll-tools/measure.mjs` の `pythonLoaded`。engine のログの `realmod: loaded python312.dll` の行から取る。動作は変えない。`notes` は段階 2 で数える(段階 1 では null)。
1. **済み(前回の差分)**: real module。`Shared\Python\python312.dll` を、#{{125}} の仕組み(`lift/plugins/python312/plugin.tsv`)で取り出し、lift する。前回の試行で、DllMain に入り、`LoadLibraryW` が成功することを確かめた。
2. **UCRT の import を実装する(依頼者の決定)**: 未対応の 162 件から、Python の起動に要る物を、次の順で実装する。
   - (a) 書式と stdio: `__stdio_common_vswprintf`・`__stdio_common_vfprintf`・`__stdio_common_vsscanf`・`_wfopen` など。書式は UCRT と同じ解釈にする(`%s`・`%ls`・`%d`・`%lld`・`%f`・`%g`・`%p`・`%x`・幅・精度・`%%`)。
   - (b) ファイルと fd: `_get_osfhandle`・`_open_osfhandle`・`_wstat64` など。VFS の上の FILE・fd の model で作る。
   - (c) 時刻: `_time64`・`localtime_s` など。
   - (d) ロケール: `setlocale` など。
   - (e) ヒープ: `malloc`・`free`・`realloc` など。
   - (f) 変換: `strtod`・`_wcstod` など。
   - (g) 環境: `getenv`・`_wgetenv`・`_putenv` など。
   - (h) 終了: `exit`・`abort`・`_exit` など。
   - 実装の場所は `lift/runtime/`(`fle_crt.c` は前回の差分に在る。これを足す)。依頼者の記録が挙げた `vst_crt.c` は、この main に無い(前回の試行で確かめた)。外部プラグインの作業の CRT が main に入ったら、そちらと共有する。入るまでは `fle_crt.c` に足す。
   - Vital の作業(#{{029}})が同じ領域を変える。衝突は、両方を残す形で解く。
   - 未対応の物は、呼ばれた時に止める(既存の方式)。stub を作らない。
   - 実装の範囲と件数(実装・未対応)を、`pm-work/findings/python312-imports.md` に反映する。
3. **名前の解決**: `LoadLibraryW "Python\Python312.dll"` が、`fle_realmod_find_path`(`lift/runtime/fle_realmod.c`)で real module の `python312.dll` を見つける(前回の試行で動いた。`plugin.tsv` の `guest` の値で通る)。
4. **起動の後の取得**: Python の DLL と標準ライブラリは、起動の後に取る(`@lazy-file:`)。前回の試行で `preBootBytes` が +2,019,086 B 増えた。増えた内訳を測り、起動の前に取っている物を、起動の後の取得に移す(lazy の上限 128 以内)。`preBootBytes` を増やさないこと。
5. **`fle_ia2_path_absent`(`Python\Python312.dll` を「見つからない」と答える choice)**: 前回の差分では外してある。ia2-W21(チャンネルラックの設定アイコン)の動作の変化は、未測定。測ってから、戻すか外したままにするかを決める。オプトインは作らない。報告に書く。
6. **測る**: 道具で、DPR 1 と乱数の DPR(1.01〜3.00)の 2 点。直す前と後の JSON の行を報告に書く。
7. **停止の記録の診断(PM の決定。#{{154}} と同じ)**: `fle_addr_fault`(`lift/runtime/fle_host.c`。64 ビットの値を番地として読んだ停止)の停止の記録として、**常に**、呼び出し元の戻り番地・`R[0..15]`・最も近い名前つきの関数を出す。環境変数・`#ifdef` を足さない。#{{154}} の作業者も同じ診断を入れる。両方の差分が衝突したら、両方を残す。
8. **読みの出所(PM の決定)**: 値 `0x5ee8604d8d490043` を読んでいる命令(RIP)を特定する。診断で、`R[0..15]` の中から、番地が `Python312.dll` の像(イメージ)の中に入る値を探す。見つかれば、その番地を `Python312.dll` の逆アセンブルで確かめる。可能性: コードのバイト列を番地として読んでいる(関数の入口でない所をデータとして読んでいる)、または、`Python312.dll` の IAT・thunk の、間違った番地を読んでいる。特定したら、結果を報告に書く。直し方は段階 1 の範囲に収まるときだけ直し、収まらなければ報告に書いて終える。

## 受け入れ

```accept
mkdir -p lift/out-app/host/resources && cp -a lift/out-app/host-extra/resources/. lift/out-app/host/resources/ && npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && D=$(node -e 'process.stdout.write((require("crypto").randomInt(101,301)/100).toFixed(2))') && node tools/piano-roll-tools/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --tool chord-progression --dprs 1,$D > "$RUNNER_TEMP/chord.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const ok=(r)=>r.error===null&&r.stopped===false&&r.engine==="engine-idle"&&r.presentedDelta>=1&&r.pythonLoaded===true&&!r.missingPaths.some((l)=>l.startsWith("missing-path: LoadLibraryW")&&l.includes("Python"))&&r.elapsedMs<=12500&&r.preBootBytes<=314368711;process.exit(L.length===2&&L[0].dpr===1&&L[1].dpr>=1.01&&L[1].dpr<=3&&L.every(ok)?0:1)' "$RUNNER_TEMP/chord.jsonl" && node tools/smoke/welcome-flow.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/flow.jsonl" && node -e 'const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const F=(f)=>L.find((r)=>r.flow===f);const ok=(r)=>!!r&&r.error===null&&r.state==="engine-idle"&&r.standIn===false&&r.trapped===false&&r.hostErrors===0&&r.opened===true;const ms=L.map((r)=>r.elapsedMs).sort((a,b)=>a-b);const med=(ms[1]+ms[2])/2;const pb=Math.max(...L.map((r)=>r.preBootBytes));process.exit(L.length===4&&["A","B","C","D"].every((f)=>ok(F(f)))&&F("B").welcomeClosed===true&&F("D").welcomeClosed===true&&med<=12500&&pb<=314368711?0:1)' "$RUNNER_TEMP/flow.jsonl" && npm test --prefix app
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
