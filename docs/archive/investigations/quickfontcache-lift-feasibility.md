# quickfontcache_x64.dll を 3 つ目のモジュールとして lift できるか（実現性調査）

実行日: 2026-10-24。HEAD: 作業開始時 `f8b5e6df`、文書作成時 `27c8068`（この間に `lift/crates/*/src` と `imports_table.rs` は変わっていない。`git diff f8b5e6d HEAD -- 'lift/crates/*/src'` と作業ツリーの `git diff` がどちらも空。測定に使ったバイナリは 03:19 のビルド）。**調査のみ。この文書は何も決めない**（決定が要る点は §8、意見は §9 に別枠）。fe-lift・fe-decode・fe-pe のソース、`lift/runtime`、`docs/decisions`、既存のテスト、既存の docs は編集していない。新規ファイルはこの文書と `lift/out-qfc/`（`.gitignore` の `lift/out*/` で追跡されない）の下だけ。

背景は `registry-unit153.md` §6.3（FLEngine の unit 158 が `quickfontcache_x64.dll` の `LoadLibraryW` で止まる）、`loadlibrary-names-classification.md` §2.2 #5・§6.2、決定 0073（dsp_ippv2 を「無い」と定義せず 2 つ目のモジュールとして lift）、0080（基底 0x2000000、ファイル段階のリベース）、0081、0082。先例は `dsp-ippv2-lift-feasibility.md`、`blend2d-lift-feasibility.md`（HEAD に入っているもの。読み取りだけ）、`dll-nonpdata-reloc-seeds.md`。

## 0. 凡例と範囲

- **実測** = このセッションで実行したコマンドの出力（`lift/out-qfc/` に保存）。**読み** = capstone による逆アセンブルとソースの読み。実行していない。**推測 / 仮説** = 読みからさらに推し量ったもの。**未確認** = 確認していない。「他者の実測」は、他の文書の数値を引いたもので、今回は再測定していない。
- 「静的な到達」は、直接 `call`/`jmp`/`jcc` と `lea reg,[rip+関数]` だけを辺にした到達（vtable、レジスタ経由、データ上のポインタ、コールバックは辺に無い）。**「届かない」は「実行されない」を意味しない**。
- 「QFC の unit N」は QFC 自身の初期化表（後述 §6.2、165 エントリ）の index。FLEngine の unit（`loadlibrary-names-classification.md` の unit、`units.tsv`）とは別物で、混ぜない。FLEngine 側は「FLEngine の unit N」と書く。
- emcc と wasm は使っていない（依頼の規則）。**wasm の locals は測っていない**（§4.5）。実行（native も）していない。統計だけ。
- 行の有無の「行」は `imports_table.rs` の `TABLE` の行（260 行。依頼文の「応答表」）。行があることは、runtime が常に答えることを意味しない（§3.4）。
- 他の作業者の成果（`lift/out-blend2d/`、`lift/out-loadlib/`、`lift/out-ext/`、`lift/out-design/`）は読み取りだけで使った（Blend2D の生成 C とスクリプト、`llpe.py`、`units.tsv`、`edges-*.tsv`、`ipp_names.txt`）。`PYTHONDONTWRITEBYTECODE=1` で実行し、`__pycache__` は作っていない（`find lift/out-qfc lift/out-loadlib -name __pycache__` が空。なお `tools/__pycache__` は 10-23 に作られた既存のもので、今回のものではない。`.gitignore` 済み）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | インストーラ内の実体 | `QuickFontCache_x64.dll` が **1 件**（エントリ 56472、2,067,968 B、SHA-256 `afc5a0ed70eb7e4af3d0f4baf771bd6f53eb014e237f857f23023471047736d2`）。32 ビット版 `QuickFontCache.dll`（56470、1,260,544 B）も別にある（抽出していない）。配置先は他の同梱 DLL と同じ `Shared` | 実測 |
| 2 | PE | PE32+ x64 DLL、**ImageBase 0x400000**（FLEngine と同じ）、SizeOfImage 0x20e000（約 2.05 MiB）、エントリ RVA 0x19c1f0、export 10、import 8 DLL・217 名（序数 0）、遅延 import 5 記述子・6 名、`.pdata` 6,347（連鎖 0）、DIR64 44,887、TLS・Load Config・Debug なし | 実測 |
| 3 | 基底の重なり | ImageBase 0x400000〜0x60e000 は **FLEngine（0x400000〜0x1cc0000）と全面的に重なる**。dsp_ippv2（リベース後 0x2000000〜0x461f000）とは重ならない。FLEngine の終端と dsp_ippv2 の基底の間の隙間は 0x340000 B で、QFC（0x20e000 B）が入る | 実測（算術） |
| 3b | 関数名の衝突 | リベースなしの生成 C は `f_<VA>` が FLEngine と **374 個同じ**（リンクすると重複定義）。0x1d00000 にリベースすると 0 | 実測 |
| 4 | lift の規模 | `fe-lift lift FILE /dev/null --pdata`: functions **6,712**、closure_instructions **209,259**、bad_nodes **0**、unsupported_total **104**、c_bytes **17,656,503**、0.18 s、31 MB。dsp_ippv2（32,648 関数、6,464,799 命令）の関数数の約 1/5、命令数の約 1/31 | 実測 |
| 5 | unsupported の内訳 | `jmp [IAT]` の行なし **88**、SSSE3 の `Pshufb` 6・`Phaddw` 4・`Pmulhrsw` 4（2 関数）、`jmp` の間接先未解決 **2**。VEX/EVEX/AVX-512 の未対応は **0**（VEX の `vmovups` 系は決定 0032 で対応済み、1 関数に 18 個） | 実測 |
| 6 | reloc seed | 非 pdata の reloc 値 **9,293**（うち 9,283 はどの RUNTIME_FUNCTION の範囲にも入らない）。**全部を種にすると bad_nodes が 0 → 17,541**（関数 16,082）。葉関数らしい形に絞った 155 個でも 48、`ret` で終わる 31 個でも 10。`.pdata` 開始の reloc 値 3,097 個を足しても閉包は不変 | 実測 |
| 7 | CPU 振り分け | 決定 0079 の形のスタブは **0 個**（`jumptabs 0`）。`cpuid` 5 か所、`xgetbv` 1 か所（ECX の OSXSAVE と AVX の両ビットが立つときだけ）。SSSE3 版と非 SSSE3 版を実行時の変数で選ぶ 1 関数（`f_57e600`） | 実測と読み |
| 8 | locals 上限（wasm 50,000） | **未測定**。生成 C の最大関数は `f_522e50`（13,559 行、1,094,671 B）。2,000 行以上は 1 関数だけ。この関数は **dsp_ippv2 の IPP 420 名を `GetProcAddress` で引くローダー**（`GetProcAddress` の呼び出し 846 か所。名前は 420 種で、各名前を `ipp…` と `legacy90ipp…` の 2 通りで引く形）で、**QFC の unit 88 から呼ばれる**（読み。unit 88 が DllMain の ATTACH で実行されるかは未確認、§6.2）。Blend2D の 4 関数（11,000 行前後で locals 約 10 万）と同じ大きさの級 | 実測（行数）、locals は未確認 |
| 9 | FLEngine が引く名前 | **8 名**（`FontCache_AcquireFont`、`FontCache_CleanUp`、`FontCache_GetStats`、`AcquireSharedThreadBank`、`AcquireSharedAnimTimer`、`LoadSharedElastiqueDll`、`CreateVST3Instance`、`InitSharedPython`）。**すべて QFC の export に実在**。export の残り 2 名（`dbkFCallWrapperAddr`、`__dbk_fcall_wrapper`）は FLEngine も同名を export し、FLEngine は引かない | 実測と読み |
| 10 | NULL のとき | ロード点（`f_665bd0`）は偽を返し、unit 158 の本体（`f_66d050`）は `MessageBoxW` の後で `GetProcAddress(NULL, …)` の経路に進む（`registry-unit153.md` §6.3 の実測）。**スロットを読む 4 関数は呼ぶ前にスロットを検査しない**（読み）。つまり NULL は「ロード点では許容」だが、使う側は許容しない | 読み |
| 11 | QFC 自身の依存 | **QFC の DllMain の初期化（unit 88、129）が `dsp_ippv2.dll` と `freetype.dll` を `LoadLibraryW` する**。Windows の DLL は `dwmapi`、`avrt`、`PowrProf.dll`、`SHCore.dll` など（unit 92）。Blend2D、ILTools などの同梱 DLL の名前は無い。さらに、このローダー（`f_5222b0`、`f_522390`）が `GetModuleFileNameW` を呼ぶ（unit 88、129 から静的に届く）。**hModule 引数の中身は未確認**（`f_5222b0` は NULL に見え、`f_522390` は `.bss` の `[0x5d2298]` を渡す。`[0x5d2298]` は、DllMain が `f_419dc0` へ `r8` として渡す `hinstDLL` を `f_419dc0` が保存する変数に読める。読み）。**実在モジュールのハンドルに対する `GetModuleFileNameW` は決定 0082 の D6 で名前付き停止** | 読み（実行していない） |
| 12 | 現行 runtime の応答 | normal import 217 名のうち行あり 130・**行なし 87**。遅延 import 6 名のうち行あり 2・行なし 4。行なし 87 のうち 83 名は FLEngine も行なしで import する | 実測 |

## 2. インストーラ内の実体と抽出（項目 2）

入力は `/home/ubuntu/ダウンロード/flstudio_win64_26.1.7.5653.exe`（1,106,033,456 B。読み取りのみ）。**この 1 ファイルだけ**を抽出した（2 MB 級。大きな一時ファイルは作っていない。NSIS のヘッダ（解凍後 3,519,116 B）をメモリ上で解いて、対象の 1 ストリームだけを `tools/extract_nsis2.py` が書き出す）。

```
python3 tools/extract_nsis2.py INSTALLER --list quickfont            # → nsis-list-quickfont.txt
python3 tools/extract_nsis2.py INSTALLER --list ""                   # 全 27,575 行（EXTRACTFILE 27,574 + 件数行）→ nsis-list-all.txt
python3 tools/extract_nsis2.py INSTALLER --hash QuickFontCache_x64.dll   # → hash-x64.txt
python3 tools/extract_nsis2.py INSTALLER --hash QuickFontCache.dll       # → hash-x86.txt
python3 tools/extract_nsis2.py INSTALLER --write QuickFontCache_x64.dll lift/out-qfc/quickfontcache_x64.dll   # 新規ファイルのみ
```

| エントリ | データオフセット | 名前（大文字小文字はインストーラ内の綴り） | サイズ | SHA-256（`--hash`） | 扱い |
|---|---:|---|---:|---|---|
| 56470 | 0x2f4c70ba | `QuickFontCache.dll` | 1,260,544 B | `c9b74d13c33048cf2c9ededc7c8fcc4f8c1dbfc1ee4dda5c3bf217ab64014a0b` | 32 ビット版。**抽出していない**（ゲストが開く名前ではない。`loadlibrary-names-classification.md` §5.1 末尾: `_x64` なしは EXTRACTFILE に無く、`_x64` 付きがある。`f_6539e0` が `dl=1` で `_x64.dll` を付ける読み） |
| **56472** | 0x2f523c1c | **`QuickFontCache_x64.dll`** | **2,067,968 B** | **`afc5a0ed70eb7e4af3d0f4baf771bd6f53eb014e237f857f23023471047736d2`** | **抽出した** |

- `quickfont` を含む名前は上の 2 件だけ（`fontcache` でも同じ。実測）。**64 ビット版の同名ファイルは 1 件**なので、選択の根拠を要する状況は無い。`file` は「PE32+ executable (DLL) x86-64、10 sections」。`sha256sum` は `--hash` と一致。インストーラ内の保存長は 474,595 B（圧縮。`nsis-ctx-qfc.txt`）。
- 配置先: 直前のエントリ 56448 が `<NSIS 変数>\Shared`（`op=11`）で、Blend2D（56450）や dsp_ippv2_x64（56484）と同じ `Shared` に置かれる並び（`nsis-ctx-qfc-wide.txt`。NSIS の変数がどの変数かは未確認）。
- FLEngine の `f_6539e0` が最終的に `LoadLibraryW` へ渡す名前は、仮の NULL で止めた測定で `quickfontcache_x64.dll`（パスなし。`registry-unit153.md` §6.1 の停止メッセージ。**他者の実測**）。`fle_realmod.h` の実在モジュールの名前の綴り（小文字・拡張子つき）と同じ形。runtime と fe-lift の `src`・`tests` に `quickfont` の文字列は無い（`grep -n 'quickfont\|QuickFont'`）。

