# pdata 外の this 補正 thunk（adjustor thunk）の調査

作成: 2026-10-21。対象 `binaries/win64/FLEngine_x64.dll`（22,227,016 B）。判断は `docs/decisions/0051-adjustor-thunks.md`、実装と再実行は `docs/investigations/adjustor-thunks-run.md`。「実測」はコマンドの出力、「読み」は静的な読み、「推測」は未検証。実機の Windows では確認していない。

測定の道具は Python の `pefile` と `capstone`（`lift/out-adj/probe1.py`、`survey.py`、`survey2.py`、`survey3.py`、`scan_all.py`、`stats.py`）。Rust の実装（iced-x86）とは別系統で、数が一致することを実装後に確かめた（run 記録 §3）。

## 0. 依頼文との食い違い（先に記す）

依頼文は「THUNK（c8a7b86）が導入した ADJUSTOR の seed 方針」「`lift/crates/fe-data/src/roots.rs` の ADJUSTOR 判定」「docs/investigations/thunks.md 相当」を前提にしていた。リポジトリには次のものが**無い**（実測）。

- `lift/crates/` は `fe-pe`、`fe-decode`、`fe-lift` の 3 つだけ。`fe-data` も `roots.rs` も無い。
- コミット `c8a7b86` は存在しない（`git log --all` は 40 件、`git show c8a7b86` は unknown revision）。`ADJUSTOR`／`adjustor` は rs と md のどちらにも 0 件。`thunk` を題にしたコミットは `fe9a1f6`（IAT の `jmp [slot]` の棚卸し）だけ。
- `indirect_ptrs` を含むファイルは 0 件。決定 0006 は生成器の import 対応表の決定で、garbage root の経緯は書かれていない。garbage root に当たる記録は `docs/investigations/data-pointers-pdata-outside.md`（§6 の結論）。

したがって「ADJUSTOR がなぜ 0x403582 を拾わなかったか」は、**拾う方針がまだ無かった**が答え。seed の出どころは、PE のエントリと export（`analyze`）、`.pdata` の関数開始（決定 0031、`pdata_seeds`）、`--seeds` ファイルの 3 つだけ（`fe-decode/src/lib.rs`、`fe-lift/src/main.rs`）。決定 0031 の「検討した代替」は「関数開始に pdata 外のサンクを足す: 足さない。閉包は直接 call 先を辿る」と書いており、これは **IAT の `jmp [slot]` thunk**（呼び出されるもの）の話。this 補正 thunk は直接 call されず、データからだけ参照されるので、この理由が当てはまらない。

## 1. 0x403582 は `.pdata` に無い（実測）

- `.pdata` は RVA 0x1607000、0xa9620 B、57,816 エントリ（`fe-pe pe-pdata-info`: entries 57816、chained 0）。最小の開始は 0x407380。**0x403579、0x403582、0x40358b は最小の開始より前**で、どの RUNTIME_FUNCTION の範囲にも入らない（`probe1.py`）。
- 3 つの thunk（バイト列は DLL から読んだもの）:

| VA | バイト | 命令 | jmp 先 | 先の `.pdata` |
|---|---|---|---|---|
| 0x403579 | `48 83 c1 f0 e9 fe 8b 01 00` | `add rcx,-0x10; jmp 0x41c180` | 0x41c180 | 開始（〜0x41c19b） |
| 0x403582 | `48 83 c1 f0 e9 15 8c 01 00` | `add rcx,-0x10; jmp 0x41c1a0` | 0x41c1a0 | 開始（〜0x41c1af） |
| 0x40358b | `48 83 c1 f0 e9 1c 8c 01 00` | `add rcx,-0x10; jmp 0x41c1b0` | 0x41c1b0 | 開始（〜0x41c20e） |

