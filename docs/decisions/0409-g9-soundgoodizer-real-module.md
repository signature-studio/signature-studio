# 0409: G9 の Fruity Soundgoodizer を実モジュール(11 本目)として取り込む(R5-6c。hook 点は専用ファイル。import の未対応 47 件を残す)
分類: (b) 契約の拡充(guest の VFS に同梱 DLL を置く seed 行と、realmod の表の 11 本目。hook 点は専用ファイル。宣言はデータ。runtime のコードは変えない)。追加行の getenv・#ifdef・URL パラメータ・新しい旗は 0 件。撤回条件: 下の §撤回条件。
日付: 2026-11-17   ROADMAP: R5-6c   一覧: G9、S1   コミット: 取り込みの commit(名は evidence/R5-6c.md §0)

## 0 範囲と件数(設計の前の確認)

- 依頼者の発言: 前任の全文 `~/work/handover/evidence/01a-user-messages-full.md` は 263 件(`grep -c '^## seq'` の結果 263)。ID(`msg_…`)を除いて `[Ss]oundgoodizer|[Pp]arametric|ildsp` に一致した発言は 1 件(seq 34564。G9 の範囲と、Soundgoodizer の dll サイズ 6MB の例)。`EQ|Shared` の広い検索では、seq 96849(EQ などのプラグインを今後追加する文脈。性能の話)と、無関係の 2 件(54836・54970。スレッドの話)が出た。FL のプラグインに関わる発言は、0408 §0 の 14 件(seq 5229 他)のうち 34564・96849 の 2 件。
- 読んだ記録: 決定 0407(全文)・0408(全文)・R5-6(§3〜§6。census・hook 点・S1)・`evidence/R5-6/hooks/soundgoodizer-own.txt`・`evidence/R5-6a.md`(§4・§5・§8)・ROADMAP の R5-6c の行。
- 読んだ入力: ci/lib/extract_fl.py（削除済み。決定 0415）(0408)・`lift/crates/fe-lift/src/indirect.rs`(IAT スロットへの jmp の扱い)・`lift/crates/fe-lift/src/imports_table.rs`(組み込みの import の表 610 行 + extra。`lookup_call` の一致)・`lift/harness/seeds/ship6-imports.txt`(235 行)・PE の import 表(Soundgoodizer は 15 の DLL・596 件)・インストールツリー(`gen_install_tree.py` の walk)。
- 検索の件数: `git grep -il soundgoodizer` は 55 件(HEAD の追加前は、docs・evidence・patches・archive を除くと hook 点の共有ファイルの注釈 1 件)。`ildsp` は lift・app・ci に 0 件。UTF-16 の `ildsp` の文字列は本体の DLL に 1 件(R5-6 §6 と一致)。
- 取り込みの前の状態(HEAD 5fb58e67): modules.tsv は 10 行(realmod 10 本)。RM_MAX は 16。Problems 窓は 5 件(R5-6a §5)。

## 決定