## 3. PE の事実（項目 3）

出典: `fe-pe pe-info`、`pe-pdata-info`、`pe-exports`（`pe-info.txt`、`pe-pdata-info.txt`、`exports.txt`）、`objdump -p`（`objdump-p.txt`）、pefile 2024.8.26（`py/qfc_peinfo.py` → `pe-pefile.txt`）。すべて実測。

### 3.1 ヘッダ

| 項目 | 値 |
|---|---|
| Machine / Characteristics | 0x8664 / 0x2022（DLL、LARGE_ADDRESS_AWARE） |
| ImageBase / SizeOfImage / SizeOfHeaders | **0x400000** / 0x20e000（2,154,496 B） / 0x400 |
| エントリポイント | RVA 0x19c1f0（VA 0x59c1f0）。`.pdata` の関数開始 |
| DllCharacteristics | 0x0160（HIGH_ENTROPY_VA、DYNAMIC_BASE、NX_COMPAT） |
| Subsystem / リンカ / OS | 2 / 8.0 / 6.0。TimeDateStamp 0x6abbb8ff、CheckSum 0 |
| スタック / ヒープ予約 | 0 / 0x100000（commit 0x2000） |

### 3.2 セクション

| 名前 | VA | VirtualSize | ファイル上のサイズ | 特性 |
|---|---:|---:|---:|---|
| `.text` | 0x1000 | 0x1a8098 | 0x1a8200 | 0x60000020（コード、実行、読み） |
| `.data` | 0x1aa000 | 0x21588 | 0x21600 | 0xc0000040 |
| `.bss` | 0x1cc000 | 0xeb68 | 0 | 0xc0000000 |
| `.idata` | 0x1db000 | 0x1daa | 0x1e00 | 0xc0000040 |
| `.didata` | 0x1dd000 | 0x39c | 0x400 | 0xc0000040 |
| `.edata` | 0x1de000 | 0x170 | 0x200 | 0x40000040 |
| `.rdata` | 0x1df000 | 0x45 | 0x200 | 0x40000040 |
| `.reloc` | 0x1e0000 | 0x16834 | 0x16a00 | 0x42000040 |
| `.pdata` | 0x1f7000 | 0x12984 | 0x12a00 | 0x40000040 |
| `.rsrc` | 0x20a000 | 0x3800 | 0x3800 | 0x40000040 |

実行セクションは `.text` の 1 つだけ（`IPPCODE` のような第 2 のコードセクションは無い。決定 0074 の複数範囲の対応は要らない）。`.idata`、`.didata`、`.rdata`（0x45 B だけ）の並びは Embarcadero（Delphi）のリンカの形で、`.rdata` に定数は無く `.text` の中に文字列と表が入る（`.rsrc` から `Embarcadero` と `Delphi` の文字列が出る。`strings-dll.txt`。ツールチェーンの断定はしない）。FLEngine にも同じ性質がある（`dll-nonpdata-reloc-seeds.md` §7.1 は FLEngine の DIR64 スロットの多くが `.text` にあると書く）。

### 3.3 export（10 件、全部。フォワーダなし）

| 序数 | 名前 | RVA | VA（ImageBase 0x400000） | 備考 |
|---:|---|---:|---:|---|
| 1 | `dbkFCallWrapperAddr` | 0x1d22b0 | 0x5d22b0 | `.bss` の変数（コードではない）。`fe-decode` が「export outside .text (data)」で種から外す（`seeds_skipped 1`）。FLEngine も同名を export する（RVA 0x11ec2b0） |
| 2 | `__dbk_fcall_wrapper` | 0x19a70 | 0x419a70 | FLEngine も同名を export する（RVA 0x1d5a0） |
| 3 | `InitSharedPython` | 0x19a2b0 | 0x59a2b0 | FLEngine が引く |
| 4 | `CreateVST3Instance` | 0x18a200 | 0x58a200 | FLEngine が引く |
| 5 | `LoadSharedElastiqueDll` | 0x186c90 | 0x586c90 | FLEngine が引く |
| 6 | `AcquireSharedAnimTimer` | 0x1864f0 | 0x5864f0 | FLEngine が引く |
| 7 | `AcquireSharedThreadBank` | 0x185190 | 0x585190 | FLEngine が引く |
| 8 | `FontCache_GetStats` | 0x17faf0 | 0x57faf0 | FLEngine が引く |
| 9 | `FontCache_CleanUp` | 0x17f980 | 0x57f980 | FLEngine が引く |
| 10 | `FontCache_AcquireFont` | 0x17f7f0 | 0x57f7f0 | FLEngine が引く |

export 名の DLL 名フィールドは `QuickFontCache_x64.dll`。コードの export 9 個は `.pdata` の関数開始（実測: `fe-decode closure --pdata` の `seeds_code`）。

### 3.4 import（217 名 + 遅延 6 名）

全一覧は `pe-pefile.txt`（`IMP` と `DLY` の行）と `imports-vs-table.txt`（行あり／行なしを分けたもの）。

| DLL | normal 名数 | 行あり | 行なし |
|---|---:|---:|---:|
| kernel32.dll | 105 | 85 | 20 |
| gdi32.dll | 59 | 6 | 53 |
| user32.dll | 27 | 14 | 13 |
| oleaut32.dll | 16 | 16 | 0 |
| advapi32.dll | 4 | 4 | 0 |
| winmm.dll | 3 | 3 | 0 |
| ole32.dll | 2 | 2 | 0 |
| shell32.dll | 1 | 0 | 1（`SHGetFolderPathW`） |
| **合計** | **217**（序数 0） | **130** | **87** |

遅延 import（`.didata`、5 記述子）: `kernel32!GetLogicalProcessorInformation`（行あり）、`kernel32!GetNativeSystemInfo`（行あり）、`kernel32!NormalizeString`（行なし）、`user32!MessageBoxA`（行なし）、`msimg32!AlphaBlend`（行なし）、`DWMAPI.DLL!DwmFlush`（行なし）。遅延 import は `jmp [IAT]` のスタブを持たず、ヘルパー経由で `GetProcAddress` に届く（`imports_table.rs` 198 行のコメントが同じ扱いを書く）。

- **名前の一致の取り方**（`imports_table::lookup` と同じ。`py/qfc_imports.py`）: DLL 名は**大文字小文字を区別しない**（`DWMAPI.DLL` は `dwmapi.dll` として比べる）。API 名は**完全一致（大文字小文字を区別）**。序数 import は名前を持たないので行に一致しえないが、QFC は**序数 import が 0 件**。API 名を大文字小文字なしで比べても数は同じ（130 / 2）。
- 行なし 87 名の内訳は gdi32 53、kernel32 20、user32 13、shell32 1。うち **83 名は FLEngine も import しており、FLEngine でも行なし**。FLEngine が import しない 4 名は `CreateSemaphoreW`、`ReleaseSemaphore`、`InterlockedPopEntrySList`、`InterlockedPushEntrySList`（`imports-vs-table.txt` と同じ表を FLEngine にも当てた。FLEngine の normal import は 738 名で行なし 514）。
- 行ありの 130 名の runtime 側の本体を `b2d_impclass.py`（Blend2D の作業者のスクリプトを読み取りで再利用。テキスト判定）で見ると、呼べば必ず名前付き停止になるのは **3 名**（`IsDebuggerPresent`、`RtlUnwindEx`、`UnhandledExceptionFilter`）で、127 名は本体がある（引数次第で止まるものを含む。`impclass.txt`）。
- 呼び方は FLEngine と同じ `jmp [IAT]` のスタブ形で、`fe-decode closure` の `import_calls` は 0、`import_jumps` は 266（実測）。**`call [IAT]` の形は無い**（dsp_ippv2 の 138 か所、Blend2D の 264 か所との違い）。決定 0077 は要らない。
- 閉包に入る行なしの `jmp [IAT]` の呼び出し点は 88 か所・86 名（`unsupported` の 88 と一致）。残る 1 名は閉包に thunk が無い（どれかは調べていない）。

### 3.5 ディレクトリ

| 項目 | 結果 |
|---|---|
| `.pdata` | 6,347 エントリ、**連鎖 0**、関数開始 6,347（`fe-pe pe-pdata-info`。dsp_ippv2 は連鎖 19,713） |
| `.reloc` | ブロック 271、**DIR64 44,887**、ABSOLUTE 135、他の種別 0（pefile）。fe-pe の `rebase` が 44,887 / 135 で通る。DIR64 のスロットの所在は `.text` 42,993、`.data` 1,882、`.didata` 12 |
| 遅延 import | **あり**（上） |
| TLS ディレクトリ | **なし**（データディレクトリ 9 が 0。`.tls` セクションなし）。Blend2D は暗黙の TLS を持つが QFC は持たない |
| Load Config（CFG を含む） | **なし**（データディレクトリ 10 が 0） |
| デバッグディレクトリ | **なし**（データディレクトリ 6 が 0） |
| Bound import / COM descriptor / 証明書 | なし / なし / なし |
| リソース | あり（`.rsrc` 0x3800 B）。`FindResourceW` の対象になるかは調べていない |

### 3.6 基底の重なりと、空いている基底の候補（事実。決めない）

既存の領域は `two-module-design.md` §2.2、`dsp-ippv2-lift-feasibility.md` §2.2、各 `fle_*.h`（今回 `grep` で再確認）から取った。

| 領域 | 範囲 | QFC の ImageBase 0x400000〜0x60e000 との関係 |
|---|---|---|
| FLEngine のイメージ | 0x400000〜0x1cc0000 | **重なる**（開始が同じ） |
| dsp_ippv2（リベース後。決定 0080 D1） | 0x2000000〜0x461f000 | 重ならない |
| `GLOBAL_BASE`（wasm のデータ、スタック、ヒープ。`build.sh` 35 行） | 0x10000000 以上 | 重ならない |
| 仮想ハンドルのトークン | 0x7F990000 以上（ウィンドウ 0x7F990000、スクリーン DC 0x7FA90000、GDI 0x7FBA0000、標準カーソル 0x7FCA0000、DLL のカーソルとアイコン 0x7FDA0000、モニタ 0x7FEA0000）、モジュールハンドル 0x7FF800000000 + 0x10000×N、擬似 VA 0x7FF900000000 + 0x10×k | 重ならない |
| 番兵 | 0xCB000000（コールバックの戻り）、0xdead0000（ハーネスの戻り） | 重ならない |
| `OR_SCRATCH` | 0x20000000（サイズ 0x100000。`harness/oracle/` のオラクル専用） | 重ならない（本体の runtime の領域ではない） |
| 依頼文の「アリーナ 0x2100000〜0x3100000」「TEB 0x1D00000」 | **定義が見つからない**（`runtime/` と `docs/` の grep。決定 0032 の記述以外に 0 件。実際のアリーナは `aligned_alloc` の 512 MiB で固定アドレスではなく、TEB は静的バッファ `fle_teb[0x2000]`。`dsp-ippv2-lift-feasibility.md` §0、`two-module-design.md` §2.2 と同じ結論） | — |

ImageBase のまま FLEngine と同時に置くことはできない（同じ範囲）。**QFC を置ける基底の候補**（SizeOfImage 0x20e000。すべて 64 K の倍数。4 GiB 未満。上の領域と重ならない）:

| 基底 | 終端 | 備考（事実の算術） |
|---|---|---|
| 0x1cc0000 | 0x1ece000 | FLEngine の終端ぴったり。dsp_ippv2 の基底 0x2000000 までに 0x132000 B 残る |
| **0x1d00000** | 0x1f0e000 | FLEngine と dsp_ippv2 の**隙間（0x340000 B）に収まる**。`fe-pe rebase` で測定済み（下）。依頼文が挙げた「TEB 0x1D00000」は定義が無いので、この値に使用中の領域は無い |
| 0x1df0000 | 0x1ffe000 | 終端が dsp_ippv2 の基底 0x2000000 以下になる最大の基底。0x1e00000 以上は dsp_ippv2 と重なる |
| 0x4700000 | 0x490e000 | dsp_ippv2 の終端 0x461f000 の後。Blend2D の調査が例に使った値 |
| 0x8000000 | 0x820e000 | `two-module-design.md` §3.2 の P2 の流儀。`GLOBAL_BASE` まで残り |

