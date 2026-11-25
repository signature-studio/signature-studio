# vital-read-1338: Vital の VST3 本体を lift し、1 ブロックを処理する(#1338)

## 5 回目の試行(受け入れを通した。main の取り込みは次の run に残した)

判定: 受理(受け入れコマンドの終了コード 0。2026-11-24 22:22 → 00:38 UTC の run。1 回目の受け入れは 23:39 UTC に measure の段で落ちた。原因と直しは下)。測定の値は runner の値。この機械とは別枠。nproc = 4。loadavg は起動確認の時点で 1.52 1.30 1.84。

### 変えたこと

- 前回の取り込みの衝突(`lift/crates/fe-build/src/main.rs`・`tests/plan/bundle.txt`・`spec/workflow.test.sh`)を解消した(commit 6421e811)。HEAD(main)の版を土台に、`plugin_steps` を OTT と Vital の 2 件にした。bundle の計画は有る時 4 段、無い時は `bundle-no-external.txt` の 2 段。
- 1 回目の受け入れの落ち: `measure.mjs vital` が `LinkError: Import #9 "env" "fle_clip_host_read"`。main の #1389 で runtime に `fle_clip.c` が入り、plugin の wasm(OTT も)が `fle_clip_host_read`・`fle_clip_host_write` を import するようになったが、`app/src/plugins/ott-host.ts` の `env` に答えが無かった。直し(commit ec1cbc37): `fle_clip_host_read: () => 0`(page の読みは無し。runtime の `os_text_read` は 0 を「文字列なし」で扱う)、`fle_clip_host_write: () => undefined`(書きは捨てる)。

### 検査

- `cargo test --locked -p fe-build`: 19 件すべて通る。
- `spec/workflow.test.sh`: 440 件すべて ok。`spec/guard.test.sh`: 67 件すべて ok。
- `guard/scan.sh`・`guard/flags.sh origin/main HEAD`: 指摘 0。

### 受け入れの実測(通った run)

- `measure.mjs vital` の 1 行(そのまま): `{"plugin":"vital","instantiated":true,"blockFrames":4096,"nonSilent":true,"error":null}`
- 中の `RESULT` 行(そのまま): `RESULT out_hash=7d2d71f375347b15 nonfinite=0 samples=8192 nonzero=8192 maxabs=0.357191 rms=0.143032 blocks=1`。前回と同じ値(入力と処理が同じ)。音の正しさは未確認(参照の出力が無い)。
- 起動確認(`boot.json`): `state` engine-idle、`error` null、`standIn` false、`elapsedMs` 12,300。P1(10 秒以内)は 2.3 秒超えた。同じ run の 1 回目(落ちた run)は 11,672。原因は未確認。Vital の wasm は起動後の取得で、起動前に取る量は増やしていない(起動時間への影響は測っていない)。
- manifest の lazy: `vital.wasm`(`plugins/vital.wasm`)、size 115,542,148 B、br の encodedSize 7,331,794 B、sha256 `5813d903…`。`ott.wasm` は 27,806,898 B(br 2,570,590 B)。
- 到着時刻: 未測定(受け入れの範囲に無い)。

### lift・import(この run の生成物から数え直した)

- lift(`--pdata`): 関数 33,567(44 単位)、生成した命令 2,090,660 件、扱えた 1,885,965 件、扱えない 204,695 件(232 種)。前回までと同じ値。
- `IPPCODE`: 130,738 件のうち扱えた 107,380 件、扱えない 23,358 件(52 種。うち 50 種は AVX・FMA の V* 命令、残りは `Pushfq`・`Fxsave`)。`IPPDATA`: 命令 0 件(データだけ)。この数は前回の記録の値で、この run の log には節ごとの内訳が無い。
- import: 501 件(序数 16 件を含む)。TABLE と VST の関数を除いた 286 件のうち、runtime の既存の定義が 26 件、`vst_crt.c` の答えが 81 件(関数の定義 48 件と、マクロ(`MATH_*`・`WCT_*`)の 33 件)、名前のある停止が 179 件(一覧は下の「名前のある停止の一覧」)。
- stub の行(`vst3a_stubs.c`)は 260 件(286 − runtime の 26)。そのうち `vst_crt.c` が答える 81 件を除くと 179 件。

### main の取り込み(この run の work.diff を、後の commit で main の上に載せ直した)

- origin/main(505ba780)は、この作業の後に 65 件先に進んでいる。取り込む(`git pull --no-rebase`)と 7 件が衝突する: `app/src/plugins/ott-host.ts`・`lift/crates/fe-build/src/main.rs`・`tests/plan/bundle.txt`・`lift/harness/vst/mk_stubs.mjs`・`lift/harness/web/mt1/plugin-worker/main_vst_ext.c`・`lift/harness/web/pack.args`・`spec/workflow.test.sh`。
- 解消の方針(この run で試して、以下の確かめまで通した。fe-build 19 件・spec 445 件・guard 67 件): main 側を土台にする。(1) `plugin_steps` は `["ott", "serum2", "vital"]`、`bundle.txt` は 5 段(ott・serum2・vital の build.sh、seed、pack)。(2) `pack.args` は serum2 の行の後に vital の行。(3) `ott-host.ts` は main に無い `fle_clip_host_read: () => 0` を足す。(4) `mk_stubs.mjs` は main の序数の判定(`o.answer`)と、こちらの `cstr(api)`。(5) `main_vst_ext.c` は main の版(note-on は main に在る)。(6) spec は manifest の見本に ott・serum2・vital の 3 件、NX の段数は正規表現(`stage bundle: [0-9]* `)、文言は (ott・serum2・vital)。
- 取り込んだ後の受け入れは、この run では通せなかった。main の #1373(`5395f429`)が `vfs-seed-idle1.tsv` に demo の素材 21 行を足しており、この run の `lift/out-app/host`(古い seed で展開した木)に無い。`fe-build bundle` の `pack-seed.mjs` が `Used by demo projects/DEDZII - NIGHT TERROR/01 DEDZII - NIGHT TERROR - FLEX Croak.ogg` で落ちた。展開は extract の段(インストーラを R2 から取る)でしか作り直せないため、取り込みの後の受け入れは、この run では通せなかった。その後、6 回目の試行で、この解消(上の (1)〜(6))を main の上に載せ直して commit した。載せ直した後の受け入れ(extract の木を作り直す run)は、その commit では実施していない。push 後の deploy(fe-build engine)で確かめる。

## 4 回目の試行(取り込みの衝突を解消して、受け入れを通した)

判定: 受理。受け入れコマンドを、この runner で最初から通した。終了コード 0(2026-11-24 19:20 → 20:37 UTC)。測定の値は runner の値。この機械とは別枠。nproc = 4。loadavg は受け入れの後に 0.24 0.83 1.68。

### 変えたこと

