# 0092: SRW ロック 4 名と条件変数 3 名を、単一スレッドの状態機械（runtime 側の表）で実装する（仮の決定）

状態: **仮の決定**。PM の決定を、作業者（サブエージェント）が記録した。本文の日付は作業日の 2026-10-25（`date` の値。依頼の文脈の日付は 2026-10-24）。決定の節と背景は実装の前に保存し、「実装した場所」「テスト」「検証」「解釈が分かれた点」「限界」は実装と検証の後に書いた（すべて記入済み）。

事実の出典（節番号は各文書のもの）:

- `docs/decisions/0091-getmodulehandlea.md`（コミット a64a41c）の「検証」の最後の行。3 モジュール構成（FLEngine + 縮小 dsp_ippv2 基底 0x2000000 + Blend2D 基底 0x4700000。種 58 個あり。基底と種は測定用の値で決定ではない）の native の停止は `fle: unsupported instruction at 0x489cab9`（top of stack `f_489caa0`）。IAT スロット `0x48b7098` = `KERNEL32!AcquireSRWLockExclusive`（原本 `Blend2D.dll` の VA は `0x1801b7098`。調査 §4.0 の表。基底 0x4700000 では `0x4700000 + 0x1b7098 = 0x48b7098`。依頼文の「元の VA `0x1801f7098`」は算術が合わず、調査の値を採った）。
- `docs/investigations/blend2d-remaining-imports.md`（以下「調査」）§4（呼び出し点、ロックの置き場、引数）、§5（Learn の引用）、§6（単一スレッドの下での整理）、§7（選択肢）、§8（影響範囲、更新が要る既存テスト）。
- 決定 0017（B: クリティカルセクション。状態を表に持つ、中身は読まず書かない、別スレッドと持たない Leave は abort）、0050（イベント。INFINITE と有限・非 0 の未シグナル待ちは停止、仮想時計の窓口は足さない）、0078（応答表に行を足す前例。末尾に足す）、0080 D9（スレッドは現行のまま）、0015（応答表の擬似 VA は末尾に足す）、`docs/investigations/sync-thread-survey.md` の案 C（単一スレッドの状態機械、スレッド生成に届いたら名前付き停止）。

## 決定（PM。内容を変えない）

対象は次の 7 名**だけ**: `AcquireSRWLockExclusive`、`ReleaseSRWLockExclusive`、`AcquireSRWLockShared`、`ReleaseSRWLockShared`（SRW）、`WakeConditionVariable`、`WakeAllConditionVariable`、`SleepConditionVariableSRW`（条件変数）。他の行なし import（`CreateThread`、`ExitThread`、`FreeLibraryAndExitThread`、`GetFileSizeEx`、`IsProcessorFeaturePresent`、`OutputDebugStringA`、`TlsFree`、`WaitForSingleObjectEx`、`GetLargePageMinimum`、`WaitOnAddress` 系）には触らない。

