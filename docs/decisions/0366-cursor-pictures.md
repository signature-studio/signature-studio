# 0366: カーソルの絵・hot spot を CSS カーソルへ(差分 0325-cursor-pictures の分類)
分類: 混在。(b) 契約の実装(CreateIconIndirect・LoadImageW/LoadCursorW のカーソルの絵と hot spot、それを既存の osstate 経路で運ぶ部分)。(a) の除去は差分に含まれない。(c) には当たらない(新しい経路を増やす)。テスト・道具(差分にある harness の 3 ファイルと app のテスト 3 ファイル)は製品に入らない。
撤回条件: 適用後に、実エンジンのホバー先ごとの CSS `cursor` の値を取って値が変わらない、または `cargo test`・`npm test` に新しい失敗が出る、または実エンジンの起動確認で `engine-idle` に達しないなら外す(R2-9 の完了条件のとおり)。旧記録の数字は書かない。
日付: 2026-11-15   ROADMAP: R2-9   一覧: L09・O18・0325   コミット: deed88b..(本記録の commit)

## 決定
決定: 今回は適用しない。根拠: 分類の材料(差分の全文の読み・`evidence/R2-9b.md`)は揃ったが、適用後の動作(ビルド・テスト・実エンジンの値)は未確認で、適用するかは reviewer の受理の後の別の依頼で決める(main の決定)。差分は変えない。

## 分類(手順 3 の結果の要約。行番号は差分の行番号)
- **(b) 契約の実装**(製品コードの `+`。Win32 のカーソル API の意味を実装する、または既存の経路でページへ運ぶ):
  - `lift/runtime/fle_gdi.c` の `make_icon_img`・`fle_gdi_icon_image`・`CreateIconIndirect` の 1 行(553)・`destroy_icon_or_cursor` の解放(634〜638)。Learn の CreateIconIndirect・ICONINFO の hot spot の規則(行 534〜539 の注釈)。
  - `lift/runtime/fle_res.c` の `fle_cursor_shared_image`(670〜797)。LoadImageW/LoadCursorW のモジュール資源からの絵と hot spot。
  - `lift/runtime/fle_appview.c` の `fle_av_cursor_image`(472〜496)と `cursor_serial`(454〜461)。
  - `lift/runtime/fle_appview.h`・`fle_gdi.h`・`fle_res.h` の宣言(行 505、513〜520、649〜650、805〜808)。
  - `app/src/engine/` の読み取りと運搬(`appview.ts` 834〜851(`readCursorImage`)、`fruity.ts` 884〜894、`os-state.ts`、`worker.ts` 957〜969、`client.ts` 859〜868、`protocol.ts` 932〜933)。
  - `app/src/ui/cursor.ts` の CSS 化(1038〜1082 の `scale2x`・`CursorPictureStore`、1084〜1096 の `cursorCss`)。
  - `app/src/main.ts` の配線(990〜1013)。
- **除去 (a)**: 0 件。差分の `-` 行 17 件は、構造体の `reserved[3]` → `reserved[2]`、`osstate` の型と `osStateKey` への serial の追加、`cursorCss` の引数の差し替え、既存テストの期待値の更新で、既存の処理を除く変更ではない。
- **(c) 通信・変換の削減**: 該当なし。下の表のとおり通信は増える。
- **書けない(場当たり)の候補**: 差分の `if`・三項の読みでは、FL のウィンドウ名・クラス名・固定座標・固定の大きさで分岐する箇所は見つからなかった(「未確認」の欄の読み)。ただし `fle_res.c` の `q`(1〜6 は序数 32762〜32767、`FLE_LI_NAMED_FIRST`〜)は `fle_loadimage.h` の既存の契約の値に依存する。FL 固有か否かは読みでは断定できない。

## 通信の一覧(手順 4。新しいメッセージの種類と経路)
- 新しいメッセージ型: **1 件**。`cursor-image`(`protocol.ts` 933 行、`worker.ts` 968 行で `post`、`client.ts` 868 行で受ける)。
- 既存の型への追加: `osstate` に `cursorSerial` の 1 項目(`protocol.ts` 929 行、`worker.ts` 944 行は `...os` で送るため形は同じ)。
- 新しい経路(Worker → ページ)は 1 本ではなく、既存の `osstate` の経路の前に `cursor-image` を 1 回ずつ送る。送信は (handle, serial) ごとに 1 回(`worker.ts` 958〜969)。
- 新しい wasm の export: 1 件(`fle_av_cursor_image`、`fle_appview.c` 472 行)。`fle_av_poll` の構造体に `cursor_serial` の項目が増える(`fle_appview.h` 505 行)。

## 旗・環境変数・条件分岐(手順 5)
- `postMessage`・`new Worker`・`fetch(`・`getenv`・`URLSearchParams`・`#ifdef` を含む `+` 行: 0 件(実測: `expire_grep` で `^\+.*(getenv|URLSearchParams|#ifdef|postMessage|new Worker|fetch\()` → No matches)。
- FL 固有の値(ウィンドウ名・クラス名・固定座標・固定の大きさ)で分岐する箇所: 0 件(読み。上の「書けない」の候補を除く)。
- 寸法の上限: `128`(`fle_appview.h` 513 行の注釈)はブラウザが表示する上限として注釈に書かれている。`256`(`fle_gdi.c` 538 行の注釈)は「作れない寸法」の線引きで、差分の注釈が置いた値であり、Learn の数字ではない(読み)。どちらも FL 固有の値ではない。

## 撤回条件の確認に要る未確認
- 適用後の `cursor` の値の変化(実エンジン、ホバー先ごと)。
- `cargo test`・`npm test` の結果。
- `lift/harness/c0325/build.sh` を含むビルドの実行。
- `client.ts` の 2 hunk と E14(R2-8)の衝突の有無(E14 の差分を見ていない)。
- `fle_gdi.c` hunk #4 が `patch -p1` で 222 行の offset で当たった理由(`evidence/R2-9a.md` 4-1)。

## 未確認
- 実エンジン・ビルド・テストは実行していない。`patch`・`git apply` も実行していない。
- 差分の `+` 行の件数は、`expire_grep` の正規表現による数であり、通信・処理の量ではない。

## 出典
- `patches/ci1/0325/diffs/0325-cursor-pictures.diff`(1270 行、33 hunk、20 ファイル)。
- `evidence/R2-9a.md`(dry-run、commit `2f9a0e6`)。
- `docs/ROADMAP.md` の R2-9 の行(完了条件)。
- `docs/decisions/0355-desk-describe-lazy.md`(形式の参照)。
- `docs/decisions/README.md`(分類の規則)。

## 補い(2026-11-15、reviewer の差し戻し)

- 分類の行(2 行目)は、差分の `lift/runtime/fle_res.c` の hunk(`@@ -640,3 +640,140 @@`、差分 657 行)を除いて読む。この hunk の 694 行(`if (q >= 1u && q <= 6u) kn.id = FLE_LI_MODULE_FIRST + q - 1u;`)は「書けない(場当たり)」。`q` の 1〜6 の分岐が、FL の DLL の資源 ID(32762〜32767。`lift/runtime/fle_loadimage.h` 30〜31 行)に依る。根拠: `evidence/R2-9b.md` の補い(1)。
- 決定: 適用の判断は、この補いと reviewer の再検査の後の別の依頼で行う。根拠: 分類に「書けない」の部分が残るため。
