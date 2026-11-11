# 決定 0243: オプトイン/オプトアウトの処理を持たない(opt1、仮の決定)

状態: **仮**(PM が取り込んでから確定)。作成 2026-11-05。基準 HEAD e52d024f(層 b → d → `ship6.diff` → mt4 の絞り込み → c の順に、この HEAD の `git archive` へ fuzz・offset・reject なしで当たる〔`ship6.diff` の `ci/test-hybrid.sh` の 1 つ目の hunk だけ、mt4 がコメントを書き換えたため fuzz 2〕)。

## 結論

- (1) 依頼者の方針(2026-11-05『今後、一切のオプトイン/オプトアウト処理を禁止します。デフォルトで全てのオプションを有効とし、既存ソースも修正してください』、`docs/policies/no-options.md`)に従い、ページ・CI・Worker(層 a、取り込み済み)、runtime の C とハーネス(層 b)、fe-lift / fe-wasmgen の旗(層 d)、ビルドスクリプトとプリセット(層 c)のオプションを、有効側を唯一の経路にして畳んだ。製品の構成は 1 つ: 8 モジュール(FLEngine と dsp_ippv2・Blend2D・QuickFontCache・FreeType・ILTools・oggio・Elastique)、直接 wasm の混成(`hyb-U-*.txt` の 3,879 関数)、上流 Blend2D、wasm-opt -Oz、レジスタファイルはスレッドローカルのブロック、1 つの共有メモリ、ファイルシステムの無い glue。プリセット名(`F2xH7` ほか)、`SHIP_PRESET`、`VARIANT`、`NO_FS`、`HYBRID_SETS`、`WASMGEN_MAX_FN_COST`(定数 50,000,000 になった)など約 70 のビルド変数、環境変数、`-D`、旗はない。
- (2) **実エンジンでの確認(Ω = 全部有効。CI の段階スクリプトで作った 1 つの bundle を、パラメータ無しの既定の入口で、隔離 Chrome 156 と Firefox 158 に載せた。この機械: 16 コア、実 GPU なし、他担当と共有で load 6〜40。依頼者の機械ではない)**:
  - **bundle**(CI の段階スクリプト、22 分): wasm 141,795,291 B(S1 の 8 モジュールの線 154,885,888 B より −13,090,597 B、6 モジュールの線 141,145,856 B より +649,435 B)、br 20,080,510 B。**この bundle は、HEAD e52d024f の `git archive` に b → d → ship6.diff → 絞り込み → c を当てた木そのもの(`diff -rq` で差なし)の CI 段階スクリプトのビルド**。1 つ前のビルド(HEAD cbf9d1e5 基準)は 141,795,274 B で、生成 C の sha256 は 9 本とも同一(差は alpha2b の `fle_raster.c`)。同じ HEAD の旧 F2xH7 は 133,868,994 B。manifest に `FLE_CALL_FRUITY` `FLE_FRUITY_NO_IDLE_STOP` は無い。
  - **Chrome 156 の `perf-real-engine`: 15/15**(最終 bundle、first idle 6.4 s、load 6 の 1 回。Firefox 158 の最終 bundle は 6.73 s、CPU 秒 15.3、load 7.5 の 1 回)。first idle は **交互 8 組**(静かな時間帯、load 4〜13、Ω と旧 F2xH7〔product 形の CI ビルド HEAD 33349a3d〕を交互)で、Chrome の中央値 Ω 5.86 s / 旧 5.95 s、ブラウザ全体の CPU 秒 15.12 / 15.16。Firefox 158 は Ω 5.56 s / 旧 5.48 s(Ω−旧 +0.23 s、Ω の方が長いのは 6/8 組)、CPU 秒 11.86 / 11.16。**P1(10 s)は 32 回すべて満たした(最大 8.95 s)。Firefox の差はこの 8 組ではばらつき〔旧の 8.83 s の外れ値 1 組〕と区別できない**。
  - **node の行**(wasm のテスト用ビルド): 16 スナップ 84 行と参照プロジェクト 55 行は旧 F2xH7 と**違う。原因は `FLE_READFILE_EOF_TRUE`(決定 0206、ReadFile の EOF が TRUE)ただ 1 つ**(6 モジュール版でその 1 つだけを元に戻すと、84 行・55 行が同じ HEAD の旧ビルドとバイト一致)。8 モジュール版の ref だけ、さらに ship6 の 2 モジュールによる行が 7 本違う。基準の表は新しい値に置き換えた(下の『行の変化』)。
  - **最終 bundle の node の行(最終の木のテスト用ビルド)**: 84 行・55 行は 1 つ前の Ω の行と**差 0**(置き換えた基準の表と一致)。ship6 の『参照プロジェクト 87 行』は `ref1-perf-wasm.g3`(開く → Enter → アイドル 30 s → Space → 再生 30 s → Space、VFS seed は `vfs-seed-nested.tsv`)が出す **`g3-` で始まる行すべて**(g3-snap 10、g3-progress 10、g3-surface 23、g3-win 23、g3-wintable 10、g3-timers-so-far 10、g3-timers 1 = 87)で、最終 Ω で 87 行(`tables/g3-lines-omega-refw-87.txt`)。終わり方は他の 2 本と同じ named stop(SIGABRT、『first non-background pixel on a drawing surface』、UpdateLayeredWindowIndirect、lid 8)。lid 139 の『Problems loading the project』は 691x343 で、55 行の run と同じ SHA。1 つ前の Ω との 87 行の比較は、作業用の木を片付けたため取っていない(84 行・55 行が同一なので差は無いと読むが未測定)。
  - **生成 C の sha256**: `flengine.c` と `m1`〜`m5` は同じ HEAD の旧パイプラインの出力とバイト同一。`m6`・`m7` は旧に無いモジュール(oggio・Elastique)、`common.c` は 6 → 8 モジュールのディスパッチ表。
  - テスト: `cargo test --workspace`(git archive 全体 + b + d + ship6 + 絞り込み + c の木)**199 結果 / 2,346 通過 / 失敗 0 / 無視 1**。`ci/test-needs.sh`(128 通り)・`test-hybrid.sh`・`test-lifecycle.sh`・`test-real-deploy.py`・`test-wrangler-config.sh` 通過。mutation の worker 層・e2e 層 41 件は 41/41 検出(e2e の 2 件は実エンジンの bundle を渡して)。11 場面: ci1 の駆動ツール(`docs/investigations/ci1/verify-81f6638c/tools/sdrive-a1.mjs`、`scn-client.json` `scn-plus.json` `scn-alpha1.json`)を最終 bundle の既定の入口で走らせた。Chrome 156: client の 11 場面のうち d1〜d7・d9〜d11 の 10 場面が停止なしで期待どおり(d7 は『engine ended with code 0』が true)、d8(メニュー 100 回)は時間が足りず未実行。Firefox 158(場面ごとに新しいプロセス): 10 場面が同様(d1 は 1 回目に Files 窓が出ず openProject で失敗、再試行で ok)。`+` > More plugins…(`scn-plus`)は Chrome・Firefox とも停止なし。Add メニューのフェード・F8 ピッカー・ピッカーの費用(`scn-alpha1`)は両ブラウザで停止なしに最後まで走ったが、フェードの画素の期待値の判定は ci1 の表との照合まで行っていない。**以前の自前の駆動ツール(ship5 の sdrive3)で d1・d3・d5・d7 が失敗したのは、現在のページに座標が合っていなかったためで、旧 F2xH7 でも同じ失敗だった(Ω の退行ではない)**
  - **走らせていない物**: a4(942/942 ほか)、app の typecheck・vitest・`test:worker`・`test:e2e`(層 b・c・d は app のコードに触らない。c は `app/scripts/build-engine-bundle.sh` だけ)、10 本の parity セッション、`ref3/tables/g3-lines-fonts-84.txt`(fonts 差分つきの別構成)、再生 CPU/テンポ(mt3 の道具)、GitHub Actions 上の CI。