- 取り込みの衝突(`lift/crates/fe-build/src/main.rs`)を解消した。main の版(外部のインストーラの有無の判定 `external_present`・`external_in`・`pack_args`)を残し、`ott_steps` を `plugin_steps`(OTT と Vital の 2 件を並べる)に変えた。bundle の計画は有る時 4 段(OTT・Vital・seed・pack)、無い時 2 段(`bundle-no-external.txt`)。
- `spec/workflow.test.sh` の (v) を、2 件の外部プラグインに合わせた: 段数の正規表現(`stage bundle: [0-9]*`)、`--lazy-file` の全件の削除(`g`)、Vital の作る段と lazy の元の検査を足した。deploy.yml(t) の固定の manifest に `plugins/vital.wasm` を足した(本番の検査は pack.args の全件を manifest で見る)。

### 検査

- `cargo test --locked -p fe-build`: 10 件すべて通る(`plans_match_recorded_texts` を含む)。
- `spec/workflow.test.sh`: 440 件すべて ok(終了コード 0)。
- `spec/guard.test.sh`: 67 件すべて ok。
- `guard/scan.sh`・`guard/flags.sh`(commit の後): 指摘 0。
- 取り出した `Vital.vst3`: 15,571,240 B、SHA-256 `a622a2c9…`(`inputs/external-plugins.tsv` の Vital の行と一致)。

### 受け入れの実測

- `measure.mjs vital` の 1 行(そのまま): `{"plugin":"vital","instantiated":true,"blockFrames":4096,"nonSilent":true,"error":null}`
- 中の `RESULT` 行(そのまま): `RESULT out_hash=7d2d71f375347b15 nonfinite=0 samples=8192 nonzero=8192 maxabs=0.357191 rms=0.143032 blocks=1`。3 回目と同じ値(入力と処理が同じ)。
- 起動確認(`boot.json`): `state` engine-idle、`error` null、`standIn` false、`elapsedMs` 12,608(runner の値)。P1(10 秒以内)は 2.6 秒超えた。過去の試行は 10,371〜12,376。Vital の wasm は起動後の取得で、起動前に取る量は増やしていない。超えた原因は未確認(Vital の追加との関係は測っていない)。
- manifest の lazy: `vital.wasm`(`plugins/vital.wasm`)、size 115,538,297 B、sha256 `f847e7a5…`、br の encodedSize 7,376,525 B。`ott.wasm` は 27,803,120 B。
- 到着時刻: 未測定(受け入れの範囲に無い)。

### import の数の照合(数え直し)

- `mk_stubs`: 501 件(序数 16 件を含む)。TABLE・harness の既存の答え・runtime の既存の定義に無いものは 286 件。内訳は runtime の既存の定義 26 件、`vst_crt.c` の答え 81 件、名前のある停止 179 件(26 + 81 + 179 = 286)。
- `vst_crt.c` の答え 81 件は、関数の定義 48 件と、マクロ(`MATH_F1`・`WCT_CLASS`・`WCT_MAP`)で答えた 33 件(`sinf`・`acos`・`iswalnum`・`towupper` など)の合計。関数の定義だけを grep すると 48 件になるので、数えるときはマクロも含める。

## 3 回目の試行(main との衝突を解消して、受け入れを通し直した)

判定: 受理。受け入れコマンドを、この runner で最初から通した。終了コード 0(2026-11-24 14:49 → 15:46 UTC)。測定の値は runner の値。この機械とは別枠。nproc = 4。loadavg は起動確認の時点で 1.96 1.47 2.02。

### 変えたこと

- 衝突の解消: `lift/harness/web/pack.args` は、OTT の行と Vital の行の両方を残した(OTT を先)。`lift/crates/fe-build/tests/plan/bundle.txt` は main の版(OTT の段)に Vital の段を足した(4 段。pack-assets の行に Vital の lazy 行を足した)。
- `lift/crates/fe-build/src/main.rs`: `ott_steps` を `plugin_steps`(OTT と Vital の 2 件を並べる)に変えた。bundle の計画(`bundle_plan_steps`)と実行(`bundle`)の両方が `build.sh vital lift/out-app/external lift/out-app/plugins/vital` を通る。本番の `fe-build bundle` が `plugins/vital.wasm` の元を作る(`spec/workflow.test.sh` の (v) が見る形)。
- 前回の試行の変更(`vst_crt.c`・`mk_stubs.mjs`・`main_vst_ext.c`・`build.sh`・`seeds/vital.txt`)は、そのまま取り込んだ。

### 検査

- `cargo test --locked -p fe-build`: 8 件すべて通る(`plans_match_recorded_texts` と `upload_plan_matches_recorded_text` を含む)。
- `spec/workflow.test.sh`: 431 件すべて ok(終了コード 0。(v) の本番の連鎖の検査を含む)。
- `spec/guard.test.sh`: 67 件すべて ok。

### 受け入れの実測

- `measure.mjs vital` の 1 行(そのまま): `{"plugin":"vital","instantiated":true,"blockFrames":4096,"nonSilent":true,"error":null}`
- 中の `RESULT` 行(そのまま): `RESULT out_hash=7d2d71f375347b15 nonfinite=0 samples=8192 nonzero=8192 maxabs=0.357191 rms=0.143032 blocks=1`。前回と同じ値(入力と処理が同じ)。
- 起動確認(`boot.json`): `state` engine-idle、`error` null、`standIn` false、`elapsedMs` 10,371。P1(10 秒以内)は 371 ms 超えた。前回は 10,566。原因は未確認。Vital の wasm は lazy 行で起動後に取るので、起動前に取る量は増えていない(起動時間への影響は測っていない。未確認)。
- manifest の lazy: `vital.wasm`(`plugins/vital.wasm`)、size 115,538,297 B、br の encodedSize 7,376,525 B、sha256 `f847e7a5227b71a2983825aa2bd4bd608dc482fed0ccc50b0fd0f17097fd78ef`。
- 到着時刻: 未測定(受け入れの範囲に無い)。
- OTT の wasm: 同じ bundle の段で作り直した(`build: done lift/out-app/plugins/ott (vst-ext.wasm 27803120 bytes)`)。OTT の 1 ブロックの測定は、この受け入れの範囲に無い(未確認)。Serum 2 は bundle の計画に無いので、作り直していない。

### lift(この runner の値。今回の入力で作り直した)

- 関数 33,567(分割 44 単位)、生成 C の命令 2,090,660 件、扱えない命令 204,695 件(232 種)。前回の所見(27,077 関数・204,668 件・229 種)と数が違う。差の原因は未確認。
- `.text`(VA 0x2001000〜): 1,959,922 件のうち、扱えない 181,337 件。
- `IPPCODE`(VA 0x28a6000〜0x29410cd): 130,738 件。扱えた 107,380 件(109 種)、扱えない 23,358 件(52 種)。扱えない 52 種のうち 50 種は AVX・FMA の V* 命令で、残りは `Pushfq` と `Fxsave`。件数の多い順に `Vaddps` 3,182・`Vsubps` 3,182・`Vmulps` 2,724・`Vmovapd` 2,720・`Vsubpd` 1,802・`Vaddpd` 1,802(生成 C の行の数)。
- `IPPDATA`(VA 0x3162000、0x9c0 B): 命令は無い(データだけ)。
- 扱えない命令の全件(232 種。lift のログの値)は、この文書の末尾の付録。

