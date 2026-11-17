# 暗黙の TLS（PE の TLS ディレクトリ）のモデル化: Blend2D を 3 つ目の実在モジュールに載せる前提の調査

状態: **調査のみ。決めない**。実装・ビルド・実行はしていない（読み取りと静的解析のみ）。

凡例: **実測**（ファイル・コマンドの出力を自分で確認した）／**読み**（ソースや逆アセンブルを読んだ。実行していない）／**未確認**（確かめていない、または取得できていない）／**意見**（別枠。決めるものではない）。仕様の引用は Microsoft Learn の原文（英語）をそのまま引き、非公式の出典は「非公式」と明記する。

## 0. 目次（この文書の骨組み。節ごとに追記する）

1. 作業環境（HEAD、SHA、実行したコマンド）
2. 現行 runtime / 生成器の読み（行番号つき）
3. Microsoft の仕様（URL と引用語。書かれていないことは「書かれていない」）
4. Blend2D.dll の TLS の実測
5. 現行 runtime で Blend2D を ATTACH したときの読み（実行していない）
6. モデル化の選択肢（事実の表。意見は別枠）
7. 決めるべき点
8. 触ったファイル・規則の遵守・git の状態

（以下、各節は追記される）

## 1. 作業環境（実測）

- ワークツリー: `/home/ubuntu/work/signature-studio-develop`、ブランチ `main`、HEAD `2e94df2`（`git rev-parse --short HEAD`）。調査の開始時の `git status --short` は未追跡 3 件だけ（`lift/harness/wasm/trace.c.new`、`trace_native.c.new`、`lift/iat-unsup-closure.txt`。**他の作業のもの。触っていない**）。
- 対象 DLL: `lift/out-blend2d/Blend2D.dll`、2,065,920 B、SHA-256 `66634d24412084b7541b4b27a11be8b0f4cb90d844c87e1ee5e5c3ecd66066c7`（`sha256sum`。依頼の値と一致）。読み取りのみ。
- ツール: python 3.14、pefile 2024.8.26、capstone 5.0.7、objdump（未使用）。`PYTHONDONTWRITEBYTECODE=1`、`TMPDIR=lift/out-tls/tmp`、cwd は `lift/out-tls/`（`lift/out-cs/` は使っていない）。
- 作ったもの（`lift/out-tls/` のみ。すべて新規）:

| ファイル | 内容 |
|---|---|
| `tls_dir.py` → `tls-dir.txt` | TLS ディレクトリを自前のパーサーと pefile で読む（§4.1） |
| `tls_xrefs.py` → `tls-index-xrefs.txt` | `_tls_index`（0x1801f1560）を参照する命令の全列挙。pdata 範囲の走査、`.text` 全体の線形走査、生バイトの rip 相対 disp32 の探索、8 バイト絶対値の探索の 4 通り（§4.2） |
| `tls_gs_sites.py` → `tls-gs-sites.txt` | `gs:`/`fs:` の全メモリ参照と、`gs:[0x58]` 各所の前後（§4.3） |
| `tls_offsets.py` → `tls-offsets.txt` | `tls_offsets2.py` の初版。**バグが 2 つあり（定数の探索が自分自身から始まる、`add blk, reg` を後続のオフセットに足さない）誤った行がある。削除禁止のため残す。正は `tls_offsets2.py`** |
| `tls_offsets2.py` → `tls-offsets2.txt` | 各所が TLS ブロックのどのオフセットを読み書きするか（§4.4） |
| `tls_reach.py` → `tls-reach.txt` | `Tls*`/`Fls*` の import と呼び出し点、ATTACH の種からの直接辺の到達（§4.6） |

- 実行した主なコマンド（すべて `cd lift/out-tls && export TMPDIR=$PWD/tmp PYTHONDONTWRITEBYTECODE=1`）: `python3 tls_dir.py > tls-dir.txt`、`python3 tls_xrefs.py > tls-index-xrefs.txt`、`python3 tls_gs_sites.py > tls-gs-sites.txt`、`python3 tls_offsets2.py > tls-offsets2.txt`、`python3 tls_reach.py > tls-reach.txt`。ビルド・実行（Blend2D の ATTACH、生成 C のコンパイル）は**していない**。

## 2. 現行 runtime / 生成器の読み（読み。行番号つき。HEAD 2e94df2）

| # | 場所 | 読んだこと |
|---|---|---|
| R1 | `lift/runtime/fle_vex.h:85-96` | `gs:[0x58]` は `ThreadLocalStoragePointer`（コメントでは「x64 TEB offset 0x58」）で、答えは「`fle_tls_values`（`fle_imports.c` の TlsAlloc の 1 スレッド用スロット表）の先頭アドレス」。`FLE_TEB_TLSPTR 0x58u`（96 行）。`fle_teb[0x2000]`（105 行、weak、0 初期化）にも 0x30（Self）と 0x58 を書く |
| R2 | `fle_vex.h:123-149` `fle_gs_ld8(off)` | 呼ぶたびに `fle_teb+0x30` に Self、`fle_teb+0x58` に `fle_tls_values` のアドレスを書き込む（126-127 行）。0x30 と 0x58 は書いた値を返す。0x10 は `R[4]` が登録済みスタックの外なら abort。それ以外のオフセットは abort（147 行 `only 0x30, 0x58 and 0x10`） |
| R3 | `fle_vex.h:113-121` `fle_teb_set_stack` | StackBase、StackLimit を `fle_teb` に置く（決定 0076）。TLS には触れない |
| R4 | `lift/crates/fe-lift/src/vex.rs:135-145` `mov_seg` | `mov r64, gs:[disp32]`（基底・添字なし、64 ビットロード）で disp が 0x30/0x58/0x10 のときだけ `R[n] = fle_gs_ld8(UINT64_C(off));`。FS は LEA 以外すべて `Err`（`vex.rs:136`）。**`gs:[0x58]` の後続の `mov rcx,[rax+rcx*8]` は通常のメモリロード**で、`fle_ld(R[0] + R[1]*8, 8)` の形になる。生成器は TLS の意味を知らない |
| R5 | `fle_imports.c:22-36` | `TlsAlloc`: `fle_tls_next_index` を `__atomic_fetch_add` で 0 から払い出し、上限 `FLE_TLS_MAX_INDEXES 1088`。上限超過は 0xFFFFFFFF（仕様に数値が無く mingw ヘッダの二次資料。決定 0004） |
| R6 | `fle_imports.c:63-89` | `uint64_t fle_tls_values[1088]`（69 行。**static でない**。0 初期化）。`fle_tls_owner_tid`（最初の TLS/last error 呼び出しのスレッドを持ち主にし、別スレッドは abort。73-82 行）。`fle_tls_freed[]`、`fle_tls_allocated(idx)`（`idx < next_index && !freed`。86-89 行） |
| R7 | `fle_imports.c:97-148` | `TlsSetValue` は未払い出しの index で 0 と last error 87、`TlsGetValue` は同様、`TlsFree` は `freed` を立て値を 0 にする（再利用しない） |
| R8 | `fle_runtime.h:42-64` | wasm（`FLE_CHECK_HI32`）では `fle_addr(a)` は `a >> 32` が非 0 のとき `fle_addr_fault`。それ以外は通す（`FLE_CHECKED_MEM` が有効なときだけ、さらにイメージ内かデータ領域以上かを見る）。**native は `fle_addr` が恒等**（62 行）。0 番地付近は wasm では通り（決定 0045）、native では SIGSEGV |
| R9 | `fle_realmod.c:113-146` | `call_dllmain(idx, reason)` は `fle_call_guest(entry, base, reason, 0, 0)`。`fle_realmod_load` は `read_headers`（DOS/NT シグネチャ、PE32+、エントリ RVA、**エクスポートのディレクトリ**だけ。TLS ディレクトリ（データディレクトリ 9）は**読まない**。`opt+112`/`+116` は export、`fle_realmod.c:108-109`）→ `state = LOADING` → `call_dllmain(idx, 1)`。0 が返ると `call_dllmain(idx, 0)`。**TLS コールバックを呼ぶ機会は無い**（`DLL_PROCESS_ATTACH` の前に呼ぶコードも、`DLL_THREAD_*` を呼ぶコードも無い） |
| R10 | `fle_realmod.c:15` | `RM_MAX 4u`（実在モジュールの上限 4）。`fle_realmod_table[]` はリンクが供給（`harness/two/realmod_table.c:14-15`。名前と基底のみ。TLS の情報は持たない） |
| R11 | `harness/image.c:91-116` | イメージを ImageBase（wasm はリニアメモリの同じ番地。native は `mmap` の `MAP_FIXED_NOREPLACE`）に置き、セクションを `min(VirtualSize, SizeOfRawData)` だけコピー（残りは 0）。`.data` の未初期化部分（Blend2D は 1,062,324 B）は 0。import の IAT を自分のスロット番地で埋める（148 行）。**TLS ディレクトリも `_tls_index` も処理しない**（`image.c` に `tls`/TLS の語は無い） |
| R12 | `harness/two/main_two.c:56-116` | FLEngine と実在モジュールのファイルを `image_load` し、FLEngine のエントリを呼ぶ。`fle_teb_set_stack` は 1 回（114 行）。実在モジュールのロードはゲストの `LoadLibraryW`（`fle_realmod.c`）が行う |
| R13 | `lift/crates/fe-pe/src/*.rs`（`lib.rs`、`rebase.rs`、`inittable.rs`、`main.rs`） | `tls`/`Tls`/`TLS` の語が **0 件**（grep）。`data_dirs` はデータディレクトリ全 16 本を `Vec<DataDirectory>` で持つ（`lib.rs:177,266`）ので、9 番（TLS）の RVA/サイズは読める。TLS ディレクトリの構造体の読み出しは無い |
| R14 | `lift/crates/fe-pe/src/rebase.rs` | 基底のファイル段階の移動（決定 0080）は DIR64 の再配置を適用する。TLS ディレクトリの 4 つの VA は再配置を持つので**移動する**（実測、§4.1 の 2 つ目の表） |
| R15 | `lift/crates/fe-lift/tests/vex.rs:528-541`、`gs_stacklimit.rs:183,438-452` | テストは `uint64_t fle_tls_values[8];` を自前で定義し、`gs:[0x58]` が `fle_tls_values` のアドレスを返すことを確認する。**この形（`gs:[0x58]` = `fle_tls_values` の先頭）を固定するテストが 2 つある**（変えるならテストも変わる） |
| R16 | `sync-thread-survey.md:126` | ゲストの大域状態に `fle_tls_values`（GS:[0x58] が指す表）と `fle_teb` が入る。複数の仮想スレッドにはそれぞれ持たせて切り替え時に入れ替える必要がある（案 A1 の JSPI の実験で `R[3]` が他のスレッドで壊れた。読み） |
| R17 | `sync-thread-survey.md:152`（案 C） | 案 C は「スレッド生成で名前付き abort。それ以前は単一スレッドの状態機械」。PM 決定。`DLL_THREAD_*` を受ける経路は決定 0080 D9 が作らない |

### 2.1 FLEngine 自身の `gs:[0x58]`（1 件）と `TlsAlloc` の使い方（読み）

- FLEngine の閉包の `gs:[0x58]` は **1 件**（決定 0032 の表: 0x41d14b、関数 0x41d110）。形は `mov rax,gs:0x58; mov ecx,[0x13a9388]; mov rax,[rax+rcx*8]` で、`[0x13a9388]` は **`TlsAlloc` の戻り値**（`lift/objdump-text.txt:40005-40008`）。この関数は先頭で `cmp BYTE [0x15ec2d8],0; je 0x41d14b`（`:39992-39993`）を実行する。**`[0x15ec2d8] == 0` なら `je` が取られ、`gs:[0x58]` の経路（0x41d14b〜0x41d15a）に入る。非 0 なら `TlsGetValue([0x13a9388])` の経路（0x41d11d〜0x41d149）に入る**（`:39994-40004`）。`[0x15ec2d8]` を書く命令は 1 つで、FLEngine のエントリ（0x41d8f0）の `reason == 1` の枝の `mov BYTE [0x15ec2d8],1`（`:40450` の 0x41d912。読み取り側は 0x41d114 のみ。grep で確認）。したがって **ATTACH（`reason == 1`）の実行後は非 0 で `TlsGetValue` の経路、ATTACH より前は 0 で `gs:[0x58]` の経路**（読み。**当初この向きを逆に書いたが、`je` の向きを `objdump-text.txt:39993` で確かめて訂正した**）。**つまり FLEngine の `gs:[0x58]` の読みは、ATTACH の前にしか通らない**（読み。`0x41d110` の呼び出し元は直接 call で 230 か所（`objdump-text.txt` の `call 0x41d110`）。どれが ATTACH の前に実行されるかは調べていない）。同じ向きは `vex-run.md:76`（決定 0032 の実行記録）にも「`0x41d14b`（`gs:0x58`）は、0x41d114 の `cmp BYTE [0x15ec2d8],0` が 0 のときだけ。0x15ec2d8 は 0x41d912 が 1 を書く」とある。**その実行記録は「GS 経由の分岐を通ったかは未確認」と書いている**。**現行の HEAD の実行で FLEngine の `gs:[0x58]`（`fle_gs_ld8(0x58)`）が実際に実行されたかは、この調査では確認していない（未確認）**。FLEngine の `gs:[0x58]` が実行されるなら、その `[rax + idx*8]` は `fle_tls_values[idx]` で、これは決定 0032 の想定どおり。
- つまり FLEngine の 0x41d110 は **「TlsAlloc で得た index を、片方は `TlsGetValue`、もう片方は `gs:[0x58]` の配列で引く」**（MSVC/Delphi 以外のランタイムの「速い経路と遅い経路」に見える。推測）。Windows では `gs:[0x58]` の配列と `TlsAlloc` の index は別の仕組み（§3）なので、決定 0032 はこの 1 件のために「`gs:[0x58]` を `fle_tls_values` の先頭にする」という作業者の判断をした（決定 0032 §6）。この判断により、FLEngine の `gs:[0x58]` 経路は `TlsGetValue(idx)` と同じ値になる。
- FLEngine の `TlsAlloc` は **`0x41d050` の 1 か所**（`0x41d061` の `call 0x41cf40`。`objdump-text.txt` で `call 0x41cf40` は 1 件。決定 0002）。index は `[0x13a9388]` に保存され、`0x41d000` が `LocalAlloc(0x40, 0x32a8)` で確保したブロックを `TlsSetValue([0x13a9388], rax)` で置く（`objdump-text.txt:39903-39925`、`tlsalloc-run.md` §5、`import-LocalAlloc.md`）。**したがって FLEngine は最初の `TlsAlloc` の結果（index 0）に 0x32a8（12,968）バイトのブロックのポインタを入れる**、が読み。
- **`0x41d050` が Blend2D の ATTACH より前に実行されるかは、今回の調査では実行していないので未確認**。`0x41d050` は `.data` の表 0x13a9410 の [1]（`table-reach-exact.md` §3.1）で、InitTable の proc として `f_4120c0` のループ（`0x41223e`、`0x41226d` の `call [rbx+rsi*8]`。`indirect-readers-852.md`）から呼ばれうる。過去の実行記録 `tls-run.md:57`（「TlsAlloc の結果（索引 0）は -1 と不一致」）は、**`0x41d050` が実行されて index 0 が払い出された**実測の記録（`tlsalloc-run.md` §4 の entry 8 が `f_41cf40` の戻り）。ただし**その実行は単一モジュール（Blend2D 無し）の古い build** で、現在の HEAD の 2 モジュール構成での順序は、この調査では**実行していない**。

