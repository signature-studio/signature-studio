# 0284c(仮): `fle_ia2_on()` を無条件にする(no-options 違反の除去。0284 の B。mt3w)

状態: **仮**。作業者 mt3w、2026-11-06 JST。基準: main `c74b9194` のコード + 0284 の第 1 報(`0284-compare-string.diff`)(= w79 の `f83ec04`)。差分: `0284c-ia2-always-on.diff`(18 ファイル、+197/−250)。A(表で S- の選択を置換)とは独立。区分: **実測**=出力を見た値/**読み**=コードを読んだ推論/**未確認**。

## 0. 冒頭の三点(正直さ)

1. **実エンジンが実行されたか**: 実行した。(a) `harness/test.sh full`(fe-lift ほか 212 実行体の cargo test)、(b) a4(`a4-multi.sh`。FLEngine 本物 + 実モジュール 5 つの lift した C を native にビルドして走らせた物)、(c) 本番形の構成(`b2d2/gen-direct.sh` 相当、FLEngine + 実モジュール 5 つ)の生成 C を、変更の前後で作って比べた。**ブラウザ・wasm の実エンジンの走行は 0 件**(opt1 の領分)。代役・再生・推測の画面や音は使っていない。
2. **この変更は『挙動を変えない』変更**: 本番の import 表(ship2/4/5)には `CompareStringW::ia2-sort-weights` の行(= `fle_imp_ia2_marker`)が必ず入っていたので、`fle_ia2_on()` は本番では常に真だった。変更はその『行があれば』の条件を消し、行も消す。**本番の挙動の差は、行を消したことによる疑似 VA のずれだけ**(§2)。
3. **未確認(最大)**: 疑似 VA が 101 行ぶん −0x10 ずれる(§2)。生成 C の差はそれだけ(§2 の表)で、ゲストのコードの行は 1 行も変わらない。ゲストが import の疑似 VA の『値』そのものに依存する経路(例: GetProcAddress の答えを整数として比較・並べ替える)があるかは**読んでいない**。実エンジン(opt1)の node 行(g3 など)で見る(§5)。

## 1. 何を変えたか(読み。18 ファイル)

| ファイル | 変更 |
|---|---|
| `runtime/fle_locale.c` | `fle_ia2_on()` を消す。ia2 の重み(`fle_ia2_cmp_weight`・`fle_ia2_cmp_primary`・`ia2_unit_weight`、`#include "fle_ia2_sortw.inc"`)を `fle_wall_ia2.c` から**ここへ移す**(static)。`fle_pua()` の `&& fle_ia2_on()` を消す。weak の既定(『`fle_wall_ia2.c` をリンクしない build は昔のまま』)も消す |
| `runtime/fle_wall_ia2.c` / `.h` | `fle_ia2_on()`・`fle_imp_ia2_marker`・重みの関数群を消す(−113/−11 行)。残りは wall の実装本体 |
| `runtime/fle_gdi.c` | `RectVisible` の条件(『layer-4 の marker があれば関数、なければ 0111 の名前付き停止』)を消す: 常に関数 |
| `runtime/fle_user_nowin.c` | `LoadLibraryW("Python312.dll")` が『見つからない』と答える条件、PUA の条件を無条件に |
| `harness/seeds/{ia2,ship2,ship4,ship5}-imports.txt` | marker 行を消す(1 行ずつ)。コメント 1 箇所の『marker row』の言及を消す |
| `harness/parity-paint/{parity_traceall.c,traceall-wrap-flags.txt}` | `fle_ia2_on` の wrap を消す |
| `crates/fe-lift/tests/wall_ia2.rs` | `marker` の引数を消す。marker=false のテストを消す/変える。診断のテストは U+00AD 対 'O' に |
| `crates/fe-lift/tests/{locale,wall_dlg1,wall_fr11,wall_fr12,wall_r2,gdi_bulk}.rs` | §4 |

残りの `fle_ia2_on` は 0 件(`grep` で確認: runtime・harness・crates/*/src・ci・app/scripts・tools に無い。docs の過去の記録〔s-choices.tsv の S-ia2-sortw-marker ほか〕は履歴なので変えていない)。

