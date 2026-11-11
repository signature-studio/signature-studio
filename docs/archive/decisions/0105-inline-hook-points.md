# 0105: ゲスト自身のインライン フック（関数先頭 5 バイトを `jmp rel32` に書き換える自己書き換え）を、註釈したフック点の入口の検査と、そのフック点に限った `VirtualProtect` の応答で扱う — 偽の成功も、環境の定義の変更も採らない

状態: **仮の決定**（PM の指示（2026-10-27。番号 0105 は PM の仮の割り当て）を、作業者（サブエージェント）が設計・実装し、第 2 段階（コンパイル・テスト・実測）まで実施した）。**第 2 段階の実測を反映した版。** 基準の木は HEAD `3772c2a` の統合木（`lift/out-git/integ1/lift`）に `resolved-H.diff` を当てたもので、HEAD に対する差分は `H-final.diff`（11 ファイル、1,506 行。`patch -p1` で当たる）。**生成器・runtime はコンパイルしてテストした**（`hook_points.rs` 16 件、`hook.rs` の単体 7 件が通った。全体テストは 1 回では通せなかった。下の「テスト」）。実在のハードウェア・実在の Windows とは照合していない。

**PM の決定（2026-10-27、第 2 段階の依頼で受けた）**: (1) 先頭 5 バイトが元と違い、かつ `e9` でもない場合は**名前付き停止**にする（第 1 段階の実装のとおり。決定 0046 方針 4 の fail-soft を避けるため。PM の元の文言「そうでなければ本体へ」からの逸脱は PM が承認した）。(2) 登録方式（`static const` の表 + 末尾の constructor）、セクション情報を生成器が表に書くこと、`fle_realmod` の表に触れないこと、`dwSize > 5` は停止、はすべて第 1 段階の実装のとおり。(3) **旧保護の実機での値（`PAGE_EXECUTE_WRITECOPY` になるか等）は一次資料で確認できない**ので、この決定の「仮」の範囲に含め、撤回条件に入れる（下の決定 5、撤回条件 4）。

> **これは A4 の達成ではない。** この決定は、census の壁（FLEngine の unit 314）と、5 モジュール構成の壁（QFC の unit 162）が同じ型の `VirtualProtect` の停止であることを受けて、その型に対する答えを定める。実測では 5 モジュール構成の停止が `VirtualProtect` から動き、init_count が 159 から 246 に増えた（下の「実測」）が、これは仮の応答を含む構成での停止点の移動であり、A4 の達成ではない（決定 0046、0060、0101。完走は 895 unit）。census の数は診断で A4 の進捗ではない（決定 0097）。

## 事実の出典（読んだもの）

- 決定 0099（`docs/decisions/0099-wall-loader-batch2.md` の W4: unit 314 の `VirtualProtect`。調査と選択肢 (A)〜(E)。**決定しない**で PM に返した）、決定 0103（`docs/decisions/0103-qfc-real-module.md` の壁 13: QFC の unit 162 の `VirtualProtect`、選択肢 (a)〜(c)）。決定 0024（メモリ保護と `VirtualProtect`）、0045（wasm はページ保護を強制しない）、0046（方針 4: 黙って意味が変わる fail-soft を許さない）、0036 W1 と 0100（`jmp` の tail dispatch）、0080/0082/0104（実在モジュール）、0096（遅延モジュール）、0097（census）、0101（A4 の判定構成）。
- **読み（第 1 段階。実測。PE を pefile と capstone で読み取り、読み取りのみ）**: FLEngine（SHA-256 `b2c64da4…9496d`）の unit 314 の proc `f_8e48a0`、フックのクラスの `f_8e0c60`／`f_8e0de0`／`f_8e0db0`／`f_8e0d80`／`f_8e0ed0`／`f_8e0ef0`。QFC（元のファイル `quickfontcache_x64.dll`、SHA-256 `afc5a0ed…736d2`、ImageBase 0x400000）の unit 162 の proc `f_5992d0`、`f_597b10`／`f_597c90`／`f_597c60`／`f_597d80`／`f_597da0`。逆アセンブルの要点は下の「背景」。
- **実測（第 2 段階）**: 下の「第 2 段階の結果」。ログは `lift/out-git/h2-results/`（本書と同じ作業の成果物）。
- Microsoft Learn（2026-10-27 に読んだ）: VirtualProtect（`lpflOldProtect` は「指定した範囲の**最初のページ**の以前の保護」、`dwSize` は「`lpAddress` から `lpAddress+dwSize` の範囲に 1 バイトでも含むページすべて」）、Memory Protection Constants（「保護は部分ではなく**ページ全体**に与える」、`PAGE_EXECUTE_READ` = 0x20、`PAGE_EXECUTE_READWRITE` = 0x40、`PAGE_EXECUTE_WRITECOPY` = 0x80）、PE Format の Section Flags（`IMAGE_SCN_MEM_EXECUTE` 0x20000000、`MEM_READ` 0x40000000、`MEM_WRITE` 0x80000000）。**どの Learn のページにも「読み取り + 実行のセクションが `PAGE_EXECUTE_READ` でマップされる」とは書かれていない**。これは推測（下の限界）。

## 背景（事実）

### 2 件の壁は同じ型

