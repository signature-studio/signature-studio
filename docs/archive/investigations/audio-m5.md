# M-5 音声デバイス(DirectSound と ASIO の最小モデル、再生位置と通知、ミキサースレッド)— 実装前の静的調査

**静的調査のみ**(実行・ビルドはしていない)。対象は `binaries/win64/FLEngine_x64.dll`(SHA-256 `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`。指示の値と一致を `sha256sum` で確認。ImageBase 0x400000)。リポジトリは `main`、HEAD 4ff4f33。道具は python の capstone 5.0.7 と pefile 2024.8.26。

凡例(先の調査文書と同じ使い方): **実測** = バイナリのバイト列・表・文字列から機械的に取り出した事実(実行はしていない)。**読み** = 私の解釈。**未確認** = 確かめていない。**引用** = 既存文書・旧ツリーの記述で、ここでは再測定していない。Learn は**取得できたページの文言だけ**を事実として使う(§2.0 に URL と取得の強さを書く)。

## 0. 要約(先頭。見つかったことをそのまま書く)

1. **使われる音声 API**(FLEngine。呼び出し箇所は `E8` で import thunk へ + `FF 15` を capstone で検証した数。表は §2)。
   - **dsound**: import は `DirectSoundCreate`(呼び出し 2 箇所、使い手 1 関数 `f_12afb50`)と `DirectSoundEnumerateA`(1 箇所、`f_12af3e0`)の 2 つだけ。`DirectSoundCreate8`、`DirectSoundCaptureCreate`、`DirectSoundNotify` 系の import は無い(通知は COM の `QueryInterface` で得る)。
   - **winmm**: import 24 のうち時間 4(`timeGetTime` 79 箇所/68 関数、`timeBeginPeriod` 2、`timeEndPeriod` 2、`timeGetDevCaps` 2)、`midiIn*` 11(`midiInOpen` 1 箇所)、`midiOut*` 9(`midiOutOpen` 1 箇所)。**`waveOut*`、`timeSetEvent`、`mixer*` は import に無い**。
   - **COM**: `CoCreateInstance` 14 箇所(ASIO ドライバーを作るのは `f_c53c60` の 1 箇所。ほかの 13 は音声と無関係と読むが、**読んだのは 2 つだけ**)、`CoInitialize` 4、`CoGetClassObject` 1。ASIO ドライバーの列挙は**レジストリ `HKLM\software\asio\*` の `clsid` 値**(`f_c526f0`)。
   - **スレッド・同期**: ミキサースレッドの待ちは **`WaitForMultipleObjects` 1 箇所(`f_12aeec0`、DS の通知モード)**、`Sleep(1)`(DS のポーリングモード。`f_5264e0` 経由)、`WaitForSingleObject`(ASIO ミキサー `f_c55740` は INFINITE)、`SignalObjectAndWait` 1 箇所(ASIO のコールバック `f_c56ae0`)。優先度は `SetThreadPriority`(`f_525ab0`)で、**既定の添字 6 = `THREAD_PRIORITY_TIME_CRITICAL`**(表 `0x13bd8c8`)。過負荷のとき `f_1253840` が添字 3(`NORMAL`)に落とす。協調スレッド(決定 0122)では優先度は無視されるので、影響しない。
2. **DirectSound の使い方(読み。DS の vtable 呼び出し 18 点(領域内 16 + 領域外 2。受け手は 4 つのグローバルで確定。領域内の `call [reg+disp]` は全部で 31 点で、残り 15 は UI・関数ポインタ・Delphi のデストラクタ・アンダーラン計数))**: `DirectSoundCreate(選択した GUID または NULL)` → 失敗なら NULL で再試行 → `SetCooperativeLevel(hwnd, DSSCL_PRIORITY)` → プライマリ `CreateSoundBuffer`(`DSBCAPS_PRIMARYBUFFER`、サイズ 0、形式 NULL)→ `SetFormat`(**16 ビット PCM か、float32 の `WAVE_FORMAT_EXTENSIBLE`**。どちらかは設定の bool)→ イベント 2〜3 個作成 → セカンダリ `CreateSoundBuffer`(**2 ブロック分のストリーミング**、`GETCURRENTPOSITION2 | GLOBALFOCUS`、通知モードなら `CTRLPOSITIONNOTIFY`、ハード無効なら `LOCSOFTWARE`)→ 通知モードなら `QueryInterface(IID_IDirectSoundNotify)` と `SetNotificationPositions`(2 点)→ ミキサースレッド作成 → `SetCurrentPosition(1 ブロック目の先)` → `Play(LOOPING)`。ミキサーは 1 ブロックごとに `Lock`/`Unlock` で**半分ずつ**書く。**使わないメソッド**: `GetCaps`、`GetSpeakerConfig`、`SetVolume`、`GetStatus`、`Restore`、`DuplicateSoundBuffer`(この領域では 0 点)。
   - **通知かポーリングか**: **両方の実装がある**。どちらで走るかは設定 bool `0x15d3294`(UI の "Use polling")で決まり、**起動時の既定がどちらかは読み切れていない(未確認)**。静的イメージの初期値は 1(= ポーリング)だが、起動中に UI コントロールの状態で上書きされる(`f_11a57c0` の 0x11acce8 と `f_135b000`)。**どちらでも動くモデルが要る**。
3. **ミキサースレッド(`TMixerThread_DS.Execute` = `f_12aeec0`、948 B)**: 周期は **1 ブロック = 既定 4096 フレーム(44.1 kHz で 92.9 ms)ごと**。ポーリング: `Sleep(1)` + `GetCurrentPosition(&play, NULL)` を、`(play + lead) mod 全長 < ブロック長` の真偽が前回と変わるまで繰り返す(1 ブロックにつき最大約 93 回)。通知: `WaitForMultipleObjects(2, 通知イベント, FALSE, INFINITE)`、戻り値の添字 `i` から「次に書く半分 = i xor 1」。起床後はミキサー用 CS を取り、ミキサー本体(`f_d0ece0`)で 1 ブロック分を作り、`Lock`/`Unlock` で書き、**`GetCurrentPosition(NULL, &write)` で write カーソルが今書いた半分に入っていないか**を調べ、入っていればアンダーランを数える。**終了条件**は `Terminated`(`[this+0x1a]`)。通知モードではイベントを `SetEvent` して起こさないと INFINITE から出られない。
4. **ASIO が G7/G8 に要るか(読み)**: **要らない見込み**。根拠: ASIO のドライバーは `HKLM\software\asio\<名前>` の下位キーが**無ければ 1 つも列挙されない**。仮想レジストリは HKLM に空の `Software` だけ(決定 0085)で ASIO キーは無い。デバイス一覧の先頭は ASIO(あれば)、次に DS、次に Plugin で、保存された `Device name` が一致しなければ**添字 0 = 一覧の先頭**が選ばれる(`f_1252d80`)。ASIO が 0 件なら先頭は `DirectSoundEnumerateA` の最初のデバイスになる。**ただし** `DirectSoundEnumerateA` が 1 件も返さないと一覧は "(none)" だけになり、音声デバイスが開かない(読み)。実機では `FL Studio ASIO`(`ILWASAPI2ASIO_x64.dll`)が登録され、既定として選ばれると読める。**この DLL は `lift/out-inventory/extracted/` に無い(入手できていない)**。
5. **推奨(決めるのは PM)**: 段階導入 **S1 DirectSound の状態モデル(native)→ S2 再生位置と通知(仮想時計の源。決定的)→ S3 wasm と AudioWorklet(実時間の源。ここで初めて G7 の字義を満たす)→ S4 ASIO(不要が確認できれば作らない)**。再生位置は**「消費フレーム数」1 本のカウンタを唯一の源とし、源を差し替える**形(テスト用 = 仮想時計から、ブラウザ用 = AudioWorklet の消費から)。詳細と判断点は §6〜§7。
6. **診断で到達していない部分**: `CreateFruityInstance` は壁 52(`SetWindowOrgEx`)まで進んだが(HEAD のコミット記述)、**音声の初期化に届いた実行の記録は見つかっていない**(診断 1 の `lanes.md`(引用): `DirectSoundCreate`・`midiOutOpen`・`CoCreateInstance` は 4 回の実行のどれにも出ない。HEAD では自分で走らせていない)。静的には、初期化の本体 `f_13751e0` の下で `f_11be0a0 → f_11a57c0 → f_1196c00 → f_1252bd0/f_11d3340 → f_1253610 → f_1253260` が**関数内の制御流れでは必須の経路が多い**(§2.3。`CreateFruityInstance` から `f_13751e0` の呼び出しだけは飛ばせる)。**実行時の値(どの種類のデバイスが選ばれるか、ポーリングか通知か、ブロック長)は実測できていない**。

## 1. 手順

作業は `/tmp/opencode/aud/work/`(RAM)。スクリプトは `docs/investigations/audio-m5/` にコピーした。**ビルド・実行・git の変更系操作はしていない**。

1. `scan_audio_sites.py`: import(通常と遅延)の IAT スロットを引き、`.text` の `FF 25`(thunk)、`E8/E9 rel32` が thunk に向かうもの、`FF 15` を拾い、`.pdata` の関数に属するかと、関数の先頭からの capstone 線形掃引で命令境界に一致するかを検査する(`docs/investigations/thread-model/scan_sites.py` と同じ方法。**対象 API の集合を音声・COM・レジストリ・待ち系に広げた**)。出力は `api-sites.tsv`(FLEngine、488 行)、`bundled-dll-sites.tsv`(同梱 5 DLL、153 行)、`api-summary.tsv`。`make_tables.py` が表を再生成する(同一出力を確認)。
2. `fdis.py`(関数単位の逆アセンブル、IAT 呼び出しと短い文字列の注釈、`--vcalls` で `call [reg+disp]` 一覧)、`xref_str.py`(文字列とその `lea` 参照)、`xref_data.py`(データ番地への rip 相対参照、全関数を capstone で掃引)、`callers.py`(直接呼び元)、`funclist.py`、`mustpass.py`(関数内の基本ブロック CFG で「ある呼び出しが関数の全経路上にあるか」を判定。例外経路は模していない)。
3. DirectSound の受け手は、`rcx` のロード元グローバルで分類した(旧ツリーの `fl_ds_surface.py` と同じ考え方。結果は §3.2 で旧報告と一致する)。vtable の並びは**ローカルの mingw-w64 `dsound.h`** の宣言順(§3.2 の注)。
4. バッファ算術は `buffer_math.py` に写した(読んだ式の再計算。測定ではない)。
5. Learn と MDN は §2.0 の URL を取得した。

## 2. 使われる API の棚卸し(実測。呼び出し箇所の数と呼び元)

### 2.0 外部の文言(取得できた範囲だけ事実。2026-10-31 取得)

| 項目 | URL(取得の強さ) | 使った文言(短い断片) |
|---|---|---|
| DirectSoundCreate | `learn.microsoft.com/en-us/previous-versions/windows/desktop/mt708921(v=vs.85)`(検索結果の抜粋) | lpGuid は「one of the GUIDs returned by DirectSoundEnumerate, or NULL for the default device」。「The application must call SetCooperativeLevel immediately after creating a DirectSound object」 |
| DSEnumCallback / 列挙 | `…/ee416829(v=vs.85)`、`…/ee417545(v=vs.85)`(抜粋) | 「The primary device is enumerated twice, first with NULL in lpGuid and then with its GUID」。コールバックは TRUE で続行 |
| SetCooperativeLevel | `…/ee418049(v=vs.85)`(抜粋) | 「The application must set the cooperative level … before its buffers can be played. The recommended cooperative level is DSSCL_PRIORITY」 |
| Play | `…/ee418074(v=vs.85)`(**全文取得**) | 「If SetCooperativeLevel has not been called, the method returns DS_OK, but no sound will be produced」。プライマリ: 「Primary buffers must be played with the DSBPLAY_LOOPING flag set」。「the thread that plays the buffer must continue to exist as long as the buffer is playing」 |
| DSBUFFERDESC | `…/ee416820(v=vs.85)`(**全文取得**) | プライマリは「dwBufferBytes … must be zero」「lpwfxFormat … must be NULL for primary buffers」。「DSBCAPS_CTRLPOSITIONNOTIFY … On VXD drivers … always a software buffer」 |
| GetCurrentPosition | `…/mt708925(v=vs.85)`、`…/ee418062(v=vs.85)`(抜粋) | 「The write cursor always leads the play cursor, typically by about 15 milliseconds' worth of audio data」。「It is always safe to change data that is behind the position indicated by … the play cursor」 |
| Play and Write Cursors | `…/ee418744(v=vs.85)`(抜粋) | 「When a buffer is created, the play cursor is set to 0 … When the buffer is stopped, the cursor remains where it is」。「always specify the DSBCAPS_GETCURRENTPOSITION2 flag」 |
| Lock | `…/ee418073(v=vs.85)`(抜粋) | 「The buffer is conceptually circular」。第 2 ポインタは折り返したときだけ。「immediately call Unlock」 |
| SetNotificationPositions | `…/ee418245(v=vs.85)`、`…/ee418746(v=vs.85)`(抜粋) | 「whenever the read or play cursor reaches one of the specified offsets, the associated event is signaled」。「The buffer must be stopped when this method is called」。「Notifications for hardware-controlled buffers are not provided reliably by all drivers」 |
| WaitForMultipleObjects | `learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitformultipleobjects`(**全文取得**) | bWaitAll = FALSE: 「the return value minus WAIT_OBJECT_0 indicates the lpHandles array index」「array index of the signaled object with the smallest index」 |
| Web Audio の自動再生 | `developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API/Best_practices`(**全文取得**) | 「Create or resume context from inside a user gesture」。ジェスチャー外で作ると `suspended` |

