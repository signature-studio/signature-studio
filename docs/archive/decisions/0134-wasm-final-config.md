# 0134: 決定 0129 の最終構成(壁 16 = `RaiseException` の停止まで)を wasm で動かす。H(フック付き -O0)と C2(フック無し -O2)を native と突き合わせ、headless Chrome 156 で起動からの時間を測る。VFS の種の渡し方(パック / `--embed-file` / `--preload-file`)の比較。すべて仮の決定

状態: **仮の決定(すべての選択)**。依頼は PM(番号 0134 は PM の割り当て、仮)。調査・実装・ビルド・測定・文書は作業者が行った。作業は HEAD **28eb8ca** の `git archive`(`lift/`、`docs/`、`tools/`)で始め、clean な写しを `clean/` に置いた。差分は `wasm0129.diff`(`diff -ruN clean src`、`patch -p1` で当たる。MERGE-NOTES.md)。**`lift/runtime/`・`lift/crates/`・tests・`a4-fruity.sh` は編集していない。** **実機 Windows との照合は 0 件。** Firefox・Safari・headed の Chrome・GPU・HiDPI・実回線(遅延・TCP の動特性)は未測定。入力(G3 の 1 周)・SEH・音声は範囲外(壁 16 の先へは進めていない)。

区分の凡例: **実測**＝このセッションのコマンドの出力を見た事実。**計算**＝実測した値からの算術。**読み**＝コードや出力からの解釈(確かめていない)。**未確認**。**出典なしの選択**＝ページにも実測にも根拠が無く、作業者が選んだ値(`S-…`、撤回条件つき)。

**肯定的に見せる点は限られる。** (1) 壁 16 は SEH を実装していない**停止**であり、見えるのはその時点の 1 枚(lid 13 の面)。入力に応答して再描画する UI ではない(G3 は 0128 のまま半分)。(2) 面の画素は決定 0126 の画素モデル(`S-` 選択)の結果で、native と同一であっても「Windows の GDI と同じ」ではない。文字の形・位置が実機と一致するかは未測定。(3) C2 は入口数を数えられない。正しさの基準は停止メッセージ・スレッド表・窓表・面の SHA だけで、最適化が未定義動作を露出させないことの証明ではない。(4) H の入口数は native より **2 少ない**(決定 0128 と同じ差。原因は今回は再調査していない)。(5) 最初の画素までの時間は 0130 の C2(旧状態)より **約 3 s(+10%)遅い**。(6) 依頼文の「種 約 74 MB」は**誤り**だった(下)。(7) 測定は 1 台・他の作業と共有の機械で、C2 の 4 回のうち 1 回(`br2`)は揺れが大きく(最初の画素 35.1 s)、別の `par=1` の 1 回目は 44.1 s の外れ値だった。

## 先に結論

| 項目 | 内容 |
|---|---|
| **ページを開いてから(Chrome、C2、brotli 配信、loopback、4 回の中央値(範囲))** | (i) 種のパックの取得完了 **0.20 s**(0.19〜0.41)、DLL 込みのデータ全部 0.38 s(0.36〜0.69)。(ii) wasm の準備完了 **1.64 s**(1.57〜2.84)。(iii) 最初の非背景画素が canvas に出る(次の rAF) **33.2 s**(32.9〜35.1)。(iv) 壁 16 の停止(停止時の面が canvas に出る) **36.2 s**(35.95〜38.07)。エンジン実行(main → exit)34.6 s(34.35〜35.18)、最初の画素から停止まで 3.0〜3.1 s。**実測**(runner の `chrome-runs-all.tsv`) |
| 面の SHA が native の期待値と一致したか | **一致。** 期待値(native の最終構成を gdb で読んだ lid 13 の面、A=255 の RGBA SHA-256): 最初の画素 `4f5a5062b25e33c8adf0e1491cb0d489c19d65ac03cc10794888eea59c8951ba`(非 0 画素 201,380)、停止時 `b12c8eceb4e86199fe70ccd1d64a8c2e68e23a3e78999616b9832e836bee51db`(1,917,816)。停止時は idle1 の PNG(`surface-lid13-at-stop-wall16.png`)を present1 の復号で RGBA にした SHA とも**一致**。**H・C2 とも、node(C の SHA-256 計算)と Chrome(モジュール・canvas A の読み戻し・canvas B の読み戻しの 3 者)の最初の画素・停止時すべてで一致**。Chrome の完走 **15 ナビゲーション**(14 回の起動。H 1、C2 14。2 回目の起動を含む)で不一致 0 |
| H の入口数・新規関数・スレッド | native: **141,732,210** 入口、新規 **5,800**、スレッド **10**、窓 110、仮想時計 3,600,008 ms。H(node と Chrome の両方): **141,732,208**(−2)、新規 **5,800**、スレッド **10**、窓 110、仮想時計 3,600,008 ms。最初の画素の入口(プログラム開始から)は native 141,387,834 / wasm 141,387,832(−2)。**差は決定 0128 と同じ −2**(同じゲストの未初期化スタック読みと**読む**。再特定はしていない)。スレッド表・窓表(110 エントリ)・面の要約 337 行は、native のログと(アドレスを伏せて)**完全一致**(H の node と Chrome)。C2 は 336/337 行一致(違う 1 行は「入口数」の欄が `0`。フックが無いため) |
| 停止時のスクリーンショット(目視) | **文字が描かれている。** メニュー(FILE EDIT ADD PATTERNS VIEW OPTIONS TOOLS HELP)、PAT/SONG、`140.000`、`1:01:00`、`Browser`、`Channel rack`、`Sampler`、`Playlist - Arrangement ▸ (none)`、`Pattern 1`、`Track 2`〜`Track 7`(と下端の `Track 17`)、ミキサーの `Wide`、列番号 1〜28、`Insert 1`〜`Insert 28`、`Master`、`Mixer - Master`。Browser パネルの本体は黒い空白。エラーダイアログは無い。**実測(私が画像を見た)** |
| wasm の大きさ(生 / gzip9 / brotli q11) | **C2: 667,115,949 / 75,036,707 / 26,525,655 B**(brotli で生の 3.98%、1 GiB の **62.1%**)。H: 588,344,749 B(gzip9 163,568,646、brotli **q9** 78,228,019。q11 は未測定)。0130 の C2(旧状態)は 525,646,427 B なので **+26.9%**(brotli は 22,559,681 → 26,525,655 B、+17.6%) |
| 種のパックの大きさ | **生 5,733,627 B / gzip9 4,073,379 B / brotli q11 3,326,299 B**(q5 3,762,964、q9 3,744,054)。SHA-256 `0f4b547e66566d18ee240e233066dc2db77c0ff305cc51f8ab8d3a8fa212b938`(2 回生成して同一)。**依頼文の「約 74 MB」は誤り**: `lift/out-idle1/inputs` の `du` 74 MB のうち 70,619,289 B は DLL(dsp 39.9 MB、FLEngine 22.2 MB ほか。種のリストが指さないファイル 7 個)で、種のリストが指すホストファイル 218 個の合計は **5,638,150 B(5.4 MiB)** |

