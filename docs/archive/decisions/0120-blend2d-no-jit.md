# 0120: ゲストの Blend2D は JIT 無効で使う（`Agg2D::attach` の `blContextInitAs` に渡す `createInfo.flags` の bit 0 を、呼び出し点の注釈 `--site-overrides` で立てる）

状態: **仮の決定**。依頼は PM（2026-10-30。決定 0120 の方式（「ゲストの Blend2D を JIT 無効で使う」）は PM の仮の決定。番号 0120 は PM の仮の割り当て）。**機構の選択（A2/A3 のどちらか）は依頼で作業者に委ねられた**ので、この文書の「機構の選択」は作業者の推奨で、PM が決める。測定・実装・テスト・文書は作業者（サブエージェント）が行った。**作業は HEAD 018ba73 の `git archive` で始めた。HEAD は 167a1db に進んだ（0119 の差分。`lift/runtime/fle_win.c` ほか 44 ファイル）。この差分が触る 8 ファイル（`lib.rs`、`main.rs`、`fle_vmap.c`、`fle_vmap.h`、`a4-multi.sh`、`main_two.c`、`build_two_native.sh`、`build_nmod_wasm.sh`）は 018ba73 と 167a1db で同一（`git diff --quiet`）。`x2.diff` は 018ba73 の clean な archive に当たる（`patch -p1 --dry-run`。「検証」）。167a1db の木には当てていない。**

**実機の Windows とは 1 件も照合していない（0 件）。** この文書の「画素が正しい」は、(a) 仕様から手計算した値、(b) ゲストの Blend2D の他の経路（X1 の JIT 経路）との一致、(c) native と wasm の一致、のどれかで、実機の出力との比較ではない。

区分の凡例: **実測**＝コマンドの出力を見た事実。**読み**＝逆アセンブルやコードを読んだ上での解釈（実行して確かめていない）。**公開資料**＝Blend2D の公式ドキュメント／公開ソースに書かれていること。**出典なしの選択**＝根拠が無く作業者が選んだもの。**未確認**。

## 先に結論

| # | 問い | 答え | 区分 |
|---|---|---|---|
| 1 | JIT 無効の機構 | **A2 の拡張: 呼び出し点の注釈 `--site-overrides`**（lift 時に、指定した命令の直前に `fle_site_store32(...)` の 1 呼び出しを出す）。Blend2D の生成 C は、1 行追加（`#include`）と 1 行変更（`0x4756ce9` の命令のブロック）以外は **418,078 行すべて同一**。注釈なしの 6 モジュール構成の生成 C は 1 バイトも変わらない | 実測 |
| 2 | 注釈つきの 6 モジュール a4 | `init_count=942 of 942`、`895/895`、`returned from entry`、終了 0、functions **5,308**、entries **1,007,947**（基準と同一）。注釈なしでも同じ。attach は a4 の経路で呼ばれない | 実測 |
| 3 | 基準の native SHA | **`27cdf764f10ffabc…`**（決定 0118 の値）を再現（OUT の名前が違う 3 つの独立なビルドで同一）。X1 の SHA の差（`f469d29f…`／`65c904e3…`）は、**X1 の 2 つのバイナリが消えていて直接は比べていない**。同じ構成を私が 2 つ別の OUT 名で作ると同じ現象（SHA が違う）が起き、原因は **ビルドしたディレクトリ名がデバッグ情報に入ること**と特定できた（実測）。X1 の差も同じ原因と**推測**する（X1 自身の推測とも一致） | 実測 |
| 4 | スロット走査（native、JIT 無効） | Agg2D の vtable **130 スロット全部を 1 回以上呼んだ**。実体のある 78 スロット（52 は 1 命令のスタブ）。**138 のテスト（名前付き 69 + ブレンドモード 29 + ファズ 40）、全部が `status=done`**。別に極端な値 132 通り（128 が完走、4 がハング）。停止 0、壁 0。ただし **値を仕様から検査できたのは 38 スロット**（SPEC。検査の直前に呼んだスロットを数える。テスト単位のタグでは過大になる）、18 スロットは「ゲストの関数の読み」に対する検査（READING）、**22 スロットは画素ハッシュだけ** | 実測 |
| 5 | native と wasm | **138 テストすべてで、`RESULT`／`CHECK`／`VALUE` の行が native と wasm でバイト一致**。さらに極端な値（NaN、無限大、DBL_MAX、非正規数、2^52、-0.0 ほか）の 128 通りも一致。-O1 の別翻訳単位の 63 関数のうち**走査で実行されたのは 44**（Blend2D の 4 関数 `f_4859b70`、`f_4865f60`、`f_4871d60`、`f_487e100` はすべて実行され、一致） | 実測 |
| 6 | 壁 | **見つからなかった（0）**。ただし極端な値のうち **4 通り**（楕円の半径が NaN／±無限大／DBL_MAX。slot 67）で、native が 20 秒で止まらない（**ハング。停止ではなく無限ループ**。native の実行では `alarm` で打ち切った。wasm では**実行していない**）。実機の Blend2D が同じ入力で止まるかは未確認 | 実測 |
| 7 | 「JIT 無効」は本当に効いているか | 138 テストの実行中、`VirtualAlloc`／`VirtualProtect`／`CreateFileMappingW`／`MapViewOfFile` の呼び出しは **0 回**（gdb の計数）。全テストで BLContext のパイプライン提供者は**固定パイプライン**（0x48f30e0）。注釈を外すと提供者が JIT（0x48f2ff0）に戻り、`clear` は `VirtualAlloc` の先で止まる | 実測 |