**取得できなかった / していない**: `IDirectSound8::CreateSoundBuffer`、`GetStatus`、`SetCurrentPosition`、`IDirectSoundBuffer8::Stop`、`Unlock`、`GetFormat/SetFormat` の各ページ(検索結果の抜粋にも主要文言が無かった)。したがって **`SetCurrentPosition` が再生中に失敗するか**などは、旧ツリーの記述(§4.5 の「引用」)に頼る未確認の点として扱う。

### 2.1 FLEngine の import(実測)

| DLL | import 名(通常 + 遅延) | 音声に関係する名前 |
|---|---|---|
| dsound.dll | 2 | `DirectSoundEnumerateA`、`DirectSoundCreate` |
| winmm.dll | 24 | `timeGetTime`、`timeBeginPeriod`、`timeEndPeriod`、`timeGetDevCaps`、`midiIn*` 11、`midiOut*` 9 |
| ole32.dll | 29 + 遅延 1 | `CoCreateInstance`、`CoGetClassObject`、`CoInitialize`、`CoUninitialize`、`CLSIDFromString`、`StringFromCLSID`、`ProgIDFromCLSID`、`CoTaskMemAlloc/Free`、`CoMarshalInterThreadInterfaceInStream` ほか。**`CoInitializeEx` は無い** |
| msacm32.dll | 14 | `acm*`(ファイル変換用と読む。引用: 旧 INVENTORY §2)。リアルタイム経路ではない |
| avrt.dll | **import に無い** | `LoadLibraryW(L"avrt")` + `GetProcAddress` で動的(`f_65c9f0`、`f_12b00f0`: `"Pro Audio"` で `AvSetMmThreadCharacteristicsW` 相当を呼ぶ。NULL なら呼ばない。決定 0088) |
| (無い) | — | `waveOut*`、`waveIn*`、`mixer*`、`timeSetEvent`、`timeKillEvent`、`DirectSoundCreate8`、`DirectSoundCaptureCreate*` |

### 2.2 呼び出し箇所の表(実測。`api-sites.tsv` から)

数は「`call` が thunk または IAT へ向かう箇所」。thunk 自身(`jmp [IAT]`)は含めない。「検証」= `.pdata` の関数内で capstone の命令境界に一致した数。**`api-sites.tsv` の全 488 行が検証済み**(`verified = yes`。表の「検証」列は箇所の数と同じ)。

| API | 箇所 | 検証 | 関数数 | 音声での使い手(呼び元) |
|---|---|---|---|---|
| `DirectSoundCreate` | 2 | 2 | 1 | `f_12afb50`(0x12afbe4 と 0x12afc01。1 回目は選んだ GUID、2 回目は NULL) |
| `DirectSoundEnumerateA` | 1 | 1 | 1 | `f_12af3e0`(0x12af437) |
| `timeGetTime` | 79 | 79 | 68 | ミキサー `f_12aeec0`(0x12aef9c)、デバイス管理 `f_1253260`(0x12532d9)、ASIO `f_c56ae0`(0xc56b3e) ほか 65 関数(ほとんどはゲームの時刻・UI。`goal-ladder.md` §3.2 の引用) |
| `timeBeginPeriod` / `timeEndPeriod` / `timeGetDevCaps` | 各 2 | 各 2 | 2 | 2 つの関数: **`f_81c0d0`**(グローバルな周期。0x81c141 ほか。呼び元は `f_81d390` と、`f_11be0a0` の 0x11be194)と **`f_6d8b00`**(オブジェクトのメソッド。呼び元 `f_6d8a80`)。0x11be194 は `f_11be0a0` の全経路上だが(mustpass)、`CreateFruityInstance` から `f_13751e0` を呼ぶ 0x1376545 は飛ばせる経路があるので、**「`CreateFruityInstance` で必ず通る」とは言えない**(§2.3)。診断 1 では `timeBeginPeriod` と `timeGetDevCaps` が出ている(引用) |
| `midiOutOpen` / `midiInOpen` | 各 1 | 各 1 | 各 1 | `f_ba4950`(0xba498b)、`f_ba3010`(0xba305b。`CALLBACK_FUNCTION`。引用: 旧 INVENTORY) |
| `CoCreateInstance` | 14 | 14 | 14 | 0x5d091b、0x5df663、0x611ba6、0x70aecd、0x87c21a、0x87c4ea(`{D57C7288-…}`、`{84BCCD23-…}`の文字列あり)、0x9f4ae0、0x9f7a47、0xbef4cc、**0xc53cce(`f_c53c60` = `TOpenASIO` のコンストラクタ。ASIO)**、0x13346d5、0x1334a45、0x1335809、0x1338105。**音声は 0xc53cce の 1 箇所と読む**(他は `f_5df610` の `"%s, ClassID: %s"` のログ、`f_87c1d0`/`f_87c4a0` の固定 CLSID など。13 箇所のうち中身を読んだのは 3 関数だけで、他 10 は未確認) |
| `CoInitialize` | 4 | 4 | 4 | `f_c53c60`(0xc53ca1。ASIO の前に `CoInitialize(NULL)`)、0x5e0e15、0x63ae1d、0x7001e7(`TDropTargetTransferThread`) |
| `CoGetClassObject` | 1 | 1 | 1 | `f_bef3a0`(0xbef41f。音声かは未確認) |
| `CreateThread` | 3 | 3 | 3 | `f_412b50`(共通のスレッド起動。0x412bc6)、`f_8670c0`(0x867152)、`f_8b42c0`(0x8b42ea) |
| `ResumeThread` / `SuspendThread` / `ExitThread` | 2 / 1 / 1 | 同 | 2 / 1 / 1 | `f_5259a0`(Start)、`f_526590`、`f_526510`、`f_412bf0` |
| `SetThreadPriority` / `GetThreadPriority` | 1 / 2 | 同 | 1 / 2 | `f_525ab0`(0x525aca。表 `0x13bd8c8` = `{-15,-2,-1,0,1,2,15}` を添字で引く) |
| `WaitForMultipleObjects` | 1 | 1 | 1 | **`f_12aeec0`(0x12aef73)= DS ミキサー**のみ |
| `WaitForSingleObject` | 16 | 16 | 16 | ASIO ミキサー `f_c55740`(0xc55760。INFINITE)、ASIO コールバック `f_c56ae0`(0xc56bb7。時間は `[0x16b8814]`)、`f_526610`(TThread.WaitFor)、`f_1267380`(TStreamThread)ほか |
| `SignalObjectAndWait` | 1 | 1 | 1 | `f_c56ae0`(0xc57084。ASIO のコールバックが ASIO ミキサーを起こして自分は待つ) |
| `Sleep` | 33 | 33 | 25 | DS のポーリングは `f_5264e0`(`Sleep(ecx)` の薄い包み。0x12aeee8 で `ecx = 1`)経由(`f_5264e0` の呼び元は 14 箇所/10 関数)。ほかは `f_4074d0` など Delphi RTL |
| `SwitchToThread` | 3 | 3 | 3 | `f_4103e0`、`f_481a00`、`f_5264f0`(デストラクタの待ち。thread-model.md §5) |
| `CreateEventW` | 17 | 17 | 14 | `f_12aed10`(0x12aed35 / 0x12aed6c: 通知イベント 2 個 + 3 個目)、`f_c54e90`(0xc54eb5: ASIO) |
| `SetEvent` / `ResetEvent` | 17 / 8 | 同 | 17 / 7 | `f_12aee30`(0x12aee81)、`f_c55630`、`f_c556a0`、`f_c55740`、`f_c56ae0` ほか |
| `CreateTimerQueueTimer` / `DeleteTimerQueueTimer` | 各 1 | 各 1 | 各 1 | `f_6d8be0`(音声との関係は未確認) |
| `RegOpenKeyExW` / `A` | 14 / 25 | 同 | 7 / 24 | ASIO の列挙は `f_5af7c0`(`TRegistry.OpenKey` 相当)経由。**呼び元の数は VCL のラッパーのため直接の箇所では数えにくい**(§4.2) |
| `QueryPerformanceCounter` / `Frequency` | 25 / 8 | 同 | 21 / 8 | `f_12aeec0`(0x12aefbc)、`f_1253260`、`f_1253800`、`f_1253840`、`f_1311a00` ほか |

**同梱 DLL(`bundled-dll-sites.tsv`、実測)**: dsound・winmm の `midi*`・`CoCreateInstance`(音声)は **FLEngine 以外には無い**。`timeBeginPeriod` は QFC 1 + ILTools 2、`CoCreateInstance` は ILTools 2(音声と無関係と読む)、`CreateThread` は QFC 1 / ILTools 2 / Blend2D 1。**`dsp_ippv2_x64.dll` は待ち・スレッド・COM・時間の呼び出し 0 件**(`sites-dsp_ippv2` の走査で 0)。したがって **M-5 は FLEngine の 1 モジュールの問題**で、QFC の `TTimerThread`(`Sleep` + `DwmFlush`)は音声と独立。

### 2.3 音声の呼び出し関係(直接 call。実測。`callers.py`)

```
CreateFruityInstance(0x1375990) → f_13751e0(0x1376545) → f_11be0a0(0x1375220) → f_11a57c0(0x11be588) → f_1196c00(0x11acf23)
 f_1196c00 ─ 0x1199479 → f_1252bd0  (デバイス一覧を作る。f_c55ad0 = ASIO 列挙、f_12af3e0 = DS 列挙、f_12517f0 = Plugin 列挙)
          ─ 0x1199496 → f_1252d80  (保存した "Device name" で一覧の添字を決める)
          ─ 0x11994cd → f_13582c0  (一覧の添字から出力を選び、その中で f_1253610 を呼ぶ)
          ─ 0x119950a → f_11d3340  (f_11d3340 の 0x11d33a6 → f_1253610)
 f_1253610(音声出力の開始)→ f_1253260(0x1253649。種類で分岐: DS / ASIO / Plugin)
 f_1253260 ─ DS:     f_12afb50(0x1253346)→ f_12afaf0(0x125335e: ミキサー作成 + SetCurrentPosition + Play)
           ─ ASIO:   f_c589b0(0x125338d)→ f_c58940(0x12533a5: ミキサー作成)
           ─ Plugin: f_1251b30(0x12533bf)→ f_1251b20
```

`mustpass.py` の結果(**実測、関数内の制御流れだけ。例外・戻らない呼び出しは模していない**): `f_1196c00` の 0x1199479〜0x119950a、`f_11a57c0` の 0x11acf23、`f_11be0a0` の 0x11be588 と 0x11be194(`timeBeginPeriod` 経路)、`f_13751e0` の 0x1375220 は**各関数の全経路上**にある。`f_1375990`(`CreateFruityInstance`)から `f_13751e0` を呼ぶ 0x1376545 は**飛ばせる経路がある**(失敗時の早期戻り。BasicInitialization 失敗など)。`f_13582c0` が `f_1253610` を呼ぶ 0x1358364 は**飛ばせる**(mustpass: skippable)が、`f_11d3340` が呼ぶ 0x11d33a6 は**全経路上**(must-pass)で、その `f_11d3340` を `f_1196c00` の 0x119950a が必ず呼ぶ。`f_1253610` 自身は **`[0x15ce278] > -1`(= 既にデバイス種別が決まっている)または `[0x15e1868] が指す byte(実体 0x16bcc7d)== 0` で早期に戻る**(0x1253615〜0x125362c)。`f_1253260` の 3 分岐は互いに排他(`mustpass`: 各分岐は飛ばせる)。**つまり静的には、`CreateFruityInstance` が `f_13751e0` を通り抜けるなら音声の初期化に届く構造で、実際にデバイスが開くかは `0x16bcc7d`(`f_11a57c0` の 0x11a83d9 が 1 を書く。`f_11d3340` の 0x11d3359 が `bl` を書く)と `[0x15ce278]`(デバイス種別。静的 -1。`f_1253070` が -1 を書く)の実行時の値による**。

## 3. DirectSound の使い方の読み

### 3.1 デバイス一覧と選択(`f_1252bd0`、`f_1252d80`、`f_1253260`)

- `f_1252bd0`(0x1252bd0、349 B): 一覧(グローバル配列 `0x16bd328`、1 要素 28 B)を空にし、`f_c55ad0`(ASIO 列挙)→ `f_12af3e0`(DS 列挙)→ `f_12517f0`(Plugin 列挙、`[0x15e01b0]` = ホストが Flags のビット 1 を立てたときだけ)を呼ぶ。`[0x15e01b0]` が 0(FL64.exe)なら末尾に "(none)" の要素(種別 -1)を足す。
- `f_c55ad0`(ASIO 列挙。0xc55ad0、523 B): `[0x15e01b0]` が 0 なら `f_c526f0`(§4.5)で見つかったドライバーの数を数え、**1 つ以上あれば**メニューの項目 "ASIO devices" と各ドライバー名を足し、一覧に種別 1 で追加する。
- `f_12af3e0`(DS 列挙。0x12af3e0、196 B): `DirectSoundEnumerateA(f_12af280, NULL)`(0x12af437)。コールバック `f_12af280` は `lpcstrDescription`(rdx)を文字列化して一覧に足し、GUID(rcx。NULL なら既定のグローバルの GUID)を 16 B 保存する。種別は 0。**列挙で 1 つ以上足されたときだけ** "DirectSound devices" の見出しを足す。
- `f_1252d80`(0x1252d80、324 B): 保存設定 `\Audio output` の `"Device name"`(既定 `"FL Studio ASIO"`)を読み、一覧の中で**名前が一致する最初の添字**を返す。一致しなければ 0。**既定の文字列が `FL Studio ASIO` なのは、レジストリが空の最初の起動では ASIO が一覧に無ければ一致せず、添字 0 になる**(読み)。
- `f_1253260`(音声出力の開始。0x1253260、695 B): `[0x15ce278]` = 選んだ種別(-1 = DS、0 = ASIO、1 = Plugin)で 3 分岐。DS: `f_617920(...)` で HWND を取り、**`f_12afb50(hwnd, &error)`**(DS の初期化)→ 成功なら `f_d0d280(0x400, 1, 0)`(ミキサーの設定)→ **`f_12afaf0`**(ミキサースレッドの作成 + 再生開始)。失敗なら `f_1253080`(エラーダイアログ。`Error|…`)。
- 種別 -1 の DS の GUID は、`f_12afb50` の入口で `[0x15e1148]`(選択した要素へのポインタ)から `+0xc` の 16 B を引く(0x12afb80〜)。**一覧の要素の GUID が NULL 既定なら `DirectSoundCreate` の第 1 引数は NULL**(読み。0x12afbc7/0x12afbcd の `rbx` = 要素 + 0xc または 0)。