## 手順と構成(実測)

| 項目 | 内容 |
|---|---|
| 入力 | FLEngine `b2c64da4…`(SHA-256 を照合済み)、dsp・Blend2D・QFC・FreeType・ILTools・フォント 2・`GM_CCs.txt`。決定 0129 の VFS の種 `vfs-seed-idle1.tsv`(237 エントリ = ホストファイル 218 + ディレクトリ行 19)、dsp の種 `dsp-ippv2-2000000-seeds-idle1.txt`(16,133 行)。ホストファイルは `lift/out-idle1/inputs/` ほかの実物(**読み取りのみ**)。pack 生成時にリストの SHA-256 列と 218 個すべてを照合(不一致 0) |
| native の再現 | idle1 の `fruity-config.sh` の列 + `FLE_FRUITY_NO_IDLE_STOP=1`(verification.md §5.1・§10 と同じ)を `a4-multi.sh` に渡す(私の `run-native.sh`)。結果: 壁 16 `kernel32!RaiseException(0x0EEDFADE, flags 1, 8 args)`、141,732,210 入口、新規 5,800、スレッド 10、窓 110、ダイアログ無し(`TMsgForm` 0)、`two-native` SHA `d07379819f16…`(verify3 の値と同一)、生成 C: FLEngine 215.8 MB、dsp(m1)**491.1 MB**、Blend2D 101.4 MB、QFC 18.2 MB、FreeType 10.0 MB、ILTools 35.7 MB(計 約 872 MB)。a4-multi 全体 6:18(`CreateFruityInstance` 247.7 s)、最大 RSS 1.93 GB |
| HEAD の進行 | 作業中に HEAD は 28eb8ca から 8d289b1 に進んだ(8356185 = 決定 0133 が `a4-fruity.sh` に `on-0129` を足した。同じ文面に『種のホストファイル 218 個は 5.6 MB、74 MB は `out-idle1/inputs` の全体』とある)。この作業の native は `on-0129` を使わず自前の `run-native.sh`(idle1 の列 + `FLE_FRUITY_NO_IDLE_STOP=1`)で走らせ、0133 の文面と同じ数字になった。測定はすべて 28eb8ca の木。差分は 8d289b1 にも当たる |
| 期待面の取得 | `gdb -batch`(`tools/gdbsurf.py`): `describe_view`(`fle_raster.c`。最初の画素のときは `fle_raster_wrote` の中の書き込み直後、停止時は `fle_raster_report` から呼ばれる)に非停止のブレークポイントを置き、lid 13 の面(1930×1092 BGRX)の 8,430,240 B を読む。1 回目の試行は `fle_win_report` に置いたが、停止時の 1 回しか呼ばれず最初の画素の面が取れなかった(診断。やり直した)。レイアウト(`wsurf`)の想定は lid・live・幅×高さで検査してから書く。BGRX → RGBA(A=255)の SHA-256 は node で計算。停止時は idle1 の PNG を present1 の `png.mjs` で復号した RGBA の SHA と同一 |
| wasm のビルド | `build_nmod_wasm.sh THREADS=1 JSPI=1 SJLJ=wasm VAR=force FRUITY=1 HEAPPROBE=1`(`HEAPPROBE`・`UNIT_SLICE` は今回の追加。未指定なら従来のコマンド)。同じ生成 C を `split_bytes.py` で 10 MB の単位に切った(FLEngine 22、dsp 50、Blend2D 11、QFC 2、FreeType 1、ILTools 4、`common.c` = 計 91 単位)。**H** = `HOOKS=1 GEN_OPT=O0 BIG_OPT=O1`(0128 と同じ)、**C2** = `HOOKS=0 GEN_OPT=O2 BIG_OPT=O2`(perf1 の C2 と同じ)。1 本ずつ、cgroup(`systemd-run --user --scope -p MemoryMax=5500M -p MemorySwapMax=0`、`memguard.sh`)、`JOBS=2`、他の測定と重ねていない。link は `LINK_OPT=O0` |
| Chrome | Chrome 156.0.8078.47(V8 15.6.75.19)、`--headless=new --disable-gpu`、DPR 1。隔離(新しい browser context、`--host-resolver-rules`、`--no-proxy-server`、`--disable-ipv6`、net-log `Default`)。基準『TCP の非ループバック 0 かつ UDP の送信パケット 0』を **全 14 回の起動(15 ナビゲーション)で満たした**(UDP は `connect()` to `[2001:4860:4860::8888]:443` が起動ごとに 4〜8 回、送信 0 バイト。`NETLOG_TOLERATE_IPV6_PROBE=1`。`logs/chrome/netlog.tsv`) |
| 機械 | AMD Ryzen 9 9950X 16 コア、Linux 7.0、RAM 14.9〜24 GB で変動。他の作業(SEH の実装・a4-fruity の整合。native)と共有 |
| 初期の Worker の版 | `fle-final-worker.js` は H の Chrome 1 回と C2 の `br1`〜`br4`・`cacheapi1` を測った後に `seedmode`(embed / preload の分岐)・`locateFile`・`monitorRunDependencies` を足した。pack の経路は変わらない(足した分岐は `seedmode !== 'pack'` のときだけ。`locateFile` は glue が自分で探すファイルにだけ効き、wasm は `instantiateWasm` で渡す)。`embed`・`preload`・`par`・`rate` の走行は新しい版 |

