# 0303(仮): UI の最大の関数 `f_863f60` の命令単位の内訳 — 『native N1 より 1.47 倍遅い』は node では再現しない(wasm は N1 の 0.40 倍の時間)、時間の 61% は inline された型検査(InheritsFrom)の依存ロード鎖、fe-wasmgen の改善余地は小さい(mt3)

状態: **仮**(第 1 報。Chrome は起動していない〔指示〕)。作業者 mt3、2026-11-06 JST。依頼: PM(0303)。入力: alpha1 の 0282 §『f_863f60』・0296 §9–§10、opt1/alpha1 の node harness(`lift/harness/mt2/node-bundle-run-sh.mjs`、G3 の `ref1-perf-short.g3`、仮想時計)、E11 の試験用 bundle(`lift/out-opt2/tw-e17/bundle/fl-test-e17`、`engine.abecc01c7b30.wasm`)。区分: **実測**=この作業の出力を見た値、**読み**=資料・コードを読んだ推論、**未確認**。

## 0. 冒頭の三点(正直さ)

1. **何を測ったか(実エンジン)**: (a) E11 の試験用 wasm を node 24.19(V8 13.6)で、G3 の仮想時計のスクリプト(起動 + 参照プロジェクトを開く + 再生 10 仮想秒 = 40.946 仮想秒)で走らせ、V8 の tick profiler(`--prof`: 各 tick の pc)と、**同じ run の** TurboFan の機械語(`--print-wasm-code`)を突き合わせて命令単位の標本を取った。(b) 同じ G3 スクリプトを native N1(lift 済みの C を clang -O2 で build したもの)で走らせ、SIGPROF の標本(`LD_PRELOAD` の shim)で命令単位と呼び出し元つきの標本を取った。**Chrome での測定は 0 件**(指示)。Chrome の値は 0282/0296 の引用。
2. **`perf` は使えない**: `kernel.perf_event_paranoid = 4`、`perf record -e cpu-clock` は『Failure to open any events』。代替: V8 の `--prof`(tick に pc があり、`code-creation` の範囲と、`--print-wasm-code` のアドレスが同じ run で一致する)と、N1 側は SIGPROF の shim(`tools/prof_shim.c`)。PMU が無いので**キャッシュミス・分岐予測ミスの直接計測は 0 件**(命令の skid からの読みだけ)。
3. **見つからなかった物**: 『wasm が native より遅い』原因は、node では**見つからなかった**(そもそも遅くない、§4)。1.47 倍が出た Chrome の測定を再現できていないので、Chrome で遅いなら**その理由は未特定**(§5 の仮説は未検証)。

## 1. 結論

