# Blend2D の行なし import の残り 14 名（+2 名）の調査（調査のみ。決めない）

凡例: **実測**（この作業で実行して得た値、取得したページ）／**読み**（逆アセンブル、ソース、文書を読んだ結果。実行していない）／**未確認**／**意見**（事実ではない。別枠に書く）。

この文書の状態: 完成（読み取りと静的解析だけ。何も実行していない。決めない）。

- HEAD: `b0dc3ab`（ブランチ main）。ワークツリーに決定 0091（`GetModuleHandleA`）の未コミット変更あり（この作業のものではない）。
- この作業が作ったもの: この文書と `lift/out-imp14/`（`tmp/` と解析スクリプト、出力、Learn の取得物）だけ。`lift/` で `cargo build` / `cargo test` は実行していない。
- 対象: Blend2D の行なし import のうち、`GetModuleHandleA`（決定 0091 で実装）の残り 14 名（ATTACH から静的に届く名前）+ 2 名（`WakeAllConditionVariable`、`GetLargePageMinimum`。ATTACH では届かない）= 16 名。原本 `lift/out-blend2d/Blend2D.dll`（SHA-256 `66634d24…66c7`）とリベース済み `lift/out-seed58/s58/dll/Blend2D.4700000.dll`（`24e01ec9…b4cb`）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 呼び出し点の全数 | 16 名 83 サイト（AcquireSRWLockExclusive 20、ReleaseSRWLockExclusive 27、Shared 3/6、Wake 4、WakeAll 1、SleepConditionVariableSRW 4、CreateThread 1、ExitThread 2、FreeLibraryAndExitThread 1、GetFileSizeEx 1、IsProcessorFeaturePresent 5、OutputDebugStringA 2、TlsFree 2（`jmp [IAT]`）、WaitForSingleObjectEx 1、GetLargePageMinimum 3）。既存の feasibility と一致。**SRW の Release の 3 か所は `jmp [IAT]`**（tail jump） | 実測（§4.0） |
| 2 | **今回の停止点** | `f_18019caa0` は `.xc` の初期化子 `f_180199d90` が `0x18019a102` で `0x1802f47b0` に書いた関数で、`0x18019a1d2: call r8` で呼ばれる。`this` は**静的な `0x1802f47d0`**（スレッドプールの静的オブジェクトに読める）で、SRWLOCK は `this+0x30`（= `0x1802f4800`）。**ATTACH で確実に実行される SRW はこの 1 対（Acquire 1 + Release 1）だけ** | 実測（停止）、読み（`this`、SRWLOCK） |
| 3 | ATTACH の直線上か | **直接 call だけの A では SRW、条件変数、スレッドは 0 か所**。`.xc` が `lea` で表に書くだけで呼ばない関数にあるものが B で 74 か所（アドレスが取られるだけ。実行は未確認）。`.xc` の末尾の直接 call 5 つ（`f_1800a9eb0`、`f_180147870` ほか）は自身は行なしの API を呼ばない。**IsProcessorFeaturePresent 5、TlsFree 2、WaitForSingleObjectEx 1 は A で届くが、いずれも通常は呼ばれない分岐**（fail-fast のエラー経路、DETACH／`FlsFree` が引けないとき、別スレッドの初期化中）。CreateThread は届かない読み。GetLargePageMinimum は届かない | 読み（§4.1）。`WaitForSingleObject(Ex)` は既存ログで ATTACH の途中まで未呼び出し（実測） |
| 4 | SRWLOCK の置き場 | 静的 2 本（`0x1802f4700`、`0x1802f4800`）は、`.text` の参照が `lea` と `.xc` の 1 回のゼロの書き込みだけ（**ゲストは値を読み書きしない**。実測）。ヒープ上のロック（スレッド 1 個のオブジェクトの `+0x80`、他）は**確認していない**。Blend2D は `InitializeSRWLock`、`InitializeConditionVariable`、`TryAcquireSRW*` を **import しない**（静的初期化 = ゼロ） | 実測（import、参照の全数）。未確認（ヒープ） |
| 5 | 戻り値 | SRW・Wake・ExitThread は VOID。SleepConditionVariableSRW の戻りは 1 か所だけ使う。**4 か所とも Flags = 0（排他）、INFINITE が 3、有限（計算値。ミリ秒の値は未確認）が 1**。述語を `while` で見る形（既に真なら Sleep しない） | 読み（§4.2） |
| 6 | **`WaitOnAddress` との関係** | `.xc` が `GetProcAddress` の 3 つがすべて非 NULL のときだけ `[0x1801f31a0] = 1`。**この環境は NULL を返す（0091 の実測）ので 0 → スレッドプールのスレッドは SRW + 条件変数版**になる。`WaitOnAddress` 系を答えれば別の版（`WaitOnAddress` 版）になり、SRW/条件変数は**スレッドのループには**使われなくなる。ただし今回の停止点（プールの `this+0x30` のロック）は両版で使う | 読み（§4.2） |
| 7 | Learn（19 ページ取得） | 引用は §5。**書く**: 排他は再帰不可（デッドロック）、共有の再帰は should not、Release は取得したスレッドで（must）、`SleepConditionVariableSRW` はロックが空なら undefined、INFINITE/0/有限と ERROR_TIMEOUT、`CreateThread` は DLL 初期化中は初期化完了まで走らない、`OutputDebugString` はデバッガーなしで何もしない、`GetLargePageMinimum` は 0（大ページなし）。**書かない**: **ロックを持たずに Release した場合の結果**、`SRWLOCK_INIT` の値、待機者のいない Wake の動作、ERROR_TIMEOUT の数値、Sleep がタイムアウトで戻るときのロック再取得 | 実測（取得）。§5 |
| 8 | 単一スレッド（0050 / 案 C）の下 | 空への Acquire は成功。**取られているロックへの Acquire は Learn が「deadlock」と書き、単一スレッドでは永遠に戻らないので名前付き停止にするのが整合**（0017 の `Sleep(INFINITE)`、0050 の INFINITE 未シグナル待ちと同じ）。Wake（待機者なし）は何もしない（Learn は書かない）。Sleep は INFINITE/有限・非 0 が停止（0050 §C と同じ）。**ただし静的初期化の登録（未登録アドレスの最初の使用 = 空）と、持たない Release の扱いは Learn が書かず、仮の決定になる** | 整理（§6） |
| 9 | 決めるべき点 | §7.3 に 8 点。**1 つの方針では扱えない**（SRW+条件変数、既存同種、実装済みで行だけ無い、案 C 決定済み、値が導けない、の 5 種類）。SRW+条件変数は 1 つの決定にまとめられ、スレッド、`IsProcessorFeaturePresent`/`TlsFree`、`WaitForSingleObjectEx`、`GetLargePageMinimum` は別 | 整理（§7） |
| 10 | 影響範囲 | 行を足すと FLEngine の生成 C の SHA が変わる（0078、0091 の前例。基準は `eb9d8e7f…`）。**FLEngine は `CreateThread` 2、`ExitThread` 1、`TlsFree` 1 を import しているので、行を足すとその 4 つの thunk が `fle_unsupported` から名前付き停止に変わる**。dsp_ippv2（縮小前）は `IsProcessorFeaturePresent` 5、`TlsFree` 1 を import。**`IsProcessorFeaturePresent`/`TlsFree` の行を足すと、`k32_dsp.rs` 200〜203・224〜225、`imports_table.rs` 126・129・131・133、`sysinfo.rs` 918 が失敗する見込み**。`WaitForSingleObjectEx` を足すと `event.rs` 233 が失敗。**末尾に足すたびに `modules.rs` 526〜529（0091 の「最後の行」）が失敗する** | 実測（import、前例）、読み（失敗の見込み。テストは未実行）（§8） |
| 11 | 既存ファイルに足せるか | 足せる（全 `fle_*.c` がリンクされる。SRW・条件変数は `fle_sys.c`、`WaitForSingleObjectEx` は `fle_event.c`、`OutputDebugStringA` は `fle_proc.c`、`GetFileSizeEx` は `fle_vfs.c`、`GetLargePageMinimum` は `fle_sysinfo.c`）。**前例が無いのは「未登録アドレスの暗黙の初期状態」** | 読み（§8.3） |
| 12 | 次に届く見込み | `f_18019caa0` が戻ると、`.xc` の残りは `call` が 6 つ（`f_18019bf60` と末尾の 5 つ）だけで、**行なしの API を直接呼ぶものは無い**（読み。直接 call の先は部分的にしか読んでいない）。SRW の実装で今の停止点が動いた後の次の停止は、SRW・条件変数・スレッドの他の名前ではない見込み | 読み（§9）。未確認が多い |

**この調査は何も直していない。仮の決定になる点、Learn が書かない点は §5.4、§6、§7 に集めた。**

### 1.1 作業終了時の git の状態（実測。`git status --short` と `git diff --stat`）

```
 M docs/README.md                                   （他の作業: 0091 の行）
 M lift/crates/fe-lift/src/imports_table.rs         （他の作業: 0091 の GetModuleHandleA の行 3 行）
 M lift/crates/fe-lift/tests/modules.rs             （他の作業: 0091）
 M lift/crates/fe-lift/tests/realmod.rs             （他の作業: 0091）
 M lift/crates/fe-lift/tests/realmod_s1/guest_host.c   （他の作業: 0091）
 M lift/crates/fe-lift/tests/realmod_s1/guest_mod2.c   （他の作業: 0091）
 M lift/crates/fe-lift/tests/realmod_s1/shared.h       （他の作業: 0091）
 M lift/runtime/fle_modules.c                       （他の作業: 0091）
 M lift/runtime/fle_modules.h                       （他の作業: 0091）
?? docs/decisions/0091-getmodulehandlea.md          （他の作業: 0091）
?? docs/investigations/blend2d-getmodulehandlea.md  （他の作業: 0091 の調査）
?? docs/investigations/blend2d-remaining-imports.md （この調査。新規）
?? lift/harness/wasm/trace.c.new                    （既存の未追跡。触っていない）
?? lift/harness/wasm/trace_native.c.new             （同上）
?? lift/iat-unsup-closure.txt                       （同上）
 9 files changed, 349 insertions(+), 11 deletions(-)   （git diff --stat。すべて 0091 の変更で、この調査のものではない）
```

**この調査が変更した追跡ファイルは 0 個**（`git diff --stat` の 9 ファイルは調査の開始時点から同じ。開始時の `git status` と一致する）。新規は本書と `lift/out-imp14/`（`.gitignore` の `lift/out*/` に入るので `git status` に出ない）だけ。編集禁止のファイル（fe-lift・fe-decode・fe-pe のソース、`lift/runtime`、`lift/crates` のテスト、`lift/harness`、`docs/decisions`、`docs/README.md`、`docs/progress.md`、既存の docs）に、この調査による変更は無い。

## 2. 手順と読んだ文書

### 2.1 実行したコマンド（実測）

すべて**読み取り**。`lift/` で `cargo build` / `cargo test` は**実行していない**。`emcc` は使っていない。`rm`、`>` による既存ファイルの上書き、git の変更系操作は無い。`/tmp` は使っていない（`TMPDIR=lift/out-imp14/tmp`）。python は `PYTHONDONTWRITEBYTECODE=1`、`lift/out-cs/` は cwd にしていない。

| 目的 | コマンド（要旨） | 出力 |
|---|---|---|
| HEAD、状態 | `git rev-parse --short HEAD`（`b0dc3ab`）、`git status --short`、`git log` | — |
| SHA-256 | `sha256sum lift/out-blend2d/Blend2D.dll`（`66634d24…66c7`。依頼どおり）、`lift/out-seed58/s58/dll/Blend2D.4700000.dll`（`24e01ec9…b4cb`。0091 の調査と同じ）、`lift/out-ext/dsp_ippv2_x64.dll`（`497dd4f7…c352`。0078 と同じ）、`binaries/win64/FLEngine_x64.dll`（`b2c64da4…496d`）、`lift/out-gmh/fle.c`（`eb9d8e7f…09cf`）、`lift/out-avrt88/fle.c`（`43985d62…37a1`） | — |
| 呼び出し点の全数 | `python3 imp14_sites.py ../out-blend2d/Blend2D.dll 8`（pefile + capstone の線形走査。IAT スロットを解決） | `sites-orig.txt` |
| 静的到達 | `python3 imp14_reach2.py ../out-blend2d/Blend2D.dll`（`imp14_reach.py` は訂正前の版。§4.1 に注記） | `reach-imp14c.txt`（`reach-imp14.txt`、`reach-imp14b.txt` は訂正前・失敗の版） |
| ロックの置き場 | `python3 imp14_locks.py ../out-blend2d/Blend2D.dll`、静的なロックの参照の全数（インライン python → `refs-static-locks.txt`） | `locks-orig.txt`、`refs-static-locks.txt` |
| import 表 | `python3`（インライン）で原本とリベース済みの import 名、FLEngine と dsp_ippv2 の import | `imports-blend2d.txt` |
| FLEngine、dsp_ippv2 の呼び出し点 | `python3 imp14_scan_iat.py <PE>`（生バイトの `ff 15` / `ff 25` の走査） | `scan-fle.txt`、`scan-dsp.txt` |
| 逆アセンブル | `objdump -d -M intel --no-show-raw-insn --start-address=… --stop-address=… ../out-blend2d/Blend2D.dll` | `dis-init.txt`、`dis-init2.txt`、`dis-pool.txt`（ほか標準出力で読んだもの） |
| Learn | `curl -sS -L`（19 ページ。HTTP 200）。本文抽出 `python3 learn_text.py` | `learn/*.html`、`learn/*.txt` |
| 既存ログ（読み取り） | `lift/out-gmh/run/m3-ro1-imports.txt`（決定 0091 の調査の gdb ログ）を `grep`・`sed` で読んだだけ。**gdb は起動していない** | — |

### 2.2 読んだ文書、ソース（行番号はワークツリー。実測は「読んだ」ことだけ）

- `docs/investigations/blend2d-getmodulehandlea.md` §1、§2.3、§7.2〜§7.4、§8、§9（`GetModuleHandleA` の測定と今回の停止点）。
- `docs/investigations/blend2d-lift-feasibility.md` §3.3、§5.2（226〜233 行）、§6（234〜253 行。247 行が `CreateThread`、`ExitThread`、`FreeLibraryAndExitThread`）、§7（261〜270 行。267 行が ATTACH の 59 か所）、§10.3 の B4（335 行）と B8（339 行）、§10.4（355〜357 行）。
- `docs/investigations/sync-thread-survey.md` §0〜§4（案 C は §3.3 の表と §3.4）、§6（PM の質問）。
- `docs/decisions/0017-sync-time-memory.md`（B: クリティカルセクション。22〜33 行）、`0050-event-sync.md`（A〜F。16 行が `WaitForSingleObjectEx` を実装しない根拠）、`0078-kernel32-for-dsp-ippv2.md`（§2 の表 29〜39 行、§3 の 51〜62 行、「実装したが行にしなかった 2 名」62 行）、`0080-…`（D9: 15 行）、`0091-getmodulehandlea.md`（全体）。
- `lift/runtime/fle_event.c`（1〜215。`fle_event_owner_tid` 18〜26 行、`fle_imp_CreateEventW` 129〜154、`fle_imp_SetEvent` 161〜170、`fle_imp_WaitForSingleObject` 195〜215）、`fle_event.h`、`fle_sys.c`（`fle_sys_check_thread` 29〜43、`fle_imp_Sleep` 69〜76、クリティカルセクション 115〜269）、`fle_imports.c`（`TlsFree` 129〜148）、`fle_sysinfo.c`（`IsProcessorFeaturePresent` 37〜106）、`fle_proc.c`（`SwitchToThread` 185〜197、`OutputDebugStringW` 199〜205、`ExitProcess` 211〜）、`fle_vfs.c`（`GetFileSize` 744〜）、`fle_modules.c`（`GetProcAddress` 172〜249）、`fle_imports.h`（宣言の書式）。
- `lift/crates/fe-lift/src/imports_table.rs`（`TABLE` の書式: 17〜319 行。末尾の 316〜318 行が 0091 の `GetModuleHandleA`。`dispatch_and_resp_tables` 339〜358、`resp_va` 360〜362、`lookup` 371〜377、`lookup_call` 388〜392）。
- テスト: `tests/k32_dsp.rs`、`imports_table.rs`、`sysinfo.rs`、`event.rs`、`modules.rs`、`call_iat_dll.rs`、`realmod.rs`（行番号は §8.2）。

