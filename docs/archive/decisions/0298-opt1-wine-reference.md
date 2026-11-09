# 決定 0298(仮): 原版 x86-64 FL Studio の参照計測(Wine 10 上)

> **先頭の注意(必読)**: これは **参照計測**です。**「Wine 上での x86 実行 + Wine の DirectSound → PulseAudio(null sink)経路」**で、**実 Windows の値ではありません**(実 Windows / 実 GPU / 依頼者の機械 = 0 件)。lift の出力ではなく、**代役・画像の再生・事前に作った値でもありません**: 原版の `FL64.exe` と `FLEngine_x64.dll`(22,227,016 B、`binaries/win64/FLEngine_x64.dll` と同じ大きさ)が、この機械の Wine で実際に動いて出した CPU 時間・曲位置・書き出しファイルです。Wine の起動のために **環境側の shim**(下の §2.2)を 1 つ使っています。FL のファイルは 1 バイトも変えていません。

> **追記(19:45、§8)**: この文書の §0〜§7 の CPU の数(ミキサー側 49.4 ms/ブロック、Export 36.9 ms/ブロック)は、**FL 内蔵のエフェクト 8 種を含む構成**の値で、wasm(内蔵 8 種の DLL が無く『8 plugin(s)』で開く)とは**仕事量が違う**。内蔵 8 種を退避した構成(=仕事量が wasm に近い)の値は §8: **Export 14.0 ms/ブロック(退避前 36.9)、再生 10〜60 s のミキサー側 32.05 ms/ブロック(待機 約 12 を引いた増分 約 20)**。公平な参照は §8 の値。

状態: **仮の決定**。作業者 `opt1`(0298 担当)、2026-11-06 JST 16:45〜19:45(`date` の時計)。区分: **実測**=このセッションのコマンドの出力を見た事実、**読み**=解釈、**未確認**。**この機械(16 vCPU の VM、他担当と共有)の値**。

## 0. 先頭の 4 点

1. **実行した事実(実測)**: FL Studio 26.1.7.5653(Trial)を Wine 10.0 の専用 WINEPREFIX(`~/work/opt1-wine/prefix`。`~/.wine` は未使用)に入れ、参照プロジェクト `serina1-serum2.flp` を開き、**DirectSound(Primary Sound Driver、44100 Hz、バッファ 4096 smp = 93 ms、polling)で Space 再生 60 s が実時間で進んで止まらず、再生後の FL の Settings は "0 underruns"**。Export(Full song、16 bit stereo WAV、wasm の Export と同じダイアログ設定)も完走し 22,915,648 B の WAV を書いた。
2. **CPU(再生の 10〜60 s、1 ブロック = 4096 フレーム = 92.88 ms 当たりの CPU ms)**: 「最も忙しいスレッド = ミキサー」の規則どおりだと **89.55 ms/ブロック(1 コアの 96.4%)**だが、**これは仕事量ではない**(§3.2): このスレッドは再生しなくても 87.6〜87.8 で、再生しても +2 程度しか増えない(正体は未特定、ここでは spinner)。ミキサー本体(`TMixerThread_DS.Execute` を持つスレッド)は別のスレッドで、**再生中 11.3、待機 8.0**(§3.3)。ミキサー側の全部(FLEngine のスレッド + ワーカープール + 音 I/O)は **49.4**(待機の約 30.5 を引いた増分 **18.9**)、UI スレッド(FL64.exe のメイン)は **37.4**(待機 45.7〜47.1)。wasm E11 の 45.1(ミキサー)・21.4(FL 本体)と並べると、ミキサー側は 0.91 倍(Wine の総量と比べる場合)〜2.4 倍(増分と比べる場合)で、**「wasm が原版の約 10 倍遅い」を示す数ではない**。ただし範囲が広く、決め手ではない(ワーカープールの待機中の CPU が spin かどうかを分けられない: §3.4)。
3. **Export(offline render)**: Wine 原版は click → 最後の書き込みまで **66.05 s の壁時計で 129.9 s の音声 = 実時間の 1.97 倍**。CPU は(spinner と UI を除く)**51.6 s = 音声 1 秒当たり 0.397 CPU 秒 = 92.88 ms ブロック当たり 36.9 ms**。wasm の同じ Export(E12 の `e19-export`)は、ファイルの時刻から **Start のクリック(17:04:43)〜WAV 完成(遅くとも 17:06:05)で最大 82 s ≒ 実時間の 1.6 倍**(Worker CPU は取っていない = **未実行**。Chrome を ci1 の counted run と同時に走らせないため)。
4. **新しい観察(読みではなく実測、原因は未確認)**: **Wine の原版が書いた WAV と、wasm(E12)が書いた WAV は同じ内容ではない**(§4)。大きさ 22,915,648 B 対 22,863,828 B、最初の無音でないフレーム 172,956 対 172,955(ほぼ同じ位置)、1 秒ごとの RMS の相関 0.973 だが、フレームごとの一致は 5.7 M フレーム中 0.39 M(うち 0.17 M は先頭の無音)、左チャンネル 2 s 区間の規格化相関は 0.44〜0.72。原因は未確認(候補: Wine / Trial / 同梱サンプルの違い、プラグイン未導入時の扱い、wasm 側の状態)。**Wine を『正解』とは言えない**ので、これだけでは lift の誤りとは言えない。切り分けには別の実験(トラック別書き出しなど)が要る。