1. 決定: 抽出の表に Soundgoodizer の 1 行(`soundgoodizer_x64.dll` → 宛先 `out-plugins/soundgoodizer_x64.dll`)を足す。SHA-256 は `148fb682930ee33b1e686b8452143ce918da5ff234a18380e41a4b9b9d3fa18a`(R5-6 の census の入力と一致。ci/lib/extract_fl.py（削除済み。決定 0415） を表の全件で実行し、この値で検証が通った)。分類 b。
2. 決定: `lift/harness/b2d2/modules.tsv` に 1 行(realmod 名 `soundgoodizer_x64.dll`、基底 `0x8800000`、引数は Delay 2 と同じ型で hook 点だけ専用ファイル、realmod のパス接尾辞 `plugins/fruity/effects/soundgoodizer/soundgoodizer_x64.dll`)。基底は R5-6 の census が選んだ未使用の基底。占有範囲は 0x8800000〜0x8e33000(SizeOfImage 0x633000)。FLEngine と既存の 10 本のどれとも重ならない(gen-all の出力の DLL で確認。evidence/R5-6c.md §3)。分類 b。
3. 決定: hook 点は専用ファイル hook 点の専用ファイル(164c8ed4 で追加。revert 済み。ファイル名 soundgoodizer-hook-points.txt)(RVA 0x1b7b0・0x389e0・0x38a00。R5-6 の `soundgoodizer-own.txt` の値)。共有ファイルは使わない。根拠: R5-6 §4 で、共有ファイルでは停止し(`is not a function entry`)、専用の RVA では受理された。3 つの先頭バイトは `55 56 53 c6 44` / `53 48 83 ec 20` / `55 48 83 ec 50` で、他の Delphi 製と同じ(evidence/R5-6c.md §2 で再確認)。分類 b(Delphi の inline hook の宣言。0105/0106/0183 の機構)。
4. 決定: 同梱 DLL を guest に見せるため、`lift/harness/seeds/vfs-seed-idle1.tsv` に 1 行(guest path `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Soundgoodizer\Soundgoodizer_x64.dll`。NSIS の install tree にある。extract_fl.py が照合して通った)。分類 b。Presets(`Data\Presets\*.dat`)と `Plugin.nfo` は足さない(読み込みに要らない。根拠: Delay 2 と Balance は DLL だけ)。
5. 決定: `lift/harness/web/pack.args` の `--args` の列に、`/data/soundgoodizer_x64.8800000.dll` を 11 番目として足す。分類: 整備(データ)。
6. 決定: `lift/harness/two/plain-units.txt` に m11 の単位 6 件(`m11-00.c`〜`m11-05.c`)を足す。分類: 整備(0322bfec と同じ型。gen-all の出力の数を、先に gen-all で数えた)。
7. 決定: runtime のコードは変えない。根拠: RM_MAX は 16 のまま。取り込み後の realmod は 11 本(上限の内)。realmod 表は gen-all が書く(realmod.c は 11 件・パス表 4 件)。分類: 変更なし。
8. 決定: hybrid の対象外(HYB_MODS に入れない)。根拠: 0408 の 8 と同じ(0396 の §決定。旧設計 `docs/archive/decisions/0309-g9-entry-estimate.md`)。分類: 変更なし。
9. 決定: 未対応の import thunk 47 件は残す。分類: 未確認の扱い(b の候補)。根拠と内訳:
   - fe-lift の出す件数は 47(census と一致。`Jmp: indirect through IAT slot (not lifted)`)。件数だけで、どの import かは出さない(`lift/crates/fe-lift/src/main.rs` の統計)。
   - 近似の内訳(thunk の到達を見ない上限): `jmp [IAT]` の先で、組み込みの表にも ship6 にも行が無い import は 58 件。DLL 別は imagehlp 1・api-ms-win-crt 1・comdlg32 2・gdi32 9・kernel32 12・ole32 7・shell32 5・user32 16・winspool 5。Delay 2 と Balance は同じ方法で 28 件(fe-lift の 21 件)。fe-lift の 47 件は 58 件のうち到達した物。到達した物の DLL 別の内訳は未確認。
   - 実行時に止まるかは実測で確かめる(Export・Problems)。止まったら、その VA の import を分類 (b) の行の拡充で直す。
10. 決定: 関数ポインタとして使われる import thunk を、関数の入口(seed)として足す。分類 b(lift の既知の入口の契約の拡充。`--seeds` の既存の仕組み)。根拠: 1 回目の bundle(164c8ed4)を実行すると、Problems の読み込みで engine が止まった: `fle: indirect call to 0x8806f80 is not a known function entry`(3 回とも同じ。対照の 3 回は止まらない)。0x8806f80 は kernel32!RaiseException の import thunk(`jmp qword [IAT]`。pdata に無い)。生成 C の `L_88113bd: R[0] = UINT64_C(0x8806f80)` が、その thunk のアドレスを作り、間接に call していた。thunk を指すアドレスの定数は lift 済みの C に 18 種類あった(data の 1 件を含む。`evidence/R5-6c.md` §4)。これを seed の専用ファイル(e27ad00f で追加。revert 済み。ファイル名 soundgoodizer-8800000-seeds.txt)(18 行の VA。基底 0x8800000)として足す。他のモジュールの出力は変わらない(76 件の cmp で確認)。内訳: 行(組み込みの表か ship6)が有る 7 件(RaiseException・ReadFile・GetTickCount・DefWindowProcW・GetScrollInfo・SetScrollInfo・ShowScrollBar)と、行の無い 11 件(RtlUnwind・GetOpenFileNameW・GetSaveFileNameW・CreateDCW・CreateICW・DefMDIChildProcW・EnableScrollBar・GetScrollPos・GetScrollRange・SetScrollPos・SetScrollRange)。行の無い物は、呼ばれると止まる(未対応)。これは測定で確かめる。seed を足すと、未対応の数は 47 から 58 に変わる(行の無い 11 件が lift され、未対応として数えられるため)。
11. 決定: 例外の着地(SEH)を登録する。`--seh-handler 0x10490`(Delphi の handler。`.pdata` の unwind 情報で 2,746 関数が使う RVA)を、Soundgoodizer の modules.tsv の行に足す。分類 b(`--seh-handler` の既存の仕組み。決定 0132。FLEngine の行の 0x112c0 と同じ型)。根拠: 2 回目の bundle(e27ad00f。seed は足したが SEH は無い)を実行すると、Problems の読み込みで engine が exit 134(SIGABRT)で終わった。停止報告の本文は画素の診断文だが、report の tail の本当の理由は `fle: RtlUnwindEx: the target frame is the function at 0x88f94b0 ..., which has no landing registration: it was not lifted with --seh-handler` だった(RaiseException は RtlUnwindEx に入る)。`--seh-handler` を足すと、landing 911 件・wrapped 870 件が生成され、landing 0x88f96ee が C に入る。未対応の数は 58 のまま。mK の単位は 7 件になる(m11-06.c を plain-units に足す)。
12. 決定: ildsp_x64.dll は足さない(未確認のまま)。根拠: Soundgoodizer の import 表に ildsp は無い(UTF-16 の文字列が 1 件あるだけ)。呼ばれるかは未確認。engine の log に ildsp の要求が出たら、その時点で分類して決める(0309 G9-d2)。

