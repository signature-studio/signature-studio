# 0411: G9 の Fruity Parametric EQ 2 を実モジュール(11 本目)として取り込む(R5-6d。結果は保留: bundle は作れたが、起動で engine が止まる。main には入れない)
分類: (b) 契約の拡充(guest の VFS に同梱 DLL を置く seed 行、realmod の 11 本目、専用の hook 点ファイル、`--seh-handler` と `--seeds` の既存の仕組みの宣言)。runtime のコードは変えない。追加行の getenv・#ifdef・URL パラメータ・新しい旗は 0 件。撤回条件: 下の §撤回条件。
日付: 2026-11-17   ROADMAP: R5-6d(blocked)   一覧: G9、S1   コミット: 取り込みの commit は e5786c48(ワークツリー r56e。**main には入れていない**)

## 結果(保留。main には入れない)

- bundle は作れた(gen-all 11 modules。engine 174,700,226 B)。Problems で、new 3 回とも engine が止まった(exit 134。`fle: indirect call to 0 is not a known function entry`。戻り先 0x9215608、PEQ2 の RVA 0x215606 の間接 call)。対照は止まらない。
- ildsp を VFS に足しても同じ(実験)。SEH と seed を入れた版の停止の原因は、決定 0416 の結論で確定(呼び先の関数表 0x935F220 が、ILVideo_x64.dll の読み込みの失敗で埋まらない。番地は RVA 0x3503A0。本文の 0x3500A0 は誤り)。
- 決定: e5786c48 は main に入れない(PEQ2 を含む project で engine が止まる回帰になる)。ROADMAP R5-6d は blocked。再開条件: ILVideo_x64.dll と、その import の閉包 8 本(決定 0423 の訂正。依頼の 6 本は avfilter-7 を落としていた)が lift され、ILVideo の読み込みが成功して表が埋まり、止まらない版を bundle で確かめたとき(決定 0416 の決定 2)。詳細は evidence/R5-6d.md。

## 0 範囲と件数(設計の前の確認)

- 依頼者の発言: 前任の全文 `~/work/handover/evidence/01a-user-messages-full.md` は 263 件(`grep -c '^## seq'` の結果 263)。ID(`msg_…`)を除いて `[Pp]arametric|パラメトリック|EQ|ＥＱ|ildsp|Shared` に一致した発言は 3 件(seq 54836・54970・96849)。PEQ2 の名前を含む発言は 0 件。96849 の『EQなどを追加』は、今後の内蔵インストゥルメントの文脈(性能の話)で、PEQ2 を指さない。54836・54970 は無関係(スレッドの話)。
- 読んだ記録: 決定 0407(§0 と census)・0408・0409(全文。revert の経緯を含む)・R5-6(§3〜§6。PEQ2 の hook 点と census)・evidence/R5-6a.md・evidence/R5-6c.md(全文。3 回の bundle の原因)・ROADMAP の R5-6d の行。
- 読んだ入力: PEQ2 の PE の import 表(510 件。RaiseException・RtlUnwind・RtlUnwindEx を含む)、`.pdata` の unwind 情報(EHANDLER の handler RVA 0xcf50 が 1,829 関数)、生成 C の `Lea` 定数(3,034 件)と、データ部のポインタ(0x1e290 の 1 件)。
- 検索の件数: `git grep -il 'parametric eq 2\|parametric_eq2'` は、取り込み前の時点で、docs・evidence・patches・archive を除くと、hook 点の共有ファイルの注釈と ROADMAP の行だけ。`ildsp` は、Soundgoodizer と PEQ2 の DLL に UTF-16 で 1 件ずつ(R5-6 §6 と一致)。
- 取り込みの前の状態(HEAD f456000d): modules.tsv は 10 行(realmod 10 本)。RM_MAX は 16。Problems 窓は 5 件(R5-6a と同じ。Soundgoodizer は revert 済みで、対照は 5 件の版)。

## 決定

