# Blend2D.dll を外部モジュールとして lift できるか（実現性調査）

実行日: 2026-10-24。HEAD 7a9af0f。調査のみ。`lift/` の既存ファイル（src、runtime、tests、harness）は編集していない。決定は一つもしていない。作業物は `lift/out-blend2d/`（§9）。先例は `dsp-ippv2-lift-feasibility.md`、`two-module-design.md`、`stage23-two-module-real-dll.md`、決定 0073、0074、0079、0080〜0083。

## 0. 凡例と範囲

- **実測** = コマンドの出力。`lift/out-blend2d/` の同名ファイルに残してある。
- **読み** = capstone による静的な逆アセンブルの読み。実行して確かめていない。
- **仮説 / 推測** = 実測や読みから言えそうだが確認していないもの。
- **未確認** = 確認できていないもの。
- 入力 DLL は §1 で抽出した `lift/out-blend2d/Blend2D.dll`。ツールは既存の `fe-pe`、`fe-decode`、`fe-lift` を `CARGO_TARGET_DIR=lift/out-blend2d/target` に release ビルドして使った（`lift/crates` は編集していない）。
- 比較に使った dsp_ippv2 の数は、同じ作業ツリーの同じバイナリで測り直した（`dsp-ref/`）。`dsp-ippv2-lift-feasibility.md` の数は決定 0075〜0079 の前のもので、今は違う（functions 32,162 → 32,648 など）。表では測り直した値を使う。
- 到達の調査（§4、§5）は**静的**で、DLL も FLEngine も実行していない。「通る」「届く」は静的な到達の話で、実行で起きるとは言っていない。
- 作業用の補助スクリプトは `b2d_*.py`（読み取り専用。`PYTHONDONTWRITEBYTECODE=1` で実行し、`__pycache__` は作っていない）。前回の調査の `lift/out-loadlib/llpe.py` と `units.tsv`、`edges-direct.tsv`、`edges-ptr.tsv` を読み取りで再利用した（再生成していない）。

## 1. 要約

| 項目 | 結果 | 区分 |
|---|---|---|
| インストーラ内の実体 | `Blend2D.dll` が 1 件（エントリ 56450、2,065,920 B）。同名は他に無い | 実測（§2） |
| 規模 | 関数 2,018、命令 398,879、誤デコード 0、`bad_nodes` 0、生成 C 36,418,770 B。dsp_ippv2 の約 6% | 実測（§3） |
| 未対応命令 | `unsupported_total` 662（生成 C の 0.17%。dsp_ippv2 は約 15%）。VEX は 497 命令（0.12%）で EVEX / ZMM / opmask は 0 | 実測（§3） |
| 未対応の内訳 | V\* 495、`call [IAT]` 79 + `jmp [IAT]` 5、`Jmp` 未解決 12、`gs:` 3、SSE4 など 8 種 | 実測（§3.3） |
| CPU 振り分け | 表引き（434 個のスタブ）は無い。`cpuid` 12 か所、`xgetbv` 3 か所。結果を変数に書き、`cmp [変数],0` で SSE2 版と FMA 版の 5 関数を選ぶ | 実測と読み（§3.5） |
| wasm の locals | 生成 C を `emcc -O0` に通して 4 関数が 50,000 を超える（最大 102,086）。**4 関数は DllMain の ATTACH 集合（エントリ、`.xc`、`.xi`）から直接 call で届く**（`reach3c`）が、**`AggCreateInstance` から直接 call だけでは届かない**（43 関数。vtable を許すと届く）（読み。静的） | 実測と読み（§3.6、§5.1） |
| FLEngine の使い方 | `LoadLibraryW("Blend2D.dll")` → `GetProcAddress("AggCreateInstance")` の 1 本だけ。NULL なら false を返して続行。使う側（`f_6cb0b0` ほか）は起動の unit からは届かず、描画経路（unit 614、745、880、895、898）から届く | 実測と読み（§4） |
| DllMain の ATTACH | 標準的な MSVC の CRT。`.xc` に C++ の静的初期化 1 つ、`.xi` に 4 つ。TLS ディレクトリを持つ（dsp_ippv2 は持たない） | 実測と読み（§6） |
| 現行 runtime の応答 | import 94 のうち行なし 17、名前付き停止 19、応答 58。行なしのうち ATTACH から静的に届くのは 15 名（59 か所） | 実測と読み（§7） |
| 判定の材料 | 規模・未対応命令・locals は dsp_ippv2 より小さい。一方で、JIT、スレッド、暗黙の TLS、`jmp reg` の switch 表、`.pdata` に無い葉関数 58 個は dsp_ippv2 に無かった論点。未決は §10.3 の B1〜B13。**決めない** | 事実の整理（§10） |

## 2. インストーラ内の実体（項目 1）

手順: `python3 tools/extract_nsis2.py <installer> --list blend2d`（読み取り専用）、`--hash Blend2D.dll`、`--write Blend2D.dll lift/out-blend2d/Blend2D.dll`（新規ファイルのみ。他のファイルは展開していない）。出力は `nsis-list-blend2d.txt`、`nsis-list-blend-wide.txt`、`hash-nsis.txt`、`write.err`。

| エントリ | データオフセット | 名前 | 区分 |
|---|---|---|---|
| 56450 | 0x2efc7ec5 | `Blend2D.dll` | 抽出対象 |
| 57742 | 0x34b8ca9a | `blend2d.txt` | テキスト（DLL ではない。抽出していない） |

- EXTRACTFILE は 27,574 件中。`blend2d` を含む名前は上の 2 件だけ。`blend` で広げると `BufferBlender.zgeproj`、`BufferBlender_Version2.zgeproj` が加わる（無関係）。**同名の DLL は 1 件**（実測）。
- 配置先: エントリ 56448 の SetOutPath 相当の文字列が `<NSIS 変数>\Shared`、直後の 56450 が `Blend2D.dll`。dsp_ippv2_x64.dll（56484）と同じ `Shared` に置かれる並び（実測。NSIS の変数エスケープがどの変数かは未確認。`nsis-ctx-blend2d.txt`）。
- 抽出したファイル: サイズ **2,065,920 B**、SHA-256 **`66634d24412084b7541b4b27a11be8b0f4cb90d844c87e1ee5e5c3ecd66066c7`**（`--hash` と `sha256sum` が一致）。`file` は PE32+ x86-64 DLL、7 セクション。インストーラ内の保存長は 638,399 B（圧縮。`stream header word 0x8009bdbf`）。
- FLEngine の `f_6539e0` の `dl`（`_x64.dll` を付けるか）は Blend2D の呼び出しで**不定**（`xor edx,edx` で 0 を渡す。0x6641b4）ので、`_x64` は付かない。LoadLibraryW に渡る名前は `Blend2D.dll`（読み。実測は gdb の `re6.log` 960 行目の `name='Blend2D.dll'`）。

## 3. PE の事実と lift の規模（項目 2、3）

### 3.1 PE（実測。`pe-info.txt`、`pe-pefile.txt`、`objdump-p.txt`）

| 項目 | 値 |
|---|---|
| Machine / 種別 | 0x8664 / PE32+、Characteristics 0x2022（DLL、LARGE_ADDRESS_AWARE）、Subsystem 2 |
| ImageBase / SizeOfImage | 0x180000000 / 0x300000（3,145,728 B = 3.0 MiB。dsp_ippv2 は 38.1 MiB） |
| エントリポイント RVA | 0x19e584（VA 0x18019e584。MSVC の `_DllMainCRTStartup` の形。§6） |
| DllCharacteristics | 0x0160（HIGH_ENTROPY_VA、DYNAMIC_BASE、NX_COMPAT） |
| TimeDateStamp / リンカ | 0x665d89e4（2024-06-03）/ 14.29 |
| 実行セクション | `.text` のみ（va 0x1000、vsize 0x1b5205、flags 0x60000020）。`IPPCODE` のような第 2 のコードセクションは無い |
| 他のセクション | `.rdata`、`.data`（vsize 0x104bb4、ファイル上 0x1600。残りは 1,062,324 B の未初期化）、`.pdata`（0x74dc）、`_RDATA`、`.rsrc`（0xf8）、`.reloc` |
| export | **1 件**: `AggCreateInstance`（ordinal 1、RVA 0x59a60、フォワーダなし、DLL 名フィールド `Blend2D.dll`） |
| import | **KERNEL32.dll の 1 DLL、94 名**、序数 0、delay import なし、bound import なし |
| `.pdata` | 2,493 エントリ、チェーン 650、関数開始 1,843（`fe-pe pe-pdata-info`） |
| `.reloc` | DIR64 **1,763**、ABSOLUTE 5、他の種別 0（pefile）。ブロック 18 |
| TLS ディレクトリ | **あり**: テンプレート 32 B（0x1801e3770〜0x1801e3790。内容は `0x…80000000` を含む 32 バイト）、`AddressOfIndex` 0x1801f1560（`.data`。ファイル上は 0）、`AddressOfCallBacks` 0x1801b7370（配列の先頭は 0 = コールバックなし）、`SizeOfZeroFill` 0（`tls-dump.txt`） |
| デバッグ | PDB パス `L:\ILSources\FL Studio\Shared\Blend2D.pdb`（debug type 2）ほか 3 件 |
| Load Config | `SecurityCookie` 0x1801f0028、`GuardCFCheckFunctionPointer` 0x1801b72f8、`GuardFlags` 0x100（pefile） |

- `.rsrc` は種別 24（マニフェスト。`res type 24`）のみ。FLEngine から見て `FindResourceW` の対象になるかは調べていない。
- 内容の読み: `.rdata` の RTTI 名が `asmjit::...::x86::Assembler`、`bl::Pipeline::JIT::FetchSolidPart` など。**Blend2D 本体（JIT つき）と、FL Studio 側の `Agg2D` / `Agg2DIntf` のラッパーを 1 つの DLL にしたもの**に見える（`rtti-names.txt`。Blend2D と AsmJit の公式ページ（blend2d.com/doc、GitHub blend2d/blend2d）が「JIT で描画パイプラインを作る」と書くことと整合）。`export` が `AggCreateInstance` の 1 つだけなので、FL Studio 向けに独自の入口を付けたものと読める。**ライブラリの版は未確認**（文字列に版番号は見つからなかった）。

### 3.2 lift の統計（実測。`lift-pdata.txt`）

`fe-lift lift Blend2D.dll blend2d-pdata.c --pdata`（所要 0.44 s、最大 RSS 47.9 MB）。