| | FLEngine unit 314 | QFC unit 162 |
|---|---|---|
| 停止 | `fle: VirtualProtect: address is not in a VirtualAlloc region (the runtime does not track other memory)`（census。決定 0099 W4） | 同じメッセージ（5 モジュール構成。決定 0103 壁 13。`f_1d1dae0` ← QFC `f_597c90` ← `f_597b10` ← `f_5992d0`。ここの VA は ImageBase 0x400000 のファイルの値で、0x1d00000 に載せると RVA 0x1dae0 などになる）。**第 2 段階で HEAD の木に対して再現した**: 註釈なしの 5 モジュール構成は 159 / 942 で停止し、スタックの先頭は `f_1d1dae0 ← f_1e97c90 ← f_1e97b10 ← f_1e992d0 ← f_1d0f390 ← f_1d0f430 ← f_1d19dc0 ← f_1e9c210 ← f_1e9c1f0 ← f_423990`（0x1d00000 に載せた VA） |
| 静的初期化の proc | `f_8e48a0` が、クラス参照 `[0x8d9ba8]` のフック オブジェクトを `f_8e0c60(cls, dl=1, target, replacement, byte [rsp+0x20]=0)` で 3 つ作る（呼び出し 0x8e4a17、0x8e4a3f、0x8e4a67） | `f_5992d0` が、クラス参照 `[0x593ad8]` のフック オブジェクトを `f_597b10(cls, 1, target, replacement, 0)` で 3 つ作る（0x599447、0x59946f、0x599497） |
| フック点（target。`lea r8,[rip+d]` の読み）→ 置き換え先（`lea r9,[rip+d]`） | **0x41c820 → 0x8dad80、0x43b130 → 0x8dac70、0x43b150 → 0x8dace0**。6 つとも `.pdata` の関数の開始。0x41c820 の先頭 5 バイトは `55 56 53 c6 44`（`push rbp; push rsi; push rbx; mov byte [rsp-0x20],0` の途中）、0x43b130 は `53 48 83 ec 20`、0x43b150 は `55 48 83 ec 50` | **0x418e80 → 0x594870、0x430ec0 → 0x594760、0x430ee0 → 0x5947d0**（ファイルの VA）。RVA は **0x18e80、0x30ec0、0x30ee0**（測定用基底 0x1d00000 では 0x1d18e80、0x1d30ec0、0x1d30ee0）。6 つとも `.pdata` の関数の開始（元のファイルと基底 0x1d00000 のファイル（置き換え先は 0x1e94870、0x1e94760、0x1e947d0）の両方で確認）。先頭 5 バイトは FLEngine の 3 つと**同じ**。同じ Delphi RTL の関数の、各モジュール自身の複製 |
| 準備（コンストラクタ） | `f_8e0c60` → `f_8e0de0`: `[obj+8]` = target、`[obj+0x10]` = replacement。`[obj+0x24]` が 0 でなければ先に `f_8e0ed0`（`f_8e0d80`: 保存した元の 5 バイトを target に書き戻す）。`dl` 引数が 0 なので `jmp [rip+d]` の thunk は辿らない。`[obj+0x18]` = `e9`、`[obj+0x19..0x1c]` = `replacement − target − 5`、`[obj+0x1d..0x21]` = target の**元の 5 バイト**。そして `VirtualProtect(target, 5, 0x40, &old)`（thunk 0x423d00）。戻り 0 なら `f_44cb40`（エラー） | `f_597b10` → `f_597c90`（同じ構造。`[obj+0x28]` が patched target。VirtualProtect の thunk 0x41dae0） |
| 有効化 | `f_8e0ef0(cl=1, dl=0)`（呼び出し 0x8e4af1）が 3 つに `f_8e0de0` をもう一度掛け（`[obj+0x24]` ≠ 0 なので**先に元の 5 バイトを書き戻し**、保存し直し、`VirtualProtect` をもう一度呼ぶ）、続けて `f_8e0db0` が `[obj+0x18..0x1c]` の `e9 rel32` を target に書く。**有効化は `ParamStr(0)` の小文字が `delphi32.exe`、`kylix`、`bds.exe`（0x8e4b94、0x8e4bbc、0x8e4bd4。Delphi の `UnicodeString`）の**どれでもないとき**だけ**（0x8e4ab6、0x8e4acd、0x8e4ae4 の比較が 0 なら `je 0x8e4af6` で飛ぶ） | `f_597da0`（0x599521）。同じ 3 つの名前（0x5995c4、0x5995ec、0x599604） |
| 他の呼び出し元 | 実測: `f_8e0c60` を呼ぶのは 3 か所だけ（E8/E9 rel32 の走査。DIR64 の再配置にも rip 相対の `lea` にも、その関数のアドレスは無い）。target の 3 つのアドレスを取るのは上の `lea` だけ（DIR64 なし） | `f_597b10` は 3 か所だけ。`VirtualProtect` の thunk 0x41dae0 の呼び出し元は `f_597c90` の 1 か所だけ（IAT スロット 0x5db800 を参照するのは thunk 1 つ） |

「この `VirtualProtect` の呼び出し元のうちフック点を準備するもの」は、FLEngine で 3 点、QFC で 3 点（上の表）。QFC の `VirtualProtect` はこの 1 か所だけ。FLEngine には他の使用者がある（下の「読みで気づいた不整合」）。

### リフト後の C がフックを黙って無効にする

リフト後の C はメモリ上の命令を読まない。`f_41c820()` は C の関数呼び出しで、`.text` に `e9 rel32` が書かれても元の本体を実行する。`VirtualProtect` に偽の成功と答えるだけだと、ゲストは `.text` に 5 バイトを書き（native のハーネスはイメージを RWX で mmap、wasm はページ保護を強制しない（決定 0045））、リフト後の C はそれを実行せず、**フックが黙って無効になる**（決定 0046 方針 4 に反する）。環境の定義を変えて（`GetModuleFileNameW(NULL)` を `delphi32.exe` にして）ゲストにフックを無効にさせる案（0099 の (E)）は、実機（`ParamStr(0)` が `delphi32.exe` でも `bds.exe` でも `kylix` でもない FL Studio 本体）と違う状態を作るので採らない。実機ではフックが**有効**で、置き換え先（0x8dad80 など。文字列クラスの関数を横取りする層。0099 の読み）が走る。