- **FLEngine と dsp_ippv2 と QFC の 3 つを隙間に詰める**場合、隙間 0x340000 B から QFC の 0x20e000 B を引いた 0x132000 B が残る。**Blend2D（SizeOfImage 0x300000）はこの残りに入らない**（算術。Blend2D の置き場は別の調査の論点 B7）。
- 単一の範囲しか持てないもの: `fle_mem_img_lo` と `fle_mem_img_size`（`fle_host.c` 13〜14 行。HEAD でも 1 範囲）。`FLE_CHECKED_MEM` を使うなら 3 範囲化が要る（`dsp-ippv2-lift-feasibility.md` §2.1、`blend2d-lift-feasibility.md` §8 と同じ指摘）。
- リベースの実測（`fe-pe rebase`。決定 0080 のツール。`out-qfc/rebased/qfc.<基底>.dll`。実在の DLL ではなく測定用のコピー）: 基底 **0x1d00000、0x4700000、0x8000000** のどれでも DIR64 44,887・ABSOLUTE 135・チェックサム 0 で通り、`fe-lift lift … --pdata` の統計は `c_bytes` を除き**全行が同一**（`logs/rebased-*.txt`。`c_bytes` は 17,656,503 → 18,171,739。`0x400000`〜の 6 桁の即値が 7 桁になるため増える）。
- **関数名の衝突**（`data/qfc-funcnames.txt`、`fle-funcnames.txt`、`qfc-1d00000-funcnames.txt`）: リベースなしの QFC の生成 C は `void f_<VA>(void)` の名前が FLEngine と **374 個同じ**。0x1d00000 にリベースすると **0 個**。したがって 2 つを 1 つのプログラムにリンクするには、ImageBase のままでは不可で、リベースが要る（`dsp-ippv2-lift-feasibility.md` §2.5 の 5 の「VA が重ならなければ衝突しない」の確認）。

## 4. lift の規模（項目 4）

手順（`lift/` で実行。`TMPDIR=lift/out-qfc/tmp`、`PYTHONDONTWRITEBYTECODE=1`、`--offline`）:

```
CARGO_TARGET_DIR=out-qfc/target cargo build --release --offline -p fe-pe -p fe-decode -p fe-lift     # 3.0 s（out-qfc/build.log）
fe-lift lift out-qfc/quickfontcache_x64.dll /dev/null --pdata                    > logs/A-pdata.txt
fe-decode closure out-qfc/quickfontcache_x64.dll --mnemonics --pdata --adjustors > logs/closure-pdata.txt
fe-decode reloc-seeds FILE [--not-function]                                      > data/reloc-seeds.txt, data/reloc-not-function.txt
fe-lift lift FILE /dev/null --pdata --seeds data/…                               > logs/B-…, C-…, D-…
fe-pe rebase FILE 0x1d00000 OUT                                                  → rebased/
```

README の形は `fe-lift lift FILE OUT.c --pdata`。`fe-lift` に `--reloc-seeds` のフラグは**無い**（使い方は `[--pdata [--no-adjustors]] [--seeds SEEDS.txt]`）。reloc seed の効果は `fe-decode reloc-seeds` の出力を `--seeds` で渡して測った（`dll-nonpdata-reloc-seeds.md` §2 と同じ形）。生成 C は `/dev/null` に捨て、保存していない（巨大関数の確認用に 1 関数 `logs/f_522e50.c`（1.1 MB）だけ保存した）。

### 4.1 統計（実測。`logs/A-pdata.txt`）

| 項目 | 値 |
|---|---:|
| functions | **6,712** |
| closure_instructions / emitted_instructions | **209,259** / 209,279 |
| bad_nodes / decode_failures | **0** / 0 |
| tail_calls / gotos | 95 / 20,922 |
| c_lines / c_bytes | 250,351 / **17,656,503**（FLEngine の生成 C 約 215.7 MB の 8%） |
| unsupported_total | **104**（emitted の 0.05%） |
| jumptabs / register_jmp_sites | 0 / 5（`jmp_ind:reg_dispatch` 6、`jmp_ind:rip_table` 6、`call_ind:dispatch` 2,302、`call_ind:rip_table` 11） |
| 所要 / 最大 RSS | 0.18 s / 31,200 KB |

`fe-decode closure`: 基本ブロック 36,015、直接 call 18,188、直接 jmp 6,114、条件分岐 14,897、ret 6,354、`import_jumps` 266、間接 call（その他）2,313、間接 jmp（その他）13、範囲外の流れと呼び 0、distinct のニーモニック 139。`--no-adjustors` では関数 6,619・命令 209,073（`unsupported` は同じ 104）。種なし（エントリと export だけ）では関数 547・命令 22,931・unsupported 12（`H-noseeds.txt`）。

### 4.2 reloc seed と、非 pdata の reloc 値（リーフ関数）（実測）

`dll-nonpdata-reloc-seeds.md` の (A)(B)(C) と同じ見方。

| 構成 | 種 | functions | closure_instructions | bad_nodes | unsupported_total | c_bytes |
|---|---:|---:|---:|---:|---:|---:|
| **(A) `--pdata`**（基準） | — | **6,712** | 209,259 | **0** | 104 | 17,656,503 |
| (B) `--pdata` + reloc seed（`.pdata` 開始の reloc 値） | 3,097 | 6,712 | 209,259 | 0 | 104 | 17,656,503 |
| (Y1) reloc seed のみ（`--pdata` なし） | 3,097 | 5,043 | 184,804 | 0 | 87 | 15,557,461 |
| (C) `--pdata` + 非 pdata の reloc 値の全件 | 9,293 | 16,082 | 414,416 | **17,541** | 62,798 | 87,566,288 |
| (D) `--pdata` + 葉関数らしい形の 155 個 | 155 | 6,774 | 211,394 | **48** | 334 | 18,014,909 |
| (D-ret) 同、`ret` で終わる 31 個だけ | 31 | 6,743 | 209,716 | **10** | 157 | — |
| (D-jmp) 同、直接 `jmp` で終わる 124 個だけ | 124 | 6,743 | 210,952 | **38** | 282 | — |

- reloc の集計（`fe-decode reloc-seeds`）: スロット 44,887、読めないスロット 0、実行範囲内を指す値 12,390、うち `.pdata` 開始 **3,097**（seed）、開始でない **9,293**（`reloc_not_function`）。**(B) は (A) と同一**（`.pdata` 開始の値はもう全部閉包に入っている）。
- **非 pdata の reloc 値 9,293 の内訳**（`py/qfc_nonpdata.py`。`.pdata` は連鎖なし）: **10** が RUNTIME_FUNCTION の範囲の途中（関数の途中。16 バイト境界のものは 0）、**9,283** がどの RUNTIME_FUNCTION の範囲にも入らない（すべて `.text`。16 バイト境界は 2,299）。これらを指すスロットは `.text` 23,278、`.data` 881、`.didata` 12 で、3 つ以上連続するスロットの並び（表）に入るのは 2,968、孤立が 21,213。
- 9,283 の先頭の命令列を、不正バイトを検出できる設定の capstone で読むと（`logs/nonpdata-strict.txt`。先頭 16 命令または最初の `ret/jmp/int3/ud2/hlt`）、**5,151 個が不正バイトで止まる**（コードではなくデータ（表や RTTI）を指す値が多い）。`call` を含むものが 176、`push/pop/rsp` の加減算を含むものが 4,476。
- **リーフ関数らしい形**（`call`、`push/pop`、`rsp` の加減算、`retf/int3/hlt/ud2` を含まず、24 命令以内に `ret` か直接 `jmp` に届き、不正バイトが無い。`py/qfc_leafcand.py`）は **155 個**（`ret` 31、直接 `jmp` 124）。**種にすると `bad_nodes` が出る**（上の (D)）ので、この形は入口であることの証拠にならない。
- 結論（事実）: **dsp_ippv2 では規則「非 pdata の reloc 値を種にする」で `bad_nodes` が増えなかったが、QFC は FLEngine と同じ側**（`dll-nonpdata-reloc-seeds.md` §7.1: FLEngine で 0 → 113,485）。QFC の起動に要る関数（DllMain、unit の proc 154 個と終了側 154 個、export 9 個）は**すべて `.pdata` の関数開始**（`py` で確認）なので、`--pdata` だけで足りる。**非 pdata の reloc 値を種にする必要は、今の範囲では見つかっていない**（実行していないので、vtable や関数ポインタ経由で `.pdata` に無い関数に入る可能性は未確認）。

### 4.3 unsupported の内訳（実測。`logs/A-pdata.txt`、`logs/unsup-sites.txt`）

`stage23-two-module-real-dll.md`、`dsp-ippv2-lift-feasibility.md` §4 と同じ数え方（生成 C の `fle_unsupported` の箇所）。

| 理由 | 箇所数 | 備考 |
|---|---:|---|
| `Jmp: indirect through IAT slot (not lifted)`（行なしの API の thunk） | **88**（86 名） | `FormatMessageW` 2、`MessageBoxW` 2、`CreateThread`、`ExitThread` ほか。gdi32 の 53 名が多くを占める |
| SSSE3（SSE3 以降）: `Pshufb` 6、`Phaddw` 4、`Pmulhrsw` 4 | 14 | **2 関数**（`f_57de50` に 4 箇所、`f_57e2a0` に 10 箇所）。どちらも `f_57e600` が実行時に選ぶ（§4.4） |
| `Jmp: indirect target unresolved` | **2** | `f_40c120`（0x40c163 `jmp [r10+rdx*8+0x40]`）と `f_40f9c0`（0x40fa22 `jmp [rcx]`。直前が `mov rsp,[rcx+8]` で、レジスタを保存する `f_40f950`（setjmp に似た形）と対になる復帰（longjmp に似た形）に読める。読み） |
| VEX（`V*`）の未対応 | **0** | 閉包の VEX は `vmovups` 12、`vmovaps` 2、`vmovntps` 2、`vzeroupper` 2 だけで、決定 0032 で対応済み。すべて `f_409840`（メモリコピーに見える関数）にある |
| EVEX / AVX-512（`K*`、ZMM、opmask） | **0** | 未対応の一覧に出ない |
| x87 | 0（未対応） | 対応済みは `fldcw` 1、`fnclex` 1、`fnstcw` 1、`emms` 1 |
| その他のレガシー命令（SSE3 以降を除く） | 0 | |
| `gs:` の未モデル | 0 | `gs:[0x58]` が 1 か所（`handled` の `mov_gs`。決定 0032 の対応済み）。`f_4195e0`（TLS 値の取得）の中で、DllMain の ATTACH が `[0x5d22d8]` を 1 にするので通らない枝に読める（読み） |
| **合計** | **104** | |

`handled` の特記: `cpuid` 5、`xgetbv_stop` 1、`ldmxcsr` 5、`stmxcsr` 2、`cmpxchg16b` 5、`pause` 5、`prefetcht0` 5、`sfence` 2、`movs` 522、`stos` 245。

### 4.4 CPU 振り分け（実測と読み）

- **決定 0079 の形（`movsxd; lea; jmp [r10+rax*8]`、バイト列 `41 ff 24 c2`）は 0 個**（`.text` を走査。`fe-lift` の `jumptabs` も 0）。dsp_ippv2 の 434 個、Blend2D の 0 個と比べる。
- `cpuid` 5 か所、`xgetbv` 1 か所（`callsites.txt`、`cpuid-funcs.txt`）:
  - `f_40c790`（0x40c79b）: 汎用の `cpuid(葉, 副葉)` の薄い関数（読み）。到達する QFC の unit は 1（静的）。
  - `f_51b320`（0x51b32d）: 葉 0 などを引く汎用の関数。`f_51b3f0`（0x51b3f5、0x51b401、**`xgetbv` 0x51b413**）: 葉 0 の最大葉、葉 1 の ECX を `0x18000000`（OSXSAVE と AVX の両ビット）と比べ、両方立つときだけ `xgetbv` を実行し `&6 == 6` を返す（AVX の OS 対応の判定。読み）。到達する QFC の unit は 72。
  - `f_45b460`（0x45b466）: 葉 1 の ECX から機能マスクを作る（ECX の bit 0、9、19、20、23、25、1 を調べ、基底値 3 に 4、8、0x10、0x20、0x40、0x80、0x100 を足す。読み）。到達する QFC の unit は 37。