### 2.2 実在モジュール（dsp_ippv2）の CRT も `TlsAlloc` を呼ぶ（実測の記録の引用）

- `stage23-two-module-real-dll.md:75`、`two-module-after-0087.md:112`、`two-module-post0083.md:136-138`: dsp_ippv2 の DllMain は `FlsAlloc` を `GetProcAddress` で引き（応答表が NULL を返す）、**代替として `TlsAlloc` を 1 回、`TlsSetValue` を 1 回**呼ぶ（過去の実測の記録。この調査では再実行していない）。dsp_ippv2 は TLS ディレクトリを持たない（実測、§4.7）ので、この `TlsAlloc` は**動的な TLS**（`fle_tls_values` の表）。FLEngine が先に index 0 を取っていれば dsp_ippv2 の index は 1 になる（読み。FLEngine が先であることは上記のとおり**未確認**）。Blend2D が 3 つ目としてロードされるころには index 0、1 が取られていて、Blend2D の CRT が `TlsAlloc` を呼ぶなら 2 以降（読み）。Blend2D の import に `TlsAlloc`/`TlsFree`/`TlsGetValue`/`TlsSetValue` がある（実測、§4.6）。

## 3. Microsoft の仕様（取得して読んだ。取得日 2026-10-24。原文は英語のまま引用）

### 3.1 取得したページ

| # | URL | 取得 | 用途 |
|---|---|---|---|
| M1 | https://learn.microsoft.com/en-us/windows/win32/debug/pe-format | 成功（ms.date 2026-09-10、updated_at 2026-09-10） | `.tls` 節、TLS Directory、TLS Callback Functions、データディレクトリ表 |
| M2 | https://learn.microsoft.com/en-us/windows/win32/procthread/thread-local-storage | 成功（ms.date 2025-07-14） | 動的 TLS（TlsAlloc）の概説 |
| M3 | https://learn.microsoft.com/en-us/windows/win32/dlls/using-thread-local-storage-in-a-dynamic-link-library | 成功 | DLL の動的 TLS のサンプル |
| M4 | https://learn.microsoft.com/en-us/cpp/parallel/thread-local-storage-tls | 成功（ms.date 2019-08-09、updated_at 2025-07-01） | `__declspec(thread)` の制限（Vista 以前） |
| M5 | https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain | 成功（updated_at 2026-07-15） | DllMain の reason、`lpvReserved`、LoadLibrary 時の呼ばれ方 |
| M6 | https://learn.microsoft.com/en-us/windows/win32/api/winternl/ns-winternl-teb | 成功（ms.date 2018-12-05） | 公式の TEB の定義（0x58 を載せているか） |
| M7 | https://learn.microsoft.com/en-us/cpp/build/reference/zc-threadsafeinit-thread-safe-local-static-initialization | 成功 | 関数内 static の初期化ガードが TLS を使うこと |

### 3.2 PE Format の `.tls` 節（M1）の引用

**概要と、動的ロード時の対応（Vista 以降）**:

> "The .tls section provides direct PE and COFF support for static thread local storage (TLS). TLS is a special storage class that Windows supports in which a data object is not an automatic (stack) variable, yet is local to each individual thread that runs the code."

> "Before Windows Vista, statically declared TLS data objects can be used only in statically loaded image files. This fact makes it unreliable to use static TLS data in a DLL unless you know that the DLL, or anything statically linked with it, will never be loaded dynamically with the LoadLibrary API function. However, starting with Windows Vista, improvements were made to the Windows loader to better support the dynamic loading of DLLs with static TLS. This change means that DLLs with statically declared TLS data objects can now be used more reliably, even if they are loaded dynamically using LoadLibrary. The loader is capable of allocating TLS slots for such DLLs at load time, mitigating the limitations present in earlier versions of Windows."

**アクセスの手順（原文の番号つき。x86 の書き方で、64 ビットは読み替える旨の注記がある）**:

> "1. At link time, the linker sets the Address of Index field of the TLS directory. This field points to a location where the program expects to receive the TLS index."
> "2. When a thread is created, the loader communicates the address of the thread's TLS array by placing the address of the thread environment block (TEB) in the FS (for x86) or GS (for x64) register. A pointer to the TLS array is at the offset of 0x2C from the beginning of TEB. This behavior is Intel x86-specific."
> "3. The loader assigns the value of the TLS index to the place that was indicated by the Address of Index field."
> "4. The executable code retrieves the TLS index and also the location of the TLS array."
> "5. The code uses the TLS index and the TLS array location (multiplying the index by 4 and using it as an offset to the array) to get the address of the TLS data area for the given program and module. Each thread has its own TLS data area, but this is transparent to the program, which does not need to know how data is allocated for individual threads."
> "6. An individual TLS data object is accessed as some fixed offset into the TLS data area."

> "The TLS array is an array of addresses that the system maintains for each thread. Each address in this array gives the location of TLS data for a given module (EXE or DLL) within the program. The TLS index indicates which member of the array to use. The index is a number (meaningful only to the system) that identifies the module."

> "References to 32-bit offsets and index multipliers of 4 apply to systems with 32-bit architectures. In a system based on 64-bit architectures, adjust them as necessary."

**TLS Directory の表（PE32+ の列）**:

| Offset (PE32+) | Size | Field | 原文の説明（引用） |
|---|---|---|---|
| 0 | 8 | Raw Data Start VA | "The starting address of the TLS template. The template is a block of data that is used to initialize TLS data. The system copies all of this data each time a thread is created, so it must not be corrupted. Note that this address is not an RVA; it is an address for which there should be a base relocation in the .reloc section." |
| 8 | 8 | Raw Data End VA | "The address of the last byte of the TLS, except for the zero fill. As with the Raw Data Start VA field, this is a VA, not an RVA." |
| 16 | 8 | Address of Index | "The location to receive the TLS index, which the loader assigns. This location is in an ordinary data section, so it can be given a symbolic name that is accessible to the program." |
| 24 | 8 | Address of Callbacks | "The pointer to an array of TLS callback functions. The array is null-terminated, so if no callback function is supported, this field points to 4 bytes set to zero." |
| 32 | 4 | Size of Zero Fill | "The size in bytes of the template, beyond the initialized data delimited by the Raw Data Start VA and Raw Data End VA fields. The total template size should be the same as the total size of TLS data in the image file. The zero fill is the amount of data that comes after the initialized nonzero data." |
| 36 | 4 | Characteristics | "The four bits [23:20] describe alignment info. Possible values are those defined as IMAGE_SCN_ALIGN_*, which are also used to describe alignment of section in object files. The other 28 bits are reserved for future use." |

**TLS Callback Functions**:

> "The program can provide one or more TLS callback functions to support additional initialization and termination for TLS data objects. A typical use for such a callback function would be to call constructors and destructors for objects."
> "Although there is typically no more than one callback function, a callback is implemented as an array to make it possible to add additional callback functions if desired. If there is more than one callback function, each function is called in the order in which its address appears in the array. A null pointer terminates the array. It is perfectly valid to have an empty list (no callback supported), in which case the callback array has exactly one member-a null pointer."
> "The prototype for a callback function (pointed to by a pointer of type PIMAGE_TLS_CALLBACK) has the same parameters as a DLL entry-point function: `typedef VOID (NTAPI *PIMAGE_TLS_CALLBACK) (PVOID DllHandle, DWORD Reason, PVOID Reserved);`"
> "The Reserved parameter should be set to zero. The Reason parameter can take the following values: DLL_PROCESS_ATTACH 1 ('A new process has started, including the first thread.'), DLL_THREAD_ATTACH 2 ('A new thread has been created. This notification sent for all but the first thread.'), DLL_THREAD_DETACH 3, DLL_PROCESS_DETACH 0."

**Learn が書いていること／書いていないこと（M1。自分で確認）**:

| 項目 | 書かれているか | 備考 |
|---|---|---|
| ローダーが `Address of Index` の指す場所に TLS index を書く | **書かれている**（手順 3、表の Address of Index） | |
| ローダーが TLS テンプレートをスレッドごとにコピーする | **書かれている**（Raw Data Start VA: "The system copies all of this data each time a thread is created"） | |
| TLS 配列の TEB 内の位置 | x86 は **0x2C**（"A pointer to the TLS array is at the offset of 0x2C from the beginning of TEB. This behavior is Intel x86-specific."）。**x64 のオフセット（0x58）は書かれていない**（ページ全体を `0x58`・`0x88`・`ThreadLocalStoragePointer`・`TEB` で grep。0x2C の 1 件のみ） | 注記は「64 ビットでは必要に応じて調整せよ」とだけ書く。**`gs:[0x58]` の根拠は Learn には無い** |
| TLS 配列の要素の大きさ（x64 で 8 バイト） | 「index に 4 を掛ける」と x86 の値だけ。64 ビットは「調整せよ」 | |
| **index の払い出し規則**（0 から順か、再利用するか、何番から始まるか） | **書かれていない**。"The index is a number (meaningful only to the system) that identifies the module." とだけ | |
| TLS 配列の長さ、スレッドごとの TLS 配列を作る時点（スレッド作成時か、最初のアクセス時か） | "When a thread is created, the loader communicates the address of the thread's TLS array" と書くが、配列の作り方は書かない | |
| TLS ブロックのアライメント | **Characteristics のビット [23:20]**（IMAGE_SCN_ALIGN_*）として書かれている。Blend2D は 0x400000 = ビット 22 = 値 4 = `IMAGE_SCN_ALIGN_8BYTES`（0x00400000。M1 の表）（実測、§4.1） | 配置の実装（アドレスをこの境界に合わせるか）の義務は Learn に明示が無い |
| TLS コールバックの呼び出し順（全モジュールの中で、DllMain との前後） | **書かれていない**（配列内の順は書く。DllMain との順は書かない） | ReactOS/Wine の実装は別（§3.5、非公式） |
| LoadLibrary で動的に載せた DLL の暗黙の TLS | **書かれている**（Vista 以降。上の Note の引用）。**どう実現するか（既存スレッドへのブロックの配布、配列の拡張）は書かれていない** | M4 は「Vista より前では LoadLibrary で動的ロードすると protection fault」と書く |

### 3.3 他のページ（M2〜M7）の引用と「書かれていない」こと

- **M2（Thread Local Storage）**: "When the threads are created, the system allocates an array of LPVOID values for TLS, which are initialized to NULL." "The maximum number of indexes per process is 1,088." これは**動的 TLS（TlsAlloc）の配列**の話で、PE の `.tls` の配列（`gs:[0x58]`）との関係は**書かれていない**。M2 は「TlsAlloc の index」と「PE の `Address of Index`」が同じ番号空間かを**書かない**。
- **M6（TEB）**: 公式の TEB の定義は `Reserved1[12]` の次に `ProcessEnvironmentBlock`、`Reserved2[399]`、`Reserved3[1952]`、`TlsSlots[64]`、`Reserved4[8]`、`Reserved5[26]`、`ReservedForOle`、`Reserved6[4]`、`TlsExpansionSlots`。**`ThreadLocalStoragePointer` は載っていない**（オフセット 0x58 は `Reserved1[12]`（x64 で 96 バイト = 0x60 まで）の内側にあたる。これは構造体定義から導いた**読み**で、M6 自身は 0x58 を何とも書かない）。M6 は "You should not directly access this structure." と書く。
- **M5（DllMain）**: "If fdwReason is DLL_PROCESS_ATTACH, lpvReserved is NULL for dynamic loads and non-NULL for static loads." "DLLs can use this opportunity to initialize any instance data or to use the TlsAlloc function to allocate a thread local storage (TLS) index." "A thread calling the DLL entry-point function with DLL_PROCESS_ATTACH does not call the DLL entry-point function with DLL_THREAD_ATTACH. Note that a DLL's entry-point function is called with this value only by threads created after the DLL is loaded by the process. When a DLL is loaded using LoadLibrary, existing threads do not call the entry-point function of the newly loaded DLL." **DllMain の前に TLS コールバックを呼ぶかは書かれていない**。
- **M7（/Zc:threadSafeInit）**: "Thread-safe static local variables use thread-local storage (TLS) internally to provide efficient execution when the static has already been initialized. The implementation of this feature relies on Windows operating system support functions in Windows Vista and later operating systems." "Thread-safe initialization of static local variables relies on code implemented in the Universal C run-time library (UCRT)." これが Blend2D の `gs:[0x58]` 26 か所の出どころ（`__Init_thread_header`/`_footer` と、その前の「`_Init_thread_epoch` を TLS から読んで比較する」インライン）である、という**読み**を支える（§4.4）。ガードの変数の名前や形（`_Init_thread_epoch` が 32 ビットのスレッドローカル）は M7 には**書かれていない**。
- **M4**: "`__declspec( thread )` ... On Windows operating systems before Windows Vista, `__declspec( thread )` has some limitations. If a DLL declares any data or object as `__declspec( thread )`, it can cause a protection fault if dynamically loaded." Vista 以降の動作は M1 の Note の引用。

### 3.4 非公式の出典（**非公式**。Learn の代わりにはならない）