| 項目 | Blend2D.dll | dsp_ippv2_x64.dll（測り直し） | FLEngine（参考。決定 0079 の記録） |
|---|---|---|---|
| 関数 | **2,018** | 32,648 | 59,297 |
| 命令（閉包） | **398,879** | 6,464,799 | 2,528,030 |
| `emitted_instructions` | 398,886 | 6,511,234 | 2,528,830 |
| `bad_nodes` / 誤デコード | **0 / 0** | 0 / 0 | 0 / 0 |
| 生成 C | **36,418,770 B**（411,794 行） | 566,300,846 B | 215.7 MB |
| `unsupported_total` | **662**（0.166%） | 974,527（14.97%） | 531（0.02%） |
| `tail_calls` / `gotos` | 152 / 31,879 | 22,594 / 450,886 | — |
| `fe-decode closure` | 関数 2,018、ブロック 47,744、直接 call 8,842、直接 jmp 7,802、条件分岐 24,211、ret 2,243、`import_calls` 264、`import_jumps` 24、間接 call 1,411、間接 jmp 125、割り込み 47、distinct のニーモニック 241 | — | — |

- `fe-decode closure --pdata --adjustors` の `decode_failures`、`out_of_text_flow`、`out_of_text_calls` はすべて 0（`closure-pdata.txt`）。
- `--pdata` の種は 1,853（`.pdata` の関数開始と export、エントリ）。**既存の fe-pe / fe-decode / fe-lift は変更なしで通る**（実測。ImageBase も `.text` のみの実行範囲も既存の機構の範囲内）。
- 決定 0074 の reloc seed: DIR64 1,763、読めないスロット 0、実行範囲内を指す値 577、うち `.pdata` 開始 **519**（seed）、開始でない **58**（`reloc_not_function`。`reloc-seeds.txt`、`reloc-not-function.txt`）。`--pdata` に 519 を足しても閉包は変わらない（`fe-decode closure --pdata --reloc-seeds` は関数 2,018 のまま）。reloc seed だけを種にすると関数 1,331、命令 262,974、`unsupported_total` 468。**開始でない 58 個を種に足すと、関数 2,066、命令 399,508、`unsupported_total` 665、`bad_nodes` 0**（`lift-pdata-nf.txt`）。この 58 個は `.pdata` に無い**葉関数**（`ret 0`、`xor eax,eax; ret`、`movaps; mov; movups; ret` の形が 43 個）で、**Agg2D の vtable（130 エントリ）の 34 個を含む**（§5.3）。
- 参考（`--seeds` だけ。`--pdata` なし）: export とエントリだけで関数 212、命令 9,190、`unsupported_total` 68。init 表の 5 項目だけを足して関数 241、命令 12,808、`unsupported_total` 71（`seeds-initterm.txt`）。

### 3.3 未対応命令の内訳（実測。`lift-pdata.txt`。合計 662）

| 理由 | 箇所数 | 備考 |
|---|---|---|
| `V*` ニーモニック（VEX）が最小集合の外 | **495**（約 40 種） | 上位: `Vmulsd` 65、`Vfmadd213sd` 58、`Vmovsd` 55、`Vaddsd` 49、`Vsubsd` 43、`Vmovapd` 31、`Vmovq` 30、`Vmovdqa` 21、`Vmovdqu` 19、`Vcomisd` 12、`Vfmadd231sd` 12 ほか。FMA（`Vfmadd*`、`Vfnmadd*`、`Vfmsub*`）は計約 150 |
| `call [IAT]`（行なしの API） | 79 | 17 名のうち §7 の行なし（`IsProcessorFeaturePresent`、SRWLock、条件変数、`OutputDebugStringA` ほか） |
| `jmp [IAT]`（行なしの API） | 5 | `TlsFree` 2、`GetModuleHandleA` ほか（`imports-scan.txt`、`b2d_impclass.py`） |
| `Jmp` の間接先が未解決 | 12 | `jmp reg` の別形（§3.4）。`jmp [reg+disp]` を含む |
| `Int3` | 41 | パディングまたは致命エラーの後ろ |
| `Int` | 6 | `int 0x29`（fail fast） |
| `gs:` が未モデル | 3 | `gs:[0x60]`（PEB）。`0x30`、`0x58`、`0x10` は対応済み |
| SSE4 などのレガシー | `Pmovzxbd` 8、`Pmulld` 4、`Pcmpistri` 2、`Prefetchw` 6、`Fprem` 1 | 計 21。`.text` に 1 か所ずつ（`0x18005ec80` の関数に `Pmovzxbd`、`Pmulld`。`Prefetchw` は別） |

- 「`Call: indirect through IAT slot (not lifted)`」の 79 と「`Jmp: ... (not lifted)`」5 は、**行のある API の `call [IAT]` は決定 0077 で既に通る**ので、全部が「応答表に行が無い名前」の呼び出し点。`handled` には `call_imp:*` 185 か所、`jmp_imp:*` 19 か所が入っている（応答表に行がある名前）。dsp_ippv2 は `call [IAT]` 138（決定 0077 で 84 が応答、決定 0078 で 133 が応答）。
- ニーモニック別の `handled`（抜粋）: `cpuid` 12、`xgetbv_stop` 3、`ldmxcsr` 8、`stmxcsr` 8、`mov_gs` 29、`vzeroupper` 2、`movntps` 8、`sfence` 1、`fld` 2、`fstp` 4、`fnclex` 1、`fnstsw` 1。dsp_ippv2 は `cpuid` 259、`xgetbv_stop` 7、`ldmxcsr` 1,315、`mov_gs` 3。
- **dsp_ippv2 の未対応の 88% を占めた AVX-512 / ZMM / opmask は Blend2D には 0**（`encscan.txt`）。VEX は 497 命令、10 関数で、EVEX は 0（実測）。

### 3.4 `jmp reg` と vtable 経由の呼び（実測と読み）

- 閉包の `jmp reg` は 99 か所（`register_jmp_sites` 99。すべて `jmp_ind:reg_dispatch`）。`fle_dispatch(R[x]); return;` の末尾呼びになり、**移動先が関数入口でなければ `fle_dispatch_miss` で止まる**（`fle_dispatch.h`）。
- 読みによる分類（`b2d_jmpreg.py`、`jmpreg.txt`）: `.pdata` 内の `jmp reg` 97 か所のうち **95 か所が MSVC の switch 表の形**（`mov Rt,[Rb+Ri*4+disp]; add Rt,Rb; jmp Rt`。表は 32 ビット RVA の配列）、2 か所が別形（`0x18019f8fb`、`0x1801b5b90`。後者は `_pRawDllMain` に当たる `jmp rax` で、スロット 0x1801b7308 から指される）。残り 2 か所は `.pdata` 外の `memmove` 小サイズの表（`0x1801a2060` の関数。移動先 16 個は `.pdata` の外）。
- **95 か所の switch 表の移動先は `.pdata` の関数開始ではない**（`distinct targets total: 1,116`、そのうち関数開始は 1）。ブロックの途中を指す。生成器は `jmp reg` を `fle_dispatch(R[x])` にするため、**これらが実行されると `fle_dispatch_miss` で名前付き停止**になる（読み。実行していない）。FLEngine の閉包でも同じ形で、決定 0036 は 3 か所を注釈つき jump table にして解いた（`jmp_ind:table`）。dsp_ippv2 は `jmp_ind:reg_dispatch` が 366、`jmp_ind:sel_table` が 434。
- **閉包の約 8%（35,082 / 430,451 命令）は生成 C に出ない**（`uncovered.txt`。線形走査の `.pdata` 範囲内で `L_` ラベルの無い命令。パディング 481、残り 34,601）。残りのうち 982 命令は switch 表の移動先の位置にある（読み）。つまり**switch 表のブロックは生成 C に入っていない**。これが §10 の B3。
- 閉包の間接 call は 1,411（`call_ind:dispatch` 1,363、`reg_const` 5、`rip_table` 43）。Agg2D オブジェクトの仮想関数呼び出しは `call [rax+disp]` の `fle_dispatch` で、vtable の中身は静的に決まらない（読み）。

### 3.5 CPUID による CPU 振り分けの有無（実測と読み）

dsp_ippv2 のような**表引きのスタブは無い**（`41 ff 24 c2` のバイト列 0 件。決定 0079 の形は 0）。構造は次のとおり（`b2d-cpuinfo.txt`、`b2d-isa-init.txt`、`sites-*.txt`）:

| 場所 | 内容 | 区分 |
|---|---|---|
| `cpuid` の関数 3 つ | `0x180009ea0`（12 か所のうち 8。AsmJit の CPU 情報。ベンダー文字列 `GenuineIntel`/`AuthenticAMD`、葉 1、7、0x80000001 を読み、`xgetbv` の結果を `&6` で見る）、`0x18019e8b8`（3 か所。MSVC の CRT の ISA 判定。葉 0、葉 1、葉 7。`xgetbv` を `bt edi,0x1b`、`bt edi,0x1c` のあとで実行）、`0x1801b0e00`（1 か所。葉 1 の ECX を `0x18001000` と比べ、OSXSAVE と FMA のビットが両方立つときだけ `xgetbv`。`&6 == 6` なら変数 0x1801f2164 を 1 にし、0x1801f2168 に写す） | 実測と読み |
| 振り分けの変数 | `0x1801f2168`。これを `cmp dword ptr [rip+...],0` で見る関数が **5 つ**（`0x1801afac0`、`0x1801b24d0`、`0x1801b2de0`、`0x1801b4480`、`0x1801b4b60`）。0 なら SSE2 版、非 0 なら FMA（VEX）版の枝に入る（読み。`xorpd`、`cvttpd2dq` を使う枝と、`vfmadd213sd` を使う枝が同じ関数の中にある） | 実測と読み |
| 現行 runtime の CPUID 応答 | 葉 1 の ECX は 0（決定 0009、0047）なので、`0x1801b0e00` は `xgetbv` に入らず、`0x1801f2168` は 0 のまま（読み。実行していない） | 読み（仮説） |
| `0x18019e8b8` の ISA 判定 | `bt edi,0x14`（SSE4.2）が立たないので `0x18019ea47` へ飛び、`xgetbv` は実行されない（読み） | 読み |
| `memmove` の AVX 版 | `0x1801a2060` の `cmp dword ptr [0x1801f0008],3; jb`（AVX2 以上のときだけ `vmovdqu ymm0`）。`0x1801f0008` は `0x18019e8b8` が書く ISA レベル。ECX が 0 なら 3 に届かず SSE 版（`0x1801a24a0`）へ飛ぶ（読み） | 読み |
| 現行の `xgetbv` の扱い | 3 か所とも `fle_xgetbv_stop`（到達すると名前付き停止。決定 0047）。**CPUID を変えない限り、前提の `bt` で飛ばされて到達しない**（読み） | 読み |