- 現行の CPUID 応答（葉 1 の ECX = 0。決定 0009、0047）では、`f_45b460` は基底値 3 を返し、`f_51b3f0` は ECX のビットが立たないので `xgetbv` に入らない（読み。**仮説。実行していない**）。`xgetbv` の生成は `fle_xgetbv_stop`（決定 0047。到達すると名前付き停止）。
- **SSSE3 版の選択**: `f_57e600` が `[[0x5cb508]+0x12]` の bit 19（0x80000）を調べ、立っていれば `f_57de50` と `f_57e2a0`（SSSE3 の命令を持つ 2 関数）を関数ポインタの変数 `0x5d9f78`、`0x5d9f80` に入れ、立っていなければ `f_57dcf0`／`f_57dfa0` と `f_57e100` を入れる。`0x5cb508` が指す先は実行時の変数で、**その値が CPUID の結果から来るか、来るなら現行の応答で bit 19 が立つかは未確認**（`f_45b460` が作る値とビットの並びが違う。関連は未確認）。`f_57e600` が届く QFC の unit は 146。この 2 関数は `fle_unsupported` を含むので、**選ばれて実行されると名前付き停止**になる（SSSE3 の `Pshufb` などは `fe-lift` の最小集合の外）。
- AVX 系の関数は閉包にほぼ無く（VEX は `f_409840` の 18 命令だけ）、dsp_ippv2 のような「実装系統を絞る」設計の問題は無い。

### 4.5 locals 上限（wasm の 1 関数 50,000）の見積もり（**未測定**。見方だけ示す）

`dsp-ippv2-lift-feasibility.md` と `two-module-design.md` §4.6、`blend2d-lift-feasibility.md` §3.6 と同じ上限（仕様の Implementation-defined Limits。1 関数の locals は 50,000）。**emcc を使わない規則なので、wasm の locals は測っていない**。代わりに、生成 C の関数ごとの行数と一時変数の数を測った（`py/qfc_cfuncs.py`、`qfc_feat.py`。`logs/cfuncs.txt`、`feat-*.txt`）。

| 関数 | 行数 | 本体のバイト | ブロック内の一時変数（`{ type t = `） | `fle_op_*` | `fle_ld`+`fle_st` | xmm（`X[`）参照 |
|---|---:|---:|---:|---:|---:|---:|
| QFC `f_522e50` | **13,559** | 1,094,671 | 1,700 | 2,121 | 7,198 | 0 |
| QFC の 2 番目 `f_582290` | 1,634 | 143,339 | 283 | 372 | 822 | 562 |
| QFC の 3 番目 `f_5910c0` | 1,309 | 117,928 | 260 | 301 | 886 | 0 |

- QFC で 1,000 行以上の関数は **4 個**、2,000 行以上は **1 個**（`f_522e50`）、中央値 17 行、p99 257 行。FLEngine は 1,000 行以上が 126 個、最大 12,817 行（`f_13173a0`）。
- **`f_522e50` の中身**（読み。`logs/f_522e50.c`、`f_522e50-head.txt`）: `dsp_ippv2.dll` を `f_522d20(name, dl=1, dir=0)` で開き、`ippMalloc` から順に `GetProcAddress` の名前を引く（まず `ipp…`、無ければ `legacy90ipp…` を引き、どちらも無ければ `"IPP function not found: …"` の文字列を `f_41d970`（`OutputDebugStringW` の thunk）に渡して偽を返す）。命令の内訳は `Mov` 5,504、`Call` 2,540、`Lea` 1,693、`Jne` 1,271、`Cmp` 848、`Xor` 848、`Jmp` 424、`Test` 423 で、**SSE も `fle_unsupported` も無い**直線的なコード。UTF-16 の名前文字列 1,270 のうち `ipp…` 系が 846（`legacy90` 付き 423 + 付かない 423）、distinct は 420 名で、**FLEngine が dsp_ippv2 に渡す 420 名（`out-ext/ipp_names.txt`）と完全に同じ集合**。FLEngine の対応する関数 `f_653f40` は 3,411 行・一時変数 855 で、QFC の方が 4 倍近く長い（`legacy90` の再試行を持つため）。
- **比較の材料**（他者の実測。再測定していない）: Blend2D は同じ `emcc -O0` で、`f_180159b70`（11,136 行、一時変数 1,771）が **locals 102,086**、`f_180076f60`（8,068 行、一時変数 570）が 36,848。locals / 行は 4.57〜9.17。FLEngine は最大 20,136（`flengine.wasm` の関数 #17538、本体 855,379 B。名前の節が無く、どの C 関数かは対応づけていない）。dsp_ippv2 の全部入りは 13〜15 関数が 50,000 超（最大 389,948）。
- **見積もり**（推測。根拠が弱いと明記する）: `f_522e50` は行数が Blend2D の 4 つの巨大関数（10,840〜11,136 行）より多く、一時変数の数（1,700）は同じ級（1,771）。Blend2D の locals / 行（4.57〜9.17）を当てはめると約 62,000〜124,000、locals / 一時変数（Blend2D の 57〜65）を当てはめると約 98,000〜110,000 で、**どちらも 50,000 を超える**。一方 FLEngine の最大 20,136 は同じ見方（行数 12,817 の関数が 1 つの 20,136 に対応するなら locals / 行は 1.6）では逆に小さい。**比が 1.6 から 9 まで散るので、`f_522e50` が上限を超えるかは言えない**。超えるなら、dsp_ippv2 の先例（決定 0080 D7 の注: 縮小、生成器の一時変数の再利用、関数の分割、`-O1`）がそのまま当てはまる。**この関数は QFC の ATTACH で呼ばれる**（§6.2）ので、生成から外すと unit 88 が `fle_dispatch_miss` で止まる。測るには wasm の `-c` コンパイルが要る（Blend2D の調査は 1 TU を 3.3 GB・17.9 s で通した）。

## 5. ゲスト（FLEngine）の使い方（項目 5。読み・逆アセンブルのみ）

方法: `binaries/win64/FLEngine_x64.dll` を `lift/out-loadlib/llpe.py`（読み取りの補助。capstone の線形走査）で読み、文字列の xref と保存先の参照は `py/fle_xrefs.py`、`py/fle_refs.py`（`lea`／`mov` の rip 相対を全バイト位置から探して capstone で確かめる）、unit への静的到達は `py/fle_reach.py`（`lift/out-loadlib/units.tsv`、`edges-direct.tsv`、`edges-ptr.tsv` を読み取りで使う。到達の限界は §0）で取った。出力は `fle/`。capstone を使う python は `lift/out-cs/` を cwd にしていない。

### 5.1 GetProcAddress で QFC から引く名前（8 名。すべて export に実在）

FLEngine で `QuickFontCache` の文字列を持つのは、DLL 名 `QuickFontCache.dll`（0x665c6c。`f_665bd0` が `f_6539e0(…, dl=1)` に渡す）、`QuickFontCache_x64.dll`（0x653428。`f_6532f0` という `GetFileAttributesW` でディレクトリ候補を探す関数の一部。`f_6539e0` が `_x64.dll` を付けたあとの名前の候補探し）、メッセージ 2 つ（`QuickFontCache DLL not found!`、`Failed to load QuickFontCache`）。**GetProcAddress に渡す名前は次の 8 つで、すべて QFC の export にある**（`py/fle_xrefs.py` で名前の文字列と xref を取り、`exports.txt` と突き合わせた。差集合は空）。

| 名前 | 引く関数（`lea rdx` の VA） | 保存先（slot）の VA | スロットを読む関数 | 備考 |
|---|---|---|---|---|
| `FontCache_AcquireFont` | `f_665bd0`（0x665c0f） | 0x15f4628 | `f_6685d0`（0x668620。`call [[0x15e0b48]]` の形） | |
| `FontCache_CleanUp` | `f_665bd0`（0x665c29） | 0x15f4630 | 読み手は直接のセルに見つからず（未確認） | `f_665d10`（終了側）が `FreeLibrary` 後に QFC のハンドルを 0 にする |
| `FontCache_GetStats` | `f_665bd0`（0x665c43） | 0x15f4638 | `f_668980`（0x668985。`call [[0x15e3330]]`） | |
| `AcquireSharedThreadBank` | `f_66cfa0`（0x66cfae） | 0x15f4648 | `f_93f160`（0x93f16d。`call [[0x15e1160]]`、`ecx = 1`） | **実行は unit 158 の `f_66d050` から**（§5.3） |
| `AcquireSharedAnimTimer` | `f_66cfa0`（0x66cfce） | 0x15f4658 | `f_736ff0`（0x736ffe。`call [[0x15e1fc0]]`） | 同上 |
| `LoadSharedElastiqueDll` | `f_677d10`（0x677d1e） | 呼び出し結果を `call rax` で直接呼ぶ | — | 取得した関数に `[0x15f4850]`、`[0x15f4858]` を渡し、成功なら ElPro 系 5 名（`ElPro_CreateInstance`、`ElProDirect_CreateInstance`、`ElProV3_CreateInstance`、`ElProV3Direct_CreateInstance`、`ElResampleSource_CreateInstance`）を **Elastique 側の DLL**（QFC が `LoadSharedElastiqueDll` の中で読む `Elastique.dll`）から引く |
| `CreateVST3Instance` | `f_b47620`（0xb4763e） | 0x14edb18 | `f_b4da00`（0xb4db7f。`call [0x14edb18]`。`TVST3Plugin.LoadPlugin` の文字列を持つ） | 先頭で `[[0x15e0a50]] != 0`（QFC のロード済み）を見る。NULL なら `f_b47610`（スロットを 0 にする）を呼び偽を返す |
| `InitSharedPython` | `f_c393a0`（0xc394d3） | `call rbx`（直接） | — | 取得結果が NULL でなければ呼ぶ（`test rbx,rbx; je`） |

- `[0x15e0a50]` は `.data` の 8 バイトで、値は `0x15f4620`（`.bss`。ファイル上のバイト列）。`0x15f4620` は **QFC の LoadLibrary の結果（ハンドル）を `f_665bd0` が保存する先**（0x665bf2）と同じ。つまり `[0x15e0a50]` が「QFC のハンドルの変数へのポインタ」で、`f_66cfa0`、`f_677d10`、`f_b47620`、`f_c393a0` はどれも `mov rax,[0x15e0a50]; mov rcx,[rax]` で**同じハンドルを読んで**から `GetProcAddress` を呼ぶ（`registry-unit153.md` §6.3 の 4 と一致）。
- 8 名のうち、`LoadLibrary` の直後に連続して引くのは `f_665bd0` の 3 名だけ。ほかは使う場面（unit 158 の `f_66d050`、Elastique、VST3、Python）ごとに引く。名前のほかに序数で引く形は見つかっていない（`GetProcAddress` のラッパー `f_4235f0` は第 2 引数の上位 48 ビットが 0 なら序数として扱う形に読める。FLEngine が序数を渡す点は調べていない。未確認）。
- 「`f_66cfa0` 周辺、`f_665bd0`、`f_668d30` など」: **`f_668d30` は QFC を引かない**。`f_668d30`（2,525 行の逆アセンブル）は `segoeui.ttf`、`OpenSans-Regular`、`tahoma`、`P_SmallBold`、`P_Small`、`P_Tiny`、`P_Normal`… などのフォント名を扱うフォント設定の関数で、`GetProcAddress` も `LoadLibrary` も呼ばない（`fle/f_668d30.txt` の `call` の宛先に API のラッパーが無い）。`f_6685d0` と `f_6689a0` などのフォント表の関数を呼ぶ。つまり**フォントの実体を QFC に頼む使い方**が静的に見える（読み）。

### 5.2 ハンドルの保存先と NULL のときの分岐（読み）

- ロード点 `f_665bd0`（0x665bd0〜0x665c5e）:
  1. `cmp [0x15f4620],0; jne 0x665c56`（すでにロード済みなら `bl = 0` のまま戻る。**戻り値は 0（偽）で、`jne` で飛んだときは `xor ebx,ebx` のまま**（読み。再呼び出しで偽を返す形に見えるが、`f_66d050` が `[0x13e86dc]` で 2 回目を止めるので通らない）。
  2. `f_6539e0(L"QuickFontCache.dll", dl=1, r8=0)` → 戻り値を `[0x15f4620]` に保存。
  3. `cmp [0x15f4620],0; setne bl; test bl,bl; je 0x665c56`（NULL なら `GetProcAddress` 3 回を飛ばして `bl = 0`）。非 NULL なら 3 名を引いて `[0x15f4628]`、`[0x15f4630]`、`[0x15f4638]` に保存し `bl = 1`。