## 3. 現行 runtime の同期・スレッドの流儀（読み）

**応答表の書式**（`imports_table.rs` 17〜319 行）: 1 行 = `("kernel32.dll", "<API>", "fle_imp_<API>", "jmp_imp:<API>")`。末尾に足す（決定 0015）。**行が 1 つ足されると** (1) `fle_dispatch_table` の末尾に擬似 VA（`0x7FF900000000 + 0x10 × (行番号 + 1)`）の項目、(2) `fle_resp_table`（`GetProcAddress` が引く）、(3) `fle_imp_*` のプロトタイプ宣言、(4) `jmp [IAT]` と `call [IAT]`（決定 0077）の `fle_unsupported` が `fle_imp_*()` に変わる。runtime 側は `void fle_imp_<API>(void)` で、引数は `R[1]`（rcx）、`R[2]`（rdx）、`R[8]`（r8）、`R[9]`（r9）、戻り値は `R[0]`（`fle_imports.h` 1〜3 行、`fle_event.h` 1〜8 行）。ゲストの戻りアドレスはスタックに残る。

**クリティカルセクション**（`fle_sys.c` 115〜269）: 状態を `fle_cs_table`（アドレスをキーにしたハッシュ表。65,536 スロット、同時に 32,768 個まで。`recursion` = 入った回数）に持つ。**オブジェクトの中身は読まず書かない**（ページが opaque と書く）。`Initialize` が登録、`Enter` が `recursion++`（所有中の再入は待たない）、`Leave` が `recursion--`。**未登録への Enter/Leave/Delete、初期化済みへの Initialize、所有していない Leave、所有中の Delete、NULL、別スレッドからの呼び出しはすべて `STOP`（abort）**（252〜258 行）。別スレッドは `fle_sys_check_thread`（`fle_host_tid()`。wasm では定数 1。決定 0045）。

**イベント**（`fle_event.c`）: 状態を `evs[]` に持つ（`manual`、`signaled`、`live`）。ハンドルは 0x7FFA80000000 + 4×(i+1)、再利用しない。`WaitForSingleObject`: シグナル済みなら 0（自動リセットは非シグナルに戻す）、非シグナルで `ms == 0` なら `WAIT_TIMEOUT`、**INFINITE と有限・非 0 は `fle_stop`**（213〜214 行）。`CloseHandle` はクローズフック（`fle_vfs_add_close_hook`）。同じ流儀: `fle_event_check_thread`（18〜26 行）、`fail()`、`fle_stop(api, why)`、コメントに Learn の引用と「assumption」の印。

**時間**（`fle_sys.c` 45〜113）: 仮想時計は `Sleep(ms)` が `ms × 10,000` tick 進める。**`Sleep(INFINITE)` は abort**（74 行）。時計を読む呼び出し（`QueryPerformanceCounter`、`GetTickCount`、`timeGetTime`）は値を返した後に 10 tick 進める。**待ちの API は時計を進めない**（決定 0017、0050 §C）。`fle_time.c` はカレンダーと時刻の変換で、時計の窓口は持たない（`fle_sys_clock_ticks` を読むだけ）。

**プロセス・スレッド**（`fle_proc.c`）: `SwitchToThread` は常に 0（185〜197 行。100 万回で停止）。`SetThreadPriority` は擬似ハンドルだけ（「no OpenProcess or CreateThread answer」: 99 行のコメント）。`OutputDebugStringW` は「デバッガーなし」で何もしない（199〜205 行）。`ExitProcess`、`TerminateProcess`、`RaiseException` は名前付き停止（211〜263 行）。

**共通の流儀**（読み）: (i) 1 つの runtime ファイル内に API を足す（新しいファイルを作らない）。(ii) Learn の引用をコメントに書き、書かれていない値は「assumption」（またはそれに相当する語）と書く。(iii) 状態はゲストのオブジェクトの外（表）に持つ。(iv) ページが定めない入力、実装しない入力は `STOP` / `fle_stop` で名前付き停止。(v) 別スレッドは停止。(vi) ホストの時計・スレッド・プロセス・ファイルを読まない（`source_uses_no_host_…` 系のテストが禁止語を grep する: `event.rs` 679〜680、`sysinfo.rs` 950〜951、`k32_dsp.rs` 739）。**SRW と条件変数は、この流儀にそのまま載せられる**（状態の表、`STOP`、`fle_sys_check_thread`）。載らないのは **「未登録のアドレスの暗黙の初期状態」**（§8.3）。

## 4. 14 名の表（Blend2D 側の事実）

### 4.0 呼び出し点の全数（実測。`lift/out-imp14/imp14_sites.py` を原本 `Blend2D.dll` に実行。出力 `lift/out-imp14/sites-orig.txt`）

pefile で IAT スロットを解決し、capstone の線形走査で `.text` の `call/jmp [rip+disp]` の宛先がスロットに当たるものを数えた（関数は `.pdata` の開始）。**既存の文書の数と一致**（SRW Acquire 20、Release 27、Shared 3/6、Wake 4、WakeAll 1、Sleep 4、CreateThread 1、ExitThread 2、FreeLibraryAndExitThread 1、GetFileSizeEx 1、IsProcessorFeaturePresent 5、OutputDebugStringA 2、TlsFree 2、WaitForSingleObjectEx 1、GetLargePageMinimum 3）。

| 名前 | 原本のスロット | 呼び出し点（原本 VA。関数） | 形 |
|---|---|---|---|
| AcquireSRWLockExclusive（20） | 0x1801b7098 | 1800a9c70(f_1800a9bb0)、18013d9b5(f_18013d7b0)、18014644b・1801464ff(f_180146040)、18019414f・18019418e(f_180194120)、1801942ae(f_180194286)、1801997eb(f_180199670)、180199d52(f_180199d00)、18019bed0(f_18019bec0)、18019bf10(f_18019bf00)、18019c4f1(f_18019c4d0)、18019c5a0(f_18019c560)、18019c63e(f_18019c5e0)、18019c997(f_18019c920)、**18019cab9(f_18019caa0)**、18019cb4a(f_18019cb20)、18019cba8(f_18019cb80)、18019cfca(f_18019cfa0)、18019d041(f_18019d020) | すべて `call [IAT]` |
| ReleaseSRWLockExclusive（27） | 0x1801b7090 | 1800a9c85、18013da08、180146490、180146541、18019416e、1801941ad、1801942e9、1801997fe、18019984a、180199d72、18019bef4（**jmp**）、18019bf56（**jmp**）、18019c506、18019c53c、18019c5ca、18019c69a、18019c6c0、18019c6d3、18019c9e4、18019c9f5、18019cae3、18019cb06、18019cb64、18019cbc1、18019cc18、18019cff1、18019d06d（**jmp**） | `call` 24、`jmp` 3 |
| AcquireSRWLockShared（3） | 0x1801b70a8 | 1800a9af5(f_1800a9ad0)、1800a9bda・1800a9cec(f_1800a9bb0) | `call` |
| ReleaseSRWLockShared（6） | 0x1801b70a0 | 1800a9b40・1800a9b66(f_1800a9ad0)、1800a9c31・1800a9cab・1800a9d41・1800a9d66(f_1800a9bb0) | `call` |
| WakeConditionVariable（4） | 0x1801b70b8 | 180199d65(f_180199d00)、18019c52f(f_18019c4d0)、18019c5bd(f_18019c560)、18019cb5a(f_18019cb20) | `call` |
| WakeAllConditionVariable（1） | 0x1801b70b0 | 18019980b(f_180199670) | `call` |
| SleepConditionVariableSRW（4） | 0x1801b70c0 | 18013d9f2(f_18013d7b0)、180199837(f_180199670)、18019c66d(f_18019c5e0)、18019c9d6(f_18019c920) | `call` |
| CreateThread（1） | 0x1801b71d8 | 1801a48ed(f_1801a4888) | `call` |
| ExitThread（2） | 0x1801b71e0 | 1801a4772(f_1801a475c)、1801a481e(f_1801a47c4) | `call` |
| FreeLibraryAndExitThread（1） | 0x1801b71e8 | 1801a4815(f_1801a47c4) | `call` |
| GetFileSizeEx（1） | 0x1801b7068 | 1801a8ff8(f_1801a8fa0) | `call` |
| IsProcessorFeaturePresent（5） | 0x1801b7148 | 18019e5f0(f_18019e5d4)、1801a3c0e(f_1801a3be8)、1801a6245(f_1801a623c)、1801b163a(f_1801b162c)、**1801b1721(f_1801b1714)** | `call`（5 か所とも `mov ecx,0x17` の直後） |
| OutputDebugStringA（2） | 0x1801b7008 | 1800aacca(f_1800aa1a0)、18019b1b5(f_18019b170) | `call` |
| TlsFree（2） | 0x1801b71c0 | 1801a1e0f(f_1801a1dd0)、1801a9a4b(f_1801a9a0c) | **`jmp [IAT]`**（関数末尾の tail jump） |
| WaitForSingleObjectEx（1） | 0x1801b70f8 | 18019dd4e(f_18019dd04) | `call` |
| GetLargePageMinimum（3） | 0x1801b7040 | 18000e66f・18000e6c2(f_18000e4d0)、1800182c8(f_180018250) | `call` |

リベース済み（基底 0x4700000）では、元の VA から `0x180000000 - 0x4700000 = 0x17900000` を引いた値（今回の停止の `0x489cab9` は元の `0x18019cab9`、IAT スロット `0x48b7098` は元の `0x1801b7098`）。

### 4.1 ATTACH の直線上か（静的な到達。**読み。実行していない**）

方法（`lift/out-imp14/imp14_reach2.py`。出力 `reach-imp14c.txt`。**読みと書く**）: 関数 = `.pdata` の範囲 + `.pdata` に無い葉（`call rel` と rip 相対 `lea` の宛先）。種は DllMain の入口 `0x18019e584`、`.xi` 4 項目（`0x18019db00`、`0x1801a4400`、`0x1801af6e4`、`0x1801b0e00`）、`.xc` 1 項目（`0x180001000`）。辺は **A（直接 `call rel`、別関数への `jmp rel`）** と **B（A + rip 相対 `lea` で取られる関数の開始。`b2d_reach3.py` と同じ種類の過大見積もり）**。**間接 call（`call reg`、`call [mem]`、vtable 呼び）は辿らない**ので、A は「実行される」の十分条件でも必要条件でもなく、B は「アドレスが取られる」の上界に近い。最初に書いた `imp14_reach.py` は jmp の宛先を関数の開始と扱って関数を途中で切る誤りがあり、数値を使っていない（`reach-imp14.txt`。削除禁止のため残す。`imp14_reach2.py` が訂正版）。

結果（16 名、全 83 サイト。**B は既存文書の「ATTACH から届く 59 か所（15 名）」と同じ向きの数え方だが、同じ数にはならない**: 既存は `reach3`（生成 C の即値を辿る）で関数 845 / 1,344 の集合、今回は strict な `.pdata`+gap の関数分割で A 251 関数 / B 870 関数。数え方が違うので 59 と 74 は直接比べない）:

| 名前 | 全 | A（直接のみ） | B（+ `lea` で取られた関数） | ATTACH の直線上か（読み） |
|---|---:|---:|---:|---|
| AcquireSRWLockExclusive | 20 | 0 | 17 | **直線上かは未確定**。ただし `f_18019caa0`（今回の停止点）は **`.xc` の初期化子 `f_180199d90` が `0x18019a102` で `lea` して `0x1802f47b0` に書き、`0x18019a1d2: call r8` で実際に呼ぶ**（読み。`[0x1802f47d0]` のオブジェクトの `[+0x20]` = 表の 8 番目 = `0x18019caa0`。実測の停止と一致）。つまり少なくとも `f_18019caa0` は ATTACH で実行される（**実測**: a4 でそこで停止した） |
| ReleaseSRWLockExclusive | 27 | 0 | 24 | 同上（`f_18019caa0` の `0x18019cae3`、`0x18019cb06` は実行される経路の続き） |
| AcquireSRWLockShared | 3 | 0 | 3 | B のみ（`f_1800a9ad0`、`f_1800a9bb0`。`.xc` の初期化子が `call 0x1800a9eb0` した先から `lea` で届く。実行は未確認） |
| ReleaseSRWLockShared | 6 | 0 | 6 | 同上 |
| WakeConditionVariable | 4 | 0 | 4 | B のみ |
| WakeAllConditionVariable | 1 | 0 | 1 | B のみ（`f_180199670` = `f_18019973f` に当たる関数の内部。`f_180199b80` ← `f_18013d7b0` ← `f_18013e290` ← `f_180147870`。**既存文書は「ATTACH では届かない」としていたが、strict な関数分割の B では届く**。実行の確からしさは未確認） |
| SleepConditionVariableSRW | 4 | 0 | 4 | B のみ。**4 か所とも `Flags = 0`（排他。`xor r9d,r9d`）。`dwMilliseconds` は 3 か所が `0xFFFFFFFF`（INFINITE。`mov r8d,0xffffffff`）、`f_18019c920` の 1 か所（`0x18019c9d6`）だけが変数 `eax`（有限）**（§4.2） |
| CreateThread | 1 | 0 | 1 | B のみ（`f_1801a4888` ← `f_18019ce4b`（`f_18019ccf0` の中）← `f_18019cfa0`（`.xc` が `0x1802f47c0` に書く表の 15 番目の関数）） |
| ExitThread | 2 | 0 | 2 | B のみ（`f_1801a475c`、`f_1801a47c4`） |
| FreeLibraryAndExitThread | 1 | 0 | 1 | B のみ |
| GetFileSizeEx | 1 | 0 | 1 | B のみ（`f_1801a8fa0` ← … ← `f_1800aa1a0`（`OutputDebugStringA` の関数と同じ祖先）。深い鎖） |
| IsProcessorFeaturePresent | 5 | **5** | 5 | **A でも届く（直接 call だけで DllMain の鎖に載る）**。ただし**5 か所すべてが `mov ecx,0x17`（PF_FASTFAIL_AVAILABLE）の直後で、呼び出し元は CRT の「fail fast」ヘルパー**（`f_18019e5d4`、`f_1801a3be8`、`f_1801a623c`、`f_1801b162c`、`f_1801b1714`）。呼ばれるのは通常**エラー経路**（`f_18019e5d4` は `0x18019dbc5: mov ecx,7; call 0x18019e5d4`、すなわち `.xi` の項目 0 の致命エラー経路。`f_1801b1714` は `mov ecx,8` の `__scrt_fastfail` 経路から。決定 0078 §1 と同じ形）。**5 か所目 `0x1801b1721`（`f_1801b1714`）は既存文書が「引数を読んでいない」とした 1 か所で、`mov ecx,0x17` の直後と確認した** |
| OutputDebugStringA | 2 | 0 | 2 | B のみ（`f_1800aa1a0`、`f_18019b170`）。`.xc` 初期化子が `call 0x1800a9eb0` の先から。ログ出力・エラー表示の形（§4.2）。実行は未確認 |
| TlsFree | 2 | **2** | 2 | A で届くが、**DETACH 側**（鎖が `f_18019e3cc`（`__scrt_dllmain_uninit` 相当。`0x18019e3f0` で `[0x1801f1550]` を減らし `0x1801f1508 == 2` なら `f_18019e00c`…を呼ぶ）か、`f_18019e03c` ← `f_18019e450`）を通る。**ATTACH の途中で `f_18019e3cc` に入るのは `0x18019e501`（`ATTACH` が失敗したときの巻き戻し: `0x18019e4e5 cmp edi,1; jne; test eax,eax; jne`）**。`TlsFree` を引く 2 つの関数は**「`FlsFree` を `GetProcAddress` で引けなければ `TlsFree` に落とす」CRT の薄いラッパー**（`f_1801a1dd0`、`f_1801a9a0c`。`"FlsFree"` の文字列 `0x1801b8498` を渡す `call 0x1801a1c38`。`rax == 0` のとき `jmp [TlsFree]`、非 0 なら `jmp [0x1801b7308]`（`__guard_dispatch_icall`）で `FlsFree` を呼ぶ）。**この環境では `FlsFree` に行があり（決定 0078）、`GetProcAddress("FlsFree")` が非 NULL を返すなら `TlsFree` の側には入らない**（読み。実行していない） |
| WaitForSingleObjectEx | 1 | **1** | 1 | A で届く（`f_18019dd04` ← `f_18019dc58`（C++ の関数内 static のガード `__Init_thread_header` 相当）← `f_18000abb0`（CPU 情報の初期化。`.xc` 初期化子が `0x180199dce` で呼ぶ）←`f_180199d90`）。**ただし呼ばれるのは「別のスレッドが初期化中（`[ガード] == -1`）」のときの `Sleep` 代替（`mov ecx,0x64; call f_18019dd04` = `0x18019dc78`）。`f_18019dd04` の中身は `[0x1801f14f8] != 0`（`SleepConditionVariableCS` を引けた）ならそれを呼び、0 なら `WaitForSingleObjectEx(event, ms, FALSE)`**（`ms = 0x64`）。`0x18019db00`（`.xi` 項目 0）が `GetModuleHandleW("api-ms-win-core-synch-l1-2-0.dll")` + `GetProcAddress("SleepConditionVariableCS")` を引き、**この環境では NULL になる**（行なし。§4.2 の末尾）ので、`CreateEventW` の代替イベント `[0x1801f14c8]` に落ちる。そのイベントは**手動リセット（`edx = 1` = `lea edx,[r9+1]`、`r9 = 0`）・非シグナル・名前なし**。ただし**ガード `[0x1802f4b88]` が `-1`（別スレッドが初期化中）になるのは、単一スレッドの通常経路では起きず `0x18019dc78` に入らない**（読み。実行していない）。**裏付け（実測、既存のログの読み。この調査は実行していない）**: 決定 0091 の読み取り gdb のログ `lift/out-gmh/run/m3-ro1-imports.txt` の #1674（`GetModuleHandleW("api-ms-win-core-synch-l1-2-0.dll")` が `0x7ff8001b0000`）、#1677 **`fle_imp_CreateEventW rcx=0x0 rdx=0x1 r8=0x0 r9=0x0 ret=0x489db95`**（= `.xi` 項目 0 の `0x18019db8f` の次の命令 `0x489db95` に戻る。**手動リセット（`edx=1`）、非シグナル、無名**。上の「代替イベント」のこと）。#1698〜#1699 は `GetSystemInfo` ×2（`ret=0x470acb5`、`0x4899f09`）で、`f_18000abb0` が通っていることを示す。この列に **`fle_imp_WaitForSingleObject` の呼び出しは 1 つも無い**（`grep -c WaitForSingleObject` が 0。ブレークポイントは 263 個の `fle_imp_*` 全部に付いており、`fle_imp_WaitForSingleObject` も含む。実測。ATTACH の途中、停止の前の 1,703 回まで）。**なお `WaitForSingleObjectEx` は応答表に行が無く `fle_imp_*` が無いので、このログでは見えない**（呼ばれれば `fle_unsupported` で止まる。停止は `0x489cab9` だったので、停止までに `WaitForSingleObjectEx` の呼び出し点には到達していない）。**ATTACH の実行では、`f_18019dd04` の `WaitForSingleObjectEx` は呼ばれていない**（実測の裏付け）。**別の論点（読みと既存ログ）**: `f_18000abb0` の入口（`0x18000abc2`〜`0x18000abdf`）は `mov rax,gs:[0x58]; mov ecx,[_tls_index 0x1801f1560]; mov rcx,[rax+rcx*8]; mov eax,[rdx+rcx]` で暗黙の TLS を読み、ガード `[0x1802f4b88]` と比べる（feasibility §6、B6）。この関数は上のログの #1698〜#1699（`GetSystemInfo` ×2）を呼ぶので**実行で通っている**（実測。値は読んでいない）。 |
| GetLargePageMinimum | 3 | 0 | **0** | **ATTACH から届かない**（A でも B でも 0。既存文書と一致。呼び出し元 `f_18000e4d0`、`f_180018250` は `VirtualAlloc` ラッパー） |

**まとめ（読み）**:

1. **今回の停止（`f_18019caa0` の `AcquireSRWLockExclusive`）は ATTACH で実行される**（実測）。同じ初期化子 `f_180199d90` が `0x1802f4750` から `0x1802f47c8` までに 15 個の関数を書き、`0x18019a1d2: call r8` で表の `[+0x20]`（= `f_18019caa0`）を呼ぶ。**この 1 回の呼び出しでは `[rsi+0x30]`（`rcx + 0x30`）の SRWLOCK を Acquire し、`[rdi]` = `rsp+0xa0` に積んだ値（`[0x1801f3128]` = アロケーション粒度から計算した値）を検査し、`[rsi+0x40]` に書き、Release で閉じる**（§4.2）。
2. **`f_18019cfa0` の鎖（`CreateThread` 1、`ExitThread` 2、`FreeLibraryAndExitThread` 1 に届く）は、`.xc` の初期化子が `0x1802f47c0` に書く表の 15 番目の関数**で、`.xc` の直線上では**呼ばれず、表に置かれるだけ**（`0x18019a11e: lea rax,[0x18019cfa0]; mov [0x1802f47c0],rax` は書き込みのみ）。呼ぶのは後で（描画の初期化）と読める。**ATTACH の実行では `CreateThread` に届かない読み**（直接 call だけの A では 0）。
3. **`.xc` の初期化子の末尾の直接 call 5 つ**（`0x18019af0d: call 0x1800a9eb0`、`0x18019b045: call 0x18006b6c0`、`0x18019b07d: call 0x180147870`、`0x18019b082: call 0x180147cf0`、`0x18019b087: call 0x18006ed70`。`dis-init2.txt`）**は、自身は SRW・条件変数・スレッドの API を呼ばない**（`f_180147870`、`f_180147cf0`、`f_18006b6c0`、`f_18006ed70` の `call` は 0 個。`f_1800a9eb0` の `call` は `0x18000fbc0` と `0x18000abb0`（CPU 情報）の 2 つだけ。objdump の `call` の数え）。これらは **`lea` で関数のアドレスを取り、vtable やオブジェクトの表に書き込む**関数に見える（`f_1800a9eb0` は `0x1800a9a90`、`0x1800a9ad0`、`0x1800a9bb0` を `lea`。`f_180147870` は 40 個ほどの `0x18013…` の関数を `lea`）。**したがって B で届く `f_1800a9ad0`、`f_1800a9bb0`、`f_18013d7b0` などは「ATTACH で表に置かれる」だけで、ATTACH の直線上で呼ばれるとは言えない**。**B の 74 か所は「アドレスが取られる」の上界で、「実行される」ではない**（読み）。実行されるのが確かなのは、(i) 実測で停止した `f_18019caa0`（`0x18019a1d2: call r8`）、(ii) `.xi` / DllMain の直接 call で届く A の 8 か所（`IsProcessorFeaturePresent` 5、`TlsFree` 2、`WaitForSingleObjectEx` 1。ただし上の表のとおり、通常は呼ばれない分岐の中）、だけ。
4. **`.xc` の初期化子が `0x18019a334` から `atexit` 風の表に書くもの**（`0x18019bea0`、`0x18019bec0`、`0x18019bf00`、`0x18019d080`、`0x18019d090`。`[0x1801f31a8]`、`[0x1801f31f0]`、`[0x1801f3238]` の配列に書く）は、`f_18019bec0`（`AcquireSRWLockExclusive` + `ReleaseSRWLockExclusive`、`rcx = 0x1802f4700`）、`f_18019bf00`（同じ）、`f_18019d080` → `f_18019c920`（`AcquireSRWLockExclusive`、`SleepConditionVariableSRW`）。**終了時（`DLL_PROCESS_DETACH` の `atexit` 経路）に呼ばれる登録と読める**。ATTACH では呼ばれない（読み。実行していない）。
5. **今回の停止（`f_18019caa0`）の中身**（§4.2）が、ATTACH で実行されるのが確実な唯一の SRW 経路。**SRW・条件変数・スレッドのうち、ATTACH の実行で確実に実行される呼び出し点は、この `f_18019caa0` の `AcquireSRWLockExclusive`（`0x18019cab9`）と、続く `ReleaseSRWLockExclusive`（`0x18019cae3` または `0x18019cb06` のどちらか 1 回）だけ**（読み。実行は `AcquireSRWLockExclusive` まで）。その後に実行が続くと、`.xc` が呼ぶ `f_18019caa0` の戻りの後の初期化（`0x18019a1d6` 以降）の中身次第で他にも届く可能性がある（未確認）。

（上の読みは、**どの関数が直接 call の先にあるか**と **`lea` で取られる関数**を根拠にしたもの。間接 call（`call r8`、vtable）は `f_18019caa0` 1 つしか確認していない。**実行で確かめる方が確かだが、この作業は実行しない**。）

### 4.2 引数、ロックの置き場、戻り値の使い方（**読み**。逆アセンブルの読みで、実行していない。出力 `locks-orig.txt`、`refs-static-locks.txt`、`dis-pool.txt`、`dis-init.txt`）

**SRWLOCK の置き場**（`imp14_locks.py`。65 サイトの `rcx`（`SleepConditionVariableSRW` は `rcx`=条件変数、`rdx`=ロック）を直前 12 命令の範囲で辿った）:

| 置き場 | サイト | 読み |
|---|---|---|
| **静的**（`.data` の BSS 側 `0x1802f4700`） | `f_180146040`、`f_180194120`、`f_180194286`、`f_18019bec0`、`f_18019bf00` の Acquire 7 + Release 7（`lea rcx,[rip+d]` で取る） | グローバルの 1 本のロック。`.xc` の初期化子が `0x18019a274: mov qword ptr [0x1802f4700], rdi`（`rdi` = 0）で**ゼロを書く**（`InitializeSRWLock` は import されていない: `imports-blend2d.txt`。Blend2D は `InitializeSRWLock`、`InitializeConditionVariable`、`TryAcquireSRW*` を **1 つも import しない**）。`.text` で `[0x1802f4700, +8)` を `lea` 以外で参照するのはこの 1 つの書き込みだけ（`refs-static-locks.txt`） |
| **スレッドプールの静的オブジェクト `0x1802f47d0` の `this+0x30`**（= `0x1802f4800`。条件変数は `this+0x38`） | `f_18019caa0`（今回の停止）、`f_18019cb80`、`f_18019cfa0`、`f_18019d020`、`f_18019cb20`（`rbx = rdx` の `+0x30`）、`f_18019c920`（`rdi+0x30`）。これらは `.xc` が `0x1802f4790` の vtable に書く関数（`[+0x20]` = `f_18019caa0`、`[+0x28]` = `f_18019cb80`、`[+0x30]` = `f_18019cfa0`、`[+0x38]` = `f_18019d020`）と、`atexit` 風の表に登録される `f_18019d080` → `f_18019c920` | **`this` は静的な `0x1802f47d0` のように読める**（`lea rcx,[0x1802f47d0]` を渡す `0x18019a1cb`（今回の呼び出し）、`0x18019d080`、`0x18019d0a6`）。`.xc` の初期化子が `0x18019a186: mov [0x1802f4800], rdi`（`rdi` = 0）で**ゼロを書く**。この 8 バイトを `lea` 以外で参照するのはその書き込みだけ（`refs-static-locks.txt`） |
| **スレッド 1 個のオブジェクト（ヒープ 0x100 バイト）の `+0x80`（ロック）、`+0x88`（条件変数）** | `f_18019c4d0`（タスク投入）、`f_18019c560`（停止）、`f_18019c5e0`（スレッドのループ） | `f_18019ccf0` が `0x18019ce3d`、`0x18019ce44` で `[rbx+0x80]`、`[rbx+0x88]` に**ゼロ**（`r14` = 0）を書く |
| 他のオブジェクトのメンバ | `f_1800a9ad0`、`f_1800a9bb0`（`this+0x58`。Shared 3 + Release Shared 6 + 排他 1 + 排他 Release 1）、`f_180199670`/`f_180199d00`（`+0xc0` ロック、`+0xc8`、`+0xd0` 条件変数）、`f_18013d7b0`（`+0x140` ロック、`+0x150` 条件変数） | 構造体の種類は未確認。ゼロ初期化の書き込みは読んでいない |

**ゲストが SRWLOCK を直接読み書きするか**（確認の範囲を限る）: (1) 静的な 2 本（`0x1802f4700`、`0x1802f4800`）は、`.text` の rip 相対の参照を全部拾い（`refs-static-locks.txt`）、**`lea`（アドレスを取るだけ）と、`.xc` が 1 回だけ書くゼロ（`SRWLOCK_INIT` の静的な代入に当たる書き込み）以外は無い**（実測）。(2) ヒープのオブジェクトのロックは、構造体のメンバのアクセスを型なしでは分けられず、**確認していない**（未確認）。初期化はゼロの書き込み（`movups xmm0` = 0、`mov [rbx+0x80], r14` = 0）。(3) したがって「ゲストが SRWLOCK の中身を読む」ことは**静的な 2 本では無い**が、ヒープのロックについては**言えない**。

