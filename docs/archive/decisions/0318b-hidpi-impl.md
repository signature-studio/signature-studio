# 0318b O7: 画面のぼやけ(HiDPI)の実装 — 差分のみ(未適用・未 push)。gpu1/gpu8、2026-11-07 08:55

**実エンジンで何を実行したか: 何も実行していない(この作業にブラウザは使っていない)。** 実エンジン上の挙動(『Main GUI scaling factor: 1.75 (system set to 168PPI)』が出るか、画面が鮮明か、Sampler・Settings・File→Open で止まらないか)は **未取得**。cand2 の build が出てからの確認項目は §6。
**実行したのは**(下に結果。どれも私の作業木 `~/work/gpu8/hidpi/tree`、4b5204c2 の `git archive`):
- `emcc -fsyntax-only -Wall -Wextra`(`fle_proc.c` `fle_sysmetrics.c` `fle_appfile.c` `fle_gdc.c`): 警告・エラーなし(HEAD にもある `fle_input.c` の `ev_total` の既存警告のみ)。
- Rust のネイティブ試験(clang で runtime の C を試験用にコンパイルして実行): §5 の表。
- vitest 全体(`--maxWorkers=1`): 68 files pass / 1 skipped、1318 tests pass / 7 skipped。`tsc --noEmit` 3 設定: エラーなし。
- **代役・再生・推測**: §4 の表の『投影』の列は**推測**(測定値に画素数の比を掛けた物)。測定値の列は 0311 の `rt-ptv-*`(実 Chrome 156 headless、E13 の実エンジン、1920×1080、dpr 1)の driver snap から。それ以外に代役はない。

## 1. 区分と方針(lift の忠実性)
- **(b) 契約の実装**: 外部 API(Win32)が文書の契約どおりに答える。DPI を答える関数(`GetProcessDpiAwareness`、`IsProcessDPIAware`、`GetDpiForMonitor`、`GetDeviceCaps(LOGPIXELSX/Y, HORZSIZE/VERTSIZE)`、`GetSystemMetrics` のアイコン・カーソル、`SetThreadDpiAwarenessContext`、`SetThreadDpiHostingBehavior`)を、**1 か所の値**(`fle_screen_dpi()` = 環境の `FLE_SCREEN_DPI`、無ければ 96)から答える。ページ側は room を device px で数えて canvas を 1:1 で出す(補間の除去 = ぼやけの除去)。
- (a) 余計な処理の除去は**ない**。FL のクラス名・窓の種類・位置による分岐は足していない。環境の値(`FLE_MONITOR_W/H`、`FLE_SCREEN_DPI`)は仮想ハードウェアの記述であり、オプションではない(旗・環境変数による動作の切り替えは足していない: `FLE_SCREEN_DPI` が無い/範囲外なら 96 = 今までと同じ答え)。
- 『プロセスは環境に依らず per-monitor aware』は PM の 0318 の指示(= 決定 0099 の撤回条件・決定 0212 §6 が『確認が要る』とした点)に従う。
- 決定 0212(`FLE_SCREEN_DPI` と `fit-device` の導入)と 0243(no-options で削除)の関係: 0243 は『ページの DPI 選択という**選択肢**』を除いた。今回は選択肢ではなく、『ゲストの画面 = 物理 px、その DPI を答える』を**唯一の動作**にする(旗なし)。0212 §6 の実測(`FLE_SCREEN_DPI=169` で FL のウェルカムウィザードが 874×570 → 1529×1003 に自分で拡大、停止なし)がこの動作の前例。

