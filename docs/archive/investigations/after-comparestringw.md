# CompareStringW の次の停止点（unit 144 → LoadLibraryW "dsp_ippv2_x64.dll"）

実行日: 2026-10-23。HEAD 8fbefc9 に未コミットの変更（決定 0072 の実装）を載せた状態。凡例: **実測** = コマンドの出力（a4-survey、gdb）、**読み** = PE のバイト列の静的な読み（capstone）、**測定用の仮の値** = gdb で注入した値（**実機の値ではない。ランタイムには入っていない**）、**未確認** = 確認できていない。成果物は `lift/out-cs/`（gitignore の対象かは未確認）: `a4/20261022T152735Z-*`、`a4-survey-run.txt`、`gdb-after-cmp.py/.txt`、`gdb-probe-null.cmd`、`gdb-probe-null2.txt`、`gdb-probe-null3.cmd/.txt`、`pe_disasm.py`、`scan_ll.py`、`test-ws.txt`。

## 1. a4（決定 0072 の実装後。実測）

`OUT=/home/ubuntu/work/signature-studio-develop/lift/out-cs WASM=1 bash lift/harness/a4-survey.sh`（生成 C は `out-cs/out` に `build.sh gen split` で再生成: functions 59297、unsupported_total 531。native は `build_native.sh`、wasm は `build.sh rt gen-c harness link`。runtime と harness は `errors=0 warnings=0`）。終了コード 0。ログ `out-cs/a4/20261022T152735Z-*`（ファイル名の時刻は UTC）。

| | 前（`window-design.md`、`0071`） | 今回 native | 今回 wasm（node） |
|---|---|---|---|
| 停止 | CompareStringW（`fle_locale.c`） | `fle: LoadLibraryW: module "dsp_ippv2_x64.dll" is not a virtual module and not a name this environment is defined to lack (external modules are M2; no handle and no failure is invented for them)`（SIGABRT、終了 134） | 同じ（134） |
| init_count | 145 / 942 | **145 / 942**（変わらない。停止は同じ unit 144 の中、次の API へ進んだ） | 同じ |
| 最後の unit | index 144（proc 0x65c540、実行中） | index 144、run_rank 132 / 895、proc 0x65c540、in_progress | 同じ |
| started / completed | 132 / 131 | 132 / 131 | 同じ |
| 関数実行数（distinct） | 803 | **821** | 821 |
| 入口数 | 15,274 | **15,685** | 15,685 |
| 呼び出し深さ | — | 9 | 9 |

- **native と wasm の停止点は一致**。a4-survey の「same a4 lines as native (exit 134)」、および `diff <(grep -v rsp= native) <(grep -v rsp= node)` と stderr の diff が空（rsp= の 1 行だけが違う）。
- 呼び出しスタック（native。wasm も同じ）: `f_423990`（LoadLibraryW の thunk）← `f_44db40`（0x44db6b、guest-return 0x44db70）← `f_6539e0`（0x653a51、guest-return 0x653a56）← `f_653f40`（0x653f65、guest-return 0x653f6a）← `f_65c540`（unit 144。guest-return 0x412129）← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← `f_1376820`。
- unit 144 の CompareStringW は 0072 の規則で **1（LESS）** を返し、ゲストは先へ進んだ（gdb で戻り値を見たものではなく、停止が LoadLibraryW に移ったことからの読み）。

## 2. 次の停止点の原因（実測と読み）

予測（SetDllDirectoryW / SetErrorMode / LoadLibraryW）のうち、**SetDllDirectoryW と SetErrorMode は通過し、止まったのは LoadLibraryW**（予測のうち 1 つが当たった。他の 2 つは通る API だった）。

### 2.1 停止の API と引数（gdb、native。`gdb-after-cmp.txt`）

CompareStringW の後の API の並び（実測。ブレークポイントを置いた 4 つの API のみ）:

