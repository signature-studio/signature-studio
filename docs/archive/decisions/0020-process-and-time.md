# 0020: プロセス・スレッドの単純な応答と、エミュレート時刻・タイムゾーン

状態: 作業者（M1、サブエージェント）が決定。依頼者の指示は「判断は作業者側が行え」。実装は `lift/runtime/fle_proc.h/.c`、`fle_time.h/.c`。仕様の出典と確認状況は `docs/investigations/import-proctime-spec.md`。実行記録は `docs/investigations/proctime-run.md`。

## 決定

### 1. 時刻の源は 1 つ（仮想時計）。起点の日時とタイムゾーンは環境の設定で、仕様ではない

- 現在時刻（UTC）は、`fle_sys.c` の仮想時計（決定 0017。100 ns の tick）から作る。UTC の FILETIME = `FLE_TIME_BOOT_FILETIME` + tick。ホストの実時刻は読まない。同じ入力は同じ出力を返す。
- `FLE_TIME_BOOT_FILETIME` は 2020-12-31T23:00:00Z（132539292000000000）。仮想時計は「システム開始から 1 時間」で始まる（0017）ので、**最初に時刻を読むと 2021-01-01T00:00:00Z** になる。これは `fle_vfs.h` の仮想ファイル時計の開始値（FLE_VFS_CLOCK_START = 2021-01-01T00:00:00Z）と同じにした（ファイルの時刻と現在時刻が食い違わないように）。依頼文の例（2026-01-01 を tick 0 に対応）は採らなかった。理由: 既存の仮想ファイル時計が 2021-01-01 を採っており、2 つの日付があると、ゲストが作ったファイルの時刻が「未来」になる。**この日時は環境の設定で、どの仕様にもない**。変えるときは `fle_time.h` の 1 か所。
- 読み出しは `fle_imp_QueryPerformanceCounter` を内部で呼ぶ形で行う。1 つの時計に、1 つの進め方（読み出しごとに 10 tick = 1 µs、Sleep の分）が適用される。GetTickCount64 も同じ時計を読む（GetTickCount と同じ起点・同じ進め方。ページはこの一致を書かない。整合のための選択）。
- **同じ型の 1 つ目の副作用**: 時刻を読むと時計が進む。ゲストが連続して GetLocalTime を呼ぶと、1 µs ずつ異なる値が返る（ミリ秒の欄は 10 回の読み出しごとに 1 変わる）。
- タイムゾーンは UTC（bias 0、DST なし）。GetTimeZoneInformation は bias 0、StandardName "UTC"、DaylightName 空、遷移日なし（wMonth 0）、戻り値 TIME_ZONE_ID_UNKNOWN（0）を返す。ローカル時刻 = UTC。**環境の設定で、仕様ではない**。
- 遷移日（DST の規則）を持つ時間帯構造体が SystemTimeToTzSpecificLocalTime に渡されたら止める（実装していない。固定 bias として扱うと誤る）。

### 2. ExitProcess、TerminateProcess、RaiseException は止める（abort）

- 実行がこの 3 つに来たことの意味は「プログラムが終了（または異常終了）を要求した」。戻ると実行が続いてしまい、静かな成功の終了にすると、要求があったことが隠れる。
- そのため `fle_stop`（メッセージ `fle: <API>: ...` を出して `abort()`、SIGABRT）で止める。メッセージに終了コード（ExitProcess、TerminateProcess）または例外コード・フラグ・引数の数（RaiseException）を含める。TerminateProcess は、擬似ハンドル以外が渡されたら「他のプロセスは存在しない」旨で止める。
- RaiseException は、ページに従い、ハンドラが見つからない場合の既定が ExitProcess の呼び出しなので、同じ扱い。SEH はモデル化していない（ゲストが `__try` のハンドラ経由で処理する例外は、この API を使わない）。
- ホストのプロセスを `exit()` で終わらせる案は、テストやハーネスの停止点の記録を失うため採らなかった。

### 3. 単純なもの

- GetCurrentProcess は -1（ページ）。GetCurrentThread は -2（**仮定**。ページは値を書かない）。GetCurrentProcessId は 0x1000（環境の設定。ホストの pid は使わない。決定性のため）。
- SetLastError は `fle_set_last_error`（既存の最終エラー）に下位 32 ビットを書く。
- SetErrorMode、SetThreadErrorMode はページの値の集合と戻り値（直前のモード）、SEM_NOALIGNMENTFAULTEXCEPT の保持を実装。一覧にないビットは止める。
- SetPriorityClass、SetThreadPriority は、擬似ハンドルだけを受け、ページの値の集合を検査し、background モードの重複・不在で 402/403、400/401 を返す。それ以外の値と、擬似ハンドル以外のハンドルは止める。
- SwitchToThread は常に 0（他のスレッドがない）。100 万回を超えたら止める（実装の選択）。
- OutputDebugStringW は何もしない（ページ）。

### 4. 実装しなかったもの

- GetDateFormatW の大部分: ゲストの 2 か所は flags = 4（DATE_USE_ALT_CALENDAR）で "gg"（era）、"yy"、"yyyy" を求める形に読める（仕様ノート §3.4）。実装は flags 0 と数値の picture に限る。ゲストの経路は止まる。era の文字列、日と月の名前が仕様にない。
- ページにない値（GetCurrentThread の値など）は仮定として明記した。

## 代替案

- 時刻の起点を実時刻（ホスト）にする案: 実行ごとに結果が変わるので却下。
- 時刻の読み出しで時計を進めない案: 時刻を待つループ（時刻が変わるまで回る）が終わらないので却下。
- ExitProcess を成功で返す（何もしない）案: 終了要求を隠すので却下。

## 限界

- 照合は MSDN のページと、テスト内の独立のカレンダー式（Hinnant の days_from_civil）だけ。Windows 実機との比較はしていない。
- 時刻は 1601〜30827 年の範囲。範囲外の変換は失敗（番号 87 は選択）か、止まる。
- 現在時刻の起点の日付（2021-01-01）は、ゲストの表示に現れる可能性がある。