- 呼び出し元 `f_66d050`（0x66d050〜0x66d09c）: `[0x13e86dc]`（初期化済みフラグ）が 0 のとき、`f_665bd0` が偽なら **`MessageBoxW(0, "QuickFontCache DLL not found!", "Failed to load QuickFontCache", 0)`**（`f_425180` 経由）を呼び、**そのまま続けて** `f_66cfa0`（`AcquireSharedThreadBank`、`AcquireSharedAnimTimer` を引く）、`f_668d30`（フォント設定）を呼び、フラグを 1 にする。つまり**ゲスト自身は「QFC が無い」ときにダイアログを出すが、そこで止まらず、`GetProcAddress(*[0x15e0a50]=NULL, …)` へ進む**（`registry-unit153.md` §6.3 の 4）。
- **NULL を許容しない箇所**（読み）: スロットを読む `f_93f160`（`ThreadBank`）、`f_736ff0`（`AnimTimer`）、`f_668980`（`GetStats`）、`f_6685d0`（`AcquireFont`）は、**呼ぶ前にスロットの中身を検査しない**（`call [rax]`、`rax` は `mov rax,[0x15e…]` で得たスロット変数へのポインタの参照先）。スロットが 0 なら 0 番地を呼ぶ。`f_b4da00` は `CreateVST3Instance` の結果を `call [0x14edb18]` で直接呼ぶが、`f_b47620` が NULL のとき `f_b47610` でスロットを 0 にして偽を返し、`f_b4da00` は偽なら `"LoadHostDLL failed - EXIT"` へ進む（読み。**検査がある**）。`f_677d10` と `f_c393a0` は `test rax,rax` で検査する。したがって **8 名のうち 4 名（`AcquireFont`、`GetStats`、`AcquireSharedThreadBank`、`AcquireSharedAnimTimer`）は、QFC が無いと 0 番地を呼ぶ形**で、ロード点の NULL 許容（`loadlibrary-names-classification.md` §2.2 #5 の「許容」）は「ロードの瞬間」の話に限られる。**この 4 つの使い手が実際に呼ばれるかは未確認**（呼び出し元の静的な unit は次の §5.3）。

### 5.3 名前ごとの、どの unit で呼ばれうるか（静的な到達。読み）

`py/fle_reach.py` の結果（`fle/reach-1.txt`、`reach-2.txt`）。「直接」は直接 call/jmp だけ、「lea 込み」は `lea reg,[rip+関数]` の辺を足した上限（`loadlibrary-names-classification.md` §0.2 と同じ見方。FLEngine の unit は `units.tsv` の index。ゲストの InitCount は index + 1）。

| 関数 | 役割 | 到達する FLEngine の unit（直接 / lea 込み） |
|---|---|---|
| `f_66d1a0`（unit 158 の proc） | `[0x15f4678]` の参照カウントが -1 になったら `f_66d050` を呼ぶ | 158（直接 158 のみ） |
| `f_66d050` | QFC のロード、`AcquireShared*` の取得、フォント設定 | 158、245、279、647（直接も lea 込みも同じ） |
| `f_665bd0`、`f_66cfa0`、`f_668d30` | 同上の各部分 | 158、245、279、647 |
| `f_665d10`（QFC の `FreeLibrary` とハンドルのクリア） | `f_66d120`（`[0x13e86dc]` が 1 のとき `f_66cf40` と `f_665d10` を呼ぶ終了側）から | **unit なし**（`f_66d140` が終了側の参照カウント加算で `f_66d120` を呼ぶ形。`units.tsv` の `second_va` 0x66d140 が finalizer の側として読める。実行するかは未確認） |
| `f_6685d0`（`FontCache_AcquireFont` を呼ぶ） | フォント表の更新 | 245、279、280、647、659、895、898（直接）／245 以降 11 unit（lea 込み） |
| `f_668980`（`FontCache_GetStats`） | 呼び出し元は `f_120b790` の 1 つ | **unit なし**（コールバックや vtable の可能性。未確認） |
| `f_736ff0`（`AcquireSharedAnimTimer` のスロット） | | 895、898（直接）／259、614、745、880、895、898（lea 込み） |
| `f_93f160`（`AcquireSharedThreadBank` のスロット） | 呼び出し元は `f_1196c00` | **unit なし**（直接も lea 込みも。`f_1196c00` は別の大きな関数。vtable 経由の可能性。未確認） |
| `f_677d10`（`LoadSharedElastiqueDll`） | 呼び出し元は `f_677f60`、`f_677fb0`、`f_678010`、`f_c67520`、`f_c69c10` | 745、895、898（直接）／614、745、880、895、898（lea 込み） |
| `f_b47620`（`CreateVST3Instance`） | 呼び出し元は `f_b4da00` | **unit なし** |
| `f_c393a0`（`InitSharedPython`） | 呼び出し元は `f_c395a0` | 895、898（直接）／614、745、880、895、898（lea 込み） |

読み: **ロード（`LoadLibraryW`）は unit 158 の起動で実行される**（`registry-unit153.md` §6 の実測）。`FontCache_AcquireFont` の使い手は、起動の中でも早い unit（245 以降）から静的に届く。`AcquireSharedAnimTimer`、`LoadSharedElastiqueDll`、`InitSharedPython` の使い手は起動の遅い方（unit 614 以降）から静的に届く。**`AcquireSharedThreadBank`、`FontCache_GetStats`、`CreateVST3Instance` の使い手は静的な unit が無い**（vtable、コールバック、ユーザー操作の経路。実行されるかは未確認）。unit 245 以降の到達は、`ILTools` の `NULL` で停止する経路（`loadlibrary-names-classification.md` §2.2 #6）と同じ範囲にある。

## 6. DLL 自身の依存（項目 6。読み）

### 6.1 import した DLL と、動的に引く DLL

- **normal import の 8 DLL**: kernel32、gdi32、user32、oleaut32、advapi32、winmm、ole32、shell32（§3.4）。FLEngine の normal import の 24 DLL のうち、comctl32、comdlg32、crypt32、dsound、imagehlp、mpr、msacm32、oleacc、opengl32、shfolder、shlwapi、version、wininet、winspool、wintrust、api-ms-win-crt-string は QFC には無い。`fle_modules[]`（仮想モジュール 30 名）には、**この 8 DLL がすべてある**（kernel32 N=9、gdi32 N=7、user32 N=19、oleaut32 N=14、advapi32 N=1、winmm N=22、ole32 N=13、shell32 N=16。`fle_modules.c` の並びから数えた）。
- **遅延 import の 4 DLL**: kernel32（`GetLogicalProcessorInformation`、`GetNativeSystemInfo`、`NormalizeString`）、user32（`MessageBoxA`）、`msimg32`（`AlphaBlend`）、`DWMAPI.DLL`（`DwmFlush`）。**`msimg32` と `dwmapi` は仮想モジュールに無い**（`fle_load_absent[]`（9 名）に `dwmapi`、`msimg32` がある。決定 0015）。遅延 import が呼ばれて `LoadLibraryA` が NULL になると、QFC の遅延ロードのヘルパーは FLEngine と同じ形（`RaiseException(0xc06d007e)` 系。**未読。FLEngine の `f_41d9a0` の形と同じかは未確認**）。
- **LoadLibrary、GetProcAddress に渡す文字列**（`py/qfc_loadsites.py`、`qfc_callsites.py`、`logs/loadsites.txt`、`callsites.txt`）:

  | 場所 | 名前 | 経路と QFC の unit（静的） |
  |---|---|---|
  | `f_522e50` | `dsp_ippv2.dll`（`f_522d20(name, dl=1, dir=0)` 経由。名前に `_x64` が付く読み） | QFC の unit 88 の proc（`f_5486e0`）から直接 call。**dsp_ippv2 の 420 名を引く**（§4.5） |
  | `f_563000` | `freetype.dll`（同じ `f_522d20`）。続いて `FT_Init_FreeType`、`FT_Done_FreeType`、`FT_Attach_File` など **14 名**の `FT_*` | QFC の unit 129 の proc（`f_563840`）から直接 call |
  | `f_586c90`（export `LoadSharedElastiqueDll`） | `Elastique.dll`（`f_5229d0` で名前を作り `f_43e9a0` で `LoadLibraryW`）。続いて `ElAllocator_SetFunctions` | export から直接。`f_587390` も `f_43e9a0` を呼ぶ |
  | `f_59a510` | `Python312`（`r8 = "Python\"`、`dl=0`）。続いて `Py_*` などを **110 回** `GetProcAddress` | `f_59a2b0`（export `InitSharedPython`）の下。unit 無し |
  | `f_548e00`（QFC の unit 92 の proc） | `dwmapi`、`user32`（`DwmDefWindowProc`、`UpdateLayeredWindowIndirect`）、`avrt`（`AvRtLeaveThreadOrderingGroup`、`AvRtWaitOnThreadOrderingGroup`）、`advapi32`（`TryAcquireSRWLock*`、`RegCopyTreeW`、`CheckTokenMembership`）、`PowrProf.dll`（`PowerGetActiveScheme`、`PowerSetActiveScheme`）、`SHCore.dll` | QFC の unit 92。FLEngine の `f_65c9f0`（unit 145）と同じ形（`dwmapi`、`avrt`、`PowrProf.dll`、`SHCore.dll` は FLEngine でも引く）。**QFC の `GetProcAddress` は 47 回**（`DwmEnableBlurBehindWindow` ほか Dwm 系を含む） |
  | `f_4f17a0` | `kernel32.dll` → `GetTickCount64` | OS のバージョンが 6 以上のとき |
  | その他 | `kernel32.dll` → `GetThreadPreferredUILanguages`、`SetThreadPreferredUILanguages`、`GetThreadUILanguage`（`f_4155b0`）、`GetLongPathNameW`（`f_415ef0`）、`CompareStringOrdinal`（`f_43d6d0`、`NTDLL.DLL` も）、`GetLogicalProcessorInformation`（`f_409580`、`f_40d7c0`）、`oleaut32.dll`、`ole32.dll`、`shell32`、`kernel32` ほか | OS の機能の有無の判定 |

- **QFC が Blend2D、ILTools、ILSvg、ildsp、oggio、wavpack、lameenc、speaker、beat_tracker、WebView2 Loader、ilvideo などの名前の文字列を持つかを検索した**（`qfc_strings.py` で `blend2d|iltools|ilsvg|ildsp|oggio|wavpack|lameenc|speaker_x64|beat_tracker|webview2|ilvideo|stemextract|declipper|ds2wav|ss2wav|…`）。出たのは `vst3dummy`、`TVST3Dummy` などの無関係な文字列で、**これらの DLL 名は無い**。`FLEngine_x64.dll`（0x522634）の名前の文字列は 1 つある（`f_522470`。`Software\Image-Line\Shared\Paths` のレジストリから FLEngine のパスを探す関数。読み）。
- **QFC は `dsp_ippv2.dll` と `freetype.dll` と `Elastique.dll` と `Python312` を自分で読み込む。うち `dsp_ippv2_x64.dll` と `freetype_x64.dll` は同じインストーラの `Shared` にある**（`nsis-list-all.txt` で `freetype_x64.dll`（56490）、`elastique_x64.dll`（56486）、`dsp_ippv2_x64.dll`（56484）を確認）。QFC を実在モジュールにすると、`dsp_ippv2_x64.dll`（決定 0073、0082 の実在モジュール）に対する QFC 自身の `LoadLibraryW` は、**dsp_ippv2 が既にロード済みなら参照が 1 増える**（`fle_realmod.h`）。`freetype` と `Elastique` は現行の runtime には名前が無く、`fle_load.c` が名前付き停止にする（読み）。

### 6.2 DllMain（エントリ 0x59c1f0）と、QFC の初期化表（読み）

