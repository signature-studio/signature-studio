# G2 再生中 T の +23 ms の切り分け(RT-F 対 E13) — ci1, 2026-11-08

## 実際に実行した物

- 実 Chrome 156 headless(CDP)で実エンジンを **各条件 2 本**(非計時の診断 run。PM 05:45 の緩和: load≤12、MemAvailable≥12 GB、swap 入出力 0)。
  | run | 条件 | 時刻 | 開始前 load | MemAvailable | swap si/so | Wine | 他 Chrome |
  |---|---|---|---|---|---|---|---|
  | tp-1-e13 | E13 協調版(page-d0447902, :5053) | 05:48:09–05:49:42 | 2.04 | 17788 MB | 0/0 | 0 | 0 |
  | tp-1-rtf | RT-F 本番形 fl-engine-ci-d45afb0e(page-rt1, :5122) | 05:50:10–05:51:46 | 5.66 | 30982 MB | 0/0 | 0 | 0 |
  | tp-2-e13 | E13 | 06:06:17–06:07:53 | 4.47 | 19687 MB | 0/0 | 0 | 0 |
  | tp-2-rtf | RT-F | 06:07:55–06:09:37 | 4.13 | 27142 MB | 0/0 | 0 | 0 |
  RT-F の最初の起動(05:49:45)は swap si=4 で gate が止め、run していない(quiet 待ちの後に再実行)。
- シナリオ `tools/scn-tp.json`: 参照 project を開き Space で再生 → 10 s 後に **CDP Tracing 開始**(toplevel, devtools.timeline, disabled-by-default-devtools.timeline, disabled-by-default-v8.cpu_profiler: renderer の全スレッド = ページ main、UI Worker、pthread Worker、AudioWorklet、Compositor)→ File メニューの開閉 12 回(Esc で閉じる)→ 停止。
  trace の時刻とページ時計(epoch ms)は trace 内の `console.timeStamp` 2 点(sync-a / sync-b)で対応づけた(2 点のずれ: 11–15 µs)。
- phase: A=tEvent→tPage(ブラウザ→ページ)、B=tPage→tInject(ページ→UI Worker)、C=tInject→最初の frame(UI Worker の処理)、D=最初の frame→次の rAF(提示)。window = 各入力の tPage ±200 ms。
  **数値は trace 区間内の入力のみ**(File click 12、Escape 11–12。区間の前の入力は除外)。
- 道具: `tools/sdriveg2-lat-tp.mjs`(syncClock 追加)、`tools/run-tp.sh`、`tools/scn-tp.json`、`tools/tp-analyze{,2,3,4,5}.py`。生 trace は `lift/out-ci1/kit/out/tp-{1,2}-{e13,rtf}/tp.trace.json`(86–113 MB。`0343/` には複写していない: 容量のため。再現手順と解析出力は置いた)。

## 読み方の注意

- trace と v8 CPU profiler を載せた run のため、**絶対値は `0342` の計数表(E13 16.46 / RT-F 39.83 ms)と一致しない**(この run の T は E13 20–22 / RT-F 45–49 ms)。E13 と RT-F を**同じ条件で採った 2 本の差**を見る。
- 中央値は加算できない(A+B+C+D は T2 に一致しない)。File click は各 run 12 件で、p90 は 12 件の上位 2 番目前後。
- RT-F の wasm の関数名は name section(126,910 件)で解決。E13 の wasm には name section が無く、E13 側は `wasm-function[N]` のまま。

## 1. +23 ms の内訳 — 再生中の File click(中央値、2 run)

| phase | E13 run1 / run2 | RT-F run1 / run2 | 差 RT-F − E13(run1 / run2) |
|---|---|---|---|
| A tEvent→tPage | 0.65 / 0.66 | 0.67 / 0.76 | +0.0 / +0.1 |
| B tPage→tInject(**入力が UI Worker に届くまで**) | 0.35 / 1.14 | 0.90 / 2.85 | +0.6 / +1.7 |
| **C tInject→最初の frame(UI Worker の処理)** | **18.96 / 18.52** | **46.87 / 44.39** | **+27.9 / +25.9** |
| D 最初の frame→rAF(**提示**) | 3.44 / 2.00 | 7.05 / 10.85 | +3.6 / +8.9 |
| T tPage→最初の frame | 20.45 / 21.77 | 49.33 / 45.38 | +28.9 / +23.6 |
| T2 tPage→rAF | 27.76 / 24.79 | 58.15 / 61.52 | +30.4 / +36.7 |

