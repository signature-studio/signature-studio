# 0017: 同期、時間、メモリ、システム情報の応答（単一スレッド、仮想時計）

状態: 作業者（M1、サブエージェント）が決定。依頼者の指示は「判断は作業者側が行え」。実装は `lift/runtime/fle_sys.h`、`fle_sys.c`。仕様の出典と読みは `docs/investigations/import-sys-spec.md`。実行記録は `docs/investigations/sys-run.md`。

## 文脈

閉包（896 seed）には、担当 15 API の未対応 thunk が計 22 か所ある（kernel32 の 14 API と winmm の timeGetTime）。呼び出しは `EnterCriticalSection` 173 箇所、`LeaveCriticalSection` 181 箇所を筆頭に、呼び出し元の関数は 199 件（`import-sys-spec.md` §1）。FLEngine はこれまでの実行で単一スレッドとして動いている（`0004-tls-and-lasterror.md`）。

## 決定

### A. 仮想時計は 1 か所（`fle_sys.c` の `fle_clock_elapsed`）

- 単位は 100 ns の tick。起点は「システム開始から 3,600,000 ms（1 時間）」。`QueryPerformanceFrequency` は 10,000,000（10 MHz）を返す。したがって `QueryPerformanceCounter` は tick 数そのもの。`GetTickCount` と `timeGetTime` は tick 数 / 10,000 の下位 32 ビット。
- 時間が進むのは 2 つの場合だけ。
  1. `Sleep(ms)` は `ms * 10,000` tick 進める。実際には待たない。
  2. 時計を読む呼び出し（QueryPerformanceCounter、GetTickCount、timeGetTime）は、値を返した後に 10 tick（1 µs）進める。時計の値を待つループ（例: GetTickCount が変わるまで待つ）を終わらせるため。
- `Sleep(INFINITE)` は abort。待つ相手のスレッドがなく、永遠に戻らない。
- 起点を 0 にしない理由（推測）: 0 や小さい値は「未初期化」「失敗」と区別しにくい。1 時間は 32 ビットの折り返し（49.7 日）から遠い。起点の値は仕様にない選択。
- 10 MHz を選んだ理由: `Acquiring high-resolution time stamps` が、ハイパーバイザ下で周波数が 10 MHz に固定される、と書く。Windows の実機がこの値だという主張ではない。
- `timeGetTime` と `GetTickCount` が同じ時計を読むこと、`GetTickCount` と QPC が同じ起点を持つことは、ページに書かれていない。整合を保つための選択。

### B. クリティカルセクションは単一スレッドで、仕様どおりの状態を外の表に持つ

- オブジェクト（CRITICAL_SECTION）の中身は読まず書かない（ページは「論理的に不透明、変更するな」と書く）。状態は `fle_sys.c` の表（アドレスをキー、入った回数を持つ。同時に 32,768 個まで）に持つ。
- `Initialize` は表に登録（入った回数 0）。`Enter` は回数を 1 増やす（所有中の再入は待たない）。`Leave` は 1 減らす。`Delete` は登録を外す。
- abort にする場合（ページが未定義、または永遠に待つ）:
  - 初期化されていない（または削除済みの）セクションへの Enter、Leave、Delete。
  - 初期化済みのセクションへの Initialize。
  - 入っていないセクションへの Leave（ページは「別スレッドの Enter が永遠に待つことがある」と書く）。
  - 所有中のセクションの Delete。
  - NULL 引数。
  - 最初に呼んだスレッドと別のスレッドからの呼び出し（他者が保持している可能性があり、Enter なら待つことになる。TLS の持ち主スレッドの規則と同じ）。
- 他者が保持して待つ場面は、スレッドが 1 つなので起きない。起きる唯一の経路（別スレッド）は abort で止める。

### C. メモリ