1. 決定: 抽出の表に PEQ2 の 1 行(`fruity parametric eq 2_x64.dll` → (抽出先 out-plugins の PEQ2 の DLL))を足す。SHA-256 は `8c7c95b85c516f3766228004df31e13f806b6012cc240f47245539199b873231`(R5-6 の census の入力と一致。ci/lib/extract_fl.py（削除済み。決定 0415） を表の全件で実行し、この値で検証が通った)。分類 b。
2. 決定: `lift/harness/b2d2/modules.tsv` に 1 行(realmod 名 `fruity_parametric_eq2_x64.dll`、基底 `0x9000000`、realmod のパス接尾辞 `plugins/fruity/effects/fruity parametric eq 2/fruity parametric eq 2_x64.dll`、引数(2 件のファイルは e5786c48 で作成、origin/main の系譜には無い)`--pdata --seeds lift/harness/seeds/peq2-9000000-seeds.txt --seh-handler 0xcf50 --hook-points lift/harness/hooks/peq2-hook-points.txt --extra-imports lift/harness/seeds/ship6-imports.txt`)。基底は R5-6 の census が選んだ未使用の基底。分類 b。
3. 決定: hook 点は専用ファイル (hook 点の専用ファイル。e5786c48 にある。main には無い)(RVA 0x17ee0・0x32100・0x32120。R5-6 の `peq2-own.txt` の値)。先頭バイトは `55 56 53 c6 44` / `53 48 83 ec 20` / `55 48 83 ec 50`(確かめた)。分類 b(0409 の 3 と同じ)。
4. 決定: 例外の着地を登録する: `--seh-handler 0xcf50`。根拠(取り込みの前に確かめた): PEQ2 は RaiseException と RtlUnwindEx を import する(R5-6c と同じ経路)。`.pdata` の handler RVA は 0xcf50 のみ(1,829 関数)。分類 b(決定 0409 の 11 と同じ。決定 0132 の仕組み)。
5. 決定: 関数ポインタとして使われる import thunk 14 件を、関数の入口(seed)として足す((seed の専用ファイル。e5786c48 にある。main には無い)。基底 0x9000000 の VA)。根拠(取り込みの前に確かめた): 生成 C の `Lea` 定数(3,034 件)のうち、PEQ2 の thunk を指すものが 14 件(RVA 0x4c20・0x4d10・0x4d20・0x1d5e0・0x1e280・0x1e290・0x1e3e0・0x1e720・0x1e730・0x1e740・0x1ecc0・0x1ecd0・0x1ece0・0x1ed90)。データ部の参照は 0x1e290 の 1 件で、これも含まれる。14 件を足すと、すべて関数として定義された(確認済み)。分類 b(決定 0409 の 10 と同じ)。
   - 行(import の表)の無い thunk は 7 件(RtlUnwind・DefMDIChildProcW・EnableScrollBar・GetScrollPos・GetScrollRange・SetScrollPos・SetScrollRange)。呼ばれると名前つきで止まる(未対応)。測定で確かめる。
   - 未対応の数は 28(import thunk 21 件 + 行の無い thunk 7 件)。
6. 決定: 同梱 DLL を guest に見せるため、`lift/harness/seeds/vfs-seed-idle1.tsv` に 1 行(guest path `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Parametric EQ 2\Fruity Parametric EQ 2_x64.dll`。NSIS の install tree にある。extract_fl.py が照合する)。分類 b。
7. 決定: `lift/harness/web/pack.args` の `--args` の列に `/data/fruity_parametric_eq2_x64.9000000.dll` を 11 番目として足す。分類: 整備(データ)。
8. 決定: `lift/harness/two/plain-units.txt` に m11 の単位 5 件(`m11-00.c`〜`m11-04.c`)を足す。分類: 整備(gen-all の出力の数を先に数えた)。
9. 決定: runtime のコードは変えない。根拠: RM_MAX は 16 のまま。取り込み後の realmod は 11 本。分類: 変更なし。
10. 決定: ildsp_x64.dll は足さない(未確認のまま)。根拠: R5-6c の実験では、ildsp を VFS に足しても Soundgoodizer の状態は変わらなかった。PEQ2 も、まず足さずに測る。ildsp が原因と分かったら、その時点で分類して決める。
11. 決定: hybrid の対象外(HYB_MODS に入れない)。根拠: 0408 の 8 と同じ(0396 の §決定。旧設計 `docs/archive/decisions/0309-g9-entry-estimate.md`)。分類: 変更なし。

## 予測(測定の前に書く。撤回条件の基準)

- C(分割なし 1 ファイル。seed と SEH を入れた版): 41,941,457 B(gen-all の出力の単一 lift。census の 40,521,516 B は seed と SEH の無い版)。
- 増分: k_C × C = 0.209407 × 41,941,457 = **8,782,851 B**(R5-6 §5 の k_C)。
- wasm: 165,776,779 + 8,782,851 = 174,559,630 B(予測。基準は revert 後の 10 モジュールの版)。
- S1: 174,559,630 ÷ (88,674,176 + 4,036,608 = 92,710,784) = **1.8828**(予測)。2 倍の上限 185,421,568 B に対して余裕 10,861,938 B。
- Problems 窓: 5 → 4 件(目視。PEQ2 が一覧から消える)。
- 未対応(gen-all の統計): 28。

## 撤回条件

- (1) 増分の実測が予測の ±20% の外に出たら、k_C の仮定を見直す(0407 の撤回条件 (1))。
- (2) Problems 窓が 4 件にならない(PEQ2 が残る、または「Exception on open」)なら、原因を分類する(0409 の『外した』と同じ判断)。
- (3) 再生・Export で止まる(`fle: unsupported` か名前つきの停止)なら、止まった VA の import を分類して直す。実装できない間は、modules.tsv の行を外した版を測る。

## 出典

- evidence/R5-6d.md(この取り込みの実測)、evidence/R5-6.md(census・S1 の式・hook 点)、evidence/R5-6/hooks/peq2-own.txt、docs/decisions/0407・0408・0409・0396、evidence/R5-6c.md(3 回の bundle の原因)、docs/ROADMAP.md の R5-6d の行。
- `~/work/handover/evidence/01a-user-messages-full.md`(上の件数)。