1. **node(V8 13.6)で、E11 の `f_863f60`(TurboFan。accessor・型検査・述語の一部を V8 が inline した body)は 60.4 ms/仮想 s(2 run: 60.2 / 60.5)。native N1 の同じ走査(`f_863f60` + 呼ぶ `f_410010`・`f_6113f0`・`f_40fe90`・`f_50aa70`・`f_863f20`・`f_611430`、スタック標本で『`f_863f60` の下』と確認した分)は 150.7 ms/仮想 s。wasm は N1 の 0.40 倍の時間(2.5 倍速い)。仮想 CPU の呼び出し先の仮想関数〔`f_5287c0` ほか〕まで含めた包含では 89.5 対 173.7(0.52)**。同じ UI の別の大関数 `f_5287c0` は 18.4 対 18.0(1.02)。= ミキサー(0296 §9: wasm は N1 の 0.43〜0.49)と同じ向き。
2. **『1.47 倍遅い』は比較の取り方で出た値**: 0282/0296 は N1 の 83.6 ms/仮想 s(= `f_863f60` 自己 + 直下の accessor ぐらい。私の同じ定義の値は 81.5)に、Chrome の `f_863f60.body` 71 ms/壁 s ÷ v 0.575 = 123 ms/仮想 s を並べて 1.47 とした。(i)Chrome の値は同じ関数で node の 1.3〜2.0 倍(§5)で、**`f_5287c0` も同じ倍率**(関数に固有でない)、(ii)N1 は『native x86』でなく **C 経路の lift(x86 のフラグを全部 `FL[]` の byte に書く)**で、型検査の 1 hop が58 命令(wasm の直 ABI は 17 命令、元の x86 は 7 命令)。N1 は『下限』ではない。
3. **命令単位の内訳(node、TurboFan 機械語 2,768 B、922 命令、標本 2,479)**: 時間の **60.8% は +0xa20〜+0xb60(319 B、79 命令)の 1 つのループ** — V8 が `f_40fe90`→`f_410010`(Delphi の `InheritsFrom`: クラス表の親を辿る `cls == target ? : p = [cls-0x78]; p == 0 ? : cls = [p]`)を `f_863f60` に inline して 2 回展開した物。**標本は『ロードした値の最初の使用者』に乗る**(ロードの直後の命令 48.1%、ロード自身 11.5%)= **依存ロードの鎖(ポインタ追跡)の待ち**。spill/reload 5.2%、call/ret 0.1%(全部 inline)、v8 のループ戻りの stack-limit 検査 4.6%。
4. **種類ごと(PM の問い)**: ① メモリ範囲検査: 命令としては無い(guard 領域)。ただし**上位 32 bit の trap 検査**(`mov; shr 32; test; jnz`)が 1 回のロードごとに 4 命令。② 間接呼び: この body には無い(全 inline)。③ spill: 5.2%。④ i64 の分割: 無い(x64)。⑤ 分岐の質: jump 7.1%、ループは 2 回展開済み。⑥ 依存ロードの待ち: 約 60%。
5. **fe-wasmgen 側の案(分類 A、同じ意味の別実装)**: 上位 32 bit の検査を `(x >> 32) != 0`(shr; wrap; br_if)から `x >u 0xFFFFFFFF`(`i64.const 0xFFFFFFFF; i64.gt_u; br_if`)に。V8 の機械語は `mov; shr; test; jnz`(4)→ `cmp; jc`(2、定数はレジスタに巻き上げ)。**合成の微小試験(L1 に載る依存鎖。呼び出しをまたいで重なるので上限): hop あたり 0.79 → 0.59 ns(−25%)。実 harness(`f_863f60` の walk family 8 関数を差し替えた wasm): 自己 tick 2,472 → 2,400(−2.9%、2 run ずつ)、上位 40 関数を差し替えても 2,428(−1.8%)で、ミキサーの大関数〔`f_b34aca0` の body ほか、各 394 か所〕に測れる差なし**。= 実効は**小さい**(UI の `f_863f60` で約 1.5〜2 ms/仮想 s)。wasm の大きさ +238 KB(+0.17%、79,395 か所 × 3 B)。差分は `impl/0303-hi-check-form.diff`(`lower.rs` のみ、+12/−18、fe-wasmgen の試験 71 passed / 0 failed)。**E12 以降に入れる価値は小さい**(基準は動かす: wasm の sha・大きさ)。
6. **P2/P3 への含意**: `f_863f60` を『native 並みに下げる』梃子(0296 の L3、見込み 10〜40 core-ms/s)は、**N1 並みを超えて既に速い**ので存在しない。残りの的は FL 自身の仕事(UI の毎 tick の走査と、子ごとの型検査の鎖)で、lowering の質では動かない。FL の挙動を変える最適化(型検査の結果の cache など)は『余計なロジック』で採らない。

## 2. 方法(再現)