1. `CompareStringW lcid=0x409 flags=0 s1='_x64.dll' n1=8 s2='ppv2.dll' n2=8`（呼び出し元は `f_4231c0` ← `f_44ec40` ← `f_44f040`）。
2. `SetDllDirectoryW rcx=<ゲストのバッファ> path='Shared\\'`（`f_423b40` ← `f_6539e0` の 0x653a43）。`fle_load.c` の既存の応答（stores）で通過。
3. `SetErrorMode uMode=0x8000`（SEM_NOOPENFILEERRORBOX。`f_423b70` ← `f_44db40` の 0x44db56。`f_6539e0` の 0x653a51 から）。既存の応答で通過。
4. `LoadLibraryW rcx=<ゲストのバッファ> name='dsp_ippv2_x64.dll'`（`f_423990` ← `f_44db40` の 0x44db6b。guest-return 0x44db70）。**ここで停止**（`fle_load.c` の `load_by_name`、166 行）。

### 2.2 名前の出どころ（読み）

- `f_6539e0`（引数: rcx = "dsp_ippv2.dll"（0x6572dc）、dl = 1、r8 = 0。`f_653f40` の 0x653f59〜0x653f65）が `f_653660` を呼ぶ（0x653a15）。`f_653660` は、`dl != 0` で名前の末尾 8 文字が L"_x64.dll"（0x6539c8）と**等しくないとき**、0x65379b〜0x6537c3 で `f_43c4c0` を呼んで別の名前を組み立てる（読み。`f_43c4c0` は `.` を探して置き換える形に見える。未精査）。実測の名前は **"dsp_ippv2_x64.dll"** で、"dsp_ippv2.dll" の `.dll` の前に `_x64` が入った形である（読みと実測が一致。組み立ての中身までは読んでいない）。
- この名前は、`tests/load.rs` の `external_and_unnamed_modules_stop_by_name` の名前の一覧（"dsp_ippv2.dll"、"dsp_ippv2" など）には**無い**。**予測（調査 §4）にあった「別名なら停止の理由が変わりうる」は、理由は同じ（「仮想モジュールでも、無いと定義した名前でもない」）で、名前だけが "dsp_ippv2_x64.dll" に変わった**。
- `f_6539e0` の 0x653a22 以降: `f_43c5e0` が組み立てた名前の文字列を取り、`f_414850` がポインタに直し（NULL ならその場所を埋める）、0x653a43 の SetDllDirectoryW（`Shared\`）、0x653a4c の `f_44db40(name, 0x8000)`（SetErrorMode(0x8000) → LoadLibraryW(name) → SetErrorMode(元の値)）、戻り値を 0x653a56 で保存（読み）。

### 2.3 停止の性質

`LoadLibraryW` は `fle_load.c` の方針（決定 0015）どおり、名前が仮想モジュールの表にも「無いと定義した名前」（`fle_load_absent[]` の 9 件）にも無いので、**ハンドルも失敗も作らず停止**している。これは意図された名前付き停止で、ランタイムの欠陥ではない。"dsp_ippv2_x64.dll" は、この環境が答えを定義していない外部のプラグイン DLL（M2 の範囲。`tests/load.rs` の同じ種類の名前と同じ扱い）。確認したのは仮想モジュールの表と `fle_load_absent[]` に無いことだけで、実機にこの DLL があるか、ゲストがどこから探すか（`Shared\` を SetDllDirectoryW で足している）は**未確認**。

## 3. さらに先の探索（**測定用の仮の値**。実機の値ではない）

gdb（`gdb-probe-null.cmd`、`gdb-probe-null3.cmd`）で、`LoadLibraryW` の戻りを **NULL、last error 126（ERROR_MOD_NOT_FOUND）** に差し替えて、ゲストの次の動きを見た。**これは「そう答えたらどうなるか」の測定であって、決定ではない。実機で NULL になる保証も無い（"dsp_ippv2_x64.dll" は実機ではあるのかもしれない）。ランタイムには入れていない（`git status` で `lift/runtime` の変更は `fle_locale.c` のみ）。**

| 段 | 測定用の仮の値 | 結果（実測） |
|---|---|---|
| 1 | `LoadLibraryW("dsp_ippv2_x64.dll")` → NULL、126 | `f_65c540`（unit 144）は戻り、次の unit 145（proc 0x65c9f0）へ進み、`LoadLibraryW("dwmapi")`（0x65ca23。既存の `fle_load_absent[]` にある名前。NULL、126 で通過）、次に `LoadLibraryW("avrt")`（0x65ccd1）で停止: `module "avrt.dll" is not a virtual module and not a name this environment is defined to lack`。呼び出し元 `f_65c9f0`（unit 145）の 0x65ccd1（guest-return 0x65ccd6）。`GetProcAddress` が `0x7ff800130000`（既存の仮想モジュール）に対して数回呼ばれた後 |
| 2 | 上に加え `LoadLibraryW("avrt")` → NULL、126 | 続けて `LoadLibraryW("advapi32")`（0x65cee7。既存の仮想モジュールで成功）、`LoadLibraryW("PowrProf.dll")`（0x65cf38）で停止: `module "powrprof.dll" ...`。呼び出し元 `f_65c9f0` の 0x65cf38（guest-return 0x65cf3d） |

- 静的な読み（`scan_ll.py`）で `f_65c9f0` 内の LoadLibraryW の呼び出し点は 0x65ca23（"dwmapi"）、0x65ccd1（"avrt"）、0x65cee7（"advapi32"）、0x65cf38（"PowrProf.dll"）、0x65cf89（"SHCore.dll"）の 5 つ（直接 call の `lea rcx` 直前の文字列。`f_65c9f0` の範囲 0x65c9f0〜0x65e000 を読んだ。この範囲外は読んでいない）。
- `tests/load.rs` の `external_and_unnamed_modules_stop_by_name` の一覧に **"avrt" と "PowrProf.dll" がある**（これらは「名前を持たない外部」の扱いで、意図どおり名前付き停止）。"SHCore.dll" は `fle_load_absent[]` にある。
- 2 段までで止めた。これ以上は探索していない。**ここから先の API の実装はしていない**（指示どおり調査のみ）。
- 重要な点: 上の NULL は**測定用**であり、"avrt" や "PowrProf.dll" が実機で NULL になるかは未確認。unit 145 が `LoadLibraryW` の戻りでどう分岐するか（GetProcAddress を呼ぶ、無ければ飛ばす等）は、gdb で NULL の後に GetProcAddress が呼ばれていることから「少なくとも 1 つのモジュールは成功扱いで進む」までしか読んでいない。

## 4. 次に要る決定（PM の決定待ち）

1. **"dsp_ippv2_x64.dll"（と、同じ `f_6539e0` 経由の名前）**: 外部のプラグイン DLL。選択肢は (a) 停止のまま、(b) `fle_load_absent[]` に加える（NULL、126。実機のこの DLL の有無は未確認で、「この環境は持たない」と定義し直すことになる。0015 の方針の外）、(c) 仮想モジュールにする（M2。API の表が要る）。**どれも一次資料では決まらない**。ゲストがこの DLL を無くても進むかは、段 1 の測定用の値で unit 145 へ進んだことから、**進む形である**（実測。ただし NULL を返しても unit 144 の戻り値が何かは未確認）。
2. その先（unit 145 の "avrt"、"PowrProf.dll"）は、Windows 10 の標準 DLL（`avrt.dll`、`powrprof.dll`）で、実機にはある。どう答えるか（仮想モジュールにするか、無いと定義するか）の決定が要る（`0055`（msctf.dll）と同じ型）。
3. 今回の変更（決定 0072）は、unit 144 の CompareStringW を通した。unit 144 の中の他の停止は、LoadLibraryW だけである（今回の実測の範囲）。

## 5. テスト（実測。`out-cs/test-ws.txt`）

- `cargo test -p fe-lift --test locale`: 15 件すべて ok（`compare_string_symbol_before_alnum`、`compare_string_symbol_outside_rule_aborts` を含む。92 秒）。
- ワークスペース全体（`cd lift && CARGO_TARGET_DIR=.../lift/out-cmp2/target FLENGINE_PE=... cargo test --workspace --no-fail-fast`）: **85 バイナリ、996 件、失敗 0**（基準 85 バイナリ・994 件に対し 2 件増。増えたのは locale.rs の 2 件）。

## 6. 未確認と限界

- `f_43c4c0` が名前をどう組み立てるかの読みは未精査。実測の名前が "dsp_ippv2_x64.dll" であることだけが確定。
- 段 1、2 の探索は gdb の仮の値によるもので、実機の挙動ではない。unit 145 以降は実行していない（段 2 は PowrProf.dll で止めた）。
- Windows の NLS の出力とは照合していない（Linux ホスト）。CompareStringW の戻り値 1 は Learn の一文からの読み。
