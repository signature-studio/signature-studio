# 参照プロジェクト `serina1-serum2.zip` の構成と、エンジンへの要求(静的解析)

作成: 2026-11-03、基準 HEAD **45402a1**(ブランチ main)。担当: 調査 subagent(PM セッション ses_eafd6e97fffeXSloTmS42jhMt4 から起動)。

**これは静的な解析で、実エンジンは動かしていない。性能の測定ではない。** FL Studio も `FLEngine_x64.dll` も実行していない(読んだのは、依頼者の zip の中身、依頼者の `~/ダウンロード/FL_STUDIO_.Plugins.ini`、`FL_STUDIO_VerifiedIDs.nfo`、`OTT.fst`、FLEngine の機械語の静的な読み取り、既存の調査文書、公開文書)。ここに出てくる「進捗」は 0 件で、G 段階表の到達にも数えない。

## 凡例

- **実測** = 自分で数えた/読んだ(コマンドの出力。再現コマンドは §1 と `tools/`)。
- **読み** = 解釈(第三者の FLP 仕様、逆アセンブルの直接 call 関係、文字列、名前の一致から)。実行して確かめていない。
- **未確認** = 調べていない、または決められない。
- **推測** = 根拠が弱い見積り(前提を書く)。
- 数字は自分で数えた/測った物だけ。FLEngine の関数は `f_XXXXXX`(ImageBase 0x400000 を含む VA)。到達は直接 call/jmp のみ(下限)。

## 0. 要約(先頭)

### 0.1 このプロジェクトの構成(実測)

| 項目 | 値 |
|---|---|
| FLP | FL **26.1.7.5654**、format 0、**PPQ 96**、**200 BPM**(テンポのイベント 156 = 200000、拍子 4/4)、1,322,619 B、イベント 7,021 個。テンポのオートメーション: 無し(読み。13 個のオートメーションクリップの名前に Tempo は無い。リンク先のバイトは未解読) |
| チャンネル | **70**: 音声クリップ **48**(zip の音声 48 個と 1:1)、オートメーションクリップ **13**、プラグイン楽器 **8**(Serum 2 ×**6**、Vital ×1〔VST3〕、FLEX ×1〔FL 内蔵〕)、Sampler 1(空) |
| パターン | **7**、ノート計 **425**(8 個の楽器チャンネルすべてにノートがある。パターン 7 は FLEX の 4 ノートで 116 回置かれる) |
| プレイリスト | アレンジメント 1、トラック枠 500 中 **30 使用**、クリップ **3,565**(音声 3,089/パターン 155/オートメーション 321)。最初のクリップは 3.9 秒、終端 **108 小節 = 129.6 秒** |
| ミキサー | インサート記録 **127**(マスター + 125 + 「現在」と読む)× スロット 10 = 1,270。使用中 **48**(名前付き 46、チャンネルが接続 37、プラグイン入り 19)。スロットの使用 **38** |
| プラグイン | インスタンス計 **46**: **FL 内蔵(ネイティブ)25**〔7 種: Soundgoodizer ×12、Fruity Parametric EQ 2 ×8、Fruity Delay 2、Fruity Balance、Fruity Filter、Frequency Shifter、FLEX〕、**サードパーティ 21**〔VST3 ×20(11 製品)、VST2 ×1(GClip)〕 |
| サードパーティの製品 | Xfer **Serum 2**(×6、楽器)、**Vital**(×1、楽器)、Xfer **OTT**(×2)、Kilohearts(kHs Filter ×2、kHs Gain、kHs Haas ×2、kHs Pitch Shifter、kHs Tape Stop、Disperser)、**Spectral Gate**(Stepwise)、iZotope **Ozone Imager 2**(×2)、**GClip**(VST2) |
| FLP の中身 | 62%(818,440 B)がプラグイン状態 46 個。Serum 2 の 6 個で 455,226 B(34%、最大は 309,918 B)、Vital 267,913 B(20%)。次がプレイリスト 313,720 B(24%)、MixerParams 48,780 B(3.7%) |
| 音声 | wav 43、mp3 4、ogg 1。計 **701.1 秒**、ファイル 61.4 MB、デコード後(float32 と読む)**246 MB**。うち **mp3 4 本が 518.8 秒・183 MB(74%)** |
| 同時に鳴る最大(読み。ノートの尾は含めない) | 音声クリップ 10、パターン 3、オートメーション 5、同時ノート 11(楽器チャンネル最大 5)、ミキサーの有効インサート 16、その中のサードパーティのエフェクト 11、ネイティブのエフェクト 12。マスターの 6 個(VST3 ×4、VST2 ×1、ネイティブ ×1)は何かが鳴る間は常時 |

### 0.2 Serum 2 を使っているか: **確定**

- FLP の `PluginParams`(event 213)の VST3 ラッパー(cid 54 = `Serum2`、cid 55 = `C:\Program Files\Common Files\VST3\Serum2.vst3\Contents\x86_64-win\Serum2.vst3`、cid 56 = `Xfer Records`)が **6 インスタンス**(チャンネル 1/2/3/28/33/37)。
- 各状態は `XferJson`(product=Serum2、productVersion=**2.1.5**、version 11.0、zstd)。依頼者の `FL_STUDIO_.Plugins.ini` に同じ GUID `{56534558-6673-5073-6572-756D20320000}`(ps_file_size = 39,121,408 B、Intel 64 ビット)がある。
- チャンネル名は `Serum` / `Serum #2` / `Serum 2` / `Serum #3` / `Serum #4` / `Serum #5`(全部 Serum 2 のラッパー)。依頼者の過去の実装 `<OWNER-HANDLE>/flp-extract-fxp` の docs が書く `serina1.flp`(5 Serum + 1 Serum2、46 個の event 213)と構成が合う(読み。同じプロジェクトを Serum→Serum 2 に変換して保存し直した版と見える。確認はしていない)。

### 0.3 エンジンの機能とのギャップのうち最大のリスク 3 つ