## 2. 変更(差分の置き場: `lift/out-git/gpu8-results/diffs/`)
| 差分 | 中身 |
|---|---|
| `0318-hidpi-runtime-v2.diff`(12 ファイル) | `fle_screen.h`(`fle_screen_dpi()` が `FLE_SCREEN_DPI` を読む)、`fle_proc.c/.h`(`GetProcessDpiAwareness` を常に PER_MONITOR(2)、`IsProcessDPIAware`、`SetThreadDpiAwarenessContext`、`SetThreadDpiHostingBehavior`)、`fle_sysmetrics.c/.h`(アイコン・カーソルの大きさ)、`fle_appfile.c`(コメントのみ)、`imports_table.rs`(user32 の 3 行、**表の末尾に追記** = decision 0015)、harness の一覧(`hyb/c-imports-engine.txt` `vis1/vis1_apis.inc` `vis1/link-flags.txt` `parity-paint/parity_traceall.c` `parity-paint/traceall-wrap-flags.txt`) |
| `0318-hidpi-rust-tests-v2.diff`(11 ファイル) | `screen1.rs`(DPI の試験を env 駆動に: 3 本)、`dpi1.rs`(新規 4 本: 3 つの新 API と GetProcessDpiAwareness)、`wall_s.rs`(GetProcessDpiAwareness の期待値)、`rows_0318/mod.rs`(新規)と `wall_fr4,5,13,15,16,19,20.rs`(rows_0267 に倣う行数の更新) |
| `0318-hidpi-page.diff` | `app/src/ui/screen-fit.ts`、`app/src/main.ts` |
| `0318-hidpi-page-test.diff` | `app/test/unit/screen-fit.test.ts` |
- **v1 の `0318-hidpi-runtime.diff` と `0318-hidpi-rust-tests.diff` は使わない**(§7 の自己申告)。
- 積み順: runtime-v2 → rust-tests-v2 は、他の diff が `imports_table.rs` の末尾に行を足す場合、**rows_0318 を最後**にする(行の位置 = 擬似 VA)。page は alpha1 の 0319-page-wall-clock-env / 0319-page-num-cpu と 3 通りの順で `patch -p1` と `git apply` が成功(4b5204c2 の `app/` に対して)。

### 2.1 ランタイムの契約(Learn の引用)
- `GetProcessDpiAwareness`: manifest の `<dpiAware>True/PM</dpiAware>`(インストール済み `FL64.exe`、実測、census `tables/0318-dpi-imports-FLEngine.txt`)= per-monitor aware なので、環境に依らず `PROCESS_PER_MONITOR_DPI_AWARE`(2)。決定 0099 の『UNAWARE』は宣言が分からない時の選択で、撤回条件(ホスト exe の宣言が分かった)に当たる。
- `IsProcessDPIAware`: 『Returns TRUE if the process is dots per inch (dpi) aware』→ 1。
- `GetDpiForMonitor`: 『PROCESS_PER_MONITOR_DPI_AWARE: the DPI of the monitor』→ `FLE_SCREEN_DPI`(3 つの `MDT_*` は同じ数 = `S-g8a-dpi`、出典なしの選択)。dpiX = dpiY。
- `GetDeviceCaps(LOGPIXELSX/Y)`(`fle_gdc.h`)、物理寸法 `HORZSIZE/VERTSIZE` = `px × 25.4 / dpi` の丸め(決定 0121)は、すでに `fle_screen_dpi()` から導出されていた(今回 env を読むようになっただけ)。
- `GetSystemMetrics`: アイコン `SM_CXICON/CYICON/CXSMICON/CYSMICON` = Learn『About Icons』(System icon sizes)の表『96 DPI 16/32、150% 24/48、200% 32/64』と『scale with the display DPI』→ `round(base × dpi / 96)`(表に無い DPI の間は `S-gpu8-icon-scale` = 出典なしの補間)。カーソル `SM_CXCURSOR/CYCURSOR` = Learn『About Cursors』の表(<144 DPI 32、144–191 48、192–287 64、288–383 96、≥384 128。『at default cursor size setting』)。
- `SetThreadDpiAwarenessContext`: 『old DPI_AWARENESS_CONTEXT for the thread. If the dpiContext is invalid, the thread will not be updated and the return value will be NULL』。有効な値は windef.h の 5 つ(-1…-5)。スレッドは manifest の awareness(-3 = PER_MONITOR_AWARE)で始まる。
- `SetThreadDpiHostingBehavior`: 『previous DPI_HOSTING_BEHAVIOR for the thread. If the hosting behavior passed in is invalid, ... DPI_HOSTING_BEHAVIOR_INVALID』(-1)。有効は 0(DEFAULT)と 1(MIXED)。
- `AdjustWindowRectEx` は DPI に関わらず窓の非クライアント領域 0 のモデル(`S-adjustrect-nc-zero`、決定 0126)のままで、今回は変えていない。窓の枠・キャプションの大きさ(`SM_CYCAPTION` など)も同じ前提(非クライアント領域を持たない)で、DPI で変わらない。
- **選択(出典なし、`S-gpu8-*`)**: `S-gpu8-dpi-max`: `FLE_SCREEN_DPI` の上限 384(400%。Cursor の表の最後の行)。`S-gpu8-icon-scale`: 上記の補間。`S-gpu8-thread-dpi`: スレッドごとの context と hosting behavior は**保存して返すだけ**で、他の答え(`GetDeviceCaps`、`GetSystemMetrics`、`GetDpiForMonitor`)は context に依らない(出典がない: Learn は context ごとの仮想化を述べるが、このモデルの画面は 1 つで、仮想化する『他の DPI』がない)。