### import(`mk_stubs`)

- `mk_stubs: 501 imports`(序数 16 件を含む)。TABLE・harness の既存の答え・runtime の既存の定義に無いものは 286 件。内訳は、runtime の既存の定義が 26 件、`vst_crt.c` が答えた 81 件、名前のある停止が 179 件。
- 名前のある停止の 179 件の一覧は、下の「名前のある停止の一覧」と同じ(`nm` で数えて一致を確かめた)。

### wasm と配信

- `build.sh vital` の出力: `lift/out-app/plugins/vital/vst-ext.wasm` 115,538,297 B。
- 本番の連鎖: `fe-build bundle` が `build.sh vital` を通し、`pack-assets` が lazy の `plugins/vital.wasm` として束ねる(manifest の lazy に入る)。

### 変えたパス(3 回目)

- `lift/harness/web/pack.args`・`lift/crates/fe-build/tests/plan/bundle.txt`・`lift/crates/fe-build/src/main.rs`(衝突の解消と計画の Vital の段)。
- 前回の試行の変更は、この commit に入る(`vst_crt.c`・`mk_stubs.mjs`・`main_vst_ext.c`・`build.sh`・`seeds/vital.txt`・この文書)。

## 2 回目の試行の記録

判定: 受理。受け入れコマンドを、Actions の runner で、最初から(`lift/out-app/plugins/vital` を作り直して)通した。終了コードは 0。
測定の値は runner の値。この機械とは別枠。nproc = 4。loadavg は起動確認の時点で 1.36 1.26 1.95。

## 結論

1. 受け入れの全段が 0 で終わった(2026-11-24 12:30:53 → 13:21:15 UTC。Vital の wasm の段の後に、エンジンと bundle を作った)。
2. `vital.json`: `{"plugin":"vital","instantiated":true,"blockFrames":4096,"nonSilent":true,"error":null}`。
3. `boot.json`: `state` engine-idle、`error` null、`standIn` false。`elapsedMs` は 10,566(10 秒を 0.6 秒超えた。前の試行は 11,345 と 12,376)。
4. `manifest.json` の `lazy` に `vital.wasm`(`plugins/vital.wasm`、115,536,635 B、br 7,330,260 B)がある。
5. 前回の差し戻しの原因だった 3 点は直した。(a) C++ のマングル名が C の識別子にならず、コンパイルが落ちた(`mk_stubs.mjs`)。(b) 答えの無い CRT の import で止まった(`vst_crt.c` を足した)。(c) Vital は synth なので、ノートが無いと無音だった(イベント入力を持つプラグインに、既定でノートオンを 1 つ送るようにした)。
6. 未確認: 出力の音の正しさ(参照が無い)、wasm の到着時刻(起動後の取得は、この受け入れでは測っていない)、この変更による Serum 2 と OTT の build(作り直していない)。

## 実測 1: 入力の照合と取り出し

- `tools/external-inputs/check.mjs lift/out-app/external`: 3 件とも `ok`(Serum 2・OTT・Vital)。`VitalInstaller.exe` は 41,808,960 B、SHA-256 `5bf1b8c7…6ed7`(`ci/artifacts.lock` の 4 行目と一致)。
- `tools/external-inputs/extract.mjs`: VST3 本体 `Vital.vst3` は 15,571,240 B、SHA-256 `a622a2c99b4066cd7945a4ab9bbdd698e7632a30702f6f0a7ccbf26a56b576e1`。`inputs/external-plugins.tsv` の Vital の行と一致。
- PE(元の本体): x86-64(machine 0x8664)、PE32+。節は `.text`・`IPPCODE`(VA 0x28a6000、0x9b0cd B)・`.rdata`・`.data`・`.pdata`・`_RDATA`・`IPPDATA`(0x9c0 B)・`.rsrc`・`.reloc`。import は 26 DLL。
- 作業用の `Vital.vst3` は ImageBase を 0x2000000 に直した版(`fe-pe rebase`)で、SHA-256 は `baafc28986f7389a23d02c6250c9f1b55645ac78cd4e5f90d6648851ddbe8967`。これは元の版と違うのが正しい(リベース後の版)。

## 実測 2: fe-lift による lift(`--pdata`、`IPPCODE`・`IPPDATA` を含む)

ログ(`.tmp-plugin-vital/log/lift-vital.txt`)の値: `functions` 27,077、`emitted_instructions` 2,062,273、`unsupported_total` 204,668(種類 229)、`tail_calls` 9,062、`c_bytes` 173,735,251(分割 44 単位)。

節ごとの内訳は、生成 C の命令の行(`/* 0x<va> <mnemonic> */`)を数えた値(ログの合計とは数え方が違うので参考値):

| 節 | 命令 | うち扱えない命令(`fle_unsupported`) |
|---|---|---|
| `.text` | 1,959,922 | 181,337 |
| `IPPCODE` | 130,738 | 23,358 |
| `IPPDATA` | 0(データだけの節) | 0 |

- 扱えない命令は、ほとんどが AVX・AVX-512(`V*` の命令。`Vaddps` 15,114、`Vmovups`(EVEX)14,690、`Vsubps` 12,791、`Vmulps` 11,252、`Vmovss` 8,678 など)。AVX-512 のマスク命令(`Kmovw` 1,953 など)、BMI(`Shlx` 374、`Shrx` 26、`Blsr` 12)、SSE4.1 の一部(`Pmulld` 231、`Pextrd` 60、`Pmovsxdq` 176 など)。`Int3` が 4,045 あり、これは関数の並びの埋め草と見られる(未確認)。全件は `Vital` の生成 C で数えた値。
- 扱えた命令の種類は、生成 C で 246 種類。
- この 1 ブロックの実行では、扱えない命令の停止(`fle_unsupported`)は 1 度も出なかった。IPP の命令の切り替え(CPUID の答えによる分岐)がどの経路を通ったかは、追跡していない(未確認)。

## 実測 3: import の答え