| # | 項目 | 決定 |
|---|---|---|
| 1 | 状態の置き場 | SRW ロックの状態は **runtime 側の表**（`fle_sys.c`。アドレスをキー。決定 0017 のクリティカルセクションの表と同じハッシュ表の形）に持つ。**ゲストの SRWLOCK の 8 バイトを runtime は読まず書かない**（Learn: 「treat it as logically opaque」「The process must not modify the object」）。表の大きさ・衝突の扱いは 0017 の流儀に従い、満杯なら名前付き停止 |
| 2 | 未登録アドレスの最初の Acquire | **空のロックを取る**（`SRWLOCK_INIT` はゼロで、`InitializeSRWLock` を呼ばない使い方を Learn が許すが、値を書かない。**仮の選択**） |
| 3 | Acquire | 空なら取る（Exclusive は排他、Shared は共有 1）。**Exclusive が取られているロックへの Exclusive／Shared、共有で取られているロックへの Exclusive は名前付き停止**（Learn: deadlock。単一スレッドでは永遠に戻らない）。**共有で取られているロックへの Shared は個数を 1 増やして取る**（共有同士は許される。共有のみの再帰は Learn が should not と書くだけで禁止とは書かない。**仮の選択**） |
| 4 | Release | 取られているロックを**同じモード**で解放する（Exclusive: 空に戻す。Shared: 個数を 1 減らし、0 になったら空）。**ロックを持たない Release（未登録、空、またはモードの食い違い）は名前付き停止**（Learn が結果を書かない。決定 0017 の `LeaveCriticalSection` の前例の類推。**仮の選択**）。空になった表の項は、次の Acquire で再利用できる（登録は取得中だけでよい。実装は 0017 の流儀） |
| 5 | 別スレッド | 別スレッドからの呼び出しは決定 0017 の `fle_sys_check_thread` と同じ規則で名前付き停止（wasm では `fle_host_tid()` が定数。決定 0045） |
| 6 | `WakeConditionVariable` / `WakeAllConditionVariable` | 待機者がいない（単一スレッドでは常にそう）ので**何もしない**（VOID。`R[0]` を触らない）。**信号は残さない**（Learn が待機者なしの動作を書かないので**仮の選択**。決定 0050 の `SetEvent` の信号が残る流儀は適用しない）。CONDITION_VARIABLE の 8 バイトは読まず書かない |
| 7 | `SleepConditionVariableSRW` | **全入力で名前付き停止**。INFINITE: 起こす者がいないので永遠に戻らない（Learn: 「The thread can be woken using the WakeConditionVariable or WakeAllConditionVariable function.」）。有限と 0: 戻すには仮想時計の窓口（決定 0050 §C が足さないとした）と `ERROR_TIMEOUT` の値（Learn の本ページに無い。System Error Codes は未取得）とロックの再取得の扱い（Learn が書かない）が要るので、今は停止。停止メッセージに `dwMilliseconds` と `Flags` の値を出す。ロックが空（または未登録）のときは Learn が「undefined」と明記するので、同じく停止（停止メッセージで区別する） |
| 8 | 応答表 | `lift/crates/fe-lift/src/imports_table.rs` の**末尾**に 7 行（kernel32.dll。`jmp_imp:` と `call_imp:` の書式は既存の行に倣う）。末尾に足すので既存の擬似 VA は動かない（決定 0015）。宣言は既存のヘッダの書式。新しい `.c`、`.h` は作らない（`fle_sys.c` に足す） |
| 9 | 状態 | **仮の決定**（2、3 の共有の再帰、4 の持たない Release、6 の Wake、7 の有限・0 の扱いが Learn に無い）。撤回条件は下に (a)〜(d) |

## 背景（事実。調査の節番号）

- **停止点**（0091 の検証、調査 §1 の 2、§4.1）: 3 モジュール構成（種 58 個あり）の停止は `fle: unsupported instruction at 0x489cab9`。`f_18019caa0`（原本の VA。リベース後 `f_489caa0`）は `.xc` の初期化子 `f_180199d90` が `0x18019a1d2: call r8` で呼ぶ関数で、`this` は静的な `0x1802f47d0`、SRWLOCK は `this+0x30`（`0x1802f4800`）。**ATTACH で確実に実行される SRW はこの Acquire 1 回と Release 1 回の対だけ**（調査 §4.1。実測は Acquire まで、Release は読み）。
- **Blend2D の SRW・条件変数の呼び出し点の全数**（調査 §4.0。実測: pefile と capstone の線形走査）: Acquire Exclusive 20、Release Exclusive 27（うち `jmp [IAT]` が 3）、Acquire Shared 3、Release Shared 6、Wake 4、WakeAll 1、Sleep 4。**Release の 3 か所が `jmp [IAT]`** なので、応答表の行は `jmp_imp:` と `call_imp:` の両方の形で使われる。
- **ロックの置き場**（調査 §4.2。読みと実測）: Blend2D は `InitializeSRWLock`、`InitializeConditionVariable`、`TryAcquireSRW*` を import しない（実測）。静的な SRWLOCK 2 本（`0x1802f4700`、`0x1802f4800`）は、`.text` の参照が `lea` と `.xc` の 1 回のゼロ書き込みだけで、**ゲストは値を読み書きしない**（実測）。ヒープ上のロック（スレッド 1 個のオブジェクトの `+0x80` など）の中身をゲストが読むかは**未確認**。
- **Sleep の引数**（調査 §4.2。読み）: 4 か所とも `Flags` = 0（排他）。`dwMilliseconds` は INFINITE が 3、有限（計算値。ミリ秒の値は未確認）が 1（`f_18019c920`。`atexit` 風の終了処理と読める。ATTACH では呼ばれない読みだが弱い）。3 か所は述語が偽のときだけ Sleep する `while` の形。
- **`WaitOnAddress` との関係**（調査 §4.2）: `.xc` が `GetProcAddress` で引く 3 つがすべて非 NULL のときだけ `[0x1801f31a0] = 1`。この環境は NULL を返す（決定 0091 の実測）ので 0 → スレッドプールのスレッドは SRW + 条件変数版になる（読み）。今回の停止点のロック（プールの `this+0x30`）は両方の版で使う。この決定は `WaitOnAddress` に触らない。
- **Learn の取得**: この作業で 2026-10-25 に取得した（下の「Learn の引用」。調査 §5 が取得した 19 ページのうち、この決定が使う 10 ページを再取得して引用を確認した）。

