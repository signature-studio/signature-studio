# 0408: G9 の Fruity Delay 2 を実モジュール(10 本目)として取り込む(Balance・Filter と同じ型。RM_MAX は 16 のまま)
分類: (b) 契約の拡充(guest の VFS に製品の同梱 DLL を置く seed 行と、realmod の表の 10 本目。宣言はデータ。runtime のコードは変えない)。追加行の getenv・#ifdef・URL パラメータ・新しい旗は 0 件。撤回条件: 下の §撤回条件。
日付: 2026-11-17   ROADMAP: R5-6a   一覧: G9、S1   コミット: 6fb75fbe(取り込み)、0322bfec(決定 9 の plain-units)。実測は evidence/R5-6a.md(完了ではない。判定は reviewer)

## 0 範囲と件数(設計の前の確認)

- 依頼者の発言: 前任の全文 `~/work/handover/evidence/01a-user-messages-full.md` は 263 件(`grep -c '^## seq'` の結果 263)。ID(`msg_…`)を除いたうえで、`プラグイン|[Pp]lugin|[Ff]ruity|[Bb]alance|同梱|Delay|Soundgoodizer|Frequency|FLEX|G9`(G9 は英数に挟まれないもの)に一致した発言は 14 件(awk で seq を集計)。
  - FL のプラグイン・G9・同梱・インストール先に関わる発言 5 件: seq 22192(FL の窓のデタッチ。プラグインの窓を含む)・34564(G9 の範囲「全プラグインの動作も含まれます」。S1 の許容)・55145・55453(プラグインの窓の半透明)・95591(依頼者機の `FL Studio 2026\Plugins` 867 MB。0407 §0 は 4 件と書いた。95591 はその一覧に無かった)。
  - OpenCode の plugin に関わる発言 9 件: seq 5229・5436・6965・7146・7317・7408・7544・7795・24699(0407 §0 と同じ。FL のプラグインではない)。
  - 誤検出 1 件: seq 7438 の一致は ID の `G9` によるもの(ID を除くと一致しない)。
- 読んだ記録: R5-5(evidence/R5-5.md、全文)・R5-5b(全文)・R5-6(全文)・決定 0396(全文)・0407(全文)・R6-28(§1〜§4)・R6-31(§1〜§9)・docs/failures/0021(全文)・ROADMAP の R5-5・R5-6・R5-6a の行。
- 読んだ入力: ci/lib/extract_fl.py（削除済み。決定 0415）(全文)・`lift/crates/fe-lift/src/gen_all.rs`(60〜170 行)・`.github/workflows/deploy.yml`(全文)・`lift/harness/b2d2/modules.tsv`・`lift/harness/web/pack.args`・`lift/harness/web/seed.args`・`lift/harness/b2d2/gen.args`・`ci/extract-dlls.tsv`・`lift/harness/seeds/vfs-seed-idle1.tsv`(Balance・Filter の行)・`lift/harness/seeds/gen_install_tree.py`(DEFAULT_INSTDIR)。
- 検索の件数: `git grep -il 'fruity_filter\|fruity filter\|fruity_balance\|fruity balance'` は 78 件(docs・evidence・patches を含む)。docs・evidence・patches・archive を除くと、ci・lift・app の 7 件(`ci/extract-dlls.tsv`・`lift/harness/b2d2/modules.tsv`・`lift/harness/hooks/fruity-delphi-effect-hook-points.txt`・`lift/harness/seeds/vfs-seed-idle1.tsv`・`lift/harness/web/pack.args`・`lift/runtime/fle_cursor.c`・`lift/runtime/fle_realmod.c`)。Delay 2 の名前(`delay 2|delay2|delay_2`)は、同じ範囲で `lift/harness/hooks/fruity-delphi-effect-hook-points.txt` の注釈 1 件だけ。
- 取り込みの前の状態(HEAD e63f7770。R5-5 の commit 9fbf6fb・6abb3a4・b7b7b5f・e84a13f・4da68cb・8b5ff31・a2518b1 を含む): modules.tsv は 9 行(dsp_ippv2・Blend2D・QuickFontCache・FreeType・ILTools・oggio・Elastique・Balance・Filter。realmod は 9 本)。RM_MAX は 9fbf6fb で 16。Problems 窓は 6 件(R5-6a の control の値は evidence/R5-6a.md §4 で再測定)。