## 決定（作業者。仮。PM の決定を反映）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 方針 | 偽の成功も、環境の定義の変更も、フックの無効化もしない。**ゲストが書いた `jmp rel32` を、生成 C が実行する**。そのために、(a) フック点（ゲストが先頭 5 バイトを書き換える関数入口）をモジュールごとに**事前に註釈**し、(b) 註釈した入口に検査を入れ、(c) `VirtualProtect` はその 5 バイトの範囲にだけ答え、註釈の無いイメージ内のアドレスは名前付きで停止する |
| 2 | 註釈（`fe-lift lift ... --hook-points FILE`） | 1 行 1 アドレス。16 進（`0x` 付きでもなくても。リフトするファイルの ImageBase での VA）、または `rva:` + 16 進（ImageBase を足す）。`#` 以降はコメント、空行は無視。RVA の行を作ったのは、基底が測定用の値（QFC の 0x1d00000 など）でも同じファイルを使えるようにするため（`lift/harness/hooks/flengine-hook-points.txt`、`qfc-hook-points.txt`）。`fe-lift` は VA が **(i) 閉包の関数入口である（`--pdata` か `--seeds` で入る）、(ii) 読み取り + 実行（書き込み不可）のセクションの中にある、(iii) ファイルに裏づけのあるコードで先頭 5 バイトがある、(iv) 元の先頭バイトが `0xE9` ではない**ことを検査し、どれかが偽なら**エラー終了**（註釈の誤り。黙って落とさない）。同じ VA の重複は 1 つにまとめ、5 バイト未満の間隔の 2 点はエラー。元の 5 バイトは PE のファイルから読む |
| 3 | 生成 C: 入口の検査（**PM 承認**: (c) の名前付き停止） | フック点 X の関数 `f_X` の**最初の命令のブロックの中**（`L_X: { ここ /* 0xX Mnemonic */ ... }`。ブロックの中なので、関数の先頭へ戻る `goto L_X` もこの検査を通る）に `{ uint64_t fle_hk = fle_hook_target(UINT64_C(X), UINT64_C(orig5)); if (fle_hk) { fle_dispatch(fle_hk); return; } }` を出す。`orig5` は元の 5 バイトを下位から並べた 64 ビット値。`fle_hook_target`（runtime）は、イメージの X〜X+5 が **(a) orig5 と同じ → 0（本体へ）、(b) 違って先頭が 0xE9 → X + 5 + rel32（符号つき 32 ビット）を返す → `fle_dispatch(target); return;`（tail dispatch。スタックとレジスタは入口のまま）、(c) 違って先頭が 0xE9 でもない → 名前付き停止**（`fle: inline hook: hook point 0x… : the first 5 bytes are now … ; the original … and 'e9 rel32' are the only ones modelled (decision 0105)`）。(b) の target が 0 のときも停止。target が関数入口でなければ `fle_dispatch` の既存の停止（`indirect call to … is not a known function entry`）に任せる |
| 4 | 生成 C: 表と登録（**PM 承認**） | フック点のあるモジュールの生成 C だけ、`#include "fle_vmap.h"` を `fle_dispatch.h` の次に出し、**最後の関数の後**に `static const fle_hook_point fle_hook_points_<ImageBase の 16 進>[] = { { VA, orig5, prot }, ... };` と、`static void __attribute__((constructor)) fle_hook_init_<…>(void) { fle_hook_register(ImageBase, SizeOfImage, 表, 点の数); }` を出す。`prot` は 0x20（`PAGE_EXECUTE_READ`。決定 2 の (ii) で読み取り + 実行のセクションに限るので定数。**この値は仮**: 決定 5）。フック点を持たないモジュールの C は**1 バイトも変わらない**（`--hook-points` を付けない出力は旧出力と一致。第 2 段階で FLEngine の C について SHA で確認した） |
| 5 | runtime（`runtime/fle_vmap.c`、`fle_vmap.h`。新しい .c/.h は作らない。**`dwSize > 5` は停止: PM 承認**） | `fle_hook_register`（イメージ最大 8 つ）、`fle_hook_target`。`VirtualProtect`（`fle_imp_VirtualProtect`）は、アドレスが登録したイメージ（base 以上 base+SizeOfImage 未満）の内側なら次のとおり。**(a) アドレスがフック点に一致し、`dwSize` が 1〜5**: 成功（戻り 1）。旧保護 = そのページの現在の保護（初めてなら表の `prot` = 0x20。以前に `VirtualProtect` が設定したページはその値）を `lpflOldProtect` に書く。新保護は範囲の**各ページ**に記録する（強制しない。ページ単位なので、同じページの 2 つのフック点は互いの設定を見る）。`lpflOldProtect` が NULL なら失敗（0、`ERROR_INVALID_PARAMETER` 87。同じ関数の既存の扱い）。**(b) フック点に一致しない**: 名前付き停止 `fle: VirtualProtect: inline hook at an address that is not an annotated hook point (decision 0105): address 0x…, size N, new protection 0x…, image base 0x…`（`lpflOldProtect` が NULL でも停止）。**(c) フック点に一致するが `dwSize` が 5 より大きい**: 名前付き停止（`range of N bytes at the hook point 0x…; a hook is the 5 bytes of jmp rel32`）。イメージの外（VirtualAlloc の領域、ビュー）は従来どおり。**旧保護の値（初めて 0x20、以後は直前の設定値）は仮**: 実機のイメージ ページが `PAGE_EXECUTE_READ` と報告されるか、書き込み可にした後に `PAGE_EXECUTE_WRITECOPY`（0x80）として報告されるかは、Learn のどのページにも書かれておらず、一次資料で確認できない。FLEngine と QFC のゲストは旧保護を使わない（`[rbp+0x2c]` などに書かれるだけ。読んだ範囲）ので、測定結果は旧保護の値に依存しない |
| 6 | 登録の方式（**PM 承認**） | 生成 C の表は `static const`、runtime への引き渡しは**生成 C の constructor が `fle_hook_register` を呼ぶ**形。理由: 複数モジュールの結合に使う `split_mod.py`／`merge_tables_n.py`（`harness/two/`、`harness/census/`）は、`fle_dispatch_table` と `fle_resp_table` の節だけを名前で切り出して結合する。同名の第 3 の `extern` 表を足すと、結合の工程が黙って落とすか、モジュールどうしで衝突する。末尾の constructor ならどの分割ツールの最後の部分にもそのまま入り、ツールを変えなくてよい。**第 2 段階で確認**: 分割・結合ツールは変更なしで 2〜5 モジュールの構成が組めた |
| 7 | 実在モジュール（**PM 承認: `fle_realmod` の表に触れない**） | 生成 C がその基底（`fe-pe rebase` 済みのファイルの ImageBase）の VA を持つので、`fle_realmod` の表（名前と基底）とは**別に**登録する（`fle_realmod_table` には触れない）。註釈ファイルは RVA で書く（決定 2）ので、測定用の基底が変わっても同じファイルを使える。`a4-multi.sh` のモジュール指定に第 6 の欄 `HOOKS`、FLEngine には環境変数 `FLE_HOOKS` を足した（`gen_two.sh` 2 本も `FLE_HOOKS` を読む） |
| 8 | 作らないもの | 下の「範囲外」 |

## 第 2 段階の結果

### 適用とコンパイル

- `resolved-H.diff` は統合木（`lift/out-git/integ1`）に `patch -p1 --dry-run`、本番とも fuzz も offset も拒否ハンクも無く当たった（11 ファイル）。
- `cargo build -p fe-lift --offline`: **エラーも警告も出なかった**（H-NOTES.md の自己点検リストの箇所に当たるものは無かった）。
- **コンパイルとテストで直した点: なし。** 最終の `H-final.diff`（HEAD の `git archive` に対する 11 ファイルの差分）の hunk は `resolved-H.diff` と同一（ヘッダ行のタイムスタンプだけが違う）。統合木の `a4-multi.sh` の `EXPECT_FLE_SHA` の既定値は `feb5b908…` だったので HEAD の `43ee118e…` に揃えた（HEAD の内容と同じにしただけで、差分には出ない）。
- **気づいた HEAD 側の不整合（直していない）: `a4-multi.sh` の `EXPECT_FLE_SHA` の既定値 `43ee118ea490…`（決定 0103 の追記にも同じ値）は、生成 C のファイルの SHA-256 ではない。** `fe-lift lift FLEngine_x64.dll /dev/stdout --pdata | sha256sum` の値で、`fe-lift` は統計を標準出力に書くため、**C の後ろに統計の行が付いた流れ**の SHA である（`cat OUT.c stats.txt | sha256sum` で同じ値になることを確認）。ファイルの SHA は `8789c1a5c36125c5ce412b75ab9696e785d98cca2bc0d34b043607345850a77e`（215,747,179 B）。既定値のまま `a4-multi.sh` を実行すると「FLEngine C sha256 8789c1a5… differs from 43ee118e…」で止まる（実行して確認: `h2-results/step3-a4-multi-default-expect-sha.txt`）。今回の測定は `EXPECT_FLE_SHA` を明示した。

