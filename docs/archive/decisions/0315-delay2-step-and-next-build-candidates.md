# 0315b(仮): Fruity Delay 2 を 3 番目の実モジュールに足す準備(0313f 型の差分)と、次の build(cand2 / RT-B)の候補表(mt3。PM の instruction 09:1x、期限 10:00)

状態: **仮**。区分: **実測**=この作業のコマンドの出力を見た事実、**引用**=他者の測定・文書、**読み**=コードや表を読んだだけ、**未確認**。**実エンジン(wasm)の実行は 0 件。Chrome・Wine は使っていない。** 実行したのは host の道具(PE の読み、`extract_fl.py`、`ci/test-hybrid.sh`、`patch --dry-run`、lift 段 1 回)だけ。

## 0. 実際に実行した物

1. 3 つのプラグイン DLL(PE パーサは `tools/pe-read.py`)(opt1 の Wine prefix の `Fruity Delay 2.off-0307/Fruity Delay 2_x64.dll` ほか)を **自作の PE パーサ(python、読むだけ)**で読み、Balance・Filter と比較した(§2)。
2. `ci/lib/extract_fl.py`(本物の抽出器)を installer `flstudio_win64_26.1.7.5653.exe` に対して、Delay 2 の行だけの `--dlls` と `--seed` で走らせた: **『1 DLL + 1 seed file written, all SHA-256 verified』rc 0**(2 つの経路: DLL を名前で探す経路と、seed の guest path を NSIS の install tree で引く経路の両方)。
3. 差分 `impl/0313g-delay2-on-filter.diff`(9 ファイル、167 行)を作り、Filter の木(`tree-g9f-build`)のコピーに実適用し、`bash -n` 6 本 ok、`ci/test-hybrid.sh` **all checks hold**(Delay 2 の新しい 2 行と『NEXTRA = EXTRA_MODULES の名前の数 = gen-direct の spec − 5、RM_MAX ≥ spec』の検査を含む)。cand1 の木・RT-A の木(tree-rt1)にも `patch --dry-run` rc 0(RT-A の木では build-engine-bundle.sh に offset 14/22 行)。
4. **その木で lift 段を 1 回実行(50 s、nice 19、CI_JOBS=1)**: Delay 2 が module 10 として lift された(§3)。build(objects・link)は走らせていない。

## 1. 差分 `0313g-delay2-on-filter.diff` の中身(Filter の木の上。適用・push しない)

| ファイル | 変更 |
|---|---|
| `ci/extract-dlls.tsv` | 行 `Fruity Delay 2_x64.dll → lift/out-plugins/fruity_delay_2_x64.dll`、SHA-256 `c4fed60da5034d5c04019a4992563334aa6eb89e2adc211214aaca520ea928fc`(installer の物。extract_fl.py が検証した) |
| `lift/harness/seeds/vfs-seed-idle1.tsv` | guest `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Effects\Fruity Delay 2\Fruity Delay 2_x64.dll` を volume に置く行(同じ SHA-256) |
| `lift/harness/b2d2/gen-direct.sh` | spec 行: `fruity_delay_2_x64.dll`、**base 0x7800000**(Balance 0x6800000、Filter 0x7000000 の続き。SizeOfImage 0x3a5000 < 0x800000、0x7800000 以降は oggio 0xc000000 まで空き)、`--pdata` と共通の hook points、パス表の suffix `plugins/fruity/effects/fruity delay 2/fruity delay 2_x64.dll` |
| `app/scripts/build-engine-bundle.sh` | `EXTRA_MODULES="… fruity_filter fruity_delay_2"; NEXTRA=5`、コメントの数。**hyb_strip の glob を `m[0-9][0-9]-NN.c` にも広げ、名前が file でない物は skip**(10 番目の module の unit は `m10-NN.c` で、一桁の glob は拾わない。Delay 2 は hybrid に入らないので strip の対象は無いが、数の検査〔`dropped` = lowered-all の行数〕を壊さないため) |
| `ci/objkeys.sh` | m5 の群の glob に `m10-NN.c` を足す(`m[6-9]` は m10 を拾わない = cache key から Delay 2 の C が漏れる所だった) |
| `ci/test-hybrid.sh` | Delay 2 の extract 行・seed 行の検査 2 行 |
| `ci/stage-bundle.sh`・`ci/stage-lift.sh`・`hooks/fruity-delphi-effect-hook-points.txt` | コメント(11 ファイルの DLL dir、10 module)。hook points は 3 つの RVA が Delay 2 でも同じバイトであることを PE から確認した旨 |

