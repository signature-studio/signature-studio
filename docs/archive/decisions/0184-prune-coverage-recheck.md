# 0184: dsp(Intel IPP)の刈り込み S+M を、決定 0144・0171・0178 のシナリオと出荷物 F2xn の実行で再検定した — 刈った関数への到達は全シナリオで 0 件(S も S+M も)。ただし検定できた DSP は閉包 25,370 関数のうち 355 で、所定プロジェクトの実 DSP は未検定。仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0184、担当 prune2、2026-11-03 16:12〜、予算 3 時間)。作業と測定は HEAD **4933389** の `git archive` した木(`lift/out-prune2/src/`)で行い、`prune2.diff` は**新しいファイルだけ**(`-` 行 0。fe-lift・runtime・harness・app は触っていない)。区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**読み**＝コードや出力を読んで得た解釈で、確かめていない。**未確認**＝確かめていない。**出典なしの選択**＝名前つき(`S-prune2-…`)で `tables/s-choices.tsv` に代替案と撤回条件を書いた。**実機 Windows との照合は 0 件**。速度は主張しない(この機械は 16 コア、load 4〜23 で他の担当が走っていた)。

## 冒頭の三点(正直さ)

1. **実測したもの(範囲)**: 実エンジン(`FLEngine_x64.dll` と同梱 5 DLL を x86-64 → C → native -O0 にした物。生成 C は素の `fe-lift` + 決定 0171/0178 の層 1+2+3 の行)を G3 ハーネスの fork プローブで、**刈らない構成 D3**(関数の入口を数える)と**S+M を刈った構成 PSM3**(刈った関数が実行されると名前つきで abort)で動かした。シナリオ: 起動 16 スナップ、Basic.flp・Sampler.flp を開いて再生しミキサー/チャンネルラック/テンポを操作、空プロジェクトの再生、決定 0171 の 133 操作+深い操作 73 の計 206 プローブ、File → Open ダイアログ経由の Sampler.flp と参照プロジェクトの FLP、Save as・Export の入口。さらに**出荷物そのもの**(F2xn の wasm 136,610,961 B、prune S+M)を node で、Basic/Sampler の再生、空プロジェクトの **370 仮想秒**の再生、決定 0171 の 195 個の固有プローブで実行した。**模擬・探索・推測**: ダイアログの『選択』はハーネスが与えた(利用者の入力の模擬)。`FLE_CMP_EXTEND`・`FLE_LOADLIB_MISSING` は dlg1 の探索用の仮定で、結果に数えない。プラグイン(Fruity Wrapper など)は動いていない。代役・画像の再生・事前生成は無い。
2. **結論**: **刈った関数(S 5,560 / S+M 18,489)のエントリは、すべてのシナリオ(native の D3 222 個の coverage ファイル、PSM3 の突き合わせ、出荷物 F2xn の node 実行)で 0 件**。『MODEL FALSIFIED』は出ず、入口に入った dsp の 355 関数はすべて M のモデルの内側、選択子は全出力で 0/0/−1/−1・マスク 7(CPUID を SSE2 のみに固定した結果と一致)。**ただし**、この 355 は閉包 25,370 の 1.4% で(要求された 420 export のうち入口に入ったのは 66)、UI 操作はほぼ DSP を使わず、再生で増えるのは ippsAdd_32f_I・ippsConvert_16s32f_Sfs・ippsSet_64s・ippsThreshold_LT_64f_I・ippsVectorSlope_64f・ippsSin_64f_A50 の 6 つの export とその内部を合わせた 20 関数(W = 刈らない側)だけである。**M のモデルを最も強く検定するはずの経路(所定プロジェクトの Fruity プラグイン・Serum 2・エフェクト・書き出し・サンプルの復号)は 1 本も走っていない**(FLP の読み込みが Fruity Wrapper の DLL で止まる。Export は IFileSaveDialog のモデルが無く、Save は Trial のため Buy FL Studio になる)。よって『S+M を出荷してよいか』には二値で答えない: **S(強い。環境の定義だけが根拠)は定義上変わらない。S+M は『確認した範囲では反証 0 件』であって、所定プロジェクトの実 DSP についてはモデルの仮定のまま(abort が保険)**。M のモデルは完全でなく(観測した辺の 5.8% を予測しなかった。§6)、S+M を条件つきにする保守的な代替(§7。**未ビルド=推計**): データモデル `extra_refs=1` だけなら F2 = 139.69 MB(S1 の 141.15 MB まで余裕 1.46 MB)、それに未要求 26 export を根に足した `prune-SM2.tsv` なら 140.84 MB(余裕 0.3 MB)。前提の再確認(§5): case 3〜6 が到達不能という前提は CPUID だけでなく『CPUID・XGETBV・IsProcessorFeaturePresent の runtime の答えが SSE2 のみで矛盾しない』ことに依り、FLEngine と Fruity Wrapper に CPUID 以外の機能判定の経路は見つからなかった(静的+gdb)。**CPUID leaf 7 には SDM の規則(最大 leaf = 1 の CPU は高い leaf に leaf 1 の情報を返す)で EDX = 0x07808101 が返り、leaf 7 を最大 leaf の確認なしに引く単位があれば AVX512_FP16 等が立って見える可能性がある**(§5。IPP は leaf 7 を引かないので現在の依存は 0。起動 16 スナップでは leaf 7 を全 0 にしても結果〔時計・入口数・画素〕は完全に一致=効かない〔実測〕。再生・操作の範囲は未実験)。
3. **未確認**: 所定プロジェクトの実 DSP(上)、ラッパー経由でプラグインが動いた段階の dsp の入口集合(§8: ラッパーが GetProcAddress で引く名前は 420 名で、420 の外は 0=追加の根なし。入口の coverage は未取得)、Export の中身(IFileSaveDialog のモデルが無い)、Save の中身(Trial は保存できない)、層 3 を入れた出荷用 F2x の wasm、ブラウザ(Chrome Worker)での実行、`prune-SM2` の実測。確認していない操作の一覧は §9。

## 1. 先に結論 — シナリオ別『刈った関数のエントリ数(S / S+M)』

| 構成 | 実行したシナリオ | 刈った関数のエントリ S / S+M |
|---|---|---|
| native D3(刈らない、入口を数える) | §3 の表の全シナリオ。222 個の coverage ファイルの和 | **0 / 0**(dsp 355 関数が入口に入り、うち 335 は起動 16 スナップと同じ、20 が再生で増えた W) |
| native PSM3(S+M を刈った版。abort で検出) | F2(148 プローブ: Basic・Sampler・空プロジェクト/Song の再生・ダイアログ経由 Sampler・参照プロジェクト 2 通り・Save as・Export の入口・決定 0171 の 133 操作)、F3(深い操作 73)、explore1(Basic を開き 3 s 再生 + F9) | **abort 0**、『indirect call … not a known function entry』0 |
| 出荷物 F2xn(wasm 136,610,961 B。node) | Basic・Sampler(各 5 s の操作と各 244 仮想秒の長い再生)・空プロジェクト 370 仮想秒・決定 0171 の 195 固有プローブ(206 個)・追加の 5 プローブ(計 206 走行) | **abort 0**、indirect-call 停止 0 |
| 負の対照 PX3(PSM に実行される関数 1 つを足した) | 起動 | abort 1(『fle: pruned function 0x2001040 was executed』、16,323 入口)。検出も働く |

## 2. 方法