- 決定 0079 のスタブ表（`fle_jumptab_miss`）は**使わない**。振り分けの結果は `jmp [表+添字*8]` ではなく、**変数を見る分岐の中に VEX 命令が直接入っている**。このため「選択値が 0 になる」仮説ではなく「`0x1801f2168` が 0 のまま」という仮説になる。現行の CPUID 応答のままでは、VEX を含む 10 関数のうち FMA 版の枝（`0x1801b24d0` ほか 5 関数）には入らない（読み。**未確認**: 実行していない）。
- ただし VEX を含む関数のうち `0x1801a40d8` と `0x1801a4228`（`vpxor`、`vpcmpeqb`、`vpmovmskb`。`.pdata` に無い `wcslen` / `strlen` の AVX2 版）と `0x1801a2060`（`memmove`）は**別の変数（`0x1801f0008`、`0x1801f000c`）で選ぶ**。これらも ECX = 0 なら入らない（読み）。
- **dsp_ippv2 との違い**: dsp_ippv2 は 6 通り（選択値 0〜5）× 434 個のスタブで、AVX / AVX2 / AVX-512 の系統が 90 万命令ある。Blend2D は SSE2 版が基本で、VEX は 497 命令。CPUID を現行のまま（決定 0080 D8）にするなら、**実行されない見込みの命令**として扱える（読み）。
- ただし**AsmJit は実行時に機械語を生成する**（§3.6、§4.3）。生成される機械語は選んだ機能セットで決まる。この DLL が JIT で作ったコードを呼ぶ経路は、**lift の対象にならない**（実行時に作られるバイト列は静的な閉包に無い）。それが実行されるかは §4.3 で扱う。

### 3.6 wasm の locals（実測と読み。**近似**）

方法: 生成 C を `emcc -O0`（`build.sh` の `CFLAGS_GEN` と同じ。`-w -fno-builtin -ffp-contract=off -fno-fast-math`）で `.o` にし、先例の `lift/out-design/tools/wasm_locals.py` で各関数の locals の数を読んだ（`wasm-locals.txt`）。`build.sh` と同じ `-finstrument-functions` つきでも測った（`obj/*-instr.o`）。**近似**: リンクもノード（`WebAssembly.instantiate`）での受理確認もしていない。locals は `.o` のコードセクションから数えた値で、リンク後の値と同じとは限らない。

| 入力 | 関数（wasm） | 最大 locals | 50,000 超 | 所要 | 最大 RSS |
|---|---|---|---|---|---|
| `--pdata`（関数 2,018） | 2,157 | **102,086** | **4** | 17.9 s | 3.3 GB |
| 同（`-finstrument-functions`） | 2,157 | 102,092 | 4 | 17.0 s | 3.4 GB |
| reloc seed のみ（関数 1,331） | 1,452 | 15,620 | 0 | 10.1 s | 2.2 GB |
| reloc seed + 開始でない 58（関数 1,393） | 1,514 | 15,618 | 0 | 7.6 s | 2.2 GB |
| export・エントリ + init 表 5（関数 241） | 281 | 1,355 | 0 | 0.5 s | 0.16 GB |

- `.o` のサイズは `--pdata` で 35.8 MB（FLEngine の生成物 173.6 MB の約 21%）。リンク後の wasm のサイズは**未測定**。FLEngine の wasm は 115.7 MB（`out-wasm2/bin-base/flengine.wasm`）で、生成 C の 0.537 倍（決定 0045）。**推測**: Blend2D は 20 MB 前後。
- 50,000 超の 4 関数: locals 102,086 / 101,762 / 98,211 / 97,904。生成 C の行数の多い 4 関数と**サイズが一致**（`f_180159b70` 11,136 行 980,209 B、`f_180171d60` 11,120 行、`f_180165f60` 10,860 行、`f_18017e100` 10,840 行。`.o` の関数サイズで照合。`big-funcs.txt`）。5 番目は `f_180076f60`（8,068 行、locals 36,848）で上限内。
- **この 4 関数の正体**: `f_1801599d0`（`lea` で関数ポインタとして取られる）と `f_1801594d0` から呼ばれる（`big4-callers.txt`）。4 関数を含む上流 81 関数のうち 56 が「呼び元なし」で、**うち 42 は `lea` で関数ポインタとして取られる**（`big4-lea-roots.txt`）。それらを取る関数は `0x180147870`、`0x180147cf0`（`.pdata` に入らない初期化関数。§5.2）で、**Agg2D の描画の vtable（`0x1801dbec0` の 130 エントリ）に連なる**（読み）。つまり**描画の実体**で、AsmJit の JIT ではない（`vtable` の中身を AsmJit の関数と同定したわけではない。**4 関数の機能は未確認**。ラスタライザの塗りつぶしに見えるが読み切っていない）。
- **到達**（読み。静的。`b2d_reach3.py`、§5.1 の表）: ATTACH の種（エントリ、`.xc` 1、`.xi` 4）から**直接 call だけで**、4 関数のうち `f_180159b70`、`f_180171d60`、`f_180165f60`、`f_18017e100` が届く（`reach3c-attach-direct.txt`）。`AggCreateInstance` を種にすると、直接 call だけでは 43 関数で、4 関数は**届かない**。vtable / 表を許すと `AggCreateInstance` からは `f_180076f60`、`f_18008ba50` までで、4 関数は届かない（`reach3b-aggcreate.txt`）。ATTACH 集合の方は初期化子 `f_180199d90` が呼ぶ `f_180147870`、`f_180147cf0` が `lea` で Agg2D 描画の関数ポインタを大量に取るため（§5.2）届く（**reach3 は生成 C の `lea` 由来の即値も辿る数え方。`fe-lift --seeds` の閉包は辿らない**ので、縮小の生成物には 4 関数の本体は入らない。§10.5）。**この作業の途中版の「4 関数は init からも届かない」という記述は、reach2 が `.pdata` に無い葉関数を落としていた値によるもので、誤りだった**。
- 参考: 決定 0080 D7（縮小 DLL）の流儀で `--seeds reloc-seeds.txt` にすると最大 15,620 で上限内。ただしこの縮小は 4 関数を**生成から外す**ので、描画が呼ばれると `fle_dispatch_miss` で止まる（§10 の B2）。

## 4. FLEngine が Blend2D から何を使うか（項目 4）

### 4.1 呼び出し点と GetProcAddress（実測。`fl-664190-664260.txt`、`fl-xrefs-blend2d.txt`、`fl-xrefs-gpa-wrapper.txt`）

`f_664190`（0x664190〜0x6641f9。**FLEngine で `"Blend2D.dll"` の文字列を使うのはここだけ**）:

```
664194: cmp qword ptr [0x15f4600], 0      ; ハンドルが非 0 なら
66419c: je 0x6641a2
66419e: mov al, 1 ; jmp 0x6641f4          ; true で戻る（再ロードしない）
6641a2: mov qword ptr [0x15f45f0], 0      ; AggCreateInstance のスロットを 0 に
6641ad: lea rcx, ['Blend2D.dll'] ; xor edx,edx ; xor r8,r8
6641b9: call f_6539e0                     ; dl=0, r8=0
6641be: mov [0x15f4600], rax              ; ハンドルを保存
6641c5: cmp qword ptr [0x15f4600], 0
6641cd: je 0x6641e9                       ; NULL なら GetProcAddress を飛ばす
6641cf: mov rcx,[0x15f4600] ; lea rdx,['AggCreateInstance']
6641dd: call 0x4235f0                     ; GetProcAddress のラッパー
6641e2: mov [0x15f45f0], rax              ; 保存先
6641e9: cmp qword ptr [0x15f45f0], 0
6641f1: setne al                          ; 戻り値 = (保存先 != 0)
```

- GetProcAddress に渡す名前は **`AggCreateInstance` の 1 つだけ**（FLEngine の UTF-16 文字列は 0x263620 に 1 件。呼び出し点は 0x6641dd の 1 か所。実測）。保存先は **0x15f45f0**（`.bss`）、ハンドルは 0x15f4600、参照カウントは 0x15f45f8。
- **この名前は Blend2D の export に実在する**（export は `AggCreateInstance` の 1 件のみ。RVA 0x59a60。集合差は空）。dsp_ippv2_x64.dll も同名の export を持つ（RVA 0x17c82f0。ゲストは引かない 26 名の 1 つ）。
- 呼び出し元: `f_664190` を呼ぶのは **2 か所**（`f_664260` の 0x664274 と、`f_6cf2a0` の 0x6cf31f）。`f_664260` は unit 149（`units.tsv` の proc 0x664260、`second_va` 0x664250）で、`[0x15f45f8]` を 1 減らして `-1` になったら `f_664190` を呼ぶ（初期化の参照カウント。0x664250 が加算側の `second_va` で、`units.tsv` の初期化表のスロットとして参照される。直接 call の呼び元は 0 件）。`f_6cf2a0` は unit 190（proc 0x6cf480）の 1 段下で、フォントの縮尺（`cvtsd2ss`、`call 0x40c650`、`0x40bc90`）などを計算してから `f_664190` を呼ぶ（読み）。
- NULL のときの分岐は `loadlibrary-names-classification.md` §2.2 の #4 と一致を再確認した: 0x6641cd で `je 0x6641e9`、GetProcAddress を飛ばし、スロットは 0 のまま `setne al` で false。`f_664260` は戻り値を使わない（`add rsp; ret`）。`f_6cf2a0` は直後の `call 0x664190` の戻りを**見ずに**次の呼び出し（`call 0x5c8500`）へ進む（読み）。**どちらも NULL を許容する形**（読み）。実測: avrt、PowrProf、Blend2D を仮の NULL にして進めた run で SIGSEGV は出ず、停止は RegCreateKeyExW まで（`re6.*`）。

### 4.2 保存先を読む側（読み。実測の一部）