### 2.2 ページ
- ゲストの画面 = room の **device px**(`round` ではなく切り捨て。canvas が room をはみ出さないため。`S-gpu8-room-floor`)。ブラウザが `ResizeObserver` の `device-pixel-content-box` で整数の device px を渡す時はそれを使う(分数の dpr 1.75 などで丸め誤差が出ない)。非対応のブラウザ(Safari など)は CSS px × dpr。
- canvas の CSS の大きさ = guest / dpr(1 guest px = 1 device px、ブラウザは補間しない)。dpr < 1(縮小表示)は 1 として扱う(今までと同じ)。`maxScale` のような上限はない。
- エンジンの環境: `FLE_MONITOR_W/H` = device px、`FLE_SCREEN_DPI = round(96 × dpr)`(96..384)。ビーコンの `dpi` を埋める(今まで null。サーバーの検証は 96..384 または null を受け付ける)。
- `SCREEN_MAX`(7680×4320)で clamp: dpr 2 の 4K(3840 CSS px 幅)は 7680 に当たる。

## 3. 変えていない物(限界)
- **実行中の dpr の変化(ブラウザのズーム、別 dpr のモニタへ動かす)は反映しない**。DPI は起動時に 1 回だけエンジンに渡す。`WM_DPICHANGED` は送らない(送るには ゲストのメモリ上に提案矩形の `RECT` を作る必要があり、FL の処理も未測定)。dpr が変わった後の room は device px の大きさだけが `WM_DISPLAYCHANGE` で伝わり、DPI は古いまま。再読み込みで反映される(0212 §6 と同じ限界)。
- スレッドごとの DPI context は答えに影響しない(上、`S-gpu8-thread-dpi`)。
- 窓の非クライアント領域のモデルは変えていない。FL が自分で描く枠・メニューは DPI で FL 自身が拡大する(0212 §6 の実測)。

## 4. 影響(P2: UI 120 fps)— 画素数と費用
**測定値(実)**: 0311 の `scn-pt.json`(`rt-ptv-*`、実 Chrome 156 headless/SwiftShader、実エンジン E13、**ゲスト 1920×1080 = 2.07 Mpx**、dpr 1、この機械、load は run ごとに違う)。『play』= 再生のみ(20 秒)、『menu』= 再生中の File メニュー Click/Esc の 35 往復の区間。present の矩形はエンジンの damage の**外接**で、実際に変わった画素の数ではない(**変わった画素だけの割合は未取得**。取る方法: 連続する 2 フレームの `deskDumpBase64` の差分。今回は Chrome を使えない)。

| arm(n) | 区間 | present/s | 矩形/present | 画素/present | 画面に占める割合 | MB/present(RGBA) | put ms/present | compose ms/present(主スレッド) |
|---|---|---|---|---|---|---|---|---|
| p0mt(8)、対照 | play | 16.9 | 1.00 | 732.6 K | 35.3% | 2.93 | 0.355 | 1.317 |
| p0mt(8) | menu | 16.1 | 0.93 | 486.6 K | 23.5% | 1.95 | 0.227 | 1.215 |
| pv7s(4)、P+V7(E14 の組) | play | 30.3 | 1.00 | 689.1 K | 33.2% | 2.76 | 0.275 | 0.006 |
| pv7s(4) | menu | 30.1 | 1.01 | 473.1 K | 22.8% | 1.89 | 0.224 | 0.007 |

(pv7s の compose の列は、P の Worker に compose が移った後の主スレッドの分だけ。Worker 側の compose の ms/present は**未取得**。Worker の CPU は `pt-table.py` の『Workers (cores)』にコア数で出る。)

