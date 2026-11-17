# 0407: G9 の残り 5 モジュールの census と S1 の見積り(Delay 2・Frequency Shifter・Soundgoodizer・Parametric EQ 2・FLEX。実モジュール化はしない)
分類: 測定(census と見積り。lift・app は変えない。増分は実測ではなく見積り)。撤回条件: 下の §撤回条件。
日付: 2026-11-17   ROADMAP: R5-6   一覧: G9、S1   コミット: この文書を含む commit(commit 名と push の確認は evidence/R5-6.md §10)

## 0 範囲と件数(設計の前の確認)
- 旧決定・記録を読んだ件数: 決定 3 件を読んだ(0396 の全文。0309 と 0315e は G9・Frequency Shifter・Soundgoodizer の行を grep で読んだ)。記録 4 件を読んだ(R5-5 の全文。R6-28・R6-31 は先頭 60〜80 行。R6-12 は先頭 40 行)。ROADMAP の R5-5・R5-6・R5-7・R6-28・R6-31・R6-32 の行。仕様は `docs/core/performance.md` §1〜§3(S1 の定義と旧分母)。
- 検索の件数(`grep -rl`): 5 モジュール名(Delay 2・Frequency Shifter・Soundgoodizer・Parametric EQ 2・FLEX)に一致するファイルは `docs/` の 57 件。`docs/decisions` と `docs/archive/decisions` では 21 件。G9 に一致する決定は 40 件。全部は読んでいない(件数だけの記録)。
- 依頼者の発言: 前任の全文 `~/work/handover/evidence/01a-user-messages-full.md` の発言は 263 件(`grep -c '^## seq'` の結果 263)。
  - FL のプラグイン・G9・同梱の範囲・プラグインの UI に関わる発言は 4 件: seq 34564(msg_19d8b7e90001ARNs0DXVNMmDEc、JST 2026-11-03 03:35:26。G9 の範囲「全プラグインの動作も含まれます」と、サイズの許容「ネイティブの 2 倍まで。可能な限り 1.2 倍」。S1 の根拠)、seq 22192(msg_18c851df70019wQk2QuiM4W1Jh、JST 2026-10-30 20:14:56。FL の窓のデタッチ、プラグインの窓を含む)、seq 55145(msg_1a891fb3d0017x2dSsfegheTZ9、JST 2026-11-05 06:58:21。プラグイン追加時の半透明オーバーレイ)、seq 55453(msg_1a8afca54001VjgPvO3mg76Qfl、JST 2026-11-05 07:30:55。プラグインピッカーの半透明)。
  - 関係しない「プラグイン」(OpenCode の plugin の話): 9 件: seq 5229・5436・6965・7146・7317・7408・7544・7795・24699。
  - サイズに関わるが FL のプラグインではない: seq 33918(wasm が 4.1 kB と小さいこと)。
  - 訂正(前の決定 0396 §1 の誤り): 0396 は「依頼者の発言は 1 件(No. 28、seq 7317 `なぜプラグインが機能していないのですか？`)。プラグインの一部を指すと読める」と書いた。文脈を確かめた結果、seq 7317 は OpenCode の停止監視プラグイン(seq 6965 の `~/stall-resume.ts`、seq 7146 の `.opencode/plugins/`、seq 7408 の `このプラグインのどこにも監視の要素はない`)の話で、FL のプラグインではない。S16-intent/catalog.md の No. 28 も同じ話。0396 の『依頼者の発言』の節は、この 1 件を外して読む。