### 3.2 vtable 呼び出し点(実測。`ds-vcalls.tsv`)と受け手の根拠

DS 領域(0x12ae000〜0x12b0300)と、DS のバッファ・デバイス管理を引く 4 関数の `call [reg+disp]` を全部列挙し、`rcx` のロード元で受け手を分類した。**受け手の根拠**: 4 つのグローバルが DS オブジェクトで、`f_12afb50` が `DirectSoundCreate` の `ppDS` に **`lea rcx,[0x16bd4f8]` を渡す**(0x12afbcf)= `0x16bd4f8` が `IDirectSound`。`CreateSoundBuffer` の第 2 引数ポインタ先に **`0x16bd500`**(プライマリ。0x12afc88)と **`0x16bd498`**(セカンダリ。0x12af721)が渡され、`QueryInterface` の `ppv` が **`0x16bd508`**(通知。0x12af77e)。解放用の共通関数 `f_41a770`(`vtable[2]` = `Release` を呼んで 0 を書く。56 B)が 4 つに適用される(`f_12affd0`、`f_12b0220`)。vtable の並びは `dsound.h` の宣言順(`IDirectSound`: QI 0x00、AddRef 0x08、Release 0x10、CreateSoundBuffer 0x18、GetCaps 0x20、Duplicate 0x28、SetCooperativeLevel 0x30、Compact 0x38、GetSpeakerConfig 0x40、…。`IDirectSoundBuffer`: GetCaps 0x18、GetCurrentPosition 0x20、GetFormat 0x28、…、GetStatus 0x48、Lock 0x58、Play 0x60、SetCurrentPosition 0x68、SetFormat 0x70、SetVolume 0x78、…、Stop 0x90、Unlock 0x98、Restore 0xa0。`IDirectSoundNotify`: SetNotificationPositions 0x18)。**注**: この一覧は mingw-w64 の `dsound.h`(ローカルの `/usr/x86_64-w64-mingw32/include/dsound.h`)で、Windows SDK の版とは**照合していない**。DirectSound の COM の vtable は互換性のため固定されているので同じと読むが、SDK の版との一致は**未確認**。

| 受け手 | メソッド | 点 | 呼び元関数(点) |
|---|---|---|---|
| `IDirectSound` | `SetCooperativeLevel`(0x30) | 1 | `f_12afb50`(0x12afc3a) |
| `IDirectSound` | `CreateSoundBuffer`(0x18) | 2 | `f_12afb50`(0x12afcaf: プライマリ)、`f_12af5c0`(0x12af748: セカンダリ) |
| プライマリ | `SetFormat`(0x70) | 1 | `f_12afb50`(0x12afd3d) |
| プライマリ | `GetFormat`(0x28) | 1 | `f_12b0020`(0x12b0045。UI 表示) |
| セカンダリ | `QueryInterface`(0x00) | 1 | `f_12af5c0`(0x12af7be) |
| セカンダリ | `GetCurrentPosition`(0x20) | 3 | `f_12aeec0`(0x12aef11: play のみ、0x12af1e5: write のみ)、`f_1311a00`(0x1311b30: UI の曲位置。play のみ) |
| セカンダリ | `Lock`(0x58) | 2 | `f_12aeec0`(0x12af124)、`f_12af520`(0x12af571。全体を無音化) |
| セカンダリ | `Unlock`(0x98) | 2 | `f_12aeec0`(0x12af19b)、`f_12af520`(0x12af5b0) |
| セカンダリ | `SetCurrentPosition`(0x68) | 1 | `f_12afaf0`(0x12afb27) |
| セカンダリ | `Play`(0x60) | 1 | `f_12afaf0`(0x12afb46) |
| セカンダリ | `Stop`(0x90) | 2 | `f_12aee30`(0x12aee4f)、`f_1253b50`(0x1253b9a。経由は `[0x15e3d30]` が指す `0x16bd498`) |
| `IDirectSoundNotify` | `SetNotificationPositions`(0x18) | 1 | `f_12af5c0`(0x12af886) |
| 全部 | `Release`(0x10) | (共通) | `f_41a770` |

**領域内の `call [reg+disp]` の全数(実測。`fdis.py --vcalls`、`.pdata` の 18 関数)**: **31 点**。内訳は **DS の 4 つのグローバルが受け手と確定したもの 16 点**(`f_12aeec0` の 4、`f_12af5c0` の 3、`f_12afb50` の 3、`f_12af520` の 2、`f_12afaf0` の 2、`f_12aee30` の 1、`f_12b0020` の 1)、**UI のリスト `[rsi+0x4f8]` の `Count` 2 点**(0x12af429、0x12af449)、**DS 以外 13 点**: UI のリストへの呼び出し(vtable `+0x78`/`+0xc8`: 0x12af2e7、0x12af487、0x12afd8b、0x12afdaf)、関数ポインタのグローバルを引く `call [rax]`(0x12af153 = float コピー `[0x15e2d40]`、0x12af1cc = メータ `[0x15e03a0]`、0x12af588 = `[0x15e0a48]`、0x12b014c = avrt のスロット `[0x15e1fe8]`、0x12b016a = `[0x15dfee8]`、0x12b01ba = `[0x15dfb88]`)、Delphi のデストラクタ(0x12b00bc: `call [rbx-0x20]`)、`[0x15e4790]->[+8]` の vtable `+0x88`(0x12af229。アンダーラン計数)、`f_12ae050` の `[r13+0x18]`(0x12ae369。UI)。**16 + 2 + 13 = 31**。表の 18 点 = 領域内の DS 16 点 + **領域外の `f_1253b50` と `f_1311a00` の 2 点**(`xref_data.py` で全数)。

**使わないもの(この領域で 0 点)**: `IDirectSound::GetCaps`、`GetSpeakerConfig`、`DuplicateSoundBuffer`、`Compact`、`Initialize`;`IDirectSoundBuffer::GetCaps`、`GetStatus`、`Restore`、`SetVolume`、`SetPan`、`SetFrequency`、`GetFrequency`、`GetVolume`。**旧 INVENTORY の集計が `SetVolume ×3`、`GetFormat ×3` などを数えたのは、同じ disp の Delphi オブジェクトへの仮想呼び出し(受け手が DS でない)を含んだため**(旧 REPORT_DS が訂正済み。私の受け手分類も一致: 0x12af2e7(`[+0x78]`)などは `0x15e13b8` 配下の UI オブジェクト)。**DS の領域外にある DS メソッドの呼び出し**は、受け手のグローバルが `0x16bd4f8`、`0x16bd500`、`0x16bd498`、`0x16bd508` を参照する関数を `xref_data.py` で全数した結果、**`f_1311a00` と `f_1253b50` の 2 関数だけ**(`0x15e3d30` は `0x16bd498` への間接セル)。それ以外の関数は 0。

### 3.3 呼び出しの順序と引数(読み。`f_12afb50`、`f_12af5c0`、`f_12afaf0`、`f_12aeec0`)

1. **`f_12afb50`(DS の初期化。640 B)**
   1. `[0x16bd4f8] != NULL` なら何もせず 1 を返す(0x12afb72)。
   2. `DirectSoundCreate(guid, &0x16bd4f8, NULL)`。失敗(`eax != 0`)なら **`f_41a770` で解放してから `DirectSoundCreate(NULL, …)` で再試行**(0x12afbed〜0x12afc01)。それも失敗なら "Couldn't create the DirectSound main object."。
   3. `SetCooperativeLevel(hwnd, 2)`(0x12afc2a: `r8d = 2` = `DSSCL_PRIORITY`。`rdx = [rbp+0x30]` = `f_12afb50` の第 1 引数の HWND)。失敗なら `f_12affd0`(全部解放)と "Couldn't set the DirectSound cooperative level."。**`DSSCL_EXCLUSIVE`、`WRITEPRIMARY`、`NORMAL` は使わない。**
   4. プライマリ: `DSBUFFERDESC { dwSize = 0x28, dwFlags = 1 (DSBCAPS_PRIMARYBUFFER), dwBufferBytes = 0, dwReserved = 0, lpwfxFormat = NULL }`(0x12afc5b〜0x12afc7d。記述子は `[rbp+0x78, rbp+0xa0)` の 40 B で、`[rbp+0x78]=0x28`、`[rbp+0x7c]=1`、`[rbp+0x80]=0`、`[rbp+0x84]=0`、`[rbp+0x88]`(qword)`=0`)。**末尾の `guid3DAlgorithm`(16 B。`[rbp+0x90, 0xa0)`)にはこの関数内で書き込みが無い**(`fdis.py` の全文で `[rbp+0x90..]` への参照 0)。`0x28` = 40 B = 4×4 + ポインタ 8 + GUID 16 は、`DIRECTSOUND_VERSION >= 0x0700` の `DSBUFFERDESC`(mingw の `dsound.h` 369〜379 行)の大きさと一致する。Learn の DSBUFFERDESC(全文取得)は「`DSBCAPS_CTRL3D` が立っていなければ `guid3DAlgorithm` は `GUID_NULL` でなければならない」と書く。**FLEngine はこのメンバーに値を入れない(スタックにあった値が渡る)**。実機の DirectSound がこれを検査するか、スタックの値がたまたま 0 かは**読めない(未確認)**。セカンダリも同じで、記述子 `[rbp+0x68, 0x90)` の末尾 16 B(`[rbp+0x80, 0x90)`)に書き込みが無い(0x12af6db〜0x12af71d)。**モデルは `guid3DAlgorithm` を読まない**のが安全(Learn に従うなら 0 でなければ `DSERR_INVALIDPARAM` を返す実装は、**実機と違う動きになる可能性**があり、採らない)。Learn の記述の範囲では、プライマリの `dwBufferBytes = 0`、`lpwfxFormat = NULL` と**一致**。 失敗なら "Couldn't create the primary DirectSound buffer."。これは Learn の DSBUFFERDESC の「プライマリは `dwBufferBytes` = 0、`lpwfxFormat` = NULL」と**一致**。
   5. 形式: `[0x15d3296]`(bool。旧作業の推定: UI の `ds_32bitbufbox`)が 0 なら **`f_674e10(wfx, rate, 16, 2)`** = `WAVE_FORMAT_PCM`(tag 1、チャネル 2、**16 ビット**、`nBlockAlign` 4)。非 0 なら **`f_674e50(wfx, 0xFFFE, rate, 32, 2)`** = **`WAVE_FORMAT_EXTENSIBLE`**(`cbSize` 0x16、`wValidBitsPerSample` 32、マスク 3、SubFormat の GUID は `0x13e9498` の先頭 16 B で **`IEEE_FLOAT` 系**と読める(`00000003-0000-0010-8000-00aa00389b71`。`xref` で先頭 8 B が `03 00 00 00 00 00 10 00` と確認))。`rate` は `[0x15e1f40]`(**既定 44100**。静的イメージ 0xAC44。設定 `\Audio output` の `Sample rate` で上書き)。wfx の置き場は `0x16bd4b0`。`IDirectSoundBuffer::SetFormat(wfx)` を**プライマリ**に呼ぶ(0x12afd3d。**戻り値は検査しない**)。
   6. `f_12aed10`: `CreateEventW(NULL, FALSE, FALSE, NULL)` を **2 個**(`0x16bd4e0`、`0x16bd4e8`。**自動リセット、初期は非シグナル**)、非ポーリングなら **3 個目**(`0x16bd4f0`)。失敗なら "Error while creating the notification events"。
   7. `f_12af5c0` はここでは呼ばれない。UI の登録(`^^External^DirectSound`)をして成功を返す。**セカンダリは `f_12afb50` の中では作らない**。