## 1. この計測が何で、何ではないか

- 目的(PM の依頼 0298): 原版 FL が**同じ曲を再生・書き出しするときの 1 ブロック当たりの CPU**を、wasm(E11/E12)の 45.1 / 21.4 ms と並べて、lift のオーバーヘッドの桁(PM の読み: 3〜5 ms なら約 10 倍、15〜20 ms なら 2〜3 倍)を見る。
- **実 Windows ではない**: Wine の DirectSound(`dsound.dll` → `mmdevapi` → `winepulse`)と PipeWire の null sink を通る。Windows の DirectSound の CPU・スレッド構成とは違い得る(§3.4)。
- **プラグイン**: 参照プロジェクトの 13 個のプラグイン(Serum ほか未導入の VST)は、wasm の run と同じく **"Problems loading the project: 13 plugin(s) could not be opened"** で無視される(ログ `logs/misc/` と下の画像)。つまり、**どちらの側もサードパーティの VST 負荷を含まない**。
- 出力設定: wasm の `DirectSound` 経路(ミキサー 1 ブロック = 4096 フレーム)に合わせ、FL の既定の "FL Studio ASIO" ではなく **Primary Sound Driver(DirectSound)**に切り替えた。ASIO のままの 60 s 再生(play1)では Settings に "12 underruns" が出た → 判定に使わない(参考。`tables/0298-play1-asio-per-thread-REFERENCE-ONLY.txt`)。

## 2. 環境(再現手順)

### 2.1 インストール

- `wine-10.0`(Ubuntu のパッケージ)、`WINEARCH=win64`、`WINEPREFIX=~/work/opt1-wine/prefix`、`DISPLAY=:88`(Xvfb 1920x1080x24)。インストーラ `~/ダウンロード/flstudio_win64_26.1.7.5653.exe`(NSIS)を Wine で実行(終了コード 199、無害)→ `C:\Program Files\Image-Line\FL Studio 2026\FL64.exe`。インストーラ・プロジェクトの zip は読み取りのみ(リポジトリにコピーしない)。プロジェクトは `~/work/opt1-wine/proj/` に展開(49 ファイル)。

### 2.2 WinVerifyTrust の shim(環境側のみ)