## 決定
- 決定: 5 モジュールの census は、fe-pe の rebase(未使用の基底 0x7800000・0x8000000・0x8800000・0x9000000・0x9800000)→ fe-lift lift(`--pdata --extra-imports lift/harness/seeds/ship6-imports.txt`、1 ファイルの C。`--split-bytes` は付けない)で行う。根拠: gen-all(lift/crates/fe-lift/src/gen_all.rs)が 1 モジュールごとに呼ぶ形と同じ。Balance・Filter で同じ手順を行い、生成 C が R5-5 の `m8.c`・`m9.c` と `cmp` で一致した(§検証 1)。
- 決定: hook 点は、Balance・Filter と同じ `lift/harness/hooks/fruity-delphi-effect-hook-points.txt` を試した。5 本とも Delphi 製(`Embarcadero Delphi for Win64` の文字列と `DelphiPlug` の文字列があり、engine の文字列は無い)。結果: Fruity Delay 2 だけは共有ファイルで通った(`hook_points 3`)。Frequency Shifter・Soundgoodizer・Parametric EQ 2・FLEX は共有ファイルでは停止した(`hook point 0x… is not a function entry`)。根拠: 共有ファイルの注釈に、他の Delphi 製は別の RVA だと書いてある。失敗の分類は (a)(我々が Balance の入力を別の DLL に当てた。製品の契約ではない)。
- 決定: Frequency Shifter・Soundgoodizer・Parametric EQ 2 は、文書の値(`hooks/*-own.txt`。`docs/archive/decisions/0315e-frequency-shifter-step.md` と ci/・hook 点の注釈の値)で通した。先頭 5 バイトを照合した(§検証 3)。根拠: 3 本とも文書の値の先頭バイトが Balance と同じ 3 つのパターン(`55 56 53 c6 44`・`53 48 83 ec 20`・`55 48 83 ec 50`)に一致した。ただし、fe-lift が受理したことは、その関数が正しい対象であることの証明ではない(未確認)。
- 決定: FLEX は hook 点を未特定のまま、hook 点なしで census を取る。根拠: FLEX の注釈の RVA は文書に無い。先頭 5 バイトの 1 つ目(`55 56 53 c6 44`)は .text に 1 か所(0x1b4b0)だけ一致したが、残りの 2 つは一意に決まらない(§検証 3)。
- 決定: FS の Engine_x64.dll・FLEX の engine_x64.dll・engineV2_x64.dll を、本体の依存として別の行で census に入れた。根拠: 0309(G9 の見積り)と 0315e で、この 3 本が本体の依存として記録されている。FS の本体は Engine を読み、FLEX は 2 本の engine を持つ。
- 決定: S1 の分母は、FLEngine_x64.dll と realmod の native DLL の合計(現在の 9 本。5 本を足すと 14 本)。根拠: `docs/core/performance.md` の旧分母 77,442,944 B は、FLEngine(22,227,016 B)と 7 モジュール(55,215,928 B)の合計と一致する(`evidence/R5-6/s1.py` の出力、§検証 4)。「分母がモジュール数で変わる」という performance.md の注は、この定義で読む。
- 決定: wasm の増分の見積りは、主に C の生成量で決める(C-model。依頼の式)。副に native の大きさで決める(native-model)。係数は R5-5 の 2 モジュールから求めた: k_C = 15,375,721 ÷ 73,424,918 = 0.209407 B/byte、k_N = 15,375,721 ÷ 7,499,264 = 2.0503。根拠: 依頼の式。両方を出すのは、2 モジュールだけでは係数の誤差が分からないため。
- 決定: 分類(a・b・未確認)。Frequency Shifter・Soundgoodizer・Parametric EQ 2 の共有ファイルでの失敗は (a)。FLEX の hook 点の未特定は未確認。FS Engine・FLEX engine・engineV2 の VEX・FMA・AVX-512・EVEX の未対応は (b)(fe-lift の命令の契約の拡充。0032・0308 §9)。新しい旗・環境変数・`#ifdef`・URL パラメータは足していない。

## 撤回条件
- (1) 5 モジュールの実取り込み(gen-all に入れて wasm を実測)の増分が、§S1 の A の見積り(約 47.8 MB)から 20% 以上ずれたら、係数の 2 モジュールの仮定を見直す。
- (2) FLEX の engine 2 本を取り込む(VEX・EVEX の対応が入る)前提が変わると、S1 の C 型が 2.02 になる。この式で 2 を超える間は、FLEX の engine を外す版で S1 を測る。
- (3) 旧分母(FLEngine + 7 モジュール)と現在の分母の定義がずれていると分かったら、§決定 の分母を直す。

