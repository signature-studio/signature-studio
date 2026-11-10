# adjustor thunk の seed 追加と harness の再実行の記録（決定 0051）

実行日: 2026-10-21。作業ディレクトリ `signature-studio-develop`、HEAD 7ea4b34、ワークツリーに本作業の変更（`git status` は本作業のファイルと既存の未追跡 `lift/iat-unsup-closure.txt` だけ）。DLL は `binaries/win64/FLEngine_x64.dll`（22,227,016 B、md5 3d7f97c3a1f9cabfa2ebb4b049146141）。調査は `adjustor-thunks.md`、決定は `docs/decisions/0051-adjustor-thunks.md`。「実測」はコマンドの出力、「読み」は静的な読み、「推測」は未検証。成果物は `lift/out-adj/`（`TMPDIR=lift/out-adj/tmp`、`EM_CACHE=lift/out-adj/emcache`、`CARGO_TARGET_DIR=lift/out-adj/target`）。

## 1. 結果

| 項目 | 実測 |
|---|---|
| 0x403582 は `.pdata` に無いか | 無い。`.pdata` の最小の開始は 0x407380。0x403579、0x403582、0x40358b はどの範囲にも入らない |
| 同形 thunk（`.pdata` 外、DIR64 で参照、`add/sub/lea/mov` + 直接 `jmp`、jmp 先が `.pdata` 開始） | **602 件**（すべて `add rcx, imm; jmp rel32`。9 B が 467、12 B が 135。jmp 先 distinct 275） |
| `sub`／`lea`／`mov` 形の thunk（`.pdata` 外、参照あり） | 0 件 |
| 範囲の内側にある同形（seed にしない） | 2 件（0x40c6fc、0x882586） |
| 連続スロットの並び（「連」） | 90 連（3 要素の連は 13。0x403598 の連はその 1 つ） |
| roots（seed）の増分 | **+602**（57,816 → 58,418） |
| 閉包の関数数 | 58,695 → **59,297**（+602。増えた関数は thunk そのもの 602 だけ。jmp 先はすべて既に関数） |
| 閉包の命令数 | 2,526,826 → **2,528,030**（+1,204 = 602 × 2） |
| そこから新しく到達した関数（thunk の先） | **0**（jmp 先 275 通りはすべて `.pdata` の開始で、既に閉包にあった） |
| 誤デコード | `decode_failures` 0、`out_of_text_flow` 0、`out_of_text_calls` 0、生成の `bad_nodes` **0**（前も 0） |
| 生成の `unsupported_total` | **554**（前も 554。変化なし） |
| 生成の `tail_calls` | 8 → 610（+602）。`jmp` の件数 49,443 → 50,045、`add` 68,207 → 68,809。ほかの `handled` は同じ |
| 生成 C | `functions 59297`、`c_bytes 215,696,123`（前 215,540,788）、分割 17 本 |
| 0x403579、0x403582、0x40358b | seed に入る。生成 C に `void f_403582(void)`（`add` と `f_41c1a0(); return;`）があり、`fle_dispatch_table` に 3 つとも入る |
| ワークスペース全テスト | **70 件の `test result` 行、656 passed、0 failed、0 ignored**（`lift/out-adj/test-ws.txt`。直前の記録は 67 行、645 件。差 +11 は本作業の新規 6 + 3 + 2） |
| harness の停止点（native） | `fle_dispatch_miss`（0x403582）→ **`fle: VirtualAlloc flAllocationType 0x101000 is not supported`**（`abort kind: (not a runtime message)`） |
| 関数実行数（distinct） | 284 → **357** |
| 入口数（`function entries (calls)`） | 5,916 → **6,221** |
| 停止時の呼び出し深さ | 6 → **10** |
| wasm（node）の停止点 | native と**同じ**（`diff <(grep -v '^calling' run-native.txt) <(grep -v '^calling' run-node-base.txt)` が差なし。`calling … rsp=` の 1 行だけ違う: native `0x72cd8f5fffc0`、wasm `0x11c4e500`） |

「前」の数値（284 / 5,916 / 深さ 6、unsupported_total 554、58,695）は `event-sync-run.md`。今回の生成 C は同じ HEAD から作り直したもので、前後の差は thunk の seed だけ（`--no-adjustors` の生成は 58,695 関数、2,526,826 命令、`bad_nodes` 0、`unsupported_total` 554、`c_bytes` 215,540,788 で、前の記録と一致）。

## 2. 実装

### 2.1 ファイル