| # | 出典 | 取得 | 読んだこと |
|---|---|---|---|
| U1 | Geoff Chappell「TEB」 https://www.geoffchappell.com/studies/windows/km/ntoskrnl/inc/api/pebteb/teb/index.htm（非公式） | 成功 | x64 の TEB の表: `0x58 PVOID ThreadLocalStoragePointer`（all versions）、`0x1480 PVOID TlsSlots[0x40]`（x86 は 0x2C と 0xE10）。**"The thread-local storage that is provided through the ThreadLocalStoragePointer has nothing to do with the API functions such as TlsAlloc. Those are supported through the TlsSlots array and TlsExpansionSlots pointer much further into the TEB and through such PEB members as the TlsBitmap."** "What the ThreadLocalStoragePointer points to, once it is set up, is an array of pointers to the thread-local data for each module that has any." "…specifically before version 6.0, because this form of thread-local storage was supported only for modules that are loaded with the process, not for DLLs that are loaded later." |
| U2 | ReactOS `dll/ntdll/ldr/ldrinit.c`（master、https://raw.githubusercontent.com/reactos/reactos/master/dll/ntdll/ldr/ldrinit.c）（非公式。GPL のオープンソース実装。Windows の実際の挙動の記述ではない） | 成功 | `LdrpInitializeTls`: **起動時に載っているモジュール全部**を走査し、TLS ディレクトリを持つ順に `*(PLONG)TlsDirectory.AddressOfIndex = LdrpNumberOfTlsEntries;`（**0 から順**に払い出す）。`LdrpAllocateTls`: `TlsVector = RtlAllocateHeap(LdrpNumberOfTlsEntries * sizeof(PVOID)); Teb->ThreadLocalStoragePointer = TlsVector;` モジュールごとに `TlsDataSize = EndAddressOfRawData - StartAddressOfRawData` の領域を確保し、`RtlCopyMemory(TlsVector[idx], StartAddressOfRawData, TlsDataSize)`。**ReactOS の `LdrpAllocateTls` は `SizeOfZeroFill` を足していない**（`TlsDataSize` は End − Start のみ。読み）。`LdrpCallTlsInitializers(entry, reason)` は `AddressOfCallBacks` の配列を、`*Array` が 0 になるまで `LdrpCallInitRoutine(callback, DllBase, Reason, NULL)` で呼ぶ。**DLL の `DLL_PROCESS_ATTACH` では、TLS コールバックを DllMain の直前に**（`LdrpRunInitializeRoutines` の `if (LdrEntry->TlsIndex && Context) LdrpCallTlsInitializers(LdrEntry, DLL_PROCESS_ATTACH); … LdrpCallInitRoutine(EntryPoint, …)`。`Context` が非 NULL のとき、つまりプロセス起動時の初期化に限る形）。`DLL_THREAD_ATTACH` も同じ順（`LdrpInitializeThread`）。**取得した `ldrinit.c` と `ldrutils.c` の範囲では**、`LdrpInitializeTls` はプロセス起動時（`LdrpInitializeProcess`（1785 行〜）の中の 2448 行付近の `Status = LdrpInitializeTls();`。直前に「Following two calls are for Vista+ support」のコメントつきの `RtlpInitializeKeyedEvent()`、`RtlpInitializeThreadPooling()` がある）に 1 回走るだけで、LoadLibrary の後に既存スレッドへブロックを配る処理（Vista 以降の動的ロード対応）は**見つからなかった**（`ldrapi.c` など他のファイルは取得していない。**未確認**） |
| U3 | Wine `dlls/ntdll/loader.c`（master。https://raw.githubusercontent.com/wine-mirror/wine/master/dlls/ntdll/loader.c。github の mirror。取得時点の master の HEAD は 840a939e…、2026-10-19）（非公式） | 成功 | `alloc_tls_slot(mod)`: TLS ディレクトリがあり `size = End − Start`、`!size && !SizeOfZeroFill && !AddressOfCallBacks` なら TLS 無しとして扱い、`tls_dirs[]`（初期 32 個、足りなければ倍）の**空き最小の添字** i を取り、**実行中の全スレッド**（`NtGetNextThread`）の TEB の `ThreadLocalStoragePointer` の配列の i 番に `size + SizeOfZeroFill` のブロックを確保して `memcpy(template, size)` + `memset(zero fill)`、配列が足りなければ拡張（古い配列は解放しない旨の FIXME）。最後に `*(DWORD *)dir->AddressOfIndex = i; tls_dirs[i] = *dir;`。`alloc_thread_tls()`（新スレッド）: `tls_module_count` 個の配列を 0 初期化で作り、各 `tls_dirs[i]` に `size + SizeOfZeroFill` のブロックを確保して同じくコピー。`NtCurrentTeb()->ThreadLocalStoragePointer = pointers`。`call_tls_callbacks(module, reason)`: `AddressOfCallBacks` の配列を `*callback` が 0 になるまで `call_dll_entry_point(callback, module, reason, NULL)`。`MODULE_InitDLL` は **`if (wm->ldr.TlsIndex == -1) call_tls_callbacks(...)` の後に `entry`（DllMain）を呼ぶ**（TLS コールバックが DllMain の前）。`alloc_tls_slot` は `fixup_imports()` の**最初**（`if (alloc_tls_slot(&wm->ldr)) wm->ldr.TlsIndex = -1;`。loader.c の 1495 行付近）で呼ばれる。つまり**そのモジュールの DllMain より前、import の解決より前**に index が書かれる（読み）。Wine の `alloc_tls_slot` は **`Characteristics` のアライメントを使っていない**（読み。`alloc_tls_slot`/`alloc_thread_tls` に `Characteristics` の参照は無い。`TRACE` の出力に使うだけ） |
| U4 | Vergilius Project の `_TEB`（決定 0032 が引用） | **未取得**（この調査では取りに行っていない。決定 0032 §6 の記述の再引用のみ: ThreadLocalStoragePointer 0x58、TlsSlots 0x1480） | — |

**非公式の出典の使い方についての注意**: U2、U3 は「実装の一例」で、Windows の実機の挙動の確認ではない。U2（ReactOS）と U3（Wine）は **index の払い出しを 0 から順（U2）／最小の空き（U3）** とし、**どちらも `AddressOfIndex` にそれを書く**。Windows の index が 0 から始まるか、モジュールの順序（ロード順）で決まるかは **Learn には書かれていない**。実機の Windows での index は**確認していない**。

## 4. Blend2D.dll の TLS の実測（読み取りのみ。`lift/out-blend2d/Blend2D.dll`、SHA-256 66634d24…66066c7）

方法は §1 の 6 本のスクリプト（`lift/out-tls/`）。逆アセンブルは capstone 5.0.7（`.pdata` の 2,493 範囲と `.text` 全体の線形走査の 2 通り）。出力は `lift/out-tls/*.txt`。

### 4.1 TLS ディレクトリ（実測。`tls-dir.txt`、`tls-rebased.txt`）

自前のパーサーと pefile の値は一致した。データディレクトリ 9 = RVA 0x1e1c00、サイズ 0x28（40 バイト。PE32+ の TLS ディレクトリは 6 フィールドで 40 バイト）。**`.tls` という名前のセクションは無い**（セクションは `.text`、`.rdata`、`.data`、`.pdata`、`_RDATA`、`.rsrc`、`.reloc`。`tls_dir.py` の出力）。ディレクトリは `.rdata` の中にある（RVA 0x1e1c00）。

| フィールド | 値（VA） | RVA / 所在 | 先の調査（§3.1）との照合 |
|---|---|---|---|
| StartAddressOfRawData | 0x1801e3770 | 0x1e3770（`.rdata`） | 一致 |
| EndAddressOfRawData | 0x1801e3790 | 0x1e3790（`.rdata`） | 一致。テンプレートは **0x20 = 32 バイト** |
| AddressOfIndex | 0x1801f1560 | 0x1f1560（`.data`）。**ファイル上の値は 0**（`.data` の生データ 0x1600 の内側。4 バイト 0） | 一致（`.data` のファイル上の 0 は「`.data` の生データの中で値が 0」で、未初期化部分の外。`tls-dir.txt`） |
| AddressOfCallBacks | 0x1801b7370 | 0x1b7370（`.rdata`） | 一致。配列の **先頭の 8 バイトが 0**（= コールバックなし） |
| SizeOfZeroFill | 0 | — | 一致 |
| Characteristics | 0x400000 | ビット [23:20] = 4 = `IMAGE_SCN_ALIGN_8BYTES`（M1 の表: 0x00400000） | **先の調査には無い値**。TLS ブロックは 8 バイト境界 |

- 4 つの VA フィールドは `.reloc` に DIR64 の再配置を持つ（pefile: Start、End、Index、Callbacks のいずれも True）。**ディレクトリ自体が再配置の対象**。`callbacks[0]` の 8 バイトと、テンプレートの 32 バイトの中には再配置が無い（True の位置が無いことを確認した）。
- `AddressOfCallBacks` の配列: 先頭 4 つの qword は `0x0, 0x0, 0x1801a7cc4, 0x1801b0434`（`tls_dir.py`）。**先頭が 0 なので配列は空**（M1: "the callback array has exactly one member-a null pointer"）。3 番目以降の値は**別のデータ（隣の配列。`.rdata` の次の表）で、コールバックではない**（0 で終端されるので呼ばれない）。
- **テンプレート 32 バイト（実測）**: `0000000000000000 0000008000000000 0000000000000000 0000000000000000`。オフセット +0x08 の 32 ビットが `0x80000000`、それ以外は 0。**先の調査（§3.1）の「`0x…80000000` を含む 32 バイト」と一致**。`+0x00` の qword は 0、`+0x08` の qword は `0x0000000080000000`、`+0x10`、`+0x18` は 0。

**ファイル段階の移動（決定 0080）後（実測。`lift/out-blend2d/Blend2D-rebased-4700000.dll`、SHA-256 24e01ec9…b4cb。基底 0x4700000）**:

| フィールド | 移動前 | 移動後 | 差が基底の差（−0x17b900000）と等しいか |
|---|---|---|---|
| Start | 0x1801e3770 | 0x48e3770 | **はい** |
| End | 0x1801e3790 | 0x48e3790 | はい |
| AddressOfIndex | 0x1801f1560 | 0x48f1560 | はい |
| AddressOfCallBacks | 0x1801b7370 | 0x48b7370 | はい |
| SizeOfZeroFill、Characteristics、テンプレート 32 B、`callbacks[0]`、index セルのファイル上の値 | — | 同じ | 同じ |

つまり **fe-pe の rebase は TLS ディレクトリの 4 つの VA も動かす**（DIR64 のため）。したがって実在モジュールとして載せた後の TLS ディレクトリの VA は基底に追随する。`fle_realmod_table` の基底と TLS の指す場所が食い違うことは、rebase が正しく行われていれば無い（読み。ただし**リベース後の TLS を使う実行は、していない**）。

### 4.2 `_tls_index`（0x1801f1560）を参照する命令の全列挙（実測。`tls-index-xrefs.txt`）

方法: (1) `.pdata` の 2,493 範囲を走査、(2) `.text` 全体（0x1000〜0x1b6205）を線形走査（capstone の skipdata）、(3) 生バイトで rip 相対 disp32 が 0x1801f1560 に当たる位置を全探索、(4) 8 バイト絶対値の探索。(1) と (2) は同じ 23 件（差は 0。pdata 範囲外の参照も無い）。(3) の生バイト探索 23 件はすべて (1)/(2) の命令の disp32 に重なった（`covered_by_decoded_insn=True` が 23 件）。(4) の絶対値は **ディレクトリの `AddressOfIndex` フィールド 1 か所**だけ（file 0x1e0410 = VA 0x1801e1c10）。

**読む命令: 23 件。書く命令: 0 件**（`.text` の中に `_tls_index` への `mov [rip+…], reg`、`lea`、`xchg`、`lock` 付きなどの**書き込みも、アドレスを取る `lea` も無い**）。

| 区分 | 件数 | 命令 |
|---|---|---|
| 読む（`mov r32, dword ptr [rip+disp]`） | 23 | `mov ecx`/`edx`/`r8d`/`r13d, [_tls_index]` |
| 書く | **0** | — |
| `lea` でアドレスを取る | **0** | — |

23 件の所在（`tls-index-xrefs.txt` の関数ごとの件数）: `f_18000abb0` 1、`f_180038d90` 1、`f_180081700` **5**、`f_1800858b0` 1、`f_180086e40` 1、`f_180089550` 1、`f_18008ba50` 1、`f_180097190` 1、`f_1800a3430` **7**、`f_1800a4120` 1、`f_180146ff0` 1、`f_18019dbf8` 1、`f_18019dc58` 1（合計 1+1+5+1+1+1+1+1+7+1+1+1+1 = 23）。

**`_tls_index` の読み（23）と `gs:[0x58]` の読み（26）の差 3 の内訳（実測。`tls-index-src2.txt`、`tls-gs-sites.txt`）**: 差は `f_180081700` だけで生じる。この関数は `gs:[0x58]` が **8** か所、`_tls_index` の読みが **5** か所。8 か所のうち 4 か所（`0x18008197b`、`0x180081a25`、`0x180081acd`、`0x180081b73`）は、`0x18008195d` の `mov r13d,[_tls_index]` **1 回**で作った `r13d` を index として使い回す。残り 4 か所（`0x180081c57`、`0x180081d62`、`0x180081e62`、`0x180081f61`）は、それぞれ直前に `mov edx,[_tls_index]` を実行する。したがって 8 − (1 + 4) = 3 か所は読みを共有する。ほかの 12 関数は 1 か所あたり 1 回の読みで、26 − 3 = 23 と合う。`tls-index-src2.txt` の線形の遡り（26/26 が `_tls_index` 由来）と矛盾しない。

**「`_tls_index` を書く」のはローダー（Windows の `ntdll`）だけで、Blend2D 自身のコードは一度も書かない**（読み。Learn の手順 3 と整合）。つまり**書き手が居ないと `_tls_index` は 0 のまま**（ファイル上の値が 0。§4.1）。

### 4.3 `gs:[0x58]` の読み: 26 か所・13 関数（実測。`tls-gs-sites.txt`）

**数の訂正**: 先の調査（§6）は「29 か所」と書く。これは `fe-lift lift` の `mov_gs` 29 件（`lift-pdata.txt`）で、**`gs:[0x58]` が 26、`gs:[0x30]` が 2、`gs:[0x10]` が 1** の合計。**`gs:[0x58]` は 26 か所（13 関数）**（`.text` の線形走査で 26、生バイト `65 [rex] 8b … 25 58 00 00 00` の探索でも 26。`sites-gs.txt` の 26 行と一致）。13 関数という数は先の調査と同じ。**先の調査の「29 か所」は `gs:` 全体（0x58 が 26 + 0x30 が 2 + 0x10 が 1）の数と読める**。ほかに `gs:[0x60]`（PEB）が 3 か所（`f_1801a4d50`、`f_1801a9e60`、`f_1801a9eb4`）あり、これらは `mov_gs` に入らず `fle_unsupported` になる（`blend2d-pdata.c` の `fle_gs_ld8` は 0x10 が 1、0x30 が 2、0x58 が 26。`gs:[0x60]` は 0 件）。`fs:` の参照は `.text` の線形走査で 2 件（`0x180020848` の `fild word ptr fs:[rcx]` と `0x1800212f4` の `adc dword ptr fs:[rdx], eax`）あるが、どちらも `.pdata` の範囲内のデータ領域（命令の途中の誤デコードの可能性が高い。**未確認**）。