- 受け入れのビルドは `fe-pe` と `fe-decode` と `gen_vst.sh` の段を通る。Vital の import は 501 件(序数 16 件を含む)。`mk_stubs.mjs` が、TABLE・harness の既存の答え(`imports-vst.txt`)・runtime の既存の定義に無い 286 件を作る。
- 内訳: runtime の既存の定義 26 件、`lift/harness/web/mt1/plugin-worker/vst_crt.c` で答えた 81 件、名前のある停止のまま 179 件(下の一覧)。
- `vst_crt.c` の答え(Vital の import のうち 81 件)の内容:
  - CRT の起動: `_initterm`・`_initterm_e`(関数ポインタの表を `fle_call_guest` で呼ぶ)、onexit の表(`_initialize_onexit_table`・`_register_onexit_function`・`_execute_onexit_table`・`_crt_atexit`、表は増やせる)、`_cexit`・`_configure_narrow_argv`・`_initialize_narrow_environment`・`_callnewh`・`_fpreset`・`_errno`。
  - ヒープと文字列: `malloc`・`calloc`・`realloc`・`free`・`_aligned_malloc`・`_aligned_free`(ホストの malloc を使う)、`memcpy`・`memmove`・`memset`・`memcmp`・`memchr`・`strlen`・`strcmp`・`strncmp`・`strchr`・`atoi`・`_atoi64`・`strtoll`・`strtoull`。
  - 数学: 単精度と倍精度の `sin`・`cos`・`tan`・`acos`・`exp`・`log`・`pow`・`fmod`・`atan2`・`round`、単精度の `*f` 系、`ldexp`・`frexp`(XMM0・XMM1 を読む。ホストの libm)。
  - 書式: `__stdio_common_vsprintf`・`_vsprintf_s`・`__stdio_common_vswprintf`(書式を 1 変換ずつホストの snprintf で整形)、`__stdio_common_vsscanf`(ホストの sscanf。書き込み先のポインタは va_list から取る)。
  - RTTI: `__RTDynamicCast`(MSVC x64 の RTTI を読む。非仮想の基底のみ。仮想の基底は名前のある停止)。
  - 時刻: `_ftime64_s`(エポック 0 を返す。実時計は使わない)、`_gmtime64_s`・`_localtime64_s`・`_mktime64`(UTC の算術で書いた。ホストの libc は `_tzset_js` を要するので使わない)。
  - ロケール: `localeconv`(C ロケール)、`iswalnum`・`iswalpha`・`iswdigit`・`iswlower`・`iswspace`・`iswupper`・`towlower`・`towupper`(ホストの `<wctype.h>`)。
  - 乱数: `rand`・`srand`(UCRT と同じ線形合同法。種 1。`srand` は Vital が import しないので、呼ばれない)。
  - 例外: `__std_exception_copy`・`__std_exception_destroy`(文字列を複製して持つ)。`_CxxThrowException` は巻き戻さないので停止する。停止の表示には投げられた型の名前を出す。
  - kernel32: `ReplaceFileW` は runtime の VFS の `DeleteFileW` と `MoveFileW` で答える(バックアップ付きは名前のある停止)。
- 答えを置いた理由(設計の判断): 答えは、本物の挙動を写すか、名前のある停止にするかのどちらか。写せないものは停止にした(`vst3a_unanswered` の規則どおり)。

## 実測 4: wasm と lazy の配信

- `build.sh vital` の出力: `lift/out-app/plugins/vital/vst-ext.wasm` 115,536,635 B、`vital.vst3`(リベース後)15,571,240 B、`seed.tsv` 342 B。
- 増分(lift を再利用)の build は約 2 分。lift を含む全体の build は約 12 分。
- `lift/harness/web/pack.args` に `--lazy-file lift/out-app/plugins/vital/vst-ext.wasm=plugins/vital.wasm` を足し、`lift/crates/fe-build/tests/plan/bundle.txt` を同じ引数に合わせた。
- manifest の lazy の行: `name` `vital.wasm`、`path` `plugins/vital.wasm`、`size` 115,536,635、`sha256` `278bab51311b1cd9041472723860027db47977c3f14a4cc90a1cb65afd24f055`、`compression` br、`encodedSize` 7,330,260。
- 到着時刻: 未測定。起動後の取得のブラウザでの時間は、この受け入れの範囲に無い。起動前に取る量は増えていない(lazy のみ)。

## 実測 5: 1 ブロックの処理(`tools/plugin-run/measure.mjs vital`)

- 出力の 1 行(そのまま): `{"plugin":"vital","instantiated":true,"blockFrames":4096,"nonSilent":true,"error":null}`
- 中の `RESULT` 行(そのまま): `RESULT out_hash=7d2d71f375347b15 nonfinite=0 samples=8192 nonzero=8192 maxabs=0.357191 rms=0.143032 blocks=1`
- 入力: 試験信号(OTT と同じ)。Vital は `events in=1` のイベント入力を持つので、ブロック 0 に C4(速度 100/127)のノートオンを 1 つ送る。この規則は `main_vst_ext.c` の既定で、オプションではない。ノートを送る前の版では、出力は全サンプル 0(`nonzero=0`)だった。
- 未確認: 音の正しさ(参照の出力が無い)。`nonSilent` は「0 でないサンプルがある」だけを言う。`nonfinite` は 0 だった。

## 実測 6: 起動確認(`tools/boot-check/boot-bundle.mjs`)

- `state` engine-idle、`error` null、`standIn` false、`elapsedMs` 10,566(runner の値)。
- P1(10 秒以内)は、この測定では 0.6 秒超えた。Vital の追加は起動前の取得を増やしていないが、この測定の差が Vital によるかは未確認。

## 変えたパス

- `lift/harness/web/mt1/plugin-worker/vst_crt.c`(新規): C ランタイムの答え(上の 81 件)。
- `lift/harness/web/mt1/plugin-worker/build.sh`: `vst_crt.c` を harness の列に足す。説明の文を直す。
- `lift/harness/web/mt1/plugin-worker/main_vst_ext.c`: イベント入力を持つプラグインに、ブロック 0 で C4 のノートオンを送る。説明の文を足す。
- `lift/harness/web/mt1/plugin-worker/seeds/vital.txt`(新規): `seeds_scan.mjs` の出力 7,739 行(lea の先と switch の先の関数の入口)。
- `lift/harness/vst/mk_stubs.mjs`: C++ のマングル名(`?`・`@` などを含む)を、`_<16 進>_` で C の識別子にする。同じ名前になる別々の API を検査で止める。C の文字列に書く `?` は `\?` とし、トライグラフにならないようにする。
- `lift/harness/web/pack.args`・`lift/crates/fe-build/tests/plan/bundle.txt`: Vital の lazy-file の 1 組。

## 製品の決まりの確認

- `guard/scan.sh` を変えた 7 ファイルに当てた: 指摘 0。`guard/flags.sh` の正規表現(`getenv(`・`#ifdef`・`URLSearchParams`・`process.env.` など)に当てた: 0 件。`getenv` の答えは、この正規表現に当たるので作らなかった(Vital はまだ呼んでいない)。
- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足していない。オプトインを作っていない(イベント入力の規則は既定で働く)。
- Python は、この経路(`build.sh`・`gen_vst.sh`・`mk_stubs.mjs`・`main_vst_ext.c`)に使っていない。前回所見のとおり、`lift/harness/vst3a/build_vst3a_wasm.sh` と `gen_vst3a.sh` が `python3` を呼ぶ(Serum 2 の旧い recipe。Vital の経路では使わない)。

## 残り・既知の限界

