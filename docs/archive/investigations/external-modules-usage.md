# 外部モジュール（dsp_ippv2_x64.dll、avrt、PowrProf、SHCore、dwmapi ほか）の使われ方

実行日: 2026-10-23。HEAD 9634829。調査のみ（runtime、生成器、テストは変更していない）。方針は決めない。PM が選ぶための事実だけを置く。

凡例:
- **実測** = コマンドの出力（7z ではなく `tools/extract_nsis2.py` の一覧・ハッシュ・書き出し、`fe-pe`、objdump、既存の native ハーネスを gdb で走らせた出力）。
- **読み** = FLEngine_x64.dll と dsp_ippv2_x64.dll のバイト列の静的な読み（capstone）。
- **測定用の仮の値** = gdb で注入した値。**実機の値ではない。ランタイムには入っていない。**
- **未確認** = 確認できていない。
- **仮説** = 読みや実測から言えそうだが、確認していないもの。

成果物: `lift/out-ext/`（下の §8）。`binaries/` は触っていない。

## 0. 先に読んだこと

- `docs/investigations/after-comparestringw.md`（停止点の記録）、`docs/decisions/0015`（仮想モジュール表）、`0055`（msctf.dll を仮想モジュールにした型）、`0010`（int-ext。この調査には関係しない。依頼にあったので目を通しただけ）、`0029`（`fle_load_absent[]` の 9 名と、外部 DLL は M2 とした理由）、`0014`（エミュレート OS は Windows 10 22H2）。
- `lift/runtime/fle_load.c`、`fle_modules.c`、`lift/crates/fe-lift/tests/load.rs`（`external_and_unnamed_modules_stop_by_name`）。

## 1. dsp_ippv2_x64.dll

### 1.1 インストーラ内に実在するか（実測）

`python3 tools/extract_nsis2.py <flstudio_win64_26.1.7.5653.exe> --list dsp_ippv2` の結果（EXTRACTFILE 27,574 件中）:

| エントリ | データオフセット | 名前 |
|---|---|---|
| 56482 | 0x2f7a373c | `dsp_ippv2.dll` |
| 56484 | 0x2fbd71d6 | `dsp_ippv2_x64.dll` |

どちらも 1 件ずつ。7z は使っていない（NSIS のヘッダ解析は既存の `extract_nsis2.py`）。

配置先: エントリ 56448 の SetOutPath 相当の文字列が `<NSIS 変数>\Shared`（先頭の 0x03 は NSIS の変数エスケープ。どの変数かは**未確認**）。56482〜56484 の前後は `beat_tracker.dll`、`elastique_x64.dll`、`freetype.dll` で、いずれも同じ `Shared` に置かれる並び（実測）。ゲストが `SetDllDirectoryW("Shared\\")` を呼ぶこと（`after-comparestringw.md` の実測）と整合する。

書き出し: `lift/out-ext/dsp_ippv2_x64.dll`（`--write`、新規ファイル）。

| 項目 | 値 | 区分 |
|---|---|---|
| サイズ | 39,942,656 B | 実測（`--hash` と `ls -l` が一致） |
| SHA-256 | `497dd4f77e859df9e86d999b3811be3c72526b41f86c38b0c3c6e395f691c352` | 実測（`--hash` と `sha256sum` が一致） |
| 種別 | PE32+（0x20b）、Machine 0x8664、Characteristics 0x2022（DLL、LARGE_ADDRESS_AWARE）、Subsystem 2 | 実測（fe-pe pe-info、objdump -p） |
| ImageBase / SizeOfImage / EntryPoint | 0x180000000 / 0x261f000 / RVA 0x17caca8 | 実測 |
| TimeDateStamp | 0x6abbb88e（FLEngine_x64.dll は 0x6abbb7e3。ほぼ同時刻のビルド） | 実測 |
| DllCharacteristics | 0x0160（HIGH_ENTROPY_VA、DYNAMIC_BASE、NX_COMPAT） | 実測 |
| リンカ | 14.29（MSVC 系） | 実測（MajorLinkerVersion/Minor） |
| セクション（9） | .text 0x17de140、IPPCODE 0x6dc771、.rdata、.data、.pdata（0x9408c）、IPPDATA、_RDATA、.rsrc（0x1e0）、.reloc | 実測 |
| TLS ディレクトリ、遅延 import、バウンド import | なし（0） | 実測 |
| 例外ディレクトリ | 0x9408c B。fe-pe pe-pdata-info: entries 50,529、function_starts 30,816 | 実測 |
| エクスポート | 446 件、すべて名前付き、フォワーダ 0、序数ベース 1、DLL 名フィールド `dsp_ippv2_x64.dll` | 実測（`pe_info_ext.py`） |
| エクスポートの内訳 | `ipps*` 366、`ippi*` 71、その他の `ipp*` 8（`ippMalloc`、`ippFree`、`ippGetLibVersion`、`ippGetNumThreads`、`ippSetNumThreads`、`ippSetCpuFeatures`、`ippSetDenormAreZeros`、`ippSetFlushToZero`）、`AggCreateInstance` 1 | 実測（`dsp-export-names.txt`） |
| インポート | **KERNEL32.dll の 1 DLL、72 名、序数 0** | 実測（fe-pe、objdump、`pe_info_ext.py` の 3 つが一致） |

KERNEL32 の 72 名のうち、`imports_table.rs`（応答表）に行がある名前は 37、無い名前は 35（実測。`HeapReAlloc`、`FlsAlloc` 系、`RtlUnwindEx`、`RtlVirtualUnwind`、`InitializeSListHead`、`GetModuleHandleExW`、`FindFirstFileExW` など MSVC の CRT 起動に使う名前）。これは「実 DLL を lift する」場合の入口の見積もりに使える事実で、**lift の可否や費用は未確認**。