**ビルド(実測)**: HEAD 4933389 の `git archive` を `lift/out-prune2/src/` に展開し、`tools/gen-native.sh`(`lift/harness/a4-multi.sh` を、`app/scripts/build-engine-bundle.sh` の `gen` と同じモジュール表・seed・env で呼ぶ)で、FLEngine と 5 つの DLL を x86-64 → C → native -O0(`-finstrument-functions`)にした。素の `fe-lift`(`--dead-flags`・`--outline-shared`・HLE のエントリなし。S-prune2-judge-plain)、行は `harness/seeds/dlg1-imports.txt`(決定 0171 の層 1+2 と決定 0178 のダイアログの行 = 層 3)。生成+コンパイルは各 約 3 分(この機械、16 コア)。

| 名前 | 内容 | 検査 |
|---|---|---|
| **D3** | 刈らない(dsp 25,370 関数の全部)。`FLE_COV_FILE` で関数ごとの入口数を数える | 起動の 30 秒確認: 5,398 関数・1,052,558 入口 |
| **PSM3** | `--prune-list prune-SM.tsv`(dsp だけ)。刈った関数が実行されると `fle: pruned function 0x… was executed` で abort | 同じ 5,398 関数・1,052,558 入口(D3 と一致)。dsp のオブジェクト(native -O0)は 615 MB → 180 MB |
| **PX3** | PSM3 に、起動で実行される関数 1 つ(0x2001040)を足した**負の対照** | 16,323 入口で abort。メッセージは stdout の『runtime message』と stderr の両方に出て、`run-g3.sh` の『pruned function』の数え上げ(stdout 1、stderr 1)で検出された |

**fork プローブ + 個別 coverage**: 決定 0171 の fork プローブ(`probe NAME … probe-end`)の子は `_exit(0)` するので `FLE_COV_FILE` は何も書かない。`tools/cov-per-probe.patch`(`harness/wasm/trace_native.c` に 2 関数、`fruity_g3.c` に 3 行。環境変数が無ければ何もしない)で、子が `<FLE_COV_FILE>.<プローブ名>` に書くようにした。coverage の数は親の起動分を含む(子は fork 時点のカウンタを引き継ぐ)ので、シナリオの『新規』は起動 16 スナップの coverage との差で数える。**正の対照**: `prune_hits.py` に、実行された関数 1 つを加えた表(`prune-SM-mutant.tsv`)を渡すと 1 件検出する(起動 coverage で 0x2001040 が 1 入口)。

**判定**: `tools/prune/judge.py`(決定 0168。変更なし)を、HEAD の素の dsp lift(`fe-lift lift dsp_ippv2_x64.2000000.dll … --seeds dsp-ippv2-2000000-seeds-idle1.txt`、491,094,508 B、sha256 `2c880576…` = prune1 の記録と一致)に当てた。**prune1 の数字を再現**: S 5,560 / M 12,929 / W 6,546 / K 335(起動 coverage を入れたとき)/ 『M の外に入口に入った関数』0、`prune-S.tsv`・`prune-SM.tsv` の (VA, 区分, 層, C bytes) は完全に一致。この上で `prune_hits.py`(各シナリオで S/S+M のリストの関数が入口に入ったか)と `summarize.py`(シナリオ表)を新しく書いた。

**突き合わせ**: (1) D3 の入口の数え上げ(0 件なら、PSM3 は abort しない)。(2) PSM3 を同じシナリオで走らせた abort の有無(F2 の 148 プローブ・F3 の 73 プローブ・explore1 の 1 走行で、刈った関数の abort 0、indirect-call 停止 0。exit が 0 でないのは 5 個(h_about、h_buy、help_item1、p_ref_missing、p_ref_nomissing。いずれも『名前つき停止』で刈った関数ではない: 参照プロジェクトの 2 つは Fruity Wrapper の壁・ACM の入口、help 系の 3 つは決定 0171 の既知の停止)。D3 と同じ名前のプローブ 210 個のうち、g3 の行(仮想時計・入口の総数・全面 SHA-256・窓表 SHA)が **完全に一致 210・不一致 0**(`tables/psm-vs-d3.tsv`))。(3) 出荷物 F2xn を node で(§4)。三つとも、刈った関数への到達は 0 件で一致した。

**ダイアログの選択(模擬)**: `FLE_FILEDLG_PICK_FILE`(1 行 1 ゲストパスのホストのファイル)。ハーネスが与える。`Show()` のたびに stderr に『THE SELECTION WAS GIVEN BY THE HARNESS』が出る(決定 0178 §2)。

**VFS の種**: `tables/vfs-seed-prune2-all.tsv` = 決定 0178 の g8 の種(起動 + 製品ファイル + Basic.flp・Sampler.flp ほか)から参照プロジェクトの 49 ファイル(`~/ダウンロード/serina1-serum2.zip` を `lift/out-prune2/proj/` に読み取りで展開。zip は無変更)を『ダウンロード\serina1-serum2\』に置く行を足したもの。

## 3. シナリオ別の結果(native -O0、刈らない D3 で入口を数えた。実測)

『刈った関数の入口』は、`prune-S.tsv`(5,560 関数)/`prune-SM.tsv`(18,489 関数)に載る関数が 1 回でも入口に入った数(異なる関数の数 / 入口の総数)。**すべての行で 0 / 0**。『dsp 関数』はそのシナリオ(の coverage ファイルの和)で入口に入った dsp_ippv2 の関数の数(閉包は 25,370)、『起動に無い』は起動 16 スナップの 335 関数に無かった数(すべて W = 刈らない側)。表は `tables/scenario-hits.tsv`。

