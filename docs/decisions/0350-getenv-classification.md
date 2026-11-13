# 0350: getenv 8 件の分類(R7-2 の 1 段目。コードは未変更)
結論: 除去 1・残す 4・テスト専用 3・未確認 0(計 8)。除去の差分は未実施(2 段目で行う)。
分類: 除去 1 件 = (a)。残す 4 件 = (b) 3 件(1〜3)と、(a)(b)(c) のどれにも当たらないパラメータ 1 件(5。注 2)。テスト専用 3 件(4・7・8)は製品に入らない、または製品で到達しない。
撤回条件: 2 段目の除去(6)の後に、P1 の起動・`cargo test` の失敗・P3 の音の書き込みのいずれかが悪化したら、その差分を戻す。
日付: 2026-11-14   ROADMAP: R7-2   一覧: -   コミット: (この文書を含む commit。`git log --oneline -1`)

## 1 分類の表

| 番号 | 場所 | 名前 | 設定する側(grep の要点) | 製品の wasm に入るか(出典) | 結果が何を変えるか | 分類 | (a)(b)(c) |
|---|---|---|---|---|---|---|---|
| 1 | `lift/runtime/fle_thread.c:824`(`fle_display_hz`) | `FLE_DWM_HZ` | `app/src/engine/hz.ts:27-28`(`hzEnv`)→ `app/src/main.ts:647-648`(`applyHz`)。`main.ts:721` で無条件に呼ぶ。値は測定値、測れなければ 60。native の動作確認・harness は設定しない(grep 0 件) | 入る。`build_nmod_wasm.sh` の stage_rt が `$RT/fle_*.c` を全部コンパイル。824 行の前後に `#if` が無い(awk で 800〜860 行の前処理行 0) | DwmFlush の周期(`fle_dwm.c:83` が呼ぶ)。24〜1000 の整数なら採用。それ以外・未設定は既定 60(`fle_thread.h:76`) | 残す(部品間パラメータ) | (b) 契約の実装: DwmFlush の周期を、ページが測った表示の値で返す |
| 2 | `lift/runtime/fle_thread.c:847`(`fle_display_settings_hz`) | `FLE_DISPLAYSETTINGS_FREQUENCY` | 1 と同じ(`hz.ts:28`、`main.ts:648`) | 入る(同じ理由。847 行の前に `#if` 無し) | `EnumDisplaySettingsW` の `dmDisplayFrequency`(`fle_monitor.c:200` が呼ぶ)。DwmFlush と違えば stderr に 1 行 | 残す(部品間パラメータ) | (b) 契約の実装: 列挙の周波数を、ページが測った値で返す |
| 3 | `lift/runtime/fle_screen.h:30`(`fle_screen_dim`) | `FLE_MONITOR_W` / `FLE_MONITOR_H`(呼び出しは `fle_screen.h:47` と `:53` の 2 件のみ) | `app/src/ui/screen-fit.ts:63-64`(`screenEnv`)→ `app/src/main.ts:339-344`(`applyScreenParam`)。`main.ts:722` で無条件に呼ぶ。native: `lift/crates/fe-lift/tests/screen1.rs:109,135-158` が `run()` の環境で設定 | 入る。`fle_screen.h` は `fle_desk.c`・`fle_input.c` 等から使われ、それらは stage_rt でコンパイルされる | 作業領域と解像度の値。640×480〜7680×4320 の外、または数字でない値は既定 1920×1080(`fle_screen.h` の `fle_screen_dim`) | 残す(部品間パラメータ) | (b) 契約の実装: GetMonitorInfoW の作業領域・EnumDisplaySettings の解像度の出所を、ページが測った部屋の大きさで与える |
| 4 | `lift/runtime/fle_test_env.h:12`(マクロの `getenv` の枝。製品の枝は `:10`) | `FLE_TEST_GETENV`(マクロ) | 製品の runtime の使用: `fle_filedlg.c:770`・`fle_appfile.c:62`・`fle_audio_ring.c:126`。harness の診断群(`fruity.c` 等) | 製品では `getenv` を呼ばない。`fle_test_env.h:9-10` の `__EMSCRIPTEN__` の枝が `((const char *)0)` を返す。製品の build(`app/scripts`・`lift/harness/two`・`web`・`b2d2`・`.github`)から `fle_test_env_wasm.h` の参照は 0 件 | 動作確認の入口を製品では開けない | テスト専用 | 対象外(動作確認の仕組み) |
| 5 | `lift/harness/fruity.c:867`(`parse_num`) | `FLE_FRUITY_MAX_CALLS`・`FLE_FRUITY_SECONDS`(呼び出しは `fruity.c:918-919`) | 製品: `app/scripts/build-engine-bundle.sh:438` が `--env FLE_FRUITY_MAX_CALLS=4000000000`・`FLE_FRUITY_SECONDS=$FRUITY_SECONDS`(36000)を渡す。E13 の `manifest.json` の `engine.env` にこの 2 キーがある(実測)。native: `lift/harness/b2d2/run-g3.sh:15`・`lift/harness/parity/run.sh:37-38` が別の値(3000 秒、400000000000 回)を渡す | 入る(`fruity.c` は `build_nmod_wasm.sh:61` でコンパイル。`parse_num` と 918〜919 行に `#ifdef` 無し) | 実行の上限(秒・呼び出し数)。範囲外や数字でない値は拒否して停止(`parse_num`) | 残す(部品間パラメータ。注 2) | 該当なし(パラメータ。注 2 を見る) |
| 6 | `lift/harness/fruity.c:884` | `FLE_CALL_FRUITY` | 製品: manifest は設定しない(E13 の `engine.env` のキーに無し。`build-engine-bundle.sh:436-438` にも無い)。native: `run-g3.sh:15`・`parity/run.sh:37` が `1` を設定 | 呼び出しは入る(884 行は `#ifdef` の外)。ただし製品では効かない: `fruity.c:892-893` で `__EMSCRIPTEN__` のとき `cfg_on = 1` の固定。読んだ値が効くのは 886〜889 行の拒否判定だけで、製品の manifest に無いので発火しない | 製品: 効果なし。native: `1` で harness の段を有効にする | オプション(除去)。製品側の読みだけを除く | (a) 製品の余計な読み。native の挙動は変えない |
| 7 | `lift/harness/fruity.c:899`(`others` の loop。配列は `fruity.c:882`) | `FLE_FRUITY_*` の 9 名(`MAX_CALLS`・`SECONDS`・`STOP_AT`・`TRAP`・`STABLE_SECONDS`・`VIRTUAL_SECONDS`・`PEEK_AT`・`PEEK_FROM`・`PEEK_TO`) | 製品は設定しない。native: 設定されていて `FLE_CALL_FRUITY` が 1 でなければ拒否(`fruity.c:900-902`) | 入るが到達しない: 899 行は `if (!cfg_on)`(897 行)の中で、`cfg_on` は製品で 1 固定(892〜893 行)。ただし製品の manifest は `MAX_CALLS`・`SECONDS` を渡すので、その 2 名は 899 行で読まれる(拒否の判定は `cfg_on` が 1 のとき飛ばされる) | 製品: 無し。native: 拒否の判定 | テスト専用 | 対象外(製品で到達しない) |
| 8 | `lift/harness/fruity.c:1062` | `FLE_FRUITY_NO_IDLE_STOP` | 製品は設定しない(E13 に無し)。native: `run-g3.sh:15`・`parity/run.sh:38` が `1` を設定 | 入らない: 1059 行の `#ifdef __EMSCRIPTEN__` の `#else`(1061 行)の中。製品の枝は 1060 行で `fle_thread_set_idle_stop(0)` を直接呼ぶ | 製品: 常に 0。native: `1` で idle 停止を切る | テスト専用 | 対象外(製品に入らない) |