- スロット 0x15f45f0 を指すセルは `.data` の 0x15e3990 の**1 か所**（`b2d_cells.py`。8 バイト値）。そのセルを読む関数は **`f_6cb0b0` の 1 か所**（`mov rax,[0x15e3990]; call [rax]`。0x6cb0b9〜0x6cb0c0）。
- `f_6cb0b0`（0x6cb0b0〜0x6cb142）: `AggCreateInstance()` を呼び、戻りの Agg オブジェクトを `[rbx+0x58]` に保存する。**NULL の検査は無い**（`call [rax]` の直後に `mov [rbx+0x58], rax`。スロットが 0 なら 0 番地を呼ぶ）。続いてオブジェクトの vtable の `+0x3f8` を呼び（画面の拡大率に応じた `cvtsi2sd`、`mulsd`）、`f_6cb020`、`f_6ce2c0`、`f_6ce5c0` を呼ぶ。
- `f_6cb0b0` の呼び出し元は **91 か所（77 関数）**。直前に `cmp qword ptr [rbx+0x58],0; jne` を置く形が多く、`f_6cb190`（`[rcx+0x58]` が 0 のときだけ `f_6cb0b0` を呼んで `[rcx+0x58]` を返す。呼び出し元 39 か所）が代表的な入口。**ゲストは Blend2D が使えない場合の代替経路を持たない**ように見える（`[rbx+0x58]` が 0 のときにスロットを検査せず `f_6cb0b0` を呼ぶ。読み。**ゲストが `f_664190` の false を見てから描画を止めるかは、呼び出し元 77 関数を読み切っていない（未確認）**）。
- Agg オブジェクトの vtable で FLEngine が呼ぶ変位（リテラルの形だけを拾った下限。`fl-vtuse.txt`）: 130 か所の呼び出し点のうち 81 か所で 31 種の変位（0x1c、0x58、…、0x408、**0x1274**）。0x408 は slot 129 で、Blend2D の vtable（130 エントリ）の範囲内。0x1274（slot 590）は 130 を超える**別のオブジェクトの vtable**（`mov V,[obj]` の追跡が別のレジスタに移った誤検出の可能性が高い。**未確認**）。0x3f8 は slot 127。
- dsp_ippv2 のときは `f_653f40` の戻り値を 3 つの呼び出し元が使わず、保存先を読む関数が 424 あった。**Blend2D は読み手が 1 つの関数（`f_6cb0b0`）に集約されている**（実測）。

### 4.3 起動（unit 初期化）の範囲か、描画経路か（読み。静的な到達）

`units.tsv` の proc を根とした前回のグラフ（`edges-direct.tsv`＝直接 call/jmp、`edges-ptr.tsv`＝`lea` つきの関数開始）で、到達する unit を測った（`b2d_reach.py`、`fl-reach-units.txt`、`fl-callers-units.txt`）。

| 関数 | 到達する unit（直接のみ / `lea` つき） | 読み |
|---|---|---|
| `f_664190`（LoadLibraryW を呼ぶ） | 149、190 / 同じ | **起動の unit 149 と 190 から直接 call で届く**（実測。gdb の InitCount 150 = index 149 で停止を確認済み） |
| `f_6cf2a0` | 190 / 同じ | unit 190 の `f_6cf480` から直接（`fl-paths.txt`） |
| `f_6cb0b0`（AggCreateInstance を呼ぶ） | 895、898 / 614、745、880、895、898 | 起動の unit のうち**遅い方**（run_rank 851、854。`lea` つきで 836、701、570 も）。最短経路は 13 段（`f_12ddfe0` → … → `f_127d420`（`P_DigitWheel` の描画）→ `f_6cb0b0`） |
| `f_6cb0b0` の呼び出し元関数 77 | **71 関数は unit に届かない**（直接も `lea` つきも）。unit に届くのは 6 関数（`f_127d120`、`f_127d420`、`f_1280530`、`f_6cb190`、`f_7fdb70`、`f_ddf630`） | 大半は vtable / コールバック（`edges` に無い間接呼び）の先 |

- 読み: **LoadLibraryW と GetProcAddress は起動の unit 149（と 190）で実行される**（実測）。**AggCreateInstance の呼び出し（`f_6cb0b0`）は静的には unit 614 以降（`lea` つき）、直接のみなら 895、898 から届く**。ただし 895、898 は `units.tsv` で run_rank 851 と 854（全 895 のうち後ろ。init_count の最後の方）で、**現行の停止点（init_count 154 / 942）よりずっと先**。したがって「ロードは unit 149 で起きるが、Blend2D の呼び出しは描画の最初の段（unit 614 以降）」と読める。**前者は実測、後者は静的な到達の読み**で、実行で確かめていない。
- 停止点の次: Blend2D を NULL にしても停止は RegCreateKeyExW（unit 153）に移る（`re6.log`）。**その先は未測定**。
- **`VirtualAlloc` の使い方（読み）**: 呼び出し点は 2 か所（`f_180018250` の 0x180018303 と `f_18000e4d0` の 0x18000e6a7。`b2d-vmem-xrefs.txt`）。どちらも `lpAddress` = NULL（`xor ecx,ecx`）、`flAllocationType` は `0x3000`（MEM_COMMIT | MEM_RESERVE）、`GetLargePageMinimum` の結果が非 0 で要求サイズがその倍数のときだけ `0x20003000`（`0x20000000` を足す。`MEM_LARGE_PAGES` の値と読むが、**Learn のページでの確認はしていない**）。`flProtect` は `f_18000e4d0` では `r9d` = 0x40（`lea r9d,[rdx+0x40]`）、`f_180018250` では引数 r8d のビットから 1、2、4、8 のどれかに計算される（`shl ebx,5` などの形）。つまり**実行可能なページ（PAGE_EXECUTE_READWRITE = 0x40）を要求する経路がある**（読み。AsmJit の `JitRuntime` が機械語のバッファを取る形と整合する）。
- 現行 runtime の `fle_imp_VirtualAlloc` は `flProtect` が `PAGE_EXECUTE`（0x10）、`PAGE_EXECUTE_READ`（0x20）、`PAGE_EXECUTE_READWRITE`（0x40）を扱い（`fle_sys.c` 494〜496 行目）、**`MEM_LARGE_PAGES` を含む `flAllocationType` は名前付き停止**（同 643 行目のコメントと 673 行目の `STOP`。読み）。`GetLargePageMinimum` は行なし（§7）なので、`f_180018250` の 0x1800182c8 は `fle_unsupported` のまま止まる（`lift-pdata.txt` の `0x1800182c8 Call`）。
- **JIT で作った機械語は lift の対象外**。wasm では実行可能メモリに書いたバイト列を実行する手段が無い（`call` で間接呼びしても `fle_dispatch` の表に入らず `fle_dispatch_miss`。`fle_dispatch.h`）。したがって **JIT のコードを使う描画は、そのままでは動かない**（読み）。JIT を使わない経路（Blend2D の `BL_CONTEXT_CREATE_FLAG_DISABLE_JIT`。公式ドキュメントに「JIT パイプライン生成を無効にする」とある。blend2d.com/doc の BLContextCreateFlags の項。2026-10-24 取得）が**この DLL の `Agg2D` ラッパーから選べるか、選ばれているかは未確認**（ラッパーの引数は読み切っていない）。これが §10 の B1。
- `f_180018250`、`f_18000e4d0` の祖先は `f_18000eabc` だけで、`f_18000eabc` は呼び元も data 参照も持たない（`b2d-vmem-callers.txt`、`jit-ancestors-*.txt`、`b2d_cells.py` で 8 バイト値 0 件）。**export からも init 表からも直接 call では届かない**。`reach3`（vtable / 表を許す over-approx。`reach3-attach.txt`、`reach3b-aggcreate.txt`）の到達集合には `f_180018250`、`f_18000e4d0` が**入る**が、`f_18000eabc` 自体は入らない。経路の読み: `f_18000e4d0` の祖先の根は `f_18000fdb0` で、これは `.rdata` 0x1801da9a8 の 3 エントリの vtable（0x1801da9a0〜0x1801da9b8）のエントリ。その vtable を `lea` で取る関数は `f_18000fbc0`、`f_18000fd60`、`f_1800a9a90` で、いずれも **`.xc` の C++ 静的初期化子 `f_180001000` から届く**（`b2d_ancestors.py` の終端が INIT `0x180001000` と DATA。直接 call だけの集合 `reach3c-attach-direct` では `f_18000fbc0` と `f_1800a9a90` は入るが `f_18000fdb0` は入らない）。つまり**ATTACH の C++ 静的初期化がオブジェクトを作り、その vtable の関数が後で JIT のメモリ確保に届く**という形に読める（読み。**どの vtable 関数が ATTACH の間に呼ばれるかは未確認**）。**JIT が実際に走るかは未確認**。

## 5. Blend2D の側から見た到達（項目 4 の続き。読み。静的）

方法: 生成 C（`blend2d-pdata-nf.c` = `--pdata` に reloc の開始でない 58 個を足したもの。関数 2,066）の呼び出しグラフと、`lea` で取られた関数ポインタ、`lea` で取られた `.rdata` / `.data` の表（連続する `.text` へのポインタの列）の全エントリを辿る（`b2d_reach3.py`。**表を辿る方は over-approx、実行時にメモリから読んだポインタは辿らないので under-approx**。両方の性質を持つ）。`--no-tables` は直接 call と、`lea` で関数を直接取る形だけ。

### 5.1 到達集合の大きさ

| 種 | 辿り方 | 関数（of 2,066） | VEX 関数（of 10） | 4 つの巨大関数 | `fle_unsupported`（箇所 / 関数） |
|---|---|---|---|---|---|
| ATTACH（エントリ、`.xc` 1、`.xi` 4） | 直接 call だけ | 845 | 9 | 届く（`f_180076f60` を含め 5 つ） | 529 / 69 |
| 同 | 表と `lea` を許す | 1,344 | 9 | 6 つとも届く | 542 / 81 |
| `AggCreateInstance` | 直接 call だけ | **43** | 0 | 届かない | **4**（`Int3` のみ）/ 3 |
| 同 | 表と `lea` を許す | 958 | 9 | `f_180076f60`、`f_18008ba50` | 469 / 53 |
| 両方 | 表と `lea` を許す | 1,471 | 9 | 6 つとも | 556 / 90 |

- **`AggCreateInstance` を直接 call でたどった 43 関数は、`LoadLibraryExW` 4、`GetProcAddress` 2、`HeapAlloc` 4、`HeapFree`、`EnterCriticalSection`、`TlsGetValue`、`TlsSetValue`、`RaiseException`、`RtlPcToFileHeader` だけを使う**（`reach3c-aggcreate-direct.txt`）。`f_18019dd68`（`operator new` に当たる再試行ループ）まで。**`AggCreateInstance` はヒープに `0x1170` バイトを確保して `f_1800568a0`（コンストラクタ。vtable `0x1801dbec0` を書く）を tail call する**（読み。`b2d-aggcreate-entry.txt`）。コンストラクタは `[rdi]` の vtable から `+0x1a0` を呼ぶ（`call [rax+0x1a0]`）ので、ここから先は仮想関数。
- **Agg2D の vtable `0x1801dbec0`: 130 エントリ、distinct 79。うち 26 は `.pdata` に無い葉関数**（`ret 0` の `0x18000c530`、`xor eax,eax; ret` の `0x18000b820`、フィールドを返す・書くだけの小さな関数）。決定 0074 の reloc seed（`.pdata` 開始のみ）では 53 しか入らない。**`--pdata --seeds reloc-not-function.txt`（58 個を足す）で 79 全部が生成 C に入る**（`vt-dispatch.txt`、`b2d_vt_dispatch.py`）。これは決定 0074 が「採らない」とした「開始でない値を seed にする」に当たる（§10 の B5）。
- init 表のエントリは 5 つのうち 4 つが `.pdata` の関数、`0x180001000`（`.xc` の C++ 静的初期化子）は `call 0x180199d90` の後に `_atexit` 相当（`0x18019e24c`）を呼ぶ（読み。`b2d-initterm-funcs.txt`）。

