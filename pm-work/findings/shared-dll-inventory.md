# 共有の DLL の棚卸し(#1511)

この表の正の資料は 3 つ。計測の出力(`pm-work/findings/shared-dll-measure.jsonl`)、表(`inputs/shared-dll-requests.tsv`、計測の出力から `tools/shared-dll/table.mjs` で作った)、分類(この文書の下の表)。

## まとめ

- 計測: 実エンジン。既定の bundle(`lift/out-app/bundle/raw/fl-Oz`)を Chrome で起動し、File・Options・Tools・Mixer・Add の操作を、実のマウスとキーで行った。計測の出力は 142 操作、`missing-path` の行は 30,636 件、うち `LoadLibrary*` の 126 は 74 件。
- 表: `inputs/shared-dll-requests.tsv`(11 行)。
- 分類(一覧の `Plugins\` の外の DLL 79 本): (a) 日常の機能に要る 6 本、(b) まれ 25 本、(c) 対象外 48 本。
- (a) の足す順: 1 `LAMEenc_x64.dll`(MP3 の書き出しで止まる)、2 `Reverb_x64.dll`(Riff machine で止まる。#1429 が扱う)、3 `WavpackDLL_x64.dll`(既定の曲を開く度に 126)、4 `oggio_x64.dll`(OGG の書き出し)、5 `EasyFLAC_x64.dll`(未計測。FLAC のサンプル)。`ILSvg_x64.dll` は #1395 が扱う。
- 束の下書き: `pm-work/issue-drafts/wave3/`(3 件。001 LAMEenc、002 WavPack と OGG、003 EasyFLAC)。

## 計測

- 道具: `tools/shared-dll/measure.mjs`(操作の走らせ方。出力は JSON の行)と `tools/shared-dll/table.mjs`(表を作る)。部品は `tools/e2e/lib`(操作・窓の表・メニュー)と `tools/smoke/welcome-flow.mjs` の部屋(1836 x 1056)と起動の手順を使う。
- 出力: `pm-work/findings/shared-dll-measure.jsonl`(153 行: 142 操作と、起動ごとの終わり)。
- 条件: runner の値(この機械とは別枠)。nproc 4。開始の時の loadavg 2.23 0.87 0.32(18:56 UTC の確認)。起動の確認の `elapsedMs` 12316(`boot.json`)。流れ A の `preBootBytes` 313,284,395。
- 群: 停止したら、同じ群の残りの操作は、新しい起動(新しい Chrome とプロフィール)で続ける。こうして、1 回の停止が後の操作を隠さない。群ごとの操作と件数:

| 群 | 操作 | 件数(操作) | 窓・要求の要点 |
|---|---|---:|---|
| start | 起動 → Welcome(No thanks → 既定の曲の行を二度押し)→ Space で再生 3 秒 | 3 | 既定の曲が開く。`WavpackDLL_x64.dll` と `ilvideo_x64.dll` の 126 が出る。再生の非無音の量子は 0(音の確認は未確認) |
| demo | 起動 → Ctrl+O で `Asher Postman - Future House.flp` | 3 | `TMissingPluginsForm`(Problems の箱)。この操作の `missing-path` は 3 件で、`LoadLibrary` の 126 は無い |
| file-7 | File > #7 の項目 1〜14 | 16 | 窓は開かず、`LoadLibrary` の 126 は無い(確認の行だけ) |
| file-8 | File > #8 の項目 1〜14 | 16 | 項目 1 だけ副メニューの popup が開いた。他は窓が無い |
| file-9 | File > #9 の項目 1〜14(書き出し。窓が開いたら Start を押し、保存の数を見る) | 18 | 項目 1(MP3 `Rendering to ?untitled.mp3`)で停止。項目 2〜13 は窓を開き、保存が増えたものもある。項目 14(MP3)で再び停止 |
| options | F10(`TMIDIForm`)・Options > #1〜#24 | 27 | 窓は `TMIDIForm` 等。項目 24 で `TCloudAccountsForm` |
| tools | F8(`TPlugListForm`)・Tools > #1〜#34 | 39 | 項目 4(Piano roll - riff machine)で `Reverb_x64.dll` の 126 と停止 |
| mixer | Mixer の挿入 1 → Slot 1 → Plugin database → Sytrus・Fruity Balance | 4 | #1484 の既知の欠陥で、項目が Plugin database に無く、挿入できなかった(`ok` が false)。確認できない |
| add | Add > #1〜#14 | 16 | 窓(`TSpeechForm`・`TMsgForm`・`TPluginListForm` 等)。内蔵のジェネレータの挿入は確認できない |

- 件数: `missing-path` の行 30,636(起動の度に約 2,600。これは起動の時の `FindFirstFileW`・`GetFileAttributesW` の確認)。`LoadLibrary*` の 126 は 74 行: `imm32.dll` 23、空の名前 22、`WavpackDLL_x64.dll` 12、`ffmpeg\x64\ilvideo_x64.dll` 14、`LAMEEnc_x64.dll` 2、`Reverb_x64.dll` 1。
- 停止: 3 回。(1) file-9 の項目 1: `LAMEEnc_x64.dll` の 126 の後、`fle: indirect call to 0 is not a known function entry (caller 0xd19bb2, near f_d199b3+0x1ff)`。(2) file-9 の項目 14: 同じ行。(3) tools の項目 4: `Reverb_x64.dll` の 126 の同じ操作で `fle: indirect call to 0x2000100010000 is not a known function entry`。
- `GetModuleHandleW` の 126 は、この計測では拾えない。`lift/runtime/fle_modules.c` の `get_module_handle` は、誤り番号を立てるだけで(253 行)、`fle_vfs_note_missing` を呼ばない。だから、この API は `missing-path` の行を出さない(ソースの確認。件数は確認できない)。
- 操作の確認: 上の表のとおり。確認できなかった物は、下の「確認できない」に書く。

## 表(`inputs/shared-dll-requests.tsv`)

作り方: `node tools/shared-dll/table.mjs pm-work/findings/shared-dll-measure.jsonl inputs/fl-installer-files.tsv > inputs/shared-dll-requests.tsv`。行は、一覧の `Plugins\` の外の DLL 79 本のうち、計測の `missing-path` の行に出た物(読み込みの 126 と、ファイルの確認の失敗の両方)。`dll` は file 名、`guest` は一覧の綴り、`trigger` は最初に出た操作(群/操作)、`count` は全操作の件数(起動の度の既定の曲を含む)、`stoppedAfter` は、同じ操作で engine が止まったか(後の操作での停止は数えない)。

| dll | guest | trigger（最初に出た操作） | count（件数） | stoppedAfter |
|---|---|---|---:|---|
| `dsp_ippv2_x64.dll` | `Shared\dsp_ippv2_x64.dll` | start/boot | 136 | false |
| `QuickFontCache_x64.dll` | `Shared\QuickFontCache_x64.dll` | start/boot | 228 | false |
| `Blend2D.dll` | `Shared\Blend2D.dll` | start/boot | 92 | false |
| `freetype_x64.dll` | `Shared\freetype_x64.dll` | start/boot | 44 | false |
| `ILTools_x64.dll` | `Shared\ILTools_x64.dll` | start/boot | 48 | false |
| `FLEngine_x64.dll` | `FLEngine_x64.dll` | start/boot | 87 | false |
| `wavpackdll_x64.dll` | `Shared\wavpackdll_x64.dll` | start/welcome-default | 60 | false |
| `ILVideo_x64.dll` | `Shared\ffmpeg\x64\ILVideo_x64.dll` | start/welcome-default | 70 | false |
| `LAMEenc_x64.dll` | `Shared\LAMEenc_x64.dll` | file-9/file-9-1 | 10 | true |
| `oggio_x64.dll` | `Shared\oggio_x64.dll` | file-9/file-9-2 | 4 | false |
| `Reverb_x64.dll` | `Shared\Reverb_x64.dll` | tools/tools-4 | 5 | true |

## 分類

基準:
- (a) 日常の機能に要る: 計測で止まりを起こした、または、既定の曲の読み込みか日常の書き出し・読み込み(音声の形式・言語)に要る見込み。
- (b) まれ: 日常の機能ではない(動画・ツール・DSP の部品など)、または、用途が未確認で計測の要求も無い。
- (c) この製品の対象外: ReWire・VSTi(FL 自身を他のホストで使う版)・LilyPond・BeatSlicer のツール・WebView2・Python の実行環境(#1394 の範囲で分類のみ)・管理コード(.NET)・遠隔のサーバ・本体(`FLEngine_x64.dll`。lift の入力)。32 ビット版と推定される物(名前に `_x64` が無い)も (c)。推定で、PE の確認は未確認。

件数: (a) 6、(b) 25、(c) 48。計 79(一覧の `Plugins\` の外の DLL。大きさの合計 260,501,104 B)。

### (a) の足す順(停止を起こす物が先)

| 順 | DLL | 分類の根拠 | 扱い |
|---|---|---|---|
| 1 | `Shared\LAMEenc_x64.dll` | MP3 の書き出しで 126 のあと engine が止まる(計測の止まり) | 束 W3-001 |
| 2 | `Shared\Reverb_x64.dll` | Riff machine で 126 のあと止まる(#1429 の型) | #1429 が扱う(下書きは作らない) |
| 3 | `Shared\wavpackdll_x64.dll` | 既定の曲を開く度に 126(計測の 12 回)。`.wv` の音声の読み込み | 束 W3-002 |
| 4 | `Shared\oggio_x64.dll` | OGG の書き出しで存在の確認(抽出の行あり。sample decoder の段) | 束 W3-002 |
| 5 | `Shared\EasyFLAC_x64.dll` | FLAC の読み書き。計測では要求が出なかった(未計測。推定) | 束 W3-003 |
| (外) | `Shared\ILSvg_x64.dll` | 言語の DLL(言語が出ない型)。計測では要求が出なかった | #1395 が扱う(下書きは作らない) |

### (b) と (c) の注記

- `Shared\ffmpeg\x64\ILVideo_x64.dll`(b)は、既定の曲を開く度に 126 が出る(計測の 14 回)。動画の機能は日常ではないので (b) とした。依存の 7 本(avcodec・avformat・avutil・avfilter・avdevice・swresample・swscale)も (b)。要判断: 起動の要求を (a) と見るかは PM が決める。束にはしていない(依存が多く、1 束 1〜3 本の規則に収まらない)。
- `Shared\dsp_ippv2_x64.dll`・`Blend2D.dll`・`freetype_x64.dll`・`QuickFontCache_x64.dll`・`ILTools_x64.dll`(b)は、起動か Riff machine で存在の確認(`GetFileAttributesW`)だけが出て、読み込み(LoadLibrary)の要求は無かった。止まりも無い。
- (b) の残り(`speaker_x64.dll`・`ildsp_x64.dll`・`DirectWaveCtEngine_x64.dll`・`beat_tracker.dll`・`elastique_x64.dll`・`DelZip64.dll`・`REX Shared Library_x64.dll`・`SG_x64.dll`・`svctl64.dll`・`sveng64.dll`・`AudioRestore.dll`・`iZAudioRestore.dll`)は、この計測の操作では要求が出ていない(未計測)。用途は名前と依存から推定した。
- 抽出の表に行がある物(`ci/extract-dlls.tsv`): `dsp_ippv2_x64.dll`・`Blend2D.dll`・`QuickFontCache_x64.dll`・`freetype_x64.dll`・`ILTools_x64.dll`・`oggio_x64.dll`・`elastique_x64.dll`・`ILSvg_x64.dll`・`Reverb_x64.dll`。作業の木へは取り出されるが、計測では実行時に `GetFileAttributesW` が 2 を返した(仮想ボリュームに載っていない)。
- (c) の 32 ビット版は、`Shared\dsp_ippv2.dll`・`Shared\freetype.dll`・`Shared\QuickFontCache.dll`。x64 の版が要求されているので、この 3 本は要らないと推定する(PE の確認は未確認)。

### 全 79 本の表

分類の列: a・b・c。「計測の要求」は、この計測で出た行(API と結果)。「lift の見込み」は、一覧の `imports` からの推定(依存が kernel32 だけなら低、窓・描画の依存があれば中、管理コードは対象外)。

| guest（一覧の綴り） | 大きさ（B） | 依存（imports） | 分類 | 計測の要求 | 理由 | lift の見込み（分かる範囲） |
|---|---:|---|---|---|---|---|
| `Shared\oggio_x64.dll` | 1,872,384 | 1 本(kernel32.dll) | a | OGG の書き出し(file-9-2)で、存在の確認(`GetFileAttributesW` 2)のみ。LoadLibrary の要求は無い | OGG の書き出し・読み込み。抽出の行あり(lift/out-sampledec)。計測では存在の確認のみ | 低(依存が少ない) |
| `Shared\ILSvg_x64.dll` | 1,504,256 | 1 本(kernel32.dll) | a | — | UI の言語(言語が出ない型)。#1395 が扱う(抽出の行あり。plugin.tsv は #1395) | 低(依存が少ない) |
| `Shared\Reverb_x64.dll` | 492,544 | 1 本(kernel32.dll) | a | Riff machine(tools-4)で、`LoadLibraryW "Reverb_x64.dll"` が 126。同じ操作で engine が停止(#1429 の扱い) | Riff machine の止まり(#1429 が扱う)。計測で止まりの型 | 低(依存が少ない) |
| `Shared\LAMEenc_x64.dll` | 465,720 | 3 本(kernel32.dll・user32.dll・advapi32.dll) | a | MP3 の書き出し(file-9-1・file-9-14)で、`LoadLibraryW "LAMEEnc_x64.dll"` が 126。直後に engine が停止(`indirect call to 0`) | MP3 の書き出し。計測で止まりの型(126 のあと `indirect call to 0`) | 中(窓・描画の依存) |
| `Shared\EasyFLAC_x64.dll` | 394,240 | 1 本(kernel32.dll) | a | — | FLAC の音声の読み書き(demo の .flac サンプル)。推定。計測では要求が出なかった(未計測) | 低(依存が少ない) |
| `Shared\wavpackdll_x64.dll` | 271,672 | 1 本(kernel32.dll) | a | 既定の曲を開く(welcome-default)の度に、`LoadLibraryW "WavpackDLL_x64.dll"` が 126(計測の 12 回) | WavPack の音声(.wv)の読み込み。既定の曲の読み込みで 126 が毎回出る | 低(依存が少ない) |
| `Shared\ffmpeg\x64\avcodec-58.dll` | 47,050,240 | 10 本(advapi32.dll・bcrypt.dll・kernel32.dll…) | b | — | 動画(ffmpeg 系)の依存。ILVideo_x64.dll が読む。まとめて扱う | 中(ネットワークの依存) |
| `Shared\dsp_ippv2_x64.dll` | 39,942,656 | 1 本(kernel32.dll) | b | 起動で、存在の確認(`GetFileAttributesW` 2)のみ | Intel IPP の DSP。起動で存在を確かめるだけ。止まりは無い | 低(依存が少ない) |
| `Shared\ffmpeg\x64\avformat-58.dll` | 12,441,600 | 11 本(advapi32.dll・crypt32.dll・gdi32.dll…) | b | — | 動画(ffmpeg 系)の依存。ILVideo_x64.dll が読む。まとめて扱う | 中(ネットワークの依存) |
| `Shared\ffmpeg\x64\avfilter-7.dll` | 11,524,608 | 13 本(advapi32.dll・cfgmgr32.dll・gdi32.dll…) | b | — | 動画(ffmpeg 系)の依存。ILVideo_x64.dll が読む。まとめて扱う | 中(ネットワークの依存) |
| `Shared\SG_x64.dll` | 5,892,096 | 15 本(winmm.dll・winspool.drv・comdlg32.dll…) | b | — | 用途は未確認(名前から推定できない)。未計測 | 中(窓・描画の依存) |
| `Shared\elastique_x64.dll` | 4,997,632 | 2 本(ws2_32.dll・kernel32.dll) | b | — | 音声の伸縮(時間の伸縮)。抽出の行あり(lift/out-sampledec)。未計測 | 中(ネットワークの依存) |
| `Shared\DirectWaveCtEngine_x64.dll` | 4,788,736 | 9 本(version.dll・shlwapi.dll・shell32.dll…) | b | — | DirectWave(内蔵のサンプラー)の部品。まれ。未計測 | 中(窓・描画の依存) |
| `Shared\ildsp_x64.dll` | 4,197,376 | 1 本(kernel32.dll) | b | — | DSP の部品。計測では要求が出ていない(未計測) | 低(依存が少ない) |
| `Shared\ILTools_x64.dll` | 3,662,336 | 14 本(winmm.dll・comctl32.dll・shell32.dll…) | b | Riff machine(tools-4)と起動で、存在の確認(`GetFileAttributesW` 2)のみ | ツール群の DLL。Riff machine と起動で存在の確認のみ | 中(窓・描画の依存) |
| `speaker_x64.dll` | 3,263,488 | 12 本(winmm.dll・comctl32.dll・shell32.dll…) | b | — | 用途は未確認(名前から推定できない)。計測では要求が出ていない | 中(窓・描画の依存) |
| `iZAudioRestore.dll` | 2,481,784 | 10 本(gdiplus.dll・kernel32.dll・user32.dll…) | b | — | 音声の修復(Audio Restore)。まれ。未計測 | 中(ネットワークの依存) |
| `Shared\QuickFontCache_x64.dll` | 2,067,968 | 8 本(winmm.dll・kernel32.dll・shell32.dll…) | b | 起動と既定の曲の読み込みで、存在の確認(`GetFileAttributesW` 2・3)のみ | フォントの索引(UI)。存在の確認のみ。止まりは無い | 中(窓・描画の依存) |
| `Shared\Blend2D.dll` | 2,065,920 | 1 本(kernel32.dll) | b | 起動と既定の曲の読み込みで、存在の確認(`GetFileAttributesW` 2)のみ | ベクタの描画(Blend2D)。起動で存在を確かめるだけ。未確認の用途 | 低(依存が少ない) |
| `Shared\ffmpeg\x64\avdevice-58.dll` | 1,879,552 | 18 本(advapi32.dll・gdi32.dll・imm32.dll…) | b | — | 動画(ffmpeg 系)の依存。ILVideo_x64.dll が読む。まとめて扱う | 中(窓・描画の依存) |
| `Shared\ffmpeg\x64\ILVideo_x64.dll` | 1,329,152 | 10 本(avutil-56.dll・kernel32.dll・user32.dll…) | b | 既定の曲を開く(welcome-default)の度に、`LoadLibraryW "ffmpeg\x64\ilvideo_x64.dll"` が 126(14 回)。動画の書き出し(file-9-5・-6)でも | 動画(ffmpeg 系)。既定の曲の読み込みで 126 が出る(要判断: 日常ではないが起動時の要求)。依存が 7 本 | 中(窓・描画の依存) |
| `Shared\beat_tracker.dll` | 983,552 | 1 本(kernel32.dll) | b | — | 音声の拍の検出。未計測 | 低(依存が少ない) |
| `AudioRestore.dll` | 953,976 | 5 本(kernel32.dll・user32.dll・advapi32.dll…) | b | — | 音声の修復(Audio Restore)。まれ。未計測 | 中(窓・描画の依存) |
| `Shared\ffmpeg\x64\avutil-56.dll` | 862,208 | 7 本(advapi32.dll・bcrypt.dll・cfgmgr32.dll…) | b | — | 動画(ffmpeg 系)の依存。ILVideo_x64.dll が読む。まとめて扱う | 中(窓・描画の依存) |
| `Shared\freetype_x64.dll` | 607,032 | 1 本(kernel32.dll) | b | 起動で、存在の確認(`GetFileAttributesW` 2)のみ | フォントの描画(FreeType)。起動で存在を確かめるだけ | 低(依存が少ない) |
| `Shared\ffmpeg\x64\swresample-3.dll` | 602,112 | 4 本(advapi32.dll・kernel32.dll・msvcrt.dll…) | b | — | 動画(ffmpeg 系)の依存。ILVideo_x64.dll が読む。まとめて扱う | 中 |
| `Shared\ffmpeg\x64\swscale-5.dll` | 566,784 | 4 本(advapi32.dll・kernel32.dll・msvcrt.dll…) | b | — | 動画(ffmpeg 系)の依存。ILVideo_x64.dll が読む。まとめて扱う | 中 |
| `Shared\sveng64.dll` | 399,160 | 1 本(kernel32.dll) | b | — | 用途は未確認(名前から推定できない)。未計測 | 低(依存が少ない) |
| `Shared\DelZip64.dll` | 362,808 | 3 本(kernel32.dll・user32.dll・shell32.dll) | b | — | zip の処理。未計測 | 中(窓・描画の依存) |
| `Shared\REX Shared Library_x64.dll` | 321,848 | 2 本(winmm.dll・kernel32.dll) | b | — | REX ファイルの読み込み。まれ。未計測 | 低(依存が少ない) |
| `Shared\svctl64.dll` | 211,768 | 4 本(winmm.dll・kernel32.dll・user32.dll…) | b | — | 用途は未確認(名前から推定できない)。未計測 | 中(窓・描画の依存) |
| `Shared\dsp_ippv2.dll` | 28,174,848 | 1 本(kernel32.dll) | c | — | 32 ビット版と推定(名前に _x64 が無い。PE の確認は未確認)。x64 の版が要求されている | 低(依存が少ない) |
| `FLEngine_x64.dll` | 22,227,016 | 24 本(shlwapi.dll・opengl32.dll・version.dll…) | c | 起動で、存在の確認(`GetFileAttributesW` 2)。本体は lift の入力 | 本体(lift の入力)。読み込みの対象ではない | 中(ネットワークの依存) |
| `FLMManaged.dll` | 9,294,848 | 1 本(mscoree.dll) | c | — | この製品の対象外: 管理コード(mscoree.dll の入口。lift していない) | 対象外(管理コード) |
| `Shared\Python\python312.dll` | 7,003,928 | 19 本(version.dll・ws2_32.dll・api-ms-win-core-path-l1-1-0.dll…) | c | — | この製品の対象外: FL の Python の実行環境(#1394 の範囲。分類のみ) | 中(ネットワークの依存) |
| `Shared\Python\libcrypto-3.dll` | 5,162,776 | 15 本(ws2_32.dll・advapi32.dll・user32.dll…) | c | — | この製品の対象外: FL の Python の実行環境(#1394 の範囲。分類のみ) | 中(ネットワークの依存) |
| `System\Tools\LilyPond\bin\gs-8.70.dll` | 5,013,824 | 8 本(advapi32.dll・libjpeg-62.dll・kernel32.dll…) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中(窓・描画の依存) |
| `System\Plugin\ReWire\FLReWire_x64.dll` | 3,612,160 | 12 本(winmm.dll・comctl32.dll・shell32.dll…) | c | — | この製品の対象外: ReWire(他のアプリとの接続) | 中(窓・描画の依存) |
| `System\Plugin\VSTi\x64\FL Studio VSTi (Multi).dll` | 3,596,800 | 12 本(winmm.dll・comctl32.dll・shell32.dll…) | c | — | この製品の対象外: FL 自身を VST として他のホストで使う版 | 中(窓・描画の依存) |
| `System\Plugin\VSTi\x64\FL Studio VSTi.dll` | 3,596,800 | 12 本(winmm.dll・comctl32.dll・shell32.dll…) | c | — | この製品の対象外: FL 自身を VST として他のホストで使う版 | 中(窓・描画の依存) |
| `System\Plugin\ReWire\x64\ReWire.dll` | 2,193,016 | 7 本(winmm.dll・version.dll・kernel32.dll…) | c | — | この製品の対象外: ReWire(他のアプリとの接続) | 中(窓・描画の依存) |
| `Shared\Python\sqlite3.dll` | 1,500,440 | 8 本(kernel32.dll・vcruntime140.dll・api-ms-win-crt-math-l1-1-0.dll…) | c | — | この製品の対象外: FL の Python の実行環境(#1394 の範囲。分類のみ) | 中 |
| `Shared\QuickFontCache.dll` | 1,260,544 | 8 本(winmm.dll・kernel32.dll・shell32.dll…) | c | — | 32 ビット版と推定(名前に _x64 が無い。PE の確認は未確認)。x64 の版が要求されている | 中(窓・描画の依存) |
| `System\Tools\LilyPond\bin\libpython2.4.dll` | 1,204,896 | 4 本(kernel32.dll・msvcrt.dll・pthreadgc2.dll…) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中(窓・描画の依存) |
| `System\Tools\LilyPond\bin\libglib-2.0-0.dll` | 1,048,096 | 8 本(advapi32.dll・libintl-8.dll・kernel32.dll…) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中(ネットワークの依存) |
| `System\Tools\LilyPond\bin\libiconv-2.dll` | 968,712 | 2 本(kernel32.dll・msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 低(依存が少ない) |
| `System\Tools\LilyPond\bin\libguile-17.dll` | 898,744 | 8 本(advapi32.dll・libgmp-3.dll・libintl-8.dll…) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中(ネットワークの依存) |
| `Shared\Python\libssl-3.dll` | 790,296 | 10 本(libcrypto-3.dll・kernel32.dll・vcruntime140.dll…) | c | — | この製品の対象外: FL の Python の実行環境(#1394 の範囲。分類のみ) | 中 |
| `System\Tools\LilyPond\bin\libfreetype-6.dll` | 504,096 | 2 本(msvcrt.dll・libz.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `Shared\freetype.dll` | 486,712 | 1 本(kernel32.dll) | c | — | 32 ビット版と推定(名前に _x64 が無い。PE の確認は未確認)。x64 の版が要求されている | 低(依存が少ない) |
| `System\Tools\LilyPond\bin\libpango-1.0-0.dll` | 323,560 | 4 本(libglib-2.0-0.dll・libgmodule-2.0-0.dll・libgobject-2.0-0.dll…) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\LilyPond\bin\libgobject-2.0-0.dll` | 314,848 | 2 本(libglib-2.0-0.dll・msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\LilyPond\bin\libgmp-3.dll` | 266,280 | 2 本(kernel32.dll・msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 低(依存が少ない) |
| `System\Tools\LilyPond\bin\libfontconfig-1.dll` | 249,904 | 5 本(libexpat-1.dll・libfreetype-6.dll・libiconv-2.dll…) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\LilyPond\bin\libpangoft2-1.0-0.dll` | 245,384 | 6 本(libpango-1.0-0.dll・libfontconfig-1.dll・libfreetype-6.dll…) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `Shared\ILRemoteServer_x64.dll` | 215,040 | 4 本(iphlpapi.dll・ws2_32.dll・user32.dll…) | c | — | この製品の対象外: 遠隔のサーバ(iphlpapi・ws2_32。ネットワーク) | 中(ネットワークの依存) |
| `System\Tools\LilyPond\bin\libpng-3.dll` | 156,080 | 2 本(msvcrt.dll・libz.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\LilyPond\bin\libjpeg-62.dll` | 142,512 | 1 本(msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 低(依存が少ない) |
| `System\Tools\LilyPond\bin\libexpat-1.dll` | 141,384 | 1 本(msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 低(依存が少ない) |
| `System\Tools\Diagnostics\WebView2Loader.dll` | 137,120 | 1 本(kernel32.dll) | c | — | この製品の対象外: WebView2(#1355 の範囲) | 低(依存が少ない) |
| `WebView2Loader.dll` | 137,120 | 1 本(kernel32.dll) | c | — | この製品の対象外: WebView2(#1355 の範囲) | 低(依存が少ない) |
| `System\Tools\LilyPond\lib\pango\1.6.0\modules\pango-basic-fc.dll` | 130,072 | 5 本(libpango-1.0-0.dll・libpangoft2-1.0-0.dll・libglib-2.0-0.dll…) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\LilyPond\bin\libz.dll` | 121,416 | 2 本(kernel32.dll・msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 低(依存が少ない) |
| `Shared\Python\vcruntime140.dll` | 119,192 | 6 本(api-ms-win-crt-runtime-l1-1-0.dll・api-ms-win-crt-heap-l1-1-0.dll・api-ms-win-crt-string-l1-1-0.dll…) | c | — | この製品の対象外: FL の Python の実行環境(#1394 の範囲。分類のみ) | 中 |
| `System\Tools\LilyPond\lib\lilypond\current\python\midi.dll` | 95,320 | 3 本(kernel32.dll・msvcrt.dll・libpython2.4.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\BeatSlicer\ZeroX_SC.dll` | 81,016 | 6 本(kernel32.dll・mfc42.dll・msvcrt.dll…) | c | — | この製品の対象外: BeatSlicer のツール(MFC42 の依存。まれ) | 中(窓・描画の依存) |
| `System\Tools\LilyPond\bin\libintl-8.dll` | 73,136 | 2 本(kernel32.dll・msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 低(依存が少ない) |
| `Shared\Python\python3.dll` | 68,376 | なし | c | — | この製品の対象外: FL の Python の実行環境(#1394 の範囲。分類のみ) | 低(依存が少ない) |
| `System\Tools\LilyPond\bin\pthreadGC2.dll` | 64,560 | 3 本(kernel32.dll・msvcrt.dll・wsock32.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\LilyPond\bin\libregex-2.dll` | 63,272 | 1 本(msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 低(依存が少ない) |
| `System\Tools\LilyPond\bin\libltdl-7.dll` | 51,024 | 2 本(kernel32.dll・msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 低(依存が少ない) |
| `Shared\Python\vcruntime140_1.dll` | 49,528 | 5 本(api-ms-win-crt-runtime-l1-1-0.dll・api-ms-win-crt-heap-l1-1-0.dll・api-ms-win-crt-string-l1-1-0.dll…) | c | — | この製品の対象外: FL の Python の実行環境(#1394 の範囲。分類のみ) | 中 |
| `System\Tools\LilyPond\bin\liblicoli-1.dll` | 43,760 | 3 本(libglib-2.0-0.dll・kernel32.dll・msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\LilyPond\bin\libguile-srfi-srfi-1-v-3-3.dll` | 42,440 | 2 本(libguile-17.dll・msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `Shared\Python\libffi-8.dll` | 39,696 | 3 本(kernel32.dll・vcruntime140.dll・api-ms-win-crt-runtime-l1-1-0.dll) | c | — | この製品の対象外: FL の Python の実行環境(#1394 の範囲。分類のみ) | 低(依存が少ない) |
| `System\Tools\LilyPond\bin\strop.dll` | 36,408 | 2 本(msvcrt.dll・libpython2.4.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\LilyPond\bin\struct.dll` | 36,000 | 2 本(msvcrt.dll・libpython2.4.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\LilyPond\bin\_locale.dll` | 30,560 | 4 本(libintl-8.dll・kernel32.dll・msvcrt.dll…) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |
| `System\Tools\LilyPond\bin\libgmodule-2.0-0.dll` | 30,336 | 3 本(libglib-2.0-0.dll・kernel32.dll・msvcrt.dll) | c | — | この製品の対象外: LilyPond(楽譜の書き出しのツール。System\Tools) | 中 |

## 前提: 既存の検査(下書きの共通)

- `lift/crates/fe-build/src/plugins.rs` の `validate`: guest は `\` で 3 要素以上(`Plugins\...\file`)、key は小文字で `.dll` で終わり、一意、sha256 は 64 桁の小文字の 16 進。`Shared\X.dll` は 2 要素なので通らない。
- `plugins.rs` の `plugin_db_names`: 本体の行を 1 つ求め、その guest の後ろから 2 番目の要素を Plugin database の名にする。`Shared\X.dll` では `Shared` になり、誤った項目を作る。
- `app/scripts/lib/plugin-rows.mjs` の `validate` は、同じ前提を持つ(`lift/crates/fe-build/src/plugins.rs` の写し)。
- 直しは #1429 の作業で行われる見込み。下書きには、この直しが要ると書いてある。main の版で確かめる。

## 確認できなかった物

- 各 DLL の `hook`・`omit`・`args`: lift の結果を見ていない。この棚卸しは `lift/plugins/` を変えない。束の作業で `fe-pe`・`fe-lift` の結果から決める(下書きに値は書いていない)。
- 用途が名前から推定できない DLL(`speaker_x64.dll`・`Shared\SG_x64.dll`・`svctl64.dll`・`sveng64.dll`)。
- Import(取り込み): File の #7・#8 の項目は、窓を開かなかった。どの項目が取り込みかを特定できていない。
- Channel rack に内蔵のジェネレータを挿す操作: 確認できない。Add の項目(#1〜#14)は、窓(`TSpeechForm`・`TMsgForm`・`TPluginListForm` など)を開いたが、生成器を挿す操作を特定していない。
- Mixer のスロットにプラグインを挿す: #1484 の既知の欠陥で、Plugin database に Sytrus・Fruity Balance の項目が無く、挿入できなかった。その DLL の要求(Sytrus・Fruity Balance)は、この計測では拾えていない。
- Export の項目と形式の対応: 窓の題名から、項目 2 は OGG、項目 3 は FLAC、項目 4 は MID、項目 5・6・13 は WAV、項目 1 と 14 は MP3 と分かった。動画(video)の形式は、ILVideo を要求した項目 5・6 の窓が WAV の題名を持つため、確かめていない。
- 既定の曲の再生: Space で再生したが、非無音の量子は 0 だった。音の出力は確認していない(P3 の範囲)。
- `Problems loading the project` の箱(`TMissingPluginsForm`)の原因: どの DLL・どの Plugin が原因かは、この計測では確かめていない。
- 空の名前の `LoadLibraryW` の 126(22 件): 呼び出し元は確かめていない(decision 0229 の既存の扱い)。

## 注記

- 計測の道具と表の作り方は、どちらも `tools/shared-dll/` に在る。表は計測の出力から作る(手で書かない)。
- 起動の確認(`tools/boot-check/boot-bundle.mjs`)と、流れ A〜D(`tools/smoke/welcome-flow.mjs`)は、この作業の受け入れの前半で通した(`elapsedMs` 12316・流れ A〜D の 4 行とも `trapped` が false・`hostErrors` が 0)。`npm test --prefix app` も通した(75 ファイル、1465 件)。
- 製品の DLL の行(`lift/plugins/`・`ci/extract-dlls.tsv`)は、この Issue では変えていない。