## 決定

1. 決定: 抽出の表に Delay 2 の 1 行(`fruity delay 2_x64.dll` → 宛先は表の 2 列目。HOST_ROOT 下の `out-plugins/fruity_delay2_x64.dll`)を足す。SHA-256 は `c4fed60da5034d5c04019a4992563334aa6eb89e2adc211214aaca520ea928fc`(R5-6 の census の入力と一致。ci/lib/extract_fl.py（削除済み。決定 0415） を表の全件で実行し、この値で検証が通った。evidence/R5-6a.md §2)。分類 b(製品の同梱 DLL の抽出の宣言)。根拠: Balance・Filter の 2 行(a2518b1)と同じ型。
2. 決定: `lift/harness/b2d2/modules.tsv` に 1 行を足す(realmod 名 `fruity_delay2_x64.dll`、基底 `0x7800000`、引数は Balance と同じ、realmod のパス接尾辞 `plugins/fruity/effects/fruity delay 2/fruity delay 2_x64.dll`)。基底は R5-6 の census が選んだ未使用の基底。占有範囲(基底〜基底+SizeOfImage 0x3a5000)は、FLEngine と他の 9 本のどれとも重ならない(gen-all の出力の DLL で確認。evidence/R5-6a.md §3)。分類 b(10 本目の宣言。gen-all が realmod 表・rebase・lift を書く。手で書く表は無い)。
3. 決定: runtime のコードは変えない。根拠: RM_MAX は 9fbf6fb で 16 になっており、10 本は上限の内。realmod の表は gen-all が書く(R6-28)。fle_cursor.c の SHARED_MODULE_REALMODS は 16(0313f2)。分類: 変更なし。
4. 決定: 同梱 DLL を guest に見せるため、`lift/harness/seeds/vfs-seed-idle1.tsv` に 1 行を足す(guest path `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Delay 2\Fruity Delay 2_x64.dll`、host path(抽出の出力) `lift/out-idle1/inputs/install/Plugins/Fruity/Effects/Fruity_Delay_2_x64.dll`、SHA-256 は 1 と同じ)。根拠: R5-5 の 9fbf6fb は Balance・Filter の 2 行を足した。足さないと、Problems 窓の Delay 2 は『could not be opened』のまま。分類 b(インストーラの同梱ファイルを guest の VFS に置く契約)。guest path は `gen_install_tree.py` の DEFAULT_INSTDIR(FL Studio 21)の下にあることを extract_fl.py が照合する(照合できなければ止まる)。
5. 決定: `lift/harness/web/pack.args` の `--args` の列に、`/data/fruity_delay2_x64.7800000.dll` を 10 番目として足す(gen-all の出力名 `<stem>.<base 16 進>.dll`)。分類: 整備(bundle の引数の宣言。データ)。
6. 決定: hook 点は共有ファイル `lift/harness/hooks/fruity-delphi-effect-hook-points.txt` を使う。根拠: R5-6 §4 で、Delay 2 は共有ファイルで通った(hook_points 3)。新しいファイルは作らない。
7. 決定: hybrid の対象外(hyb-U の一覧に入れない。HYB_MODS に入れない)。根拠: Balance・Filter と同じ(0396 の §決定。旧設計 `docs/archive/decisions/0309-g9-entry-estimate.md` の G9 の見積り)。分類: 変更なし。
8. 決定: `.github/workflows/` は変えない。`deploy.yml` の手順(step 8〜10)は R6-31 の形を使う。根拠: 規則。
9. 決定: `lift/harness/two/plain-units.txt` に m10 の単位 4 件(ビルドの出力 `lift/out-app/gen/src/m10-00.c`〜`m10-03.c`)を足す(分類: 整備。データ)。根拠: 最初の bundle の step 9c で、hybrid の検査が `the merged module still imports a lifted function (f_<va>) from env` で止まった。C 部分の import 16,440 件のうち 12,565 件が m10 の VA(0x7803441 など)で、m10 の単位がコンパイル単位の一覧に無く、オブジェクトが作られていなかった(一覧は R6-25 §12 の手書きで、gen-all の出力に追随しない)。m10 の単位を足すと、その import は C 部分の定義になると予測する(未確認。9c の再実行で確かめる。evidence/R5-6a.md §5)。この決定は、取り込みの commit の後の 2 つ目の commit に入れる。

