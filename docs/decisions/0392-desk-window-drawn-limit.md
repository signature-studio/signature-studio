# 0392: 上限 64 は描かれる窓で数え、非表示の窓は数えない。窓の一覧が上限(4096)に達したら名前つきで停止する(F-7、R3-5)
分類: (a) 除去。上限 64 は残す。黙った切り捨ては除いた。足したのは名前つき停止 2 本(描かれる窓の 65 件目。窓の一覧の上限 4096 への到達。どちらも Windows に無い壁で、F-7 の選択)と全面再送 1 本(既存の resync 経路。0385 のまま)。
撤回条件: 実機または実エンジンで `fle: fle_desk_gather:` の停止が出たら、その時の窓表(描かれる窓の件数と非表示の件数、原因の窓)を記録し、この決定を見直す。停止の文面が「reach the limit of the window list」なら、生きている最上位窓の件数を記録する。上限 64 を広げるときは `app/src/engine/desk-gl.ts` の SCENE_MAX_WINDOWS と SCENE_MAX_UPDATES を同じ commit で変える。`fle_win.c` の SIB_MAX を変えたら `DESK_LIST_MAX`(fle_desk.c)を同じ値にする。
日付: 2026-11-16   ROADMAP: R3-5   一覧: L22(F-7)   コミット: 3ded666・4f10fc4(0385 の版)→ 9e0f3a5(コードとテスト)→ この文書の commit(後続)

## 決定
- 置き換える文(0385 の決定 1 行目と読み 4): 0385 の「決定: 上限 FLE_DESK_MAX_WINDOWS(64)は外さない。最上位窓が 65 以上あるとき、`gather()` … が `fle_stop("fle_desk_gather", …)` で名前つきに停止する。」と、読み 4 の「停止の条件は、非表示の窓も含めた最上位窓の数(`fle_win_input_toplevels` の返り値が 65 以上)。非表示の窓は描かれないが、数える。」を、次の決定で置き換える。
- 決定: 停止の条件は「描かれる窓(可視で、面(surface)があり、層の状態が NONE でない窓)が 65 件目に達したとき」。非表示の窓は数えず、停止は 65 件目を `cur[]` に格納する直前(`gather()`、`fle_desk.c`)で行う。根拠: 描かれない窓の件数で止まると、描画に関係のない窓が増えただけで実行が止まる(0385 の読みの弱点)。
- 決定: 窓の一覧は、生きている最上位窓を全件読む(配列 `DESK_LIST_MAX` = 4096 件)。返り値が 4096 に達したら、切られた可能性があるので名前つきで停止する。根拠: 件数の上限が一覧の中に隠れると、黙った切り捨てが残る。
- 決定: scene の溢れ(`scene_wrote` の全面の再送、0385 の決定 2)は変えない。

## 実測(コマンド・条件・日付・n)
1. 0385 の根拠の見直し(2026-11-16、`sed -n 288,299p evidence/R5-1/engine-log-run2.txt | grep -o "visible [01]" | sort | uniq -c`): G2 証拠の窓表(最初の可視の窓が現れた時点。見出し「8 entries, 8 live」、表の行 7 件)のうち、visible 1 が 1 件(lid 8)、visible 0 が 6 件。窓表は 1 時点だけで、件数の推移は測っていない。
2. 基準プロジェクトのプラグイン(`docs/archive/investigations/reference-project/tables/plugins.tsv`、`awk -F'\t' 'NR>1' … | wc -l` = 46 行。internal 列が Fruity Wrapper の行 21、その他 25)。
3. 新版の挙動(`/tmp/opencode/r35/run-limit.py`。テストの LIMIT_MAIN を同じフラグで clang し、各場面を 1 回ずつ実行。終了コードは SIGABRT の -6、シェルでは 134)。
   新版: compose と scene とも、64 描画 + 非表示 0・10、10 描画 + 非表示 70 は停止せず(64・10 件描画)。65 描画は名前つきで停止(`fle: fle_desk_gather: more than 64 windows are drawn`)。非表示 4095 は停止せず(0 件描画)。非表示 4096 は名前つきで停止(`… reach the limit of the window list (4096)`)。
   旧版(3ded666 の `fle_desk.c`): 64 描画 + 非表示 10 と 10 描画 + 非表示 70 は、どちらも停止(`more than 64 top-level windows are live`)。非表示 4095 も停止(同じ文面)。
4. cargo: 219 行、通過 2428、失敗 0、無視 1(`evidence/R3-5-cargo.txt`。基準は 2426・1・1)。
5. 静的配列の大きさ(式): `DESK_LIST_MAX` × `sizeof(uint64_t)` = 4096 × 8 = 32768 B。3ded666 までのスタック配列は 65 × 8 = 520 B。

## 読み(解釈。実行していない)
1. 一覧の上限が切りを検出できる根拠: `fle_win_input_toplevels` は `siblings(-1, idx, SIB_MAX)` の返りを数える(`fle_win.c:464`、`:7118-7121`)。SIB_MAX = 4096。z 順の一覧 `zl` は破棄された窓を `zl_remove` で外す(`fle_win.c:4621`・`:4645`)ので、返りは生きている最上位窓の数で、4096 に満たなければ切られていない。msgonly の窓は一覧に入らない: `in_group`(`lift/runtime/fle_win.c:391`)が、親が -1 の群では msgonly を除き、`siblings`(`:455-462`)は `in_group` で選ぶので、`siblings(-1, …)` の返りに msgonly は含まれない(ソースの読み。実行していない)。
2. 停止の位置: 65 件目は `cur[]` の外へ書かない。`dwin` を `nw` に作り、`cur[ncur++] = nw` の直前に停止する。
3. コスト(未測定): `gather()` は生きている最上位窓の全件に `fle_win_input_view` を呼ぶ。3ded666 までは 65 件まで読んでいた。通常の件数(数件)では差は無い。件数が多いときは呼び出しが増える。
4. 文面: `fle_stop` は `fle_env.c:10-13` で stderr に `fle: <api>: <why>` を出し、abort する。依頼者の画面にどう出るかは未確認。

## 未確認
- 実エンジン(wasm bundle)での、描かれる窓 65 件の挙動。bundle は作っていない。
- 実機の描かれる窓と非表示の窓の件数の最大(0385 の未確認 2 を継ぐ)。
- wasm32 のビルド(標準ヘッダが無く、構文検査できなかった。ネイティブの `clang -fsyntax-only -Werror` は通過)。

## 出典(旧決定番号・ファイル)
- 0385(置き換え元。読み 4 と決定 1 行目)、0223・0240・0244(合成器と scene)、`docs/core/pitfalls.md` の F-7(行 15: 切るなら名前つき停止)。
- `lift/runtime/fle_win.c:464`(SIB_MAX)・`:4621`・`:4645`(zl_remove)・`:7112-7122`(fle_win_input_toplevels)、`lift/runtime/fle_env.c:10-13`(fle_stop)。
- `evidence/R5-1/engine-log-run2.txt:288-295`(G2 の窓表)、`evidence/R3-5.md`、`evidence/R3-5-review.md`(reviewer の検査)、`evidence/R3-5-cargo.txt`。