### 5.2 AsmJit / Blend2D の初期化子 `f_180199d90`（ATTACH の `.xc` から呼ばれる。読み）

`f_180001000` が `f_180199d90`（0x180199d90〜0x18019b0b1、4,897 B）を呼ぶ。内容（`b2d-ctor-full.txt`、`b2d-ctor-calls.txt`）:

- `lock xadd` で初回だけ実行（一度きりの初期化）。
- `f_18000abb0`（CPU 情報の初期化。`gs:[0x58]` を読み、`f_180009ea0`（`cpuid` 8 か所）を呼ぶ）で機能ビットを集める。
- ベンダー文字列を `AMD`、`INTEL`、`VIA` と比べ、結果を `0x1801f3198`/`0x1801f319c` に書く（現行の応答ではベンダーは 0 なので**どれにも一致しない**。読み）。
- **`GetSystemInfo`**（`0x1801b7000`）を `lea rcx,[rsp+0x40]` に呼び、`dwAllocationGranularity` と `dwPageSize` を `0x1801f3128`、`0x1801f3130` に書く（`FLE_PAGE_SIZE` 4,096、粒度 65,536 の現行の応答を使う）。
- `GetModuleHandleA("api-ms-win-core-synch-l1-2-0.dll")`（0x180199fed。**行なし。`call [IAT]` で `fle_unsupported`**）。結果が NULL でなければ `GetProcAddress` で `WaitOnAddress`、`WakeByAddressSingle`、`WakeByAddressAll` を引く。**現行では `GetModuleHandleA` の呼び出し点が `fle_unsupported(0x180199fed)` なので、ここで名前付き停止になる**（`lift-pdata.txt` で生成 C の 380614 行目を確認）。
- `call r8`（0x18019a1d2。`[rax+0x20]` の vtable 呼び）で `fle_dispatch(t)` に入る。ターゲットは `0x1802f47d0` の構造体の vtable（`lea rax,[rbx+0x2f4790]` で作る。`.data` の 14 個の関数ポインタ表 `0x1802f4750〜0x1802f47c8`、初期化関数 5 つが `0x18019c4a0`、`0x18019c480` ほか）。`0x18019c480`、`0x18019ca20`、`0x18019ca80`、`0x18019ca90` は**生成 C に関数として入っていない**（`.pdata` に無い葉）。58 個を足せば入る。
- その後は `f_1800a9eb0`、`f_18006b6c0`、`f_180147870`、`f_180147cf0`、`f_18006ed70`（Agg2D の vtable 表を作る `lea` + `mov` の連続）を呼ぶ。**これらは `.pdata` に無い関数**（`f_180147870`、`f_180147cf0` は `.pdata` に入らない。生成 C には 58 個を足したときと `--pdata` の両方で入る。`lea` で取られた関数の開始を表す）。

## 6. DllMain の ATTACH（項目 5 の前半。読み。静的）

エントリ `0x18019e584`（`_DllMainCRTStartup` の形。`b2d-aggcreate-entry.txt`、`b2d-dllmain-sub*.txt`）:

1. `reason == 1`（`DLL_PROCESS_ATTACH`）のとき `f_18019ea70`（`__security_init_cookie`。`GetSystemTimeAsFileTime`、`GetCurrentThreadId`、`GetCurrentProcessId`、`QueryPerformanceCounter` を XOR する。FLEngine と dsp_ippv2 で通っている形。決定 0017、0020、0056）を呼ぶ。
2. `f_18019e450`（`__scrt_dllmain_dispatch`）→ `f_18019e264`（reason で分岐）→ `0x18019e2b4`（ATTACH 本体）。ここで `[0x1801f1508]`（初期化状態）、`f_18019df54`、`f_18019eb64`（`_initterm` 前の準備）、`f_18019eb1c`、`f_18019eb40`、`_initterm_e`（`.xi` の 4 項目: `0x18019db00`、`0x1801a4400`、`0x1801af6e4`、`0x1801b0e00`）、`_initterm`（`.xc` の 1 項目: `0x180001000`）を呼ぶ。`0x1801b7308`（`_pRawDllMain`）のスロットは `0x1801b5b90` の `jmp rax`（`.rdata` のスロット 0x1801b7478 が NULL なので呼ばれない読み。`b2d-dllmain-sub.txt` の `test rax,rax; jne`）。
3. **`.xi`（C の初期化）4 項目**:
   - `0x18019db00`: `InitializeCriticalSectionAndSpinCount(0x1801f14d0, 0xfa0)`、`GetModuleHandleW("api-ms-win-core-synch-l1-2-0.dll")`（NULL なら `GetModuleHandleW("kernel32.dll")`）、`GetProcAddress` で `SleepConditionVariableCS`、`WakeAllConditionVariable`。**両方 0 でなければ保存、どちらかが 0 なら `CreateEventW` で代替**。保存先が 0 のときの致命経路 `f_18019e5d4`（`mov ecx,7`）。
   - `0x1801a4400`: `__acrt_initialize_lowio` 相当（`_calloc_crt` で 0x200 個の表を作り、58 バイトずつのハンドル表を初期化。`0x1801f0050`、`0x1801f0068`）。
   - `0x1801af6e4`: `f_1801ab2c8`（`__isa_available_init` の先の判定。0 を返さなければ 0 を返す）。
   - `0x1801b0e00`: `cpuid`（葉 1）→ ECX の `0x18001000` を見て `xgetbv`。結果は `0x1801f2164`、`0x1801f2168`（§3.5）。
4. **`.xc`（C++ の静的初期化）1 項目 `0x180001000`**: `f_180199d90`（§5.2。`GetSystemInfo`、`GetModuleHandleA`、`GetProcAddress`、CPU 情報、vtable 表の構築）→ `_atexit`。

- **スレッド**: import に `CreateThread`、`ExitThread`、`FreeLibraryAndExitThread` がある（dsp_ippv2 には無い）。呼び出し点は `f_1801a4888`（`CreateThread` 0x1801a48ed）と `f_1801a475c`、`f_1801a47c4`（`ExitThread`、`FreeLibraryAndExitThread`）。`f_1801a4888` の呼び元は `f_18019ccf0` の 1 か所（0x18019ce6e）で、その祖先は `f_18019cfa0`（`.data` 0x1802f47c0 のポインタ表のエントリ。§5.2 の 14 個の 1 つ）。**Blend2D のスレッドプール（`BL_CONTEXT_CREATE_...`、`thread_count`）の作成**に見える（Blend2D の公式ドキュメントの「Multithreaded Rendering」が、既定は単一スレッドで、`BLContextCreateInfo::thread_count` を渡すとスレッドを使うと書く。blend2d.com/doc。2026-10-24 取得）。**FL Studio の呼び出しが `thread_count` を渡すかは未確認**。決定 0080 D9（スレッドは現行のまま。`DLL_THREAD_*` を受ける経路は作らない）と **衝突しうる**（B4）。`CreateThread` は応答表に行がなく `fle_unsupported`（`call [IAT]`）。`f_1801a4888` は ATTACH の直接 call 集合には入らず、表を許す集合（`reach3-attach.txt`）では `CreateThread 1/1` が入る。
- **TLS ディレクトリがある**（§3.1）。`_tls_index` は `0x1801f1560`。`gs:[0x58]`（`ThreadLocalStoragePointer`）の読みが **13 関数 29 か所**（`gs:[0x58]` を `mov rax,gs:[0x58]; mov rcx,[rax+rcx*8]; mov eax,[rdx+rcx]` で引く MSVC の `thread_local` の形。C++ の「関数内 static の初期化ガード」`__Init_thread_header` / `_footer`。`f_18019dbf8`、`f_18019dc58`。`__Init_thread_epoch` は `0x1801f0000`）。現行の生成器は `gs:[0x58]` を `fle_tls_values`（`TlsAlloc` のスロット表。最大 1,088 個）の先頭アドレスで答える（`fle_vex.h`）。**暗黙の TLS（`.tls` のテンプレートをスレッドごとに配る）をモデル化しているわけではない**。読みの帰結（実行していない）:
  - `mov ecx,[0x1801f1560]`（`_tls_index`）はファイル上 0 のまま。PE Format は「ローダーが `Address of Index` の指す場所に TLS index を書く」と述べる（Microsoft Learn の PE Format の `.tls` 節、2026-10-24 取得）が、現行の harness / runtime にそれを書く処理は**見つからなかった**（`lift/harness`、`lift/runtime`、`lift/crates` の `TLS`/`tls_index`/`AddressOfIndex` の grep。`fle_vex.h` と `fle_imports.c` の `TlsAlloc` の表だけ）。
  - したがって `mov rcx,[rax+rcx*8]`（`rax` = `gs:[0x58]` = `fle_tls_values`）は `fle_tls_values[0]` を読む。これは **`TlsAlloc` が最初に返す index 0 のスロットと同じ場所**で、`TlsSetValue(0, …)` で別の用途の値が入るか、未設定なら 0。0 のとき `mov eax,[rdx+rcx]`（`rdx` = 8）は**アドレス 8 を読む**（`fle_addr` は wasm では `a >> 32` が 0 なので通り、リニアメモリの 8 番地を読む。native では SIGSEGV）。
  - テンプレート 32 B（§3.1。内容は `0x…80000000` を含む）をスレッドの TLS ブロックにコピーする処理も**無い**。
  - **FLEngine と dsp_ippv2 は TLS ディレクトリを持たない**（実測。pefile で `DIRECTORY_ENTRY_TLS` なし、データディレクトリ 9 が 0 / 0）ので、この経路は今まで通っていない。決定 0032 の `gs:[0x58]` は `TlsAlloc` の表を指すだけの最小の答えで、**暗黙の TLS を決めた決定ではない**（`fle_vex.h` のコメント）。これが §10 の **B6**。