- T の差(+28.9 / +23.6 ms)のほとんどが **C(UI Worker の処理)の +27.9 / +25.9 ms**。入力が UI Worker に届くまでの待ち(A+B)は +0.6 / +1.8 ms。
- 提示(D)は +3.6 / +8.9 ms でばらつきが大きい(rAF までの待ちで、最初の frame を出した後)。T2(rAF まで)の差は +30.4 / +36.7 ms。

**Escape(key-down、再生中)**

| phase | E13 run1 / run2 | RT-F run1 / run2 | 差(run1 / run2) |
|---|---|---|---|
| B tPage→tInject | 0.30 / 0.23 | 1.20 / 0.46 | +0.9 / +0.2 |
| **C tInject→最初の frame** | **3.19 / 4.61** | **6.64 / 6.62** | **+3.5 / +2.0** |
| D 最初の frame→rAF | 2.15 / 2.92 | 2.91 / 3.42 | +0.8 / +0.5 |
| T tPage→最初の frame | 4.59 / 5.03 | 10.27 / 9.06 | +5.7 / +4.0 |

- Escape の T の差は +5.7 / +4.0 ms(今回の trace run)。`0342` の計数表の +10.71 ms より小さい。**この差は trace run の標本(11–12 件)の幅の中で、0342 の値と食い違う。どちらが代表かは、この採取だけでは決められない。**

## 2. UI Worker の phase C の中身 — File click(ms/入力、v8 CPU profile、self 時間)

| | E13 run1 / run2 | RT-F run1 / run2 |
|---|---|---|
| phase C に UI Worker が使った時間(profile の合計) | 20.61 / 19.77 | 47.65 / 42.17 |
| wasm の self | 19.7 / 19.1 | 37.0 / 33.5 |
| (program)(V8 が JS/wasm の関数に帰属しない時間) | 0.5 / 0.2 | **10.19 / 8.30** |
| (idle) | 0.0 / 0.0 | 0.3 / 0.2 |
| js | 0.2 / 0.4 | 0.1 / 0.2 |

- **phase C の UI Worker は idle がほぼ 0**(E13 0、RT-F 0.2–0.3 ms)。入力を待っているのではなく、処理している。
- RT-F の増分(run1 +27.0、run2 +22.4 ms)の内訳: wasm の self +17.3 / +14.4、(program) +9.7 / +8.1。
- self 時間の上位(RT-F、2 run で同じ順): wasm-function[1368] 12.41 / 12.41、(program) 10.19 / 8.30、wasm-function[3035] 5.59 / 4.45、**g9_hook 3.93 / 3.93**、fle_desk_compose 2.07 / 1.67、wasm-function[3145] 1.79 / 1.58、wasm-function[5212] 0.79 / 0.97、fle_gil_wake 0.57 / 0.51、fle_gil_acquire_slow 0.40 / 0.23。
- E13 の上位: wasm-function[1367] 10.38 / 9.64、wasm-function[3034] 3.65 / 3.21、wasm-function[99865] 1.14 / 1.57。
  **RT-F の [1368] / [3035] は E13 の [1367] / [3034] に対応するとみられる**(RT-F の wasm は import が 1 つ多く 49 対 48、関数 index が +1)。これは **推定**。関数の同一性は確認していない。RT-F の [1368] は E13 の [1367] より +2.0 / +2.8 ms 重い。
- **名前の解決**: RT-F の name section でも [1368]・[3035]・[3145]・[5212] は名前が無い。これらが何かは **未取得**。
- **g9_hook の包含時間: 34.90 / 31.82 ms/入力(phase C の 73–75%)**。g9_hook の呼び出し先の下に dispatch_message → fle_call_guest が続く(g9_hook 自身の self は 3.93 ms)。[1368] / [3035] / [3145] の祖先のうち名前が付いているものは、3 つとも g9_hook(RT-F)。
  E13 の wasm には名前が無く、g9_hook に相当する関数が E13 にあるかは未取得。
- 名前の付いた分(RT-F の self、ms/入力): g9_hook + fle_hyb* 4.08 / 3.98、fle_desk_compose などの合成・描画 2.31 / 1.82、**fle_gil_\* 0.97 / 0.74**(fle_gil_wake + fle_gil_acquire_slow)、poll / wait / idle 系 0.49 / 0.45。
  **GIL の関数の self は合計 1 ms/入力ほどで、+23〜28 ms の主因ではない**(GIL を待っている時間が、wasm 関数の self にも (program) にも見えている可能性は残る。下の注)。