- (3) 畳んで変わった物は 1 件ずつ説明した(下の『効果の表』)。変わらなかった物: FLEngine と m1〜m5 の生成 C(sha256 同一)。
- (4) 残した物(逸脱)と、まだ畳んでいない物は『残り』に書いた。

## 効果の表(測定した物だけ)

| 項目 | 前 | 後 | 理由 |
|---|---|---|---|
| FLEngine を `--pdata` だけで lift した既定の生成 C | `0130eeee…c71c6`(215,805,703 B、関数 59,380) | `34a376db…78062a`(215,740,414 B、関数 59,391) | `--dead-flags` と `--outline-shared` が旗でなくなり既定で効く。outline の 11 関数が増え、C は 65,289 B 減る(`tables/felift-default-c-sha256.txt`)。mt4(決定 0246)の前の値は `cd9107ce…` → `5b587a50…` で、差は同じ |
| 出荷の構成の生成 C(`flengine.c`・`m1`〜`m5`) | 旧パイプラインの出力 | **同一**(6 本の sha256 が一致) | リフターが旧ラッパの分岐(swap なら無し、SEH なら `--dead-flags` だけ、他は両方)を 1 か所の条件で持つ。出荷の `flengine.c` は `e72edd4a…` で旧→新で不変 |
| 直接 wasm のコード(hyb-U-fle の 2,772 関数、wasm-opt 前、HEAD a49391d9 で測定) | 5,546,233 B(`--no-elide`) | 5,255,733 B(−5.2%) | `--no-elide` の字義どおりの有効側(elision が唯一)。`ok` の関数数は同じ |
| wasm 全体 | 133,868,994 B(旧 F2xH7、同じ HEAD) | 141,795,274 B(+7,926,280 B) | 8 モジュール(oggio・Elastique)、hybrid U の 3,813 関数〔旧 L5s は 12,853〕、TLS_REGS・MT2_SHARED。S1 の 8 モジュールの線より −13,090,614 B。br は 30,232,712 → 20,077,388 B |
| first idle(Chrome、交互 8 組) | 5.95 s(中央値) | 5.86 s | 差なし(Ω−旧 −0.20 s、CPU 秒 15.16 → 15.12) |
| first idle(Firefox、交互 8 組) | 5.48 s | 5.56 s | +0.23 s(中央値の差)、CPU 秒 11.16 → 11.86。8 組ではばらつきと区別できない |
| 16 スナップ(84 行) | 旧 F2xH7 の行 | 36 行が変わる(下) | `FLE_READFILE_EOF_TRUE` |
| 参照プロジェクト(55 行) | 旧 F2xH7 の行 | 25 行が変わる(下) | 18 行は `FLE_READFILE_EOF_TRUE`、7 行は ship6 の 2 モジュール |

## 行の変化(原因ごと)

基準 = 同じ HEAD cbf9d1e5 の旧 F2xH7(NO_FS=0 の node 用ビルド)。mt3 の保存行は 84 行では一致、55 行では 18 行違った(その後の HEAD の変更)ので、基準は取り直した。切り分けは 6 モジュール版(opt1 の全部有効、oggio・Elastique なし)の node 用ビルドで、ReadFile の EOF だけを元に戻した版(`fle_vfs.c` の 4 行)と比べた。**元に戻した版は 84 行・55 行とも基準とバイト一致**。