**`WaitOnAddress` の有無が SRW/条件変数を使うかを決める**（読み。`dis-pool.txt`、`dis-init.txt`）: `.xc` の初期化子は `GetProcAddress` で `WaitOnAddress`、`WakeByAddressSingle`、`WakeByAddressAll` を引き、**3 つとも非 NULL のときだけ `[0x1801f31a0] = 1`、それ以外は 0**（決定 0091 の調査 §2.3、§7.3）。`f_18019ccf0`（スレッドの作成。`CreateThread` を呼ぶ `f_1801a4888` の呼び元）は `mov r14d,[0x1801f31a0]` を読み、**非 0 なら 0xc0 バイトのオブジェクト（vtable `0x1802f4750`、スレッドの本体 `f_18019c7f0` は `lock cmpxchg` と `call [0x1801f3280]`（= `WaitOnAddress`）で待つ）、0 なら 0x100 バイトのオブジェクト（vtable `0x1802f4770`、本体 `f_18019c5e0` は `AcquireSRWLockExclusive` + `SleepConditionVariableSRW(INFINITE)`、投入 `f_18019c4d0` と停止 `f_18019c560` は `WakeConditionVariable`）を作る**。**この環境は決定 0081 により `GetProcAddress(N=27, "WaitOnAddress")` が NULL を返すので（実測: 決定 0091 の測定 1701〜1703）`[0x1801f31a0]` = 0 → SRW + 条件変数の側**（読み）。つまり、**`WaitOnAddress` を NULL にしている限り、スレッドプールのスレッドは SRW/条件変数版になる**（`WaitOnAddress` を答える決定をすれば `WaitOnAddress` 版になり、SRW/条件変数は使わなくなる。**この二択は「SRW を実装する」と「`WaitOnAddress` を実装する」の関係を作る**。ただし**スレッドプールの `.xc` の呼び出し（`f_18019caa0`）の SRW はどちらでも残る**: プールの `this+0x30` のロックは変数に関係なく使われる）。

**各 API の引数と戻り値の使い方**（呼び出し点の読み）:

| API | 引数に何が渡るか | ゲストが戻り値を使うか |
|---|---|---|
| AcquireSRWLockExclusive / ReleaseSRWLockExclusive | 上の表のとおり。**全サイトが「同じスレッドが Acquire → 同じ関数で Release」の対**（`f_18019caa0` は Acquire 1 回に対し Release 2 回: 分岐の 2 つの出口の各 1 回）。Release を `jmp [IAT]`（tail jump）で呼ぶ 3 か所（`f_18019bec0`、`f_18019bf00`、`f_18019d020`） | VOID。使わない |
| AcquireSRWLockShared / ReleaseSRWLockShared | `this+0x58`（`f_1800a9ad0`、`f_1800a9bb0`）。`f_1800a9ad0` は Acquire Shared 1 回に対し Release Shared 2 回（分岐の出口）、`f_1800a9bb0` は Acquire Shared 2 回、Release Shared 4 回、排他 1 組（共有を Release してから排他を Acquire する形に見える: `0x1800a9c31` の Release Shared の後に `0x1800a9c70` の Acquire Exclusive。**共有 → 排他のアップグレードではなく一度手放す**。読み） | VOID。使わない |
| WakeConditionVariable | `this+0x88`（スレッドの cv）、`this+0x38`（プールの cv）、`this+0xd0`（`f_180199d00`）。**Wake の前に同じロックを Acquire している**（`f_18019c4d0`、`f_18019c560`、`f_18019cb20`、`f_180199d00` とも。Learn は「inside or outside the lock」と書くので両方可） | VOID |
| WakeAllConditionVariable | `this+0xc8`（`f_180199670`、同じロック `+0xc0` の中） | VOID |
| SleepConditionVariableSRW | 4 か所とも **`Flags` = 0（`xor r9d,r9d`。排他）**。**`dwMilliseconds`**: `f_18019c5e0`（スレッドのループ）、`f_180199670`、`f_18013d7b0` の 3 か所は **INFINITE（`mov r8d,0xffffffff`）**。`f_18019c920`（`f_18019d080` から `rcx = 0x1802f47d0` で呼ばれる。`atexit` 風の登録）の 1 か所は**有限**（`r8d = eax`。`eax` は `[rdi+0x24]` に 1000 を掛けて 0xcccccccccccccccd の乗算で 5 で割り、`shr 0x2`、さらに `0x624dd2f1a9fbe77` の乗算を経て `shr 0x9` した値を `0xffffffff` で上限にした値。**数値の意味（ミリ秒でいくつか）は算術を追っていないので「有限・計算値」とだけ書く。先の「20 ms」という値は推定で、確認していない**。`esi = 5`、ループ）。ただし `f_18019c920` は `[rdi+0x18]`（スレッド数）が 0 なら Sleep に入らない（`0x18019c93e: test eax,eax; je 0x18019c9fb`）。**3 か所は「条件（フラグ）が真になるまで `while` で待つ」形**（`f_18019c5e0`: `test dil,0xa; jne` で抜ける。`f_180199670`: `cmp [rdi+0x40],0; jne` で繰り返す。`f_18013d7b0`: `cmp [r15+0xc4],0; je` で抜ける） | `f_18019c9d6` だけ使う（`test eax,eax; jne 0x18019c9f1`: 非 0 = 起床 → Release して抜ける、0 = タイムアウト → Release して次の周回）。他の 3 か所は**戻り値を見ず**（直後に述語を読み直す）。**どの `Sleep` も、条件（述語）が既に真なら呼ばれない**形（`while (!pred) Sleep`） |
| CreateThread | **`f_1801a4888` は CRT の `_beginthreadex` に当たる関数**（逆アセンブルの読み。`f_1801a4828` で `_tiddata` 相当のブロックを作り、そのブロックを `lpParameter`（`r9 = rbx`）に渡す）。`lpThreadAttributes` = `rcx`（`f_18019ccf0` は 0 = `xor ecx,ecx`）、`dwStackSize` = `edx`（`f_18019ccf0` は 0）、**`lpStartAddress` = `f_1801a475c`**（`lea r8,[rip-0x183]` = `0x1801a475c`。スレッド開始のラッパー。`_beginthreadex` の `thread_start`）、`dwCreationFlags` = `[rsp+0x60]`（`f_18019ccf0` が `[rsp+0x20] = eax` に積む値 = `[rsi+0x40]`（プールのスタックサイズ設定）が 0 なら 0、非 0 なら `0x10000`（`STACK_SIZE_PARAM_IS_A_RESERVATION`）。`0x18019ce5c: mov eax,0x10000; cmove eax,r14d`）、`lpThreadId` = `rsp+0x50` | `f_1801a4888` が `rax` を `rdi` に保存し `test rax,rax; jne`。NULL なら `GetLastError` を `f_1801a62cc`（`_dosmaperr` 相当）へ渡して**戻り値 0**（`0x1801a4908: xor edi,edi`）。成功なら `lpThreadId` の値を呼び元の引数 `[rcx]` に書き戻して**ハンドルを返す**。`f_18019ccf0` は `0x18019ce73: cmp rax,-1; je` で比べる（`_beginthreadex` の失敗値。**`f_1801a4888` は失敗時に 0 を返すので `-1` にはならない。比較は常に偽になり、NULL の失敗は呼び出し元で別に扱われる形に見える**。読み。未確認） |
| ExitThread / FreeLibraryAndExitThread | `ExitThread(eax)`（`f_1801a475c`: `rcx==0` なら `GetLastError` の戻り値を `ecx` に）。`f_1801a47c4` は `[rbx+0x18]`（モジュールハンドル）が有効なら `FreeLibraryAndExitThread(h, edi)`、無効なら `ExitThread(edi)` | **戻らない**（直後が `int3`。ラッパーのスレッドの終端） |
| GetFileSizeEx | `GetFileSizeEx(hFile = rbx 由来、&size = rsp+0x30)`（`0x1801a8ff8`。直前に `SetFilePointerEx` 呼び出し `0x1801a8fe6`（`rdx = 0`、`r8 = rsp+0x38`、`r9d` なし。**`SetFilePointerEx` は行ありの名前付き停止**）の後） | `test eax,eax` で分岐（0 なら失敗経路 `0x1801a9011`） |
| IsProcessorFeaturePresent | **5 か所すべて `ecx = 0x17`（PF_FASTFAIL_AVAILABLE）** | `test eax,eax; je`（0 なら `int 0x29` を飛ばす。1 なら `int 0x29` fail fast。決定 0078 §1 の形）。呼ばれるのは CRT の fail-fast（`__scrt_fastfail` 相当）の中 |
| OutputDebugStringA | `0x1800aacca`: `rcx` = SSO 文字列のバッファ（`cmp byte [rsp+0x58],0x1f; cmovae rbx,[rsp+0x70]`）= **ログ文字列**。`0x18019b1b5`: `rcx = rsp+0x30`（`f_1801a3be0` が 0x400 バイトに書いたエラー文字列。`mov r8d,0x400`） | VOID |
| TlsFree | `ecx` = FlsFree ラッパーの引数（TLS/FLS インデックス）。`jmp [IAT]` | tail jump なので戻り値は呼び出し元へ。呼び元 `f_18019fca0` は `eax` を見ない（`or [0x1801f0030], -1; mov al,1`）。呼ばれるのは `GetProcAddress("FlsFree")` が NULL のときの代替（上の 4.1） |
| WaitForSingleObjectEx | **`hHandle` = `[0x1801f14c8]`（`.xi` 項目 0 `f_18019db00` が `CreateEventW(NULL, TRUE(edx=1), FALSE, NULL)` で作ったイベント。手動リセット・非シグナル・名前なし）、`dwMilliseconds` = `ebx`（呼び元は 0x64 = 100）、`bAlertable` = 0（`xor r8d,r8d`）** | 戻り値を見ない（直後は `lea rcx,[0x1801f14d0]` で CS の Enter へ戻る） |
| GetLargePageMinimum | 引数なし | `test rax,rax; je`（0 なら大ページを使わない）。**ATTACH では届かない** |

**`f_18019caa0`（今回の停止）の中身**（読み。`dis-pool.txt` の `0x18019caa0`〜）: `this+0x30` を `AcquireSRWLockExclusive`、引数の `[rdx]`（`.xc` が `0x18019a13a` で `[0x1801f3128]` を `rsp+0xa0` に積んだ値）を `r8d` に読み、`r8d == 0` か、`r8d >= [0x1801f3128]` かつ `r8d % [0x1801f3130] == 0` のときに `[this+0x40] = r8d` を書いて `ReleaseSRWLockExclusive`、戻り値 0。そうでなければ `ReleaseSRWLockExclusive`、戻り値 `0x10001`。`[0x1801f3128]`、`[0x1801f3130]` は `.xc` が `GetSystemInfo` の結果から計算する（`dwAllocationGranularity` = 65,536（この環境の応答）から `0x20000` と 65,536。`0x180199f0d`〜`0x180199f31`）。**スレッドのスタックサイズの設定（最小値 0x20000）に読める**。この環境の応答では引数 0x20000 は条件を満たし、`[this+0x40] = 0x20000`、戻り値 0 になる（読み。`GetSystemInfo` の `dwAllocationGranularity` は決定 0017 の 65,536）。

## 5. Learn のページの規則

取得: 2026-10-25 に `curl`（HTTP 200、19 ページ。`lift/out-imp14/learn/*.html`、本文抽出 `learn/*.txt`。抽出は `lift/out-imp14/learn_text.py`）。**実測（取得）**。ページの `updated_at` はメタから。引用は英語の原文（訳さない）。**「書かれていない」はページ本文に無いという意味**で、実機が何もしないという意味ではない。

### 5.1 SRW ロック全体と 4 つの API

URL（すべて `https://learn.microsoft.com/en-us/windows/win32/`）:

- 解説: `sync/slim-reader-writer--srw--locks`（`updated_at` 2025-04-15）
- `api/synchapi/nf-synchapi-acquiresrwlockexclusive`（2025-08-27）、`…-acquiresrwlockshared`、`…-releasesrwlockexclusive`、`…-releasesrwlockshared`、`…-initializesrwlock`（いずれも 2025-08-27）

ページが**書くこと**（引用）:

| 項目 | 引用 |
|---|---|
| 位置づけ | 「Slim reader/writer (SRW) locks enable the threads of a single process to access shared resources; they are optimized for speed and occupy very little memory.」 |
| 共有モード | 「Shared mode, which grants shared read-only access to multiple reader threads…」「Shared mode SRW locks should not be acquired recursively as this can lead to deadlocks when combined with exclusive acquisition.」 |
| **排他は再帰不可** | 「Exclusive mode SRW locks cannot be acquired recursively. If a thread tries to acquire a lock that it already holds, that attempt will fail (for TryAcquireSRWLockExclusive) or deadlock (for AcquireSRWLockExclusive)」 |
| 排他の意味 | 「When the lock has been acquired in exclusive mode, no other thread can access the shared resource until the writer releases the lock.」 |
| 順序 | 「There is no guarantee about the order in which threads that request ownership will be granted ownership; SRW locks are neither fair nor FIFO.」 |
| サイズ・状態 | 「An SRW lock is the size of a pointer. … very little state information can be stored, so SRW locks do not detect incorrect recursive use in shared mode. In addition, a thread that owns an SRW lock in shared mode cannot upgrade its ownership of the lock to exclusive mode.」 |
| **初期化（静的）** | 「The caller must allocate an SRWLOCK structure and initialize it by either calling InitializeSRWLock (to initialize the structure dynamically) or assign the constant SRWLOCK_INIT to the structure variable (to initialize the structure statically).」 |
| 不透明 | InitializeSRWLock: 「An SRW lock cannot be moved or copied while in use. The process must not modify the object, and must instead treat it as logically opaque. Only use the SRW functions to manage SRW locks.」 |
| 初期状態 | InitializeSRWLock: 「An unlocked SRW lock with no waiting threads is in its initial state and can be copied, moved, and forgotten without being explicitly destroyed.」 |
| 戻り値 | AcquireSRWLockExclusive/Shared、ReleaseSRWLockExclusive/Shared、InitializeSRWLock はすべて `VOID`（「Return value: None」） |
| **Release のスレッド** | Release の Remarks: 「The SRW lock must be released by the same thread that acquired it. You can use Application Verifier to help verify that your program uses SRW locks correctly (enable Locks checker from Basic group).」 |

ページが**書かないこと**（本文に無いことを確認した。`grep` で `undefined`、`unlocked`、`not held`、`does not own` を検索し、SRW の 4 つの API のページに該当が無いことを確認）:

1. **ロックを持たずに Release した場合の動作**: Release の 2 つのページは「同じスレッドが取得したロックを解放しなければならない（must）」とだけ書き、**違反した場合の結果（未定義か、エラーか、無視か）を書かない**。`undefined` の語は SleepConditionVariableSRW のページに 1 回出るだけ（下の 5.2）。
2. **取られているロックへの Acquire（異なるスレッド）がブロックする**ことは、解説の「no other thread can access the shared resource until the writer releases the lock」から読めるが、Acquire のページ自身は動作（ブロックする）を明記していない（「Acquires a slim reader/writer (SRW) lock in exclusive mode.」だけ）。
3. 共有の Acquire を同じスレッドが 2 回行う（再帰）の結果: 「should not … can lead to deadlocks when combined with exclusive acquisition」と書き、排他との組み合わせでないときの動作は書かない。
4. SRWLOCK 構造体の内部のビット配置（`SRWLOCK_INIT` の値 0 も、この解説とこの 5 ページには**書かれていない**。「Ptr」メンバと `SRWLOCK_INIT` という名前があるだけ。**数値 0 は非公式の知識**で、今回の呼び出し点の「静的初期化が 0」という読みは、IDA/Windows SDK ヘッダー（`RTL_SRWLOCK_INIT {0}`）に基づく非公式の知識。この調査では SDK ヘッダーを取得していない）。
5. 排他を持つスレッドが、同じロックに共有で Acquire する場合の動作。
6. ロックの所有者の記録の有無（「detect incorrect recursive use in shared mode」できないという記述はあるが、所有スレッドを持つとも持たないとも書かない）。

### 5.2 条件変数

URL: `sync/condition-variables`（`updated_at` 2025-03-11）、`api/synchapi/nf-synchapi-sleepconditionvariablesrw`（2025-10-31）、`…-wakeconditionvariable`、`…-wakeallconditionvariable`（2025-08-27）。