1. **サードパーティの VST3 20 インスタンス(Serum 2 ×6 を含む)が、現状の計画のどこにも無い(G11)。** ノート中の楽器は最大 5 チャンネル同時(Serum 2 が最大 4 つ + FLEX。Serum 2 が 4 つ同時の区間が 31.2 秒、尾は含めない)。マスターにも VST3 ×4 + VST2 ×1 が常時。VST3 のホスト、プラグインの実体(Windows の x64 DLL、依頼者の PC で計 67.7 MB)、パラメータ(kHs 2,086 個、Serum 2 2,623 個、Vital 2,983 個)、ライセンスのどれも lift/実装の対象になっていない。プラグインを供給しなければ、FL は警告を出して**そのプラグインを無視して読み込みを続ける**(エンジンの文字列。§4.3)ので、開くことはできても Serum 2 の 6 チャンネルと Vital の音は出ない = 「このプロジェクトの音」ではなくなる。P3 の前提。
2. **サンプルの読み込みが完了しない可能性(G8)。** mp3 4 本(音声の 74%)はエンジンが **ACM(msacm32)** で復号する(`f_a65690` → `f_a5ee30` → `acmDriverOpen`/`acmStream*`)。ランタイムに行があるのは `acmMetrics` だけで、ACM の MP3 ドライバーもモデルも無い。Vorbis 2 本は未 lift の `oggio_x64.dll`。さらに zip の取り扱い(`DelZip64.dll` 未 lift、またはアプリ側で展開)と、FLP のサンプル参照 48 個がすべて作者の一時フォルダーの絶対パスである点(エンジンは『サンプル N 個が見つからない』警告と、FL Cloud からのダウンロードを含む自動解決に入る。§2.8。48 個をその絶対パスにそのまま置けば避けられる、と読む)。アプリに FLP を開く経路自体が無い(`app/src`・`app/worker`・`app/shared` を `openProject|\.flp|demo` で grep して 1 件だけ、`.flpack` の MIME 設定)。
3. **実時間の計算量(P2/P3)。** 空のプロジェクトの時点で、native -O2 の再生は実時間の約 1/20(決定 0144。2.5 仮想秒に壁 48 秒)、ブラウザの UI は 0.3〜2 fps(`performance-targets.md`)。このプロジェクトはそこに、127 インサートのミキサー、3,565 クリップ、46 プラグイン(同時に最大でサードパーティ楽器 4〜5 + 11 エフェクト、ネイティブ 12 エフェクト)、時間伸縮の可能性のある音声チャンネル 10 個、246 MB のデコード済みサンプルを足す。実測した数字は無い(§5)。

## 1. 手順・出典・環境

- 作業木は読み取り専用(自分の出力は `lift/out-git/proj1-results/`〔新規〕、展開物は `lift/out-proj1/x/`〔新規。62,747,002 B、`git` に入れない〕、一時物は `/tmp/opencode/proj1/`)。元の zip は変更・移動していない。`unzip -q` で `lift/out-proj1/x/` に展開(49 ファイル。サブディレクトリ無し)。
- 自作のパーサー: `tools/flp_model.py`(標準ライブラリのみ。`PYTHONDONTWRITEBYTECODE=1`)。表の生成 `tools/flp_report.py`、音声 `tools/audio_probe.py`、不変条件の検査 `tools/selfcheck.py`(17 項目すべて OK。`tables/selfcheck.txt`)、予算の入力 `tools/budget.py`。
- エンジン側の静的な読み: `docs/investigations/goal-ladder/fle_index.py` で索引(`/tmp/opencode/proj1/fle.pkl`、3.97 MB)を作り、`g8-project-load/fle_query.py`/`fle_disasm.py` で問い合わせた(いずれも読み取りのみ)。`binaries/win64/FLEngine_x64.dll`(SHA-256 `b2c64da4…`、22,227,016 B)。
- 公開資料(取得 2026-11-03):
  1. https://pyflp.readthedocs.io/en/latest/architecture/flp-format.html(HTTP 200、38,216 B)。取得できた文言: 「FLP is a binary format used by Image-Line FL Studio … An FLP file contains of basically 2 sections or "chunks"」、ヘッダ `struct { char magic[4]; uint32_t size; // always been 6; int16_t format; uint16_t num_channels; uint16_t ppq; }`、イベント ID 0-63 = 1 byte、64-127 = 2 bytes、128-191 = 4 bytes、192-255 = length prefixed(varint)。
  2. PyFLP のソース(`raw.githubusercontent.com/demberto/PyFLP/master/pyflp/{_events,channel,plugin,project,mixer,pattern,arrangement}.py`、全部 HTTP 200。**GPL-3.0 の第三者のリバースエンジニアリング**で、Image-Line の資料ではない)。イベント ID の名前と構造体の欄(プレイリスト項目、ノート、MixerParams、InsertFlags、VST ラッパーのレコード)の出典。コードは取り込まず、構造だけを読んで自分で書いた。
  3. 依頼者の助言にあった https://github.com/<OWNER-HANDLE>/flp-extract-fxp(`docs/flp-conversion.md`、`docs/flp-serum2-conversion.md`、`src/flp.rs`、`README.md`、`AGENTS.md` を取得。HTTP 200)。取得できた文言: 「`PluginParams` (event 213) … Layout: `u32 LE format version`, then a sequence of records `[u32 LE cid][u64 LE size][data]` … All samples use version 12」、「cid 53 = state、54 = name、55 = filename、56 = vendor」、「`serina1.flp` … 6,875 events, 46 of them event 213 … 5 Serum + 1 genuine Serum2」。`AGENTS.md` は同リポジトリのエージェント向けで、私への指示ではないので従っていない(grep の出力で 2 行見えただけ)。
  4. https://github.com/Holzchopf/flp-file の README(`raw…/main/README.md` は 404。使っていない)。Image-Line 公式の FLP 仕様書は見つけていない(g8-project-load.md §1 と同じ)。
  5. websearch(Serum 2 の動作条件。§4.4 に断片だけ)。
- ネットワークで取得した物に、依頼者でない指示は含まれていなかった。

## 2. FLP の解析

### 2.1 形式と、見つけたこと(実測)

- `FLhd`(長さ 6: format 0、チャンネル 70、PPQ 96)→ `FLdt`(1,322,597 B)→ イベント列。**イベント列は FLdt の長さとちょうど一致、読み残し 0**(`selfcheck`)。
- **event id 172(0xAC)の値は 3 バイト**として読まないと整合しない(経験則、公開資料に無い): 汎用規則(128〜191 は 4 バイト)だと、1 バイトの ID 0 のイベントが「値 > 1」で 38 個現れ(テキスト領域をバイト対として読む)、偶然に同期が戻る。3 バイトにすると 0 個。FL 25.2.3(同梱の `NewStuff.flp`)でも同じ(46 → 0)。FL 24.2.99 以前の同梱 3 本には 172 が無い。**エンジンの実際の読み方は未確認**。2 回現れる(先頭の `FL Studio 26.1.7.5654.5654` の前と、コメントの直後)。
- **プレイリスト項目は 88 B**(PyFLP は 32/60 B)。313,720 B = 3,565 × 88。全項目で `pattern_base` = 0x5000。60 B の読みでは割り切れない(余り 40)。構造は 60 B の先頭 32 B と同じで、後ろに 56 B(1.0 の float など)が付く。**FL 25 以降の版の変更**と読む。
- チャンネルは `NewChan`(64)で始まり、パターン(65)はチャンネルより前、アレンジメント(99)、ミキサーのインサート記録(236 で始まる)、最後に MixerParams(225)。インサートの順序は「記録 0 = マスター」(マスターのチェーンに Ozone Imager・GClip が入り、オートメーション名 `… - Master - …` と `Filter - Insert 47` が記録 0 と記録 47 に合う)。
- **チャンネル → インサートの割り当ては event 104(`WORD+40`)で、値 = インサート番号 + 1**(0 = 未割り当て)。同名のチャンネルとインサートが 33 組あり、33 組すべてで一致(PyFLP の `RoutedTo`〔ID 22〕はこのファイルに無い)。読み。

