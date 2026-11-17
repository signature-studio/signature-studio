# 0034: システム情報の応答（GetNativeSystemInfo ほか 6 API。ヘッドレス・仮想環境の設定値）

状態: 作業者（サブエージェント）が決定。実装は `lift/runtime/fle_sysinfo.h`、`fle_sysinfo.c`、応答表への追記は `lift/crates/fe-lift/src/imports_table.rs`。テストは `lift/crates/fe-lift/tests/sysinfo.rs`。実測と出典は `docs/investigations/sysinfo-run.md`。同期系と GetSystemInfo の前提は `0017-sync-time-memory.md`、応答表の方式は `0015-virtual-modules.md`。

## 文脈

all-pdata の harness は 0x41cfa0（`RaiseException(0xc06d007f)`）で止まっていた（`pdata-seeds-run.md` §5.3）。直前の `GetProcAddress(kernel32, "GetNativeSystemInfo")` が、応答表に行が無いため NULL（ERROR_PROC_NOT_FOUND）を返し、DLL の遅延ロードのヘルパが例外を上げる経路に入っていた。

## 決定 A: 実装する API（6 件。すべて kernel32、DLL の IAT に実在する）

DLL の import 表（`lift/out-imports/imports.txt`、normal 738 + delay 57）にある名前のうち、システム情報の近縁で未対応だったもの。

| API | DLL 内の IAT | 呼び出し元の thunk | 備考 |
|---|---|---|---|
| GetNativeSystemInfo | delay 0x1976390 | 0x4236f0（呼び出し 2 か所: 0x45413e、0x115d144） | 遅延ロードの thunk。GetProcAddress 経由で到達 |
| GetLogicalProcessorInformation | delay 0x19762d0 | 0x407320（呼び出し 4 か所: 0x409a07、0x409a42、0x41052d、0x410555） | 同上 |
| IsWow64Process | delay 0x1976358 | 0x425a80（呼び出し 1 か所: 0x84333b） | 同上 |
| GlobalMemoryStatus | normal 0x19717d8 | 0x4238d0（呼び出し 1 か所: 0x12622b3） | `jmp [IAT]` の thunk として処理される |
| GetProcessAffinityMask | normal 0x1971638 | 0x423690（呼び出し 3 か所） | 同上 |
| SetProcessAffinityMask | normal 0x1971680 | 0x423bf0（呼び出し 9 か所） | 同上 |

GetProcessAffinityMask / SetProcessAffinityMask の呼び出し元（計 12 か所）のうち、objdump で周囲を読んだ 8 か所（0xca9947〜0xca996f、0xcae26c〜0xcae279、0xcb1c2c〜0xcb1c54、0xcaa2fc〜0xcaa30b、0xcaa446〜0xcaa455、0xcaeba0〜0xcaebaf、0xcaecca〜0xcaecd9、0xcb25d6〜0xcb25e5）では、`call 0x423440`（`GetCurrentProcess` の thunk、IAT 0x19715b8）の戻り値を rcx に使う形だった。残りの呼び出し元は読んでいない。したがって渡されるハンドルは GetCurrentProcess の擬似ハンドルだけと見ているが、これは推測（全箇所の確認と実行での確認はしていない）。擬似ハンドル以外が来たら実装は abort する（決定 C）。

実装は 1 つのファイルに置いた（`fle_sysinfo.c`）。`GetSystemInfo` は既にある（`fle_sys.c`）ので重複させない。GetNativeSystemInfo は `fle_imp_GetSystemInfo` を呼ぶだけで、構造体の中身を二重に持たない。

## 決定 B: 仕様から決まる値と、決まらない値（設定値）

### 仕様（Microsoft Learn）から決まるもの