- 0x403598 以降の 3 要素の表は `0x403579, 0x403582, 0x40358b`（各 8 B）。**この表自身が `.text` セクションの中にある**（0x403598 は `.text` 0x401000〜）。3 つの値は IMAGE_REL_BASED_DIR64 の再配置が付いたスロット（0x403598、0x4035a0、0x4035a8）。
- 直前 0x403578 と直後 0x403594〜0x403597 は `int3`。

## 2. 同形 thunk の全数（実測）

### 2.1 母集団

- 再配置: 278,531 件が DIR64、993 件が ABSOLUTE（パディング）。他の型は 0。
- DIR64 スロットの値が `.text` に入るもの: **252,486 スロット、distinct 80,188 値**。
  - `.pdata` の関数開始: 24,592 値（すでに seed）
  - `.pdata` の範囲の内側（開始でない）: 10 値
  - どの範囲にも入らない: 55,586 値。大半はデータ（UTF-16、ASCII 等。`data-pointers-pdata-outside.md` と同じ傾向）。
- 参照元スロットの置き場所: 再配置を **すべてのセクション**から集めた。thunk を指す 602 スロットは**全部 `.text` の中**にある（0x403598 など）。前回の調査（`data-pointers-pdata-outside.md` §0）は「非 .text ページ」の DIR64 だけを母集団にしていたので、`.text` 内の表は入っていなかった（読み。前回の 6,974 値と今回の 80,188 値の差はこの定義の違いによる）。

### 2.2 thunk の形の判定と結果

判定: 候補アドレスから最大 4 命令をデコードし、（`add`/`sub` reg, imm、`lea r64,[base+disp]`、`mov r64,[base+disp]`）の 1〜3 個のあとに、直接 near `jmp`、で終わるもの。`.pdata` の開始でも範囲内でもない。`jmp` のみ（補正命令なし）は除く。

| 区分 | 件数 |
|---|---|
| 候補（参照される、`.pdata` の開始でない、上の形） | 604 |
| うち `.pdata` の範囲の外 | **602**（すべて `add rcx, imm; jmp rel32`） |
| うち `.pdata` の範囲の内側 | 2（0x40c6fc `add ebx,1; jmp 0x40c746`、0x882586 `mov rdx,[rbp+0x80]; jmp 0x88259a`。どちらも参照元は `.data`、関数の途中でコードの一部。seed にしない） |
| `sub`／`lea`／`mov` 形で範囲外 | **0** |
| 参照されない同形（`scan_all.py`: `.pdata` 範囲外の全バイト位置から探した一致のうち、参照されないもの） | 604（例 0x40357a、0x403583。どのスロットも指さない。内訳は §4） |

- 602 件は、サイズ 9 B（467）か 12 B（135）。レジスタはすべて `rcx`。
- `rcx` に足す即値は 37 通り、負の値のみ。上位: -0x20（135）、-0x28（68）、-0x30（56）、-0x18（32）、-0x48（31）、-0x38（30）、-0x50（30）、-0x40（22）、-0xa8（17）、-0x58（15）、-0x10（14）、-0x540〜-0x558（計 51）。
- jmp 先: distinct 275、**全部 `.pdata` の関数開始**（`.text` 内）。最頻は 0x41c180、0x41c1a0、0x41c1b0 が各 96 件、0x41c210、0x41c230、0x41c240 が各 5 件、0xbf1850、0xbf1860 が各 8 件。0x41c1xx の 3 つは、0x403579 などの jmp 先と同じ（IUnknown 相当の共通の 3 関数と読めるが、推測）。
- 参照のされ方: 602 件のどれも、DIR64 スロット 1 つだけから参照される（reloc 1、平文のスロット 0）。
- 602 スロットは、8 B 間隔で連続する並び（「連」）が 90 個。連の長さの分布: 4 が 37 連、3 が 13 連、7 が 7 連、2 が 6 連、6 が 5 連、8 が 5 連、5 が 4 連、20 が 2 連、14 が 2 連、1 が 1 連、9・10・11・17・18・21・25・84 が各 1 連。**0x403598 の 3 連は 3 要素の連の 13 個のうちの 1 つ**。連は表と一致するとは限らない（表の他の要素が thunk でない関数のこともありうる。表の数は主張しない）。
- thunk のアドレス範囲: 0x403579〜0x136bf04。