- 変更: `lift/crates/fe-pe/src/lib.rs`（`dir::BASERELOC` と `Pe::dir64_relocation_vas` を末尾に追加）、`lift/crates/fe-decode/src/lib.rs`（`pub mod adjustor`、`pdata_and_adjustor_seeds`、`analyze_with_pdata_adjustors`）、`lift/crates/fe-decode/src/main.rs`（`closure --pdata --adjustors`）、`lift/crates/fe-lift/src/main.rs`（`--pdata` に thunk を含める、`--no-adjustors`）。
- 新規: `lift/crates/fe-decode/src/adjustor.rs`、`lift/crates/fe-decode/tests/adjustor.rs`（6 件）、`lift/crates/fe-pe/tests/reloc.rs`（3 件）、`lift/crates/fe-lift/tests/adjustor.rs`（2 件）、`docs/decisions/0051-adjustor-thunks.md`、`docs/investigations/adjustor-thunks.md`、本書、`lift/out-adj/`。
- 既存のテスト・関数の挙動は変えていない（`pdata_seeds`、`analyze_with_pdata`、`analyze`。`fe-lift lift` の `--pdata` なしも同じ）。**`fe-lift lift --pdata` の出力は変わった**（thunk 602 本が増える）。従来の出力が要るときは `--no-adjustors`。

### 2.2 判定（`adjustor::adjustors`）

DIR64 スロットの値（`.text` 内）を集め、`.pdata` の開始または範囲の内側のものを除き、残りを iced-x86 でデコードして、補正命令 1〜3 個 + 直接 near `jmp`（全体 16 B 以下）で、jmp 先が `.text` 内の `.pdata` 開始であるものだけを採る。補正命令: `add`/`sub` の 32/64 bit レジスタ + 即値、`lea r64,[base+disp]`、`mov r64,[base+disp]`（index なし、rip 相対でない、セグメントなし）。補正なしの `jmp` だけ、`jmp` が間接、補正のあとに ret/call があるもの、無効バイト、補正命令が 4 個以上、16 B を超えるものは採らない。

## 3. 検証

- 独立な測定との一致: Python（pefile + capstone。`survey2.py`）は 602 件、Rust の `adjustors()` も 602 件。602 件の VA 集合の一致そのものは比べていない（両方で先頭 0x403579、末尾 0x136bf04 と件数が一致。テストが先頭と末尾と全 602 件の形を検査している）。
- `fe-decode/tests/adjustor.rs`（6 件、うち 4 件は DLL）: 手作りバイトで形（add、sub、lea、mov+add、拒否 9 通り + 4 命令 + 16 B 超 + 範囲外）、DLL で 0x403579/0x403582/0x40358b が seed で `add+jmp`、長さ 9、jmp 先 0x41c180/0x41c1a0/0x41c1b0、`.pdata` の seed に無いこと、602 件が昇順で全部 `add+jmp`、jmp 先が `.pdata` 開始、閉包の増分（関数 +602、命令 +1,204、新関数はちょうど thunk、旧命令を失わない、`decode_failures` 0）。
- `fe-pe/tests/reloc.rs`（3 件）: 手作りの reloc ブロック（DIR64 だけ、ABSOLUTE を除く、不正なブロックは Malformed）、DLL で 278,531 件。
- `fe-lift/tests/adjustor.rs`（2 件）: `generate_with_seeds` で `.pdata` のみと thunk 付きを比べ、関数 +602、`bad_nodes` 0、`unsupported_total` 同じ、命令 +1,204、`tail_calls` +602、`gotos` 同じ、3 つの thunk の関数と dispatch 表の行。
- 3 つのテストファイルは `FLENGINE_PE=/home/ubuntu/work/signature-studio-develop/binaries/win64/FLEngine_x64.dll` で実行（`FLENGINE_PE` が無いときは既定の DLL で、無ければスキップ）。
- 変異テストはしていない。

実装中に 1 件、テストの期待が誤っていた: 0x403583（0x403582 の REX の次のバイト）は `add ecx,-0x10; jmp` と読めて thunk の形に合う（`classify` は Ok を返す）。形だけでは命令境界は決まらず、境界を決めるのは「DIR64 スロットが 0x403583 を持たない」こと（seed に入らない）。テストはその事実を検査するよう直した。

## 4. harness の再実行

手順は `event-sync-run.md` §4 と同じ。

1. 生成: `fe-lift lift FLEngine_x64.dll out/wasm-pdata.c --pdata`（`harness/wasm/build.sh gen split`、`OUT=lift/out-adj`）。`functions 59297`、`bad_nodes 0`、`unsupported_total 554`。`out/` は約 431 MB、分割 17 本（`split_c.py`）。
2. native: `OUT=lift/out-adj NATIVE_TRACE=lift/out-pdata/trace.c bash lift/harness/wasm/build_native.sh` → `lift/out-adj/native/flengine-native`（258,061,736 B）。実行は `lift/out-adj/run-native.txt`、`run-native.err`。トレースは `out-pdata/trace.c`（`SEEN_SIZE` 262144。build_native.sh の既定は `out-loc/trace.c`。`cmp` で 2 つが同一であることを確認した）。
3. wasm: `OUT=lift/out-adj bash lift/harness/wasm/build.sh rt gen-c harness link`（同時 2 本、`JOBS=2`）。runtime 33 本、生成 C 17 本、harness 3 本が `exit=0 errors=0`（`fle_proc.c` の 1 件の警告 `fle_thread_prio set but not used` は既存）。link `exit=0`、`flengine.wasm` 115,862,038 B。実行: `node bin-base/run_node.cjs binaries/win64/FLEngine_x64.dll`（`run-node-base.txt`）。
4. 比較は §1 のとおり差なし。