| シナリオ | 内容 | 実行数 | 終わり方 | dsp 関数 | 起動に無い | 刈った関数の入口 S / S+M |
|---|---|---|---|---|---|---|
| `s16` | 起動 + ウィザードを閉じるまでの 16 スナップ(決定 0161/0168 と同じ) | 1 | スクリプトの終わり | 335 | 0 | 0 / 0 |
| `flp_bs_standalone` | (a) Basic.flp を WM_COPYDATA で開く → 再生 5 s → ミキサー(つまみ 2・フェーダー 2・ミュート)・ステップ・チャンネル vol/pan・テンポのドラッグ → PAT/SONG → Song 4 s → 停止。続けて Sampler.flp を開く(『保存しますか』の Confirm が出て開かず、Space で Buy FL Studio が開く) | 1 | スクリプトの終わり | 352 | 17 | 0 / 0 |
| `F1:p_basic` | (a) 上の Basic の部分だけ(follow セッションの fork プローブ) | 1 | exit 0 | 352 | 17 | 0 / 0 |
| `F1:p_sampler` | (a) Sampler.flp を開く → 再生 5 s → ミキサー・テンポ → 停止 | 1 | exit 0 | 355 | 20 | 0 / 0 |
| `F1:p_dlg_sampler` | (a)(c) File → Open... のダイアログで Sampler.flp を選ぶ(選択は模擬)→ 再生 5 s → 停止 | 1 | exit 0 | 351 | 16 | 0 / 0 |
| `F5:q_sampler_notes` | (a+) Sampler.flp を開き、チャンネルを選んでコンピュータのキー(A S D F G H)で 6 音を演奏(ピッチごとの再サンプリング) | 1 | exit 0 | 348 | 13 | 0 / 0 |
| `F5:q_basic_long` | (a+) Basic.flp を 20 仮想秒再生し、再生中にテンポをドラッグ | 1 | exit 0 | 339 | 4 | 0 / 0 |
| `F5:q_fx_slot_menu` | (a+) ミキサーの FX スロット(推定座標)をクリック → 何も開かない | 1 | exit 0 | 335 | 0 | 0 / 0 |
| `F5:q_fx_slot_pick` | (a+) 同、メニュー項目(推定座標)をクリック → 何も開かない | 1 | exit 0 | 335 | 0 | 0 / 0 |
| `F5:q_rack_add` | (a+) チャンネルラックの「+」(推定座標)をクリック | 1 | exit 0 | 335 | 0 | 0 / 0 |
| `F1:p_play_empty` | (d) 空プロジェクトを Space で 30 仮想秒再生(native -O0 の計装つきは再生 1 仮想秒に約 10〜20 CPU 秒かかるため短い。300 秒以上は F2xn を node で、§4) | 1 | exit 0 | 335 | 0 | 0 / 0 |
| `F1:p_song_empty` | (d) Song モードで 20 仮想秒再生 | 1 | exit 0 | 335 | 0 | 0 / 0 |
| `F1:p_implog_basic` | Basic.flp を開いて再生 3 s の間、すべての import 呼び出しを記録(`importlog on`) | 1 | exit 0 | 335 | 0 | 0 / 0 |
| `ops_A_91_probes` | (b) 決定 0171 の操作 probes-1・2(ツールバー全ボタン、ブラウザ、チャンネルラック、ミキサー、プレイリスト、窓ボタンほか) | 91 | 全部 exit 0 | 339 | 4 | 0 / 0 |
| `ops_B_42_probes` | (b) 同 probes-3(メニューバー 8 項目、キー 42 ほか)。A+B = 決定 0171 の 133 操作 | 42 | 全部 exit 0 | 335 | 0 | 0 / 0 |
| `ops_C_47_probes` | (b) 同 probes-4〜6(再実行・変種)。同名のプローブがあるため coverage ファイルは 37 | 47 | 全部 exit 0 | 339 | 4 | 0 / 0 |
| `ops_D_26_probes` | (b) 同 probes-7〜9(File 10 か所・Help 10 か所・右クリック・importlog つき) | 26 | 23 が exit 0、3 は決定 0171 の既知の停止(help_item1・h_buy〔ShellExecuteW〕、h_about〔GetPrivateProfileStringW〕) | 335 | 0 | 0 / 0 |
| `ref_nomissing_standalone` | (c) File → Open... で参照プロジェクトの FLP(選択は模擬、FLE_CMP_EXTEND=1) | 1 | `LoadLibraryW: an empty module name`(Fruity Wrapper の壁。決定 0178 と同じ) | 335 | 0 | 0 / 0 |
| `F1:p_ref_nomissing` | (c) 同じものを follow セッションのプローブで | 1 | 同上 | 335 | 0 | 0 / 0 |
| `F1:p_ref_missing` | (c) 探索: 上に FLE_LOADLIB_MISSING=1(プラグイン DLL は『見つからない』という仮定。結果ではない) | 1 | `unsupported instruction at 0x6708e0`(= msacm32!acmDriverEnum の thunk。MP3 の ACM の入口) | 335 | 0 | 0 / 0 |
| `F1:p_saveas` | (e) Basic.flp を開いた状態で File → Save as | 1 | exit 0。ダイアログは開かなかった | 335 | 0 | 0 / 0 |
| `F1:p_save_dialog` | (e) 空プロジェクトで File → Save as → エンジンの Save as ダイアログ(TNewProjForm)→ Save を押す | 1 | exit 0。Save で『Buy FL Studio』(TInAppShopForm)が開く(Trial は保存できない) | 335 | 0 | 0 / 0 |
| `F1:e1_menu` | (e) File → Export のサブメニューが開く(Audio: Wave file... Ctrl+R、MP3、OGG、FLAC、MIDI、M4A、Last exported、Master、Upload ほか) | 1 | exit 0 | 335 | 0 | 0 / 0 |
| `F1:p_export_wave_menu` | (e) メニューから Wave file... を選ぶ(座標は推定) | 1 | exit 0。何も開かない | 335 | 0 | 0 / 0 |
| `F1:p_export_ctrlr_empty` | (e) Ctrl+R(Wave file...)。空プロジェクト | 1 | exit 0。エンジンが `CoCreateInstance(CLSID_FileSaveDialog {C0B4E2F3-…}, IID_IFileSaveDialog {84BCCD23-…})` を要求し、モデルが無いので何も開かない | 335 | 0 | 0 / 0 |
| `F1:p_export_ctrlr_basic` | (e) Ctrl+R。Basic.flp を開いた状態(SHGetKnownFolderPath(Documents) → 同じ CoCreateInstance) | 1 | exit 0。同上 | 335 | 0 | 0 / 0 |
| **全シナリオの和**(222 個の coverage ファイル) | 上のすべて | 229 回の実行 | — | **355**(K 335 + W 20) | 20 | **0 / 0** |
**要求された 420 export のうち入口に入ったのは 66 だけ**(`tables/dsp-entered-union.tsv` の export 名の列と `exports-requested.tsv` の突き合わせ。UI の描画用の `ippi*`〔画像〕と、再生の `ipps*` 数本、`ippMalloc`/`ippFree`)。**354 の export は 1 度も呼ばれていない**ので、その先の内部関数に M のモデルの穴があっても、これらのシナリオでは検出されない。

## 4. 出荷物そのもの(F2xn)を node で動かした結果 — 刈った関数への到達 0、indirect-call 停止 0

**対象**: `lift/out-websrv/assets/fl-engine-fin1-F2xn/`(決定 0176 の F2xn。wasm `engine.eea68773e13b.wasm` 136,610,961 B、sha256 `eea68773e13b7975…86da`〔manifest と一致を確認〕、prune-SM.tsv〔S+M〕、層 2 までの行、層 3〔dlg1〕の行は無い、HEAD 4aac2d6 でビルド)。**ビルドし直していない**: 同じファイルを、決定 0176 のブラウザ用ではなく本決定の `tools/node-bundle-run.mjs` で動かした。

**方法(S-prune2-node-glue)**: ブラウザ用の glue(`engine-glue.*.js`)は web+worker 用に作られていて node を拒む。そこで glue の**複製**の 2 か所(node 版の最小 runtime 検査と node 検出)だけ無効にし、`WorkerGlobalScope` を定義して worker として動かした。wasm・DLL pack(flpack1)・seed pack(flseedp1)・env〔manifest の値〕はバンドルのもの。seed pack は決定 0178 の g8 の種(Basic.flp・Sampler.flp ほか)を `pack_seed.py` で作り直したもの(`node/seed-g8.flpack` 6.0 MB)。G3 スクリプトは MEMFS の `/data/script.g3` に置き `FLE_G3_SCRIPT` で渡す。出力の `fle: pruned function 0x… was executed` と `indirect call to 0x… is not a known function entry` を数える。起動から最初の idle まで約 9〜11 秒(この機械、load 5〜13)。

