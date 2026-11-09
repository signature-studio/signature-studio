# 決定 0307(仮): Wine 上の原版 FL の WAV と wasm(E12)の WAV の差の切り分け

> **先頭の注意(必読)**: 比べているのは (A) wasm E12 の `runs/e19-export/export.wav`(sha256 `381cdb5a…f979`、Export の基準と一致)と (B) **Wine 10 上の原版 x86-64 FL Studio 26.1.7.5653(Trial)**が同じ設定で書いた `serina1-serum2.wav`(sha256 `2a4d136a…b803`)。どちらも 16 bit stereo 44.1 kHz。**B は『Wine 上での x86 実行 + Wine の DirectSound → PulseAudio null sink 経路』で、実 Windows ではない**(実 Windows / 実 GPU / 依頼者の機械 = 0 件)。lift でも代役でもなく、原版の実行結果だが、**B を『正解』とは言えない**。以下の数値は numpy で計算した実測、原因に関する文は**読み**で、未確認と書いたものは確認していない。

状態: **仮の決定**。作業者 `opt1`、2026-11-06 JST 18:10〜(`date` の時計)。依頼: PM(0307、0298 の Export で見つかった新しい観察の切り分け)。環境は 0298(`../0298/docs/0298-opt1-wine-reference.md` §2)。

## 0. 先頭の点

1. **差の大半は『FL 内蔵プラグインの有無』だった(実測)**: wasm は参照プロジェクトを『8 plugin(s) could not be opened』で開く(FL 内蔵 8 種の DLL が出荷の volume に無い、alpha1 の読み)。Wine の FL でその 8 種(9 フォルダ)を退避すると、Problems 窓が同じ『8 plugin(s)』になり、Export は **wasm の WAV と同じ大きさ・同じフレーム数・同じ最初/最後の非ゼロフレーム・同じ長さの末尾(551 フレーム)・遅れ 0・利得 0.999〜1.001・1 秒ごとの RMS の相関 1.0000** になった(退避前は 22,915,648 B で、Wine の方が 13,506 フレーム長く、低域が 4.4〜7.1 ms 遅れていた)。= 低域の遅れも余韻の有無も、プラグインの有無で説明できる(§2b.2)。
2. **ビット一致ではない(未解明)**: 退避構成の Wine と wasm E12 の差は、差のあるフレーム 79%、最大 36,664、差の RMS 2,031(退避前は 23,654)。区間に偏る(5〜20 s、45〜60 s、80〜95 s、120〜130 s が RMS 1,000〜5,100、20〜40 s と 100〜120 s は約 1%)。差は 1 kHz 以下でコヒーレンス 0.995〜1.000、1〜20 kHz で 0.49〜0.97(alpha1 の解析、PM 経由)。鳴っている物との相関は 48 kHz のサンプルとパターンの区間(§2b.2)。
3. **Wine 側の揺れでも、スレッド数でも、CPU 機能の経路でもない(実測、E1・E6・E2)**: 退避構成の Wine は、`WINE_CPU_TOPOLOGY=1:0`(論理 CPU 1 つ)でも 16 CPU でも、`+loaddll` を付けても、Export が 5,715,911 フレーム中 5,715,907〜5,715,909 フレームで一致(違うのは差 1 の数フレーム)= 決定的。さらに、cpuid(87 回)と `IsProcessorFeaturePresent` を wasm の runtime と同じ答え(SSE2 だけ)に差し替える shim を入れた Wine(E2b)でも、wasm との差は **RMS 2,031.3**(shim なしの 2,031.2 と同じ)。shim は出力を動かした(対 hide1 で RMS 16.3 = 約 −62 dB)が、残差の原因ではない。= wasm との残差は**系統的な差**で、Wine の揺れ・スレッド数・CPU 機能の経路では説明できない。退避前の Wine 対 Wine の差(RMS 29)は内蔵プラグイン側の揺れ。
4. **残差は『mp3 パッドのクリップの伸縮』に絞れた(実測、§2f。alpha1 の変種 FLP を使った)**: その伸縮をなくした変種(nostretch-mp3pad / nostretch-all)で、wasm と Wine の Export の差は **RMS 2,031 → 2.1(約 −80 dB、最大の差 31 / 20)**。別のクリップの伸縮をなくした変種(nostretch-pad)では 2,030.8 のまま動かない。= 残差の原因は mp3pad の伸縮(Elastique の経路、または mp3 のデコード + 伸縮の経路)。
4b. **残差の入口は『mp3 の復号結果(clip の int16 PCM)が Wine と wasm で違う』ことだった(実測、§2k・§2l。01:00〜01:25)**: FL は MP3 を ACM(msacm32)でしか復号できず(FLEngine_x64.dll の import、静的)、Wine 10.0 では内蔵の `l3codeca.acm`(Wine 同梱の mpg123 1.32.9)が応え、wasm では runtime の minimp3 が応える。ch7 の巨大 clip(n=0x8c5800 = 7,984 × 1,152 フレーム、int16 stereo、`[fadr.com] Vocals - Dot To Dot….mp3` の復号結果)の 262,144 サンプルの窓で、Wine の PCM は wasm の PCM(minimp3 の素直な出力と **0 個違いで一致**)と **105,486 個(40.2%)が違い、差は −2〜+3 LSB**。ffmpeg mp3float・libmpg123 1.33.3・minimp3 の 3 系統は互いに 1 LSB 以内(3.2% 以下)で、**Wine の復号だけが外れ値**(どの復号器ともバイト一致しない)。短い mono の clip(inst 3)と他の int16/float32 clip の窓は一致。= 入力の ±1〜3 LSB の差を Elastique の time-stretch が呼びごとに増幅して RMS 2,031 になった、という読み(増幅の機構は未確認)。**決め手の A/B は実行済み(01:44〜01:46、§2m)**: 同じ PCM を WAV で Wine の FL に読ませると、Wine と wasm E12 の Export の差の RMS が **2,031.2 → 0.96**(最大 73、全チャンネル同一のフレーム 21.07% → 80.67%)に落ちた。= 残差の全体は MP3 復号器(ACM の実装)の差を Elastique が増幅したもの。
4c. **残差は完全に閉じた(実測、§2n。03:44〜03:46、elog14)**: 入力の MP3 復号を揃え(mp3wav-all)、さらに Wine の CPUID を wasm と同じ SSE2 に揃える(prefix-e2 + 統合 shim)と、**Wine 上の元の x86 FL と wasm の lift 版の Export は、11,431,822 サンプル中 4 サンプル(すべて +1 LSB)を除いてビット一致する**(差の RMS 0.0006、最大 1、全チャンネル一致のフレーム 99.99993%)。pad の `f_a97930` の入力・出力 f32(1,647,180 語)も wasm とバイト一致、`a97930x` の h2 も一致(9f693dc37d74a97d)、Init 3,089 行の clip の疎ハッシュも全部一致。内訳: RMS 2,031.2 = Wine の MP3 復号器(l3codeca、mpg123 1.32.9 同梱)が主要な 3 系統の外れ値で、その差を Elastique が増幅したもの / RMS 約 1.0 = Wine の実 CPUID(AVX/FMA)対 wasm の SSE2 の経路での pad の float の差(相対 1.3e-5)。**どちらも lift の欠陥ではなく、外部の実装差(ACM の MP3 復号器)と実行環境の選択(CPUID の応答)**。記録に残す読み(未確認): 実 Windows(AVX2/FMA の CPU)では FL は別の ISA 経路を通るので、wasm の Export と実機の Export は pad の相対 1e-5 程度(約 −96 dB)離れる見込み。
4d. **G9-a(Fruity Balance)の Wine 側の参照(実測、§2p、05:47〜05:51、elog15b)**: Balance だけを有効にした Wine の Export〔入力 PCM と CPUID を揃えた構成〕は、wasm の G9-a の実モジュールの Export と、11.4M サンプル中 5 サンプル(+1 LSB)を除いてビット一致(RMS 0.0007)。Balance の効果の大きさは Wine と wasm で RMS が小数 4 桁まで同じ(3,986.6208)。
5. **以下は絞り込み前の候補(参考)**: lift した側の数値(x87 / libm / MXCSR / denormal の扱いなど)、読み込み / デコード / 補間の経路の差(48 kHz サンプルのリサンプリング、MP3 デコード、Elastique の伸縮/ピッチ付きクリップ)、cpuid を書き換えていないモジュール(FL64.exe、プラグイン DLL)。**wasm では未 lift の同梱 DLL(ildsp_x64、beat_tracker、LAMEEnc、ILSvg)の候補は棄却**: 退避構成の Export の間に Wine の FL が読み込んだ FL のインストール先の DLL は 13 本で、それらは含まれない(E6、§2e)。次の二分探索は Split mixer tracks(トラックごとの書き出し)。
6. **実 Windows ではない**。Wine は『正解』ではない。以上は『Wine 上の原版 FL と wasm の違い』で、lift の誤りの有無はここでは言えない。

## 1. 比べたもの

| | A: wasm E12 | B: Wine 上の原版 |
|---|---|---|
| ファイル | `lift/out-opt2/runs/e19-export/export.wav` | `~/work/opt1-wine/proj/serina1-serum2.wav` |
| 大きさ / フレーム | 22,863,828 B / 5,715,911(129.612 s) | 22,915,648 B / 5,728,866(129.906 s) |
| sha256 | `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` | `2a4d136a44573d0ee5d67b80834094b173da9ca5d72256b91a4709f67545b803` |
| 書いた時刻 | 2026-11-06 17:04:43〜17:06:05(Chrome、E12 の product bundle) | 17:56:46〜17:57:52(Wine) |
| Rendering ダイアログ | Full song、Tail = Leave remainder、108 bars、2'10"、WAV 16Bit int Stereo、32-point sinc、HQ for all plugins、Disable maximum polyphony、Trim PDC silence、Enable insert / master effects | 同じ(0298 の `logs/misc/export-dialog-wine.png` と wasm の `e19-export/export-dialog.png`) |

**どちらも『13 plugin(s) could not be opened』(Serum ほか未導入の VST)で、サードパーティの VST は鳴っていない**。

## 2. 結果(`tables/`)

### 2.1 先頭

無音の長さは A 172,955 フレーム(3.9219 s)、B 172,956 フレーム(3.9219 s)。最初の無音でないフレームから値が違う(0 フレームずらし・+1 フレームずらしのどちらでも 172,955 が最初の不一致)。`tables/0307-wasm-e12-vs-wine-time-evolution.txt`。

### 2.2 時間発展(5 s 窓、左チャンネル全帯域、±2000 フレームで最良の遅れ)

- 最良の遅れは大半の窓で **+1 フレーム**(遅れが蓄積する形ではない)。50〜55 s、60〜80 s、80〜85 s の窓だけ +152〜+234 フレームで、85 s 以降は +1 に戻る。
- 窓ごとの RMS はほぼ同じ(B/A = 0.98〜1.15、85〜90 s だけ 1.28)。全帯域の規格化相関は 0.27〜0.74。
- 60〜80 s の窓は Mid 相関 0.42〜0.48 に対し **Side 相関が 0.02 前後**(= ステレオの差成分が別物)。それ以外の窓は Side 相関 0.5〜0.82。

### 2.3 帯域別(`tables/0307-wasm-e12-vs-wine-end-and-bands.txt`、4 s 窓、B を +1 フレームずらし)

300 Hz〜20 kHz は 0.5〜0.96 で正に相関するが、20〜100 Hz は −0.24〜−0.77(90〜94 s だけ +0.69)、100〜300 Hz は −0.35〜+0.53。= 差は低域に集中している。

### 2.4 低域の遅れ(`tables/0307-wasm-e12-vs-wine-lf-lag.txt`、FFT 相互相関、6 s 窓、左チャンネル)

| 窓 | 20〜100 Hz の最良相関 @ B の遅れ(フレーム) | 300〜3000 Hz の最良相関 @ 遅れ |
|---|---|---|
| 8〜14 s | 0.982 @ +315 | 0.692 @ −1 |
| 20〜26 s | 0.969 @ +316 | 0.739 @ 0 |
| 30〜36 s | 0.847 @ +313 | 0.865 @ 0 |
| 45〜51 s | 0.980 @ +318 | 0.650 @ 0 |
| 62〜68 s | 0.840 @ +193 | 0.819 @ 0 |
| 70〜76 s | 0.809 @ +192 | 0.789 @ 0 |
| 90〜96 s | 0.904 @ +849 | 0.500 @ −1 |
| 110〜116 s | 0.930 @ +210 | 0.814 @ 0 |

(「B の遅れ」は、Wine の低域が wasm の低域より何フレーム後にあるか。44.1 kHz で 315 フレーム = 7.1 ms、192 フレーム = 4.4 ms。90〜96 s の +849 は低域のエネルギーが小さい窓の曖昧な最大で、信頼しない。)

**読み**: 低域だけがほぼ同じ波形のまま数 ms ずれている。形としては『ある経路(低音のトラックなど)のレイテンシ補償(PDC)の量の違い』か『低域に位相の回る処理の有無』に見える。**どのトラック・どのプラグインかは未特定**。Wine 同士で再現するかを先に見る。

### 2.5 末尾(12,955 フレーム = 0.2938 s)

`tables/0307-wasm-e12-vs-wine-end-and-bands.txt`: 10 ms ごとの RMS は 129.59 s までほぼ一致(数 % 以内)。wasm は 129.600 s で打ち切られる(最後の 0.1 s の RMS は 11,722、信号が大きいところで終わる)。Wine は同じ点から減衰が続く(RMS 約 2,900 → 129.84 s で 3.0)。Wine の余分な 12,955 フレームのうち 11,983 が非ゼロ、ピーク 4,072、最後の 0.1 s の RMS は 4.7。曲の長さ 108 小節(200 bpm)= 5,715,360 フレームに対し、A は +551 フレーム、B は +13,506 フレーム。

**読み**: Wine の余分な部分は『曲の終わりの後の余韻』。wasm の WAV にはそれが無い。Tail = Leave remainder の設定は同じなので、余韻を書き出すかどうかは FL が実行時に決めている(未確認)。

