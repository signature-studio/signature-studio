
## 追記(ref2、2026-11-04 17:00〜19:30): 再生・アイドルの CPU の内訳、周期の起点、混成の候補、サイズ・heap、Chrome

**何を測ったか**: 参照プロジェクトを開いた状態の実エンジンを、(1) native -O2(API 呼び出し回数の計数 `--wrap`、1 ms の CPU サンプラー〔ゲストスレッド番号つき〕、Win32 API の戻り番地)、(2) wasm〔node JSPI、`--cpu-prof` 1 ms、名前つき wasm(wasm-opt -g)〕、(3) Chrome 156 ヘッドレス(本物の Worker、実時間)で実行した。入力は全部ハーネスの模擬(G3 / CDP)。この機械(16 コア、SwiftShader)、load 1〜16。依頼者の機械ではない。
**結論**: (a) 再生・アイドルの CPU に『我々の余計な処理』と呼べる大きな物は runtime 層に見つからなかった。アイドルの 94.6% はリフト済みの FL 自身のコード、runtime の Win32 API 本体+スケジューラは 1.4%。(b) 60 Hz の周期(62.5/s)は**ゲスト自身のスレッドの `Sleep(16)` → `PostMessageW(0x8ffc)`**で、runtime が起こしているのではない。(c) 再生中のプレイリスト全面再描画(882×963 を 51 回/s)はゲストが作った領域を `RedrawWindow` に渡している。runtime が更新領域を広げた形跡は無い。(d) P3 は『リフト後コードの速さ』の不足で、API の重さではない。出荷構成の wasm(w2)は node で再生 3.70・アイドル 0.461 CPU 秒/仮想秒(プロファイルなし)(native 1.5〜1.7・0.29〜0.34)、Chrome 156 で再生中の窓の更新は 7.3/s、first-idle 7.5 s。非出荷構成(ref1 が作った wasm)は再生 7.4・アイドル 1.86。
**未確認**: 実機 Windows の FL が同じ回数の全面再描画をするか。ゲストが全面再描画を決める直前に読む値(戻り値の違いで余計に描いていないか)の切り分け。混成 wasm の実測(候補リストだけ)。heap の 1.08 GB の内訳。brotli のサイズ。Firefox での再測定。

### 層別の CPU(native、`tables/cpu-profile-native-c5.txt`、CPU 秒/仮想秒、load 8〜14)
| 層 | アイドル 30 s | 再生 30 s |
|---|---:|---:|
| 合計 | 0.292(0.30〜0.34) | 1.514(1.51〜1.74) |
| ゲスト FLEngine | 94.6% | 50.4% |
| ゲスト Elastique / dsp_ippv2 / QFC / Blend2D | 0 / 1.0 / 0.1 / 0.1% | 9.2 / 5.9 / 3.1 / 0.9% |
| runtime: GDI 画素(BitBlt〔FAST_COPY なし〕) | 0.6% | 13.8% |
| runtime: x86 命令のエミュレーション(SSE/x87 の関数呼び出し) | 0.2% | 13.0% |
| runtime: Win32 API 本体+スケジューラ | 1.4% | 1.4% |
| スレッド別 | メイン 91.0%、ワーカー(スレッド 18)8.9% | メイン 85.2%(1.29)、ワーカー 14.6%(0.221) |

スレッド 18 は開始関数が `f_412ae0` の汎用ワーカー(17 本以上同じ開始関数で作られる)で、アイドルでは Sleep(1)+CS+SList の pop/push のポーリング、再生では Elastique を実行する。ミキサーのスレッドは特定できていない(ref2 の第 3 報の『ミキサー』は読みの誤りで撤回)。