## 種の渡し方の比較と選択

ブラウザでは native のようにホストファイルを `fopen` できない。種の読み込み口(`vfs_seed.h`)は、リストの TSV を読み、各行のホストファイル(`FLE_VFS_SEED_ROOT` からの相対パス)を `fopen`/`fread` で **全部 `malloc` に読み込み**、SHA-256 を照合してから、ゲスト向けの C の窓口(`CreateFileW` / `WriteFile` ほか)で仮想ボリュームへ書き、最後に読み込みバッファを `free` する(読み)。したがって wasm 側は **MEMFS(または埋め込み・preload)に同じ相対パスでファイルがあれば C を変えずに動く**。

| 案 | 内容 | 測った結果(C2、Chrome、各 2 回、面の SHA は全て一致) |
|---|---|---|
| **(a) パック(採用)** | `pack_seed.py` が TSV と 218 個のホストファイル + レジストリの種 2 + claims を 1 ファイルにする(`FLSEEDP1` + ヘッダ長 + JSON 索引 + 連結バイト列。222 エントリ、5,733,627 B)。Worker が `fetch` し、索引を読み、`FS.writeFile(path, view, {canOwn: true})` で MEMFS(`/data/seed/<相対パス>`)に書いてから `main`。MEMFS の節点はパックの ArrayBuffer への view を持つ(コピー無し、読み) | 取得完了 0.20 s(br、loopback)、**MEMFS 書き込み 20.7〜24.2 ms**(222 ファイル)、書き込み前後の **線形メモリ 0 B の変化**(671,088,640 B のまま。MEMFS は JS 側)。wasm は 667,115,949 B のまま。最初の画素 32.9〜35.1 s |
| (b1) `--embed-file` | `seedstage/seed@/data/seed` を `emcc --embed-file`(`LINK_EXTRA`)。C2 の `.o` を再リンクするだけ | **wasm が +5,712,122 B**(672,828,071 B。データは wasm に入る)。gzip9 +4,071,008 B、brotli q9 +3,716,722 B(32,771,403 → 36,488,125)。**線形メモリの最終値 +5,701,632 B**(980,549,632 B。静的データ)。`memfs_ready` → `main` が約 27 ms(パックは 1〜3 ms)。最初の画素 31.7〜32.2 s |
| (b2) `--preload-file` | 同じ階層を `--preload-file`。再リンクのみ | wasm は**同一バイト**(SHA 一致)。`flengine6.data` 5,691,421 B(brotli q11 3,317,500、gzip9 4,065,410)をグルーが取得。glue が +39 KB。run dependency の解消は `memfs_ready` の約 6 ms 後。線形メモリ 974,848,000 B(パックと同じ)。最初の画素 31.9〜32.5 s |
| (c) 遅延読み込み(範囲外) | ゲストが開いたときに fetch(JSPI で待つ)。大きな設計 | 作っていない。(a) の効果から言える上限: 種の brotli 3,326,299 B は配信量の合計 44,346,945 B(wasm 26.5 + DLL 14.5 + 種 3.3 MB)の **7.5%**、25 Mbit/s の回線で 1.06 s 分(**計算**)。ゲストが実際に開くファイルの数・バイトは**測っていない** |

- **選択: (a) パック。** 理由(**読み**を含む): 種が 5.7 MB と小さく、3 案の起動時間は揺れの範囲で区別できない(最初の画素 31.7〜35.1 s)。(a) は wasm と種を**独立に更新・キャッシュ**でき(種を変えても 667 MB の wasm を再配信しない)、Worker が種の取得と DLL の取得を並行して行え、wasm のサイズ(1 GiB の上限に対する余裕)を使わない。(b1) は wasm が種の分だけ大きくなり、種を変えるたびに wasm の再リンクと再配信が要る。(b2) は wasm と種の取得が glue 任せになり、2 回目の起動の保存(Cache API)を種に別途組み込みにくい。**ただし 5.7 MB では 3 案とも実害はなく、これは設計上の好みで、時間の差ではない。**
- **「74 MB」について(実測)**: 依頼文の約 74 MB は `lift/out-idle1/inputs` のディレクトリ全体(76,145,401 B)で、うち 70,619,289 B は 7 個の DLL(種のリストが指さない)。リストが指す 218 個は 5,638,150 B。DLL 6 個(70,572,928 B、brotli q11 14,494,991 B)は従来どおり別ファイルでページが取る。
- **MEMFS に置いたあとの線形メモリの増分(実測、`fle_heap_probe`)**: MEMFS への書き込みは線形メモリを増やさない(0 B)。C 側の種の取り込み(`vs_run`)では、`vfs seed: list` の時点で使用中 14,812,200 B、`vfs seed: done` で 21,249,536 B(**保持される分 +6,437,336 B**。仮想ボリュームの中の写し)、ヒープの先頭(`sbrk(0)`)は 305.7 → 313.1 MB(**一時的な増分 約 +7.4 MB**)。初期の線形メモリ 671,088,640 B の中に収まり `memory.grow` は起きない。取り込み(SHA-256 の照合 + ボリュームへの書き込み)の時間は 11〜13 ms(`vfs-seed-start` → `vfs-seed-done` の Worker 時刻、C2)。JS 側(MEMFS の ArrayBuffer 5.7 MB)の増分は `performance.memory` が Worker で使えず**未測定**。
- 読み込み時間のまとめ(C2、br、loopback): 取得 0.20 s → MEMFS 21〜23 ms → C の取り込み 11 ms。種は起動の時間にほとんど効かない(エンジン実行が 34.6 s)。