**`gs:[0x58]` の形（26 か所すべて同じ形）**:

```
mov rax, gs:[0x58]          ; TLS 配列
mov ecx, [rip+_tls_index]   ; index（gs の前か後。位置は関数により異なる）
mov edx, 8                   ; オフセット（定数。r12/r14/r8/r9/rcx のこともある）
mov rcx, [rax + rcx*8]       ; ← この関数のブロックのアドレス（index が _tls_index から来る）
mov eax, [rdx + rcx]         ; ← ブロック + 8 の 32 ビットを読む（_Init_thread_epoch のスレッドローカル版）
cmp [rip+guard], eax         ; ガード変数（.data の 0x1802f4b88 など）と比較
jle  skip
lea rcx,[rip+guard]; call 0x18019dc58  (_Init_thread_header)
cmp [rip+guard], -1; jne skip …; (初期化)  call 0x18019dbf8  (_Init_thread_footer)
```

- **index が `_tls_index` から来るか（実測、`tls-index-src2.txt`）**: `mov blk,[arr + idx*8]` の idx レジスタについて、同じ関数内で直前に書いた命令を線形に遡った結果、**26 か所すべて `mov r32,[rip+_tls_index]`**（26/26。初版の `tls_index_src.py` は 3 件を取り逃した（gs の読みから探索を始めたため。例えば `f_18000abb0` では `mov ecx,[_tls_index]` が gs の読みと block の読みの間にある）。出力は訂正後の `tls-index-src2.txt`。初版の出力 `tls-index-src.txt` は残してあるが誤り）。これは線形の遡りで、**制御フローの証明ではない**（分岐をまたぐ書き込みは見ていない）。
- **`f_180097190`（`gs:[0x58]` は 1 か所）と `f_1800a4120`（1 か所）** は `mov rsi/r14, [rax+rcx*8]; add rsi/r14, rdx/r9(=8)` の形で、ブロック + 8 のポインタを作り、`cmp ecx,[rsi]`／`mov eax,[r14]` で読む。**ブロック + 8 の 32 ビット読み**（他と同じオフセット）。`f_180097190` は、この 1 つのポインタ `rsi` を同じ関数内の後続の 3 つの静的ローカルの確認（`0x18009728a`、`0x1800972f2`、`0x18009735a` の `cmp ecx,[rsi]`）でも使い回す（`gs:[0x58]` の再読みは無い。`0x180097274` 付近の `movups` の直後。`capstone` の逆アセンブルで確認）。**サイトごとの内訳は次の表（実測。`tls-offsets3.txt` のサイトごとの集計）**。

| サイトの種類 | サイト数 | そのサイトのアクセス | 件数 |
|---|---|---|---|
| +0x08 の 4 バイト読みを行う | **23**（`f_180097190` の 1 サイトは 4 件、ほか 22 サイトは 1 件ずつ） | 読み | 4 + 22 = **26 件** |
| +0x08 の 4 バイト書きだけを行う（`f_18019dbf8`、`f_18019dc58`。`gs:[0x58]` の後で `[blk+8]` に書く） | **2** | 書き 1 件ずつ | **2 件** |
| +0x18 の 8 バイト読みと書きを行う（`f_180146ff0`） | **1** | 読み 1 件 + 書き 1 件 | **2 件** |
| 合計 | 23 + 2 + 1 = **26** | | 26 + 2 + 2 = **30 件**（集計と一致: +0x08 読み 26、+0x08 書き 2、+0x18 読み書き 2） |

「`gs:[0x58]` のサイト 26」と「+0x08 の読み 26 件」が同じ数なのは**偶然**で、前者はサイトの数、後者は +0x08 を読むサイト 23 か所の読みの件数（`f_180097190` の 1 サイトが 4 件）である。
- **`f_180146ff0`（1 か所）** は別の形: `mov rdx,[rax+rcx*8]; mov rcx,[rdx + r8 + 8]`（`r8 = 0x10`）→ **ブロック + 0x18 の 64 ビット読み**、後で `mov [rdx + r8 + 8], rax`（**ブロック + 0x18 への 64 ビット書き**）。これは**`thread_local` の 64 ビット変数**（カウンタ。`lock xadd qword ptr [0x1802..], rcx` で大域のカウンタを 0x100 ずつ進めて自分に割り当てる形）に見える（読み。`0x180147145` の `lock xadd`）。

### 4.4 ブロック内のオフセットと、32 B のテンプレートの対応（実測。`tls-offsets3.txt`）

| ブロック内のオフセット | 幅 | 読み | 書き | 件数 | 備考 |
|---|---|---|---|---|---|
| **+0x08** | 4 | 26 | 2 | 読み 26（`mov r32,[r+blk]` 形と、`add blk,8` を経由する `cmp ecx,[rsi]`／`mov eax,[r14]` 形。`f_180097190` の読みは 4 件（`0x18009721e` と、窓を広げて見えた `0x18009728a`、`0x1800972f2`、`0x18009735a`）、`f_1800a4120` は 1 件）、書き 2（`f_18019dbf8` の `mov [r9+r8], eax` と `f_18019dc58` の `mov [r8+rdx], eax`） | **`_Init_thread_epoch`（スレッドローカルの 32 ビット）** に見える。テンプレートの +0x08 は **0x80000000**（実測、§4.1）で、**`_Init_thread_epoch` の初期値が `INT_MIN`（0x80000000）**、`eax = [blk+8]; cmp [guard], eax; jle skip`（ガードが epoch 以下ならスキップ）と整合。書き込み 2 件は `__Init_thread_header`/`_footer` が **グローバルの `_Init_global_epoch`（0x1801f0000。ファイル上の初期値は `ffffffff80000000`、つまり下位 32 ビットが 0x80000000）** を +1 して**自分のスレッドの epoch に書く**形（`f_18019dbf8` の `inc eax; mov [0x1801f0000], eax; … mov [r9+r8], eax`）。**読み。MSVC の CRT のソースは取得していない**（名前 `_Init_thread_epoch` の対応は推測）。 |
| **+0x18** | 8 | 1 | 1 | 1 関数（`f_180146ff0`） | `thread_local` の 64 ビット変数。**テンプレートの +0x18 は 0**（実測）。同じ関数がブロック + 0x18 を読んで 0 なら `lock xadd [0x1802…], 0x100` で割り当て、`+1` を書く形（読み） |
| +0x00 | — | 0 | 0 | — | 読み書きの命令は無い（テンプレートの qword は 0） |
| +0x10 | — | 0 | 0 | — | 読み書きの命令は無い |
| **読み書きの最大オフセット** | | | | | **+0x18 の 8 バイト = ブロックの先頭から 0x20（32 バイト）のうち最後の qword**。32 B のテンプレートの外（+0x20 以上）に触る命令は**無い**（26 か所のサイトの 30 件のアクセスについて。`tls-offsets3.txt`。線形の遡りで、他の関数からポインタを渡して読み書きする経路（ブロックのアドレスをどこかに保存して後で別のオフセットで使う）は見ていない）。 |

- 26 か所すべてで、ブロックを指すレジスタが `lea`/`mov` でどこかへ保存されるか（アドレスが外へ漏れるか）は見ていない。見た範囲では、レジスタは `cmp`、`mov eax,[…]`、`add`（+8）にしか使われない。
- オフセットの定数（`mov edx,8` などで作る 8）は、同じ関数内の直前の `mov reg, 8`／`mov reg, 0x10`。**再代入はなかった**（`tls_offsets2.py`／`tls_offsets3.py` の「定数レジスタの書き直しの検査」: 27 のうち 0。**直線の順序での検査**で、分岐をまたぐ書き直しは見ていない）。`f_180097190` の 4 件の読みは `add rsi, rdx`（`rdx = 8`）で作った 1 つのポインタ `rsi` を、同じ関数内で `cmp ecx,[rsi]` に繰り返し使う形（読み）。`rsi` が途中で上書きされていないことは、`f_180097190` の `0x180097211`〜`0x180097370` を capstone で走査した結果（`rsi`/`esi` を第 1 オペランドにする書き込み、`xchg`、`movs`/`rep` 系を探した）、**書き込みは `mov rsi,[rax+rcx*8]`（0x180097211）と `add rsi,rdx`（0x180097215）の 2 件だけ**だった。この範囲に call が 7 つあるが、すべて `0x18019dc58`（`__Init_thread_header` に見える）か `0x18019dbf8`（`_footer`）で、Win64 では `rsi` は callee-saved（呼び出し先が保存して戻す）。これは**直線の順序の走査**で、分岐をまたぐ別経路の書き換えまでは見ていない（読み）。

### 4.5 ATTACH の直線上にあるか（読み。`tls-reach.txt`）

直接 call/jmp の辺だけの到達（間接呼び出し（vtable、レジスタ）は追わない = 下界）。種 6 つ（`0x18019e584` エントリ、`.xc` の `0x180001000`、`.xi` の `0x18019db00`、`0x1801a4400`、`0x1801af6e4`、`0x1801b0e00`）から **151 関数**（先の調査 `reach-attach.txt` の 151 と一致）に届き、`gs:[0x58]` の 13 関数のうち **3 関数**が入る:

| 関数 | 到達の経路 | `_tls_index` を読む | 備考 |
|---|---|---|---|
| `f_18000abb0` | `.xc` の `f_180001000` → `f_180199d90`（`call 0x18000abb0`。`0x180199dce` と `0x180199f31` の 2 か所）→ `f_18000abb0` | **はい（`0x18000abcd`）** | `f_180199d90` は `.xc`（C++ の静的初期化）の 1 項目 `0x180001000` の本体。CPU 情報の初期化（先の調査 §5）。**`f_18000abb0` の先頭付近で `gs:[0x58]` と `_tls_index` を読み、`[blk+8]` を読む**（`0x18000abc2`〜`0x18000abdc`）。**`__Init_thread_header`/`_footer` の呼び出し（`0x18019dc58`、`0x18019dbf8`）が続く** |
| `f_18019dbf8`（`__Init_thread_footer` に見える） | `f_18000abb0` から | **はい（`0x18019dc1b`）** | `EnterCriticalSection(0x1801f14d0)` → `edx = _tls_index` → `[0x1801f0000]` を +1 → `[blk+8] = [0x1801f0000]` → `LeaveCriticalSection` |
| `f_18019dc58`（`__Init_thread_header` に見える） | `f_18000abb0` から | **はい（`0x18019dc92`。ただし条件つき）** | `cmp [rbx],0; jne …; cmp [rbx],-1; je …`（ガードの状態）の `jne 0x18019dc84` 側だけで `gs:[0x58]` と `_tls_index` を読み `[blk+8] = [0x1801f0000]` を書く |

- 残りの 10 関数（`f_180038d90`、`f_180081700`、`f_1800858b0`、`f_180086e40`、`f_180089550`、`f_18008ba50`、`f_180097190`、`f_1800a3430`、`f_1800a4120`、`f_180146ff0`）は、**ATTACH の直接辺では届かない**（`reach-attach.txt` の「gs:[0x58] functions 13 reached 3」と一致）。これらは Blend2D の描画の本体側（Agg2D の関数、`f_180081700` は 8 か所、`f_1800a3430` は 7 か所）で、ATTACH の**後**に `AggCreateInstance` や Agg の vtable から呼ばれる（読み）。実際に呼ばれるかは**実行していない**。
- **`f_180199d90` の中の最初の `lock xadd [0x1801f3110]` は 1 回目の呼び出しだけが先へ進むガード**（`0x180199db6`〜`0x180199dc2`）。
- **したがって ATTACH（DllMain → `.xc`）の直線上で、`_tls_index` と `gs:[0x58]` の読みに必ず届く**（`.xc` の項目 `f_180001000` → `f_180199d90` → `f_18000abb0`。分岐 `jne 0x18019b08c`（`0x180199dc2`）は、`lock xadd` の戻りが 0 でなければ別の道。1 回目は 0 なので直線）。**DllMain が `_tls_index` を使わずに終わる経路は、`.xc` を通る限り無い**（読み）。ただし **DllMain から `.xc` / `.xi` の表の項目への呼び出しは、CRT の `_initterm` が表を走査する間接呼び出し**（`.rdata` の表 0x1801b7328〜0x1801b7358 に、種 5 つのアドレスが並ぶ。`tls-reach.txt`）で、上の「直接辺の到達」の種に表の項目を**手で足している**（`reach-attach.txt` と同じ種の取り方）。DllMain のエントリ `0x18019e584` から `.xc` の項目までの辺は、この調査では**自分では辿っていない**（先の調査 §6 の読みを引いている。**未確認**）。`.xi`（4 項目）が `.xc` より先に走る順序は MSVC の CRT の慣習で、**この DLL の `_initterm` の呼び出し順は読んでいない（未確認）**。

### 4.5.1 参考: 先に走る `.xi` の `f_18019db00`（読み。`tls-reach.txt` と先の調査 §6 の 3）

`InitializeCriticalSectionAndSpinCount(0x1801f14d0, 0xfa0)` は、`__Init_thread_header`／`_footer` が `EnterCriticalSection(0x1801f14d0)` で使うロック（§4.5 の表）と**同じアドレス**（`0x1801f14d0`）。`.xi` が先なら、`.xc` の `f_18019dc58`／`f_18019dbf8` に入るときにはロックは初期化済み（読み）。

### 4.6 import と TLS API（実測。`tls-reach.txt`）

Blend2D は import に **`TlsAlloc`、`TlsFree`、`TlsGetValue`、`TlsSetValue`**（4 つ。`FlsAlloc` 系は無い）を持つ。各 IAT スロットを通る `jmp`/`call` は `f_1801a1d88`（TlsAlloc）、`f_1801a1dd0`（TlsFree）、`f_1801a1e18`（TlsGetValue）、`f_1801a1e60`（TlsSetValue）と、`f_1801a99c4`、`f_1801a9a0c`、`f_1801a9a54`、`f_1801a9a9c`（同じ 4 つの 2 つ目のコピー。UCRT と VCRuntime の 2 組）。つまり **CRT の `TlsAlloc` 系の使用（`__vcrt_` の TLS と `__acrt_` の TLS に見える。読み）** と、暗黙の TLS（`_tls_index`）が**併存**する。

### 4.7 FLEngine と dsp_ippv2 の TLS ディレクトリ（実測）

