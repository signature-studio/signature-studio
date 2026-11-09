# 0086: インストーラに同名のある同梱 DLL のうち、調査済みの Blend2D.dll と QuickFontCache_x64.dll は、dsp_ippv2_x64.dll と同じ方向（「無い」とも仮想モジュールとも定義せず、実体を lift して実在モジュールとして載せる）で扱う（方向の決定）

状態: PM の方針決定を、作業者（サブエージェント）が 2026-10-24 に記録した。**方向の決定**であり、「仮の決定」ではない（決定 0073 と同じ位置づけ）。この決定は値（基底アドレス、`fle_realmod` の表の上限、縮小 DLL の種、wasm の locals 上限への対処、JIT・TLS・スレッドの扱いなど）を一つも決めない。それらは未決事項（下）で、別の決定で決める。実装もこの決定の後続作業で、この決定では何も変更していない（`lift/` 配下は読み取りだけ）。方向の根拠のうち**実機で確認していない事項**は「限界」に書く。この記録は調査に無い主張を足さない。

事実の出典は次の調査文書で、節番号はその文書のもの（以下の略称を使う）。

- 「分類」= `docs/investigations/loadlibrary-names-classification.md`
- 「Blend2D 調査」= `docs/investigations/blend2d-lift-feasibility.md`
- 「QFC 調査」= `docs/investigations/quickfontcache-lift-feasibility.md`
- 「unit153」= `docs/investigations/registry-unit153.md`
- 「外部モジュール」= `docs/investigations/external-modules-usage.md`
- 「0084 後」= `docs/investigations/two-module-after-0084.md`

## 背景

決定 0073 は、`dsp_ippv2_x64.dll` を「無い」と定義せず、`FLEngine_x64.dll` と同じ Rust ツールチェーン（fe-pe → fe-decode → fe-lift）で lift して 2 つ目のモジュールとする**方向**を決めた（値は決めない）。決定 0080 は dsp_ippv2 のリベース先の基底（D1 の 0x2000000）とファイル段階のリベース（D2）などを決め、決定 0081 は API セット名 4 名を仮想モジュールにした。決定 0055 は msctf.dll を仮想モジュールにし、決定 0015、0029 は仮想モジュールと absent（`fle_load_absent[]`）の扱いを決めている。0086 はこれらを変更しない。

### 1. 現行の runtime の挙動と、FLEngine の LoadLibraryW の順

- LoadLibraryW/A/ExW は、(1) 実在モジュール表（`fle_realmod`。2 モジュール構成のときだけ有効）、(2) 仮想モジュール表 `fle_modules[]`、(3) `fle_load_absent[]` の順に名前を引き、どれにも無ければ `fle_stop`（分類 §0.1）。`Blend2D.dll` と `quickfontcache_x64.dll` はどの表にも無い（この作業で `lift/runtime` を大文字小文字を区別せず grep して、`blend2d`、`quickfont` が 0 件であることを確認した。QFC 調査 §2 末尾も runtime と fe-lift の `src`・`tests` に `quickfont` の文字列が無いと書く）。
- FLEngine の LoadLibraryW は、次の順に現れる。unit は初期化表の 0 始まりの index で、ゲストの InitCount は index + 1（分類 §4 末尾）。

  | unit | LoadLibraryW に届く名前 | 出典 | 区分 |
  |---:|---|---|---|
  | 144 | `dsp_ippv2_x64.dll` | 分類 §4（InitCount 145 の行） | 実測（gdb）。2 モジュール構成では実在モジュールとして応答 |
  | 149 | `Blend2D.dll` | 分類 §4、§7.1。Blend2D 調査 §2（gdb の `re6.log` 960 行目 `name='Blend2D.dll'`）、§4.1（呼び元 `f_664190` ← `f_664260`。unit 190 の 1 段下の `f_6cf2a0` からも同じ `f_664190` に届く。読み） | 実測（仮の値つきの構成） |
  | 158 | `quickfontcache_x64.dll` | 分類 §7.2（静的に届く最小 unit）、unit153 §6.1、§6.3（停止メッセージ `LoadLibraryW: module "quickfontcache_x64.dll" is not a virtual module …`） | 実測（仮の値つきの構成で、この名前で停止） |