### テスト

| 何 | 結果 |
|---|---|
| `hook.rs` の単体（`cargo test -p fe-lift --lib hook`） | 7 件が通った |
| `tests/hook_points.rs`（`cargo test -p fe-lift --test hook_points`） | 16 件が通った（約 16 秒。単独で 1 回、全体テストの中で 1 回、どちらも通った） |
| 全体（`cargo test --workspace --no-fail-fast --offline`） | **111 件の test result（基準 110 + `hook_points`）、passed 1,360、failed 41（wall_q 5、wall_r 27、wall_s 9）**。41 件はすべて `No space left on device`（`/` の空きが 0 になった。`lift/out-wallQ-test`、`out-wallr`、`out-wallS-test` が統合木の中の実ディレクトリで `/` の上にあり、テストが C のソースと実行ファイルをそこに書く）。**この 3 つの binary を、統合木の `crates`／`runtime`／`harness` の複製（`diff -rq` で差なし）を tmpfs に置いて再実行したところ、wall_q 5、wall_r 31、wall_s 15、すべて通った。** 合算すると passed **1,401**（= 1,360 − 10 + 51。基準 1,378 + 7 + 16 = 1,401 と一致）、failed 0。**1 回の実行で通ったわけではない**（下の「ディスク」） |

全体テストのビルドは、`/`（target-integ1）では 2 回リンクで `ld terminated with signal 7 [Bus error]`（ENOSPC）になったため、`CARGO_TARGET_DIR=/tmp/opencode/h2/target-test`、`CARGO_INCREMENTAL=0`、`CARGO_PROFILE_DEV_DEBUG=0` で tmpfs に作り直した（テストの意味は変わらない。デバッグ情報と増分ビルドの有無だけ）。

### 変異検査（Rust 側。16 件、すべて検出）

`lift/out-git/h2-results/rustmut.py` が 1 か所ずつ変異を入れ、`hook.rs` の単体と `hook_points.rs` を実行し、戻して `cmp` で一致を確認した（結果: `step2-rust-mutation-results.txt`）。

| 変異 | 検出したテスト |
|---|---|
| R1 `lib.rs`: ガードを常に空にする | hook_points 7 件 |
| R2 `lib.rs`: フック関数のすべての命令ブロックにガード | hook_points 2 件（`…only_changes`、`…listed_twice…`） |
| R3 `lib.rs`: `#include "fle_vmap.h"` を出さない | hook_points 11 件（clang のコンパイル失敗） |
| R4 `lib.rs`: 末尾の表を出さない | hook_points 6 件 |
| R5 `lib.rs`: 表のイメージサイズを 0 | hook_points 5 件 |
| R6 `lib.rs`: `finish()` を飛ばす（整列・重複・間隔） | hook_points 2 件 |
| R7 `hook.rs`: 入口検査を外す | 単体 1 件、hook_points 2 件 |
| R8 `hook.rs`: `e9` 検査を外す | 単体 1 件、hook_points 1 件 |
| R9 `hook.rs`: セクション特性の検査を外す | 単体 1 件、hook_points 1 件 |
| R10 `hook.rs`: `orig5` を上位バイトから並べる | 単体 1 件、hook_points 5 件 |
| R11 `hook.rs`: 間隔検査を `< 1` に弱める | 単体 1 件のみ（`finish_sorts_dedups_and_refuses_overlap`。hook_points は通る） |
| R12 `hook.rs`: `to_vas` が RVA に基底を足さない | 単体 1 件、hook_points 1 件 |
| R13 `hook.rs`: ガードの `return` を外す | 単体 1 件、hook_points 4 件 |
| R14 `hook.rs`: `PAGE_EXECUTE_READ` を 0x40 に | 単体 1 件、hook_points 3 件 |
| R15 `main.rs`: `--hook-points` の中身を無視 | hook_points 2 件 |
| R16 `main.rs`: `hook_points` の行を常に出す | hook_points 1 件 |

（C 側の変異 20 件は第 1 段階で実施済み。第 2 段階では再実行していない。）

### 生成 C

- **註釈なし**: `fe-lift lift FLEngine_x64.dll … --pdata` の C は、HEAD の統合木が作る C と同一。流れ（C + 統計）の SHA-256 は `43ee118ea490e47f363c3402e05cbca13cd6f5bd5e9c5edce97831f14fe0a0d6`（PM の記録と一致）、C のファイルだけの SHA-256 は `8789c1a5c36125c5ce412b75ab9696e785d98cca2bc0d34b043607345850a77e`（215,747,179 B、2,886,306 行）。統計も不変。
- **註釈あり**（`--hook-points lift/harness/hooks/flengine-hook-points.txt`）: C のファイルの SHA-256 `fc84a6d5cd6e98861a9c834e7d4d11ab5c0a7a3a4778624355df5f5c1095b735`（215,748,026 B、2,886,317 行）、流れの SHA-256 `5624ac31221bfdce4ed8b44e4d37538bc87625dfe7decf58d5789f4390c55cc5`。**註釈なしとの差は 3 つだけ**（`h2-results/hdiff-stream.py` で、2 つの C を同時にディスクに置かずに行ごとに比較。`step4-fle-hdiff.txt`）: `#include "fle_vmap.h"` の 1 行（5 行目）、フック点 3 関数の入口の行のガード（`L_41c820`、`L_43b130`、`L_43b150`。行数は増えない）、最後の関数の後の表と constructor 10 行。ガードを取り除いたあとの他の行はすべて一致（差の行 0）。統計の差は `c_lines` +11、`c_bytes` +847、`hook_points 3` の 1 行。
- **QFC**（`fe-pe rebase` で 0x1d00000、`--pdata`）: 註釈なしの C のファイルの SHA-256 `2ed28bd8e2c77f5fd0fc331c11a3a14bb3decc18597c2ec555b66fff8bf52d97`（18,187,173 B）、註釈あり（`qfc-hook-points.txt`）`9e60a27f40a9777003a61431a090e60c181341c8e81b6195efaa840b6a86fb88`（18,188,029 B）。差は FLEngine と同じ 3 つだけ（点 0x1d18e80、0x1d30ec0、0x1d30ee0。末尾の表は基底 0x1d00000、SizeOfImage 0x20e000）。
- dsp_ippv2、Blend2D、freetype には `--hook-points` を付けないので、これらの C は `generate_with_hooks(&[])` の経路（旧 `generate_with_options` の本体と同じ）を通る。FLEngine の C が SHA で不変であること以外に、旧 `fe-lift` との直接比較はしていない。