`pefile`（fast_load、データディレクトリ 9）: **FLEngine_x64.dll は RVA 0 / サイズ 0（`DIRECTORY_ENTRY_TLS` なし）、ImageBase 0x400000**。**dsp_ippv2_x64.dll は RVA 0 / サイズ 0（なし）、ImageBase 0x180000000**。**Blend2D.dll は RVA 0x1e1c00 / サイズ 0x28（あり）**。先の調査 §3.1 と §6 の記述と一致。

### 4.8 `gs:[0x30]`（TEB の Self）と `gs:[0x10]`（実測。`tls-gs30.txt`）

TLS とは別だが、同じ `fle_gs_ld8` を通るので記録する。`gs:[0x30]` は 2 か所: (a) `0x18019def1`（`mov rax,gs:[0x30]; mov rcx,[rax+8]`。**`NT_TIB.StackBase`（+0x8）を Self 経由で読む**。直後は `lock cmpxchg [0x1801f…]` で「最初に入ったスレッドの識別」に見える。読み）、(b) `0x1801a1ba7`（`mov rax,gs:[0x30]; mov rcx,[rcx+0x98]; cmp rcx,[rax+0x10]; jb …; cmp rcx,[rax+8]; jbe …; mov ecx,0xd; int 0x29`。**`StackLimit`（+0x10）と `StackBase`（+0x8）で範囲検査**し、範囲外なら `int 0x29`（fail-fast）。MSVC の `_chkesp` 系のスタック検査に見える。読み）。`gs:[0x10]` は `0x1801b18cc`（`__chkstk` の形）。**Self 経由の +0x8、+0x10 は決定 0076 が `fle_teb[0x8]`、`[0x10]` に入れた値で答えられる**（決定 0076 §「実装」。ここでは確認のみで、実行していない）。**それ以外の TEB のオフセットを Self 経由で読む命令は、この 2 か所には無い**（上のとおり +0x8、+0x10 のみ）。

## 5. 現行 runtime で Blend2D を ATTACH したときに何が起きるか（**読み。確かめていない。実行していない**）

**先に、別作業の実測が示す前提（読み取りのみ。この調査では再実行していない）**: 並行して走っている 3 モジュールの測定（`docs/investigations/three-module-blend2d.md` §6。`lift/out-three/run/ro2-imports.txt`）は、現行の runtime で Blend2D の ATTACH が **13 回の import の後、`.pdata` に無い葉関数（調査 B5）が生成 C に無く `fle_dispatch_miss` で止まった**と記録している。つまり**現状の生成 C（縮小）では、ATTACH は暗黙の TLS の読みに届く前に止まる**（同文書 §6.2 の 9。実測）。以下の §5.1 以降は「B5 が解けて ATTACH がさらに進んだ場合」の読みで、**その場合の到達自体が未測定**。

### 5.1 実測として確認できた順序（別作業の記録から。この調査では再実行していない）

`ro2-imports.txt`（`call_dllmain entered idx=0 reason=1` から `idx=1 reason=1` まで）と `ro1-stdout.txt` の entry log:

- FLEngine のエントリ `f_41d8f0`（entry 3）の後、**`f_41d050`（entry 8）→ `f_41cf40`（TlsAlloc のスタブ。entry 9）→ `f_41d000`（entry 10）→ `f_41cfd0` → `f_41cf20`（LocalAlloc）→ `f_41cf70`（TlsSetValue）→ `f_41cf60`（TlsGetValue）**（`lift/out-three/run/ro1-stdout.txt:40-47`）。つまり **FLEngine の `TlsAlloc`（index 0）は、dsp_ippv2 の ATTACH（`call_dllmain entered idx=0`）より前に実行されている**（InitTable の実行順で `f_41d050` が先。実測の記録）。
- dsp_ippv2 の DllMain（`idx=0`）の import 13 番目までに **`TlsAlloc`（11 番目）と `TlsSetValue(rcx=0x1, …)`（13 番目）**（`ro2-imports.txt:18,21`）。**`TlsSetValue` の index は 1**。→ dsp_ippv2 の `TlsAlloc` は **index 1** を返した（`TlsSetValue(1, …)` から。FLEngine の 0 に続く）。
- Blend2D の DllMain（`idx=1`）の `TlsAlloc`（通し番号 1536）と **`TlsSetValue(rcx=0x2, rdx=0x48f1590, …)`**（通し番号 1538。`ro2-imports.txt:2013,2016`）。**`TlsSetValue` の index は 2**。→ Blend2D の CRT の `TlsAlloc` は **index 2** を返した。

**まとめ（実測の記録からの導出）**: 現行の 3 モジュール構成では、動的 TLS の index は **FLEngine = 0、dsp_ippv2 = 1、Blend2D = 2**（`fle_tls_values[0..2]`）。FLEngine の `fle_tls_values[0]` には `LocalAlloc(0x40, 0x32a8)` のブロックのポインタが入っている（読み。`0x41d041` の `TlsSetValue([0x13a9388], rax)`。FLEngine の `TlsSetValue` の実際の引数値を、この調査では**測っていない**。`ro2-imports.txt` は dsp_ippv2 の ATTACH 以降のログ）。

### 5.2 B5 が解けて ATTACH が `f_18000abb0`（`.xc`）に届いたとき（読み）

現行の runtime（本文書 §2）と、Blend2D のバイト列（§4）を重ねる。

| ステップ | 何が起きるか | 区分 |
|---|---|---|
| 1 | `0x18000abc2` `mov rax, gs:[0x58]` → 生成 C は `R[0] = fle_gs_ld8(UINT64_C(0x58))`（`blend2d-pdata.c:14925`）→ `fle_tls_values` のアドレス（`fle_gs_ld8` は呼ぶたびに `fle_teb` にも書く） | 読み（生成 C の行は実測） |
| 2 | `0x18000abcd` `mov ecx, [rip+_tls_index]` → `R[1] = (uint32_t)fle_ld(UINT64_C(…), 4)`。**今回のオリジナルの `Blend2D.dll` から作った生成 C**（`blend2d-pdata.c:14927`）は `0x1801f1560` を持つが、**実在モジュールとして載せるのは、ファイル段階の移動後（基底 0x4700000）のファイルから作った生成 C** で、`_tls_index` の VA は `0x48f1560`（§4.1 の 2 つ目の表。別作業の測定が `InitializeCriticalSectionAndSpinCount(0x48f1618, …)` と `TlsSetValue(2, 0x48f1590)` を記録しているのと同じ基底）。**この 4 バイトは `harness/image.c` が `.data` の生データ（ファイル上の値 0）をコピーしたまま**（R11）。書く処理は無い（R9、R11、§4.2）→ **`_tls_index` = 0** | 読み（`fle_realmod.c`、`image.c`、`tls-dir.txt` から） |
| 3 | `0x18000abd8` `mov rcx, [rax + rcx*8]` → `R[1] = fle_ld(R[0] + R[1]*8, 8)` = `fle_tls_values[0]` = **FLEngine が `TlsSetValue(0, p)` で置いたブロックのポインタ `p`**（FLEngine の `TlsAlloc` の index が 0 であることと、`0x41d000` が `LocalAlloc(0x40,0x32a8)` を `TlsSetValue([0x13a9388], …)` で置くことからの読み。§2.1、§5.1。FLEngine がそれを `TlsFree` していなければ） | 読み（FLEngine の `TlsSetValue(0, p)` の `p` の値は未測定） |
| 4 | `0x18000abdc` `mov eax, [rdx + rcx]`（`rdx = 8`）→ `fle_ld(8 + p, 4)` = **FLEngine のこのブロック（12,968 バイト）の +8 の 32 ビットを `_Init_thread_epoch` として読む**。`p` が非 NULL なら**メモリアクセスは通り**（FLEngine が `LocalAlloc(LMEM_ZEROINIT)`（0x40 = `LMEM_ZEROINIT`）で 0 初期化したブロックなので、読む値は FLEngine が書いた値か 0）。`p` が 0（FLEngine が先にここへ来て `TlsSetValue(0, …)` が呼ばれていない、または `TlsFree(0)` された）なら**アドレス 8 を読む**（native は SIGSEGV、wasm は `a >> 32 == 0` で通りリニアメモリの 8 番地を読む。R8）。**どちらの場合も、「ガード変数 `0x1802f4b88`（`.data`、0）と `cmp`」へ進む**（`cmp [guard], eax; jle skip`）。`eax` が 0 で guard が 0 なら `0 <= 0` で `jle` を取り**スキップ**（= 「初期化済み」と見なして `__Init_thread_header` を呼ばない）。`eax` が 0 でないと分岐が変わる | **読み**。FLEngine のブロックの +8 の値は**測っていない**。初期値 0 は `LMEM_ZEROINIT`（`0x40`、`fle_imports.c:46` の `FLE_LMEM_ZEROINIT 0x0040u`）による |
| 5 | スキップされると、`f_18000abb0` の「静的ローカルの初期化」（ガード 0x1802f4b88 を `0` にして `call 0x18019dbf8`）が**実行されない**。通常（Windows の実機）は `_Init_thread_epoch` の初期値 `0x80000000` と `guard = 0`（`.data` 0 初期化）から `guard (0) > epoch (INT_MIN)` となり `jle` が**取られず**に `__Init_thread_header` が呼ばれて初期化が走る（`0x80000000` は符号つき 32 ビットで最小値。`cmp [guard], eax; jle` は符号つき比較）。**現行の runtime はテンプレートを配らないので、`eax` の初期値が 0x80000000 にならない。結果、「静的ローカルの初期化が黙って飛ばされる」ことになりうる**（読み。`jle` が符号つきで、初期値 0x80000000 と 0 の比較の向きが違う、という導出。**実機の Windows での動作は確認していない**） | 読み（命令の意味から）。**この「黙って飛ばされる」が実害になるか（`f_18000abb0` が初期化する 1 組の静的変数 0x1802f4af0〜0x1802f4b78 は CPU 情報のテーブルに見える。先の調査 §5）は未確認** |
| 6 | `__Init_thread_header`（`f_18019dc58`）／`_footer`（`f_18019dbf8`）が実行された場合: `EnterCriticalSection(0x1801f14d0)`（`InitializeCriticalSectionAndSpinCount` で初期化済み。§4.5 のとおり `.xi` 先頭の `f_18019db00`）→ `_Init_global_epoch`（0x1801f0000）の +1 → `[blk+8]` に書く。**`blk` = `fle_tls_values[0]` の指す FLEngine のブロックの +8 に書き込む**（FLEngine のブロックの内容を**壊す**。12,968 バイトのブロックの意味は Delphi/FLEngine 側で、+8 の 32 ビットを何に使っているかは**未確認**） | 読み。**FLEngine の TLS ブロックを Blend2D が書き換える**ことになる（衝突） |

### 5.3 `fle_addr` の検査（R8、決定 0045、0049）

- **native**: `fle_addr` は恒等。`fle_ld(8, 4)` は SIGSEGV（`p == 0` のとき）。`p != 0` なら通る。
- **wasm（`FLE_CHECK_HI32`、既定で有効）**: `a >> 32` が非 0 のときだけ `fle_addr_fault`（`kind = 1`）。`a = 8 + 0 = 8` や `a = p + 8`（`p` は `LocalAlloc` の戻り。wasm ではリニアメモリの 32 ビットの範囲）は通る。**0 番地付近は通る**（NULL を検出しない。決定 0045 §「領域チェック `FLE_CHECKED_MEM`」は、データ領域未満（`x < __global_base` かつイメージ外）を `fle_addr_region` で止める。**`FLE_CHECKED_MEM` はオプション（既定オフ）**。有効にすれば `a = 8` は止まる（`x - fle_mem_img_lo >= fle_mem_img_size && x < &__global_base`））。決定 0049 は `fle_addr_opt`（NULL を通す）を足したが、**`fle_ld`／`fle_st` は `fle_addr`（NULL を通さない）を使う**（`fle_runtime.h:66-74`）。つまり `FLE_CHECKED_MEM` を有効にしたビルドでは `fle_ld(8, 4)` が `[fle_addr_region]` で名前付き停止し、既定の wasm と native では通る（native は SIGSEGV）。
- **したがって（現状のまま = 選択肢 A）の結果は、ビルド設定によって 3 通り**: native は p が 0 なら SIGSEGV（名前なし）、既定 wasm は黙って 8 番地を読む（fail-open）、`FLE_CHECKED_MEM` 付きは名前付き停止。いずれも **`_tls_index` を読んだ時点では何も止まらない**（読みは 0 を得る）。

### 5.4 B6（調査 §6）の記述の訂正（この調査の読み）

調査 `blend2d-lift-feasibility.md` §6（行 250）は「`fle_tls_values[0]` は `TlsAlloc` が最初に返す index 0 のスロットと同じ場所で、`TlsSetValue(0, …)` で別の用途の値が入るか、未設定なら 0」と書く。**この調査の読みは、これに「FLEngine の `TlsAlloc` は 0x41d050 の 1 回で、ATTACH より前に index 0 を取り、`0x41d000` がブロックを入れる」を足す**（§2.1、§5.1。実測の記録）。したがって「未設定なら 0」の場合は、**現行の実行順では起きない**（読み）。`fle_tls_values[0]` は非 0 のはずで、**0 番地付近の読みにはならず、FLEngine のブロックの読み書きになる**（§5.2 の 4〜6）。**どちらの結果も、実測していない**。

## 6. モデル化の選択肢（事実の表。**決めない**。意見は 6.7 に別枠）

### 6.0 選択肢を比べるための共通の事実

