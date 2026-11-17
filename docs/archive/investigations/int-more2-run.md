# 整数・システム命令の追加後の実行（896 seed の閉包）

日付: 2026-10-20。決定は `docs/decisions/0023-int-more2.md`。出力はすべて `lift/out-int2/`（`tmp/` は TMPDIR）。前回の記録は `docs/investigations/null0-run.md`（停止点 0x407b85、`unsupported_total` 541）。

## 1. 対象と実装

- 対象（`remaining-unsupported.md` §4 の整数・システム）: Bsf、Bsr、Bts、Btr、Btc、Rol、Ror、Xchg、Pause、Jecxz/Jrcxz、Xgetbv、Sfence/Lfence/Mfence、Cmpxchg16b。
- 前回の生成 C（`out-null0/out/after-null0-896.c`）で、これらのうち `fle_unsupported` になっていた行は 40 件（Bsf 5、Btr 2、Bts 5、Cmpxchg16b 2、Jecxz 2、Pause 3、Rol 4、Ror 3、Sfence 2、Xchg 11、Xgetbv 1。`out-int2/targets.txt`）。Bsr、Btc、Lfence、Mfence、Jrcxz は閉包に 0 件だが、同じ作業で実装した。
- 既に実装済みで重複したものはない。`insn_int.rs`、`int_ext.rs`、`int_more.rs`、`lib.rs` の `match` に、対象の mnemonic は 1 つもなかった（grep で確認）。
- 新規: `lift/crates/fe-lift/src/int_more2.rs`、`lift/crates/fe-lift/tests/int_more2.rs`、`docs/decisions/0023-int-more2.md`、この文書。
- `lib.rs`: `mod int_more2;` 1 行と、`lift_insn` の先頭に `if let` 4 行（`git diff --stat` で +5 行）。先頭に置いた理由は決定 0023 に書いた（jecxz、jrcxz が `ConditionalBranch` なので）。編集の直前に `lib.rs` を読み直した。この作業中の `lib.rs` の差分は私の 5 行だけ（`git status` で `lib.rs` が `M`、`git diff` で +5）。`imports_table.rs` は触っていない（md5 が作業の前後で `2464403238a42a29469feffce8c7bbc5` のまま）。
- runtime に新規ファイルはない。生成 C は既存の `R`、`FL`、`fle_ld`、`fle_st`、`fle_sext`、`__atomic_thread_fence` を使う。

## 2. 仕様

- **Intel SDM Vol. 2（Order Number 325383-093US、2574 ページ）の公式 PDF を取得できた**: https://cdrdv2.intel.com/v1/dl/getContent/671110 （HTTP 200、11,294,447 バイト）。`lift/out-int2/spec/sdm-vol2.pdf` と、`pdftotext -layout` の `sdm-vol2.txt`。各命令の節を読んだ（節とページは決定 0023 に書いた）。図と画像は見ていない。
- SDM が決めない／食い違う点は、ホスト（AMD Ryzen 9 9950X）で実測した。実測の結果と、それをどう使ったかは決定 0023 に書いた。要点:
  - BSF/BSR の PF（ソースが 0 でないとき）は、ホストでは「宛先に書いたインデックスの下位 8 ビットの偶数パリティ」と全件一致し、SDM の文面（ソースの 1 の数の偶数パリティ）とは約半分しか一致しない。仕様を確認できない値なので、**PF を `FLE_UNDEF` にした**（読めば abort する）。
  - XGETBV は XCR0 の値を SDM が決めないので、**実装せず未対応のまま**にした。

## 3. テスト

- `cargo test --offline -p fe-lift --test int_more2`: 10 件、すべて通過（`test-int_more2-2.txt`、EXIT=0）。内訳と規模（ベクタ、行、比較の数）は決定 0023。ホスト突合の行数は合計 122,920 行（bsf/bsr 14,400、bts/btr/btc 57,240、rol/ror 39,680、xchg 8,000、pause と fence 1,600、cmpxchg16b 1,600）に jecxz/jrcxz の rcx 値 210 通り×2 命令。
- 1 回目のテスト実行（`test-int_more2-1.txt`）は 10 件通過。その後、私は BSF/BSR のソース 0 の PF を 1 から 0 に書き換える誤った編集をして、`bsf_bsr_match_host` が失敗した（ホストは PF = 1。`probe3-out.txt`）。その編集を元の値（PF = 1）に戻して再実行し、10 件通過（`test-int_more2-2.txt`）。誤った編集の失敗ログのファイルは残していない（その場の出力のみ）。生成と実行（§4、§5）は、戻した後のコードで行った。
- `cargo test --offline --workspace --no-fail-fast`（`test-ws.txt`、`test-ws.exit` は EXIT=0）: **42 バイナリ、266 件 passed、0 failed、0 ignored**（`test result` 行の集計）。前回（null0-run.md）の 256 件に対し +10 は、今回の `int_more2` の 10 件と一致する（他の担当のテストの増減は確認していない）。

## 4. 生成、clang、リンク