## 変種の測定表

### ビルド(段ごと。cgroup のピーク = ツリー全体)

| 変種 | rt | harness | units | o1 | link |
|---|---|---|---|---|---|
| **H**(フック付き -O0、JOBS=2) | 6 s | 2 s | **205 s**、ピーク 3,705 MB(最大プロセス 1.68 GB)。91 単位。V8 の locals 上限 50,000 を超える関数が 9 単位に **61 個**(最大 397,472 locals。0128 は 63) | **1,488 s**、ピーク 1,162 MB(直列、61 関数を -O1) | 8 s(ピーク 2.49 GB) |
| **C2**(フック無し -O2、JOBS=2) | 7 s | 2 s | **2,016 s + 1,906 s = 3,922 s(65 分)**。`UNIT_SLICE=1/2` と `2/2` の 2 回の前景。ピーク 5,055 / 5,105 MB(上限 5,500)、最大プロセス 3.11 / 2.99 GB。locals 超過 0 件(o1 段は不要) | — | 4 s(ピーク 2.6 GB) |

- 生成 C が 0130 の約 1.4 倍(dsp の種が 1,784 → 16,133 行で、dsp の関数 25,370 個・491 MB)なので、C2 の units は 0130 の 3,531 s より長くなり、前景 1 本 60 分の上限に収まらない見込みだった(読み)ため、`build_nmod_wasm.sh` に `UNIT_SLICE=k/n` を足して 2 回に分けた(存在する `.o` は飛ばすので、2 回で 1 回分)。

### 大きさ

| 変種 | 生 | gzip9 | brotli | 1 GiB に対する割合 | 関数数 | code セクション | 備考 |
|---|---|---|---|---|---|---|---|
| H | 588,344,749 | 163,568,646 | q5 92,204,728 / q9 78,228,019 | 54.8% | 122,831 | 585,814,481 B(99.57%) | 0128 の A は 465,880,569 B |
| **C2** | **667,115,949** | **75,036,707** | q5 36,646,369 / q9 32,771,403 / **q11 26,525,655** | **62.1%** | 113,629 | 664,630,077 B(99.63%) | 0130 の C2 は 525,646,427 B(+26.9%)。1 GiB まで 406,625,875 B(**計算**) |
| C2 + `--embed-file` | 672,828,071 | 79,107,715 | q9 36,488,125 | 62.7% | — | — | 下の (b1) |

- data セクションは 2,055,430 B(0.31%)。DLL のイメージも種も wasm の data には入っていない(パック・preload のとき)。
- DLL 6 個(別ファイル)70,572,928 B → gzip9 23,589,638 B、brotli q11 14,494,991 B。
- 配信量の合計(C2、brotli q11): wasm 26,525,655 + DLL 14,494,991 + 種 3,326,299 = **44,346,945 B**。gzip9 だと 75,036,707 + 23,589,638 + 4,073,379 = 102,699,724 B。

### 時刻と実行(Chrome 156、ナビゲーション開始からの秒、loopback、HTTP キャッシュなし)

`chrome-runs-all.tsv`。「最初の画素」「停止」は canvas B に描いた次の rAF(0130 の方式)。

| 構成 | 実行 | (i) 種パック取得 | データ全部 | (ii) wasm 準備 | main 開始 | (iii) 最初の画素 | (iv) 停止 | renderer のピーク RSS | プロセス木のピーク |
|---|---|---|---|---|---|---|---|---|---|
| **H**(-O0+フック、無圧縮) | 1 | 0.10 | 0.22 | 2.24 | 2.27 | **313.89** | **329.00** | 1,539 MB | 2,866 MB |
| **C2** br | br1 | 0.20 | 0.36 | 1.57 | 1.60 | 32.93 | 35.95 | 1,411 MB | 2,733 MB |
| C2 br | br2 | 0.41 | 0.69 | 2.84 | 2.89 | 35.07 | 38.07 | 1,504 MB | 2,820 MB |
| C2 br | br3 | 0.20 | 0.39 | 1.69 | 1.71 | 33.13 | 36.19 | 1,464 MB | 2,794 MB |
| C2 br | br4 | 0.19 | 0.36 | 1.59 | 1.62 | 33.21 | 36.26 | 1,464 MB | 2,788 MB |
| **C2 中央値(4 回)** | | **0.20** | 0.38 | **1.64** | 1.67 | **33.17** | **36.22** | 1,464 MB | 2,791 MB |
| C2 Cache API(gzip)1 回目 | cacheapi1#1 | 0.14 | 0.36 | 2.33 | 2.36 | 33.67 | 36.55 | — | — |
| C2 Cache API(gzip)**2 回目** | cacheapi1#2 | 0.24 | 0.25 | 1.78 | 1.82 | 32.40 | 35.30 | 1,943 MB | 3,367 MB |
| C2 `--embed-file` | embed1 / 2 | —(wasm に内蔵) | 0.32 / 0.35 | 1.50 / 1.56 | 1.53 / 1.59 | 32.24 / 31.74 | 35.07 / 34.56 | 1,515 / 1,428 MB | 2,826 / 2,758 MB |
| C2 `--preload-file` | preload1 / 2 | —(glue が取得) | 0.35 / 0.34 | 1.57 / 1.60 | 1.58 / 1.61 | 31.96 / 32.50 | 34.78 / 35.35 | 1,394 / 1,402 MB | 2,679 / 2,725 MB |
| C2 `par=1`(wasm とデータを並行) | par1 / 2 | 0.35 / 0.23 | 0.71 / 0.43 | 2.80 / 1.60 | 2.85 / 1.62 | **44.13** / 33.05 | 47.06 / 35.98 | — | — |
| C2 回線 100 Mbit/s(1 本の共有回線のエミュレーション) | rate100 | 1.05 | 1.51 | 4.40 | 4.43 | 36.01 | 38.95 | — | — |
| C2 回線 25 Mbit/s | rate25 | 4.03 | 5.84 | 15.07 | 15.10 | 45.20 | 48.02 | — | — |