| # | 事実 | 区分 |
|---|---|---|
| F1 | Blend2D の TLS ブロックが要る大きさは **32 バイト**（テンプレート 32 B + zero fill 0）。実際に触るのは +0x08（32 ビット）と +0x18（64 ビット）だけ（§4.4）。アライメントは Characteristics で 8 バイト（§4.1） | 実測 |
| F2 | `_tls_index` を**読む**命令は 23 件、**書く**命令は 0 件（§4.2）。`gs:[0x58]` の読みは 26 か所（13 関数）で、index は 26/26 が `_tls_index` 由来（§4.3） | 実測 |
| F3 | ATTACH の直線上（`.xc` → `f_180199d90` → `f_18000abb0`）に `_tls_index`、`gs:[0x58]` の読みがある（§4.5。直接辺の到達。読み）。FLEngine は TLS ディレクトリを持たず、`gs:[0x58]` は 1 件（§4.7、§2.1）。dsp_ippv2 は持たない | 読み／実測 |
| F4 | 現行の生成器は `gs:[0x58]` を `R[n] = fle_gs_ld8(0x58)` に、直後の `mov rcx,[rax+rcx*8]` を `fle_ld(R[0]+R[1]*8, 8)` にする。`_tls_index` の読み（`fle_ld(0x…1560, 4)`）も普通のロード（§5.2）。**生成器は TLS を知らない**。生成 C に `gs:[0x58]` を持つのは FLEngine で 1 件、Blend2D で 26 件 | 実測（生成 C 行）／読み |
| F5 | 現行の動的 TLS の index は FLEngine = 0、dsp_ippv2 = 1、Blend2D = 2（別作業の実測の記録。§5.1）。`fle_tls_values` は 1,088 個（`TlsAlloc` の上限と同じ）の `uint64_t` 配列 | 実測の記録の引用 |
| F6 | `fle_realmod.c` の `read_headers` は export だけを読む。`RM_MAX = 4`。ロードは `fle_realmod_load` → `call_dllmain(idx, 1)`。TLS の機会（ATTACH の前に呼ぶコールバック、スレッドごとの配布）は無い（§2 R9、R10） | 読み |
| F7 | FLEngine の生成 C の SHA-256 は決定 0078 以降 `43985d62321a90f562ce1b8144253e00af13df795e4190d26b6b8ce2537137a1`（決定 0088 まで不変）。**この調査では `fe-lift lift` を実行していない**ので、以下の「変わるか」は**読み**（生成 C を変える変更が何かの読み） | 引用（`progress.md:28`、決定 0088 §検証）／読み |
| F8 | 決定 0032 の `gs:[0x58]` の答えは「`fle_tls_values` の先頭」。これは FLEngine の 1 件のための**作業者の判断**で、`vex.rs`、`fle_vex.h`、テスト 2 本（`vex.rs:528`、`gs_stacklimit.rs:183`）が固定している（§2 R1、R15）。FLEngine の `gs:[0x58]` は ATTACH の前（`[0x15ec2d8] == 0`）にだけ実行される形（§2.1。実行は未確認） | 読み |
| F9 | wasm では `fs`/`gs` はホストのものを使わない（`fle_teb`、`fle_tls_values` は runtime の静的バッファ。決定 0045 §領域チェック）。`fle_addr` は wasm で上位 32 ビットを検査する（R8） | 読み |
| F10 | Learn は `_tls_index` の払い出しの規則、TLS 配列の長さ、ブロックを配るタイミング、TLS コールバックと DllMain の順序、x64 の `ThreadLocalStoragePointer` のオフセットを**書いていない**（§3.2 の表） | 実測（ページの確認） |

### 6.1 選択肢 A: 現状のまま（何もしない）

| 項目 | 内容 |
|---|---|
| 内容 | runtime、生成器、harness を変えない |
| 必要な変更の場所 | なし |
| 起きること（読み。§5） | B5 が解けて ATTACH が `f_18000abb0` に届くと、`_tls_index` = 0、`gs:[0x58]` = `fle_tls_values`、`[rax+0*8]` = `fle_tls_values[0]` = FLEngine の `TlsSetValue(0, p)` のブロック `p`（FLEngine の 0x32a8 B の領域）。`mov eax,[8+p]` は **FLEngine のブロックの +8 を `_Init_thread_epoch` として読む**。`__Init_thread_header`/`_footer` が走れば**同じブロックの +8 に書く**（FLEngine のデータを壊す可能性）。`p == 0` なら native は SIGSEGV、wasm は 8 番地を読む |
| 停止するか | **しない**（名前付き停止なし）。結果は「静的ローカルの初期化の判定が、FLEngine のブロックの中身に依存する」。fail-open |
| FLEngine の生成 C | 変わらない |
| テスト | なし（新しい挙動を足さない） |
| スレッド（案 A の JSPI 版）との関係 | スレッドごとの TLS が無いので、仮想スレッドを足すと `fle_tls_values` の切り替え（R16）とは別に、暗黙の TLS の扱いが未定義のまま |
| 決定 0032 との整合 | 変えないので整合する（ただし 0032 の判断は FLEngine 1 件向けで、Blend2D の 26 件には合わない。F8） |
| wasm | 上の「起きること」のとおり。既定の wasm では 8 番地の読みが通る（fail-open）。`FLE_CHECKED_MEM` 付きなら止まる |
| 失うもの／リスク | 初期化の結果が FLEngine の TLS ブロックの内容に依存し、**FLEngine 側のブロックを Blend2D が書き換える**。実測していない（読み） |

### 6.2 選択肢 B: ロード時に `_tls_index` を書き、テンプレートから TLS ブロックを作って配列に置く

内容の骨子（どの B も共通）: (1) 実在モジュールのロード時（`fle_realmod_load` の `call_dllmain(idx, 1)` の前）に、TLS ディレクトリ（データディレクトリ 9）を読み、(2) `*(uint32_t*)AddressOfIndex = index`、(3) `size = End − Start + SizeOfZeroFill` のブロックを確保して `memcpy(block, Start, End − Start)`、残りを 0 で埋め、(4) `gs:[0x58]` が返す配列の `index` 番に `block` のアドレスを置く。

**B の中の分岐点（事実として並べる。Learn に書かれていない部分が決めどころ。§7）**:

| 分岐点 | 案 | 事実・制約 |
|---|---|---|
| **B-1 配列の置き場所** | (a) 既存の `fle_tls_values` の `[index]` | **動的 TLS の index（`TlsAlloc`）と衝突する**。`TlsAlloc` は `fle_tls_next_index` が 0 から払い出すので、`fle_tls_values[0..2]` は FLEngine、dsp_ippv2、Blend2D の動的 TLS で既に使われる（F5）。暗黙の TLS の index をこの配列に置くと、**同じ添字を `TlsGetValue` と `gs:[0x58]` が別の意味で使う**。回避には、(i) 暗黙の index を `TlsAlloc` が払い出さない範囲（例: 後ろ）に置き `fle_tls_next_index` の上限を下げる／予約する、(ii) `TlsAlloc` 側で暗黙の index を飛ばす、のどちらかが要る。(i) は `TlsAlloc` の「最大 1,088」（M2）との整合も要る |
| | (b) 別の配列（例: `fle_implicit_tls[N]`）を作り、`fle_gs_ld8(0x58)` がそちらの先頭を返す | **FLEngine の `gs:[0x58]`（`[rax + idx*8]`、idx = `TlsAlloc` の値）が `fle_tls_values` を引けなくなる**（FLEngine の 1 件の経路が `TlsGetValue(idx)` と同じ値を返す、という決定 0032 の想定が壊れる）。FLEngine の経路は ATTACH の前にだけ実行される形（F8）なので、**ATTACH の前は `fle_tls_values`、後は別の配列、と `gs:[0x58]` の答えを切り替える案**もありうる（ただし FLEngine の 230 の呼び出し元のうち ATTACH の後に実行される経路が無いことの確認が要る。未確認） |
| | (c) 1 本の配列にして、**添字の空間を分ける**（`TlsAlloc` の index を 0..N−1、暗黙の index を N 以上、など）。`gs:[0x58]` の答えは 1 本の配列の先頭 | FLEngine の `[rax+idx*8]` はそのまま通る。`TlsGetValue`/`TlsSetValue`/`TlsFree` の `fle_tls_allocated` と、`FLE_TLS_MAX_INDEXES`（1,088）の再配分が要る。決定 0078 の `TlsFree` の「再利用しない」はそのまま |
| **B-2 index の払い出し** | (a) 暗黙 TLS を持つ実在モジュールのロード順に 0, 1, 2…（ReactOS 流。U2） | 動的 TLS の index と空間を分けないと B-1(a) の衝突 |
| | (b) 空きの最小（Wine 流。U3） | 同上 |
| | (c) 暗黙 TLS の index を**動的 TLS と同じ `TlsAlloc` の払い出しで取る**（`fle_tls_next_index` を使い、`_tls_index` に書く） | 空間が 1 つで衝突しない。**「暗黙の index は `TlsAlloc` の index と同じ番号空間」という Windows の仕様**（Learn は書かない。U1 は「別の仕組み」と書く）**に反する**が、現行の runtime の `gs:[0x58]` と `TlsAlloc` の関係（決定 0032）とは同じ考え方 |
| | 複数の実在モジュールが暗黙 TLS を持つ場合の順序 | Learn は書かない（F10）。今の 3 モジュールで暗黙 TLS を持つのは Blend2D の 1 つだけ（§4.7） |
| **B-3 ブロックのメモリの置き場** | (a) `calloc`/`malloc` | 現行の `LocalAlloc` と同じ。32 バイト。native では高いアドレス（恒等写像）、wasm ではリニアメモリ内。**`fle_addr` の 32 ビット制限を超えない**（wasm は 32 ビットのアドレス）。アライメントは `malloc` の 16 バイトで Characteristics の 8 を満たす |
| | (b) 仮想アリーナ（決定 0017。`FLE_VMEM_ARENA`、wasm のみで 512 MiB を 1 ブロック取って `VirtualAlloc` が切り出す）から | ゲストの `VirtualAlloc` の領域と同じ空間。`FLE_VMEM_ARENA_BYTES` を食う（32 バイトのために 64 KiB の `FLE_ALLOC_GRANULARITY`（`fle_sys.h:29`）を使うと無駄が大きい） |
| | (c) 静的配列（`uint8_t fle_implicit_tls[N][64]` など） | 確保が要らず決定的（再現性）。**実在モジュールの数（`RM_MAX = 4`）と各ブロックの最大サイズを事前に決める**必要。32 バイト × モジュール数なら小さい。サイズ超過は名前付き停止にできる |
| **B-4 SizeOfZeroFill、アライメント** | `SizeOfZeroFill` = 0（Blend2D）。一般には `ブロック = End − Start + SizeOfZeroFill` で、テンプレートを `memcpy` して残りを 0 | Learn: "The total template size should be the same as the total size of TLS data in the image file" と "The system copies all of this data each time a thread is created"。**ReactOS の `LdrpAllocateTls` は zero fill を足していない**（U2。読み）が、Wine は足す（U3）。Learn が zero fill の扱いを明示するのは Size of Zero Fill の説明のみ。アライメント: Characteristics の [23:20]（Blend2D は 8 バイト）。**配置の義務は Learn に明示が無い**（Wine は使わない（U3）。ReactOS も使わない（U2。読み）） |
| **B-5 いつ書くか** | (a) `fle_realmod_load` の `call_dllmain(idx, 1)` の直前（`ST_LOADING` に入る前後）。Wine は `fixup_imports` の最初（DllMain より前。U3） | `fle_realmod.c:136-138` の間。**`read_headers` に TLS ディレクトリの読みを足す**（データディレクトリ 9 = `opt + 112 + 8*9`）か、別関数にする |
| | (b) `image_load`（harness）が TLS ディレクトリを読む | `image_load` は import の IAT を埋め（R11）、イメージを置く場所なので、`_tls_index` を書く場所としても自然。ただし harness は FLEngine 用と実在モジュール用で共用（`main_two.c`）。**harness は実在モジュールのロード（ゲストの `LoadLibraryW`）より前に動く**ので、ロード時に index を払い出す案（B-2）とは時点が合わない（`_tls_index` を先に書くと、ロードの前に index が確定する） |
| | (c) 生成器（fe-lift）が `fle_realmod_table` 相当の表を出し、TLS の情報（Start、End、Index、ZeroFill、Characteristics）を C の定数にする | `fle_realmod_table`（`harness/two/realmod_table.c`）は手書き／生成（`gen_two.sh`）で名前と基底のみ。TLS の 5 値を足すなら、**ここ（リンク時の表）に載せる**のが `fle_realmod.h` の方針（「Nothing is registered at run time and the generator is not involved」）と合う |
| **B-6 fe-pe の変更** | `lift/crates/fe-pe/src/lib.rs` に TLS ディレクトリの構造体（6 フィールド）を読む関数（`data_dirs[9]` と `.rdata` の RVA 変換）を足し、`fe-pe pe-tls-info`（`pe-pdata-info` に倣う）のような表示を足す | 現状 fe-pe に TLS の語は無い（R13）。rebase（`rebase.rs`）は 4 つの VA を既に動かしている（§4.1）ので、**rebase の変更は要らない**（読み） |
| **B-7 コールバック** | Blend2D の `AddressOfCallBacks` の先頭は 0（空）。B で**コールバックを呼ぶ機能は不要**だが、呼ばない場合は「空でないコールバックを持つ実在モジュールは名前付き停止」にする、が fail-closed の形 | 実測（Blend2D は空）。他のモジュール（QFC など）は未調査 |

| 項目 | 内容 |
|---|---|
| 必要な変更の場所 | (i) `fe-pe`（TLS ディレクトリの読み）、(ii) runtime の `fle_realmod.c`（`fle_realmod_load` の `call_dllmain` の前に `_tls_index` の書き込みとブロックの作成）と、`fle_vex.h` の `fle_gs_ld8`（`gs:[0x58]` の答え。B-1 の選択で変わる）と `fle_imports.c`（`fle_tls_values`、`TlsAlloc` の払い出し。B-1(a)(c) の場合）、(iii) リンクの表（`harness/two/realmod_table.c`、`gen_two.sh`。B-5(c) の場合）または `fle_realmod.c` の `read_headers`（B-5(a) の場合）。`harness/two/main_two.c` は (b) の場合のみ |
| **FLEngine の生成 C が変わるか** | **変わらない見込み**（読み）。変更は runtime と fe-pe（生成器 `fe-lift` は変えない）。`fle_gs_ld8` の本体を変えても、生成 C の `R[0] = fle_gs_ld8(UINT64_C(0x58));` の行は同じ。**ただし**: `vex.rs` の `mov_seg` や `fe-lift` の `imports_table.rs` を触る場合（例: `gs:[0x58]` を別の関数名にする、`TlsAlloc` の応答を変える）は生成 C が変わる可能性がある（決定 0078 は応答表の行を足して SHA-256 が変わった前例）。**確認は `fe-lift lift … --pdata` の SHA-256（基準 43985d62…37137a1）で行う**（この調査では未実行） |
| テストの書き方 | (1) fe-pe: 合成 PE（`lift/crates/fe-lift/tests/realmod_s1/` に倣う。mingw で作る）に TLS ディレクトリを持たせ、構造体の読みの単体テスト。実在の Blend2D の TLS ディレクトリ（§4.1 の値）の照合は `lift/out-blend2d/Blend2D.dll`（SHA-256 固定）で。(2) runtime: `tests/realmod.rs` の形（合成 PE を 2 つ作ってリンク）で、`_tls_index` に index が書かれる、`gs:[0x58]` + `[idx*8]` + `[8]` がテンプレートの +8 を返す、`TlsAlloc` の index と衝突しない、2 つ目の暗黙 TLS モジュールで別の index、`SizeOfZeroFill` が 0 で埋まる、ブロックの変更が別のモジュールに見えない、空でないコールバックは名前付き停止、の組み合わせ。(3) 既存の `vex.rs:528` と `gs_stacklimit.rs:183` のテストが（B-1 の選択によっては）変わる。(4) 実機（Windows）の突合は**できない**ので、期待値は Learn と §4 の実測から書く |
| スレッド（案 A の JSPI 版）との関係 | **スレッドごとに TLS ブロックを作る必要が出る**（Learn: "The system copies all of this data each time a thread is created"）。B で `gs:[0x58]` が返す配列を `fle_teb` ごとスレッド別にし、切り替え時に入れ替える（R16 の「仮想スレッドごとの大域状態」に `fle_tls_values` と同じく加わる）。**単一スレッドの間は 1 組で済む**（案 C。スレッド生成で停止）。TLS コールバックの `DLL_THREAD_ATTACH` は Blend2D では空なので不要 |
| 決定 0032 との整合 | B-1(a)(c) は「`gs:[0x58]` = `fle_tls_values` の先頭」を保てる。B-1(b) は 0032 の判断を変える（FLEngine の 1 件の経路が別の配列を引く）。**B-1(c)（1 本の配列で添字空間を分ける）が 0032 の「最小の答え」を一番変えない**（`gs:[0x58]` の答えは同じ。`TlsAlloc` の払い出し範囲だけが変わる）、が読み |
| wasm | `fle_teb`/`fle_tls_values`（またはその代わりの配列）は runtime の静的バッファで、ホストの `gs` を使わない（F9）。**ブロックのアドレスは 32 ビットに収まる**（`malloc`/静的配列）。**`FLE_VMEM_ARENA`（B-3(b)）は wasm 専用**で、native は `mmap`（同じコードを native で走らせる手段は `FLE_VMEM_ARENA` を手で立てる）。B-3(c)（静的配列）はどちらでも同じ |
| 失うもの／リスク | 仕様（Learn）が書かない index の払い出しを**作る**ことになる（§7）。FLEngine の `gs:[0x58]` 経路（230 の呼び出し元のうち実行されるもの）を壊さないことの確認が要る |