| シナリオ(スクリプト) | 内容 | 壁 | 刈った関数への到達 | 終わり方 |
|---|---|---|---|---|
| `flp-basic.g3` | Basic.flp を `open`、再生 5 s、ミキサー/チャンネルラック/テンポの操作、PAT/SONG、Song 4 s、停止 | 23 s | 0 | スクリプトの終わり(exit 134 = 通常) |
| `flp-sampler.g3` | Sampler.flp を `open`、同じ操作 | 24 s | 0 | 同上。主窓の題が `Sampler.flp - FL Studio 2026` |
| `flp-basic-sampler.g3` | 上の 2 つを続けて(2 つ目は『保存しますか』の Confirm〔TMsgForm〕で止まり、Space で『Buy FL Studio』が開く。native と同じ) | 23 s | 0 | 同上 |
| `play-empty-300.g3` | 空プロジェクトを Space で 310 仮想秒再生(パターン)+ Song 60 仮想秒 = **370 仮想秒**、停止 | 76 s | 0 | 同上。主窓の面の SHA は各スナップで変わる(再生位置の表示) |
| `play-basic-long.g3` | Basic.flp を 180 仮想秒再生(パターン)、再生中にテンポを 2 回ドラッグ、Song モード 60 秒、停止(計 **244 仮想秒**) | 67 s | 0 | スクリプトの終わり |
| `play-sampler-long.g3` | Sampler.flp で同じ(244 仮想秒) | 179 s | 0 | 同上。主窓の題が `Sampler.flp - FL Studio 2026` |
| `ops-E`(5 プローブ) | Sampler のキー演奏 6 音、Basic の 20 秒再生+テンポ変更、ミキサーの FX スロット/チャンネル追加メニューの座標クリック(座標は推定で、窓は開かなかった) | 各 11〜25 s | 0 | スクリプトの終わり |
| 決定 0171 の fork プローブ 206 個(別の実行ファイルに分けた 195 個の固有スクリプト) | 起動 → ウィザードを閉じる → 各操作(ツールバー全ボタン、メニューバー 8 項目、ブラウザ、チャンネルラック、ミキサー、プレイリスト、窓ボタン、キー 42、深い操作) | 各 10〜25 s | **0** | 190 個はスクリプトの終わり。5 個は別の終わり方(下) |

別の終わり方の 5 個(**どれも刈った関数とは無関係**): `win_close`・`file_exit`(メインウィンドウを閉じる/File → Exit。エンジンが exit 0 で終わる。native の fork プローブでも exit 0)、`help_item1`・`h_buy`(shell32!ShellExecuteW が未対応: 『unsupported instruction at 0x5928b0』)、`h_about`(kernel32!GetPrivateProfileStringW: 『unsupported instruction at 0x4235d0』)。この 3 つは決定 0171 が最終構成でも残した停止 3 件と一致する。

**負の対照**: wasm 側の『刈った関数の abort』の検出を、私は wasm では再現できない(刈った関数を入れた変異 wasm のビルドをしていない)。決定 0176 の `fl-engine-fin1-PXxn`(変異 1 つを入れた F2xn。ブラウザで abort を確認)がその対照だが、このディレクトリは今は無い。**代わりに native の変異 PX3 で、同じ『pruned function』の文字列検出が働くことを確かめた**(§2)。出荷物の wasm の data セクションに `fle: pruned function %#llx was executed (prune list, decision 0168; …)` と `indirect call to %#llx is not a known function entry` の書式文字列があることを確かめた(それぞれ 1 か所)ので、検出に使った文字列は実際に出力される形である。wasm の stderr 取り込みが働いていることは、同じ run の stderr に runtime の窓表の出力が入っていることで確かめた。

## 5. 前提の再確認 — IPP の選択子 case 3〜6 が到達不能という前提は、CPUID の固定(決定 0118)だけに依るか

**問い**: dsp_ippv2 の CPU 別選択子(433 のスタブ、4 つの選択変数、3,091 の表スロット)が case −1/0 だけを取るのは、CPUID の固定の結果である。FLEngine や同梱 DLL が CPUID 以外(`xgetbv`、`IsProcessorFeaturePresent`、レジストリ、環境変数、`GetSystemInfo`)で CPU 機能を判定し、IPP の別の case を選ばせる経路がないか。

**静的(実測。`tables/cpu-sites-*.tsv`、`tables/cpu-imports.txt`)**

| 経路 | FLEngine | dsp_ippv2 | 備考 |
|---|---|---|---|
| `cpuid` 命令 | 5 か所(f_40f470〔ラッパー、呼び出し元 f_40f490〕、f_477730〔f_477790〕、f_639900〔呼び出し元 6 関数〕、f_6399e0〔f_639a20〕に 2 か所) | 133 か所(73 関数。うち入口に入った K は 9 関数) | FLEngine の 4 関数は 0x639xxx の MSVC CRT(`__isa_available_init` 系)と cpuid ラッパー。f_6399e0 は『leaf 0 → leaf 1 → ECX の OSXSAVE|AVX を見てから xgetbv』の定型 |
| `xgetbv` 命令 | 1 か所(f_6399e0。ECX&0x18000000 が両方立つときだけ) | 4 関数(うち 2 つが K、2 つが W) | runtime は `fle_xgetbv_stop`(`fle: xgetbv at …`)で名前つき停止。leaf 1 の ECX = 0(OSXSAVE なし)なので実行されない |
| `IsProcessorFeaturePresent` | **import していない** | import があり、**呼び出し 5 か所と jmp 1 か所**(0x37cada6、0x37cae8d、0x37cb670、0x37d095f〔jmp〕、0x37d25c6、0x37d2a3d)がある。lift は IAT スロット経由のこの呼び出しを『lift されない』(`fle_unsupported`)にしていて、**実行されれば『unsupported instruction at 0x…』で名前つき停止**する。含む関数 5 本のうち K は f_37ca8f0 だけ(入口 181 回。読み: `__security_check_cookie` 型で、該当箇所はそのエラー経路)、他の 4 本は W(どのシナリオでも入口に入らない) | runtime の答え(`fle_imp_IsProcessorFeaturePresent`)は SSE2 のみと一貫(PF 2・3・6・10 だけ nonzero、AVX 系は 0、未決の番号は名前つき停止)。dsp の閉包にこの runtime 関数を呼ぶコードは無い |
| `GetSystemInfo`/`GetNativeSystemInfo` | 2 関数(f_407230、f_423700) | import していない | プロセッサ数・ページ大きさ。IPP の選択には使われない(dsp が import していない) |
| レジストリ/環境変数 | `RegOpenKeyExW/A`・`RegQueryValueExW`・`GetEnvironmentVariableW` を import | `GetEnvironmentStringsW` だけ(CRT) | 仮想レジストリの種は決定 0116 の `reg-seed-nsis.tsv`・`reg-seed-app.tsv`(Image-Line の設定だけ)で、`HARDWARE\DESCRIPTION\System\CentralProcessor` は無い。dsp は環境変数を読まない |
| IPP を呼び分ける export | FLEngine は `ippSetCpuFeatures`(f_2001000。選択変数 0x4541000 に引数を書く)も `ippInit` 系も要求しない(要求 420 名に無い。決定 0168 の未要求 26 名に入っている) | — | 134 の Fruity プラグインは同じ 420 名の表を持つ(prune1 の静的確認)。wrp1 の実走ログでの確認は §8 |

**選択子の決まり方(読み)**: `f_20034d0`(dsp の ippInit 風)が、特徴マスクを返す `f_2065940`/`f_20659c0` の結果の bit(0x100・0x200・0x8000・0x40・0x20・0x10・0x8)を順に調べ、`0x45410a0` に 6・5・4・3・2・1・0 のどれかを書く。マスクが 7(bit 0〜2 = MMX/SSE/SSE2)なら 0x2003601 の分岐(失敗経路)または 0x20035bf の分岐で値は 0。マスクは CPUID の結果だけから作られる(`f_2001820`/`f_20013c0` が `0x4560ec0` に書く)。