2. **`f_12af5c0`(セカンダリ作成。798 B。呼び元 `f_1252f80` 0x1252f9f、種別 -1 のとき)**
   1. `rate = [0x15e1f40]`、`frames_floor = max(round(rate / 99.0), 128)`(`0x12af8e0` の double = 99.0)。`blk_frames = max(frames_floor, round([0x16bd4a0] / 16.0) << 4)`。`[0x16bd4a0]` は設定 `Buffer length (DS)` のコピー(**既定 4096 サンプル**。`f_11a57c0` の 0x11acc8d)。この値を `[0x15e3140]`(= `[0x15e31f8]` にもコピー)に保存。
   2. `blockBytes = blk_frames × nBlockAlign`(`[0x16bd4a8]`。`nBlockAlign` は `wfx+0xc`。16 ビットステレオ 4、float 8)、`totalBytes = 2 × blockBytes`(`[0x16bd4a4]`)。**セカンダリは 2 ブロック**。
   3. `leadBytes = round(totalBytes × float([0x15e4bc8] = 0.01) × [obj+0x3c0])`(`[0x16bd4ac]`。`obj+0x3c0` はミキサーオブジェクト内の整数で、**実行時の値は未確認**。0 なら 0)。
   4. `DSBUFFERDESC { 0x28, flags, totalBytes, 0, &wfx(0x16bd4b0), …}`、`flags = 0x18000`(`GETCURRENTPOSITION2 | GLOBALFOCUS`)、**ポーリングが偽なら `| 0x100`(`CTRLPOSITIONNOTIFY`)**、**`[0x15d3295]` が偽なら `| 0x8`(`LOCSOFTWARE`)**。`CreateSoundBuffer` 失敗なら "Error|Couldn't create the main streaming DirectSound buffer."。
   5. ポーリングが偽(通知モード)なら `QueryInterface(IID_IDirectSoundNotify = {B0210783-89CD-11D0-AF08-00A0C925CD16}、0x12afa0e の 16 B)` をセカンダリに(結果は `0x16bd508`)。取れなければ "Error|Couldn't set the DirectSound notifications"。
   6. **通知位置 2 点**: `offset_i = (i × blockBytes + leadBytes) mod totalBytes`、`i = 0, 1`。配列は `DSBPOSITIONNOTIFY { dwOffset, hEventNotify }` × 2(イベントは `0x16bd4e0` と `0x16bd4e8` から)。配列は **昇順になるよう `i xor (leadBytes > (blockBytes + leadBytes) mod totalBytes)` の添字で書く**(0x12af805〜0x12af86b)。`SetNotificationPositions(2, array)`(0x12af886)。**`DSBPN_OFFSETSTOP` は使わない**。lead = 0 なら offset は {0, blockBytes} で、**1 ブロック再生するごとに 1 回ずつ**イベントが立つ。
3. **`f_12afaf0`(ミキサーの起動。94 B)**: `f_12aedf0`(`TMixerThread_DS` を作成: `CreateSuspended = TRUE` で `f_12b0050`、続いて `f_526500` = Start)→ `[0x15e3138] = 0` → `SetCurrentPosition(blockBytes)`(0x12afb27。**1 ブロック目の先から再生を始める**)→ `Play(0, 0, DSBPLAY_LOOPING)`(0x12afb46。`r9d = 1`)。**セカンダリの内容は `f_12af520` で全体を `Lock(0, 0, …, DSBLOCK_ENTIREBUFFER)` + `Unlock` する**(`f_12af520` は 0x12af571 が `Lock` の 6 引数目に `2` = `ENTIREBUFFER`)ので、再生前の無音の初期化は呼び出し側にある(`f_1253ae0` の 0x1253b03 から。解放・再開の対)。
4. **`f_12aeec0`(ミキサーの本体)**: §4。
5. **停止(`f_12aee30`)**: セカンダリ `Stop`(0x12aee4f)→ スレッドの `Terminate`(`f_5265d0`: `[this+0x1a] = 1`)→ **通知モードなら `SetEvent(0x16bd4e0)`**(0x12aee81。INFINITE の待ちから出す)→ `WaitFor`(`f_526610`)→ 解放。
6. **解放(`f_12affd0`)**: `f_12aee30` → `Release` を 4 つ(通知、セカンダリ、プライマリ、IDirectSound)→ イベントを `CloseHandle`(`f_12aed90`)。

### 3.4 通知かポーリングか(読み)

- **両方ある**。分岐は `[0x15d3294]`(bool)の 1 か所に集まる: `f_12aed10`(3 個目のイベントを作るか)、`f_12aee30`(`SetEvent` するか)、`f_12aeec0`(ループの形)、`f_12af5c0`(`CTRLPOSITIONNOTIFY` と `QueryInterface`/`SetNotificationPositions`)。UI の文字列は `forms.options.audiopage.audiooutpanel.dspanel.pollbox` = "Use polling"(旧作業の推定と一致)。
- **起動時の値**: `f_135b000`(0x135b00d)と `f_11a57c0`(0x11accc8)が `[0x15d3294] := byte [ctl+0xc40 → +0x49e]`(UI コントロールのチェック状態)を書く。つまり**保存された設定(`DSTITF` の値など)がコントロールを通って bool になる**。レジストリが空の初回起動でチェック状態が何になるかは、フォームのリソース(DFM)を読んでいないので**未確認**(旧作業も同じ)。静的イメージの `0x15d3294` は 1(ポーリング)、`0x15d3295` は 1(ハード = 非ソフトウェアの否定ではなく、**偽でなければ `LOCSOFTWARE` を付けない**)、`0x15d3296` は 0(16 ビット)。
- **含意**: **S1/S2 は通知とポーリングの両方を通す**(どちらも小さい)。**ポーリングは `GetCurrentPosition` の答えが時間とともに進まないと終わらない**(§5.2)。通知は `SetEvent` を呼ぶ源(S2 の位置の源)が要る。

## 4. ミキサースレッドの形

### 4.1 スレッドの生成と優先度(実測 + 読み)

- 生成: `f_12b0050(クラス, TRUE)` は Delphi の `TThread.Create(CreateSuspended = TRUE)` 相当(`f_5252d0(self, 0, 1)`。`f_5252d0` の呼び元は 12 の TThread 派生コンストラクタ)。`f_526500` = `Start` → `f_5259a0` → **`ResumeThread` が 1 を返さなければ例外**(0x5259cc。thread-model.md §5.3 と同じ)。**Windows の `CreateThread` は `f_412b50` の 1 箇所に集まる**(呼び元は `f_525400` の 2 点)。
- `Execute`(`f_12aeec0`)は最初に **`f_12b00f0`**: `f_63a660`/`f_63a6d0`(FPU・MXCSR の設定)→ **`SetThreadPriority(handle, 表[0x15e1ef0 の値])`**(`f_525ab0`)→ `"Pro Audio"` で avrt の `AvSetMmThreadCharacteristicsW`(スロット `[0x15e1fe8]` が非 NULL のときだけ。NULL なら呼ばない)。優先度の索引(`[0x16bd340]`。`[0x15e1ef0]` のセルがこの番地を指す)は、`f_1253260` の 0x1253290 が **`[0x15ce27c]`(静的イメージ 6)** から書き、**表 `0x13bd8c8` = `{-15, -2, -1, 0, 1, 2, 15}` の添字 6 = `THREAD_PRIORITY_TIME_CRITICAL`(15)** になる。**`f_1253840`(0x12538d0/0x12538e1)が、CPU 負荷の比(`[0x15e0cf8]` が `[0x15ce280]` = 0.95 を超えたとき)に索引を 3(= 0 = `NORMAL`)へ落とす**(過負荷時に優先度を下げる。`f_12b01f0` が次の周回で `SetThreadPriority` を呼び直す)。**ミキサーを既定で `TIME_CRITICAL` にし、過負荷で `NORMAL` に落とすスレッド**(thread-model.md の `TTimerThread` = LOWEST と対照的)。
- **協調スレッドでは優先度は無視される**(決定 0122)ので、優先度は影響しない。**ただし** `SetThreadPriority` が**新しいスレッドのハンドルで呼ばれる**ことは決定 0122 の E2 が既に扱っている(`fle_proc.c` の疑似ハンドル以外の停止は 0122 で置き換え済み。引用)。`avrt` は NULL 経路(決定 0088 の仮想モジュール)。

### 4.2 `TMixerThread_DS.Execute`(`f_12aeec0`、948 B)の本体(実測 + 読み。命令の引用は本節末尾の 11 行だけ)

```
Execute(this):
  f_12b00f0(this)                          ; 優先度設定
  while this.Terminated == 0:              ; [this+0x1a]
    --- 待ち ---
    if poll_flag [0x15d3294] != 0:         ; ポーリング
        repeat:
            Sleep(1)                       ; f_5264e0(1)
            EnterCriticalSection(CS)       ; CS = [0x15e3d40 の指す]、ミキサー全体の CS
            IDirectSoundBuffer::GetCurrentPosition(&pos, NULL)     ; play のみ
            pos = (pos + lead) mod total                           ; lead = [0x16bd4ac], total = [0x16bd4a4]
            half = (pos < block)           ; block = [0x16bd4a8]
            LeaveCriticalSection(CS)
        until  [0x15e4d60] != half  or  this.Terminated
        [0x15e4d60] = half                 ; 前回の半分の記録
    else:                                   ; 通知
        i = WaitForMultipleObjects(2, &0x16bd4e0, FALSE, INFINITE)
        [0x15e4d60] = i xor 1               ; 次に書く半分
    --- 後処理(時計) ---
    if [0x15e48a0] == 1:                    ; 追跡方式の設定値が 1 のとき(§5.4): timeGetTime と QPC を記録
        ...
    EnterCriticalSection(CS)
    [0x15e3138] = ([0x15e4d60] > 0) ? block : 0        ; 書く半分の先頭バイト
    song_time_ms += [0x15e3140]            ; 1 ブロックのフレーム数
    if [0x15e0b78] >= 0x7fa81bff: 全時計から 0x7fa81bff を引く(桁あふれ対策 = 約 24.8 日分の ms)
    f_1253800()                            ; QPC を記録
    f_d0ece0(...)                          ; ミキサー本体(1 ブロック分の音を生成)
    IDirectSoundBuffer::Lock(offset = [0x15e3138], bytes = block, &p1, &n1, &p2, &n2, 0)
    if Lock == DS_OK (eax == 0):
        if wBitsPerSample == 32:   ([0x16bd4be] == 0x20)
            fp = [0x15e2d40]; fp(dstL = [0x15e06a0 + 0x34], src = p1, frames = n1 >> 3)   ; float: コピー
            clipped = 0
        else:
            clipped = f_12c7420(dst, p1, n1 >> 2)   ; 16 ビット: クリップ検査 + 変換
        IDirectSoundBuffer::Unlock(p1, n1, p2, n2)
    f_12536b0(frames, clipped)             ; 過負荷・クリップの表示更新
    fp = [0x15e03a0]; fp([0x15e06a0 + 0x34], frames)    ; (UI 用のメータ類)
    IDirectSoundBuffer::GetCurrentPosition(NULL, &write)   ; write のみ
    if write の位置が [block 先頭, block 先頭 + block - 1] の範囲内:       ; 「書いた半分にまで write カーソルが入った」
        if (obj = [0x15e4790]->[+8]) vcall[+0x88](...)          ; 未確認(ミキサー統計)
            [0x15e16d0] += 1                                    ; アンダーラン計数
        f_bd68e0(1, 0)                                          ; コマンド(UI のアンダーラン表示と読む)
    f_1253840()                            ; 実時間の QPC 差分から CPU 使用率らしい値を更新
    LeaveCriticalSection(CS)
    f_12b01f0(this)                        ; 優先度が設定と変わっていれば再設定
```

**実測に基づく数値**(`buffer_math.py`。読んだ式の再計算): 

| 設定(`Buffer length (DS)`)| rate | 形式 | 1 ブロック(フレーム)| ブロックの長さ | `blockBytes` | `totalBytes`(セカンダリ全体)|
|---|---|---|---|---|---|---|
| 4096(**既定**)| 44100 | 16 ビット | 4096 | 92.9 ms | 16384 | 32768 |
| 4096 | 44100 | float32 | 4096 | 92.9 ms | 32768 | 65536 |
| 4096 | 48000 | 16 ビット | 4096 | 85.3 ms | 16384 | 32768 |
| 1024 | 44100 | 16 ビット | 1024 | 23.2 ms | 4096 | 8192 |
| 512 | 48000 | 16 ビット | 512 | 10.7 ms | 2048 | 4096 |

**ブロックの下限は `max(round(rate/99), 128)` フレーム**(44100 で 445、48000 で 485)。設定が小さくてもこれを下回らない。

**要点(読み)**:

- **(a) 待ち方**: ポーリングは **`Sleep(1)` の短周期 + `GetCurrentPosition`(play のみ)**。通知は **`WaitForMultipleObjects(2 個、FALSE、INFINITE)`**(**タイムアウトなし**)。**どちらも「再生カーソルが時間とともに進む」ことが前提**。
- **(b) 供給の周期とサイズ**: **1 ブロック(既定 4096 フレーム)ごとに 1 ブロック**を書く。ブロック = セカンダリの半分。再生カーソルが半分を渡るたび(通知は offset 0 と blockBytes でイベントが立つたび)に、**今カーソルが入った半分の反対の半分**を書く(`i xor 1`)。**ミキサー本体の計算は 1 回の起床で 1 ブロック分**(`f_d0ece0` は 1 回の呼び出しで `[0x15e0b78] < ebx` の間繰り返す形に見える。**1 起床あたりの計算量・時間は未測定**)。
- **(c) 他スレッドと共有する状態**: ミキサー全体の CS(`[0x15e3d40]` が指す。FLEngine 全体で 267 関数が参照する巨大な CS = 「ミキサーの CS」)、`[0x16bd4f8..0x16bd508]` の COM ポインタ、`[0x15e3138]`(書く半分の先頭)、`[0x15e4d60]`(前回の半分)、曲時刻 `[0x15e0b78]`/`[0x15e0338]` ほか(時計の群。0x7fa81bff で一斉に減算)、アンダーラン計数 `[0x15e16d0]`。**UI スレッドも `GetCurrentPosition`(`f_1311a00`: 曲位置の補間用。play のみ)と `Stop`(`f_1253b50`)をロックなしに呼ぶ**。
- **(d) 終了条件**: `Terminated`(`[this+0x1a]`)。ループの先頭と、ポーリングの内側の `until` で検査される(0x12aef4f)。通知モードは **`INFINITE` の待ちの中では検査されない**ので、停止側(`f_12aee30`)が `Terminate` の後で **`SetEvent(0x16bd4e0)`** して起こす(0x12aee81)。**`SetEvent` しても、このスレッドが `Terminated` を見るまでに 1 ブロック分のミキサー計算が走る**(`WaitForMultipleObjects` から戻った後の流れ。読み)。