### 6.3 選択肢 C: 名前付き停止（`_tls_index` を読んだとき、または `gs:[0x58]` が DLL の TLS を要求したとき）

停止の仕掛けの場所は少なくとも 4 通りある（事実として並べる）:

| 停止の場所 | 仕掛け | 検出できること／できないこと |
|---|---|---|
| C-1 **ロード時**（`fle_realmod_load`） | `read_headers` が TLS ディレクトリ（データディレクトリ 9）を読み、`size != 0` なら `fle_stop("LoadLibraryW", "the real module has a TLS directory (implicit TLS is not modelled)")` | **確実に早く止まる**（ATTACH の前）。FLEngine、dsp_ippv2 は TLS ディレクトリを持たないので影響しない。**Blend2D は必ず止まる**（TLS ディレクトリがあるので）。つまり Blend2D の ATTACH の他の項目（`GetModuleHandleA`、ロケール、B5 など）を測る前に止まる。**ATTACH の他の停止点の測定を妨げる**（回避は「TLS ディレクトリの停止を無効にする環境変数」など別の仕掛けが要る） |
| C-2 **`_tls_index` を読んだとき**（生成 C の `fle_ld(0x…1560, 4)`） | 実在モジュールのロード時に `_tls_index` のセルを**特別な値**（例: 0xFFFFFFFF）にしておく。すると生成 C の `fle_ld(R[0] + R[1]*8, 8)` のアドレスは `fle_tls_values` + 0x7FFFFFFF8 で、**上位 32 ビットが非 0**（読み。計算）。wasm（`FLE_CHECK_HI32` 既定）なら `fle_addr_fault`（kind 1）で止まる（`fle_runtime.h:45`）、native は `fle_addr` が恒等なので未マップのアドレスの SIGSEGV（名前なし）になる見込み（**どちらも実行していない**）。つまり**ホストによって止まり方が違い、名前付きにできるのは wasm だけ** | `fle_ld` は一般のロードで、特定のアドレスを監視する仕掛けは現状無い（R8 の `fle_addr` は 32 ビットの上位と領域の検査のみ）。**生成 C を変えるか、`fle_addr` に仕掛けを足す必要がある**。実用的には「`_tls_index` の値をセンチネルにして、`fle_gs_ld8(0x58)` が返す配列を**ガード領域（保護された領域）に置く**」が考えられるが、wasm には保護ページが無い（決定 0076 の限界）ので、検査は `fle_ld` 側の領域チェック（`FLE_CHECKED_MEM`）頼みになる |
| C-3 **`gs:[0x58]` を読んだとき**（`fle_gs_ld8(0x58)`） | `fle_gs_ld8` が、**呼び出した命令の所属モジュール**を見て、暗黙 TLS を持つ実在モジュールの関数からの呼び出しなら `fle_stop`。**所属の判定は現状できない**（`fle_gs_ld8` は VA を受け取らない。生成 C の呼び出しは `fle_gs_ld8(UINT64_C(0x58))` で、**呼び出し元の命令アドレスは渡されない**。`R[]` の状態からは分からない） | 呼び出しに VA を足すなら**生成器（`vex.rs`）と生成 C が変わる**（FLEngine の 1 件の行も変わり、FLEngine の SHA-256 が変わる）。**FLEngine の SHA-256 を保つには、Blend2D 用の生成だけ別の形（`--seeds` のオプション）にする**必要 |
| C-4 **ATTACH の後の実行時**（`_Init_thread_header` が `[blk+8]` に書くとき） | 検出の仕掛けが無い | — |

| 項目 | 内容 |
|---|---|
| 必要な変更の場所 | C-1: `fle_realmod.c` の `read_headers` 付近（TLS ディレクトリの読み。`fe-pe` は不要で、runtime が PE のヘッダを読む。`read_headers` は既に `opt+112/+116` で export を読んでいる）。C-3: 生成器と生成 C |
| FLEngine の生成 C | C-1 は**変わらない**（runtime のみ）。C-3 は**変わる**（呼び出しに VA を足す場合） |
| テスト | C-1: 合成 PE に TLS ディレクトリを持たせ、`LoadLibraryW` が名前付き停止することと、TLS ディレクトリの無い PE（既存の `realmod.rs` の合成 PE、FLEngine、dsp_ippv2）が影響を受けないこと |
| スレッドとの関係 | スレッドを足す前は不要。足すときに B 系へ進む足場になる |
| 決定 0032 との整合 | 変えない |
| wasm | C-1 はホストに依存しない（PE のヘッダ読みだけ） |
| 失うもの／リスク | **Blend2D の ATTACH が C-1 では先へ進めない**。調査 §10.3 の B13（停止のまま）に当たる。B5 など他の停止点の測定には、TLS の停止を回避する仕掛けが別に要る |

### 6.4 選択肢 D: 生成器が `gs:[0x58]` の読みを別の仕組みに置換

事実として、置換の対象にできる形は 2 つある（どちらも**読み**。実装・実行していない）:

| 案 | 内容 | 事実・制約 |
|---|---|---|
| D-1 `mov rax,gs:[0x58]` 1 命令だけを別の関数 `fle_gs_tls_base(module)` に置換する | 生成器が命令の所属モジュールを知っている（FLEngine なら 0、Blend2D なら 1）という前提で、モジュールごとの配列の先頭を返す | **生成器は 1 つの PE を 1 つの C にする**（`fe-lift lift X.dll out.c`。モジュールごとに別々に走る）ので、**所属は生成時に分かる**（`--module-id` のようなオプションを足せる。読み）。ただし FLEngine の生成 C が変わらないようにするには、既定（オプション無し）で今の `fle_gs_ld8(0x58)` のままにする必要がある。**`_tls_index` の読み（`fle_ld(0x…1560, 4)`）は普通のロードのまま**で、書き手（ロード時の `_tls_index` の書き込み）は結局要る（B と同じ） |
| D-2 `mov rax,gs:[0x58]; mov ecx,[_tls_index]; mov rcx,[rax+rcx*8]` の 3 命令の連なりをパターンで認識して、`rcx = fle_implicit_tls_block(module)` に置換する | `_tls_index` の読みも `[rax+rcx*8]` も要らなくなる（**ローダーが `_tls_index` を書く必要が無い**）。26 か所すべてが同じ形（§4.3）だが、**命令の間に別の命令が挟まる**（例: `f_1800858b0` は gs の読みと block の読みの間に `movups xmm0,[rdi+0x90]`、`mov edx,8` が入る。§4.3）。**3 命令の連なりでなく、データフロー（レジスタの定義-使用）で認識する必要がある**。`f_180097190`、`f_1800a4120` は `add blk, 8` を挟む。`f_180146ff0` は `lea` でなく `[rdx + r8 + 8]` | 生成器にデータフロー解析（`rax` が `gs:[0x58]`、`rcx` が `_tls_index` 由来、のタグ付け）を足す大きな変更。**誤認識の危険**（`_tls_index` の読みと別の用途の読みを取り違える）。26/26 が同じ形である実測（§4.3）は、**この DLL 1 つについて**の事実 |

| 項目 | 内容 |
|---|---|
| 必要な変更の場所 | `fe-lift`（`vex.rs` の `mov_seg`、または新しいパスで連なりを認識）、`fle_vex.h`（新しい関数）、生成 C、runtime |
| **FLEngine の生成 C が変わるか** | D-1 は既定を保てば**変わらない**が、オプションの足し方次第（`mov_seg` の分岐を触るので、生成器のテスト `vex.rs:528`、`gs_stacklimit.rs` を再確認）。D-2 は FLEngine の 1 件（`mov rax,gs:0x58; mov ecx,[0x13a9388]; mov rax,[rax+rcx*8]`）も**同じ形**（§2.1）なので、**FLEngine の生成 C が変わる**（パターンが FLEngine の 1 件にも合う）。FLEngine の `[0x13a9388]` は `TlsAlloc` の値で `_tls_index` ではないため、**誤って置換するとその 1 件が壊れる**。**区別の根拠は、読み先のアドレスが TLS ディレクトリの `AddressOfIndex` かどうか**（`0x13a9388` は FLEngine の TLS ディレクトリの `AddressOfIndex` ではない。FLEngine は TLS ディレクトリを持たない。§4.7）。D-2 では、生成器が PE の TLS ディレクトリを読み、`AddressOfIndex` への読みだけを対象にする必要がある |
| テスト | 生成器の単体テスト（命令列 → 生成 C の文字列）、合成 PE の実行テスト |
| スレッドとの関係 | D-2 は「現在のスレッドのブロック」を返す関数にできるので、スレッド別化の差し込み口になる |
| 決定 0032 との整合 | D-1 は 0032 の関数 `fle_gs_ld8` を別の関数に分ける形で、0032 の「`gs:[0x58]` の最小の答え」は FLEngine に対してそのまま。D-2 は 0032 の `gs:[0x58]` の読みを Blend2D では使わなくなる |
| wasm | 生成 C が C 関数を呼ぶだけで、ホストに依存しない |
| 失うもの／リスク | 生成器の変更が最も大きく、**FLEngine の SHA-256 が変わるリスクが最も高い**（既存の 0078、0088 の運用では SHA-256 の不変を条件にしている）。**生成器は他の作業者が編集中**の範囲（`mmx`、`vex` など） |

### 6.5 一覧（比較の表）

| 観点 | A 現状 | B ロード時に index とブロック | C 名前付き停止（C-1） | D 生成器で置換 |
|---|---|---|---|---|
| Blend2D の ATTACH が `f_18000abb0` を越えるか | 越える（**黙って。FLEngine のブロックを読み書き**） | 越える（テンプレートの値 0x80000000 を読む） | **止まる**（ロード時） | 越える（D-1 は B と同様 `_tls_index` の書き手が要る） |
| fail-closed か | いいえ | ブロックの外を読み書きする命令が無い（実測 §4.4）ので、**配った範囲では閉じている**。ただし配列の添字衝突を作ると開く | はい | D の実装次第 |
| 必要な変更の量（読み） | なし | 中（fe-pe の TLS 読み、runtime、リンクの表。生成器は不要） | 小（runtime の `read_headers`） | 大（生成器） |
| FLEngine の生成 C（SHA-256 43985d62…） | 変わらない | 変わらない見込み | 変わらない | **変わりうる**（D-2 は確実に影響。D-1 はオプション次第） |
| 決定 0032 の最小の答えとの整合 | 変えない | B-1(c) なら保つ。B-1(b) は変える | 変えない | D-1 は FLEngine に対して保つ |
| 暗黙 TLS が 2 つ目のモジュールに増えたとき | 同じ問題が増える | index の払い出しと配列の大きさの決定が要る（§7） | モジュールごとに停止 | モジュール ID の払い出しが要る |
| スレッドを足す将来 | 未定義 | スレッドごとにブロックを作る仕組みが要る（B-3、R16） | 先送り | D-2 は差し込み口になる |
| 実機の Windows との突合 | できない | できない（Learn と §4 の実測から期待値を書く） | できない | できない |
| 6 つの Blend2D 固有の他の停止点（B5、`GetModuleHandleA` など）の測定を妨げるか | いいえ | いいえ | **妨げる**（回避が別に要る） | いいえ |

### 6.6 選択肢の組み合わせ（事実として挙げる。決めない）

- **C-1 を入れてから B に進む**: C-1（TLS ディレクトリを持つ実在モジュールは名前付き停止）は fail-closed の足場で、B を実装するまでの間、暗黙 TLS を持つ未知のモジュールが黙って通るのを防ぐ。B を入れるとき C-1 は「TLS ディレクトリを持つモジュールのうち、B が配れる形（コールバックが空、サイズが上限内）以外は停止」に縮む。
- **B で暗黙 TLS を配り、空でないコールバック、`SizeOfZeroFill` が大きい、Characteristics のアライメントが 8 を超えるものは名前付き停止**: Blend2D は 3 つとも該当しない（§4.1）ので通る。他のモジュールは未調査。

### 6.7 意見（**意見**。事実ではない。決めるものではない）