**runtime の変更は無い**(RM_MAX = 16 で k = 10 は収まる。cursor の共有 handle の上限 16 は 0313f2 が入れた物で、**Delay 2 はそれに依存する**: 0313f2 なしでは Filter と同じ DestroyCursor の stop に落ちる)。`build_nmod_wasm.sh:77` の `m[0-9]*-NN.c` は m10 も拾う(読み)。

## 2. Delay 2 の PE(実測、PE パーサ)

| | Balance | Filter | **Delay 2** |
|---|---|---|---|
| file の大きさ | 3,718,656 | 3,780,608 | **3,731,968** |
| SizeOfImage | 0x3a1000 | 0x3b2000 | 0x3a5000 |
| entry RVA | 0x2c13a0 | 0x2cdf20 | 0x2c3800 |
| `CreatePlugInstance` の export RVA | 0x2bdde0 | 0x2cb670 | **0x2bf1a0** |
| .pdata の関数数 | 11,736 | 11,737 | 11,760 |
| import の数(重複なし) | 505 | 505 | 505 |
| hook point の先頭 5 byte(RVA 0x17bc0 / 0x31d70 / 0x31d90) | 55 56 53 c6 44 / 53 48 83 ec 20 / 55 48 83 ec 50 | 同じ | **同じ** |

- **import の集合は 3 つの DLL で完全に同一**(`balance == filter`、`delay2 == balance`、差 0)。export も同じ 3 つ(`CreatePlugInstance`・`__dbk_fcall_wrapper`・`dbkFCallWrapperAddr`)。= **runtime に行が無い import の集合(Balance・Filter で『未対応 thunk 44 個、実行された物 0』と数えた物。ImageList_*・DestroyCursor 型を含む)は Delay 2 でも同一**で、**新しい穴は静的には増えない**(読み。Delphi の VCL が静的に入っているため。**実行時に通る API が同じかは未確認**: Delay 2 の GUI が使うコントロールが違うので、stage 3 の run で確かめる)。
- 手法の検証: Balance は 0x6800000 + 0x2bdde0 = **0x6abdde0**、Filter は 0x7000000 + 0x2cb670 = **0x72cb670** で、どちらも実 Chrome の g9-diag の観測値(Balance 〔gpu1 の V3〕)/ 予測(Filter)と一致する形。

## 3. lift 段の結果(実測、50 s)

- module 10 `fruity_delay_2_x64.dll`(base 0x7800000): **4 units、C 36,889,647 B、lift した関数 12,573**(Balance 12,549 / 36.63 MB、Filter 12,550 / 37.36 MB)。realmod の表は **10 行**、パス表は **3 行**(`…\fruity delay 2\fruity delay 2_x64.dll → fruity_delay_2_x64.dll`)。DLL dir は `fruity_delay_2_x64.7800000.dll` を含む 10 個(FLEngine を足して 11)。
- 他の module の lift の C は変わらない(新しい module は `common.c` と表だけを変える。読み: m1〜m9 の sha は未比較〔この lift は新しい work dir のため〕)。

## 4. V3 の予測値(Balance 0x6abdde0、Filter 0x72cb670 と同じ形。読み、未確認)

- `LoadLibraryW "…\Fruity Delay 2\Fruity Delay 2_x64.dll" -> 0x7800000`
- `GetProcAddress(0x7800000, "CreatePlugInstance") -> 0x7abf1a0`
- Problems 窓の件数: Balance 8 → 7、Filter 7 → 6 の続きで **6 → 5**。opt1 の Wine の参照(elog17 = config A + Delay 2)の『Problems 窓 5 件』と合う〔引用〕。Problems 窓の高さは Filter で 16 px ずつ縮む形(Balance 691×343 → 327)の続きで 691×295 前後(**読み**、画面で未確認)。
- Export の gate: Wine の elog17 は 5,715,911 フレーム・RMS 20433.0423、elog16 との差 RMS 1040.5284〔引用〕。wasm の Delay 2 の Export を ci1 が Wine と比べる(Balance は ±1 LSB で 5 フレーム、Filter は未)。

## 5. S1(wasm と native の比)の予測(読み。実 build の値ではない)

引用: Balance の増分 +7,763,869 B(その module の native 3,718,656 → 2.088)、Filter +7,587,106(3,780,608 → 2.007)。lift の C は Balance 36.63 MB で +7.76 MB、Filter 37.36 MB で +7.59 MB(C の大きさに単調には比例しない)。

