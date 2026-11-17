# import の使用状況（閉包 896 件、IAT 経由の未対応 jmp の集計）

作業: 作業者（M1）。日付: 2026-10-20。
成果物: `lift/out-usage/`（集計ツールと出力。下記の一覧を参照）。
生成器と runtime は変更していない。読み取りのみ。

## 1. 結論（要約）

- IAT 経由の未対応 jmp（`Jmp: indirect through IAT slot (not lifted)`）は **435 件**（実測）。gen-896.txt の 435 と一致する。
- その 435 件は **390 種類の slot** に分かれる。内訳は通常 import 390、delay import 0。
- 435 件のうち、対応表（`imports_table.rs` の 6 件）にある slot への jmp はない。
- import slot は全 795 件（通常 738、delay 57）。対応表にない slot は 789 件（実測）。
- import を使う関数（呼び出し元）は **763 件**（実測）。うち対応表にない import を使う関数は 743 件、対応表にある import だけを使う関数は 52 件（実測）。

## 2. 数え方と注意

- C の `Jmp` 行は 455 行（gen-896.txt の 435 + 20 に対応）。このうち同じアドレスが別の関数に重複して出ているものがあり、ユニークなアドレスは 447 件。
- ユニークな IAT 経由の未対応 jmp は 435 件。以下の件数はこれを基準にする。
- 各 jmp の slot は objdump テキストの `jmp QWORD PTR [rip+...] # 0xSLOT` から引いた（実測）。依頼文の `IAT 0x1571658` は RVA 表記で、VA は `0x1971658`（imports.txt と objdump で一致）。
- objdump テキストの読み取り範囲は 0x11fb4af まで。範囲外の thunk（0x1375030〜0x1375090、7 件）は、DLL を `objdump -d --start-address=0x1375000 --stop-address=0x1375100` で読み取り専用に抜いて補った（`lift/out-usage/extra-objdump-0x1375000.txt`）。
- 初回の集計（`aggregate_iat.py` 第 1 版）は、この 7 件を取りこぼして 428 件になった。第 3 版（`-v3` の出力）で 435 件に一致した。
- 呼び出し元の件数は推測を含む。関数の判定は次の 3 種類を数えた。(a) C 内の `call f_stub`、(b) objdump の `call QWORD PTR [rip+slot]`、(c) objdump の `jmp rel32` による tail call。間接 call（レジスタ経由）は数えていない（推測）。
- 「jmp 数」と「call 数」は別に数えてあり、同じ関数の重複は除いた（呼び出し元の件数は関数単位）。

## 3. 上位 40 件（未対応 IAT jmp の件数順）

| 順位 | import 名 | DLL | slot VA | 件数 |
|---|---|---|---|---|
| 1 | SendMessageW | user32.dll | 0x1970c48 | 3 |
| 2 | SystemParametersInfoW | user32.dll | 0x19710d0 | 3 |
| 3 | FreeLibrary | kernel32.dll | 0x1971370 | 3 |
| 4 | CompareStringW | kernel32.dll | 0x19713f8 | 3 |
| 5 | SetCurrentDirectoryW | kernel32.dll | 0x1971598 | 3 |
| 6 | GetProcAddress | kernel32.dll | 0x1971600 | 3 |
| 7 | GetLocaleInfoW | kernel32.dll | 0x19716f8 | 3 |
| 8 | CharNextW | user32.dll | 0x1970ba0 | 2 |
| 9 | DrawTextW | user32.dll | 0x1970bd8 | 2 |
| 10 | DispatchMessageW | user32.dll | 0x1970e28 | 2 |
| 11 | GetWindowThreadProcessId | user32.dll | 0x1970f18 | 2 |
| 12 | InvalidateRect | user32.dll | 0x1970ff0 | 2 |
| 13 | GetWindow | user32.dll | 0x19710d8 | 2 |
| 14 | RegOpenKeyExW | advapi32.dll | 0x19711f0 | 2 |
| 15 | RegQueryValueExW | advapi32.dll | 0x1971218 | 2 |
| 16 | RegCloseKey | advapi32.dll | 0x1971230 | 2 |
| 17 | GetACP | kernel32.dll | 0x1971278 | 2 |
| 18 | LocalFree | kernel32.dll | 0x1971298 | 2 |
| 19 | GetModuleHandleW | kernel32.dll | 0x1971368 | 2 |
| 20 | GetModuleFileNameW | kernel32.dll | 0x19713c8 | 2 |
| 21 | GetVersion | kernel32.dll | 0x1971450 | 2 |
| 22 | Sleep | kernel32.dll | 0x1971520 | 2 |
| 23 | EnterCriticalSection | kernel32.dll | 0x1971530 | 2 |
| 24 | GetTickCount | kernel32.dll | 0x1971550 | 2 |
| 25 | InitializeCriticalSection | kernel32.dll | 0x19715a8 | 2 |
| 26 | VirtualAlloc | kernel32.dll | 0x19715d0 | 2 |
| 27 | GetSystemInfo | kernel32.dll | 0x19715e8 | 2 |
| 28 | LeaveCriticalSection | kernel32.dll | 0x19715f8 | 2 |
| 29 | GetVersionExW | kernel32.dll | 0x1971618 | 2 |
| 30 | VerifyVersionInfoW | kernel32.dll | 0x1971620 | 2 |
| 31 | VerSetConditionMask | kernel32.dll | 0x1971658 | 2 |
| 32 | FindFirstFileW | kernel32.dll | 0x1971660 | 2 |
| 33 | QueryPerformanceCounter | kernel32.dll | 0x19716a8 | 2 |
| 34 | WideCharToMultiByte | kernel32.dll | 0x19716c8 | 2 |
| 35 | FindClose | kernel32.dll | 0x19716d8 | 2 |
| 36 | GetEnvironmentVariableW | kernel32.dll | 0x1971728 | 2 |
| 37 | WriteFile | kernel32.dll | 0x1971750 | 2 |
| 38 | IsValidLocale | kernel32.dll | 0x19717a0 | 2 |
| 39 | GetFileVersionInfoSizeW | version.dll | 0x19709f0 | 1 |
| 40 | VerQueryValueW | version.dll | 0x19709f8 | 1 |