- `GetNativeSystemInfo`: 「64 ビットのアプリから呼ぶと `GetSystemInfo` と同等」。構造体は `SYSTEM_INFO`（48 バイト）。戻り値は VOID。
- `SYSTEM_INFO`: `wProcessorArchitecture` AMD64 = 9、`dwProcessorType` PROCESSOR_AMD_X8664 = 8664、`dwPageSize` は VirtualAlloc のページ（4096、ホストのページと一致を `fle_sys.c` が確認）、`dwActiveProcessorMask` は「bit 0 が プロセッサ 0」。
- `GetLogicalProcessorInformation`: BOOL。バッファが足りれば TRUE、`*ReturnedLength` = 返したバイト数。足りなければ FALSE、`GetLastError` = ERROR_INSUFFICIENT_BUFFER（122）、`*ReturnedLength` = 必要な長さ。構造体 `SYSTEM_LOGICAL_PROCESSOR_INFORMATION` は x64 で 32 バイト（ULONG_PTR 8 + enum 4 + パディング 4 + 共用体 16）。関係の値は ProcessorCore 0、NumaNode 1、Cache 2、ProcessorPackage 3。ProcessorMask は「少なくとも 1 ビットが立つ」。NUMA でないシステムは 1 つのノードを報告する（ノード番号 0）。Cache 記述子は Level、Associativity、LineSize、Size、Type（Unified 0、Instruction 1、Data 2、Trace 3）。
- `IsWow64Process`: 「64 ビットのアプリが 64 ビット Windows 上」は FALSE。成功は 0 以外。
- `GlobalMemoryStatus` / `MEMORYSTATUS`: x64 で 56 バイト（DWORD 2 つ + SIZE_T 6 つ）。`dwLength` は関数が設定する。`dwMemoryLoad` は 0〜100。`dwTotalVirtual` は「プロセスの仮想アドレス空間のユーザーモード部分の大きさ」。戻り値は VOID。
- `GetProcessAffinityMask`: 成功は 0 以外。プロセスのマスクはシステムのマスクの部分集合。`SetProcessAffinityMask`: 「システムに構成されていないプロセッサを要求すると ERROR_INVALID_PARAMETER」（87）。成功は 0 以外。

### 仕様で決まらないもの = 「ヘッドレス・仮想環境の設定値で、実機の値ではない」

次の値はどのページにも書かれていない。この環境の設定であり、どの実機の値でもない。変えるときは `fle_sysinfo.h` の定数（と `fle_sys.c` の `GetSystemInfo`）だけを変える。

| 値 | 設定 | 場所 |
|---|---|---|
| 論理プロセッサ数、アクティブマスク、システム親和マスク、プロセス親和マスクの初期値 | 1 個、マスク 1 | `FLE_SYSINFO_PROC_MASK`（`fle_sysinfo.h`）。`GetSystemInfo` の値（`fle_sys.c`）と同じ |
| `dwAllocationGranularity` | 65536 | `fle_sys.c`（0017 で決定済み。ページに数値なし） |
| `lpMinimumApplicationAddress`、`lpMaximumApplicationAddress` | 0x10000、0x7FFFFFFEFFFF | `fle_sys.h`（0017 で決定済み。ページに数値なし） |
| `wProcessorLevel`、`wProcessorRevision` | 0、0 | `fle_sys.c`（0017、CPUID が型番を主張しないこと〔0009〕と揃える） |
| 物理メモリ | 合計 4 GiB、利用可能 2 GiB | `FLE_MEMSTATUS_*` |
| コミット上限（`dwTotalPageFile`、`dwAvailPageFile`） | 8 GiB、6 GiB | 同上 |
| `dwTotalVirtual`、`dwAvailVirtual` | ユーザー範囲全体（`max - min + 1`）、同じ値 | 同上（GetSystemInfo の範囲から導く） |
| `dwMemoryLoad` | 50（物理 2 つの値から導いた値と一致） | 同上 |
| キャッシュ | L1 データ 32 KiB 8-way、L1 命令 32 KiB 8-way、L2 統合 256 KiB 8-way、L3 統合 8 MiB 16-way、ライン 64 バイト | `fle_sysinfo.c` の `fle_imp_GetLogicalProcessorInformation` |
| レコードの構成と並び | コア、NUMA ノード 0、L1 データ、L1 命令、L2、L3、パッケージの 7 件（224 バイト）。マスクはすべて 1 | 同上。ページは「並びは呼び出しごとに変わりうる」と書く。ここでは常に同じ |
| ProcessorCore の Flags | 0（機能ユニットを共有しない） | 同上 |

メモリの値は追跡しない（呼び出しごとに同じ値）。ページは「volatile」と書くが、値が変わらないことは禁じていない。

### プロセッサ数を 1 にする理由

- FLEngine は単一スレッドで動く前提（0017: 同期系、クリティカルセクションは単一スレッドの状態だけを持つ。別スレッドからの呼び出しは abort）。2 個以上を報告すると、ゲストが worker スレッドを作って待つ経路に進む可能性がある。それは 0017 の範囲で abort になる（CreateThread、WaitForSingleObject は未実装）。1 個なら「並列化しない」経路に入ると読めるが、これは推測（DLL の使い方は見ていない）。
- 既存の `GetSystemInfo` が 1 個（0017 の決定）。同じ情報を返す API がずれないように、`FLE_SYSINFO_PROC_MASK` を一箇所にして、GetNativeSystemInfo は GetSystemInfo の出力をそのまま使い、GetLogicalProcessorInformation のマスクと GetProcessAffinityMask のシステムマスクも同じ値にした。テスト `logical_processor_information_records_follow_the_pages` が、論理プロセッサ数（コアのレコードのマスクのビット数）と `dwNumberOfProcessors`、システムマスクと `dwActiveProcessorMask` の一致を確かめる。
- ホストの CPU 数を返す案は、実行環境によって結果が変わるので却下（`sysconf`、`sched_getaffinity` などを使わないことをテストで確認）。