### 2.6 クリップ

どちらも ±32767 に張り付く: +32767 が A 1,277,984 / B 1,352,207、−32767 が A 1,287,081 / B 1,368,466(+32767 と −32767 を合わせて、全サンプル 11.43 M〔5,715,911 フレーム × 2 チャンネル〕のうち A は約 22.4%、B は約 23.7%)、−32768 は 0。= 両方とも強く歪んでいるので、振幅や位相のわずかな違いが波形の相関を大きく下げる(2.2 の 0.3〜0.6 はこの影響を含む)。

## 2b. 追加の観察(18:25〜18:45): Wine 対 Wine と、FL 内蔵 8 種の退避

### 2b.1 Wine 対 Wine(FL 内蔵プラグインあり、Export 2 本。`tables/0307-wine-run1-vs-runA-WITH-fruity-*.txt`)

同じ設定で、FL を起動してプロジェクトを開いた Export 2 本(run1 = 前の再生のあとの同じセッション、runA = 起動直後の新しいセッション)。sha256 `2a4d136a…b803` 対 `57f78042…34d1`: **同じ大きさ 22,915,648 B・同じフレーム数 5,728,866 だが、ビット同一ではない**。差のあるフレーム 4,654,163(81%)、最大の差 303(満尺の 0.9%)、差の RMS 29.2(信号の RMS 21,041 の約 −57 dB)。最初に違うフレームは 210,771(4.78 s)。全 2 s 区間の規格化相関は 1.000(遅れ 0)、低域(20〜100 Hz)も中域も 8 窓すべてで 1.000 @ 遅れ 0。**余韻(曲の終わりの後の 13,506 フレーム)は再現する**(最後の非ゼロフレーム 5,728,364 対 5,728,362)。= Wine 対 wasm の差(RMS 23,654)は Wine どうしの差の約 800 倍で、**低域の遅れと余韻は FL 自身の揺れでは説明できない系統的な差**。

### 2b.2 内蔵 8 種の退避(alpha1 の読みを検証。`tables/0307-*HIDDEN8*`)

- **alpha1 の読み(PM 経由、未確認だった)**: wasm は参照プロジェクトを『8 plugin(s) could not be opened』で開く。8 行は FL 自身の物(Fruity Wrapper〔サードパーティ VST 全部〕、FLEX、Frequency Shifter、Fruity Balance、Soundgoodizer、Fruity Parametric EQ 2、Fruity Delay 2、Fruity Filter)で、出荷の volume に Plugins\Fruity の DLL が無いため。Wine の FL は 13 plugin(s)(サードパーティの種類)だけが開けず、内蔵 8 種は開いて鳴らす → 仕事量が違う。
- **実験**: Wine のインストール先の `Plugins/Fruity` の 9 フォルダ(Effects の Fruity Parametric EQ 2・Soundgoodizer・Fruity Delay 2・Fruity Filter・Fruity Balance・Frequency Shifter・Fruity Wrapper、Generators の FLEX・Fruity Wrapper)に末尾 `.off-0307` を付けて rename(削除なし。復元は `tools/hide-fruity.sh restore`、ログ `logs/wine-sessions/hide-fruity.log`)。
- **Problems 窓は "8 plugin(s) could not be opened"**(退避前は 13。`logs/wine-sessions/hide1-problems-dialog-8-plugins.png`)。窓の一覧部分は空。『N sample(s) required by this project could not be located』の行は Wine 側には出ていない(wasm は『2 sample(s)』と PM から聞いたが、wasm の窓の画面との比較は未実施)。
- **Export(hide1、FL 起動からプロジェクトを開き終えるまで 31.5 s、Start から 27.0 s)**: **22,863,828 B・5,715,911 フレーム・最初/最後の非ゼロフレーム 172,955 / 5,715,730 が wasm E12 の WAV と同じ**(退避前の Wine は 22,915,648 B・5,728,866 フレーム)。1 秒ごとの RMS の相関 1.0000、平均 RMS 19,784 で同じ、**全 25 窓(5 s)で遅れ 0、B/A の利得 0.999〜1.001**、全帯域の規格化相関 0.937〜1.000(100〜120 s・20〜40 s は 1.000)。末尾は曲の終わり(5,715,360)以降の 551 フレームだけで、そのうち違うのは 14 フレーム。クリップの数もほぼ同じ(+32767 が 1,277,984 対 1,277,616)。
- **= 低域の遅れ(4.4〜7.1 ms)も、余韻の有無も、FL 内蔵 8 種の有無で説明できる**。wasm と Wine の食い違いの大半は『プラグインの有無』で、非決定性・runtime の忠実性の問題ではない。
- **ただしビット一致ではない**(sha256 `bd917194…6cad` 対 `381cdb5a…f979`): 差のあるフレーム 4,511,490(79%)、最大の差 36,664、差の RMS 2,031(退避前は 23,654)。差は区間に偏る: 5〜20 s、45〜60 s、80〜95 s、120〜130 s は 5 s 窓の RMS が 1,000〜5,100(相関 0.937〜0.995)、20〜40 s・100〜120 s は RMS 150〜260(信号の約 1%、相関 1.000)。`tables/0307-wasm-e12-vs-wine-HIDDEN8-hide1-diff-by-window.txt`。
- **残差の局所化(オフライン、相関であって原因ではない。`tables/0307-residual-hide1-vs-wasm-localization-by-clip.txt`)**: 差の RMS が 1,000 を超える 54 秒に鳴っている物の上位は、パターン 1(54 秒中 30 秒に、差の小さい 61 秒のうち 4 秒に)、`Synthion Downlifter 2.wav`(48 kHz・24 bit。28 対 4)、パターン 3、`Synthion Clap Impact.wav`(48 kHz・24 bit。18 対 2)、`celtix_Albiorix_Snare_20.wav`(24 bit。16 対 0)、`car crash and siren.wav`、`windchaim.mp3`。= **48 kHz のサンプル(リサンプリングあり)と、パターン(Sampler など)の区間に差が出る**。2 つの Vorbis サンプル(`anvil_land.ogg` 45.3 s、`dltzk glass break 1.wav` 22.8 s と 99.6 s)の位置は特に目立たない(差の RMS 1,929 / 56 / 97。直前の 0.5 s と同じ程度)。
- **性能(参考、退避構成 = 仕事量が wasm に近い、開始 load 3.3 前後、nice 19)**: Export の壁時計 27.0 s = 実時間の 4.81 倍。CPU は spinner(98%)と UI を除く合計 19.45 s = 音声 1 秒当たり 0.150 CPU 秒 = **92.88 ms ブロック当たり 13.9 ms**(退避前は 36.9)。= 内蔵エフェクトが Wine の Export CPU の約 62% を占める。

## 2c. E1: 論理 CPU 数を 1 にした Export(退避構成。`tables/0307-E1-hideT1-1cpu-vs-hide1-16cpu-and-wasm.txt`)

`WINE_CPU_TOPOLOGY=1:0` で FL を起動して同じ設定で Export(nice 19、FL 起動から読み込み完了まで 39.5 s、Export の壁時計 29.5 s、開始 load 7 前後)。`hideT1.wav`(sha256 `02b7f988…376d`)は、16 CPU の `hide1.wav`(`bd917194…6cad`)と **5,715,911 フレーム中 5,715,907 が一致(違う 4 フレームは差 1)**。wasm との差は hide1 と同じ(一致 1,204,421 フレーム、RMS 2,031.2、最大 36,664)。= スレッド数・論理 CPU 数は残差の原因ではない。

> **訂正(11-07 10:2x、opt1)**: この §2c の『論理 CPU 数 1』は**実際には 1 になっていなかった**。この Wine 10.0(Ubuntu 10.0~repack-12ubuntu1)には `WINE_CPU_TOPOLOGY` という文字列が Wine のどのファイルにも無く(`grep -rl` で 0 件)、`WINE_CPU_TOPOLOGY=1:0 wine stub/sysinfo.exe` でも `GetSystemInfo.dwNumberOfProcessors` は 16 のままだった。したがって hideT1 は『同じ条件の再実行』で、『プロセッサ数を変えても Export は変わらない』の根拠にならない(冒頭 §0 の 3.『スレッド数でもない』の根拠も同様)。プロセッサ数 1 の最初の実測は 0320 §17(`LD_PRELOAD` の `stub/libfakecpu.so` で sysconf と /sys/devices/system/cpu/{present,online} を差し替え、sysinfo で 1 を確認した上での ins1-lp の Export)で、結果は『変わらない』。

## 2d. E2 の shim(cpuid と IsProcessorFeaturePresent を wasm と同じ答えにする。測定専用)

- **目的**: Wine の原版 FL を『SSE2 だけの CPU』に見せ、残差(RMS 2,031)が消えるか縮むかを見る。消える/縮む(目安 RMS < 100)なら、残差は IPP / Elastique の CPU 経路の差で、lift の誤りではない。縮まなければ、alpha1 の次手(Elastique 単体の harness)。**結果は縮まなかった(§2e)**。
- **手段**(測定専用。元の prefix・元のファイルは触らない):
  1. `prefix-e2`(元の prefix の複製、5.2 GB)の中だけで、alpha1 の `a22-cpuid-sites.py`(capstone で verified)が出した cpuid の位置のうち、FL64.exe 以外の 7 DLL の **388 箇所**(FLEngine 5、Blend2D 12、dsp_ippv2 267、elastique 50、ILTools 5、oggio 44、QuickFontCache 5)を `0F A2` → `0F 0B`(UD2)に書き換える(`tools/e2-patch.py`、ログ `logs/e2/patch.log`)。xgetbv(25 箇所)は書き換えない(cpuid が OSXSAVE = 0 を返すので実行されない前提)。FL64.exe の cpuid 4 箇所も書き換えない(wasm は FL64.exe を実行しない)。
  2. 複製の FL ディレクトリの `wintrust.dll` を `tools/wine-stub/wintrust_cpuid.c` に差し替える(WinVerifyTrust の stub は同じ)。DllMain で **VEH** を入れ、UD2(`0F 0B`)が上の 7 DLL の中で起きたら、wasm の `fle_cpuid()`(`lift/runtime/fle_cpuid.c`、決定 0009 / 0118)と同じ値を返して RIP を 2 進める: leaf 0 → EAX = 1、他 0。leaf 0x80000000 以上 → 全部 0。それ以外の leaf(1、2、4、7、0xD を含む。サブリーフ無視)→ EAX = EBX = ECX = 0、EDX = 0x07808101。DllMain で **KUSER_SHARED_DATA の ProcessorFeatures**(0x7ffe0274、BOOLEAN[64])を、wasm の `fle_imp_IsProcessorFeaturePresent`(`lift/runtime/fle_sysinfo.c`)と同じ答え(PF 2、3、6、9、10、23 = 1、他は 0)に書き換える(Wine の kernelbase はこれを読む)。
  3. **FL64.exe の import 順**は KERNEL32、USER32、ADVAPI32、SHELL32、**WINTRUST**、CRYPT32(静的)で、`+loaddll` の以前のログでは WINTRUST.dll(行 189)が FLEngine_x64.dll(行 236)より先に読み込まれる = VEH が最初の UD2 より前に入る。
- **単体試験(Wine、CPU 1 本に固定、nice 19、15 s、`tools/wine-stub/e2test.c`)**: 実測で `IsProcessorFeaturePresent(0..45)` は 2、3、6、9、10、23 だけ 1。UD2 を入れた stand-in の `oggio_x64.dll` に leaf 0 / 1 / 2 / 4 / 7 / 0xD / 0x80000000 / 0x80000001 を投げた応答は、上の値と一致(`logs/e2/e2test-output.txt`)。
- **実行結果(§2e)**: FL 本体で `prefix-e2` を使った Export を実行した。


## 2e. E6 と E2 の結果

**E6(Export の間に読まれた DLL。`logs/e6-loaddll-summary.txt`、`tables/0307-E6-*`)**: 退避構成の Export(+loaddll、16 CPU、壁時計 34.0 s)で FL のインストール先から読まれたのは 13 本: FL64.exe、WINTRUST(shim)と wintrust_real、FLEngine_x64、dsp_ippv2_x64、Blend2D、QuickFontCache_x64、freetype_x64、ILTools_x64、WebView2Loader、oggio_x64、Elastique_x64、WavpackDLL_x64。**ildsp_x64・beat_tracker・LAMEEnc・ILSvg は読まれない**。wasm が lift する実モジュールは dsp_ippv2・Blend2D・QFC・freetype・ILTools・oggio・elastique の 7 つで、Wine が読むのはそれに WebView2Loader と WavpackDLL が加わる(wasm 側のこの 2 本の扱いは alpha1 が確認)。= 『wasm で NULL になる同梱 DLL が Wine では読まれる』という残差の候補は Export では効かない(E4 / E5 は不要)。この run の WAV(`e6.wav`、sha256 `513c19ac…6c85`)は hide1 と 5,715,911 フレーム中 5,715,908 が一致、wasm との差は hide1 と同じ(一致 21.1%、RMS 2,031.2)。

**E2(cpuid と IsProcessorFeaturePresent を wasm と同じにした shim。`tables/0307-E2*`、`logs/e2/`)**:

- **E2a(失敗)**: 最初の run は、`run-fl.sh` が `env.sh` を再読み込みして元の prefix で動き、shim が入っていなかった(`wineserver -k` は prefix-e2 の方を止めたので、元の prefix の FL64.exe が 1 分ほど残った)。結果は hide1 と 5,715,909 / 5,715,911 一致で、退避構成の 4 本目の再現性の証拠にはなる。`run-fl.sh` が `OPT1_ENVF` を読むように直した。
- **E2b(prefix-e2、shim 入り)**: FL は起動してプロジェクトを開き(読み込み完了まで 40 s)、Export を最後まで書いた(22,863,828 B、壁時計 33.5 s)。**shim のカウンタ**(`logs/e2/shim-counts-e2b.txt`): FL64.exe の 1 プロセスで cpuid の UD2 を **87 回**処理(leaf 0 が 32、leaf 1 が 31、他の basic leaf が 18、extended が 6)、他の UD2 の通過は 0。
- **結果**(`e2b.wav`、sha256 `458f69b2…363b`): wasm E12 との差 = 一致 1,252,676 フレーム(21.9%)、最大の差 36,662、**差の RMS 2,031.3**(hide1 の 2,031.2 と同じ。10 s 区間ごとの RMS も同じ: 1499, 2511, 420, 221, 1982, 2807, 1060, 1010, 4115, 3607, 187, 237, 862)。hide1 との差は 一致 81.1%、最大 2,260、RMS 16.3(区間別 1, 5, 0, 0, 0, 0, 58, 1, 5, 0, 0, 0, 5。60〜70 s だけ 58)。
- **読み**: CPU 機能を SSE2 だけに見せると出力は信号の約 −62 dB だけ動く(経路は効く)が、wasm との残差(約 −20 dB)は縮まない。**残差の原因は CPU 機能の経路ではない**。未確認: cpuid を書き換えていないモジュール(FL64.exe、プラグイン DLL)、xgetbv(OSXSAVE = 0 なので実行されない前提だが、踏まれたかは未確認)、`IsProcessorFeaturePresent` を KUSER_SHARED_DATA の書き換えで済ませた点(UCRT が別の経路で機能を調べていないかは未確認)。

## 2f. 変種 FLP の Export(alpha1 の依頼。`tables/0307-variants-*`)

alpha1 の変種 FLP 3 つ(`/home/ubuntu/work/alpha22w/flp-variants/{nostretch-pad,nostretch-mp3pad,nostretch-all}.flp`、元と同じ 1,322,619 B)を、退避構成の Wine で Export(`proj/serina1-serum2.flp` を差し替え、1 本ずつ、nice 19、`tools/wine-variants.sh`。元の FLP は復元して sha256 を確認)。Wine の WAV は `~/work/opt1-wine/exports/runs/<変種名>.wav`(out-git には置かない)。alpha1 が node で取った wasm の同じ変種の WAV(`/home/ubuntu/work/alpha22w/runs/exp-<変種>/dump/00-serina1-serum2-export.wav`、読み取りのみ)との差:

| 変種 | フレーム(wasm 対 Wine) | 一致 | 最大の差 | 差の RMS |
|---|---|---|---|---|
| 元(Wine hide1 対 wasm E12) | 5,715,911 対 5,715,911 | 21.1% | 36,664 | **2,031.2** |
| nostretch-pad | 5,715,911 対 5,715,911 | 21.1% | 36,664 | **2,030.8** |
| nostretch-mp3pad | 5,748,435 対 5,748,435 | 38.0% | 31 | **2.1** |
| nostretch-all | 5,748,435 対 5,748,435 | 39.1% | 20 | **2.1** |

(mp3pad と all の変種は、伸縮をなくした結果、元より 32,524 フレーム長い。変種を元と比べた差〔Wine の変種 対 Wine の元 hide1〕は nostretch-pad RMS 5,759、mp3pad 14,595、all 17,488 で、変種が音を変えている大きさ。残差とは別の物。)

**読み**: wasm と Wine の −20 dB の残差は、mp3 パッドのクリップの伸縮(stretch)をなくすとほぼ消える(RMS 2.1 = 約 −80 dB、16 bit の最下位ビット付近)。別のクリップ(pad)の伸縮をなくしても動かない。= 残差の原因は mp3pad の伸縮の経路(Elastique の処理、または mp3 のデコードと伸縮の組合せ)で、他の経路(CPU 機能、DLL の読み、スレッド数、リサンプリング、他のクリップ)は実質的に関係しない。どの段階が違うか(mp3 のデコード / Elastique の入力 / 出力)は未特定: alpha1 の次手(Elastique 単体の harness)。

## 2g. Wine 側の記録の設計(未実行。PM の宿題への答え。`tools/wine-stub/wintrust_hook.c`、コンパイルのみ)

- 目的: 残差が mp3 パッドの伸縮(FLEngine の `f_a981f0` が `f_a97930` を何回か呼んで行うクリップ全体のオフライン事前レンダリング)にあると絞れたので、その入口と出口の入力 PCM・出力 PCM・引数を Wine で記録して、wasm 側(alpha1 が生成 C にログを足す)と比べる(入力が同じで出力が違えば Elastique の算術の差、入力が違えば上流の差)。
- 読んだ事実(FLEngine_x64.dll を capstone で逆アセンブル、実行なし): ImageBase は 0x400000、`f_a97930` = VA 0xa97930 = RVA 0x697930。入口は `push rbp; push rbx; sub rsp,0xd8; mov rbp,rsp; mov [rbp+0x28],rbp`(先頭 12 バイトが 4 命令の境界、相対分岐も RIP 相対もなく、先頭 40 バイトに再配置なし)。呼び元 `f_a981f0` の 0xa98449 の `call` の前で、rcx = this([rbp+0x70])、rdx = [クリップ+0x8c](結果の置き場所と読める)、r8 = [rbp+0x58]、r9 = rax、5 番目 = [rbp+0xa0](新しい長さ)、6 番目 = double の比、7 番目 = dword(チャンネル数 [クリップ+0x74])、8 番目 = byte(旗 [クリップ+0x75])を渡す(意味は推定)。
- 方式: shim の中だけ、C だけ。ウォッチャのスレッドが FLEngine_x64.dll を待ち、各関数の先頭 1 バイト(0x55)を 0xCC にする。VEH が入口の INT3 で引数・バッファのハッシュを書き、戻りアドレス `[rsp]` を shim 内の出口用 INT3 に差し替え、元の最初の命令 `push rbp` を代行して続行。出口の INT3 で結果を書いて元の戻りアドレスへ戻る。出力は `HOOK_LOG` + `.<pid>` に 1 呼び出し 1 行。PM の仕様: 記録点 = `f_a97930` の入口/出口 + `f_a981f0` の入口 + `f_a97780` の入口、FNV-64 の対象は生バイトの全長(フレーム数 × チャンネル × 要素サイズ)、double は生の bit、固定形式 `<site> <call#> <thread> <args hex...> in=<hash> out=<hash> n=<bytes>`(形式は alpha1 と合意してから)。
- 見積り: 実装と単体試験 約 1 時間、Wine の試走 2〜3 回(各 約 2 分)と形式合わせ 約 1 時間。例外が `f_a97930` の中を通り抜ける場合は戻りアドレスの差し替えで巻き戻しが外れる(最初の試走で入口/出口の件数が一致するかで確認)。

## 2h. ELOG の初回(Wine、退避構成、Export 1 本。23:32〜23:34、`logs/hook/elog-wine-hide8-export-1.txt`)

- 6 か所すべての hook が入り(FLEngine のロード先 0x7a730000)、FL は起動してプロジェクトを開き Export を最後まで書いた(クラッシュなし)。shim は元の `wintrust.dll`(stub)と差し替えて実行し、終了後に復元(sha 先頭 `9e5084c18ccb8f54`)。元の FLP も復元(`d6783e08886b4b48`)。
- 件数: `a97930` 入口 1・出口(`a97930x`)1(例外の通り抜けなし)、`a981f0` 1、`a97780` 1、`677fb0` 1、`677f60` 0、**`678010` 1,254**。
- `a981f0`(クリップ p=a617810): len = n = 0xc9126(823,590 フレーム)、ratio = 0x3ff74a5b00000000(1.4556531906…)、mode = 5(Auto)、ch = 2、sr = 44100.0。`a97780`: mode = 1、edx = 0x100、r8d = 2、d3 = 44100.0。`677fb0`(Elastique v3 の CreateInstance): ecx = 0x100、edx = 2、f2 = 44100.0f、r8 = 1、r9 = 0、f5 = 0.1f。`a97930`: n1 = n2 = 0xc9126、ch = 2、flag = 1、入口 h1 = 出口 h1 = `66151d6726caba6b`、出口 h2 = `e3398e3c97f5b612`(入口 h2 は未初期化の出力)。
- **`678010` の 1,254 回**: edx = 2 が 1,210・1 が 44、f2 = 44100.0f が全部、r8 = 2 が 1,210・1 が 44、r9 = 0xffffffffffffffff が 742・0x5377ca0 が 512、ecx は 14 種のポインタ。**読み(未確認)**: ch7 / ch35 の伸縮は `f_a97930` の事前レンダリングではなく、`f_678010` が作るソース(ResampleSource の呼び)経由で行われている可能性が高い。`f_a97930` を通る 1 回は mode Auto の別のクリップ。記録点を `f_678010` が作る物の処理側(Elastique の ResampleSource の process 呼び)に移す必要があるかもしれない。alpha1 の fle-03.c の読み待ち。
- 比較 script `tools/elog-compare.py A.elog B.elog`(関数ごとに呼び出し順で対応づけ、`p=` と `678010` の `ecx=` を除いて、全キー一致の呼び出し数とキー別の一致数を出す)を用意。wasm 側の ELOG(alpha1)が出たら流す。

## 2i. ELOG の第 2〜3 版 shim(Wine、退避構成、Export 各 1 本。`logs/hook/`、`tools/wine-stub/wintrust_hook.c`)

- **第 2 回(23:42〜23:43、`elog-wine-hide8-export-2.txt`)**: elastique_x64.dll の export 2 本(`ElResampleSource_CreateInstance` = RVA 0x285320、`ElProV3_CreateInstance` = RVA 0x285040)の入口/出口を足した。先頭命令が push でないので『先頭バイトを戻して 1 命令を単一ステップし、INT3 を戻す』方式。全 8 か所が入り、FL は最後まで走り、Export の WAV は hide1 と 5,715,911 フレーム中 5,715,906 一致(最大差 1、RMS 0.0 = shim は出力を動かさない)。`ElResampleSource_CreateInstance` は入口 1,254・出口 1,254(f_678010 の 1,254 回と 1 対 1)、`ElProV3_CreateInstance` の export 入口は 0 回(f_677fb0 は別の経路で作る)。FL 側の 6 か所は第 1 回と全キー一致。
- **alpha1 の訂正(23:52)**: 0308b §6 の『ch7(−2)・ch35(−1)は ElProV3・旗 0 に行く』は誤り。f_a97780 / f_a981f0 / f_a97930 の経路は Auto(5) のパッドで(Wine では Export 全体で 1 回)、ch7 / ch35 の伸縮は `f_678010` = `ElResampleSource_CreateInstance` の経路。ResampleSource の処理は elastique_x64.dll の RVA 0x285460(`this=rcx, rdx=バッファ A, r8=バッファ B, r9d=n`、n 個の float)と読まれる。
- **第 3 版(23:49〜23:50、`elog-wine-hide8-export-3-HEAD-ONLY.txt` は先頭 40 行だけ。全体は 100,000 行で打ち切られた)**: hook を 12 か所に(f_67db90・f_67d9b0・f_67dd90・f_67ddf0 の double、`ElResampleSource_CreateInstance` の戻り値に生成順の番号 `inst=`、elastique の RVA 0x285460 の入口/出口、f_678010 の xmm3 とスタック 5〜8 番)。first instruction の代行は push r64 / push r8-r15 / `sub rsp,imm8` / `mov [rsp+disp8],r64` を shim 内で行い(単一ステップで byte を戻す窓をなくし、複数スレッドが同時に入っても取りこぼさない)、書式を `ELOG b285460 seq inst= ic= n= h1= h2=`(`ic` = そのインスタンスの何回目の呼びか)、`b285460x`、`b285320x seq obj= inst=` にした。**問題**: 全体 100,000 行の上限を f_67d9b0(73,861 行)と f_67db90(25,390 行)が使い切り、b285460 は 0 行・f_678010 は 244 行で切れた(f_67d9b0 / f_67db90 は 1 クリップのブロックごとに呼ばれる)。**直した**: 全体 400,000 行、f_67db90 / f_67d9b0 / f_67dd90 / f_67ddf0 は先頭 5,000 行ずつ、b285460 は先頭 10,000 呼び出し(入口 + 出口で 20,000 行、入口と出口を対にする)。再実行は窓待ち。
- 比較 script `tools/elog-compare.py A.elog B.elog [--keyby b285460,b285460x] [--ignore p,ecx@678010] [--alias B のキー=A のキー]`(値は小文字・0x なし・先頭の 0 なしに正規化。`--keyby` はスレッドの割り込みに依らないよう (inst, インスタンス内の n 回目) で対応づける)。

- **既存のログだけでの確認(23:58〜00:02)**: (a) a97930 の h1 / h2 の長さは、私の shim も alpha1 の wasm 側も先頭 1 MiB(262,144 float、n1×ch×4 = 6,588,720 バイトのうち)で同じ。入口の h2(呼びの前の出力バッファ、ゼロ埋め)が両側で `a96777069d622325` と同じ値になることが、範囲の長さが同じことの裏づけ。出口の h2 の不一致(Wine `e3398e3c97f5b612` 対 wasm `9f693dc37d74a97d`)は比較条件の違いではない。(b) Wine の出口 h2 は 3 回の Export で毎回 `e3398e3c97f5b612`(決定的)。(c) `f_678010` の呼び出し列 (edx, r8, f2) を Wine(1,254 回)と wasm(alpha1 の exp-elog1、1,252 回)で整列すると、残り 1,252 回は完全に一致し、**Wine にだけ『最初の 1 回』と『最後の 1 回』(どちらもステレオ)が余分にある**(wasm はプロジェクトの最初と最後に ResampleSource を作るクリップが 2 つ少ない。どのクリップかは未特定)。
- **第 4 版 shim(`wintrust_hook.c`、コンパイル済み・Wine での試験は窓待ち)**: ① elastique のロードを `LdrRegisterDllNotification` で同期に検出して hook(ElProV3 の生成が 20 ms の監視スレッドより先に終わる競合を避ける)。② 追加の hook: `ElProV3_CreateInstance`(RVA 0x285040、入口 `ELOG b285040 seq p= ch= f2= f3= f4=`、出口 `b285040x seq obj= inst=`)、ラッパ vtable の入口候補 RVA 0x285020 / 0x284c60 / 0x284fe0(`ELOG <fn> seq inst= rdx= r8= r9= s5= s6=`)、ResampleSource の設定 thunk RVA 0x284700(`ELOG b284700 seq inst= d1= d2= d3=`)。先頭命令 `mov rcx,[rcx+8]` の代行を追加。③ pad の最初の `f_a97930` で、入口の rdx から n1×ch×4 バイト、出口の r8 から n2×ch×4 バイトを `<HOOK_LOG>.<pid>.a97930-in.f32` / `.a97930-out.f32`(生の float32、各 約 6.6 MB)に書く。④ `inst` の番号は ResampleSource と ElProV3 で別々の生成順。