## 2 注

### 注 1: 6 の除去(2 段目の差分)
- 方法: `fruity.c:884` の `getenv("FLE_CALL_FRUITY")` を `FLE_TEST_GETENV("FLE_CALL_FRUITY")` に替える。その行の残りは変えない。
- 挙動: 製品では `e` が NULL になる。886〜889 行の拒否判定は `e` が非 NULL のときだけ働き、製品の manifest はこの名前を渡さないので発火しない。`cfg_on` は 893 行で 1 のまま。native は `getenv` のままなので変わらない。
- 読みの確認(`sed -n '882,896p' lift/harness/fruity.c`): `e` の宣言は 884 行、値の検査は 886〜889 行、`cfg_on` の代入は 892〜896 行(`#ifdef __EMSCRIPTEN__` の枝が 893 行、`#else` の枝が 895 行)。製品では 893 行の枝が通り、`e` は 886 行と 895 行のどちらでも使われない。
- 今の既定値と同じ値になるか: 同じ。製品の `cfg_on` は現在も 1 で、読みの結果は判定に使われていない。
- 撤回条件: 製品の manifest に `FLE_CALL_FRUITY` を渡す必要が出たら、この読みを戻す。

### 注 2: 5 の分類(規則の文面との食い違い)
- `docs/ops/fidelity-and-performance.md` §2 の規則「分類できない物は足さない=除去候補」を文字どおり当てると、5 は (a)(b)(c) のどれにも当たらないので除去候補になる。この文書では残す。
- 根拠 (i): 名前は製品の manifest と native の動作確認の両方が値を渡す唯一の経路で、値は実行の長さの上限である。`fidelity-and-performance.md` §6 は「オプションではない物: パラメータ(サイズの上限)」と書く。
- 根拠 (ii): 除去すると、製品の 36000 秒の打ち切りが無くなり、native の動作確認(`run-g3.sh:15`・`parity/run.sh:38`)が値を渡せなくなる。
- 残す判断は、規則の文面との食い違いとして報告に挙げる。