- **node 側**(`tools/run-node.sh`、`tools/codefilter.py`、`tools/v8ticks.py`、`tools/v8incl.py`): 0296 の harness と同じ呼び出し(`node --experimental-wasm-jspi --max-old-space-size=8192 node-bundle-run-sh.mjs BUNDLE seed.flpack ref1-perf-short.g3 OUT FLE_FILEDLG_PICK=…`)に `--prof --no-logfile-per-isolate --logfile=v8.log --print-wasm-code` を足す。`--print-wasm-code-function-index` はこの node では**効かず**全関数の機械語を出す(最初の試行で 420 MB を出して止めた)ので、標準出力を `codefilter.py` に通して対象の関数のブロックだけを残す。**同じ run の `code-creation`(範囲)と機械語(アドレス)から pc → 命令を引く**。`tick` の pc は 1 ms 間隔のメインスレッド(Worker の isolate も同じ log に入る)。関数の索引は E11 の試験用 wasm の body 索引: `f_863f60` = `wasm-function[1369]`、`f_863b50` = 1367、`f_863f20` = 1368、`f_6113f0` = 710、`f_611430` = 711、`f_40fe90` = 119、`f_410010` = 123、`f_50aa70` = 456(`a18-mk-bodymap.py` を試験用 wasm に当てた。0296 の `tables/0296-bodymap-e17.tsv` は別 wasm で索引が 2 ずれている)。
- **N1 側**(`tools/build303.sh`、`tools/run303.sh`、`tools/prof_shim.c`、`tools/natticks.py`、`tools/natincl.py`): alpha1 の gen3 の lift 済み C + t12h の runtime を clang -O2 で build(0295 の build95.sh と同じ recipe、約 1 分)。`LD_PRELOAD=prof_shim.so` が `setitimer(ITIMER_PROF, 1 ms)` と SIGPROF の handler で割り込まれた RIP を file に追記する。2 版: pc だけ(prof2、24,359 標本)と、**スタックの先頭 2 KB を `process_vm_readv` で読み、`f_863f60` / `f_863b50` の範囲への戻り番地があれば flag**(prof3、29,981 標本。この走査で CPU が約 23% 増えるので、**絶対値は prof2、帰属の割合は prof3**)。`nice`/`taskset` にも LD_PRELOAD が付くので、shim は `/proc/self/comm` が `PROF_COMM` の program でだけ timer を張る(最初の版は timer が exec をまたいで SIGPROF で殺された〔rc 155〕)。
- **G3 の入力**: node と N1 で同じ `ref1-perf-short.g3`(40.946 仮想 s)。N1 には `FLE_FILEDLG_PICK=C:\Users\user\Downloads\serina1-serum2\serina1-serum2.flp` が要る(無いと 8 s で終わる)。
- **仕事量の同一性の確認**(実測、`tables/parity-*-g3-progress.txt`): 間接呼びの数 `progress` が、UI だけの区間の終わり(`idle30`)で node 189,174,719 対 N1 189,942,262(差 0.4%)。再生の終わり(`play_b`)は 389.1 M 対 366.1 M(差 6%、ミキサー側の差)。= **UI の仕事は同じ**として、ms/仮想 s で比べる。

## 3. node: 命令単位の内訳(`tables/node-f_863f60-turbofan-ticks-annotated.tsv`、`node-f_863f60-turbofan-code.txt`)

TurboFan の `f_863f60`: 命令 2,768 B(922 命令)、標本 2,479(prof2)。領域別(関数先頭からの offset):

| 領域 | 標本 | 割合 | 中身(読み) |
|---|---|---|---|
| +0x000〜+0x400 | 217 | 8.8% | prologue・初期化・スタック検査 |
| +0x400〜+0x600 | 508 | 20.5% | 可視 byte `cmpb [r15+rdi],0`(+0x445 だけで 6.2%: 制御オブジェクトの初回の触り)、子の数と i 番目の子の取得(accessor の inline) |
| +0x600〜+0xa20 | 169 | 6.8% | 述語の呼び出しの準備・戻り |
| **+0xa20〜+0xb60** | **1,507** | **60.8%** | **inline された型検査の鎖(`InheritsFrom`)。79 命令、2 回展開** |
| +0xb60〜 | 78 | 3.1% | 返り値・後始末 |