## 2j. Elastique の記録(第 5〜9 回の ELOG、00:20〜00:48。`logs/hook/`、`tables/0307-elastique_x64-import-and-instruction-census.txt`)

- **elastique_x64.dll の import と命令の census(宿題 A)**: import は KERNEL32 77 関数と WS2_32(ntohl・htonl)だけで、**数学関数の import は 0 件**(CRT は静的リンク)。命令 833,531 個・613 種類: x87 の超越関数(fsin 等)は 0、x87 一般 1,548(fldcw 149・fnstcw 66)。**近似命令: rsqrtps 731・rcpps 118・rsqrtss 48(SSE)、vrsqrtps 32・vrcpps 25(AVX)、vrcp14ps 14・vrsqrt14ps 2(AVX-512)**。丸めモード依存の変換 2,919(cvtdq2ps 1,417 …)。制御語に触る命令 415(fldcw 149・ldmxcsr 112・stmxcsr 87)。FMA 26,186、VEX(FMA 以外)198,034、cpuid 50・xgetbv 7(CPU 機能による多重版)。= 外部 API(数学関数)の実装差の経路はこの DLL には無く、差は命令の意味・CPU 分岐・MXCSR/x87 制御のどれか(alpha1 の宿題 B と合う)。
- **ElProV3(Auto の pad)**: `ElProV3_CreateInstance`(RVA 0x285040)は 1 回(`p=100 ch=2 f2=44100.0f f3=0 f4=0.1f`、戻り obj)。**この生成の入口を取るには、elastique のロードを `LdrRegisterDllNotification` で同期に検出する必要があった**(20 ms の監視スレッドでは、生成が `f_677fb0` の中の LoadLibrary の直後に走るため間に合わない。LDR_DLL_LOADED_NOTIFICATION_DATA の名前は UNICODE_STRING への**ポインタ**であることを最初の版で取り違えて、通知が効かなかった)。pad の `f_a97930` の中で、ラッパの slot +0x28(RVA 0x284c10)が 3,203 回(256 フレームの入力ブロックを渡す feed の呼び: `rdx` = フレーム位置 0x200〜0xc8100、n = 0x100、呼びの前後でバッファ不変)、+0x8(RVA 0x284700)が 5,000 回以上、+0x40(RVA 0x284c60)が 1 回。+0x10・+0x38・ResampleSource の +0x40(RVA 0x285460)は 0 回。
- **ResampleSource の process は RVA 0x284e60**(alpha1 が内部実装 RVA 0x2abc20 と特定): `this=rcx, rdx=float**(チャンネルごとのポインタ), r8d=n フレーム`。外側のラッパの呼びだけ(内側の転送で同じ関数が 2 回呼ばれる)を、alpha1 の書式 `ELOG b284e60 seq inst= ic= n=`(+ `cw= mx=`)/`ELOG b284e60x seq inst= ic= n= ch= h=`(h = FNV-1a 64、ch0 から連結、各 n×4 バイト)で記録した(先頭 8,000 呼び出し。`elog8.elog`・`elog9.elog`)。n の分布: 0x8a が 6,248 回、0x89 が 1,445 回、他は少数(0x76 など)。最初の 8 インスタンスの最初の 20 呼びの出力は `elog8.rs-<inst>-<ic>.f32`(120 ファイル、約 117 KB)。
- **FPU の制御状態(PM の方針 (A))**: 最初の hook に入った 19 本の thread すべて、a97930・b285320(1,254 回)・b284e60(8,000 回)の入口と出口で、**x87 制御語 = 0x037F、MXCSR = 0x9FE0**(一部 0x9FF0 = underflow の sticky フラグだけ)。入口と出口の MXCSR が違った b284e60 の呼びは 0 件。wasm 側(alpha1: x87 CW 0x037F、エンジンは MXCSR 0x9FE0 でほぼ常に走る)と同じ。= OS が thread に与える初期状態の契約の不足ではない。
- **f_678010 の呼び列**: Wine 1,254 回、wasm(exp-elog2)1,252 回で、整列すると残り 1,252 回は 8 引数すべて一致し、Wine にだけ最初の 1 回(同じ ecx の連続 2 回)と最後の 1 回がある。最初の 678010 は pad の `a97930x` の直後(プロジェクトの読み込み中)で、最後の 1 回は通常の使い回し。理由は未確定(alpha1 が `f_67de50` の 1 本の道として読んでいる)。
- **pad の出力**(PM の numpy): 入力 1,647,180 語は全語 bit 一致、出力は 99.5% の語が違い、最初の語から 1 ulp、差の rms −96.6 dB。E2b(ISA の経路を SSE2 に揃えた)が hide1 から動かしたのは −62 dB で、ISA の経路や rcpps/rsqrtps の実装差の寄与は −62 dB 以下(PM の判断)。残差の本体(ch7 の位相 0.2〜0.5 rad)の候補は time-stretch の入力ブロックの切り方で、次は alpha1 の第 3 版の `(inst, ic)` ごとの n の列と h・dump との突き合わせ(`tools/elog-compare.py --keyby b284e60,b284e60x`、`tools/rs-compare.py`)。

## 2k. 第 4 版 ELOG(Wine、退避構成、Export 各 1 本。elog10〜elog12、00:55〜01:06。`tables/0307-elog12-*`、`logs/hook/elog12-*`、`tools/wine-stub/wintrust_hook.c`)

- **フック**: `f_67d630`(clip の読み出し。`this=rcx, pos=rdx〔int64〕, n=r8d, float**=r9`、戻り値なし、n フレームを全部書く)の入口/出口(最初の 30,000 呼び。出口で `ch0 の n float → ch1 の n float` の FNV-1a 64 と、外側の b284e60 の中の呼びのダンプ `rin-<inst>-<ic>.f32`)と、`f_67de50`(provider の Init。`this=rcx, clip=rdx, flag=r8b`。alpha1 の書式)。Init の行に `p clip f ra n ch es h sr r9 s5 s6 m38 ma0 ma1 me4 mf8 c74 c75 c7c`、最初の 6 個の clip に `ELOG clipdump k clip= off= len=` と `clip-<k>.f32`(窓: len = min(512 KiB, bytes)、off = bytes ≥ 4.5 MiB なら 4 MiB、bytes > 512 KiB なら bytes − 512 KiB、それ以外は 0)。
- **clip の形式(実測)**: `[clip+0x75]` が 1 なら float32、0 なら **int16**(interleaved)、`es = (c75 == 1) ? 4 : 2`、`bytes = n*ch*es`。elog11 は alpha1 の最初の前提(float32 固定)のまま `bytes = n*ch*4` で、int16 の clip の疎ハッシュが 0(読めない)や後ろのごみを含んでいた = **elog11 の h は無効**。elog12 で直した(alpha1 も同じ直しを入れた)。窓の中身: int16 の clip を float32 として読むと NaN だらけ(NaN 8,504〜53,914 個/131,072)で、int16 として読むと音(最初の 6 clip のうち c75=0 の 4 個)。
- **Init 3,091 回**(ra はすべて d283c6): m38=0 が 1,837、m38=1 が 291、m38=2 が 963。**ResampleSource の作成(b285320)1,254 回 = m38≥1 の Init 291 + 963 に完全一致**(m38=0 では作られない)。s5 は m38=2 が 0、m38=1 が 1。50 種の clip(`tables/0307-elog12-init-groups.txt`)。
- **ResampleSource の inst と clip の対応**(`tables/0307-elog12-resamplesource-to-init.txt`): inst 0 = m38=1・n=0x13578・stereo・int16(Wine の先頭の余分な 1 回)、**inst 1 = m38=2・n=0x8c5800・stereo・int16(ch7、Vocals の MP3)**、**inst 2 = m38=1・n=0x8c5800・stereo・int16(ch35、`Other` の MP3 と読む)**、inst 3 = m38=1・mono・n=0x1e704・int16。PM の検証で『最初の呼びから 1〜2% 違う』のは inst 1,2、float 丸め級で一致するのは inst 3。
- **67d630**(elog10): 最初の 30,000 呼びのうち、外側の b284e60 の中が 2,426(inst 1: 1,046、inst 2: 1,044、inst 0: 160、inst 3: 96、inst 4: 50、inst 5: 30)、外側の無いのが 27,574。最初の読みは `pos=0 req=0x800`、続いて `req=0x200` 刻み。wasm 側では 67d630 が新後端に lower されていて記録できないので、比較は Init の clip(上)。
- **Wine の再現性**: clip の窓(clip-2、clip-4、clip-5)は elog11 と elog12 でバイト一致。

## 2l. MP3 の復号器(PM の宿題 A と C、01:15〜01:25。Wine・build は動かしていない)

- **FL の MP3 経路(静的、`tools/acm-xref.py`)**: FLEngine_x64.dll が msacm32 から import するのは 14 関数(acmStreamOpen / Convert / Size / PrepareHeader / UnprepareHeader / Message / Close、acmFormatSuggest、acmFormatChooseW、acmDriverEnum / Open / Close / DetailsA、acmMetrics)。import 全 DLL に mfplat・quartz・winegstreamer・mpg123・ffmpeg 系は無い。同梱 DLL では SG_x64.dll(Export では読まれない)が同じ 14 関数、ILTools_x64.dll は acmMetrics だけ。呼び元: `f_6729d0`(acmStreamOpen → acmStreamSize → acmStreamPrepareHeader → acmStreamConvert ×2 → acmStreamClose)と `f_673f40`(f_6729d0 の後、`acmMetrics(0x32)`、出力形式の wFormatTag = WAVE_FORMAT_PCM で `acmFormatSuggest(…, ACM_FORMATSUGGESTF_WFORMATTAG)`)。= 出力の形式は『ドライバが決める 16 ビット PCM』で、c75=0 の int16 clip の源。n=0x8c5800 は 7,984 × 1,152。
- **Wine の ACM(静的 + 既存のログ)**: Wine 10.0(Ubuntu 10.0~repack-12ubuntu1)の `x86_64-windows/l3codeca.acm`(1,104,248 B)の文字列に "Wine MPEG3 decoder"、"1.32.9"、"libs/mpg123/src/libmpg123/…"(システムの libmpg123.so ではなく DLL 内の静的な同梱)、復号器名 "dithered generic" / "SSE_vintage" / "3DNowExt_vintage" / "3DNow_vintage"(実行時の選択は未測定)。`logs/fl4.log`(17:14 の +loaddll、プロジェクトの読み込み中)で、msacm32.dll・msacm32.drv と内蔵 ACM ドライバ 5 種(imaadp32・msadp32・msg711・l3codeca・msgsm32)が読み込みと解放を繰り返し、l3codeca.acm の読み込みは 10 回(各回すぐ解放)。この run 自体で l3codeca が復号した『呼びごとの』直接の確認は無い(読み)。
- **窓の比較**(`tables/0307-mp3-decoders-vs-wine-window.txt`、`tools/mp3dec/`): Wine の clip-2 の窓(elog11・elog12 でバイト一致)は `[fadr.com] Vocals - Dot To Dot….mp3` の復号結果(もう 1 本の `Other – …` とは全く合わない)。Wine と各復号器の違うサンプル数(/262,144)と最大差: minimp3(NO_SIMD、fle_acm.c と同じ設定の私の build)105,486 / 3 LSB、libmpg123 1.33.3 の AVX・x86-64・generic 101,985 / 2、ffmpeg n9.0 mp3float 101,988 / 2、ffmpeg mp3(固定小数点)116,113 / 3、libmpg123 generic_dither 133,905 / 3。**Wine 以外の 3 系統は互いに 1 LSB 以内**(minimp3 対 mpg123 8,294、mp3float 対 mpg123 51、minimp3 対 mp3float 8,345)。**alpha1 の wasm の clipdump 1 は私の native minimp3 の出力と 262,144 サンプルすべてバイト一致**。差(Wine − mp3float)は平均 0.0003、率はどの信号の大きさでも約 0.39、スペクトルは 16.5 kHz まで平坦で 19.3 kHz 以上が約 11 倍、自己相関 lag1 −0.58 = 雑音状(丸めの向きの偏りではない)。
- **入力同一化の A/B の設計**(未実行): 実行が要るのは Wine の 1 本だけ(wasm の Export E12 は既にあり、その復号は minimp3 の素直な出力)。用意: `~/work/opt1-wine/variants/mp3wav/` に minimp3 で復号した 4 本の WAV(Vocals・Other = 各 9,197,568 フレーム、Sleigh_Bells 4,134,528、windchaim 355,968)、FLP の変種 `mp3wav-all.flp`(sha 先頭 ca4ad59fe2b3272a)・`mp3wav-vocals.flp`(9b2f830570ffbf30)・`mp3wav-pad2.flp`(f140b9d9f61c85b2)(FLP の標本パスは UTF-16 の 4 か所で、`.mp3` → `.wav` は同じ文字数なので長さの prefix の更新は不要、変わるバイトは 12 個)、`tools/wine-mp3wav.sh <変種> <TAG>`。比較: Wine の新しい WAV 対 既存の wasm E12 の WAV。**RMS 差が 2,031 → 約 2 に消えれば残差の全体 = MP3 復号の違い**。alpha1 側は seed の差し替えだけで build 不要(alpha1 の info)。