### 60 Hz の周期と全面再描画の起点(`tables/callers-native-c8.txt`、`api-counts-native-c8.txt`、`paint-blit-native-c8.txt`)
- 62.5/s: ゲストのスレッド(開始関数 `f_1d0fdb0`)が `Sleep(16)`〔`f_1df1db0`〕→ `PostMessageW(0x8ffc)`〔`f_1e860a0`〕を回す(30 仮想秒で Sleep(16) 1,874 = PostMessage 1,874 = メインの DispatchMessage 0x8ffc 1,874)。DwmFlush・timeSetEvent は 0 回、WM_TIMER の配信は 0。runtime は全スレッドが待つとき仮想時計を最小の期限へ進めるだけで、メッセージを自分では投げない。
- 1 周期のメインの仕事: `SendMessageW(WM_USER+0xb03f)` 32 回〔`f_604f60`〕、`IsWindowVisible` 24 回・`IsWindowEnabled` 10 回〔`f_86b300`〕、`WaitMessage` 2 回〔`f_86b56e`〕、ヒント窓の追跡〔`f_604ea0`/`f_6074b0`〕。アイドルでは `RedrawWindow(TFruityLoopsMainForm, 1×870, RDW_INVALIDATE)`〔`f_11f0270`〕→ BeginPaint → BitBlt 1×870 が毎周期。
- Sleep(1) 962/s(アイドル、スレッド 18)・991/s(再生、スレッド 8): ワーカーの待ち行列ポーリング(`f_5264e0`)。
- 再生中: `RedrawWindow(Playlist〔TEventEditForm、1047×998〕, NULL, hrgn≠NULL〔ゲストが作った領域〕, RDW_INVALIDATE|RDW_NOCHILDREN)` が 103/s〔`f_820f10` から〕。hrgn が渡されるので矩形引数は無視され、BeginPaint の rcPaint = 882×963(51/s)。BitBlt の要求面積 1.558 G px と書いた面積 1.513 G px(50.4 Mpx/s)がほぼ一致。`IntersectRect` 1.13 M/s〔`f_6ce3f0`〕、`GetTickCount` 9,058/s〔`f_1e82290`〕、`CreateRectRgn` 1,209/s、BeginPaint=BitBlt=GetDC=ReleaseDC=EndPaint 7,146 回/30 s。
- 切り分けられなかった点: ゲストが全面再描画を出す直前に読む値。スタック走査の鎖は rbp が使えず信頼できない(S-ref2-callers-stack-scan)。

### wasm の V8 プロファイル(非出荷構成、`tables/cpu-profile-wasm-v8.txt`、node の主スレッド、load 5)
| 区間 | CPU 秒/仮想秒 | native 比 |
|---|---:|---:|
| アイドル 30 s | 1.856 | 5.5〜6.4 倍 |
| 再生 30 s | 7.402 | 4.3〜4.9 倍 |
| 開く 4.8 s | 5.803 | 4.8 倍(native 1.216) |

再生の層: x86 命令エミュレーション 37.9%(2.80、native 0.197 の 14 倍。`fle_ld` 16.1%・`fle_st` 4.3%・`fle_op_sub` 2.6%・`fle_cc` 2.0% が別関数として呼ばれる)、ゲスト FLEngine 37.7%(2.79、3.7 倍)、dsp 5.4%、QFC 4.3%、Elastique 4.2%、Win32 本体 3.1%(11 倍)、名前なし wasm 3.1%、ILTools 2.2%、GDI 画素 0.7%(FAST_COPY あり)。アイドルの層: ゲスト FLEngine 61.6%、x86 エミュレーション 20.1%、名前なし wasm-function[97676] 9.4%(wasm-opt が作った外出し関数)、Win32 本体 6.7%。
**この wasm は出荷構成ではない**(LOWREGS=0、AI_UNITS なし、SIMD ヘルパなし、混成なし)。出荷構成(F2xH3 相当の旗)での再測定は次の節で、そちらが主な数字。

### 混成に足す候補(`lift/harness/seeds/hyb-ref1-*.txt`、`tables/hyb-ref1-candidates.tsv`、`hyb-ref1-excluded.tsv`)
アイドル+再生の自己時間(native、1 ms サンプル 62,300 のうちゲスト 45,017)のうち HEAD の混成表(`hyb-p1000r-F1x-*.txt` + `hyb-sse2-F2x-*.txt`、1,009 関数)に無い関数の 95%(S-ref2-hyb-95)を覆う 242 関数: fle 141、Elastique 56、dsp 36、b2d 6、qfc 3。ゲスト自己時間の 34.4%。fe-wasmgen の出荷の旗(`--mx-window --trap-vex --no-elide --lazy-ext --hybrid-merge --static-calls --sync-saved --sse-int`)で lower できるのは 173(15.2%)=`hyb-ref1-{fle,elastique,dsp,b2d,qfc}.txt`。`--mx-packed --mx-rc`(決定 0181)を足すと 30 関数(12.3%)が lower できる=`hyb-ref1-mxpacked-*.txt`。どちらでも lower できない 39 関数(7.0%)の理由は `hyb-ref1-excluded.tsv`(x87 制御語が C 経路の状態 7、32 ビットアドレッシング 4、opaque な関数の呼び出し 5、Cvtps2dq 4、Cpuid 3、Fld 2、ほか各 1)。混成 wasm を実際には作っていない(出荷の混成パイプラインが 6 モジュール固定・パス固定で、Elastique・oggio を足す直しが時間内に終わらなかった)ので、効果の実測は無い。Elastique(ワーカー上の 62.8%)はモジュールごと表に無い。

