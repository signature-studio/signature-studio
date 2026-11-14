# 0347: DPI(FLE_SCREEN_DPI)の分類。分類 (b)・案 A。新しい名前であることと決定 2 の読みは reviewer の検査を受ける
分類: (b) 契約の実装。ページが測った DPI を、既存の 1 本の経路でゲストへ渡す部品間パラメータとして `FLE_SCREEN_DPI` を残す(案 A)。
撤回条件: 取り込み後に P1 の起動・`cargo test`・`npm test` の新しい失敗、または P3 の音の書き込みの悪化が出たら、`patches/gpu8/diffs/0318-hidpi-*` の適用 commit を戻す。reviewer が決定 2 の読みを受理しなければ案 B か C に戻す。`FLE_SCREEN_DPI` の範囲 96〜384 の出典を確かめて違うと分かったら範囲を直す(決定 6)。

日付: 2026-11-14   ROADMAP: R2-5   一覧: -   コミット: 未取り込み(R2-5 の取り込み commit は別の依頼)。この文書と ROADMAP の R2-5 の文言の commit は、`git log --oneline -- docs/decisions/0347-screen-dpi.md` で見る

## 決定
1. **分類 (b)**。`FLE_SCREEN_DPI` は、ページが測った `devicePixelRatio` からパッチの `dpiOfRatio`(HEAD には無い。`0318-hidpi-page.diff` が足す)で作る値を、既存の 1 本の経路(`screenEnv` → `Object.assign(manifest.engine.env, …)` → runtime の `fle_screen_dim`)で渡す。根拠: 値の出所はページの測定。届く経路は 1 本。`getenv` の呼び出し箇所は増えない(`fle_screen.h:30` の 1 箇所)。動作を切り替える旗も、比較用の別経路も無い。`docs/ops/fidelity-and-performance.md:82` の「オプションではない物: パラメータ」に当たる。`FLE_MONITOR_W/H` と同じ分類(`0350` §1 の表の 3 番の行・§2 注 3)。
2. **AGENTS §6 との関係(main の読み。reviewer の検査を受ける)**。名前 `FLE_SCREEN_DPI` は HEAD の `lift/` と `app/` に無い新しい名前である。AGENTS §6 の「新しい環境変数を作らない」と、`docs/decisions/README.md` の規則 11「新しい `#ifdef`・`getenv`・URL パラメータを足さない」の文面に触れる。main の読み: この 2 つが禁じるのは、動作を切り替えるオプション(旗)と別経路の追加。ページが測って既存の経路で渡すデータ(画面の寸法と同じ種類)は、既存の経路の続きとして (b) に分類する。**受理されなければ案 B か C に戻す。**
3. **案 B・C を採らない**。案 B(`IN.DISPLAY` の `hwev` に DPI を載せる)は、起動時の値を env 以外で届けられず、起動順序を変える(`evidence/R2-5a.md` §4)。案 C(見送り)は L13 と O7 の原因候補を残し、R2-6 も止まる。
4. **harness の 4 ファイル(6 hunk)は取り込まない**(下の節)。
5. **順序: page を先に当て、R2-6 で d1 を HEAD に対して作り直す**。根拠: `evidence/R2-5a.md` §5 の記録では、page を先にすると d1 の 4 hunk のうち 2 hunk が失敗し、d1 を先にすると page の 14 hunk のうち 2 hunk が失敗する。page の hunk 8 は `FLE_SCREEN_REMAXIMIZE` の行(`main.ts:343`)を残し、`dpi` だけを足す。L07 の対象の行は変えない。ROADMAP の「衝突する page の 4 hunk」は実測と合わない(数は当てる順で変わる)。
6. **`FLE_SCREEN_DPI_MAX 384` と範囲 96〜384 は出典なし**(撤回条件に含める)。`0318-hidpi-runtime-v2.diff:148` が `S-gpu8-dpi-max`(「a choice without a source」)と書く。Windows の拡大率の上限との関係は未確認。撤回条件に含める。