| 項目 | 引用 |
|---|---|
| 解説 | 「Condition variables are synchronization primitives that enable threads to wait until a particular condition occurs. Condition variables are user-mode objects that cannot be shared across processes.」「Condition variables support operations that "wake one" or "wake all" waiting threads. After a thread is woken, it re-acquires the lock it released when the thread entered the sleeping state.」 |
| 初期化 | 「the caller must allocate a CONDITION_VARIABLE structure and initialize it by either calling InitializeConditionVariable … or assign the constant CONDITION_VARIABLE_INIT to the structure variable (to initialize the structure statically).」 |
| 偽の起床 | 「Condition variables are subject to spurious wakeups (those not associated with an explicit wake) and stolen wakeups … Therefore, you should recheck a predicate (typically in a while loop) after a sleep operation returns.」 |
| Wake の位置 | 「You can wake other threads using WakeConditionVariable or WakeAllConditionVariable either inside or outside the lock associated with the condition variable.」 |
| Wake / WakeAll | 「The WakeAllConditionVariable wakes all waiting threads while the WakeConditionVariable wakes only a single thread. Waking one thread is similar to setting an auto-reset event, while waking all threads is similar to pulsing a manual reset event but more reliable (see PulseEvent for details).」（2 つのページの Remarks に同文）。戻り値は `VOID`。 |
| Sleep のパラメータ | `dwMilliseconds`: 「The time-out interval, in milliseconds. The function returns if the interval elapses. If dwMilliseconds is zero, the function tests the states of the specified objects and returns immediately. If dwMilliseconds is INFINITE, the function's time-out interval never elapses.」`SRWLock`: 「A pointer to the lock. This lock must be held in the manner specified by the Flags parameter.」`Flags`: 「If this parameter is CONDITION_VARIABLE_LOCKMODE_SHARED, the SRW lock is in shared mode. Otherwise, the lock is in exclusive mode.」 |
| Sleep の戻り値 | 「If the function succeeds, the return value is nonzero. If the function fails, the return value is 0. To get extended error information, call GetLastError. If the timeout expires the function returns FALSE and GetLastError returns ERROR_TIMEOUT.」 |
| Sleep の未定義 | Remarks: 「If the lock is unlocked when this function is called, the function behavior is undefined.」 |
| Sleep の起床 | 「The thread can be woken using the WakeConditionVariable or WakeAllConditionVariable function. After the thread is woken, it re-acquires the lock it released when the thread entered the sleeping state.」 |

ページが**書かないこと**:

1. **待っているスレッドがいないときの Wake / WakeAll の動作**（何もしない、と明記する文は**無い**。「wakes all waiting threads」「wakes only a single thread」と書くだけで、待機者がゼロのときは何も書かない。`SetEvent` のページは「待機者がいなければ信号のまま残る」と明記しているが、Wake は**対応する文が無い**。Wake を「auto-reset event を set する」に似ている、と書くが、**Wake の後に来た Sleep が即座に起きる（イベントのように信号が残る）とは書いていない**。解説の「Condition variables … wait until a particular condition occurs」と、SleepConditionVariableSRW の「if the timeout expires」の記述から、**Wake は待機者にだけ作用し状態を持ち越さない**と読むのが通例だが、**ページはそう明記しない**＝読み）。
2. `ERROR_TIMEOUT` の数値（1460）はページに書かれていない（System Error Codes ページの値。この調査では取得していない。**未確認**）。
3. `dwMilliseconds` が有限でタイムアウトするときのロックの再取得（「Sleep が戻るとき、ロックを再取得している」ことは「After the thread is woken, it re-acquires the lock」の文から読めるが、**タイムアウトで戻る場合にも再取得して戻る**とはページは書かない）。
4. `Flags` の値（`CONDITION_VARIABLE_LOCKMODE_SHARED` = 1）はページに数値が無い。
5. 「ロックが排他で保持されているが Flags が共有」のような**食い違いの結果**。
6. **条件変数の初期状態の検査**（`CONDITION_VARIABLE_INIT` の値 0 もページに数値が無い。非公式の知識）。

### 5.3 その他の API

| API（URL: `api/…`、`updated_at`） | ページが書くこと（引用） | ページが書かないこと |
|---|---|---|
| WaitForSingleObjectEx（`synchapi/nf-synchapi-waitforsingleobjectex`、2024-08-22） | `dwMilliseconds`: 「If a nonzero value is specified, the function waits until the object is signaled, an I/O completion routine or APC is queued, or the interval elapses. If dwMilliseconds is zero, the function does not enter a wait state if the criteria is not met; it always returns immediately. If dwMilliseconds is INFINITE, the function will return only when the object is signaled or an I/O completion routine or APC is queued.」`bAlertable`: 「If this parameter is TRUE and the thread is in the waiting state, the function returns when the system queues an I/O completion routine or APC, and the thread runs the routine or function. Otherwise, the function does not return, and the completion routine or APC function is not executed.」戻り値の表: WAIT_ABANDONED 0x80、WAIT_IO_COMPLETION 0xC0、WAIT_OBJECT_0 0、WAIT_TIMEOUT 0x102、WAIT_FAILED 0xFFFFFFFF。「The function modifies the state of some types of synchronization objects.」 | 無効ハンドルの last error の値。イベント以外の種類を待つ場合の動作は種類ごとに別ページ。`bAlertable` が TRUE で APC が無いときの扱い（待つ。APC キューの有無を単一スレッドでどう答えるか） |
| CreateThread（`processthreadsapi/nf-processthreadsapi-createthread`、2025-07-01） | `dwCreationFlags` の 0:「The thread runs immediately after creation.」CREATE_SUSPENDED 0x4:「The thread is created in a suspended state, and does not run until the ResumeThread function is called.」戻り値:「If the function succeeds, the return value is a handle to the new thread. If the function fails, the return value is NULL.」Remarks:「If the thread is created in a runnable state … the thread can start running before CreateThread returns」「During process startup and DLL initialization routines, new threads can be created, but they do not begin execution until DLL initialization is done for the process.」「Only one thread in a process can be in a DLL initialization or detach routine at a time.」「A thread in an executable that calls the C run-time library (CRT) should use the _beginthreadex and _endthreadex functions … If a thread created using CreateThread calls the CRT, the CRT may terminate the process in low-memory conditions.」「When a thread terminates, the thread object attains a signaled state」 | スレッドがいつ走るかのスケジュール（「can start running before CreateThread returns」とだけ）。**DllMain の中で CreateThread した場合、スレッドは DLL 初期化が終わるまで走らない**ことは書く（これが今回の ATTACH の中の `CreateThread` に効く）。スレッド ID の値、ハンドルの値 |
| ExitThread（`processthreadsapi/nf-processthreadsapi-exitthread`、2025-08-27） | 「When this function is called (either explicitly or by returning from a thread procedure), the current thread's stack is deallocated, all pending I/O … is canceled, and the thread terminates. The entry-point function of all attached dynamic-link libraries (DLLs) is invoked with a value indicating that the thread is detaching from the DLL.」「If the thread is the last thread in the process when this function is called, the thread's process is also terminated.」 | 戻らない（「Return value: None」で、`int3` が後続する呼び出し点の形と整合するが、ページは「戻らない」と明記しない。**読み**） |
| FreeLibraryAndExitThread（`libloaderapi/nf-libloaderapi-freelibraryandexitthread`、2025-08-27） | 「allows threads that are executing within a DLL to safely free the DLL in which they are executing and terminate themselves.」 | 同上（戻らない、の明記なし） |
| GetFileSizeEx（`fileapi/nf-fileapi-getfilesizeex`、2025-07-01） | 「If the function succeeds, the return value is nonzero. If the function fails, the return value is zero. To get extended error information, call GetLastError.」`lpFileSize`: 「A pointer to a LARGE_INTEGER structure that receives the file size, in bytes.」 | 失敗する場合の last error の値、ハンドルの種類（ファイル以外）の扱い |
| GetLargePageMinimum（`memoryapi/nf-memoryapi-getlargepageminimum`、2025-07-01） | 「If the processor supports large pages, the return value is the minimum size of a large page. If the processor does not support large pages, the return value is zero.」「The minimum large page size varies, but it is typically 2 MB or greater.」 | 「このエミュレートされたプロセッサが大ページを持つか」（エミュレートする CPUID に結びつく。決定 0009、0047 の CPUID は葉 7 も拡張葉も無い。**ページから導けない**） |
| OutputDebugStringA（`debugapi/nf-debugapi-outputdebugstringa`、2025-08-27） | 「If the application does not have a debugger and the system debugger is not active, OutputDebugString does nothing.」「Return value: None」 | 「このエミュレートされた環境にデバッガーがあるか」（既存の `OutputDebugStringW` は「デバッガーなし」と決めている。fle_proc.c の 199〜205 行目、決定 0020 系） |
| IsProcessorFeaturePresent（`processthreadsapi/nf-processthreadsapi-isprocessorfeaturepresent`、2025-12-18） | 「The processor feature to be tested. This parameter can be one of the following values.」（PF_FASTFAIL_AVAILABLE 23「_fastfail() is available.」ほか）。「PF_PAE_ENABLED 9 … All x64 processors always return a nonzero value for this feature.」 | **各番号の値**（決定 0078 が既に整理した「値はページに無い」）。エミュレートする環境の答え |
| TlsFree（`processthreadsapi/nf-processthreadsapi-tlsfree`、2025-07-01） | 「The TLS index that was allocated by the TlsAlloc function.」「If the function succeeds, the return value is nonzero. If the function fails, the return value is zero.」「The TlsFree function does not free memory blocks whose addresses have been stored in the TLS slots associated with the TLS index. It is expected that DLLs call this function (if at all) only during DLL_PROCESS_DETACH.」 | 失敗する条件と last error の値（決定 0078 が 87 を選択した。仮） |

`WakeAllConditionVariable` と `GetLargePageMinimum` は ATTACH では静的に届かない（§4.2 の到達の読み）。

### 5.4 出典の区別

- 上の表は**すべて Microsoft Learn（公式）**。非公式の出典は**使っていない**。
- 「`SRWLOCK_INIT` の値が 0」「Wake が待機者無しのとき状態を持ち越さない」「SRWLOCK が実質ポインタ 1 個の値で、保持中は状態ビットが立つ」は、Learn のこの 19 ページに**書かれていない**。これらを使う設計は「Learn が書かない動作を作る」仮の決定に当たる（§7）。

## 6. 単独スレッド（決定 0050 / 案 C）の下での意味（**整理。読みと Learn の引用。決めない**）

前提（読んだ文書。§3 に行番号）: 決定 0050（イベントは単一スレッドの状態機械。INFINITE または有限で未シグナルの待ちは abort）、決定 0017（`EnterCriticalSection` は所有中の再入を許し、`Leave` は回数を減らす。初期化前の Enter、持たない Leave、所有中の Delete は abort。**別スレッドからの呼び出しは abort**）、決定 0080 D9（`DLL_THREAD_*` を受ける経路は作らない）、`sync-thread-survey.md` §3.4（案 C: スレッド生成に届いたら名前付き abort）。

### 6.1 SRW ロック

「ブロックは単一スレッドでは永遠に起こらない」という前提のもとで、Learn が書く範囲で整理する。

| 状況 | Learn が書くこと | 単一スレッドの下で素直に導ける答え | 書かない部分（作ると仮） |
|---|---|---|---|
| **空のロックへの `AcquireSRWLockExclusive`** | 「Acquires a slim reader/writer (SRW) lock in exclusive mode.」VOID | ロックが取られ、戻る。SRWLOCK の中身は不透明（「treat it as logically opaque」）なので、状態は**ゲストのメモリではなく runtime の表**（アドレスをキー。決定 0017 のクリティカルセクションと同じ流儀）に持つのが「ゲストが読み書きしない」前提と整合する | **静的初期化（`SRWLOCK_INIT`、ゲストが `0` を置いた領域）の登録**: Learn は `SRWLOCK_INIT` の値も、`InitializeSRWLock` を呼ばない使い方での登録も書かない。**クリティカルセクションの流儀（`Initialize` で表に登録、未登録は abort）は Blend2D の静的な SRWLOCK には使えない**（`InitializeSRWLock` を import しない: §4.2）。「未登録のアドレスへの最初の Acquire = 初期状態（空）」と**扱う仮の決定**が要る |
| **排他で取られているロックへの `AcquireSRWLockExclusive`（同じスレッド）** | 「Exclusive mode SRW locks cannot be acquired recursively. If a thread tries to acquire a lock that it already holds, that attempt will fail (for TryAcquireSRWLockExclusive) or deadlock (for AcquireSRWLockExclusive)」 | **Learn は「deadlock」と書く**。単一スレッドでは永遠に戻らない（他に起こす者がいない）。**名前付き停止にするのは Learn の記述と整合し、fail-closed である**（`Sleep(INFINITE)` と INFINITE の未シグナル待ちを abort にした決定 0017、0050 と同じ整理）。戻る実装（無視して通す）は Learn の「deadlock」に反する | なし |
| **共有で取られているロックへの `AcquireSRWLockShared`（同じスレッド）** | 「Shared mode SRW locks should not be acquired recursively as this can lead to deadlocks when combined with exclusive acquisition.」 | **共有同士は取れる**（「shared read-only access to multiple reader threads」）。Learn は共有の再帰を「should not」と書くだけで、共有だけの再帰が即座に壊れるとは書かない | 共有の個数を数えるか（`SRWLOCK` は「very little state information」）。**共有のみの再帰を許すか、停止にするか**は Learn が書かない |
| **共有で取られているロックへの `AcquireSRWLockExclusive`** | 「a thread that owns an SRW lock in shared mode cannot upgrade its ownership of the lock to exclusive mode」 | 単一スレッドでは**待つ相手（共有を手放す者）がいない**ので永遠に戻らない。名前付き停止が整合 | なし |
| **排他で取られているロックへの `AcquireSRWLockShared`** | 「When the lock has been acquired in exclusive mode, no other thread can access the shared resource until the writer releases the lock.」 | 同じく永遠に戻らない（デッドロック）。名前付き停止が整合 | なし |
| **`ReleaseSRWLockExclusive/Shared`（取られているロックを、同じモードで）** | 「The SRW lock must be released by the same thread that acquired it.」 | ロックを空に戻す（共有は個数を 1 減らす） | なし |
| **ロックを持たずに Release した場合** | **ページは書かない**（§5.1。「must be released by the same thread that acquired it」だけ。違反時の結果は無い）。決定 0017 の `LeaveCriticalSection` は同じ状況（所有していない Leave）を、ページの「別スレッドの Enter が永遠に待つことがある」を根拠に **abort** にした | **名前付き停止にするのが決定 0017 と同じ整理**（ページが結果を書かず、実機では「未定義」または他スレッドに害が出る）。ゲストの全サイトは Acquire → Release の対（§4.2）なので、停止は起きない見込み（読み） | 「`SRWLOCK_INIT` のまま（空）を Release する」入力の結果を作らない（停止） |
| **モードの食い違い**（排他で取って共有で Release、その逆） | **書かない** | 名前付き停止 | 作らない（停止） |
| **別スレッドからの呼び出し** | — | 決定 0017 の `fle_sys_check_thread` と同じ規則で名前付き停止（wasm では `fle_host_tid()` が定数 1。決定 0045） | 流儀の適用だけ |

