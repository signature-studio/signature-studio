# 0362: 読む側の無い FLE_SCREEN_REMAXIMIZE の送る行を除去する(L07)

分類: (a)。除去。読む側の無い送る行 `FLE_SCREEN_REMAXIMIZE: '1'`(`app/src/main.ts`)を除く。新しい処理・分岐・環境変数は足さない。
撤回条件: 除去後に P1 の起動・`npm test`・`cargo test` の新しい失敗、または display change(viewport の縮小)の挙動の悪化が出たら戻す(`git revert 6d95158`)。

日付: 2026-11-14   ROADMAP: R2-6   コミット: 除去 `6d95158`(`app/src/main.ts`・`app/src/ui/screen-fit.ts`・`lift/runtime/fle_win.c` の 3 ファイル、5 行の追加・5 行の削除)。記録は `evidence/R2-6.md` と `evidence/R2-6-cargo.log`(commit `e4dc679`)。

## 決定
決定: `FLE_SCREEN_REMAXIMIZE: '1'` の送る行を除去する(`app/src/main.ts:343` は `const env = screenEnv(g.width, g.height);`)。`patches/ci1/0322/diffs/d1-dead-remaximize-env.diff` を取り込み、そのコメント 3 箇所(`main.ts:322`、`ui/screen-fit.ts:4`、`lift/runtime/fle_win.c:7125`)を事実に合わせる。

## 根拠
1. **読む側が 0**: `lift/runtime` で `FLE_SCREEN_REMAXIMIZE` を読む `getenv`・`process.env`・`ENV[` は 0 件。`app/` にも名前は残らない(`expire_grep` の結果は `evidence/R2-6.md` の手順 2)。`fle_screen.h:47,53` の `getenv` は `FLE_MONITOR_W/H` だけ、`fle_thread.c:824,847` は `FLE_DWM_HZ`・`FLE_DISPLAYSETTINGS_FREQUENCY` だけを読む。
2. **挙動 0**: `fle_input.c:749` は `apply_display` の中で `fle_win_input_remaximize(...)` を無条件に呼ぶ(環境による分岐は無い)。`fle_win.c:7128-7141` の関数は窓の表を見て、`zoomed` の窓だけを `apply_pos` で動かす。環境変数は読まない。除去で変わるのは manifest の env の 1 キーだけ。
3. **guest に見えない**: 送る行は `manifest.engine.env` に入るだけで、guest の環境ブロックには入らない。guest の環境ブロック(`lift/runtime/fle_env.c`)へ書くのは guest 自身の SetEnvironmentVariable の実装(`fle_env.c:157-189`)だけで、ホスト環境は読まない(同ファイル冒頭のコメント)。
4. **経緯**: 決定 0243 は `FLE_SCREEN_REMAXIMIZE` を残した物に挙げる(`docs/archive/decisions/0243-no-options.md:53`(getenv で残した物・ページが渡す)、`:109`(パラメータとして残した物))。現 HEAD の `lift/runtime` に、この名前を読む `getenv`・`process.env`・`ENV[` は無い(実測)。読む側がいつ・どの commit で無くなったかは未確認(`docs/archive/decisions/0322-lift-fidelity-audit.md:123` が引く `6eb1737d` は、このリポジトリに無い: `git cat-file -t 6eb1737d` が失敗)。6d95158 の時点で、コメント 3 箇所は「opt-in」と書いていた(`0322:123` の表が、d1 の対象として挙げる)。

## 検査の記録(除去の前後)
- `npm run build`: 前 rc 0、後 rc 0。
- `npm test`: 前 65 files passed・1 skipped、1290 passed・7 skipped。後 同じ。新しい失敗 0。
- `cargo test --workspace --offline --no-fail-fast`: 基準 `evidence/R2-3-cargo.log` は通過 2419・失敗 1・無視 1。後 `evidence/R2-6-cargo.log` も通過 2419・失敗 1・無視 1。失敗名の集合は `getmodulehandleexw_returns_in_every_build` の 1 件だけで両方一致。`undefined reference` は 0 件。
- 字義: ROADMAP の「`npm test` 失敗 0」は満たす。「`cargo test` 失敗 0」は字義どおりには未達(既知の 1 件)。0353 の読み(既知の 1 件を除く新しい失敗 0)で通過。

## 未実施(この決定の範囲外)
- 実エンジンの確認: wasm のビルド・実行は未実施。push 後に GitHub の engine-bundle のビルドが走る。
- display change の前後比較(O5 の再現手順)は未実施。P1〜P3 の測定も未実施。撤回条件の『display change の悪化』は、この記録では未確認。
- 6d95158 の時点では、`lift/runtime/fle_win.c:7124` のコメントに名前が 1 件残っていた(『decision 0243 removed the environment switch FLE_SCREEN_REMAXIMIZE』という説明文)。この直しの commit で名前を消した。直した後の `grep -rn FLE_SCREEN_REMAXIMIZE app lift/runtime`(`node_modules` を除く)は 0 件(実測)。

## 残る状態
- `fle_win_input_remaximize`(窓の再配置)は、この除去で変わらない。その挙動の忠実性(Windows が最大化窓を置き直すという推測)は未確認のまま(0322 の群 (C) とは別件)。

## 追記 2026-11-14(reviewer 指摘の直し)

- 直す前の 0362:15(経緯): 「決定 0243(依頼者の方針『今後、一切のオプトイン/オプトアウト処理を禁止します』)が、読む側の `getenv` を消した。送る側はページに残った(…)。コメントはまだ「opt-in」と言っていた。」
- 直した後: 上の 4 の文(0243 は `FLE_SCREEN_REMAXIMIZE` を残した物に挙げる。`:53`・`:109`。現 HEAD の `lift/runtime` に読む `getenv` は無い(実測)。いつ・どの commit で読む側が無くなったかは未確認。`0322:123` が引く `6eb1737d` はこのリポジトリに無い)。
- 直す前の `fle_win.c:7124` のコメント(直す直前の HEAD `d4be398` で実測。reviewer の 7125 は 1 行ずれ): 「Decision 0212 (PROVISIONAL; S-remaximize; decision 0243 removed the environment switch FLE_SCREEN_REMAXIMIZE, so this always runs; an inference …」。
- 直した後: 「Decision 0212 (PROVISIONAL; S-remaximize; the page no longer sends an environment switch for it and the runtime reads none, so this always runs; an inference …」(残りの文は変えない)。名前はコメントから消した。コードの行は変えていない(`git diff -U0` が 1 行のコメントだけ)。
- 記録の commit: 除去 `6d95158`、記録 `e4dc679`(上の 6 行目)。
- 字義の grep: `grep -rn FLE_SCREEN_REMAXIMIZE app lift/runtime`(`node_modules` を除く)は、6d95158 の時点で 1 件、この直しの commit の後は 0 件(実測)。