## 実測(本依頼で読んだ物。HEAD d2530d7。実行はしていない)
| 対象 | 行 | 結果 |
|---|---|---|
| `lift/runtime/fle_screen.h` | `:28-30` | `fle_screen_dim` の `getenv(name)` は `:30` の 1 箇所 |
| 同 | `:47`・`:53` | `FLE_MONITOR_W/H` の呼び出し |
| 同 | `:58-59` | `FLE_SCREEN_DPI_DEFAULT 96u`、`fle_screen_dpi()` は固定の 96。`evidence/R2-5a.md` §1 の「:141 付近」とは合わない |
| `app/src/main.ts` | `:339-349` | `applyScreenParam`。`:343` で `FLE_SCREEN_REMAXIMIZE: '1'` を入れ、`:344` で `Object.assign` |
| 同 | `:330-335` | `:333` の `sendRaw` は `IN.DISPLAY` の幅と高さだけを送る |
| `app/src/ui/screen-fit.ts` | `:63-64` | `screenEnv` は `FLE_MONITOR_W/H` を返す |
| `lift/runtime/fle_input.c` | `:737-750`・`:844` | `apply_display`。`:749` が `fle_win_input_remaximize`。`:844` が `FLE_IN_DISPLAY` の分岐 |
| `patches/gpu8/diffs/0318-hidpi-runtime-v2.diff` | `:148`・`:149`・`:152` | `FLE_SCREEN_DPI_MAX 384u`、weak 変数 `fle_screen_dpi_v`、`fle_screen_dim("FLE_SCREEN_DPI", …)` |
| 同 | `grep -c '^@@'` | 16 hunk(`evidence/R2-5a.md` §1 と一致) |
| `git log --all -- lift/harness/vis1 lift/harness/parity-paint` | 出力 | 空。一度も無い |
| `lift/runtime` の `getenv` | 7 行(`fle_test_env.h` 2・3・5・12、`fle_thread.c:824,847`、`fle_screen.h:30`) | HEAD の値 |

`+` 行の分類(`evidence/R2-5a.md` §1 の記録。本依頼で再 grep はしていない):
| 語 | 分類 | 根拠 |
|---|---|---|
| `FLE_SCREEN_DPI`(`fle_screen_dim` の引数) | 新しい名前。(b) | HEAD の `lift/`・`app/` に 0 件。`getenv` は `fle_screen.h:30` の 1 箇所 |
| `FLE_SCREEN_DPI_MAX`・`FLE_SCREEN_DPI_DEFAULT` | 定数 | MAX は出典なし(決定 6) |
| `__attribute__((weak)) uint32_t fle_screen_dpi_v;`(patch `:149`) | 既存の続き | `fle_screen.h:43` の `fle_screen_px` と同じ形 |
| `FLE_SCREEN_REMAXIMIZE`(page hunk 8 の残り) | 既存の続き。L07 の対象 | `main.ts:343` と同じ行 |

- `getenv` の行数: 取り込み前 HEAD は 7 行(実測)。取り込み後は**見込み 7 行**(`evidence/R2-5a.md` §7 の一時コピーでの記録)。取り込み後に再実測する。

## 取り込まない差分
- `0318-hidpi-runtime-v2.diff` の harness の 4 ファイル 6 hunk: `lift/harness/parity-paint/parity_traceall.c`(2)・`traceall-wrap-flags.txt`(1)・`lift/harness/vis1/vis1_apis.inc`(2)・`link-flags.txt`(1)。HEAD に無い。新しい処理ではない。適用は当たる 10 hunk だけにする(元の `patches/` は変えない。一時の diff を作る)。
- `0318-hidpi-runtime.diff`・`0318-hidpi-rust-tests.diff`(v2 でない物)は v2 に置き換えられた旧版と読めるが、未確認(本依頼では内容を読んでいない)。

## 未実施・未確認
- 未実施: 取り込み、`cargo test`、`npm test`、wasm のビルド、実エンジンでの確認、P1〜P3 の測定。
- 未確認: O7 の原因が DPI か。`emcc` が `FLE_SCREEN_DPI` を含む wasm を作るか。`evidence/R2-5a.md` §5 の dry-run の数字(rc 1・1 out of 9 など)は記録の値で、本依頼では再実測していない。
- 未確認: `evidence/R2-5a.md` §1 の残りの行番号(`fle_proc.c`・`fle_input.c:114` など)。
- 未確認: 実行中の `dpr` の変化への追従。HEAD と同じく、起動時の値のまま(`evidence/R2-5a.md` §8)。

## 出典
- `evidence/R2-5a.md`(d2530d7)、`docs/decisions/0350-getenv-classification.md`(表 3 の行・注 3)、`0361-virtual-pointer-contextmenu.md`(書式)、`docs/decisions/README.md`(規則 11)。
- `docs/ops/fidelity-and-performance.md:82`。
- 番号: README は「最大番号 + 1」(0362)とするが、ROADMAP が R2-5 の分類を 0347 と定めており、0347 は空いている(`ls docs/decisions`)。0346〜0349 は欠番。0347 を使う。