**機械語抜粋(11 行。待ちと位置の核。`fdis.py` の出力から、コメントは本調査)**:

```
12aeee3  mov  ecx, 1            ; Sleep(1)  (ポーリング。呼び先 f_5264e0 = Sleep(ecx) の包み)
12aeee8  call f_5264e0
12aef11  call [rax+0x20]        ; GetCurrentPosition(&pos, NULL)   受け手 = [0x16bd498] (セカンダリ)
12aef73  call WaitForMultipleObjects   ; (2, 0x16bd4e0, FALSE, -1)
12af124  call [rax+0x58]        ; Lock(offset, bytes, &p1, &n1, &p2, &n2, 0)
12af19b  call [rax+0x98]        ; Unlock(p1, n1, p2, n2)
12af1e5  call [rax+0x20]        ; GetCurrentPosition(NULL, &write)
12af1f6  cmp  rax, rcx          ; write(rax) vs [0x15e3138](書いた半分の先頭): < なら範囲外
12af210  cmp  rax, rcx          ; write vs 先頭 + block - 1: > なら範囲外
12af229  call [rax+0x88]        ; rax = [0x15e4790]->[+8] の vtable (DS ではない)。範囲内のときだけ通る
12af244  call f_bd68e0          ; (1, 0): UI のコマンド
```

### 4.3 `TMixerThread_ASIO`(`f_c55740`、905 B)と ASIO のコールバック `f_c56ae0`(2353 B)

- `f_c55740`(Execute): 先頭で `f_12b00f0`(優先度)、ループの先頭で **`WaitForSingleObject(0x16b87d8, INFINITE)`**(0xc55760。イベント = 「ASIO のコールバックが 1 ブロックを渡した」)→ CS → 時計を進める(`[0x15e0338] += [0x15e3140]`)→ `f_1253730`(時計の桁あふれ処理。0x7fa81bff)→ `f_d0ece0`(ミキサー本体)→ `SetEvent(0x16b87e0)`(0xc55a69。「ミキサーが 1 ブロックを作った」)→ ... → CS を放す。
- ASIO のコールバック表(`f_c5e360`、0xc5e360〜0xc5e398: `0x16b87e9` から 8 B × 4): `bufferSwitch = f_c57420`、`sampleRateDidChange = f_c576b0`、`asioMessage = f_c576c0`、**`bufferSwitchTimeInfo = f_c56ae0`**。コールバックは **ドライバーのスレッドから呼ばれる**。`f_c56ae0` は `WaitForSingleObject(0x16b87e0, timeout = [0x16b8814])`(0xc56bb7。ミキサーが 1 ブロックを作るのを待つ)、`SetEvent(0x16b87d8)`(0xc56ce0。ミキサーを起こす)、あるいは `SignalObjectAndWait(0x16b87d8, 0x16b87e0, timeout)`(0xc57084)の 2 通り(**どちらで走るかは `[0x15e25e0]` の条件**)。**ミキサースレッドとドライバーのスレッドの 2 本が相互に待つ構造**で、**ASIO は協調スレッドだけでは足りない**(ドライバー側のスレッドは実機では OS のスレッド。ブラウザでは AudioWorklet が `bufferSwitch` の呼び出し元になる必要がある)。
- **IASIO の呼び出し(実測)**: ラッパー関数 **19 個**(0xc53da0〜0xc53ff0。各 22〜35 B)。うち **vtable 呼び出しを持つのは 19 個**(`f_c53eb0` だけ受け手が `rbx` で、他は `rax`。いずれも `call [reg+disp]` が 1 点)。確認できた disp は次の 19 種(`fdis.py --vcalls`):

| ラッパー | disp | ラッパー | disp | ラッパー | disp |
|---|---|---|---|---|---|
| `f_c53f50` | 0x18 | `f_c53ed0` | 0x28 | `f_c53fd0` | 0x38 |
| `f_c53ff0` | 0x40 | `f_c53e70` | 0x48 | `f_c53ef0` | 0x50 |
| `f_c53e30` | 0x58 | `f_c53da0` | 0x60 | `f_c53f30` | 0x68 |
| `f_c53fb0` | 0x70 | `f_c53e90` | 0x78 | `f_c53f90` | 0x80 |
| `f_c53f10` | 0x88 | `f_c53e50` | 0x90(呼び元 2) | `f_c53de0` | 0x98 |
| `f_c53e10` | 0xa0 | `f_c53dc0` | 0xa8 | `f_c53f70` | 0xb8(呼び元 5) |
| `f_c53eb0` | 0x20(受け手 `rbx`) | | | | |

  disp が 8 B 刻みの 24 スロットに収まる(0x18〜0xb8)ことは、`IASIO`(IUnknown 3 + 21 メソッド)の形と矛盾しない。**どのスロットがどの ASIO メソッドか**(例: 0x18 = `init`、0x88 = `createBuffers`)は、旧 INVENTORY(引用)の対応に頼っており、**本調査では ASIO SDK の宣言を取得していないので未確認**。`f_c57a00`(開く関数)が `0xc53de0`(disp 0x98)、`0xc53ef0`(0x50)、`0xc53eb0`(0x20)、`0xc53ed0`(0x28)を呼ぶこと、エラー文字列("CreateBuffers failed" は `f_c53de0` の直後 0xc582cc)から、**0x98 = `createBuffers`** と読める(読み。旧 INVENTORY は 0x88 と別の対応を書いている可能性があるので **対応表は PM に渡さない**)。

### 4.4 ほかのスレッド(G8 で加わる)

- `TWAVStreamingThread.Execute`(`f_678660`、173 B。実測): ループ `EnterCriticalSection([this+0x50])` → リストを末尾から走査して各ストリームの `f_691bd0`(ディスクの先読みと読む)→ `LeaveCriticalSection` → **`[this+0x78]`(TEvent)の `WaitFor(INFINITE)` の仮想メソッド(`vtable+0x10`、引数 -1)**。**CPU を連続して使い続ける形ではない**(イベント待ち)。**誰がそのイベントを立てるか(`SetEvent`)は未確認**。
- `TMIDIThread.Execute`(`f_bb0920`): `Sleep`、`WaitForSingleObject`、`midiOutShortMsg`(`execute_waits.tsv` の引用)。**MIDI を開かなければ作られない**と読む。
- `TRenderThread`/`TDWPRenderThread`(書き出し): `Sleep`/`SetEvent`/`PostMessageW`。G10 の話で、本調査の範囲外。

## 4.5 ASIO の経路(読み。実測を含む)

### ドライバーの列挙(`f_c526f0`、781 B。実測 + 読み)

- `[0x15e01b0]`(ホストの Flags ビット 1。FL64.exe では 0)が 0 のときだけ `f_c55ad0` が `f_c526f0` を呼ぶ。
- `f_c526f0`: `TRegistry`(`f_5af120`: **既定のルートは `HKEY_CURRENT_USER`(`0xffffffff80000001`)**。作成直後に `0x2c` へ `0xf003f` = `KEY_ALL_ACCESS` を入れる)→ `RootKey := HKEY_LOCAL_MACHINE`(`0xffffffff80000002`。0xc52779)→ `OpenKey(L"software\\asio", …)`(`f_5af7c0`、0xc5278c)→ 成功なら `GetKeyNames`(`f_5afe30`)で下位キー名の一覧 → 各名前について `OpenKey(L"software\\asio\\<name>")`(0xc52818)→ `ReadString(L"clsid")`(`f_5b03a0`、0xc52850)→ `f_c522f0`(**ドライバーの DLL が実際に存在するかの検査**: `HKCR\clsid\<GUID>\InprocServer32` を読み、`LoadLibraryW` ではなく `f_44db40`(`SetErrorMode` + `LoadLibraryW` + `SetErrorMode`)で一度ロードして `FreeLibrary` で捨てる。0xc525c9〜0xc525dd。`GetSystemDirectoryW`/`GetWindowsDirectoryW` でパスを補う)→ 成功なら `CLSIDFromString`(0xc528a0)で GUID を作り、`{ name, GUID }` の要素(`0x290` 前後の長さ。名前は 0x200 B)を配列に足す。
- **`LoadLibraryW` で ASIO ドライバーの DLL を一度ロードしてから捨てる**(0xc525c9 の `f_44db40` と 0xc525dd の `FreeLibrary`)ので、仮想 `LoadLibraryW` が**ドライバーの DLL 名に答える必要がある**。**ASIO キーが無ければ `GetKeyNames` が 0 件なので、ここには来ない**。
- **仮想レジストリ**: 起動時の HKLM は空の `Software` だけ(決定 0085。引用: `lift/runtime/fle_reg.c` の冒頭コメント)。`software\\asio` は存在しない → `OpenKey` 失敗 → 配列 0 件 → `f_c55ad0` は何も足さない(読み)。**したがって既定の仮想環境では ASIO は一覧に出ない**(読み。**実行では確かめていない**)。
- **インストーラが `HKLM\\SOFTWARE\\ASIO\\FL Studio ASIO` を作る**かは、既存調査の `nsis-registry-values.md` が `FL Studio ASIO`(変数 `$v35`/`$v36`)と `Paths\\FL Studio ASIO`(入口 61)を記録している(引用)。**CLSID の出所は ILWASAPI2ASIO 側の `DllGetClassObject` の表**で、旧 INVENTORY が `{188135E1-7171-3434-854F-01A3C71F3DF9}` と読んでいる(**引用。DLL が無いので再測定していない**)。FLEngine には ASIO の CLSID の定数が無い(`xref_str` で `{…}` 形の ASIO 用文字列 0。`{D57C7288-…}` と `{84BCCD23-…}` は別の COM)。

### ドライバーの作成(`f_c53c60`、219 B。実測)

- `f_c53c60(this, alloc, clsid)`: **`CoInitialize(NULL)`**(0xc53ca1)→ `this+8` の既存インターフェースを `f_41a770` で解放 → **`CoCreateInstance(clsid, NULL, CLSCTX_INPROC_SERVER = 1, clsid(= IID も同じ GUID), &this+8)`**(0xc53cce。`r8d = 1`、`r9 = clsid` を IID として渡す: ASIO 慣例どおり**クラス ID とインターフェース ID が同じ**)。失敗(`eax & 0x80000000`)なら解放する。**`CoUninitialize` は `f_c53d50`(デストラクタ)**。
- **`CoInitialize` は `CoInitializeEx` ではない**ので、**メインスレッドの STA**(`CoInitialize(NULL)` = `COINIT_APARTMENTTHREADED`。Learn の CoInitialize ページは取得していない。**未確認**)。

### 開く(`f_c589b0`、19639 B。実測 + 読み)

- 巨大な関数(Delphi のコンパイラが try/finally ごと展開した)。`f_c53c60` → `init`(`f_c53f50` = disp 0x18。`hwnd` を渡す。失敗なら "Couldn't init the ASIO driver.")→ `getChannels`(`f_c54320`。2 チャネル未満なら "Not enough ASIO output channels available. At least 2 channels are needed.")→ `f_c57a00`(`getBufferSize`/`getLatencies`/`createBuffers` ほか。エラー文字列 "ASIO: CreateBuffers failed with error %d"、"Couldn't create the streaming ASIO buffer.")→ サンプルレートが取れなければ警告 "The needed sample rate (%d Hz) couldn't be set or is not supported by the driver"→ `start`(`f_c53ef0`?? ではなく `f_c58970`/`f_c56840`)。**ASIO の `start` の呼び出し順の細部は読み切っていない(未確認)**。
- ホストが ASIO ドライバーに渡すもの: コールバック表 4 本(§4.3)。`bufferSwitchTimeInfo` が本体で、ドライバーのスレッドから呼ばれる。**ASIO のバッファ形式は `getChannelInfo` の型で分岐**(`f_c545d0` に "Float32MSB"/"Float64MSB"/"Float32LSB"/"Float64LSB" の文字列。他の型(Int16/Int24/Int32 の LSB/MSB)は文字列に出ない)。引用(旧 INVENTORY): 基準ドライバー `ILWASAPI2ASIO` は `ASIOSTFloat32LSB`、2 入力 2 出力、`getBufferSize` は 最小 256 / 最大 4096 / 推奨 = 設定(既定 512)、`canSampleRate` は 22050/44100/48000/96000/192000、クロックソース 1 件 "Internal"。

### ASIO が G7/G8 に要るか(読み)

