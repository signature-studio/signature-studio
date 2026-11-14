# 0347: DPI(FLE_SCREEN_DPI)。案 A は採らない。R2-5 は blocked。案 B は測定の後に決める

結論(main の決定。reviewer の判定を採る):
- **案 A(`FLE_SCREEN_DPI` を `FLE_MONITOR_W/H` と同じ経路で残す)は採らない。** 根拠: `AGENTS.md:66` が限定なしで「新しい URL パラメータ・環境変数・`#ifdef`・ビルド変数・workflow_dispatch の入力を作らない」と禁じる。名前 `FLE_SCREEN_DPI` は依頼者の no-options 方針(`docs/archive/decisions/0243-no-options.md:7`)で一度畳まれ、無視されることがテストで固定されている(`lift/crates/fe-lift/tests/screen1.rs:143`)。復活は依頼者の決定を覆し、AGENTS §6 の文面にも反する。
- **案 C を当面採る: R2-5(`0318-hidpi-*` の取り込み)は見送り、状態は `blocked`。** L13(DPI 96 固定)と O7(画面のぼやけ。原因が DPI かは未確認)は未解消のまま。
- **案 B は未決。** 案 B = `IN.DISPLAY` の `hwev` に DPI を載せる、または real Windows の契約に沿って `WM_DPICHANGED` を送る。成否の鍵は「FL が DPI を問い合わせる時刻」で、これは未確認。
- 撤回条件: (1) FL が DPI を問い合わせる時刻が分かり、起動時の DPI を案 B(`IN.DISPLAY` か `WM_DPICHANGED`)で届けられると確かめられたら、案 B を別の決定文書で採る(分類 (b) の 1 行を付ける)。(2) 依頼者が `FLE_SCREEN_DPI` の復活を明示的に許可したら、案 A を改めて検討する(その場合 AGENTS §6 の扱いも決める)。

日付: 2026-11-14(書き直し)。ROADMAP: R2-5(`blocked`)、R2-6(依存 R2-3 に変更)、R7-6(追加)。
最初の版: `50d77cb`(案 A を採る案。取り込みは未実施)。この書き直しの commit は `git log --oneline -- docs/decisions/0347-screen-dpi.md` で見る。

## 決定

1. **案 A は採らない。** 根拠(実測):
   - (a) `AGENTS.md:66`(§6)は「新しい URL パラメータ・環境変数・`#ifdef`・ビルド変数・workflow_dispatch の入力を作らない。比較はコミット同士。」と書き、環境変数に限定を付けていない。案 A は新しい名前 `FLE_SCREEN_DPI` を環境変数として足すので、この文に当たる。
   - (b) `docs/archive/decisions/0243-no-options.md:7` は依頼者の方針「今後、一切のオプトイン/オプトアウト処理を禁止します」。0243 の「getenv で残した物」(`:53`)と「パラメータとして残した物」(`:109`)に `FLE_SCREEN_DPI` は無い。残したのは寸法の `FLE_MONITOR_W/H` と `FLE_SCREEN_REMAXIMIZE` 等。
   - (c) `lift/crates/fe-lift/tests/screen1.rs:139-143` は「DPI は 96 で、`FLE_SCREEN_DPI=192` は無視される」をテストで固定している(`:143` の `FLE_SCREEN_DPI` が 192 のとき `96 96`)。
   - (d) 単一の値(寸法)は「切り替えるスイッチ」ではない(`docs/ops/fidelity-and-performance.md:77` の定義)。`FLE_MONITOR_W/H` を残したことはこの読みに沿う。ただしこれは既存の名前の話で、新しい名前には及ばない。
