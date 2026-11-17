# 0083: GetStartupInfoW — 入力の cb の検査を撤回し、cb の値に関係なく STARTUPINFOW 全体を 0 で埋めて cb に 0x68 を書く

状態: PM の決定（2026-10-23）を、作業者（サブエージェント）が記録し、実装した。**仮の決定**（実 Windows の kernel32 が入力の cb を読むかは一次資料に無く、未確認）。事実の出典は `docs/investigations/getstartupinfow-cb.md`（以下「調査」。節番号はその文書のもの）。この記録は調査に無い主張を足さない。

## 決定（PM。変更しない）

| # | 項目 | 決定 |
|---|---|---|
| 1 | `lpStartupInfo` が NULL | **従来どおり名前付き停止**（`GetStartupInfoW: lpStartupInfo is NULL`）。理由: GetStartupInfoW のページに NULL の規則がないので、決定 0018 §決定の「仕様が定める範囲を超える入力は abort で止める」に従う |
| 2 | 入力の cb の検査 | **撤回する**。入力の cb が 0x68 でなければ停止、という規則（`fle_env.c` のコメントと `env.rs` のテストにだけあった）をなくす |
| 3 | 書く内容 | `lpStartupInfo` が NULL でなければ、**入力の cb の値に関係なく**、STARTUPINFOW 全体（0x68 バイト）を 0 で埋めてから、cb（+0）に 0x68 を書く |
| 4 | 継承ハンドル | `cbReserved2` = 0、`lpReserved2` = NULL（継承ハンドル無し） |
| 5 | その他 | 決定 0018 §7 のとおり: フラグ、位置、標準ハンドルはない（全メンバー 0。cb を除く） |

## 背景（事実。調査の節番号）

- 決定 0018 の本文に「呼び出し側の cb が 0x68 でなければ停止」という文は**ない**。あるのは §決定（PM）の 1 行「仕様が定める範囲を超える入力は abort で止める」と、§7 の「`cb`（0x68）以外の全メンバーを 0 にする」だけ。cb の停止規則は `lift/runtime/fle_env.c` のコメントと `lift/crates/fe-lift/tests/env.rs` の `startup-wrong-cb` にだけあった（調査 §1 の 1、§2.1）。
- Learn（GetStartupInfoW のページ）は `lpStartupInfo` を `[out]`（「startup information を受け取る構造体」）とし、cb の入力規則を書かない。ページ全体に `cb` という語がない。STARTUPINFOW のページは cb を「The size of the structure, in bytes」とだけ書く（調査 §3）。つまり停止の根拠になる一次の記述がなかった。
- 呼び出し点は 2 つ。FLEngine（0x40739a）は cb に 0x68 を入れてから呼ぶ。dsp_ippv2（0x1817d34d4）は 0x68 バイトの全体が未初期化（cb も）のスタックのバッファを渡す。どちらも呼び出し後に cb を読み返さない（調査 §4.4。読み）。dsp_ippv2 が読むのは `cbReserved2`（+0x42）と `lpReserved2`（+0x48）だけ。
- 未初期化の cb の値は実行ごとに違う（0x65d0ab08、0x65d0bb08。調査 §4.3、既存ログ）。「未初期化」と「誤った値」は区別できない。

## 選択肢と却下

調査 §6 の選択肢のうち:

| | 内容 | 結論 | 理由 |
|---|---|---|---|
| (a) | 現状維持（`cb != 0x68` で停止） | **却下** | Learn に cb の規則も、cb が違うときの結果もない（調査 §3.3）。dsp_ippv2 の DllMain(ATTACH) が、CRT の lowio 初期化（`f_37d34b4`）で止まる。停止は仕様ではなく、作業者が §決定の 1 行を cb に当てはめたもの（調査 §2.2） |
| (b) | cb を出力として扱い 0x68 を書いて続行（NULL の停止は残す） | **採る**（この決定） | `[out]` の引数で、cb の入力規則が一次にない。両呼び出し点とも呼び出し後に cb を読み返さない。続行すると CRT は `cbReserved2 == 0` の枝に入る（調査 §5。読み） |
| c1 | 0x68 のときだけ通し、「未初期化」と「誤った値」を分ける | 却下 | 成立しない。未初期化の値は実行ごとに違い、誤った値と区別できない（調査 §6 c1） |
| c2 | cb が 0x68 以外のときは cb だけ書いて他を触らない | 却下 | Learn に根拠がない。dsp_ippv2 では `cbReserved2`、`lpReserved2` が未初期化のまま CRT に読まれ、`lpReserved2` の先を読みに行く（読み）。実行では危険（調査 §6 c2） |
| c3 | cb の値に応じて書く量を変える | 却下 | Learn に書く量の規則がなく、未初期化の値に対して意味がない（調査 §6 c3） |
| c4 | 生成側（lift）か harness で呼び出し点の前に cb を書く | 却下 | ゲストの命令列と違う動作を足す。Learn の範囲外。`patches_stage1.py` の gdb の仮の値はこの形の一時版（調査 §6 c4） |
| c5 | `cbReserved2`、`lpReserved2` に非 0 を入れる | 採らない（別軸） | cb の選択とは独立。0 / NULL は決定 0018 の判断のまま。非 0 にすると CRT が `GetFileType` に進み、runtime は名前付きで停止する（調査 §6 c5） |

