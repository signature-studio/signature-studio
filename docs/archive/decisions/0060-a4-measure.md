# 0060: A4 の進捗の測り方（unit が「進んだ」の定義）

状態: 作業者（サブエージェント）が測定の定義だけを書く。**判定基準（0046 の A4: DllMain → InitUnits の完走、InitTable の runnable 895 件が全て実行される）は変えない**。定義の確認は PM。使い方と実測値は `docs/investigations/a4-survey-tool.md`。

## 事実（DLL の実測。f_4120c0 の逆アセンブル 0x4120c0〜0x41214b）

- InitContext は VA 0x15EAC90。`+0` が PackageUnitHeader へのポインタ（0x13768B0）、`+8` が u32 InitCount（0x15EAC98）。
- ループは unit i について、先に `InitCount = i+1` を書き（0x41210f）、そのあと `proc != 0` かつ `LD64(proc) != 0` のときだけ `call proc`（0x412126）。**InitCount は「実行中の index+1」で、完了した件数ではない**。proc が 0 の unit と空 unit でも InitCount は進む。
- 表は 942 件。runnable 895（proc の異なる値も 895）、empty 46（全て proc 0x13a9390）、noproc 1（index 941）。0046 の記録と一致した（差 0）。

## 定義

| 用語 | 定義 |
|---|---|
| runnable | proc ≠ 0 かつ LD64(proc) ≠ 0（f_4120c0 が呼ぶ条件） |
| started | runnable かつ index < InitCount の unit。ループは index 順に呼ぶので、InitCount が分かれば決まる |
| in_progress | 最後に started になった unit の proc が停止時の呼び出しスタック（shadow stack）にあれば 1、なければ 0 |
| completed | started − in_progress（最後の unit が戻っていれば started 全部） |
| 深さ（報告する値） | `init_count / 942`、最後の started unit の index と run_rank（runnable の中の順位、1〜895） |
| all_units_done | InitCount == UnitCount かつ in_progress == 0 |

- 「進んだ」の数え方は **completed**。InitCount そのものを「完了数」と書かない。
- 実行確認（entry_check）: started の unit の proc が関数入口の計数で見えていること（`started_but_not_entered` が 0）。0 でなければ InitCount と実行が食い違う。
- A4 の「895 件が全て実行される」の測定は、`completed == 895` かつ `all_units_done=yes` とする（DllMain が返ること `returned from entry` と合わせて 0046 の判定になる）。この定義は 0046 の文言を測定可能にしただけで、基準は変えていない。

## 限界

- 完了は「戻った」ことだけを意味する。unit の内部で fail-soft に進んだかは見ない（0046 の方針 4 は、停止（abort）しないことを進捗と認めない。この測定は abort で止まった位置を測るので、fail-soft の混入は別の確認が要る）。
- 同じ proc を共有する runnable unit は無い（895 件とも異なる値）。共有があれば in_progress の判定は粗くなる。