**投影(推測)**: 画素数だけを比で拡大(線形。present 率・矩形の数は同じ)。dpr 1.75 は『同じ CSS の room が 3.06 倍の device px』、4K は 8.29 Mpx(1080p の 4.0 倍)。

| ゲスト | 面 | 倍率 | 画素/present(play、pv7s / p0mt) | 画素/present(menu、pv7s) | MB/present(play) | 120 present/s の場合(play) |
|---|---|---|---|---|---|---|
| 1920×1080(dpr 1、**測定**) | 2.07 Mpx | ×1 | 0.689 / 0.733 Mpx(33–35%) | 0.473 Mpx | 2.8 / 2.9 | 83 / 88 Mpx/s、0.33 / 0.35 GB/s |
| 3360×1890(CSS 1920×1080、dpr 1.75) | 6.35 Mpx | ×3.06 | 2.11 / 2.24 Mpx | 1.45 Mpx | 8.4 / 9.0 | 253 / 269 Mpx/s、1.0 / 1.1 GB/s |
| 3840×2160(dpr 1.75 の CSS 2194×1234、または dpr 1) | 8.29 Mpx | ×4.0 | 動く部分が DPI 比で増えるだけ(×3.06)なら 2.11 / 2.24 Mpx(25–27%)、割合が同じ(33–35%)なら 2.75 / 2.93 Mpx | 1.45–1.89 Mpx | 8.4–11.7 | 253–352 Mpx/s、1.0–1.4 GB/s |
- 本機の present 率(pv7s 30/s、p0mt 17/s)は SwiftShader(ソフト GL)に縛られている。120/s の列は**目標の P2 の present 率を仮定した算術**であり、実 GPU(依頼者の RTX 5080)での費用は**未取得**。
- 本機の画素当たりの費用(測定、線形と仮定): put 0.40–0.48 ms/Mpx、draw 0.10–0.17 ms/Mpx、compose(p0 の主スレッド)1.8–2.5 ms/Mpx。2.1–2.9 Mpx/present に当てはめると put 0.8–1.4 ms、draw 0.2–0.5 ms、p0 の主スレッドの compose 3.8–7 ms/present(= 120 fps の予算 8.3 ms の 46–87%。P+V7 は compose を Worker に移すので主スレッドには出ない)。
- **別の実測(0212 §7.1、旧ページ、実エンジン、dpr 1.7647、3 回の中央値)**: 画素 2.07 → 6.03 Mpx(×2.9)で『playing』の全プロセス CPU 0.79 → 1.20 コア(+52%)、ページ主スレッド 0.167 → 0.355、Worker 0.480 → 0.655。idle はほぼ変わらない(0.345 → 0.390)。現在のページ(P+V7 の別 Worker、damage の外接の送出)での値は**未取得**(cand2 の build 後に同じ手順で取る)。
- メモリ(実測、`logs/fb5a`): 3840×2160 のゲスト(8.29 Mpx)で WebAssembly.Memory 1,027 → 1,124 MB(+97 MB、1080p 比 +6.2 Mpx で ≈ 16 MB/Mpx)。dpr に依らず画素数で決まる(読み)。0212 §7.1 は 1.94 → 6.03 Mpx で 932 → 1,030 MB(+98 MB、≈ 24 MB/Mpx)で、傾きは揃っていない(窓の数・大きさが DPI で変わるため。原因は未調査)。
- 面 1 枚の大きさ(BGRX 32 bit = 4 B/px、算術): 主窓の面は 1920×1080 で 8.3 MB、3360×1890 で 25.4 MB、3840×2160 で 33.2 MB(実測の裏づけ: cand1 の停止報告に『surface of top-level window lid 14: 2260 x 1080 (BGRX 32-bit top-down)』)。実測の +97 MB(4K、1080p 比)は、主窓の面の増分(+24.9 MB)の約 3.9 倍 = 全画面大の面が数枚ぶん増える計算(読み。どの面かの内訳は未取得)。
- エンジン側(wasm)の合成(窓 → デスクトップ面)の費用の画素当たりの値は**未取得**(0212 の total−主スレッド−Worker から読めるが、窓の数・大きさが DPI で変わる = 線形ではない)。