32 ビット版 `dsp_ippv2.dll`: 28,174,848 B、SHA-256 `e86a96edba35f8a68018ab3efe4004ca8facbecdc158eb7de07562d0476ba8f4`、Machine 0x14c（PE32）（実測。`lift/out-ext/lift-dsp_ippv2-32.dll`）。ゲストが実測で探した名前は `_x64` 付きなので、これは使われない。

### 1.2 ゲストの使い方（読み。一部実測）

#### 呼び出しの流れ

```
f_65c540（unit 144、init）: [0x15f40e0] を 1 減らし、-1 になったら f_653f40 を呼ぶ。戻り値は使わない（call のあと ret）
f_653f40  : [0x15f40e8]（モジュールハンドル）が 0 でなければ true で戻る。
            0 なら f_6539e0("dsp_ippv2.dll", dl=1, r8=0) を呼び、戻りを [0x15f40e8] に保存
              [0x15f40e8] == 0 → ebx = 0（false）、0x6572c7 へ。GetProcAddress は 1 回も呼ばない
              != 0            → 423 回 f_653dd0 を呼ぶ（下記）。最後に 0x63a660（fnclex + fldcw + emms）と 0x63a6d0（ldmxcsr）
f_6539e0  : f_653660 で名前を組み立て（dl=1 なら末尾が "_x64.dll" でなければ "_x64.dll" を付ける）、
            SetDllDirectoryW(<ディレクトリ部。空なら f_6535a0 が既定値を入れる。実測では "Shared\\">)、
            f_44db40(name, 0x8000)（SetErrorMode(0x8000) → LoadLibraryW → SetErrorMode(元)）、戻りをそのまま返す
```

- `f_653660` は、`f_43ba70`（GetFileAttributesW を使う存在確認。`call 0x4234f0` = kernel32!GetFileAttributesW、実測した IAT 名）で 4 つのディレクトリ候補（`f_652750`、`f_652670`、`f_6535a0`、`f_6525b0`。各関数の中身は精査していない）を順に試し、見つかればそのパス、見つからなければ名前だけを返す（読み）。実測の LoadLibraryW の引数が裸の `dsp_ippv2_x64.dll` だったことと整合する（`after-comparestringw.md`）。
- `f_653f40` の呼び出し元は 3 つ（`f_65c540` の 0x65c554、`f_680610` の 0x680648、`f_11b2780` 付近の 0x11b285c）。**3 つとも戻り値を使わない**（0x65c554 は ret、0x680648 は直後に `cmp byte [rbp+0x58],0`、0x11b285c は直後に `nop`。読み）。
- `[0x15f40e8]` を読む関数は `f_653dd0`、`f_653f40`、`f_65c4f0`（FreeLibrary 側）の 3 つだけ（xref の実測。rip 相対のみ。絶対アドレスで書かれたものは .text に 0 件）。つまり「DLL があるか」を表す別のフラグは、この調査の範囲では見つからなかった。

#### 戻り値が NULL のとき（読み）

- `f_653f40` は false を返して終わる。エラー表示、例外、Halt はない（`f_653f40` の 0x653f71〜0x653f7d）。
- `f_65c4f0`（終了側）は `cmp [0x15f40e8],0; je skip` で、NULL なら FreeLibrary を呼ばない。NULL を許す形。
- `[0x15f40e8]` は 0 のままなので、`f_653f40` が再度呼ばれるたびに LoadLibraryW を**もう一度**呼ぶ（読み）。何回呼ばれるかは**未確認**（`f_680610` と `f_11b2780` 付近が実行されるかどうかを見ていない）。
- 測定用の仮の値（NULL、126）では unit 144 は戻り、unit 145 へ進んだ（`after-comparestringw.md` の実測）。

#### GetProcAddress に渡す名前（読み。実測の一部）

`f_653f40` が、成功時に `f_653dd0(rcx=rbp, rdx=<保存先のアドレス>, r8=<名前（UTF-16 リテラル）>, r9=0)` を **423 回**呼ぶ。`r9=0` は 423 回すべて（0x653f40〜0x6572d0 の逐次走査）。