### サイズと heap(`tables/wasm-size-breakdown.txt`〔非出荷構成〕、`wasm-size-breakdown-w2.txt`〔出荷構成〕)
wasm 148.66 MB(名前つき;名前なしは 147.1 MB)。コード 144.6 MB: FLEngine 58.8、dsp_ippv2 40.1、**Elastique 13.3**、ILTools 8.8、名前なし 7.6、**oggio 6.0**、QFC 4.1、freetype 2.3、Blend2D(リフト)1.4、upstream 1.2、runtime 0.7(MB)。oggio+Elastique の追加分は 19.4 MB(コードの 13.4%)。出荷構成では wasm 146.03 MB(名前つき 147.59 MB)、コード 143.55 MB: FLEngine 65.1、dsp 35.5、Elastique 10.6、ILTools 7.6、名前なし 6.6、QFC 6.0、oggio 4.8、freetype 4.0、Blend2D 1.2+1.2、runtime 0.7(MB)で、oggio+Elastique は 15.4 MB(10.7%)。heap: リンクの最大 2 GiB、初期 640 MiB、エンジン起動直後で 1,078,657,024 B、プロジェクトを開いて 60 仮想秒動かした後 1,110,900,736 B(51.7%)。このプロジェクトが足すのは約 32 MB で、残りはエンジン自身。native の最大 RSS 541〜546 MB。プラグインが入ると増える。2 GiB を超えるにはリンクの max-memory を 4 GiB に上げる(wasm32 の上限)。4 GiB 超は memory64(未検討)。

### Chrome 156(非出荷構成の wasm、ヘッドレス、SwiftShader、実時間、load 1〜8)
参照プロジェクトの 49 ファイル(71 MB)を seed に入れた bundle で実行(`tools/mk-bundle-chrome.sh`)。File>Open から『Problems loading』まで 17.0 s / 17.7 s。first-idle 11.4 s / 12.7 s(P1 の 10 s に届かない)。Worker の CPU はどの op も 100%(飽和)。
| op | wall s | 窓の更新 /s |
|---|---:|---:|
| アイドル 10 s | 10.0 | 0.7 |
| 再生 10 s | 10.6 | 7.0(変化あり 6.0) |
| Playlist ホイール 60 Hz | 6.5 | 1.4 |
| 中ボタンドラッグ 3 s | 4.6 | 2.8 |
| Mixer フェーダー 2 s | 2.8 | 1.8 |
| Mixer ホバー 2 s | 2.5 | 1.6 |
ビーコン(再生後): UI 更新 5.8/s、エンジン idle 5.0/s、イベント→画面 p50 127〜213 ms、最大 16.7 s、audio underrun 852 quanta(累積;最初の音以降 25)。P2(120 fps)・P3(音切れなし)とも未達。