命令のクラス別(922 命令のうち):

| クラス | 命令数 | 標本 | 割合 |
|---|---|---|---|
| cmp / test / shift(多くは**ロードの直後の最初の使用者**) | 130 | 1,430 | 57.7% |
| ゲスト memory のロード/ストア | 62 | 312 | 12.6% |
| reg move / imm | 186 | 183 | 7.4% |
| jump / branch | 137 | 175 | 7.1% |
| stack slot(spill / reload / frame) | 198 | 128 | **5.2%** |
| stack-limit 検査(`cmpq rsp,[r13-0x60]`、ループの戻り辺) | 8 | 113 | 4.6% |
| arith | 80 | 77 | 3.1% |
| zero-extend(`movl r,r`) | 44 | 59 | 2.4% |
| call / ret | 39 | 2 | **0.1%** |

skid の読み: ゲスト memory のロードの**直後の命令**に 48.1%、ロード自身に 11.5% = **約 60% が依存ロードの待ち**。上位 10 命令は全てループの `cmpq r14,rdi` / `testq r14,r14` / `shrq rdx,32`(ロードの直後)。

ループの機械語(1 hop = 17 命令、ロード 2 回。`tables/node-loop-excerpt.txt`):

```
      +a40  movl r14,r14                     ; ゲストの 32 bit アドレスへの zero-extend
   2  +a43  movq r14,[r15+r14*1]             ; ロード A
 119  +a47  cmpq r14,rdi                     ; cls == target ?   ← ロードの最初の使用者(標本が乗る)
      +a4a  jz   found
      +a50  subq r14,0x78                    ; cls - 0x78
   4  +a54  movq rdx,r14
  76  +a57  shrq rdx,32                      ; 上位 32 bit の trap 検査: 4 命令
      +a5b  testl rdx,rdx
      +a5d  jnz  trap
   1  +a63  movl r14,r14
   1  +a66  movq r14,[r15+r14*1]             ; ロード B  (p = [cls - 0x78])
 102  +a6a  testq r14,r14                    ; p == 0 ?
      +a6d  jz   notfound
      +a73  movq rdx,r14
  66  +a76  shrq rdx,32                      ; 検査
      +a7a  testl rdx,rdx
      +a7c  jnz  trap
      ...                                    ; 次の hop(cls = [p])
```

## 4. native N1 との並置(`tables/n1-samples.txt`、`n1-f_410010-x86.txt`、`flengine-f_410010-original-x86.txt`)

| 項目 | node の wasm(E11 試験用) | native N1(C 経路の lift、clang -O2) |
|---|---|---|
| `f_863f60` の自己 | **60.4 ms/仮想 s**(inline された accessor・型検査を含む。2 run: 60.2 / 60.5) | 35.6(自己だけ) |
| 呼ぶ関数の分(N1 で別の関数) | (inline されて自己に含まれる) | `f_410010` 55.8、`f_6113f0` 17.9、`f_40fe90` 15.6、`f_50aa70` 13.5、`f_863f20` 11.2、`f_611430` 1.0(スタック標本で『`f_863f60` の下』の割合を掛けた値: 88%・91%・89%・90%・99%・75%) |
| 走査の合計(仮想関数の呼び先を除く) | **60.4** | **150.7**(prof3 で `f_863f60` を含むスタックの標本の割合を prof2 の絶対値に掛けた) |
| 仮想関数の呼び先まで(`f_5287c0`・`f_5d76b0`) | 89.5(`f_863f60` の包含 3,731 / 3,602 標本 ÷ 40.946 s) | 173.7 |
| 比(wasm / N1) | | **0.40**(自己側)/ **0.52**(包含) |
| 参考: `f_5287c0` | 18.4(792 / 713 標本) | 18.0 → **1.02**(alpha1: 1.06) |