**動的(実測)**: 全シナリオの coverage 出力(`FLE_COV_PEEK`、スナップではなく各シナリオの終了時)で、選択変数 `0x4541000` = 0、`0x45410a0` = 0、`0x454f270` = −1(未初期化。スタブが一度も呼ばれない)、`0x455d050` = −1、マスク `0x4560ec0` = 7、初期化済み `0x4560ec8` = 1。222 個の出力すべてで同じ値。**動的(実測。`tools/cpu-api.gdb`・`tools/run-gdb-cpu.sh`、`tables/gdb-cpu-api-summary.txt`)**: D3 のバイナリを gdb で、起動 → 最初の idle → ウィザードを閉じる(仮想 6 秒)の間の、CPU を見せうる runtime 関数の呼び出しを数えた(各ブレークポイントは名前つき関数、引数は guest のレジスタ配列 `R` から読む)。

| 呼び出し | 回数 | 内訳(読み) |
|---|---|---|
| `fle_cpuid` | 65 | leaf 0 ×23、leaf 1 ×19、leaf 2〜7 が各 3、leaf 0x80000000 ×4(+ 0xffffffff80000000 ×1)。呼び出し元(bt のフレーム 1 の f_… のモジュール、`tables/gdb-cpuid-callers.tsv`): FLEngine 15・quickfontcache 15・iltools 15(**3 つは同じ Image-Line の CPU 情報の単位のコピー**。ラッパー f_40f470/f_1d0c790/f_600ae60 → leaf 1〜7 と 0x80000000、ほかに CRT の `__isa_available_init` 系)、Blend2D 6、dsp 14。**dsp は leaf 0・1・0x80000000 だけで leaf 2〜7 は見ない** |
| `fle_xgetbv_stop` | **0** | OSXSAVE = 0 なので到達しない |
| `fle_imp_IsProcessorFeaturePresent` | **0** | 呼ぶコードが閉包に無い(上の静的) |
| `GetSystemInfo` / `GetNativeSystemInfo` / `GetLogicalProcessorInformation` | 14 / 3 / 12 | プロセッサ数・トポロジ(スレッド数の決定に使われると読む) |
| `GetProcessAffinityMask` | 0 | |
| `RegOpenKeyExW/A` | 826 / 7 | 引いたキー(UTF-16 を復号。`tables/gdb-cpu-api-summary.txt` の上位): `General\\FruityLoopsMainForm`、`General\\MIDIForm`、`Search paths`、`MRU`、`Software\\Image-Line\\Shared\\General`、`…\\Registrations\\Token`、`Windows\\1920x1080x0\\…`、`Software\\Embarcadero\\Locales`、`…\\FontSubstitutes` ほか。**`HARDWARE\\…\\CentralProcessor` や `ProcessorNameString` は 0 回** |
| `GetEnvironmentVariableW` | 5 | `LANG` ×3、`CLAP_PATH`、`OneDrive` |

最初の idle より後(fork プローブの `importlog on`、Basic.flp を開いて再生 3 s で import 139 種・371k 回、決定 0171 の il_* プローブで 176 種・652k 回): CPU に関係しうる名前は `RegOpenKeyEx*`・`RegQueryValueExW`・`QueryPerformanceCounter` だけで、`IsProcessorFeaturePresent`・`GetSystemInfo` 系は 0 回。

**CPUID leaf 7 の EDX(SDM の規則の結果。決定 0009/0118 どおりで、不整合ではない)**: `fle_cpuid.c` は『最大基本 leaf = 1、leaf 0 の EAX は 1、最大基本 leaf より大きい EAX には最高の基本 leaf(= leaf 1)の情報が返る』という Intel SDM の規則を実装している(最大 leaf が 1 の CPU の振る舞いの再現)。そのため leaf 7(sub-leaf 0)には EAX=EBX=ECX=0・**EDX=0x07808101** が返る。leaf 0 で最大 leaf を確かめてから leaf 7 を引くコードは読まないが、確かめずに leaf 7 を引く単位(上の FLEngine/qfc/iltools の CPU 情報の単位)があれば、leaf 7 の EDX の bit 割り当てで読んで AVX512_FP16・AMX などが『ある』ように見える可能性がある(この値の bit 8・15・23・24・25・26 が leaf 7 の EDX では VP2INTERSECT・Hybrid・AVX512_FP16・AMX-TILE・AMX-INT8・IBRS/IBPB に当たる、という読みは SDM の表を私の記憶で読んだもの。**再確認は未**)。**IPP(dsp)は leaf 7 を引かない**ので、IPP の選択子には効かない(観測: マスク 7)。runtime は変えていない。**実験(実測。`tools/run-gdb-leaf7.sh`、`tables/leaf7-experiment.txt`)**: gdb の python で `fle_cpuid` の後に leaf 7 の答えを全レジスタ 0 に書き換え(3 回: FLEngine・qfc・iltools の単位。runtime の変更なし)、起動 16 スナップ(cmp-short.g3)を native で走らせた。g3 の 101 行(仮想時計、入口の総数、全面の SHA-256、窓表の SHA、進捗)が、書き換えなしの走行と**完全に一致**した。つまり**起動からウィザードを閉じるまでの範囲では、leaf 7 の値(EDX = 0x07808101 か 0 か)はこれらの単位の挙動(画素・時間列・入口の数)に効かない**。再生・操作の範囲は未実験。

**結論(項目 4)**: IPP の選択子 case ≥ 1 が到達不能という前提は、CPUID の固定だけでなく、**『CPUID・XGETBV・IsProcessorFeaturePresent の三つの runtime の答えが SSE2 のみで互いに矛盾しない』ことに依る**(`fle_cpuid.c`・`fle_sysinfo.c` の注釈にもそう書かれている)。FLEngine 側にこの三つ以外の経路で IPP の case を選ばせるコードは見つからなかった(静的。FLEngine は IPP の CPU 設定 export を引かず、選択変数に書く唯一の外部入口は `ippSetCpuFeatures`)。残る依存: (a) 利用者の機械の実 CPU は一切見えない(wasm なので当然)。(b) 将来の Fruity プラグインや Serum 2 が **別の経路で** CPU を調べて同じ DLL の別の export を呼ぶ場合(特に `ippSetCpuFeatures`)は未確認(プラグインは未 lift)。(c) `xgetbv` と未決の PF 番号は『名前つき停止』なので、別の機能判定を追加すると黙って誤動作せずに止まる。

## 6. M(データモデル)の穴の目安 — 観測した辺のうちモデルが予測しなかったもの

`judge.py` の M は『呼び出し元の関数が、(a) 直接の呼び出し・即値、(b) 参照するデータ番地が所有するバイト列の中の関数ポインタ、(c) ジャンプ表の case 本体、(d) 選択子の case −1/0 の辺、のどれかで呼ぶ』という辺の集合の到達可能性である。`--cov` は観測した (呼び出し元 → 呼び出し先) の辺のうち、dsp の内側のもので『呼び出し元のモデルの辺に無かった』ものを数える。本決定では `tools/judge_edges.py`(judge.py に、その辺を表で出す 10 行を足した写し)で、起動 16 スナップ・Basic/Sampler の再生・決定 0171 の操作・ダイアログ・Save as・Export の入口のすべてを合わせた coverage(222 ファイル)に対して数えた(`tables/unpredicted-edges.tsv`):