- 素のままだと FL64.exe が "The validity of the program could not be verified. Please reinstall it and try again." で止まる(`logs/misc/shot1-validity-dialog.png`)。Wine の `WinVerifyTrust` が `FLEngine_x64.dll` の署名(RSA-3072)で `0x80096004`(TRUST_E_CERT_SIGNATURE)を返す(`CryptMsgControl` / `CryptVerifySignatureW` が失敗)。
- **回避**: FL のインストール先に自作の `wintrust.dll`(`tools/wine-stub/wintrust_stub.c` + `.def`、mingw でビルド)を置く。`WinVerifyTrust` / `WinVerifyTrustEx` は 0 を返し、他の export は Wine 組込み版のコピー `wintrust_real.dll` に転送。起動は `WINEDLLOVERRIDES="wintrust=n;wintrust_real=n,b"`。FL のファイルは変更していない。

### 2.3 音と MIDI

- 専用の PipeWire + pipewire-pulse + **WirePlumber**(`XDG_RUNTIME_DIR=~/work/opt1-wine/run`)、null sink `opt1null`(`factory.name=support.null-audio-sink`、`media.class=Audio/Sink`、`audio.rate=44100`、`node.driver=true`)。
- **WirePlumber が無いと pulse の stream 作成が毎回 30 秒でタイムアウト**する(`logs/misc/wasapi-probe-without-wireplumber.log`、pipewire-pulse の `create_stream_timeout`)→ FL は `Error: Couldn't initialize rendering in shared mode! Code:-2004287473`(= 0x8889000F)を出し、プロジェクトの読み込みにも 5 分近くかかった。最初の 2 回の起動はこれで失敗。結果には使っていない。
- MIDI: `ALSA_CONFIG_PATH` を空ファイルにして Wine の MIDI を無効化(PipeWire の ALSA seq ポートが見えると、FL が "The MIDI input device is already allocated" を出し続けて Settings を開き続ける)。音は `winepulse` なので影響しない。
- 起動: `tools/run-fl.sh`。操作は XTEST(`tools/xt.py`、ctypes + libXtst)。スレッド別 CPU は `/proc/PID/task/TID/stat` の utime+stime を 1 秒ごと(`tools/thrsample.py`)。

### 2.4 スレッドの正体の調べ方(`tools/wine-stub/thrinfo.c`、`stackscan.c`)

- `thrinfo.exe`: 同じ wineserver の中で動く Win32 の小さなプログラム。`CreateToolhelp32Snapshot` で FL64.exe の全スレッドを列挙し、Win32 の開始アドレス(`NtQueryInformationThread` の `ThreadQuerySetWin32StartAddress`)・ユーザー/カーネル時間・開始アドレスの属するモジュールを出す。Linux の tid とは **utime/stime(ms)の一致**で対応づけた(`tools/mapthreads.py`、全スレッドで差 0〜50 ms)。
- `stackscan.exe`: 各スレッドを一瞬 Suspend → `GetThreadContext` → RSP から 64 KiB を読み、指定範囲(ロード後のアドレス)の値があるか走査 → Resume。

## 3. 再生 60 s(DirectSound、play2)

実行: 本機 date 17:48:37〜17:49:55。Space は標本 10.09 s(17:48:47)、止めたのは 70.08 s(17:49:47)。開始 load **11.40**、終了 9.62(基準の 8 超なので**参考**)。スレッド CPU 時間は負荷の影響を受けにくい(CPU 時間であって経過時間ではない)が、同じ機械を共有する他の作業と同時だった。

### 3.1 実時間で進んだ証拠(実測)

曲位置(BAR:STEP:TICK、200 bpm、`logs/play2/crops.png`): 再生 +5 s で **5:03:21**、**+30 s で 26:01:00**(= ちょうど 25 小節 = 100 拍 ÷ 3.333 拍/s = 30 s)、+57 s で 48:09:01(47.5 小節 ≒ 57 s)。止めた後は 1:01:00 に戻る。再生後の Settings: **0 underruns**(Primary Sound Driver、4096 smp、polling、hardware buffer)。FL の CPU メーターは待機 47→42 / 再生中 51〜60(spinner のせいで待機でも 40 台)。