1. **要らない見込み**: (a) 既定の仮想レジストリに `software\\asio` が無く、ASIO は一覧に出ない(上)。(b) ブラウザには ASIO ドライバーが無い。(c) DS だけで 0〜2 の入出力と音を出す手順が §3 に閉じる。
2. **ただし要る可能性が残る点(未確認)**:
   - **`Device name` の既定が `"FL Studio ASIO"`**(0x1252f54。保存設定が無いときの既定値)。**ASIO が一覧に無ければ一致せず添字 0** になる。添字 0 が何か: 一覧の先頭。`f_c55ad0` が 0 件なら先頭は DS の最初のデバイス(`DirectSoundEnumerateA` の最初のコールバック = "Primary Sound Driver"、GUID NULL)。**DirectSoundEnumerateA が 1 件も呼ばなければ一覧は空 + "(none)" 要素のみ**。
   - **`f_1369da0`**: デバイス名が "FL Studio ASIO" のとき "Your audio device is set to FL Studio ASIO. This can cause inaccurate MIDI sync…" の注意を出す(実測の文字列)。FL Studio ASIO が**既定として想定されている製品の挙動**の証拠(読み)。
   - **ASIO が選ばれたときのコードの量**(`f_c589b0` 19.6 KB、`f_c57a00` 3.1 KB、`f_c56ae0` 2.3 KB)は DS(`f_12afb50` 0.6 KB + `f_12af5c0` 0.8 KB + `f_12aeec0` 0.9 KB)の約 10 倍。ASIO を要求すると**ブラウザ側が 2 本目のスレッド(ドライバーのスレッド)と COM のサーバー(`IASIO` の 24 スロットの vtable、`IClassFactory`)を要する**。
3. **ILWASAPI2ASIO_x64.dll**: `lift/out-inventory/extracted/` には **無い**(`ILTools_x64.dll` と `fonts/` だけ)。インストーラ exe(`flstudio_win64_26.1.7.5653.exe`、1.1 GB、PE)がユーザーのダウンロード先にあるが、**展開は試していない**(`7z l` は型が PE と出ただけで、内部の一覧は出なかった)。**入手できていない**。既存調査(`host-engine-boundary.md` 56 行、`m2-inventory.md` 217 行)が 531,456 B の COM サーバー DLL と記録している(引用)。

## 5. 協調スレッド(決定 0122)で走らせたときの落とし穴(読み)

前提(引用): 協調スケジューラは**待ち・譲りの API でだけ**切り替える。仮想時計は `Sleep(ms>0)` と全員待ちの最小期限への前進、**時計を読むたびに 10 tick(1 µs)進む**(`fle_sys.h`)。`WaitForSingleObject` は時計を進めない。期限なしで全員が待つと `deadlock` の名前付き停止。`DwmFlush` は次の 60 Hz 境界まで待つ。

### 5.1 通知モード(`WaitForMultipleObjects(2, …, INFINITE)`)

1. **全員が INFINITE で待つと止まる**。メインスレッドがメッセージ待ち(`GetMessageW`)で、QFC のタイマースレッドは `Sleep`/`DwmFlush`(期限あり)、レイヤード窓スレッドは `GetMessageW`、ミキサーが INFINITE。**期限を持つスレッドが 1 本でも居れば時計が進む**ので deadlock にはならない(`TTimerThread` が居る間は進む)。**ただし**、そのタイマースレッドの `Sleep`/`DwmFlush` が進んでも、**ミキサーを起こす `SetEvent` を誰も呼ばない限り、ミキサーは永久に待つ**。つまり**通知イベントを立てる「デバイス」が時計の上に存在しなければならない**。
2. **デバイスの時計を持つ「第 4 のスレッド」は不要**: スケジューラが `next_deadline()`(`fle_thread.c` 550 行)で決める期限の集合に、**音声デバイスの次の通知時刻**を足す(`fle_win_next_timer_tick` と同じ形のフック)のが最小。そうしないと**全員が待ったとき仮想時計が通知時刻まで進まず、他に期限を持つスレッドが居なければ deadlock の停止**になる(**読み**: `fle_thread.c` 593 行の `idle_stop` は「最初のスレッドが空キューで入力待ち、他に走れるものが無い」で名前付き停止する。**音声のミキサーが INFINITE で待つだけだと、`next_deadline` に載る期限がタイマースレッド由来だけになり、idle 停止が先に出うる**。`FLE_FRUITY_NO_IDLE_STOP=1` で切れる(`harness/fruity.c` 782 行)。実行して確かめてはいない)。
3. **自動リセット**: `0x16bd4e0`/`0x16bd4e8` は自動リセット。**2 つの通知位置の 2 つのイベントを `WaitForMultipleObjects(…, FALSE)` で待つ**ので、**同時に両方が立っていたら小さい添字が返り、もう一方は立ったまま次の呼び出しで返る**(Learn: 「smallest index」)。通知を**取りこぼす/合体する**モデル(旧実装の「合体」)を採ると、`i xor 1` が `0 → 1 → 0` と交互に進む前提が崩れる**かもしれない**(読み。1 ブロック時間に 2 回起きれば、ミキサーは同じ半分を 2 度書く)。
4. **起こしても `Terminated` を見ない**: 停止は `Terminate` + `SetEvent`(§3.3 の「停止(`f_12aee30`)」)。デバイスが `SetEvent` を呼ぶ側と競合しても、イベントは 1 つなので問題ない。

### 5.2 ポーリング(`Sleep(1)` + `GetCurrentPosition`)

