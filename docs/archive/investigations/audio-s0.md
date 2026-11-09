# audio-s0: FLEngine が音声 API を実行時にどう呼ぶか(S0 の観察。決定 0139 の一部)

作業者: audio1(2026-11-01、UTC)。表記: **実測**(実行して取った値)/ **読み**(コードを読んだ範囲)/ **未確認**。静的な調査 `docs/investigations/audio-m5.md` の「診断で到達していない部分」(同 §0 の 6 と §8)を、実行で埋めるのが目的。決定 0139 の本文は `docs/decisions/0139-audio-s0-s2.md`。

## 0. 先に結論

1. **実測(native、gdb、900 秒)**: ゲストは起動の 52.7〜53.2 秒(gdb の経過。仮想時計ではない)に音声の初期化へ届く。API は `DirectSoundEnumerateA` が**1 回**(コールバック `0x12af280`、ctx 0)、**ほかの dsound の API は 0 回**(`DirectSoundCreate` なし)。ASIO の列挙はレジストリ `software\asio` を開いて 0 件。winmm は `midiInGetNumDevs` 1、`midiOutGetNumDevs` 4、`midiOutGetDevCapsW` 2、`midiOutOpen` 1(MIDI_MAPPER、268.9 秒)、`timeBeginPeriod` 2、`timeGetDevCaps` 2。`waveOut*`・`mixer*`・`timeSetEvent`・WASAPI・XAudio は**呼ばれない**(import にも無い)。
2. **デバイスが 0 台のとき、ゲストは音声出力を開かない**(実測: `f_1253260` が 2 回呼ばれ、2 回とも何もせず 0 を返す。DS 初期化 `f_12afb50`・ミキサースレッド作成 `f_12afaf0`・`TMixerThread_DS.Execute` `f_12aeec0` は **900 秒の間に 1 回も入口に届かない**)。一覧には "(none)" だけが入る。ミキサーは**作られない**。つまり S0 の問いへの答えは「**0 台ならミキサーは存在しない**」。S1 で 1 台を列挙すると初めてミキサー経路に入る。
3. **ゲストの設定値(実測)**: ポーリングモード(`[0x15d3294]` = 1)、ソフトウェアバッファ(`[0x15d3295]` = 1)、float32 でない = 16 ビット PCM(`[0x15d3296]` = 0)、ブロック長 `[0x16bd4a0]` = 0x1000 = 4096 フレーム。audio-m5.md が「起動時の値は読み切れていない(未確認)」とした項目の実測。
4. **wasm(Chrome 156、-Oz)**: 同じ構成の実エンジンが 30 秒前後で段階 `idle` に届き、そのまま 661 秒まで待った(実測。`logs/s0-wasm/`)。この走行の進捗記録に DirectSound の文字列は出ない。**API の呼び出し列は wasm では取っていない**(gdb を使えない)。native と wasm で初期化の経路が同じことは、生成 C が同じ(gencmp)ことからの読み。
5. 静的調査(audio-m5.md)との差: **訂正すべき点は無かった**。audio-m5.md の「0 件なら一覧は "(none)" だけで音声デバイスが開かない(読み)」は実測で確かめられた。「既定がポーリングか通知かは未確認」は**ポーリングと確定**した。

## 1. 手順