### 3.2 「最も忙しいスレッド」は仕事量ではない

`tables/0298-play2-class-summary.txt`(再生 10〜60 s = 標本 20〜70 s、待機は標本 0〜10 s と 72〜78 s):

| 区分(単位 ms/ブロック。1 コア = 92.88) | 待機(前 / 後) | 再生 10〜60 s | 1 コアに対する割合(再生) |
|---|---|---|---|
| **spinner**(Win TID 476、開始 `QuickFontCache_x64.dll`+0xfdb0、1 本) | 87.6 / 87.8 | **89.55** | 96.4% |
| FL 本体(`FL64.exe`、メインスレッド = UI) | 45.7 / 47.1 | **37.43** | 40.3% |
| ワーカープール(同じ開始アドレスの 32 本のうち 15 本が動く) | 19.6 / 18.6 | 32.98 | 35.5% |
| `FLEngine_x64.dll` 開始のスレッド(2 本。うち最大の 1 本 = ミキサー、11.31) | 8.0 / 7.4 | 12.75 | 13.7% |
| 音 I/O(`mmdevapi` + `dsound` + `ucrtbase`) | 3.6 / 3.8 | 3.69 | 4.0% |
| **ミキサー側の合計(spinner と UI を除く全部)** | **31.2 / 29.9** | **49.43** | 53.2% |

- spinner(94〜97% の 1 スレッド)は、**再生しなくても同じ**で、再生で +2 しか増えない。ユーザー時間が 98〜99%。以前の run では起動中の splash 画面の時から 100% 近いスレッドが 1〜2 本あった(同じスレッドかは未確認)。何をしているかは**未特定**。FL の CPU メーターが待機で 40〜70 を示すのはこれが原因と読める(読み)。
- 「最も忙しいスレッド」をミキサーとして扱う規則(PM の指定)は、この環境では **89.55** になる。この値は使えない。

### 3.3 ミキサー本体の特定(PM の指示 (b))

- `TMixerThread_DS.Execute`(`f_12aeec0`、948 B、VA 0x12aeec0〜0x12af274。ImageBase 0x400000、ロード先 0x7a730000 なので 0x7b5deec0〜0x7b5df274)を呼ぶスレッドを `stackscan.exe`(60 回 × 30 ms)で探した(`logs/misc/stackscan-output.txt`): **Win TID 1284(開始 `FLEngine_x64.dll`+0x12ae0 = TThread の共通開始関数)だけ**が 60/60 回で、スタックに Execute の戻り番地を持つ。RIP は ntdll 53 / `Engine_x64.dll` 4 / `dsp_ippv2_x64.dll` 1 / `FLEngine_x64.dll` 1 / `Fruity Filter_x64.dll` 1(= ほぼ待ち、時々処理)。**spinner(開始は QuickFontCache の関数、Win TID 476)とは別のスレッド**。
- 注意: この走査は Export の後(DirectSound を開き直した後)に行ったので、見つかったのは再生 run の後に作られたミキサー(TID 1284)。再生 run 中のミキサー(TID 1236、`FLEngine_x64.dll` 開始、Linux 2940986)も同じ役のスレッドと**読んだ**(同じ開始アドレスで、DirectSound に切り替えた時に作られた最大の `FLEngine` スレッド)が、再生 run 中に走査はしていない(**未確認**)。TID 1284 の累積 CPU(ユーザー 14.8 s + カーネル 13.1 s、作成から約 5 分)は待機 8% に合う。
- ミキサーのスレッド自身は軽い(再生 11.3、待機 8.0 ms/ブロック)。待機中の 8.0 はほとんどが `Sleep(1)` + `GetCurrentPosition` の polling(1 ブロックに最大約 93 回: `docs/investigations/audio-m5.md`)。**実際のミキシングはワーカープールに分散**している(再生で +13〜14)。

### 3.4 wasm との並べ方と、言えないこと