## 2. 生成 C の前後(**実測**: `b2d2/gen-direct.sh` 相当、FLEngine + 実モジュール 5 つ、`tools/rungen.sh`)

前 = `f83ec04`、後 = 本差分の適用後。7 つのファイル(`flengine.c`、`m1.c`〜`m5.c`、`common.c`)の `diff`(`tables/0284c-generated-c-before-after-classification.tsv`、生の差 `tables/0284c-generated-c-flengine.c.diff`)。

| 区分 | 行数(各ファイル) | 内容 |
|---|---|---|
| 前にだけある行(`<`) | 207 | 宣言 `void fle_imp_ia2_marker(void);` 1、dispatch 表の marker 行 1、**dispatch 表の marker より後の行 101**(疑似 VA が 0x10 ずれる前の姿)、resp 表の marker 行 1、**resp 表の marker より後の行 101**、count 行 2(`fle_dispatch_count`・`fle_resp_count`) |
| 後にだけある行(`>`) | 204 | 同じ 101 + 101 行の−0x10 した姿、count 行 2(−1 ずつ) |
| 分類できなかった行 | **0** | |

- 7 ファイルとも 207/204 で同じ構造(表は各ファイルに include されている同じ物)。dispatch 表の 101 行・resp 表の 101 行は、**関数名と DLL 名・API 名が同じで、疑似 VA だけが全部ちょうど −0x10**(`bad 0`)。marker の行が 1 つ消えて、その後ろの行が 1 つずつ前に詰まっただけ。
- **ゲストのコード(lift した関数)の行は 1 行も変わっていない。** dispatch 表と resp 表の外の差は、宣言 1 行と count 行だけ。
- 生成 C の sha256(前後 7 つずつ)は `tables/0284c-generated-c-sha256-{before,after}.txt`。

**PM への注意(決定 0015、ship5 のヘッダ『前の行は動かさない』)**: marker 行を消すと、その後ろの疑似 VA がずれる。**消さずに残す(使わない行として)ことは、no-options の趣旨(死んだ行を残さない)に反するので採らなかった**。ずれを受け入れる判断をお願いします。ずれの影響が出うるのは、ゲストが import の疑似 VA の値に依存する場合だけ(§0 の 3)。

## 3. 同一性の根拠(読み)

- 本番(ship2/4/5)の import 表は marker 行を含む → `fle_ia2_on()` は真 → 重みの経路・PUA・RectVisible・Python312.dll の『見つからない』は有効だった。変更後は無条件に有効。同じ挙動。
- marker 行の無い構成(テストの一部、`gen_two.sh` の既定など)は、昔は『ia2 無し』だったが、本番では存在しない構成。この変更でそれらも本番と同じ挙動になる(テストの期待値を §4 で書き直した)。

## 4. テストの期待値の書き直し(手で導いた値。ランタイムの出力を写していない)

| テスト | 昔 | 今 | 根拠(Microsoft の Sorting Weight Table、`Windows10-SortingWeightTable.txt`、SHA-256 81fcfa1e…) |
|---|---|---|---|
| `locale.rs` `compare_string_symbol_outside_rule_aborts` | U+00E9 対 "a"、U+2022 対 "a" は停止 | 停止の行から外し、新しいテスト `compare_string_nonascii_units_by_the_sorting_weight_table` へ: 3 / 1 / 1 / 3 | U+00E9 = 14/33、"a" = 14/2 → 大きい。U+2022 = 11/39(SM 11 < 14)→ 小さい |
| `locale.rs` `lcmap_stops_locale_zero_…` のスイープ | 範囲 D の外は全部停止: 4×(65536−284) | 4×(65536−284−6400) 停止、4×(284+6400) 応答 | PUA U+E000..F8FF = 0x1900 = 6,400 単位が、決定 0191 で全 build で写像される |
| `wall_dlg1.rs` ヒフン規則 | 出力 10 行 + カナで停止 | 12 行(kana-vs-ascii 1、ascii-vs-kana 3)+ 最後の `a-b` 対 `ab-` で停止(メッセージが変わる) | U+30AA = SM 3、"U" = 14 → カナが小さい |
| `wall_fr11.rs` | 共通の接頭辞の制御文字 0x01 は停止 | `a\x01b` 対 `a\x01c` = 1(逆 3) | U+0001 = 6/3(重みあり)、同じ重み同士は決めず、'b'(14/9)対 'c'(14/10) |
| `wall_fr12.rs` | 一般句読点対文字は停止 | U+2013 対 'x' = 1(逆 3) | U+2013 = SM 6、'x' = 14 |
| `wall_r2.rs` | `0xe9` 対 "a"、"a b" 対 "a"+U+00E9、`a\x01b` 対 `a\x01c` は停止 | 3、1、1/3 | 上と同じ。**"ABC" 対 "abc"(case-only)は停止のまま**(B の対象ではない。ASCII の大文字小文字のみの差は S- の選択の外で、A で扱う) |
| `gdi_bulk.rs` | `RectVisible` は名前付き停止(45 行) | 44 行(RectVisible は応答) | |

