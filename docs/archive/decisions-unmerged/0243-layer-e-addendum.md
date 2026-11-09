## opt1 の追補(層 e、2026-11-05 18:20): 再棚卸しと、残っていた選択肢の畳み込み

基準 HEAD b42fad36。差分は `opt1-e.diff`(`patch -p1 --dry-run` が b42fad36 の `git archive` に通る)と、alpha3 の取り込み後に取り直せるよう別にした `opt1-e-gdi-gdc.diff`(fle_gdi.c / fle_gdc.c の `REUSE_AFTER == 0` の死んだ枝)。測定はすべてこの機械の値(持ち主の機械の値ではない)。

### 1. 再棚卸し(`tables/option-reinventory-e.tsv`、`option-reinventory-e-added.tsv`)
段階 1 の 484 行を、コードに(コメント以外で)まだ出るかで調べ直した: 畳み済み(コードに無い)220 / 試験の入口 175 / パラメータ 86 / 残る逸脱 3。層 e で見つけた追加 15 行(WRP3_TLS_THREADS、`FLE_MXT` の空スタブ、`FLE_MXC_INLINE/OUTLINE`、`#ifndef` の上書き口、draw1 HLE エントリ、`--opaque-stub`/`--unbal-stub`、固定アドレス経路、FLE_TEST_ENV、b2d_swap の MUT/allow list/LIBM_BRIDGE/STRICT、音声の URL 4 つ、読まれない `-D`、死んだスクリプト)は畳んだ。畳む対象の残りは 0。ページの URL パラメータは 0 個。
- 残る逸脱 3 行 = `FLE_MX_STATS`(2 行)と `FLE_MX_NOFAST`: wall_fr18(`harness/fp1`)の差分試験と vst/mt1 の計測の道具が使う計装で、製品は定義しない。設計案(計装つきヘッダの写しを試験専用に置く)は優先度が低いので着手していない。
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
- 途中で出力を変えた設計が 2 つあり、どちらも採らなかった: (1) `FLE_TEST_GETENV` を weak 関数にした版は wasm を +9,708 B・import +1・関数 +26 にした(未定義の weak 関数が env import になり、試験の入口のコードがオブジェクトに残る)。(2) 製品が NULL を返す強い定義を持つ版も +9,705 B(wasm-opt が畳まなかった)。採った設計: 製品の木の `fle_test_env.h` は層 c までと同じ前処理(wasm では null の定数)で、wasm の試験ビルドは**木の写し**の中でこのヘッダ 1 つを `harness/parity/fle_test_env_wasm.h` に差し替え、include する 8 単位のオブジェクトを作り直す(`make-test-tree.sh`)。旗も `-D` も分岐も無く、製品の木とビルドは試験ビルドの有無で変わらない。
- ゼロからのビルドで出た欠陥(cargo test では見えなかった): 固定アドレス経路を消したとき、`stage_hybn0`(ダミーアドレスで関数を数える n0 パス)が `--tls-ctx` 無しで `--hybrid-r` を渡していた。n0 パスに `--tls-ctx 0x0` を足して直した(`exports.rsp` は基準と同一)。

### 3. 実エンジン(wasm の試験ビルド = 上の木の写し、node、JSPI、仮想時計)
`cmp-short.g3` の 16 スナップショット 84 行、`ref1-perf-short.g3` の 55 行、`ref1-perf-wasm.g3` の `g3-` 87 行が、層 c の最終 Ω の行(`tables/g3-lines-omega-*`)と **差 0 行**(同一)。Chrome 156 の既定の入口で、omE4 の本番 bundle のエンジンが first idle に達した: 15.1 s、CPU 22.05 s(load 23.8 の最中の 1 回。P1 の 10 s の判定には使えない。静かな窓での 15 回測定は未実施)。

### 4. 試験(この機械)
- `cargo test --workspace --no-fail-fast`(debug): 200 結果 / 2,347 通過 / 0 失敗 / 1 無視(基準 b42fad36 の alpha3 の試験 1 本を含む。HEAD 5535f603 では 199 / 2,346 と同数)。release では `int_ext` の 2 本(`movsx_matches_host` `idiv_imul_matches_host`)が落ちるが、**基準の木(b42fad36)の release でも同じ 2 本が落ちる**(既存の問題。`logs/e-int-ext-base-release.txt`)。
- app: typecheck OK、vitest 1,269 通過 + 7 skipped、`test:worker` 49/49、`test:e2e` 125/125(基準の木でも 125/125)。mutation の worker/e2e 層は 20 分の上限で途中まで(32 DETECTED、MISSED 3 は unit 層で検出する物)で、完走した値ではない。
- ci: `test-needs`(128 通り)、`test-hybrid`、`test-lifecycle`、`test-b2d-key`、`test-wrangler-config`、`test-real-deploy.py`、shellcheck 0.11 `-x -S warning`、`bash -n`、`py_compile` すべて OK。
- 走らせていない物: a4 FRUITY off(942/942 …)、11 場面、Firefox、静かな窓の first idle 15 回、再生 CPU/テンポ(`tools/tempo_opt1.sh` は tpage の `vite preview` に対して使える状態)、alpha3 を含む木での全面ビルド(CI 41994297607 が行う)。

### 5. CI のキャッシュキー(`tables/ci-cache-keys-e.tsv`)
`ci/plan.sh` を基準と層 e の木で実行: k_felift / k_lift / k_rt / k_link / k_seed / k_bundle / k_app が変わり、k_b2d / k_extract / k_vst3 は変わらない(Blend2D・抽出の restore は正しい)。触った入力ごとの対応表を付けた。