- **線形メモリの最終値**: C2・パック 974,848,000 B(14,875 ページ = 2 GiB の上限の 45.4%、`memory.grow` 2 回: +2,971 ページで 865,796,096 B(main 開始の約 0.2 s 後)、+1,664 ページ(main 開始の約 7.8 s 後))。H は 975,896,576 B(14,891 ページ)。0130 の C2 は 946,601,984 B(**+28.2 MB**)。embed は 980,549,632 B。
- **0130 の C2(旧状態: ダイアログあり・文字なし、134,880,081 入口)との比**: 最初の画素 30.1 → 33.2 s(+3.1 s、+10%)、idle/停止 33.0 → 36.2 s(+3.2 s)。入口数は 141.7M で 5.1% 増、さらに QFC の字形処理が加わる(**読み**。内訳は未測定)。
- 最初の画素から停止までの 3.0〜3.1 s は、面が 201,380 画素から 1,917,816 画素へ描き足される時間(0130 は 2.9〜7.2 s)。
- 外れ値: `br2`(wasm 準備 2.84 s、最初の画素 35.07 s)と `par1`(最初の画素 44.13 s、エンジン実行 41.3 s)。原因は調べていない(機械は他の作業の native 走行と共有で、**読み**として CPU の取り合い)。`par=1`(wasm の取得とデータの取得を重ねる)の効果は loopback では見えない(`par2` の wasm 準備 1.60 s は、重ねない `br1`・`br4` の 1.57・1.59 s と同じ)。
- **node**(`node --experimental-wasm-jspi`、NODERAWFS): H **5:41**(`CreateFruityInstance` 338.3 s、最大 RSS 1.98 GB)、C2 **42.0 s**(同 40.4 s、RSS 1.47 GB)。0128 の node は 320〜329 s(旧状態)。

### 回線のエミュレーションと 2 回目の起動

- エミュレーション(0130 と同じ方式: サーバーが全リクエストで 1 本の回線を共有して応答を絞る。遅延・TCP の動特性なし): 100 Mbit/s で最初の画素 36.0 s・停止 38.95 s、25 Mbit/s で 45.2 s・48.0 s(各 1 回)。転送量 44.35 MB ÷ 回線 = 3.5 s / 14.2 s(**計算**)に対し、wasm 準備完了は 4.40 s / 15.07 s。
- **2 回目の起動(Cache API に gzip のバイト、同じ browser context で 2 回ナビゲート、`cacheapi=gz`)**: wasm(`.wasm.gz` 75,036,707 B)・種のパック(4,073,379 B)・DLL 6 個(23,589,638 B)の**全部**を Cache API に置いた(1 回目はネットワークから gzip のバイトを取って `put`、復号は `DecompressionStream('gzip')`)。2 回目の**転送は 0 B**(request log)、種のパック取得完了 **0.24 s**(1 回目 0.14 s)、データ全部 0.25 s(1 回目 0.36 s)、wasm 準備 **1.78 s**(1 回目 2.33 s)、最初の画素 **32.40 s**(1 回目 33.67 s)、停止 35.30 s(36.55 s)。**種のパックを含めた起動の効果は小さい(エンジン実行 34 s が支配)。効くのは転送量**(25 Mbit/s なら gzip の合計 102.7 MB だけで 約 33 s の転送を省く。**計算**)。1 回目も gzip で運ぶので br 配信(44.3 MB)より転送量は 2.3 倍。
- **Chrome 156 の `DecompressionStream` が受ける形式(実測、probe)**: `gzip`・`deflate`・`deflate-raw` のみ。`br`・`brotli`・`zstd` は `Unsupported compression format`。したがって Cache API に置いて復号できるのは gzip で、brotli のバイト(26.5 MB)をそのまま Cache API に置いて使う方法は標準の API では取れない(JS/wasm の brotli 復号器を入れれば可能だが未検討)。
- HTTP キャッシュは 0130 のとおり 6 MB 超の応答に効かない(今回は再測定していない)。

### brotli の品質比較(perf1 から移管。C2 の wasm 667,115,949 B と種のパック 5,733,627 B)

`node` の `zlib`(lgwin 24、size hint 付き、ストリーム)。エンコード CPU = `process.cpuUsage()` の user+system、1 設定ずつ(重ねない)。デコードは同じ `zlib` の単一スレッド(ブラウザの復号器そのものではない)。

