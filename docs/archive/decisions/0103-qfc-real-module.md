# 0103: quickfontcache_x64.dll を実在モジュール（測定用基底 0x1d00000）として載せ、unit 158 の LoadLibraryW → QFC の DllMain(ATTACH) を通すために当たった壁を連続して直した記録（仮の決定）

状態: **仮の決定**（作業者（サブエージェント）の記録。PM の決定ではない。応答の値に実機で未確認のものを含む。撤回条件は壁ごとの表）。骨子を実装の前に保存し（`0103-qfc-real-module.skeleton-edited.md`）、実装・測定の後にこの文書にした。書式は 0088、0094 に倣う。作業中にディスクが満杯になり（他の作業と共有。下の「限界」）、全体テストは途中で `No space left on device` になったが、空きが戻ったあとの 3 回目で完走した。

**統合時の調整（merge1。この版が統合後の木に合う）**: 壁 Q は、壁 I（決定 0104）、R（0098）、S（0099）、B（決定 0102）の**後**に統合した（順序 I → R → S → B → Q）。B が同じ壁（GetVersionExA ほか CRT の API、EncodePointer、HeapSize、`ret 0`、SPI 0x200C、memset の inline 表）を別実装で直していたので、**重複は B（と、実在モジュールの `GetModuleFileNameW`／資源は I）の実装を採用**し、この文書の壁ごとの表の「決定（仮）と修正」の列に、取り込んだか取り込まなかったかを書き足した（下の一覧と、表の各行の先頭の【…】）。値と実装が採用されなかった項目の**判断（理由、撤回条件）はこの文書の元の記述のまま残してある**（実装は統合後の木に無い）。