## 2m. 入力同一化の A/B の結果(Wine、退避構成、フック付き Export 1 本。run `elog13`、01:44:48〜01:46:22。`tables/0307-AB-mp3wav-all-wine-vs-wasm-e12.txt`)

- **やったこと**: FLP の変種 `mp3wav-all`(4 か所の標本パス `.mp3` → `.wav`、同じ文字数、sha 先頭 ca4ad59fe2b3272a)と、minimp3(NO_SIMD、フレーム単位、gapless 無し)で復号した 4 本の WAV(Vocals・Other = 各 9,197,568 フレーム、Sleigh_Bells 4,134,528、windchaim 355,968)を `proj/` に置いて、退避構成の Wine の FL で Export 1 本(フック付き)。終わったら FLP を元(sha d6783e08886b4b48)に戻し、WAV を消し、wintrust.dll を素の stub(sha 先頭 9e5084c18ccb8f54)に戻した。FL64 と wineserver は 0 本。
- **結果(実測)**: 出力 WAV は 22,863,828 B・5,715,911 フレーム(wasm E12 と同じ)。**wasm E12 の WAV との差の RMS = 0.96(元の Wine〔MP3 を Wine の ACM で復号〕は 2,031.20)、最大 |差| = 73(元 36,664)、全チャンネルが同一のフレーム 80.67%(元 21.07%)**。標本ごとの差: 0 が 88.54%、±1 が 9.94%、|差| ≥ 2 が 1.52%。|差| > 10 のフレームは 5.7M 中 8,594(最初は 173,277 フレーム目)。10 秒窓ごとの RMS は全区間で小さい(表)。PM も a22-wavcmp2.py と numpy で確認(diff rms 1.0、一致フレーム 80.7%、最大差 73)。
- **同じ run の Init 行と clip の窓(wasm の elog5 に対して)**: Init は Wine 3,091 対 wasm 3,089(Wine の余分な 2 回: n=0x13578 と n=0x162f3)。n=0x8c5800 の 2 本の clip の疎ハッシュは wasm と完全一致(m38=2〔ch7、Vocals〕963/963 回、m38=1〔ch35、Other〕184/184 回)。不一致は n=0xc9126 の float32(Auto の pad の出力、181 回、窓は 130,791/131,072 語が違い最大 6.3e-6 = 既知の f_a97930 の出力差)だけ。clip の窓(4 MiB 位置の 512 KiB): **Vocals と Other の両方が 262,144 サンプル中 0 個違い**(MP3 経路の元の run では 105,486 個違った)。n=0x36b1c の int16 2 本、n=0x62e08 の float32 も 0 差。WAV 経路は ACM 経路と同じ形で入る(n・es・c74・c75・c7c の列は一致)。フックに『bytes ≥ 4.5 MiB の clip を最大 4 個、clip-6〜9 として追加で窓を保存』を足したので、ch35 の窓(clip-6)が取れた。
- **読み**: 入力 PCM を同一にしただけで、Wine と wasm の Export の差の RMS が約 2,100 分の 1(−66.5 dB)に落ちた。**残差の全体は MP3 の復号の差(Wine の l3codeca〔mpg123 1.32.9〕対 wasm の minimp3)を Elastique の time-stretch が増幅したもの**で、lift した Elastique の算術の欠陥ではない(区分: lift-fidelity の (b) 外部契約〔msacm32/MP3 復号器〕の実装差)。残る RMS 0.96 は、既知の pad(f_a97930、float の相対 1.3e-5)とその周辺と読む(未確認、−80 dB 未満)。PM の判断: Elastique の lift の追跡は一旦終了、wasm の MP3 復号器は minimp3 のまま(実 Windows の l3codeca の出力は手元に無い。Wine を正解とは言えない)。ch7 と ch35 の分け前(`mp3wav-vocals`)は不要。
- **未確認**: 残る 0.96 の源(pad の差か他か)。WAV 経路と ACM 経路で wasm の出力が同一か(alpha1 の wasm mp3wav-all の Export、窓の後)。複数回の再現(1 回だけの run)。

## 2n. スロット2(pad の f_a97930 の差を閉じる実験。設計 01:50〜01:53、**実行済み: elog14、03:44〜03:46**。`tools/elog-run-e2.sh`、`tools/wine-stub/wintrust_hook.c` の `-DWITH_CPUID_SHIM`)

- **仮説(PM)**: Wine の Elastique は実 CPUID(AVX2/FMA)で VEX/FMA の経路を通り、wasm は cpuid を SSE2 にしているので非 VEX の経路を通る = 実装の選択の差。実験: mp3wav-all の変種 + E2b の cpuid → SSE2 の書き換え(prefix-e2)+ ELOG の hook を同じ run に重ね、(a) 出力 WAV を wasm E12 と比べる(RMS 0.96 → 0 または 1 LSB 以下か)、(b) `a97930x` の h2 が wasm の 9f693dc37d74a97d と一致するか、pad の f32 ダンプの差。
- **重ねる方法**: hook と cpuid shim は両方とも `wintrust.dll` の名前なので、**1 本の DLL に統合**(`-DWITH_CPUID_SHIM`、`stub/wintrust_hook_cpuid.dll`)。VEH は 2 本(cpuid_veh は ILLEGAL_INSTRUCTION、hook の veh は BREAKPOINT だけ)。`tools/elog-run-e2.sh` は `OPT1_ENVF=env-e2.sh` を export し、WINEPREFIX が prefix-e2 でなければ止まる(E2a の取り違えの再発防止)。run の後に shim のカウンタ(UD2 の応答数)が 0 でないことを確認する。`ELOG_RUN=tools/elog-run-e2.sh bash tools/wine-mp3wav.sh mp3wav-all elog14` で実行。
- **衝突の点検(確認済み)**: prefix-e2 と元の prefix の 7 DLL の違いは 388 バイトちょうど(FLEngine 5・dsp_ippv2 267・elastique 50・Blend2D 12・ILTools 5・oggio 44・QuickFontCache 5)、FL64.exe とプラグイン DLL は同一。hook の 28 関数の先頭 [−4, +16) バイトに入る cpuid の箇所は 55 箇所(FLEngine 5・elastique 50)のうち 0 件。prefix-e2 の wintrust.dll は E2b の cpuid shim(sha 先頭 fe4d813e13ec2fc7、stub/wintrust_cpuid.dll と同じ)。
- **予想される限界(読み)**: cpuid を SSE2 に揃えても、rcpps / rsqrtps(Elastique に 118 / 731 個)は CPU 実装の近似で、この機械(AMD Zen)の結果と wasm の runtime の実装が違えば pad の差は残る。残れば原因は『CPU 命令の近似の仕様差』(別の外部契約)。
- **実行の経緯(02:10〜02:17)**: 統合 DLL の cpuid 側の単体試験(`stub/e2test.exe`、Wine 1 CPU)は通った(`IsProcessorFeaturePresent` は 2・3・6・9・10・23 だけ 1、cpuid の応答は E2b と同じ)。`hooktest.exe`(古い stand-in の FLEngine)は**統合 DLL の `install_one`+0x47(関数の先頭 1 バイトを読む命令)でページフォルトを起こし、`winedbg --auto` で約 4 分止まった**(02:15:45 に `wineserver -k` で止めた)。原因は stand-in が第 3 版以降の hook 表(RVA 0x27xxxx・0x69xxxx)に対して小さい(image の大きさは未確認、命令の位置は確認済み)ためで、本物の FLEngine では 12 本とも `installed`(下)。以後 hooktest は使わず、統合 DLL の確認は本番のログで行う。
- **本番の run `elog14`(02:16:07 開始)は、同じ時刻に出た Chrome の窓の info を読んで、02:16:21 に止めた(Wine が動いたのは約 14 秒、Export は未完了、A/B の結果なし)**。14 秒で分かったこと(実測、実エンジン): prefix-e2 の FL64 が統合 DLL を読み、FLEngine の hook 12 本が全部 installed、cpuid の shim は UD2 を 65 回応答(leaf0 22・leaf1 20・other-basic 18・extended 5)で passed-on 0。後片付け: FLP の sha d6783e08886b4b48、proj/ の WAV 残り 0、FL64/wineserver 0 本、元の prefix の wintrust.dll = 素の stub(9e5084c18ccb8f54)、prefix-e2 の wintrust.dll = E2b の cpuid shim(fe4d813e13ec2fc7)。**やり直しは窓の後に PM の info で**(同じ手順の `elog14`、約 100 秒)。
- **wasm 側の写しが無い**: alpha1 の `runs/exp-elog*` が 02:18 に消え、wasm の `a97930-in/out.f32` と b284c10 の h の写しが手元に無い(a97930x の h2 = 9f693dc37d74a97d だけ PM の `/tmp/opencode/wasm-elog2.txt` に残る)。f32 のバイト比較をするには、alpha1 の次の wasm の Export で複製してもらう(依頼済み)。
- **結果(elog14、実測、実エンジン。`tables/0307-AB2-mp3wav-all-plus-cpuid-sse2-wine-vs-wasm-e12.txt`)**: 03:44:31〜03:46:20 の 1 本。出力 WAV 22,863,828 B(sha 先頭 6e67e5f56d181d1c)。wasm E12(sha 381cdb5a…f979)との差: **4 サンプルだけ違い(11,431,822 中、すべて +1 LSB。フレーム 495,392〔ch0〕・5,333,434〔ch1〕・5,333,941〔ch0〕・5,422,135〔ch0〕)、RMS 0.0006、最大 1、全チャンネル一致のフレーム 99.99993%**。3 段の経過(wasm E12 に対する差の RMS): 元の Wine(MP3 を Wine の ACM で復号、実 CPUID)2,031.20 → 入力 PCM を同一に(mp3wav-all、elog13)0.96 → CPUID も SSE2 に(elog14)0.0006。pad: `a97930x` の入力 h1 66151d6726caba6b・出力 h2 9f693dc37d74a97d が wasm と一致(以前の Wine は e3398e3c97f5b612)、`a97930-in.f32` と `a97930-out.f32`(各 6,588,720 B)は wasm の再生成とバイト一致(`cmp` 0、sha 先頭 7791275b99172ce5)。Init: Wine 3,091 対 wasm 3,089 で、(n, ch, h) の一致が 3,089 回(Wine の余分な 2 回〔n=0x13578 と n=0x162f3〕を除いて全部)、n=0xc9126 の float32 の窓も 0/131,072 語差。cpuid shim の応答: leaf0 31・leaf1 30・other-basic 18・extended 6、passed-on 0。統合 DLL の hook は 28 本 installed。PM も同じ数字を独立に確認(一致フレーム 5,715,907、diff rms −150.8 dB)。
- **読み**: pad の差(相対 1.3e-5、99.5% の語)の原因は、実 CPUID(AVX2/FMA)による Elastique の VEX/FMA の経路で、cpuid を wasm と同じ SSE2 にしたらビット一致した。rcpps/rsqrtps の近似の仕様差は原因ではない(mt3 の 0312k: この機械〔Zen 5〕の実命令と wasm の runtime の C 実装が 2^32 全入力で不一致 0 件)。残る 4 サンプルの +1 LSB は、同じ構成の Wine の run 間の揺れ(以前の観察: 同じ構成でスレッド数や +loaddll を変えても最大 約 6 フレームが 1 LSB 違う)と同じ大きさ(同じ構成をもう 1 本流して切り分けるのは未実施、優先度は低い)。
- **この run の後片付け**: FL64/wineserver/winedbg 0 本、FLP の sha d6783e08886b4b48、proj/ の WAV 残り 0、元の prefix の wintrust.dll = 素の stub(9e5084c18ccb8f54)、prefix-e2 の wintrust.dll = E2b の cpuid shim(fe4d813e13ec2fc7)。最初の起動(03:44:11)は `wine-mp3wav.sh` の安全確認が PM の確認用シェルのコマンド行の文字に反応して何も変えずに止まり、確認を `comm` で見る形(awk)に直して再起動した。

- **f_a97930 の呼びの順の静的な読み(窓の間の読み。未検証、`logs/hook/a97930-disasm-FLEngine-0xa97930.txt`)**: 比 = n2/n1(`cvtsi2sd` + `divsd`)を double で作り、`f_a97500` を通して float 化(`cvtsd2ss`)して、`[this+0x10]` が非 0 ならそちら(slot +0x88)、0 なら `[this+8]` のオブジェクト(slot +0x40、`r9=-1`、`xmm1` = float の比、`r8` = float へのポインタ)に 1 回設定する(= 私の `b284c60`〔+0x40、1 回〕)。ブロックごとの繰り返しでは、`[this+8]` 側で slot **+0x28 が引数なしで int を返し**(`[rbp+0xcc]` = ブロックの長さとして使われる)、その後 `f_a97730`、memcpy 風の呼び、slot **+0x18**(`rdx = r9 = this+0x1d`、`r8d` = n)、flush 側で slot **+0x38**(`rdx = this+0x1d`、戻り値 = 出力フレーム数)。= 私の hook の『b284c10 = +0x28 は 256 フレームの feed で、r8 の buffer の h1/h2 を取る』は、引数の読みが違う可能性がある(+0x28 が引数なしなら、記録した `rdx`/`n=r9d`/`h1` は呼び出し時のレジスタの残りで、意味を持たない)。pad の出力を返す slot は **+0x18 の process(in-place、`this+0x1d` のチャンネルごとのポインタ)と +0x38** と読む。pad の差が cpuid の SSE2 で閉じなければ、Wine と wasm の両方に +0x18 と +0x38 の前後の buffer のハッシュを足す(build 要)。

## 2o. G9-a の Wine 側の参照の準備(Fruity Balance を戻した構成。**Wine なし**、03:50〜03:58。実行は Chrome の窓の後の PM の info)