- VirtualAlloc: 予約はホストの `mmap(PROT_NONE, MAP_NORESERVE)`。コミットは `mprotect`。領域の状態（ページごと）は表で持つ。指定アドレスの予約は `MAP_FIXED_NOREPLACE`（既存の割り当てを壊さない）。NULL アドレスは割り当て粒度（65536）に整列した基準を返す。
- VirtualFree: MEM_RELEASE は `munmap`。MEM_DECOMMIT は `madvise(DONTNEED)` と `PROT_NONE`（再コミット時に 0 になる。ページの MEM_COMMIT の保証）。
- 仕様が定める失敗は NULL / 0 と GetLastError。コードが書かれている場合（ERROR_INVALID_ADDRESS 487）はそれを使う。書かれていない場合のコード（予約済み範囲への予約は 487、メモリ不足は 8、VirtualFree の失敗は 87）は選択。
- 仕様が定めない入力は abort: MEM_RESET、MEM_RESET_UNDO、MEM_LARGE_PAGES、MEM_PHYSICAL、MEM_TOP_DOWN、MEM_WRITE_WATCH、フラグなし、dwSize 0、WRITECOPY 系と修飾子（PAGE_GUARD など）、複数の保護値、領域外または複数領域にまたがる MEM_DECOMMIT。
- 「MEM_COMMIT のみ、アドレス NULL」は予約とコミットを同時に行う。ページの MEM_COMMIT の説明はこの組を明示しないが、ゲストの 3 か所（0x40769f、0x40779b、0x408847。`rcx=0, r8=0x1000, r9=4`）がこの形で呼ぶ。推測（ゲストが依存する形に合わせた）。
- ヒープ: GetProcessHeap は固定のトークン 0x7FF700000000 を返す（何もマップしない）。HeapAlloc と HeapFree は LocalAlloc と同じホストの `malloc`/`calloc`/`free`。HeapCreate は呼ばれないので実装せず、他のハンドルは abort。
- GetSystemInfo: AMD64、ページ 4096、プロセッサ 1 個（マスク 1）、タイプ 8664、粒度 65536、レベルとリビジョン 0（CPUID が型番を主張しないことと揃える。0009）。アドレス範囲は 0x10000〜0x7FFFFFFEFFFF（一般に知られた値。このページ群では確認していない）。

### D. ゲストのアドレス空間と arena の確認（恒等写像）

- 既存の設計: `lift/harness/` のメモリは恒等写像（`fle_ld`、`fle_st` がホストポインタを直接使う。`0003-localalloc.md`）。DLL は ImageBase 0x400000 にマップ（`harness/image.c`）。スタックは `mmap(NULL, 16 MB)` で、実行では 0x7c…、0x76… のような高いアドレス。専用の arena はない（`lift/runtime/` と `docs/` を検索。`arena` は 0 件）。
- 判断: VirtualAlloc は新しい arena を作らず、ホストの `mmap` が返す領域を恒等写像のアドレスとしてそのまま返す。LocalAlloc、スタックが既にホストの高いアドレスをゲストに返しているのと同じ扱い。
- 根拠（実測）: `out-sys/run-entry-4087e0.txt` で VirtualAlloc の戻りが 0x71232a170000 台（`rax=0x71232a170001`、下位バイトは setne の結果）。ゲストのコードは、返されたアドレスを 64 ビットのまま使っている（0x407783 の `and rdi, 0xffffffffffff0000` と 0x4076bd の 64 ビット格納）。
- 未確認（推測）: DLL 全体でポインタを 32 ビットに切る箇所があるかは調べていない。あれば VirtualAlloc の戻りで壊れる。ただし TlsSetValue に渡す LocalAlloc のポインタ（0x41d041）も同じ高いアドレスで、これまでの実行で問題は出ていない。
- 制限: ハーネスは `RLIMIT_AS` を 4 GB に絞る（`boot.h`）。それを超える予約は NULL と 8 で失敗する。

## 代替案

- 時計: 実時間（`clock_gettime`）を使う案は、実行ごとに結果が変わり、Sleep が実際に待つことになるため却下。固定値を返す案は、時間を測るループが終わらないため却下。
- クリティカルセクション: 全部 no-op にする案（旧調査の shim の方針）は、状態の違反（初期化していないものの Enter、Leave の数の不一致）を見逃す。依頼が「仕様どおりの状態を保持する」ことを求めたため却下。
- VirtualAlloc: `calloc` で返す案は、予約と保護（PROT_NONE で読み書きを止める）を表現できず、保護のテストができないため却下。

## 限界

- スレッドは 1 つ。別スレッドは abort。待つ処理（WaitForSingleObject など）は担当外で未実装。
- 時計は決定的。実行間でも同じ値になる。
- DeleteCriticalSection は閉包で呼ばれない（実測 0 件）。実装は仕様の読みとテストのみで、実行で確かめていない。
- GetSystemInfo の割り当て粒度とアドレス範囲の数値は、Microsoft のページで確認していない。
- 時計を読むたびの 10 tick は、ゲストの測定値（フレーム時間など）に現れる。ゲストがどう使うかは見ていない。