### 2.2 FLP のバイトの内訳(`tables/event_bytes.tsv`)

| event | 名前(PyFLP) | 個数 | バイト | FLP に占める割合 |
|---|---|---:|---:|---:|
| 213 | PluginData | 46 | 818,440 | 61.9% |
| 233 | Playlist | 1 | 313,720 | 23.7% |
| 225 | MixerParams | 1 | 48,780 | 3.7% |
| 238 | TrackData | 500 | 35,000 | 2.6% |
| 218 | EnvLfo | 350 | 23,800 | 1.8% |
| 196 | SamplePath | 48 | 15,194 | 1.1% |
| 215 | ChanParams | 70 | 11,760 | 0.9% |
| 224 | PatNotes | 7 | 10,200 | 0.8% |

### 2.3 チャンネル(`tables/channels.tsv`)

| 種類 | 個数 | 備考 |
|---|---:|---|
| AudioClip(type 4 + サンプル + プラグイン無し) | 48 | 全部 `KeepOnDisk`(0x100)が無い = メモリに読み込む(event 143 の値は 0x2 が 44、0x6 が 4。PyFLP の欄の読み)。時間伸縮の欄(ChanParams の PyFLP 位置): mode 0(リサンプル)38、mode −1 が 8、mode −2 が 1(`pitch` 50)、mode 5(Auto)が 1(`pitch` 650)。−1 が elastique を呼ぶかは未確認 |
| AutomationClip(type 5) | 13 | §2.7 |
| Instrument(Fruity Wrapper) | 7 | Serum 2 ×6(iid 1/2/3/28/33/37)、Vital(iid 55) |
| Instrument(FL 内蔵) | 1 | FLEX(iid 36、名前 `Agogo`、状態 2,707 B) |
| Sampler | 1 | 空(サンプル無し) |

楽器チャンネルの状態(`tables/plugins.tsv`): iid 1 = 20,908 B、iid 2 = 38,898 B、iid 3 = 48,200 B、iid 28 = 18,680 B、iid 33 = 18,622 B、**iid 37 = 309,918 B**(`Serum #5`。zstd 展開後 378,409 B)、Vital 267,913 B。cid 4(パラメータ ID 表)は Serum 2 が 2,623 個、Vital が 2,983 個。

### 2.4 パターン(`tables/patterns.tsv`)

| パターン | ノート | 内訳(チャンネル iid:ノート数) | プレイリストに置かれた回数 |
|---:|---:|---|---:|
| 1 | 17 | 1:17 | 2 |
| 2 | 65 | 2:32、3:33 | 16 |
| 3 | 115 | 2:55、3:60 | 3 |
| 4 | 150 | 28:27、33:123 | 14 |
| 5 | 58 | 33:47、37:11 | 3 |
| 7 | 4 | 36:4(FLEX) | **116** |
| 8 | 16 | 55:16(Vital) | 1 |

### 2.5 プレイリストと Song Mode(`tables/playlist_*.tsv`、`timeline_intervals.tsv`)

- 3,565 クリップ。**音声 3,089**(48 チャンネル。最多は mp3 ボーカル `[fadr.com] Vocals - Dot To Dot…` の 963、`zap14` の 916、`celtix_Albiorix_Kick_06` の 254、mp3 `Other - Dot To Dot…` の 184)。長さの中央値 24 tick(0.075 秒。最小 6 tick、最大 6,148 tick = 19.2 秒)= **細かく刻んだクリップが大量**。パターン 155(中央値 96 tick = 0.3 秒)、オートメーション 321(`Balance - Insert 1 - ボリューム` だけで 256 回)。
- 30 トラックを使用(トラック 11 が 570 クリップ、10 が 484、4 が 367、8 が 332、6 が 320)。最初のクリップ 3.92 秒、最後の終端 41,472 tick = **108.0 小節 = 129.6 秒**(テンポ固定 200 BPM と読んだ場合。PPQ 96。時間の単位は読み)。
- **同時に鳴る数**(境界ごとの掃引。3,901 区間。パターンの長さは「ノートの終端を 1 小節に切り上げ」と読み。クリップがパターンより長いときは繰り返す前提。尾・残響は含めない):

| 量 | 最大 | 曲全体の平均 | 備考 |
|---|---:|---:|---|
| 音声クリップ | 10 | 6.25 | 9 個以上の区間が 17.2 秒 |
| パターンクリップ | 3 | 1.72 | |
| オートメーションクリップ | 5 | 1.73 | |
| 同時ノート | 11 | 5.21 | ノートがある区間 115.0 秒 |
| 楽器チャンネル(ノート中) | 5 | 2.77 | 5 同時の区間 29.1 秒 |
| 有効インサート(音源 + 経路をたどる) | 16 | 8.81 | |
| そのサードパーティのエフェクト | 11 | 7.16 | 9 個以上の区間が 30.2 秒 |
| そのネイティブのエフェクト | 12 | 6.85 | |

- 最も重い区間(読み): **59.7〜61.2 秒付近**(音声 9、パターン 3、ノート 11、楽器 5 チャンネル、有効インサート 14、サードパーティのエフェクト 11、ネイティブ 12。小節では 50 小節目付近)。合成の重み(音声 + ノート + サードパーティ + ネイティブ)の上位 6 区間は、すべて 59.7〜61.2 秒にある。
- 無音(音声もノートも無い): 1,262 tick = 3.9 秒(ほぼ先頭の空白のみ)。

### 2.6 ミキサー(`tables/mixer.tsv`)