- 149 と 158 の順は、仮の値を入れた構成の測定である（分類 §0.2 末尾、§7.1。「仮の値を入れないと DllMain は完走せず、avrt に届かない」）。仮の値を入れない構成では、決定 0084 の runtime での測定で、単独構成は unit 144 の `LoadLibraryW("dsp_ippv2_x64.dll")`（名前付き停止）、2 モジュール構成は dsp_ippv2 の DllMain の 127 回目の import `GetStringTypeW` で止まり、どちらも unit 149 に届いていない（0084 後 §1 の 4、5、§6）。
- 149 と 158 の間の unit 153 は `RegCreateKeyExW`（LoadLibrary ではない。決定 0085 が扱った）で、LoadLibraryW の順には入らない（分類 §4 の表、unit153 §6.3）。

### 2. 2 つの DLL の事実

| 項目 | `Blend2D.dll` | `QuickFontCache_x64.dll` |
|---|---|---|
| インストーラ内 | 1 件。エントリ 56450、2,065,920 B、SHA-256 `66634d24…66c7`（Blend2D 調査 §2。実測。外部モジュール §4 の「あり」の列にも `Blend2D.dll` がある） | 1 件。エントリ 56472、2,067,968 B、SHA-256 `afc5a0ed…36d2`（QFC 調査 §1 の 1、§2。実測。外部モジュール §4 の「あり」の列にも `QuickFontCache_x64.dll` がある）。32 ビット版 `QuickFontCache.dll`（56470）は別にある |
| Windows の DLL か | ではない（分類 §5.1） | 同左 |
| 配置先 | `Shared`（直前のエントリ 56448 が `<NSIS 変数>\Shared`。dsp_ippv2_x64.dll（56484）と同じ並び。NSIS の変数がどの変数かは未確認。Blend2D 調査 §2） | 同左（QFC 調査 §2。未確認も同じ） |
| ゲストの NULL の扱い | ロード点は許容（分類 §2.2 の #4）。使う側 `f_6cb0b0` は NULL 検査なし（Blend2D 調査 §4.2。読み） | ロード点は許容（分類 §2.2 の #5）。使う側 4 名（`AcquireFont`、`GetStats`、`AcquireSharedThreadBank`、`AcquireSharedAnimTimer`）は呼ぶ前にスロットを検査しない（QFC 調査 §5.2。読み） |
| FLEngine が引く名前 | `AggCreateInstance` の 1 名。export に実在（Blend2D 調査 §4.1） | 8 名。すべて export に実在（QFC 調査 §5.1） |
| lift の統計（`fe-lift lift FILE /dev/null --pdata`。既存のツールが変更なしで通る） | 関数 2,018、命令 398,879、`bad_nodes` 0、`unsupported_total` 662（Blend2D 調査 §3.2。実測） | functions 6,712、closure_instructions 209,259、`bad_nodes` 0、`unsupported_total` 104（QFC 調査 §1 の 4、§4.1。実測） |
| ImageBase / SizeOfImage | 0x180000000 / 0x300000（Blend2D 調査 §3.1） | 0x400000 / 0x20e000（QFC 調査 §3.1）。FLEngine（0x400000〜0x1cc0000）と重なり、生成 C の `f_<VA>` が FLEngine と 374 個同じ（QFC 調査 §1 の 3、3b） |
| `fe-pe rebase`（決定 0080 のツール） | 0x4700000 で通る。往復で元と全バイト一致（Blend2D 調査 §8。測定用のコピー） | 0x1d00000、0x4700000、0x8000000 で通る（QFC 調査 §3.6。測定用のコピー） |
| 自身の依存 | import は KERNEL32.dll だけ（Blend2D 調査 §3.1） | 自身の初期化表の unit 88、129 が `dsp_ippv2.dll`、`freetype.dll` を LoadLibraryW する形に読める（QFC 調査 §1 の 11、§6.2。**読み。実行していない**） |

どちらも、生成 C のリンクと wasm の実行は確認していない（Blend2D 調査 §8、§10.4。QFC 調査 §0、§4.5）。

### 3. dsp_ippv2 の現在の状態