1. 実在モジュールに TLS ディレクトリが 1 つだけ現れる今の状況（§4.7）では、**B-1(c)（1 本の配列で添字空間を分ける）+ B-3(c)（静的配列）+ B-5(a)（`fle_realmod_load` の中、`call_dllmain` の前）** が、変更の場所が `fe-pe`（TLS 構造体の読み）と runtime に閉じ、FLEngine の生成 C を変えず、決定 0032 の `gs:[0x58]` の答えも保てるように見える。ただし B-1(c) は `TlsAlloc` の払い出しの上限（1,088）の再配分を伴い、`fle_imports.c` の `TlsAlloc`/`TlsFree` の挙動（決定 0002、0004、0078）に触る。**実測していない**ので、この組み合わせが他の組み合わせより良いとは言えない。
2. **C-1 を先に入れる**のは安全側だが、Blend2D の ATTACH の他の停止点の測定を妨げる。**B5（`.pdata` に無い葉関数）が先に解ける**のが順序として自然に見える（別作業の測定が、現状は B5 で止まることを示している）。TLS の停止は、B5 が解けた**後**の次の停止点として現れるはず（読み。実測は無い）。
3. D は、FLEngine の生成 C を変えないことを条件にすると、使いにくい（D-2 は確実に FLEngine の 1 件に当たる）。D-1 のモジュール ID の仕掛けは、将来 `fe-lift` を複数モジュールの同時生成にする（決定 0080 の方向）なら自然だが、今は要らないように見える。
4. **Windows の実機での挙動を確認できない**（仕様の空白）ので、`_tls_index` の値や配列の形は、アプリ（Blend2D）が**この環境で読むだけで動く**ことを基準に決めるしかない。その点は §7 に分けて挙げる。

## 7. 決めるべき点（**列挙のみ**。決めない）

各項目に、Learn が書いているか、現行の事実、選択肢との関係を付す。「書かれていない」は §3.2 の表で確認した事項。

| # | 決めるべき点 | Learn / 事実 | 関係する選択肢 |
|---|---|---|---|
| T1 | **暗黙の TLS を扱うか／止めるか**（A: 何もしない、B: 配る、C: 名前付き停止、D: 生成器で置換、または C→B の順） | 選択肢の表は §6.5。Blend2D の ATTACH は B5（別作業の実測）で先に止まっているので、TLS が効くのは B5 が解けた後（§5。読み） | 全部 |
| T2 | **`_tls_index` の払い出しの規則**（0 から順か、空きの最小か、`TlsAlloc` と同じ空間か、別空間か） | **Learn は書かない**（"The index is a number (meaningful only to the system) that identifies the module."）。ReactOS は 0 から順（U2）、Wine は空きの最小（U3）。Geoff Chappell は動的 TLS と別の仕組みと書く（U1）。**実機の値は未確認** | B-2 |
| T3 | **TLS 配列の置き場所と、動的 TLS（`fle_tls_values`）との衝突の回避**（同じ配列の添字空間を分ける、別の配列にする、`gs:[0x58]` の答えを時期で切り替える） | Learn は「TLS 配列（`gs:[0x58]`）」と「`TlsAlloc` のスロット」が別のものか同じものかを**書かない**（M2 は動的の配列、M1 は静的の配列）。U1 は別と書く。決定 0032 は FLEngine の 1 件のために両者を同じ表にした（F8）。現行の index は FLEngine 0、dsp_ippv2 1、Blend2D 2 | B-1 |
| T4 | **決定 0032 の `gs:[0x58]` = `fle_tls_values` の先頭、を変えてよいか**（変えるなら FLEngine の 1 件が引く表と、`vex.rs:528`、`gs_stacklimit.rs:183` のテストも変わる） | 0032 §6 は「作業者の判断」と書く。FLEngine の `gs:[0x58]` が現行の実行で通ったかは**未確認**（`vex-run.md:76`、§2.1） | B-1、D |
| T5 | **TLS ブロックのメモリの置き場所**（`calloc`、仮想アリーナ（決定 0017）、静的配列）と、**サイズの上限**（モジュール数 × ブロックのサイズ。`RM_MAX = 4`） | Blend2D は 32 バイト（F1）。アライメントは Characteristics（8 バイト）。**Learn はブロックの確保の方法を書かない**。wasm は 32 ビットのアドレスに収まる必要（F9） | B-3 |
| T6 | **`SizeOfZeroFill` とアライメントの扱い**（zero fill を足す、Characteristics のアライメントを守る、守れない場合は停止） | Learn は Size of Zero Fill と Characteristics を**書く**が、配布時の実装の義務（アライメントを守るか）は明示しない。ReactOS は zero fill を足さない（読み）、Wine は足す（U2、U3）。Blend2D は zero fill 0、アライメント 8 | B-4 |
| T7 | **TLS ディレクトリを読む場所**（`fle_realmod.c` の `read_headers`、`image_load`（harness）、リンク時の表 `fle_realmod_table`、生成器）と、その時点（`call_dllmain` の前か、`image_load` か） | Wine は `fixup_imports` の最初、つまり DllMain の前（U3）。Learn は書かない。`fle_realmod.h` の方針は「ロード時に run time で登録せず、リンクの表で渡す」 | B-5、C-1 |
| T8 | **TLS コールバックを呼ぶか、呼ばないか、空でなければ停止するか**（ATTACH の前か後か。DllMain との順序） | **Learn は TLS コールバックと DllMain の順序を書かない**。ReactOS と Wine は TLS コールバックを DllMain の前に呼ぶ（U2、U3）。Blend2D のコールバックは空（§4.1）。他のモジュール（QFC など）は未調査 | B-7 |
| T9 | **複数の実在モジュールが暗黙 TLS を持つときの index の順序とブロックの配置** | **Learn は書かない**。今は Blend2D の 1 つだけ（§4.7）。`RM_MAX = 4`（`fle_realmod.c:15`） | B-2 |
| T10 | **スレッド（案 A の JSPI 版）に進むときの、スレッドごとの TLS ブロックの作り方と、`DLL_THREAD_ATTACH`/`DETACH` の TLS コールバック** | Learn: "The system copies all of this data each time a thread is created"。決定 0080 D9 は `DLL_THREAD_*` の経路を作らない。案 C（スレッド生成で停止）は PM 決定（`sync-thread-survey.md`）。仮想スレッドごとの大域状態の入れ替え（R16）に TLS の配列とブロックを足す必要 | B |
| T11 | **`TlsAlloc` の上限 1,088 と暗黙の index の取り合い**（暗黙の index を `fle_tls_values` の末尾から取る、`FLE_TLS_MAX_INDEXES` を増やす、など。M2: 1,088 は Windows の上限） | `TlsAlloc` の上限は M2 に書かれている（1,088）。暗黙の TLS の index がこの上限に数えられるかは**書かれていない**（U1 は別の仕組みと書く） | B-1 |
| T12 | **`_tls_index` を読む前に書く保証**（ロード時に書く場合、FLEngine が実在モジュールの `_tls_index` を直接読む場合は無い。**DllMain の途中の `GetModuleHandle` で自分を引く経路で、ATTACH 中に別モジュールが `_tls_index` を読むか**） | `fle_realmod_module_handle` は ATTACH 中の自分を「未ロード」と答える（`fle_realmod.c:158-164`。決定 0082 で未決の点）。Blend2D の TLS の読みは自モジュールの `_tls_index` のみ（§4.2） | B-5 |
| T13 | **TLS ディレクトリが `.tls` セクションでなく `.rdata` にあること**（実測。§4.1）が、ほかのモジュールでも成り立つか | Learn は TLS ディレクトリがどのセクションにあるかを**書かない**（"do not assume that the RVAs in this table point to the beginning of a section"。M1 のデータディレクトリの節）。データディレクトリ 9 の RVA で読めば足りる（§4.1） | B-6 |
| T14 | **`SizeOfImage` と TLS テンプレートの位置**（テンプレートが `.rdata` にあるので、イメージの一部としてすでに置かれている。ブロックはコピーなので、ファイル段階の移動でテンプレートの VA が動くことが `Start`/`End` の追随（§4.1）で保たれる） | 実測（rebase の後でも差は基底の差） | B-6 |
| T15 | **FLEngine の生成 C の SHA-256（43985d62…）を条件として維持するか** | 決定 0078 以降、0088 まで不変を条件にしている。生成器を触る D は影響する（§6.4）。C-1 と B は runtime と fe-pe のみ | D、C-3 |
| T16 | **この調査の読みを実測で確かめる順序**（B5 の解決後、Blend2D の ATTACH を実行して `_tls_index`、`fle_tls_values[0]`、`[blk+8]` を読み取りのデバッガで見る） | 別作業（3 モジュール）の測定が先。読み取りの gdb で測れる（`ro_imports3.py` の形。値を書く機構を持たない） | A、B、C |

### 7.1 仕様が書かないこと（再掲。§3.2 の表のうち、決めるときに根拠が Learn に無いもの）

1. x64 の `ThreadLocalStoragePointer` のオフセット（Learn は x86 の 0x2C だけ。**0x58 は非公式**（U1））。決定 0032 の 0x58 は非公式の出典に基づく。
2. `_tls_index` の払い出しの規則、TLS 配列の長さ、ブロックを配る時点。
3. TLS コールバックと DllMain の呼び出し順序。
4. 動的ロード（LoadLibrary）した DLL の暗黙 TLS を、**既存のスレッドにどう配るか**（Vista 以降はサポートされると Learn は書く。実現の仕方は書かない）。
5. 暗黙の TLS の index と `TlsAlloc` の index が**同じ番号空間かどうか**（M2 と M1 は別々に述べる）。
6. アライメント（Characteristics）を配布時に守る義務。

## 8. 触ったファイル、規則の遵守、git の状態

### 8.1 触ったファイル（新規のみ）

- `docs/investigations/implicit-tls-design.md`（新規。この文書）
- `lift/out-tls/`（新規）: `tls_dir.py`、`tls_xrefs.py`、`tls_gs_sites.py`、`tls_offsets.py`（初版。バグあり。削除禁止のため残す）、`tls_offsets2.py`、`tls_offsets3.py`（`tls_offsets2.py` の `blk_at + 16` を `+ 60` に変えた派生。`sed` で作った）、`tls_index_src.py`、`tls_reach.py`、`tls_rebased.py`、`tls_gs30.py` と、出力 `tls-dir.txt`、`tls-index-xrefs.txt`、`tls-gs-sites.txt`、`tls-offsets.txt`（初版）、`tls-offsets2.txt`、`tls-offsets3.txt`、`tls-index-src.txt`（初版。3 件を取りこぼす）、`tls-index-src2.txt`（訂正後）、`tls-reach.txt`、`tls-rebased.txt`、`tls-gs30.txt`、`tmp/`（空）。
- **訂正の記録**: `tls_offsets.py`、`tls-offsets.txt`（定数の探索が自分自身から始まり、`add blk, reg` を後続のオフセットに足さない）と `tls_index_src.py` の初版の出力 `tls-index-src.txt`（gs の読みの前から探索を始めたため 3 件を取りこぼす）は**誤り**。正は `tls_offsets2.py`/`tls-offsets3.txt` と `tls-index-src2.txt`。`tls_offsets2.py` は「ブロックのレジスタを使う命令」を探す窓を、ブロックの読みから 16 命令先までにしていた。`diff tls-offsets2.txt tls-offsets3.txt`（窓を 60 命令にした `tls_offsets3.py` の出力との差）で、**窓の外にあった `cmp ecx, [rsi]` の +0x08 の読みが `f_180097190` に 3 件**（`0x18009728a`、`0x1800972f2`、`0x18009735a`。同じ関数の 2 つ目以降の静的ローカルの確認）増えた。**`tls-offsets2.txt` の要約の「+0x08 の 4 バイト読み 23」は窓の狭さによる数え漏れで、正は `tls-offsets3.txt` の 26**（`tls-offsets3.txt` のほかの差は、窓が広がって「ブロックのレジスタが上書きされた」位置まで進んだ行が増えただけで、数には効かない）。`tls-offsets3.txt` のアクセスは 30 件（すべて別の命令アドレス）: **+0x08 の 4 バイト読み 26、+0x08 の 4 バイト書き 2、+0x18 の 8 バイト読み 1、+0x18 の 8 バイト書き 1**。`tls-gs-sites.txt`、`tls-index-xrefs.txt`、`tls-index-src2.txt` はこの窓の影響を受けない。§4.4 の本文の「27 件のアクセス」は誤りで、**正しくは 26 か所のサイト、30 件のアクセス**（§4.4 の本文はこの訂正済みの形に直した）。
- 読み取りのみ: `lift/out-blend2d/Blend2D.dll`（SHA-256 66634d24…）、`lift/out-blend2d/Blend2D-rebased-4700000.dll`、`lift/out-blend2d/*.txt`、`lift/out-three/run/ro1-*`、`ro2-*`（別作業のログ）、`lift/objdump-text.txt`、ソース（`lift/runtime`、`lift/crates`、`lift/harness`）、`docs/`。

### 8.2 規則の遵守

- `rm` は使っていない（`__pycache__` も作っていない。`PYTHONDONTWRITEBYTECODE=1`）。既存ファイルの全体上書き（`>`）は使っていない（新規ファイルの作成と `>>` の追記のみ。出力ファイルはすべて新規）。git の変更系操作は使っていない（`git rev-parse`、`log`、`diff --stat`、`status` のみ）。**commit していない**。
- `/tmp` は使っていない。`TMPDIR` は `lift/out-tls/tmp`。sleep、timeout、nohup、末尾の `&` は書いていない。ビルド、実行、emcc は**していない**。`lift/out-cs/` を cwd にしていない。2 GB 級のファイルは作っていない。python の標準ライブラリと衝突する名前のファイルは作っていない（`tls_*.py`）。
- 編集したのは `docs/investigations/implicit-tls-design.md`（新規）と `lift/out-tls/` のみ。`fe-lift`、`fe-decode`、`fe-pe` のソース、`lift/runtime`、`lift/crates` のテスト、`lift/harness`、`docs/decisions`、`docs/README.md`、`docs/progress.md`、既存の docs は変更していない。
- 権限拒否は無かった。Wine の公式の GitLab（gitlab.winehq.org）は bot 対策（Anubis）で取得できず、**github の mirror（wine-mirror/wine）の master** で代えた（§3.4 U3）。Vergilius Project は取得していない（U4、未取得）。

### 8.3 取得できなかった／確認できなかったこと

- Windows の実機での `_tls_index` の値、TLS 配列の形、`gs:[0x58]` の挙動（**実機が無い**）。
- MSVC の CRT のソース（`_Init_thread_epoch`、`__Init_thread_header` の名前と形の対応は**推測**。§4.4）。
- Wine の `ldr.c` 以外のファイル（`ldrapi.c` 相当）と、ReactOS の動的ロード時の TLS（§3.4 U2 の「見つからなかった」）。
- Vergilius Project の `_TEB`（U4）。
- 現行 HEAD の 2 モジュール構成で、FLEngine の `gs:[0x58]`（`0x41d14b`）が実際に実行されるか（§2.1）。
- Blend2D の ATTACH が B5 の後で `f_18000abb0` に届いたときの実際の挙動（**実行していない**。§5 は読み）。