- **方針(PM 承認)**: 元の prefix は触らず、**prefix-e2 の中で Fruity Balance だけを戻す**(5.2 GB の複製は不要)。入力 PCM は mp3wav-all、CPUID は SSE2(elog14 と同じ)。elog14(Fruity Balance を隠した同じ構成)との差が『Fruity Balance だけの寄与』、wasm の G9-a の Export と比べるのは elog15(比較は G9-a の bundle が V2b・V3 を通ってから。PM)。
- **Fruity Balance_x64.dll の census**(`tables/0307-FruityBalance_x64-import-and-instruction-census.txt`): 命令 1,005,009 個、**cpuid 5・xgetbv 1**、rcpps 26・rsqrtps 2、fldcw 13・fnstcw 10・ldmxcsr 5・stmxcsr 2。cpuid の位置は alpha1 の `a22-cpuid-sites.py` で 5 箇所とも verified(`tables/0307-FruityBalance-cpuid-sites-dryrun.tsv`)。これを UD2 にして shim の対象に足さないと、プラグインは実 CPUID の経路で動き、wasm の SSE2 の経路と食い違う。
- **用意したもの(未実行)**: `tools/g9-prep.sh prep|undo|dry "<フォルダ名>" Effects|Generators "<DLL 名>"`(prefix-e2 の中だけ: `.off-0307` を戻す → cpuid の位置を抽出 → `tools/e2-patch-one.py` で UD2 に書き換え〔idempotent、prefix-e2 の外は拒否〕→ `e2/extra-mods.list` と `e2/extra-mods.env`〔`CPUID_EXTRA_MODS`〕を更新。`undo` は元の prefix の DLL を書き戻して `.off-0307` に戻し、バイト一致を確認)。`env-e2.sh` が `e2/extra-mods.env` を読む(あれば)。**dry-run 済み**: `g9-prep.sh dry "Fruity Balance" Effects "Fruity Balance_x64.dll"` は『5 verified cpuid sites, 5 rewritten to UD2 (dry run, file not written)』。統合 shim は `CPUID_EXTRA_MODS`(`;` 区切りの小文字の DLL 名)を読む形に変更(再コンパイル 1 回、`stub/wintrust_hook_cpuid.dll` に差し替え済み。elog14 の版は `.elog14` に退避)。
- **「Balance が Wine で実際に読み込まれたか」を同じ run の ELOG で確認する形**: ローダ通知(`LdrRegisterDllNotification`)の callback に、FL のインストール先(`Image-Line` を含む path)の module の読み込みごとに `ELOG modload base=… name=…` を記録する行を追加した。本番の run で、`Fruity Balance_x64.dll` の行が出るか(と、Problems 窓の件数の変化は run の画面で)を確認する。
- **後片付け**: G9 の参照が済むまで prefix-e2 と `variants/mp3wav` は残す(PM)。`undo` で prefix-e2 を隠した 8 構成に戻せる。
- **未実行・未確認**: 統合 shim の新しい版の Wine での動作(次の run のログで確認)、xgetbv 1 箇所は書き換えない(cpuid が OSXSAVE = 0 を返すので実行されない前提、E2 と同じ)、Fruity Balance を戻した FL の起動・Problems 窓の件数。
- **elog15 の 1 回目(05:01:14〜05:06:52、Export なし)**: PM の go で `g9-prep.sh prep "Fruity Balance" Effects "Fruity Balance_x64.dll"`(cpuid 5 箇所を UD2 に書き換え、DLL sha 先頭 f77e34e0f70eddb9)の後、`ELOG_RUN=tools/elog-run-e2.sh bash tools/wine-mp3wav.sh mp3wav-all elog15`。run の前の記録: MemAvailable 14.3 GB、swap 3.0 GB 使用で vmstat の si/so は 0(5 本)、load 0.6。結果(実エンジン): **`ELOG modload` に `Fruity Balance_x64.dll`(t=0x11d99)が出て、Balance が Wine で実際に読み込まれた**。cpuid shim の応答は leaf0 31・leaf1 34(elog14 は 30)・other-basic 24(18)・extended 7(6)・passed-on 0 = Balance の cpuid を処理した。pad の h2 は 9f693dc37d74a97d(wasm と一致)。しかし **driver が Problems 窓を認識できず 332 s で TIMEOUT**、FL を止めた(Export なし、WAV なし)。原因(読み): driver は参照画像 2 枚(problems.png、problems8.png)との画素の差で窓を認識し、Balance を戻して『7 plugin(s)』になった窓の形が合わなかった(スクリーンショットは取っていない)。project の読み込みは elog14 の約 2.5 倍遅かった(pad の a97930 の終了が t=0x71be〔29 s〕対 t=0x11add〔73 s〕、原因未確認)。
- **driver の修正(未実行)**: `tools/wine-export-run.py` に `OPT1_PROB_FALLBACK=1`(待ち時間 `OPT1_PROB_WAIT`)を追加: 参照のどちらにも合わなければ、画面全体を `…-problems-unrecognized.png` に保存し、Return を押し(OK が既定のボタン)、押した後の画面を `…-after-return.png` に保存して続行、Save 窓が出なければ OK の位置(683・723・703)を順にクリックして Ctrl+R をやり直す。Return が効くことは未確認。次の run の画面から `problems7.png` を作る。
- **状態(05:09)**: 元の prefix は無傷、prefix-e2 は `Fruity Balance` が見える状態のまま(再実行の後に `g9-prep.sh undo`)、FLP・WAV・wintrust.dll は復元済み、FL64/wineserver 0 本。

## 2p. G9-a の Wine 側の参照(Fruity Balance のみ有効。run `elog15b`、05:47:50〜05:51:33。`tables/0307-G9a-balance-wine-elog15b-vs-wasm-g9a.txt`)

- **構成と手順**: prefix-e2(元の prefix の複製、隠した 8 構成)で `Fruity Balance` だけを戻し(`g9-prep.sh prep`、Balance_x64.dll の cpuid 5 箇所を UD2 に書き換え、DLL sha 先頭 f77e34e0f70eddb9)、入力 PCM は mp3wav-all、CPUID は SSE2(統合 shim、`CPUID_EXTRA_MODS=fruity balance_x64.dll`)。run の前: MemAvailable 24 GB、swap 2.9 GB 使用で si/so 0。`OPT1_PROB_FALLBACK=1 OPT1_PROB_WAIT=150 ELOG_RUN=tools/elog-run-e2.sh bash tools/wine-mp3wav.sh mp3wav-all elog15b`。
- **Problems 窓**: 7 件に変わった窓(『Problems loading the project / 7 plugin(s) could not be opened and the plugin(s) will be ignored.』、`logs/wine-sessions/elog15b-problems-unrecognized-7plugins.png`、Ok ボタンは (983, 675))は、8 件の窓の参照(Ok の y = 683)とも最初の参照(723)とも、窓の位置が 8 px ずれていて矩形が合わず、driver が 150 s 待ってから fallback(Return)で閉じた。Return は効いた(Save 窓が出た)。この画像を `tools/refs/problems7.png` にして、driver の認識の三つ目に足した(Ok の y = 675。次の run での動作は未確認)。
- **結果(実エンジン、実測。私の numpy で、PM も a22-wavcmp2.py で確認)**: WAV 22,863,828 B(sha 先頭 1cad2739b3a6fefd)、Export の wall 36.0 s。ELOG: `modload` に `Fruity Balance_x64.dll`、cpuid shim の応答 leaf0 35・leaf1 34・other-basic 24・extended 7・passed-on 0、`a97930x` は wasm と一致(h2 9f693dc37d74a97d)、hook 28 本 installed。**Wine の elog15b〔Balance あり〕対 wasm の G9-a の Chrome Export〔Balance あり、sha 先頭 94c93666…〕は、11,431,822 サンプル中 5 サンプルだけが違い(すべて +1 LSB。フレーム 1,891,929〔ch0〕・3,709,012〔ch0〕・5,333,434〔ch1〕・5,333,941〔ch0〕・5,422,135〔ch0〕)、RMS 0.0007、最大 |差| 1、一致フレーム 99.99991%**。Balance の効果の大きさ(無し→有り)は、Wine で RMS 3,986.6208(差のあるサンプル 1,120,677、最大 |差| 65,534、一致フレーム 88.79%)、wasm で RMS 3,986.6208(1,120,675、65,534、88.79%)と、**RMS が小数 4 桁まで同じ**。
- **読み**: wasm の Balance の実モジュール(G9-a、Insert 1 のボリューム automation を含む音の処理の経路)は、原版 FL の Balance と同じ出力を出す(入力 PCM と CPUID の経路を揃えたとき)。残る ±1 LSB の数サンプルは、同じ構成の Wine の run 間の揺れの大きさ(位置の一部は elog14 と同じ、一部は違う)と読む(同一構成の再実行は未実施)。G9 は未到達(PM: 7 種のうち 1 種、GUI 未踏、他 6 種と Wrapper が未実装)。
- **状態(05:54)**: 元の prefix は無傷(`.off-0307` の Effects フォルダ 7 個のまま、wintrust.dll は素の stub 9e5084c18ccb8f54)、prefix-e2 は `Fruity Balance` が見える状態のまま(PM の指示で `g9-prep.sh undo` は保留)、FLP の sha d6783e08886b4b48、proj/ の WAV 残り 0、FL64/wineserver 0 本。

## 2q. Fruity Filter の Wine 参照の準備(**Wine なし**、06:06〜06:10。実行は PM の info の後)

- `Fruity Filter_x64.dll`(3,780,608 B)の静的な読み: `g9-prep.sh dry "Fruity Filter" Effects "Fruity Filter_x64.dll"` は『5 verified cpuid sites, 5 rewritten to UD2 (dry run)』、xgetbv 1 箇所(書き換えない)。census(`tables/0307-FruityFilter_x64-import-and-instruction-census.txt`): 命令 1,015,378 個、rcpps 26・rsqrtps 3、FMA 0、VEX 33、fldcw 9・ldmxcsr 5、数学関数の import なし。cpuid の揃え方は Balance と同じ手順で足りる。
- 手順(PM に送った 1 ページ): (A) 累積〔Balance + Filter 有効、Problems 窓『6 plugin(s)』〕を推奨、`g9-prep.sh prep "Fruity Filter" …` → `OPT1_PROB_FALLBACK=1 OPT1_PROB_WAIT=150 ELOG_RUN=tools/elog-run-e2.sh bash tools/wine-mp3wav.sh mp3wav-all elog16`(約 5〜7 分)、`modload` に両方の DLL、shim の passed-on 0、pad の h2 一致を同じ run で確認、Filter だけの効果 = (A) の run − elog15b を wasm 側の『Filter の bundle の Export − G9-a の Export』と並べて比べる。リスク(読み): Filter は再帰的なので cutoff automation の 1 ULP の差が数 LSB に増幅される可能性。

### 2q-1. 同一構成の Wine の再実行(alpha1 の §11: elog15c/d/e)を一括で回すスクリプト案(**未実行**、06:12〜06:18)

- `tools/wine-repeat.sh TAG1 TAG2 …`: 直列、1 本ずつ。構成は prefix-e2 / proj / variants の今の状態のまま(何も変えない)。各 run の前に `ps`・`free -m | sed -n 2,3p`・`vmstat 1 3 | tail -2` を `exports/repeat-<日時>.log` に記録し、**Wine が動いていれば、MemAvailable < 8 GB なら、vmstat の si/so の和 ≠ 0 なら始めずに止まる**(終了コード 1/2/3)。run は `OPT1_PROB_FALLBACK=1 OPT1_PROB_WAIT=150 ELOG_RUN=tools/elog-run-e2.sh bash tools/wine-mp3wav.sh mp3wav-all <TAG>`(Problems 窓は `problems7.png` で認識される見込み、認識できなければ fallback の Return)。各 run の後に 1 行: 所要秒・WAV の B と sha・FLP の sha・Problems 窓を認識した数・fallback を使った数・shim の応答数。最後に `tools/repeat-compare.py TAG…`(既定の相手は ci1 の G9-a の Chrome Export): 各 run の wasm との差のあるサンプル(位置と値)、run 同士の差、**全 run で wasm と違うサンプル(系統的)と一部の run だけの違い(揺れ)の数**。構文検査と `repeat-compare.py elog15b` の試験(5 サンプルを列挙)は済み。
- 判定の目安: 全 run で同じ位置が違うなら系統的(lift か構成の差)、run ごとに位置が変わるなら Wine の run 間の揺れ。

### 2q-2. Wine の実測の所要時間(`exports/runs/*.events.txt`)と、Filter の参照 elog16 の見積もり

| run | Problems 窓を認識 | Start を押した | Export の wall | done(起動〜停止) | 備考 |
|---|---|---|---|---|---|
| hide1 | 31.6 s | 50.6 s | 27.0 s | 87.1 s | |
| hideT1 | 39.5 s | 58.6 s | 29.5 s | 97.6 s | |
| e2b | 37.0 s | 56.1 s | 33.5 s | 99.1 s | |
| elog8 | 33.9 s | 53.0 s | 31.5 s | 94.0 s | |
| elog12 | 100.2 s | 121.3 s | 35.0 s | 165.8 s | 読み込みが遅い日 |
| elog13 | 31.6 s | 50.6 s | 32.5 s | 92.6 s | |
| elog14 | 37.2 s | 56.3 s | 41.0 s | 106.9 s | |
| elog15 | 認識できず 332.5 s で TIMEOUT | - | - | 338.0 s | 7 件の窓 |
| elog15b | 認識できず 150 s で fallback(Return) | 175.7 s | 36.0 s | 221.2 s | 7 件の窓 |

- **認識できる窓**: 起動〜停止 87〜107 s(7 本の中央値は 97.6 s)、読み込みが遅い日は 166 s(elog12)。Export の wall は 27〜41 s。Problems 窓の認識から Start まで 19〜21 s で一定。スクリプトの前後の作業(WAV 92 MB のコピー、FLP の差し替えと復元、ELOG の収集)で +約 10 s。
- **認識できない窓(新しい件数の最初の run)**: `OPT1_PROB_WAIT` の 150 s を待ってから fallback するので、起動〜停止 約 221 s(elog15b の実測)。待ち時間は起動からの固定なので、読み込みが 150 s 以内なら一定。
- **Filter の参照 elog16(Balance + Filter、窓は 6 件の見込み)の見積もり**: 1 本目 = 認識できない窓なので約 221 s + 約 10 s = **約 4 分**(その後 `problems6.png` を作る)。2 本目以降は 90〜110 s + 約 10 s = **約 2 分**。最悪(読み込みが遅い日)は +70 s。elog16 だけなら約 4 分、同一構成の再実行 elog15c/d/e を同じ枠で足すと +約 6 分(1 本 約 100〜115 s + 記録の 8 s、遅い日が混じると 9 分)。**合計で約 10 分、最悪 13 分**。前提: swap の入出力 0・MemAvailable ≥ 8 GB。
- 順序は PM が決める。elog16 の前に `problems6.png` を持つ手段はない(件数の違う窓の画面は 1 回見ないと作れない)。