| 行 | 変わった物 | 理由 |
|---|---|---|
| g3-snap(84 行の 16 組、55 行の 3 組) | clock_ms が +1 ms(例: wiz0 で 3601763 → 3601764) | FL は起動中に EOF まで読む。ReadFile が EOF で FALSE+ERROR_HANDLE_EOF の代わりに TRUE+0 バイトを返す(Learn『Testing for the End of a File』の契約)ので、エラー側の分岐を通らない。ゲストの実行量が 2 progress 分(11,096,162 → 11,096,160)ずれ、仮想時計を進める slice(10,000 ticks = 1 ms)の境界が 1 つ後ろへ動く |
| g3-progress(16 組 / 10 組) | progress が −2 から +6、slices +1、deferred −1、switches +1、ticks +10,000 | 上と同じ。slice の境界の移動の結果(wiz0 から全スナップで同じ向き) |
| g3-surface lid 14(84 行の 4 組: e1300・e2300・e3300・end、55 行の 6 組) | メイン窓(1930x1092)の **13 画素**(x=1042、y=12〜24 の 1 画素幅の縦線)の明るさだけ。ほかの画素は全て同じ。wiz0 は 0 画素 | ツールバーの脈動する表示(0x5e→0x67→0x70 の灰色)の位相。仮想時計が 1 ms ずれるので、同じ snapshot の時刻で別の位相になる(基準の e1300 = 5e666a、Ω の e1300 = 70787d)。内容(FL が描く物)は変わらない |
| g3-surface lid 139 ×2、g3-wintable ×2、open 後の g3-progress(55 行、8 モジュールだけ) | 『Problems loading the project』窓が 691x423 → 691x343(−80 px)。progress は 41,663,620 → 92,526,541 | 基準は『8 plugin(s) could not be opened / **2 sample(s) required by this project could not be located.** / This is not a serious error, the project will continue to load normally / and the plugin(s) will be ignored.』、Ω は『8 plugin(s) could not be opened / and the plugin(s) will be ignored.』。**sample の 2 行と注意の 1 行が消える**: oggio(サンプルのデコーダ)が実モジュールとして読み込めるので、プロジェクトの 2 つのサンプルが見つかる(窓の画像: `tables/problems-window-*.png`)。open 後の progress が増えるのは oggio が実際に動くため。6 モジュール版は基準と同一 |

基準の表は次の 4 ファイルを新しい値に置き換えた(旧版は残さない): `docs/investigations/load1/tables/g3-lines-base-84.txt`、`docs/investigations/ref3/tables/g3-lines-base-84.txt`、`docs/investigations/mt3/tables/g3-lines/F2xH7-cmp-v0v1v2.txt`(以上 84 行)、`docs/investigations/mt3/tables/g3-lines/F2xH7-ref-v0v1v2.txt`(55 行)。これらを読むテスト・スクリプトは無い(`git grep` で確認)。置き換えの値は 8 モジュールの最終 bundle の node 用ビルドの行(`opt1-c.diff` に含まれる)。

## テスト用の計装マクロ(製品は定義しない)

MX の計測用マクロは層 e で `FLE_MX_STATS`(命令の種類ごとの命中/外れの数え)と `FLE_MX_NOFAST`(速い経路を持たない参照の実装)だけになった。fp1(wall_fr18)と vst/mt1 の計測の道具が使う計装で、製品のビルドは定義しない(no-options.md §2.1 の 6)。層 e より前に挙げていた `FLE_MX_TRACE` `FLE_MX_TRACE_FILE` `FLE_MX_NORC` `FLE_MX_NOVEC` `FLE_MXR_STATIC` `FLE_MXR_DIRECT` `FLE_HLE_TEST_FALLBACK` とそのフック(空の `FLE_MXT` `FLE_MXVT2`)は削除した。`FLE_TEST_ENV` は `-D` でなく、テストビルドだけがリンクする weak 関数の定義に置き換えた(下の『残り』)。

## 残り