## 5. テスト結果(実測。`w79`、`CARGO_BUILD_JOBS=3`)

**`harness/test.sh full`**(`tables/0284c-test-full.log`): `212 of 212 test executables selected + doc-tests`、**results=216 passed=2399 failed=0 ignored=1 SKIPPED-lines=1**(SKIPPED は `fe-lift/wall_fr9t` の `FLE_FR9T_CLEAN_RT is not set` の 1 件で、0279 の full と同じ)。executables run=107 **cached=105**(キャッシュされた実行体は、同じ実行体・環境・入力ファイルの最後の合格をそのまま使う=走らせていない。B で変わった物は走っている)、tests run=1680 cached=719。壁時計 161 s。0279 の full(passed=2395)との差 +4 は B が足した/変えたテスト。

**a4**(`tools/a4run.sh`: 実モジュール 5 つ〔dsp・blend2d・qfc・freetype・iltools〕+ FLEngine、`FLE_SEEDS=flengine-seeds-thunks.txt`・`FLE_HOOKS=flengine-hook-points.txt`・`FLE_SEH_HANDLERS=0x112c0`、`EXPECT_FLE_SHA=skip`):

| | init_count | runnable started/completed | functions distinct | function entries | started_but_not_entered | harness exit | FLEngine の生成 C sha256 |
|---|---|---|---|---|---|---|---|
| **B(本差分)** | 942 of 942 | 895 / 895 | 5,310 | 1,009,737 | 0 | 0 | 2715c3f3… |
| 基準(`f83ec04`、同じ手順)| 942 of 942 | 895 / 895 | 5,310 | 1,009,737 | 0 | 0 | 2715c3f3…(同一) |

(`tables/0284c-a4-B.log`、`tables/0284c-a4-base-f83ec04.log`。期待値〔942/942・895/895・5,310・1,009,737・0〕と一致。)この構成の import 表には marker 行が無いので、B は FLEngine の生成 C を変えない(sha が同一)。**`a4-multi.sh` の既定の期待 sha `f6d3a971…` はこの構成では基準でも合わない**(`2715c3f3…`)ので、`EXPECT_FLE_SHA=skip` で走らせた。この pin が古いのは B の前からで、B のせいではない(基準でも不一致)。最初に FLE_SEEDS/FLE_HOOKS なしで走らせたときは、FLEngine の自分の hook point への VirtualProtect で 315 of 942 で止まった(基準でも同じ 315 of 942、B と無関係の構成の取り違え。上の構成に直して解決)。

**`ci/test-hybrid.sh`**: `all checks hold`(ship5 = ship4 + ref1 の行、が marker 行の削除後も成り立つ)。

**差分のドライラン**: `patch -p1 --dry-run`が (a) `f83ec04` の export、(b) main `c74b9194` + `0284-compare-string.diff` の上で、どちらも全 18 ファイルでエラーなし。

## 6. 実エンジン(opt1)で見る物