- **CRT のロック**: `InitializeCriticalSectionAndSpinCount` 3、`GetModuleHandleW` と `GetProcAddress` はいずれも決定 0078、0081 の範囲で答えられる。**`GetModuleHandleW("api-ms-win-core-synch-l1-2-0.dll")` は N=27 の仮想モジュール（決定 0081）で応えられる**（`.dll` 付き。`fle_modules[]` の `"api-ms-win-core-synch-l1-2-0.dll"`）。`GetProcAddress(N=27, "SleepConditionVariableCS")` は応答表に行がないので NULL と 127（読み。決定 0081。**ここで CRT は `CreateEventW` の代替に落ちる**）。`GetModuleHandleA` の方は行なし（§3.3）。

## 7. import と現行 runtime の突き合わせ（項目 5 の後半）

方法: `lift/crates/fe-lift/src/imports_table.rs` の `TABLE` の行と、`lift/runtime/*.c` の `fle_imp_*` の本体を読み取りで突き合わせた（`b2d_impclass.py`。**テキストの判定**: `FLE_NAMED_STOP`／`FLE_VOL_NAMED_STOP` の定義、または本体が 1 つの `STOP`/`fle_stop` だけの関数を NAMED-STOP、それ以外を ANSWERS とする。ANSWERS の中にも引数次第で停止する関数がある）。出力は `imports-class.txt`、`imports-class-dsp-ippv2.txt`。

| 区分 | 数 | 名前 |
|---|---|---|
| **行なし**（`call [IAT]` / `jmp [IAT]` は `fle_unsupported(VA)`） | **17** | `AcquireSRWLockExclusive`、`AcquireSRWLockShared`、`ReleaseSRWLockExclusive`、`ReleaseSRWLockShared`、`SleepConditionVariableSRW`、`WakeConditionVariable`、`WakeAllConditionVariable`、`CreateThread`、`ExitThread`、`FreeLibraryAndExitThread`、`GetFileSizeEx`、`GetLargePageMinimum`、`GetModuleHandleA`、`IsProcessorFeaturePresent`、`OutputDebugStringA`、`TlsFree`、`WaitForSingleObjectEx`（IAT 参照 84 か所） |
| **名前付き停止**（行あり。呼ばれたら必ず止まる） | **19** | `EncodePointer`、`FindFirstFileExW`、`FlushFileBuffers`、`GetConsoleMode`、`GetConsoleOutputCP`、`GetFileType`、`HeapReAlloc`、`HeapSize`、`IsDebuggerPresent`、`RtlCaptureContext`、`RtlLookupFunctionEntry`、`RtlPcToFileHeader`、`RtlUnwindEx`、`RtlVirtualUnwind`、`SetFilePointerEx`、`SetStdHandle`、`SetUnhandledExceptionFilter`、`UnhandledExceptionFilter`、`WriteConsoleW`（dsp_ippv2 と同じ 19 名） |
| **応答する**（行あり、本体あり） | **58** | `GetLastError`、`SetLastError`、`TlsAlloc`、`TlsGetValue`、`TlsSetValue`、`HeapAlloc`、`HeapFree`、`GetProcessHeap`、`VirtualAlloc`、`VirtualFree`、`CreateFileMappingW`、`MapViewOfFile`、`UnmapViewOfFile`、`CreateFileW`、`WriteFile`、`CloseHandle`、`CreateEventW`、`SetEvent`、`ResetEvent`、`WaitForSingleObject`、`InitializeCriticalSection`、`InitializeCriticalSectionAndSpinCount`、`Enter/Leave/DeleteCriticalSection`、`InitializeSListHead`、`InterlockedFlushSList`、`GetSystemInfo`、`GetModuleHandleW`、`GetProcAddress`、`LoadLibraryExW`、`FreeLibrary`、`GetModuleHandleExW`（本体は `fprintf; abort`。**実質は名前付き停止**。上の判定が ANSWERS にした）、`GetStartupInfoW`、`GetStringTypeW`（本体は `fle_locale_stop`。**実質は名前付き停止**）、`GetACP`、`GetOEMCP`、`GetCPInfo`、`IsValidCodePage`、`MultiByteToWideChar`、`WideCharToMultiByte`、`LCMapStringW`、`GetEnvironmentStringsW`、`FreeEnvironmentStringsW`、`GetCommandLineA`、`GetCommandLineW`、`GetModuleFileNameW`、`GetCurrentProcess`、`GetCurrentProcessId`、`GetCurrentThreadId`、`GetSystemTimeAsFileTime`、`QueryPerformanceCounter`、`GetStdHandle`、`FindClose`、`FindNextFileW`、`ExitProcess`、`TerminateProcess`、`RaiseException`（最後の 3 つは本体が `fle_stop` を呼ぶ形。実質は名前付き停止） |

- 上の「応答する」の中の `GetModuleHandleExW`、`GetStringTypeW`、`ExitProcess`、`TerminateProcess`、`RaiseException` は本体が `snprintf` と `fle_stop`、または `fprintf; abort` で、`b2d_impclass.py` のテキスト判定（本体が 1 文だけ）で NAMED-STOP にならなかったもの。**実質の名前付き停止は 19 + 5 = 24 名**（読み。`fle_modules.c` 140 行目、`fle_locale.c` 149 行目、`fle_proc.c` 211〜263 行目で確認）。
- dsp_ippv2 との差（`imports-class-dsp-ippv2.txt`）: 共通 68、Blend2D だけ 26、dsp_ippv2 だけ 4（`FlsAlloc`、`FlsFree`、`FlsGetValue`、`FlsSetValue`）。**Blend2D にだけある 26 名のうち行なしは 13**（SRWLock 4、条件変数 3、`CreateThread`、`ExitThread`、`FreeLibraryAndExitThread`、`GetFileSizeEx`、`GetLargePageMinimum`、`GetModuleHandleA`、`OutputDebugStringA`、`WaitForSingleObjectEx`）。dsp_ippv2 の行なし 2 名（`IsProcessorFeaturePresent`、`TlsFree`）は共通。
- **ATTACH から静的に届く行なしの名前は 15 名 59 か所**（`iat-in-attach-direct.txt`。直接 call だけの集合でも同じ 15 名 59 か所）: `AcquireSRWLockExclusive` 12、`ReleaseSRWLockExclusive` 18、`AcquireSRWLockShared` 3、`ReleaseSRWLockShared` 6、`WakeConditionVariable` 3、`SleepConditionVariableSRW` 2、`CreateThread` 1、`ExitThread` 2、`FreeLibraryAndExitThread` 1、`GetFileSizeEx` 1、`GetModuleHandleA` 1、`IsProcessorFeaturePresent` 4、`OutputDebugStringA` 2、`TlsFree` 2、`WaitForSingleObjectEx` 1。**ATTACH の直線上で確実に実行される読みのもの**: `GetModuleHandleA`（0x180199fed。`.xc` の初期化子の中。上の §5.2）。**SRWLock と条件変数は `0x18019c4d0` 以下（vtable `0x1802f4750` の関数）にあり、ATTACH の直線上かは読み切っていない**（未確認）。
- ATTACH の直接到達集合（845 関数）で**実質の名前付き停止になる API**: `GetModuleHandleExW`（2 関数）、`GetStringTypeW`（1）、`HeapSize`（1）、`HeapReAlloc`（1）、`IsDebuggerPresent`・`RtlCaptureContext` ほか致命エラー経路（4 関数）、`GetConsoleMode`、`GetConsoleOutputCP`、`WriteConsoleW`、`FindFirstFileExW`、`FlushFileBuffers`、`SetFilePointerEx`、`ExitProcess`、`TerminateProcess`、`RaiseException`。ほとんどは dsp_ippv2 のときと同じ CRT のエラー経路（決定 0078 の表）で、通常は呼ばれない。
- **ロケールの初期化**: `GetStringTypeW`（`0x1801adfa6`、`f_1801ade74`）は `f_1801aae6c` と `f_1801abb04` の下で、`f_1801aae6c` の祖先に `f_18019e584`（エントリ）、`f_1801af6e4`（`.xi` の 1 項目）、`.data` の関数がある（`b2d-locale-anc.txt`）。`MultiByteToWideChar` は `0x1801ab707` の `jmp [IAT]` 1 か所。**決定 0078 §8 が dsp_ippv2 で「ATTACH は完走しない見込み」とした理由（`MultiByteToWideChar` の 256 バイト表で 0x80 以上のバイトが停止、`GetStringTypeW` が停止）と同じ形**（読み。`__acrt_initialize_locale` 系）。**`MultiByteToWideChar` の 0x80 以上のバイトは現在別の調査（`lift/out-mbtwc/`、`docs/investigations/mbtwc-high-bytes.md`）が扱っており、この調査は触らない**。ATTACH の到達する `GetACP` と `GetCPInfo`、`IsValidCodePage`、`LCMapStringW` は応答済みの名前。
- **ATTACH は完走しない見込み**（読み。dsp_ippv2 と同じ理由が 2 つ以上: `GetModuleHandleA` の `fle_unsupported`、ロケールの `GetStringTypeW`。**実行して確かめていない**）。

## 8. 置き方、リンク、メモリ（項目 3 の続き。実測と読み）

- **リベース**: `fe-pe rebase Blend2D.dll 0x4700000 OUT`（決定 0080 のツール）が通った。DIR64 1,763、ABSOLUTE 5、ブロック 18。**往復（0x180000000 → 0x4700000 → 0x180000000）で元の抽出ファイルと全バイト一致**（`cmp`。`rebase-4700000.txt`）。リベース後の DLL で `fe-lift lift --pdata` の統計は `c_bytes` を除き**同一**（関数 2,018、命令 398,879、`unsupported_total` 662、`bad_nodes` 0。`c_bytes` は 36,418,770 → 34,691,060）。リベース後の生成 C に `UINT64_C(0x18…)` 形の即値は 0 か所（リベース前は 19,108 か所）。**値 0x4700000 は例**で、決定 0080 D1 の 0x2000000 は dsp_ippv2 の置き場（終端 0x461f000）。Blend2D の終端は 0x4a00000（`0x4700000 + 0x300000`）で、0x2000000 に置けば終端 0x2300000。2 つのモジュールをどこに置くかは**決めていない**（B7）。
- **メモリ**: SizeOfImage は 3 MiB で、`.data` のファイル外（未初期化）が 1.01 MiB。FLEngine の終端（0x1cc0000）から `GLOBAL_BASE`（0x10000000）までの空きは 227.25 MiB。dsp_ippv2（38.1 MiB）と合わせても 41.1 MiB で入る（読み。2 つの実在モジュールが同時に置けるかは**未検証**）。
- **ビルド側の機構**: `lift/harness/two/gen_two.sh` は REAL を 1 つ取り（`REAL=`）、`realmod_table.c` は 1 行の表。**runtime の `fle_realmod.c` は最大 4 モジュール**（`RM_MAX 4`。ただし決定 0082 の限界の欄は「複数の実在モジュール（`RM_MAX` 4）は未確認」と書く）。3 つ目のモジュールを足す作りはあるが、**ビルドスクリプト（`gen_two.sh`、`build_two_*.sh`）は 2 モジュール目を 1 つしか扱わない**（読み）。`R[16]`、`FL`、`X[16]`、`fle_dispatch_table` の重複定義の解消（`merge_tables.py`）を、さらに 1 つ足したときに通るか**は未確認**（`two-module-design.md` §4。「(b) の方が将来の 3 つ目のモジュールに素直」と書く）。
- **生成 C の `emcc -O0` コンパイル**（実測。`obj/`）: `--pdata` 版が 17.9 s（最大 RSS 3.3 GB）で通り、`.o` 35.8 MB（`wasm-locals.txt`）。FLEngine は 16 TU に分け、1 TU が最大 1.8 GB（`wasm-port-run`）。Blend2D は **1 TU のまま 3.3 GB**（分割していない。`split_c.py` の 3,700 関数ずつなら 1 TU で足りる）。**リンクと node での実行はしていない**。
- **`GLOBAL_BASE` と `FLE_CHECKED_MEM`**: `fle_mem_set_image` は範囲が 1 つ（決定 0080、dsp_ippv2 調査 §2.1）。3 つ目のイメージを許すなら 3 範囲化が要る（読み。dsp_ippv2 の 2 範囲化がまだ入っていないなら別の作業）。