- **FLE_TEST_ENV**(層 e で置き換えた): `-D` は無い。製品の木の `lift/runtime/fle_test_env.h` は、wasm では `FLE_TEST_GETENV` が null の定数(診断の環境変数を読まない。層 c までと同じ前処理)、native では `getenv`。wasm のテストビルドは**木の写し**の中で、このヘッダ 1 つを `lift/harness/parity/fle_test_env_wasm.h`(getenv)に差し替え、そのヘッダを include する単位(8 つ)のオブジェクトを作り直す(`make-test-tree.sh`)。製品の木とそのビルドには旗も分岐も無く、テストビルドの有無で変わらない。(途中で試した『weak 関数 `fle_test_getenv`』と『製品が NULL を返す強い定義』は、どちらも wasm にテストの入口のコード・import・関数を残して製品の出力を変えたので採らなかった: 下の層 e の節。)
- **MX の計装マクロ**は `FLE_MX_STATS` と `FLE_MX_NOFAST` だけが残る(層 e: NORC/NOVEC/MXR_STATIC/MX_TRACE/MXR_DIRECT と `FLE_HLE_TEST_FALLBACK` の無効側、FLE_MXT/FLE_MXVT2 の空の定義と呼び出し、`FLE_MXC_INLINE/OUTLINE` の上書き口は削除)。wall_fr18(`harness/fp1`)の差分テストがこの 2 つで組むため。設計案: 計装つきのヘッダの写しをテスト専用に置いて wall_fr18 を書き換える(PM: 優先度低)。`--opaque-stub` / `--unbal-stub` と fe-wasmgen の固定アドレス経路(`--tls-ctx` なし)は層 e で削除した。fe-wasmgen のテストの入口(`--export-all` `--export` `--roots` `--select-file` `--rel-sites` `--adapt-all` `--census` `--mx-force-slow` と `Config` の `names` / `export_funcs`)は残る。
- **getenv で残した物**(部品間のパラメータ): `FLE_AUDIO_LEAD_MS`、`FLE_DWM_HZ`、`FLE_DISPLAYSETTINGS_FREQUENCY`、`FLE_MONITOR_W/H`(`fle_screen.h`)、`FLE_SCREEN_REMAXIMIZE`(ページが渡す)、manifest の env(`FLE_VFS_SEED` `FLE_REG_SEED` `FLE_AUTH_CLAIMS` `FLE_FRUITY_MAX_CALLS` `FLE_FRUITY_SECONDS`: データの場所と上限)。
- **mt4 の絞り込みの補い**: `mt4-atomic-functions.tsv` は HEAD の 6 モジュールの lift から作られていて oggio・Elastique を含まない。`mt4-atomic-functions-m67.tsv`(60 関数、新規)を足し、`hyb-U-oggio.txt` から 0xc071ee0 を外した(hyb-U は 3,879 → 3,814 → 3,813)。`list-atomic-functions.py` と `filter-hybrid-list.py` の冒頭に 2 つの表の役割を書き、hybn0 の『listed が emit されない』検査は fe-wasmgen の status(『atomic read-modify-write』など)と直し方を出すようにした。
- **古い道具**: 層 e で、変数も対象も無いスクリプト(`harness/{b2d1/mutate,draw1/{bb,nodelink,hlecheck},pick1/rebuild-n1b,two/build_two_wasm}.sh`、`web/{fp1,perf2}` の旧 wasm ビルド、`web/build/`、`web/jspi/tools` の旧ビルド 9 本、`rz1/`、`tools/prune{,2}/`)を削除した。`docs/investigations/*/tools` の履歴スクリプトは触らない(`docs/investigations/HISTORY-NOTE.md`)。
- **未確認**: R2 の 180 日の寿命の段(`stage-deploy.sh lifecycle`)は、キャッシュ用の認証情報がバケットの設定を読み書きできるか未検証。できなければ main への push のこの段で job が止まる(Worker の更新の前なので公開中の版は変わらない)。
- **再生 CPU/テンポ**(mt3 の道具)は走らせていない。push 後の確認の使い方: ページを `vite preview`(`FL_ASSETS_DIR` に最終 bundle と `proj/`)で出し、`node lift/out-mt2/tools3/sdrive-ref3.mjs --browser chrome --url http://127.0.0.1:PORT/ --proj-dir lift/out-proj1/x --out DIR --scenarios scn-ref3-tempo.json --viewport 2176x1224 --app app`、続いて `tempo-fit.py` `pcm-repeat.py` `summ.py`(ラッパ: `tools/tempo_opt1.sh BROWSER TAG PORT`)。
- 10 本の parity セッション(`F2xH7-parity-ui-*`)と fonts 差分つきの基準(`ref3/tables/g3-lines-fonts-84.txt`)は新しい値に取り直していない(ReadFile の EOF の影響を受けるはず)。

## PM の決定と取り込み(層 b・c・d、2026-11-05)

