# 0345: G2 の 3 者比較 E13 / RT-A / RT-H(plain)(+ RT-F の 1 本) — ci1, 2026-11-08

## 実際に実行した物

- 実 Chrome 156 headless(CDP)で実エンジン。シナリオ `tools/scn-lat-ff.json`(0342 / 0344 と同じ)、解析は `tools/g2-multi.py`(0342 の `g2-analyze.py` と同じ record の選択規則)。run は交互(E13・RT-F・RT-H → RT-A・E13・RT-H・RT-A)。
- **RT-H の plain 版**: bundle `~/work/mt3w-s1/ci-rth/engine/bundle/fl-engine-ci-b2fdfca0`(SHA256SUMS 9/9 OK、wasm 170,941,858 B、`engine.498095c419b0.wasm`、dll/glue/seed は RT-G と同一)。manifest は **bundle 自身の manifest の複写で、`engine.env` に FLE_DIAG なし**(env = FLE_AUTH_CLAIMS / FLE_FRUITY_MAX_CALLS / FLE_FRUITY_SECONDS / FLE_REG_SEED / FLE_VFS_SEED / FLE_VFS_SEED_ROOT の 6 つだけ)。mt3 の `mk-rth-assets.sh BUNDLE plain` の出力(`assets-rth-plain`)と、私が先に作った `assets-rth`(`mk-bundle-assets.sh`)の manifest は **byte 同一**(`cmp` で確認)。測定には `assets-rth`(port 5143、page-rt1)を使った。bundle 本体は書き換えていない。`ci-` と `test-` の wasm は `cmp` で同一。
- E13 = `assets-fl-engine-ci-0363b050`(port 5053、page-d0447902)、RT-A = `assets-rta`(bundle `fl-engine-test-rt1-91bf78f6`、port 5101、page-rt1)、RT-F = `assets-rtf`(`fl-engine-ci-d45afb0e`、port 5122、page-rt1)。
- **counted の run 7 本**(全部 ok、停止なし、`state=engine-idle`)。開始前の gate はすべて load≤8、swap si/so 0/0(gate が止めた起動は run していない)、MemAvailable≥8 GB、他 Chrome 0、Wine 0(開始時に `wait-no-wine`、`ps` で FL64 / wineserver 0)。

  | run | 条件 | 開始〜終了 | 開始前 load | MemAvailable | swap si/so | Wine | 他 Chrome | first idle |
  |---|---|---|---|---|---|---|---|---|
  | prev-10 | E13-1 | 07:20:31–07:24:53 | 2.01(quiet 判定)| 17133 MB | 0/0 | 0 | 0 | 5.52 s |
  | new-11 | RT-F | 07:25:36–07:30:00 | 2.66 | 28613 MB | 0/0 | 0 | 0 | 9.31 s |
  | new-12 | RT-H-1 | 07:30:21–07:34:43 | 3.77 | 26008 MB | 0/0 | 0 | 0 | 8.43 s |
  | new-13 | RT-A-1 | 07:35:10–07:39:33 | 4.01 | 25227 MB | 0/0 | 0 | 0 | 8.26 s |
  | prev-14 | E13-2 | 07:39:44–07:44:05 | 3.88 | 24970 MB | 0/0 | 0 | 0 | 5.27 s |
  | new-15 | RT-H-2 | 07:44:12–07:48:36 | 2.78 | 25187 MB | 0/0 | 0 | 0 | 8.28 s |
  | new-16 | RT-A-2 | 07:48:58–07:53:21 | 3.65 | 28998 MB | 0/0 | 0 | 0 | 8.20 s |

  起動を止めた gate: swap si=8(07:19)、si=4(07:25 の 2 回)、si=4(07:48)。いずれも待って再実行。
- 診断の trace run(非計時、counted ではない): RT-H を `run-tp.sh rth` で 1 本(`tp-5-rth`、07:54:24–07:56:08、load 2.94、MemAvailable 29843 MB、swap 0/0、Wine 0、他 Chrome 0)。0343 の RT-F / E13 と同じ手順(CDP Tracing + v8 cpu_profiler)。

## 結果 1: G2(post-play、T = 最初の answered frame − tPage、中央値 ms)

run 別の中央値(括弧は n)。E13・RT-A・RT-H は 2 本ずつ、RT-F は 1 本。

