# 0104: ILTools_x64.dll を実在モジュールとして載せる（基底 0x6000000 は測定用の値。census で ATTACH の壁を 7 件直し、8 件目の lift の形で止めた。仮の決定を含む）

状態: **仮の決定**（基底、種の規則、`GetModuleFileNameW` の値、カーソルのハンドルの値は仮。下の「仮の決定の一覧」）。PM の指示（決定 0101 の判定構成に ILTools を入れる作業）を、作業者（サブエージェント）が 2026-10-27 に記録した。決定 0086 の「実行が届いて調査した時点で決める」11 名のうち ILTools の個別の方向は、0086 と同じ（無いと定義しない、仮想にしない、実体を lift して実在モジュールにする）を採る。0046、0060、0073、0080、0082、0086、0101 の本文は書き換えない。

記入状況: 基底、種、lift の統計、census の測定、壁の表、仮の決定は記入済み（実測）。**検証の (c)（単独の a4）は実行できていない**（作業中にディスクが満杯になり、単独のバイナリ約 250 MB を作る余地が無かった）。(d)（2 モジュールの a4）は反復用のビルドで測った（159 / 942 で土台と同じ。依頼文の 150 ではない理由は「検証」）。(a)（`cargo test --workspace`）は、ディスクが満杯になって通し実行が途中で落ちたので、落ちた分を個別に再実行して全部通した（98 バイナリ・1,278 件・失敗 0。「検証」と「限界」）。

事実の出典（節番号は各文書のもの）:

- `docs/investigations/bundled-dll-inventory.md`（ILTools の所在、PE の事実 §7、不足の整理 §8）、`loadlibrary-names-classification.md`、`external-modules-usage.md`。
- 決定 0080（基底の方向、`fe-pe rebase`）、0082（実在モジュールのロードの意味。D6 は今は決めない）、0086（同梱 DLL の方向）、0094・0095（MSVC switch。触らない）、0096（遅延モジュール）、0097（census）、0101（A4 の判定は実在モジュール構成）、0036（注釈つきの jump table）、0051（thunk の種）、0074（実行範囲と reloc の種）、0025（資源 API）、0061（LoadImageW）、0070・0071（ウィンドウクラスと窓）。
- `docs/investigations/dll-nonpdata-reloc-seeds.md`（dsp の「`.pdata` 開始でない reloc 値」）、`blend2d-seed58.md`（Blend2D の種 58 個）。
- 測定の詳細は `iltools-attach-measurement.md`（同じディレクトリ）。

## 決定