型検査の 1 hop の命令数: 元の x86 `f_410010` = 7(cmp/je/mov/test/je/mov/jmp)、**N1(C 経路)= 58**(x86 のフラグ 6 個を `FL[]` の byte へ全部書く: `setb [FL]`、`mov [FL+2]`、`shr`/`xor`/`not` の連鎖など。標本は全部その算術に乗る: `tables/n1-f_410010-ticks.tsv`)、**wasm の直 ABI = 17**。N1 が遅いのは C 経路の lift の形(フラグの実体化)で、**直 wasm のほうが実際の x86 に近い形**。= N1 は『これ以上速くならない下限』ではない。

## 5. 『1.47 倍』の再検討(読み + 実測)

- 0296 §9 の表: `f_863f60` = N1 83.6 ms/仮想 s(0282)、wasm 123(71 ms/壁 s ÷ v 0.575、E7b と同じ)→ 1.47。0296 の E11 の窓の実測は `f_863f60.body` 1,419 ms/20 s(7.393 ms/ブロック × 192.0 ブロック)。
- この 1,419 ms/20 s を**ブロック(音声)で正規化すると**(192.0 ブロック × 4096 ÷ 44100 = 17.83 仮想 s)79.6 ms/仮想 s。v 0.575 で正規化すると 123。**どちらの正規化が UI の仕事量に当たるかで 1.5 倍違う**(未解決: UI の tick は仮想時計の DwmFlush の周期に従うので v のほうが理屈に合うが、v の測定窓が `f_863f60` の窓と同じかは未確認)。
- node の 60.4 ms/仮想 s に対し Chrome は 1.32 倍(音声正規化)〜 2.04 倍(v 正規化)。**`f_5287c0` は Chrome 2.345 ms/ブロック → 25.2(音声)/ 39.1(v)ms/仮想 s、node 18.4 で 1.37 倍 / 2.1 倍 = `f_863f60` と同じ倍率**。UI の 2 大関数の**比** `f_863f60 : f_5287c0` は Chrome 3.15、node 3.28(N1 は 8.4)。= **Chrome の値が node より一様に大きい(関数に固有でない)**。原因の候補(**いずれも未検証**): Chrome 156 の V8 と node 24 の V8 13.6 の違い、DevTools の 2 ms cpuprofile の費用、Chrome の他のスレッド(SwiftShader・合成)との CPU 競合・SMT、正規化の v。
- よって『`f_863f60` だけが native より遅い』という前提は、node の測定では成立しない。**もし Chrome でも本当に遅いなら、それは lowering の質(wasm の命令列)ではなく Chrome の実行環境の側**(`f_5287c0` も同じ倍率)。

## 6. fe-wasmgen の案: 上位 32 bit の検査の形(分類 A)