### 実測（native。構成ごとに `a4-multi.sh` で測り、同じ `two-native`／`flengine-native` を `a4-survey.sh` でもう一度実行して、2 回の `native.txt` が `calling entry … rsp=` の 1 行を除いて同一であることを確認した）

測定の条件: HEAD の統合木 + H、FLEngine `b2c64da4…`、dsp_ippv2（`lift/out-ext/dsp_ippv2_x64.dll`、基底 0x2000000、種 452）、Blend2D（基底 0x4700000、種 58、`--pdata`）、QFC（`lift/out-qfc/quickfontcache_x64.dll`、基底 0x1d00000、`--pdata`）、freetype（基底 0x4b00000、種 160、`--pdata`）。基底と種は測定用の値で決定ではない（決定 0103）。ビルドは `PATH` の先頭に置いた `clang` のラッパー（`/tmp/opencode/h2/bin/clang`）で、(1) 裸の `-g` の引数だけを外し、(2) コンパイルが成功した `…/src/*-NN.c` を消した。さらに `a4-multi.sh` の複製（`h2-results/a4-multi-trim.sh`。差は `HERE` の固定と、gen 段の後に生成 C を消す 1 行だけ）で使った（`/tmp/opencode` の quota のため。リポジトリのスクリプトは変えていない。`-g` が測定値に影響しないことは**確かめていない**。傍証は、FLEngine 単独が従来どおり 145 / 942 で停止メッセージも同じこと、註釈なしの 5 モジュールが決定 0103 の記録（159 / 942、functions 1,912、entries 77,849）と停止を含めて近い値（1,914、77,904）になったこと）。

| 構成 | 註釈 | init_count / 942 | last unit | functions | entries | 停止 | 終了コード |
|---|---|---|---|---|---|---|---|
| FLEngine 単独 | なし | 145 | index 144 (proc 0x65c540) | 822 | 15,686 | `LoadLibraryW "dsp_ippv2_x64.dll"` | 134 |
| FLEngine 単独 | FLEngine（3 点） | 145 | 同じ | 822 | 15,686 | 同じ | 134 |
| 2 モジュール（+ dsp） | なし | **246** | index 245 (proc 0x733090) | 1,198 | 43,053 | `LoadLibraryW "iltools_x64.dll"` | 134 |
| 3 モジュール（+ Blend2D） | なし | **246** | 同じ | 1,330 | 43,859 | 同じ | 134 |
| 5 モジュール（+ QFC + freetype） | **なし** | **159** | index 158 (proc 0x66d1a0) | 1,914 | 77,904 | **`VirtualProtect: address is not in a VirtualAlloc region`**（`f_1d1dae0`） | 134 |
| 5 モジュール | **FLEngine + QFC** | **246** | index 245 (proc 0x733090) | 2,148 | 94,011 | **`LoadLibraryW "iltools_x64.dll"`** | 134 |

- 5 モジュールの 2 構成は同じ入力で註釈だけが違う。**停止点の移動**: `VirtualProtect`（QFC の unit 162）→ unit 245 の `LoadLibraryW "iltools_x64.dll"`。**init_count の増加**: 159 → 246（+87）。両方起きた。ただし 246 は、遅延モジュール（決定 0096）が効く 2・3 モジュールの構成の値と同じで、その停止も同じ（unit 245 の iltools）。
- **HEAD の runtime は blend2d.dll と quickfontcache_x64.dll を遅延モジュール（決定 0096）として持つ**ので、2 モジュールと 3 モジュールの構成は、従来の 150 / 942（停止 `LoadLibraryW "blend2d.dll"`）にも 159 / 942（停止 `LoadLibraryW "quickfontcache_x64.dll"`）にもならず、**246 / 942** になった（blend2d と QFC の `LoadLibraryW` が成功するため）。遅延モジュールを外す切り替えは runtime に無い（環境変数もコンパイル定数のスイッチも無い。`fle_modules.c` の `FLE_DEFERRED_FIRST`／`FLE_DEFERRED_N` は定数で、同名の実在モジュールが登録されたときだけ実在が勝つ）。したがって HEAD では「dsp だけを実在にして blend2d を未解決にする」構成（決定 0101 の表の 150 / 942）は測れない。**決定 0101 の A4 の判定構成（実在モジュール構成）に近いのは、4 つすべてを実在にした 5 モジュールの構成**で、その値が 159（註釈なし）→ 246（註釈あり）。遅延モジュールを外す方法は作っていない。
- 5 モジュール + 註釈の C スタック（先頭 10 フレーム、`call depth at stop: 17`）: `f_423990 ← f_44db40 ← f_6539e0 ← f_815e10 ← f_814e60 ← f_812e80 ← f_732f30 ← f_733090 ← f_6539e0 ← f_665bd0`。3 モジュールの停止（`call depth 13`）の先頭 8 フレームは同じだが、9 フレーム目以降が違う（3 モジュールは `f_4120c0 ← f_412160`）。深さが 4 多く、フレームが InitUnits の loop でない理由は**調べていない**（C のスタックの記録が `longjmp` 的な戻りで巻き戻らない残りの可能性があるが、確認していない）。
- **QFC の unit 162 の中身（gdb の読み取り。5 モジュール + 註釈）**: `VirtualProtect` は 6 回（0x1d18e80、0x1d30ec0、0x1d30ee0 が各 2 回。size 5、新保護 0x40）、停止時にこの 3 点の先頭 5 バイトは `e9 eb b9 17 00`、`e9 9b 38 16 00`、`e9 eb 38 16 00`（飛び先 0x1e94870、0x1e94760、0x1e947d0 ＝ 期待した置き換え先）。**有効化のあとに QFC のフック点が呼ばれた回数は 0**（`fle_hook_target` が元の 5 バイト以外を見た回数 0）。つまり**置き換え先は 5 モジュールの A4 の経路で一度も実行されていない**。FLEngine 自身の `VirtualProtect` は 0 回（unit 314 に届かない）。
- FLEngine の unit 314（フックの準備 6 回の `VirtualProtect`、有効化）は、A4 の構成のどれでも届かない（246 < 314）。census でだけ届く（次）。

### census（診断。A4 の進捗ではない）

HEAD の統合木 + H で、`harness/census/` のスクリプト（`gen_two.sh`、`build_two_native.sh`、`census_report.py`）を使った。FLEngine + 縮小 dsp_ippv2（種 452）と FLEngine 単独。比較のため、同じ木で註釈なしも測った。