## 9. 作ったファイル（`lift/out-blend2d/`。既存のファイルは変更していない）

- ビルド: `target/`（fe-pe、fe-decode、fe-lift の release。`CARGO_TARGET_DIR`）、`target-enc/`（`ipp-encscan` を、`lift/out-ipp/encscan` のソースのコピー `encscan/` から再ビルド）、`emcache/`、`obj/`（`emcc -c` の `.o` とログ。`.o` は `--pdata`、`-finstrument-functions` 版、reloc 版など）、`tmp/`（空。`TMPDIR`）、`cargo-build*.log`。
- 抽出した DLL（**1 ファイルだけ**）: `Blend2D.dll`（2,065,920 B、SHA-256 `66634d24…66c7`）。測定用のコピー（**実在の DLL ではない**）: `Blend2D-rebased-4700000.dll`、`Blend2D-roundtrip.dll`（往復の確認。元と同一）。
- 測定の出力: `nsis-*.txt`、`hash-nsis.txt`、`pe-*.txt`、`objdump-p.txt`、`closure-pdata.txt`、`lift-*.txt`（`fe-lift` の統計。`lift-pdata`、`lift-reloc`、`lift-init`、`lift-pdata-nf`、`lift-reloc-all`、`lift-rebased-4700000`）、`encscan.txt`、`reloc-seeds.txt`、`reloc-not-function.txt`、`seeds-initterm.txt`、`imports-scan.txt`、`imports-class*.txt`、`sites-*.txt`、`jmpreg.txt`、`uncovered.txt`、`wasm-locals.txt`、`big-funcs.txt`、`reach*.txt`、`reached*.txt`、`iat-in-*.txt`、`fl-*.txt`（FLEngine 側の逆アセンブルと xref）、`b2d-*.txt`（Blend2D 側の逆アセンブル）、`dsp-ref/`（dsp_ippv2 の測り直し）。
- 生成 C（`.gitignore` の `lift/out*/` に入る。合計 約 150 MB。コンパイルは `emcc -c` まで）: `blend2d-pdata.c`（36.4 MB）、`blend2d-pdata-nf.c`、`blend2d-reloc.c`、`blend2d-reloc-all.c`、`blend2d-init.c`、`blend2d-rebased-4700000.c`。
- 補助スクリプト（読み取り専用）: `b2d_peinfo.py`、`b2d_dump.py`、`b2d_xref.py`、`b2d_cells.py`、`b2d_sites.py`、`b2d_reach.py`（FLEngine の unit への到達）、`b2d_path.py`、`b2d_reach_dll.py`、`b2d_reach2.py`、`b2d_reach3.py`（生成 C ベースの到達。**この文書の到達の数は reach3 のもの**。reach / reach2 は途中の版で、reach2 は `.pdata` に無い葉関数を落とすため小さく出る）、`b2d_ancestors.py`、`b2d_calls_in.py`、`b2d_gpa_names.py`、`b2d_impclass.py`、`b2d_iat_in_set.py`、`b2d_unsup_reach.py`、`b2d_jmpreg.py`、`b2d_uncovered.py`、`b2d_vt_dispatch.py`、`b2d_vtuse.py`、`b2d_initterm.py`、`b2d_tlsdump.py`、`b2d_strings.py`、`b2d_nsis_ctx.py`、`encscan/`（Rust）。`b2d_reach.py`、`b2d_path.py` は `lift/out-loadlib/` の `units.tsv` などを読み取りで参照する。
- **文書内の訂正**: 途中版（`b2d_reach_dll.py`、`b2d_reach2.py` の出力 `reach-*.txt`、`reach2-*.txt`）の関数数は `.pdata` に無い葉関数を落とした値なので、この文書では使っていない（reach3 に置き換え。`reach3-*.txt`、`reach3b-*.txt`、`reach3c-*.txt` が根拠）。

## 10. 結論（事実の整理）。決めない

### 10.1 dsp_ippv2 との比較表

| 項目 | Blend2D.dll | dsp_ippv2_x64.dll | 区分 |
|---|---|---|---|
| ファイル / SizeOfImage | 2.07 MB / 3.0 MiB | 39.9 MB / 38.1 MiB | 実測 |
| 実行セクション | `.text` 1 つ（決定 0074 の複数範囲は不要） | `.text` + `IPPCODE` | 実測 |
| export | 1（`AggCreateInstance`） | 446 | 実測 |
| import | KERNEL32 94 名 | KERNEL32 72 名 | 実測 |
| 関数 / 命令 | 2,018 / 398,879 | 32,648 / 6,464,799 | 実測 |
| 生成 C | 36.4 MB | 566.3 MB | 実測 |
| `unsupported_total`（割合） | 662（0.17%） | 974,527（15.0%） | 実測 |
| VEX / EVEX / ZMM / opmask | 497 / 0 / 0 / 0 | 835,902 / 219,112 / 139,583 / 28,744 | 実測（`encscan`。dsp は `.text` 745,714 + `IPPCODE` 90,188。決定 0079 後の閉包 32,648 関数。調査文書 §3.2 の 834,972 / 218,819 は閉包が 32,162 関数だった時点の値） |
| CPUID 振り分け | 表引きなし。変数を見る分岐 5 関数 + AVX2 の 3 関数 | スタブ 434 個（表 6〜9 エントリ） | 実測と読み |
| `cpuid` / `xgetbv` | 12 / 3 | 259 / 7 | 実測 |
| TLS ディレクトリ | **あり**（暗黙の TLS、`gs:[0x58]` 29 か所 13 関数） | なし | 実測 |
| スレッド作成 API | **あり**（`CreateThread` 他） | なし | 実測 |
| JIT（実行可能メモリに機械語を生成） | **あり**（AsmJit。実行されるかは未確認） | なし | 実測（import と RTTI）と読み |
| C++ の静的初期化（`.xc`） | あり（1 項目が `GetModuleHandleA` を呼ぶ） | 空 | 実測と読み |
| 行なしの import | 17 名（ATTACH で届く 15 名 59 か所） | 2 名 | 実測 |
| wasm の locals 50,000 超 | 4 関数（最大 102,086。描画の実体に読める） | 13〜15 関数（最大 389,948） | 実測（近似）と読み |
| 閉包に入らない命令（`jmp reg` の switch 表の中） | 約 8%（35,082 / 430,451。95 か所の switch） | 未測定（434 スタブは決定 0079 で解決済み） | 実測 |
| FLEngine の使い方 | `LoadLibraryW` → `GetProcAddress` 1 名。保存先を読む関数 1 つ（`f_6cb0b0`） | `GetProcAddress` 420 名、保存先を読む関数 424 | 実測 |
| ゲストの NULL 許容 | 許容（分類文書 §2.2 #4 と再確認一致）。ただし使う側（`f_6cb0b0`）は NULL 検査なし | 許容（戻り値を使わない）。ただし使う側は検査なし | 読み |
| 起動の範囲での使用 | LoadLibraryW は unit 149、190。AggCreateInstance は unit 614 以降（描画の最初の段） | LoadLibraryW は unit 144。IPP の関数は後 | 実測（ロード）と読み（使用） |
| 2 モジュール機構（決定 0073〜0083）との関係 | 同じツールチェーンで lift できる。**3 つ目**のモジュール（ビルドは 2 つ目を 1 つしか取らない） | 2 つ目 | 読み |

### 10.2 事実として言えること（要点）

1. インストーラに `Blend2D.dll` は 1 件だけある（2,065,920 B）。Windows の DLL ではない（分類文書 §3 と整合）。
2. 既存の fe-pe / fe-decode / fe-lift は**変更なしで**通る（誤デコード 0、`bad_nodes` 0、0.44 s）。生成 C は 36.4 MB で、`emcc -O0` で `.o` になる（17.9 s、3.3 GB）。
3. 未対応命令は 662 箇所（0.17%）。大半（495）は **FMA を含む VEX の `sd` 系**で、CPUID を現行のままにすれば**実行されない見込み**（読み）。AVX-512 は無い。
4. FLEngine が引くのは `AggCreateInstance` の 1 名だけ。**export に実在する**。NULL のときの分岐は分類文書の読みと一致するが、**使う側（`f_6cb0b0`、呼び出し元 91 か所）は NULL 検査をしない**。
5. ロードは起動の unit 149 で実行される（実測）。使用は静的には unit 614 以降（描画の最初の段）に見える（読み）。
6. 現行 runtime が答えられない import は 17 名（行なし）+ 実質の名前付き停止 24 名。**ATTACH で届く行なしは 15 名**。dsp_ippv2 より多い（SRWLock、条件変数、スレッド、`GetModuleHandleA` が新しい）。
7. ATTACH は**完走しない見込み**（読み）。理由は少なくとも 3 つ: `.xc` の初期化子の `GetModuleHandleA`（行なし）、ロケールの `GetStringTypeW` と `MultiByteToWideChar`、TLS の `_tls_index` の未設定（B6）。
8. dsp_ippv2 のときに最大だった AVX / AVX-512 の問題（90 万命令）は**ここには無い**。代わりに、**JIT（AsmJit）、スレッドプール、暗黙の TLS、`jmp reg` の switch 表（閉包の 8%）、`.pdata` に無い葉関数（58 個）**が新しい。

### 10.3 未決定の論点（決めない）