## Learn の引用（URL つき。この作業で取得して確認した。英語の原文）

URL はすべて `https://learn.microsoft.com/en-us/windows/win32/` の下。

- `sync/slim-reader-writer--srw--locks`（`updated_at` 2025-04-15）:
  - 「Exclusive mode SRW locks cannot be acquired recursively. If a thread tries to acquire a lock that it already holds, that attempt will fail (for TryAcquireSRWLockExclusive) or deadlock (for AcquireSRWLockExclusive)」
  - 「Shared mode SRW locks should not be acquired recursively as this can lead to deadlocks when combined with exclusive acquisition.」
  - 「When the lock has been acquired in exclusive mode, no other thread can access the shared resource until the writer releases the lock.」
  - 「…very little state information can be stored, so SRW locks do not detect incorrect recursive use in shared mode. In addition, a thread that owns an SRW lock in shared mode cannot upgrade its ownership of the lock to exclusive mode.」
  - 「The caller must allocate an SRWLOCK structure and initialize it by either calling InitializeSRWLock (to initialize the structure dynamically) or assign the constant SRWLOCK_INIT to the structure variable (to initialize the structure statically).」
- `api/synchapi/nf-synchapi-initializesrwlock`（2025-08-27）: 「An SRW lock cannot be moved or copied while in use. The process must not modify the object, and must instead treat it as logically opaque. Only use the SRW functions to manage SRW locks.」「An unlocked SRW lock with no waiting threads is in its initial state and can be copied, moved, and forgotten without being explicitly destroyed.」
- `api/synchapi/nf-synchapi-acquiresrwlockexclusive`、`…-acquiresrwlockshared`（2025-08-27）: 戻り値は「None」。ページは取られているロックでの動作（ブロックする）を書かない。
- `api/synchapi/nf-synchapi-releasesrwlockexclusive`、`…-releasesrwlockshared`（2025-08-27）: 「The SRW lock must be released by the same thread that acquired it.」。**ロックを持たずに Release した場合の結果は書かない**。
- `api/synchapi/nf-synchapi-wakeconditionvariable`、`…-wakeallconditionvariable`（2025-08-27）: 「The WakeAllConditionVariable wakes all waiting threads while the WakeConditionVariable wakes only a single thread. Waking one thread is similar to setting an auto-reset event, while waking all threads is similar to pulsing a manual reset event but more reliable」。戻り値は「None」。**待機者がいないときの動作は書かない**。
- `sync/condition-variables`（2025-03-11）: 「You can wake other threads using WakeConditionVariable or WakeAllConditionVariable either inside or outside the lock associated with the condition variable.」
- `api/synchapi/nf-synchapi-sleepconditionvariablesrw`（2025-10-31）: 「If dwMilliseconds is zero, the function tests the states of the specified objects and returns immediately. If dwMilliseconds is INFINITE, the function's time-out interval never elapses.」「If the timeout expires the function returns FALSE and GetLastError returns ERROR_TIMEOUT.」「If the lock is unlocked when this function is called, the function behavior is undefined.」「The thread can be woken using the WakeConditionVariable or WakeAllConditionVariable function. After the thread is woken, it re-acquires the lock it released when the thread entered the sleeping state.」。**`ERROR_TIMEOUT` の数値、`SRWLOCK_INIT` と `CONDITION_VARIABLE_INIT` の値、`CONDITION_VARIABLE_LOCKMODE_SHARED` の数値、タイムアウトで戻るときのロックの再取得は書かない**。

## 範囲

- 7 名の実装（`lift/runtime/fle_sys.c`）、応答表の末尾 7 行、宣言（`fle_sys.h`）。
- 3 モジュール構成の**測定のみ**（次の停止点の記録。新しい停止は直さない）。