| 入力 | E13(run1 / run2) | RT-A | RT-H plain | RT-F |
|---|---|---|---|---|
| **再生中 File click** | 20.00 / 20.52 (n=35,35) | 45.22 / 44.94 (n=35,35) | **39.86 / 40.38** (n=35,35) | 45.39 (n=35) |
| **再生中 key down** | 5.11 / 5.12 (n=36,36) | 12.36 / 10.71 (n=36,35) | **8.64 / 12.79** (n=36,36) | 8.61 (n=36) |
| idle File click | 7.27 / 7.46 (n=36,36) | 8.85 / 9.25 (n=36,36) | 8.08 / 8.23 (n=36,36) | 9.27 (n=36) |
| idle key down | 2.18 / 2.20 (n=31,33) | 2.42 / 2.56 (n=34,35) | 2.35 / 2.40 (n=33,35) | 2.48 (n=35) |

run 間のばらつき(同じ条件の 2 本の差): E13 は 再生中 File click +0.52、key down +0.02。RT-A は File click −0.28、key down −1.65。RT-H は File click +0.52、**key down +4.15**(8.64 → 12.79)。

差(各条件の 2 本の中央値の平均 − E13 の平均、ms):

| 入力 | RT-A − E13 | RT-H − E13 | **RT-H − RT-A** | RT-F − E13(1 本) |
|---|---|---|---|---|
| 再生中 File click | +24.82 | +19.86 | **−4.96** | +25.12 |
| 再生中 key down | +6.41 | +5.60 | **−0.82** | +3.49 |
| idle File click | +1.68 | +0.79 | −0.89 | +1.90 |
| idle key down | +0.30 | +0.18 | −0.12 | +0.29 |

(E13 の平均: File click 20.26、key down 5.12。RT-A 45.08 / 11.54、RT-H 40.12 / 10.71、RT-F 45.39 / 8.61。差の列は小数第 3 位以下の丸めを含む。全 run を並べた表は `g2-all-runs.md`。)

## 結果 2: 再生中 File click の phase 別(中央値の平均、同じ lat run のページ側の時刻)

| phase | E13 | RT-A | RT-H | RT-F |
|---|---|---|---|---|
| A tEvent→tPage | 0.60 | 0.64 | 0.60 | 0.65 |
| B tPage→tInject(UI Worker に届くまで) | 0.40 | 2.65 | 2.37 | 2.97 |
| **C tInject→最初の frame(UI Worker の処理)** | **19.58** | **42.42** | **36.19** | **42.42** |
| D 最初の frame→rAF(提示) | 2.67 | 4.40 | 5.21 | 5.22 |
| T tPage→最初の frame | 20.26 | 45.08 | 40.12 | 45.39 |

## 結果 3: UI Worker の phase C の中身(trace + v8 CPU profile、File click 12 回、ms/入力、非計時)

| run | UI Worker の phase C 時間 | (program) | wasm-fn [1368] / E13 の [1367] | [3035] / E13 の [3034] | g9_hook の self / 包含 | dispatch_message 包含 | fle_gil_* の self |
|---|---|---|---|---|---|---|---|
| E13 run1 / run2 | 20.61 / 19.77 | 0.55 / 0.21 | 10.38 / 9.64 | 3.65 / 3.21 | なし | なし | なし |
| RT-F run1 / run2(0343) | 47.65 / 42.17 | 10.19 / 8.30 | 12.41 / 12.41 | 5.59 / 4.45 | 3.93 / 34.90 → 3.93 / 31.82 | 34.55 / 31.60 | 0.97 / 0.74 |
| RT-G run4(0344) | 44.02 | 10.58 | 11.35 | 5.04 | 3.27 / 31.58 | 31.53 | 1.33 |
| **RT-H plain** | **39.81** | **8.16** | **12.48** | **3.94** | **0 / 0(g9_hook が現れない)** | **28.95** | **1.52** |

- **RT-H plain では g9_hook が profile に現れない**(self 0、包含 0)。RT-F / RT-G では phase C の 31〜35 ms/入力(73〜75%)が g9_hook の下だった。RT-H の `[1368]`・`[3035]` の最寄りの名前付きの祖先は `f_868c00`(RT-F では g9_hook)。観測できたのは「**phase C の 12 クリック分の profile に g9_hook が 1 サンプルも現れない**」ことまで(mt3 の説明どおり、plain では設置されていないことと矛盾しない)。「run 全体で一度も呼ばれない」ことまでは測っていない。wasm の name section には `g9_hook` が RT-F にも RT-H にも 1 件ある(関数としては存在する)。
- それでも RT-H の phase C は 39.81 ms/入力 で、E13(約 20 ms)の約 2 倍。**g9_hook を外しても、UI Worker の phase C は 約 20 ms 長いまま。**

## 判定(この表だけから)