## 予測(測定の前に書く。撤回条件の基準)

- wasm の増分の予測: 約 7.67 MB(R5-6 §5 の k_C × C。k_C = 0.209407 B/byte、C = 36,603,525 B。0.209407 × 36,603,525 ≒ 7,665,000 B)。
- Problems 窓: 6 件 → 5 件(目視。Delay 2 が一覧から消える)。
- Export の sha256: 変わるか未確認(予測しない。Delay 2 が音に入るかは測らないと分からない。判定しない)。
- S1: wasm ÷ (FLEngine 22,227,016 + realmod の native 合計 84,942,208 + 3,731,968 = 88,674,176)。

## 撤回条件

- (1) 増分の実測が予測の ±20% を超えたら、k_C の仮定(2 モジュールの係数)を見直す(0407 の撤回条件 (1) と同じ)。
- (2) Problems 窓が 5 件にならない、または Delay 2 が一覧に残るなら、原因を分類(a / b)して直す。
- (3) 再生 10 秒で止まる、または `fle: unsupported` か名前つきの停止が出たら、止まった VA を API 名に直し、契約を実装する(0396 の撤回条件 (1) と同じ)。実装できない間は、Delay 2 の行を modules.tsv から外した版を測る。

## 実測(evidence/R5-6a.md。数字の出典はそこ)

- wasm: 158,494,081 → 165,776,779 B(+7,282,698 B)。予測 7,665,049 B に対し −4.99%。撤回条件 (1)(±20%)は発動しない。
- Problems 窓(目視): 対照 6 件 ×3 → 取り込み後 5 件 ×4。Delay 2 が消えた(予測どおり)。
- Export: 対照は今日も 94c93666…4090(R5-5 と一致)。取り込み後 3066e51f…f01e。値は変わった。正しさは判定しない。
- 再生 10 秒: 取り込み後 2 回とも止まらない。
- P1(n=5 交互): 取り込み後の中央値 4,109.5 ms、対照 4,043.6 ms。差 +65.9 ms、範囲は重なる。
- S1: 1.8659 → 1.8695(分母 84,942,208 → 88,674,176 B)。2 倍(177,348,352 B)に対し余裕 11,571,573 B。
- 失敗 1 回: r56a の bundle の hybrid(9c)が、m10 の単位が一覧に無いため止まった(決定 9。evidence/R5-6a.md §4)。
- 公開側: 6fb75fbe と 0322bfec は、別の worker の push で origin に入り、run 43944303574(success)が取り込みの engine を公開の manifest に載せた(`docs/failures/0023-…`)。取り消すかどうかは main の判断。

## 出典

- evidence/R5-6a.md(この取り込みの実測)、evidence/R5-6.md(census・S1 の式)、docs/decisions/0396-g9-balance-filter-real-modules.md、docs/decisions/0407-g9-rest-census-and-s1.md、evidence/R5-5.md・R5-5b.md、docs/ROADMAP.md の R5-6a の行。
- `~/work/handover/evidence/01a-user-messages-full.md`(上の件数)。