- 127 記録 × 10 スロット。**使用中 48**: 名前付き 46、チャンネルが接続 37(接続されているチャンネルは 39、31 チャンネルは未割り当て〔値 −1〕)、プラグイン入り 19。
- ルーティング(event 235 は「どのインサートへ送るか」のフラグ列と読む): 107 インサートがマスター(0)へ、**16 が記録 1(ボーカルの名前を持つバス)へ**、2 が記録 47 へ(47 → 1 → 0)。
- 未割り当て(値 −1)の音声クリップのうち、プレイリストに置かれているのが **18 チャンネル**(例: `snare1 (consolidated)` 10 回、`celtix_Albiorix_Snare_20` 25 回)。これらが無音か、マスターへ行くかは**未確認**(私の同時数の計算では有効インサートに数えていない)。
- MixerParams: 4,065 項目(12 B)。インサート音量(id 192、127 件)は 12800 が 102 件、16000 が 6 件、14124 が 2 件、残り 17 件はそれ以外(12800 = 既定と読む)。スロットの無効化は 2 件(記録 69 と 75。プラグインの入っていないスロットかもしれず未確認)。
- `InsertFlags` は 0x4c が 124、0xc が 3(記録 0、1、126)。使用の指標にはならない。

### 2.7 オートメーション(`tables/automation_clips.tsv`)

13 クリップ(各 2〜7 点)。名前は自動命名: `…Channel pitch`、`Balance - Insert 1 - ボリューム`、`zap14 - Panning`、`kHs Filter - … - Cutoff` ×2、`Phase Plant - 4: FX`、`kHs Tape Stop - Phase Plant - Play`、`Spectral Gate - Master - Cutoff`、`… - Panning`、`Ozone Imager 2 - Master - Width`、`Filter - Insert 47 - Cutoff frequency`、`FreqShift - Master - Dry/Wet mix`/`Frequency`。**リンク先のバイトは未解読**。名前に `Phase Plant`(このプロジェクトに無いプラグイン)が 3 件ある(古いリンクの名前が残ったものか、未確認)。

### 2.8 サンプルの参照(`tables/channels.tsv`)

- 48 個すべて**絶対パスで、1 つのフォルダー**: `C:\Users\cabbage\AppData\Local\Temp\Image-Line\76728-{9BABC52F-…}\{6952ECA0-…}\<名前>`。FL が「zip のループパッケージ」を展開する一時フォルダーの形と読む(zip を開くたびに GUID が変わる)。**同じ名前 48 個がすべて zip のルートにある**。そのままではどこの機械でも存在しないパス。エンジンの FLP 読み込み(`f_f46490` → `f_da11b0`)には**見つからないサンプルの処理が実在する**(文字列の実測): 「%d sample(s) required by this project could not be located.」「FL Studio will attempt to resolve these issues automatically and download them from FL Cloud」「All errors resolved automatically. Reload project to continue.」(`Warning|` の通知)。解決の枝には `CopyFileW` と `FindFirstFileW`、「Preparing download...」(FL Cloud からの取得 = ネットワーク)がある(`f_da3310`/`f_da34d0`)。48 個がこの枝に入ったときの結果は**未確認**。最も単純な回避は、アプリが 48 個を FLP が記録した絶対パスにそのまま仮想ボリュームへ置くこと(読み。48 個は同じフォルダーなので 1 つのディレクトリで足りる)。

## 3. 音声ファイルの解析(`tables/audio_files.tsv`)

### 3.1 形式(ヘッダの読み。実測)

| コンテナ/コーデック | ビット | ch | レート | ファイル | 秒 | ファイル B | デコード後 float32 B | プレイリストでの使用回数 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| WAV PCM | 16 | 2 | 44.1k | 17 | 59.6 | 10,518,468 | 21,024,072 | 166 |
| WAV IEEE float | 32 | 2 | 44.1k | 10 | 49.5 | 17,454,666 | 17,452,232 | 455 |
| WAV PCM | 24 | 2 | 44.1k | 9 | 41.9 | 11,096,060 | 14,788,400 | 1,261 |
| **MP3(MPEG-1 Layer 3、VBR/CBR)** | - | 2 | 44.1k | **4** | **518.8** | 15,245,907 | **183,048,192** | 1,150 |
| WAV PCM | 24 | 2 | 48k | 2 | 23.0 | 6,617,946 | 8,823,528 | 4 |
| WAV PCM | 8 | 1 | 22.05k | 1 | 1.5 | 33,116 | 131,872 | 1 |
| WAV IEEE float | 32 | 1 | 44.1k | 1 | 0.6 | 105,480 | 105,252 | 36 |
| WAV PCM | 16 | 1 | 22.05k | 1 | 1.6 | 71,788 | 143,280 | 5 |
| WAV PCM | 16 | 1 | 44.1k | 1 | 2.8 | 250,674 | 498,704 | 8 |
| **WAV タグ 0x674f(Vorbis を RIFF に包む。中は OggS)** | 16 | 1 | 44.1k | **1** | 1.0 | 21,068 | 173,640 | 2 |
| **Ogg Vorbis** | - | 1 | 44.1k | **1** | 0.8 | 9,210 | 132,304 | 1 |
| 計 | | | | **48** | **701.1** | **61,424,383** | **246,321,476** | 3,089 |

- mp3 の中身: `Other - Dot To Dot…`(208.5 秒、VBR 32〜320 kbps、Xing)、`[fadr.com] Vocals - Dot To Dot…`(208.5 秒、同じ長さ、VBR 32〜320)、`Sleigh_Bells04-01…`(93.7 秒、VBR 32〜256)、`windchaim`(8.0 秒、CBR 128、Info)。長さは全フレームを数えた値。
- WAV のチャンクは `fmt`/`data` のほかに `smpl`/`inst`/`acid`/`strc`/`cue `/`bext`/`LGWV`/`LIST`/`id3 ` が混在(「読み」としてエンジンが使うかは未確認)。`smpl` のループは `Sampler` のフラグ(event 143)に出ていない(`UsesLoopPoints` 0x8 が 0)。
- **デコード後のサイズ**は、FL が各サンプルを「32 ビット float・ファイルのレートのまま」メモリに持つと**読み**で置いた値(根拠: `KeepOnDisk` が全部 0 でメモリに読み込む、エンジンの読み込み関数の進捗文字列が "Loading"/"Decoding" で全体を読む形、FL の公開仕様の一般知識。エンジンでの確認は**未確認**)。16 ビットで持つなら 123,160,738 B。PCM 以外は読み込み時に全部デコードされ、再生中に復号する形ではないと読む(`SmpLoader_*` が "Decoding" を出して戻る)。

### 3.2 FL が各形式の復号に使う経路(静的な読み)

`f_a65690`(サンプル読み込みの振り分け。進捗文字列 "Loading"/"Importing"/"Decoding" を出す)が、`f_a65260` で**拡張子**から種別を決める(14 の拡張子の表 → `0x14dcae4` の dword 表。**実測**で値を読んだ):

