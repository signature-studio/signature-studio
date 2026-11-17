# 0081: D4（dsp_ippv2_x64.dll の LoadLibraryExW が引く API セット名）は、仮想モジュールとして「ある」と答える（選択肢 (a)）

状態: PM の決定（2026-10-23）を、作業者（サブエージェント）が記録した。**仮の決定**（実機 Windows 未確認。根拠の限界は下の「仮の決定である点」）。**実装はしない**。実装は W6（決定 0078）の完了後に別作業で行う。事実の出典は `docs/investigations/dsp-ippv2-dynamic-imports.md`（以下「調査」。節番号はその文書のもの）。保留だった決定 0080 の D4 に答える。この記録は調査に無い主張を足さない。

## 決定（PM。変更しない）

- D4（`dsp_ippv2_x64.dll` の `LoadLibraryExW` が引く API セット名への応答）は、選択肢 (a)「仮想モジュールとして『ある』と答える」を採る。
- 対象は次の 4 名。仮想モジュール表 `fle_modules[]` の**末尾（N = 27 以降）**に足す。ハンドル値は決定 0015 の式（`0x7FF800000000 + 0x10000 × N`）と「追加は末尾に限り、並べ替えない」に従う。

  | 名前（`.dll` は runtime の正規化が足す） | DLL が引く API（調査 §3.2） |
  |---|---|
  | `api-ms-win-core-synch-l1-2-0` | `InitializeCriticalSectionEx` |
  | `api-ms-win-core-fibers-l1-1-1` | `FlsAlloc`、`FlsFree`、`FlsGetValue`、`FlsSetValue` |
  | `api-ms-win-core-localization-l1-2-1` | `LCMapStringEx`、`LocaleNameToLCID` |
  | `api-ms-win-appmodel-runtime-l1-1-2` | `AppPolicyGetProcessTerminationMethod` |

  N の割り当て順は実装の作業で決める（この決定は 27 以降の末尾追加だけを定める）。`kernel32` は既に仮想モジュール（N = 9）なので足さない。
- `GetProcAddress` は決定 0015 のとおり。応答表の `dll` 欄がそのモジュール名の行がある API だけ擬似アドレスで答え、無ければ NULL と ERROR_PROC_NOT_FOUND（127）。**`api-ms-*` の下には当面、応答表の行を作らない**。
- したがって DLL は、`GetProcAddress` が NULL になり、CRT の代替に落ちる（調査 §3.2、§4.1。読み。ATTACH で実行されることを実測したのは `InitializeCriticalSectionEx`、`FlsAlloc`、`FlsSetValue` の 3 件で、他は到達が未確認）:
  - `FlsAlloc` 系 → `TlsAlloc` 系
  - `InitializeCriticalSectionEx` → `InitializeCriticalSectionAndSpinCount`
  - `AreFileApisANSI` → 1 を返す（`kernel32` だけを引く呼び出し点。この決定の対象の名前ではない）
  - `LCMapStringEx` → `LCMapStringW`（`LocaleNameToLCID` の照会を経由）
  - `AppPolicyGetProcessTerminationMethod` → STATUS_NOT_FOUND（0xc0000225）

## 選択肢と却下

| | 内容 | 結論 |
|---|---|---|
| (a) | 仮想モジュール表に足す（`LoadLibraryExW` が非 NULL）。応答表の行が無い API は NULL と 127 | **採る** |
| (b) | `fle_load_absent[]` に足す（NULL と 126） | 却下 |
| (c) | 何もせず名前付きで停止のまま | 却下 |

- **(b) の却下理由**: 「この環境に無い」という主張になる。決定 0055 の基準 (c)（環境の定義 0014、Windows 10 22H2 と矛盾しない）を、一次資料で確認できない（調査 §5.3: Learn は個別の `api-ms-win-*` 名が Windows 10 22H2 で `LoadLibrary` により解決できるかを書かない）。
- **(c) の却下理由**: ATTACH が完走しない（調査 §7.2: `api-ms-win-core-synch-l1-2-0` の `LoadLibraryExW` で停止、exit 5）。

## 理由