- エントリ `f_59c1f0`（`push rbp; sub rsp,0x180`）: `f_40fa30(&buf)` を呼び、戻り値が 0 のとき `f_419dc0(&buf, 0x59c280, rcx（hinstDLL）, edx（reason）, r8（lpReserved）)` を呼び、続いて `f_40fb80` を呼ぶ。戻り値は `f_40fa30` の結果で `cmp eax,1; setle` を作る形（**`eax` が 0 か 1 のとき真になる**。意味は未確認）。`f_40fa30` は `lea rcx,[rcx+0x38]; jmp f_40f950` で、**レジスタを保存する関数（setjmp に似た形）**に飛ぶ。
- `f_419dc0` は `reason == 1`（ATTACH）のとき `[0x5d2290]=1`、`[0x5d22d8]=1`、`[0x5d2298]=hinstDLL` などを書いてから `f_419640` を呼び、続いて `f_40f430` に引き継ぐ。`f_40f430` は `reason` に応じて `[rbx+rsi*8]` の関数を呼び、`reason == 1` のとき `f_40f390` を呼ぶ。`f_40f390` が **`[0x5d0c90]` のポインタ表（ヘッダ `{count, units_ptr}`）を順に走査し、`proc != 0` かつ `[proc] != 0` のものを呼ぶ**（FLEngine の `f_4120c0`（InitUnits）と同じ形。`inittable.rs` の説明と一致）。
- ヘッダ 0x59c280: UnitCount = **165**、UnitsPtr = 0x59c2b0（`fe-pe pe-init-table` が FLEngine のために作ったリーダーが QFC にもそのまま通った。`header_from_entry 0x59c280 (lea rdx at 0x59c228)`）。実測の内訳: **runnable 154**（distinct proc 154）、empty 10、noproc 1、unreadable 0（`data/qfc-units.tsv`）。**runnable の proc 154 個、二つ目の関数（finalizer）154 個、エントリ、export 9 個は、すべて `.pdata` の関数開始**（`py` で確認）。
- QFC の unit の例（静的。`qfc_rev.py`）: unit 1 `f_40c790`（`cpuid`）、unit 16 `f_43c900`（`MessageBoxW` の使い手 `f_43b1d0` に lea 経由で届く）、unit 37 `f_45b4c0`（機能マスク）、unit 72 `f_51bf20`（`0x51b3f0` の AVX 判定）、**unit 88 `f_5486e0`（`dsp_ippv2.dll`）**、unit 92 `f_548e00`（Windows の API の動的取得）、**unit 129 `f_563840`（`freetype.dll`）**、unit 146 `f_5841c0`（`f_57e600` の SSSE3 の選択）、unit 162 `f_5992d0`（`delphi32.exe`、`bds.exe` の文字列を持つ。`GetModuleFileNameW`）。
- **これらの unit が ATTACH で実際に実行されるか**: `f_40f390` の走査は `reason == 1` のとき呼ぶ形で、静的には ATTACH で全 unit を走査する（読み）。ただし QFC を DllMain まで実行した記録は無く（**実行していない**）、`f_40f430` の途中（`[rbx+rsi*8]`、`[rbp+0x160]` の呼び出し）の意味は読み切っていない。**unit 88 と 129 が ATTACH の途中で `dsp_ippv2.dll`、`freetype.dll` を `LoadLibraryW` する**と読めるが、確認していない。FLEngine の場合と違って QFC の DllMain の前後に `LoadLibraryW` が出るため、**QFC を実在モジュールとして `LoadLibraryW` したときの `fle_realmod_load`（DllMain を呼んでから戻る）の途中で、さらに `LoadLibraryW`（dsp_ippv2、freetype、dwmapi 他）が入れ子になる**（読み。入れ子の扱いは `fle_realmod.h` の「DllMain の中の LoadLibraryW は…」の記述（`fle_realmod_load` が DllMain を `LoadLibraryW` の戻る前に実行する）に依存するが、入れ子の LoadLibraryW を許すかは `stage1-two-module.md`、決定 0082 を確認していない）。

### 6.3 CPUID、スレッド、COM、GDI などの API（import 名と呼び出し点から）

| 項目 | 結果 | 区分 |
|---|---|---|
| CPUID | `cpuid` 5 か所、`xgetbv` 1 か所（§4.4）。**DllMain の ATTACH から静的に届く**（unit 1、37、72、146）。現行の応答（葉 1 の ECX = 0）のとき `xgetbv` は実行されない見込み | 実測（数）と読み |
| スレッド生成 | **`CreateThread` を import する**（行なし）。呼び出し点は `f_40fe20`（0x40fe96）の 1 か所で、`AcquireSharedThreadBank`、`AcquireSharedAnimTimer` の下（`qfc_rev.py`）。`ExitThread`、`ResumeThread`、`SuspendThread`、`CreateSemaphoreW`、`ReleaseSemaphore`、`GetExitCodeThread`、`SetThreadPriority`、`GetThreadPriority`、`SwitchToThread` も import する。**DllMain の ATTACH から静的には届かない**（`f_40fe20` の逆到達は export の 2 つだけで、unit も DllMain も無い）。決定 0080 D9（スレッドは現行のまま）との整合は、この 2 つの export が呼ばれたとき | 実測（import）と読み |
| COM | `CoInitialize`、`CoUninitialize`（ole32）、oleaut32 の 16 名（`SafeArray*`、`Variant*`、`Sys*String*`、`GetErrorInfo`）。`CoInitialize` の呼び出しは `f_51dc80`（0x51dd05）の 1 か所。到達する QFC の unit は（lea 込みで）77。`CoCreateInstance` などは import に無い。ウィンドウ名「`vst3dummy`」「`TVST3Dummy`」の文字列がある | 実測と読み |
| ウィンドウ・メッセージ | user32 の `CreateWindowExW`、`RegisterClassW`、`UnregisterClassW`、`DefWindowProcW`、`DestroyWindow`、`PeekMessageW`、`PostMessageW`、`PostQuitMessage`、`MsgWaitForMultipleObjects`、`GetClassInfoW`、`LoadIconW`、`SystemParametersInfoW`、`GetSystemMetrics`、`EnumDisplaySettingsW`、`MessageBoxW` ほか 27 名。呼び出し点は `f_41e130`（`CreateWindowExW` 0x41e1a5）、`f_585950`（`RegisterClassW` 0x5859c0）で、**どちらも `AcquireSharedAnimTimer` の下**（静的な unit なし）。`f_585e20` が `timeBeginPeriod`（winmm） | 実測と読み |
| GDI | gdi32 の 59 名（`SelectObject`、`BitBlt`、`StretchBlt`、`CreateDIBSection`、`CreateFontIndirectW`、`GetTextMetricsW`、`ExtTextOutW`、`Polygon`、`Ellipse` など。パレット系も多い）。**行ありは `CreateFontIndirectW`、`CreatePalette`、`DeleteObject`、`GetDeviceCaps`、`GetObjectW`、`GetStockObject` の 6 名**。QFC の unit 71 から `SelectObject`、`GetTextMetricsW`、`GetSystemPaletteEntries`、`GetSysColor` に届く（静的） | 実測と読み |
| DirectWrite / GDI+ / D2D | **無い**（`DirectWrite`、`GDI+` の文字列なし。import に `d2d1`、`dwrite` なし）。フォントの実体は FreeType（`freetype.dll`。`FT_*` 14 名）を `LoadLibraryW` して使う形で、QFC のフォント名の文字列 `FreeType`（ASCII 28 か所、UTF-16 2 か所）がある | 実測（文字列）と読み |
| ファイルの読み書き | `CreateFileW`、`ReadFile`、`WriteFile`、`FindFirstFileW`、`GetFileAttributesW` など（行あり）。QFC の `ReadFile` 系は FreeType のフォントファイル読み込みに使われる読み（未確認） | 読み |
| レジストリ | `RegOpenKeyExW`、`RegQueryValueExW`、`RegCloseKey`、`RegFlushKey`（advapi32。いずれも行あり）。`Software\Image-Line\Shared\Paths`（`f_522470`。FLEngine と同じキー）と、Delphi の RTL の `Locales` | 読み |
| 例外 | `RaiseException` 7 か所、`RtlUnwind`（行なし）、`RtlUnwindEx`（行あり、名前付き停止）、`UnhandledExceptionFilter`（同）。`.pdata` は 6,347 エントリ（SEH の巻き戻し情報を持つ x64） | 実測 |
| 出力 | `OutputDebugStringW` が **424 か所**（IAT thunk への呼び出し。`f_41d970`）。FLEngine は 68 点の LoadLibrary 系のほかに多数持つ（未集計） | 実測 |

## 7. 比較表（項目 7）

dsp_ippv2 は `dsp-ippv2-lift-feasibility.md`、`two-module-*.md`、`dll-nonpdata-reloc-seeds.md`、`blend2d-lift-feasibility.md` §3.2・§10.1 にある数（dsp_ippv2 は決定 0075〜0079 の後の値で、Blend2D の調査が同じ作業ツリーで測り直したもの。functions 32,648 など）を引いた（再測定していない。**他者の実測**）。Blend2D は HEAD（`47118ac`）に入っている調査文書の数。QFC は今回の実測。

| 項目 | quickfontcache_x64.dll | Blend2D.dll | dsp_ippv2_x64.dll | 区分 |
|---|---|---|---|---|
| ファイルのサイズ | 2,067,968 B | 2,065,920 B | 39,942,656 B | 実測 |
| ImageBase / SizeOfImage | **0x400000（FLEngine と同じ）** / 0x20e000（2.05 MiB） | 0x180000000 / 0x300000（3.0 MiB） | 0x180000000 / 0x261f000（38.1 MiB） | 実測 |
| 実行セクション | `.text` 1 つ | `.text` 1 つ | `.text` + `IPPCODE` | 実測 |
| export 数 | 10 | 1 | 446 | 実測 |
| import（normal） | 217 名・8 DLL | 94 名・kernel32 のみ | 72 名・kernel32 のみ | 実測 |
| 遅延 import | **あり**（5 記述子・6 名） | なし | なし（動的に `GetProcAddress`） | 実測 |
| 行なしの import 数（normal） | **87**（gdi32 53、kernel32 20、user32 13、shell32 1） | 17 | 2 | 実測 |
| 行なしの遅延 import 数 | 4 | — | — | 実測 |
| 名前付き停止の import（行あり。必ず止まる） | 3（`IsDebuggerPresent`、`RtlUnwindEx`、`UnhandledExceptionFilter`） | 19（実質 24） | 19 | 実測（テキスト判定） |
| `call [IAT]` の形 | **0**（`jmp [IAT]` のスタブ形のみ） | 264（`import_calls`） | 138 | 実測 |
| functions | **6,712** | 2,018 | 32,648 | 実測 |
| closure_instructions | **209,259** | 398,879 | 6,464,799 | 実測 |
| bad_nodes | 0 | 0 | 0 | 実測 |
| 生成 C | 17.7 MB | 36.4 MB | 566.3 MB（リベース後 536.7 MB） | 実測 |
| unsupported_total（割合） | **104**（0.05%） | 662（0.17%） | 974,527（15.0%） | 実測 |
| 未対応の内訳 | `jmp [IAT]` 88、SSSE3 14、`jmp` 未解決 2。VEX/EVEX/x87 0 | VEX 495、`call [IAT]` 79、`jmp [IAT]` 5、`Jmp` 未解決 12、`gs:` 3、SSE4 等 21 ほか | `V*` 885,938、レガシー 73,713、EVEX 46,975、`K*` 4,537 ほか | 実測 |
| VEX / EVEX 命令 | VEX 18（`vmovups` 系 4 種のみ）/ 0 | 497 / 0 | 835,902 / 219,112 | 実測 |
| CPU 振り分け | **スタブ 0 個**。変数を見て SSSE3 版を選ぶ関数が 1 つ | 表引き 0。変数を見る分岐 5 関数 + AVX2 の 3 関数 | **スタブ 434 個**（決定 0079） | 実測と読み |
| `cpuid` / `xgetbv` | 5 / 1 | 12 / 3 | 259 / 7 | 実測 |
| `jumptabs`（決定 0079 の switch） | 0 | 0 | 434 | 実測 |
| 閉包の `jmp reg`（`register_jmp_sites`） | 5 | 99 | 366 | 実測 |
| 非 pdata の reloc 値（種にしたとき） | 9,293。**全部を種にすると bad_nodes 17,541**。葉関数らしい 155 個でも 48 | 58（種にすると 2,066 関数、bad_nodes 0） | 2,750（(ii) 1,544 を種にすると bad_nodes 0） | 実測 |
| TLS ディレクトリ | **なし** | あり（暗黙の TLS） | なし | 実測 |
| スレッド生成 API | **あり**（`CreateThread` 1 か所） | あり | なし | 実測 |
| JIT（実行可能メモリへの書き込み） | 不明。`VirtualAlloc` の thunk 経由の呼び出しが 5 か所（`f_407450`、`f_407540`、`f_407690`、`f_4085b0` の 4 関数。Delphi の RTL のメモリ管理に見える）。**`flProtect` の値は読んでいない**（`PAGE_EXECUTE_*` の要求の有無は未確認） | あり（AsmJit） | なし | 実測（数）と未確認 |
| wasm の locals 50,000 超 | **未測定**。最大関数 13,559 行（`f_522e50`）。見積もりは散る（§4.5） | 4 関数（最大 102,086） | 13〜15 関数（最大 389,948） | 実測（行数）、locals は他者の実測 |
| 最大の関数（生成 C の行数） | 13,559 | 11,136 | 25,744 | 実測 |
| 1,000 行以上の関数 | 4（FLEngine は 126） | 未集計（`big-funcs.txt` の上位 10 は 2,821〜11,136 行） | 未集計 | 実測（QFC、FLEngine）。他は未集計 |
| 他の同梱 DLL への依存 | **dsp_ippv2、freetype、Elastique、Python312**（自分で `LoadLibraryW`） | なし（kernel32 のみ。`api-ms-win-core-synch` は `GetModuleHandleA/W`） | なし（`api-ms-win-*` を引く） | 読み |
| FLEngine が引く名前 | 8 名（すべて export に実在） | 1 名（`AggCreateInstance`） | 420 名 | 実測と読み |
| 使う側の NULL 許容 | ロード点は許容。使う側 4 名は呼ぶ前の検査なし | ロード点は許容。使う側 `f_6cb0b0` は検査なし | 戻り値を使わない。使う側は検査なし | 読み |
| 起動の範囲での使用 | `LoadLibraryW` は FLEngine の unit 158。`FontCache_AcquireFont` の使い手は unit 245 以降 | unit 149、190（ロード）。使用は 614 以降 | unit 144（ロード）。IPP の関数は後 | 実測（ロード）と読み |
| 2 モジュール機構との関係 | **3 つ目**。基底の重なりあり（0x400000）で、リベースが要る | 3 つ目 | 2 つ目（決定済み） | 実測 |