| 経路 | 内容 |
|---|---|
| native の API トレース | `tools/gdbaudio.py` / `gdbaudio.sh`(新規)。FRUITY + SEH の `two-native`(決定 0132 の構成: FLEngine と 5 モジュール)を gdb の下で 900 秒走らせ、`kernel32`・`winmm`・`ole32`・`dsound`・`avrt` 系の import のスタブに breakpoint を置いて、呼び出しごとに引数・呼び出し元の 7 段・回数を記録。ゲストの関数(`f_1196c00`、`f_1252bd0`、`f_c55ad0`、`f_c526f0`、`f_12af3e0`、`f_12af280`、`f_12517f0`、`f_1252d80`、`f_13582c0`、`f_11d3340`、`f_1253610`、`f_1253260`、`f_12afb50`、`f_12af5c0`、`f_12afaf0`、`f_12aedf0`、`f_12aeec0`、`f_12b00f0`、`f_c589b0`、`f_c53c60`、`f_1251b30`、ほか)の入口と戻りにも置き、音声の状態(下の §3)を読む。 |
| 実行の時刻 | 20:29:16Z〜20:44:19Z(2026-11-01)。ログは `logs/s0-gdb/`(`gdbaudio.txt` = 呼び出しの記録 1,005 行(末尾に API ごとの COUNT と関数ごとの GCOUNT)、`run.out` / `run.err` = ゲストの出力、`gdb.out`)。 |
| wasm | `-Oz`(JSPI + SEH、決定 0134 の構成)の実エンジンを Chrome 156 で 661 秒。`logs/s0-wasm/`(`s0-oz.json` はページ側の記録、`runner.stdout` は要約)。 |
| 基準の木 | HEAD **ce16052** + audio1 の差分(DirectSound は 0126 の「0 台」のまま = 差分は S0 に効かない)。**fa11947 への再ベースの後は S0 を取り直していない**(未確認。再ベース後に native で S1 の確認を 1 回取った: 決定文書 §検証)。 |

## 2. 音声に関係する API の呼び出し(実測。gdb の 900 秒。`logs/s0-gdb/gdbaudio.txt`)

### 2.1 回数

| API | 回数 | 備考 |
|---|---:|---|
| `DirectSoundEnumerateA` | **1** | callback `0x12af280`、ctx 0。呼び元 `f_12af3e0` ← `f_1252bd0` ← `f_1196c00`。**コールバックは呼ばれない**(0126 の「0 台」のため。`f_12af280` に breakpoint を置いて 0 回) |
| `DirectSoundCreate` / `DirectSoundCreate8` / `DirectSoundEnumerateW` / `DirectSoundCaptureCreate` | **0** | |
| `midiInGetNumDevs` | 1 | `f_ba24f0` ← `f_117cd70` ← `f_1196c00`(52.7 秒) |
| `midiOutGetNumDevs` | 4 | 52.7 秒に 2 回(`f_ba3f60` / `f_ba4420`)、268.9 秒に 2 回 |
| `midiOutGetDevCapsW` | 2 | 52.7 秒(dev 0xffffffff = MIDI_MAPPER)、268.9 秒 |
| `midiOutOpen` | 1 | 268.9 秒、`dev=0xffffffff`(MIDI_MAPPER)、`f_ba4950` ← MIDI プラグイン(`f_f8fcc0`…)。DirectSound とは別の経路 |
| `timeBeginPeriod` | 2 | 引数 1。6.6 秒(`f_1e85480`、スレッドプール系)と 6.6 秒(`f_663450`) |
| `timeGetDevCaps` | 2 | 6.6 秒 |
| `timeGetTime` | 428 | 数えただけ(引数は記録していない) |
| `acmMetrics` | 2 | 1.8 秒、2.2 秒。音声出力とは無関係と読む |
| `CoInitialize` / `CoCreateInstance` / `CoUninitialize` | 1 / 1 / 1 | CLSID `{56FDF344-FD6D-11D0-958A-006097C9A090}`(タスクバーの `ITaskbarList`。決定 0129 の壁 16 の経路)。**音声ではない** |
| `LoadLibraryW "avrt"` | 3 | 1.3、1.7、2.1 秒。`AvSetMmThreadCharacteristics` 等の呼び出しは **0 回**(`avrt` は読み込まれるだけ) |
| `CreateThread` | 16 | 開始アドレスはすべて `0x412ae0`(TThread のラッパー)か `0x1d0fdb0`(スレッドプール)。**`TMixerThread_DS` を作る `f_12afaf0` は未到達なので、ミキサースレッドは 0**。52.7〜53.2 秒に作られる 2 本は `f_bb1410`(MIDI/プラグイン管理)と `f_92f550` から |
| `SetThreadPriority` | 11 | |
| `Sleep` | 331 | 引数ごとの内訳は取っていない(未確認) |
| `WaitForMultipleObjectsEx` | 6 | |