## 5. 試験の結果(cargo、fe-lift、私の木)
**cargo test -p fe-lift --no-fail-fast(私の木、4b5204c2 + 4 本の diff、2 回目、08:52–08:58)**: 162 個の test 結果、**2129 passed / 0 failed / 0 ignored**。実 DLL の試験も走った(`binaries/` と `tools/` を repo から繋ぎ、`lift/out-vfs/seeds-896.txt` を複製。`vcall_thunk_dll` は 31.8 秒かかり、`unsupported_total` は 223 のまま = 追加した 3 行は FLEngine に thunk を持たない delay-load の import)。1 回目(v1 の diff)の失敗は §7。
- 新規・変更した試験: `screen1`(8 本: env 駆動の DPI〔192 / 168 / 96 / 384〕、範囲外・不正な `FLE_SCREEN_DPI` は 96、アイコン・カーソルの大きさ〔96 / 120 / 144 / 168 / 192 / 288 / 384〕、getenv の名前に `FLE_SCREEN_DPI`)、`dpi1`(新規 4 本: `GetProcessDpiAwareness` が `FLE_SCREEN_DPI` に依らず 2、`IsProcessDPIAware` = 1、`SetThreadDpiAwarenessContext` の保存と戻り値と不正値、`SetThreadDpiHostingBehavior` 同)、`wall_s`(`GetProcessDpiAwareness` の期待値を 2 に)、`wall_fr4,5,13,15,16,19,20`(rows_0318)。
- 期待値は Learn の文と windef.h の値から手で書いた(C の出力を写していない)。
- 『ランタイムの C を実エンジンで動かした』試験ではない(clang で試験用にコンパイルしたネイティブ実行)。wasm の build(emcc の link)は私の木では行っていない(構文検査のみ)。

## 6. 実エンジン上の確認(cand2 の build の後、未取得)
1. Chrome で dpr 1.75(`--dpr 1.75`、viewport 2194×1234 など): `canvas.width` = `devicePixelContentBoxSize` の幅、canvas の CSS 幅 = guest/1.75、スクリーンショットが dpr 1 の絵の整数倍拡大ではなく、エッジが鋭いこと(周波数成分で比較)。
2. engine log(Settings → Debug): 『Main GUI scaling factor: 1.75 (system set to 168PPI)』(依頼者の native の log の文言)。
3. 止まる所の確認: Sampler(プラグイン窓)、Settings(General の Main GUI scaling)、File → Open、FLP の読み込み、Filter / Balance 等を DPI 168 で開く(0212 §6 が『未確認』とした範囲)。`SetThreadDpiAwarenessContext` / `SetThreadDpiHostingBehavior` を FL が**いつ・どの値で**呼ぶかを記録する(wrap の trace)。
4. 性能: §4 の表と同じ手順で dpr 1.75 の run を P+V7 で取り、present/s・Worker のコア・メモリを実測に置き換える。
5. Firefox 159: `device-pixel-content-box` に対応するはず(Firefox 93+)だが、依頼者の機械の版では**未確認**。

## 7. 自己申告(08:44 に渡した v1 の不備)
- **何が**: `0318-hidpi-runtime.diff` が `imports_table.rs` の 3 行を表の**途中**(GetDpiForMonitor の直後)に挿入していた。decision 0015(append only、行の位置 = 擬似 VA)に反する(後ろの行の擬似 VA がずれる)。同じ v1 は、`vis1/link-flags.txt` と `parity-paint/traceall-wrap-flags.txt` を更新しておらず、`wall_fr4,5,13,15,16,19,20` が行数・位置を固定している試験(rows_0267 に倣う)も更新していなかった(wall_fr21 は `link-flags.txt` と `vis1_apis.inc` の一致を見る)。
- **いつ・なぜ**: 08:44、cargo test 全体を回す前に diff を作って渡した(`patch --dry-run` が通ることだけを見ていた)。全体を回して(08:43–08:49 の 1 回目)、行の位置・数を見る 7 件(wall_fr4,5,13,15,16,19,21)の失敗を見て気づいた。ほかに authenticode_check 9 件も落ちたが、私の木に `tools/` が無いためで私の変更とは無関係(2 回目は `tools/` と `binaries/` を繋いだ)。
- **是正**: 08:51 に PM に info を送り、v2(末尾に追記、rows_0318、一覧の更新)に差し替え。2 回目の全体は §5。