## 予測(測定の前に書く。撤回条件の基準)

- 増分: k_C × C = 0.209407 × 58,771,400 = **12,307,166 B**(R5-6 §5 の k_C。C は census の分割なし 1 ファイルの値。gen-all の分割後の合計は未確認)。
- wasm: 165,776,779 + 12,307,166 = 178,083,945 B(予測)。
- S1: 178,083,945 ÷ (88,674,176 + 6,407,680 = 95,081,856) = 1.873(予測)。2 倍の上限 190,163,712 B に対して余裕 12,079,767 B。
- Problems 窓: 5 → 4 件(目視。Soundgoodizer が一覧から消える。ildsp が原因なら残る)。
- 未対応(gen-all の統計): 58(seed を足した後。決定 10)。

## 撤回条件

- (1) 増分の実測が予測の ±20%(10,000,000 〜 14,800,000 B 程度)の外に出たら、k_C の仮定(2 モジュールの係数)を見直す(0407 の撤回条件 (1))。
- (2) Problems 窓が 4 件にならない(Soundgoodizer が残る)なら、原因を分類する(hook 点・ildsp・未対応の thunk のどれか)。
- (3) 再生・Export で止まる(`fle: unsupported` か名前つきの停止)なら、止まった VA の import を分類して直す。実装できない間は、modules.tsv の行を外した版を測る。

## 実測(evidence/R5-6c.md。判定は reviewer)

- bundle は 3 回。164c8ed4(seed 無し)は止まる(0x8806f80 = RaiseException の thunk を間接 call)。e27ad00f(seed 18 件)は止まる(RtlUnwindEx の着地。`--seh-handler` が無い)。48ccf733(`--seh-handler 0x10490`)は止まらない。
- Problems 窓(目視): 対照 5 件 ×3 → 取り込み後 5 件 ×3。Soundgoodizer は「Error」から「Exception on open」に変わった(開けていない。中身は未確認)。撤回条件 (2) に当たる(件数が 4 にならない)。原因の分類は未了。
- wasm: +13,275,105 B(予測 12,307,166 B に対し +7.86%。撤回条件 (1) の ±20% の内)。S1 は 1.8695 から 1.8831。
- Export: 対照と同じ 3066e51f…f01e。P1(n=5 交互): 取り込み後の中央値 4,211.0 ms、対照 4,119.1 ms(差 +91.9 ms。範囲は重ならない)。
- ildsp_x64.dll を VFS に足す実験(一時の seed)では、状態は変わらない(Exception on open)。ildsp を原因とする根拠は、この実験では得られなかった。
- 分類: 決定 10(関数ポインタの thunk の seed)と決定 11(`--seh-handler`)は、どちらも (b)。

## 出典

- evidence/R5-6c.md(この取り込みの実測)、evidence/R5-6.md(census・S1 の式)、docs/decisions/0407-g9-rest-census-and-s1.md、docs/decisions/0408-g9-delay2-real-module.md、docs/decisions/0396-g9-balance-filter-real-modules.md、evidence/R5-6/hooks/soundgoodizer-own.txt、docs/ROADMAP.md の R5-6c の行。
- `~/work/handover/evidence/01a-user-messages-full.md`(上の件数)。

## 外した(2026-11-17。main の決定)

- 決定: Soundgoodizer を main に残さない(11 本目として取り込まない)。根拠: wasm の増分 +13,275,105 B(+7.86%)・P1 の +91.9 ms・S1 1.8831 に対し、Problems の件数は 5 のまま(目標 4)。利用者に見える効果が無い。AGENTS §6(効果の無い処理を足さない)。
- 外した commit: 3f4d7177(164c8ed4 の revert)・360e3ed5(e27ad00f の revert)・5fb14b10(31f7cda4 の revert)。取り込み前の版と一致することを確かめた(modules.tsv・extract-dlls.tsv・vfs-seed-idle1.tsv・plain-units.txt・pack.args の `--args`)。本文と記録(この文書・`evidence/R5-6c.md`・0023 の追記)は残した。
- 再開条件: `Exception on open` の原因(Problems の More info の中身。native ではなく実エンジンの open 時の例外)が分かったとき。再現の bundle は 31f7cda4 から作れる(seed と SEH を含む版)。
- 分類: 撤回(外す)。新しい旗・環境変数・`#ifdef` は、どの版でも増やしていない。