## 範囲外（今回扱わない）

- 他の行なし import（調査 §7.2 の `CreateThread`、`ExitThread`、`FreeLibraryAndExitThread`、`GetFileSizeEx`、`IsProcessorFeaturePresent`、`OutputDebugStringA`、`TlsFree`、`WaitForSingleObjectEx`、`GetLargePageMinimum`）と `WaitOnAddress` 系（調査 §7.2 の c1）。決定 0050 の本文（`WaitForSingleObjectEx` を実装しない根拠）も変えない。
- `InitializeSRWLock`、`InitializeConditionVariable`、`TryAcquireSRWLockExclusive/Shared`、`SleepConditionVariableCS`（Blend2D は import しない。応答表に行を作らない）。
- 仮想時計の窓口、`ERROR_TIMEOUT` の値、タイムアウトで戻るときのロックの再取得（決定 0050 §C のとおり足さない）。
- ヒープ上のロックの中身をゲストが読むかの確認（調査 §4.2 の未確認）。

## 仮の点（Learn が書かない動作を作る）

| # | 仮の点 | 何を選んだか | Learn が書かないこと |
|---|---|---|---|
| a | 未登録アドレスの最初の Acquire（決定 2） | 空のロックとして取る | `SRWLOCK_INIT` の数値、`InitializeSRWLock` を呼ばない使い方での登録の扱い（ページは静的初期化の使い方を許すが値は書かない） |
| b | 共有で取られているロックへの Shared（決定 3） | 個数を 1 増やして取る | 共有のみの再帰の結果（「should not … when combined with exclusive acquisition」とだけ書く） |
| c | ロックを持たない Release（決定 4） | 名前付き停止 | 違反時の結果。決定 0017 の `LeaveCriticalSection` の類推（別の API のページの文に基づく） |
| d | 待機者のいない Wake / WakeAll（決定 6） | 何もしない。信号は残さない | 待機者なしの動作。ページは Wake を「auto-reset event を set するのに似る」と書くので、信号が残る読みも可能（イベントの `SetEvent` はページが「待機者がいなければ信号のまま残る」と明記するが、Wake にはその文が無い） |
| e | `SleepConditionVariableSRW` の有限と 0（決定 7） | 名前付き停止（値を返さない） | `ERROR_TIMEOUT` の数値、タイムアウトで戻るときのロックの再取得 |

## 撤回条件

- **(a)** 実機 Windows 10 22H2 で、静的初期化（ゼロ）のロックの最初の使用が「空」として扱われないことが一次資料で確認されたとき（決定 2。未登録アドレスの最初の Acquire を停止に変えるか、別の初期状態にする）。
- **(b)** 実機または Learn が、共有で取っているスレッドが同じロックを共有でもう一度取ることを禁止（デッドロックまたは失敗）と示したとき（決定 3 の共有の個数の増加を停止に変える）。
- **(c)** 実機または Learn が、ロックを持たない Release（空のロック、他のスレッドが持つロック、モードの食い違い）の動作を示し、決定 4 の停止と食い違うとき（その動作に置き換える。例: 何もしない、または未定義でない結果が決まっている）。
- **(d)** 実機または Learn が、待機者のいない Wake / WakeAll の動作を示し、決定 6 の「何もしない・信号を残さない」と食い違うとき（例: 次の Sleep が即座に起きる）。
- 追加（記録）: (e) Learn が `ERROR_TIMEOUT` の値、タイムアウトで戻るときのロックの再取得を書いたとき、または仮想時計の窓口を足す決定が出たとき、決定 7 の有限・0 の停止を値を返す実装に置き換えてよい（未決）。(f) PM が環境の定義（決定 0014）または案 C（単一スレッド）を変えたとき。

## 実装した場所（記入済み）