| 対象 | 設定 | 出力 B | 生に対する割合 | エンコード wall / CPU | デコード wall / CPU |
|---|---|---|---|---|---|
| C2 wasm | br q5 | 36,646,369 | 5.49% | 3.9 s / 4.3 s | 3.34 s / 3.28 s |
| C2 wasm | br q9 | 32,771,403 | 4.91% | 16.6 s / 16.9 s | 3.41 s / 3.28 s |
| C2 wasm | **br q11** | **26,525,655** | **3.98%** | **561.4 s / 561.8 s** | 3.57 s / 3.46 s |
| C2 wasm | gzip9 | 75,036,707 | 11.25% | 12.9 s / 13.4 s | 3.82 s / 3.71 s |
| 種のパック | br q5 | 3,762,964 | 65.63% | 0.1 s / 0.1 s | 0.06 s |
| 種のパック | br q9 | 3,744,054 | 65.30% | 0.4 s / 0.4 s | 0.06 s |
| 種のパック | **br q11** | **3,326,299** | **58.01%** | 13.0 s / 13.0 s | 0.06 s |
| 種のパック | gzip9 | 4,073,379 | 71.04% | 0.2 s / 0.2 s | 0.05 s |

- C2 wasm で **q11 は q9 より 19% 小さい(−6.2 MB)が、エンコードは 33 倍の CPU(562 s と 17 s)**。q5 は q11 より 38% 大きい。デコードは品質によらず約 3.3〜3.5 s(CPU、単一スレッド。Chrome の復号が重なる程度は未測定)。種のパックは中身の多くが圧縮済みの PNG/WAV 等で、q5/q9 では 65%、q11 で 58%。
- H の wasm: br q5 92,204,728 / q9 78,228,019 B(エンコード CPU 7.8 s / 77.5 s)。-O0 は圧縮が効きにくい(13.3%、C2 は 4.9%)。DLL 6 個の q11: FLEngine 38.1 s、dsp 53.0 s ほか(`logs/compress/`)。

## native との突き合わせ(H・C2)

- H(node と Chrome): 上の「先に結論」のとおり、入口数 −2、新規関数・スレッド・窓・仮想時計・停止メッセージ(`fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no structured exception handling is modelled, stopped here`)・面の SHA が一致。`vfs seed:` の 238 行(ゲストのパス・バイト数・SHA-256)も native と一致(list 行のパスだけ違う)。
- C2: node 1 回と Chrome の `br1`・`embed1`・`preload1` の stdout を native のログと比べた: 停止メッセージ、スレッド 10・窓 110・仮想時計・スレッド表・窓表・面の要約(337 行中 336 行一致。違う 1 行は入口数の欄)、面の SHA が一致。新規関数は数えない(`0`)。Chrome の他の走行は面の SHA(3 者)・終了コード 134 だけ確認した。
- 突き合わせたのは `logs/compare/native-vs-wasm-tail.json`。G2(主窓の可視、40,344,626)は、この構成のログに該当行が出ないので**比べていない**。
- **H の入口数が −2 であることの扱い**: 決定 0128 の `f_79c0f0` が未初期化のスタック欄 `[rbp+0x5c]` を読む、native はホストの 64 ビットポインタの上位半(≠ 0)、wasm32 は 0 のため `f_79bec0` の呼び出しが 2 回少ない、という読みと同じ数(−2)、最初の画素の入口も同じく −2、新規関数・画素・スレッドは影響を受けない。今回は診断(`FLE_WF1_POKE` 相当)を再実装していないので、**原因は 0128 の読みを引用するだけで、今回の木では再確認していない(未確認)**。差が変わっていない(−2 のまま)ことは実測。

## 面の SHA(期待値の取り方)

1. native の最終構成(`oonat` の `two-native`、`FLE_FRUITY_NO_IDLE_STOP=1`)を gdb の下で走らせ(`CreateFruityInstance` 227.2 s)、`describe_view` のブレークポイントで lid 13 の面を読む: 最初の画素(非 0 画素 201,380。ログの「non-zero pixels 201380」と一致)と停止時(1,917,816)。
2. BGRX → RGBA(A=255)の SHA-256: 最初の画素 `4f5a5062…`、停止時 `b12c8ece…`。
3. 停止時の PNG(`docs/investigations/idle1/logs/fruity-on/surface-lid13-at-stop-wall16.png`、ファイル SHA-256 `d357754c0adb3d483833e9b362e0439924733fe408bfedeb1baaf05bdc59e205`)を present1 の `png.mjs` で復号して RGBA(A=255)にした SHA-256 は `b12c8ece…` で、gdb で読んだ値と**同一**(最初の画素には PNG が無い)。
4. wasm 側の SHA: C の `wf1-surface:` 行(C の SHA-256)、node の `crypto`、Chrome の canvas A の `getImageData`、canvas B の `getImageData` の 4 つが同じ値。

## スクリーンショットの所見(目視)

停止時の canvas(1930×1092)をページの矩形で `Page.captureScreenshot`(`captureBeyondViewport`)した画像を私が見た。H の 1 回と C2 の 12 回の SHA-256 は同一(`dcfd2258…`)。`rate` の 2 回だけ、ページの配置が違い(矩形 x=6, y=202)別の値(canvas の SHA は期待値と一致)。H と C2 が同じ画像を描く。見えたもの: 上端のメニュー行(FILE EDIT ADD PATTERNS VIEW OPTIONS TOOLS HELP)、PAT/SONG、再生・停止・録音のボタン、`140.000`、`1:01:00`、左の Browser パネル(見出し `Browser` とアイコン列。本体は黒い空白)、`Channel rack` と `Sampler`、`Playlist - Arrangement ▸ (none)`、`Pattern 1`、`Track 2`〜`Track 7`、下のミキサー(`Wide`、`M` と列番号 1〜28、`Insert 1`〜`Insert 28`、`Master`、`Mixer - Master`、緑のレベルメーター)。エラーダイアログは無い。拡大(3 倍)でメニューの文字・`1:01:00`・`140.000` が読める。**文字の形・位置が実機と一致するかは未測定**(実機の画面が無い)。PM が決定 0129 の PNG で確認した内容と同じ文字が wasm の canvas に出ている。
(保存した画像: `logs/screenshots/` の 3 枚 — H の停止時、C2 の停止時、C2 のメニュー行の 3 倍拡大。)