行なしの import の数は、3 つの DLL を同じ `imports_table.rs`（260 行）、同じ大文字小文字の規則で数え直した（`py/qfc_imports.py` を 3 つに実行。dsp_ippv2 は行あり 70・行なし 2、Blend2D は行あり 77・行なし 17。Blend2D の調査が書く 17 と一致）。

## 8. 決定が要る点（選択肢と事実。**決めない**）

以下は事実の整理で、推奨は §9 の「意見」にだけ書く。

### Q1. quickfontcache_x64.dll を、どのように扱うか

| 選択肢 | 内容 | 事実 |
|---|---|---|
| (a) 実在モジュールとして lift する（M2。決定 0073、0082 の形） | 同じ fe-pe → fe-decode → fe-lift で C にし、`fle_realmod_table` に名前（`quickfontcache_x64.dll`）と基底を登録する | **ツールは変更なしで通る**（§4。bad_nodes 0、0.18 s）。必要な追加: リベース（§3.6）、3 つ目のモジュールの組み込み（Q2）、行なしの import の応答（Q3）、DllMain を通す（Q4） |
| (b) 仮想モジュール（`fle_modules[]` の末尾 N=31）にして「ある」と答える | `LoadLibraryW` は非 NULL、`GetProcAddress` は応答表に行が無ければ NULL と 127（決定 0015、0081 の形） | 決定 0073 は dsp_ippv2 についてこれを**却下**した（「実機の DLL の有無について偽の主張」「420 関数の意味を runtime が創作する」）。QFC は 8 名だけを引く点が違うが、**8 名の関数は FLEngine が実際に呼ぶ実体（フォントの取得、スレッドバンク、アニメ用タイマー、VST3 など）なので、応答表の行を作るなら 8 つの関数の意味を runtime が創作することになる**（読み）。応答表に行が無いままだと、`GetProcAddress` は NULL で、§5.2 の 4 名が 0 番地を呼ぶ形になる |
| (c) `fle_load_absent[]` に足す（NULL と 126） | 「無い」と答える | 決定 0073 が dsp_ippv2 について採らなかった理由と同じ形（実機にある DLL を「無い」とする主張）。**ゲストの経路は §5.2 のとおり、NULL で 0 番地呼び出しに進む 4 名がある**ので、「ゲストが NULL を許容する」（決定 0055 の (a)）も満たさない。`registry-unit153.md` §6.3 の 3〜4 の実測（`MessageBoxW` の後で `GetProcAddress(NULL, …)` の abort）とも一致 |
| (d) 停止のまま | 今の挙動（`LoadLibraryW` で名前付き停止） | 測定としては正しいが、FLEngine の unit 158 より先（unit 159 以降）の測定ができない（`registry-unit153.md` §7 の 5） |

### Q2. 基底の値と 3 つ目のモジュールの置き方（(a) の場合）

- QFC の ImageBase 0x400000 は FLEngine と同じ範囲で、リベースが要る（§3.6。生成 C の関数名 374 個が衝突する）。候補は §3.6 の表（0x1cc0000、0x1d00000、0x1df0000、0x4700000、0x8000000 など）。**決定 0080 D1 は 1 つ目の実在モジュールの基底を 0x2000000 に決めた**が、3 つ目の置き場は決めていない。FLEngine と dsp_ippv2 の隙間（0x340000 B）に QFC は入るが（0x132000 B 余る）、**Blend2D（0x300000）は入らない**。3 つ（dsp_ippv2、QFC、Blend2D）を同時に置く場合の割り付けは、Blend2D の調査の B7 と共通の論点。
- ビルドの仕組み: `lift/harness/two/gen_two.sh` は `REAL=` を **1 つだけ**取る（実在モジュールは 1 つ）。runtime の `fle_realmod.c` は `RM_MAX = 4`（最大 4 モジュール）で、3 つ目を足す作りはあるが、ビルドスクリプトは 2 モジュール目を 1 つしか扱わない（Blend2D の調査 §8 と同じ。`merge_tables.py` による `R[16]`、`FL`、`X[16]`、`fle_dispatch_table` の重複の解消を**さらに 1 つ足して通るかは未確認**）。QFC は dsp_ippv2 との**組**で必要になる（QFC が `dsp_ippv2.dll` を自分で読むため。§6.1）。
- `fle_mem_set_image` は 1 範囲（`fle_host.c` 13〜19 行。HEAD でも同じ）。`FLE_CHECKED_MEM` を使うなら 3 範囲化が要る。
- 決定 0080 D5（モジュールハンドルの値は基底）、D6（`GetModuleFileNameW` の仮想パス。**保留**）、決定 0082 D11（`FreeLibrary` で参照が 0 になるときは名前付き停止）。QFC には `FreeLibrary` が 25 か所あり、FLEngine 側にも QFC の終了側 `f_665d10` が `FreeLibrary` する経路がある（`f_66d120` → `f_665d10`。静的な unit なし）。

### Q3. 行なしの import をどうするか（87 名 + 遅延 4 名）

- FLEngine も行なしの 83 名は、FLEngine が先に（または同時に）直す問題で、**QFC にだけ増えるのは 4 名**（`CreateSemaphoreW`、`ReleaseSemaphore`、`InterlockedPopEntrySList`、`InterlockedPushEntrySList`）。
- **DllMain の ATTACH から静的に届く**もの（`logs/graph-units.txt`。QFC の全 unit のうち runnable の proc 154 個とエントリを根にした直接 call の閉包 802 関数。lea 込みは 1,311 関数）: 行なしは直接で 6 名（`GetSystemPaletteEntries`（unit 71）、`GetTextMetricsW`（71）、`SelectObject`（71）、`EnumCalendarInfoW`（18）、`FormatMessageW`（20）、`SHGetFolderPathW`（146））、lea 込みで 10 名（上に `GetDiskFreeSpaceW`（18）、`VirtualQueryEx`（56）、`GetSysColor`（71）、`MessageBoxW`（16）を足したもの）。**DllMain の経路（dsp_ippv2 の `call [IAT]` のように、すべての行なしが即座に止まる）ではなく、unit ごとの個別の名前**。
- 閉包 6,712 関数のうち、export の使い手が届く行なしは `CreateThread`、`ExitThread`、`ResumeThread`、`CreateSemaphoreW`、`ReleaseSemaphore`（`AcquireSharedThreadBank`）、`EnumDisplaySettingsW`、`PeekMessageW`、`PostMessageW`、`PostQuitMessage`、`UnregisterClassW`（`AcquireSharedAnimTimer`）など（`logs/graph-roots.txt`）。
- 仕様の確認は 1 名ごとに要る（`loadlibrary-names-classification.md` の流儀）。応答の値を決めるには、GDI のオブジェクト、ウィンドウ、スレッド、セマフォなど、`fle_gdi.c`、`fle_win.c`、`fle_event.c` に既に近いものがあるかを個別に調べる必要がある（今回は調べていない）。

### Q4. QFC の DllMain を通すために要るもの（(a) の場合。読みで、実行していない）

1. **DllMain の中の `LoadLibraryW` の入れ子**: QFC の DllMain が `dsp_ippv2.dll`（unit 88）、`freetype.dll`（unit 129）、`dwmapi`、`avrt`、`PowrProf.dll`、`SHCore.dll`、`advapi32`、`user32`（unit 92）を `LoadLibraryW` する。現行の runtime の答え: `dsp_ippv2_x64.dll` は実在モジュールにすれば応答（ロード済みなら参照 +1）、`avrt` と `PowrProf.dll` は**今は名前付き停止**（`loadlibrary-names-classification.md` §6.1）、`dwmapi` と `SHCore.dll` は absent（NULL と 126）、`advapi32`、`user32` は仮想、**`freetype` は名前が無く名前付き停止**。つまり QFC の ATTACH は FLEngine の unit 145 と同じ `avrt`、`PowrProf.dll` の停止に再び当たる（読み。QFC の `f_548e00` は FLEngine の `f_65c9f0` と同じ形）。
2. **`GetModuleFileNameW`**: 決定 0082 の D6 で、実在モジュールのハンドルに対する `GetModuleFileNameW` は名前付き停止（値を決めていない）。QFC では `f_5222b0`、`f_522390`（dsp_ippv2 と freetype のローダーの下）、`f_409ba0`、`f_415320`、`f_416130`、`f_416b70` が呼ぶ（hModule 引数が `hinstDLL` かは `f_522390` だけ `[0x5d2298]` として確認、他は未確認）。
3. **ロケール**: `f_416b70` などが `GetLocaleInfoW`、`CompareStringW`、`LCMapStringW`、`MultiByteToWideChar`、`WideCharToMultiByte`、`GetACP` を使う Delphi の RTL の初期化（`Locales` のレジストリ読み。FLEngine の unit 12、17 と同じ形）。決定 0083、0084 の範囲。QFC で届く `GetStringTypeW` は import に無い（FLEngine の次の停止点 `GetStringTypeW` は QFC では当たらない見込み。読み）。
4. **`gs:[0x58]`**: 1 か所（`f_4195e0`）で、決定 0032、0076 の範囲。QFC の `[0x5d22d8]`（`f_419dc0` が ATTACH で 1 にする）が真のとき通らない枝に読める。
5. **CPUID**: §4.4。現行のままなら `xgetbv` に届かない見込み。SSSE3 の 2 関数（`fle_unsupported` を含む）が選ばれると停止する（`f_57e600` の条件は未確認）。
6. **`jmp` 未解決 2 か所**: `f_40c120`（`jmp [r10+rdx*8+0x40]` の表）、`f_40f9c0`（`jmp [rcx]`。setjmp/longjmp に似た復帰）。`fle_dispatch_miss` で止まる。例外の巻き戻しで通る経路に読める（未確認）。
7. **wasm の locals 上限**: §4.5。QFC の ATTACH が呼ぶ `f_522e50`（13,559 行）が上限を超える可能性があり、**測定が要る**（`emcc -O0` の `.o` が 1 TU でできる規模。Blend2D の調査は 17.9 s、3.3 GB で通した）。
8. **メモリ**: 生成 C 17.7 MB は FLEngine（215.7 MB）の 8%、wasm にして約 10 MB の級（推測。同じ比 0.537 を当てた。実測していない）。SizeOfImage 2.05 MiB。決定 0080 D10 の使用量の測定は、2 モジュール構成が動いた後の話。

### Q5. QFC が `dsp_ippv2.dll` を自分でも読む点

- `dsp_ippv2_x64.dll` は FLEngine が unit 144 で読み（決定 0073、0082）、QFC も DllMain の初期化（unit 88）で読む（読み）。**同じ 420 名を QFC も引く**（`f_522e50` の名前集合は FLEngine の `ipp_names.txt` と等しい。実測）。ロードの順序は、FLEngine の unit 144（dsp_ippv2）→ … → unit 158（QFC）で、QFC の ATTACH のとき dsp_ippv2 は**既にロード済み**になる（読み。FLEngine の unit 144 が実行されたあとなら）。QFC の `LoadLibraryW("dsp_ippv2.dll")` はそのとき参照 +1 で戻る。
- 逆に、QFC を先に（FLEngine の unit 158 が unit 144 より前に）ロードすることは無い（unit の順序は 144 < 158）。
- QFC は IPP の関数を引くだけで、呼び出し点（`ippMalloc` などの使い手）は 420 名ぶんのスロットを使う関数が別にある（調べていない）。dsp_ippv2 の `ippMalloc` などを QFC が使う場合、dsp_ippv2 の CPU 振り分け（選択値 0）の仮説（`dsp-ippv2-lift-feasibility.md` §5.4）と同じ前提になる。