- wasm E11(PM の値): ミキサー 45.1、FL 本体 21.4 ms/ブロック。Wine 原版: ミキサー側 49.4(総量)/ 18.9(増分)、UI 37.4。
- **言えること**: ミキサー側の比(wasm ÷ Wine)は、Wine の総量と比べれば 0.91 倍、増分と比べれば 2.4 倍。**どちらでも約 10 倍ではない**。
- **言えないこと**: 「本当の仕事量」は 18.9〜49.4 の間としか言えない。待機中でもワーカープールが 19 ms/ブロック食っている(ユーザー時間とカーネル時間がほぼ同じ = futex の待ち回りの形)ので、どこまでが spin か分けられない。UI は Wine の GDI→X11 の描画(Xvfb)を含むので、wasm の 21.4 と単純に比べられない。Wine の DirectSound は実 Windows のそれとは違う。

## 4. Export(offline render、DirectSound を開いたまま)

- 実行: 本機 date 17:56:43〜17:57:56。Ctrl+R → 保存ダイアログ(ファイル名 `serina1-serum2.wav`、`proj/` に保存)→ Rendering ダイアログ: Mode = Full song、Tail = Leave remainder、Length 108 bars、Total time 2'10"、16Bit int Stereo、Resampling 32-point sinc、HQ for all plugins、Disable maximum polyphony、Trim PDC silence(`logs/misc/export-dialog-wine.png`。wasm の `e19-export/export-dialog.png` と**設定は同じ**)。**Start** をクリックしたのは標本 3.18 s(17:56:46)、WAV の最後の書き込みは 69.23 s(17:57:52)。開始 load 6.7〜7.5。
- **壁時計 66.05 s で 22,915,648 B(= 129.9 s の音声)= 実時間の 1.97 倍**。
- CPU(標本 4〜69 s、`tables/0298-export1-class-summary.txt`): spinner 62.25 s(94.3%)、FL 本体(UI)16.68 s、ワーカープール 38.76 s(15 本が各約 2.58 s = 各 4%)、書き出しのスレッド(Export 中だけ存在し、標本の後に終了したので開始モジュールは**未特定**)12.75 s(19%)。**spinner と UI を除く合計 51.60 s = 音声 1 秒当たり 0.397 CPU 秒 = 92.88 ms ブロック当たり 36.9 ms**(再生中の 49.4 より小さい。ワーカーの待機の spin が減るため、と**読んだ**が未確認)。どのスレッドも飽和していない(最大でも書き出しのスレッドの 19%)ので、この Export は **CPU の速さではなくスレッド間の待ち合わせで決まっている**(読み)。
- **wasm 側**: `runs/e19-export` のファイルの時刻から、Start のクリックは 17:04:43、WAV の完成(`exportSha` が安定を確認)は遅くとも 17:06:05 → **最大 82 s(実時間の 1.6 倍以上)**。Worker CPU は取っていない(**未実行**)。

### 4.1 WAV の内容の比較(新しい観察)

`tables/0298-wav-wasm-e12-vs-wine-original.txt`(wasm の `runs/e19-export/export.wav`、sha256 381cdb5a…f979 対 Wine の `serina1-serum2.wav`、sha256 2a4d136a…b803):

- 大きさ 22,863,828 B(5,715,911 フレーム、129.612 s)対 22,915,648 B(5,728,866 フレーム、129.906 s)。**Wine の方が 12,955 フレーム(0.294 s)長い**。最初の無音でないフレームは 172,955 対 172,956。
- 1 秒ごとの RMS の相関 **0.973**(平均 RMS は wasm 19,784 対 Wine 20,538)= 同じ曲の同じ構成。
- フレームごとの一致は 0 フレームずらしで 388,278 / 5,715,911(うち先頭の無音が約 173 k)、+1 フレームずらしで 410,890。左チャンネル 2 s 区間の規格化相関は 0.44〜0.72(最良の遅れはほぼ +1 フレーム)。**ビット一致ではなく、波形としても別物**。
- **読み(未確認)**: 構成は同じで、内容(音量とフェーズ)が違う。Wine が正解とは限らない(Wine / Trial / 同梱サンプル / プラグイン未導入時の扱いの違いがあり得る)。**これだけでは lift の誤りとは言えない**。切り分け案: トラック別の書き出し(Split mixer tracks)の比較、サンプルの有無の確認。