- 疑似 VA が 101 行ずれた build(E10 以降)と E9 の node 行(g3 の progress/slices/surface/wintable、Export の sha、first idle)を比べる。**差が出るなら、ゲストが疑似 VA の値に依存する経路があるということ(未確認の最大の点)**。
- 差が出ないことが期待(読み)。

## 7. 疑似 VA の −0x10 のずれは害か(静的な読み。PM の 12:10 の依頼 (a)(b)(c)。コードの変更なし)

**まず何がずれるか(読み。`imports_table.rs:779–861`、決定 0015 の 41–43 行)**: 応答表の行 k(0 始まり)の擬似 VA は `0x7FF900000000 + 0x10 × (k + 1)`。**行の位置で決まる**ので、marker 行を消すと後ろの行の VA が 0x10 ずつ前に詰まる(実測: 101 行、全部ちょうど −0x10。§2)。順序と一意性は保たれる(テスト `dispatch_table_stays_sorted_and_pseudo_vas_are_unique` は full で合格)。この VA は **GetProcAddress の戻り値と、COM の vtable の埋め込みにだけ**使われる(下の (a))。静的にリンクされた import(`jmp_imp:` の行)は fe-lift が直接呼びに解決するので、ゲストの IAT の値としては現れない(読み)。

**(a) runtime が擬似 VA を使う所(読み。`fle_resp_table`・`0x7FF9` を runtime と harness で grep)**
| 場所 | 使い方 | VA の値への依存 |
|---|---|---|
| `fle_modules.c:469` GetProcAddress | (DLL 名, API 名) で表を引き、`R[0] = fle_resp_table[i].va` を返す | 値そのものは不透明に返すだけ |
| `fle_ole.c:734`、`fle_filedlg.c:232,241,274` | shell32 の行(`IMalloc::Release`、`IFileOpenDialog::Show` ほか)を**名前**で引き、その `va` を COM の vtable の枠に書く | 名前で引く。これらの行は marker より**前**(dlg1/ia1 の層)なので VA は動かない(読み。ずれる 101 行は ScrollWindowEx〔ia2 の層 4〕以降) |
| `fle_dispatch.h`(`fle_dispatch`) | 間接呼びの target を `fle_dispatch_table` の二分探索で完全一致 | 昇順と一意性だけ(保たれる) |
| `fle_dsound.h:12` の `FLE_COM_SLOT_BASE = 0x7FF940000000` | COM のメソッド枠の別の範囲 | 応答表の行の上に固定(現在の最大の行は `0x7ff900003330`)。ずれで衝突しない |
| `harness/two/wrp1_vst3.c`、`harness/vst3a/` | VST3 の harness(本番外) | — |
runtime に『擬似 VA の範囲(`>= 0x7FF9...`)かどうか』で分岐する所、擬似 VA の演算(加減算・ビット演算・ハッシュ)をする所は**見つからなかった**(上の grep に出た物がすべて)。`app/`(ページ・JS)にも `7ff9` は無い。

**(b) ゲストの lift した C の中の擬似 VA(実測 + 読み)**: FLEngine と実モジュール 5 つの生成 C(§2 の前後各 7 ファイル)で、`0x7ff9` + 8 桁の 16 進の定数は、**dispatch 表と resp 表の行以外に 0 件**(FLEngine: 表の外 0 件。m1〜m5: 各ファイルの該当行 1,638 = 819 + 819 で全部が表の行)。前後の diff に出た差もこの 2 つの表だけ(§2)。つまり、ゲストが擬似 VA を**定数として**持つ形は無い。残るのは、GetProcAddress が返した値をゲストが**実行時に**比べる・並べる・ハッシュにする形で、これは生成 C を読んでも見えない(任意の load と比較になっている)。値の**順序と等値**は保たれる(単調で一意に詰まるだけ)。壊れうるのは、ゲストが擬似 VA の**絶対値**でハッシュの bucket を決める場合(例: ポインタの `shr 4` を key にする辞書で、キーに GetProcAddress の戻り値が入る)だけで、−0x10 は `shr 4` で 1 bucket のずれ。FL がそういう辞書に API のアドレスを入れるかは**読んでいない(未確認)**。