- 名前の一覧は `lift/out-ext/ipp_names.txt`（呼び出しアドレス、保存先、名前）と `ipp_names.json`。distinct な名前は **420**（`ippsCos_64f_A53`、`ippsFFTInit_R_32f`、`ippsFFTInit_R_64f` が 2 回ずつ）。種別: `ipps*` 345、`ippi*` 71、`ippMalloc`、`ippFree`、`ippSetFlushToZero`、`ippSetDenormAreZeros`、`ippSetNumThreads`、`ippGetNumThreads`、`ippGetLibVersion` など。最初の名前は `ippMalloc`（0x653f98、保存先 0x15f33c0）、最後は `ippiMul_8u_C4IRSfs`（0x6572b2、保存先 0x15f40d8）。
- 420 名は**すべて** dsp_ippv2_x64.dll のエクスポートにある（集合差が空。実測）。DLL のエクスポート 446 のうち、ゲストが引かないのは 26（`AggCreateInstance`、`ippSetCpuFeatures`、`ippsFIRGenLowpass_64f` など）。
- `f_653dd0` の中身（読み）: 名前を `f_414850` で PWideChar にし、`f_4235f0`（GetProcAddress のラッパー。`shr rax,16` で序数かを見て `jmp` thunk 0x4235e0 を呼ぶ）で引く。0 なら `"legacy90"` を前に付けた名前（`f_414c70` で連結）で再度引く。それも 0 なら、r9 が 0 のため DLL を解放せず、`"IPP function not found: " + 名前` を OutputDebugStringW（thunk 0x423a90）に出して 0 を返す。保存先には 0 が入る。
- 保存先のグローバル: **0x15f33c0〜0x15f40d8（.bss、420 個の distinct アドレス）**。モジュールハンドルは 0x15f40e8、参照カウントは 0x15f40e0。
- 保存先を読む関数: **424 関数、1,007 アクセス**（`cell_readers.json`）。内訳（読み）: 保存先のアドレスを .data の 143 個のセル（8 バイト値が保存先アドレス。`absptr`/`cell_readers_ext.py`）に持ち、各所が `mov rax,[セル]; call [rax]` で呼ぶ形（977 アクセス）。残り 24 は同じ形で引数の並べ替えが間に入る。ほかに、`ippsMove_8u`（0x15f3470）、`ippsMove_32f`（0x15f3480）、`ippsZero_8u`（0x15f34a0）、`ippsZero_32f`（0x15f34b0）、`ippsAdd_32f_I`（0x15f3608）、`ippsMulC_32f_I`（0x15f3678）は、6 つの小さい関数（0x653c60、0x653c80、0x653cb0、0x653ce0、0x653d10、0x653d70。呼び出し点 23）が rip 相対の `call [slot]` で直接呼ぶ。
  - 420 個のうち、静的に読み手が見つかった保存先は 143 個（セル経由 143、直接 6。直接の 6 はセルにも含まれる）。**残り 277 個の読み手は見つからなかった**（rip 相対と絶対アドレスの走査。基底からのオフセット計算で読む形は走査していない。**未確認**）。
- **NULL 検査**: セル経由の呼び出し 977 アクセスは、読み込みの直後 3 命令以内に `cmp [reg],0` がない（`nullcheck_ext.py`、読み）。6 つの直接呼びにも検査がない（`653c6d` などの `cmp r8,0x7fffffff` は長さの検査）。つまり、DLL が無い（保存先が全部 0 の）ときに IPP 関数を呼ぶ経路へ実行が届けば、0 番地を呼ぶ形になる（読み）。**上位の呼び出し元が別のフラグで守っているかは未確認**（上で書いたとおり、ハンドルを見るフラグは見つかっていない）。

#### DLL が無い場合のゲストの振る舞い（まとめ）

| 場面 | 読み・実測 |
|---|---|
| 起動時（unit 144） | `f_653f40` が false を返すだけ。戻り値は 3 つの呼び出し元とも使わない。表示・例外・Halt はない（読み）。測定用の NULL で unit 145 へ進んだ（実測） |
| IPP 関数の使用時 | 検査の無い間接呼びが 1,000 ほどある（読み）。到達するかは未確認 |
| 終了時 | NULL を許す（読み） |

### 1.3 ほかの呼び出し元（`f_6539e0` を使う別の DLL。読み）