### 出荷構成の wasm での再測定(w2、これが主な数字)
前任の wasm は出荷構成ではなかった(LOWREGS=0、AI_UNITS なし、SIMD ヘルパなし)ので、同じ生成 C(g4)を HEAD(25e01e2)の F2xH3/F2xH4 と同じ旗でビルドし直した(`tools/wasm-build2.sh`: `LOWREGS=2 LOWREGS_BASE=0x1000`〔`lowregs.py` を各ユニットに適用〕、`AI_UNITS_FILE=ship-hot-units-F2.tsv`、`FLE_WASM_SIMD=1`、`RT_OPT=O2`、`RT_DEFS=-DFLE_R_FAST_COPY=1`、prune S+M、SIZE_OPTS、wasm-opt -Oz。ユニットのコンパイル 353 s)。違い: 混成なし(出荷も同じ)、`NO_FS=0` と `-DFLE_FILEDLG_BROWSER` なし(node の測定用。速さには効かない)、`ship-hot-units-F2` のユニット名は ref1 の分割に同名で当てた(分割が同じ 10 MB 区切りなので近似)。wasm 146.03 MB(名前つき 147.59 MB)。
| 区間(node の主スレッド CPU 秒/仮想秒) | 非出荷(w1、V8 プロファイル入り、load 5) | 出荷構成(w2、プロファイルなし、load 4) | 出荷構成(w2、V8 プロファイル入り、load 8〜11) | native -O2(load 2〜16) |
|---|---:|---:|---:|---:|
| 開く 4.8 仮想秒 | 5.80 | **4.01** | 4.27 | 1.22 |
| アイドル 30 仮想秒 | 1.856 | **0.461** | 0.575 | 0.29〜0.34 |
| 再生 30 仮想秒 | 7.402 | **3.696** | 4.114 | 1.51〜1.74 |
出荷構成(プロファイルなし)の wasm は native(ref1 の load 2〜5 の測定 0.299・1.601・1.216)の、アイドルで 1.5 倍、再生で 2.3 倍、開くで 3.3 倍。非出荷構成(プロファイル入り)に比べ、アイドル 4.0 倍、再生 2.0 倍速い(プロファイルの有無と load の違いを含む。前任のプロファイルなしの非出荷構成は 1.726・7.115・5.240)。
層(再生、出荷構成、`tables/cpu-profile-wasm-v8-w2.txt`): ゲスト FLEngine 42.7%(1.76)、x86 命令エミュレーション 30.2%(1.24、native の 6.3 倍;`fle_ld` 7.5%・`fle_st` 1.8%・`fle_op_sub` 1.3%・`fle_mx_subps`/`addps` 各 1.2%)、dsp 8.4%、Elastique 6.3%、QFC 4.1%、Win32 本体 2.3%、GDI 画素 0.9%、名前なし 2.1%。単独で最大の関数は `f_6becc0`(10.8%、混成表に無く、`--mx-packed` を足すと lower できる〔Maxps〕)。アイドルの層: ゲスト FLEngine 72.9%(`f_863f60` 34.6%)、名前なし 7.5%、x86 エミュレーション 6.6%、Win32 本体 5.5%(`fle_dispatch` 4.8%)。混成表に入っている物は、アイドルで 68%、再生でゲスト FLEngine の 20.5 ポイント(入っていない FLEngine は 22.2 ポイント、Elastique 6.3、dsp 4.4)。
Chrome 156(出荷構成、ローカルの vite preview、**load 12〜16 の間に実行**したので実時間の数字は悪い方に振れる、`tools/chrome-ref1.mjs`、`runs/chrome4`): File>Open から『Problems loading』まで 15.8 s、**first-idle 7.54 s**(P1 の 10 s 以内。ローカルサーバーでネットワークなし)。
| op | wall s | Worker CPU s | 窓の更新 /s(変化あり) |
|---|---:|---:|---:|
| アイドル 10 s | 10.0 | 6.31 | 1.1 |
| 再生 10 s | 10.6 | 10.04(飽和) | 7.3(6.4) |
| Playlist ホイール 60 Hz | 6.5 | 5.63 | 5.7 |
| 中ボタンドラッグ 3 s | 5.2 | 5.06 | 4.2 |
| Mixer フェーダー 2 s | 3.0 | 2.99 | 2.4 |
| Mixer ホバー 2 s | 3.6 | 3.43 | 48.8(43.8) |
ビーコン: UI 更新 17.3/s、エンジン idle 20.6/s(非出荷構成の再生後は 5.8/s・5.0/s)、イベント→画面 p50 48.8 ms・p95 77.6 ms(n=20)、audio underrun は『最初の音以降』27 quanta(非無音 2,586)、canvas 描画 12.3/s。P2(120 fps)・P3(音切れなし)は未達。アイドルは Worker が 63% で収まった(非出荷では 100% で飽和)。
**P3 の見通し(読み、混成の実測なし)**: 出荷構成で再生は 3.7 CPU 秒/仮想秒(プロファイルなし。入りで 4.1)で、1 コアの実時間に 3.7 倍足りない。残りのコストはゲスト FLEngine 42.7%(うち混成表に無い 22.2 ポイント〔`f_6becc0` が 10.8〕)と x86 命令エミュレーション 30.2%(混成で直接 wasm にならない関数が `fle_ld` などを呼び続ける)。混成候補 203 関数(うち 30 は `--mx-packed` が要る)はゲスト自己時間の 27.5% を覆うので、その分が native 並みになるなら再生は 3.7 から 2 台後半への短縮にとどまる見込み。1.0 には構造的な短縮(再描画の回数と量、ゲストの描画コード、エミュレーション層)が要る。

