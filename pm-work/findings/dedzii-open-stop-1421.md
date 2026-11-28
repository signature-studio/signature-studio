# DEDZII - NIGHT TERROR.flp を開いた時の停止の原因と直し方(Issue #1421、2026-11-25)

## 0. 範囲と条件

- 実行: `node tools/e2e/run.mjs --scenarios smoke --only file-open-demo --displays 4k-dpr1`(修正の前と後)。修正の後は `--displays 4k-dpr1,4k-random`(受け入れと同じ)。
- 機械: runner の値。この機械とは別枠。nproc 4。この作業の受け入れの実行の開始時の loadavg 0.70 0.52 0.25(前回の run 45593939525 は 1.32 0.49 0.20)。
- 版: この Issue の作業の commit の bundle(`fe-build bundle`、この作業の中で作った)。

## 1. 停止の内容(修正の前。前回の run 45593939525 の実測)

E2E の JSON の `engineStop`(修正の前、4k-dpr1)の要点:

- `exit`: stopped、code 134。
- `message`: `fle: indirect call to 0xb2844c0 is not a known function entry [guest return 0xb2c1e28]`
- `stop.stopLine`: `== stop: SIGABRT (runtime abort, see runtime message)`
- `stop.abortKind`: `fle_dispatch_miss`
- `returnAddress`: `0xb2c1e28`
- `registers`(R[0..15]、停止時): rax=0x6601 rcx=0 rdx=0x2 rbx=0x3e6375f8 rsp=0x3e1e2198 rbp=0x100 rsi=0x2 rdi=0x3e6375f8 r8=0x6600 r9=0 r10=0x7fd r11=0x413438c0 r12..r15=0
- 停止の直前のログ: `image-read: elastique_x64.b000000.dll requested=1`(Elastique の real module の読み込み)。

## 2. 経路(実測と、生成コード・PE のバイトの読み)

1. 戻り番地 0xb2c1e28 は `lift/out-app/gen/src/m7-04.c` の `L_b2c1e23: call f_b2c1600`。`f_b2c1600` は `fle_dispatch(fle_ld(0xb4afc40))`、つまり `jmp [0xb4afc40]`。
2. `0xb4afc40` は `L_b2c1690`(m7-03.c:101768)が `R[1]` で書く。その直前は `f_b284520`(Elastique の登録関数)の末尾で、`lea R[1], 0xb2844c0`(L_b28456d)と `cmove` で選ばれた値。
3. 0xb2844c0 は `f_b284460` の末尾の、関数の後ろの padding(Nop、0xb2844af〜0xb2844bf)の次にある thunk。生成コードでは `L_b2844c0: fle_dispatch(fle_ld(0xb4b30b0, 8))` という、`f_b284460` の中のブロック。**関数の入口ではない**ので、`fle_dispatch` の表に入らず、間接呼び出しで `fle_dispatch_miss` になる。
4. Elastique の DLL のファイル(`lift/out-app/dll/elastique_x64.b000000.dll`)で、RVA 0x2844c0 のバイトは `48 ff 25 e9 eb 22 00`(REX.W 付きの `jmp qword [rip+0x22ebe9]`)。移動先は 0x2844c0 + 7 + 0x22ebe9 = **0xb4b30b0**。0xb4b30b0 には `f_b284520` が `R[8]` を書いている。
5. 結論(実測と読みから): 停止の直接の理由は「呼び出し先 0xb2844c0 が、関数の入口の表に無い」。その thunk は、`lea` で関数ポインタとして取られ、保存された後に間接的に呼ばれる。lifter は、関数の padding の後ろに続く thunk を、関数の入口として登録していない。

## 3. 直し方(製品の既定の動作。旗・環境変数・`#ifdef` を足さない)

- `lift/runtime/fle_realmod.c`: `fle_realmod_jmp_thunk(va, &target)` を足した。real module(メモリに在る PE の像)の SizeOfImage の中で、実行可能なセクション(IMAGE_SCN_MEM_EXECUTE)の中の `[REX] FF 25 disp32` なら、`target` に slot(`va + 6 + disp32`、REX があれば 1 つ後ろから数える)の値を入れて 1 を返す。`rm_t` に `size`(SizeOfImage)を足した。
- `lift/runtime/fle_win.c`: `fle_win_try_thunk` は、まず `fle_realmod_jmp_thunk` を聞く。取れたら、その `target` を `fle_dispatch` する。取れなければ、従来どおり VirtualAlloc の領域の判定(`thunk_target`)へ行く。`fle_dispatch(` の呼び出しは、この関数の中で 1 箇所のまま(`tests/window.rs` が数える)。
- 理由: 直した命令は、lifter が既に生成した `L_b2844c0`(`fle_dispatch(fle_ld(0xb4b30b0, 8))`)と同じ意味をもつ。生成コードは変えない(`fle_dispatch.h` も変えない)。lifter の関数の入口の集め方(address-taken な thunk を入口にしない)は、まだ直していない。

## 4. 修正の後の実測(この作業の最後の受け入れの実行。runner の値)