## 背景（事実）

X1（`docs/investigations/blend2d-jit-and-drawing-path.md`）の結論: FLEngine の描画は Blend2D.dll の `Agg2D` ラッパー（vtable 130 スロット）を通る。`Agg2D::attach`（vtable スロット 1。RVA 0x56c90）は `blContextInitAs`（RVA 0x146ff0）を `createInfo` 全 0 で 1 か所（RVA **0x56ce9**）から呼び、Blend2D は既定で JIT（AsmJit）を使う。最初の描画で `VirtualAlloc(0, 0x20000, MEM_COMMIT|MEM_RESERVE, PAGE_EXECUTE_READWRITE)` を呼び、実行可能メモリの機械語を `call` する。これは lift した C でも wasm でも実行できない。`createInfo.flags` の bit 0 を立てると固定パイプラインになり、64×64 の 6 手順が完走した（X1。ホストの CPU で JIT 生成コードを実行した結果とバイト一致）。

**公開資料（本作業で確認。X1 は 404 で未照合だった）**: Blend2D の公式ドキュメント（`blend2d.com/doc/group__bl__rendering.html`）に `BL_CONTEXT_CREATE_FLAG_DISABLE_JIT`「Disables JIT pipeline generator」、`BLContextCreateInfo::flags`「Create flags」。値は、同ライブラリの Crystal 束縛（`DisableJit = 1_u32`）が **1** と書く。ゲストのバイナリでは `test al,1`（RVA 0x146267）と実測（`flags=1` で提供者が 0x48f30e0 になる）が一致する。**ライブラリの版は未確認**（古い版の公開ヘッダ（Woboq の `blcontext.h`）では bit 0 は `BL_CONTEXT_CREATE_FLAG_FORCE_THREADS` で、現行の `DISABLE_JIT` ではない。ゲストの版の定数は**ゲストのバイナリの挙動**で確かめた: bit 0 を立てると固定パイプライン、立てないと JIT）。

## 決定（仮）