## 仮の決定である点

- **実 Windows の kernel32 が入力の cb を読むかは未確認**（一次資料に無い。ホストは Linux で、Windows では実行していない）。cb が 0x68 でない、またはバッファが 0x68 未満のときの実 Windows の挙動も未確認（調査 §9）。
- 「入力の cb を無視する」「関数が cb を書く」と**明記した一次の記述はない**（調査 §6 の表）。cb に 0x68 を書くことは、取得した構造体の 1 メンバーとして自然、という読みで、現行の実装（0x68 を書く）をそのまま残したもの。
- 作成元プロセスが無いときの `lpReserved`、`lpDesktop`、`lpTitle`、`hStd*` の値（0 / NULL）は決定 0018 の判断で、一次には無い（調査 §9）。

## 撤回条件

- **実機で、入力の cb を検査することが一次資料で確認されたとき**（たとえば、小さい cb で失敗する、cb が違うと構造体を埋めない、など）。そのときは別の決定で扱いを決める。
- Learn が cb の入力規則、またはバッファが 0x68 未満のときの結果を書いたとき。
- PM が方針を変えたとき。

## 影響範囲

- FLEngine 側の呼び出し点（0x40739a。cb = 0x68 を入れてから呼ぶ）の挙動は**変わらない**。`dwFlags` = 0 なので `wShowWindow` は読まれず、戻り値は 10 のまま（調査 §4.2。読み）。FLEngine の生成 C は変わらない（runtime のみの変更）。
- dsp_ippv2 側（0x1817d34d4。未初期化のバッファ）は、これまで cb 不一致で停止していたものが**通る**。通った先で `cbReserved2 == 0` により継承ハンドルの処理を飛ばし、標準 3 本は `GetStdHandle` が NULL を返すので「コンソールなし」になる（調査 §5。読み）。
- 通った先の次の停止（`MultiByteToWideChar` 等、ロケール表。`stage23-two-module-real-dll.md` §9 の 4）は**別項目**で、この決定では扱わない。この決定だけでは DllMain は完走しない（調査 §5、§7 の 4）。
- 変更したもの: `lift/runtime/fle_env.c` の `fle_imp_GetStartupInfoW` とそのコメント、`lift/crates/fe-lift/tests/env.rs` の startup 関連の期待、決定 0018 §7 の行への注記、`stage23-two-module-real-dll.md` §5.3 と §9 の 5 への訂正の注記、索引と仮の決定の一覧への追記。
- 変えないもの: `lift/harness/two/patches_stage1.py`、`patches_chain.py`（gdb の仮の値の harness。コメントに「decision 0018 text」とあるが、この決定の範囲ではない。気づいた点として報告するだけ）。

## 限界

- runtime は呼び出し側のバッファの大きさを知らず、0x68 バイトを書く。両 DLL の呼び出し点のバッファはどちらも 0x68 バイト以上と読める（FLEngine `[rsp+0x28, +0x90)`、dsp_ippv2 `[rsp+0x20, +0x88)`。調査 §4.2、§4.3）。静的に見つかった呼び出し点はこの 2 つだけで、レジスタ相対の IAT 参照などは追えていない（調査 §4.1 の限界）。
- 根拠は Learn の 2 ページの読みと、両 DLL の逆アセンブルの読みに限る。実 Windows との照合はしていない。非公式の実装（ReactOS、Wine。調査 §8）は根拠にしていない。