| 区分 | 辺の数 | 内容(読み) |
|---|---|---|
| ジャンプ表の case 本体への辺 | 7 | 呼び出し元 f_27ad5f8 から case の入口 7 つ(同じ関数の中のジャンプ表。入口は seed だが関数の始まりではない) |
| 関数ポインタの表を回して呼ぶ辺 | 15 | 呼び出し元 f_37de9b0(CRT の `_initterm` 型。初期化関数の表を順に呼ぶ)から f_37d23b0 ほか 15 関数 |
| データ経由の間接呼び出し | 2 | f_33e97c0 → f_33d6e70(HLE の warp_bilin)、f_3476d30 → f_344fcf0 |
| 観測した辺の総数(dsp 内、全シナリオの和) | 415 | 予測された辺 391(94.2%)、予測されなかった 24(5.8%) |

**読み**: 24 本の呼び出し先はどれも R_M に入っている(別の経路で届くため)ので、**モデルの反証にはならない**(`judge.py` の『MODEL FALSIFIED』は出ない)。しかし M の辺のモデルは完全ではなく、『表を回して呼ぶ型』の呼び出しは、その表の所有範囲の取り方(参照番地から次の参照番地まで)に依存して届き方が決まる。決定 0168 の感度(`extra_refs=1`、`back=512` など)は、この所有範囲を広げたときに M から R_M へ戻る関数の数を出しており、それが §7 の表である。**この感度は観測(coverage)で選んだものではない**ので、どのモデルが『正しい』かは本決定でも決められない。

**検定の力の目安(読み)**: 入口に入った dsp の 355 関数のうち M の外は 0。これを独立な標本とみなすと、『必要な関数がモデルの外に落ちる割合』の 95% 上限は 3/355 ≈ 0.85%(rule of three)。ただし (a) 355 関数は起動と数個の ipps に偏っていて独立ではない、(b) 刈る対象の M 12,929 関数(= S 以外で、モデルの外と判定された関数)のうち、実際に必要になる関数が何本あるかは、所定プロジェクトの実 DSP が走らないと分からない。つまりこの数字は『これらのシナリオで使う関数にはモデルの穴が当たっていない』以上のことを言わない。M のモデルが保持する 6,881 関数(R_M)のうち、1 度でも入口に入ったのは 355(5.2%)で、残りの 94.8%(W 6,546)はこれらのシナリオでは検定されていない。

## 7. 大きさ(S1)への影響 — 修正したリストが要る結果は出ていないので、保守的な代替の推計と、prune 以外の候補の列挙だけ

刈った関数が実行された例が無いので、**S+M の線を引き直す必要は生じていない**。ただし M のモデルは §6 のとおり完全でなく、所定プロジェクトの実 DSP は検定できていない(§9)ので、**保守的な代替**を用意した。`tables/prune-SM2.tsv` = S + (M のうち、データモデル `extra_refs=1`〔参照番地が次の参照番地の区間も所有する〕でも、『未要求の 26 export を根に足す』でも R_M に入らない関数)。16,223 関数(既定の S+M は 18,489 関数)。

| モデル | 刈る関数 | 既定から戻る関数 | 戻る C の量 | 戻る wasm_oz_A | 推計 F2 の wasm | S1(141.146 MB)との差 |
|---|---|---|---|---|---|---|
| 既定 prune-SM.tsv | 18,489 | 0 | 0 | 0 | 136.60 MB(実測 F2 = 136,597,836 B) | −4.55 MB(余裕) |
| 未要求 26 export を根に足す | 18,017 | 472 | 3.9 MB | 1.80 MB | 137.81 MB | −3.34 MB |
| extra_refs=1 | 16,664 | 1,825 | 35.6 MB | 4.59 MB | 139.69 MB | −1.46 MB |
| back=512 | 17,132 | 1,357 | 47.9 MB | 9.70 MB | 143.12 MB | **+1.97 MB(超過)** |
| extra_refs=3 | 15,288 | 3,201 | 64.1 MB | 14.86 MB | 146.59 MB | **+5.44 MB(超過)** |
| **prune-SM2(extra_refs=1 と 26 export の両方)** | 16,223 | 2,266 | 39.2 MB | 6.31 MB | **140.84 MB** | **−0.30 MB(ほぼ余裕なし)** |

**推計の方法(読み。ビルドしていない)**: 戻る関数の `wasm_oz_A`(prune1 が -Oz の単体で測った関数ごとの bytes)の合計に 0.672 を掛けた(S → S+M で実測した wasm-opt 後の減少 34,135,973 B ÷ M だけの wasm_oz_A 合計 50,782,094 B)。F2 の hot-units 表(`ship-hot-units-F2.tsv`)も M 側の関数の一部を含むので、実測は誤差がある。実測するなら `SHIP_PRESET=F2x PRUNE_LIST=<abs>/prune-SM2.tsv` を fin1 の道具で 1 本ビルドする(本決定では**ビルドしていない**: 結果が出ていないリストのために 50 分のビルドを足すより、PM の判断を先に求めるため)。

**prune 以外の大きさの削減候補(列挙だけ。測っていない。読み)** — S+M をそのまま出荷しなくても S1 に届く可能性があるもの:

| 候補 | 想定される効き(読み) | 制約 |
|---|---|---|
| F2xn の wasm は code セクションが 98.4%(134,447,631 B)、data 1.3%(1.8 MB)、element 0.19%、name セクションなし(測定: `tables/wasm-sections-F2xn.txt`) | data/element/export の削減は合計でも 2 MB 未満。**大きさは実質『コードの量』だけ** | 構造上、効くのはコード |
| FreeType を上流の wasm に置換(決定 0159 の第 5 の候補。QFC が読む約 20 フィールドの shadow struct が要る) | 決定 0159 の読みで lift 版 2.9 MB → 上流 0.37 MB、約 −2.5 MB | 『最後にする』と決定 0159。QFC 経由なので描画の同一性の再検定が要る |
| wasm-opt の追加パス(`--converge`、`--merge-similar-functions`、`--dae-optimizing`、`--inlining-optimizing` の閾値、`--directize`)と `-Oz` の `--shrink-level` | 一般に 0.5〜2%(0.7〜2.7 MB)。lift した同形の関数が多いので `--merge-similar-functions` が効く可能性 | wasm-opt の時間が伸びる。挙動の同一性(82 行)の再検定 |
| 『always-inline の単位』(F2 は hot-units 15 単位)を減らす | 単位ごとに -Oz へ戻す量は `attrib-units.tsv` | P2(UI 120 fps)と引き換え。速度の測定が要る |
| リンク時最適化(`-flto`) | 単位をまたぐ同形関数の統合・不要関数の除去。ただし wasm-opt は既に重複関数除去をする | 400 MB の IR のリンクメモリ(GB 級)と時間 |
| FLEngine や他モジュールの未実行の関数を除く | 大きい | **忠実性の観点で不可**(PM の方針) |
| 配布物の分割(遅延ロード) | S1 の『raw wasm』の定義では減らない | S1 の定義 |

## 8. Fruity Wrapper が dsp_ippv2 から引く名前は 420 に収まるか(PM の指示、wrp1 の実走ログの読み取り)