### Q6. NULL（`QuickFontCache` が無い）にする案を取る場合（(c) または (d)）

- ゲスト自身が `MessageBoxW` を出し、そのまま `GetProcAddress(NULL, …)` に進む。`fle_modules.c` は hModule が仮想モジュールのハンドルでなければ abort する（`registry-unit153.md` §6.1 の実測。`modec2`、`moded2` の停止 `unsupported instruction at 0x425180`、`fin-*` の停止 `GetProcAddress with hModule 0 …`）。つまり**停止が移るだけで先に進まない**。Learn の `GetProcAddress` は hModule が NULL のときの戻りを書かない（一次に無い）。

## 9. 意見（別枠。決めない。PM の判断の材料）

この節は作業者の意見で、事実の節ではない。

1. **QFC は dsp_ippv2、Blend2D より lift の規模が小さい**（関数 6,712、unsupported 104、VEX 18）ので、**ツールの面では (a) の障害は小さい**。障害は規模ではなく、(i) 基底が FLEngine と同じ 0x400000（リベースが必須）、(ii) 3 つ目のモジュールをビルドスクリプトが扱えない、(iii) QFC の DllMain が `dsp_ippv2`、`freetype`、`avrt`、`PowrProf.dll` などを読み込む（入れ子の LoadLibraryW）こと、(iv) wasm の locals 上限が未測定（`f_522e50`）、にあると考える。
2. **(b) 仮想モジュールと (c) absent は、決定 0073 の理由が当てはまり、勧めない**。特に QFC は FLEngine が呼ぶ 8 つの関数の実体（フォント、スレッド、タイマー、VST3、Python）を持つので、応答表の行を作ると 8 つの関数の意味の創作になる。(c) は §5.2 のとおりゲストが NULL で 0 番地を呼ぶ 4 名があり、「NULL 許容」の根拠も立たない。
3. **QFC は dsp_ippv2 に依存する**（§6.1、Q5）ので、**(a) を採るなら dsp_ippv2 の実在モジュール化（決定 0073、0082。実装は W6 後）が先**になる。QFC だけを先に作る順序は、QFC の ATTACH が `dsp_ippv2.dll` を引くところで、dsp_ippv2 が実在でなければ止まる。
4. 非 pdata の reloc 値は**種にしない**のが QFC には安全（§4.2。全件 17,541、葉らしい 155 個でも 48 の `bad_nodes`）。`--pdata` だけで起動の関数は足りている。ただし vtable 経由で `.pdata` に無い関数に入る可能性は未確認。
5. 次の測定の候補（着手は PM の判断）: (1) `f_522e50` の `emcc -O0` による locals（`-c` コンパイルのみ。Blend2D の調査と同じ方法）。(2) `fe-pe rebase` で 0x1d00000 に置いた QFC と、dsp_ippv2（0x2000000）、FLEngine の 3 つをリンクして DllMain まで進める試作（決定 0080 の段階 2・3 の流儀。`gen_two.sh` が 1 つしか取らない点の確認を含む）。(3) 行なしの import のうち ATTACH で届く 10 名の仕様確認。
6. **この調査は QFC の ATTACH を実行していない**ので、§6.2 の「unit 88、129 が ATTACH で `LoadLibraryW` する」は読みにとどまる。結論を出す前に、リベース済みの QFC を FLEngine と同じハーネスで DllMain まで走らせる測定が要る（stage23 の手順）。

## 10. 未確認

- QFC の DllMain を実行して、unit 88、92、129 が ATTACH で実際に走るか、どこで止まるか。`f_40f430`（`[rbx+rsi*8]`、`[rbp+0x160]` の呼び出し）の意味。`f_40fa30` の戻り値の意味。
- wasm の locals（`f_522e50` を含む）。emcc を使わなかった。
- `f_57e600` の SSSE3 の選択条件（`[[0x5cb508]+0x12]` の値の出どころ）と、現行の CPUID 応答での結果。
- `AcquireSharedThreadBank`、`FontCache_GetStats`、`CreateVST3Instance` の使い手（`f_93f160`、`f_668980`、`f_b47620`）が実行されるか（vtable、コールバックの経路）。
- FLEngine が序数で `GetProcAddress` を呼ぶか。QFC の export の序数（1〜10）の使い方。
- `msimg32.dll`、`dwmapi.dll`（遅延 import）が NULL になったときの QFC の挙動（`RaiseException` 系か）。
- QFC の `VirtualAlloc` の呼び出し 5 か所（`f_407450`、`f_407540`、`f_407690`、`f_4085b0` の 4 関数）の `flProtect`（`PAGE_EXECUTE_*` の要求の有無）。JIT が無いことの確認。
- 実機で `QuickFontCache_x64.dll` が `LoadLibraryW` で読み込まれる経路（`f_6539e0` の 4 つのディレクトリ候補。`f_6532f0` の `Shared` ディレクトリ）。インストーラにあることは、ゲストが実行時にそのファイルを見つけることを意味しない（`loadlibrary-names-classification.md` §5.1 末尾の注意と同じ）。
- 32 ビット版 `QuickFontCache.dll`（56470）の用途（x64 の FL Studio が使うかは、`dl=1` で `_x64` が付くので使わない読みだが、確認していない）。
- QFC の SHA-256 の同一性は、このインストーラの 1 版（26.1.7.5653）の抽出物だけ（他の版は見ていない）。

## 11. 作ったファイルと手順の一覧

すべて `lift/out-qfc/`（`.gitignore` の `lift/out*/`）。`lift/` の既存ファイル、`docs/` の既存ファイルは変更していない（`git status` の他者の変更は §12）。

- 抽出した DLL（**1 ファイルだけ**）: `quickfontcache_x64.dll`（2,067,968 B、SHA-256 `afc5a0ed…36d2`）。測定用のコピー（**実在の DLL ではない**）: `rebased/qfc.0x1d00000.dll`、`qfc.0x4700000.dll`、`qfc.0x8000000.dll`（`fe-pe rebase` の出力）。
- インストーラ: `nsis-list-quickfont.txt`、`nsis-list-all.txt`、`hash-x64.txt`、`hash-x86.txt`、`nsis-ctx-qfc.txt`、`nsis-ctx-qfc-wide.txt`、`write.err`。
- PE: `pe-info.txt`、`pe-pdata-info.txt`、`pe-pefile.txt`、`objdump-p.txt`、`exports.txt`、`imports-vs-table.txt`、`impclass.txt`。
- lift: `build.log`、`logs/A-pdata.txt`（`.time`）、`B-…`、`C-…`、`D-…`、`H-noseeds.txt`、`I-pdata-noadj.txt`、`Y1-…`、`closure-*.txt`、`rebased-*.txt`、`unsup-sites.txt`、`cfuncs.txt`、`cfuncs-flengine.txt`、`feat-*.txt`、`leafcand.txt`、`nonpdata*.txt`、`logs/f_522e50.c`（1 関数の C。1.1 MB）。生成 C 全体は保存していない。
- 到達、依存: `callsites.txt`、`cpuid-funcs.txt`、`dllmain-*.txt`、`ll-sites-1.txt`、`logs/graph-*.txt`、`logs/rev-*.txt`、`logs/loadsites.txt`、`logs/strings-dll.txt`、`data/qfc-units.tsv`、`data/reloc-*.txt`、`data/leaf-*.txt`、`data/qfc-export-names.txt`、`data/*-funcnames.txt`。
- FLEngine 側（読み取り・逆アセンブルのみ）: `fle/`（`f_665bd0.txt`、`f_66d050.txt`、`f_66cfa0.txt`、`f_668d30.txt`、`refs-*.txt`、`reach-*.txt`、`xrefs-export-names.txt`、`index.tsv` ほか）。
- スクリプト（`py/`。すべて読み取り専用。`PYTHONDONTWRITEBYTECODE=1`）: `qfc_peinfo.py`、`qfc_imports.py`、`qfc_callsites.py`、`qfc_loadsites.py`、`qfc_dump.py`、`qfc_graph.py`、`qfc_graph_union.py`、`qfc_rev.py`、`qfc_cfuncs.py`、`qfc_feat.py`、`qfc_mix.py`、`qfc_nonpdata.py`、`qfc_leafcand.py`、`qfc_strings.py`、`fle_dump.py`、`fle_xrefs.py`、`fle_refs.py`、`fle_reach.py`、`fle_index.py`。再利用（読み取りのみ）: `lift/out-loadlib/llpe.py`、`units.tsv`、`edges-direct.tsv`、`edges-ptr.tsv`、`lift/out-blend2d/b2d_impclass.py`、`b2d_nsis_ctx.py`、`lift/out-ext/ipp_names.txt`、`lift/out-design/tools/wasm_locals.py`、`lift/out-blend2d/blend2d-pdata.c`（Blend2D の生成 C。`qfc_feat.py` の比較のために読んだ）。

## 12. 作業ツリーの確認（規則への適合）

完了時（HEAD `27c8068`）の `git status --short` と `git diff --stat`:

```
 M docs/README.md
 M docs/decisions/0018-virtual-fs-reg-env.md
 M docs/investigations/provisional-decisions.md
 M lift/crates/fe-lift/tests/reg.rs
 M lift/runtime/fle_reg.c
?? docs/decisions/0085-hklm-software-initial-key.md
?? docs/investigations/quickfontcache-lift-feasibility.md
?? lift/harness/wasm/trace.c.new
?? lift/harness/wasm/trace_native.c.new
?? lift/iat-unsup-closure.txt
```

```
 docs/README.md                               |   4 +-
 docs/decisions/0018-virtual-fs-reg-env.md    |   4 +-
 docs/investigations/provisional-decisions.md |   3 +
 lift/crates/fe-lift/tests/reg.rs             | 216 +++++++++++++++++++++++++++
 lift/runtime/fle_reg.c                       |  27 +++-
 5 files changed, 249 insertions(+), 5 deletions(-)
```

- **この作業が追跡ファイルを変更したものは 0 件**。新規は `docs/investigations/quickfontcache-lift-feasibility.md`（この文書。`??`）だけで、`lift/out-qfc/` は `.gitignore` の `lift/out*/` で `git status` に出ない。
- 上の `M` の 5 件と `??` の `docs/decisions/0085-hklm-software-initial-key.md` は**並行して走っている別の作業**（決定 0085。`HKLM\Software` の初期キー。`registry-unit153.md` の続き）のもので、この作業のものではない。`lift/harness/wasm/trace*.c.new` と `lift/iat-unsup-closure.txt` は作業開始時から未追跡で残っていたもの（最初の `git status` に出ていた）。
- 作業中に HEAD は `f8b5e6d` から `27c8068` に進んだ（他者のコミット `47118ac`（Blend2D の調査）、`d43db3e`（決定 0079 の訂正）、`27c8068`（決定 0084））。`lift/crates/*/src` と `imports_table.rs` は `f8b5e6d` から HEAD と作業ツリーまで**変更なし**（`git diff` が空）なので、測定したバイナリの前提は変わっていない。runtime は決定 0084 などで変わったが、この文書の runtime の記述（`fle_modules.c` の 30 名、`fle_load_absent[]` の 9 名、`fle_host.c` の 1 範囲）は文書作成時の HEAD で読み直して確かめた。
- 規則: `rm` を使っていない。既存ファイルの全体上書き（`>`）は無い（新規ファイルへの `>` と `create_new` の書き出しだけ。`lift/out-qfc/logs/nonpdata.txt` は修正前のスクリプトの出力で、修正後は別名 `nonpdata-strict.txt` に保存した。前者は「不正バイトを検出できない設定」の出力なので結論に使っていない）。git の変更系操作は無い（`rev-parse`、`log`、`status`、`diff` のみ）。`/tmp` は使っていない（`TMPDIR=lift/out-qfc/tmp`）。コマンドに `sleep`、`timeout`、`nohup`、末尾の `&` は無い。emcc と wasm は使っていない。2 GB 級のファイルは作っていない（最大の保存物は `lift/out-qfc/target/`（ビルドの成果物）と `fle/index.tsv`（FLEngine の線形走査の索引 435,383 行）。`lift/out-qfc` の合計は約 52 MB）。python の名前は標準ライブラリと衝突しない（`qfc_*.py`、`fle_*.py`）。`lift/out-cs/` を cwd にしていない。`__pycache__` は作っていない。