- 名前のある停止が 179 件残っている(下の一覧)。msvcp140 の 58 件は、C++ の iostream・locale・mutex の関数。kernel32・user32・gdi32・winmm・imm32・shell32 の GUI と音声の関数は、この 1 ブロックの経路には無かった。
- C++ の例外は、巻き戻さない。投げられたら、型の名前を出して停止する。この経路では投げられなかった(前の試行では、設定ファイルの parse が投げていた。`ReplaceFileW` を正しく答えて解消した)。
- 時刻は UTC と、`_ftime64_s` の epoch 0 に固定している。実時計は模型化していない。
- 2 つのクラス(Audio Module と Component Controller)の CID が、同じ値に表示される(先頭は 0000000010AB で、2 つとも同じ値)。harness の表示の問題か、読みの問題かは未確認。VST3 では 2 つの CID は普通は違う。
- wasm は 115.5 MB(br 7.3 MB)。起動後の取得で届ける。P3(Song Mode で音が途切れない)と P2(120 fps)への影響は、この受け入れでは測っていない(未確認)。
- Serum 2 と OTT の wasm は作り直していない。`build.sh` は `vst_crt.c` を 3 つのプラグインすべてに足すし、`main_vst_ext.c` の既定のノートオンは Serum 2 にも効く。どちらも未確認。
- 前回試行の `tools/plugin-run/measure.mjs`(`BUNDLE PLUGIN FILE` の形)と `spec/plugin-run.test.mjs` は、今の契約と合わないので取り除いた(main の `measure.mjs` を使う)。
- 前回の差し戻しの指摘「受け入れの第 3 引数 `$RUNNER_TEMP/external/vital.exe`」は、今の受け入れ(`lift/out-app/external` を読む)では起きない。

## 停止の記録(この受け入れを通すまでに出た停止の順)

1. `mk_stubs`: C++ のマングル名が C の関数名として不正 → コンパイル失敗 → 名前を変換して直した。
2. `api-ms-win-crt-runtime-l1-1-0.dll!_initialize_onexit_table` が未答 → `vst_crt.c` の onexit と `_initterm`。
3. `_initterm` の引数を、`fle_call_guest` の後で読み直していた(R[1]・R[2] は guest が書き換える)→ 局所変数に写して直した。
4. `EnterCriticalSection` の「初期化されていない」停止は、`_initterm` の順の問題の結果だった(静的初期化子が正しく走らなかった)。`_initterm` を直して解消。
5. `tanf`・`__stdio_common_vsprintf`・`__stdio_common_vsscanf` の未答 → math・書式の答え。
6. `indirect call to 0x23a9c60 is not a known function entry` → `lea` の先の関数の入口が seed に無かった → `seeds/vital.txt` を作った(`seeds_scan.mjs`)。
7. `onexit: more functions than the table keeps` → 表を増やせるようにした。
8. `rand`・`__RTDynamicCast`・`towlower`・`_ftime64_s`・`_gmtime64_s`・`localeconv`・`__stdio_common_vswprintf` の未答 → 答えを足した。`_tzset_js` の link 失敗 → 時刻の算術に変えた。
9. `ReplaceFileW` の未答 → 拒否の答え → 設定の parse が投げた → VFS の `DeleteFileW`・`MoveFileW` で答えた。
10. `measure` の `nonSilent` が偽(synth にノートが無い)→ イベント入力を持つプラグインに、ブロック 0 でノートオンを送った。

## 名前のある停止の一覧(179 件、DLL ごと。関数名は生成 C の `fle_imp_<API>`)

