# 0354: L05・L06 のコールバックの入れ子の上限をスタック残量に置き換える(0319)
分類: (a)。`FLE_TLS_MORE` の分岐: 除去して weak 変数の参照だけ残す
分類(追加): 深さ 16 の上限の除去は (a)。スタック残量の検査(guest スタックの StackLimit との比較、wasm の host スタックの残量)は (b 契約の実装)。Windows の入れ子の上限はスタックの大きさだけ、という契約を実装する。
撤回条件: R1-6 の O5 の再現で `callback nesting` の停止行が減らないとき、または R2-7 の P1 が手元の基準 +100 ms を超えたとき、この 2 本(0319 本体と tests)を外す。
日付: 2026-11-14   ROADMAP: R1-4   一覧: -   コミット: (commit 1 = 本体、commit 2 = tests。`git log --oneline -2`)

## 決定
決定: 次の 2 本を `lift/` に取り込む。`patches/alpha22/impl/0319-callback-stack-limit.diff`(L05。`FLE_TLS_MORE` の分岐だけ除いた形)と `patches/alpha22/impl/0319-callback-stack-limit-tests.diff`(L06)。根拠: `FLE_CB_MAX_DEPTH 16` は Windows にない上限で、ROADMAP の O5 で止まる入れ子を止めていた。上限をスタックの残量に置き換える。

## 元のパッチとの違い
元の `0319-callback-stack-limit.diff` の `fle_callback.c` には、次の 4 行の `#ifdef` が入る。取り込みでは手で削った。
```
#ifdef FLE_TLS_MORE
        cur = fle_teb_cur;
#else
        if (&fle_teb_cur != NULL) cur = fle_teb_cur;   /* a weak VARIABLE: ... */
#endif
```
取り込み後は `#else` の側の 1 行だけが残る。
```
        if (&fle_teb_cur != NULL) cur = fle_teb_cur;   /* a weak VARIABLE: ... */
```
この書き換えは「パッチからの意図した差」である。commit 1 のメッセージにも書く。

## 根拠
- `FLE_TLS_MORE` は `lift/` のどこにも定義が無い(`grep -rn FLE_TLS_MORE lift` → 0 件。`evidence/R1-4.md`)。`#ifdef` の側は到達しないので、除いても動きは変わらない。
- `fle_teb_cur` は `lift/runtime/fle_vex.h` 109 行で `uint8_t *fle_teb_cur __attribute__((weak)) = 0;` と定義される weak 変数。`&fle_teb_cur != NULL` を確かめてから読むのは、テストが一部のファイルだけをリンクしたとき未定義参照を避ける形で、`fle_vex.h` の定義と合う。
- 新しい `#ifdef` は作らず、残していない。

## 途中の commit の `cargo test` の事実(`evidence/R1-4.md`)
- commit 1(本体だけ): 通過 2414・失敗 3・無視 1。失敗は `getmodulehandleexw_returns_in_every_build`(取り込み前から残る 1 件)と、`callback` テストの 2 件(`the_constants_are_the_decided_ones`: 期待 16 に対し 256。`nesting_of_exactly_the_limit_is_allowed_and_one_more_stops`)。`callback` の 2 件は、テストが旧定数 16 を前提にしているため落ちる。`evidence/R1-4a.log`。
- commit 2(本体+tests): 通過 2417・失敗 1・無視 1。失敗は `getmodulehandleexw_returns_in_every_build` の 1 件だけで、HEAD(`evidence/R1-head.log`)と同じ。`evidence/R1-4b.log`。
- commit 1 で緑でないのは、tests を同じ commit に入れないためである。両方を 1 つの commit にはしなかった(決定 `R1-4` の commit 数どおり)。

## 元の文書の番号について
ROADMAP の R1-4 は、決定文書を `docs/decisions/0346-*.md` とする予定だった。しかし `docs/decisions/` は 0351 から始まり、0346 のファイルは無い。0353 の次の番号 0354 を使った。ROADMAP の行は変えていない(状態の更新は reviewer の検査の後に main が頼む)。

## 出典
- `patches/alpha22/impl/0319-callback-stack-limit.diff`、`patches/alpha22/impl/0319-callback-stack-limit-tests.diff`
- `evidence/R1-4.md`(各コマンドと出力)、`evidence/R1-4a.log`、`evidence/R1-4b.log`、`evidence/R1-head.log`
- `evidence/R1-dryrun.md`(適用可否。本体 3 ファイル・tests 1 ファイル)
- `lift/runtime/fle_vex.h` 109 行(`fle_teb_cur` の weak 定義)
- `docs/decisions/0353-l01-l04-and-sweep1-stubs.md`(分類と撤回条件の書き方)