dsp_ippv2 は決定 0084 の runtime で `MultiByteToWideChar` を通り、DllMain(ATTACH) の 127 回目の import `GetStringTypeW` で止まっている。ATTACH は完走しない（0084 後 §1 の 3、4、§5）。

## 決定

1. **同梱 DLL の扱い。** インストーラに同名のファイルがある同梱 DLL のうち、調査済みの `Blend2D.dll` と `QuickFontCache_x64.dll`（FLEngine 側の名前は `quickfontcache_x64.dll`）は、`dsp_ippv2_x64.dll` と同じ方向（決定 0073 の延長）で扱う。
   1. 「実機に無い」と**定義しない**。`fle_load_absent[]` に加えない。
   2. 仮想モジュール（`fle_modules[]` の表。実体の無い名前だけのモジュール）にも**しない**。
   3. 実体を lift して、2 つ目以降の実在モジュール（`fle_realmod`。決定 0080 のリベース方式）として載せる。「2 つ目以降」は、FLEngine を 1 つ目とする数え方（決定 0073）で、dsp_ippv2 が 2 つ目、Blend2D と QFC は調査が「3 つ目」と書くもの（Blend2D 調査 §10.1。QFC 調査 §7）。QFC 調査 §8 の Q2 は、決定 0080 D1 の基底 0x2000000 を「1 つ目の実在モジュールの基底」と書く（`fle_realmod` の表の中の数え方）。数え方は 2 通りあるが、指すものは同じ（dsp_ippv2、Blend2D、QFC）。`fle_realmod` の表では dsp_ippv2、Blend2D、QFC の 3 件になる。
2. **載せる時期と順序。** 実在モジュールとして載せるのは、実行がその `LoadLibraryW` に届いたときに、その順で行う（ブートの停止点の順。上の背景 §1 の表のとおり、FLEngine の LoadLibraryW は unit 144 の dsp_ippv2、unit 149 の Blend2D、unit 158 の quickfontcache_x64 の順に現れる）。**Blend2D と QuickFontCache のどちらを先にするかは、今は固定しない**（PM の文言のまま。この記録は、停止点の順という事実と、先にする方を固定しないという決定を、解釈を足さずに並べる）。
   - 制約: **QuickFontCache を実在モジュールにするには、先に dsp_ippv2 が実在モジュールとして動いていること**が要る。根拠は、QFC 自身が `dsp_ippv2.dll` を LoadLibraryW する形に読めること（QFC 調査 §1 の 11、§6.1、§8 の Q5、§9 の 3。**読み**。QFC の DllMain を実行した記録は無い）。
   - dsp_ippv2 は現在 DllMain(ATTACH) の途中（`GetStringTypeW`）で止まっており、完走していない（背景 §3）。**この前提が崩れたら見直す**（下の「見直す条件」）。
3. **値は決めない。** 基底アドレス、`fle_realmod` の表の上限、仮想ハンドルの領域との衝突の検証、縮小 DLL の種の定義、wasm の locals 上限への対処、Blend2D 固有の論点（Blend2D 調査 §10.3 の B1〜B13）は、**すべて別の決定**。下の「決めないこと」に列挙する。
4. **これは方向の決定**であり、「仮の決定」ではない。ただし、方向の根拠のうち実機で確認していない事項は「限界」に書く。
5. **実装はこの決定の後続作業**とする。

## 範囲