**意見ではなく整理として言えること**: (i) 「ブロックしうる Acquire は名前付き停止」は、Learn の「deadlock」の記述と決定 0017 / 0050 の前例の両方と整合する。(ii) 「ロックを持たない Release」の扱いは Learn が書かず、決定 0017 の前例（abort）を適用するのが唯一の導き方だが、**その前例は別の API（CriticalSection）のページの文に基づく**ので、SRW には「類推」になる（Learn の SRW ページには根拠が無い）。(iii) **状態の置き場**: Blend2D の SRWLOCK は静的な 2 本（`0x1802f4700`、`0x1802f4800`）とヒープ上のメンバ（スレッドごと、プール、他のオブジェクト）。**静的な 2 本はゲストが `lea` とゼロの書き込み以外で触らない**（§4.2。実測）ので、**runtime が表でアドレスをキーに持つ**ことと、**ゲストが値を読む**ことは衝突しない。ヒープのメンバについてはゲストが読み書きしないとは**確認していない**。(iv) 表を持たず SRWLOCK の 8 バイトに状態を書く案（ポインタ 1 個ぶん）は、Learn が「The process must not modify the object」と書き、決定 0017 の流儀（オブジェクトの中身を読まず書かない）と反する。

### 6.2 条件変数

| 状況 | Learn が書くこと | 単一スレッドの下で素直に導ける答え | 書かない部分 |
|---|---|---|---|
| **`WakeConditionVariable` / `WakeAllConditionVariable`（待機者がいない）** | 「The WakeAllConditionVariable wakes all waiting threads while the WakeConditionVariable wakes only a single thread.」。**待機者がいないときの動作は書かない** | **単一スレッドでは待機者がいない**（Sleep は永遠に戻らず停止か、タイムアウトで戻る）。何も起こらず戻る（VOID）。「待機者がいないとき何もしない」は、`WakeConditionVariable` が「similar to setting an auto-reset event」と書かれることから、**イベントのように信号が残る可能性**も読めるが、Learn は書かない。決定 0050 の `SetEvent` は「待機者がいなければ信号のまま残る」とページが明記しており、**Wake にはその文が無い**ので、イベントの流儀（信号を残す）は**作れない**。**信号を残さず何もしない**のが、条件変数の通常の意味（条件変数は述語を持たない）だが、**ページはそう書かない**（§5.2）。仮の決定になる | 「待機者がいないとき状態を残すか」 |
| **`SleepConditionVariableSRW`（`dwMilliseconds = INFINITE`）** | 「If dwMilliseconds is INFINITE, the function's time-out interval never elapses.」「The thread can be woken using the WakeConditionVariable or WakeAllConditionVariable function.」 | **起こす者がいない**（Wake を呼べるのはこのスレッドだけで、このスレッドは Sleep の中で動かない）。**永遠に戻らない**。名前付き停止が整合（決定 0017 の `Sleep(INFINITE)`、0050 の INFINITE 未シグナル待ちと同じ）。Blend2D の `INFINITE` の 3 か所（`f_18019c5e0` スレッドの本体、`f_180199670`、`f_18013d7b0`）は**述語が偽のときだけ Sleep する `while`**（§4.2）なので、述語が真のままなら Sleep に入らない | なし |
| **`SleepConditionVariableSRW`（有限、非 0）** | 「The function returns if the interval elapses」「If the timeout expires the function returns FALSE and GetLastError returns ERROR_TIMEOUT.」 | **タイムアウトまで待つ = 仮想時計が `ms` 進む**ことになるが、決定 0050 §C は「仮想時計を進める窓口は必要になるまで足さない」ので**有限・未シグナルの待ちを abort にした**。**条件変数の有限 Sleep も同じ整理なら名前付き停止**。Blend2D の有限 Sleep は `f_18019c920` の 1 か所（`atexit` 風の終了処理。値は計算値で未確認。スレッド数 `[rdi+0x18]` が 0 のときは入らない） | **ERROR_TIMEOUT の数値 1460 は Learn の本ページに無い**（System Error Codes にある。この調査は未取得）。**時計を進めずに FALSE と ERROR_TIMEOUT を返す**案は、決定 0050 §C が却下した「間隔が経過したと報告しながら何も経っていない」と同じ問題を持つ |
| **`SleepConditionVariableSRW`（`dwMilliseconds = 0`）** | 「If dwMilliseconds is zero, the function tests the states of the specified objects and returns immediately.」 | 単一スレッドでは起こす者がいないので、**条件を満たさず即座に戻る** = タイムアウト扱い（FALSE、ERROR_TIMEOUT）が導けるが、**ERROR_TIMEOUT の値は上のとおり未取得**。Blend2D は 0 を定数で渡さない（4 か所は INFINITE が 3、計算値が 1。計算値が 0 になりうるかは未確認） | ロックを手放して再取得する扱い |
| **`SleepConditionVariableSRW` で、ロックを持たない** | 「If the lock is unlocked when this function is called, the function behavior is undefined.」 | **名前付き停止**（ページが「undefined」と明記） | なし |
| **ロックを手放す／再取得する** | 「Sleeps on the specified condition variable and releases the specified SRW lock as an atomic operation」「After the thread is woken, it re-acquires the lock it released」 | Sleep が戻る場合（0 か有限で戻るなら）、**ロックは再取得された状態で戻る**必要がある。Learn はタイムアウトで戻る場合の再取得を明記しない（§5.2） | タイムアウトで戻るときの再取得 |

### 6.3 スレッド系

| API | Learn | 単一スレッド（案 C）の下 |
|---|---|---|
| **CreateThread** | 戻り値はスレッドのハンドル。「During process startup and DLL initialization routines, new threads can be created, but they do not begin execution until DLL initialization is done for the process.」 | **案 C（PM 決定）は「スレッド生成に届いたら名前付き停止」**。Learn は DLL 初期化中に作られたスレッドが**初期化の完了まで走らない**とだけ書き、**走らせないまま作る（ハンドルを返し、実行しない）**ことは Learn の記述の範囲内。ただし、これは**スレッド実体を作ることになり**（スレッド ID、ハンドルの値、`WaitForSingleObject(thread)` の意味、`GetExitCodeThread`）、決定 0080 D9、案 C と衝突する。**§4.1 のとおり、ATTACH の実行では `CreateThread` に届かない読み** |
| **ExitThread、FreeLibraryAndExitThread** | 「the current thread's stack is deallocated … the thread terminates. The entry-point function of all attached DLLs is invoked with a value indicating that the thread is detaching」「If the thread is the last thread in the process when this function is called, the thread's process is also terminated.」 | **単一スレッドでメインスレッドが呼ぶ = プロセス終了**。`ExitProcess` / `TerminateProcess` が名前付き停止（fle_proc.c 211〜）と同じ扱いが整合。スレッドのラッパー内でしか呼ばれない形（直後が `int3`）なので、スレッドを作らない限り到達しない読み |
| **WaitForSingleObjectEx** | 戻り値、`bAlertable`。Blend2D の引数は **手動リセット・非シグナルのイベント、100 ms、bAlertable = 0** | 決定 0050 の `WaitForSingleObject` と**同じ状態機械で答えられる**（イベントの表を共有）。非シグナルで有限 100 ms は **0050 §C の abort**（「a finite, nonzero time-out on an event that is not signaled needs the virtual clock to advance」）。**`WaitForSingleObjectEx` の `bAlertable = FALSE` なら `WaitForSingleObject` と同じ**（Learn の `bAlertable` の記述から導ける。TRUE は APC が要るので停止）。**ただし Blend2D の単一スレッドの経路では `0x18019dc78` に入らない読み**（§4.1） |

### 6.4 その他

| API | 単一スレッド（案 C）・既存の決定との整合 |
|---|---|
| **GetFileSizeEx** | 既存の仮想ファイルシステム（`fle_vfs.c`、決定 0018）のハンドルに対する答えが作れる（ファイルのサイズ）。呼び元の直前に `SetFilePointerEx`（行ありの名前付き停止）があるので、**この経路は `SetFilePointerEx` で先に止まる**（`0x1801a8fe6`）。ATTACH では届かない読み（B のみ、深い鎖） |
| **OutputDebugStringA** | Learn: 「If the application does not have a debugger and the system debugger is not active, OutputDebugString does nothing.」。既存の `OutputDebugStringW` は「デバッガーなし」で何もしない（fle_proc.c 199〜205、`imports_table.rs` 116 行）。**同じ流儀で何もしない**。引数文字列は読まない（`lpOutputString` は optional） |
| **IsProcessorFeaturePresent** | runtime の関数は**決定 0078 で実装済み**（fle_sysinfo.c 72〜106。PF 6、9、10、23 は 1、15 個は 0、他は名前付き停止）。応答表に行が無いだけ。Blend2D の 5 か所は PF 23 だけ（`ecx = 0x17`）で 1 を返す |
| **TlsFree** | runtime の関数は**決定 0078 で実装済み**（fle_imports.c 135〜148）。応答表に行が無いだけ。Blend2D の 2 か所は `FlsFree` を `GetProcAddress` で引けなかったときの代替で、FLS の行がある環境では非 NULL の `FlsFree` を呼ぶので届かない読み（§4.1） |
| **GetLargePageMinimum** | Learn: 「If the processor does not support large pages, the return value is zero.」。**このエミュレートされた CPU が大ページを持つかは Learn から導けない**。既存の `VirtualAlloc` は `MEM_LARGE_PAGES` を名前付き停止にする（fle_sys.c の VirtualAlloc。決定 0017 §C）ので、**0 を返す（大ページなし）と `VirtualAlloc` の大ページ経路に入らない**という整合は取れる。ATTACH では届かない（§4.1） |

## 7. 決定が要る点と選択肢（事実の表。**決めない**）

### 7.1 1 つの方針で扱えるか（事実）

**1 つの方針では扱えない**（事実としての根拠）: 16 名は、(i) 状態を持つもの（SRW 4、条件変数 3。単一スレッドで状態機械にできる）、(ii) 既存の runtime に同種の実装があるもの（`WaitForSingleObjectEx`: 決定 0050 のイベント表、`OutputDebugStringA`: `OutputDebugStringW`、`GetFileSizeEx`: `GetFileSize`）、(iii) 実装済みで行だけが無いもの（`IsProcessorFeaturePresent`、`TlsFree`）、(iv) 案 C が既に方針を決めているもの（スレッド 3）、(v) Learn から値が導けないもの（`GetLargePageMinimum`）の 5 種類に分かれ、**Learn が書く範囲、既存の決定との整合、ATTACH の実行で届くか**がそれぞれ違う。(i) の中でも SRW は「静的初期化の登録」という Learn が書かない扱いが要り、条件変数は「待機者がいない Wake」が書かれない。

### 7.2 各名前への選択肢（事実）

凡例: **(a)** Learn の規則どおり単一スレッドの状態機械で実装し、ブロックしうる場合は名前付き停止／**(b)** 名前付き停止のまま（応答表に行を足す＝呼ばれたら名前付きで止まる／行を足さない＝現状の `fle_unsupported(VA)`）／**(c)** その他（下に書く）。「届く」は §4.1 の読み。

| 名前 | 種類 | ATTACH で実行（読み） | (a) の中身（Learn の範囲） | (b) | 仮の決定になる点（Learn が書かない動作を作る場合） |
|---|---|---|---|---|---|
| AcquireSRWLockExclusive | SRW | **実行される**（`f_18019caa0`。実測の停止点）。他の 19 か所は未確認 | アドレスをキーとした表で状態（空／排他／共有 N）。空なら取る。取られていれば名前付き停止（Learn「deadlock」） | 行を足し名前付きで停止（`SRW...: not implemented` の形）。行を足さなければ現状の `fle_unsupported`（`GetProcAddress` は NULL と 127）。**今回の停止点で止まり続ける** | **静的初期化（`SRWLOCK_INIT`）の登録**: Learn は値を書かない。未登録アドレスの最初の Acquire を初期状態とみなす扱い |
| ReleaseSRWLockExclusive | SRW | 同上（`f_18019caa0` の後 1 回） | 排他で取られていれば空に戻す。**ロックを持たない Release は Learn が結果を書かない** | 同上 | **持たない Release の扱い**: 決定 0017 の `LeaveCriticalSection` の前例（abort）を適用するなら停止。**SRW のページには根拠が無い** |
| AcquireSRWLockShared | SRW | B のみ（アドレスが取られる。未確認） | 共有の個数を表に持つ。排他で取られていれば停止 | 同上 | **共有のみの再帰**（Learn: should not）を許すか、**共有の個数**を持つか |
| ReleaseSRWLockShared | SRW | B のみ | 個数を 1 減らす | 同上 | 同上 |
| WakeConditionVariable | 条件変数 | B のみ（`f_18019c4d0` の投入、`f_18019c560` の停止などで呼ばれる形。**ATTACH で実行されるかは未確認**） | 待機者がいないので何もしない（VOID）。**Learn は待機者なしの動作を書かない** | 同上 | **待機者がいないとき何もしない（信号を残さない）**。イベントの `SetEvent`（信号が残る）とは違う |
| WakeAllConditionVariable | 条件変数 | B のみ（`f_180199670` の中） | 同上 | 同上 | 同上 |
| SleepConditionVariableSRW | 条件変数 | B のみ。4 か所は INFINITE 3、有限（計算値）1。有限の 1 か所は `atexit` 風の終了処理（`f_18019c920`）で、ATTACH では呼ばれない読み | INFINITE: 起こす者がいない → 名前付き停止。有限: 決定 0050 §C と同じく時計の窓口が無いので名前付き停止。0: FALSE と ERROR_TIMEOUT（**値 1460 は Learn の本ページに無い**）。ロックを持たない: 停止（Learn「undefined」） | 同上 | **有限・0 で戻す場合の ERROR_TIMEOUT の値**と、時計を進めないで戻す問題（0050 §C）。**戻る場合のロックの再取得**（Learn はタイムアウトでの再取得を明記しない） |
| CreateThread | スレッド | **届かない読み**（A で 0、B では `f_18019cfa0` の鎖。ATTACH の `.xc` は表に置くだけ） | (a) は案 C と衝突（スレッド実体が要る）。DLL 初期化中は走らせない、は Learn の範囲だが、**スレッド ID、ハンドルの値、WaitFor(thread)、GetExitCodeThread、ExitThread の意味**が要る | **名前付き停止（案 C。PM 決定済み）**。行を足す（`CreateThread: not implemented` で止まる）か足さないか | (a) を選ぶならスレッド実体の設計（決定 0080 D9 を超える）。**PM 決定済みの案 C を変える** |
| ExitThread | スレッド | 届かない読み | メインスレッドが呼ぶとプロセス終了（Learn）→ 名前付き停止（`ExitProcess` の流儀。fle_proc.c 211〜） | 同上 | なし |
| FreeLibraryAndExitThread | スレッド | 届かない読み | 同上 | 同上 | なし |
| GetFileSizeEx | ファイル | B のみ（深い鎖。直前の `SetFilePointerEx` が名前付き停止） | 仮想ファイルのハンドルならサイトの値を返す（`fle_imp_GetFileSize` と同じ表。ファイル以外のハンドルは停止） | 同上 | 失敗時の last error の値（Learn は書かない。`GetFileSize` の既存の選択に揃える） |
| IsProcessorFeaturePresent | CPU 機能 | **A でも届くが、5 か所すべて PF 23 のエラー経路の中**（通常は呼ばれない） | **runtime 関数は実装済み**（決定 0078）。**応答表に行を足す = (a)** | 行なし（現状。`fle_unsupported`、`GetProcAddress` は 127） | 行を足すと**既存テストの期待値の更新が要る**（§8）。値は決定 0078 の選択（仮）のまま |
| OutputDebugStringA | デバッグ出力 | B のみ（ログ・エラー表示の形） | 何もしない（Learn「does nothing」。`OutputDebugStringW` と同じ流儀）。文字列を読まない | 同上 | なし（既存の `OutputDebugStringW` と同じ「デバッガーなし」の前提） |
| TlsFree | TLS | **A で届くが DETACH 側**。ATTACH では `FlsFree` を引けたときは届かない | **runtime 関数は実装済み**（決定 0078）。**行を足す = (a)** | 行なし（現状） | 行を足すと**既存テストの期待値の更新が要る**（§8）。失敗コード 87 は決定 0078 の選択（仮） |
| WaitForSingleObjectEx | 待機 | **A で届くが、単一スレッドでは入らない分岐**（ガードが `-1` のとき）。`SleepConditionVariableCS` を引けなければ代替イベントで待つ | 決定 0050 のイベント表で、`bAlertable = FALSE` は `WaitForSingleObject` と同じ。TRUE は停止。非シグナルの有限 100 ms は 0050 §C の abort | 同上 | なし（0050 の前例の適用）。ただし **`event.rs` 233 行が「WaitForSingleObjectEx は TABLE に無い」と表明している**（§8） |
| GetLargePageMinimum | 大ページ | **届かない**（A でも B でも 0） | Learn から値が導けない。**0（大ページなし）を返す**のが既存の `VirtualAlloc`（`MEM_LARGE_PAGES` は停止）と整合するが、**エミュレートする CPU が大ページを持つかを決める必要がある**（決定 0009、0047 の CPUID は拡張葉なし） | 同上 | **値の選択そのもの**（Learn が値を書かない）。今は届かないので必要が出るまで見送れる |