### 起動直後 1.08 GB の内訳(簡単な表)
| 項目 | 大きさ | 出所 |
|---|---:|---|
| wasm の初期メモリ | 640 MiB | `--initial-memory=0x28000000`(`link-web.sh`) |
| 最大 | 2 GiB | `--max-memory=0x80000000` |
| エンジン起動直後(プロジェクトなし)の線形メモリ | 1,078,657,024 B | Chrome のエンジンログ『linear memory now』 |
| 参照プロジェクトを開いて 60 仮想秒動かした後 | 1,110,900,736 B(51.7%) | ビーコンの `memory.wasmBytes`。差 32 MiB は成長の刻み 1 回分で、プロジェクトの実使用量ではない |
| MEMFS に載るファイル | DLL pack 77.4 MB + seed 5.7 MB(参照プロジェクト入りの bundle では seed 71.3 MB) | bundle |
| native の RSS の内訳(再生後 565 MB) | コード 98 MB、[heap] 113 MB、FLEngine の画像 26 MB、dsp の画像 40 MB、無名の mmap(デコード済みサンプル・領域・スタック・Elastique の作業域など)約 290 MB | `tables/smaps-native-c13.txt` |
wasm の線形メモリの内訳(malloc の使用量・VirtualAlloc の合計・スタック)は計測していない。native の RSS から読むと、データはおよそ 470 MB で、残りの約 600 MB は wasm の初期メモリの確保(未使用ページ)と思われる(読み)。

### IntersectRect 1.1 M/s の中身(`tables/intersectrect-native-c12.txt`)
再生 30 仮想秒の IntersectRect 約 3,300 万回のうち 3,183 万回は**ゲストの 1 か所の呼び出し(`0x6c763c`、`f_6c7610`=キャンバスの矩形描画プリミティブ。`RectVisible(canvas, rect)` を `f_6ce3f0` で呼び、見えなければ即座に戻る)から**で、対象の矩形は 1×0・-2×1・-1×1・1×-1 などの**ごく小さい(線分程度の)矩形**、位置は x -1〜1190、y -2〜997(プレイリスト全体)に散らばり、クリップは主に 743×21(TQuickScroller)で、**見える割合は 0.0%**。つまりゲストは画面全体に散らばる 1 画素規模の図形を 1 つずつ作ってはクリップで捨てていて、BeginPaint 1 回あたり約 4,450 回(7,146 回の BeginPaint)になる。これは『ゲストが要求していること』の事実で、実機の FL が同じことをするかは不明。runtime の IntersectRect 本体は 1 回数十 ns で、Win32 本体の 1.4%(native)に収まる。


### 出典のない選択(S-ref2-…)
| id | 選択 | 理由・代替 |
|---|---|---|
| S-ref2-prof-window | アイドル=Enter の 3 仮想秒後からの 30 仮想秒、再生=Space の 3 仮想秒後からの 30 仮想秒 | ref1 の測定区間に合わせた。代替: 他の区間 |
| S-ref2-sampler-1ms | ITIMER_PROF 1 ms(プロセスの全スレッドの CPU)、ゲストスレッド番号は `fle_thread_current()` | 道具の選択。perf は perf_event_paranoid=4 で使えない |
| S-ref2-hyb-95 | 混成表に無い関数の自己時間の 95% を覆うところまで | PM の指示。代替: 99%、上位 N 件 |
| S-ref2-hyb-flags | 出荷の旗+`--sse-int` と、`--mx-packed --mx-rc` を足した 2 通りで fe-wasmgen を `--hybrid-only` で走らせ、`--report` の status が ok の関数を『lower できる』とした | 実際の混成パイプライン(prep/n0/…)は通していない。lower できても画素同一の確認は無い |
| S-ref2-callers-stack-scan | RedrawWindow の呼び元の鎖は、ゲストスタックの語のうち 0x401000〜0x1376000 で直前に E8 があるものを 4 個(`mincore` で読める範囲のみ) | rbp 鎖は使えなかった。古い戻り番地が混ざり得るので結論には使わない |