- **advapi32.dll** (2): `fle_imp_RegEnumKeyW` `fle_imp_RegOpenKeyW`
- **api-ms-win-crt-convert-l1-1-0.dll** (4): `fle_imp__strtod_l` `fle_imp_mbstowcs` `fle_imp_strtod` `fle_imp_wcstombs`
- **api-ms-win-crt-environment-l1-1-0.dll** (1): `fle_imp_getenv`
- **api-ms-win-crt-heap-l1-1-0.dll** (1): `fle_imp__aligned_realloc`
- **api-ms-win-crt-locale-l1-1-0.dll** (1): `fle_imp__create_locale`
- **api-ms-win-crt-math-l1-1-0.dll** (6): `fle_imp__copysign` `fle_imp__dclass` `fle_imp__dsign` `fle_imp__fdclass` `fle_imp__fdsign` `fle_imp__finite`
- **api-ms-win-crt-runtime-l1-1-0.dll** (7): `fle_imp__beginthreadex` `fle_imp__endthreadex` `fle_imp__invoke_watson` `fle_imp__seh_filter_dll` `fle_imp_abort` `fle_imp_exit` `fle_imp_terminate`
- **api-ms-win-crt-stdio-l1-1-0.dll** (12): `fle_imp___acrt_iob_func` `fle_imp___stdio_common_vfprintf` `fle_imp___stdio_common_vswscanf` `fle_imp__fseeki64` `fle_imp__ftelli64` `fle_imp_fclose` `fle_imp_fflush` `fle_imp_fopen` `fle_imp_fread` `fle_imp_fseek` `fle_imp_ftell` `fle_imp_fwrite`
- **api-ms-win-crt-time-l1-1-0.dll** (1): `fle_imp_wcsftime`
- **api-ms-win-crt-utility-l1-1-0.dll** (1): `fle_imp_qsort`
- **comdlg32.dll** (2): `fle_imp_GetOpenFileNameW` `fle_imp_GetSaveFileNameW`
- **dbghelp.dll** (1): `fle_imp_MiniDumpWriteDump`
- **gdi32.dll** (10): `fle_imp_ChoosePixelFormat` `fle_imp_DescribePixelFormat` `fle_imp_GetGlyphIndicesW` `fle_imp_GetGlyphOutlineW` `fle_imp_GetKerningPairsW` `fle_imp_GetOutlineTextMetricsW` `fle_imp_RemoveFontMemResourceEx` `fle_imp_SetMapperFlags` `fle_imp_SetPixelFormat` `fle_imp_SwapBuffers`
- **imm32.dll** (5): `fle_imp_ImmGetCompositionStringW` `fle_imp_ImmGetContext` `fle_imp_ImmNotifyIME` `fle_imp_ImmReleaseContext` `fle_imp_ImmSetCandidateWindow`
- **kernel32.dll** (20): `fle_imp_CancelIo` `fle_imp_ConnectNamedPipe` `fle_imp_CreateMutexW` `fle_imp_CreatePipe` `fle_imp_CreateProcessW` `fle_imp_DeviceIoControl` `fle_imp_DisconnectNamedPipe` `fle_imp_GetExitCodeProcess` `fle_imp_GetOverlappedResult` `fle_imp_GetPriorityClass` `fle_imp_GlobalSize` `fle_imp_K32EnumProcessModules` `fle_imp_K32GetModuleBaseNameA` `fle_imp_K32GetProcessMemoryInfo` `fle_imp_PeekNamedPipe` `fle_imp_ReleaseMutex` `fle_imp_SetHandleInformation` `fle_imp_SetThreadAffinityMask` `fle_imp_TerminateThread` `fle_imp_TryEnterCriticalSection`
- **msvcp140.dll** (58): `fle_imp__3f__3f_0_3f__24_basic_ios_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_IEAA_40_XZ` `fle_imp__3f__3f_0_3f__24_basic_iostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAA_40_PEAV_3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_1_40__40_Z` `fle_imp__3f__3f_0_3f__24_basic_istream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAA_40_PEAV_3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_1_40__N_40_Z` `fle_imp__3f__3f_0_3f__24_basic_ostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAA_40_PEAV_3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_1_40__N_40_Z` `fle_imp__3f__3f_0_3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_IEAA_40_XZ` `fle_imp__3f__3f_1_3f__24_basic_ios_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_UEAA_40_XZ` `fle_imp__3f__3f_1_3f__24_basic_iostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_UEAA_40_XZ` `fle_imp__3f__3f_1_3f__24_basic_istream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_UEAA_40_XZ` `fle_imp__3f__3f_1_3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_UEAA_40_XZ` `fle_imp__3f__3f_6_3f__24_basic_ostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAAEAV01_40_H_40_Z` `fle_imp__3f__3f_6_3f__24_basic_ostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAAEAV01_40_N_40_Z` `fle_imp__3f__3f_6_3f__24_basic_ostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAAEAV01_40_P6AAEAV01_40_AEAV01_40__40_Z_40_Z` `fle_imp__3f__3f_6_3f__24_basic_ostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAAEAV01_40_P6AAEAVios_base_40_1_40_AEAV21_40__40_Z_40_Z` `fle_imp__3f__3f__D_3f__24_basic_ostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAXXZ` `fle_imp__3f__Ipfx_40__3f__24_basic_istream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAA_N_N_40_Z` `fle_imp__3f__Lock_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_UEAAXXZ` `fle_imp__3f__Osfx_40__3f__24_basic_ostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAXXZ` `fle_imp__3f__Syserror_map_40_std_40__40_YAPEBDH_40_Z` `fle_imp__3f__Throw_Cpp_error_40_std_40__40_YAXH_40_Z` `fle_imp__3f__Unlock_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_UEAAXXZ` `fle_imp__3f__Xbad_alloc_40_std_40__40_YAXXZ` `fle_imp__3f__Xbad_function_call_40_std_40__40_YAXXZ` `fle_imp__3f__Xlength_error_40_std_40__40_YAXPEBD_40_Z` `fle_imp__3f__Xout_of_range_40_std_40__40_YAXPEBD_40_Z` `fle_imp__3f_classic_40_locale_40_std_40__40_SAAEBV12_40_XZ` `fle_imp__3f_cout_40_std_40__40_3V_3f__24_basic_ostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_1_40_A` `fle_imp__3f_flush_40__3f__24_basic_ostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAAEAV12_40_XZ` `fle_imp__3f_good_40_ios_base_40_std_40__40_QEBA_NXZ` `fle_imp__3f_imbue_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAAXAEBVlocale_40_2_40__40_Z` `fle_imp__3f_overflow_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAAHH_40_Z` `fle_imp__3f_pbackfail_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAAHH_40_Z` `fle_imp__3f_put_40__3f__24_basic_ostream_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAAEAV12_40_D_40_Z` `fle_imp__3f_sbumpc_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAHXZ` `fle_imp__3f_seekoff_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAA_3f_AV_3f__24_fpos_40_U_Mbstatet_40__40__40_2_40__JHH_40_Z` `fle_imp__3f_seekpos_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAA_3f_AV_3f__24_fpos_40_U_Mbstatet_40__40__40_2_40_V32_40_H_40_Z` `fle_imp__3f_setbuf_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAAPEAV12_40_PEAD_J_40_Z` `fle_imp__3f_setprecision_40_std_40__40_YA_3f_AU_3f__24__Smanip_40__J_40_1_40__J_40_Z` `fle_imp__3f_setstate_40__3f__24_basic_ios_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAXH_N_40_Z` `fle_imp__3f_setw_40_std_40__40_YA_3f_AU_3f__24__Smanip_40__J_40_1_40__J_40_Z` `fle_imp__3f_sgetc_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAHXZ` `fle_imp__3f_showmanyc_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAA_JXZ` `fle_imp__3f_snextc_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAHXZ` `fle_imp__3f_sputc_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAAHD_40_Z` `fle_imp__3f_sputn_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEAA_JPEBD_J_40_Z` `fle_imp__3f_sync_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAAHXZ` `fle_imp__3f_uflow_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAAHXZ` `fle_imp__3f_uncaught_exceptions_40_std_40__40_YAHXZ` `fle_imp__3f_underflow_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAAHXZ` `fle_imp__3f_widen_40__3f__24_basic_ios_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_QEBADD_40_Z` `fle_imp__3f_xsgetn_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAA_JPEAD_J_40_Z` `fle_imp__3f_xsputn_40__3f__24_basic_streambuf_40_DU_3f__24_char_traits_40_D_40_std_40__40__40_std_40__40_MEAA_JPEBD_J_40_Z` `fle_imp__Cnd_broadcast` `fle_imp__Cnd_wait` `fle_imp__Mtx_lock` `fle_imp__Mtx_unlock` `fle_imp__Query_perf_counter` `fle_imp__Query_perf_frequency` `fle_imp__Thrd_yield`
- **ole32.dll** (1): `fle_imp_PropVariantClear`
- **shell32.dll** (6): `fle_imp_CommandLineToArgvW` `fle_imp_ExtractAssociatedIconW` `fle_imp_SHBrowseForFolderW` `fle_imp_SHCreateShellItem` `fle_imp_SHParseDisplayName` `fle_imp_Shell_NotifyIconW`
- **shlwapi.dll** (1): `fle_imp_PathStripToRootW`
- **user32.dll** (10): `fle_imp_AttachThreadInput` `fle_imp_CreateWindowExA` `fle_imp_EndDialog` `fle_imp_EnumDisplaySettingsA` `fle_imp_FindWindowExW` `fle_imp_GetWindowInfo` `fle_imp_GetWindowLongW` `fle_imp_SendMessageTimeoutW` `fle_imp_SetWindowLongW` `fle_imp_TrackMouseEvent`
- **vcruntime140.dll** (8): `fle_imp___C_specific_handler` `fle_imp___current_exception` `fle_imp___current_exception_context` `fle_imp___intrinsic_setjmp` `fle_imp___std_terminate` `fle_imp___std_type_info_destroy_list` `fle_imp__purecall` `fle_imp_longjmp`
- **vcruntime140_1.dll** (1): `fle_imp___CxxFrameHandler4`
- **wininet.dll** (3): `fle_imp_FtpOpenFileW` `fle_imp_InternetCrackUrlW` `fle_imp_InternetSetFilePointer`
- **winmm.dll** (15): `fle_imp_midiInAddBuffer` `fle_imp_midiInClose` `fle_imp_midiInMessage` `fle_imp_midiInOpen` `fle_imp_midiInPrepareHeader` `fle_imp_midiInReset` `fle_imp_midiInStart` `fle_imp_midiInStop` `fle_imp_midiInUnprepareHeader` `fle_imp_midiOutClose` `fle_imp_midiOutLongMsg` `fle_imp_midiOutMessage` `fle_imp_midiOutPrepareHeader` `fle_imp_midiOutUnprepareHeader` `fle_imp_timeKillEvent`
- **ws2_32.dll** (2): `fle_imp_freeaddrinfo` `fle_imp_getaddrinfo`