## 5. 未実行(§0〜§7 の時点。§8 で更新)

- **Wine の起動〜プロジェクト読み込み完了の時間**: WirePlumber の無い run しか launch 直後から見ていない(使えない)。WirePlumber あり・ALSA 空の run は、launch から 101 s 後の最初の確認の時点で既に読み込み済みだった(= **101 s 以下**、上限だけ)。PM の P1 の 10 s とは別の物(Wine の起動は 100 s 規模)。
- **静かな窓での再生の 2 回目**: ci1 の counted run(〜18:30、延びて〜18:45)と重ならないよう、Wine を 18:03 に止めた(PM の指摘: 約 2.3 コアの外乱)。
- **wasm の Export の Worker CPU**: Chrome を ci1 の counted run と同時に走らせないため未実行。

## 6. 自己申告

1. 最初の 2 回の起動は WirePlumber が無く、音デバイスが開けず、読み込みに 5 分近くかかった。結果に使っていない(原因: §2.3)。
2. 最初のスレッド標本(play1、ASIO)は、開始スレッドの対応づけを取る前に Wine を再起動したので、区分別の表は DirectSound の run(play2)だけ。
3. `timeout` 付きの Wine 起動(wasapi_test)を 1 回、`head` に繋いで出力を失った(ファイルへの出力に変えて再実行)。
4. 並行して他担当の作業があり、play2 の開始 load は 11.4(基準 8 超)。Wine の FL を、測っていない間も約 27 分(17:35〜18:03)動かしたままにして、ci1 の counted run の外乱になった(PM の指摘 18:03。すぐ停止)。
5. 最初の報告の「待機時の spinner は以前の run の splash 画面の時から」は、同じスレッドかを確認していない(§3.2 に書いた)。

## 7. ファイル

- `docs/0298-opt1-wine-reference.md`(この文書)、`docs/README-0298-line.txt`。
- `tables/`: `0298-play2-class-summary.txt`、`0298-play2-class-per10s.txt`、`0298-export1-class-summary.txt`、`0298-wav-wasm-e12-vs-wine-original.txt`、`0298-play1-asio-per-thread-REFERENCE-ONLY.txt`。
- `logs/`: `play1/`・`play2/`・`export1/`(スレッド別 CPU の 1 秒標本 TSV、イベント時刻、Linux tid ↔ Win TID の対応表、WAV の大きさの推移)、`misc/`(validity ダイアログ、Export ダイアログ、WirePlumber 無しの WASAPI 試験、stackscan の出力)。
- `tools/`: 起動(`run-fl.sh`、`env.sh`)、入力(`xt.py`)、標本(`thrsample.py`、`play60.py`、`exportwatch.py`、`startup-watch.py`)、集計(`thr-summary.py`、`class-summary.py`、`mapthreads.py`、`wavcmp.py`)、`wine-stub/`(`wintrust_stub.c` / `.def` / `names.txt`、`thrinfo.c`、`stackscan.c`、`wasapi_test.c`)。

## 8. 追記(18:45〜19:45): FL 内蔵 8 種を退避した構成 = wasm に近い仕事量の参照

(経緯は決定 0307 `../0307/docs/0307-opt1-wine-vs-wasm-wav.md`。Wine の FL の `Plugins/Fruity` の 9 フォルダ〔Fruity Parametric EQ 2、Soundgoodizer、Fruity Delay 2、Fruity Filter、Fruity Balance、Frequency Shifter、Fruity Wrapper、FLEX、Fruity Wrapper〕を `.off-0307` に rename。Problems 窓は『8 plugin(s)』で wasm と同じ。)