2. **案 C を当面採る。** R2-5 の `blocked` の理由は決定 1。L13 と O7 は未解消。R2-5 の行に理由を書き足した(`docs/ROADMAP.md`)。
3. **案 B は未決(採否は測定の後)。** 現在のページは `IN.DISPLAY` を最初の idle の後に送る(`app/src/main.ts:332` の `inputEnabled()` が偽なら戻り、`:333` で送る)。次に測るなら、製品に診断を足さずに、既存の観測(engine log・import thunk の記録・native の harness)から FL の `GetDpiForMonitor`・`GetProcessDpiAwareness` の呼び出し時刻を読めるかを別の依頼で調べる。読めるかは未確認。
4. **L07(R2-6)は R2-5 を待たない。** d1(`patches/ci1/0322/diffs/d1-dead-remaximize-env.diff`)は HEAD に対して rc 0 で当たる(`evidence/R2-5a.md:140`)。page を当てない限り、page との衝突は起きない。R2-6 の依存は `R2-3` に付け替えた。
5. **(a) の新しい候補 R7-6。** `lift/runtime/fle_proc.c:408` の opt-in の三項演算子(`fle_screen_dpi() > FLE_SCREEN_DPI_DEFAULT ? FLE_PROCESS_PER_MONITOR_DPI_AWARE : FLE_PROCESS_DPI_UNAWARE`、コメント「decision 0212 (opt-in FLE_SCREEN_DPI)」)。`fle_screen_dpi()` の定義は `lift/runtime/fle_screen.h:59` の 1 つで、常に `FLE_SCREEN_DPI_DEFAULT`(`:58`、96u)を返す。`96 > 96` は偽なので、真の枝(`FLE_PROCESS_PER_MONITOR_DPI_AWARE`、`lift/runtime/fle_proc.h:97`)には HEAD で届かない。`lift/` で `FLE_PROCESS_PER_MONITOR_DPI_AWARE` を参照するのは `fle_proc.h:97` と `fle_proc.c:408` の 2 箇所だけ(grep 実測)。除去は R7-6 の別の依頼で行う。
6. **案 A の材料の節は消さずに残す**(下の「案 A の材料(採らない)」)。見出しに「採らない」と付けた。

## reviewer の判定との照合

- 根拠 1・2 は支持する(決定 1)。根拠 3 は既存の名前の範囲で支持する(決定 1 の (d))。
- reviewer の引用との食い違い(実測):
  - reviewer は「オプション」の定義を `AGENTS.md:77` とした。実測では `AGENTS.md:77` は §8 の認証情報の行で、定義ではない。定義は `docs/ops/fidelity-and-performance.md:77`(§6)。
  - 「新しい環境変数を作らない」は最初の版の要約で、原文ではない。原文は `AGENTS.md:66`(上の決定 1 の (a))。
  - HEAD の `lift/runtime/fle_proc.c:408` の三項演算子は、最初の版の表に無かった。決定 5 で扱う。

## 誤りの訂正(最初の版 `50d77cb`)

- 最初の版の「HEAD の `lift/` と `app/` に無い」(`:9`・`:33` 相当)は**誤り**。`lift/` には名前が出る(実測):
  - `lift/runtime/fle_proc.h:97`(`FLE_PROCESS_PER_MONITOR_DPI_AWARE` のコメント)
  - `lift/runtime/fle_proc.c:408`(opt-in の三項演算子のコメント)
  - `lift/runtime/fle_appfile.c:239`(コメント)
  - `lift/crates/fe-lift/tests/screen1.rs:1,109,140,143`(テスト)
- 正しいのは「読む側の `getenv` が 0 件」の部分だけ。`app/` には `FLE_SCREEN_DPI` が 0 件(実測)。
- 最初の版の「取り込み後も `getenv` の呼び出し箇所は増えない」は、名前の経路の前提に依る。決定 1 で案 A は採らないので、この前提は今は使わない。

## 案 A の材料(採らない)

以下は案 A を採った場合の材料として残す。**採らない。**

実測(最初の版の読み。HEAD d2530d7。実行はしていない。本依頼で再確認したのは `fle_screen.h:30`・`main.ts:339-349`・`screen-fit.ts:63-64`・`fle_proc.c:408` と `screen1.rs:139-143`):

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
| `lift/runtime` の `getenv` | 7 行(`fle_test_env.h` 2・3・5・12、`fle_thread.c:824,847`、`fle_screen.h:30`) | 最初の版の記録。本依頼では再実測していない |

`+` 行の分類(`evidence/R2-5a.md` §1 の記録。本依頼で再 grep はしていない):

| 語 | 分類 | 根拠 |
|---|---|---|
| `FLE_SCREEN_DPI`(`fle_screen_dim` の引数) | 新しい名前(案 A の場合)。(b) とした | 読む側の `getenv` は `fle_screen.h:30` の 1 箇所で、名前の文字列はこの経路に無い。名前は `lift/` にコメントとテストで出る(上の訂正) |
| `FLE_SCREEN_DPI_MAX`・`FLE_SCREEN_DPI_DEFAULT` | 定数 | MAX は出典なし(決定 6 の撤回条件) |
| `__attribute__((weak)) uint32_t fle_screen_dpi_v;`(patch `:149`) | 既存の続き | `fle_screen.h:43` の `fle_screen_px` と同じ形 |
| `FLE_SCREEN_REMAXIMIZE`(page hunk 8 の残り) | 既存の続き。L07 の対象 | `main.ts:343` と同じ行 |