| 拡張子 | 種別 | 呼ぶ関数(直接 call) | 経路 |
|---|---:|---|---|
| .WAV | 0 | `f_677290` | WAV の単位(`MapViewOfFile`)。PCM/float は DLL 無し(g8-project-load §0)。`f_677290` → `f_677250` → `f_675660` → `f_675310`(`LoadLibraryW("oggio")`)の直接 call 鎖があり(実測)、Vorbis タグ(0x674f)の枝が oggio に行くと読む(どの枝かは未確認) |
| .AIF/.AIFF/.SND | 7 | `f_a62cb0` | `SmpLoader_AIFF` |
| .WV | 9 | `f_a64940` | `SmpLoader_WavPack` → `WavpackDLL_x64.dll`(`f_a58ba0`) |
| **.MP3** | 6 | **`f_a5ee30`**("Decoding") | **`SmpLoader_MP3`: `msacm32.dll!acmDriverOpen`/`acmDriverClose` を直接呼び、`f_674380` → `f_673f40` → `f_6729d0` → `acmStreamOpen`。到達する msacm32 は 12 名**(`acmDriverClose/Enum/Open`、`acmFormatSuggest`、`acmMetrics`、`acmStreamClose/Convert/Message/Open/PrepareHeader/Size/UnprepareHeader`)。レジストリ(`RegOpenKeyExW`)も読む |
| .OGG | 8 | `f_675660`("Decoding") | oggio(`OggioLoadFile(W)`) |
| .FLAC | 10 | `f_675660` | .OGG と同じ関数(oggio のローダー `f_675310` に到達)。EasyFLAC を使う枝があるかは未確認(このプロジェクトに FLAC は無い) |
| .M4A | 11 | `f_a61250`("Decoding") | `SmpLoader_Ffmpeg` → `f_66d230`(`ffmpeg\x64\ilvideo_x64.dll`)→ ffmpeg の DLL 群(avcodec-58 47 MB ほか 7 本 75 MB) |
| .RX2/.REX/.RCY | 5 | `f_a607b0`("Importing") | REX |
| .XI | 1 | `f_a64da0` | |

末尾の失敗時の経路(`f_a65690` の 0xa65940 以降)は `f_a57670` → `f_675660` → `f_a61250` を順に試す形で、**MP3 の ACM が使えないときの代替が ffmpeg に落ちるかは未確認**。

### 3.3 このプロジェクトで必要な復号の部品(未 lift の要否)

| 部品 | 要るファイル | 状態 |
|---|---|---|
| エンジン内蔵の WAV 読み(PCM 8/16/24、float32、mono/stereo、22.05/44.1/48 kHz) | 43 − 1(0x674f)= 42 | lift 済み(FLEngine の中)。Sampler.flp/Basic.flp では PCM16 だけ実走。**24 ビット・float・8 ビットは未実走** |
| **`oggio_x64.dll`**(1,872,384 B) | 2(0x674f の WAV 1 + .ogg 1) | **未 lift・未実走**(g8-project-load §5)。必要 |
| **ACM の MP3 復号**(msacm32 + Windows の MPEG Layer-3 ACM コーデック) | 4(音声の 74%) | **未実装**。ランタイムに `acmMetrics` の行だけ。ACM の `AcmDriver*`/`AcmStream*` と MP3 デコーダ(ホスト側)が要る |
| `wavpackdll_x64.dll`、`EasyFLAC`、`ildsp`(ildsp は復号ではない)、`LAMEenc` | 0 | このプロジェクトでは不要(書き出しは G10) |
| `elastique_x64.dll`(4,997,632 B、時間伸縮) | 0〜10(mode ≠ 0 の音声チャンネル 10 個。mode −1 の扱いは未確認) | 未 lift |
| `DelZip64.dll`(362,808 B、`.zip` の展開) | zip を FL に開かせる場合 | 未 lift。アプリ側で展開して FLP と音声を仮想ボリュームに置く案もある(その場合 R01 のサンプルパス問題) |

## 4. 使うプラグインの特定

### 4.1 ネイティブ(FL 同梱。`tables/plugins.tsv`)

| 内部名 | 個数 | DLL(インストーラ内) | 無圧縮 B | lift |
|---|---:|---|---:|---|
| Soundgoodizer | 12 | `Plugins\Fruity\Effects\Soundgoodizer\Soundgoodizer_x64.dll` | 6,407,680 | 未 |
| Fruity Parametric EQ 2 | 8 | `…\Fruity Parametric EQ 2\Fruity Parametric EQ 2_x64.dll` | 4,036,608 | 未 |
| Fruity Delay 2 | 1 | `…\Fruity Delay 2\Fruity Delay 2_x64.dll` | 3,731,968 | 未 |
| Fruity Balance | 1 | `…\Fruity Balance\Fruity Balance_x64.dll` | 3,718,656 | 未 |
| Fruity Filter | 1 | `…\Fruity Filter\Fruity Filter_x64.dll` | 3,780,608 | 未 |
| Frequency Shifter | 1 | `…\Frequency Shifter\Frequency Shifter_x64.dll` | 3,876,864 | 未 |
| FLEX(生成器) | 1 | `Plugins\Fruity\Generators\FLEX\FLEX_x64.dll` | 5,194,752 | 未 |
| 合計 | 25 | 7 DLL | 30,747,136 | 0 個 lift 済み(Delphi 製。FLEX は音色のデータの置き場も未確認) |

出典: DLL 名・サイズは `docs/investigations/g8-project-load/plugins.tsv`(インストーラ内の実測)。lift の状態は `m2-inventory.md`(lift 済みは FLEngine、dsp_ippv2、Blend2D、QFC、ILTools、freetype)。読み込みは `f_d8cab0`(`<名前>_x64.dll` + `CreatePlugInstance`。呼び元は `TPluginFormatFL` の仮想メソッドで直接の経路は無い)。

### 4.2 サードパーティ(`tables/plugins.tsv`。形式は FLP の cid 55 と依頼者の Plugins.ini の読み)

| 製品 | 形式 | 個数 | ファイル(FLP が記録したパス) | ini のサイズ B | 状態(event 213)B/個 | 役割 |
|---|---|---:|---|---:|---:|---|
| Serum 2(Xfer) | VST3 | 6 | `C:\Program Files\Common Files\VST3\Serum2.vst3\Contents\x86_64-win\Serum2.vst3` | 39,121,408 | 18,622〜309,918 | 楽器 |
| Vital(Vital Audio) | VST3 | 1 | `…\VST3\Vital.vst3` | 15,571,240 | 267,913 | 楽器 |
| OTT(Xfer) | VST3 | 2 | `…\VST3\OTT.vst3` | 3,244,032 | 583 | エフェクト |
| kHs Filter | VST3 | 2 | `…\VST3\kiloHearts\kHs Filter.vst3` | 247,296 | 9,345 | マスターほか |
| kHs Gain / kHs Haas ×2 / kHs Pitch Shifter / kHs Tape Stop / Disperser | VST3 | 1/2/1/1/1 | `…\VST3\kiloHearts\….vst3` | 各 247,296 | 約 9.3 k | エフェクト |
| Spectral Gate(Stepwise) | VST3 | 1 | `…\VST3\Spectral Gate.vst3\Contents\x86_64-win\…` | 5,261,824 | 9,263 | マスター |
| Ozone Imager 2(iZotope) | VST3 | 2 | `…\VST3\iZotope\Ozone Imager 2.vst3` | 2,194,432 | 1,811 | マスター、記録 45 |
| GClip(GVST) | **VST2** | 1 | `C:\Program Files\Steinberg\VstPlugins\GClip.dll` | 843,776 | 752 | マスター |