1. **RT-H と RT-A の差が +10 ms 以内か**: 再生中 File click は RT-H − RT-A = **−4.96 ms**、key down は **−0.82 ms**(いずれも +10 ms 以内。RT-H の方が小さい)。idle も −0.89 / −0.12 ms。
2. **フック由来(RT-A と RT-H の差)**: g9_hook を外した RT-H は、RT-A より再生中 File click で **約 5 ms 速い**(45.08 → 40.12)。key down は RT-A と同程度(11.54 / 10.71、run 間で RT-H は 8.64 と 12.79 に割れる)。**フックを外して回復するのは、File click で約 5 ms(RT-A の遅れ +24.8 ms の約 2 割)。**
3. **実スレッド由来(E13 と RT-H の差)**: フックを持たない RT-H でも、E13 比で再生中 File click は **+19.86 ms**(2 本の平均。E13 の 1 本目との差で run 別に出すと +19.86 / +20.38)、key down は **+5.60 ms**(run 別 +3.53 / +7.68)。RT-A の遅れ(+24.8 ms)の **約 8 割(File click)が、g9_hook を外しても残る**(計算: 19.86 / 24.82 = 80%、フックを外して減る分は 4.96 / 24.82 = 20%)。key down は RT-A − E13 が +6.41、RT-H − E13 が +5.60 で、**フックの有無の差は run 間のばらつき(RT-H で 4.15 ms)より小さく、key down では割合を出さない**。
4. 以上から、**再生中 File click の遅れ +24.8 ms は、フック由来 約 5 ms、実スレッド固有(フックなしで残る)約 20 ms に分かれる**(RT-A と RT-H は bundle が異なる: RT-H は RT-G + g9_hook の条件付き設置 + alpha1 の expos 修正 + O20 delta pointer。**この分け方は「RT-A → RT-H の差が g9_hook の有無だけ」という前提**で、RT-H の他の変更(expos 修正・O20 delta pointer)の影響は分離していない)。
5. **+10 ms の gate(E13 比)**: 再生中 File click は RT-H でも **+19.86 ms で範囲外**。key down は +5.60 ms(範囲内だが run 別 +3.53 / +7.68 とばらつく)。G2 は RT-H でも **不合格**。

## 注意 / ばらつき

- RT-H の key down は 2 本で 8.64 と 12.79(差 4.15 ms)。RT-A も 12.36 と 10.71(1.65 ms)。key down の差(RT-H − RT-A = −0.82 ms)は、この run 間のばらつきの内側で、**RT-H と RT-A の key down の差は区別できない**。File click の差(−4.96 ms)は、run 間のばらつき(各 0.3〜0.5 ms)より大きい。
- 標本は各 run で File click 35 件、key down 34〜36 件。中央値の信頼幅は出していない(run 間の差で見ること)。
- RT-F は 1 本のみ(0342 の +23.37 / +10.71 と、今回の +25.12 / +3.49 は、File click は同程度、key down は 10.71 → 3.49 と大きく違う: **key down の run 間のばらつきは 3〜4 ms に収まらない場合がある**)。
- E13 の run 間のばらつきは小さい(+0.52 / +0.02)。
- trace run(結果 3)は 1 本ずつ(RT-H は 1 本、RT-F は 2 本、RT-G は 1 本、E13 は 2 本)。(program) の中身(待ちか native 処理か)は、今回も区別できていない。

## 未取得 / 未確認

- phase C の約 20 ms の残りの中身: RT-H の phase C のうち、E13 との差 約 20 ms の内訳(wasm の self +約 17、(program) +約 8)。上位の wasm 関数 [1368] / [3035] / [3145] は name section に名前が無く、何をしているかは **未取得**。
- (program) が待ち(Atomics.wait / futex)か native 処理か。
- RT-H の他の変更(expos 修正・O20 delta pointer)の寄与。RT-G(g9_hook あり・expos 修正なし)の G2 は測っていない。**分離したい場合は、RT-G の G2 を同じ手順で 1 組測れば、RT-G → RT-H の差が (expos 修正 + O20 delta pointer + g9_hook の条件付き設置) になる**(これも 3 変更の合算で、g9_hook だけは分けられない)。
- E13 + フック(E13 に g9_hook を足した版)は存在しない。

## 保存先

`lift/out-git/ci1-results/0345/`: この REPORT.md、`g2-all-runs.md`(全 7 run の表)、`lat-chrome-{prev-10,new-11,new-12,new-13,prev-14,new-15,new-16}/`(lat-*.json、report.json、gate.txt)、`tp-5-rth/`(RT-H の trace run の lat-tp-play.json など)、`tp{1,2,3}-5-rth.md`(0343 と同じ解析の出力)、`g2-multi.py`、`run-rta2.sh`、`run-tp.sh`。生 trace(RT-H 約 115 MB)は `lift/out-ci1/kit/out/tp-5-rth/tp.trace.json` にあり、複写していない。