## 検証(実測。数字の出典は evidence/R5-6.md)
1. 較正: fe-lift が Balance(0x6800000)と Filter(0x7000000)から出した C は、36,346,414 B と 37,078,504 B。R5-5 の `onat/v-tb2d2/gen/m8.c`・`m9.c` と `cmp` で一致(`evidence/R5-6/census.sh`、`evidence/R5-6.md` §3)。
2. 入力: 5 本の抽出元は `flstudio_win64_26.1.7.5653.exe` の NSIS ストリーム(`tools/extract_nsis2.py --write`)。Balance と Filter の SHA-256 は ci/extract-dlls.tsv の値と一致(a399cbe0…、f4801444…)。engine 3 本は、同じ installer の同名候補の全ストリームの SHA-256 を計算し、一致を確かめた(`evidence/R5-6/engine-cands.py`。FS Engine 1e14656b…、FLEX engine a45f66f3…、engineV2 e4755e14…)。
3. hook の先頭バイト: `layout-check.py` の出力。Delay 2 は共有ファイルの 3 つの RVA(0x17bc0・0x31d70・0x31d90)が Balance と同じ先頭バイト。FS(0x17e90・0x320b0・0x320d0)・PEQ2(0x17ee0・0x32100・0x32120)・SG(0x1b7b0・0x389e0・0x38a00)も、3 つとも同じ 3 パターン。FLEX は 1 つ目のパターンが 1 か所(0x1b4b0)だけ。
4. 分母: `evidence/R5-6/s1.py`。FLEngine 22,227,016・9 モジュールの native 合計 62,715,192(分母 84,942,208)。5 本の native 合計 23,247,872(分母 108,190,080)。
5. 占有範囲: `layout-check.py` で、FLEngine(0x400000〜0x1cc0000)・既存の 9 モジュール・新しい 5 本の範囲(基底〜基底+SizeOfImage)に重なりは無い(overlaps: none)。ただし FLEX の engine の SizeOfImage は 0x199d000(26.6 MB)で、engineV2 と既存の elastique(0xb000000)と重なる仮の基底を使った。census では個別に lift するので問題はない。ROADMAP の行に、配置には 26.6 MB の空きが要ると書く。

## S1 の見積り(式と数字。performance.md の S1 の定義)
- S1 = wasm ÷ (FLEngine + realmod の native 合計)。現在の wasm の基準は R5-5 の実測 158,494,081 B(HEAD 9135826。R6-31 の 158,281,333 B は別の build の値なので使わない)。
- 現在: 158,494,081 ÷ 84,942,208 = 1.8659(9 モジュール)。旧: 143,118,360 ÷ 77,442,944 = 1.848。
- 5 本の見積り(C-model。増分 = k_C × C の生成量の和):
  - A(5 本の本体): C 和 228,051,835 B → 増分 約 47.76 MB → wasm 約 206.25 MB ÷ 分母 108,190,080 = **1.906**(2 倍以内。余裕 約 10.1 MB。C の許容量は 276,428,052 B。見積り 228,051,835 B に対し 21% の余裕)。native-model では 1.906。
  - B(A + FS の Engine): **1.907**。
  - C(A + FLEX の engine 2 本): C 和 422,178,204 B → 2.024(**2 倍を超える**)。native-model では 1.922。
  - D(A + 3 本の engine): 2.024(native-model 1.922)。
- 判定: 5 本の本体だけなら 2 倍以内(A・B)。FLEX の engine 2 本を入れると、C-model では 2 倍を超える(C・D)。native-model では超えない。どちらのモデルが正しいかは未確認(2 モジュールから求めた係数で、MSVC 製の engine には当てはまらないかもしれない)。
- 目標の 1.2 倍(wasm 約 129.8 MB)には届かない。現在でも 1.866 で、1.2 倍は超えている(旧記録の 1.848 から同じ傾向)。

## 未確認
- engine 3 本の C の大きさと、AVX・EVEX 命令の未対応(FS Engine 692 件、FLEX engine 18,773 件、engineV2 190,303 件)を、fe-lift が出す代わりの命令で実装した後の C の大きさ。未対応の命令が実行時にどう止まるかも未測定。
- FLEX の hook 点の位置(2 つ目と 3 つ目)。
- ildsp_x64.dll への依存(PEQ2・FS・SG・FLEX の DLL に UTF-16 の文字列 `ildsp` がある。0309 G9-d2)。実行時に呼ばれるかは未確認。Delay 2 と Balance には無い。
- 5 本を実際に gen-all に入れた wasm の増分(未実測)。P1〜P3 も未測定。

## 出典
- evidence/R5-6.md(census の表・入力の SHA-256・S1 の式)、evidence/R5-6/census.tsv、evidence/R5-6/census.sh・census-first.sh・engines.sh・engine-cands.py・layout-check.py・s1.py、evidence/R5-6/hooks/*.txt
- docs/decisions/0396-g9-balance-filter-real-modules.md、docs/archive/decisions/0309-g9-entry-estimate.md、docs/archive/decisions/0315e-frequency-shifter-step.md、docs/core/performance.md §1〜§3
- ~/work/handover/evidence/01a-user-messages-full.md(seq の一覧は §0)、~/work/handover/setup/S16-intent/catalog.md(No. 28)