### 7.1 0318 の作業全体の自己申告(未記録だった分、事実と是正)
- 07:29 と書いた info の見出しの時刻は、実際の時刻(07:24)と違っていた。以後、見出しの時刻は `$(date +%H:%M)` で入れ、送る前に時計を確認している。
- O9(General タブのスクロール破損)の報告で、『peek』と『dump』の sha を取り違えて書いた(canvas / peek / dump はスクロール後に全て一致、が正しい)。訂正の info は送付済み。
- 引用符なしの heredoc で info の本文を書き、本文の一部が欠けて送られた。訂正の info は送付済み。以後、本文は quoted heredoc(`<<'EOF'`)で `~/work/gpu8/msg/` に書く(今回の info は時刻だけ引用符なしで変数展開し、本文に `$` や backtick を含む箇所は事前に確認した)。
- `( cargo ... &)` の形での起動と、`sleep 0` / 末尾 `&` のコマンドを使った(後者は拒否された)。以後、待機は `background: true` か前面の `tail --pid` だけ。
- `/tmp/opencode` に置いた自分のファイルを `~/work/gpu8/{tmp,msg}` に移した。

## 8. RT-B の候補での再確認(09:5x、PM の指示)
- mt3 の `tree-rtb`(`~/work/mt3w-s1/tree-rtb`)には **HiDPI v2 の runtime と Rust 試験が既に入っている**(私の v2 の 2 本を `patch --dry-run` すると全 hunk が『previously applied』)。page の 2 本(main.ts / screen-fit.ts / screen-fit.test.ts)はまだ入っていない。
- 23 ファイルを私の木と1つずつ比べた: runtime(`fle_appfile.c` `fle_proc.h` `fle_screen.h` `fle_sysmetrics.c/.h`)、harness の一覧 5 本、`dpi1.rs` `rows_0318/mod.rs` `screen1.rs` `wall_s.rs` は**同一**。違うのは他の担当の分だけ: `imports_table.rs`(`kernel32!CreateSemaphoreW` / `ReleaseSemaphore` の 2 行〔decision 0234、mt3〕が SetWindowPlacement の後・私の 3 行の前)、`wall_fr4,5,13,15,16,19,20.rs`(`rows_0234` も引く)、`wall_fr15.rs` の他 2 行、`fle_proc.c`(下の 1)。**rows_0318 は表の最後**(`IsProcessDPIAware` `SetThreadDpiAwarenessContext` `SetThreadDpiHostingBehavior` が 781〜783 行)で、decision 0015(append only)を満たす。
- **1 件の指摘**: `tree-rtb` の `lift/runtime/fle_proc.c` の `fle_imp_ExitProcess` の中(285〜288 行、ファイルの更新時刻は 09:50)に『`extern void fle_dbg_stack(void) __attribute__((weak));   /* spike diagnostic (not for the deliverable): the wasm stack of the caller */ if (fle_dbg_stack) fle_dbg_stack();`』がある。HiDPI の差分ではなく(私の木と HEAD にはない)、コメントが『deliverable ではない』と言っている。RT-B の候補に入れるものかの確認が要る。
- **cargo の確認(tree-rtb の複写、10:0x)**: `tree-rtb` の `lift/{Cargo.*,crates,runtime,harness}` を複写(`binaries/` `tools/` `out-vfs/seeds-896.txt` は repo から繋ぐ)して、HiDPI に関わる 15 本の試験 binary を実行: `dpi1` `screen1` `sysmetrics` `gdc` `monitor` `wall_s` `wall_fr4` `wall_fr5` `wall_fr13` `wall_fr15` `wall_fr16` `wall_fr19` `wall_fr20` `wall_fr21` `vcall_thunk_dll` = **205 passed / 0 failed**(実 DLL の `vcall_thunk_dll` を含む)。`rows_0234`(semaphore)と `rows_0318` が両方とも数えられた状態で行の位置の試験が通る = rows_0318 を最後に積む順が RT-B の木で成立している。試験の全体(fe-lift の全 binary)は未実行(RT-B の CI に任せる)。