- 環境について新しく主張する量が少ない。主張するのはモジュールの存在だけで、API の存在は応答表の行ごとに別に決める（決定 0015）。
- 決定 0055 の流儀（環境の定義と矛盾するか確認できないものを「欠いている」とは定義しない）に沿う。
- 調査 §6.2 の実測では、現行の応答表（`kernel32.dll` の行なし）で (a) と (b) は同じ代替に落ちる。違いは、どのモジュール名が `GetProcAddress` の `hModule` として DLL に渡るかと、環境について何を主張するかである（調査 §6.3）。

## 仮の決定である点

- Learn は、個別の `api-ms-win-*` 名が Windows 10 22H2 で `LoadLibrary` により解決できるかを**一次に書いていない**（調査 §5.3）。windows-apisets のページは「スキーマが契約をホストに結び付けているとき、成功する」という一般論だけを書く。上の 4 名が Windows 10 22H2 にあることは、確認できていない。
- `docs/investigations/two-module-design.md` D4 の「実機で `api-ms-win-core-synch-l1-2-0` は Windows 8 以降にある」は、**出典が確認できていない**（調査 §5.3、§8 未確認）。この決定の根拠にしない。
- (a) の「ある」は、Learn が書く API セットの性質（契約名で `LoadLibrary` に渡せる）と矛盾しない、という範囲にとどまる。
- 実機 Windows の実測は無い（ホストは Linux）。
- ハンドル値は決定 0015 の式による選択で、どのページにも値は無い。

## 撤回条件

- 実機 Windows（決定 0014 の Windows 10 22H2）で、`LoadLibraryExW` がこれらの名前で NULL を返すと確認されたとき: 該当の名前を `fle_modules[]` から外し、`fle_load_absent[]` へ移すかを PM が判断する（決定 0055 の撤回条件と同じ形）。表の途中の名前を外すと後ろのハンドルが変わる（決定 0015）ので、その扱いはその時に決める。
- 一次資料が、これらの名前の存在について逆を書いたとき: 同上。
- PM が環境の定義（決定 0014）を変えたとき。

## 帰結（記録のみ。この決定では扱わない）

- ATTACH は、この決定の先で `InitializeCriticalSectionAndSpinCount` と `FlsSetValue`（IAT のスタブ `f_37d7570`）で止まる（調査 §7.3。HEAD の runtime での実測）。これは **W6（決定 0078）の項目**で、D4 とは別。
- 応答表に `kernel32.dll` の行（`InitializeCriticalSectionEx`、`FlsAlloc` など）を足しても、(a) では DLL は `api-ms-*` のハンドルで NULL を受け、`kernel32` を試さない（調査 §2.4 の手順 5: `GetProcAddress` が NULL なら次のモジュールは試さない。§4.4: `kernel32.dll` の行は `api-ms-*` のハンドルでは引かれない）。DLL に非 NULL を渡したい場合は、`api-ms-win-core-synch-l1-2-0.dll` や `api-ms-win-core-fibers-l1-1-1.dll` の `dll` 欄で応答表に行を足す**別の決定**が要る。その場合、行を足すには `fle_imp_*` の実装も要る（調査 §4.3）。

## 実装（この決定ではしない）

- W6（決定 0078）の完了後に、別作業で行う。
- 対象: `lift/runtime/fle_modules.c`、`fle_modules.h`（表の末尾に 4 名）。
- 既存テストの固定数の扱いを含める（調査 §6.2 末尾の確認: `msctf_is_the_26th_virtual_module`、`load.rs` の `N_CRT_STRING` の表、`fle_module_count()` を使う `modules.rs`、`msctf.rs`。数の期待値を持つかは調査時点で未精査）。
- この決定の作業では `lift/` 配下を一切編集していない。

## 限界

- ハンドルは実メモリの住所ではない（決定 0015）。ゲストがヘッダを読むと SIGSEGV になる。
- `api-ms-*` のモジュールは「ある」が中身は空（応答表の行が無い限り、`GetProcAddress` は常に NULL と 127）。実機の API セットが `kernel32` へ転送する挙動とは違う可能性がある（`GetProcAddress` の戻りは一次に無い。調査 §5.3）。
- `LoadLibraryExW` の `dwFlags` = 0x800 と API セット名の組み合わせの扱いは一次に無い（調査 §5.3）。
- `LocaleNameToLCID` の代替（DLL 内の 0x17d9f1c）の中身は未精査（調査 §8）。
- Windows 本体との照合はしていない（ホストは Linux）。