| # | 項目 | 決定 |
|---|---|---|
| 1 | 方針 | 「この環境には実行可能メモリが無い（wasm は実行時に生成した機械語を実行できない）。**Blend2D には、ライブラリに組み込み済みの移植可能なパイプラインを使うよう指示する**」を、**環境の定義**とする。ゲストの DLL は書き換えない（バイトパッチ A1 は採らない）。Blend2D の他の関数は 1 バイトも変えない |
| 2 | 機構 | `fe-lift lift ... --site-overrides FILE`。ファイルの 1 行 = 1 つの上書き: `ADDR store32 REG[+DISP|-DISP] EXPECT NEW`（`ADDR` は VA または `rva:` + 16 進 RVA、`REG` は 64 ビットレジスタ名、`EXPECT`／`NEW` は 32 ビットの 16 進）。生成 C は、その VA の命令の**ブロックの先頭**に `{ fle_site_store32(VA, R[REG] + DISP, EXPECT, NEW); }` を出す（関数のコピーが複数あれば、すべてのコピー）。`fle_site_store32`（`runtime/fle_vmap.c`）は 32 ビットの語が `EXPECT` なら `NEW` を書き、すでに `NEW` なら何もせず、**それ以外の値なら名前付きで停止**する（ゲストの値が違うなら、注釈の前提が崩れている） |
| 3 | 注釈 | `lift/harness/hooks/blend2d-no-jit-sites.txt`: `rva:0x56ce9 store32 r8 0x0 0x1`。RVA 0x56ce9 は `call blContextInitAs`、直前の `lea r8,[rsp+0x60]`（RVA 0x56cdb）で r8 は `BLContextCreateInfo` を指し、attach が 2 つの 16 バイトの 0 書き込み（RVA 0x56cc7、0x56ccc）で作った。`[r8+0]` が `flags`（読み） |
| 4 | 付け方（方針） | **opt-in**。`a4-multi.sh` のモジュール指定の第 7 欄 `SITES`（`NAME:DLL:BASE[:SEEDS[:OPTS[:HOOKS[:SITES]]]]`）。Blend2D を載せる構成（G4）で PM がこの欄を常に付けるかは PM の判断。**付けない構成は現行と生成 C がバイト不変**（「検証」）。付けた構成も a4 は変わらない |
| 5 | 範囲外 | 下の「範囲外」 |

## 機構の選択（推奨）と理由

候補: (A1) DLL のバイトパッチ、(A2) 決定 0105 の仕組みの拡張、(A3) runtime で `blContextInitAs` の入口に介入。

| 案 | 評価（実測と読み） |
|---|---|
| A1 | 採らない。「実物の DLL をそのまま lift する」（決定 0073／0055）から外れる。ファイルの SHA が変わり、以後の記録（SHA 照合）が全部別の物になる。他の案が可能なので不要 |
| **A2（採用）** | **注釈は「呼び出し点」に付く**: `blContextInitAs` は Blend2D の内部（`BLContext::begin` 相当）で、`Agg2D::attach` 以外から呼ばれる経路が静的には見つからない（`xref` 1 件。RVA 0x56ce9 のみ。実測）が、**ゲストが将来別の入口を使う場合に備えて、入口でなく呼び出し点を選んだ**。呼び出し点なら「FL Studio の `Agg2D::attach` が作る文脈だけ」を指示でき、Blend2D 自身の他の使い方（もし他にあれば）には触れない。決定 0105 の `--hook-points` と同じ形（ファイル、RVA 指定、lift 時の註釈、runtime の小さな関数）で、生成器の変更は局所的（`site_override.rs` 新規、`lib.rs`／`main.rs` に数十行） |
| A3（不採用） | 入口の書き換えは、runtime が「どのモジュールの `blContextInitAs` か」を知る必要がある（`fle_realmod` の基底の突き合わせ）。関数の入口は全呼び出し元に効くので、**将来 `createInfo` に別の旗（スレッド数など）を立てる呼び出し点が見つかったときに区別できない**。また関数の入口に注釈を付けても、そのときの `r8` の意味（`createInfo` か否か）は呼び出し規約の仮定で、呼び出し点の注釈より前提が弱い |

**環境の定義としての意味**: 「この環境には実行可能メモリが無い。Blend2D には移植可能なパイプラインを使うよう指示する」。これは **ゲストの外から、ゲストが渡す引数を 1 語だけ書き換える**操作で、決定 0046 方針 4（黙って意味が変わる fail-soft を許さない）に対して: (i) 書き換える値の前提（ゲストが書いた 0）を検査し、違えば名前付きで停止する、(ii) 書き換えた後の挙動（パイプラインの提供者）をハーネスが確認できる（`FLE_B2D_SCAN` の attach 後の行 `pipeline provider fixed (no JIT)`）、(iii) 注釈を外すと JIT が戻って止まる（変異検査）。

**撤回条件**:

1. 実機の Windows で、FL Studio の `Agg2D` が JIT 有りで描いた画素が、この環境の固定パイプラインの画素と**異なる**ことが確認された（本作業は実機と照合していない。JIT 経路と固定経路の一致は X1 が 6 手順で測っただけ）。
2. Blend2D の別の版が使われる（RVA 0x56ce9 が `call blContextInitAs` でなくなる）。`fle-lift` が「命令が閉包に無い」でエラーにするので、黙って外れることはない。ただし RVA が別の命令に当たり、かつ `r8` の指す語が偶然 0 の場合は検出できない（その場合は画素の測定で見つかる）。
3. FLEngine が Blend2D を別の経路（`Agg2D::attach` 以外の `blContextInitAs`、`BLContext` を直接作る別のラッパー）で使うことが分かった。
4. ブラウザの wasm が実行時の機械語生成（`WebAssembly.instantiate` を実行中に呼ぶ）を採ることになった（M-10 の (b)）。そのときは JIT を有効に戻す選択もできる。
5. 固定パイプラインでは描けない／遅すぎる描画が G4 で見つかった（本作業の測定は 64×64 から 640×360 まで。速度は測っていない）。

## 実装した場所

| ファイル | 内容 |
|---|---|
| `crates/fe-lift/src/site_override.rs`（新規） | ファイルの解析、レジスタ名、RVA の解決、重複の検査、ガードの C 文字列。単体テスト 6 件 |
| `crates/fe-lift/src/lib.rs` | `generate_with_overrides`（旧 `generate_with_hooks` は `&[]` を渡す薄い包み）、`Stats::site_overrides`、ガードの合成（既存のフック点のガードと同じブロック）、`#include "fle_vmap.h"` の条件、「命令でない」と「閉包にあるが出力されない」のエラー |
| `crates/fe-lift/src/main.rs` | `--site-overrides FILE`、統計の `site_overrides N`（0 のときは出さない） |
| `runtime/fle_vmap.c`、`fle_vmap.h` | `fle_site_store32`（新しい .c/.h は作っていない）。**0119 が変更中のファイルではない**（0119 の差分は `fle_win.c` `fle_class.c` `fle_load.c` など。`fle_vmap.*` は 167a1db でも 018ba73 と同一） |
| `harness/hooks/blend2d-no-jit-sites.txt` | 注釈（根拠をコメントに書いた） |
| `harness/a4-multi.sh` | 第 7 欄 `SITES`、TAG とレポートへの反映（欄が空なら TAG は従来と同じ）、`FLE_BLEND2D_SCAN=1` |
| `harness/two/build_two_native.sh`、`build_nmod_wasm.sh`、`main_two.c` | `B2D_SCAN=1`（native と wasm の両方。未設定なら従来と同じコマンド） |
| `harness/b2d_scan.c`、`b2d_scan.h`、`b2d_scan_tests.h`、`b2d_scan_tests2.h`、`b2d_scan_table.h` | スロット走査のプローブ（`FLE_B2D_SCAN=<test>`、`list`）。native と wasm で同じソース（signal・alarm・backtrace は native の `extreme` の 20 秒の打ち切りだけ `#ifndef __EMSCRIPTEN__`） |
| `harness/seeds/blend2d-4700000-seeds964.txt` | X1 の 964 個の種（`docs/investigations/blend2d-jit-and-drawing-path/seeds/` と同一。`cmp` で確認） |
| `crates/fe-lift/tests/site_override.rs`（新規） | 11 件 |

**fe-decode は変えていない**（`diff -rq` で確認）。未解決 `jmp reg` の認識器にも触れていない（走査で停止が 0 だったため直す対象が無かった。下の「限界」）。

## 範囲

- Blend2D の `Agg2D::attach` が作る文脈を、JIT 無し（固定パイプライン）にする。
- 64×64（走査の標準）から 640×360、333×77（ストライドに余白）、48×900、3×3、1×1 の 32 ビット PRGB32 のバッファ、attach の繰り返し。

## 範囲外（今回扱わない）

- 画像・パターン・テキスト・`blit` に当たるスロット（Agg2D の入口に見当たらない。X1 §8.1）。**スロット 0〜129 のうち「呼んだ」ことと「正しい」ことは別**で、正しさを仕様から検査したのは 38 スロット（`docs/investigations/blend2d-nojit-slots.md` の表）。
- 実機との照合（0 件）。JIT の画素との照合も、X1 の 6 手順以外は**していない**（JIT 経路は native の gdb の実験でしか動かない）。
- 速度。FLEngine が実際に呼ぶ入口の頻度（`CreateFruityInstance` の先は未到達）。
- 暗黙 TLS の本格モデル（JIT 無しの経路では不要だった。実測: `FLE_B2D_TLS` なしで 138 テスト完走）、スレッド、JIT 以外の実行可能メモリ。**これらの壁には当たらなかった**ので「壁として記録して止める」ものは 0 件。
- ブラウザ（Chrome）での実行。**wasm の測定は node だけ**。