- 生成器: `out-int2/tool`（`out-null0/tool` の複製）を `out-int2/target-tool` にビルド（`build-tool.txt`、EXIT=0）。seed は `out-int2/seeds-896.txt`（896 件、`out-null0` の複製）。
- 生成: EXIT=0（`gen-int2-896.txt`、`.err` は空）。`out/after-int2-896.c`（42,536,508 バイト）。functions 8427、closure_instructions 497367、emitted 497527（前回と同じ）。c_lines 548442（前回と同じ）。
- **`unsupported_total`: 541 → 502**（−39）。`unsupported` の種類は 49 → 39（`gen-int2-896.txt`）。
  - 減ったのは、Bsf 5、Btr 2、Bts 5、Cmpxchg16b 2、Jecxz 2、Pause 3、Rol 4、Ror 3、Sfence 2、Xchg 11 の計 39 件。
  - 残る対象は **Xgetbv 1 件**（`Xgetbv: mnemonic outside minimal set`）。
  - 他の主な内訳: `Jmp: indirect through IAT slot (not lifted)` 304、`Jmp: indirect target unresolved` 20（どちらも前回と同数）。
- clang -O0 とリンク: `out-int2/build.sh`（`out-null0/build.sh` の複製）、`EXTRA_RT="fle_int_more fle_sse_more fle_locale fle_sys fle_x87ctl fle_vfs fle_reg fle_env fle_uilang fle_proc fle_time fle_uipref"`（null0-run.md §7 と同じ）、EXIT=0（`build-896.txt`）。`bin/flengine-after-int2-896`。

## 5. 実行

コマンド: `./bin/flengine-after-int2-896 ../../binaries/win64/FLEngine_x64.dll`（`out-int2/` で、TMPDIR は `out-int2/tmp`）。出力 `run-int2-896.txt`、`.err`、`.exit`（EXIT=134、SIGABRT）。

| 項目 | 前回（null0-run.md） | 今回 |
|---|---|---|
| 停止点 | 0x407b85（f_407a50 内、`bsf eax, esi`） | **0x407030**（f_407030） |
| abort の種類 | fle_unsupported | fle_unsupported（`fle: unsupported instruction at 0x407030`） |
| 関数実行数（distinct） | 112 | 116 |
| 関数エントリ数（calls） | 208 | 340 |
| 呼び出し深さ | 18 | 9 |

- 0x407b85 は通過した。根拠: f_407a50 のエントリがログに複数回ある（entry 97、108、120、151、157、177、187、207、221、254 など。ログは先頭 256 件）。ログの `f_409c80`（AVX 版を持つコピー関数）も複数回入っている。実行がこれらを戻ったかは、後続のエントリが続くこと（entry 340 まで）から読める（推測ではなく、停止が別の VA であることによる）。
- 前回の記録で予想した `bts`、`rol`、`pause`、`xchg` の位置に到達したかは、この実行で個別には確認していない（到達した命令の追跡ログは取っていない）。

## 6. 次の停止点

- VA **0x407030**、関数 f_407030（呼び出し元 0x41c88a、戻り先 0x41c88f、呼び出し元の関数は f_41c820）。スタック（実行ログ）: `#0 0x407030`、`#1 0x41c820`、`#2 0x4123a0`、`#3 0x4123c0`、`#4 0x42d870`、`#5 0x4120c0`、`#6 0x412160`、`#7 0x41d8f0`、`#8 0x1376820`（エントリ）。
- 命令: `jmp QWORD PTR [rip+0x1569c42]`（ff 25 42 9c 56 01、`objdump-text.txt` 9876 行）、IAT スロット 0x1970c78。
- IAT の名前: **`user32.dll!LoadStringW`**（`out-imports/imports.txt` の 87 行）。
- 未対応の種類: x86 命令ではなく、`Jmp: indirect through IAT slot (not lifted)`（`imports_table.rs` に LoadStringW の行がない）。生成 C の該当行は `L_407030: { /* 0x407030 Jmp */ fle_unsupported(UINT64_C(0x407030)); }`。
- 呼び出し点の引数（0x41c875〜0x41c88a の読み）: `rcx = [[rsi]]` を 0x418be0 に渡した結果、`edx = [rsi+8]`、`r8 = rbp+0x20`、`r9d = 0x1000`。LoadStringW(hInstance, uID, lpBuffer, cchBufferMax) の形に見える（読みと API の引数の並びからの推測）。
- `remaining-unsupported.md` §5.1 の A2（リソース: LoadStringW を含む）に当たる。今回は実装していない（依頼の範囲外）。

## 7. 残り

- 依頼の対象のうち、Xgetbv は実装していない（理由は決定 0023）。0x639a03 の位置が実行に出るかは未測定。
- 未対応の命令（SSE、MMX、AVX、`Lea`/`Mov` の FS/GS）は今回の対象外で、件数は前回と同じ。直前の生成（`out-null0/gen-null0-896.txt`）と今回（`gen-int2-896.txt`）の「unsupported」の一覧を `sort` して `diff` した結果（`u-null0.txt`、`u-int2.txt`）、違いは §4 の 39 件の減少と、それに伴う `unsupported`（種類）49 → 39、`unsupported_total` 541 → 502 だけだった。`remaining-unsupported.md`（615 件の時点）の SSE の件数と今回の件数が違う箇所（例: Psllw 12 と 14）は、生成 C の時点が違うためで、今回の作業の影響ではない（前者は `out-x87ctl` の C、今回は `out-null0` 以降の C。差の理由は調べていない）。
- Intel の CPU での挙動の確認、BSF/BSR の PF の仕様の食い違いの原因は未確認。

## 8. 失敗・拒否された操作

- 拒否はなし。
- 失敗: BSF/BSR のソース 0 の PF を 0 に書き換える誤った編集（§3）。ホストの実測と矛盾していたことをテストが検出し、戻した。
- 外部への通信は、SDM の PDF の取得（`cdrdv2.intel.com`）の 1 回のみ。