`f_6539e0` の呼び出し点は 13（0x653f65、0x6641b9、0x665bed、0x66d255、0x66dc2a、0x66ea2a、0x675334、0x815e33、0x95eb53、0xa58bbd、0xa5d914、0xba4c31、0xc395c5）。引数が文字列リテラルのもの 11 は §3。`dl=1`（`_x64.dll` を付ける）で呼ぶのは dsp_ippv2、QuickFontCache、ILSvg、ildsp、oggio、ILTools、WavpackDLL_x64、LAMEEnc（読み。rcx の `lea` の後ろの `mov dl,1`）。`dl` を設定しない（不定）のは Blend2D.dll、`ffmpeg\x64\ilvideo_x64.dll`、`Python312`（`r8` = `Python\`）。

## 2. avrt / PowrProf.dll / SHCore.dll / dwmapi（f_65c9f0、unit 145）

`f_65c9f0`（0x65c9f0〜0x65cfbf）の逐次走査（`f_65c9f0.txt`、`f_65c9f0_map_ext.py`）。入口で `sub [0x15f4268],1; cmp -1; jne 終了` の一度だけ初期化する形。

### 2.1 呼び出しと NULL 分岐（読み）

| 点 | 呼び出し | 保存先 | NULL のとき | GetProcAddress の名前の数 |
|---|---|---|---|---|
| 0x65ca23 | `LoadLibraryW(L"dwmapi")` | 0x15f4158 | `cmp [..],0; je 0x65cc93`（Dwm 系の 23 回を飛ばす） | 23 |
| 0x65cc9a | `GetModuleHandleW(L"user32")`（LoadLibrary ではない） | 0x15f4290 | `je 0x65ccca`（飛ばす） | 1（`UpdateLayeredWindowIndirect`） |
| 0x65ccd1 | `LoadLibraryW(L"avrt")` | 0x15f4288 | `je 0x65ce09`（飛ばす） | 11 |
| 0x65ce10 | `GetModuleHandleW(L"kernel32")` | 0x15f4298 | `je 0x65cee0`（飛ばす） | 7（SRWLock 系） |
| 0x65cee7 | `LoadLibraryW(L"advapi32")` | 0x15f42a0 | `je 0x65cf31`（飛ばす） | 2 |
| 0x65cf38 | `LoadLibraryW(L"PowrProf.dll")` | 0x15f42a8 | `je 0x65cf82`（飛ばす） | 2 |
| 0x65cf89 | `LoadLibraryW(L"SHCore.dll")` | 0x15f42b0 | `je 0x65cfb9`（終了） | 1 |

- LoadLibraryW の呼び出し点は 5、GetProcAddress は合計 47 回。**すべての LoadLibraryW/GetModuleHandleW の直後に、戻りを保存し、同じ場所に `cmp qword ptr [..],0` と `je` が来る**（NULL なら、そのモジュールの GetProcAddress を全部飛ばす）。停止、例外、Halt の分岐はない（読み）。
- 名前の一覧:
  - dwmapi: `DwmEnableBlurBehindWindow`、`DwmEnableComposition`、`DwmEnableMMCSS`、`DwmExtendFrameIntoClientArea`、`DwmGetColorizationColor`、`DwmGetCompositionTimingInfo`、`DwmGetWindowAttribute`、`DwmIsCompositionEnabled`、`DwmModifyPreviousDxFrameDuration`、`DwmQueryThumbnailSourceSize`、`DwmRegisterThumbnail`、`DwmSetDxFrameDuration`、`DwmSetPresentParameters`、`DwmSetWindowAttribute`、`DwmUnregisterThumbnail`、`DwmUpdateThumbnailProperties`、`DwmSetIconicThumbnail`、`DwmSetIconicLivePreviewBitmap`、`DwmInvalidateIconicBitmaps`、`DwmAttachMilContent`、`DwmDetachMilContent`、`DwmFlush`、`DwmDefWindowProc`
  - avrt: `AvSetMmMaxThreadCharacteristicsW`、`AvSetMmThreadCharacteristicsW`、`AvSetMmThreadPriority`、`AvQuerySystemResponsiveness`、`AvRevertMmThreadCharacteristics`、`AvRtCreateThreadOrderingGroup`、`AvRtCreateThreadOrderingGroupEx`、`AvRtDeleteThreadOrderingGroup`、`AvRtJoinThreadOrderingGroup`、`AvRtLeaveThreadOrderingGroup`、`AvRtWaitOnThreadOrderingGroup`
  - advapi32: `RegCopyTreeW`、`CheckTokenMembership`
  - PowrProf.dll: `PowerGetActiveScheme`、`PowerSetActiveScheme`
  - SHCore.dll: `GetDpiForMonitor`
  - user32: `UpdateLayeredWindowIndirect`、kernel32: `InitializeSRWLock`、`ReleaseSRWLockExclusive`、`ReleaseSRWLockShared`、`AcquireSRWLockExclusive`、`AcquireSRWLockShared`、`TryAcquireSRWLockExclusive`、`TryAcquireSRWLockShared`
- 終了側（`f_65c750`。`f_65c9f0` と対になる）も、5 つのハンドル（0x15f4158、0x15f4288、0x15f42a0、0x15f42a8、0x15f42b0）それぞれ `cmp [..],0; je` の後で FreeLibrary（thunk 0x4233d0）を呼ぶ。NULL を許す（読み）。

### 2.2 保存先を使う側（読み。NULL のときの機能）

- Dwm 系: `f_65c630`（`DwmIsCompositionEnabled` の保存先 0x15f4198 が 0 なら false）、`f_65c670`（保存先 ≠ 0 かを返す）、`f_65c590`（`DwmEnableBlurBehindWindow` の保存先 0x15f4160 が 0 なら false）、`f_65c680`（`DwmEnableMMCSS` が 0 なら `0x80004001` を返す）、`f_65c6a0`（`DwmGetCompositionTimingInfo` が 0 なら `0x80004001`）。いずれも NULL なら機能を無効にして戻る形。
- PowrProf: `f_65c6c0` が `PowerSetActiveScheme` の保存先 0x15f4258 を `cmp [..],0; je 終了` で検査し、0 なら何もしない（読み）。
- avrt、advapi32、SHCore、user32、SRWLock の保存先は .data のセル経由で読まれ、見た範囲（`bb0962`、`71e60d`、`812f36`、`12b0125` など）では `cmp qword ptr [rax],0` の後で呼ぶ形（読み）。**全件は読んでいない（未確認）**。
- **副次的な所見（読み、未確認を含む）**: kernel32 の SRWLock 7 名は、現在の応答表（`fle_resp_count = 227`、生成 C で確認）に**無い**ので、`GetProcAddress(kernel32, "InitializeSRWLock")` などは NULL と 127 を返す。測定用の run（§5）はこの経路を通ったが、その直後は RegCreateKeyExW の停止で、SRWLock の呼び出しには届いていない（実測）。`bb0586`/`bb058d` のように、SRWLock のスロット（`AcquireSRWLockExclusive`、0x15f4230）を `mov rbx,[セル]; call [rbx]` で検査なしに呼ぶ箇所を 1 つ見たが、それが NULL のときに選ばれる経路かは読んでいない。これは avrt の議論とは別の話で、次の停止点を探すときの注意としてだけ書く。

### 2.3 これらの DLL が Windows 10 にあるか（読みと一次資料）

Learn の各 API ページの Requirements（2026-10-23 取得）: `AvSetMmThreadCharacteristicsW` は Windows Vista 以降、DLL は Avrt.dll。`PowerGetActiveScheme` は Windows Vista 以降、PowrProf.dll。`GetDpiForMonitor` は Windows 8.1 以降、Shcore.dll。`DwmIsCompositionEnabled` は Windows Vista 以降、Dwmapi.dll。決定 0014 のエミュレート OS は Windows 10 22H2 なので、**ページから導ける範囲では、4 つとも Windows 10 にある**（0055 と同じ導出。ページが「Windows 10 にある」と書いているわけではない）。一方、決定 0015 / 0029 は dwmapi と shcore を「ロードされていない」（`fle_load_absent[]`）と決めている。同じ導出では dwmapi と shcore も Windows 10 にあることになる。この矛盾の有無は未決（事実としては、0055 の「矛盾する場合は欠いているとは定義しない」の基準を 0015 の 9 名にそのまま当てると矛盾する、までが言える）。

## 3. ゲスト全体の LoadLibrary / GetModuleHandle の名前（直接 call で届くもの）

方法: `.text` の逐次走査（`pe_dasm_ext2.py calls`、`calls-modules.txt`）。kernel32 の thunk と IAT 名を突き合わせた（thunk: LoadLibraryW 0x423990 と 0x4239a0、LoadLibraryA 0x41cf90、LoadLibraryExW 0x407090、GetModuleHandleW 0x4235b0 と 0x407060、GetProcAddress 0x4235e0 / 0x407070 / 0x41cfb0）。`jmp` や関数ポインタ経由で thunk を使う形は 0 件（`jmp` 形は `callers_ext.py`、データ中の thunk アドレスは 0 件。実測）。

呼び出し点の数（実測。thunk への直接 call）: LoadLibraryW 48（0x423990 が 46、0x4239a0 が 2）、LoadLibraryA 1、LoadLibraryExW 1、GetModuleHandleW 37（0x4235b0 が 31、0x407060 が 6）。ここで数えた「LoadLibraryW 48」には `f_44db40` 内の 1 点（0x44db6b）が含まれ、`f_44db40` と `f_6539e0` の呼び出し元は別に数えている。ラッパー: `f_44db40`（SetErrorMode → LoadLibraryW → SetErrorMode）の呼び出し点 27、`f_6539e0` の呼び出し点 13、GetProcAddress ラッパー `f_4235f0` の呼び出し点 433。

文字列リテラルで名前が分かる点は 102 点（`modules-table.tsv`。LoadLibraryW 直接 44、GetModuleHandleW 34、`f_44db40` 経由 13、`f_6539e0` 経由 11）。**名前が分からない点**（引数が動的、またはリテラルを結び付けていない）: LoadLibraryW 系 6（0x41a45b の LoadLibraryExW、0x41dbd1 の LoadLibraryA、0x44db6b の `f_44db40` 内部、0x8b419d と 0x8b41c1、0x9b594b）、GetModuleHandleW 3（0x8eb0b9 は NULL、0x8eb0fb、0x9da828）、`f_44db40` 経由 14（うち 1 は `f_6539e0` 内の 0x653a51）、`f_6539e0` 経由 2（0x95eb53、0xba4c31 は .data のグローバル `[0x14f5780]` で、イメージ上の値は 0）。0x41dbd1（LoadLibraryA）は run の最初に `kernel32.dll` を 2 回引いている（実測。`gdb-chain2.txt`）。

「ランタイム表」= `fle_modules[]`（virtual、26 名）または `fle_load_absent[]`（absent、9 名）。`tests/load.rs` の列は `external_and_unnamed_modules_stop_by_name` の名前の一覧にあるか。LoadLibrary 系と GetModuleHandleW は別の関数で、GetModuleHandleW は表に無い名前が 126 を返す（停止しない。`fle_modules.c`）。**停止するのは LoadLibrary 系だけ**なので、下の「応答」列は LoadLibrary 系の場合の結果。

| 正規化した名前（`fle_load.c` の規則） | 点 | 呼び方 | ランタイム表 | load.rs の一覧 | 応答（LoadLibrary 系） |
|---|---|---|---|---|---|
| kernel32.dll | 19 | GetModuleHandleW 17、LoadLibraryW 2 | virtual（N=9） | — | ハンドル |
| shfolder.dll | 25 | LoadLibraryW 25 | virtual | — | ハンドル |
| user32.dll | 7 | GetModuleHandleW 6、LoadLibraryW 1 | virtual | — | ハンドル |
| shell32.dll | 4 | LoadLibraryW 3、GetModuleHandleW 1 | virtual | — | ハンドル |
| comctl32.dll | 3 | GetModuleHandleW 3 | virtual | — | ハンドル |
| wininet.dll | 3 | LoadLibraryW 1、GetModuleHandleW 2 | virtual | — | ハンドル |
| ole32.dll | 2 | GetModuleHandleW 2 | virtual | — | ハンドル |
| oleaut32.dll | 2 | `f_44db40` 1、GetModuleHandleW 1 | virtual | — | ハンドル |
| ntdll.dll | 2 | GetModuleHandleW 2 | virtual | — | ハンドル |
| advapi32.dll | 1 | LoadLibraryW 1（0x65cee7） | virtual | — | ハンドル |
| msctf.dll | 1 | `f_44db40` 1（0x5de9fa） | virtual（決定 0055） | — | ハンドル |
| dwmapi.dll | 1 | LoadLibraryW（0x65ca23） | absent | — | NULL、126 |
| shcore.dll | 1 | LoadLibraryW（0x65cf89） | absent | — | NULL、126 |
| imm32.dll | 1 | `f_44db40`（0x620a47） | absent | — | NULL、126 |
| **dsp_ippv2.dll → dsp_ippv2_x64.dll** | 1 | `f_6539e0`（0x653f65、dl=1） | **無し** | `dsp_ippv2.dll`、`dsp_ippv2` はあるが `dsp_ippv2_x64.dll` は無い | **未応答（停止）** |
| **avrt** | 1 | LoadLibraryW（0x65ccd1） | **無し** | あり（"avrt"） | **未応答** |
| **powrprof.dll** | 1 | LoadLibraryW（0x65cf38） | **無し** | あり（"PowrProf.dll"） | **未応答** |
| **psapi.dll** | 1 | LoadLibraryW（0x4263e1。直後の 0x4263f6 は KERNEL32.dll） | 無し | あり | 未応答 |
| **riched20.dll** | 1 | LoadLibraryW（0x6d644d） | 無し | あり | 未応答 |
| **urlmon.dll** | 1 | LoadLibraryW（0x707bfa） | 無し | あり | 未応答 |
| **webview2loader.dll** | 1 | LoadLibraryW（0xbe3539） | 無し | あり | 未応答 |
| **beat_tracker.dll** | 1 | LoadLibraryW（0x11336a4） | 無し | あり | 未応答 |
| **declipper.dll** | 1 | LoadLibraryW（0xca1008。同じ関数内に文字列 0xca0fa9。引数との結び付けは読みのみ） | 無し | あり | 未応答 |
| **stemextract.dll** | 1 | LoadLibraryW（0xcad9c4） | 無し | あり | 未応答 |
| **ws2_32.dll** | 1 | `f_44db40`（0xc07a0f） | 無し | あり | 未応答 |
| **iphlpapi.dll** | 1 | `f_44db40`（0xc192fa） | 無し | あり | 未応答 |
| **blend2d.dll** | 1 | `f_6539e0`（0x6641b9、dl 不定） | 無し | 無し | 未応答（実測。§5） |
| **quickfontcache_x64.dll**（推定） | 1 | `f_6539e0`（0x665bed、dl=1） | 無し | 無し | 未応答 |
| **ilsvg_x64.dll**（推定） | 1 | `f_6539e0`（0x66dc2a、dl=1） | 無し | 無し | 未応答 |
| **ildsp_x64.dll**（推定） | 1 | `f_6539e0`（0x66ea2a、dl=1） | 無し | 無し | 未応答 |
| **oggio_x64.dll**（推定） | 1 | `f_6539e0`（0x675334、dl=1） | 無し | 無し | 未応答 |
| **iltools_x64.dll**（推定） | 1 | `f_6539e0`（0x815e33、dl=1） | 無し | 無し（`ILTools_x64.dll` が一覧にあるが、正規化後の名前は別） | 未応答 |
| **wavpackdll_x64.dll** | 1 | `f_6539e0`（0xa58bbd、dl=1） | 無し | 無し | 未応答 |
| **lameenc_x64.dll**（推定） | 1 | `f_6539e0`（0xa5d914、dl=1） | 無し | 無し | 未応答 |
| **python312.dll**（推定。`Python\` を前置） | 1 | `f_6539e0`（0xc395c5） | 無し | 無し | 未応答 |
| **ffmpeg\x64\ilvideo_x64.dll** | 1 | `f_6539e0`（0x66d255） | 無し | 無し | パス付きなので `fle_load.c` は別の理由（path）で停止 |
| **speaker_x64.dll** | 1 | `f_44db40`（0xa65b24） | 無し | 無し | 未応答 |
| **ds2wav.dll**、**ss2wav.dll** | 各 1 | `f_44db40`（0xb953d6、0xf360bc） | 無し | 無し | 未応答 |
| **mswsock.dll**、**wship6.dll**、**fwpuclnt.dll**、**idndl.dll**、**normaliz.dll** | 各 1 | `f_44db40`（0xc12afb、0xc14b47、0xc14c5e、0xc171fa、0xc17278） | 無し | 無し | 未応答 |

- 「推定」は、`f_6539e0`（dl=1）が `_x64.dll` を付けるという読みと、`dsp_ippv2.dll` → `dsp_ippv2_x64.dll` の**実測 1 例**からの推定（`f_43c4c0` の組み立て、`f_43c870` の処理は精査していない）。名前の変換結果が、インストーラ内の名前と一致する例が多い（§4）。
- **名前が分かる範囲の下限**: 未応答の名前は **30**（distinct。パス付き 1 を含む）。LoadLibrary 系（LoadLibraryW 直接、`f_44db40` 経由、`f_6539e0` 経由）の名前が分かる点 68 のうち、ランタイム表で答えられるのは 8 名・35 点（virtual）と 3 名・3 点（absent）で、未応答は 30 名・30 点（`modules-table.tsv` の GetModuleHandleW 以外の行を集計）。ゲスト全体で実行される数ではなく、**呼び出し点に名前が書いてある数**であり、実行されるかは別（未確認）。名前の分からない LoadLibrary 系 6 点と間接参照（`LoadLibraryW` をポインタで呼ぶ形は 0 件、実測）は、この数に入っていない。
- `load.rs` の一覧にある名前のうち、ゲストのリテラルとして**見つからなかった**もの: `dsp_ippv2`（拡張子なし。リテラルは `dsp_ippv2.dll`）、`ILTools_x64.dll`（リテラルは `ILTools`）。コメントにある出典 `out-a6/work/ext_runs.tsv` は今回は見ていない。

## 4. インストーラ内の実在（§3 の外部 DLL。実測）

`extract_nsis2.py` の一覧と同じヘッダを読み、EXTRACTFILE のファイル名の末尾で照合（`list_names_ext.py`、`list2_ext.py`）。

- あり: `dsp_ippv2_x64.dll`、`dsp_ippv2.dll`、`ildsp_x64.dll`、`ILSvg_x64.dll`、`ILTools_x64.dll`、`oggio_x64.dll`、`QuickFontCache.dll` と `QuickFontCache_x64.dll`、`Blend2D.dll`、`wavpackdll_x64.dll`、`LAMEenc_x64.dll`、`ILVideo_x64.dll`、`beat_tracker.dll`、`WebView2Loader.dll`（2 件）、`speaker_x64.dll`、`REX Shared Library_x64.dll`、`python312.dll`（ほか `python312._pth`、`python312.zip`）。
- **なし**: `declipper.dll`、`stemextract.dll`、`ds2wav.dll`、`ss2wav.dll`、`mswsock`、`wship6`、`fwpuclnt`、`idndl`、`normaliz`、`iphlpapi`、`ws2_32`（Windows 標準のもの）。`avrt`、`powrprof`、`shcore`、`dwmapi`、`psapi`、`riched20`、`urlmon`、`shfolder` もない（すべて Windows の DLL）。
- 「なし」は EXTRACTFILE のファイル名の照合の結果だけ。別の入手経路（別インストーラ、ZIP、ダウンロード）は**未確認**。`ffmpeg\x64\` の下のディレクトリ名は照合していない。

## 5. 測定用の仮の値で進めた連鎖（実測。**実機の値ではない**）

`lift/out-ext/gdb_chain2_ext.cmd` と `gdb_chain2_ext.py`。既存の native ハーネス（`lift/out-cs/native/flengine-native`、未再ビルド）を gdb で走らせ、停止する名前（ランタイム表にも `fle_load_absent[]` にも無い名前）の LoadLibraryW だけを NULL と 126 に差し替えた。ログ: `gdb-chain2.txt`。

| 順 | 名前 | ランタイム表 | 呼び出し元（上位 3 つ） |
|---|---|---|---|
| 1 | `kernel32.dll`（LoadLibraryA 2 回） | virtual | `f_41cf90` ← `f_41d9a0` |
| 2 | `Msctf.dll` | virtual | `f_44db40` ← `f_5de9e0` |
| 3 | `imm32.dll` | absent | `f_44db40` ← `f_620a20` |
| 4 | **`dsp_ippv2_x64.dll`** | **未応答（仮の NULL）** | `f_6539e0` ← `f_653f40` |
| 5 | `dwmapi` | absent | `f_65c9f0` |
| 6 | **`avrt`** | **未応答（仮の NULL）** | `f_65c9f0` |
| 7 | `advapi32` | virtual | `f_65c9f0` |
| 8 | **`PowrProf.dll`** | **未応答（仮の NULL）** | `f_65c9f0` |
| 9 | `SHCore.dll` | absent | `f_65c9f0` |
| 10 | **`Blend2D.dll`** | **未応答（仮の NULL）** | `f_6539e0` ← `f_664190` |
| — | （その後） | — | `fle_stop`: `RegCreateKeyExW`「creating a direct child of HKEY_LOCAL_MACHINE or HKEY_USERS is not allowed (no code on the page)」（`f_422ff0` ← `f_5af5e0` ← `f_664fd0` ← `f_665b90`） |

- 4 つの未応答（dsp_ippv2_x64、avrt、PowrProf、Blend2D）を仮の値で通すと、停止は LoadLibrary ではなく **RegCreateKeyExW（レジストリ）に移る**（実測）。SHCore と dwmapi は既存の `fle_load_absent[]` で通る。
- `f_664190`（Blend2D）の読み: LoadLibrary 系の戻りが 0 なら GetProcAddress（`AggCreateInstance`）を飛ばし、保存先 0x15f45f0 が 0 のまま `setne al` で false を返す。NULL を許す形（読み）。
- 経路の全体で、仮の NULL の後に**クラッシュ（SIGSEGV など）は出ていない**（実測。停止まで）。IPP 関数が呼ばれる経路には届いていない。
- 最初に書いた `gdb_chain_ext.py`（1 本の python）は、`gdb.Breakpoint.stop` の中で `return` を実行する形が動かず、15 分で打ち切った。出力（`gdb-chain.txt`）は dsp_ippv2_x64 の 1 件目まで。作り直したのが `gdb_chain2_ext.*`。

## 6. 一次資料（Microsoft Learn）で確認できたこと

2026-10-23 に取得。
- LoadLibraryW（https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryw）: 「If the function succeeds, the return value is a handle to the module. If the function fails, the return value is NULL. To get extended error information, call GetLastError.」「If the function cannot find the module, the function fails.」「If the specified module is a DLL that is not already loaded for the calling process, the system calls the DLL's DllMain function with the DLL_PROCESS_ATTACH value. If DllMain returns TRUE, LoadLibrary returns a handle to the module. If DllMain returns FALSE, the system unloads the DLL ... and LoadLibrary returns NULL.」ページは失敗時の**エラーコードを名指ししていない**。
- System Error Codes（https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-）: `ERROR_MOD_NOT_FOUND` = 126（0x7E）「The specified module could not be found.」、`ERROR_PROC_NOT_FOUND` = 127「The specified procedure could not be found.」。
- GetProcAddress（https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getprocaddress）: 失敗時は NULL、GetLastError で拡張エラー。
- LoadLibraryW のページの Security Remarks: 「Do not make assumptions about the operating system version based on a LoadLibrary call that searches for a DLL. If the application is running in an environment where the DLL is legitimately not present but a malicious version of the DLL is in the search path, the malicious version of the DLL may be loaded.」（DLL が無い状況がありうることをページが書いている）。
- Requirements（§2.3）: avrt（Vista 以降、Avrt.dll）、PowerGetActiveScheme（Vista 以降、PowrProf.dll）、GetDpiForMonitor（8.1 以降、Shcore.dll）、DwmIsCompositionEnabled（Vista 以降、Dwmapi.dll）。

確認できなかったこと: 「LoadLibraryW が存在しないモジュールに対して必ず 126 を付ける」ことを書いたページ（0055 と同じ。126 は決定側の選択）。Windows 10 22H2 に avrt.dll などが「ある」とそのまま書いたページ。

## 7. 実測 / 読み / 未確認 / 仮説

### 実測
- dsp_ippv2_x64.dll（39,942,656 B、SHA-256 `497dd4f7…c352`）はインストーラ内にある（エントリ 56484、`Shared` 配下）。PE32+ x64、446 エクスポート（名前付き、フォワーダなし）、インポートは KERNEL32.dll の 72 名のみ。
- 同じ場所に 32 ビット版 `dsp_ippv2.dll`（28,174,848 B）がある。
- 未応答名を仮の NULL で通すと、dsp_ippv2_x64 → dwmapi/avrt/advapi32/PowrProf/SHCore → Blend2D と進み、停止は RegCreateKeyExW に移る。クラッシュは出ない。
- 呼び出し点の数（§3）。thunk への直接 call は LoadLibrary 系 50（LoadLibraryW 48、LoadLibraryA 1、LoadLibraryExW 1）、GetModuleHandleW 37。
- 未応答のリテラル名は少なくとも 30（distinct）。

### 読み
- `f_653f40`: DLL が NULL なら false を返して終わり、3 つの呼び出し元は戻り値を使わない。NULL でも停止・例外・Halt はない。終了側も NULL を許す。
- `f_653f40` は NULL の間、呼ばれるたびに LoadLibraryW を再実行する。
- 成功時は 423 回の GetProcAddress（420 名、すべて DLL がエクスポートする名前）。足りない名前があっても `IPP function not found: …` を OutputDebugStringW に出して 0 を入れるだけ（DLL は解放しない）。
- 420 の保存先のうち 143 は静的に読み手があり（424 関数、1,007 アクセス）、その呼び出しに NULL 検査がない。
- `f_65c9f0`: dwmapi / avrt / advapi32 / PowrProf / SHCore のすべてで、NULL なら以降の GetProcAddress を飛ばす。NULL なら機能を無効にして進む形で、停止や例外の分岐はない。Dwm、PowrProf の保存先を使う関数は NULL なら 0 または `0x80004001` を返す。
- Learn の Requirements からは、avrt、PowrProf、Shcore、Dwmapi は Windows 10 にある（導出）。

### 未確認
- dsp_ippv2_x64.dll が実機（実 Windows の FL Studio）でどう読み込まれるか、`f_653f40` が実行中に何度呼ばれるか。
- IPP 関数を呼ぶ経路が DLL なしで実行されるか（NULL 検査のない 977 アクセスを守る別のフラグがあるか。残り 277 個の保存先の読み手）。
- `f_43c4c0` / `f_43c870` の中身（名前の組み立て）。`f_652750` など 4 つのディレクトリ関数の中身。
- avrt、advapi32、SHCore、SRWLock の保存先を使う全箇所の NULL 検査。kernel32 の SRWLock 7 名が NULL の場合の経路（応答表に無い名前）。
- インストーラ以外の入手経路で `declipper.dll` などが来るか。
- 名前が分からない LoadLibrary 系 6 点と GetModuleHandleW 3 点が何を引くか。
- 実 DLL を lift する場合の費用（PDATA 50,529 件。KERNEL32 72 名のうち応答表に無いのは 35）。

### 仮説（確認していない）
- DLL が無くても IPP を使わない設定（または IPP を呼ばない機能の経路）に落ちる設計で、検査のないセル経由の呼び出しは IPP 版の機能の中にだけある。
- `ILSvg`、`ildsp` などの `_x64.dll` 付きの名前は、`f_6539e0` が dl=1 で付けた結果として、インストーラの `ILSvg_x64.dll` 等と一致する（§4 の一致は 6 例）。
- 未応答 30 名のうち、起動の初期 unit（init_count の範囲）で実行されるのは一部で、`Blend2D.dll` までの 4 つの外部名が当面の連鎖（§5）。

## 8. 作ったファイル（`lift/out-ext/`）

- 抽出物: `dsp_ippv2_x64.dll`（39,942,656 B）、`lift-dsp_ippv2-32.dll`（28,174,848 B）。
- 解析スクリプト（読み取り専用）: `pe_info_ext.py`、`pe_dasm_ext.py`（最初の版。skipdata なし）、`pe_dasm_ext2.py`（本命）、`callers_ext.py`、`argscan_ext.py`、`ipp_names_ext.py`、`slot_readers_ext.py`、`slot_rawscan_ext.py`、`slotuse_ext.py`、`cell_readers_ext.py`、`cellrd_ext.py`、`nullcheck_ext.py`、`absptr_ext.py`、`nearstr_ext.py`、`f65c9f0_map_ext.py`、`list_names_ext.py`、`list2_ext.py`、`outpath_ext.py`、`module_table_ext.py`、`gdb_chain_ext.py`（動かなかった版）、`gdb_chain2_ext.py`/`.cmd`（本命）。
- 出力: `dsp-objdump-p.txt`（22 MB）、`dsp-export-names.txt`、`calls-modules.txt`、`argscan-wrappers.txt`、`f65c9f0.txt`、`f65c9f0_slots.json`、`ipp_names.txt`/`.json`、`cell_readers.json`、`slot-readers.txt`、`slot_rawscan.json`、`slotuse.txt`、`slotuse2.txt`、`xref-15f40e8.txt`、`modules-table.tsv`、`gdb-chain.txt`/`.err`、`gdb-chain2.txt`/`.err`、`__pycache__/`、`tmp/`（空）。
- 注: `lift/out-cs/dis.py` が Python 標準ライブラリの `dis` と衝突するため、`lift/out-cs/` を作業ディレクトリにすると capstone が import できない（`inspect` → `dis`）。既存ファイルなので触っていない。解析は `lift/out-ext/` から実行した。