- 最初の版の決定 6(案 A の材料): `FLE_SCREEN_DPI_MAX 384` と範囲 96〜384 は出典なし(`0318-hidpi-runtime-v2.diff:148` が `S-gpu8-dpi-max` を「a choice without a source」と書く。Windows の拡大率の上限との関係は未確認)。
- 最初の版の順序(案 A の材料): page を先に当て、R2-6 で d1 を HEAD に対して作り直す。`evidence/R2-5a.md` §5 の記録では、page を先にすると d1 の 4 hunk のうち 2 hunk が失敗し、d1 を先にすると page の 14 hunk のうち 2 hunk が失敗する。「衝突する page の 4 hunk」という ROADMAP の文言は実測と合わない(数は当てる順で変わる)。

### 取り込まない差分(案 A の材料)
- `0318-hidpi-runtime-v2.diff` の harness の 4 ファイル 6 hunk: `lift/harness/parity-paint/parity_traceall.c`(2)・`traceall-wrap-flags.txt`(1)・`lift/harness/vis1/vis1_apis.inc`(2)・`link-flags.txt`(1)。HEAD に無い。新しい処理ではない。適用は当たる 10 hunk だけにする(元の `patches/` は変えない。一時の diff を作る)。
- `0318-hidpi-runtime.diff`・`0318-hidpi-rust-tests.diff`(v2 でない物)は v2 に置き換えられた旧版と読めるが、未確認(内容を読んでいない)。

### `getenv` の行数(案 A の材料)
- 取り込み前 HEAD は 7 行(最初の版の記録)。取り込み後の見込みは 7 行(`evidence/R2-5a.md` §7 の一時コピーでの記録)。取り込む場合は再実測する。

### 分類 (a) の候補(決定 5 の根拠)
- `lift/runtime/fle_proc.c:408` の opt-in の三項演算子(0212 の opt-in の残り)。`fle_screen_dpi()` の定義は 1 つ(`fle_screen.h:59`)で、常に 96 を返す。真の枝は HEAD で届かない。除去の分類と撤回条件は R7-6 の決定文書に書く。

## 未実施・未確認
- 未実施: 案 A も案 B も取り込んでいない。`cargo test`、`npm test`、wasm のビルド、実エンジンでの確認、P1〜P3 の測定は行っていない。R7-6 の除去も行っていない。
- 未確認: FL が DPI を問い合わせる時刻(案 B の成否の鍵)。`WM_DPICHANGED` の契約(real Windows の読みが要る)。O7 の原因が DPI か。`emcc` が `FLE_SCREEN_DPI` を含む wasm を作るか。
- 未確認: `evidence/R2-5a.md` §5 の dry-run の数字(rc 1・`1 out of 9` など)は記録の値で、本依頼では再実測していない。`evidence/R2-5a.md` §1 の残りの行番号も未確認。
- 未確認: `docs/decisions/README.md` の規則 11(「新しい `#ifdef`・`getenv`・URL パラメータを足さない」)の文面は、最初の版の引用のまま。本依頼では読み直していない。
- 未確認: 実行中の DPR の変化への追従。HEAD と同じく起動時の値のまま(`docs/archive/decisions/0212-screen-fit.md:199` の記録)。
- 未確認: `docs/archive/decisions/0212-screen-fit.md:193` の「FL が `FLE_SCREEN_DPI=169` で UI を拡大する」の実測は、当時の記録。本依頼では再実行していない。

## 出典
- `AGENTS.md:66`(§6)。`AGENTS.md:77` は §8 の認証情報の行(オプションの定義ではない)。
- `docs/ops/fidelity-and-performance.md:77`(オプションの定義)・`:78`(新しいオプションを作らない)・`:82`(オプションではない物: パラメータ)。
- `docs/archive/decisions/0243-no-options.md:7`・`:53`・`:109`。
- `lift/crates/fe-lift/tests/screen1.rs:139-143`。
- `lift/runtime/fle_proc.c:408`・`lift/runtime/fle_proc.h:97`・`lift/runtime/fle_appfile.c:239`・`lift/runtime/fle_screen.h:30`・`:58-59`。
- `app/src/main.ts:332-333`・`:339-349`。`app/src/ui/screen-fit.ts:63-64`。
- `evidence/R2-5a.md` §1・§5(`:140-144`)・§7。
- `docs/archive/decisions/0212-screen-fit.md:72`・`:76`・`:193`・`:199`。
- `docs/decisions/0350-getenv-classification.md`(表 3 の行・注 3)、`0361-virtual-pointer-contextmenu.md`(書式)、`docs/decisions/README.md`(規則 11)。
- 番号: README は「最大番号 + 1」とするが、ROADMAP が R2-5 の分類を 0347 と定めており、0347 は空いている。0346〜0349 は欠番。0347 を使う。