## 決定 C: エラーと「止める」場合

- `GetLogicalProcessorInformation`: 足りなければ FALSE、122、必要な長さ。`Buffer` が NULL でも長さが足りないときは同じ（サイズ問い合わせ）。成功時は `*ReturnedLength` を返したバイト数にする（入力が大きくても）。成功は last error に触れない（ページに書かれていない）。
- `IsWow64Process`: ハンドルは GetCurrentProcess の擬似ハンドルだけ。成功時 `*Wow64Process`（BOOL、4 バイトだけ書く）は FALSE。
- `SetProcessAffinityMask`: 構成されていないプロセッサを含むマスクは FALSE と 87（ページ）。
- 止める（`fle_stop` で abort、メッセージは `fle: <api>: <理由>`。ページが定義しない入力を成功扱いにしない）: 出力ポインタ、`ReturnedLength`、`Buffer`（長さが足りるとき）が NULL、擬似ハンドル以外のハンドル、空のアフィニティマスク（ページに記述なし）、別スレッドからの呼び出し（0017 と同じ規則）。

## 決定 D: GetProcAddress の応答表

`fle_modules.c` は API 名を持たず、`fle_resp_table` は生成器が `imports_table.rs` の TABLE から出す（0015 §5）。したがって応答表への追加は TABLE の末尾への 6 行の追記だけで行った。`fle_modules.c` は編集していない。テーブルの行数は固定値でテストしていない。「応答表にない名前は NULL と ERROR_PROC_NOT_FOUND（127）」の仕様は変えていない。

行を足すと、0015 §6 が書いたとおり、ゲストの経路が変わる。この作業では実際に変わった（実測は `sysinfo-run.md` §5）。

## 決定 E: 実装しなかったもの

| 名前 | 理由 |
|---|---|
| GetLogicalProcessorInformationEx、GetNumaHighestNodeNumber、IsProcessorFeaturePresent、GlobalMemoryStatusEx、GetPhysicallyInstalledSystemMemory、IsWow64Process2、GetActiveProcessorCount、GetMaximumProcessorCount | DLL の import 表（normal と delay）に無い。DLL 内の ASCII 文字列にも無い（`grep -a`）ので、GetProcAddress で問われる証拠も見つからなかった。実装すると応答表を増やすだけで経路を変える（0015 §6）。呼ばれない API は足さない |
| GetSystemTimes | import 表に無い。文字列 `GetSystemTimes` が DLL 内に 1 つ（ファイルオフセット 0xdacbc、近くに `GetCPUUsage`、`PrevSystemTimes` の文字列がある）。`name_xrefs.py`（NUL 終端の文字列を探して rip 相対参照を調べる）は、この名前では一致が 0 件で、参照の調査は行われなかった（文字列が NUL ではなく長さバイトの後ろに続く形のため、推測）。したがって参照の有無は未確認で、何に使われる文字列かも不明 |
| GetComputerNameW、GetComputerNameExW | import 表にあるが、依頼の列挙（システム情報の近縁）に含まれない。値（ホスト名）は設定値になるが、ページを読んでいない。未対応のまま（`jmp [IAT]` の未対応が各 1、ExW の呼び出し元は 26 か所） |

## 限界

- キャッシュの構成、メモリ量は実機の値ではない。ゲストがこれらで機能を切り替える（例: キャッシュサイズでバッファの大きさを決める）なら、その経路は実機とは違う。DLL がどう使うかは見ていない。
- `GetLogicalProcessorInformation` を応答表に入れたため、ゲスト（0x4099c0、0x4104f0）は「この API は有る」と判断して遅延ロードの thunk（0x407320）を呼ぶ。遅延ロードのヘルパは、GetProcAddress の戻り値（擬似 VA）へ `jmp rax` で飛ぶ。この `jmp rax` は、今の生成器では未対応（`fle_unsupported`）。実測は `sysinfo-run.md` §5。範囲外のため実装していない。
- GetNativeSystemInfo も同じヘルパ（0x423120）を使うので、この名前だけでも同じ `jmp rax` に到達する（反事実の実測、`sysinfo-run.md` §5.3）。
- Windows 本体との照合はしていない（ホストが Linux）。