### 注 3: 1〜3 を (b) とする根拠
- 値の出所(表示の周期・解像度・作業領域)は Win32 の契約が返す値で、我々の runtime は既定値以外の出所を持たない。ページが測った値を唯一の経路で渡すので、env の読みは契約の実装の一部である。
- 値の選び方は別に検討する: `hz.ts` の `STANDARD_HZ` の上限 120(decision 0268)は (a) の候補で、ROADMAP R3-3 の行で扱う。この文書は env の読みだけを分類する。

## 3 食い違い(実測。2 段目の前に直すかは別の判断)
- `fruity.c:893` のコメント「the manifest passed FLE_CALL_FRUITY=1」と、`fruity.c:1063` の「the manifest passed FLE_FRUITY_NO_IDLE_STOP=1」は、現行の `build-engine-bundle.sh:436-438` と E13 の `manifest.json`(`engine.env` のキーは `FLE_AUTH_CLAIMS`・`FLE_FRUITY_MAX_CALLS`・`FLE_FRUITY_SECONDS`・`FLE_REG_SEED`・`FLE_VFS_SEED`・`FLE_VFS_SEED_ROOT` の 6 件。実測)に無い。コメントが古い。
- `lift/runtime/fle_gdc.h:53` と `fle_monitor.h:34` の「opt-in」は、現行の `main.ts:722`(`applyScreenParam` の無条件の呼び出し)のため、製品では常に設定される。
- E13 の `artifacts/bundles/fl-engine-ci-0363b050/manifest.json` には `FLE_DWM_HZ`・`FLE_DISPLAYSETTINGS_FREQUENCY`・`FLE_MONITOR_W`・`FLE_MONITOR_H` が無い。E13 は既定値(60 Hz・1920×1080)で動いたことになる。現行ページの bundle とは別。
- 現木の `lift/runtime` に `getenv("FLE_SCREEN_DPI")` は無い(grep 0 件)。R2-5 の差分で入る予定で、この文書の対象外。

## 4 未実施・未確認
- 未実施: 除去の差分(2 段目)。コードの変更。`cargo test`・`npm test` の実行。ビルド(emcc・cargo)。headless Chrome の実行と P1〜P3 の測定。wasm の中身(残る `getenv` の呼び出し)の解析。
- 未確認: `emcc` が `__EMSCRIPTEN__` を定義すること(測定中のため emcc を走らせていない。製品の wasm が 4 の `FLE_TEST_GETENV` の枝を通ることは、この読みの前提)。E13 の wasm の中の `getenv` 呼び出しの有無(解析していない)。
- 読みで確かめた物: `build_nmod_wasm.sh` の `CFLAGS_H`(`-D` は `FLE_FRUITY=1` の `main_two.c` だけ。`fruity.c` には `-D` 無し)、`link-web.sh:21` の `-sEXPORTED_RUNTIME_METHODS=ENV,...`(`getenv` は Module.ENV を読む)、`fruity.ts:127`(manifest の `engine.env` を ENV へ写す)。

## 5 出典
- 呼び出し: `grep -n getenv lift/runtime/fle_thread.c lift/runtime/fle_screen.h lift/runtime/fle_test_env.h lift/harness/fruity.c`
- 製品の入り方: `lift/harness/two/build_nmod_wasm.sh`(stage_rt・stage_harness)、`lift/harness/web/fruity/link-web.sh:21`、`app/scripts/build-engine-bundle.sh:436-438`
- 設定する側: `app/src/engine/hz.ts:27-28`・`app/src/ui/screen-fit.ts:63-64`・`app/src/main.ts:647-648,339-344,721-722`・`app/src/engine/fruity.ts:127`、native は `lift/harness/b2d2/run-g3.sh:15`・`lift/harness/parity/run.sh:37-38`・`lift/crates/fe-lift/tests/screen1.rs`
- E13: `artifacts/bundles/fl-engine-ci-0363b050/manifest.json`(`engine.env` のキーと `args`)
- 既存の書式: `docs/decisions/0352-write-only-vars.md`・`0354-callback-stack-limit.md`(分類と撤回条件の書き方)

## 6 2 段目(除去の差分。測定が終わってから別の依頼で行う)
完了条件:
1. `grep -rn 'getenv' lift/runtime lift/harness/fruity.c` の残りが、この表の行に対応すること。想定される残り: `fle_test_env.h:2,3,5`(コメント)と `:12`(マクロ本体)、`fle_thread.c:824,847`、`fle_screen.h:30`、`fruity.c:867,899,1062`。`fruity.c:884` は 6 の差分で消える。
2. `cargo test` の失敗 0(取り込み前から残る `getmodulehandleexw_returns_in_every_build` を除く)。
3. 差分の commit は 6 だけ。単独の commit にする。