## 実測（要約。詳細は `docs/investigations/blend2d-nojit-slots.md`）

| 項目 | 値 |
|---|---|
| 基準（注釈なし。6 モジュール、FRUITY 無効、フォント配置あり） | a4 942/942、895/895、`returned from entry`、終了 0、functions 5,308、entries 1,007,947、native `27cdf764f10ffabc…`（`-g` を外した 3 ビルド、OUT の名前は 3 通り: `g1`、`g2`、`g-a-much-longer-output-directory-name-3`）。`-g` あり（`b1`）は `7831cbea96a5245f…`、`b2`（別の OUT 名）は `1fad5a72417841dd…` |
| SHA の差の原因 | 112 個の `.o` のうち 107 が違い、**107 すべてが `objcopy --strip-debug` 後に一致**（5 個は元から一致）。リンク後のバイナリの差は `.note.gnu.build-id`、`.debug_str`、`.debug_line_str` の 3 セクションだけで、`.text`、`.rodata`、`.data` は一致。`.debug_str` に OUT のパス（`.../out-x2/b1/...`）が入る（`DW_AT_name`、`DW_AT_comp_dir`）。**パスの文字数が同じでも内容が違えば SHA が違う**。パスを入れない（`-g` を外す）と OUT の名前を変えても SHA は同一 |
| 注釈つき 6 モジュール（既定のスキャン無しのビルド `a1`） | a4 は基準と同一（942/942、895/895、5,308、1,007,947、終了 0）。生成 C: FLEngine と m1・m3・m4・m5 は基準と同一の SHA-256、m2（Blend2D）は 1 行追加・1 行変更のみ（418,078 行を逐次比較） |
| 注釈なし・runtime 変更あり（`n0`） | 生成 C 同一、a4 同一。`.o` は 112 個中 `fle_vmap.o` の 1 個だけが違う（runtime に関数を足したため）。native の SHA は基準と違う（`349c9190…`）。**`fle_vmap.c` を変えたので native の SHA は `27cdf764…` から変わる**（これは変更の帰結で、不具合ではない）。a4 の出力は同一 |
| スロット走査（native、138 テスト、1 プロセス 1 テスト） | 全部 `status=done`、stderr 0 バイト、130 スロットすべてを 1 回以上呼んだ。CHECK 100 件（SPEC 80、READING 20）が全部 PASS、FAIL 0。画素ハッシュが異なるテスト 73 種 |
| wasm（node、FORCE、`-sJSPI` なし。flengine6.wasm 453,210,447 B、ビルド: units 4:15（最大 RSS 8.4 GiB）、o1 29:19、link 0:09） | a4 の出力は native と**パスと実アドレスの 2 行以外同一**（functions 5,315、entries 1,007,959。これは走査のビルドで Blend2D の種が 964 個のため。「解釈が分かれた点」）。138 テストの `RESULT`／`CHECK`／`VALUE` が native と**全部一致**（差 0）。極端な値 128 通りも一致 |
| 変異検査（機構。12 件） | 11 件が検出、1 件は対照（無変更。PASS が正）。さらに実行時の変異 2 件（注釈の前提値を間違える、書き込む値の bit 0 を立てない）も検出 |
| テスト | `fe-lift` の全部。2 回目の通しは時間切れ（3,600 秒）で途中まで。「テスト」の節 |

## テスト

`site_override.rs` の単体 6 件、`tests/site_override.rs` 11 件（合成 PE。期待値は命令の意味と決定から書いた）:

| 区分 | 要点 |
|---|---|
| 生成器 | ガードと include だけが旧出力との差（取り除くと旧出力とバイト一致）／サイトなしは旧出力とバイト一致／命令でない VA（命令の途中、関数の外、イメージの外、0）と重複はエラー／2 つのサイトは各 1 ガード／コマンドラインが RVA、VA、レジスタ+変位、コメントを読む／構文エラー（2 行目）、命令でない点、EXPECT=NEW、ファイル無しで終了 |
| 実行 | 注釈した呼び出しは新しい語を見て、注釈していない関数は古い語を見る／戻りを 2 回呼んでも同じ／語が既に NEW なら何もしない／隣の語は変わらず 32 ビット全体を比べる／EXPECT でも NEW でもない値は名前付きで停止 |