| 壁 | 0103 の項目 | 統合 | 採用された実装 |
|---|---|---|---|
| 0 | D1 `GetModuleFileNameW/A` の実在モジュールのパス | **取り込まない** | 決定 0104（I）の値 `C:\Program Files\Image-Line\FL Studio 21\Shared\` + 像の export directory の `Name`、構造は B（`module_path()`、W と A） |
| 1 | D2 資源（`LoadStringW`／`FindResourceW`／`LockResource` ほか）の実在モジュール対応 | **取り込まない**（実装）／テストは残した | 決定 0104（I）の `fle_res.c`（同じ設計。`LockResource` は表の全実在モジュールを探す）。0103 の `wall_q.rs` の資源のテスト 2 件は、この実装に対して残した |
| 2 | D3 memset の inline 表（`recognize_abs`、`Origin::AbsRun`） | **取り込まない** | 決定 0102 W3（B）の `recognize_inline`／`resolve_inline`／`verify_inline`（`Origin::Inline`）。0103 のテスト（abs_run.rs 6 件、wall_q_lift.rs の switch）は B の仕組み向けに書き換えて残した（`inline_run.rs`、`wall_q_lift.rs`） |
| 3、8 | D4 freetype を 4 つ目の実在モジュールに、種 160 個 | 測定構成（`a4-multi.sh` の引数）。変更なし | 種のファイルは B の `harness/seeds/ft-seeds-4b00000.txt` と並べ替えた内容が同一（160 行）。1 つにした |
| 4 | D5 `GetVersionExA`／`GetStartupInfoA`／`SetHandleCount`／`HeapCreate`／`HeapSetInformation`／`GetModuleFileNameA` | **取り込まない** | 決定 0102（B）W4〜W7、W10。応答表の行も B の（Q が足した 7 行は重複なので落とした）。B は先取りで `GetEnvironmentStrings`／`FreeEnvironmentStringsA` の行も足していて、残した（freetype の CRT が 1 か所と 2 か所から呼ぶ） |
| 5 | D6 `EncodePointer`／`DecodePointer`（秘密 `0x5be2a4c971d3086f`、rol 17） | **取り込まない** | 決定 0102 W8（B）の秘密 `0x5A3C96E1D2B478F0`、ror |
| 6 | D7 `ret imm16`（一般） | **取り込んだ** | `fe-lift/src/lib.rs`: 0103 の一般形（callee が imm16 を RSP に足す）に、B の `ret 0`（ハンドラ名 `ret`）を重ねた。`ret 0` と `ret` は同じ `return;` で、B の統計（ハンドラ `ret`）のまま。imm16 ≠ 0 だけハンドラ `ret_imm16` |
| 7 | D8 `HeapSize` | **取り込まない** | 決定 0102 W11（B）。ブロックのヒープも記録する点が違う（別のヒープのブロックは停止） |
| 9 | D9a SPI 0x200C = 1400 | B と同じ値。**取り込まない**（実装は B の） | 決定 0102 W13（B） |
| 10、11 | D9b SPI 0x200A = STANDARD (1)、D9c SPI 0x2012 = RGB (1) | **取り込んだ** | `fle_spi.c`／`.h`。**B の 0102 は 0x200A を「答えない」（止まる条件 (d)）にして終わっていた**: 和集合にする PM の方針で、0103 の仮の値を採用した。0x200A の値の選択は B の方針と食い違う（B: 機械の設定で既定がないので答えない、Q: SET の項の既定 STANDARD を答える） |
| 12 | D10 `SHGetFolderPathW(CSIDL_FONTS)` | **取り込んだ** | `fle_env.c`／`.h`、応答表の末尾に `shell32.dll!SHGetFolderPathW`（310 行目。擬似 VA `0x7FF900001360`） |
| — | 実在モジュールのアクセサ `fle_realmod_total`／`name`／`base` | **取り込んだ**（PM の方針） | `fle_realmod.c`／`.h`。ただし統合後の `fle_vfs.c`／`fle_res.c`（I と B）は表の記号を直接参照しないので、**この 3 関数を呼ぶ箇所は木に無い**（`fle_realmod_base(i)` は I の `fle_realmod_base_of(i)` から範囲検査を除いたもの） |

取り込んだテスト: `fe-decode/tests/inline_run.rs`（6 件。0103 の abs_run.rs を B の仕組み向けに。`unrelocated_first` は B の方針で「認識せず、報告もしない」に変えた）、`fe-lift/tests/wall_q_lift.rs`（3 件。ハンドラ名と行の一覧を直した）、`fe-lift/tests/wall_q.rs`（12 件のうち 5 件: 他のハンドルは経路が無い、資源 2 件、SPI の 3 値、`SHGetFolderPathW`。**残りの 7 件（`GetModuleFileNameW/A` の実在モジュール、`GetVersionExA`、`GetStartupInfoA`／`SetHandleCount`、`HeapCreate` 系 3 件、`EncodePointer`／`DecodePointer`）は B の `wall_b.rs` が同じ API を B の実装で検査しているので取り込まない**）。更新した既存テスト: `spi.rs` の `fle_st(` の数（統合後 **12**: 9（R）+ 1（B の 0x200C）+ 2（0103 の 0x200A、0x2012））、`wall_b.rs` の 2 件（`ret 8` が lift されるようになった、0x200A が答えになった）。`k32_dsp.rs` の `ANSWERED` に `DecodePointer` を足す 0103 の変更は**取り込まない**: dsp_ippv2_x64.dll は `DecodePointer` を import しないので、実 DLL がある環境では `real_dll_leaves_only_the_two_unrowed_names` が「call_imp も jmp_imp の箇所も無い」で落ちる（0103 を作った木には `lift/out-ext/` が無く、そのテストは飛ばされていた）。

この文書の他の数値（a4 の functions／entries、FLEngine の SHA-256、全体テストの数）は **Q 単独の木**（census も遅延モジュールも I、R、S、B も入っていない土台）での実測で、統合後の木では測り直す。FLEngine の生成 C は、各レーンの文書が書く IAT thunk の変化（B: 0x423910 `HeapCreate`、S: 0x593410 `ILFree`、Q: 0x593450 `SHGetFolderPathW`、R: `unsupported_total` 525 → 519 の分）が重なって変わる。SHA-256 と `unsupported_total` は統合後に測り直す。


> **TLS（暗黙 TLS）: 選んでいない。** QFC（`quickfontcache_x64.dll`）も freetype（`freetype_x64.dll`）も TLS ディレクトリを持たない（データディレクトリ 9 のサイズが 0。pefile で確認）。暗黙 TLS の設計（`implicit-tls-design.md` の A〜D）はこの作業で必要にならなかった。レーン B の Blend2D（暗黙 TLS あり）とは別の問題。

> **これは A4 の達成ではない。init_count は 159 / 942 のまま動いていない。** 基底（dsp_ippv2 0x2000000、Blend2D 0x4700000、QFC 0x1d00000、freetype 0x4b00000）と種（dsp 452、Blend2D 58、freetype 160）は測定用の値で決定ではなく、応答に仮の決定を含む。QFC を載せた構成は DllMain の途中でレーン S の壁（`VirtualProtect`）まで進んで止まった。停止点が進んだことは進捗ではない。

## 事実の出典（読んだもの）

`docs/investigations/quickfontcache-lift-feasibility.md`（QFC の PE・lift・unit・FLEngine 側の使い方）、決定 0015、0018、0025、0036、0045、0058、0066、0073、0074、0077、0078、0079、0080、0082（D6）、0083、0086（項目 8: QFC 自身が読む DLL は未決）、0088、0089〜0095、0098、0099（レーン S の W4）、0101、`lift/out-harness-int/docs-new/multi-module-harness.md`。Microsoft Learn（2026-10-27 に読んだ）: SetHandleCount、HeapSetInformation、HeapCreate、GetEnvironmentStrings、FreeEnvironmentStringsA、GetVersionExA、EncodePointer（アーカイブ）、SystemParametersInfoW、SHGetFolderPathW、CSIDL。mingw-w64 のヘッダ（第三者）: `shlobj.h`（CSIDL_FONTS 0x14）、`winuser.h`（FE_FONTSMOOTHING*）。

## 土台（実測）

- 作業木 `lift/out-wallQ/src/`: HEAD `0e9b703` の `git archive` に、作業ツリーの 0095 差分（`git diff HEAD -- lift`、SHA-256 `6c9204c6…`）とハーネス取り込み `lift/out-harness-int/patches/harness-int.diff` を当てたもの（`base/`）の複製。PM が 0095 とハーネスをコミットした後の木が基準になるので、`patches/wallQ.diff` はそれに当たる形（`diff -u -r -N -x 'out-*' -x target base src`、25 ファイル。`patch -p1 --dry-run` が `base/` の複製で通る。コミット後の HEAD への当て直しは未実施）。
- 入力の SHA-256: FLEngine `b2c64da4…9496d`、QFC `afc5a0ed70eb7e4af3d0f4baf771bd6f53eb014e237f857f23023471047736d2`、dsp_ippv2 `497dd4f7…c352`、Blend2D `66634d24…066c7`、**freetype_x64.dll `6ea6c106ad2695b5ec75738a0c790bba81dca7866d63254c2c69602973837cb8`**（607,032 B。インストーラ `flstudio_win64_26.1.7.5653.exe` から `tools/extract_nsis2.py --write` で取り出した。読み取りのみ）。
- 3 モジュール（dsp + Blend2D）の再現（0095 込み）: **159 / 942**、last unit 158（proc 0x66d1a0）、functions 1,106、entries 27,842、停止 `LoadLibraryW "quickfontcache_x64.dll"`（`not a virtual module and not a name this environment is defined to lack`）。
- QFC を `fe-pe rebase` で 0x1d00000 に載せて lift: `--pdata` で functions 6,712、closure_instructions 209,259、`bad_nodes 0`、unsupported_total 104（調査の値と一致）。種なし（entry と export だけ）は functions 547、unsupported 12。非 pdata の reloc 値 9,293 は種にしない（調査: `bad_nodes` が 17,541）。
- 実在モジュールは dsp、Blend2D、QFC、freetype の 4 つで `RM_MAX 4` ちょうど。5 つ目（iltools_x64.dll。unit 280、659）は載らない。

## 壁ごとの表

停止は `lift/out-wallQ/logs/` と `run-*/runs/*/a4/*-native.txt` の全文。「前→後」は **init_count / functions（distinct）/ entries**。**init_count は全行 159 のまま**。

| # | 停止（要約。C のスタック先頭） | 読み | 仕様 | 決定（仮）と修正 | 撤回条件 | テスト | 前→後 |
|---|---|---|---|---|---|---|---|
|  0 | QFC を載せた直後: `fle: GetModuleFileNameW: only the lifted DLL has a path (hModule is not NULL or 0x400000)`、`f_1d06e50` | QFC の Delphi RTL `f_415320`（`FindResourceHInstance`）が `GetModuleFileNameW(QFC のハンドル = 基底 0x1d00000, buf, 0x105)` を呼ぶ。決定 0082 D6 が「そこに実行が届いたら使い方を読んで別の決定で値を決める」とした地点。返ったパスは `f_416b70` が資源モジュール探索に使う | Learn: 完全なパス。nSize 超過は切り詰めて nSize を返し ERROR_INSUFFICIENT_BUFFER | **【統合で取り込まない: 実在モジュールのパスは決定 0104（I）の値 `...\Shared\` + export directory の `Name`。以下は 0103 作成時の値】** **D1**: 実在モジュールの基底に対し、仮想 DLL パス（決定 0018）のディレクトリ + **小文字の表の名前**（`C:\Program Files\Image-Line\FL Studio 21\quickfontcache_x64.dll`）。W と A。`fle_vfs.c` | PM が実在モジュールの仮想パスを決めたとき。大文字小文字と置き場所は**創作**（インストーラでは QFC は `Shared`。NSIS 変数は未確認） | `wall_q.rs`、`realmod.rs` の 1 件を更新 | 159 / 1,184 / 27,950 → 159 / 1,233 / 28,201  |
|  1 | `fle: LoadStringW: hModule 0x1d00000 is not a module handle of this runtime`、`f_1d06e30` | `f_418e80`（`LoadResString`）が QFC 自身の RT_STRING を読む。`fle_res.c` は 0x400000 のイメージしか読めない | Learn: LoadStringW。決定 0025 の規則 | **【統合で取り込まない: 決定 0104（I）の `fle_res.c` が同じ設計で実装済み。以下は 0103 作成時の記述】** **D2**: hModule が実在モジュールの基底なら、そのイメージの資源ディレクトリを読む（`res_img.base`）。FindResourceW／LoadResource／SizeofResource／LoadStringW。LockResource はデータ VA を 0x400000 のあとに表の実在モジュールで探す（ハーネスは表の全イメージを先に載せる）。`fle_res.c` | 資源を持つ実在モジュールの扱いを PM が決めたとき | `wall_q.rs`: FLEngine と `fe_pe::rebase` の複製（0x2000000）で「Set Size Exceeded.」（独立パーサーの値）、RT_VERSION 728 B、仮想モジュール／無関係な値の停止 | → 159 / 1,337 / 29,676  |
|  2 | `fle: unsupported instruction at 0x1d0c163`、`f_1d0c120`（QFC の 0x40c163） | memset の尾（`lea r10,[rip+5]; jmp [r10+rdx*8+0x40]`、表は jmp の直後に 9 スロット）。FLEngine の 0x40d253 と同形（そちらは決定 0036 の手書き注釈） | 命令の意味のみ。表は DIR64 の再配置が指すコード番地 | **【統合で取り込まない: 決定 0102 W3（B）の `recognize_inline`／`Origin::Inline` が同じ形を認識する。以下は 0103 作成時の記述】** **D3**: fe-decode に `recognize_abs`／`resolve_abs`／`verify_abs`（`Origin::AbsRun`）。`lea B,[rip+d]; jmp [B+I*8+disp]` で `rip+d` = jmp の終わり。表 = そこから続く「再配置された、同じ `.pdata` 関数内のコード」を指す 8 バイトスロットの連なり。`Select::Slot` の switch で表外は `fle_jumptab_miss`。**手書き注釈と 0079 のスタブが先**（FLEngine の C は変わらない）。dsp の 46 個の同形（表が離れた場所）は対象外にし、dsp の C を不変にした | 同形を別の場所へ広げるとき（認識条件の変更は別の決定） | `abs_run.rs` 6 件、`wall_q_lift.rs`（switch の C） | → 159 / 1,635 / 44,791  |
| 3 | `fle: LoadLibraryW: module "freetype_x64.dll" is not a virtual module …`、`f_1d1d910` | QFC の unit 129（`f_563000` ← `f_522d20`）が freetype を読む。失敗なら `f_563000` は偽で戻るが、unit 146 の下の `f_574920` は `FT_Init_FreeType` のスロットを検査せず呼ぶ。**実験**（`fle_load_absent[]` に足した作業コピー `exp-src`。決定ではなく、パッチに入れていない）: `indirect call to 0`。「無い」は成り立たない | 決定 0086（却下 (b)）、0073、QFC 調査 §6.1 | **D4**: `freetype_x64.dll` を 4 つ目の実在モジュールにする（インストーラの `Shared` にある）。名前 `freetype_x64.dll`、基底 **0x4b00000**（測定用）、`--pdata` + 種 160 個（壁 8）。`a4-multi.sh` の引数のみ | PM が決定 0086 の項目 8 を決めたとき | `a4-multi.sh` の実測 | → 159 / 1,639 / 44,795 |
|  4 | `unsupported instruction at 0x4b57daa`（`GetVersionExA` の `call [IAT]` に行なし）、`f_4b57d70` | freetype は MSVC の静的 CRT（2012 年）を持つ。DllMain の `_DllMainCRTStartup` が `GetVersionExA`、`HeapCreate`、`HeapSetInformation`、`GetStartupInfoA`、`SetHandleCount`、`GetModuleFileNameA` を呼ぶ。**gdb の呼び出し回数（読み取り）: 各 1 回**。ほかに用意した GetEnvironmentStrings、FreeEnvironmentStringsA、HeapDestroy は**到達せず、パッチから外した** | Learn: GetVersionExA（8.1 以降は未マニフェストなら 6.2 と書くが、決定 0014 の 10.0 を優先）、HeapCreate（max 0 = 伸びる）、HeapSetInformation（class 1 は NULL と 0。class 0 は ULONG 2）、SetHandleCount（Win32 では効果なし、**返す数は書かれていない**）、GetStartupInfo、GetModuleFileName | **【統合で取り込まない: 決定 0102（B）W4〜W7、W10 の実装と応答表の行を採用。以下は 0103 作成時の記述】** **D5**: 行 6 つ（GetVersionExA、GetModuleFileNameA、HeapCreate、HeapSetInformation、GetStartupInfoA、SetHandleCount）と実装。GetVersionExA は OSVERSIONINFO(EX)A のサイズ 148／156、他は 0 と 87。HeapCreate は max 0 と flags 0/1/4 だけ（トークン `0x7FF700010000 + 0x10000k`、最大 8）、実行可能・固定サイズは停止。HeapSetInformation は class 1（NULL, 0）と class 0（ULONG 2, 長さ 4）だけ。GetStartupInfoA は W と同じ（0x68 ゼロ + cb）。**SetHandleCount は uNumber をそのまま返す（創作）**。`fle_osver.c`、`fle_vfs.c`、`fle_env.c`、`fle_sys.c` | PM／実機が SetHandleCount の戻り値、HeapDestroy を決めたとき | `wall_q.rs` 6 件、`wall_q_lift.rs`（行の存在と宣言） | → 159 / 1,644 / 44,800  |
|  5 | `fle: EncodePointer: not answered (reached only by the C++ exception and terminate code of the lifted DLL)`、`f_4b5c740`（`_mtinit`） | 決定 0078 は dsp の CRT に対し名前付き停止にした。freetype の CRT は ATTACH の `_mtinit` で呼ぶ（gdb: Encode 17、Decode 16） | Learn（アーカイブ）: 「秘密で難読化する。秘密はプロセスごとに違う。使う前にデコードする」 | **【統合で取り込まない: 決定 0102 W8（B）の秘密 `0x5A3C96E1D2B478F0`・ror を採用。以下は 0103 作成時の記述】** **D6**: `Encode(p) = rol64(p ^ SECRET, 17)`、Decode は逆（**SECRET `0x5be2a4c971d3086f` は創作の固定値**。再現性を優先）。EncodePointer の名前付き停止を外し、DecodePointer に行 | PM が秘密の扱いを決めたとき | `wall_q.rs`（往復、0 と全ビット、値が隠れる、同入力同出力）、`k32_dsp.rs` の ANSWERED／STOPPED の一覧を更新 | → 159 / 1,651 / 44,807  |
|  6 | `unsupported instruction at 0x4b529c0`（`ret 0`）、`f_4b529c0` | `ret imm16`（MSVC は `ret 0` も出す）が fe-lift で未対応（freetype に 82 か所、Blend2D に 1 か所） | x86 の RET imm16: 戻り番地のあと imm16 バイト RSP を進める。lift した呼び出し側が戻り番地の 8 を足すので、被呼び出し側が imm16 を足す | **【統合で取り込んだ（`ret 0` のハンドラ名は B の `ret`、imm16 ≠ 0 は `ret_imm16`）】** **D7**: `fe-lift` の `Mnemonic::Ret` に `ret imm16`（`R[4] += imm; return;`、imm 0 は `return;`、ハンドラ名 `ret_imm16`）。FLEngine と dsp の C は変わらない、Blend2D は 1 か所（0x470c530） | 呼び出し規約の実機との照合 | `wall_q_lift.rs`（`ret 16`、`ret 0`、`ret 65535`） | → 159 / 1,696 / 45,179  |
|  7 | `fle: HeapSize: the requested size of a process-heap block is not recorded by HeapAlloc (not answered)`、`f_4b5a0c0` | 決定 0078 は HeapSize を名前付き停止にした（要求サイズが記録されない） | Learn: HeapSize は「確保したブロックの要求サイズ」。flags は 0 か HEAP_NO_SERIALIZE。失敗は (SIZE_T)-1 | **【統合で取り込まない: 決定 0102 W11（B）を採用】** **D8**: HeapAlloc が要求サイズをブロックの外のハッシュ表に記録し、HeapFree が落とす。HeapSize は記録を返す。**HeapAlloc が返さなかった番地（NULL を含む）は (SIZE_T)-1 ではなく停止**（LocalAlloc のブロックかもしれない）。`fle_sys.c` | HeapReAlloc を答えるとき | `wall_q.rs`（100／37／0、ゼロ初期化、flags、解放後、未知のヒープ） | → 159 / 1,751 / 45,869  |
| 8 | `fle: indirect call to 0x4b24820 is not a known function entry`（`ft_mem_qalloc` ← `FT_New_Library` ← `FT_Init_FreeType`） | `ft_alloc`／`ft_free`／`ft_realloc` は `.pdata` に無い葉関数で、`lea` で関数ポインタに入る（再配置を持たない） | 種の選び方（決定 0074: `.pdata` 開始に一致する reloc 値だけを種にする。ここは測定用） | **D4 の一部**: freetype の種 **160 個** = 再配置の「非 pdata の値」131（`fe-decode reloc-seeds --not-function`。Blend2D の 58 個の作り方）+ `.pdata` 関数の外を指す rip 相対 `lea` のコード番地 32（`tools/lea_code.py`。重複を除いて 160）。`bad_nodes 0` | 種の決定 | `a4-multi.sh` の実測 | → 159 / 1,792 / 46,167 |
|  9 | `SystemParametersInfoW: uiAction 0x200c is not answered`、QFC `f_57eeb0` | SPI_GETFONTSMOOTHINGCONTRAST | Learn: UINT。「1000〜2200。**既定値は 1400**」 | **【B の W13 と同じ値（1400）。B の実装を採用】** **D9a**: 1400。pvParam NULL／uiParam≠0／fWinIni≠0 は停止。`fle_spi.c` | PM が SPI の値を決めたとき（レーン R と同じ表） | `wall_q.rs` | → 159 / 1,806 / 76,755  |
|  10 | 同 `0x200a` | SPI_GETFONTSMOOTHINGTYPE | Learn: STANDARD か CLEARTYPE。GET の既定の記載なし。SET（0x200B）の項に「既定は STANDARD」 | **【統合で取り込んだ。B の 0102 はこの 0x200A を「答えない」で終えていた（PM の和集合の方針）】** **D9b（仮の値）**: STANDARD = 1（数は mingw-w64 の winuser.h。Learn の本文に数は無い）。実機の Windows 10 は CLEARTYPE（2）かもしれない（未確認） | 同上 | `wall_q.rs` | → 159 / 1,806 / 76,756  |
|  11 | 同 `0x2012` | SPI_GETFONTSMOOTHINGORIENTATION | Learn: BGR か RGB。**どちらが既定とも書かれていない** | **【統合で取り込んだ】** **D9c（仮の値。一次資料の既定が無い。止まる条件 (d) の種類だが続行した）**: RGB = 1（mingw-w64: BGR 0、RGB 1）。実機のユーザー設定は読んでいない | 同上。**最優先で見直す** | `wall_q.rs` | → 159 / 1,809 / 76,759  |
|  12 | `unsupported instruction at 0x1d52240`（`shell32!SHGetFolderPathW` の `jmp [IAT]` に行なし）、QFC `f_452240` | **PM が担当に指定した壁**。QFC の ATTACH の中の `f_57ed50` が `SHGetFolderPathW(NULL, 0x14, NULL, 0, buf)` | Learn SHGetFolderPathW: S_OK。末尾の `\` なし。**「実フォルダだけ有効。仮想フォルダは失敗」**。CSIDL: 「`CSIDL_FONTS`: 仮想フォルダ。典型は `C:\Windows\Fonts`」。**0x14 は shlobj.h の値**（mingw-w64 `shlobj.h:652`。Learn の CSIDL ページは数を載せない） | **【統合で取り込んだ】** **D10（仮の決定）**: csidl 0x14 だけ、hToken NULL、dwFlags 0／1 で **S_OK（0）と `C:\Windows\Fonts`**（決定 0018 の仮想 Windows ディレクトリ + `\Fonts`。ホストの値を見せない）。存在確認はしない（仮想ボリュームに system フォルダの実体はない。GetWindowsDirectoryW と同じ）。Learn の二つの記述の矛盾は PM の指示で settle。それ以外は停止。`fle_env.c` | PM が仮想 `C:\Windows\Fonts` の扱い（VFS に作るか、S_FALSE か）を決めたとき | `wall_q.rs`（パス、DEFAULT、他の csidl／フラグ／token／NULL の停止） | → 159 / **1,912 / 77,849**  |
| 13 | `fle: VirtualProtect: address is not in a VirtualAlloc region (the runtime does not track other memory)`、`f_1d1dae0` ← QFC `f_597c90` ← `f_597b10` ← unit 162 `f_5992d0` | **レーン S の壁（0099 W4: イメージに対する VirtualProtect、インラインフックの準備）と同じ型**。QFC の `f_597c90` は対象関数の先頭 5 バイトを保存し `0xe9 + rel32` を作り、`VirtualProtect(対象, 5, PAGE_EXECUTE_READWRITE, &old)` を呼ぶ（読み） | — | **止まる (e)**。修正しない | レーン S の決定 | — | 最終停止 |

停止は 14（0〜13）。13 個を直し、最後の 1 つはレーン S の壁で止まった。15 壁の上限には達していない。壁 4 の 6 行はいずれも gdb で 1 回以上の呼び出しを確認した。

## a4 の前後（実測。`run-5`、最終のソースで新規にビルド）

| 構成 | init_count | last unit | functions | entries | 停止 |
|---|---|---|---|---|---|
| 単独（FLEngine） | **145 / 942** | 144 | 821 | 15,685 | `LoadLibraryW dsp_ippv2_x64.dll`（不変） |
| 2 モジュール（+ dsp） | **150 / 942** | 149 | 943 | 26,585 | `LoadLibraryW blend2d.dll`（不変） |
| 3 モジュール（+ Blend2D） | **159 / 942** | 158 | 1,106 | 27,842 | `LoadLibraryW quickfontcache_x64.dll`（不変） |
| 5 モジュール（+ QFC + freetype） | **159 / 942（動かない）** | 158 | 1,912 | 77,849 | `VirtualProtect`（レーン S の壁） |

FLEngine の生成 C の SHA-256: 基準 `feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`、最終 `818a2555ae7132f46b83df0eb62f932a8ce7bba9262f7470fb90fa818e65fcbe`。**変わった理由は応答表の 8 行の追加と、FLEngine 自身の IAT thunk 2 か所（0x423910 の `HeapCreate`、0x593450 の `SHGetFolderPathW`）が `fle_unsupported` から runtime 呼び出しになったことだけ**（`diff` の 32 行をすべて確認。関数本体の変化は 0。`logs/fle-c.diff`）。dsp の C（`5ce10e3b…` → `f918def9…`）は応答表の行だけ（28 行）。Blend2D の C は応答表の行と 0x470c530 の `ret 0`（壁 6）だけ（30 行）。

DLL ごとの統計（最終）: FLEngine functions 59,297／unsupported 523（基準 525）／bad_nodes 0／msvc_tables 0、dsp_ippv2 functions 8,030／unsupported 391,396／bad_nodes 0／msvc_tables 4、Blend2D functions 2,067／unsupported 598（基準 599）／bad_nodes 0／msvc_tables 31、**QFC functions 6,712／unsupported 101／bad_nodes 0／msvc_tables 0／abs_table 1**、**freetype functions 1,625／unsupported 78／bad_nodes 0／msvc_tables 17**。

## テスト

（以下は 0103 を作った Q 単独の木の記述。統合後に何が残ったかは冒頭の「統合時の調整」。）

新規 21 件: `fe-decode/tests/abs_run.rs` 6、`fe-lift/tests/wall_q_lift.rs` 3、`fe-lift/tests/wall_q.rs` 12。固定数や期待値を決め打ちした既存テストで更新したもの（その行と理由）:

| ファイル | 変更 | 理由 |
|---|---|---|
| `fe-lift/tests/k32_dsp.rs` | `ANSWERED` 13 → 16（`EncodePointer`、`DecodePointer`、`HeapSize` を追加）、`STOPPED` 20 → 18（`EncodePointer`、`HeapSize` を除く） | 壁 5、7（名前付き停止だった 2 名が答える名前になり、DecodePointer に行が付いた） |
| `fe-lift/tests/realmod.rs` の `get_module_file_name_w_of_a_real_module_…`（名前も変更） | 期待を「停止（exit 134）」から「答える（exit 0、`HGN1` = ディレクトリ + `mod2.dll` の長さ）」に | 壁 0（決定 0082 D6 の停止が 0103 で答えに変わった） |
| `fe-lift/tests/realmod_s1/guest_host.c` | コメントのみ | 同上 |
| `fe-lift/tests/spi.rs` の `source_uses_no_host_…` | `fle_st(` の数 5 → 8 | 壁 9〜11（新しい SPI の 3 値が各 1 回の 4 バイト書き込み） |

**全体テストで見つかった不具合（修正済み）**: 最初の版は `fle_vfs.c`／`fle_res.c` が表の記号（`fle_realmod_table`、`fle_realmod_count`）を直接参照していて、`fle_realmod.c` を載せないテストのドライバ（`env`、`event`、`reg`、`res`、`loadimage`、`realmod` の `without_a_table_…`）が「undefined reference」でリンクできなかった（`fle_realmod.h` の約束 — 表も `fle_realmod.c` も無いプログラムは従来どおり — を破っていた）。`fle_realmod.c` に `fle_realmod_total`／`fle_realmod_name`／`fle_realmod_base`（`FLE_REALMOD_REF` の弱い参照。`FLE_REALMOD_PRESENT()` の後だけ呼ぶ）を足して直した。単体の `wall_q.rs` は全 `fle_*.c` をリンクするので、この種の不具合を見つけられなかった。

**全体テスト（`cargo test --workspace --no-fail-fast --offline`、最終のソース、`test-ws.txt`）: 101 binaries、1,275 passed、0 failed、0 ignored、exit 0**（基準 98 binaries／1,228 + 0095 の 26 = 1,254 に、新規 21 件と 3 binaries が加わった数に一致）。1 回目（`test-ws-partial-enospc.txt`）と 3 回目の直前（`test-ws-partial2-enospc.txt`、`test-ws-partial3-realmod-link-error.txt`）は途中経過で、失敗は `No space left on device` と上の undefined reference だけだった。

**HEAD が進んだ**（作業中に `0e9b703` → `a951ecc`（0095）→ … → `d56f42f`（決定 0096: deferred modules））。`patches/wallQ.diff` は **`d56f42f` の `git archive` に当たる形に作り直した**（28 ファイル。`patch -p1` で `imports_table.rs` の 1 hunk だけが文脈の衝突で当たらなかったので、行を表の**最後**（0096 の行の後）に足した。既存の行の擬似 VA は動かない）。旧基準（`0e9b703` + 0095 の作業ツリー差分 + ハーネス）向けは `patches/wallQ-on-0e9b703-base.diff`。`d56f42f` + 本差分で、fe-lift の 18 binaries（modules、deferred、load、k32_dsp、imports_table、realmod、wall_q、wall_q_lift、spi、res、vfs、msctf、event、glob、vol、env、reg、loadimage）と fe-decode／fe-pe の 23 binaries（208 passed）は 0 failed。**新しい HEAD での全体テストと a4 の再測定はしていない**（ディスク）。決定 0096 は「同名の実在モジュールが仮想モジュールに勝つ」と定めていて（`fle_realmod.c` の `table_check`）、この作業と衝突しない（読み。a4 は未測定）。

## 止まった条件と選択肢

**止まった条件**: (e) 別レーン（レーン S）の壁。`VirtualProtect` が QFC のイメージのコード（インラインフックの対象）に対して呼ばれる。FLEngine の unit 314（0099 の W4）と同じ型。

選択肢（レーン S の決定の後。推奨は書かない）:

| | 内容 | 作業量の目安 |
|---|---|---|
| (a) | イメージの保護変更を追跡しないことにして成功（旧保護値を返す）にする。決定 0045 の「wasm のページ保護は強制しない」と整合 | 小。ただしフックの書き込み（`f_597e..` の「有効化」）も実行されるので、書き込み先がコード領域でないか（生成 C と対応しない）の確認が要る |
| (b) | 停止のまま | 0 |
| (c) | フックを入れる呼び出し元（QFC の unit 162）だけをスキップ | 中。実モジュールの DllMain を変える形になり、原則に反する |

その先の壁（未測定）: unit 162 の `GetModuleFileNameW`（`delphi32.exe`／`bds.exe`）、unit 245 以降の `FontCache_AcquireFont` のスロット、`f_66d050` の残り（`MessageBoxW`）、レーン R の `EnumFontFamiliesExW`。

## 限界・未確認

- **ディスクが満杯に近い**（`/` 使用率 100%、空き 0.1〜2.7 GB で、他の作業と共有）。自分の作業物（`lift/out-wallQ/` 約 40 GB）は `rm` 禁止の規則のため消せなかった。空きが少ない間は、シェルツールが出力を返さなくなり（`(no output)`）、全体テストが ENOSPC で失敗した。PM は `lift/out-wallQ/run-0`〜`run-4`、`fast/`、`exp-src/`、`chk/` を消してよい（再現は `run-5` と `env.sh`／`run-multi.sh` で足りる）。
- すべての応答は実機の Windows と照合していない。創作の値: D1 のパスの大文字小文字と置き場所、D5 の SetHandleCount の戻り値、D6 の秘密、D9c の orientation（RGB）。D9b（STANDARD）は SET の項の既定に基づく。
- freetype の種 160 個は測定用。`FT_Init_FreeType` が通ったあとの freetype の実行（`FT_New_Face` など）は未到達。
- a4 の測定（上の前後の表）は旧基準（`0e9b703` + 0095 の作業ツリー差分 + ハーネス）の木で行った。`d56f42f`（0096 など）での a4 の再測定はしていない（ディスク。`run-5` 一式が 7 GB）。最終のランタイム（`fle_realmod_total`／`name`／`base` のアクセサを足した後）での 5 モジュール構成の再確認は `fast/final-check`（`run-5` の生成物に再リンク）で行い、159／942、functions 1,912、entries 77,849、停止 `VirtualProtect` は同じ。単独・2・3 モジュールの再測定は、アクセサの変更が実在モジュールのハンドルに対する経路にしか触れないので省いた（推測）。
- wasm（`emcc`）は使っていない。QFC の `f_522e50`（dsp の 420 名を引く）が wasm の locals 上限を超えるかは未測定。
- `patches/wallQ-first-with-test-artifacts.diff` と `patches/wallQ-files.txt` は途中の出力（前者はテストが `src/lift/target/` に書いた 5 ファイルを含む。後者は空の整形に失敗した出力）で、使わない。

## 作ったもの

`lift/out-wallQ/`: `base/`（旧基準）、`src/`（編集した木）、`nh-base/`／`nh-src/`（`d56f42f` とそれに差分を当てた木）、`patches/wallQ.diff`（`d56f42f` 向け。28 ファイル、2,029 行。`patches/wallQ-changed-files-final2.txt`。統合版は `merge1/resolved-Q.diff`: 14 ファイル）、`patches/wallQ-on-0e9b703-base.diff`（旧基準向け）、`docs-new/`（この文書、`wall-q-measurement.md`、`README-0103-line.txt`）、`run-0`〜`run-5`（a4-multi の OUT。最終は `run-5`）、`logs/`、`qfc/`、`freetype/`、`tools/`（`qdis.py`、`lea_code.py`、`impcheck.py`、`unsup_sites.py`、`reach_c.py`、`gdb_hits.py`）、`fast.sh`／`qrebuild.sh`／`run-multi.sh`／`env.sh`、`exp-src/`（壁 3 の実験。パッチに入れない）。

## コミット時の追記（2026-10-27、PM）

- 統合した木（I・R・S・B・Q を当てたもの）の全体テストで、`tests/vcall_thunk_dll.rs` の `after_the_memory_form_jmp_only_the_import_thunks_remain_unsupported` が期待値 516 に対して 515 を返した。原因は、本決定が応答表に足した `shell32!SHGetFolderPathW` の行で、FLEngine の IAT thunk（0x593450）が名前付き停止から `fle_imp_SHGetFolderPathW` への振り分けに変わったこと（統合木の生成 C で `L_593450` の行を確認した）。期待値を 515 に直した（コメントに 0103 の分を足した）。
- 【訂正（決定 0105 のコミットで）】下の `43ee118e…` は C ファイルの SHA-256 ではなく、`fe-lift lift … /dev/stdout --pdata | sha256sum` の流れ（C の後ろに統計の行が付く）の SHA だった。C ファイルそのものの SHA-256 は `8789c1a5c36125c5ce412b75ab9696e785d98cca2bc0d34b043607345850a77e`（215,747,179 B。I・R・S・B・Q を当てたもの）で、`lift/harness/a4-multi.sh` の既定値はこちらに直した。
- 統合木の FLEngine 生成 C の流れの SHA-256 は `43ee118ea490e47f363c3402e05cbca13cd6f5bd5e9c5edce97831f14fe0a0d6`（I・R・S・B・Q を当てたもの。`lift/harness/a4-multi.sh` の既定値）。