**(c) 『前の行は動かさない』(決定 0015)の意図**: ship5・ship4 のヘッダの文言は『so that the pseudo-VAs of the earlier rows do not move (fe-lift gives a row its pseudo VA by its position)』で、**ビルド間で VA を安定させる約束**(位置が VA になるため、追記だけを許す)。**同じ種類のずれで何かが壊れた記録は、私が探した範囲では見つからなかった**(`git log -S"ia2-sort-weights" -- lift/harness/seeds` の 0191・0208・0218・0230 の記述は、すべて『追記・順序固定で earlier rows は動かない』形で、ずれた例そのものが無い)。逆に言えば、**ずれても大丈夫だった前例も無い**。0208 に『ia2-imports.txt に exp1 の 6 行が欠けていた』という、行の欠落を直した記録はあるが、そのときの VA への影響は書かれていない。

**動的な確認(実測。この項は a4 の出力が出た後に埋める)**: 本番の import 表(`FLE_EXTRA_IMPORTS=ship5-imports.txt`)で a4 を基準(`f83ec04`、marker あり)と B(marker なし、101 行がずれる)で走らせる: **a4 の init の全体(942 unit、895 runnable)が基準と B で同一**(`tables/0284c-a4-ship5-{base-f83ec04,B}.log`、`tools/a4ship.sh`): どちらも init_count=942 of 942、runnable started/completed 895/895、functions distinct **5,312**、function entries **1,009,745**、started_but_not_entered 0、exit 0。FLEngine の生成 C の sha は基準と B で異なる(疑似 VA のずれが入っている証拠)。(ship5 の行が足す関数があるので、5,312・1,009,745 は ship の既定構成の 5,310・1,009,737 と違う。)GetProcAddress を使う起動時の経路(delay-load、COM の vtable 構築)がずれた VA で全部通った、という確認。ブラウザの node 行(g3 など)ではない。

**結論(読み)**: 静的に、害になりうる形は (1) ゲストが GetProcAddress の値の絶対値でハッシュを作る/辞書の bucket を決める、の 1 つだけで、その形が FL にあるかは未確認。runtime 側に VA の値へ依存する所は見つからなかった。**E11 に単独で載せ、opt1 が node の行(g3-progress/surface/wintable、Export の sha、first idle)を E9/E10 と比べる**のが、残る確認として妥当(差が出なければ害なし、出れば原因は疑似 VA の 1 つに絞れる)。

## PM の判断と取り込み(2026-11-06 12:30、docs のみ)

取り込み: mt3 の 0284 B の文書・表(生成 C の前後の分類、a4 の前後ログ、test.sh full)・差分 `0284c-ia2-always-on.diff`(18 ファイル +197/-250)を置いた。PM の dry-run: 0286 → 0284 第 1 報 → 0284c の順に e9-push へ rc=0、offset 0、fuzz 0。mt3 の検証は native(test.sh full 216 results / 2,399 passed / 0 failed / 1 SKIPPED〔wall_fr9t〕、a4 942/942・895/895・5,312 関数・1,009,745 entries・started_but_not_entered 0 を標識あり/なしの両方で同じ値、FLEngine の生成 C の sha は変わる)。ブラウザ・wasm の実エンジンの走行は 0 件。
判断: **本番では `fle_ia2_on()` が常に真なので挙動は変わらない設計だが、import の疑似 VA(0x7FF900000000 + 0x10×(行+1))が標識行の除去で −0x10 ずれる行が 101 行ある**(ScrollWindowEx より後)。mt3 の静的な読みでは疑似 VA は GetProcAddress の戻り値・COM vtable の slot・fle_dispatch の二分探索にしか使われず、ランタイムに範囲やハッシュで分岐する所はないが、ゲストが GetProcAddress の戻りを絶対値でハッシュする可能性は未読。よって **E10 には載せず、E10 の後に単独で(E12)** 出し、opt1 が node の行(g3-progress / surface / wintable)・Export・first idle を E11 と比べる。no-options 違反の除去として方針に沿う(採る)。