PM 連絡(決定 0183 の新情報): Fruity Wrapper の DllMain は `LoadLibraryW("dsp_ippv2_x64.dll")` を呼び、`ippMalloc` 等を GetProcAddress で引く。prune1 の静的確認は『134 の Fruity プラグインと FLEngine が同じ 420 名のリテラル集合』であり、**実際に GetProcAddress で引かれる名前**の確認が必要。

**確認(実測。`lift/out-wrp1/runs/r*/stderr.txt・logs/traces/*.err` を読み取りで解析。`tools/wrp1_names.py`、結果 `tables/wrp1-getprocaddress-dsp.txt`)**: 解析したログ 27 本(`runs/r*/stderr.txt` と `logs/traces/*.err`)で、dsp(hmod = 0x2000000)に GetProcAddress した名前は、FLEngine・quickfontcache・iltools がそれぞれ **420 名(各 5499 回)**、**Fruity Wrapper(base 0x7000000、呼び出し元 ret が 0x7……)が 420 名(5076 回、12 本のトレース)**で、**すべて 420 名の内側、420 の外 []、不在 0**(`tables/wrp1-getprocaddress-dsp.txt`)。ラッパーは `LoadLibraryW("dsp_ippv2_x64.dll")`(ret 0x70497a0)を呼ぶ。**追加の根は 0**(prune-S/SM の根の集合 = 420 名は、ラッパーが引く名前の集合を覆う)。(最初に `runs/r*/stderr.txt` だけを読んでラッパーの検索を 0 件としたのは、ラッパーのトレースが `logs/traces/` にあることを知らなかったための誤読で、訂正した。)

**読み**: 『名前の集合』は 420 に収まり、追加の根は要らない。ただし、ラッパーが動いて dsp の関数を**実際に呼ぶ**段階(CreatePlugInstance 以降。wrp1 は GetPluginFactory まで届いている)の dsp 関数の**入口集合**は本決定では取れていない(wrp1 の走行には入口の coverage が無く、本決定の D3/PSM3 にはラッパーが入っていない)。wrp1 の走行に `FLE_COV_FILE` を足すか、D3 にラッパーを足して `judge.sh` に渡せば、M の外の入口が出ないかを同じ判定にかけられる(§9)。

## 9. 確認していないもの(『問題なし』とは書かない)

**S+M の安全性の判定に直接かかわるもの**

0. **書き出し(Export)の DSP は 1 度も走っていない**(先頭に置く。下の項目 3 の詳細: File → Export → Wave file... は IFileSaveDialog のモデルが無く何も開かない)。
1. **所定プロジェクト(serina1-serum2.flp)の実 DSP**: 70 チャンネル中、Fruity Wrapper 経由の Fruity プラグイン・Serum 2・エフェクト、書き出し(Export)、MP3/OGG/WAV のサンプルの復号と再サンプリング。FLP の読み込みは最初のプラグイン(チャンネル 1 の Fruity Wrapper の DLL)で止まり(決定 0178)、本決定では dsp の関数の入口がそこで増えなかった(起動と同じ 335 関数)。仮定(`FLE_LOADLIB_MISSING`)の探索でも次の壁(msacm32!acmDriverEnum〔0x6708e0。MP3 の復号の入口。決定 0178 は ACM の行を足すと ilvideo〔FFmpeg〕まで進むと報告〕)で止まり、サンプルの復号には届かない。**M のモデルを最も強く検定するはずの DSP 経路が、まだ 1 本も走っていない。**
2. **Fruity プラグイン(134)・Serum 2 の VST3 がこの dsp から別の経路を引く場合**: §8 のとおり、ラッパーが GetProcAddress で引く名前は 420 に収まる(追加の根 0)が、ラッパー経由でプラグインが動いた段階の dsp の**入口集合**は未取得(wrp1 の走行に coverage が無い)。
3. **Export(書き出し)**: File → Export → Wave file...(Ctrl+R)は、エンジンが `SHGetKnownFolderPath(Documents)` の後に `CoCreateInstance(CLSID_FileSaveDialog {C0B4E2F3-BA21-4773-8DBA-335EC946EB8B}, IID_IFileSaveDialog {84BCCD23-5FDE-4CDB-AEA4-AF64B83D78AB})` を要求する所まで進むが、モデル(決定 0178 は IFileOpenDialog だけ)が無く何も開かない(窓も停止も無し)。**書き出し(レンダリング)の DSP は 1 度も走っていない**。メニューの『Wave file...』の項目を座標で押す版(座標は推定)も何も開かなかった。
4. **モデルの穴**: §6 の 24 本は予測外の辺で、結果は R_M に収まったが、モデルが完全だという証拠ではない。保守的な代替(`prune-SM2`)の実測は未実施。
5. **実機 Windows との照合**: 0 件(選択子の値 0・−1 は CPUID を固定した結果で、実機の IPP が実機でどの case を選ぶかは見ていない。ブラウザの wasm 上では CPUID が固定なので、利用者の機械の CPU には依存しない)。

**シナリオの網羅の範囲で確認していない操作**

- マウスホイール、ドラッグの途中経過の多段(つまみ/フェーダーは 2〜3 点だけ)、ダブルクリック以外の連打、キーボードの修飾キーの組み合わせの大半(決定 0171 の 42 キー以外)。
- ミキサーの FX スロットからのプラグインの追加(Fruity プラグインの DLL の読み込み。壁)、ピアノロール上の音の入力と再生、オートメーション、オーディオ録音(マイク)、MIDI 入力、サンプルのブラウザでのプレビュー。
- 『別名で保存』の実際の保存(Save as のダイアログ〔TNewProjForm〕は開き、Save を押すと Trial は保存せず『Buy FL Studio』〔TInAppShopForm〕が開く。保存の DSP・書き込みは走らない)、プロジェクトの再読み込み(File → Revert)、テンポ以外の再生パラメータ(スウィング、メトロノーム、カウントイン)の再生中の変更。
- 書き出しの音の中身、再生音の正しさ(音は出ているが、実機の音との照合は 0 件)。
- 層 3(dlg1 の行)を入れた出荷用の F2x(ダイアログ経由の File → Open を含む)の wasm: 出荷物 F2xn は層 2 まで(§4)。
- ブラウザ(Chrome Worker)での実行: 本決定は native -O0 と node の wasm だけ(PM の指示)。

**次にやること**: (a) wrp1 がラッパーを動かしたら、その coverage を `judge.sh` に渡す(`MODEL FALSIFIED` が出なければ M の外の入口は 0)。(b) 層 3 の F2x を作ったら `node-bundle-run.mjs` で ops 一式を再実行する。(c) 保守的な代替を採る場合は 1 本ビルドして大きさを測る。(d) Export とプロジェクトの保存が走るようになったら、それを足す。

## 10. 出典なしの選択(名前つき。代替案と撤回条件は `tables/s-choices.tsv`)

`S-prune2-probe-prefix`(プローブの開始状態 = 起動 + ウィザードを閉じた後)、`S-prune2-coords`(ミキサー/チャンネルラック/テンポのドラッグの座標)、`S-prune2-play-lengths`(native は 5 s + 4 s、node の F2xn は 370 仮想秒)、`S-prune2-cmp-extend`(参照プロジェクトで FLE_CMP_EXTEND=1)、`S-prune2-loadlib-missing`(探索のみ)、`S-prune2-judge-plain`(native は素の fe-lift で、出荷の dead-flags/outline/HLE なし)、`S-prune2-node-glue`(ブラウザ glue を node で worker 扱いにする複製)、`S-prune2-f2xn-no-layer3`(出荷物 F2xn は層 2 まで)、`S-prune2-sm2`(保守的な代替リストの定義)、`S-prune2-size-ratio`(推計の係数 0.672)、`S-prune2-max-over-files`(シナリオの入口数は coverage ファイルの最大)。計 11 件。