**(c) その他の選択肢**（事実としての列挙。評価しない）:

| # | 選択肢 | 内容 |
|---|---|---|
| c1 | `WaitOnAddress` / `WakeByAddressSingle` / `WakeByAddressAll` を答える（API セット `api-ms-win-core-synch-l1-2-0.dll` の行を足す） | `.xc` の `[0x1801f31a0]` が 1 になり、スレッドプールが SRW/条件変数でなく `WaitOnAddress` 版（vtable `0x1802f4750`、本体 `f_18019c7f0`）になる（§4.2）。**ただし今回の停止点 `f_18019caa0`（プールの `this+0x30` の SRW）は両方の版で使われ、SRW は不要にならない**。`WaitOnAddress` は別の新しい API で、Learn の確認とブロックの扱いの決定が要る |
| c2 | SRW 4 名を「最小の状態機械」でなく、**表を持たない空実装（Acquire/Release とも何もしない）** | 決定 0017 が「全部 no-op にする案」を**却下した理由**（状態の違反を見逃す）が当てはまる。単一スレッドで Learn が「deadlock」と書く入力（同じスレッドの再 Acquire）も黙って通す。**この環境の前例に反する**（事実としての前例の指摘であって推奨ではない） |
| c3 | 今回の停止点だけを通す（`f_18019caa0` の Acquire/Release の 1 対）最小の SRW 状態機械、他は名前付き停止 | 事実: ATTACH で実行が確実なのはこの 1 対だけ（§4.1）。残りの 19 Acquire などは**到達が未確認**なので、実行で届いた順に名前付き停止から実装へ進める進め方。（選択肢の記述で、推奨ではない） |
| c4 | スレッドプールを使わない構成を前提に、`thread_count` が 0 のままであることを根拠に CreateThread 系を「届かない」と扱う | Blend2D の公式ドキュメントは既定が単一スレッドと書く（feasibility §6。取得済み、未再取得）。**FL Studio が `thread_count` を渡すかは未確認**。今回の ATTACH の実行では `CreateThread` に届かない（§4.1 の読み） |

### 7.3 決めるべき点（事実の整理。結論ではない）

1. **SRW 4 名を実装するか、行を足して名前付き停止にするか**。（今回の停止点で必要になる最小の範囲は c3。）実装する場合の**仮の決定**: (1) 静的初期化（未登録アドレスの初回使用を空とみなす）、(2) 持たない Release の扱い（決定 0017 の前例を類推するなら停止）、(3) 共有のみの再帰の扱い、(4) ヒープ上のロックでゲストが中身を読む可能性（未確認）。
2. **条件変数の Wake（待機者なし）の扱い**。何もしない（信号を残さない）か、名前付き停止にするか。Learn が書かないので、(a) を選ぶなら仮の決定。
3. **`SleepConditionVariableSRW` の INFINITE／有限／0 の扱い**。決定 0050 §C と同じ（仮想時計の窓口が無いので有限は停止）にするか。`ERROR_TIMEOUT`（1460）を使うなら System Error Codes の確認が先に要る。
4. **スレッド系を応答表に行として足すか**（足すと `GetProcAddress` が非 NULL を返す。足さないと NULL と 127）。案 C（PM 決定済み）は「届いたら名前付き停止」で、**行を足す／足さないは案 C の範囲内で選べる**。ただし**行を足すと FLEngine の生成 C の SHA が変わる**（§8）。FLEngine は `CreateThread`、`ExitThread`、`TlsFree` を既に import している（§8）ので、**FLEngine の thunk が `fle_unsupported` から名前付き停止に変わる**。
5. **`IsProcessorFeaturePresent`、`TlsFree` の行を足すか**。足すなら既存テストの期待値の更新が要る（未決。PM の決定が要る。決定 0078 の撤回条件にも触れる）。**ATTACH で実行される見込みが低い**ので急がない、とも読める（§4.1）。
6. **`WaitForSingleObjectEx`**: 決定 0050 §A が「DLL が import していないので実装しない。テストで TABLE に無いことを固定する」と決めている。Blend2D が import するので前提が変わった（この調査の事実）。**0050 の文言を更新する必要がある**。
7. **`GetLargePageMinimum` の値**。ATTACH では届かないので、必要が出るまで見送れる。
8. **`WaitOnAddress` 系を答えるか**（c1）。

**1 つの決定にまとめるか、種類ごとに分けるか**: 事実として、(1)〜(3) は同じ API 群（SRW + 条件変数）で互いに依存する（Sleep の仮の決定は SRW の状態機械に依存する）ので**1 つにまとめられる**。(4) は案 C（PM 決定済み）の適用、(5) は決定 0078 の撤回条件とテストの更新、(6) は決定 0050 の更新、(7) は CPU（決定 0009、0047）の問題で、**それぞれ別の決定に属する**。16 名を同時に 1 つの決定にするには、これらの前提の違いを 1 つの文書に混ぜることになる。

### 7.4 意見（事実ではない。決めない）

- 今回の停止点を通すだけなら c3（SRW を取る・放す 1 対だけの状態機械）で足りる。ただし、`f_18019caa0` の後に実行が続くと `.xc` の初期化子の残り（`0x18019a1d6` 以降。`atexit` 風の登録、vtable の他の関数を呼ぶ `f_1800a9eb0` など。ATTACH で呼ばれるかは未確認）に進むので、SRW 4 名と条件変数 3 名を**同じ種類の 1 つの決定**にして、`WaitOnAddress`（c1）の判断を別に出す、という切り分けが素直に見える。
- 「ロックを持たない Release」と「待機者のいない Wake」は Learn が書かないので、**停止にする側に倒す**（fail-closed）か、**何もしない側に倒す**かは未決（PM の決定が要る）。SRW の Release は全サイトが Acquire → Release の対なので、停止にしても今の経路では起きない読み。Wake は状態を持たないので、何もしない側の方が副作用が小さい（ただし Learn の根拠は無い）。
- `IsProcessorFeaturePresent`、`TlsFree` の行は、ATTACH でも通常は届かないので、SRW の決定と一緒にせず別に判断してよく見える。

## 8. 影響範囲（**読み**。実行していない。テストも走らせていない）

### 8.1 応答表に行を足すと何が変わるか（決定 0078、0091 の前例。実測の前例と読み）

- **FLEngine の生成 C の SHA-256 は変わる**。前例（実測）: 決定 0078（応答表 33 行 → `67827424…` → `43985d62…`）、決定 0091（1 行 → `43985d62…` → `eb9d8e7f…`。変わった行は応答表の 5 か所 7 行だけ。**変わった関数は 0 個**）。行を `TABLE` の**末尾に足す**限り既存の擬似 VA は動かない（決定 0015）。**今の基準は `eb9d8e7fa74ae562742fb1999f6da9b3ca82f2884d1845b5b7319153e1b709cf`**（実測: `lift/out-gmh/fle.c` の `sha256sum`。PM の全体テストが終わるまで 0091 は未コミット）。
- **FLEngine は `CreateThread`、`ExitThread`、`TlsFree` を import している**（pefile。実測。`scan-fle.txt`: スロット `0x19713f0`、`0x1971760`、`0x1971670`。**`jmp [IAT]` の thunk が 4 つ**: `CreateThread` が `0x406f80` と `0x423260`、`ExitThread` が `0x406fb0`、`TlsFree` が `0x41cf50`）。この 4 つの thunk は現在 **`fle_unsupported(UINT64_C(0x…))`**（`lift/out-gmh/fle.c` の 119526、147523、119541、144882 行。実測）。**行を足すと、この 4 つが `fle_imp_*(); return;` に変わる**（関数の本体が変わる。0078 で 6 つの thunk が変わったのと同じ形。0091 では FLEngine の thunk は変わらなかった）。**FLEngine は `AcquireSRWLock*`、`ReleaseSRWLock*`、`WakeConditionVariable`、`SleepConditionVariableSRW`、`WaitForSingleObjectEx`、`GetFileSizeEx`、`GetLargePageMinimum`、`IsProcessorFeaturePresent`、`OutputDebugStringA`、`WaitOnAddress`、`GetModuleHandleA` を import していない**（delay import 57 件も含めて調べた。`scan-fle.txt`、`pefile`。実測）。したがって**行を足したときの FLEngine の thunk の変化は、`CreateThread` 2、`ExitThread` 1、`TlsFree` 1 の 4 か所だけ**で、他の名前は「応答表の行と宣言の追加」だけ（`GetModuleHandleA` と同じ形）。
- **dsp_ippv2 は `IsProcessorFeaturePresent`（5 か所 `call [IAT]`）と `TlsFree`（1 か所 `jmp [IAT]`）を import している**（実測: `scan-dsp.txt`、縮小前の `lift/out-ext/dsp_ippv2_x64.dll`（SHA-256 `497dd4f7…`）。pefile の import 表）。**この 2 つの行を足すと、dsp_ippv2 の生成 C の関数が変わる**（5 か所の `fle_unsupported` が `call_imp` に、1 か所が `jmp_imp` に。決定 0078 §3 が「行にしなかった」理由）。dsp_ippv2 は上の SRW、条件変数、スレッド、`GetFileSizeEx`、`OutputDebugStringA`、`WaitForSingleObjectEx`、`GetLargePageMinimum` を import していない（実測）。縮小版（種 `seeds-idx0-plus-initterm2.txt`）では `TlsFree`、`IsProcessorFeaturePresent` の呼び出し点が含まれるかは**未確認**。
- 3 モジュール構成の `two-native` では、FLEngine（`gen_two.sh` は `EXPECT_FLE_SHA` で基準の SHA を検査する。既定は `43985d62…`。0091 の後は `eb9d8e7f…` を環境変数で渡している）、縮小 dsp_ippv2、Blend2D の 3 つの生成 C の応答表が同じ行を持つ（`merge_tables.py` が 69,654 行に統合。「複数の表に在る行 261」）。**行を足すと 3 つとも表が変わる**。

### 8.2 `IsProcessorFeaturePresent` と `TlsFree` の行を足すときに更新が要る既存テスト（**読み。行番号は HEAD + 0091 のワークツリー**）

| ファイル | 行 | 表明している内容 | 行を足すと |
|---|---|---|---|
| `lift/crates/fe-lift/tests/k32_dsp.rs` | 9（コメント）、**200〜203**（`for n in ["IsProcessorFeaturePresent", "TlsFree"]`: `!table.contains("\"{n}\", \"fle_imp_")` かつヘッダーに宣言） | 2 名に行が無いこと | **失敗する**。この 2 名を `ANSWERED` に移し、203 行を直す |
| 同上 | 206〜208（コメント）、**210〜226**（`real_dll_leaves_only_the_two_unrowed_names`）。**224 行**: `unsupported["Call: indirect through IAT slot (not lifted)"] == 5`、**225 行**: `unsupported["Jmp: indirect through IAT slot (not lifted)"] == 1` | 実 DLL（`lift/out-ext`。無ければ早期 return）で、未応答が `IsProcessorFeaturePresent` 5 と `TlsFree` 1 だけ | **DLL があれば失敗する**（0 になる）。DLL が無い環境（CI）では何もせず通る。関数名の `…leaves_only_the_two_unrowed_names` も意味を失う |
| `lift/crates/fe-lift/tests/imports_table.rs` | 5（コメント）、89〜90、**126**（`!st.handled.contains_key("jmp_imp:TlsFree")`）、**129**（`!c.contains("fle_imp_TlsFree")`）、131（`TlsFree` の thunk が `fle_unsupported`）、**133**（`unsupported["Jmp: …"] == 2`） | 合成 PE の `kernel32.dll!TlsFree` を「表に無い名前」の例にしている | **失敗する**（`TlsFree` が応答する）。別の名前（例: 表に無い API）を「表に無い名前」の例に差し替える必要がある |
| `lift/crates/fe-lift/tests/sysinfo.rs` | 883（`gpa(k, "IsProcessorFeaturePresent")`）、**918**（`"IsProcessorFeaturePresent null 127"`） | `GetProcAddress(kernel32, "IsProcessorFeaturePresent")` が NULL と 127 | **失敗する**（擬似 VA を返す）。「応答表に無い名前」の例を別の名前にする必要がある（`GetNumaHighestNodeNumber`、`GlobalMemoryStatusEx` などが同じ配列にある） |
| `lift/crates/fe-lift/tests/call_iat_dll.rs` | 47〜48（`assert_eq!(total, 138)`、`answered_total + unanswered == total`） | 実 DLL の `call [IAT]` 138 か所、応答数と未応答の合計 | 合計は 138 のまま、**応答数だけ 133 → 138 に変わる**（この表明は合計だけなので通る見込み）。**DLL があるときだけ実行** |
| `lift/crates/fe-lift/tests/event.rs` | **232〜233**（`assert!(!table.contains("WaitForSingleObjectEx"))`） | `WaitForSingleObjectEx` の行が `TABLE` に**無い**こと（決定 0050 §A。「DLL が import していない」が根拠） | **`WaitForSingleObjectEx` の行を足すなら失敗する**。決定 0050 の文言（§A「実装しない」）の更新も要る |
| `lift/crates/fe-lift/tests/modules.rs` | **526〜529**（0091 の新規テスト。`GetModuleHandleA` の行が**最後の行**であること: `0x7ff900000000 + 0x10 * n`、`n = st.resp_entries`） | `GetModuleHandleA` が応答表の末尾 | **新しい行を末尾に足すと失敗する**（`GetModuleHandleA` が末尾でなくなる）。`TABLE` の末尾以外（途中）に足すと既存の擬似 VA が動くので不可（決定 0015）。**行を足すたびに、直前の「最後の行」のテストを直す必要がある**。0091 は未コミットなので、この調査の時点でこのテストはまだ HEAD に無い |
| `lift/crates/fe-lift/tests/modules.rs` | 177〜191（`n >= 13` と `fle_resp_count = {n}`、dispatch count）、213〜218、762〜766 | 行の数は固定しない（「他のテストは数を固定しない」とコメントがある） | 通る（数を固定していない） |
| `lift/crates/fe-lift/tests/realmod.rs` | 539（コメント: 「no response row has an api-ms-* dll」） | `api-ms-*` の DLL 名の行が無いこと | **`WaitOnAddress` 系（c1）の行を足すと** 失敗する可能性（`api-ms-win-core-synch-l1-2-0.dll` の行を足す場合）。今回の 14 名の行は kernel32 なので影響しない |