取り込んだ基準: HEAD e52d024f に、opt1 の `opt1-b.diff`(runtime の C とハーネス、102 ファイル)→ `opt1-d.diff`(fe-lift / fe-wasmgen の旗、31 ファイル)→ ship6 の `ship6.diff`(データとテスト、12 ファイル。`ci/test-hybrid.sh` の hunk #1 は fuzz 2 で当たり、.orig は削除)→ mt4 の除外フィルタ(`filter-hybrid-list.py mt4-atomic-functions.tsv hyb-U-*.txt` で hyb-U から 65 関数を除外、3,879 → 3,813 関数)→ `opt1-c.diff`(ビルドスクリプト・ci・deploy.yml・R2 perf/ の lifecycle の step・8 モジュールの配線、70 ファイル)を、この順に PM のワークツリーへ `patch -p1`(reject なし)。opt1 の複製とは主要ディレクトリの `diff -rq` で同一を確認した。

PM が測った値(この機械、`lift/out-pmchk-o2/`):
- FLEngine の既定の生成 C(`fe-lift lift … --pdata`)の sha256 `0130eeee…` → **`34a376dbf23a30c62df5ce6d04c07ed9beb5b651dd82d608cf5bcc59af78062a`**(215,805,703 → 215,740,414 B、関数 59,380 → 59,391)。出荷の構成の `flengine.c` と m1〜m5 は旧 lifter と新 lifter でバイト一致(opt1 の測定)。
- `ci/test-hybrid.sh`・`test-needs.sh`・`test-lifecycle.sh`(新)・`test-real-deploy.py`・`test-wrangler-config.sh` すべて rc=0。app: typecheck rc=0、vitest 64 ファイル 1,253 passed + 7 skipped、worker 49/49、エンジン無しの e2e 172/172。
- 全体テスト **199 results / 2,346 passed / 0 failed / 1 ignored**(opt1 の報告と同数)、a4 FRUITY 無効 942/942・895/895・functions 5,310・entries 1,009,737(不変)。
- PM が CI の段スクリプト(`ci/stage-*.sh` 相当の `run-all.sh`、F2xH7 の名前のまま・8 モジュールの唯一の構成、CI_JOBS=3)で、PM の木と同一の複製から bundle を作り直した: `flengine6.opt.wasm` = **141,795,291 B**(opt1 の報告とバイト数が一致)。S1 の 8 モジュールの線 154,885,888 B の下 13,090,597 B。

判断:
1. **層 b・c・d と ship6.diff(hyb-U の 3,813 関数、oggio・Elastique の 2 モジュールを含む 8 モジュール)を 1 回の push で取り込む。** 層 b 単独では TLS_REGS が要する MT2_SHARED・LOWREGS・上流 Blend2D の `-matomics` が無い構成になるため、層 c のビルドスクリプトと同時でなければ噛み合わない(opt1 の指摘)。
2. **`FLE_READFILE_EOF_TRUE` を有効にした影響(Learn: 同期ファイルの EOF は成功で lpNumberOfBytesRead = 0)で、基準の g3 行(84 行・55 行)は変わる**(clock_ms +1 ms、g3-progress の計数、窓 lid 14 の面の 13 画素=ツールバーの脈動の位相)。opt1 は 6 モジュール版でこの 1 つだけを元に戻すと 84/55 行が旧ビルドとバイト一致することを示した。基準の表(`load1`・`ref3`・`mt3` の g3-lines)は opt1-c.diff が新しい値に置き換えた。
3. **8 モジュール版の参照プロジェクトの窓が 691x423 → 691x343 になる**のは、oggio が入ってサンプル 2 件と通知 1 行が出なくなるため(期待された改善、progress 41,663,620 → 92,526,541)。
4. 既定の `fe-lift lift --pdata` の sha256 が `0130eeee…` → `34a376db…` に変わるのは、層 d が `--dead-flags`・`--outline-shared` を旗でなく常時の変換にしたため(FLEngine の関数 59,380 → 59,391、C 215,805,703 → 215,740,414 B)。出荷の構成の `flengine.c` と m1〜m5 は旧 lifter と新 lifter でバイト一致(opt1 の測定)なので、出荷の生成 C は変わらない。以後の基準は `34a376db…`。
5. **逸脱として受け入れて記録**(opt1 の報告): `FLE_TEST_ENV`(テスト用ビルドが定義する)、MX 計装マクロ(製品は定義しない)、`--opaque-stub`/`--unbal-stub`(不具合のため未使用)、fe-wasmgen の固定アドレス経路(テストが使う)。
6. **取り込み時に PM が実行していないもの**: Chrome の File メニュー 100 回(d8)・フェードの画素判定を最終の Ω bundle で(opt1 も未実施)、最終 bundle での node の 84/55/87 行、R2 `perf/` の 180 日 lifecycle の step の実 CI での冪等な実行。これらは push 後の CI の bundle で ci1 が確認する。
7. 残る課題: `FLE_READFILE_EOF_TRUE` 後の基準の `g3-surface` の更新(alpha1 の alpha3 の再ベース時)、ship6 の「87 行」スクリプトでの最終 Ω の行(opt1 に依頼済み)。

## PM の追補(2026-11-05 16:30): 層 c の push で CI の link が失敗した件(実行 41982180403、e840e4d4)

- 事実(CI のログ): build ジョブの `run link` が `hybn0` を通過(8 モジュールの 3,813 関数すべて listed = emitted)した後、`weblink` で `wasm-ld: error: --shared-memory is disallowed by array.cpp.o because it was not compiled with 'atomics' or 'bulk-memory' features.` で失敗した。plan の出力は `restore_b2d=true / run_b2d=false`、`Cache restored from key: fl-b2d-cb54413d…`。デプロイは skipped で、本番は bundle e61f80af のまま。
- 原因: 層 c が `ci/stage-wasm.sh` の上流 Blend2D のビルドに `EMCC_CFLAGS="-matomics -mbulk-memory"` を足した(共有メモリの実スレッド/TLS 構成のため)が、`ci/plan.sh` の `KB2D` のキーにそのフラグが入っていなかった。キーが変わらず、アトミクス無しで作られた旧い `libblend2d.a` がキャッシュから復元された。opt1 と私の手元の CI 段階ビルドは b2d をキャッシュ無しで作ったので通っていた(CI だけが踏む経路)。
- 直し: `ci/lib/common.sh` に `B2D_CFLAGS="-matomics -mbulk-memory"` を定義し、`KB2D` に `cflags=$B2D_CFLAGS` を入れ、`stage-wasm.sh` は `EMCC_CFLAGS="$B2D_CFLAGS"` を使う(文面の二重持ちをやめる)。再発防止に `ci/test-b2d-key.sh`(b2d の build の入力 — フラグ・最適化・パッチ — がキーに入っていること)を新設し、deploy.yml の静的検査に加えた。PM が測った値: bash -n、shellcheck -x -S warning、yaml、`ci/test-needs.sh`(128 通り)、`test-hybrid.sh`、`test-lifecycle.sh`、`test-b2d-key.sh`、`test-real-deploy.py`、`test-wrangler-config.sh` すべて rc=0。
- 教訓(層 e 以降の規則): ビルドの入力(環境変数・フラグ)を足す変更では、そのビルドのキャッシュキーにも入れる。手元の段階ビルドはキャッシュ無しなので、この種の欠陥は CI の実行でしか出ない。

### 追補 2(16:35): 修正後の実行 41986214303(c5ed023b)が次の欠陥で失敗した — ci/needs.sh がリンクの fe-wasmgen を数えていなかった

- 事実: Blend2D は作り直され(キーの変更が効いた)、objects は復元されたが、`run link` の `hybn0` が最初の 1 秒で `build-engine-bundle: cargo build of fe-wasmgen failed` で失敗した(ログ: `restore_lift=true`、`restore_felift=false`、`run_felift=false`、`restore_link=false`)。デプロイは skipped、本番は bundle e61f80af のまま。
- 原因: `ci/needs.sh` は `run_felift/restore_felift` を `run_lift`(リフトを実行するとき)だけから決めていた。リンク段の `hybn0` と `hybrid` は fe-wasmgen を実行するので、**リフト出力はキャッシュから復元できて(run_lift=false)リンクだけ作り直す**とき、fe-wasmgen が無く、`hyb_few` のオフライン cargo ビルド(runner に Rust も crate も無い)が失敗する。今日の他の実行はどれも `run_lift=true`(リフトのキャッシュが無かった)で、felift が必ず復元/実行されていたので表に出なかった。前の失敗(e840e4d4)がリフトのキャッシュを保存したため、初めてリフトが復元される実行になった。層 c より前にあった穴で、F2xH7 のハイブリッドの時代から潜んでいた(リフトの鍵が変わらず runtime だけが変わる push で出る)。
- 直し: `need_felift = run_lift または need_link` を導入し、`run_felift = need_felift × (1 − HF)`、`restore_felift = need_felift × HF`。`ci/test-needs.sh` に不変条件を追加(リフトを走らせるかリンクを走らせるなら、lifter のバイナリが復元か実行で必ず用意される。使う者が無いときは使わない)。旧い needs.sh に対して新しい test-needs.sh は失敗する(欠陥を検出)ことを確認した。PM が測った値: test-needs(128 通り)、test-b2d-key、test-hybrid、test-lifecycle、test-real-deploy、test-wrangler-config すべて rc=0、shellcheck -x -S warning、bash -n、yaml。
- 規則(層 e 以降): CI のキャッシュの経路(復元/実行の組)は、各ステージが**実行時に使う道具**(cargo のバイナリ、emsdk、Rust)まで含めて表にする。手元の段階ビルドは全道具が揃っているので出ない。

## PM の追補 3(2026-11-05、CI run 41987198683 の deploy ジョブの失敗)

run 41987198683(a6331e92)は build ジョブが成功した(34 分:link 1,369 秒、bundle は約 9 分)が、deploy ジョブが step 13『R2: the beacons (perf/) expire after 180 days』で失敗した(`aws s3api get-bucket-lifecycle-configuration` が AccessDenied)。ファイルのアップロード(step 12)までは済み、Worker の新版(step 14)・既定 manifest の切替(step 15)・Access の確認(step 16)は走らなかったので、**本番は切り替わらなかった**(e61f80af のまま)。run 41988827762(5535f603、決定 0248)も bundle キャッシュが当たって build は skipped、deploy が同じ step で失敗した。

- 原因: 層 c が `ci/deploy-manual.sh`(手動の 2 本目の経路)を消すとき、R2 `perf/` の 180 日 lifecycle の設定を CI の step にした(opt1-c)。この step は一度も実 R2 で実行されていなかった(単体テストは `r2_lifecycle.py` の JSON 処理だけ)。CI が使うキャッシュ用の R2 認証は『オブジェクトの読み書き』で、**バケットの設定(lifecycle)は読み書きできない**。これは先の 2 回の失敗(b2d の鍵、fe-wasmgen の準備)と同じ型=『実環境でしか出ない前提(権限・ツール・キャッシュ)を、局所のビルドと単体テストでは確かめていない』。
- 直し(ci1、`ci/stage-deploy.sh`、`ci/test-lifecycle.sh`):lifecycle の読み/書きが `AccessDenied` のときは `::warning title=R2 lifecycle not applied::…` と step summary の 1 行を出して続ける(perf/ の期限切れは衛生で、デプロイの条件にしない)。それ以外のエラー(接続できない等)は今までどおり失敗する。旗・設定なし。`test-lifecycle.sh` に stand-in の `aws` で 5 通りのテストを足した(読み AccessDenied → 警告で rc 0、読み NoSuch + 書き ok → 追加で rc 0 警告なし、読み NoSuch + 書き AccessDenied → 警告で rc 0、読み他 → rc 2、書き他 → rc 2)。PM が測った値:dry-run 問題なし、`bash -n` 全 ci スクリプト、shellcheck -x -S warning rc=0、`test-lifecycle.sh`・`test-needs.sh`・`test-hybrid.sh`・`test-b2d-key.sh`・`test-real-deploy.py`・`test-wrangler-config.sh` がすべて rc=0。
- 取らなかった道:バケット設定の権限を持つ R2 トークンを依頼者に作ってもらう(`docs/policies/owner-contact.md` に反する)。その間 perf/ の 180 日の期限切れは掛からず、CI の警告として毎回出る。ビーコンは数だけで小さい(1 件 数 KB)ので、期限が掛からない間の費用は容量だけ。
- 学び:CI の step を足すときは、(1) 実際に使う認証で許される操作か、(2) 失敗したときにデプロイを止めてよいか(衛生は止めない)、を決定文書に書く。本番への切替(Worker・manifest)の前に置く step は、衛生のものを後ろか非致命にする。

## opt1 の追補(層 e、2026-11-05 18:20): 再棚卸しと、残っていた選択肢の畳み込み

基準 HEAD b42fad36。差分は `opt1-e.diff`(`patch -p1 --dry-run` が b42fad36 の `git archive` に通る)と、alpha3 の取り込み後に取り直せるよう別にした `opt1-e-gdi-gdc.diff`(fle_gdi.c / fle_gdc.c の `REUSE_AFTER == 0` の死んだ枝)。測定はすべてこの機械の値(持ち主の機械の値ではない)。

### 1. 再棚卸し(`tables/option-reinventory-e.tsv`、`option-reinventory-e-added.tsv`)
段階 1 の 484 行を、コードに(コメント以外で)まだ出るかで調べ直した: 畳み済み(コードに無い)220 / テストの入口 175 / パラメータ 86 / 残る逸脱 3。層 e で見つけた追加 15 行(WRP3_TLS_THREADS、`FLE_MXT` の空スタブ、`FLE_MXC_INLINE/OUTLINE`、`#ifndef` の上書き口、draw1 HLE エントリ、`--opaque-stub`/`--unbal-stub`、固定アドレス経路、FLE_TEST_ENV、b2d_swap の MUT/allow list/LIBM_BRIDGE/STRICT、音声の URL 4 つ、読まれない `-D`、死んだスクリプト)は畳んだ。畳む対象の残りは 0。ページの URL パラメータは 0 個。
- 残る逸脱 3 行 = `FLE_MX_STATS`(2 行)と `FLE_MX_NOFAST`: wall_fr18(`harness/fp1`)の差分テストと vst/mt1 の計測の道具が使う計装で、製品は定義しない。設計案(計装つきヘッダの写しをテスト専用に置く)は優先度が低いので着手していない。
- パラメータとして残した物: manifest の env とその数値、`hostFilesAbi`、`compression`(保存されたバイト列の記述)、fe-lift/fe-wasmgen の入力の指定(`--pdata --seeds --hook-points --site-overrides --extra-imports --hle-entries --prune-list --swap-entries --seh-handler --hybrid-* --c-imports --mem-import --teb --tls-ctx`)、`FLE_MONITOR_W/H` `FLE_DWM_HZ` `FLE_DISPLAYSETTINGS_FREQUENCY` `FLE_SCREEN_REMAXIMIZE`、ビルドの経路・ジョブ数。

### 2. 製品の出力(CI 段階スクリプトでゼロから。基準 omB と層 e の omE4)
| | 基準 omB(6eb1737d の素) | 層 e(omE4) |
|---|---|---|
| 出荷の `flengine.c` sha256 | `e72edd4a…8509` | 同じ |
| `hyb/exports.rsp` / `c-imports-used.u.txt` | 2,227 行 | diff で同一 |
| 直接 wasm 7 本の -Oz 後の大きさ | fle 5,271,979 … elastique 1,183,785 B | 同一 |
| リンク直後の `flengine6.wasm` | 175,210,527 B | 175,209,978 B(−549) |
| `flengine6.opt.wasm` | 141,795,291 B、`3d0279d4…cac3`(本番 623aba04 と一致) | 141,794,762 B(−529)、`f08ae557…03e3` |
| glue | `6ac38751…` | `2d3baf71…` |
| .o(155 個) | | 151 同一 / 4 違い |
- 基準 omB は本番と byte 一致した(経路の違う私の木で再現 = 決定的)。
- 違う 4 つの .o と理由(どれも層 e の意図した変更): `b2d/b2d_swap.o`(MUT/allow list/LIBM_BRIDGE/STRICT の畳み込み。20,423 → 20,193 B)、`rt/fle_audio_ring.o` と `rt/fle_audio_transport.o`(音声: lead をページの URL パラメータ/EM_JS から 60 ms の定数に。`env.fle_js_lead_ms` の import が 1 つ減る)、`rt/fle_hle.o`(`fle_hle_pixscale` の削除。リンクで落ちるので wasm には効かない)。wasm の違いは import −1(`env.fle_js_lead_ms`)・export −1・global −1・memory-data −213 B だけ。有効側の契約: 音声の lead 60 ms は決定 0139 の既定値(変更なし)で、ページから変える口を消しただけ。
- 途中で出力を変えた設計が 2 つあり、どちらも採らなかった: (1) `FLE_TEST_GETENV` を weak 関数にした版は wasm を +9,708 B・import +1・関数 +26 にした(未定義の weak 関数が env import になり、テストの入口のコードがオブジェクトに残る)。(2) 製品が NULL を返す強い定義を持つ版も +9,705 B(wasm-opt が畳まなかった)。採った設計: 製品の木の `fle_test_env.h` は層 c までと同じ前処理(wasm では null の定数)で、wasm のテストビルドは**木の写し**の中でこのヘッダ 1 つを `harness/parity/fle_test_env_wasm.h` に差し替え、include する 8 単位のオブジェクトを作り直す(`make-test-tree.sh`)。旗も `-D` も分岐も無く、製品の木とビルドはテストビルドの有無で変わらない。
- ゼロからのビルドで出た欠陥(cargo test では見えなかった): 固定アドレス経路を消したとき、`stage_hybn0`(ダミーアドレスで関数を数える n0 パス)が `--tls-ctx` 無しで `--hybrid-r` を渡していた。n0 パスに `--tls-ctx 0x0` を足して直した(`exports.rsp` は基準と同一)。

### 3. 実エンジン(wasm のテストビルド = 上の木の写し、node、JSPI、仮想時計)
`cmp-short.g3` の 16 スナップショット 84 行、`ref1-perf-short.g3` の 55 行、`ref1-perf-wasm.g3` の `g3-` 87 行が、層 c の最終 Ω の行(`tables/g3-lines-omega-*`)と **差 0 行**(同一)。Chrome 156 の既定の入口で、omE4 の本番 bundle のエンジンが first idle に達した: 15.1 s、CPU 22.05 s(load 23.8 の最中の 1 回。P1 の 10 s の判定には使えない。静かな窓での 15 回測定は未実施)。

### 4. テスト(この機械)
- `cargo test --workspace --no-fail-fast`(debug): 200 結果 / 2,347 通過 / 0 失敗 / 1 無視(基準 b42fad36 の alpha3 のテスト 1 本を含む。HEAD 5535f603 では 199 / 2,346 と同数)。release では `int_ext` の 2 本(`movsx_matches_host` `idiv_imul_matches_host`)が落ちるが、**基準の木(b42fad36)の release でも同じ 2 本が落ちる**(既存の問題。`logs/e-int-ext-base-release.txt`)。
- app: typecheck OK、vitest 1,269 通過 + 7 skipped、`test:worker` 49/49、`test:e2e` 125/125(基準の木でも 125/125)。mutation の worker/e2e 層は 20 分の上限で途中まで(32 DETECTED、MISSED 3 は unit 層で検出する物)で、完走した値ではない。
- ci: `test-needs`(128 通り)、`test-hybrid`、`test-lifecycle`、`test-b2d-key`、`test-wrangler-config`、`test-real-deploy.py`、shellcheck 0.11 `-x -S warning`、`bash -n`、`py_compile` すべて OK。
- 走らせていない物: a4 FRUITY off(942/942 …)、11 場面、Firefox、静かな窓の first idle 15 回、再生 CPU/テンポ(`tools/tempo_opt1.sh` は tpage の `vite preview` に対して使える状態)、alpha3 を含む木での全面ビルド(CI 41994297607 が行う)。

### 5. CI のキャッシュキー(`tables/ci-cache-keys-e.tsv`)
`ci/plan.sh` を基準と層 e の木で実行: k_felift / k_lift / k_rt / k_link / k_seed / k_bundle / k_app が変わり、k_b2d / k_extract / k_vst3 は変わらない(Blend2D・抽出の restore は正しい)。触った入力ごとの対応表を付けた。

## PM の決定と取り込み(層 e、2026-11-05)

取り込んだ基準: HEAD 4d70b8f4(runtime は b42fad36 = 本番 bundle afd46e67 と同一)に、opt1 の `opt1-e.diff`(184 ファイル)→ `opt1-e-gdi-gdc.diff`(`fle_gdi.c`・`fle_gdc.c` の死んだ枝、別 hunk)を `patch -p1 -E`(offset・fuzz・reject なし)。`-` 行のうち製品に関わる物(`app/scripts/build-engine-bundle.sh`、fe-wasmgen の `lib.rs`・`rcgen.rs`・`lower.rs`・`main.rs`・`wasm.rs`、`fle_audio_ring.c`、`b2d_swap.cpp`、`fle_hle.c`、`app/src/audio/*`)を読んだ: 無効側の分岐・URL パラメータ・`-D` の削除と、`stage_hybn0` の n0 経路への `--tls-ctx 0x0` の追加だけで、有効側の挙動は変わらない。

PM が測った値(この機械、`lift/out-pmchk-e/`):
- FLEngine の既定の生成 C(`fe-lift lift … --pdata`)の sha256 **`34a376dbf23a30c62df5ce6d04c07ed9beb5b651dd82d608cf5bcc59af78062a`**(215,740,414 B)= 層 d 以後の基準と同一(lifter の出力は不変)。
- `bash -n`、shellcheck 0.11 `-x -S warning`、`ci/test-{needs,hybrid,lifecycle,b2d-key,wrangler-config}.sh`、`test-real-deploy.py` rc=0。
- app: typecheck rc=0、vitest 1,269 passed + 7 skipped(音声の URL パラメータ削除に伴うテストの整理で基準 1,274 から減)、`test:worker` rc=0、エンジン無しの `test:e2e` rc=0。
- `test.sh full`: 200 results / 2,347 passed / 0 failed / 1 ignored(rc=0)。
- a4(FRUITY 無効): a4 FRUITY 無効 init_count 942/942・run_rank 895/895・functions 5,310・entries 1,009,737・started_but_not_entered 0(不変)

opt1 の測定(PM は再現していない): 層 e 前後の Ω の wasm は 141,795,291 B/`3d0279d4…` → 141,794,762 B(−529 B)/`f08ae557…`(**pre-alpha3 の runtime オブジェクトで比べた値**)。155 個の .o のうち違うのは意図した 4 つ(`b2d/b2d_swap.o`、`rt/fle_audio_ring.o`、`rt/fle_audio_transport.o`、`rt/fle_hle.o`)で、`hyb/exports.rsp` と出荷の `flengine.c`(`e72edd4a…`)は同一、import が 1 減る(`env.fle_js_lead_ms`)。b42fad36 を基準にした再比較(omB2/omE5)は opt1 が 18:50〜19:00 に出す。想定: omB2 が本番 bundle の wasm 141,803,497 B/`c56d8542…` を再現し、omE5 が約 −530 B。

判断:
1. **採る。** 残る逸脱は `FLE_MX_STATS`(2 行)と `FLE_MX_NOFAST`(wall_fr18 と測定道具が使うテスト専用)だけで、製品は定義しない。畳む対象の残りは 0。
2. 製品の出力が変わる変更は「有効側が既に唯一」だったもの(音声の先行量 60 ms は決定 0139 の既定値のまま)に限られる。draw1 の HLE エントリ(出荷したことのない実験)は削除し、外部 DLL の HLE は残す。
3. 未実行(取り込み時): 最終 bundle での 11 場面・Firefox・first idle 15/15、omE5 の node 行(84/55/87)、mutation 41/41 の記録対応(3 件の MISSED は unit 層で検出)、`tools/tempo_opt1.sh` の `openRef` タイムアウトの原因(基準ページでも出るかを omB2 で確認)。push 後の本番 bundle で ci1 が確認する。
4. 履歴スクリプトは触らず、`docs/investigations/HISTORY-NOTE.md` に「当時のビルド変数・`lift/out-*` を前提にし、現行では動かない記録」と注記した。

### PM の追補(層 e の CI の結果、2026-11-05 19:37)

CI 実行 42003204549(main cd4923f5)が成功し、19:34 JST に本番へ出た(Worker 版 77699bbe-885e-4d82-9202-948d282d6511、bundle `fl-engine-ci-251dfabc`)。ci-report.sh の値: felift 26 s、lift 46 s、wasm objects 1,398 s(全 8 モジュール冷間)、link 1,562 s、bundle 506 s。runner の cgroup memory.peak(system.slice)7.70 GB、swap 0、OOM なし。lifecycle は警告のみ(`perf/` の期限切れは未適用のまま、デプロイは続行)。Access の無認証 16 件は 302。**engine wasm は 141,802,968 B、sha256 の先頭 9b3a91d2d8ee で、opt1 の手元の基準 b42fad36 + 層 e の再構築(omE5: 141,802,968 B、9b3a91d2d8eee178…)と byte 一致**した。前の本番(afd46e67、層 e なし)は 141,803,497 B で、差は予想どおり −529 B。つまり層 e は「製品の出力を変えない畳み込み」という規則の範囲(音声の先行量 60 ms の定数化と `env.fle_js_lead_ms` の import 1 本の消滅、draw1 HLE 関数の除去、b2d_swap の畳み込みの 4 オブジェクト)に収まった。実ブラウザでの確認(first idle、場面、Firefox、beacon、音声の先行量)は ci1 が `fl-engine-ci-251dfabc` で行う(未了)。