## 3. (program) と他の Worker(RT-F)

- **(program) の中身**: UI Worker の (program) は、すべて root 直下(祖先に fle_gil_* の frame が **0 ms**)。V8 が wasm に帰属させなかった時間で、**待ち(Atomics.wait / futex)か native 処理かは、この採取では区別できない**(未取得)。
  phase C の UI Worker の CPU/wall(trace event の間の `tts`/`ts`)は RT-F 0.84(E13 1.00)。CPU を使っていない時間が 16% ある(RT-F)。(program) 10.19 ms/入力が phase C の 21% で、CPU/wall の 0.16 とほぼ同じ大きさ。**この対応は推定**で、因果は確認していない。
- UI Worker の (program) の連続した塊(≥1 ms): 12 click で 30 回 / 22 回(run1 / run2)、中央値 1.3 ms、最大 2.9 / 2.3 ms、合計 3.6 / 2.6 ms/click。(program) 10.19 / 8.30 ms/click の大部分は、1 ms 未満の細かい断片。
  その塊の間、他の Worker は program 85% / 84%、wasm 15% / 16%、idle 0%。
- **他の pthread Worker 7 本**(400 ms の window): 6 本が (program) 88–91%(self 時間)、1 本(2975500 / 3045712)は (program) 65% + wasm 34%。trace event 間の CPU/wall は 6 本が 0.19–0.27、1 本が 0.53–0.56。**多くの Worker は CPU を使わずに待っている**。phase C の間も同じ(1 本が wasm 16 ms/入力、他は wasm 3–6 ms/入力)。
- 400 ms window の UI Worker: E13 は CPU/wall 0.92–0.95、RT-F は 0.68–0.70(idle 18–30% を含む)。
- ページ main の phase C の CPU は E13 2.2 ms、RT-F 5.0 ms/入力(idle は約 88%)。main はボトルネックではない。

## 4. 結論(測れた範囲)

1. 再生中の File click の T の差 +24〜+29 ms(2 run)の 90% 以上は **UI Worker が入力を受けてから最初の frame を出すまでの処理 C(+26〜+28 ms)**。入力が UI Worker に届くまで(A+B)は +0.6〜+1.8 ms、提示(D)は +3.6〜+8.9 ms で、主因ではない。
2. C の中身: UI Worker は **idle ではなく処理している**。増分は wasm の self が +14〜+17 ms と (program) が +8〜+10 ms。(program) は GIL 関数の下にはない(V8 が帰属させなかった時間)。
3. C の 73–75% は g9_hook の下(dispatch_message → fle_call_guest)。上位の wasm 関数 4 本(約 20 ms/入力)は **名前が無く、何をしているかは未取得**。
4. **GIL の関数 fle_gil_wake / fle_gil_acquire_slow の self は合計約 1 ms/入力**。GIL 待ちを主因とする証拠は、この採取からは出ていない。ただし GIL の待ちの時間そのもの(待ち・保持の分布)は `FLE_DIAG_GIL` が 0 行で **未取得**(mt3 が RT-G で修正中)。
5. Escape の差は +4〜+6 ms(今回)で、0342 の +10.71 ms より小さい。

## 未取得 / 未確認

- wasm 関数 [1368] / [3035] / [3145] / [5212] の名前(name section に無い)。
- (program) が待ち(Atomics.wait / futex)か native 処理かの区別。
- E13 側の g9_hook 相当の関数の有無と時間(E13 の wasm に名前が無い)。
- GIL の待ち・保持の分布(`FLE_DIAG_GIL` 0 行 → RT-G 待ち)。RT-G の bundle ができれば、同じ `run-tp.sh` で採り直せる。
- 区間内の標本が 12 件のため、中央値の差の信頼幅は出していない(run1 と run2 の差で幅を見ること)。
- 生 trace の `0343/` への複写(容量 約 400 MB。必要なら PM が指示)。

## 保存先

- `lift/out-git/ci1-results/0343/`: この REPORT.md、解析出力 `tp1…tp5(b)-{e13,rtf}.md`、`tp-analyze*.py`、`run-tp.sh`、`scn-tp.json`、`sdriveg2-lat-tp.mjs`、各 run の `lat-tp-play.json` / `report.json` / `gate.txt` / `sync-*.json`。