| | wasm | native の合計 | (i) 全体の比(正式) | (ii) module 単体 | (iii) pack 増分(raw) |
|---|---|---|---|---|---|
| Filter bundle(実測) | 158,438,681 | 84,942,208 | 1.865 | 2.007 | +7.56 MB |
| **+ Delay 2(予測)** | **約 166.1 MB(+7.6〜7.8 MB)** | 88,674,176 | **約 1.873(1.872〜1.875)** | **約 2.04(2.03〜2.08)** | dll pack と seed に +3.73 MB ずつ |

- 追加の module ごとの限界の比は約 2.05 で、全体の比 1.87 より大きいので、**module を足すたびに全体の比は 2.0 へ向かって上がる**(式: (158.44 + 7.7 n) / (84.94 + 3.74 n) = 2.0 になる n は約 52。7 本ほどなら約 1.89)。B-FLE の方式 B(共有本体)が入れば 2 本目以降は約 1.1 MB(0312g〔読み〕)。
- cand1 の 1.966 は strip の印の誤りによる(別報)。strip した版の cand1 は約 1.866。

## 6. 次の build(cand2 / RT-B)の候補の表

**build の費用の型(実績の引用に基づく読み)**: **A** page のみ = engine の build 無し。**B** seed 行・manifest だけ = `seed`+`bundle` 段(数分、wasm は再 link しない)。**C** runtime の C のみ(rt 群)= objects は rt 群だけ(数十秒)+ link(wasm-opt を含む約 30 分): B2 の実績(objects 15 s、link 20 分 13 秒、b2d 54 s、lift 26 s)。**D** `ship6-imports.txt` に行を足す = lift の C が全 unit で変わる(cand1 で実測: 61 ファイル)→ **全 lifted objects の再 compile(約 28〜30 分)+ C の費用**= 約 60 分。**E** `imports_table.rs`(fe-lift の crate)に行を足す = felift の再 build + lift やり直し + hyb/c-imports の再生成(link 段の hyb-gen)= D と同じ約 60 分以上。**F** 新しい module = lift は 50 s、新 module の objects(約 7 分)+ rt 群の `common.c` + link(約 30 分)= 約 40 分(他の module の lifted objects は再利用できる。ただし E/D と同時なら全部)。

