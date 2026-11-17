# 0056: ゲストに見せるスレッド ID は固定の仮想値（FLE_EMULATED_TID = 0x1001）

状態: **仮の決定**。PM の方針（「ホストの実環境値をゲストに見せない。GetCurrentThreadId の値は native でも wasm でも同一の固定の仮想値にする」）に従い、作業者（サブエージェント）が値を選んだ。値の選び方は仕様から決まらない。決定 0001（native は実 tid を返す）と決定 0045 a（wasm は定数 1、native と値が違ってよい）の、ゲストが見る値の部分を置き換える。ホスト側の `fle_host_tid()`（単一スレッド規則、決定 0017）は変えない。調査と実行記録は `docs/investigations/virtual-thread-id.md`。

## 文脈

- native と wasm（node）で同じ生成 C を実行すると、停止点（`0x424ee0`、user32!GetSystemMetrics の thunk、関数実行数 572）は一致するが、入口数が native 13,227、wasm 13,230 で 3 件ずれた（`msctf-caller.md` §6.1）。差は入口列の 13,159 番目。
- 原因（読みと、native 側の gdb 実測）: `GetCurrentThreadId` が native では Linux の tid（6 桁の 16 進数）、wasm では定数 1（1 桁）を返す。ゲストの `f_43aa10`（16 進数の文字列化。桁数 = `edx` を 4 ビットずつ右に送って数える）が桁数を `f_43a890` → `f_414ad0` の長さに渡し、`f_414ad0` は「長さが同じなら何もしない」（0x414b04）か「再確保する」（0x414b32 の `call 0x409470`）に分かれる。**wasm 側の分岐は gdb で追っていない**。
- ゲストが戻り値を使う箇所（静的な読み。`objdump-text.txt`）: `0x406f90`（thunk）の呼び出しは 8 か所（0x410638、0x4108b8、0x4109c1、0x410e78、0x410e9e、0x4121c7、0x4128e0、0x41cee4）。0x4108b8・0x4109c1・0x410e9e は、所有者として `[obj+8]` に保存する（再帰ロックの形）。0x410638 と 0x410e78 は `[obj+8]` との比較、0x4128e0 は `[0x15eaddc]` との比較。0x4121c7 は `[r13+0x34]` に保存。0x41cee4 は `[0x15e6080]` に保存。どの箇所も、値を**別の API の引数**として渡す形ではなく、保存と比較（等しいか）に使う。文字列化（`f_43aa10` の呼び出し元 10 件以上）に届く経路は、gdb で `edx == gettid` を見た 2 回だけを確認している（`msctf-caller.md` §6.1）。他の呼び出し元に値が届くかは未確認。

## 決定

1. `fle_imp_GetCurrentThreadId` は、native でも wasm でも、固定の `FLE_EMULATED_TID = 0x1001`（4097）を `R[0]` に返す。実装は `lift/runtime/fle_imports.c`、定数は `fle_imports.h`。引数は読まない。ホストの tid は返さない。
2. 値の選び方（仕様から決まらないので選択）:
   - `FLE_EMULATED_PID`（0x1000、`fle_proc.h`、決定 0020）の次の値にした。ゲストが ID を DWORD として扱うだけなので 32 ビットに収まる。0 でない。
   - プロセス ID と同じ値にしない理由は、ゲストが両者を取り違えても気づけないことを避けるため（Learn にこの要求はない。作業者の選択）。
3. `fle_host_tid()`（`fle_host.h`）は変えない。単一スレッド規則（9 ファイルの owner、決定 0017、0050、0053、0054）の持ち主の確認に使い続ける。wasm では定数 1（0 にしない。owner 変数が 0 を「未記録」にするため）。
4. TEB: ゲストの TEB（`fle_vex.h` の `fle_teb`）は 0x30（Self）と 0x58（ThreadLocalStoragePointer）の 2 つだけをモデル化している。ClientId（x64 で UniqueProcess 0x40、UniqueThread 0x48）は**モデルしていない**。ゲストの `gs:` 参照は、逆アセンブルで `gs:0x58`（0x41d14b）の 1 件だけを確認した（オペランドとして解釈できない byte 列は除く）。したがって、今は GetCurrentThreadId と TEB の ID を合わせる必要はない。TEB の ClientId を将来モデルするときは、同じ `FLE_EMULATED_TID`（と `FLE_EMULATED_PID`）を使うこと（下の拡張点）。
5. GetCurrentProcessId は既に固定値（`FLE_EMULATED_PID`、host を見ない。`getpid` を使わない）。**触っていない**。

## 選択肢