## 11. 規則違反と手順の誤りの自己申告

- 存在しない `bash` ツールを 2 回呼んだ(無害)。`sleep` を 1 回、末尾 `&` を 1 回(いずれも shell に拒否された)。
- 作業ディレクトリが一時 8.4 GB(上限 5 GiB)を超えた(D3・PSM3・PX3 の生成 C とオブジェクト)。生成 C を消し、同一オブジェクトをハードリンクにして約 3.8 GB に整理した。終了時に 500 MB 以内へ整理した(§12)。
- 同時の重い処理が一時 5 本(native 2〜3 + node 2)になった(load 9〜13)。PM の指示を受けて以後は native 2 本以内にした。
- follow モードの FIFO に、エンジンが FIFO を開く前(起動 3.5 分の間)に送って 1 回分を失い、再送した(F1 セッションが約 6 分遅れた)。
- 初期の `split-probes.py` は同名のプローブを上書きした(コンテンツの SHA で、失われた固有のスクリプトが 0 であることを確認して直した)。
- `importlog` は native の fork プローブ専用のコマンドで、wasm の G3 ハーネスは『unknown script command』で止まる(5 個のプローブが最初は実行されなかった。除いて再実行した)。
- Basic.flp → Sampler.flp を同じセッションで開く最初のシナリオは、2 つ目で『保存しますか』の Confirm が出て Sampler が開かなかった(native と F2xn の両方)。Sampler は別のプローブにした。結果表の『flp_bs』はこのため『Basic + Confirm〔Buy FL Studio〕』である。
- PSM3 の 3 本目の follow セッション(F4: 書き出しと操作 133 の重複)を起動し、F2 が同じ操作を先にこなしていると分かったので、結果を取る前に自分で kill した(その run は結果に数えない)。
- `gdb` の最初の 2 回の記録は R の読み出しに失敗した(別のオブジェクトの宣言を拾って 'optimized out')。正しい読み出し(ELF のアドレスと PIE の基底)に直して取り直した。

## 12. 再現と置き場

- 道具と使い方: `tools/prune2/README.txt`(`lift/out-git/prune2-results/tools/`)。ビルド `gen-native.sh D3|PSM3|PX3 [PRUNE_LIST]` → `link-fp.sh`(先に `cov-per-probe.patch` を当てる)→ `run-g3.sh` / `serve-fp.sh`+`send-fp.sh` → `final_tables.sh`(表と judge)。F2xn は `node-bundle-run.mjs` + `node-probes.sh`。
- 表: `tables/scenario-hits.tsv`(シナリオ表)、`dsp-entered-union.tsv`(入口に入った dsp 355 関数)、`unpredicted-edges.tsv`、`prune-SM2.tsv`、`sensitivity-size-estimate.tsv`、`node-F2xn-probes.tsv`、`cpu-sites-*.tsv`、`gdb-cpu-api-summary.txt`、`wrp1-getprocaddress-dsp.txt`、`wasm-sections-F2xn.txt`、`s-choices.tsv`、使ったスクリプト(`*.g3`)と VFS の種。
- ログ: `logs/`(ビルド・各走行の load.txt・judge・node の summary)。
- 作業ディレクトリ `lift/out-prune2/`: 終了時に 500 MB 以内へ整理(生成 C・オブジェクト・バイナリ・展開した参照プロジェクトを削除。`gen-native.sh` で再生成できる)。
- テスト基準への影響: **なし**(新規ファイルのみ。`cargo test` の対象を触っていない)。**確認**: 本決定のために HEAD の `git archive` から作った `fe-lift`(`cargo build --release --offline -p fe-lift -p fe-pe`、素のオプション)が FLEngine の既定の生成 C を **215,811,678 B・sha256 `cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`** で出すことを測った(基準と一致。`logs/felift-default.sha`)。全体テストは**走らせていない**(コードを変えていないため。走らせるなら基準は 162 results / 2,192 passed / 0 failed)。


## PM の判断と取り込み(2026-11-03、追記)

取り込み: `prune2.diff`(新規ファイルのみ 104 本: `docs/investigations/prune2/` と `tools/prune2/`。`-` 行 0、`patch -p1 --dry-run` は HEAD 4f23c01 に offset・fuzz・reject なしで当たる)。fe-lift・runtime・harness・app には触れていない(既存の 2 ファイルへの小さな opt-in の追加 `cov-per-probe.patch` は**適用していない**: `fruity_g3.c` が `trace_native.c` の新しい関数を参照する形で、`trace_native.c` を含まないビルドがあればリンクが落ちる。必要になったら弱い参照にして別の担当で入れる)。コード変更が無いので全体テストは再実行していない(基準 169 results / 2,199 passed / 0 failed のまま)。PM は結果を再実行していない。

PM の判断:

1. **S は安全(定義上)。S+M は『条件つき』のまま**。確認した範囲(起動・Basic/Sampler の再生と操作・206 プローブ・空プロジェクト 370 仮想秒・ダイアログ経由の FLP・参照プロジェクトの Fruity Wrapper の壁まで)で刈った関数のエントリは S も S+M も 0、PSM3 の abort 0、出荷物 F2xn の node 実行 206 走行で abort 0、負の対照 PX3 で検出が働いた。しかし検定できた dsp は閉包 25,370 関数のうち 355(1.4%)で、**所定プロジェクトの実 DSP(Fruity プラグイン、Serum 2、エフェクト、書き出し、サンプルの復号)は 1 本も走っていない**。これらが走る経路(wrp1 の Fruity Wrapper→Serum 2、書き出し用の IFileSaveDialog、MP3 の ACM)が動いた時点で、prune2 の道具(`tools/prune2/`、PSM3/D3 の手順)で再判定する。それまで、出荷する構成は **S(F1 系: 2.43 倍で S1 は未達)**。F2 系(S+M、1.936 倍)は、abort が『名前つきの安全側の失敗』であることを理解した上での**候補**に留め、既定の入口にしない。
2. S1 の余裕: F2 の推計 140.84 MB(prune-SM2)は余裕 0.3 MB、`extra_refs=1` だけなら 139.69 MB(余裕 1.46 MB、推計=未ビルド)。S+M を出荷する場合は、保守的な代替の実ビルドと S1 の実測が要る。prune 以外の削減候補(wasm-opt の追加パス 0.5〜2%、FreeType の上流置換 約 −2.5 MB、always-inline 単位の縮小〔P2 と引き換え〕)は測っていない。
3. **leaf 7** は決定どおり(SDM の規則)で不整合ではない。担当の gdb 実験(leaf 7 = 0 で起動 16 スナップ 101 行が完全一致)は、再生・操作の範囲で未実験。runtime は変えない。
4. **ラッパーが引く dsp の名前**は 420 名の内側(wrp1 の実走ログ 27 本)。ラッパー経由でプラグインが動いた段階の dsp の入口集合は未取得: wrp1 の走行が coverage を出すようになったら(wrp1 の取り込み後)追加の周回をする。
5. 確認していない操作の先頭は、**書き出し(Export)の DSP が 1 度も走っていない**こと(IFileSaveDialog のモデルが無く何も開かない)。IFileSaveDialog のモデル化は G8/G10 の作業に入れる。