| 候補 | 中身(場所) | 依存 | lift/hyb の再生成 | imports_table.rs の append | ship6-imports の append | 型 |
|---|---|---|---|---|---|---|
| **HiDPI v2**(gpu1) | `0318-hidpi-runtime-v2.diff`(runtime C 6 + harness 5〔うち `hyb/c-imports-engine.txt`〕+ `imports_table.rs`、12 ファイル)+ rust tests v2 | page 側 `0318-hidpi-page`(+ test)、**0319 の page 積み順の後**。dpr = 1 で現在と同じ値(96)を返すこと(Export の sha を ci1) | **要**(imports_table の行で lift が変わる。`hyb/c-imports-engine.txt` の変更で hybrid の export 表も再生成) | **要(3 行。他の append diff より最後に積む: gpu1 の注)** | — | **E** |
| **日本語フォント**(資材、opt1) | msgothic.ttc(Noto Sans CJK JP の subset、2,123,288 B、br 1,135,949 B)を `C:\Windows\Fonts` に | 2 通り: (a) seed の eager な行 = **型 B**(seed に +2.1 MB raw / 約 1.1 MB br。起動の取得量が増える)、(b) 遅延 = **lazy VFS V1 が前提**(下)+ 2 つ目の索引(root `C:\Windows\Fonts`、128 B)。フォントの選択・fallback の契約(QuickFontCache が見る名前、固定 10 名)は alpha1 の読み | 不要 | 不要 | 不要 | (a) B / (b) lazy VFS に従う |
| **DCX_LOCKWINDOWUPDATE**(alpha1) | `0319-dcx-lockwindowupdate.diff`(fle_win.c・fle_win.h・fle_gdi.c、61 行) | `fle_win.c`/`fle_gdi.c` を触る他の diff(scroll-model、scrolldc、0323、0325)と文脈が近い。順序は alpha1 の README | 不要 | 不要 | 不要 | **C** |
| **ImageList の drag 系**(alpha1) | `0319-imagelist-drag.diff`(659 行: `fle_imagelist.c/h`〔新規〕・`fle_gdi.c/h`・`ship6-imports.txt`) | 私の 0313 §11(22 API・5 層の設計)の一部の実装。Balance/Filter/Delay 2 の『未対応 thunk 44』の ImageList_* を埋める(drag の 4〜5 本)。GUI の client が描かれない問題(0313 §15)とは別の可能性(§15 の表で切り分け中) | **要**(import 行で lift の C が変わる) | 不要(ship6) | **要(行を足す)** | **D** |
| **0319 の page 積み順**(alpha1) | `0319-page-wall-clock-env` → `0319-page-num-cpu` → `0318-hidpi-page` → `0318-hidpi-page-test` | `screen-fit.test.ts` が runtime の `fle_screen.h` を読む(HiDPI の runtime と同じ push)。`FLE_NUM_CPU = clamp(hardwareConcurrency − 1, 2, 8)` をページが engine.env に渡す | 不要 | 不要 | 不要 | **A**(ただし page の build/deploy が要る) |
| **診断の整理**(私・alpha1) | diag-g9-fl / diag-g9-fl-scroll / diag-g9-hook / `0319-fle-stop-context` を、(製品に残す物)と(消す物)に分ける。fle_stop-context は『stop は元から致命』の診断で製品に残す案 | 何を残すかは未決。`FLE_DIAG_*` を getenv で読む sed(cand1/RT-A の木だけ)は製品に入れない | 不要 | 不要 | 不要 | **C**(消す側は rt 群の縮小) |
| **Delay 2**(私、本メモ) | `0313g-delay2-on-filter.diff` | **0313f2 が前提**(cursor の k ≤ 16)。0313-g9a+0313f の積みの上 | **要(新 module のみ。他 module の lift は同じ)** | 不要 | 不要 | **F**(D/E と同時なら全体に吸収) |
| **lazy VFS V1**(私) | `0314v1-lazy-vfs-{runtime,host}.diff`(fle_vfs.c/h、vfs_lazy.h、vfs_seed.h、TS 6 本 + hostfiles.ts) | **engine への配線が未**(worker.ts/fruity.ts が `mountLazyTree` と I/O Worker を起動し、manifest が遅延 tree の assets と索引の sha を持つ)、R2 への chunk の配置(opt1 の V0.1 の出力、約 940 MiB)、roots.json(複数 root: 日本語フォントの 2 つ目の索引)。V2(OPFS・R2・Chrome)は未 | 不要(imports なし) | 不要 | 不要 | **C**(`vfs_seed.h` は lift/harness にあり H_WASM を変える = rt 群と h-fruity が再 compile)+ **page** |
| **num-cpu の ship6 の 6 行**(alpha1) | RT-A では外した(呼び手が無い。契約の整合のための行) | — | 要(入れるなら) | — | 要(6 行) | **D** |
| **S3b / S4**(私) | `FLE_WORKERS`/`FLE_PROCESSORS`/`FLE_NUM_CPU` の env の除去、協調 scheduler と `rt_mode` の整理 | n の掃引と RT-A の結果の後(未決) | 不要 | 不要 | 不要 | **C** |

**まとめ方の案(読み。判断は PM)**:
- **imports に触る候補(HiDPI v2〔E〕、ImageList drag〔D〕、num-cpu の 6 行〔D〕、Delay 2〔F〕)は 1 回の冷間 build にまとめる**(RT-B。約 60〜65 分、`GIL=1`)。個別に足すと、そのたびに全 lifted objects の再 compile(約 28 分)が要る。HiDPI は imports_table の append が他の append diff の**最後**(gpu1 の注)、ship6 は scroll-model → scrolldc → imagelist-drag → num-cpu の順(append only、位置 = 擬似 VA)。
- **runtime のみ(C 型)の候補(DCX、診断の整理、lazy VFS V1 の engine 側)は、RT-A の work dir の増分 build で約 30 分**。
- **page のみ(A 型)**: 0319 の page 積み順 + E14(P+V7)+ 0325 の page 側(`ui/cursor.ts`)。engine の build に依らず、RT-A の bundle に対して作れる。
- **日本語フォントは seed の eager な行(B 型)が最も安い**(再 link なし)が、起動の取得量が増える(seed +約 1.1 MB br)。遅延(lazy VFS)にするなら V1 の配線が先。
- RT-A の結果を見る前に決められること: Delay 2 の差分は完成していて、他の imports 候補と一緒に RT-B に入れられる。RT-A が実スレッドで致命的に失敗したら、cand2 は協調の木(cand1 の strip 済み)の上に同じ候補を載せる(型は同じ、`GIL` なし)。

## 7. 自己申告

1. 最初の木 `tree-g9f` は `tools/` を持たないので `extract_fl.py` が動かなかった(import error)。`tree-g9f-build` のコピーでやり直した(3 分の無駄)。
2. 本メモの S1 の予測は、Balance・Filter の 2 点の増分(2.088 / 2.007)からの外挿で、C の大きさに単調には比例しない(Balance 36.63 MB → +7.76 MB、Filter 37.36 MB → +7.59 MB)ので範囲で書いた。実 build の値ではない。