## 出典なしの選択(S-…)

| 名前 | 値 | 理由 | 撤回条件 |
|---|---|---|---|
| `S-seedpack-format` | `FLSEEDP1` + u32 ヘッダ長 + JSON 索引(path・size・offset・SHA-256)+ 連結バイト。圧縮はしない(配信側で br/gzip) | 標準ライブラリだけで書け、Worker が数行で読める | 標準のコンテナ(zip/tar)や遅延読み込みの単位が要るとき |
| `S-seed-memfs-root` | MEMFS の `/data/seed/<リポジトリ相対パス>`(TSV の host 欄の相対パスをそのまま使う)、`FLE_VFS_SEED_ROOT=/data/seed` | `vfs_seed.h` を変えずに動かす | 種の置き場を製品のツリーに合わせて設計するとき |
| `S-par-default-off` | wasm の取得はデータの取得のあと(0130 と同じ)。`par=1` で並行 | 0130 との比較可能性 | 本番の配信設計を決めるとき |
| `S-wall-limits-wasm` | wasm・node の `FLE_FRUITY_SECONDS=3000`、`FLE_FRUITY_MAX_CALLS=4000000000`(native の a4-multi は 900 / 同)。結果に影響しない上限 | 壁 16 までを確実に走らせる | — |
| `S-brotli-lgwin-24` | 品質比較は lgwin 24(0130 と同じ)、size hint 付き | 比較可能性 | — |
| `S-screenshot-clip` | canvas の矩形(1932×1094、枠 1 px を含む)を `captureBeyondViewport` で撮る | 目視用。画素の一致の根拠ではない(根拠は SHA) | — |
| (読み)`S-heap-probe-dlmalloc` | `fle_heap_probe` は dlmalloc の `mallinfo` を読む | emcc 6.0.12 の既定の `malloc` | 別の malloc に替えたとき |

## 見つかったこと・意外なこと

1. **「種 約 74 MB」は誤りで、実際は 5.7 MB(DLL を含めた 76 MB のディレクトリのうち 70.6 MB が DLL)。** 設計の前提が変わる(下の PM の未決点)。
2. **dsp の種(16,133 行)で生成 C が 1.4 倍**(dsp 491 MB)。wasm は +26.9%(C2)になり、C2 のビルドは 65 分(0130 の 59 分)、1 GiB に対する割合は 49% → 62%。残りは 406,625,875 B(1.6 倍)。
3. 最初の画素までは 30.1 → 33.2 s(+10%)、停止まで 36.2 s。
4. **Chrome 156 の `DecompressionStream` は brotli を受けない**ので、Cache API の 2 回目の起動は gzip(wasm 75 MB)になる。2 回目の起動の効果は時間では小さい(最初の画素 −1.3 s)。
5. brotli q11 は q9 より 19% 小さいが、エンコードは 33 倍の CPU(C2 の wasm で 562 s)。
6. 2 回目の `par=1` と `br2` に大きな揺れ。機械が他の作業と共有。
7. native の gdb で、最初の画素の面は `fle_raster_wrote` の中の `describe_view` で取れる(`fle_win_report` は停止時の 1 回だけ)。

## PM の未決の点(決めない。推奨は書く)

1. **種の渡し方。** 5.7 MB なので (a) パック / (b1) embed / (b2) preload のどれでも時間は同じ。推奨: (a) パック(種と wasm を独立に差し替え・キャッシュ)。(b1) は wasm +5.7 MB(brotli +3.7 MB)で、種を変えるたびに 667 MB の wasm を再配信する。(c) の遅延読み込みは、上限効果が配信量の 7.5%(3.3 MB)なので、当面不要と見る(ゲストが開くファイル数は未測定)。
2. **C2 の時間の目標。** 最初の画素 33.2 s のうちエンジン実行(`CreateFruityInstance` の 1.41 億入口)が 31.3〜32.2 s(main → 最初の画素、中央値 31.5 s)。ネットワーク・種・wasm 準備(合計 1.6 s)は小さい。次の桁は初期化の仕事そのものを減らす(snapshot 等)必要がある(0130 の判断点 2 と同じ)。
3. **brotli の品質。** 推奨: リリースは q11(26.5 MB。エンコード 9 分)、開発中の反復は q9(32.8 MB、17 s)。
4. **Cache API の方式。** gzip のバイトを置く方式は転送を 0 にするが、1 回目と 2 回目で運ぶ量が 2.3 倍(gzip 102.7 MB vs br 44.3 MB)。JS/wasm の brotli 復号器を入れて br のバイトを置く案は未検討。
5. **H の −2。** 原因の再確認(診断の再実装)を行うか。推奨: 実機の値が分かるまで偽の値を入れず記録のみ(0128 の推奨と同じ)。
6. **1 GiB の上限。** C2 で 62%。dsp の種をさらに増やす・G9 を入れる前に、wasm の分割か共有(M-8)の調査が要る(0130 の判断点 3)。
7. **`a4-fruity.sh on` の整合**(verify3 §10)は別の担当。この作業の `run-native.sh` は idle1 の列 + `FLE_FRUITY_NO_IDLE_STOP=1` を直接 `a4-multi.sh` に渡す。
8. ビルド: `UNIT_SLICE`(units を k/n に分ける)を恒久的に入れるか。C2 は 1 コマンドの上限(60 分)を超える見込みだったので使った。