- 起動確認(`tools/boot-check/boot-bundle.mjs`): engine-idle、`standIn` false、`error` null、`elapsedMs` 8228(上限 12500)。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`): 4 行とも `error` null、engine-idle、`standIn` false、`trapped` false、`hostErrors` 0、`opened` true、`welcomeClosed` true(elapsedMs 7486〜7674)。
- `node tools/e2e/run.mjs --scenarios smoke --only file-open-demo --displays 4k-dpr1,4k-random`(3 行。DPR 1、2.5、2.46): 3 行とも `ok` true、`engineState` engine-idle、`engineStops` 0、`engineStop` null、`stuckToasts` 0、`pageErrors`・`consoleErrors`・`failedRequests` 0。`dumpRetries` は 8・5・46、`engineRunningMaxMs` は 6250・8732・8749。
- `node tools/fe-lift-test.mjs`: 通った 2096 件、既知の失敗 53 件、新しい失敗 0 件(修正の前の同じ道具は、新しい失敗 14 件・終了 1。§8)。
- `npm test --prefix app`: 74 files passed(1 skipped)、1439 tests passed(10 skipped)。
- 停止の JSON の path(`lib/stop.mjs` の `stopOf`)は、停止の報告の行を入れた合成のログで確かめた(`stop`・`abortKind`・`returnAddress`・`registers`・`report` が出る)。実際の停止は、この修正の後は起きないので、E2E の JSON では確かめていない。

## 5. 経路の違い(確かめたこと)

- 現在の `tools/p3-audio-probe/measure.mjs` の `openProject` も、E2E と同じ経路を使う: Ctrl+O で開く箱(`#filedlg-input` または `#fx-input-files`)へ `DOM.setFileInputFiles` で渡し、`Open` を押す。Issue の「VFS へ直接置く経路」の記述は、この版には当てはまらない。
- 停止は、ファイルを開いた後、プロジェクトの読み込みの途中(Elastique の real module の読み込みの直後)で起きた。E2E の JSON の通知(`missing-path` の行)は、プロジェクトの素材の探しを示す。
- 未確認: 修正の前に、ワーカーの `openProject`(VFS へ直接)の経路で DEDZII が止まるかは、この作業では実行していない。修正後は、E2E の経路で開くことを確かめた。

## 6. 記録の欠陥(この Issue では直していない。次の Issue の候補)

- **stdout と stderr の並び**: `app/src/engine/worker.ts` の `flushLogs` は、1 回の送信で `out` の行を先に、`err` の行を後に送る。そのため、停止の報告(stdout)の後に、それより前の stderr の行(`fle:`・`missing-path`・`enum-dir`)が並ぶ。修正の前の停止では、報告の後に約 6,600 行の stderr が続き、`fle:` の行は最後に来た。
- **停止の事実の窓**: `app/src/perf/exit-message.ts` の `engineStopFacts`・`engineExitMessage` は末尾 2,000 行だけを見る。報告の後ろに 2,000 行を超える stderr が続くと、`== stop:` の行が窓の外になり、`engine.exit.stop` が空になる。推定: 実機の記録の、停止の項目が無いセッション(`pm-work/findings/real-user-stops-2026-11-25.md`、版 e29a730b932d と d7a6e6f392f2)の原因の候補。確かめていない。
- 直し方の案: (a) worker.ts で out と err を 1 つの順序付きの列にして送る。(b) `engineStopFacts` の窓を、`== stop:` の行から後ろに取る。どちらも `app/`(製品)の変更で、deploy が動く。

## 7. lifter の入口の集め方(次の検討)

- 同じ型の停止(address-taken な thunk が、関数の入口の表に無い)は、Issue #1403 の Mehran の曲の報告(間接呼び出しの飛び先の判定)にも、型として近い。推定。確かめていない。
- 根本の直し方は lifter(`lift/crates/fe-lift`)で、`lea`・`mov imm` の、実行可能な像の番地を入口にすること。影響が大きいので、この Issue では、実行時の shim(`fle_realmod_jmp_thunk`)で直した。

## 8. この作業で直した試験の link(fe-lift の deferred)

- 修正の前、`node tools/fe-lift-test.mjs` は「新しい失敗 14 件」で終了 1 だった。14 件は全て `deferred` の binary で、link の `multiple definition of fle_clip_host_write`(と `fle_clip_host_read`)。
- 原因: c2689869(#1418)で `deferred.rs` の `runtime_objs()` が `tests/support/clip_host_stub.c` を全ての driver に link するようになった。`deferred.rs` の driver の C ソース(#1389 の定義)も同じ 2 つの関数を定義していて、二重になっていた。
- 直し方: driver の C ソースの 2 つの定義を消した(stub が同じ関数を持つ。`build_driver` は必ず `runtime_objs()` を link する)。`--test deferred` で 16 件通ることを確かめ、続けて全体を回した。

## 9. 残り(この作業では変えていない)

- 印 `expectFail: '#1421'`(`scenarios/smoke/file-open-demo.mjs`)は残した。`spec/workflow.test.sh` の (x) が印の存在を求め、`judge.jq` は通ったときに「印を外す」と警告する。外すなら、spec と README を合わせて直す(門の規則を変える判断は、監査・受付で)。
- `lift/crates/fe-lift/tests/known-failures.txt` の 6 件(今は通る: adjustor 1・authenticode_check 4・vcall_thunk_dll 1)は消していない。`spec/fe-lift-test.test.mjs` が件数 59 を求めるため。vcall_thunk_dll の 1 件は、この修正(import thunk の shim)と合う(推定)。authenticode_check の 4 件は、この修正とは関係が無いと見える(入力の有無の差と推定。確かめていない)。