### 2.3 同形でないが「参照される .pdata 外の短い thunk」

最初の広い走査（`survey.py`: 5 命令以内で jmp か ret に終わる、補正命令の種類を問わない）で、`.pdata` 範囲外の候補は次のとおり。**seed にしない**。

| 区分 | 件数 | 読み |
|---|---|---|
| `cwde; ret 0x15e`（`98 c2 5e 01`） | 604 | 同じ 4 バイトの値の並びで、命令ではなくデータの偶然（読み）。参照元は `.data` の `0x15e....`（スロット値が 0x425ab0 など） |
| `push rdx; lea rdx,[rip+disp]; jmp rel32` | 57 | 参照元は `.didata`（遅延 import 領域）の 2 スロット。遅延 import の stub と読める（推測） |
| `jmp [rip+disp]`（補正なし） | 7 | IAT 経由の import thunk |
| その他（`add r12b,r15b; jmp [rsi-0x30]` など、`add byte ptr` の列） | 7 | バイト列がデータ（誤デコード） |

## 3. 既存の方針との関係

- 決定 0031 の seed は `.pdata` の開始（57,816）。この 602 件は含まれない（実測）。
- 前回の「データ内ポインタ」調査（garbage root、`data-pointers-pdata-outside.md`）の教訓は、**値が `.text` に入るだけでは関数でない**（6,110 件で強い基準を満たすものは 0）。今回の判定は「DIR64 の参照」+「命令の形が thunk」+「jmp 先が `.pdata` の開始」の 3 条件を要求する。第 3 条件は、データの偶然では満たしにくい強い条件（読み。確率は測っていない）。
- 実測の補強: 同じ形を「参照なし」で全バイト位置から探すと 1,206 件で、参照されるのはちょうど 602 件。残り 604 件は命令境界でない位置（REX をまたいだ読み）。参照を要求しないと境界の誤りが混ざる（`scan_all.py`）。

## 4. 限界

- 参照元は DIR64 の再配置が付いたスロットだけを seed の根拠にした。再配置の付かない 8 B 値（`.data`/`.rdata`/`.didata` の整列スロット）の走査では、thunk 形で `.pdata` 外のものは 2 件（0x40c6fc、0x882586。どちらも範囲内側）。つまり再配置なしで参照される `.pdata` 外の thunk は 0 件（`survey2.py`）。
- 参照元が `.text` 内の表であることまでは確かめたが、それが Delphi の interface の vtable かは確かめていない（読み。メソッドの並びが 3 つ組（QueryInterface、AddRef、Release）に見えるだけ）。
- thunk の jmp 先の関数が「その Self 補正で呼ぶ関数」として正しいかは見ていない。命令列としての判定のみ。
- ret で終わる形は、参照される `.pdata` 外で 606 件（`survey.py`）。604 件が上の `cwde; ret 0x15e`（データの偶然と読む）、残り 2 件が `ret 0xb1b3`、`ret 0xf0`（同じくデータの偶然と読む）。補正命令を持つ ret thunk は 0 件。seed にしない。
- 「参照されない同形 604 件」（§2.2 の表）の全件の内訳は確認していない。先頭の例（0x40357a、0x403583、0x40358c…）は、参照される thunk の REX 接頭辞の次のバイトから読んだ `add ecx,imm; jmp`。`scan_all.py` の出力は `add` 1,204 + `add+add` 2 で、参照される 602 件は全部 `add`。
- §2.1 の「前回の 6,974 値と今回の 80,188 値の差は定義の違い」は読み。前回のスクリプト（`lift/out-ptr/classify_pdata_outside.py`）での再現はしていない。