全体テスト（`cargo test --offline -p fe-lift --no-fail-fast`）の経過: (1) 最初の通しは 50 分 23 秒で終了 101、**9 件失敗**: すべて `authenticode_check`（`tools/authenticode_check.py` と `binaries/win64/FLEngine_x64.dll` が `git archive HEAD lift docs` の木に無かったため。`tools/` と DLL へのリンクを足して `authenticode_check` を単独で再実行し 14 件通った）。(2) 2 回目の通しは 3,600 秒の上限で打ち切られた。統合テストの binary 92 本のうち 83 本が走り（lib・doc を含めて 84 の test result）、**1,278 passed、1 failed**。失敗の 1 件は `wall_fr5::the_entry_is_chosen_by_the_rules_of_lookup_icon_id_from_directory_ex`（`FLENGINE_PE` なしでも、`binaries/win64/FLEngine_x64.dll` がある環境で 256×256 の RT_ICON 6 が PNG 圧縮で `fle_icon_group_pick` が名前付きで止まる）。**`git archive 018ba73` の clean な木（私の変更なし）でも同じ失敗**を実測した（`binaries/win64/FLEngine_x64.dll` があると落ち、無いと SKIP で通る）。この差分は `fle_res.c` と `wall_fr5.rs` を変えていない（`cmp` で同一）。**決定 0119 の全体テストの記録（HEAD の clean archive でも同じ結果）と同じで、決定 0121（HEAD cfb2b1e）が原因を「テストの期待の誤り（256×256 の MAINICON エントリは PNG 圧縮）」と切り分けてテスト側を直した**（`docs/investigations/fruity7/wall-fr5-triage.md`）。本作業の差分はこれに触れていないので、cfb2b1e の木ではこの 1 件は通るはず（**cfb2b1e の木では走らせていない**）。(3) 通しで走らなかった 9 本と打ち切り時に実行中だった `wall_q` の計 10 本（`wall_q`、`wall_q_lift`、`wall_r`、`wall_r2`、`wall_s`、`wasm_port`、`window`、`x87`、`x87ctl`、`x87_trans`）は個別に実行して**すべて通った**（5、3、31、17、15、6、36、14、7、9 件）。**1 回の通しで全部が通ったわけではない**。`site_override`（11）、`hook_points`（17）、`fe-lift` の lib 単体（20）は通った。

## 変異検査

`tools/mutate_x2.py`（木のコピー `mut/` で 1 件ずつ変更し、対象のテストを走らせ、元に戻して `cmp`）。**1 回目は全 12 件が検出と出たが、cargo が mtime で再ビルドを判断するため、復元したファイルの古い mtime でビルドが再利用された可能性があり、その結果は破棄した**（`logs/mutation-results-run1-DISCARDED-stale-build.txt`）。スクリプトに (a) 変更と復元の後に `utime`、(b) 変異の前に対照の実行（通らなければその変異は「無効」）を足して取り直した（`logs/mutation-results.txt`）:

| # | 変異 | 検出したテスト |
|---|---|---|
| M1 | ガードを出さない（上書きが黙って外れる＝JIT が戻る） | 5 件（生成器 3、実行 2 ほか） |
| M2 | `fle_site_store32` が書かない | 3 件（実行） |
| M3 | 予期しない語を停止せず上書きする | 1 件 |
| M4 | 語が既に NEW のとき停止する（ループで止まる） | 1 件 |
| M5 | ガードが別のレジスタを読む | 1 件（単体） |
| M6 | 対照（無変更） | PASS（正） |
| M7 | 命令でないサイトを受理する | 2 件 |
| M8 | include を出さない | 5 件（生成 C がコンパイルできない） |
| M9 | EXPECT=NEW を受理する | 1 件（単体） |
| M10 | コマンドラインが `--site-overrides` を無視する | 2 件 |
| M11 | RVA に基底を足さない | 1 件（単体） |
| M12 | 決定 0105 のフック点のガードが消える（回帰） | 7 件（`hook_points`） |