### 2q-3. prefix-e2 の状態の一覧(06:15 の実測、sha256 先頭 12 桁。元の prefix との差のバイト数)

| ファイル | 状態 | 元の prefix の sha | prefix-e2 の sha | 差のバイト数 |
|---|---|---|---|---|
| FLEngine_x64.dll | patched (cpuid → UD2) | b2c64da40921 | ef55ad77804a | 5 |
| Shared/dsp_ippv2_x64.dll | patched | 497dd4f77e85 | e86d7cc816c3 | 267 |
| Shared/elastique_x64.dll | patched | 8551e92209cd | fa54561fc120 | 50 |
| Shared/Blend2D.dll | patched | 66634d244120 | 04ced2dd10ec | 12 |
| Shared/ILTools_x64.dll | patched | 20409dcbc5eb | a0fb0cd1b316 | 5 |
| Shared/oggio_x64.dll | patched | b26c544f5d39 | 8879eac8772d | 44 |
| Shared/QuickFontCache_x64.dll | patched | afc5a0ed70eb | 97680e993b85 | 5 |
| Plugins/Fruity/Effects/Fruity Balance/Fruity Balance_x64.dll | **見える**、patched(cpuid 5 → UD2) | a399cbe060ec | f77e34e0f70e | 5 |
| Plugins/Fruity/Effects/Fruity Filter.off-0307/Fruity Filter_x64.dll | 隠れている、未 patch | f48014442242 | f48014442242 | 0 |
| wintrust.dll(FL のフォルダ) | 元: 素の stub / prefix-e2: E2b の cpuid shim | 9e5084c18ccb | fe4d813e13ec | - |

- 隠れているフォルダ(`.off-0307`): 元の prefix は 9 個(Frequency Shifter・Fruity Balance・Fruity Delay 2・Fruity Filter・Fruity Parametric EQ 2・Fruity Wrapper〔Effects〕・Soundgoodizer・FLEX・Fruity Wrapper〔Generators〕)、prefix-e2 は 8 個(Fruity Balance だけ戻してある)。`e2/extra-mods.env` = `CPUID_EXTRA_MODS="fruity balance_x64.dll"`、`stub/wintrust_hook_cpuid.dll` の sha 先頭 12 桁 b144ba213b2b(`CPUID_EXTRA_MODS` を読み、FL のインストール先の module の読み込みを記録する版)。proj/ の FLP は元(sha d6783e08886b4b48、1,322,619 B)で、mp3wav-all の変種は `variants/mp3wav-all.flp`(ca4ad59fe2b3272a)、WAV は `variants/mp3wav/` に 4 本(run の間だけ proj/ に置く)。
- Filter を足すと: `g9-prep.sh prep "Fruity Filter" …` で `Fruity Filter` が見え(cpuid 5 → UD2)、`e2/extra-mods.env` は `fruity balance_x64.dll;fruity filter_x64.dll` になる。`undo` で元の DLL を書き戻して隠す(バイト一致を確認する)。

## 3. 次の実験

1. **mp3pad の伸縮のどの段階で違うか**(alpha1 の次手): mp3 のデコード結果の一致(伸縮前)、Elastique の入力 / 出力の一致を、Wine と wasm で段階ごとに比べる(Elastique 単体の harness)。
2. (参考)Split mixer tracks の二分探索は、残差が mp3pad の伸縮に絞れたので優先度が下がった(1 回目の試行は未完了、§4)。
3. DirectSound での退避構成の再生 CPU/ブロック(0298 §8 の未実行)。
3b. **入力同一化の A/B と CPUID の揃え**: 実行済み(§2m、§2n)。`mp3wav-vocals` は不要(結論が出た)。alpha1 の wasm の mp3wav-all の Export は wasm E12 と bit 同一(PM の確認)。Elastique と Export の差の追跡は終了。
4. **alpha1 の絞り込みの変種 4 本**(`nopitch-ch7-35`、`notimemode-ch7-35`、`nostretch-ch7`、`nostretch-ch35`)の Wine Export(窓待ち、`tools/wine-variants.sh <変種名…>`)。
5. **Wine 側の記録**: §2g〜§2j。alpha1 の wasm 側の第 3 版(b284e60 の入口・出口のハッシュと dump)が出たら、`(inst, ic)` ごとの n の列と h を突き合わせる。
6. tt-e20(E12 + 0302 の FS 付き試験ビルドの node 行)は **取り消し**(E13 が出荷されたため)。代わりに E13(x21、d0447902)の試験ビルドの node 行(84/55/87)を E12c と比べる(低優先、nice 19・JOBS 1・BINARYEN_CORES=1、sdriveg2 と ci1 の counted が無い時だけ)。

## 4. 自己申告

00. **Split mixer tracks の Export は未完了**: 1 回目の試行(19:41)は、Problems 窓の検出が窓の描画途中(題名が空)で一致せず、3 分待って止めた(`logs/wine-sessions/split1-ATTEMPT-NOT-COMPLETED.events.txt`)。検出を本文の文字の領域に直したが、ci1 の `sdriveg2-lat`(Chrome)が走り出したので、再実行していない。

0. **E2a は取り違え**: `run-fl.sh` が `env.sh` を再読み込みして、`prefix-e2` ではなく元の prefix で FL を動かした(shim なし)。出力が hide1 と同じなのに気づいて、カウンタのファイルが無いことで確認した。直して E2b を実行した。E2a の FL は `wineserver -k` が別の prefix を対象にしたため約 1 分残った。

0b. 60 s 再生の run(hideP1)は、ci1 の非 counted の `sdriveg2-export`(Chrome)が走っている間に始めた(PM の 19:27 の指示: ci1 の scenes・Export は Wine と並行してよい)。ci1 の pure(counted)の前に終えた。
1. 第 2 報(0298)の見出しの時刻 `date 18:18` は誤りで、実際の送信は 18:11。
2. 低域の遅れの最初の計算(総当たりの相互相関)は重すぎて 170 s でタイムアウトした。FFT 相互相関に変えて再実行(`tools/wavlf.py`)。タイムアウトのやり直しで、結果は変わらない。
3. 私の Wine セッション A(18:25〜18:27)は、gpu1 の rt-e11cb-300(18:21 開始)と重なった: 起動前に `ps` を見たが、同じコマンドで続けて起動し、結果を読まなかった。gpu1 の run は『参考』扱いになった。
4. 時刻の記載が 3 回ずれた(0307 第 0 報の 18:26 は実際は 18:21、Wine 対 Wine の 18:37 は 18:32、0298 第 2 報の 18:18 は 18:11)。以後は `date` を打ってから書く。
5. 退避実験の最初の試行(hideA)は、Problems 窓の検出の座標が退避後の窓の位置(40 px 下)と合わず、5 分近く空回りした。座標の検出を直して hide1 を実行した(hideA の結果は使っていない)。
6. 帯域別の相関の符号(低域が負)は、『極性が反転』ではなく『数 ms のずれの影響』と§2.4 で読み直した(最初は極性の反転を疑った)。

7. 第 4 版 ELOG の最初の run(elog11)は、alpha1 の『clip のデータは float32』という前提のまま `bytes = n*ch*4` で疎ハッシュを出し、int16 の clip で 0(読めない)やごみを含む値になった。窓のダンプの中身を見て(NaN だらけ)気づき、`[clip+0x75]` が形式の旗であることを確かめて elog12 で直した。elog11 の h は無効。
8. 宿題 A の最初の ctypes の mpg123 呼び出しで、`MPG123_FLAGS` の番号と `ENC_SIGNED_16` の定数を取り違え、エラー行を数千行端末に流した(結果には影響なし。直して再実行)。
9. elog10〜elog12 の 3 本の Wine は、PM の『Wine は alpha1 の build と同時に 1 本まで、01:12 まで』の許可の中(00:55〜01:06:31)に収めた。その間 `sdriveg2`(Chrome の smoke、PM が許可)が走っていた。
10. 時刻の記載: 01:26 の PM への info の冒頭を、`date` を打たずに書いて『01:27』とした(実際は 01:26:12)。結果には影響しない。以後、時刻を書く前に `date` を打つ。

11. スロット2の設計の info の冒頭を、`date` を打たずに書き換えて『01:54』とした(実際は 01:52:57)。2 回目の同種の誤り。送信後に気づいたので、文書で訂正する。

12. スロット2の本番の起動(02:16:07)を、窓の開始の info を読む前に行い、Wine を約 14 秒動かした(窓の開始の info の直前)。統合 DLL の単体試験に 5 分かけ、本番の起動が info と前後した。自分で止めて、後片付けを sha で確認した。
13. `hooktest` が最初の 90 秒のタイムアウトで返らなかったのに、原因を見ずにバックグラウンドでやり直し、合計約 4 分 winedbg を残した(自分で止めた)。単体試験に `timeout` を付けて拒否された(結果に影響なし)。

14. 最初の elog14 の起動(03:44:11)は、`wine-mp3wav.sh` の安全確認(`ps -eo args | grep FL64|sdriveg2`)が PM の確認用シェルのコマンド行に反応して止まった(何も変えていない)。確認を `comm` で見る形に直した。

15. elog15 の前に、driver の Problems 窓の認識が『8 件の窓』の参照画像に依存していて、7 件の窓で動くかを点検していなかった(承認済みの手順で始めたが、点検が足りなかった)。約 5.5 分 Wine を動かして Export なしで TIMEOUT した(Chrome の窓の開始の info を読む前に終了)。

16. elog15b の driver の fallback は、画面を実際に見て『7 件の窓か』を確かめる前に Return を押す仕組みだった(PM が承認済み)。run の後に画像を見て 7 件の窓であることを確認した。

## 5. ファイル

- `tables/`: `0307-wasm-e12-vs-wine-time-evolution.txt`、`0307-wasm-e12-vs-wine-end-and-bands.txt`、`0307-wasm-e12-vs-wine-lf-lag.txt`。
- `tools/`: `wavdiff-time.py`、`wavbands.py`、`wavlf.py`、`wavcmp-all.sh`(0298 の `tools/wavcmp.py`)、Wine の駆動(`wine-export-run.py`、`wine-play-run.py`、`hide-fruity.sh`、`chain-0307.sh`、`refs/`)、E2 の shim(`wine-stub/wintrust_cpuid.c`、`e2test.c`、`oggio_x64_test.c`、`e2-patch.py`、`a22-cpuid-sites.py`〔alpha1〕、`env-e2.sh`)、E6(`e6.sh`、`loaddll-summary.py`)。
- 第 4 版 ELOG と MP3 の復号器(§2k・§2l): `tables/0307-elog12-init-groups.txt`、`tables/0307-elog12-resamplesource-to-init.txt`、`tables/0307-elog12-clipdump-sha256.txt`、`tables/0307-mp3-decoders-vs-wine-window.txt`、`logs/hook/elog12-67de50-first40.txt`、`logs/hook/elog12-clipdump.txt`、`logs/hook/elog10-67d630-first60.txt`、`tools/wine-stub/wintrust_hook.c`(第 4 版)、`tools/init-compare.py`、`tools/acm-xref.py`、`tools/mp3dec/{dec2.c,mpg2.py}`、`tools/wine-mp3wav.sh`(実行済み: elog13)、`tables/0307-AB-mp3wav-all-wine-vs-wasm-e12.txt`、`logs/hook/elog13-67de50-first40.txt`、`logs/hook/elog13-clipdump.txt`。大きなファイル(clip の窓、WAV、FLP の変種)は `~/work/opt1-wine/exports/elog/`・`~/work/opt1-wine/variants/` に置き、リポジトリには入れていない。
- elog14(§2n): `tables/0307-AB2-mp3wav-all-plus-cpuid-sse2-wine-vs-wasm-e12.txt`、`logs/hook/elog14-67de50-first40.txt`、`logs/hook/elog14-shim-counts.txt`、`tools/elog-run-e2.sh`、`tools/slot2-analyze.py`、`tools/wine-stub/wintrust_hook.c`(`-DWITH_CPUID_SHIM` つき)。
- G9 の参照の準備(§2o): `tools/g9-prep.sh`、`tools/e2-patch-one.py`、`env-e2.sh`、`tables/0307-FruityBalance_x64-import-and-instruction-census.txt`、`tables/0307-FruityBalance-cpuid-sites-dryrun.tsv`。
- elog15 の 1 回目(§2o): `logs/hook/elog15-modload.txt`、`logs/hook/elog15-events.txt`、`logs/hook/elog15-shim-counts.txt`、`tools/wine-export-run.py`(`OPT1_PROB_FALLBACK` つき)。
- G9-a の Wine 側の参照(§2p): `tables/0307-G9a-balance-wine-elog15b-vs-wasm-g9a.txt`、`logs/wine-sessions/elog15b-problems-unrecognized-7plugins.png`、`logs/hook/elog15b-{events,modload,shim-counts}.txt`、`tools/refs/problems7.png`、`tools/wine-export-run.py`(三つ目の参照つき)。
- Filter の準備(§2q): `tables/0307-FruityFilter_x64-import-and-instruction-census.txt`、`tables/0307-FruityFilter-cpuid-sites-dryrun.tsv`。


## PM の判断と取り込み(2026-11-06 19:50、docs のみ。製品の差分なし)