- `lift/runtime/fle_sys.c`: クリティカルセクションの節の直後に新しい節（`/* ---- Slim reader/writer locks and condition variables (decision 0092, provisional) ---- */`）。
  - 状態: `fle_srw_table[65536]`（`fle_cs_hash` を共有。同時に 32,768 本まで。満杯は `fle: more than 32768 SRW locks held at once` の名前付き停止）。**項は取得中だけ**（`shared == 0` は排他、`shared == n` は共有 n 個）。未登録のアドレスは空のロック。空けた項は、次の項が EMPTY なら EMPTY（直前の DELETED の連なりも EMPTY）、そうでなければ DELETED にして、探索の連鎖を切らず、再利用できるようにした。
  - Acquire Exclusive: 未登録なら排他で登録。登録済みは名前付き停止（排他中は「already held exclusively」、共有中は「held in shared mode (n acquisitions)」）。Acquire Shared: 未登録なら共有 1、共有なら +1（`UINT32_MAX` で停止）、排他なら停止。Release Exclusive／Shared: 同じモードで登録済みなら項を空ける（Shared は個数を 1 減らし 0 で空ける）。未登録とモードの食い違いは停止（メッセージで区別）。NULL は 6 名とも停止。別スレッドは `fle_sys_check_thread(api)`（決定 0017 と同じ関数）。
  - Wake、WakeAll: `fle_sys_check_thread` と NULL の検査だけ。`R[0]` は触らない。信号は残さない。
  - Sleep: 全入力で名前付き停止。メッセージ `fle: SleepConditionVariableSRW(ConditionVariable %#llx, SRWLock %#llx, dwMilliseconds %u, Flags %u): the lock is held exclusively|in shared mode; <理由>`（ロックが未登録のときは「the lock is not held (never acquired, or released); the page says the behavior is undefined」）。`dwMilliseconds` と `Flags` は `R[8]`、`R[9]` の下位 32 ビットを 10 進で出す（Flags は解釈しない）。理由の文言は INFINITE、0、有限の 3 通り。
  - ゲストの 8 バイトは読まず書かない（`fle_ld`、`fle_st`、`fle_addr`、`memcpy` を使わない）。コメントに Learn の引用と「assumption」を書いた。
- `lift/runtime/fle_sys.h`: 7 関数の宣言（`fle_imp_SleepConditionVariableSRW` は `__attribute__((noreturn))`。`fle_imp_HeapSize` の前例）。
- `lift/crates/fe-lift/src/imports_table.rs`: `TABLE` の**末尾**（`GetModuleHandleA` の次）に 7 行（`AcquireSRWLockExclusive`、`ReleaseSRWLockExclusive`、`AcquireSRWLockShared`、`ReleaseSRWLockShared`、`WakeConditionVariable`、`WakeAllConditionVariable`、`SleepConditionVariableSRW`）。擬似 VA `0x7ff900001060`〜`0x7ff9000010c0`。既存の擬似 VA は動かない。他の行は触っていない。
- 新しい `.c`、`.h` は作っていない。決定 0017、0050、0078 の本文は書き換えていない。

## テスト（記入済み）

`cargo test --workspace`: **98 binaries / 1,195 passed / 0 failed / 0 ignored**（基準 97 / 1,178 / 0 / 0。**+1 binaries**（新規 `srw.rs`）、**+17 件**。SKIPPED 0 件）。

- **新規 `lift/crates/fe-lift/tests/srw.rs`（17 件）**: 生成器の `jmp [IAT]`（7 名、擬似 VA の位置と不変、`GetModuleHandleA` の直後に 7 行）、`call [IAT]` の形、空→Acquire Exclusive→Release Exclusive の往復、未登録アドレスの最初の Acquire、Shared の複数取得と個数の解放、Exclusive／Shared 中の取得の停止、持たない Release（空、未登録、解放済み、モードの食い違い）の停止、NULL の停止、別スレッドからの停止（7 名）、表の満杯（32,768 本まで取れて 32,769 本目で停止。並べ替えた順の解放後に再利用）、**同じハッシュの連鎖**（真ん中の解放が後ろのロックを隠さない）、Wake／WakeAll が何もしない（`R[0]` と 8 バイトが変わらない）、Sleep の全入力（INFINITE、有限、0、ロックなし、解放後、NULL）の停止とメッセージの値、**ゲストの 8 バイトを PROT_NONE のページに置いて runtime が読み書きしない**、新しいコードの禁止語（`fle_ld`、`getenv`、`clock_gettime` ほか）、`GetProcAddress` と `fle_dispatch`、Blend2D の形（静的ロックの `call [IAT]` の Acquire と `jmp [IAT]` の Release、`lea rcx,[rbx+0x30]` の形）の往復と解放しない関数の 2 回目の停止。
- **既存テストの書き換え（1 件。件数は増減しない）**: `lift/crates/fe-lift/tests/modules.rs` の `get_module_handle_a_has_a_response_row_…`（旧名 `…_at_the_end_…`）。PM の指示どおり「`GetModuleHandleA` が応答表の最後の行」という表明を外し、「全行の擬似 VA が `base + 0x10 × (位置 + 1)`、`GetModuleHandleA` の行が 1 つあり位置 n で `base + 0x10 × n`、`GetModuleHandleW`（`0xc0`）と `GetProcAddress`（`0xd0`）が前のまま、`GetModuleHandleW`、`GetProcAddress`、`GetModuleHandleExW` が `GetModuleHandleA` より前」に書き換えた。新しい最後の行（`SleepConditionVariableSRW`）の位置は固定しない。
- **他に失敗した既存テストは無かった**（`IsProcessorFeaturePresent`、`TlsFree`、`WaitForSingleObjectEx` に関するテストは触っていない）。
- 変異検査（自己検査。決定の根拠ではない）: Wake が `R[0] = 0` を書く作業コピーで 3 件が落ち、空けた項の連鎖を切る作業コピーで衝突の専用テストが落ちた。元に戻して全件通過。