### 2.2 初期化の順(52.7〜53.2 秒。実測)

呼び元はすべて `f_1196c00`(音声設定の初期化。`f_11a57c0` ← `f_11be0a0` ← `f_13751e0` ← `CreateFruityInstance` 0x1375990 の下)。

| # | 経過 | 関数 / API | 結果 |
|---:|---:|---|---|
| 1 | 52.7 s | `midiInGetNumDevs`、`midiOutGetNumDevs`、`midiOutGetDevCapsW(MAPPER)` | MIDI のデバイス数の取得 |
| 2 | 52.8 s | `f_1252bd0`(デバイス一覧の構築) | 一覧を空にして以下を呼ぶ |
| 3 | | `f_c55ad0`(ASIO 列挙)→ `f_c526f0`(`software\asio` を開く)| `f_c526f0` が 0 を返す = **ASIO は 0 件** |
| 4 | | `f_12af3e0`(DS 列挙)→ **`DirectSoundEnumerateA(0x12af280, 0)`** | コールバックは呼ばれない |
| 5 | | `f_12517f0`(Plugin 列挙。`[0x15e01b0]` が 0 のとき何もしない = 読み)| 戻り 0 |
| 6 | | `f_1252bd0` の戻り | 一覧のメモリ(`[0x16bd328..+64]`)に、DS・ASIO の要素の代わりに 0xffffffff(種別 -1 = "(none)" と読む)が 1 つ現れる(実測はメモリのダンプ)。"(none)" の足し方は audio-m5.md §3.1 の読み |
| 7 | 52.8 s | `f_1252d80`(保存した `Device name` の一致)| 0(添字 0) |
| 8 | 52.8 s | `f_13582c0` → `f_1253610`(1 回目)→ `f_1253260`(1 回目)| `f_1253260` が 0 を返す。種別 `[0x15ce278]` = 0xffffffff(DS)だが**一覧が "(none)" だけなので何も開かない** |
| 9 | 53.0 s | `f_11d3340` → `f_1253610`(2 回目)→ `f_1253260`(2 回目)| 同上。0 |
| 10 | 53.2 s | `CreateThread`(`f_bb1410`)、`SetThreadPriority(prio=1)`、`CreateThread`(`f_92f550`) | 用途は未確認(アドレスが MIDI 関連の `f_ba…` の近く。音声のミキサーではない: `f_12afaf0` は未到達) |

`f_12afb50`(DS 初期化)・`f_12af5c0`(セカンダリバッファ)・`f_12afaf0`(ミキサースレッド作成と `Play`)・`f_12aeec0`(`TMixerThread_DS.Execute`)・`f_c589b0`(ASIO を開く)・`f_c53c60`(ASIO ドライバーの作成)・`f_12b00f0`・`f_12aedf0`・`f_12affd0` は、**どれも 900 秒の間に入口に届かない**(GCOUNT に現れない)。

### 2.3 音声の状態(実測。`f_c55ad0` 以降の各戻りで読んだ)

| 変数 | アドレス | 値 | 意味(audio-m5.md の読み) |
|---|---|---|---|
| 選んだ種別 | `[0x15ce278]` | 0xffffffff | -1 = DirectSound |
| ポーリング | `[0x15d3294]` | **1** | ポーリング(`Sleep(1)` + `GetCurrentPosition`)。通知モードは使われない |
| ソフトウェア | `[0x15d3295]` | **1** | `DSBCAPS_LOCSOFTWARE` を付けない側(audio-m5.md §3.4: 「偽でなければ付けない」)|
| float32 | `[0x15d3296]` | **0** | 16 ビット PCM |
| ブロック長 | `[0x16bd4a0]` | **0x1000 = 4096** フレーム | 44.1 kHz で 92.9 ms |
| `IDirectSound`・セカンダリ・プライマリ・通知 | `[0x16bd4f8]`、`[0x16bd498]`、`[0x16bd500]`、`[0x16bd508]` | すべて 0 | 何も作られていない |

