# 0385: 最上位窓の上限 64 は、超えたら名前つき停止にする。scene の溢れは全面の再送にする(F-7、R3-5)
分類: (a) 除去。黙って切る上限(我々の処理)を、名前つき停止と全面の再送に置き換える。Windows の契約は実装していない(b ではない)。足した分岐は 2 つ(停止・全面の再送)だけ。
撤回条件: 実機または実エンジンで `fle: fle_desk_gather:` の停止が出たら、その時の最上位窓の件数と原因の窓を記録し、この決定を見直す。上限を広げるときは `app/src/engine/desk-gl.ts` の SCENE_MAX_WINDOWS と SCENE_MAX_UPDATES を同じ commit で変える。
日付: 2026-11-16   ROADMAP: R3-5   一覧: L22(F-7)   コミット: 3ded666(コードとテスト)、この文書と evidence/R3-5.md は後続の commit

## 決定
- 決定: 上限 FLE_DESK_MAX_WINDOWS(64)は外さない。最上位窓が 65 以上あるとき、`gather()`(`lift/runtime/fle_desk.c`)が `fle_stop("fle_desk_gather", …)` で名前つきに停止する。根拠: 読み 2・3 と、実測 2。
- 決定: scene の表(64 窓・64 更新)が溢れる書き込み(65 番目の異なる窓の `fle_desk_wrote`)は、黙って捨てず `need_full = 1` を立てる。次の `fle_desk_scene()` は全窓を全面で送る(`fle_desk_resync` と同じ経路)。根拠: 停止にすると、窓が多いだけで実行が終わる。全面の再送は既存の経路で、画素は届く。
- 決定: 同じ打ち切りの 4 箇所(`fle_appview.c:61` の 32、`fle_appfile.c:110`・`fle_input.c:746`・`fle_win.c:7130` の 64)は変えない。根拠: 依頼の範囲外。報告に列挙する。

## 実測(コマンド・条件・日付・n)
1. 構造体の大きさ(2026-11-16、`fle_desk.c` を読み込んだ probe の型エラーの出力から。x86_64 は `clang -fsyntax-only`、wasm32 は `clang --target=wasm32-unknown-unknown -fsyntax-only` で、`fle_desk.c` の定義の写し)。dwin 88 B(x86_64)・80 B(wasm32)。fle_scene_win 80 B、fle_scene_upd 20 B(両方)。cur と prev の合計 11264 B(x86_64)・10240 B(wasm32)。
2. 上限を 256 にした場合の増分(式。1 の値から計算): cur と prev 2 × 192 × 80 = 30720 B(wasm32)、desc 192 × 300 = 57600 B、scene の win と upd 192 × (80 + 20) = 19200 B。合計 107520 B(約 105 KiB)。ただし、上限は 256 でも残る。scene の表は TypeScript が固定オフセットで読むので(下の読み 2)、表を広げると `desk-gl.ts` を同時に変える必要がある。
3. 窓表の実測: 実エンジンの停止報告の窓表は「8 entries, 8 live」か「9 entries, 9 live」(`evidence/R1-6/e13-run-2.json`、`evidence/R5-1/engine-log-run2.txt` など。`grep -rhoE "window table: [0-9]+ entries"` で 52 行、8 件 4 行・9 件 48 行。同じ報告が JSON の複数の欄に出る)。上限 64 に対して 1 桁台。
4. 旧版(`git show HEAD:lift/runtime/fle_desk.c` と `fle_desk.h` を一時ディレクトリに置き、`FLE_D1_SRC_DIR` で向けた)での新しいテスト: 65 窓は停止せず `limit: 65 windows: no stop`、終了コード 0。scene の溢れは `FAIL … (flags 0, 0 updates)`。どちらも黙って欠落していた。
5. 新版の `cargo test --offline -p fe-lift --test desk1_compose`(FLENGINE_PE 付き)と `--test desk_scene`: 3 件通過(出力は `evidence/R3-5.md`)。

## 読み(解釈。実行していない)
1. 64 は Windows の値ではない。本 runtime が決めた表の大きさ(`fle_desk.h:33`)。Windows 側の上限の出典は未確認。
2. 上限を外せない理由: scene の表は C の構造体の固定配列で、`app/src/engine/desk-gl.ts:10-11`(SCENE_MAX_WINDOWS・SCENE_MAX_UPDATES = 64)と `:29`・`:39` の固定オフセットで読まれる。上限を動的にするには表の形を変え、ブラウザ側を同時に直す必要がある。それには bundle の再ビルドと実機の確認が要るが、本依頼の範囲外(R3-5 は native テストだけ)。
3. 停止を選んだ理由: 65 番目の窓を描かずに画面を出すと、黙って欠落する(F-7)。停止は名前が出る。実測(3)の窓数から、通常の利用で停止になる場面は見込まれない(読み。実機の件数は未確認)。
4. 停止の条件は、非表示の窓も含めた最上位窓の数(`fle_win_input_toplevels` の返り値が 65 以上)。非表示の窓は描かれないが、数える。この差が問題になるのは実機で件数が 64 を超えたとき。
5. 全面の再送の理由: `need_full` は `fle_desk_resync()` が既に使う経路(`fle_desk_scene` の `scene_whole`)。溢れた回は全窓を全面で送るので、捨てた書き込みも届く。
6. 分類の確認: 足した分岐は「黙った切り捨て」の置き換えだけで、Windows の契約は足していない(a)。

## 未確認
- 実エンジン(wasm bundle)での 65 窓の挙動。bundle は作っていない。
- 実機の最上位窓の件数の最大(実測は停止報告の 8・9 件だけ)。
- 停止の文面が依頼者の画面に出る経路(engine の stderr の表示)。
- 範囲外の 4 箇所の挙動の実測。

## 出典(旧決定番号・ファイル)
- 0223(desk1、合成器)、0240(gpu1: S-gpu1-5「scene の上限 64 窓・64 更新は `FLE_DESK_MAX_WINDOWS` と同じ」、`docs/archive/decisions/0240-window-move-and-frame-rate.md:187`)、0244(glass・窓の領域)。
- `docs/core/pitfalls.md` の F-7 の行、`docs/ROADMAP.md` の R3-5 の行。
- 旧コード: `fle_desk.c` の gather(上限 64 で切る)・scene_wrote(溢れで黙って return)、`fle_win.c:7112-7122`(max で切る)。