## 検証（記入済み。実測値と手順は `docs/investigations/blend2d-srw.md`。作業物は `lift/out-srw/`）

| 項目 | 結果 |
|---|---|
| `cargo test --workspace` | **98 binaries / 1,195 passed / 0 failed / 0 ignored**（上のとおり） |
| **FLEngine の生成 C の SHA-256** | **変わった**: `eb9d8e7fa74ae562742fb1999f6da9b3ca82f2884d1845b5b7319153e1b709cf` → **`feb5b9087f8d7a8a1282900212d7bd1f5b6071b0326e7149642f98c168e8080f`**（215,707,705 B → 215,709,232 B）。変わったのは応答表の 5 か所 32 行だけ（`fle_imp_*` のプロトタイプ 7 行、`fle_dispatch_table` の末尾 7 行、`fle_dispatch_count` 59558 → 59565、`fle_resp_table` の末尾 7 行、`fle_resp_count` 261 → 268）。**変わった関数は 0 個**（その 5 か所を除くと新旧の C はバイト単位で同一。`diff` は 32 行）。functions 59,297、bad_nodes 0、unsupported_total 525、register_jmp_sites 20、jumptabs 4、msvc_tables 0、msvc_rejected 0 は不変。原因は「応答表に行を足したこと」だけ（FLEngine は 7 名を import していない）。**基準の SHA を固定するテストは無い**。今後の基準は `feb5b9087f8d…` |
| 単独構成の a4（新しい OUT で runtime の .o を最初から作り直した） | **145 / 942**、functions 821、entries 15,685、停止 `LoadLibraryW "dsp_ippv2_x64.dll"`。基準と一致。2 回同一 |
| 3 モジュール（種 58 個あり、基底 0x4700000。測定のみ） | SRW の 1 対（通し番号 1704 `AcquireSRWLockExclusive`、1705 `ReleaseSRWLockExclusive`、rcx `0x49f4800` = `f_489caa0` の `this+0x30`）が**停止せず戻った**。**次の停止は `fle: indirect call to 0x48a20ff is not a known function entry`**（top of stack `f_48a2060`。`abort kind: fle_dispatch_miss`）。a4: init_count 150/942（不変）、last unit 149（proc `0x664260`）、functions_distinct 1,069（前回 1,059）、function_entries 27,332（前回 27,319）、ATTACH 内の import 183 回（前回 178。1526〜1703 は前回と同一、後ろに SRW の 2 つと `GetSystemInfo`、`HeapAlloc`、`InitializeCriticalSection`）。DllMain は戻らず、LoadLibraryW は 0x4700000 を返さない。分類 **(v) その他**（生成器が解決しない MSVC の switch 表 `jmp r9`（`0x48a208b`、`msvc_rejected: N not derived`）。`memcpy` 風の 3 バイトコピーの表の項 `0x48a20ff` が既知の入口でない。0091 の時点から拒否されていた同じ 77 件の 1 つ）。2 回同一。unit 158 には届いていない |

## 解釈が分かれた点（記入済み）

詳細は `docs/investigations/blend2d-srw.md` §7。決定の内容は変えていない。実装できなかった項目は無い。