| 案 | 内容 | 採否 |
|---|---|---|
| A | 現行 wasm の 1 に native を合わせる | 不採用。1 はゲストの文字列化で 1 桁になり、実 Windows の ID の桁数から遠い（Learn は値を定めない。桁数が実 OS に近いことを求める根拠はないが、1 は「最小の値」でゲストの `0` や `1` を特別扱いする分岐を通すおそれがある。推測）。 |
| B | 別の固定値（0x1001） | **採用**。PID の隣で、桁数が 4 桁。 |
| C | 実 tid を決定論的に写像する（例: 最初に見た tid → 0x1001、次の tid → 0x1002） | 今は不採用。単一スレッドなので B と同じ値になる。スレッドを増やす将来には使える（拡張点）。写像の表がホストの値の出現順に依存するので、「最初に見る tid」の決まり方が実行ごとに変わりうる。 |
| D | 比較から入口数を除く | 不採用。差の原因を残し、tid を使う経路が増えるたびに他の差が出る。 |

## 仕様の出典（確認状況）

- 出典: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentthreadid （2026-10-21 に取得）。
- 読んだ内容: Syntax は `DWORD GetCurrentThreadId();`。Return value は「呼び出しスレッドのスレッド識別子」。Remarks は「スレッドが終了するまで、その識別子は system 全体でそのスレッドを一意に識別する」。要件は Windows XP 以降、Kernel32.dll。
- この出典から言えること: 戻り値は DWORD、呼び出しスレッドを（終了までの間）一意に識別する。
- 言えないこと: 値の範囲、0 を返さないこと、桁数、Windows 内の ID の割り当ての規則。**Learn に書かれていない経験則（Windows のスレッド ID が 4 の倍数になりがち、など）は使っていない**。0 を返さないことも Learn には無く、0 を避ける理由は本リポジトリの内部事情（owner slot が 0 を「未記録」とする。決定 0017 の各ファイル）にある。ただし GetCurrentThreadId の戻り値自体は owner slot には入らない（owner slot には `fle_host_tid()` が入る）ので、**0 を避ける必要は厳密にはない**。0 にしないのは、ゲストが 0 を「未設定」に使う場合（上の 0x4108b8 の `[obj+8]` は所有者 0 を「いない」とみる形に見える。読み）と衝突しないための選択。
- 値が全スレッドで一意であること（Remarks）は、スレッドが 1 つの間は 1 つの値で満たす。

## 結果（実測）

- 生成 C は変えない（`lift/out-msc/out` の 17 パーツを複製して再利用）。runtime だけが変わるので runtime 36 ファイルと harness を再コンパイルしてリンクした。
- native と wasm（node）の停止点 `0x424ee0`、関数実行数 572、入口数 **13,230**、呼び出し深さ 10 で、`rsp=` の行を除いて出力が完全に一致した。それぞれ 2 回実行して同じ。詳細は `docs/investigations/virtual-thread-id.md`。
- 13,230 は、以前の wasm（tid = 1）と同じ値。1 桁と 4 桁で同じ経路に入ったと読めるが、ゲストの分岐そのものは今回も追っていない（推測）。

## 撤回条件

- Learn または一次資料が GetCurrentThreadId の値の範囲、または PID との関係（同じ名前空間で重ならない、など）を定めていると分かったとき: 値を見直す。
- ゲストの文字列化（`f_43aa10` など）の結果が、4 桁の 16 進数で不都合を出したとき（例: 実 Windows の ID の桁数に依存する分岐が見つかったとき）: 値を変える。変える場所は `FLE_EMULATED_TID` の 1 行と、テスト（`tests/tid.rs`、`wasm_port.rs` の 4097）。
- PM が方針（ホスト値を見せない）を変えたとき: 決定 0001 に戻す。

## 将来の拡張点（今は実装しない）

- スレッドを増やす将来（案 A: JSPI 版のスレッド）では、スレッドごとに値が違う必要がある（Learn: 一意）。拡張は次の形になる。`fle_imp_GetCurrentThreadId` は、いま走っているゲストのスレッドの仮想 ID を返す。仮想 ID はスレッドの生成時に割り当てる（`FLE_EMULATED_TID` を最初のスレッドの値にして、2 つ目から 0x1002、0x1003 …。決定論的な連番。ホストの tid の値は使わない）。ホスト tid → 仮想 ID の表（上の案 C）は、`fle_host_tid()` を鍵にして持つ。`fle_host_tid()` は wasm でスレッドごとに違う値を返すように変える必要がある（今は定数 1）。単一スレッド規則（owner のチェック）はそのとき別の決定で見直す。
- TEB の ClientId（0x40、0x48）をモデルするとき: 同じ仮想 ID（`FLE_EMULATED_PID`、スレッドの仮想 ID）を書く。GetCurrentThreadId と TEB の値が食い違わないようにする。
- テスト `guest_thread_id_is_the_same_from_another_host_thread` は、第 2 のゲストスレッドを作る変更で意図して失敗する（そのとき直す）。

## 限界

- 値の選び方は仕様から決まらない（仮の決定）。
- 実 Windows との照合はしていない（ホストは Linux）。
- wasm 側のゲストの分岐を実測で追っていない。native と wasm の一致は、入口数、関数実行数、停止点、呼び出しスタックなど出力の一致で確認した範囲。
- ゲストの他の箇所が ID を文字列化して他の API に渡すかは、全部は調べていない（`f_43aa10` の呼び出し元は 10 件以上）。
