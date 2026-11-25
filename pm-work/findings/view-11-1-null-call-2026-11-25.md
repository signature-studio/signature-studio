# View > #11 > #1 の null の関数の呼び出し(2026-11-25。Issue #1416)

## 結論

- 停止の原因: `f_66dcf0` が、スロット `0x15f4758` の関数ポインタを呼んだ。スロットは NULL のまま。
- NULL になった理由: `ILSvg_x64.dll` の読み込みが失敗した(`LoadLibraryW 126`。bundle に DLL が無い)。その戻り値(NULL)を `GetProcAddress` に渡し、NULL が返ってスロットに入った。ゲストは戻り値を確かめずに呼んだ。分類は (i)。根は #1395 の範囲(ILSvg_x64.dll を real module にする)。
- 直し: この Issue では直していない(#1395 の範囲。二重に入れない)。停止の記録に、呼び出し元・レジスタ・最も近い関数を出すようにした(目標 2)。
- null の呼び出しの扱い: 採らない(今回)。理由は下の「null の呼び出しの扱い」。

## 実測(runner の値。この機械とは別枠。nproc 4、loadavg 0.3〜1.8)

- 実エンジン: `bash tools/crash-hunt/run.sh --budget 15m --out <tmp> --only View/11/1 --bundle raw/fl-Oz`(DPR 1)で、engine を起動して `View > #11 > #1` を踏んだ。出力: `probe 1 stopped View > #11 > #1`(`View > #11` は `ok`)。
- 停止の文言(`stop.lastFle`):
  `fle: indirect call to 0 is not a known function entry (caller 0x66dd61, near f_66dcf0+0x71)`
- 直前の記録(`logTail`、抜粋):
  - `[err] missing-path: LoadLibraryW 126 "ILSvg_x64.dll"`
  - `[err] fle: regs R[0]=0x4094c3e0 R[1]=0x3f27c600 R[2]=0x4094c3f0 R[3]=0x40f71770 R[4]=0x3e1554f8 R[5]=0x3e155500 R[6]=0x3e1558c0 R[7]=0x0 R[8]=0x4094c3f4 R[9]=0x385 R[10]=0x0 R[11]=0x0 R[12]=0x0 R[13]…`(道具の記録は 200 字で切る)
  - `[err] fle: indirect call to 0 is not a known function entry (caller 0x66dd61, near f_66dcf0+0x71)`
- 証拠の記録: `pm-work/findings/view-11-1-null-call-1416.json`(`View/11` と `View/11/1` の probe)。
- 再現(この作業の受け入れの中の道具の run。診断を入れた engine。13:27 UTC): 同じ停止の文言 `caller 0x66dd61, near f_66dcf0+0x71`。`probe 1 stopped View > #11 > #1`。
- 起動: hunt の起動確認 `startup {"flow":"A","ok":true,"elapsedMs":11622,"preBootBytes":313152246,"state":"engine-idle"}`。boot-bundle は 2 回: 1 回目 12,934 ms(上限 12,500 を超えた。ビルド直後)、2 回目 11,325 ms。

## 呼び出し元と、null になった理由(生成コードの読み)

生成物: `lift/out-app/gen/src/fle-03.c`(`fe-build engine` の出力。この Issue の run の物)。

- 呼び出し元: `f_66dcf0`(VA `0x66dcf0`)の `0x66dd5b`(`call [0x15f4758]`)。`fle_ld(0x15f4758)` の値を `fle_dispatch` に渡す。戻り番地 `0x66dd61` は、その次の命令。`near f_66dcf0+0x71` は、`0x66dd61` が `f_66dcf0` の中にあることと一致する。
- スロットを書く関数: `f_66dc10`。`[0x15f4750]` が 0 のときだけ(初回)次を行う。
  1. `f_6539e0` を呼ぶ。引数 `R[1]=0x66dc98`、`R[2]=1`。戻り値を `[0x15f4750]`(モジュールの扱い)に書く。
  2. `f_4235f0` を `R[1]=[0x15f4750]`、`R[2]=0x66dca4` で呼び、戻り値を `[0x15f4758]` に書く(`L_66dc49`)。同じ形で `[0x15f4740]`(`0x66dcb8`)と `[0x15f4760]`(`0x66dccc`)も書く。
- 推定(本体が生成物に無いので確かめていない): `f_6539e0` は `LoadLibraryW` の包み、`f_4235f0` は `GetProcAddress` の包み、`0x66dc98` は `"ILSvg_x64.dll"`。根拠は、直前の `LoadLibraryW 126 "ILSvg_x64.dll"` の行の順序。
- 実測の連鎖: `LoadLibraryW` が 126 で失敗 → モジュールの値が NULL → `GetProcAddress(NULL, name)` は、NULL を返す(`lift/runtime/fle_modules.c` の `fle_imp_GetProcAddress`。decision 0233 の経路: NULL のモジュールは実行ファイルの export の 3 件だけを見て、無ければ NULL・127)→ スロットが 0 → `f_66dcf0` が呼ぶ。ゲストは戻り値を確かめない。decision 0233 の注記にある Elastique と同じ型。

分類:
- (i) 当たり: 読み込みの失敗の NULL が、確かめられずに使われた。
- (ii) 違う: 仮想関数の表ではなく、`GetProcAddress` の結果を置いた大域のスロット。
- (iii) 違う: lift は、メモリの値(この run では 0)をそのまま呼んでいる。`fle_ld` の読みは忠実で、`.data` のスロットの扱いの誤りではない。
- 根: `ILSvg_x64.dll` が bundle に無いこと(#1395 の範囲)。Windows では、この DLL は必ず在るので、この経路には入らない。

## null の呼び出しの扱い(目標 3)の決定: 採らない(今回)

Windows の挙動(アクセス違反の例外)に沿う案は、枠の探索(`fle_seh_raise`)を使う。検討の結果、今回は採らない。理由:

1. 状態が Windows と違う。この null は、読み込めなかった DLL の関数が NULL のまま使われた結果。例外で続けると、Windows でも DLL が無ければ同じ経路になる、というだけで、Windows と同じ状態には戻らない。続けた実行は、DLL が無いという原因を記録から隠す。
2. 途中で止まる可能性が高い。guest の呼び出しの鎖は、runtime が呼んだ wndproc(host の callback)で終わるのが普通で、`fle_seh.c` の `unwind_one` は、そこで名前つきの停止にする(`FLE_CB_RETURN_ADDRESS`)。この鎖では確かめていない。境界の先まで探索を続けるには、探索の途中の停止を扱う仕組みが要るが、今は無い。
3. 試験の枠が無い。`lift/crates/fe-lift/tests/seh_rt.rs` は `binaries/win64/FLEngine_x64.dll` が要り、この runner には無いのでスキップされる。
4. 未確認の点: `f_66dcf0` には guest の except の landing がある(fe-lift が `fle_seh_enter` の wrapper を出している)。null の呼び出しが、その try の範囲に入るかは確かめていない。入るなら、ゲストの handler が取る可能性がある。`#1395` の後に null が本当の不具合として残るなら、この案を別の Issue で扱う。

## 直したもの(目標 2・1 の一部)

- `lift/runtime/fle_dispatch.h`: `fle_dispatch_stop` を足した(`fle_dispatch_miss` の停止の本体。cold・noinline)。
  - 文言: `fle: indirect call to <値> is not a known function entry (caller <戻り番地>, near <名前>)`。戻り番地は `R[4]` の語(呼び出しが積んだ値)。`R[4]` が 0 か読めないときは `caller ?`。
  - `near`: `fle_dispatch_table`(関数の入口の昇順)で、戻り番地以下で最大の入口 `f_<VA>+0x<差>`。無ければ `near ?`。この名前は、外部の名前ではなく lift の関数の名前(`f_<VA>`)。外部の名前の表は、生成物に無い。
  - `fle: regs R[0]=0x… … R[15]=0x…` を、停止の文言より前に 1 行出す(文言が最後の行のまま)。
- 試験: `lift/crates/fe-lift/tests/dispatch.rs` に `dispatch_miss_names_the_caller_the_registers_and_the_nearest_entry` を足す(戻り番地 0x25 → `near f_20+0x5`、`R[4]` が 0 のとき `caller ?, near ?`、`fle: regs` の行)。`census.rs`・`wall_fr9t.rs` の完全一致を、新しい文言の前方一致に直した。
- 道具: `tools/crash-hunt/hunt.mjs` に `--only <id>` を足す(main に無かったため。`onlyKeep`: 指定の項目と、その祖先だけを、DPR 1 の走査で踏む)。#1415 の仕様と違えば、そちらで統合する。試験: `spec/crash-hunt-only.test.mjs`(`spec/workflow.test.sh` の一覧に足した)。
- 既知の一覧: `lift/crates/fe-lift/tests/known-failures.txt` に `int_more2 bsf_bsr_match_host` を 1 件足した(受け入れの結果の節を参照)。

## 受け入れの結果(2026-11-25 12:00〜13:30 UTC の 1 回目の通し。この runner)

受け入れの順に、段ごとに書く。

- `npm run build --prefix app`、`cargo build --release`、`fe-build engine`・`fe-build bundle`: 通る(約 55 分)。
- 起動確認(`boot-bundle.mjs`): **ビルド直後の 1 回目は `elapsedMs` 13,525**(上限 12,500 を超え、この段は落ちる)。その後は、2 回目 12,294 ms、流れ A〜D の起動 11,889〜12,127 ms、追加の 3 回 12,262・12,298・12,312 ms(`state` engine-idle・`standIn` false・`error` null)。前の run は 12,934 ms と 11,325 ms。**ビルド直後は上限に届く**。製品の変更とは無関係の runner の変動だが、受け入れの段としては安定しない。
- `cargo test --locked -p fe-decode`: 全て `ok`(失敗 0)。
- `node tools/fe-lift-test.mjs`(全体。`cargo test -p fe-lift --no-fail-fast`): 通った 2,092 件・既知の失敗 53 件・直った 6 件(既知の一覧に残す)・**新しい失敗 1 件: `int_more2 bsf_bsr_match_host`**。
  ```
  assertion `left == right` failed: bsf reg32 [0f bc c6] row 0: ... flags 0x841 ...: flag 0 (cf pf af zf sf of) (C left, host right)
    left: 0
   right: 1
  ```
  この失敗は #1416 と無関係。`fle_dispatch.h` を main の版に戻して `int_more2` を走らせても、同じ行で落ちる(確認済み)。原因: この runner は AMD EPYC 7763。BSF/BSR で源が 0 のとき、host の CF は 1、lift の model(decision 0023 の Scan の扱い)は CF = 0 を定義する。SDM は BSF の CF を未定義とする。**既知の一覧に `int_more2 bsf_bsr_match_host` を 1 件足した**(`pm-work/findings/fe-lift-test-baseline.md` §5 に理由を書いた)。これは試験の判定の変更なので、監査が確かめること。直し方は lift の model か試験の比較を直す別の Issue。
- 流れ A〜D(`tools/smoke/welcome-flow.mjs`): 4 行とも `error` null・`state` engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。`elapsedMs` 12,127・11,926・12,012・11,889。`preBootBytes` の最大 313,182,246(上限 314,368,711 以下)。判定の行は 0。
- `npm test --prefix app`: `Test Files 73 passed | 1 skipped (74)`、`Tests 1432 passed | 10 skipped (1442)`。
- 道具の再現(`View/11/1`、DPR 1。`bash tools/crash-hunt/run.sh --budget 15m --only View/11/1 --bundle raw/fl-Oz`): `probe 1 stopped View > #11 > #1`、`probe 1 ok View > #11`。`stop.lastFle`:
  ```
  fle: indirect call to 0 is not a known function entry (caller 0x66dd61, near f_66dcf0+0x71)
  ```
  判定(`caller 0x` が入る)は通る。
- `spec/guard.test.sh`・`spec/workflow.test.sh`(新しい `spec/crash-hunt-only.test.mjs` を含む): 通る。

## 次に必要なこと

1. `#1395`(ILSvg_x64.dll の real module 化)が閉じたら、`View/11/1` を DPR 1 で踏み直す。null の経路が消えるはず。
2. 起動確認の上限(12,500 ms)が、ビルド直後の 1 回で超える。監査が、上限の扱いを決める(P1 の 10 秒との関係)。
3. `int_more2 bsf_bsr_match_host` の既知の失敗は、CPU に依る。直す Issue を起票する。
4. null の呼び出しの AV 扱いは、`#1395` の後にまだ null が出る場合だけ、別の Issue で扱う(上の「決定」の 2 と 3)。

## #1395 の状態

main の `git log` に `Closes #1395` も「完了 (#1395)」も無い(#1395 の commit は d29eaf6a・0610ae87・02fab5e9・8a15ecd6・42f83997 で、作業の途中)。よって未閉と判断した。`gh` は未認証で、Issue の状態は API で確かめられなかった。