| 構成 | ok | stop | fault | hang | empty | noproc | 計 | 壁の種類 |
|---|---|---|---|---|---|---|---|---|
| 統合前の census-2（dsp。`wall-census-2.md`） | 876 | 19 | 0 | 0 | 46 | 1 | 942 | 16 |
| 統合前の census-2（single） | 873 | 22 | 0 | 0 | 46 | 1 | 942 | 18 |
| HEAD、註釈なし（dsp） | 887 | 8 | 0 | 0 | 46 | 1 | 942 | — |
| HEAD、註釈なし（single） | 883 | 12 | 0 | 0 | 46 | 1 | 942 | — |
| **HEAD + H、註釈あり（dsp）** | **888** | **7** | 0 | 0 | 46 | 1 | 942 | 4 |
| **HEAD + H、註釈あり（single）** | **884** | **11** | 0 | 0 | 46 | 1 | 942 | 5 |

- **unit 314 は stop（`VirtualProtect`）から ok になった**（dsp と single の両方。entries 787 → 890、新しい関数 53 → 59）。註釈の有無で状態が変わった unit は 314 だけ（`h2-results/census-hooks/diff-vs-no-hook.txt`。unit 830 の `new_fns` が 26 → 25 になるのは、314 で初めて実行された関数が 830 で初めてでなくなったため）。
- 統合前の census-2 からの差（dsp: ok 876 → 887、stop 19 → 8）は、I・R・S・B・Q の統合による（註釈なしの HEAD の値）。H の寄与は 887 → 888（unit 314 の 1 件）。
- 註釈あり dsp の壁（上位。全 4 種）: (1) `LoadLibraryW "iltools_x64.dll"` 3 unit（245、280、659）、(2) call through a null function pointer 2 unit（830、880。二次の疑い）、(3) `GetSystemMetrics: index 86 (SM_TABLETPC)`（unit 276）、(4) `CompareStringW: character other than an ASCII letter or digit in unequal strings`（unit 283）。single は上に `LoadLibraryW "dsp_ippv2_x64.dll"`（144）、null 呼び 5 unit（170、830、833、880、935）が加わる。TSV: `h2-results/census-hooks/top-walls-{dsp,single}.tsv`。
- **census でも置き換え先は実行されなかった**: gdb で single の census を測ると、`VirtualProtect` は 6 回（0x41c820、0x43b130、0x43b150 が各 2 回）、census 終了時に 3 点の先頭 5 バイトは `e9 5b e5 4b 00`、`e9 3b fb 49 00`、`e9 8b fb 49 00`（飛び先 0x8dad80、0x8dac70、0x8dace0 ＝ 期待どおり）、**6 回目の `VirtualProtect` のあとの `fle_hook_target` の呼び出しは 0 回**（それ以前の 0x41c820 への 63 回はすべて元のバイト）。したがって**置き換え先（`f_8dad80` など）で次の壁が出るかは、census でも未測定のまま**。
- census の数は診断で、A4 の進捗ではない。

### 次の壁（実測）

- 5 モジュール（実在 4 つ + 註釈）の停止: **unit 245 の `LoadLibraryW "iltools_x64.dll"`**（スタックは上）。決定 0101 の表では `iltools_x64.dll` は unit 280、659 で出る名前だが、unit 245 の経路（`f_733090` ← `f_732f30` ← `f_812e80` ← `f_814e60` ← `f_815e10` ← `f_6539e0` ← `f_44db40` ← `f_423990`）でも出る（census でも unit 245 が iltools で放棄される）。
- そのあとの census の壁は上のとおり（`SM_TABLETPC`、`CompareStringW`、null 関数ポインタ呼び）。
- 置き換え先（0x8dad80、0x8dac70、0x8dace0 とその先）は未測定。

## 範囲

- 註釈されたフック点（関数入口）の、先頭 5 バイトの `jmp rel32`（`e9 rel32`）への書き換えと、元の 5 バイトへの書き戻し（何度でも）。FLEngine と QFC の 3 点ずつ。
- `VirtualProtect` が、そのフック点の 1〜5 バイトに対して呼ばれること（旧保護はページごとの記録）。
- 生成器は新しい命令を扱わない。`fle_dispatch` と landing（決定 0100）には触れない。

## 範囲外（今回扱わない）

- 自己書き換えの一般対応: 註釈の無い点への書き込み、動的に生成されるコード（FLEngine の `0x562bb8` の周辺は、ヒープ上に thunk の雛形を作って `VirtualProtect(…, PAGE_EXECUTE_READWRITE)` するように読める。読みだけ。未測定）、JIT。**註釈の無い関数に `e9 rel32` が書かれても、その関数の C は本体を実行する**（ガードが無い。これがこの設計の限界で、網は `VirtualProtect` の停止だけ。`VirtualProtect` を呼ばずに書く自己書き換えは見つけられない）。
- 複数スレッド（`fle_vmap.c` は単一スレッド。既存の `fle_vmap_check_thread` が先に止める）。
- フック先（`jmp` の飛び先）が関数入口でない場合: `fle_dispatch` の既存の名前付き停止に任せる。
- `VirtualQuery`（イメージは従来どおり停止）。`VirtualProtect` の `PAGE_GUARD` などの修飾（既存の停止のまま）。
- イメージ内のデータ領域への `VirtualProtect`（FLEngine の `f_8eb310`／`f_8eb390` の 8 バイトのポインタ書き換え。下の「読みで気づいた不整合」）。名前付き停止のまま。**第 2 段階の A4・census では到達しなかった**（この停止の新しいメッセージは出ていない）。
- wasm（`emcc` でビルドしていない）。

## 壁ごとの表

| 壁 | 読み（区分） | 規則 | 撤回条件 | テスト |
|---|---|---|---|---|
| FLEngine unit 314（0099 W4） | **実測**: フック点 3、置き換え先 3。census で unit 314 が stop → ok（`VirtualProtect` 6 回、有効化後の先頭 5 バイトは 3 点とも `e9 rel32` で飛び先は期待どおり）。有効化は `ParamStr(0)` の小文字が 3 つの名前のどれでもないとき（この環境の `GetModuleFileNameW(NULL)` は `FLEngine_x64.dll` のパスなので有効化される。census で有効化されたことを確認）。**推測**: 置き換え先は文字列クラスの関数を横取りする層で、有効化の後に元の本体は呼ばれない（`f_8e0d80`／`f_8e0db0` の呼び出し元は `f_8e0ed0`／`f_8e0ef0` だけで、置き換え先の側から元の本体を呼び直す経路は読んだ範囲に無い）。**未測定**: 置き換え先の実行（census でも有効化のあとフック点が呼ばれなかった） | 決定 1〜5。註釈 `harness/hooks/flengine-hook-points.txt`（RVA 0x1c820、0x3b130、0x3b150） | (1) 実機で、この 3 点のフックが無効になる条件（`ParamStr(0)` の名前以外）が確認された。(2) ゲストが註釈の無い点にフックを張った（`VirtualProtect` の停止で見つかる）。(3) 生成器が自己書き換えに一般に対応した | `fe-lift/tests/hook_points.rs`（下） |
| QFC unit 162（0103 壁 13） | **実測**: フック点 3（RVA 0x18e80、0x30ec0、0x30ee0）。5 モジュール構成で `VirtualProtect` が 6 回通り、3 点に `e9 rel32` が書かれ、停止が `VirtualProtect` から unit 245 の iltools に動いた。**未測定**: 置き換え先の実行 | 同上。註釈 `harness/hooks/qfc-hook-points.txt`（RVA）。`a4-multi.sh` の指定例: `quickfontcache_x64.dll:/abs/quickfontcache_x64.dll:0x1d00000::pdata:/abs/lift/harness/hooks/qfc-hook-points.txt` | 同上 | 同上 |