上位 40 件の件数の合計は 85 件（実測）。全体は 435 件、390 slot。

## 4. 全体の件数（実測）

- 未対応 IAT jmp の総数: **435 件**（ユニークなアドレス）。
- slot の種類数: **390**（通常 390、delay 0）。
- 件数が 1 の slot: 352。件数が 2 以上: 38。
- DLL 別の slot 数（未対応のもの）: user32 133、kernel32 128、gdi32 54、advapi32 14、winmm 11、oleaut32 11、ole32 7、crypt32 7、wininet 6、shell32 6、comctl32 4、version 3、その他 6。

## 5. 対応表（imports_table.rs）との関係（実測）

- 対応表は 6 件: GetCurrentThreadId、TlsAlloc、LocalAlloc、TlsSetValue、TlsGetValue、GetLastError（kernel32）。
- 閉包の中で対応表の thunk（`fle_imp_*` への jmp）は 10 件。呼び出し箇所は全体で 64 件。
- 対応表にない未対応 IAT jmp: 435 件（対応表の slot への未対応 jmp は 0 件）。
- import slot の全体: 795 件（通常 738、delay 57）。
- 対応表にある slot: 6 件。**対応表にない slot: 789 件**（実測）。
- 対応表にない 789 件のうち、閉包から jmp も call もされていない slot は 399 件、何らかの jmp/call がある slot は 390 件（実測）。

## 6. 呼び出し元（import を使う関数）

- import を使う関数: **763 件**（実測。C の関数 8427 件のうち）。
- うち対応表にない import を使う関数: 743 件（実測）。
- うち対応表の import を 1 つ以上使う関数: 52 件（実測。対応表にない import も使う関数と重なるため、「対応表だけを使う関数」の数ではない。その件数は未集計）。
- 集計の定義（推測を含む）: 呼び出し元は、閉包内の `call` 先が import の thunk、または thunk への tail jmp を含む関数。間接呼び出しは含まない。

## 7. 残りの作業量（推測を含む）

- 未対応 IAT jmp 435 件のうち、対応表で処理できるのは 0 件（すべて対応表外の slot）。
- 対応表の追加は、各 import の runtime 関数（`runtime/fle_imports.c`）の実装を伴う。件数の多い順に実装すると、上位 40 件で閉包内の未対応 jmp の一部を解消できる（実装量は推測。未見積もり）。
- 呼び出し元 763 件の多くは、import 以外の未対応命令（Movsx、Idiv など）にも依存している可能性がある（推測。`gen-896.txt` の全体の未対応は 1617 件）。
- 未対応の間接 jmp（`Jmp: indirect target unresolved`）は 20 行（ユニーク 12 件）。import 解決とは別の問題。

## 8. 出力ファイル（lift/out-usage/）

- `aggregate_iat.py`: 集計ツール（現在のディスク上の版。v3 の結果を出したもの。第 1 版・v2 の出力は、この版より前のコードで作られた）。
- `run-aggregate.log`、`run-aggregate-v2.log`、`run-aggregate-v3.log`: 実行ログ（v3 が最終）。
- `extra-objdump-0x1375000.txt`: 範囲外の thunk の逆アセンブル（読み取り専用の抜き出し）。
- `iat-jmp-unsupported-all-v3.tsv`: 未対応 IAT jmp の全件（jmp アドレス、slot、DLL、名前）。
- `iat-slot-counts-v3.tsv`: slot ごとの件数（順位つき、391 行）。
- `unhandled-imports-usage-v3.tsv`: 対応表にない 789 件の slot、件数、呼び出し数、呼び出し元の関数数。
- `import-callers-v3.tsv`: 呼び出し元の関数ごとの import 一覧（763 行）。
- `summary-v3.txt`: 集計の要約。
- 旧版: `iat-jmp-unsupported-all.tsv`、`iat-slot-counts.tsv`、`unhandled-imports-usage.tsv`、`import-callers.tsv`、`summary.txt`（第 1 版。取りこぼしがあるため参照しない）、および `*-v2.*`（不完全）。

## 9. 失敗と注意

- `cd lift/out-cpuid-run` が作業ディレクトリの誤りで失敗した（ワークディレクトリが既に lift だった）。出力への影響はない。
- 第 1 版・第 2 版の集計は取りこぼしがあった（7 件）。第 3 版で修正し、gen-896.txt と一致した。旧版の出力は残している（既存のものを上書きしていない）。
- 作業中に rm、既存ファイルの上書き、git 操作、/tmp の使用はしていない。