**実機能の変異（実際の 6 モジュール構成。`logs/mx-*`）**: 注釈の EXPECT を 5 にすると、`clear` も `rect` も `fle: site override: site 0x4756ce9: the word at … is 0; the override expects 0x5 … (decision 0120)` で止まる。NEW を 2（bit 0 が立たない）にすると、提供者が JIT（`NOT the fixed provider`）のまま、`indirect call to 0x…40 is not a known function entry`（X1 と同じ停止）。注釈なしの構成（`e1`）も同じ停止（`clear` と `rect` は dispatch miss、`path`・`ellipse` は `unsupported instruction at 0x489b1b5`＝X1 の記録と同じ）。

## 検証（実測）

- **注釈なしの構成の生成 C がバイト不変**: 実装前の 6 モジュールの既定ビルド（`g1`）と、実装後で注釈なしのビルド（`n0`）で、FLEngine と m1〜m5 の生成 C の SHA-256 が一致（`logs/gen-g1.sha`、`logs/gen-n0.sha`）。
- **注釈ありの Blend2D の生成 C**: 注釈なしの `fe-lift lift ... --pdata --seeds seeds58` の出力（SHA-256 `7d41e5ec…`）が、既定ビルドの m2 と一致し、注釈あり（`a1` の m2）との逐次比較で、418,078 行のうち 1 行追加（`#include "fle_vmap.h"`）と 1 行変更（`L_4756ce9`）。964 個の種のビルド（走査用）でも同じ形（1,206,622 行、`logs/gen-diff-site-vs-plain.txt`）。
- **差分の当たり方**: `x2.diff` は `git archive 018ba73 lift docs` の clean な木に `patch -p1 --dry-run` で当たる（`logs/patch-verify.txt`）。167a1db の木には**当てていない**（同じファイルが変わっていないことだけ確認した）。

## 出典なしの選択

| 選択 | 内容 |
|---|---|
| 注釈の書式 | `ADDR store32 REG[+DISP] EXPECT NEW`。0105 の `--hook-points` に倣ったが、書式そのものは作業者の設計 |
| 値が違うときの停止 | EXPECT でも NEW でもなければ停止。`fail-soft` を避ける（0046 方針 4）。仕様ではない |
| 付ける場所 | `Agg2D::attach` の `blContextInitAs` の呼び出し点（RVA 0x56ce9）。「他に呼び出し点が無い」は静的な `xref` 1 件（読み） |
| 走査のバッファ | 背景 `33 22 11 ff`、64×64、ストライド 256。値は X1 に倣った |
| 固定パイプラインが「正しい」の基準 | 仕様から手計算できるもの（単色、source-over、矩形、`clipBox`、ベジエの中点、楕円の面積、グラデーションの端と中央）のみ SPEC。それ以外は READING（ゲストの関数の読みに対する検査）か、ハッシュのみ |

## 解釈が分かれた点

1. **走査のビルドの functions／entries が基準（5,308／1,007,947）と違う（5,315／1,007,959）**: 走査用の Blend2D は X1 の 964 個の種で作る（種 58 個の基準とは別。DllMain の経路が少し変わる。X1 も同じ差を報告し、原因の同定はしていない）。**種 58 個の構成では注釈つきでも 5,308／1,007,947 で基準と同一**（実測）。
2. **種 964 個は 6 手順だけの保証ではなくなった**: 走査 138 テスト（native）で Blend2D の関数 3,093 個のうち**少なくとも 350 個**が実行され、停止は 0。ただし未解決の `jmp reg` サイトは **81 個（出力の複製を含めて 351 か所）**残っており、そのうち**走査が関数に入った**のは 18 サイト。**走査が通っていないサイトに当たる入力（画像、テキストなど）は未測定**。
3. **`-O1` の別翻訳単位は 63 関数（`a4-six-module-wasm-verification` の「19 個」（dsp 15 + Blend2D 4）は種 58 個の構成の数。種 964 個の本ビルドでは Blend2D が増える）**: dsp 15（`m1-05.c` に 10、`m1-10.c` に 5）と Blend2D 48（`m2-02.c`）。依頼が挙げた 4 関数（`f_4859b70`、`f_4865f60`、`f_4871d60`、`f_487e100`）は Blend2D の 48 のうちの 4 個。**走査で 1 回以上実行されたのは 44 関数で、すべて Blend2D**（48 のうち `f_48643d6`、`f_487026b`、`f_487c551`、`f_48883c7` の 4 個は走査で入らなかった）。dsp の 15 個は走査で 0 回（Blend2D の走査は dsp を呼ばない）。**-O1 版の一致を測れたのは Blend2D の 44 関数だけ**で、dsp の 15 関数と Blend2D の 4 関数の -O1 版の正しさは測っていない。
4. **`aligned_alloc(64, 4)` が emscripten で NULL を返す**: C11 はサイズが整列の倍数でなければならないとしており、glibc は許すが emscripten の `aligned_alloc` は NULL を返す（`errno` も 0）。私のプローブの `tiny` が wasm でだけ `rc=99`（ヒープ破壊の検査）になった原因。**プローブを直した**（サイズを 64 の倍数に切り上げる）。ゲストの Blend2D や runtime の不具合ではない。