| # | 論点 | 材料 |
|---|---|---|
| B1 | **JIT（AsmJit）をどうするか。** `VirtualAlloc` で実行可能メモリを取って機械語を書き、それを呼ぶ形。wasm では実行できない。JIT を無効にする経路（Blend2D の `BL_CONTEXT_CREATE_FLAG_DISABLE_JIT`）を `Agg2D` ラッパーが選べるか、選ぶ呼び出しがあるかは未確認 | §4.3。`flAllocationType` の `0x20003000`（`MEM_LARGE_PAGES`）は runtime が停止にする。JIT に届く経路は ATTACH の `.xc` の vtable 経由と読めるが、実行は未確認 |
| B2 | **描画の実体 4 関数（locals 約 10 万）の扱い。** 縮小（生成から外す）なら描画は `fle_dispatch_miss`。全部入りなら wasm が `local count too large`。先例の選択肢（決定 0080 D7 の注: 縮小、生成器の一時変数の再利用、関数の分割、`-O1`）がそのまま当てはまる | §3.6。`-O1` の測定は**していない** |
| B3 | **`jmp reg` の switch 表（95 か所。閉包の 8% が生成 C に入らない）。** 決定 0036 の注釈つき jump table に当たる。生成器の変更か、注釈の追加か、到達しない前提で停止のままにするか | §3.4。`fle_dispatch_miss` で停止する。FLEngine の閉包では 3 か所だけで、dsp_ippv2 は表引きだった |
| B4 | **スレッド。** `CreateThread`、`ExitThread` の import があり、DllMain が `DLL_THREAD_*` を受ける形。決定 0080 D9（現行のまま）と整合するか。Blend2D は `thread_count` が 0 のとき単一スレッドと読めるが、FL Studio が何を渡すかは未確認 | §6 |
| B5 | **`.pdata` に無い葉関数 58 個（vtable の 34 個を含む）を種にするか。** 決定 0074 が「採らない」とした「開始でない値を seed にする」に当たる。入れないと Agg2D の vtable の 26 エントリが生成 C に無く、呼ばれると `fle_dispatch_miss` | §3.2、§5.1。58 個を足した閉包は関数 2,066、`bad_nodes` 0。入口として正しいかの検査は、命令が `ret` で終わる短い列であることの確認までで、**入口かどうかの確認ではない** |
| B6 | **暗黙の TLS（TLS ディレクトリ）をモデル化するか。** `_tls_index` を誰が書くか、テンプレート 32 B をどこに置くか、`fle_tls_values[0]` と `TlsAlloc` の 0 番の衝突。FLEngine にも dsp_ippv2 にも無かった問題 | §6。決定 0032 の `gs:[0x58]` は `TlsAlloc` の表を指すだけで、暗黙の TLS を決めたものではない。**新しい決定が要る** |
| B7 | **置き場所。** 2 つの実在モジュールの基底（決定 0080 D1 は 0x2000000 を 1 つ目に決めた）。ビルドスクリプトの 2 モジュール目を 1 つしか取らない作り。`fle_mem_set_image` の 3 範囲化 | §8 |
| B8 | **行なしの 17 名**（SRWLock 4、条件変数 3、`CreateThread` 他 3、`GetModuleHandleA`、`GetFileSizeEx`、`GetLargePageMinimum`、`OutputDebugStringA`、`WaitForSingleObjectEx`、`IsProcessorFeaturePresent`（決定 0078 で行は作らず runtime 関数だけ。テストの期待値の更新が要る）、`TlsFree`（同））。Learn の仕様確認は 1 名ごとに要る。**`GetModuleHandleA` が ATTACH の `.xc` の中で確実に呼ばれる** | §6、§7 |
| B9 | **ロケールの初期化**（`GetStringTypeW`、`MultiByteToWideChar` の 0x80 以上）。決定 0078 §8 が dsp_ippv2 で止まると書いた形と同じ。**別調査（`lift/out-mbtwc/`）が扱っているので、この調査では触らない** | §7 |
| B10 | **使う側が NULL を検査しない。** `f_6cb0b0` は `AggCreateInstance` のスロットが 0 のとき 0 番地を呼ぶ。呼び出し元 77 関数のうち、描画の前に `f_664190` の戻り値（false）を見て止めるものがあるかは未確認。**NULL のままにする案（停止のまま、または仮想モジュール・absent にしない）の場合、描画に届いたところで停止する**（読み） | §4.2 |
| B11 | **CPUID を現行のままにするか**（決定 0080 D8）。現行のままなら FMA 版の枝（VEX 5 関数）は入らない見込み（読み）。`xgetbv` の前提（`bt edi,0x1b`、`0x1c`）で飛ばされる読み。**実行で確かめていない** | §3.5 |
| B12 | **`GetModuleFileNameW` の仮想パス（決定 0082 D6）。** Blend2D の `GetModuleFileNameW` は `hModule` = NULL（`xor ecx,ecx`。`0x1801a5099`）で、決定 0082 D6 の「FLEngine 側の既存の応答（決定 0018）の範囲」に当たる（読み）。自身のハンドルを渡す呼び出しは**見つからなかった** | §6 |
| B13 | **停止のまま（選択肢 (c)）を選ぶ場合の影響。** ゲストは NULL を許容するので、(b) absent や NULL の仮の値で通すと**実機に存在する DLL を「無い」と言う**ことになる（分類文書 §7.1）。(c) は fail-closed を保つが、unit 149 の後の全体の観測が先へ進まない（Blend2D の次の停止は unit 153 の `RegCreateKeyExW`） | 分類文書 §2.2 #4、§6.2、§7.1。stage23 §5.2 |

### 10.4 確認できていないこと

- DLL を実行していない。選択値・変数（`0x1801f2168`、`0x1801f0008`）が 0 のままか、`xgetbv` に入るか。
- 生成 C のリンクと wasm の実行（node の `local count too large` の受理確認を含む）、リンク後の wasm の大きさ。
- ATTACH の実際の呼び出し列（静的な到達と実行の差）。どの vtable 関数が ATTACH の間に呼ばれるか。
- JIT が走る経路の有無、`thread_count` の値。ラッパー `Agg2D` が引数で何を選ぶか。
- 4 つの巨大関数の機能（ラスタライザの塗りつぶしに見えるが読み切っていない）。
- ライブラリの版（Blend2D と AsmJit の版は文字列から見つからなかった。RTTI の `_abi_1_13` は AsmJit の ABI 番号）。
- `f_6cb0b0` の呼び出し元 77 関数が `f_664190` の false を見て止まる形になっているか。
- `OutputDebugStringA` の 2 か所（`0x1800aacca`、`0x18019b1b5`）が実行されるか。前者は文字列バッファ（SSO の `cmp byte [rsp+0x58],0x1f; cmovae`）を渡す形でログ出力に見える。後者は `f_18019b170`（`or rcx,2` の後。asmjit のエラー表示に見える）。どちらも通常の描画で呼ばれるかは未読。

（読んだもの）`IsProcessorFeaturePresent` の呼び出し点は IAT 参照 5 か所のうち、読めた 4 か所（`0x18019e5f0`、`0x1801a3c0e`、`0x1801a6245`、`0x1801b163a`）がすべて `mov ecx,0x17`（PF_FASTFAIL_AVAILABLE）。dsp_ippv2（決定 0078）と同じ形で、エラー経路の中。残る 1 か所は引数を読んでいない。`0x1801a3c0e` は `test byte [0x1801f0040],2` の枝の中（致命エラー経路かは未確認）。
- reloc の開始でない 58 個が全部**関数の入口**であること（vtable の 34 個は入口と読めるが、残りの 24 個は未確認）。
- `lift/runtime` の `fle_modules.c` の `GetProcAddress` が実在モジュールの export（1 名）を返す形が、仮想モジュールの表（`fle_resp_table`）と衝突しないか（決定 0082 は 446 名の dsp_ippv2 で測った）。

### 10.5 推奨（意見。事実ではない。決めない）

- **段階を分ける**。(1) 「ロードして `AggCreateInstance` を引き、`NULL` でない関数ポインタを返す」までと、(2) 「Agg2D の vtable の関数を呼ぶ」以降を別の作業にする。(1) は dsp_ippv2 の W1〜W7 に近い量だが、**TLS（B6）、`GetModuleHandleA` と SRWLock など行なし 15 名（B8）、ロケール（B9）**が新しく要る。(2) は JIT（B1）と locals（B2）と switch 表（B3）の決定が先に要る。
- (1) を目的にするなら、**縮小（生成を ATTACH の到達集合と export から始める）**の流儀（決定 0080 D7）が使える。init 表 5 項目と export だけなら関数 241、命令 12,808、locals 最大 1,355（`lift-init.txt`、`wasm-locals.txt`）。ただし `.pdata` に無い葉関数（B5）が vtable の中にあるので、(2) に進むなら種の定義を足す必要がある。
- **(c) 停止のまま**を維持する間は、unit 149 の次の停止点（RegCreateKeyExW）の観測ができない。その代わり、実機の DLL について偽の主張をしない（決定 0073、0055 の基準）。(d) に進むなら、dsp_ippv2 の W 番号に対応する作業の見積もりを別に作ることを勧める（この文書は見積もりを作っていない）。
- この調査の数字のうち**影響が大きい読み**は、(a) 描画の実体 4 関数が **ATTACH の集合から直接 call で届く**こと（`reach3c-attach-direct.txt`。`.xc` の初期化子が Agg2D の関数ポインタを大量に `lea` で取るため。実行時に呼ばれるかは別）。ただし**生成器の閉包と reach3 の数え方は違う**（実測。`blend2d-init.c`、`blend2d-reloc.c` を grep）: `fe-lift --seeds`（init 表 5 項目だけ。関数 241）の閉包には `f_180199d90`、`f_180147870`、`f_180147cf0` は**入っている**が、4 関数は**入っていない**。生成器は直接 call だけで閉包を作り、`lea` で取った関数ポインタは種にしない（`--pdata` なら全部入る）。reach3 は生成 C の即値（`lea` 由来）も辿るので 4 関数に届く。つまり **ATTACH の初期化子は 4 関数のアドレスを `.data` の表に書くが、縮小（init 表 5 項目だけ）の生成物には 4 関数の本体が無く、`fle_dispatch` の表にも無い**（読み）。ATTACH 自体は通っても、あとで描画が vtable 経由で 4 関数を呼ぶと `fle_dispatch_miss` で止まる。縮小で locals の上限を避ける案は、この「後で止まる」を引き受けることになる（B2 と B5 の相互作用）、(b) ATTACH が完走しない理由が 3 つあること、(c) `_tls_index` の未設定。これらは実行で確かめる価値がある。**未確認のまま採用しないこと**。