| # | 項目 | 決定 |
|---|---|---|
| 1 | 方向 | ILTools_x64.dll（ゲストの綴り `iltools_x64.dll`）は、dsp_ippv2・Blend2D・QFC と同じ方向で扱う。**「無い」と定義せず、仮想モジュールにもせず、実体を lift して実在モジュール（`fle_realmod`）として載せる**。決定 0101 の決定 4 の表に出ている 4 名（dsp_ippv2、blend2d、quickfontcache_x64、iltools_x64）の最後の 1 名を、他の 3 名と同じ扱いにする。他の同梱 DLL 10 名は決めない（0086 のまま） |
| 2 | 基底（**測定用の値。決定にするなら仮**） | **0x6000000**。ImageBase 0x400000 は FLEngine と全面的に重なるので、`fe-pe rebase`（決定 0080）で移す。往復（0x400000 → 0x6000000 → 0x400000）は原本と全バイト一致（SHA-256 `20409dcb…0f95`）。理由は下の「基底の選定」 |
| 3 | `RM_MAX` | **4 のまま**（`fle_realmod.c`）。dsp_ippv2、Blend2D、QFC、ILTools でちょうど 4。5 つ目（`ildsp_x64` など）を載せるときは別の決定（0086 の決めないことの 2 のとおり）。**4 モジュールを同時に載せた構成は測っていない**（この作業は dsp_ippv2 + ILTools の 2 モジュール。Blend2D と QFC は別の作業） |
| 4 | 種 | `fe-lift lift リベース版 OUT.c --pdata` に、下の規則で選んだ **400 個**（`lift/harness/seeds/iltools-6000000-seeds400.txt`。基底 0x6000000 の VA）を `--seeds` で足す。規則と統計は「種」の節。規則は決定ではなく測定用（基底と `fe-lift` の版に結びつく） |
| 5 | 注釈つきの jump table | `fe-decode/src/jumptab.rs` に `ILTOOLS_6000000`（1 件）を足し、`fe-lift` の CLI が `FLENGINE` に連結して渡す。決定 0036 の既存の形（`Kind::Abs64`、`jmp [r10+rdx*8+0x40]`）で、新しい形は設計していない。FLEngine の 0x40d210 と同じバイト列の塗りつぶしルーチン（ILTools の 0x600a580）。VA を名指すので基底 0x6000000 の像にだけ効く（`FLENGINE` が 0x400000 にだけ効くのと同じ） |
| 6 | 実在モジュールの hInstance を受ける API | 実在モジュールのハンドルは基底（決定 0080 D5）で、DllMain の `hinstDLL` と同じ。**FLEngine の hInstance（0x400000）だけを受けていた runtime の判定に、実在モジュールの基底を加える**: 資源 API 5 つ（`FindResourceW`、`LoadResource`、`LockResource`、`SizeofResource`、`LoadStringW`。像の `.rsrc` を読む）、`LoadImageW`（カーソルの `RT_GROUP_CURSOR`）、`GetClassInfoW`・`RegisterClassW`・`CreateWindowExW` の hInstance。モジュールのハンドルが指す像の資源・クラスを引くのは各 API の仕様どおりなので、**これは仮の決定にしない**（値は作っていない）。決定 0025 の「他のモジュールは資源を持たない」は、実在モジュールについては置き換わる |
| 7 | `GetModuleFileNameW`（実在モジュールのハンドル。**仮**） | `C:\Program Files\Image-Line\FL Studio 21\Shared\` + 像の export directory の `Name` フィールド（ILTools は `ILTools_x64.dll`）。決定 0082 の D6 が「今は決めない」としていた値を、ILTools の ATTACH がこの呼び出しに届いたので、仮に決める。ディレクトリの「Shared」は同梱 DLL の配置先（インストーラの NSIS で `<変数>\Shared`。`bundled-dll-inventory.md` §4。変数の値は未確認）、手前は決定 0018 の `FLE_VIRTUAL_DLL_PATH`（PM の値。Windows の規則ではない）のインストール先。ファイル名は像が自分で記録している名前（創作ではない）。未ロードの実在モジュールのハンドルは停止のまま（`fle_stop`） |
| 8 | 実在モジュールのカーソルのハンドル（**仮**） | `FLE_LI_MODULE_HANDLE_BASE + 0x1000 × (実在モジュールの表の index + 1) + 4 × (id − 32762 + 1)`（`fle_loadimage.h`）。決定 0061 のハンドルと同じ「トークン」で、同じ id でもモジュールごとに別の値にする（領域 1 MiB の内側。最大 4 モジュール） |
| 9 | 8 件目の壁 | `jmp [rax+disp]`（Delphi のインターフェース呼び出しの thunk）は `tailjmp.rs` が扱わない形（決定 0036 W1）。**新しい形を設計せず、記録して止める**。FLEngine の生成 C に同じ未解決の `jmp` が 1 か所あり、扱えば FLEngine の C が変わるため。別の作業（レーン L の「終了フェーズの `jmp [rcx]`」）が同じ形を扱うなら、統合のときにこの壁は変わりうる（読み。統合後に再測定が要る） |
| 10 | 状態 | 仮の決定。撤回条件は「仮の決定の一覧」 |

## 基底の選定

| 領域 | 範囲 | 出典 |
|---|---|---|
| FLEngine_x64.dll | 0x400000 〜 0x1cc0000 | SizeOfImage 0x18c0000（実測） |
| dsp_ippv2（決定 0080 D1） | 0x2000000 〜 0x461f000 | SizeOfImage 0x261f000 |
| Blend2D（測定用） | 0x4700000 〜 0x4a00000 | SizeOfImage 0x300000 |
| QFC | 別の作業が選択中 | SizeOfImage 0x20e000。0x1d00000（FLEngine と dsp の隙間）や 0x4700000 が測定に使われた |
| **ILTools（本決定）** | **0x6000000 〜 0x6394000** | SizeOfImage 0x394000（実測） |

- `fle_realmod.c` の検査（`table_check`）が基底に課すのは「0 でない 64 KiB の倍数、0x400000 でない、仮想モジュールのハンドル域（0x7FF800000000）より下、他の実在モジュールと名前も基底も違う」。**像どうしの重なりは検査していない**（`image.c` の mmap が `MAP_FIXED_NOREPLACE` で失敗するだけ）。0x6000000 はどれも満たし、他の 3 領域と重ならない（算術）。
- 理由: (1) 上の 3 領域の外にあり、dsp_ippv2 の終端（0x461f000）と Blend2D の終端（0x4a00000）の後に 0x1600000（22 MiB）の空きを残すので、QFC がどちらの隙間に入っても、Blend2D の領域が測定用の値から動いても重ならない。(2) 64 KiB の倍数。(3) wasm の `GLOBAL_BASE`（0x10000000）、トークン（0x7F990000 以上）、番兵（0xCB000000）より下（決定 0080 D1 と同じ流儀）。(4) ネイティブの実行で像が mmap できた（実測: `real module image: base=0x6000000 size=0x394000`）。wasm では**未確認**。
- 代替案: **0x4c00000**（Blend2D の終端の直後。ILTools の終端は 0x4f94000）。空きが小さく、QFC が 0x4a00000 以降に置かれると詰まる。0x8000000 などの高い値も同じ条件を満たす（決定 0080 D1 の注意書きと同じで、この値でなければならない理由はない）。0x4a00000 ちょうどは Blend2D の終端と接し、余裕が無い。
- 撤回条件: Blend2D・QFC の基底の決定、または wasm のメモリ配置（決定 0080 D10）と衝突すると分かったとき。

## 種

- `.pdata` は 11,747 エントリ（`fe-pe pe-pdata-info`）。`--pdata` だけで functions 12,526（closure が足す分を含む）、bad_nodes 0。
- reloc: DIR64 72,082、実行範囲内を指す 20,448。うち `.pdata` 開始 5,785（`--pdata` に含まれる）、**開始でない 14,663**（`.pdata` の範囲内 10、どの範囲にも入らない 14,653）。比較: dsp_ippv2 は 2,750（決定 0074）、Blend2D は 58（`blend2d-seed58.md`）。
- 14,653 個のうち、`--pdata` の閉包が覆わない（生成 C に `L_<VA>:` が無い）ものは **14,456 個**。これを全部種にすると functions 27,114、**bad_nodes 25,927**（case ラベルやコードでない領域を指す値が多い）。FLEngine で無条件に足すと bad_nodes が出るのと同じ型（0074 の項目 8）。
- 規則（測定用）: 種 = 上の 14,456 個のうち、**その種から始まる閉包に undecodable な node（`Node::Bad`）を 1 つも含まないもの**。`lift/harness/seeds/refine_seeds.py` が、全部を種にして lift し、Bad を含む関数に届く種を捨てて、Bad が 0 になるまで繰り返す（実測 4 ラウンド）。結果は **400 個**（14,456 → 420 → 401 → 400）。`iltools-6000000-seeds400.txt` に基底 0x6000000 の VA で入っている。
- 400 個を足した lift: functions **12,930**（+404）、closure_instructions 403,636、**bad_nodes 0**、jumptabs 1、msvc_tables 0、unsupported_total 866。unsupported の増えた分は、閉包に入った別の thunk の `jmp [reg+disp]`（132）と、padding のように見える命令（Int3 72、Int1 66、Outsb 27、`ret imm16` 22）。**これらの命令が実行されるかは未確認**（コードでない領域を読んだ可能性がある。bad_nodes 0 は「undecodable な node が無い」ことだけを言う）。
- この規則が選んだ種が**正しい関数の先頭であることの証明はない**（読み: `0x605dd99`、`0x605dda3` は 3 命令の thunk の先頭で、その一族に合う）。実行が必要とする関数が 400 個の外にあれば、`fle_dispatch_miss` で止まる（今回の壁 7 がそうだった）。撤回条件: 基底か `fe-lift` の版が変わったとき（再計算する）、または実行がこの 400 個の外の関数を必要としたとき（種を足す）。

## 測定（census。A4 の進捗ではない）

構成は FLEngine + 縮小 dsp_ippv2（0x2000000、種 452）+ ILTools（0x6000000）。土台は遅延モジュール（0096）を含む。詳しくは `iltools-attach-measurement.md`。

| 項目 | 結果 |
|---|---|
| strict の a4（census なし） | init_count 159 / 942（遅延モジュールで unit 149・158 を通る。functions 978、entries 27,134）。停止は `shell32!SHGetFolderPathW` の IAT thunk（unit 158）。**ILTools には届かない** |
| census の unit 280 | `LoadLibraryW "iltools_x64.dll"` に**成功**し、DllMain(ATTACH) に入る（基準の census では名前付き停止だった）。直した 7 件の後、`jmp [rax+0x10]` の thunk（0x605dda3）で放棄される |
| census の unit 659 | `LoadLibraryW` に成功（unit 280 の放棄で ST_LOADING のまま、参照 +1）→ `GetProcAddress("CreateCursorFromFont")` が export の実在のアドレス（0x6262b60）を返す → ゲストがそれを呼ぶ → 中の `GetSystemMetrics(13)`（SM_CXCURSOR）で停止（決定 0057）。**`GetProcAddress(NULL, …)` の abort は避けられた**（実測）。ただし ILTools の初期化は済んでいない状態での呼び出しで、「unit 280 が成功した後」の測定ではない |
| 件数 | census の内訳は基準と同じ（ok 878、stop 17、fault 0、hang 0、empty 46、noproc 1。この 17 件の入れ替わりは unit 280 と 659 の停止の理由だけ） |

## 壁の表（7 件直した。8 件目で止めた）

| # | 停止 | 原因 | 修正 | 区分 |
|--:|---|---|---|---|
| 1 | `GetModuleFileNameW`（hModule = 0x6000000） | 決定 0082 D6 の保留 | `fle_vfs.c`、`fle_realmod.c/h`（決定 7） | 仮 |
| 2 | `LoadStringW` | 資源 API が 0x400000 だけ | `fle_res.c`（決定 6） | 仕様どおり |
| 3 | 0x600a5c3 `jmp [r10+rdx*8+0x40]` | 注釈が FLEngine の VA だけ | `jumptab.rs`、`fe-lift/main.rs`（決定 5） | 既存の形 |
| 4 | `LoadImageW` | hInst が 0x400000 だけ | `fle_loadimage.c/h`、`fle_res.c/h`（決定 6、8） | 仕様どおり + ハンドルは仮 |
| 5 | `GetClassInfoW` | 同上 | `fle_class.c/h`（決定 6） | 仕様どおり |
| 6 | `CreateWindowExW` | 同上 | `fle_win.c`（決定 6） | 仕様どおり |
| 7 | dispatch miss 0x605dda3 | `.pdata` に無い thunk | 種 400 個（決定 4） | 測定用 |
| 8 | 0x605ddaa `jmp [rax+0x10]` | `tailjmp.rs` が `jmp [mem]` を扱わない | **直さない**（決定 9） | 残り |

応答表（`imports_table.rs`）の行は足していない。FLEngine の生成 C の SHA-256 は土台と同じ `170bd18f…`（`feb5b908…` との差は遅延モジュールの行だけ）。

## 仮の決定の一覧と撤回条件

| # | 仮の決定 | 撤回条件 |
|---|---|---|
| P1 | 基底 0x6000000 | 上の「基底の選定」の撤回条件 |
| P2 | 種の規則と 400 個 | 基底か `fe-lift` の版の変更、実行が 400 個の外の関数を必要としたとき |
| P3 | `GetModuleFileNameW` の値（決定 7） | PM が値を与えたとき。NSIS の `Shared` の変数が分かったとき。ILTools がこのパスを使う経路で、値が実機と合わないと分かったとき |
| P4 | カーソルのハンドルの値（決定 8） | 決定 0061 のハンドルの値が変わったとき。ゲストがハンドルの値そのものに依存すると分かったとき |
| P5 | 注釈 `ILTOOLS_6000000` の VA | 基底の変更（注釈を作り直す）。扱う形が `tailjmp` などの一般の仕組みに取り込まれたとき |

決定 6（hInstance の判定）は仕様から決まるので仮に数えない。ただしクラスのアトム（0070）と窓（0071）のハンドルは、それぞれの決定の仮の値のまま。

## 範囲外

- 8 件目より先の壁（ILTools の InitUnits の 283 unit、行の無い import 346 名）。`jmp [mem]` の tail dispatch の設計。
- Blend2D、QFC の実在モジュール化と、4 モジュール同時の構成（別の作業）。他の同梱 DLL 10 名。
- `DllMain(DETACH)` と `FreeLibrary`（決定 0082 D11 のまま）。ILTools が読む `dsp_ippv2.dll`・`Blend2D.dll`・`QuickFontCache.dll`・`ilvideo_x64.dll`（`bundled-dll-inventory.md` §7.3。ATTACH で実行されるかは未確認）。
- wasm（未測定）。実機 Windows との照合（0 件）。A4 の定義・判定の変更（決定 0046、0060、0101 のまま）。

## 検証

| 項目 | 結果 |
|---|---|
| (a) `cargo test --workspace --no-fail-fast`（スナップショット `lift/out-wallI/src`、`FLENGINE_PE` 指定、`CARGO_TARGET_DIR=lift/out-wallI/target-test`、`CARGO_PROFILE_DEV_DEBUG=0`） | **98 バイナリ（fe-decode 11、fe-lift 77、fe-pe 10）、通過 1,278、失敗 0**（+ doc-test 3 バイナリ、0 件）。**1 回の通し実行ではない**: 通し実行（`test-ws.txt`）の途中でディスクが満杯になり（空き 0）、`modules` と `monitor` 以降が `No space left on device` で落ちた（47 バイナリ・574 件はその前に通過）。落ちた分を、ディスクに空きがあるときに `cargo test -p fe-lift --test …` で個別に再実行して全部通した（`test-sel1.txt`〜`test-sel6.txt`: modules 16、realmod 14、res 19、tailjmp 4、vfs 22、window 36、monitor から wasm_port までの 32 バイナリ 480、fe-pe 83、x87 系 3 バイナリ 30。通し実行で通った 574 と合わせて 1,278）。**変更が既存のテストに触る箇所**: `tests/realmod.rs` の 1 件（0082 D6 の停止から決定 7 の答えへ。期待値を書き換えた。mingw で作る実 DLL の実行で通った）。新規のテストは 0 件。基準: 土台に census（9 件）と遅延モジュール（15 件）を足した木で 98 バイナリ。先行の報告「HEAD a951ecc の木で 98 バイナリ・1,254 件」に census と deferred の 24 件を足すと 1,278 で、**件数が一致する**（土台の木で通し実行はしていない） |
| (b) FLEngine の生成 C の SHA-256 | **`170bd18fe2a365de47068e85ffe1c2e1a3f04f912db7e3335f40f6eecaecca43`**（実測）。土台と同じ。表の部分も一致 |
| (c) 単独の a4（145 / 942、functions 821、entries 15,685） | **実行できていない**（ディスク）。FLEngine の生成 C が土台と同じで、runtime の変更は実在モジュールのハンドルの時だけ働く（読み） |
| (d) 2 モジュール（FLEngine + dsp のみ） | 最終の木で測った（`lean2.sh`: FLEngine と dsp_ippv2 のオブジェクトは土台のもの、runtime は最終の木。`verify/two-dsp-final.out`）: init_count **159 / 942**、last unit index 158、started 146 / completed 145、**functions 978、entries 27,134**、停止 `fle: unsupported instruction at 0x593450`（`SHGetFolderPathW` の thunk）。**土台の木の値と同じ**（`a4-multi.sh` の完全なビルドの dsp + ILTools、`run2` の strict の a4 と同じ数）。**依頼文の「150 / 942、functions 943、entries 26,585、停止 blend2d」にはならない**: それは遅延モジュール（0096）を含まない木の値で、土台の木には遅延モジュールが入っており、土台でも 159 になる（遅延モジュールを含まない木の 150 は再測定していない） |
| 追加 | 反復ビルド it00 の census は、土台のビルドの census と時間の列を除いて一致（実測）。`patch -p1 --dry-run` は土台に当たる |

## 解釈が分かれた点

1. **8 件目の扱い**: 「新しい形を設計しない」の対象に `jmp [mem]` の tail dispatch が入ると読んだ。`tailjmp.rs` の先頭のコメントが明示している未対応の形で、FLEngine の C が変わるため。注釈（決定 0036）は、新しい形ではなく既存の形の別の像への適用と読んで足した（壁 3）。
2. **種**: 「必要なら reloc 先の規則」を、dsp（手で選んだ 452 個）や Blend2D（`--not-function` の全部 58 個）と違い、14,653 個の全部は使えない（bad_nodes）ので、閉包の健全性で絞る規則にした。他の規則（直前のバイトが int3、アラインメント）は試していない。
3. **`GetModuleFileNameW` の値**: ファイル名は像の export directory の `Name`（像が自分で記録している名前）を使い、ディレクトリだけを仮にした。ILTools の実機での位置は未確認。
4. **census の中で直す**: 反復ごとに FLEngine と dsp_ippv2 のオブジェクトを再利用する測定用のビルド（`lean.sh`）を使った。応答表の行を足していないので、これらの生成 C は土台と同一で、再利用は正確（実測: 同じ census の出力）。`a4-multi.sh` による完全なビルドは 1 回（strict の a4、census の土台）。

## 限界

- **ディスクが満杯になり（19 GB → 0。他の作業の出力と、この作業の約 6.5 GB の合計）、検証の (c) と、完全なビルド（`a4-multi.sh`）での最終の census を実行できなかった**。(d) は反復用のビルド（`lean2.sh`）で測った。(a) は 1 回の通し実行ではなく、落ちた分の再実行を合わせた結果（上の表）。最終の census の数（ok 878 / stop 17）は `lean/it07` の反復ビルド（FLEngine と dsp_ippv2 のオブジェクトは土台のもの。ILTools と runtime は最終の木）の実測で、`a4-multi.sh` の完全なビルドではない。
- 7 件の修正は census の中でだけ確かめた（unit 280 の ATTACH が 8 件目まで進む）。ATTACH が戻ること、unit 280 が完了すること、`CreateCursorFromFont` が初期化済みの ILTools で動くことは**未確認**。
- 4 モジュール（dsp_ippv2、Blend2D、QFC、ILTools）同時の構成、`RM_MAX` 4 の上限での動作、`census/` の `MAIN_REAL_MAX` 4 での動作は未測定。wasm は未測定。
- 種 400 個の中の関数が実機で必要かは未確認。padding のような命令（Int3 など）を含む関数は、実行されると `fle_unsupported` で止まる。
- 実機 Windows（ILTools の DllMain の挙動、`Shared` の位置）との照合は 0 件。