1. **`GetCurrentPosition` が仮想時計と整合しなければ無限にポーリングする**: ループの終了条件は `half` の変化(play の位置が半分の境界を越える)。**`Sleep(1)` は仮想時計を 1 ms 進める**(決定 0122: `Sleep(ms > 0)` は待ち、他が走る。他が無ければ時計が進む)。**play 位置が「消費フレーム数」で決まり、その源が時計に結ばれていなければ位置が止まり、`half` が変わらず永久に回る**。native の「100 万回で停止」の保険(`SwitchToThread` の 100 万回停止。thread-model.md)は `Sleep(1)` には無いので、**`Sleep` が仮想時計を進める限り無限ループでも `fle: …` の停止にはならず、時計が際限なく進む**(**停止しないことが落とし穴**。実時間の上限は `harness` の entry limit/wall time のみ)。
2. **1 ブロックあたり約 93 回の `Sleep(1)`**(4096 フレーム、44.1 kHz = 92.9 ms)。**呼び出し 1 回ごとに協調スレッドの切り替えが入る**(他に ready が無ければ `schedule()` の中で時計を進めて戻るだけ。決定 0124 の測定: idle `Sleep(1)` のループは native 2.72 M/s、node 3.52 M/s、Chrome 1.56 M/s)。**1 秒の音声あたり約 11 ブロック × 93 = 約 1,000 回**の `Sleep(1)` + `GetCurrentPosition` で、コストは問題にならない(読み。**FLEngine 側のこのループの 1 回あたりの命令数は未測定**)。
3. **play 位置のオフセット規則**: `(pos + lead) mod total < block` の `half` は、**`lead = 0` なら「play カーソルがセカンダリの前半にいるか」**。`lead > 0`(`[obj+0x3c0]` > 0)なら遅延補正。**`lead` の値が実行時に決まる**ので、モデルは `GetCurrentPosition` に **play を `0 … total-1` の任意のバイトで返せる**必要がある(4 バイト境界など nBlockAlign の倍数のみ。`blockBytes` が nBlockAlign の倍数であることから、全バイト値は要らない)。
4. **`GetCurrentPosition(NULL, &write)` の検査**(ミキサーの末尾): **write カーソルが今書いた半分の `[先頭, 先頭 + block - 1]` に入っているとアンダーラン**(0x12af1fb〜0x12af213 の比較)。**write = play + 先行量**なので、モデルが返す write が **play より `block` 以上先に進まない**こと、かつ**ミキサーが今書き終えた半分に play が入っていない**ことが保たれていないと、**毎ブロックでアンダーランが数えられ、UI のコマンド(`f_bd68e0(1, 0)`)が呼ばれる**。実機の write − play は「約 15 ms 分」(Learn の GetCurrentPosition: 「typically by about 15 milliseconds' worth」)。**既定のブロック(92.9 ms)に対して write − play が 15 ms なら、play が書いた半分に入る前にミキサーは書き終わる**。モデルは **write = play + min(約 15 ms 分, ブロック − 1 フレーム)**のような答えを返せば検査を通る(読み。**実機の値は未測定**。旧実装は P(既定 256 フレーム)を使っている=引用)。
5. **初回**: `Play` の直後の最初の `GetCurrentPosition` が **`SetCurrentPosition(blockBytes)` で置いた位置**を返さなければならない(Learn: 「When the buffer is stopped, the cursor remains where it is」。`Play` の前に置いた位置から進む)。0 を返すと最初の `half` が前半(`pos < block`)になり、**最初のブロックの書き込みが 1 半分ずれる**(読み)。
6. **UI スレッドの `GetCurrentPosition`**(`f_1311a00`)は曲位置の補間用で、**ミキサーの CS を取らずに呼ぶ**(0x1311b30。`f_1311a00` は CS を取らない)。協調スレッドでは**この 1 点が切り替えなしで戻る**ので問題ない。

### 5.3 時計の飛び(`0x7fa81bff`)と 32 ビット

- `f_12aeec0` の 0x12af010〜: `[0x15e0b78]`(実体 `0x196ec38`)が `0x7fa81bff`(= 2,141,723,647)以上になると、複数の時刻のグローバル(`[0x15e0338]` = `0x196ec3c`、`[0x15e4b40]` が指す先、`[0x15e2978]`、`[0x15e1a00]`、`[0x15e4688]`、double の `[0x15e3450]` と `[0x15e07b0]`(3 回)から**一斉に 0x7fa81bff を引く**(32 ビットの符号つき整数の桁あふれを避ける折り返しと読める)。
- **この値の単位は読み切れていない**: ミキサーは毎ブロック `[0x15e0338] += [0x15e3140]`(1 ブロックのフレーム数。`f_c55740` の 0xc55771〜0xc55781 と同じ形)と足すので、**少なくとも `[0x15e0338]` は「フレーム数」**。`f_1311a00` の 0x1311aff は `[0x196ec3c] - [0x15d8920](初期値 4096)` を曲位置の基準にしている。`0x7fa81bff` フレームは 44.1 kHz で約 **13.5 時間**、48 kHz で約 12.4 時間(`0x7fa81bff` を ms と見れば約 24.8 日。**どちらの単位か、`[0x15e0b78]` が何かは未確認**)。**テストが何時間も走らない限り通らない**(読み)。仮想時計の起点は 3,600,000 ms(`FLE_CLOCK_START_MS`。引用)で、この折り返しとは無関係。

### 5.4 追跡方式 `[0x15e48a0]` と `timeGetTime`/QPC の記録

- `f_12539b0`(112 B。実測): `[0x15e48a0] := ` UI の設定 `[[0x15e13b8]->[+0xba0] → +0x3c0]` から決める。**DS(`[0x15ce278] == -1`)のとき: 値が 0 なら 3、非 0 ならその値**。ASIO(0)と Plugin(1)のときはその値。種別が範囲外なら -1。旧作業は `pbtrackingmode` と呼ぶ(UI 名。引用。**この変数名との対応は私は確かめていない**)。**静的イメージの初期値は `0xFFFFFFFF`(-1)**。
- `f_12aeec0` は **`[0x15e48a0] == 1` のときだけ** `timeGetTime`(0x12aef9c)と `QueryPerformanceCounter`(0x12aefbc)を読んで記録する。DS で 1 になるのは設定値が 1 のとき。**既定の設定値は未確認**(0 → 3 と読むなら通らない)。**時計を読むたびに 10 tick(1 µs)進む仮想時計**なので、この読みはミキサーの 1 回の起床あたり 20 tick(2 µs)で、時計への影響は無視できる(読み)。
- **`[0x15e48a0]` が 3 のときの意味**(再生カーソルの補間の方式と読めるが、`f_1311a00` の分岐 `cmp [0x15d89d0], 2 / 3` を全部は読んでいない)は**未確認**。**曲位置の UI 補間(`f_1311a00`)が `GetCurrentPosition` の値と QPC の差の両方を使う**ので、**play 位置の答えが QPC(= 仮想時計)と大きく食い違うと、再生ヘッドの表示が飛ぶ**(読み。G7 の合否(無音・アンダーラン 0)には効かない。G8 の画面の再生ヘッドには効く)。

## 6. ブラウザ側との接続の設計案(決めない。選択肢と推奨)

### 6.1 DirectSound の最小モデル(Learn の規則どおり、状態だけで)

**モデル化するもの(§3.2 の表の 13 メソッド + `Release`)**。`lift/runtime/fle_ole.c` の `SHGetMalloc`/`IMalloc`(決定 0098)が先例で、**形は同じ**: 「host メモリに `{ vtable ポインタ, スロット[N] }` の配列を `calloc` し、各スロットに応答表の疑似 VA を入れ、ゲストの `call [vtable+n]` を `fle_imp_<名前>` に届ける」。具体的には、

| 項目 | `IMalloc` の先例(実測。`fle_ole.c` 391〜484 行) | DirectSound で必要になるもの |
|---|---|---|
| オブジェクト | `calloc((1 + 9) × 8)`、`o[0] = &o[1]`(vtable 自身が後ろに続く) | **4 種のオブジェクト**: `IDirectSound`(vtable 11 スロット)、プライマリ/セカンダリ(`IDirectSoundBuffer` 21 スロット。同じクラスで `DSBCAPS_PRIMARYBUFFER` の旗だけ違う)、`IDirectSoundNotify`(4 スロット) |
| スロット → 関数 | `imalloc_slot_va(api, "IMalloc::Release")` が応答表(`imports_table.rs` の行)から疑似 VA を引く | 行を足す: `IDirectSound::SetCooperativeLevel`/`CreateSoundBuffer`/`Release`/`unimplemented`、`IDirectSoundBuffer::GetCurrentPosition`/`Lock`/`Unlock`/`Play`/`Stop`/`SetCurrentPosition`/`SetFormat`/`GetFormat`/`QueryInterface`/`Release`/`unimplemented`、`IDirectSoundNotify::SetNotificationPositions`/`Release`(**約 16〜20 行**。`DirectSoundCreate`/`DirectSoundEnumerateA` の 2 行は dsound.dll の行として別)|
| 使わないスロット | 全部 `IMalloc::unimplemented` に向け、呼ばれたら `fle_stop` | 同じ形。**使わないと確認したメソッド(§3.2)が呼ばれたら名前付き停止**にする(`GetCaps`、`GetSpeakerConfig`、`GetStatus`、`Restore`、`SetVolume` など)。**停止の存在自体が「この領域では呼ばない」という調査の読みの反証の検出器になる** |
| `this` の検査 | `imalloc_this`: `R[1] != imalloc_obj` なら停止 | 4 つのオブジェクトの 1 つであることを検査 |
| 参照カウント | `imalloc_refs` | `QueryInterface(IID_IDirectSoundNotify)` が AddRef する前提で数える(`f_41a770` は `Release` を呼んで 0 を書く) |
| 出力の書き込み | `fle_st(pp, 8, imalloc_obj)` | `Lock` の `*ppvAudioPtr1`/`*pdwAudioBytes1` は **ゲストのアドレス空間のバッファ**を指す必要がある(**下の「バッファの置き場所」**) |

**状態**(モデルが持つ最小): 作成の有無、協調レベル(`DSSCL_PRIORITY` = 2 以外は停止してよい)、プライマリの形式と全長 0、セカンダリの `WAVEFORMATEX`/`WAVEFORMATEXTENSIBLE`(PCM16 と IEEE float32 の 2 形式。他は停止)、`dwBufferBytes`、フラグ、バッファのバイト列、**play カーソル**、**write カーソル**、再生中か、`LOOPING` か、ロック中の範囲、通知位置の配列(最大 2 点)と各イベントのハンドル。

**バッファの置き場所(判断点)**: `Lock` が返す `ppvAudioPtr1` はゲストが直接書き込むポインタ。**native の harness ではゲストのメモリ = ホストのメモリ**(`R[]` の `fle_st` は実アドレスに書く)なので、**セカンダリのバイト列をランタイムが `calloc` して返すのが最小**。**wasm では線形メモリ上の領域**になり、**AudioWorklet が読むには SAB(共有メモリ)上に置く必要がある**(§6.3)。**ゲストの `Lock` の戻りの領域 = 共有リングそのもの**にすれば、`Unlock` でコピーが要らない(旧実装は shim 内の循環バイト列から planar f32 のリングへ変換して書く形)。**どちらも決めない**。

**DS を開いたときの仮想環境に必要な行**(既存の行があるかの確認。実測 `imports_table.rs`): `CreateEventW`/`SetEvent`/`WaitForMultipleObjects`/`Sleep`/`CreateThread`/`ResumeThread`/`SetThreadPriority`/`ExitThread`/`timeGetTime`/`timeBeginPeriod`/`timeEndPeriod`/`timeGetDevCaps`/`QueryPerformanceCounter`/`CoInitialize`/`CoUninitialize` は**既にある**(決定 0122、0030 ほか)。**無い行**: `DirectSoundCreate`、`DirectSoundEnumerateA`(dsound.dll の行)、`SignalObjectAndWait`(ASIO のときだけ)、`CoCreateInstance`(ASIO のときだけ)、`midiInOpen`(G7 には効かない。**`midiOutOpen` はある**)。`avrt.dll` は決定 0088 の仮想モジュール。**G7 の DS だけなら新規は `DirectSoundCreate`、`DirectSoundEnumerateA` と上の vtable の約 16〜20 行**。

**`DirectSoundEnumerateA` の応答**(決める点): コールバックを呼ぶのは**ゲストの関数**(`f_12af280`。`BOOL CALLBACK`(Win64 の `rcx = lpGuid`、`rdx = lpcstrDescription`、`r8 = lpcstrModule`、`r9 = lpContext`)なので `fle_call_guest`(`fle_callback.h`)で呼ぶ。**戻り値は TRUE で続行**。Learn: 「primary device is enumerated twice, first with NULL」。**最小は 1 回だけ `(NULL, "Primary Sound Driver", "", ctx)` を呼ぶ**(旧実装が何を返したかは未確認)。**デバイスが 0 件だとゲストの一覧が "(none)" だけになる(§3.1)ので、最小でも 1 件は必要**。

### 6.2 再生位置の駆動(A: 仮想時計、B: 実時間、C: 混成)

位置の源を 1 本の**消費フレーム数 `consumed`**(64 ビット)に抽象し、play カーソル = `(base + consumed × nBlockAlign) mod total`、write カーソル = `play + lead`(lead は §5.2 の 4)とする。**源を差し替える**のが共通の骨格。

| | **A 仮想時計** | **B AudioContext の実時間** | **C 混成** |
|---|---|---|---|
| `consumed` の源 | 仮想時計(100 ns tick。決定 0122 の `fle_sys_clock_ticks()`)から `Play` 時刻を引いた差 × rate | AudioWorklet が数える消費フレーム(SAB の `Atomics.load`。`browser-event-loop.md` §8 の構成。本文書の §8 ではない) | 通常 B、**テストと native は A**。ゲストから見える API は同じ |
| **決定性(native/wasm の一致、テスト)** | **完全に決定的**(時計は純関数。native と wasm で同じトレース。決定 0124 の 36 シナリオと同じ性質) | **非決定**(実時間のジッタ。A5 の bit-exact・テストの再現性と両立しない) | A の部分は決定的。B の部分は非決定 |
| アンダーラン・遅延 | **アンダーランは構造上起きない**(時計が音の生成を待つ。スレッドが走るまで時計が進まない)。**ただし「音が出る」ことと無関係**(AudioWorklet は無関係に無音を再生し続ける) | ミキサーが 1 ブロック(93 ms)を作る時間が実時間を超えるとアンダーラン(`browser-event-loop.md` §8 の実測: リング 1024 フレーム以上 + 生産側の 1 回 6 ms まで 0)。**G7 の字義(AudioWorklet が一定のブロックを連続して受け取る)を満たすのは B 側** | A の決定性と B の実音を両立するが、**切替点の仕様が要る** |
| JSPI での切り替え(決定 0124)との相性 | **最良**: ポーリング `Sleep(1)` は時計を進めるだけ。切り替えのコストは決定 0124 の idle 型と同じ(Chrome 1.56 M/s) | **要注意**: ミキサーが `WaitForMultipleObjects(INFINITE)` で待つ間、**実時間を待つには JS のイベントループへ戻る**必要があり、**JSPI の `Suspending` で中断して AudioWorklet の通知(`Atomics.waitAsync` か `MessagePort`)で再開する**形になる。**全員待ちのときの時計の進め方(待つ vs 進める)が決定 0122 と違う**(実時間で待つ = 仮想時計の起点と別の時間軸が混ざる) | A の骨格に B の「実時間のペース合わせ」を足す形になる |
| 実デバイス | 出力なし(無音でよい G7 の「受け皿」は AudioWorklet だけで、エンジンからの供給は無い) | 実際に鳴る | 同 B |
| 規模(桁) | S〜M(数百〜数千行。**DS のモデル + 位置の式 + `next_deadline` への音声の期限**) | M〜L(数千行 + JS 側の Worklet、SAB のリング、COOP/COEP、`Atomics`) | A + B の差分 |

**読み(推奨。決めるのは PM)**: **C(混成)を最終形とし、段階導入の最初は A**。理由: (1) **A だけで G7 の「エンジンが音声出力を開き、空のプロジェクトで再生」までの機能は通る**(実機の字義の「AudioWorklet が連続して受け取る」は S3 で満たす)。(2) A なら native でテストが書ける(決定性)。(3) **骨格(`consumed` の式、vtable、Lock/Unlock、通知)は A と B で同じ**で、`consumed` の源だけを差し替える。**C の「切替点」は、`consumed` の源を 1 関数 `fle_audio_consumed()` に閉じ込める**ことで実現できる(読み。**設計だけで実装は無い**)。

**A の落とし穴(再掲)**: **全員が待ったとき `next_deadline()` に音声の次の通知時刻を足さないと、仮想時計が進まない**(§5.1 の 2)。ポーリングは `Sleep(1)` が時計を進めるので追加不要。**通知モードだけ**フックが要る。

### 6.3 AudioWorklet 側(browser-event-loop.md §8 の実測との関係)

- **構成**(引用。実測は `browser-event-loop.md` §8): Worker(エンジン)→ SAB のリング(インターリーブ float32 ステレオ)→ AudioWorklet が 128 フレームごとに読む。足りなければ無音にしてアンダーランを数え、生産側を `Atomics.notify` で起こす。**48 kHz/44.1 kHz、リング 1024 フレーム以上、生産側の 1 回 512 フレーム、定常のアンダーラン 0**(5 秒)。**リング 256 フレーム/生産 128 フレームでは約 45%** がアンダーラン。**生産側の 1 回に 6 ms(256 フレーム 5.3 ms の 113%)かかるとアンダーラン 168**。
- **FLEngine の側の数字**: 既定の 1 ブロックは **4096 フレーム(92.9 ms)**。リングが 8192 フレーム以上なら 1 ブロックを丸ごと置ける。**ミキサーの 1 ブロックの計算時間は未測定**(実エンジンのミキサーは一度も走っていない)。**空のプロジェクトのミキサーは軽い**と読むが、**G8 の再生では実測が要る**。
- **COOP/COEP**: SAB に要る(`browser-event-loop.md` と決定 0124 の引用。取得済み)。**`AudioContext` は user gesture の中で作る**(MDN の「Create or resume context from inside a user gesture」。取得済み)。**エンジン(`CreateFruityInstance`)の起動は gesture のあとにする設計が要る**(未決定。実機でなく `--autoplay-policy=no-user-gesture-required` の headless では動いた = 実測の前提)。
- **JSPI でなく保険の方式**(Worker で `Atomics.wait` して戻らない): ミキサーが `WaitForMultipleObjects(INFINITE)` で待つのを **Worker の `Atomics.wait`** に写せば、AudioWorklet の `Atomics.notify` で直接起こせる(**実時間の B と相性が良い**)。**ただし**協調スレッド(決定 0122/0124)の枠外になる(ミキサーだけ別 Worker = **別の線形メモリ**になる。`R[]`、ゲストのヒープを共有できない)。**したがって `pthreads` 相当(COOP/COEP + 共有メモリ + 全オブジェクトの再ビルド)が要り、決定 0122 の T3 の領域**。**推奨しない**(読み)。
- **サンプル形式**: ゲストが書くのは PCM16(**変換は `f_12c7420`: `f_6a2910` のクリップ検査(閾値 1.0)→ 関数ポインタ `[0x15e37b8]` → 実体は `f_6a0a90` か `f_6a5650`(各 121 B)で、倍率 **32767.0**(`0x12c7474`)。CPU 機能で選ぶ)**または float32(コピーのみ)。**旧実装の shim は 16 ビットを `v/32768` で float 化した(旧 REPORT_DS)。FLEngine 側の書き出し係数は 32767 倍なので、`/32768` では**満尺 ±1.0 が 0.99997 になる**(読み。**どちらが実機の WASAPI/ASIO 経路の係数と一致するかは未確認**)。**モデルは 16 ビット(`v/32768`)を受けて float に直す**のが標準的な解釈で、誤差は 1/32768 以下。

### 6.4 G7(無音・アンダーラン 0)に足りる範囲と、G8(非ゼロ)に必要な追加

| | G7 | G8 |
|---|---|---|
| 必要 | `DirectSoundCreate`/`DirectSoundEnumerateA`(1 件)、DS の 13 メソッドの状態モデル、`Lock`/`Unlock` のバッファ、play/write カーソルの式、通知 2 点または `Sleep(1)` のポーリング、協調スレッドでミキサーが走ること、**AudioWorklet への供給(S3)** | G7 の全部 + **ミキサーが実際に音を作る**(`f_d0ece0`)= 内蔵プラグイン(G9 の一部)、サンプルの復号(`wavpackdll`、`oggio`、`ildsp` = 未 lift の同梱 DLL)、プロジェクトの読み込み(`f_f51fd0`)、`TWAVStreamingThread`(ディスクリーダー)、`f_12c7420` が呼ぶ `[0x15e37b8]` の関数(FLEngine 内。lift 済み) |
| 出力 | 無音(ゼロのフレーム) | 非ゼロ |
| 規模(桁。日数は書かない) | **S〜M**(DS のモデルと駆動で数百〜数千行) | **M〜L**(**音声の経路そのものは G7 とほぼ同じ**。G8 の大半は音声以外: ファイル I/O、デコーダー、プラグイン) |
| 新しい不確実性 | 実行時の値(ポーリングか通知か、`lead`、ブロック長)が**実測できていない** | ミキサーの 1 起床あたりの計算時間(協調スレッドが UI を止めるか。thread-model.md S4 の止まる条件)、非ゼロの音の**実機との一致**(比較素材なし) |

**G7 を字義どおり満たす最小**は **S3(AudioWorklet に連続してブロックを渡す)まで**。S1・S2 だけでは「エンジン側が音声出力を開いて 1 ブロックずつ書く」までで、**ブラウザへの出力は無い**。この区別は PM の G7 の定義(`goal-ladder.md` 39 行)に照らして決める。

## 7. 推奨と判断点(決めるのは PM)

### 7.1 段階導入案

| 段階 | 内容 | 受け入れ(案) | 規模(桁) | 止まる条件 |
|---|---|---|---|---|
| **S0**(先行・小) | **実行で確かめる**: 現行 HEAD で `CreateFruityInstance` の経路に **`f_1253610`、`f_12afb50`、`f_1253260`、`f_c55ad0`、`f_12af3e0` への入口の到達有無**を取る(トレースの `FLE_FRUITY_THREAD_TRACE`/関数入口の記録で足りる。**新しい実装は不要**)。**どの種別(DS/ASIO/Plugin)が選ばれるか、`[0x15d3294]` の値**を実測する | 静的な「届きうる」を「届いた/届かない」に変える | S | 壁 52 以降の別の壁で到達できないなら、**その壁の先でしか確かめられない**ことを記録して止まる |
| **S1** | DirectSound の状態モデル(native。`fle_audio.c` のような 1 ファイル)。`DirectSoundCreate`/`DirectSoundEnumerateA`(1 件)、IDirectSound 11 スロット、IDirectSoundBuffer 21 スロット(13 メソッドを実装、他は `unimplemented`)、IDirectSoundNotify 4 スロット。バッファ = `calloc`。位置は**固定値**でテスト(単体) | `f_12afb50`/`f_12af5c0`/`f_12afaf0` が最後まで通り、`TMixerThread_DS` が作られ、`Play` が呼ばれる。使わないスロットが呼ばれたら名前付き停止 | S〜M | **実機の戻り値と食い違う出力の検証**(下の「止まる条件」) |
| **S2** | 再生位置の駆動と通知(**A: 仮想時計**)。`consumed` の式、play/write のカーソル、通知位置を跨ぐたびに `SetEvent`、**`next_deadline` への音声期限のフック**(`fle_thread.c`)。ポーリングは `Sleep(1)` が時計を進めるので追加不要 | ミキサーが 1 ブロックずつ `Lock`/`Unlock` を繰り返し、**アンダーラン計数 `[0x15e16d0]` が 0 のまま**、native と wasm で同じトレース | M | ミキサーの 1 起床が長く UI が止まる(thread-model.md S4 の止まる条件)。**通知イベントの取りこぼしモデルが `i xor 1` を壊す**(§5.1 の 3) |
| **S3** | wasm と AudioWorklet(**B: 実時間**、C の混成)。SAB のリング、`AudioContext`(user gesture)、COOP/COEP。`consumed` の源を Worklet の消費フレームに差し替え(`fle_audio_consumed()`) | `browser-event-loop.md` §8 と同じ「定常アンダーラン 0」を**エンジン側のブロックで**再現(48/44.1 kHz、リング ≥ 8192 フレーム)。**G7 の字義を満たす** | M〜L | **実エンジンのミキサーの 1 ブロックの計算が実時間を超える**(アンダーランが出る)。Safari の JSPI/AudioWorklet が未確認(`thread-model.md` の引用) |
| **S4** | ASIO(**不要が S0 で確認できれば作らない**)。`IASIO` の 24 スロットの vtable、`IClassFactory`、`CoCreateInstance`/`software\asio` の仮想レジストリ、ドライバーのスレッド(AudioWorklet が `bufferSwitch` の呼び元) | — | L | **ILWASAPI2ASIO が音声経路に絡む(ライセンス・認証)と分かったら止まる**(下) |

### 7.2 PM の判断点

1. **G7 の字義**(`goal-ladder.md` 39 行): 「AudioWorklet が一定のブロックを連続して受け取る」を**エンジンのミキサーが書いたブロックで**要求するか、**DS の状態モデルが通ること(S1+S2)で足りる**とするか。前者は S3 まで、後者は S2 まで。**私は S3 までを推奨**(G7 の「AudioWorklet」の字義)。**S2 までの完了を G7 達成と数えない**(エンジン側の動作に限り、ブラウザへの出力が無い)。
2. **再生位置の源**: **A(仮想時計)→ C(混成)** を推奨。**B のみ**にするとテストの決定性を失い(決定 0017、A5 のオラクル)、**A のみ**では G7 の「音が出る」を言えない。
3. **ポーリングと通知**: 両方通す(S1/S2 のテストを両モードで)。**実行時の既定は S0 で確かめる**。
4. **ASIO を作らない前提(S4 を後に回す)**で良いか。**仮想レジストリに `software\asio` を入れない限り ASIO は一覧に出ない**(読み)ので、この選択は「ASIO のキーを種付けしない」という**決定**(出典なしの選択)になる。
5. **バッファの置き場所**(§6.1): ゲストの `Lock` の戻りの領域を最初から共有リングにするか、コピーするか。**S3 の設計に効く**ので S1 の前に決めたい。
6. **`next_deadline` への音声期限のフック**(`fle_thread.c` の変更): 決定 0122/0124 の枠(`fle_win_next_timer_tick` と同じ形の弱い参照)で足すことを許すか。**スケジューラの規則(0122)は変えず期限の集合を増やすだけ**だが、`fle_thread.c` に触れる。
7. **16 ビット変換の係数**: FLEngine は float を **32767 倍**で丸める(§6.3)。ブラウザに渡すとき **`/32768`**(誤差 4.5e-5)か **`/32767`**(満尺 1.0)か。**どちらも実機の WASAPI/DS の係数としては出典なし**。実機との同等性(U5)は音の比較素材が無いので判定できない。

### 7.3 止まる条件(案)

- **実機と食い違う出力の検証**: DS の戻り値(`GetCurrentPosition` の play/write の差、`Lock` の 2 領域の分割、`SetFormat` の戻り)は、**Learn の記述と実機の値の間に空白がある**(例: write − play の既定の値)。**実機(Windows)で `GetCurrentPosition` の値列を取った素材が無い**。S1 の出力は「ページから読める範囲」で、実機との一致は**主張できない**。
- **ライセンス・認証が音声経路に絡む**: 本調査で読んだ範囲(DS 領域 0x12ae000〜0x12b0300、デバイス管理 0x1251000〜0x1254000、ASIO 領域 0xc52000〜0xc5e400)に `WinVerifyTrust`(`f_4256d0`)の呼び出しは **0 件**(`callers.py` の全 4 箇所は音声領域の外)。**ライセンス検証の関数が不明**(`host-engine-boundary.md` U9)なので、**音声経路に絡まないとは言えない**(未確認)。
- **ミキサーが UI を止める**: 協調スレッドは先取りしない(決定 0122)。**ミキサーの 1 起床で `f_d0ece0` が長く CPU を使うと**、メインスレッドのメッセージ処理が飢える。**G8 の再生で実測するまで分からない**。
- **S0 の結果が「音声に届かない」**: 壁 52 以降の別の壁が先に出る場合、**M-5 の実装は G1 の完了を待つ**。

## 8. 診断で到達していない部分(実測できていないこと)

- **`CreateFruityInstance` の先**: 診断 1(`diag1/lanes.md`)は「`DirectSoundCreate`、`midiOutOpen`、`CoCreateInstance`、`waveOutOpen` は 4 回の実行のどれにも一度も現れない」と書く(**引用**)。HEAD のコミット記述(壁 52 `SetWindowOrgEx` まで、8,651,367 入口)は、音声の初期化に届いたとは書いていない(**引用**)。
  - **補足(実測だが、今は再確認できない)**: 本調査の作業中、別の作業者が走らせていた native 実行の記録(`lift/out-fruity10/` の下の `a4` の出力。**その作業者の一時ディレクトリ**)を 1 つだけ読んだ。停止は `FillRect` の名前付き停止で、シャドウスタックの #29 が `f_11c8950`(戻り先 `0x11be41c`)、#30 が `f_11be0a0`、#31 が `f_13751e0` だった。`0x11be41c` は `f_11be0a0` 内の 3 つ目の `f_11c8950` 呼び出し(`0x11be417`)の直後で、**音声の初期化へ進む呼び出し 0x11be588(`f_11a57c0`)より前**にある(`fdis.py` で確認。0x11be41c < 0x11be588)。つまりその 1 回の実行は、音声の初期化の手前で止まっていた。**この記録のディレクトリは、本文の作成の終わり頃には既に無かった(`lift/out-fruity10/ow57/` が消えていた)ので、再確認できない**。リポジトリに入っていない他者の生成物なので、**根拠としては弱く、診断 1 の引用を主とする**。
  - **したがって「実行は音声の初期化に届いていない」は、診断 1(引用)と上の 1 回の記録(再確認不可)による**。HEAD 4ff4f33 のビルドを自分で走らせて確かめてはいない。
- **音声の実行値**: `[0x15d3294]`(ポーリング)、`[0x15d3295]`(ソフトウェアバッファ)、`[0x15d3296]`(float32)、`[obj+0x3c0]`(`lead`)、`[0x15e1f40]`(サンプルレート。静的 44100)、`[0x16bd4a0]`(バッファ長。静的 4096)、`[0x15ce278]`(種別。静的 -1 = DS、`f_1253070` が -1 を書く)、`[0x15e48a0]`(追跡方式)は**どれも実行では見ていない**。
- **ミキサーの 1 起床あたりの計算量**(`f_d0ece0`)と、1 ブロック(93 ms)を作る時間。
- **実機の `GetCurrentPosition` の値列**と `Lock` の分割。
- **ASIO の個別メソッド対応**、`ILWASAPI2ASIO_x64.dll` の中身(DLL が手元に無い)。
- **Safari**、実デバイス出力、`outputLatency`、バックグラウンドタブ(`browser-event-loop.md` §8 の「できなかったこと」の引用)。

## 9. 限界

1. **すべて静的**。命令を実行していない。「must-pass」は関数内の制御流れだけ(例外・戻らない呼び出し・間接ジャンプは模していない。`f_1196c00` などの巨大関数はスイッチ表を含む可能性があり、`mustpass.py` は間接 `jmp` があれば `unknown` を返す。**今回の結果で `unknown` は出ていない**ので、判定した関数には間接 `jmp` が無い)。
2. **vtable の対応は mingw-w64 の `dsound.h` の宣言順**(Windows SDK との照合なし)。IASIO は SDK の宣言を取得していない。
3. **受け手の分類**は `rcx` のロード元のグローバル(`0x16bd498`、`0x16bd4f8`、`0x16bd500`、`0x16bd508`)と、`DirectSoundCreate`/`CreateSoundBuffer`/`QueryInterface` の出力ポインタの引数で確定した。**DS 領域外の関数**(`f_1311a00`、`f_1253b50`)は `xref_data.py` で拾えた。**受け手が間接(別のポインタ経由)になる呼び出しは拾えていない**(旧 `fl_ds_surface.py` は領域内に「受け手未解決の `[+0]` 呼び出し 6 件」を報告している。**本調査の `f_41a770` の共通 Release 6 点と対応すると読む**が、照合はしていない)。
4. **`CoCreateInstance` 14 箇所のうち読んだのは `f_c53c60`、`f_5df610`、`f_87c1d0`、`f_87c4a0` の 4 関数**。他 10 は音声との関係を確認していない(**音声でない**と断定しない)。
5. **`TWAVStreamingThread` のイベントを誰が立てるか**、`f_d0ece0`、`f_c589b0` の全体(19.6 KB)は読んでいない。**ASIO の `start` の順序**は未確認。
6. **Learn の取得**は、主要ページの全文(Play、DSBUFFERDESC、WaitForMultipleObjects)と、検索結果の抜粋(他)。**`CreateSoundBuffer`、`GetStatus`、`SetCurrentPosition`、`Stop`、`Unlock` の各ページは取得できなかった**。
7. **旧ツリー**(`lift/out-oracle/old-ref/`。2026-10-18 時点の別の作業の複製。`.gitignore` 対象)の記述(`INVENTORY.md`、`REPORT_DS.md`、`docs/PROTOCOL.md`)は**引用**。再測定していない。旧実装の数字(「43/43 PASS」ほか)は**この HEAD の結果ではない**。
8. **規模は桁だけ**(日数なし)。
9. **自己訂正(作業中に起きたこと)**: (a) §4.3 の IASIO の disp は、最初のスクリプトが時間切れで 17 個分しか出さなかったため、19 個のラッパーを引き直して 19 種にした(最初の下書きの文は言い淀んでいたので差し替えた)。(b) §2.2 の初稿は「検証済みでない行は 0〜2」と書いたが、`api-sites.tsv` の全 488 行が検証済み(`verified = yes`)なので 0 に直した。(c) **「旧実装の `/32768` が実機と一致する」とは書いていない**(FLEngine の書き出し係数は 32767 倍と、変換関数 `f_6a0a90` の 121 B と `0x12c7474` の定数から実測した)。(d) 旧作業の推定「`0x15d3296` = `ds_32bitbufbox`」は**引用**で、フォームのリソース(DFM)を読んでいないので未確認のまま。

## 10. 成果物

- `docs/investigations/audio-m5.md`(本文)
- `docs/investigations/audio-m5/`(合計 約 100 KB):
  - `api-sites.tsv`(FLEngine の音声・COM・レジストリ・待ち系の呼び出し箇所 488 行)、`api-summary.tsv`(62 API の集計)、`bundled-dll-sites.tsv`(同梱 5 DLL、153 行)、`ds-vcalls.tsv`(DS の vtable 呼び出し点 19 行)
  - `scan_audio_sites.py`、`make_tables.py`、`fdis.py`、`xref_str.py`、`xref_data.py`、`callers.py`、`mustpass.py`、`funclist.py`、`buffer_math.py`
- 再現: `PYTHONDONTWRITEBYTECODE=1 python3 scan_audio_sites.py FLEngine <pe> > sites_FLEngine.tsv`(他の DLL も同様)→ `make_tables.py <dir> <out>`。