`grep` の範囲: `lift/crates/**/*.rs` で `TlsFree`、`IsProcessorFeaturePresent`、`WaitForSingleObjectEx`、`OutputDebugStringA`、`GetFileSizeEx`、`CreateThread`、`SRWLock`、`ConditionVariable`。**`OutputDebugStringA`、`GetFileSizeEx`、`SRWLock`、`ConditionVariable` を「行なし」と表明するテストは見つからなかった**（`grep` で 0 件）。`CreateThread` は `msctf.rs`（別の名前の部分一致）だけで、行なしを表明するテストは無い。

**FLEngine の生成 C の SHA を固定するテストは無い**（`grep 43985d62|eb9d8e7f` が `*.rs` で 0 件。固定しているのは `lift/harness/two/gen_two.sh` の `EXPECT_FLE_SHA`（既定 `43985d62…`）だけ。0091 でも「環境変数で渡した」と記録されている。**行を足すたびにこの値を渡す必要がある**。スクリプトは変えない）。

### 8.3 新しい runtime ファイルを作らず既存ファイルに足せるか（**読み**）

- `lift/crates/fe-lift/tests/modules.rs`（`runtime_sources`: 135 行）ほか多くのテストが `runtime/fle_*.c` を**全部**コンパイルしてリンクする（`starts_with("fle_") && ends_with(".c")`）ので、**新しい `fle_*.c` を足しても自動でリンクされる**（`modules.rs` 129〜138 行、`k32_dsp.rs` 40〜47 行など。ただし `callback.rs` 670 行は `fle_callback.c` を除く）。**足す先の既存ファイルの候補**（読み）: SRW・条件変数は `fle_sys.c`（クリティカルセクションの表と `fle_sys_check_thread`。決定 0017。`FLE_CS_*` の近く）、`WaitForSingleObjectEx` は `fle_event.c`（`event_arg`、`fle_event_check_thread` を共有）、`OutputDebugStringA` は `fle_proc.c`（`OutputDebugStringW` の隣。199〜205 行）、`GetFileSizeEx` は `fle_vfs.c`（`fle_imp_GetFileSize` の隣。744〜）、`IsProcessorFeaturePresent` は `fle_sysinfo.c`、`TlsFree` は `fle_imports.c`（既に実装済み）、スレッド名前付き停止は `fle_proc.c`（`ExitProcess` の隣。211〜）、`GetLargePageMinimum` は `fle_sysinfo.c`（`GetNativeSystemInfo` の隣）。宣言は `fle_imports.h`（ただし同ファイルは `fle_imp_*` の宣言の書式が「`/* kernel32!X: … */ void fle_imp_X(void);`」）か、各ファイルの `.h`。
- **注意**: 決定 0017 の「同期は `fle_sys.c`」の関数群は、`fle_sys_owner_tid`（別スレッド停止）と `STOP` マクロを持つ。SRW を足すなら同じファイルが流儀を共有できる。`fle_sys.c` を変えると `tests/sys.rs` など `fle_sys.c` を読むテスト（`source_uses_no_host…` 系）が走る（禁止語の `getenv`、`clock_gettime` などを書かない必要がある。`event.rs` 679〜680、`sysinfo.rs` 950〜951、`k32_dsp.rs` 739）。
- **既存の流儀に載る点**: (i) 状態は表（`fle_cs_table`: アドレスをキー、`FLE_CS_SLOTS` = 65,536）に持つ。SRW も同じハッシュ表の形で書ける。(ii) 停止は `STOP` / `fle_stop(api, why)`。(iii) 別スレッドは `fle_sys_check_thread`。(iv) 戻り値が VOID の API は `R[0]` を触らない（`fle_imp_EnterCriticalSection` の形）。
- **前例が無い点**: **「未登録のアドレスを初期状態（空）として扱う」** 流儀は、現行のどの表（臨界領域: 未登録は abort、SList: 未登録は abort）にも無い。SRW だけが静的初期化（`SRWLOCK_INIT`）を許す API なので、**最初の使用で暗黙に登録する**扱いが新しく要る。

### 8.4 テストを走らせていない

この調査は `cargo build` / `cargo test` を実行していない（PM の全体テストとワークツリーを汚さないため）。§8.2 は `grep` と読みで、実際に落ちるかは実行で確かめていない。

## 9. 次に届く API の順の見込み（任意。**読み**。実行していない）

今回の停止点 `f_18019caa0`（`.xc` の `0x18019a1d2: call r8`）に SRW を実装して戻せたと仮定した場合の、続きの**静的な読み**:

1. `f_18019caa0` が戻ると、`.xc` の初期化子は `0x18019a1d6`〜`0x18019a209` で `atexit` 風の表に 2 つ登録を書く（`0x18019d080`、`0x18019d090`。**表に書くだけで呼ばない**）。続いて `0x18019a210`〜`0x18019a2b4` で **グローバルのロック `0x1802f4700` 周辺（`0x1802f4700`〜`0x1802f4748`。`0x18019a274` でゼロ）と `0x1801f4620` 周辺をゼロ・定数で初期化し、`0x18019a2cb: call 0x18019bf60`**（連結リストへの挿入。中で SRW は呼ばない: `f_18019bf60` の本体 `0x18019bf60`〜は `lock` も API 呼び出しも無い。逆アセンブルの読み）。その後 `0x18019a334`〜`0x18019a384` で `0x18019bea0`、`0x18019bec0`、`0x18019bf00` を `atexit` 風の表に登録（**表に書くだけ**）、`0x18019a38b`〜以降は `.data` への定数の書き込みが続く。`0x18019a1d6` 以降の **`call` は `0x18019a2cb`（`f_18019bf60`）と、末尾の 5 つ（`0x18019af0d`、`0x18019b045`、`0x18019b07d`、`0x18019b082`、`0x18019b087`）だけ**（`dis-init.txt`、`dis-init2.txt` の `grep call`。**間接 call は無い**）。
2. 続く `0x18019af0d: call 0x1800a9eb0`、`0x18019b045: call 0x18006b6c0`、`0x18019b07d: call 0x180147870`、`0x18019b082: call 0x180147cf0`、`0x18019b087: call 0x18006ed70` は**自身は行なしの API を呼ばない**（§4.1 の 3）。したがって `.xc` の初期化子の残りで**新しい行なしの API が呼ばれる見込みは低い**（読み。直接 call の先の中身を読んだ範囲では 0。`f_1800a9eb0` の `0x18000fbc0`、`0x18000abb0` の先は読み切っていない。**未確認**）。
3. `.xc` の初期化子 `f_180199d90` が戻ると、その呼び元（`0x18019e349: call 0x1801a59a8` = `_initterm`。`.xc` の表を回す CRT の関数）→ `0x18019e34e: mov [0x1801f1508], 2`（初期化状態）→ `f_18019e1c0` → DllMain の残り。`_pRawDllMain`（`[0x1801b7478]`）は NULL なので呼ばれない（読み。feasibility §6、`0x18019e48d`〜）。**この「戻りの後」の中身（`f_18019e1c0`、`0x18019e367` 以降の `call r9` = `0x1801b7308` の `__guard_dispatch_icall` 経由）は読み切っていない**（未確認）。
4. **DllMain(ATTACH) が戻れば、次の停止はロケール（`GetStringTypeW`、`MultiByteToWideChar` の 0x80 以上。B9）か TLS（B6）か、DllMain の外（`LoadLibraryW` の戻りの後の `GetProcAddress("AggCreateInstance")`、unit 153 の `RegCreateKeyExW`）**になる読み（feasibility §7、§10.3 B13。ここは**この調査の範囲外**で、決定 0091 の調査 §9 が「次の停止以降は未測定」と書いたとおり未測定）。
5. すなわち、**SRW の実装で今の停止点が動いた後に最初に当たりそうなもの**は、この調査の読みの範囲では、SRW・条件変数・スレッド系の**他の名前ではない**。ただし、(i) `.xc` の直接 call の先の中身（`f_1800a9eb0`、`f_180147870` など）の分岐と間接 call（`call r9`、`call rax`、vtable）は追っていない、(ii) ロケール（B9）と TLS（B6）は別の調査が扱う。**実行して確かめるのが確実**。

### 9.1 SRW 以外の 15 名が ATTACH で最初に当たる可能性が高い順（読み。低い見込みを含む）

| 順 | 名前 | 根拠 |
|---:|---|---|
| 1 | AcquireSRWLockExclusive / ReleaseSRWLockExclusive | 実測の停止点。`f_18019caa0` の 1 対 |
| 2 | （SRW・条件変数・スレッドの他の名前） | **ATTACH の直線上では確認できず**（A=0、B のみ。`.xc` は関数のアドレスを表に置くだけ）。描画の初期化（`AggCreateInstance` の後。unit 614 以降）で `f_18019cfa0` などが呼ばれたときに届く読み |
| 3 | IsProcessorFeaturePresent | A で届くが、すべて PF 23 のエラー経路。致命エラーが起きない限り呼ばれない（決定 0078 と同じ） |
| 4 | TlsFree、WaitForSingleObjectEx | A で届くが、前者は DETACH 側か `FlsFree` を引けないとき、後者は別スレッドの初期化中。単一スレッドの通常経路では届かない（§4.1。後者は既存ログで ATTACH の途中まで呼ばれていないことを確認） |
| 5 | OutputDebugStringA、GetFileSizeEx、GetLargePageMinimum | ATTACH では届かない（B のみ・または 0） |

## 10. 限界、未確認

- **何も実行していない**。到達は静的な読みで、**間接 call（`call r8`、`call [rdi+0x18]`、vtable 呼び、`__guard_dispatch_icall`）は 1 つ（`f_18019caa0`）しか確認していない**。`imp14_reach2.py` の A は直接 call だけ、B は `lea` で取られた関数まで。どちらも実行を保証しない。
- **SRWLOCK の中身の読み書き**は、静的な 2 本（`0x1802f4700`、`0x1802f4800`）では「`lea` とゼロの書き込みだけ」を確認したが、**ヒープ上のオブジェクトのロックは確認していない**（構造体の型を復元していない）。
- `Learn` の 19 ページは 2026-10-25 に取得（実測）。**`SRWLOCK_INIT` の値、`ERROR_TIMEOUT` の数値、`CONDITION_VARIABLE_LOCKMODE_SHARED` の値は Learn の該当ページに書かれておらず、他の Learn ページ（`Using Condition Variables`、System Error Codes、`synchapi.h` のリファレンスなど）は取得していない**（確認していない）。実機の Windows での挙動は未確認（ホストは Linux）。非公式の出典は使っていない。
- **実在の Windows の SRW の実装**（ロックが「ポインタ 1 個の値」でどう状態を持つか）は、この調査では使っていない（Learn が内部を書かない）。
- `GetModuleHandleA` の決定 0091 は**未コミット**（ワークツリー）。PM の全体テスト（`lift/out-git/target-m91`、`test-ws-m91.txt`）には触れていない。**行番号は 0091 の変更を含むワークツリーのもの**で、コミット後に変わりうる。
- §8.2 の「失敗する」は**読み**（`grep` と該当行の読み）。実際にテストを走らせていない。
- **縮小 dsp_ippv2 に `TlsFree`、`IsProcessorFeaturePresent` の呼び出し点が含まれるか**は未確認（縮小前の `out-ext` の DLL では含まれる）。
- `OutputDebugStringA`、`GetFileSizeEx` の呼び出し元の鎖が実行で届くかは未確認（B のみ）。`ExitThread` などが戻らないことは Learn に「戻らない」の明記が無い（直後の `int3` からの読み）。
- 既存文書の「ATTACH から静的に届く行なし 15 名 59 か所」（`reach3`。生成 C の即値を辿る）と、この調査の A/B（`.pdata` + 葉の関数分割）は**数え方が違い、同じ数にならない**（§4.1）。**どちらも実行ではない**。
- 3 モジュールの基底（`0x2000000`、`0x4700000`）と種 58 個は測定用の値で、決定ではない（決定 0086）。
- **`f_18019c920` と `f_18019d080` を「終了処理」と読んだ根拠は、`.xc` が `[0x1801f31a8]`、`[0x1801f31f0]`、`[0x1801f3238]` の配列に `0x18019d080` などを書いて `inc` する形（`atexit` / 終了時に呼ぶ関数の登録に見える）だけ**。この配列を誰がいつ呼ぶか（`_initterm` / `atexit` の実装の読み）は追っていない（未確認）。したがって「ATTACH では呼ばれない」は読みの中でも弱い。
- **`.pdata` の関数の範囲が短い断片になっている箇所がある**（例: `0x18019b740` の `.pdata` は 8 バイトだけで、実際の関数はその先まで続く。`imp14_reach2.py` は `.pdata` の範囲と `call` / `lea` の宛先で関数を切るので、**関数の切り方が実際の関数と一致しない可能性がある**）。A / B の数値（251 / 870 関数）は関数分割の方法に依存する（既存の `reach3` と同じ数にならない理由の 1 つ）。**サイトごとの「どの関数の中か」は、`sites-orig.txt` の前後の命令を読んで確認した範囲では正しい**が、鎖（`chain`）の中間の関数名は近似。
- 呼び出し点の表（§4.0）の `f_…` は `.pdata` の開始（`imp14_sites.py`）で、`reach` 系の関数分割とは一致しないことがある（例: `f_180199670` と `f_18019973f`、`f_18019c5e0` と `f_18019c630`）。

## 11. 作ったファイル

- 新規（追跡対象）: `docs/investigations/blend2d-remaining-imports.md`（この文書）。
- 作業物（`lift/out-imp14/`。`.gitignore` の `lift/out*/` に入る）:
  - スクリプト（読み取り専用）: `imp14_sites.py`（呼び出し点の全数）、`imp14_reach.py`（**訂正前の版**。関数分割の誤りがあり数値は使っていない。削除禁止のため残す）、`imp14_reach2.py`（訂正版の静的到達）、`imp14_locks.py`（ロックの置き場）、`imp14_scan_iat.py`（生バイトの IAT 呼び出しの走査）、`imp14_strs.py`（文字列・表の読み）、`learn_text.py`（Learn の HTML → 本文）。
  - 出力: `sites-orig.txt`、`reach-imp14.txt`（訂正前）、`reach-imp14b.txt`（`CS_MODE_64` の import 漏れで失敗した版。内容は空）、`reach-imp14c.txt`（訂正版）、`locks-orig.txt`、`refs-static-locks.txt`、`imports-blend2d.txt`、`scan-fle.txt`、`scan-dsp.txt`、`dis-init.txt`、`dis-init2.txt`、`dis-pool.txt`、`*.err`（空）。
  - `learn/`: Learn の 19 ページの `*.html`（取得物）と本文抽出 `*.txt`。
  - `tmp/`（`TMPDIR`。objdump の中間出力 `fn.txt` が 1 つ。削除禁止のため残す）。
- **変更した追跡ファイルは無い**（`git status --short` / `git diff --stat` は §1 の末尾）。