| 項目(退避構成、この機械、実 Windows ではない) | 値 |
|---|---|
| FL の起動 → プロジェクトを開き終えるまで(Problems 窓の出現) | **21.3〜39.5 s**(6 本: 31.6、39.5、21.4、25.8、37.0、21.3 s。load 3.3〜13。nice 0 / 19)。内蔵プラグインありのときは 45.5 s(load 9.5) |
| Export(Full song、16 bit stereo、32-point sinc、HQ) | 壁時計 **27.0 s**(load 3.3、実時間の 4.81 倍)〜 34.0 s(load 12。+loaddll)。WAV 22,863,828 B(wasm と同じ大きさ) |
| Export の CPU(spinner と UI を除く。`tables/0298-hidden8-export-cpu.txt`) | 19.61 s = 音声 1 秒当たり 0.151 CPU 秒 = **14.0 ms/ブロック**(内蔵ありは 36.9) |
| 再生 10〜60 s(出力は FL の既定の **FL Studio ASIO**、下の注意。`tables/0298-hidden8-play-ASIO-class-summary.txt`) | spinner 90.33、FL 本体(UI)50.23、ワーカープール 13.08、FLEngine のスレッド 12.77、音 I/O 6.20 ms/ブロック。**ミキサー側(spinner と UI を除く)= 32.05**(うち音 I/O 6.20)。待機(前後)= 約 11.6〜12.6、プールは待機 0 |
| 再生の実時間性 | 曲位置: +5 s で 5:04:16、+30 s で 26:01:19、+57 s で 48:07:08(200 bpm の実時間どおり)。FL の CPU メーターは待機 14 → 再生 31〜40 |

- **内蔵 8 種の寄与**: Export の CPU は 36.9 → 14.0(内蔵エフェクトが約 62%)。再生のミキサー側は 49.4(内蔵あり、DirectSound)→ 32.05(退避、ASIO)。待機中のワーカープールの CPU(内蔵ありで 19〜20 ms/ブロック)は退避構成では 0 = **待機の spin の正体は内蔵エフェクト側だった**(読み)。
- **wasm E11/E12 のミキサー 45.1 ms/ブロックとの並べ方**: Wine 退避構成のミキサー側は総量 32.05(wasm ÷ Wine = 1.41 倍)、待機を引いた増分 約 20(wasm ÷ Wine = 約 2.3 倍)。音 I/O(6.20)を除いたエンジン+プールだけなら総量 25.85(1.74 倍)。**どれでも『wasm が原版の約 10 倍遅い』を示す数ではなく、約 1.4〜2.3 倍**(PM の読み: 15〜20 ms なら 2〜3 倍)。UI スレッドは Wine 50.2(待機 約 59〜57)対 wasm 21.4 で、Wine の GDI→X11 描画を含むので比べられない。
- **注意(再生の出力経路)**: 退避構成の再生 run(hideP1)の出力は、FL を起動し直すたびに設定が戻る **FL Studio ASIO**(ilwasapi2asio → WASAPI → winepulse)で、DirectSound(wasm と同じ経路)ではない。DirectSound の選択は FL を `wineserver -k` で止めると保存されない(0298 の play2 は手で Settings を開いて DirectSound に切り替えた)。音 I/O のスレッド(6.20)は経路依存なので、エンジン+プール(25.85)を主な比較値にする。DirectSound の退避構成の再生は未実行(Settings の自動操作が要る)。
- **wasm の Export の Worker CPU は未実行**(Chrome を ci1 / gpu1 の測定と同時に走らせないため)。wasm の Export の壁時計は既存 run のファイル時刻から最大 82 s(実時間の 1.6 倍以上)。Wine 退避構成の 27.0〜34.0 s(load の条件が違う参考値)と比べると、壁時計では Wine の方が約 2.4〜3 倍速い。
- **ビット一致の観点(決定 0307)**: 退避構成の Wine の Export は wasm の WAV と同じ長さ・同じ末尾・遅れ 0 だが、ビット一致ではなく差の RMS 2,031(信号の約 −20 dB)が残る。CPU 機能の経路(cpuid / IsProcessorFeaturePresent を SSE2 だけに揃えた shim)は原因ではなかった。