取り込み: opt1 の最終(2026-11-06 19:48 の opt1 の報告)の文書・表・道具(`tools/refs` の画像 4 枚 1.4 MB は入れない。原本は `lift/out-git/opt2-results/0307/`)を `docs/investigations/opt1/0307/` に置いた。実行は Wine 10 上の原版 FL(決定 0298 と同じ環境)。**実 Windows・依頼者の機械は 0 件**。

事実として受け取る(wasm の基準 = E12 の Export の WAV、22,863,828 B、sha256 `381cdb5a…f979`):
1. 内蔵プラグインありの Wine の Export 2 本は同じ設定でもビット同一でない(差の RMS 29、信号の約 −57 dB、全帯域の相関 1.000・遅れ 0)。余韻 +13,506 フレームは再現する。wasm と Wine(あり)の差は RMS 23,654(約 800 倍)で、低域 20〜100 Hz だけが 4.4〜7.1 ms(192〜315 フレーム)Wine のほうが遅く、wasm は 129.600 s で信号が大きい所で切れる。
2. **Wine で FL 内蔵 8 種(Plugins\Fruity の 9 フォルダ)を退避すると、Problems 窓が wasm と同じ『8 plugin(s)』になり、Export は wasm と同じ長さ(22,863,828 B、5,715,911 フレーム)・同じ最初と最後の非ゼロフレーム・遅れ 0・利得 0.999〜1.001 になる**。= **Wine 対 wasm の差の主因は内蔵プラグインの有無(確定)**。余韻と低域の遅れは内蔵エフェクトの物。私の numpy の再計算でも、退避構成 vs wasm は差の RMS 2,031.2・不一致フレーム 4,511,490・最大差 36,664(信号の RMS 20,412)。
3. **残る差(RMS 2,031、約 −20 dB)は、論理 CPU 数(1 と 16 で同一〔4 フレームの差、最大 1〕)・CPU 機能の経路(cpuid 87 回と IsProcessorFeaturePresent を wasm と同じ SSE2 の答えにした shim:差は RMS 2,031.3 のまま)・DLL の読み込み(+loaddll: ildsp_x64・beat_tracker・LAMEEnc・ILSvg は Export で読まれない)では動かない**。退避構成の Wine は決定的(5,715,911 中 5,715,907〜5,715,909 が一致)。

判断:
1. 「wasm の Export が Wine と違う」は**主として G9(同梱プラグイン)未到達の結果**で、時刻・乱数・スレッド数・CPUID の非決定性の問題ではない。**Export の sha256 `381cdb5a…f979` が E8〜E12 で同一なのは、プラグイン抜きの仕事に対する決定的な出力**であり、原版の WAV とは一致しない(一致するのは G9 後)。
2. 残りの RMS 2,031 は未解明の別の差(alpha1 の決定 0308: MP3 デコードは −85〜−95 dB で除外、候補は Elastique の stretch/pitch が最有力)。変種 FLP(stretch/pitch を外す)の両側 Export と Split mixer tracks の突き合わせで切り分ける(opt1 の Wine 側、alpha1 の wasm 側)。**残差が lift の誤り(忠実性)かどうかは未確定**。
3. P2/P3 の数値は『内蔵エフェクトが無視された負荷』のまま。G9 後は mixer の負荷が増える(決定 0298 の CPU 比)。
4. 未実行: Split mixer tracks の Export(Wine と wasm の両側)、変種 FLP の Export、wasm の Export の Worker CPU。

### 追記(2026-11-06 22:33、opt1 の最終 22:31 版の取り込み)

最終版(変種 3 本の Wine 側 Export、E2・E6 の結果を含む)に差し替え、`docs/investigations/opt1/0307/` の表・道具を更新した。判断は決定 0308 の追記(同名の変種 FLP の差の RMS: 元 2,031.2 → ch7・35・45 の伸縮/ピッチを外すと 2.1)にまとめた。Wine 側の記録(`f_a97930` の入出力の 64 bit ハッシュ)は opt1 が shim の VEH + INT3 で行う設計を評価済み(ImageBase は 0x400000。約 2 時間。窓の後、変種 4 本の Wine Export の後)。**実 Windows・依頼者の機械は 0 件**。


## PM の追記(2026-11-07 02:38、opt1 の入力統一の A/B と 4 回目までの ELOG の取り込み、docs のみ。製品の差分なし)

取り込み: opt1 の 0307 の最終版(§2k〜§2m、5 復号器の表、入力統一の A/B)と表・道具・ログ(`docs/investigations/opt1/0307/`。400 KB 超と `*.log`・png は入れない)。**実エンジン(Wine 10 上の元の x86 FL、隠した構成)の実測。代役・再生なし。依頼者の機械は 0 件。**

PM の検証: A/B の WAV(`mp3wav-all`、4 本の MP3 を minimp3 で復号した同一の WAV に差し替えた FLP)と wasm E12 の Export の差の RMS は 2,031.2 → 0.96(−86.6 dB)、一致フレーム 80.7%、最大差 73(`a22-wavcmp2.py`、PM の再計算)。Wine の復号器の同定: FL は MP3 を ACM(msacm32)でしか復号せず、Wine の ACM は内蔵の l3codeca.acm(mpg123 1.32.9 内蔵)。5 復号器の比較で Wine の復号だけが外れ値(窓の 40%、±1〜3 LSB)。

**未完了(正直に)**: Wine の CPU 機能を SSE2 に揃えた mp3wav の run(pad の差を閉じる試験)は、02:16:08〜02:16:21 の約 14 秒だけ動いて止めた(Export 未完了。gpu1 の Chrome 窓と開始が重なった可能性があり、gpu1 の最初の run は参考扱い)。opt1 の自己申告: 本番の起動が私の窓の info の前後だった、単体試験の `hooktest` が stand-in の FLEngine の image と第 3 版以降の hook の RVA が合わず `winedbg` で約 4 分止まった(02:15:45 に自分で止めた。hooktest は今後使わず本番 run のログで確認)。窓の後に同じ手順の `elog14` を 1 本流す。


### 追記(2026-11-07 03:48、elog14 の結果〔入力統一 + CPUID を SSE2 に揃えた Wine の Export〕、docs のみ。製品の差分なし)

opt1 の 03:47 の報告と PM の検証。**実エンジン(Wine 10 上の元の x86 FL、prefix-e2、統合 hook DLL \`wintrust_hook_cpuid.dll\`)の実測 1 本(elog14、03:44:31〜03:46:20)。代役・再生なし。依頼者の機械は 0 件。** PM の検証(numpy 1 スレッド nice 19、\`a22-wavcmp2.py\`、\`cmp\`): Wine の WAV(\`exports/runs/elog14.wav\`、22,863,828 B)対 wasm E12 の WAV(sha 381cdb5a…f979)は 5,715,911 フレーム同長で、**一致フレーム 5,715,907(差のあるフレームは 4 個: 495392、5333434、5333941、5422135、すべて最大 1 LSB)、差 RMS 0.001(−150.8 dB)**。パッドの出力 f32(1,647,180 語)は wasm の a97930-out.f32 と **cmp でバイト一致**(sha 7791275b99172ce5…)。\`a97930x\` の h1 66151d6726caba6b・h2 9f693dc37d74a97d が wasm と一致。Init(\`f_67de50\`)は Wine の先頭の余分な 2 回を除いた 3,089 行で n・ch・es・m38・c74・c75・c7c・me4・**h が 3,089/3,089 一致**。shim: UD2 応答 leaf0 31・leaf1 30・other-basic 18・extended 6、passed-on 0。後片付け(opt1 の確認): FLP sha d6783e08886b4b48 に復元、proj の WAV 残りなし、FL64/wineserver 0 本、元の prefix の wintrust.dll は素の stub。

**判断**: (1) 入力の MP3 復号器(ACM の実装差)と CPU 機能の経路(実 CPUID の AVX/FMA 対 wasm の SSE2 応答)を揃えれば、Wine 上の元の FL と wasm の lift 済み FL の Export はほぼビット一致する。Wine 対 wasm の Export の残差(2,031 → 0.96 → 0.001)は **すべて外部の実装差/実行環境の選択で、lift した Elastique・IPP・ミキサ・出力の算術の欠陥は見えない**。Elastique と Export の差の追跡は終了する。(2) wasm の CPUID は SSE2 の応答のまま(決定論的で、lift に AVX の経路を持ち込まない)。実 Windows の AVX2/FMA の CPU では FL が別の ISA 経路を通るので、実機の Export とは約 −96 dB(パッド相対 1e-5)離れる見込み(読み・未確認。依頼者の機械の値は 0 件)。(3) 残る 4 サンプルの +1 LSB は Wine 対 Wine の run 間の揺れと同じ大きさで、切り分けは未実施(優先度低)。(4) Wine の作業領域: 元の \`prefix\`(隠した 8 構成)は G9-a の Wine 側の参照のため壊さない。\`hide-fruity.sh restore\` と prefix の複製(5.2 GB の I/O)は Chrome の窓が全部終わってから。\`variants/mp3wav\`(92 MB)は消してよい。\`prefix-e2\` は 4 サンプルの切り分けをするまで残す。


### 追記(2026-11-07 06:04、opt1 の最終〔§2o 準備・§2p elog15b〕の取り込みと PM の確認、docs のみ。製品の差分なし)

取り込み: opt1 の 0307 の現行版(本文を差し替え。上の PM の節と追記は残した)、表(`docs/investigations/opt1/0307/tables/`: elog14 の A/B、Balance の cpuid 箇所の dry-run、Balance の census、elog15b の比較)、道具(`tools/g9-prep.sh`、`e2-patch-one.py`、`slot2-analyze.py`、driver の `OPT1_PROB_FALLBACK` 追加版)。参照画像 PNG(`tools/refs/`)・WAV は含めない。**実エンジン(Wine 10 上の元の x86 FL、prefix-e2、Balance だけ有効、MP3→WAV 入力、cpuid SSE2 の shim)の実測 1 本(`elog15b`、05:47:50〜05:51:33)。代役・再生なし。依頼者の機械は 0 件。**

**PM が自分で確認した事実**: (1) Wine の WAV `elog15b.wav`(22,863,828 B、sha256 `1cad2739b3a6fefd367c9fbe5e1357f09cb6a4c6dcb6f61627a6208fadc2d7ae`)対 wasm の G9-a(Balance を実モジュールにした test bundle)の Chrome Export(ci1、sha256 `94c93666…4090`)は 5,715,911 フレーム同長で、**差のあるフレームは 5 個、いずれも ±1 LSB(差 RMS 0.0007、−149.8 dB)**。Balance なしの wasm 対 Wine は 88.8% 一致・RMS 3,986.6。Balance の効果の大きさは Wine と wasm で RMS 3,986.6208 と小数 4 桁まで同じ。(2) driver が認識できなかった窓を opt1 が保存した `elog15b-problems-unrecognized.png` を私が読み、Wine でも『7 plugin(s) could not be opened』(wasm の G9-a と同じ 7 件)だった。(3) ELOG `modload` に `Fruity Balance_x64.dll` が出ており(Balance は Wine で実際に読まれた)、cpuid shim は UD2 を leaf0 35・leaf1 34・other-basic 24・extended 7 回応答し passed-on 0、pad `a97930x` の h2 は `9f693dc37d74a97d`(wasm と一致)。

**判断**: (a) Fruity Balance の実モジュールの Export の出力は、MP3 の復号器と CPUID の経路を揃えた Wine 上の元の FL と ±1 LSB(5 フレーム)で一致する。lift の忠実性に欠陥は見えない。残る 5 フレームは Wine の run 間の揺れの大きさで、そのうち 3 フレームは Balance を隠した elog14 にも現れる(切り分けは alpha1 の 0310 §9)。(b) **G9 は未到達のまま**: 7 種のうち 1 種のみ、Export の音のみ(GUI は未観測、再生中の挙動・first idle は未測定)、test bundle は診断入りで push しない(push するのは `0313-g9a-balance-module.diff` だけ)。(c) driver は 7 件の窓の参照 `tools/refs/problems7.png` を持つ版で次の run に使う(未試験)。(d) prefix-e2 は Balance 有効のまま残す(G9-b 以降の Wine 側参照に使う。元の隠した 8 構成の prefix は無傷)。`g9-prep.sh undo` は私の指示まで実行しない。



### 追記(2026-11-07 06:19、opt1 の宿題〔§2q-1〜§2q-3: 同一構成の再実行の一括スクリプト、Wine の所要の実測と elog16 の見積もり、prefix-e2 の状態の表〕の取り込み、docs のみ。Wine は起動していない)

opt1 の 06:18 の報告を取り込んだ。読みと準備だけ(実行なし)。要点: (1) `tools/wine-repeat.sh`(直列、各 run の前に ps/free/vmstat を記録し、Wine が動いている・MemAvailable < 8 GB・swap の入出力 ≠ 0 のどれかで始めない)と `tools/repeat-compare.py`(各 run の wasm との差のサンプルを、全 run で違う〔系統的〕か一部の run だけ〔揺れ〕かで分類)。構文検査と elog15b での repeat-compare の試験は済み。(2) 9 本の events からの所要の実測: 認識できる窓は起動〜停止 87〜107 s(中央値 97.6 s)、遅い日は 166 s、Export の wall は 27〜41 s。認識できない窓の最初の 1 本は 150 s 待つので約 221 s。elog16(Balance + Filter、窓は 6 件で最初の 1 本は認識できない見込み)は約 4 分、同一構成の再実行 3 本を足すと合計約 10 分、最悪 13 分。(3) prefix-e2 の状態: patch 済み 7 DLL(388 バイト)、Balance は見える(5 バイト patch、sha 先頭 f77e34e0f70e)、Filter は隠れていて未 patch(元と同一 f48014442242)、wintrust.dll は E2b の cpuid shim。

PM の判断: (a) 実行は Filter の bundle ができて Chrome の Export の WAV が取れた後(Wine と Chrome は重ねない)。順は elog16 → 同一構成の再実行(alpha1 の 0310 §11)。(b) `problems6.png` の参照は elog16 の最初の fallback の画面から作る。(c) いま Wine は起動しない。G9 は未到達のまま。