## 撤回条件

- 実機の Windows で、当該のフック（FLEngine と QFC の各 3 点）が無効になる条件が確認された（決定 1 の前提が崩れる）。
- ゲストが註釈の無い点にフックを張った（`VirtualProtect` の停止で見つかる）。または、`VirtualProtect` を呼ばずに書き換える自己書き換えが見つかった（網が破れる）。
- 生成器が一般の自己書き換えに対応した（この方式を置き換える）。
- **実機で、イメージのページの旧保護が `PAGE_EXECUTE_READ` と違うことが確認された（決定 5 の旧保護の値、特に初めての 0x20 と 2 回目以降の 0x40。書き込み可にしたあとの `PAGE_EXECUTE_WRITECOPY` の報告を含む）。旧保護の値は一次資料で確認できていないので、この決定の「仮」の範囲に含める（PM の決定）。**
- 元でも `e9` でもない先頭を名前付き停止にすること（決定 3 の (c)）は PM が承認した。実機のゲストが別形式のフック（`ff 25`、`48 b8 … ff e0` など）を張ることが分かった場合は、その形式を扱う別の決定を立てる。

## 実装した場所

| ファイル | 内容 |
|---|---|
| `crates/fe-lift/src/hook.rs`（新規、235 行。単体テスト込み） | 註釈ファイルの解析、フック点 1 つの検査、整列と重複・間隔、ガードと表の C 文字列。他の crate に依存しない（単体のテスト 7 件つき） |
| `crates/fe-lift/src/lib.rs` | `pub mod hook;`、`Stats::hook_points`、`generate_with_hooks`（旧 `generate_with_options` の本体。旧関数は `&[]` を渡す薄い包み）、ヘッダの `#include`、入口ブロックのガード（`pre`）、末尾の表 |
| `crates/fe-lift/src/main.rs` | `--hook-points FILE`、`hook_vas`、統計の `hook_points N`（0 のときは出さない） |
| `runtime/fle_vmap.h`、`fle_vmap.c` | `fle_hook_point`、`fle_hook_register`、`fle_hook_target`、`VirtualProtect` の分岐 |
| `harness/hooks/flengine-hook-points.txt`、`qfc-hook-points.txt`（新規） | 註釈（根拠をコメントに書いた） |
| `harness/a4-multi.sh`、`two/gen_two.sh`、`census/gen_two.sh` | `HOOKS` 欄、`FLE_HOOKS`（スタンプとタグに含める） |
| `crates/fe-lift/tests/hook_points.rs`（新規） | 下 |

差分は `H-final.diff`（HEAD `3772c2a` の `git archive` に対して 11 ファイル、+1,198／−20 行。`patch -p1` で当たる。当てた木と統合木は 11 ファイルとも `cmp` 一致）。

## テスト

`hook.rs` の単体 7 件: 註釈ファイルの形、エラーの行番号、RVA への基底の加算、5 バイトの下位からの並び、註釈の誤り（入口でない、セクションなし、特性、コードなし、先頭 `e9`）、整列と重複と間隔、ガードと表の文字列。

`tests/hook_points.rs` 16 件。合成 PE（ImageBase 0x10000000。H1 = `mov eax,0x11; ret`、置き換え先 T1 = `lea rax,[rcx+0x22]; ret`、H2 = フレームを持つ関数（H1 と同じページ）、N1 = 註釈なし）:

| 区分 | テスト（要点） |
|---|---|
| 生成器 6 | ガード・include・表の 3 つだけが旧出力との差（取り除くと旧出力と一致）／フック点なしは旧出力とバイト一致／同じ VA の重複は 1 点／註釈の誤り（入口でない 4 種、先頭が `e9`、セクションの特性 4 種）でエラー／コマンドラインが VA と `rva:` とコメントを読む／コマンドラインが構文エラー・入口でない点・ファイルなしで終了する |
| 実行 10 | フック前は本体／`e9 rel32` で tail dispatch（スタックとレジスタが入口のまま。H2 の本体の push が走らない）／`fle_dispatch` 経由も同じ／元の 5 バイトを戻すと本体、patch-unpatch-patch を 3 回／飛び先が入口でない → dispatch の停止／元でも `e9` でもない（`cc ×5`、最後の 1 バイトだけ変更、target 0）→ 名前付き停止／rel32 の符号と基準（前後 1 GiB を含む）／註釈の無い関数はガードが無い（限界の記録）と、その `VirtualProtect` の停止／フック点の `VirtualProtect`（旧保護 0x20 → 0x40 → …、同じページの 2 点、サイズ 1〜5、NULL → 87）／イメージ内の註釈の無いアドレス（中、`.text` 全体、ヘッダ、最後の 1 バイト、NULL 旧保護、サイズ 6）の停止／イメージの外は従来どおり（スタック、イメージの直後、VirtualAlloc の領域） |

**テストの置き場の注意**: `hook_points.rs` は `lift/out-hook/test/` に pid つきの名前の実行ファイル（約 360 KB ずつ）を実行のたびに残す（消さない）。変異検査（16 回）と通常の実行を合わせて 18 回ほど実行して 189 ファイル、6.7 MB が `/` に溜まった。既存の wall_q／wall_r／wall_s／wall_b も同じ型で `/` に残す（`out-wallQ-test` 32 MB、`out-wallb` 38 MB など。このうち今回の実行分は未区分）。

## 解釈が分かれた点（第 1 段階の 10 件。PM の決定を反映）