## 限界・未確認

- **実機の Windows との照合は 0 件**。
- **ブラウザでは測っていない**。wasm は node のみ（FORCE、`-sJSPI` なし）。
- 「正しい」と言えるのは SPEC の 38 スロットの範囲。22 スロット（0、1、13、15、22、57、59、60、69、91、92、94、95、96、98、99、100、101、122、123、128、129）は**画素ハッシュのみ**（native と wasm の一致は測った）。READING の 18 スロット（6、11、12、23、25、26、29、43、44、45、46、47、48、49、68、72、73、84）は、**ゲストの関数の読みに対する検査**で、仕様との照合ではない。
- スロット 15（`blendMode`）: 29 種を測ったが、本当に効いた（画素が変わった）のは **0、1、3、13、25〜28 の 8 種**で、**2、4〜12、14〜24 は背景のままの画素**（`33 22 11 ff`）だった。0 と 1 は `80 00 00 80`（ソースの単純コピー）。これが仕様どおりかは**検証していない**（SPEC の期待値を手計算していない）。「23 種のブレンドモード」という X1 の数は、このテーブルの 0〜28 とは別の数え方。
- **走査が手で選んだ引数に依存する**。引数の読み（`double` が xmm の位置、整数が `rdx`／`r8`／`r9`、スタック）は逆アセンブルの読みで、**スロット 69（star）、91（arcTo）、92（arcRel）、122、123 などは引数の意味を確認していない**（呼べたこと、画素が変わったことだけ）。
- **FLEngine が実際に呼ぶ入口の優先順**: 静的な数（窓 200 命令）は多い順に 103（50）、85（44）、82（31）、42（27）、121（25）、11（23）、9（22）、83（21）、26（17）、63（17）。上界（`call [reg+disp]` の変位が 8×スロットの全部。他の vtable も数える）は 3,603（スロット 3）などで、**過大**。窓 200 で数えた 99 か所は、130 の呼び出し点のうち 99。**呼び出し点のうち 31（130 − 99）は静的には追えていない**。
- **ハング 4 通り**: 楕円の半径が NaN／+inf／-inf／DBL_MAX（slot 67）。native で 20 秒で止まらない（`alarm` で打ち切った）。**wasm では実行していない**。実機の Blend2D が同じ入力でどうなるかは未確認。FLEngine がそのような値を渡すかも未確認。
- `fuzz` は決定的な乱数の 40 シード × 150 手で、停止 0。**乱数のシードを増やせばハングが見つかる可能性は否定できない**（極端な値では見つかった）。
- 速度は測っていない（最終の走りで wasm の 138 テストの実行は 513 秒、native は 80 秒。プロセスごとに DllMain を実行する分を含む）。
- **走査のビルドの native SHA は決定 0118 の `27cdf764…` と違う**（走査のコードが入るため、さらに注釈なしでも `fle_vmap.c` の変更で変わる）。`FLE_BLEND2D_SCAN` を付けない既定のビルドで SHA を比べるなら、`fle_vmap.c` を変えた分は、**同じ runtime の注釈なしの構成と比べる**必要がある（`n0`: `349c9190…`、注釈あり `a1`: `8d78c920…`）。

## 規則違反の自己申告

「報告」の節にまとめた（`MERGE-NOTES.md` にも同じ）。