1. **停止メッセージの区別**（決定 7 は「ロックが空（または未登録）のときは停止メッセージで区別する」）: 実装では空と未登録を区別しない。表が持つのは取得中のロックだけ（決定 4 の「登録は取得中だけでよい」）なので、「空」と「未登録」は同じ状態（項が無い）である。メッセージは「the lock is not held (never acquired, or released)」で両方を指す。有限・0・INFINITE（ロックが取られている）の 3 通りは文言で区別する。
2. **共有で取られたロックへの Sleep の `Flags`**: Learn は `Flags` が `CONDITION_VARIABLE_LOCKMODE_SHARED` のときロックは共有で保持されていなければならないと書き（「This lock must be held in the manner specified by the Flags parameter」）、食い違いの結果は書かない。決定 7 は「全入力で名前付き停止」なので、`Flags` の値とロックのモードの食い違いも停止（解釈しない）。停止メッセージにロックのモード（「exclusively」「in shared mode」）と `Flags` の値を両方出し、どちらが違うかは読み手が見る。
3. **共有の個数の上限**: 決定 3 は個数を 1 増やすとだけ書く。`uint32_t` の個数がオーバーフローするときは名前付き停止にした（`UINT32_MAX`。現実の経路では起きない）。
4. **表の満杯の閾値**: 決定 1 は「0017 の流儀」。`FLE_CS_MAX_LIVE`（スロット数の半分 = 32,768）と同じ割合を同時に取得中のロック数の上限にした（クリティカルセクションの表とは別の表）。登録が取得中だけなので、実際に同時に取る本数がこの数に届くことは、Blend2D の呼び出し点（同時に高々数本）からは起きない見込み（読み。未確認）。
5. **擬似 VA と `GetModuleHandleA` のテスト名**: PM の指示は「`modules.rs` の表明を書き換える」。関数名の「`_at_the_end_`」は表明の内容と合わなくなるので `…_after_the_existing_ones_…` に変えた（`#[test]` の件数は同じ。関数名は他から参照されない）。
6. **依頼文の「元の VA `0x1801f7098`」**: 依頼文の背景に `IAT スロット 0x48b7098 = AcquireSRWLockExclusive（元の VA 0x1801f7098）` とあるが、基底 0x4700000 から 0x48b7098 を引き戻すと `0x1801b7098` で、調査 §4.0 の表（`0x1801b7098`）と合う。`0x1801f7098` は算術が合わないので、調査の値を使った（実害の無い数字の食い違い。決定ファイルの背景に注記した）。
7. **`SleepConditionVariableSRW` の `noreturn`**: 決定は触れない。`fle_imp_HeapSize` の前例に倣って宣言に付けた（実装が必ず `STOP` する）。生成 C は `{ void fle_imp_SleepConditionVariableSRW(void); ... }` と自前で宣言するので、`noreturn` の有無は生成 C の見た目を変えない。

## 限界（記入済み）

- **実機の Windows での挙動は未確認**（ホストは Linux）。仮の点 (a)〜(e) は Learn が書かない動作を作っている。
- ヒープ上のロックの中身をゲストが読むかは**未確認**（調査 §4.2、§10。静的な 2 本は `lea` とゼロの書き込みだけと実測）。ゲストが 8 バイトを読む経路があれば、runtime が書かない状態とゲストが読む値が食い違いうる。
- 3 モジュールの測定は ATTACH の 1708 件目の import の後までで、SRW の他の 5 名（Shared の Acquire/Release、Exclusive の他の呼び出し点）、Wake、WakeAll、Sleep は**1 回も呼ばれていない**。これらの実行での動作（特に Sleep の名前付き停止がどこで起きるか）は合成バイト列のテストだけで確かめた。
- `WaitOnAddress` 系は答えていない（決定 0081、0091 のまま NULL）。`[0x48f31a0]` を読む側の経路は追っていない。決定 0092 は SRW/条件変数版のスレッドプールの経路を選ぶ（読み。実行して確かめていない）。
- `CreateThread` ほかの行なし import（`IsProcessorFeaturePresent`、`TlsFree`、`WaitForSingleObjectEx`、`GetFileSizeEx`、`OutputDebugStringA`、`GetLargePageMinimum`）は触っていない。FLEngine の `CreateThread`、`ExitThread`、`TlsFree` の thunk は `fle_unsupported` のまま（変えていない）。
- 次の停止（`0x48a208b` の `jmp r9`）は**直していない**（測定のみ）。3 モジュールの基底（`0x2000000`、`0x4700000`）と種 58 個は測定用の値で、決定ではない。