## 5. 次の停止点

- `fle: VirtualAlloc flAllocationType 0x101000 is not supported`（`runtime/fle_sys.c:594`、`STOP`）。API の名前は VirtualAlloc。引数の値は 0x101000。
- 呼び出しスタック（native と wasm で同じ）: `f_4073c0` ← `f_407770`（guest-return 0x4077a0）← `f_407a50` ← `f_409410` ← `f_545b30` ← `f_545d10` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← エントリ `f_1376820`。`f_4073c0` は `jmp [IAT VirtualAlloc]` の thunk（読み。0x4073c0 の命令は `jmp qword ptr [rip+0x156a20a]`、IAT スロット 0x19715d0 が kernel32 の VirtualAlloc）。
- 読み: `f_407770` は 0x40777c〜0x40779b で `rcx=NULL`、`rdx=(size+0x10027) & ~0xffff`、`r8d=0x101000`、`r9d=4`（PAGE_READWRITE）として VirtualAlloc を呼ぶ。`0x101000` は `MEM_COMMIT (0x1000) | MEM_TOP_DOWN (0x100000)`（Microsoft Learn の VirtualAlloc の値表による読み。表を今回は取得していない）。runtime は `fle_sys.c:564-566` で MEM_COMMIT と MEM_RESERVE とその組み合わせだけを受け、MEM_TOP_DOWN など他のフラグは停止させる。つまり未実装の API ではなく、**既存の VirtualAlloc の flAllocationType の範囲外の値**。
- 範囲外なので実装していない。
- 0x403582 が停止の前に実行されたかは、この実行では**確かめられていない**。entry log は先頭 256 件で、0x403579/0x403582/0x40358b/0x41c180/0x41c1a0/0x41c1b0 は 256 件の中に無い。停止点が dispatch_miss から先へ進み、関数実行が 284 → 357 に増えたことは、dispatch が 0x403582 を通ったことと整合する（推測）。

## 6. 未確認、限界

- 602 件のうち実行時に呼ばれる数、thunk の先の関数が実行された数は未計測。
- 再配置の無い DLL で参照される thunk には、この条件 1 は働かない（この DLL は再配置あり）。
- 実 Windows との比較はしていない。
- 変異テストなし。

## 7. 触ったファイルと制約

- §2.1 のとおり。`lift/out-adj/` の中身: `probe1.py`、`survey.py`、`survey2.py`、`survey3.py`、`scan_all.py`、`stats.py`、`ret.py`、`other.py`（測定スクリプト）、`survey.json`、`survey2.json`、`scan_all.json`、`closure-pdata.txt`、`closure-pdata-adj.txt`、`gen/`、`gen-noadj.txt`、`out/`、`native/`、`obj-base/`、`bin-base/`、`log-base/`、`emcache/`、`tmp/`、`target/`、`build-native.txt`、`build-wasm.txt`、`run-native.*`、`run-node-base.*`、`test-ws.txt`。
- `rm`、既存ファイルの `>` 上書き、git 操作（読み取りのみ: `git log`、`git show`、`git status`、`git worktree list`、`git branch`）、`/tmp`、コマンド文字列の `sleep`/`timeout`/`nohup` は使っていない。権限拒否はなし。
- 例外 2 点: (a) 自分で新規作成した `out-adj/probe1.py` を `sed -i` で直した（編集ツールではない。既存ファイルではなく自分の作業ファイル）。(b) `cp -n -r out-loc4/wasm/emcache out-adj/emcache` は、`build.sh gen split` が先に空の `out-adj/emcache` を作っていたため、`out-adj/emcache/emcache` として入れ子に複製された（EM_CACHE の中身ではなく、ただの余分なコピー。EM_CACHE である `out-adj/emcache` の直下には emcc が作った `sysroot`、`build` などがある。`out-adj/emcache` 全体で 55 MB）。キャッシュの複製は効いていない（wasm のビルドは emcc が `out-adj/emcache` に作り直した）。
- 2 GB 級のファイルは作っていない（最大のものは `native/flengine-native` 258 MB、`out-adj` 全体は約 3.5 GB）。`out-wasm2`、`out-pdata` の大きなファイルは複製していない（参照のみ）。