- **意味**: `(x >> 32) != 0`(x が 64 bit のゲストアドレスで上位 32 bit が非ゼロなら trap)と `x >u 0xFFFFFFFF` は同じ真偽値。trap する条件・位置・回数は変わらない(FL の挙動は変わらない)。
- **差分**(`impl/0303-hi-check-form.diff`、`lift/crates/fe-wasmgen/src/lower.rs` のみ、+12/−18、main a977401d に `patch --dry-run` rc=0): 補助 `hi_nonzero()`(`i64.const 0xFFFFFFFF; i64.gt_u`)を足し、`i64.const 32; i64.shr_u; i32.wrap_i64; trap_if` の 6 か所(`hi_check`、`mem_addr` の 2 か所、間接呼びの 2 か所、string 命令の closure)を置き換える。2 つの値を `or` で合わせる箇所(string の bulk 検査)と、`shr 32` が値として使われる箇所(div の上位など)は触らない。
- **合成の微小試験**(`tools/gen.py`、`bench.js`、`tables/bench-12.txt`・`bench-3.txt`。InheritsFrom の鎖を wasm で書き、検査の形を変える。12 段の鎖を 4,000,000 回、node、3 回の中央値、**L1 に載る鎖で、呼び出しをまたいで鎖が重なるので命令数律速の上限**): 出荷の形 0.79 ns/hop、検査なし 0.48(−39%)、`i64.gt_u 0xFFFFFFFF` 0.59(−25%)、wrap の再利用(`extend_u(wrap a) != a`)0.59(−26%)、検査を hop あたり 1 回にまとめる 0.66(−16%)。3 段の鎖でも同じ順(0.51 → 0.35)。
- **実 harness**(上の 6 run、`tables/node-runs-summary.tsv`): `f_863f60` の自己 tick — 差し替え前 2,465 / 2,479、walk family 8 関数の差し替え 2,411 / 2,390(**−2.9%**)、上位 40 関数の差し替え 2,425 / 2,431(−1.8%)。ミキサーの `f_b34aca0` の body(各 394 か所を差し替え)は 1,128 / 1,218 / 890 / 935 → 1,092 / 1,143 / 945 / 961 で**揺れの内**(±5%)。全体の tick も load の揺れ(61〜65 k)に埋もれた。差し替え後の TurboFan の機械語は想定どおり `mov ecx,0xffffffff`(巻き上げ)+ `cmp; jc`。
- **見込み**: UI の `f_863f60` で約 1.5〜2 ms/仮想 s(UI の 0.15〜0.2%)。**入れる価値は小さい**。入れるなら基準を動かす(wasm の sha・大きさ +238 KB = 79,395 か所 × 3 B。fe-wasmgen の試験は 71 passed / 0 failed で動かない)。E13 の『基準を動かす変更の束』に載せる程度。
- **他に見つかった余地**: 無い。(a)spill は 5.2% で、V8 の割り当ては悪くない。(b)呼び出しは全て inline 済み。(c)ループ戻り辺の stack-limit 検査(4.6%)は V8 が入れる。(d)ロードの待ち(約 60%)はデータ構造(クラス表の親を辿る鎖)で、lowering で縮められない。FL の挙動を変えずに縮められるのは、同じ鎖を辿る回数を減らす FL 自身の変更だけ(採らない)。

## 7. 未確認・出典なしの選択・逸脱

- Chrome での再現(別の V8・環境)。PMU が無いのでキャッシュミス・分岐予測の直接計測。実 x86 の native(Wine。0296 §12.2 のメモのまま未実施)。
- node の `--prof` の標本は 1 ms の wall 間隔で、load 6〜9 の機械で取った(run 間の `f_863f60` の揺れは 0.3〜1%、patched との差 −2.9% は 2 run ずつなので**弱い**)。
- N1 の帰属(スタックの先頭 2 KB に戻り番地がある標本)は保守的な走査で、2 KB の外にある呼び出し元は取りこぼす(`f_410010` 88%、`f_611430` 75% など。残りは別の呼び元か取りこぼし)。N1 の絶対値は prof2、割合は prof3。
- N1 は gen3(older lift)+ t12h の runtime で、E11 の C 経路ではない。UI の仕事の同一性は `progress` で 0.4%(UI だけの区間)まで確認した。
- 合成の微小試験は命令数律速の上限で、実関数の実効(−2.9%)とは桁が違う。

## 8. 方針との適合(必須の記載)

- **lift の忠実性**: 差分案は (a)余計の除去でも (b)契約の実装でもなく **変換/通信の削減に近い『同じ意味の別実装』(分類 A)**で、trap の条件・位置・回数は不変。FL の挙動を補う・代行する処理、class/アドレスの分岐は無い。型検査の結果の cache など FL の仕事を省く案は『余計なロジック』なので採らない。
- **no-options**: 旗・環境変数・オプションの追加は無い。
- **進捗の誠実性**: 代役・再生・推測を『進捗』として出していない。実エンジンの測定(node の E11 試験用 wasm、native N1)と、Chrome の引用(出典 0296)を分けて書いた。