1. **表の渡し方（決定 6）**: 採用: `static const` の表 + 末尾の constructor が登録。**PM 承認。** 却下: 同名の `extern const fle_hook_points[]`。constructor の欠点: 登録が main の前に起きること（実害は見ていない）、emscripten の constructor は未確認（wasm は範囲外）。
2. **イメージの範囲とセクションの情報の出どころ**: 採用: 生成器が表に書く（ImageBase、SizeOfImage、各点の保護）。**PM 承認。** 違い: **フック点の表を持たないモジュール（Blend2D、dsp など）のイメージへの `VirtualProtect` は、新しいメッセージでなく従来の「address is not in a VirtualAlloc region」で止まる**（どちらも名前付き停止）。
3. **「違って、かつ先頭が 0xE9 でない」場合（決定 3 の (c)）**: 採用: 名前付き停止。**PM 承認**（0046 方針 4 の fail-soft を避ける）。
4. **フック点が閉包の関数入口でない場合**: 採用: エラー（`--pdata` か `--seeds` で入れさせる）。PM の明示なし（第 1 段階のまま）。
5. **`dwSize` が 5 を超えるフック点**: 停止。**PM 承認。** ゲストが複数の点をまとめて 1 回で保護する場合は、この点を直す。
6. **旧保護**: 初めてのページは 0x20（推測）、その後は直前の `VirtualProtect` が設定した値。**仮（PM の決定）。撤回条件に入れた。** FLEngine と QFC のゲストは旧保護を使わないので、測定結果は変わらない。
7. **註釈の形式**: VA と `rva:` の両方。PM の明示なし（第 1 段階のまま）。
8. **ガードの置き場所**: 入口ブロックの中。PM の明示なし（第 1 段階のまま）。
9. **FLEngine の有効化条件**: 3 つの名前（`delphi32.exe`、`kylix`、`bds.exe`）。結論は同じ（この環境では有効化される。census で確認）。
10. **実在モジュールへの渡し方**: `fle_realmod_table` には触れず、生成 C 自身が登録する。**PM 承認。** 基底は `fe-pe rebase` 済みのファイルの ImageBase（= `fle_realmod_table` の基底と同じ値でなければならない。`a4-multi.sh` は同じ BASE を両方に渡す）で、足すのは**リフト時**。食い違いを検出する仕組みは入れていない。

## 読みで気づいた不整合・追加の事実

- 0099 W4 の「`delphi32.exe` でないとき」は、正しくは 3 つの名前のどれでもないとき（QFC も同じ）。
- 0099 W4 は「`f_8e0ef0` が 3 つに `f_8e0de0` をもう一度掛け」としか書かないが、2 回目の `f_8e0de0` は `[obj+0x24]` が 0 でないので**先に `f_8e0ed0`（元の 5 バイトの書き戻し）を呼ぶ**。したがって有効化の前に、同じページの `.text` への書き込みが 2 回起き、`VirtualProtect` は 1 点につき 2 回呼ばれる（ゲストにとって 2 回目の旧保護は 0x40）。**第 2 段階で確認**: QFC と FLEngine の両方で 1 点につき 2 回（gdb の記録）。
- 0103 の壁 13 の選択肢 (a) は「フックの書き込み（`f_597e..` の「有効化」）」と書くが、実際の関数は `f_597da0`（有効化）と `f_597c60`（書き込み）。QFC のフック点は 0103 の本文に書かれていない（本書が初めて特定した）。
- FLEngine には、フック点の準備でない `VirtualProtect` の使用者がある（読みのみ。**第 2 段階の A4・census では到達しなかった**）: thunk 0x423d00 の呼び出し 0x562bb8（0x562b70 付近の関数。直後の 0x562bcc に `mov r10,rcx; mov r11,imm64; jmp r11` の形の雛形があり、書いたヒープの領域に `PAGE_EXECUTE_READWRITE` を設定する。動的に生成するコードの可能性。**推測**）、thunk 0x423d10（同じ IAT スロットの別の thunk）の呼び出し 0x8eb343／0x8eb379（`f_8eb310`）と 0x8eb3bd／0x8eb3f0（`f_8eb390`）: `IsBadWritePtr` の後に **8 バイトのポインタ**を書き換える（vtable スロットなどの可能性。**推測**）。対象がイメージ内なら、この決定では新しい名前付き停止になる（メッセージは「inline hook at an address that is not an annotated hook point」だが実体はデータ書き換えで、文言が実態と合わない）。選択肢: (i) 停止のまま（現状）、(ii) イメージの非実行セクションへの `VirtualProtect` は成功させる、(iii) 対象を調べてから決める。
- `VirtualProtect` の thunk が FLEngine に 2 つ（0x423d00 と 0x423d10）あり、どちらも IAT スロット 0x19712b0 を指す。
- **`a4-multi.sh` の `EXPECT_FLE_SHA` の既定値の不整合**（上の「適用とコンパイル」）。決定 0103 の追記と `a4-multi.sh` 48 行（HEAD では 43 行）の値は、C のファイルの SHA ではなく、C に統計の行が付いた流れの SHA。
- HEAD の runtime は blend2d／QFC を遅延モジュールとして持つので、2・3 モジュールの測定は従来の値（150、159）にならない（上の「実測」）。決定 0101 の表の 150（dsp のみ実在）は HEAD では再現できない。

## 限界・未確認

- 置き換え先が走ったときに新しい壁に当たるかは**未測定**。A4（5 モジュール）の経路でも census でも、有効化のあとにフック点は呼ばれなかった（`fle_hook_target` の呼び出し 0）。置き換え先（0x8dad80、0x8dac70、0x8dace0 とその先 `f_8dfb80`、`f_8da0d0` など）は一度も実行されていない。合成 PE のテスト（`hook_points.rs`）が tail dispatch を確認しているだけ。
- 実機の Windows と照合していない: 旧保護の値、イメージのページに `PAGE_EXECUTE_READWRITE` を設定したあとの報告値。
- `fle_hook_target` は入口の呼び出しの時点の 5 バイトだけを見る。関数の実行中に先頭が書き換わる場合（再入）は見ていない。
- 註釈の無い点の自己書き換えを `VirtualProtect` なしで行うコードは見つけられない（範囲外）。
- wasm: constructor の登録（`__wasm_call_ctors`）、`FLE_CHECK_HI32` 下の `fle_ld(va, 5)`（FLEngine と QFC の VA は 32 ビットに収まる）は**未確認**（`emcc` を使っていない）。
- C 側の変異検査は第 1 段階の 20 件のまま（第 2 段階で再実行していない）。
- 全体テストは 1 回の実行で通っていない（41 件が `/` の空き 0 で失敗し、3 つの binary を別に再実行した）。
- 5 モジュールの停止時の C スタックの深さ（17）が 3 モジュールの停止（13）と違う理由は調べていない。
- 統合前の census-2 と HEAD の差の内訳は、I・R・S・B・Q の寄与を個別には切り分けていない。
- 分割ツール（`split_c.py`、`split_mod.py`）が末尾の表を最後の部分に残すこと: 2〜5 モジュールの構成が組めて動いたことで確認した（個別のテストは無い）。

## 作ったもの

`lift/out-git/h2-results/`（`H-final.diff`、本書、`README-0105-line.txt`、構成ごとの a4 の行・停止メッセージ・C スタック・census の TSV、`rustmut.py` と結果、`hdiff-stream.py` と結果、gdb の記録）。第 1 段階の成果物は `lift/out-git/merge1/`。