## 規則違反・不備の自己申告

- **指示された「task agent を通じて作業する」(AGENTS.md)は守れていない**: 私は subagent で task を呼ぶ道具が無いため、作業はすべて自分で行った。
- **報告の時刻の見積り**: 報告の見出しの時刻(「16:08Z」「17:58Z」など)は、実際の送信時刻の概算で、1 通(「16:08Z」)は実際は約 16:03Z だった。1 時間以内の間隔は守った(15:08、15:27、15:31、約 16:03、16:14、16:47、17:38、17:58)。最初の中間報告の「DLL 6 個 70,572,...」は数字の途中で切れた文で、正しくは 70,572,928 B。
- **gdb の 1 回目の走行は無駄になった**(`fle_win_report` のブレークポイントは停止時の 1 回だけ。約 4 分)。
- **誤ったツール名の呼び出し**(`bash` という名前の道具。2 回。実行されていない)。
- **`cp -n` の警告**(動作に影響なし)。
- **他の作業の領域の名前を一覧で見た**: 最初の `ls lift/` と `ls lift/out-git | head -50` の出力に `out-a4f`(と `out-git` の他のディレクトリ名)が出た。`git status --short` に他の担当の未追跡ファイル 2 つの名前(`docs/investigations/README-0101-line.txt`、`lift/iat-unsup-closure.txt`)が出た。中身は読んでいない・触れていない。終了時の `systemctl --user list-units --failed` の出力に他の作業の scope(`a4-fruity.sh`・`measure.sh` のコマンド行。`out-g3in` を含む)が 3 件出た。私の scope ではなく、触れていない。
- 規則: 作業は `lift/out-wasm0129/` と `/tmp/opencode/wasm0129/` のみ。`rm` は自分のファイル(`logs/native-surfaces` の最初の試行、`$TMPDIR/hp.o`、`/tmp/opencode/wasm0129/seedpack2.bin*`)だけ。git の変更系操作・commit なし。`sleep`・`nohup`・末尾の `&`・`pgrep -f`・待ちのループは使っていない。実行中のビルドのスクリプトは編集していない(`build_nmod_wasm.sh` と `link-web.sh` は走行の前に編集した。`link-web.sh` の `LINK_EXTRA` は C2 の `br1`〜`cacheapi1` の後、embed の前)。他の作業の領域(`out-seh2`、`out-a4f`、`out-git/seh2-results`、`out-git/a4f0129-results`、`a4-fruity.sh`)は開いていない。
- 作業ディレクトリ `lift/out-wasm0129/` は最大 約 10 GiB(上限 15 GiB)。終了時に生成 C・`.o`・ELF・wasm・cargo target・種のパックを削除して 400 MB 以内にした(MERGE-NOTES.md の「掃除」)。

## 限界(確認できなかったこと)

- **実機 Windows との照合は 0 件**(画素・文字の形・位置・版や登録の答え)。
- C2 は入口数を数えられない。最適化で未定義動作が露出しないことの証明ではない。G3 以降の経路(入力後の再描画)は測っていない。
- Chrome は headless・GPU なし・1 台・loopback(回線のエミュレーションは遅延なしの共有回線)。Firefox・Safari・headed・HiDPI・実回線は未測定。
- C2 の Chrome 4 回のうち 1 回は揺れが大きい(中央値と範囲を併記)。H の Chrome は 1 回、H の node は 1 回。
- ゲストが実際に開く種のファイル数・バイトは測っていない((c) の効果は上限の計算のみ)。JS 側(MEMFS の ArrayBuffer)のメモリ増分は未測定(`performance.memory` が Worker で `null`)。
- V8 のコードキャッシュが 2 回目の起動で効いたかは確認していない。H の gzip/brotli は q9 までで q11 は未測定。wasm-opt は適用していない。
- G2(主窓の可視、40,344,626 入口)は、この構成のログに行が無いため突き合わせていない。
- 決定 0129 の再現手順(インストーラからのホストファイルの再作成)は、今回も通しで再実行していない(`lift/out-idle1/inputs/` の実物を使った)。

## 再現の手順(要約)

1. HEAD 28eb8ca の `git archive` に `wasm0129.diff` を `patch -p1` で当てる(MERGE-NOTES.md)。
2. `tools/run-native.sh onat`(`memguard.sh 5500` の下): 生成 C と native。`tools/gdbsurf.sh onat <dir>`: 期待面。
3. `tools/resplit.sh 10000000`、`VARIANT=H HOOKS=1 GEN_OPT=O0 BIG_OPT=O1 tools/wasm-build.sh rt harness units o1 link`、`VARIANT=C2 HOOKS=0 GEN_OPT=O2 BIG_OPT=O2 tools/wasm-build.sh rt harness`、`UNIT_SLICE=1/2 …units`、`UNIT_SLICE=2/2 …units`、`…link`。
4. `python3 lift/harness/web/final0129/pack_seed.py --root <repo root> --tsv lift/harness/seeds/vfs-seed-idle1.tsv --extra …reg-seed-nsis.tsv --extra …reg-seed-app.tsv --extra …authenticode-claims.tsv --out data/seedpack.bin`。`OBJ=… bash lift/harness/web/fruity/link-web.sh <dist>/web`、`data/` に DLL 6 とパック(と `.br`/`.gz`)を置く。
5. `VARIANT=C2 tools/run-chrome.sh NAME enc=br [cacheapi=gz runs=2] [seedmode=embed|preload] [par=1] [rate=<B/s>]`、`VARIANT=C2 tools/run-node.sh NAME`。