## 9. 自己申告(0303 の区間)

- 最初に `wasm-function[1367]` を f_863f60 と思い込んで走らせた(0296 の body 索引は別 wasm のもの。試験用 wasm では 1369)。tick の上位に出た [1369] で気づき、`a18-mk-bodymap.py` を試験用 wasm に当て直して再走した(1 run 分の無駄)。
- `--print-wasm-code-function-index` が効かず 420 MB を出したので止めて、filter を挟んだ。N1 の最初の run は shim の timer が `nice`/`taskset` を殺し(rc 155)、次は `FLE_FILEDLG_PICK` 無しで 8 s で終わった。どちらも直した。
- 合成の微小試験の最初の想定(検査が hop の 36〜45%)は、呼び出しをまたぐ重なりで実関数の効きを大きく見せていた。実 harness で測り直して −2.9% と報告した。
- 禁止された `sleep 0` と末尾の `&` を含むコマンドを 1 回出して拒否された。
- bundle93 の再生成(PM の 17:08 の依頼)を 0303 の途中で挟んだ(完了済み。sha は元と一致)。
- 作業 dir: `~/work/mt3w-prof303`(run と patched wasm)、`~/work/mt3w-n303`(N1)。最後に消す。


## PM の判断と取り込み(2026-11-06 18:12、docs のみ。製品の差分なし)

取り込み: mt3 の第 1 報(文書・表・道具・差分 1 本)を `docs/investigations/mt3w/0303/`、差分を `docs/investigations/mt3w/patches/0303-hi-check-form.diff` に置いた。実行は node 24.19(V8 13.6)で E11 の試験用 wasm(`engine.abecc01c7b30`)と native N1(lift した C を native 化した物。**実 x86 ではない**)を G3 の模擬入力で走らせた値と、`--print-wasm-code` の読み。**Chrome・実 Windows・依頼者の機械は 0 件**。

事実として受け取る(node の値):
1. `f_863f60` の TurboFan コード(2,768 B、922 命令)の 60.8% は、inline された `InheritsFrom` のクラス表の親をたどるループ 1 つで、標本は依存 load の直後の最初の使用に載る(load-to-use の待ち)。1 load あたりの上位 32 bit の検査 4 命令、spill 5.2%、ループ末の stack-limit 検査 4.6%、jump 7.1%。
2. node では `f_863f60` は native N1 の 0.40 倍の時間(wasm のほうが速い)。仕事量は同じ(間接呼び 189.17 M 対 189.94 M、差 0.4%)。**0282・0296 の『UI の `f_863f60` は N1 の 1.47 倍遅い』は、Chrome の値と N1 を比べた物で、node どうしの比較では成り立たない**(原因の分解は決定 0303b)。
3. 上位 32 bit の検査の class-A な別形(`(x>>32) != 0` → `x >u 0xFFFFFFFF`)は、合成では −25% だが、実の harness では `f_863f60` −2.9%、上位 40 関数 −1.8%、ミキサーに差なし、wasm +238 KB(+0.17%)。

判断:
1. **0296 の L3(UI の `f_863f60` を native 並みにする梃子)は撤回する**。根拠が 1.47 倍の比較にあり、node の比較では wasm のほうが速い。決定 0296 の末尾に訂正を書いた。
2. `0303-hi-check-form.diff`(lower.rs のみ +12/−18、fe-wasmgen 71 passed)は**採らない(保留)**。効果は −1.8〜−2.9% で任意、wasm が +238 KB 増える。E13 に lowering の変更が他に入るときに、効果と大きさを見て再検討する。
3. 未確認: Chrome での同じ差分の効き、PMU(cache miss)の値(`perf_event_paranoid=4` で取れない)、N1 の標本の帰属(スタックの上 2 KB の保守的な走査)。