### 8.1 自己申告(追加)

1. 退避構成の再生 run の出力が ASIO だったことに、結果を出した後で気づいた(map の `ilwasapi2asio_x64.dll` のスレッドで確認)。DirectSound の再生は未実行。
2. 上の「FL の起動 → プロジェクトを開き終えるまで」の 6 本は、nice と load が揃っていない(25.8 s は E2a の run〔prefix-e2 でなく元の prefix〕、37.0 s は E2b〔prefix-e2、shim 入り〕、21.3 s は nice 0 の再生 run)。範囲として書いた。


## PM の判断と取り込み(2026-11-06 19:50、docs のみ。製品の差分なし)

取り込み: opt1 の最終(§8 追記つき)の文書・表・道具を `docs/investigations/opt1/0298/` に置いた(WAV と生ログは入れない)。実行は **Wine 10 上の原版 x86-64 FL Studio 26.1.7.5653(Trial)**(専用 WINEPREFIX、環境側の `WinVerifyTrust` の shim だけ、FL のファイルは不変、PipeWire の null sink)。**実 Windows でも lift でもなく、Wine を正解とは言えない**。依頼者の機械は 0 件。

事実として受け取る:
1. **`TMixerThread_DS.Execute` を呼ぶスレッドは 1 本(Win TID 1284、60/60 回の走査)で、QuickFontCache 起動の 'spinner'(待機中でも 1 コア近く)とは別**。実際のミキサーの仕事は QuickFontCache 側の作業スレッドプール(32 本中 15 本が稼働)に分かれて動く。= 原版は mixer を複数スレッドに分けて動かしている(wasm の協調版は 1 Worker)。
2. Export(offline render、同じ設定): 内蔵プラグインあり 66.05 s(実時間の 1.97 倍)、CPU は音声 1 秒あたり 0.397 CPU 秒(= 92.88 ms ブロックあたり 36.9 ms)。**内蔵 8 種を退避した構成(= wasm と同じ仕事量に近い。決定 0307)では 14.0 ms/ブロック、壁時計 27.0 s(実時間の 4.81 倍)**。= 内蔵プラグインは Wine の Export CPU の約 62%。
3. 再生(退避構成、10〜60 s、ASIO 出力): ミキサー側 32.05 ms/ブロック(待機分 約 12 を引いた増分 約 20)。**wasm E11 の 45.1 ms/ブロックとの比は 1.4〜2.3 倍**で、「約 10 倍遅い」ではない。ただし Wine には spin が混ざる(増分と総量の幅)、再生は DirectSound でなく ASIO だった、load が 3〜13 と共有、で決め手にはならない。

判断:
1. **「wasm は原版に比べて桁違いに遅い」という懸念は、この参照計測では支持されない**(同じ仕事量で 1.4〜2.3 倍。読み)。P3 の差は、原版が mixer を複数スレッドに分けていること(実スレッド版の方向の根拠の 1 つ)と、wasm の CPU 効率(1.4〜2.3 倍)の両方にある。CPU 効率の梃子(IPP/Elastique の 79%)は忠実性を保つ範囲では限られる(決定 0296・0303・0303b)。
2. **これまでの P2/P3 の値はすべて『FL 内蔵エフェクト 7 種 25 インスタンス + Fruity Wrapper が無視された負荷』**。原版は内蔵ありで Export CPU 36.9 に対し無しで 14.0(2.6 倍)なので、G9 到達後の mixer の負荷は大幅に増える(決定 0308・0309)。
3. 未実行: 退避構成の DirectSound での再生 CPU、Split mixer tracks の Export、wasm の Export の Worker CPU(Chrome)。優先度は ci1 の counted と E13 の確認の後。