## 3. 0 台のときのゲストの分岐(実測 + 読み)

- **実測**: 一覧に要素が 1 つも足されないとき、`f_1252bd0` は末尾に "(none)" を足す(`[0x15e01b0]` = 0)。`f_1253260` は種別 -1(DS)でも、一覧の選択が "(none)" なので `f_12afb50` を呼ばない。エラーダイアログ `f_1253080`(`Error|…`)も呼ばれない。
- **読み**: audio-m5.md §3.1 の読みと一致。
- **帰結**: 決定 0126 の「0 台」は、**ゲストが静かに音声出力なしで起動する**構成であり、S0 の時点で `TMixerThread_DS` の挙動を観察する方法は無かった。S1 は 1 台を列挙して初めて、ミキサー経路を実行できる。

## 4. 実エンジンの DirectSound の呼び出し(S1 の後。native。参考として同じ文書に置く)

S1 のモデルで 1 台を列挙したときの native の実測(old 木 = ce16052 + 差分。`logs/s1a/`、`logs/s1b/`): 

| 順 | 呼び出し | 引数(実測) |
|---:|---|---|
| 1 | `DirectSoundEnumerateA` | callback `0x12af280` → モデルが 2 回呼ぶ(§決定文書) |
| 2 | `DirectSoundCreate` | 選んだ要素の GUID |
| 3 | `IDirectSound::SetCooperativeLevel` | hwnd `0x7f990034`(仮想窓)、`DSSCL_PRIORITY`(2) |
| 4 | `CreateSoundBuffer`(プライマリ) | flags 1 = `DSBCAPS_PRIMARYBUFFER`、サイズ 0、形式 NULL |
| 5 | `IDirectSoundBuffer::SetFormat`(プライマリ) | 44100 Hz、16 ビット、2 ch |
| 6 | `CreateSoundBuffer`(セカンダリ) | flags `0x18000`(`GLOBALFOCUS` + `GETCURRENTPOSITION2`)、32768 バイト = 2 ブロック × 4096 フレーム |
| 7 | `SetCurrentPosition(0x4000)` → `Play(LOOPING)` → `Stop`(+6.6 ms)→ `Lock(全体)` → ゼロ → `Unlock` → `SetCurrentPosition` → `Play(LOOPING)` | 起動時の 1 度の開始と停止の確認と読める |
| 8 | `Sleep(1)` + `GetCurrentPosition` のポーリング | **約 92.4 ms ごと**に半分(16384 バイト)を `Lock` / `Unlock`(最初は +102 ms)。ブロックの peak は 0(無音) |

300 秒の間に新しい壁は無い(native)。

## 5. 限界(確認できなかったこと)

- **wasm では API の呼び出し列を取っていない**(§1)。native の gdb の結果を wasm に当てはめるのは、生成 C が同じことからの読み。
- **実機の Windows との照合は 0 件**。実機が何台の DirectSound デバイスを返すか、順序、説明文字列は取っていない(決定文書の S-ds-device-strings / S-ds-guid)。
- 900 秒より先の呼び出しは見ていない。FL のプロジェクトの読み込み、再生の開始(G8)は走っていない。`Sleep` の引数、`timeGetTime` の引数は記録していない。
- gdb の breakpoint は import の**スタブ**(ランタイムの `fle_imp_*`)に置いた。ゲストが `GetProcAddress` で取った関数ポインタ経由で呼んでも、ランタイムの同じスタブを通る(読み)ので掛かるはずだが、**確認はしていない**。ログに名前を残した `GetProcAddress` は 1,866 回のうち 41 行だけで、その 41 行に `DirectSound*`・`waveOut*`・`mixer*`・`Av*` は無い(`grep` 0 件)。
- audio-m5.md への訂正は無いが、**そこに書かれた静的な数(呼び出し箇所の数など)はこの観察の対象外**。