## 付録: 扱えない命令の全件(3 回目の試行の lift のログ。232 種)

件数は fe-lift のログの値。生成 C の `fle_unsupported` の行の数と一致する(節の区別はしない。IPPCODE の内訳は本文)。

```
     15114  Vaddps: mnemonic outside minimal set
     14690  Vmovups: EVEX encoding
     12791  Vsubps: mnemonic outside minimal set
     11252  Vmulps: mnemonic outside minimal set
      8678  Vmovss: mnemonic outside minimal set
      7728  Vpshufd: mnemonic outside minimal set
      7716  Vmovaps: EVEX encoding
      7153  Vfmadd231ps: mnemonic outside minimal set
      7081  Vshufps: mnemonic outside minimal set
      4942  Vmovupd: mnemonic outside minimal set
      4672  Vaddpd: mnemonic outside minimal set
      4111  Vmovsd: mnemonic outside minimal set
      4045  Int3: mnemonic outside minimal set
      3960  Vsubpd: mnemonic outside minimal set
      3503  Vmovapd: mnemonic outside minimal set
      3438  Vpermpd: mnemonic outside minimal set
      3358  Vfnmadd231ps: mnemonic outside minimal set
      3288  Vaddss: mnemonic outside minimal set
      3164  Vfmadd213ps: mnemonic outside minimal set
      3109  Vmovd: mnemonic outside minimal set
      3047  Vmulpd: mnemonic outside minimal set
      2848  Vmovdqu: mnemonic outside minimal set
      2759  Vextractf128: mnemonic outside minimal set
      2542  Vinsertf128: mnemonic outside minimal set
      2295  Vmulss: mnemonic outside minimal set
      2169  Vsubss: mnemonic outside minimal set
      2100  Vfmadd231ss: mnemonic outside minimal set
      2066  Vbroadcastss: mnemonic outside minimal set
      1998  Vfmadd231pd: mnemonic outside minimal set
      1953  Kmovw: mnemonic outside minimal set
      1797  Vperm2f128: mnemonic outside minimal set
      1782  Vmovlps: mnemonic outside minimal set
      1630  Vpaddd: mnemonic outside minimal set
      1615  Vxorps: mnemonic outside minimal set
      1595  Vfnmadd213ps: mnemonic outside minimal set
      1537  Vmovdqa: mnemonic outside minimal set
      1412  Vpsrld: mnemonic outside minimal set
      1394  Vpermd: mnemonic outside minimal set
      1378  Vpunpcklqdq: mnemonic outside minimal set
      1292  Vaddsd: mnemonic outside minimal set
      1200  Vpand: mnemonic outside minimal set
      1162  Vfmadd213pd: mnemonic outside minimal set
      1148  Vextractf64x4: mnemonic outside minimal set
      1050  Vshufpd: mnemonic outside minimal set
      1025  Vpmulld: mnemonic outside minimal set
      1014  Vfnmadd231pd: mnemonic outside minimal set
       895  Vfmaddsub231ps: mnemonic outside minimal set
       840  Vsubsd: mnemonic outside minimal set
       835  Vfmsubadd213ps: mnemonic outside minimal set
       816  Vfmsub231ps: mnemonic outside minimal set
       812  Vfmaddsub213ps: mnemonic outside minimal set
       807  Vmulsd: mnemonic outside minimal set
       758  Vpslld: mnemonic outside minimal set
       706  Vfmsubadd231ps: mnemonic outside minimal set
       662  Vmovhps: mnemonic outside minimal set
       660  Vextractps: mnemonic outside minimal set
       655  Vfmadd231sd: mnemonic outside minimal set
       623  Vbroadcastf128: mnemonic outside minimal set
       620  Vshufi32x4: mnemonic outside minimal set
       619  Vfmadd213ss: mnemonic outside minimal set
       604  Vgatherdps: mnemonic outside minimal set
       584  Vunpcklps: mnemonic outside minimal set
       582  Vfmsub213ps: mnemonic outside minimal set
       580  Vpextrd: mnemonic outside minimal set
       555  Vmovhlps: mnemonic outside minimal set
       477  Vfnmadd132ps: mnemonic outside minimal set
       454  Vunpckhps: mnemonic outside minimal set
       444  Vandps: mnemonic outside minimal set
       412  Vblendmps: mnemonic outside minimal set
       374  Shlx: mnemonic outside minimal set
       369  Vpxord: mnemonic outside minimal set
       348  Vfmsub231ss: mnemonic outside minimal set
       305  Vpcmpeqb: mnemonic outside minimal set
       288  Vpandd: mnemonic outside minimal set
       286  Vxorpd: mnemonic outside minimal set
       284  Valignd: mnemonic outside minimal set
       284  Vcmpps: mnemonic outside minimal set
       260  Vinsertps: mnemonic outside minimal set
       258  Vfmadd132ps: mnemonic outside minimal set
       245  Vpor: mnemonic outside minimal set
       240  Vmovshdup: mnemonic outside minimal set
       240  Vmovsldup: mnemonic outside minimal set
       239  Vfmadd213sd: mnemonic outside minimal set
       231  Pmulld: mnemonic outside minimal set
       212  Vfnmadd213pd: mnemonic outside minimal set
       212  Vpxor: mnemonic outside minimal set
       202  Vfmsub213pd: mnemonic outside minimal set
       184  Vunpcklpd: mnemonic outside minimal set
       180  Vfmsub213ss: mnemonic outside minimal set
       178  Vunpckhpd: mnemonic outside minimal set
       176  Pmovsxdq: mnemonic outside minimal set
       163  Vmovdqu32: mnemonic outside minimal set
       146  Vmovlhps: mnemonic outside minimal set
       144  Vfmsub231pd: mnemonic outside minimal set
       144  Vpbroadcastd: mnemonic outside minimal set
       134  Vfnmadd231ss: mnemonic outside minimal set
       126  Vorps: mnemonic outside minimal set
       111  Vscatterdps: mnemonic outside minimal set
       106  Vpmuldq: mnemonic outside minimal set
       104  Vcvtps2pd: mnemonic outside minimal set
        95  Vpternlogd: mnemonic outside minimal set
        90  Vpandnd: mnemonic outside minimal set
        89  Vfmsub213sd: mnemonic outside minimal set
        86  Vfmadd132sd: mnemonic outside minimal set
        86  Vpmullq: mnemonic outside minimal set
        85  Vcvtpd2ps: mnemonic outside minimal set
        76  Vgatherqps: mnemonic outside minimal set
        74  Vblendvps: mnemonic outside minimal set
        70  Vfmsub231sd: mnemonic outside minimal set
        70  Vmovddup: mnemonic outside minimal set
        70  Vmovhpd: mnemonic outside minimal set
        68  Vaddsubps: mnemonic outside minimal set
        68  Vinsertf64x4: mnemonic outside minimal set
        66  Vmovq: mnemonic outside minimal set
        64  Vfmadd132ss: mnemonic outside minimal set
        61  Vscatterqps: mnemonic outside minimal set
        60  Pextrd: mnemonic outside minimal set
        58  Vfmadd132pd: mnemonic outside minimal set
        58  Vpermt2ps: mnemonic outside minimal set
        57  Vpermi2ps: mnemonic outside minimal set
        56  Vpackssdw: mnemonic outside minimal set
        54  Vfmsub132pd: mnemonic outside minimal set
        54  Vldmxcsr: mnemonic outside minimal set
        52  Call: indirect through IAT slot (not lifted)
        52  Vcvtsd2ss: mnemonic outside minimal set
        52  Vfmaddsub132ps: mnemonic outside minimal set
        51  Vfmsubadd132ps: mnemonic outside minimal set
        49  Vbroadcastf32x4: mnemonic outside minimal set
        48  Vpmovmskb: mnemonic outside minimal set
        46  Vfmsub132ps: mnemonic outside minimal set
        44  Vextracti128: mnemonic outside minimal set
        42  Vmovmskps: mnemonic outside minimal set
        42  Vpcmpgtd: mnemonic outside minimal set
        41  Vcvtsi2sd: mnemonic outside minimal set
        40  Kshiftrd: mnemonic outside minimal set
        36  Vfnmadd231sd: mnemonic outside minimal set
        36  Vptestmd: mnemonic outside minimal set
        36  Vstmxcsr: mnemonic outside minimal set
        35  Vbroadcastsd: mnemonic outside minimal set
        35  Vpsrlvd: mnemonic outside minimal set
        34  Vpacksswb: mnemonic outside minimal set
        33  Vandnps: mnemonic outside minimal set
        32  Vmaskmovps: mnemonic outside minimal set
        32  Vmovntdq: mnemonic outside minimal set
        30  Vmovlpd: mnemonic outside minimal set
        29  Vcvtdq2pd: mnemonic outside minimal set
        29  Vextractf32x4: mnemonic outside minimal set
        28  Vcvtsi2ss: mnemonic outside minimal set
        28  Vdivss: mnemonic outside minimal set
        26  Shrx: mnemonic outside minimal set
        26  Vshuff32x4: mnemonic outside minimal set
        25  Vdivsd: mnemonic outside minimal set
        24  Kandw: mnemonic outside minimal set
        24  Popcnt: mnemonic outside minimal set
        24  Vpcmpeqw: mnemonic outside minimal set
        22  Vbroadcasti32x4: mnemonic outside minimal set
        22  Vpexpandd: mnemonic outside minimal set
        20  Vblendps: mnemonic outside minimal set
        20  Vfnmadd213sd: mnemonic outside minimal set
        20  Vfnmadd213ss: mnemonic outside minimal set
        19  Blendvps: mnemonic outside minimal set
        19  Kshiftrw: mnemonic outside minimal set
        18  Vpord: mnemonic outside minimal set
        16  Vpsraq: mnemonic outside minimal set
        15  Vextracti64x4: mnemonic outside minimal set
        14  Pinsrd: mnemonic outside minimal set
        14  Vcvtss2sd: mnemonic outside minimal set
        14  Vptestmb: mnemonic outside minimal set
        12  Blsr: mnemonic outside minimal set
        12  Vbroadcastf64x4: mnemonic outside minimal set
        12  Vextractf32x8: mnemonic outside minimal set
        12  Vpblendw: mnemonic outside minimal set
        12  Vpsubd: mnemonic outside minimal set
        12  Vucomisd: mnemonic outside minimal set
        11  Vpermi2d: mnemonic outside minimal set
        10  Pmovzxwd: mnemonic outside minimal set
        10  Vpcmpd: mnemonic outside minimal set
        10  Vpcmpeqd: mnemonic outside minimal set
         9  Pcmpestri: mnemonic outside minimal set
         9  Vpunpckldq: mnemonic outside minimal set
         8  Pmovsxwd: mnemonic outside minimal set
         8  Vfnmadd132sd: mnemonic outside minimal set
         8  Vmovdqu16: mnemonic outside minimal set
         6  Kortestw: mnemonic outside minimal set
         6  Pmovzxbd: mnemonic outside minimal set
         6  Pushfq: mnemonic outside minimal set
         6  Vinsertf32x8: mnemonic outside minimal set
         6  Vmovdqa32: mnemonic outside minimal set
         6  Vpbroadcastb: mnemonic outside minimal set
         6  Vpermt2d: mnemonic outside minimal set
         6  Vpinsrd: mnemonic outside minimal set
         6  Vpunpcklwd: mnemonic outside minimal set
         6  Vucomiss: mnemonic outside minimal set
         5  Vpblendmd: mnemonic outside minimal set
         4  Kmovd: mnemonic outside minimal set
         4  Lzcnt: mnemonic outside minimal set
         4  Pmaxud: mnemonic outside minimal set
         4  Pmovzxdq: mnemonic outside minimal set
         4  Vcompressps: mnemonic outside minimal set
         4  Vexpandps: mnemonic outside minimal set
         4  Vpermq: mnemonic outside minimal set
         4  Vpshufb: mnemonic outside minimal set
         3  Blendvpd: mnemonic outside minimal set
         3  Vfmsub132sd: mnemonic outside minimal set
         3  Vpbroadcastq: mnemonic outside minimal set
         3  Vpmaskmovd: mnemonic outside minimal set
         3  Vpsllvd: mnemonic outside minimal set
         3  Vpunpcklbw: mnemonic outside minimal set
         3  Vsqrtsd: mnemonic outside minimal set
         3  Vsqrtss: mnemonic outside minimal set
         2  Insertps: mnemonic outside minimal set
         2  Int: mnemonic outside minimal set
         2  Stc: mnemonic outside minimal set
         2  Tzcnt: mnemonic outside minimal set
         2  Vandpd: mnemonic outside minimal set
         2  Vfnmsub213sd: mnemonic outside minimal set
         2  Vfnmsub213ss: mnemonic outside minimal set
         2  Vorpd: mnemonic outside minimal set
         2  Vpandn: mnemonic outside minimal set
         2  Vpcmpeqq: mnemonic outside minimal set
         2  Vpermi2pd: mnemonic outside minimal set
         2  Vpermilps: mnemonic outside minimal set
         2  Vpermt2pd: mnemonic outside minimal set
         2  Vpmovzxbd: mnemonic outside minimal set
         1  Call: indirect call memory operand is not 64-bit
         1  Fxsave: mnemonic outside minimal set
         1  In: mnemonic outside minimal set
         1  Pcmpeqq: mnemonic outside minimal set
         1  Ptest: mnemonic outside minimal set
         1  Vfmsub132ss: mnemonic outside minimal set
         1  Vpmovzxbq: mnemonic outside minimal set
         1  Vpsllq: mnemonic outside minimal set
```