- 12 ファイルの ini サイズの合計 67,720,488 B。20 個の VST3 すべてで FLP の UID から作った GUID が ini の GUID と一致し、VST2 はファイル名で一致(`ini_section` 列)。VST3 は Windows では `*.vst3`(単一の PE DLL、または `Contents\x86_64-win\` の中の DLL)で、ini は 64 ビット(`ps_file_bitsize` = 64、`ps_file_arch` = Intel)。つまり lift の対象になるなら x86-64 の PE(**読み**。バイナリは見ていない)。
- 同梱の x64 以外(32 ビット)のプラグインは無い(全部 64 ビット)。
- ini の GUID/VerifiedIDs は依頼者の PC のプラグインデータベースで、FL がプラグインを識別する材料。FLP は cid 55 にフルパスも持つ。

### 4.3 プラグインが見つからないときの FL の処理(文字列の実測。実行はしていない)

FLP の読み込み(`f_f51fd0` → `f_f46490`)から `f_da11b0` と `f_12b8890` が呼ばれ、次の文字列を出す(`Warning|` で始まる通知): 「%d plugin(s) could not be opened」「This is not a serious error, the project will continue to load normally and the plugin(s) will be ignored.」「Missing plugin: "%s"%s」「The plugin |%s| is missing. Please make sure it was installed correctly.」。RTTI に `TMISSINGPLUGINSFORM`、ヘルプ `FLStudio_MissingPluginsHelp`。つまり**プラグインが無くても読み込みは続く(プラグインは無視される)が、警告の窓(モーダルかは未確認)が出る**。このプロジェクトでは、プラグインを供給しなければ 21 件(サードパーティ)+ 供給しないネイティブ分が対象になる。無視されたインスタンスのチャンネルは鳴らない = 「このプロジェクトの音」ではない。

### 4.4 サードパーティを lift の対象にするときの依存(読み。インストーラは展開していない)

- **インストーラの種類(先頭バイトとサイズだけ)**: `Install_Xfer_Serum2_2.1.5.exe` = PE(`MZ`)、1,313,084,056 B。`Xfer Serum2 v2.1.5.pkg` = xar(mac)。`Install_Xfer_OTT_137.exe` = PE、8,397,928 B。`VitalInstaller.exe` = PE、41,808,960 B。`Kilohearts Offline Installer.zip` = zip、3,833,108,300 B。`Fresh Air.Win.zip`、`Native Instruments Massive X v1.7.1.exe`(PE、1,870,391,792 B)は**このプロジェクトでは使わない**(FLP に無い)。`OTT.fst` = FL の状態ファイル(`FLhd`、323 B。同じ FLP 形式。OTT の UID が FLP のものと同じ)。インストーラの中身(プラグイン本体のサイズ、データ)は見ていない。インストーラが 1.3 GB なのは音色・波形などのデータと読むが未確認。
- **Serum 2**: 動作条件の断片(websearch の結果。第三者・メーカーの発言の引用で未検証): 「A 64-bit x86 CPU with AVX2」(monosounds.studio の Serum 2 Linux ベータ記事)、「According to the developer, it requires a 64-bit x86 CPU with AVX2 support」(synthanatomy.com 2026-08)、Xfer のフォーラムで「Serum 2.0.3 will restore performance controls and fix the OpenGL/CPU issue」(GUI が OpenGL を使うことを示唆)。→ **AVX2 の命令が本体にある可能性が高い**(読み。決定 0009/0075 は CPUID を SSE2 までに固定していて、AVX2 の経路が生成されないか、別途の対応が要る)。GPU/OpenGL の GUI、ライセンスの確認(オンラインか否か)は**未確認**。
- **Vital**: 15.6 MB の VST3。GUI の描画方式・命令セットは**未確認**。
- **OTT、Kilohearts の各種(247 KB の薄い VST3)、Spectral Gate、Ozone Imager 2、GClip**: 小さい。薄い kHs の VST3 が実体を別の場所の共有エンジンから読むかは**未確認**(ini のサイズは 247,296 B)。Ozone Imager 2 の iZotope のライセンス確認は**未確認**。
- 共通: これらは Image-Line の資料でも本リポジトリの資料でもなく、第三者の商用バイナリで、**再配布・変換の条件は未確認**。ブラウザに載せるなら、依頼者の PC からの取り込み(依頼者のデータ)に限るなど、方針の決定が要る。

## 5. 性能の予算の入力(カウントと、読める重さだけ。時間の見積りは無い)

### 5.1 (a) プロジェクトを開くときにエンジンがすること

| 作業 | 量(実測) | 重さの読み |
|---|---|---|
| FLP の解析 | 7,021 イベント、1.3 MB | イベントごとの処理 `f_f46490`(43 KB の関数)。1 イベントの重さは未測定。プラグイン状態の読みは各プラグインに渡すだけ(46 個、計 818 KB) |
| サンプルの読み込み | 48 ファイル、701 秒、246 MB(float32 と読む) | **mp3 4 本の復号(519 秒)が支配的と読む**(ACM モデルとホスト側のデコーダの速度次第。デコーダが実時間の 100 倍以上で動くなら 5.2 秒以内 〔推測。前提: コンパイル済みの MP3 デコーダの一般的な速度、wasm の遅さは含めない〕)。WAV の取り込みは 16/24 ビット → float の変換と、ピーク(波形)の作成 |
| プラグインの生成 | ネイティブ 25(DLL 7 種を `LoadLibraryW`)、VST3 20(DLL 11 種)、VST2 1 | 各プラグインの DLL のロードと初期化。Serum 2 ×6 は 39 MB の DLL を 1 回ロードして 6 インスタンスを作る形(読み)。状態の復元(`setState`)は Serum 2 で zstd 展開後 17.9 KB〜454 KB |
| ミキサー・チャンネルラック・プレイリストの構築 | 127 インサート × 10 スロット、70 チャンネル、3,565 クリップ、4,065 のミキサーパラメータ | UI の部品の生成と描画。未測定 |
| zip の展開 | 62.7 MB | FL に開かせるなら `DelZip64`(未 lift)、アプリ側なら JS |
| 起動時のエンジンの初期化(参考) | first idle まで 37〜40 秒(`performance-targets.md` §4) | プロジェクトを開く時間はこれに加算される |

### 5.2 (b) Song Mode の再生で 1 ブロック(既定 4096 フレーム = 44.1 kHz で 92.9 ms)に要る計算の量

| 量 | ピーク | 平均 | 備考 |
|---|---:|---:|---|
| 1 曲のブロック数 | 1,395(129.6 s) | | |
| 鳴っている音声クリップ | 10 | 6.25 | 時間伸縮 mode ≠ 0 は 10 チャンネル(どのクリップが同時かは未算出) |
| 鳴っている楽器インスタンス(ノート中) | 5(Serum 2 が最大 4 + FLEX 1) | 2.77 | 尾は含めない。Serum 2 が 4 つ同時 31.2 秒、3 つ 36.0 秒、2 つ 46.1 秒、0 個 16.3 秒。Vital は 1.4 秒だけ |
| 同時ノート | 11 | 5.21 | |
| 有効なインサート | 16 | 8.81 | |
| サードパーティのエフェクト | 11 | 7.16 | マスターの 5 個が下限(何かが鳴る間) |
| ネイティブのエフェクト | 12 | 6.85 | マスターの Frequency Shifter が下限 |
| 全プラグインのインスタンス(上限。スマート無効化が無い場合) | 46 | | |

- **エンジン側の既知の基準値**(他の調査の実測。私は再測定していない): 空のプロジェクトでミキサーの 1 ブロック平均 6.5 ms/92.9 ms(決定 0139、native)。native -O2 の再生は実時間の約 1/20(決定 0144)。決定 0147 の報告は 6.8〜27 倍の比(PM 未再測定)。wasm は未測定。したがって**このプロジェクトの 1 ブロックの計算時間は不明**。上の「量」(ネイティブのエフェクト 12、サードパーティ 11、楽器 5 チャンネル、音声クリップ 10)の各コストが、lift 後のコードで 92.9 ms に収まるか(P3)が問題になる。
- P2 の 120 fps は 1 フレーム 8.3 ms 以内で、1 ブロック(92.9 ms)あたり約 11 フレーム。1 つのスレッドで UI と音を回す設計(決定 0147 の仮想時計)では、UI のフレームが予算ぎりぎりだと音の計算に残る時間がほぼ無い。両者の CPU の取り合いは**未測定**。

### 5.3 (c) P2(UI 120 fps)に効く物

- プレイリスト: 使用 30 トラック、クリップ 3,565(波形を持つ音声クリップ 3,089)。表示範囲のクリップだけ描く場合の数は表示のズームに依る(**未確認**)。再生中のカーソルの移動、波形の描画。
- ミキサー: 127 ストリップ(うち使用 48)、各ストリップのピークメーター(ステレオ)。同時に見えるストリップ数は画面幅に依る(**未確認**)。
- チャンネルラック: 70 チャンネル。ステップ列。
- プラグインの GUI: Serum 2、Vital(OpenGL を使う可能性。§4.4)。ネイティブのプラグインの GUI。
- 描画の経路は Blend2D + dsp_ippv2。空のプロジェクトでも 0.3〜2 fps(`performance-targets.md`)なので、プロジェクトを足した影響は**測れない状態**。

### 5.4 (d) 依頼者の観察『デモプロジェクトを開くまでに 10 秒以上』

- 事実: アプリ(`app/src`)に FLP を開く経路は**無い**(`grep -rn -i "openProject\|\.flp\|demo" app/src app/worker app/shared` が 1 件だけで、それは `.flpack` の MIME 設定。`fle_app_put_file`/`fle_app_open`/`WM_COPYDATA` は 0 件)。ブラウザで動くのは、エンジンが起動時に自分で `Data\System\Empty.flp` を開く経路だけ(決定 0144 の入口 A/B はランタイム側にあるが、アプリには未接続と読む)。したがって依頼者の言う『デモを開く』が何を指すかは**未確認**(起動後のウィザード→新規? 別の画面?)。
- 推測しないこと: 10 秒が FLP の解析なのか、起動の最後の部分なのか、UI の更新の遅さなのか。測定していない。

## 6. エンジンの機能との突き合わせ(`tables/requirements.tsv` に全 17 行)

| ID | 必要なこと | 現状(出典) | G | リスク |
|---|---|---|---|---|
| R08 | VST3 ホスト + 20 インスタンス(11 製品) | 無い | G11 | **最大** |
| R05 | MP3 4 本の ACM 復号 | `acmMetrics` の行だけ | G8(新規) | H |
| R12/R13 | 127 インサートのミキサー、Song Mode の再生、プラグインへのノート | Sampler.flp が native で非ゼロ、実時間の約 1/20(0144)。Song Mode は未観測 | G7/G8 | H |
| R14 | ブラウザの音声出力 | 未接続 | G7 | H |
| R16 | このプロジェクトの UI 描画(プレイリスト・ミキサー) | 空で 0.3〜2 fps | G4〜G6 | H |
| R07 | ネイティブのプラグイン 7 DLL(30.7 MB) | 未 lift | G9 | H |
| R04 | oggio(Vorbis 2 本) | 未 lift | G8d | M-H |
| R10 | プラグインの識別(Plugins.ini)と、見つからないプラグインの扱い | 未モデル | G11 | M-H(モーダルが出るなら) |
| R15 | メモリ: デコード後 246 MB + 生ファイルの二重保持 62 MB + プラグイン | wasm の線形メモリは空で 930 MiB | G8 | M-H |
| R01 | zip の扱い、サンプルの絶対パス(48 個すべて存在しないフォルダー) | 未観測、アプリに経路無し | G8/G10 | M |
| R02 | FL 26.1 の 1.3 MB の FLP の解析 | 同梱の小さな FLP(FL 20〜24)だけ観測 | G8 | M |
| R06 | 音声クリップのリサンプル/時間伸縮(elastique 未 lift) | 未観測 | G8/G9 | M |
| R11 | オートメーション 13 クリップ(プラグインのパラメータ) | 未観測 | G8/G11 | M |
| R09 | VST2(GClip) | 無い | G11 | M |
| R17 | ライセンス(Serum 2、iZotope ほか) | 未調査 | G11 | M |
| R03 | 内蔵の WAV 読み(24 ビット、float) | PCM16 だけ実走 | G8 | L |

**G 段階表への当て方**: このプロジェクトを「開いて再生する」には G7(音)、G8(プロジェクト)に加えて **G9(内蔵プラグイン 7 DLL)と G11(外部プラグイン 11 製品、ライセンス)が前提**になる。`performance-targets.md` §3 の『Serum 2 を使っているなら G9/G11 が P3 の前提』は**確定した**(§0.2)。G8a〜c(決定 0144)で native で開けた `Basic.flp`、開いて再生できた `Sampler.flp` との差は、(1) FLP が 16〜27 倍大きい(81,831 B と 49,457 B に対し 1,322,619 B)、(2) プラグインが 0 個 → 46 個、(3) サンプルが PCM16 のみ → 24 ビット・float・MP3・Vorbis、(4) パターン 1 本/ノート 7 → 7 本/425 ノート/プレイリスト 3,565、(5) 再生が数秒 → 129.6 秒。

## 7. 確認できなかったこと

- 実エンジン・FL の実行はしていない(静的解析のみ)。見つからないプラグイン/サンプルの処理は文字列まで読んだ(§2.8、§4.3)が、実際にどの枝に入るか、zip を開くときの挙動、`f_f46490` が event 172 を 3 バイトとして読むか、サンプルを float32 で保持するか、mode −1 が elastique を使うか、未割り当て(値 −1)の音声クリップの出力先、オートメーションのリンク先、スロット無効化 2 件の意味、パターンの実際の長さ、「Song Mode」かどうかの再生モードのイベント。
- 同時数の掃引は、パターンの長さを『ノートの終端を 1 小節に切り上げ』、クリップがパターンより長いときは繰り返す、と置いた(読み)。プラグインの尾・残響、スマート無効化、VST の遅延補正は入れていない。
- インストーラの中身(プラグイン本体の CPU 命令、依存、ライセンスの仕組み)は見ていない(依頼どおり)。Serum 2 の条件は第三者の発言の断片だけ。
- 『デモプロジェクトを開くまでに 10 秒以上』の指すものと、その時間の内訳。
- `FLP の event 172` の意味、プレイリスト項目の後ろの 56 B の意味。

## 8. 規則違反・訂正の自己申告

- 存在しない `bash` ツールを 1 回呼んだ(内容は `echo test` のみ。ハーネスが shell に読み替えて実行した。副作用なし)。
- report 1(02:20)で「約 20 分時点」と書いたが、開始(02:06:46)からの実経過は約 14 分だった。また「本ファイルが serina1 の Serum→Serum 2 変換版に当たることを確認」と書いたが、**確認ではなく読み**(プラグイン構成と名前の一致だけ)。本書は「読み」に直している。
- 指示の『`docs/investigations/reference-project/` に置く』と『`lift/out-git/proj1-results/tools/` に置く』が食い違っていたので、後者に置いた(`docs/` の tracked ファイルは編集していない)。
- `git` の変更系操作なし。`rm` は自分の物だけ(終了時の掃除は §9)。sleep/timeout/nohup/disown/systemd-run/末尾 & を書いていない。`/tmp` 直下に書いていない(`/tmp/opencode/proj1/` のみ)。インストーラの展開・実行なし(先頭バイトとサイズの識別、`FL_STUDIO_.Plugins.ini`/`FL_STUDIO_VerifiedIDs.nfo`/`OTT.fst` の読み取りのみ)。資格情報・ブラウザのプロファイルに触れていない。製品の機械語の引用なし(エンジンの逆アセンブルは関数名・アドレス・API 名の要約のみ)。
- 取得した FLP と音声ファイルは成果物に入れていない(依頼者のデータ。`lift/out-proj1/x/` に展開物が残る。git には入れない)。

## 9. 成果物

`lift/out-git/proj1-results/`:

- `reference-project.md`(本書)、`README.txt`
- `tools/`: `flp_model.py`(パーサーとモデル)、`flp_report.py`(表の生成)、`audio_probe.py`(音声のヘッダ解析)、`selfcheck.py`(不変条件)、`budget.py`(予算の入力)
- `tables/`: `channels.tsv`、`plugins.tsv`、`mixer.tsv`、`patterns.tsv`、`playlist_items.tsv`(3,565 行)、`playlist_tracks.tsv`、`automation_clips.tsv`、`audio_files.tsv`、`timeline_intervals.tsv`(3,901 区間)、`event_bytes.tsv`、`budget_inputs.tsv`、`requirements.tsv`(17 行)、`summary.json`、`selfcheck.txt`
- 再現: `PYTHONDONTWRITEBYTECODE=1 python3 tools/flp_report.py FLP ZIPDIR tables ~/ダウンロード/FL_STUDIO_.Plugins.ini docs/investigations/g8-project-load/plugins.tsv` → `tools/audio_probe.py ZIPDIR tables/audio_files.tsv tables/channels.tsv` → `tools/budget.py tables` → `tools/selfcheck.py FLP`。
- 展開物 `lift/out-proj1/x/`(62.7 MB。git に入れない)は残してある。`/tmp/opencode/proj1/`(取得した公開資料、FLEngine の索引 pickle、逆アセンブルの一時出力。4.5 MB)は終了時に `rm -rf` で削除済み(`ls /tmp/opencode` に残っていない)。

## 10. 次に確かめるとよい物(意見。決めるのは PM)

1. **native の実エンジンにこの FLP を開かせる**(入口 A = 起動時のコマンドライン、決定 0144 の方式)。48 個の音声は FLP が記録した絶対パスに置き、サードパーティのプラグインは供給しない。見るもの: `Warning|` の通知(サンプル/プラグイン)、`f_f46490` が完走するか、壁の数、読み込みの時間。**プラグインが無視された状態なので、所定のプロジェクトの音ではない**(結果にそう書く)。これで『プロジェクトを開く 10 秒以上』の内訳に近づける。
2. **MP3 の ACM 呼び出しの記録**: `f_a5ee30` が `acmDriverEnum`/`acmFormatSuggest`/`acmStreamOpen` を呼ぶ順序と引数を、native の実行(lift した FLEngine。gdb またはランタイムの記録)で取り、ACM の最小モデルの仕様にする(`f_a65690` の振り分けは拡張子だけなので、`Sleigh_Bells…mp3`〔93.7 秒〕1 本で足りる)。
3. **サードパーティ VST3 の方針を依頼者に確認する**: (a) 依頼者の PC の DLL を lift の対象にする(Serum 2 は AVX2 の可能性、OpenGL の GUI、ライセンス、再配布の条件)、(b) 置き換える/無視する(P3 の対象が『内蔵のみの縮小版』になる = 所定のプロジェクトではない)、(c) ゴールの字義(P2/P3 は所定のプロジェクト)を維持して G11 を優先に上げる。本書の数字はどれを選んでもそのまま使える。