- **断定する対象は 2 つだけ**: `Blend2D.dll` と `quickfontcache_x64.dll`（どちらも専用の実現性調査がある）。
- **インストーラに同名がある同梱 DLL 14 名**（分類 §1 の 3。dsp_ippv2_x64、webview2loader、beat_tracker、blend2d、quickfontcache_x64、ilsvg_x64、ildsp_x64、oggio_x64、iltools_x64、wavpackdll_x64、lameenc_x64、python312、ilvideo_x64、speaker_x64）のうち、dsp_ippv2（決定 0073）と上の 2 つを除く **11 名**（webview2loader、beat_tracker、ilsvg_x64、ildsp_x64、oggio_x64、iltools_x64、wavpackdll_x64、lameenc_x64、python312、ilvideo_x64、speaker_x64）は、**実行がそこに届いて調査した時点で決める**。0086 は個別には決めない。
  - 「absent にも仮想にもしない」方向は、同じ根拠（インストーラに実在する。Windows の DLL ではない。分類 §6.2 冒頭）が当てはまる限り、同じである。ただし**断定は調査済みの 2 つに限る**。
  - 分類 §6.2 は 11 名の名前ごとに事実を並べるだけで、評価していない。名前ごとに事情が違う（例: beat_tracker は NULL で例外、ilsvg_x64 は NULL 検査なし、iltools_x64 はゲストが `hModule` = NULL の `GetProcAddress` を呼ぶ、ilvideo_x64 は名前が `ffmpeg\x64\` を含む、python312 は範囲が大きい）。同じ根拠がそのまま当てはまるかは、名前ごとに調べて決める。
- **扱わないもの**: Windows の DLL 12 名とインストーラに同名が無い FL の DLL 4 名（分類 §1 の 3、§6.1、§6.3）。特に unit 145 の `avrt`、`PowrProf.dll` の停止は、unit 149 より前に現れる別件（分類 §7.1）で、0086 は扱わない。
- `fle_modules[]` と `fle_load_absent[]` の中身は変えない。

## 決めないこと（すべて別の決定。ここには値を書かない）

| # | 未決の項目 | 現状と出典 |
|---|---|---|
| 1 | **基底アドレス** | QFC は 0x1d00000、Blend2D は 0x4700000 が、測定用コピーで `fe-pe rebase` が通った候補にすぎず、決定ではない（QFC 調査 §3.6。Blend2D 調査 §8 は 0x4700000 を「例」と書く）。QFC 調査 §3.6 の候補表には 0x1cc0000、0x1df0000、0x4700000、0x8000000 もある。0x4700000 は両方の調査が測定に使った値だが、3 つ（dsp_ippv2、QFC、Blend2D）を同時に置く割り付けは決まっていない（QFC 調査 §8 の Q2。Blend2D 調査 B7）。FLEngine と dsp_ippv2 の隙間は 0x340000 B で、QFC（0x20e000 B）は入り、その残り 0x132000 B に Blend2D（0x300000）は入らない（算術。QFC 調査 §1 の 3、§3.6）。決定 0080 D1 の 0x2000000 は dsp_ippv2 の基底（1 つ目の実在モジュール） |
| 2 | **`fle_realmod` の表の上限** | 現在 4 件（`lift/runtime/fle_realmod.c` の `RM_MAX 4u`）。dsp_ippv2、Blend2D、QFC で 3 件なので当面は足りる。**4 件を超える場合は別の決定**。ビルドスクリプト `gen_two.sh` は実在モジュールを 1 つしか取らず、3 つ目以降を足したときに `merge_tables.py` が通るかは未確認（Blend2D 調査 §8。QFC 調査 §8 の Q2）。決定 0082 の限界も複数の実在モジュール（`RM_MAX` 4）は未確認と書く |
| 3 | **仮想ハンドルの領域との衝突の検証** | 実在モジュールのハンドルは基底（決定 0080 D5）。仮想ハンドルのトークン（0x7F990000 以上）、モジュールハンドル（0x7FF800000000 + 0x10000×N）、擬似 VA の領域との関係は QFC 調査 §3.6 の表。追加する実在モジュールの基底についての検証は、基底の決定（上の 1）と合わせて別に行う |
| 4 | **縮小 DLL の種の定義** | 決定 0080 D7（当面は縮小 DLL で進める）の流儀を 2 つの DLL にどう当てるかは未決。Blend2D は B2、B5（Blend2D 調査 §10.3。§3.2、§10.5）。QFC は非 pdata の reloc 値を種にすると `bad_nodes` が 0 → 17,541 になる（QFC 調査 §4.2。`--pdata` だけで起動の関数は足りている読み） |
| 5 | **wasm の locals 上限（1 関数 50,000）への対処** | Blend2D は 4 関数が超える（最大 102,086。Blend2D 調査 §3.6、B2。近似の測定）。QFC は**未測定**で、最大関数 `f_522e50` は 13,559 行、超えるかは言えない（QFC 調査 §4.5。この関数は dsp_ippv2 の 420 名を引くローダーで、QFC の unit 88 から呼ばれる読み） |
| 6 | **Blend2D 固有の論点** | Blend2D 調査 §10.3 の B1〜B13。B1 JIT（AsmJit）、B2 描画の実体 4 関数の locals、B3 `jmp reg` の switch 表、B4 スレッド、B5 `.pdata` に無い葉関数 58 個を種にするか、B6 暗黙の TLS、B7 置き場所、B8 行なしの 17 名、B9 ロケールの初期化（別調査が扱う）、B10 使う側が NULL を検査しない、B11 CPUID を現行のままにするか（決定 0080 D8）、B12 `GetModuleFileNameW` の仮想パス（決定 0082 D6）、B13 停止のまま（Blend2D 調査の選択肢 (c)）を選ぶ場合の影響。**すべて未決** |
| 7 | **QFC 固有の論点** | QFC 調査 §8 の Q2〜Q5 と §10。行なしの import 87 名と遅延 import の行なし 4 名（Q3）、DllMain を通すために要るもの（Q4。入れ子の `LoadLibraryW`、`GetModuleFileNameW`、ロケール、`gs:[0x58]`、CPUID、`jmp` 未解決 2 か所、locals、メモリ）、dsp_ippv2 を QFC も読む点（Q5）。**すべて未決** |
| 8 | **QFC 自身が読む DLL** | QFC は `freetype.dll`、`Elastique.dll`、`Python312` を自分で読み込む。`freetype` と `Elastique` は現行の runtime に名前が無く、`fle_load.c` が名前付き停止にする（QFC 調査 §6.1。読み）。これらの扱いは 0086 では決めない |
| 9 | **残り 11 名の同梱 DLL** | 範囲の節のとおり。実行が届いて調査した時点で決める |

## 却下した選択肢

0086 の選択肢の名前（(a)〜(d)）は、調査文書の選択肢の名前（分類 §6 は (a) 仮想、(b) absent、(c) 停止のまま、(d) lift。QFC 調査 §8 の Q1 は (a) lift、(b) 仮想、(c) absent、(d) 停止のまま）と違う。対応は右の列。却下の理由は、調査文書と決定 0073 に書かれている事実に限る。

| | 内容 | 結論 | 調査文書の対応する選択肢 |
|---|---|---|---|
| (a) | 停止のまま | **却下ではなく、現行の runtime の挙動（単に未着手）**。決定ではない。0086 が定める方向の実装が入るまで、この停止は続く。停止は測定としては正しいが、停止点より先は測れない（QFC 調査 §8 の Q1(d): unit 158 より先（unit 159 以降）が測れない。Blend2D 調査 B13: unit 149 の後の全体の観測が先へ進まない） | 分類 §6 の (c)。QFC 調査 Q1 の (d)。Blend2D 調査 B13 |
| (b) | `fle_load_absent[]` に足す（NULL と 126） | **却下** | 分類 §6 の (b)。QFC 調査 Q1 の (c) |
| (c) | 仮想モジュール（`fle_modules[]`）にする | **却下** | 分類 §6 の (a)。QFC 調査 Q1 の (b) |
| (d) | 自前の実装に置換する | **却下** | 調査文書に対応する選択肢は無い（どの調査もこの選択肢を比べていない） |

### (b) の却下の理由

- 両 DLL はインストーラに実在する（Blend2D 調査 §2。QFC 調査 §2。実機での読み込みの成否は未確認で、「限界」に書く）。「無い」と答えるのは、実機の DLL の有無について偽の主張になる。これは決定 0073 が dsp_ippv2 について採らなかった理由で、同じ形で当てはまる（分類 §6.2 冒頭）。
- QFC は、決定 0055 の (a)「ゲストが NULL を許容する」を満たさない。ゲストの経路には、NULL で 0 番地呼び出しに進む 4 名がある（QFC 調査 §8 の Q1(c)。§5.2）。unit153 §6.3 の測定でも、仮の NULL の後は `MessageBoxW`、続いて `GetProcAddress(NULL, …)` の abort に進む。
- Blend2D は、ロード点は NULL を許容する（分類 §2.2 の #4）が、使う側 `f_6cb0b0` は `AggCreateInstance` のスロットが 0 のとき 0 番地を呼ぶ。呼び出し元 77 関数が `f_664190` の false を見て描画を止めるかは未確認（Blend2D 調査 §4.2、B10）。

### (c) の却下の理由

- 決定 0073 は dsp_ippv2 について、仮想モジュール（偽の応答表）の形を却下した（「実機の DLL の有無について偽の主張」「関数の意味を runtime が創作する」）。Blend2D と QFC は Windows の DLL ではなく、「Windows 10 には必ずある」とは言えないので、決定 0055 の流儀（Windows の DLL を仮想モジュールにする）には当たらない（分類 §6.2 冒頭）。
- QFC: FLEngine が引く 8 名は、FLEngine が実際に呼ぶ実体（フォントの取得、スレッドバンク、アニメ用タイマー、VST3 など）なので、応答表の行を作るなら 8 つの関数の意味を runtime が創作することになる（QFC 調査 §8 の Q1(b)。読み）。応答表に行が無いままなら、`GetProcAddress` は NULL で、§5.2 の 4 名が 0 番地を呼ぶ形になる。
- Blend2D: 仮想モジュール案だけを個別に比べた記述は、Blend2D 調査には無い。この記録が引ける事実は 2 つ。(1) 分類 §6.2 冒頭の一般論（Windows の DLL ではないので、仮想モジュールは決定 0055 の流儀に当たらない）。(2) 応答表に行が無いままなら、`GetProcAddress` は NULL で、使う側 `f_6cb0b0` は 0 番地を呼ぶ（Blend2D 調査 §4.2。読み）。Blend2D に固有の「創作になる」という理由は、調査に書かれていないので立てない（参考の事実: `AggCreateInstance` はヒープに 0x1170 バイトを確保して Agg2D のコンストラクタを tail call し、vtable は 130 エントリ。Blend2D 調査 §5.1。読み）。

### (d) の却下の理由

- 調査文書は、Blend2D と QFC を自前の実装に置換する案を比べていない。したがって却下の理由として、調査の事実は言えない。
- 決定 0073 の却下 (c)（自前の IPP 実装への置換）は 3 点を挙げた。そのうち 3「実機にある実在のコードを、創作で置き換えることになる。本リポジトリの方針（創作しない。仕様から言えないことを事実として書かない）に反する」は、置換という選択肢の形についての理由で、この記録はそれを 0073 から借りる（Blend2D と QFC に当てはまるかを個別に調べた調査文書は無い。PM の方向は (d) を採らない）。
- 0073 の 1（bit 一致の根拠が無い）と 2（規模が大きい。420 名）は IPP について書かれた理由で、Blend2D と QFC については調査文書に書かれていないので、この記録は立てない。

## 限界

- **実機の Windows 10 22H2 で `Blend2D.dll`／`QuickFontCache_x64.dll` が FL の同梱ファイルとして存在すること**は、確認していない。インストーラに同名のファイルが入っていることは確認済み（Blend2D 調査 §2。QFC 調査 §2）だが、**インストール先やロード経路が実機でどうなるかは未確認**。実機では確認していない（ホストは Linux）。
  - 配置先の NSIS 変数がどの変数か（Blend2D 調査 §2。QFC 調査 §2）。
  - `f_6539e0` が探す 4 つのディレクトリ候補は読んでいない。インストーラにあることは、ゲストが実行時にそのファイルを見つけることを意味しない（分類 §5.1 末尾。QFC 調査 §10）。
  - `Blend2D.dll` は `f_6539e0` の `dl` が不定で `_x64` が付かない読みで、LoadLibraryW に渡る名前の実測は gdb の 1 例（Blend2D 調査 §2）。
  - 別インストーラ、ダウンロード、実行時に配置される DLL は未確認（分類 §5.2）。
  - QFC の SHA-256 の同一性は、このインストーラの 1 版（26.1.7.5653）から抽出した 1 つだけ（QFC 調査 §10）。
- unit 149 と 158 の LoadLibraryW が現れる順は、仮の値を入れた構成の測定である（分類 §0.2 末尾、§7.1。unit153 §6.1）。この記録は、仮の値なしの構成で 149 や 158 に届いた測定を読んでいない（背景 §1）。仮の値は決定でも runtime の挙動でもない（分類 §0.2 末尾）。
- 2 つの DLL を実行していない。Blend2D 調査と QFC 調査は、静的な読み（到達、CPUID、DllMain、依存）が中心で、統計だけが実測である（Blend2D 調査 §10.4。QFC 調査 §9 の 6、§10）。QFC の DllMain の途中の `LoadLibraryW`（dsp_ippv2、freetype ほか）は読みにとどまる。
- 「先に dsp_ippv2 が実在モジュールとして動いていること」の「動いている」の線（DllMain が戻ること、など）は、この決定では引かない（PM の文言のまま）。
- 「現在」の停止（背景 §1、§3）は、決定 0084 の runtime での測定（0084 後 §1、§6）を引いた。決定 0085（`HKLM\Software`）は unit 153 の `RegCreateKeyExW` に関するもので、dsp_ippv2 の DllMain の停止より後ろの話であり、この記録は 0085 の後の runtime で測り直していない。

## 見直す条件

- （PM の指示。決定 2 の末尾）決定 2 の制約の前提が崩れたとき。例: QFC が `dsp_ippv2.dll` を LoadLibraryW する形（読み）が実行で違うと分かったとき。dsp_ippv2 が実在モジュールとして動かないと分かったとき（決定 0073 の撤回条件と重なる）。
- （決定 0073 の撤回条件の形に倣った作業者の記載。PM の指示文には無い。PM が不要と判断すれば削除してよい）実機の Windows 10 22H2 で、`Blend2D.dll` または `QuickFontCache_x64.dll` が FL の同梱ファイルとして存在しない（LoadLibraryW が失敗する）ことが、一次の根拠で確認されたとき。その DLL について PM に報告し、上の選択肢を改めて比べる。
- （同上。0073 の撤回条件の形に倣った作業者の記載で、PM の指示文には無い）後続作業で、Blend2D または QFC を同じツールチェーンで lift して wasm で動かすことが不可能と分かったとき。その DLL について PM に報告し、選択肢を改めて比べる。
- PM が方針を変えたとき。

## 関連する決定への影響

- **決定 0073**: 0086 は 0073 の方向を、dsp_ippv2 から Blend2D と QFC に**広げる（拡張する）**。0073 の本文は書き換えない。0073 への注記も足さない。関連は `docs/README.md` の索引で示す。
- **決定 0080**: 変えない。D2（ファイル段階のリベース）を 2 つの DLL にも使う方向（PM の指示の (iii)）。D1 の 0x2000000 は dsp_ippv2 の基底で、2 つの DLL の基底は決めない。D5（モジュールハンドルは基底）、D7（縮小 DLL）、D8（CPUID は現行のまま）、D9（スレッドは現行のまま）、D10（メモリ）の 2 つの DLL への当てはめは、0086 では決めない（調査が論点として挙げる箇所: Blend2D 調査 B4、B11、QFC 調査 §8 の Q2、Q4）。
- **決定 0081**: 変えない。
- **決定 0055**: 変えない。仮想モジュールにする基準（環境の定義と矛盾しないこと）は Windows の DLL に対するもので、Blend2D と QFC はそれに当たらない（分類 §6.2 冒頭）。
- **決定 0015、0029**: 変えない。0029 は「外部 DLL（プラグインと道具）への応答は M2 の判断で、この決定では作らない」と書き、仮想モジュール表にも `fle_load_absent[]` にも無い名前は `fle_stop` で止める（0029 の判断 1 の区分 3、判断 2）。0086 が方向を決めるのは `Blend2D.dll` と `quickfontcache_x64.dll` の 2 名だけで、それ以外の名前の扱いは 0029 のまま。
- **決定 0082**: 変えない。実在モジュールのロードの意味（D6 の `GetModuleFileNameW`、D11 の `FreeLibrary`、DllMain 内の `LoadLibraryW` の数え方）は 0082 のまま、2 つの DLL への当てはめは別の決定。
- この決定の作業では `lift/` 配下を一切編集していない。実装、テストの固定数の更新（分類 §0.1）は、後続作業で行う。
