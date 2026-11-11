# 0139: 音声レーン(G7/G8): S0 実行観察・S1 DirectSound モデル・S2 ブラウザ(AudioWorklet)の試作

状態: **仮の決定**。PM の決定 0139(音声レーンの S0 = 実行観察、S1 = DirectSound のモデル、S2 = ブラウザの AudioWorklet の試作。出典なしの選択は名前・値・理由・撤回条件を列挙、`fe-lift` / `fe-decode` は変更しない、`app/` は読み取り専用)を、作業者(audio1)が調査・設計・実装・測定した(2026-11-01 20:00Z ごろ〜、PM の指示で基準を ce16052 から **fa11947** に移した後、再ベースして検証)。仮の決定にする理由: 実エンジンの音声の動作を**実機の Windows と照合した例は 0 件**(Learn に書かれない出典なしの選択が 47 名)、S2 の実エンジン側の測定は**再ベース前の木(ce16052 + 差分)で取った 1 構成(Chrome 156 headless、wasm `-Oz`)だけ**、実際の音声デバイスへの出力は聴いていない(headless)。

## PM の判断(途中報告への返信。この版に反映済み)

1. 出力のリード(write − play)の**既定は 60 ms**。調整できるようにし、出典なしの選択 **S-audio-lead** として列挙する(下の表)。**実装した**: `FLE_AUDIO_LEAD_MS` の既定を 15 → 60、環境変数 `FLE_AUDIO_LEAD_MS`(native)とページの `leadMs`(wasm)で変えられる(1〜1000 ms、範囲外は既定)。
2. **G7 の字義「実エンジンのブロックで連続アンダーラン 0」は未達**(実エンジンは 98% がアンダーラン。C のポーリングミキサーでは 0)。これは下の S2 に明記した。
3. 単一スレッド協調の制約は、**速度改善(IPP 描画などの手書き等価実装 = HLE。perf2 のホットスポット表が入力)と、API 呼び出し点での強制 yield** で解く。**ミキサーを別 Worker に出す案は今は採らない**(共有メモリの設計が大きい)。「遅いスライスの内訳と強制 yield の見積もり」と「PM の判断点」に、yield の候補点の見積もりと遅いスライスの内訳を書いた(**強制 yield だけでは足りない**ことが分かった: 仮想時計の進め方の変更が要る)。
4. 基準: 指示は「最終工程は HEAD ce16052 の木で」とあったが、提出前の `git log -1` は **fa11947**(G3 = 決定 0136 を含む)。**先の PM の指示で基準を fa11947 に移して再ベース済み**なので、最終工程(全体テスト・a4・変異検査)は fa11947 の木 + 差分で行った。応答表の 3 行は行 598〜600(G3 の 588〜597 の後)に付け替え済み。

## 先に結論(S0 / S1 / S2)

### S0: 実行観察(native の gdb、900 秒。詳細 `docs/investigations/audio-s0.md`)

| 項目 | 内容(実測) |
|---|---|
| 音声 API の呼び出し | **`DirectSoundEnumerateA` 1 回**(callback `0x12af280`、ctx 0。呼び元 `f_12af3e0` ← `f_1252bd0` ← `f_1196c00`、52.8 秒)。`DirectSoundCreate` ほかの dsound は **0 回**。`midiInGetNumDevs` 1、`midiOutGetNumDevs` 4、`midiOutGetDevCapsW` 2、`midiOutOpen` 1(MAPPER、268.9 秒、MIDI プラグイン経由)、`timeBeginPeriod` 2、`timeGetDevCaps` 2、`CoCreateInstance` 1(`ITaskbarList`。音声ではない)、`LoadLibraryW "avrt"` のみ(`Av*` 呼び出し 0)。`waveOut*`・`mixer*`・`timeSetEvent`・WASAPI・XAudio は無い。 |
| 順序 | `midi*GetNumDevs` → `f_1252bd0`(`f_c55ad0`(ASIO 列挙: `f_c526f0` が `software\asio` を開いて 0 件)→ `f_12af3e0`(`DirectSoundEnumerateA`)→ `f_12517f0`)→ `f_1252d80` → `f_13582c0` → `f_1253610` → `f_1253260`(**2 回**。`f_11d3340` 経由の 2 回目も) |
| 0 台のときのゲストの分岐 | 一覧は "(none)" だけ。`f_1253260` は何もせず 0 を返す。DS 初期化 `f_12afb50`、ミキサースレッド作成 `f_12afaf0`、`TMixerThread_DS.Execute` `f_12aeec0` は **900 秒の間に 1 回も届かない** = **ミキサーは作られない**。 |
| ゲストの設定 | ポーリング(`[0x15d3294]` = 1)、ソフトウェア(`[0x15d3295]` = 1)、float32 でない = PCM16(`[0x15d3296]` = 0)、ブロック長 4096 フレーム。audio-m5.md の「未確認」を確定した。 |

### S1: 仮想デバイス 1 台のときの次の壁とミキサーの挙動(実測。native は ce16052 + 差分、再ベース後は下の「再ベースの後の確認」)

| 項目 | 内容 |
|---|---|
| 列挙 | `DirectSoundEnumerateA/W` は callback を **2 回**呼ぶ: (NULL, "Primary Sound Driver", "") と (GUID `0F5D1E00-0001-4000-8000-464C53545544`, "Browser Audio Output", "")(S-ds-guid)。NULL の callback は `E_INVALIDARG`(0x80070057)。 |
| 実エンジンの DS 呼び出し順(native 実測、`logs/s1a/`) | `DirectSoundCreate` → `SetCooperativeLevel(hwnd 0x7f990034、PRIORITY)` → `CreateSoundBuffer`(プライマリ)→ `SetFormat(44100/16/2)` → `CreateSoundBuffer`(セカンダリ 32768 B = 2 × 4096 フレーム、flags `0x18000`)→ `SetCurrentPosition(0x4000)` → `Play(LOOPING)` → `Stop`(+6.6 ms)→ `Lock(全体)`(ゼロ)/ `Unlock` → `SetCurrentPosition` / `Play` → **`Sleep(1)` + `GetCurrentPosition` のポーリング**。 |
| ミキサー(native、仮想時計) | 16384 B の半分ずつを約 **92.4 ms** ごとに `Lock` / `Unlock`(最初は +102 ms)。ブロックの peak は 0(無音)。300 秒の間に**新しい壁は無い**(経過 300 秒、入口 62,195,738、新規関数 6,715)。 |
| wasm の次の壁 | **thread 2 の C スタック(1 MiB)の canary が破られる**(`fle: scheduler: the canary at the low end of the shadow stack of thread 2 was overwritten … deeper than 1048576 bytes`、`logs/s2a/s2d.json`)。`FLE_THREAD_WASM_CSTACK_BYTES=8388608`(`RT_EXTRA_CFLAGS`)で通る。**8 MiB を既定にするのは層状窓レーン(決定 0140)**。この差分は既定を変えない。 |
| 次の壁(その先) | 壁ではなく**実時間の不足**: S2 を見よ。 |

### S2: ブラウザ(AudioWorklet)の試作(実測。Chrome 156.0.8078.47、headless、wasm `-Oz` + SEH、再ベース前の木)

| 項目 | 内容 |
|---|---|
| `AudioContext` | 44100 Hz で `running`。ctx 時間/実時間 = **0.999**。`baseLatency` 0.0107 s、`outputLatency` 0 → 0.024〜0.032 s。 |
| **実エンジン**(60 秒測定) | ページ開始から PLAYING に入るまで約 **11.3 秒**。測定 60 秒の再生クォンタ 19,053 のうち **18,684(98.1%)がアンダーラン**(s2f。同条件の s2e は 18,676 / 19,060 = 98.0%)。仮想時計は 88 秒の実時間で **+424 ms** しか進まない(実時間の約 0.5%)。エンジンが実際に混ぜてリングへ公開できたフレームは測定終了時点まで累積で 49,285(1.1 秒分)、AudioWorklet が読んだ位置(RD)は 2,440,576(55 秒分)。 |
| なぜか(実測) | **ミキサーは軽い**: ブロック 12 個の生成が平均 **6.5 ms**・最大 **12.8 ms**(リード 15 ms の走行。ブロックの周期 92.9 ms − リード 15 ms = 77.9 ms を超えたもの 0。**既定のリード 60 ms では余裕は 92.9 − 60 = 32.9 ms**: 12.8 ms は収まる)。**ほかのスレッドのスライスが長い**: メイン(thread 0)は 88 秒のうち **69.7 秒**を 35 スライスで使い、最大 **11.3 秒**の連続、thread 2 は 30 スライスで 13.8 秒(最大 0.70 秒)。協調スレッドは先取りしないので、ミキサーは他のスレッドが譲るまで走れない。 |
| C のシナリオ `rt_mixer`(同じ DS モデル + リング + JSPI スレッド + 1:1 の実時間ペース。9 秒。再ベース後の木・**既定のリード 60 ms**、`logs/s2c-lead60default/`) | hog なし: **アンダーラン 0 / 3,098 クォンタ**、リング充填 2262〜2646 フレーム。`Unlock` から AudioWorklet が読むまで**中央値 84.3 ms**(最小 77.4、最大 92.7、n = 97)。(リード 15 ms の再ベース前の走行: 0 / 3,098、充填 277〜661、中央値 84.2 ms。) |
| ほかのスレッドの長いスライスを模したテスト(5 ブロックごとに hog。**出力側の欠落 = AudioWorklet のアンダーラン**) | **既定のリード 60 ms(再ベース後、9 秒)**: hog 60 ms **0 / 3,098 = 0%**、hog 100 ms **7.79%**(241 / 3,094)、hog 250 ms **27.79%**(861 / 3,098)。リード 15 ms(再ベース前): hog 60 ms 8.36%、100 ms 17.0%。リード 30 ms・hog 60 ms 5.0%(再ベース前)。リード 60 ms・hog 60 ms は再ベース前 0.19%、再ベース後(`logs/s2b-rebased`)0.03%。 |
| **同じテストでゲストが見るアンダーラン**(`GetCurrentPosition` の write カーソルが書いたばかりの半分に入ったとき、エンジンが数える。`logs/s2d-engine-check/`、再ベース後、60 ブロック) | hog なし: リード 60・15 ms とも **0 回**。hog 60 ms: リード 60 ms で **12 回**(hog を入れた 12 ブロックすべて)、リード 15 ms で 0 回。hog 100 ms: どちらも 12 回。**リードを長くすると、出力側の欠落は減るが、ゲストが「アンダーラン」と数える条件は厳しくなる**(ミキサーが半分の入れ替わりから 92.9 − リード ms 以内に書き終えないと、write カーソルが書いた半分に入る)。実エンジンの 1 ブロックの生成(最大 12.8 ms)は 32.9 ms に収まるが、他スレッドのスライスが 33 ms を超えると、ゲストの計数にも出る(読み: ゲストが数えたあとどうするかは `f_1253840` の優先度変更で、協調スレッドでは効かない)。 |
| 実時間の見込み(読み + 上の実測) | ミキサー自体の計算は足りる(6.5 ms 平均 / 12.8 ms 最大、既定のリード 60 ms の余裕 32.9 ms)。**足りないのは他スレッドのスライスの長さ**: 実エンジンは最大 11.3 秒。hog のテストは、スライスの長さがリード(60 ms)を超えるとアンダーランが出始め、長いほど増える(100 ms で 7.8%、250 ms で 27.8%)ことを示す。**G7 の字義「実エンジンのブロックで連続アンダーラン 0」は未達**(実エンジンは 98% がアンダーラン。同じ DS モデルを C のポーリングミキサー `rt_mixer` で実時間に走らせると 0)。 |

### 再ベース(fa11947)の後の確認(ここまでの全部は再ベースした差分を適用した木の値)

| 項目 | 内容 |
|---|---|
| 差分 | `audio1.diff` = `diff -ruN clean src` 28 ファイル(+3,713 −27)。fa11947 の `git archive` に `patch -p1`: dry-run・実適用とも終了 0、fuzz / offset / reject 0、適用後の木が `src` と一致。**提出時の HEAD は ac5a790(決定 0138 = perf2。docs・`a4-fruity.sh`・`lift/harness/web/perf2/` だけが fa11947 と違い、audio1.diff と重ならない)で、その `git archive` にも終了 0・fuzz / offset / reject 0 で当たる**(`logs/make-diff.txt`: 木の差は perf2 の分だけ)。 |
| 全体テスト | 1 回目(再ベース後の木、47 分): 136 result・**2,000 passed・1 failed**(`wall_r::the_vtable_rows_reach_the_runtime_through_the_generated_tables`: `powf` が未解決)。`wall_r.rs` の全ランタイムリンクに `-lm` を足して直し、`wall_r` 単独 31/31。**修正後の最終の木での 1 回の通し: 136 result・2,002 passed・0 failed(下の「検証」)**。基準 fa11947 は 135 result・1,984 passed(PM 通知)。 |
| FRUITY 無効 a4(独立 2 回、**最終の木 = リード 60 ms の版**: `a4offC`・`a4offD`、経過 3:13.9 / 3:30.6) | 942/942、895/895、`returned from entry`、functions 5,310、entries 1,009,721(基準と同じ)。native の SHA は 2 回で異なる(OUT のパスが `-g` で埋まる。基準の値と比べていない)。(リード 15 ms の版の `a4offA`・`a4offB` も同じ値。`final-verify.sh` の最初の a4 は OUT 名の再利用で即失敗した: 自己申告) |
| 実エンジン native(fa11947 + 差分、FRUITY + SEH、`logs/nat2/`) | a4-multi の FRUITY 有効構成(6 モジュール、`FLE_FRUITY_SECONDS=300`、`FLE_AUDIO_TRACE=1`)で生成・ビルド・実行(23:35:08Z〜約 23:46Z、経過 10:49)。**DirectSound の呼び出し列が ce16052 の走行(`logs/s1a`)と同じ**: `enumerate`(callback `12af280`)→ `create` → `SetCooperativeLevel`(hwnd `7f990034`、level 2)→ プライマリ → `SetFormat` 44100/16/2 → セカンダリ(flags `18000`、32768 B)→ `SetCurrentPosition(0x4000)` → `Play(LOOPING)` → `Stop`(cursor 5050 フレーム)→ `Lock` / `Unlock` を 9 回(最初は 32768 B、その後 16384 B ずつ。2 回目〜は 92.4 ms 間隔: クロック +116.7 ms、+209.0 ms、…)。`GetCurrentPosition` 785 回。停止 = 時間切れ 300 秒、入口 61,966,725、新規関数 6,715(s1a は 62,195,738 / 6,715)。a4 部 5,396 関数・1,052,540 入口。**新しい壁なし**。ブロックの peak は記録していない(トレースレベル 1)。native の SHA は `829fddef…`。 |
| wasm の C シナリオ `rt_mixer`(fa11947 + 差分、Chrome 156、リード 15 ms は旧既定値の走行、`logs/s2b-rebased/`) | `build-scen-wasm.sh` で再ビルド(`logs/builds/scen-wasm2-build.txt`)し、同じ条件で 9 秒測定: リード 15 ms・hog なし **アンダーラン 0 / 3,099 クォンタ**(`Unlock` → worklet の読みは中央値 84.0 ms、最小 76.2、最大 92.4)、リード 15 ms・hog 60 ms(5 ブロックごと)**8.52%**(264 / 3,098。再ベース前 8.36%)、リード 60 ms・hog 60 ms **0.03%**(1 / 3,099。再ベース前 0.19%)。C のシナリオだけで、**実エンジンの wasm は再ベース後に取り直していない**。 |
| gencmp | 既定の引数: FLEngine の生成 C のうち本体が変わるのは **1 関数**(`f_9824e0` = `DirectSoundCreate` の import thunk。行が「応答なし」から「応答あり」に変わった)。`--seh-handler 0x112c0` の比較も同じ 1 関数。dsp_ippv2・Blend2D・QFC・FreeType・ILTools は 0。 |
| 変異検査 | **20 件中 20 件検出**(最終の木 = リード 60 ms の版、`logs/mutations-final2.txt`。M09 は時間切れ = 走行が止まって検出)。リード 15 ms の版も 20/20(`logs/mutations-final.txt`)。最初の 1 回で M01(パターンが見つからない)と M10(生き残り)があり、テストを強め/直してやり直した(「不備の自己申告」)。 |

## PM の決定(決定 0139。作業者が受け取った指示の要点。変えていない)

| # | 項目 | 内容 |
|---|---|---|
| 1 | S0 | 実エンジンの音声 API の呼び出しを**実行で**観察する(名前・回数・順・呼び元)。デバイス 0 台でのゲストの分岐を確かめる。 |
| 2 | S1 | DirectSound の仮想デバイス 1 台のモデル。次の壁とミキサーの挙動。ASIO は種付けしない。 |
| 3 | S2 | ブラウザの AudioWorklet に実エンジンのブロックを渡す試作。受け取ったブロック数・アンダーラン・遅延・実時間の見込み。 |
| 4 | 範囲 | `fe-lift` / `fe-decode` は変更しない(新規ファイル優先)。`app/` は読み取り専用で、載せる口を文書にする。`imports_table.rs` は末尾追記のみ。 |
| 5 | 出典 | Learn に書かれない値は出典なしの選択として名前・値・理由・撤回条件を列挙。 |
| 6 | 基準 | 作業の途中で HEAD が ce16052 → fa11947 に移った(G3 = 決定 0136 が行 588〜597 を追記)。再ベースして衝突を記録。`fle_thread.c` が G3 のアイドルフックと重なるなら G3 を優先。 |

## 設計: S1 のモデル(`runtime/fle_dsound.c`、`fle_audio_ring.c`)

- **仮想デバイスは 1 台**。`DirectSoundEnumerateA/W` は callback を 2 回呼ぶ(既定デバイス = NULL GUID、次に GUID つきの同じデバイス。Learn の DSEnumCallback。callback が FALSE を返したら 2 回目は呼ばない)。`DirectSoundCreate` / `DirectSoundCreate8` は `IDirectSound` / `IDirectSound8` を返す(GUID は NULL・`DSDEVID_DefaultPlayback`・仮想デバイスの GUID のみ。ほかは `DSERR_NODRIVER`)。
- **COM オブジェクト**: ホストの確保領域の先頭 8 バイトが vtable。vtable のスロットは **擬似 VA**(`0x7FF940000000 + 0x10 × メソッド番号`、S-com-slot-range)で、ゲストの `call [reg+disp]` が `fle_dispatch_miss` → `fle_win_try_thunk` → **`fle_com_try_slot`** に来て C の関数を呼ぶ。応答表の行は **3 つだけ**(`DirectSoundCreate`・`DirectSoundCreate8`・`DirectSoundEnumerateW`。行 598〜600)。`fle_dispatch.h` は変更せず、生成 C は触れない(gencmp)。モデルしないメソッドは**名前つきで停止**する("IDirectSoundBuffer::SetFX is not modeled …")。
- **バッファ**: セカンダリは `calloc` したバイト列(円形)。`Lock` はその領域を直接ゲストに渡す(1〜2 領域)、`Unlock` は検査して確定。**プライマリ**はデバイスの形式だけを持つ(データなし)。
- **デバイスの動作**: 再生中のバッファを play カーソルの位置から読み、(音量 `10^(vol/2000)`、パン)をかけて足し合わせ、float32 のステレオにして**リング**(`fle_audio_ring`、32768 フレーム = 0.74 秒)へ、**消費済みのフレーム + リード(既定 60 ms = S-audio-lead)**まで書く。write カーソルはこのリード分だけ play カーソルより先(`GetCurrentPosition` の説明: 「通常は約 15 ミリ秒分」)。
- **再生位置の源は 1 本**: `fle_audio_consumed()`。native = 仮想時計(最初の `Play` から `(t − t0) × rate / 10^7`)、wasm = AudioWorklet が読んだフレーム数(SAB の RD)。2 つの違いはこの関数だけ。
- **通知**(`IDirectSoundNotify`): play カーソルが指定のオフセットを**越えた**ときイベントを立てる(`fle_event_host_set`)。スケジューラが `fle_audio_poll` を呼び、`fle_audio_next_tick` で「次の通知の時刻」を期限に足すので、通知を待つスレッドは仮想時計で起こされる。ポーリング(実エンジンの設定)は `GetCurrentPosition` が毎回仮想時計を `FLE_CLOCK_READ_TICKS` 進める(S-ds-pos-read-tick)ことで終わる。
- **wasm の運び方**: Worker が持つ線形メモリは SAB でない(共有できない)ので、AudioWorklet はリングを直接読めない。**エンジンの EM_JS が C のリングから SAB のリングへ同期的にコピー**する(`fle_js_publish`。エンジンが秒単位で計算していても動く)。AudioWorklet(`audio-worklet.js`)は 128 フレームごとに RD を 128 進め(データの有無に関わらず。**出力は時間軸**)、無いフレームは無音にしてアンダーランを数える。ページは `FLE_AUDIO_SAB.make(cap, rate)` で SAB を作り、`{cmd:'audio-init', sab, cap, dataOffset, leadMs}` を Worker へ送る。
- **ペース**(`fle_audio_paced` / `fle_audio_wait_until`): 外部の消費者(AudioWorklet)があって再生中のバッファがあるとき、スケジューラは次の期限まで**実時間で待つ**(仮想時計を 1:1 で実時間に結びつける。JSPI の `EM_ASYNC_JS`)。待っている間 Worker のイベントループが回る。実エンジンが実時間に遅れているときは待たず、遅れ(lateness)を数える。

## 出典なしの選択(名前・値・理由・撤回条件)

Learn に値や規則が書かれていないもの。コードの注釈に名前があるものは `fle_dsound.c` / `fle_audio_ring.c` にも同じ名前がある。出典は取得時点(2026-11-01)の Microsoft Learn(`learn/index.tsv` に URL と SHA-256)。IID・フラグ値・WAVE_FORMAT 番号は mingw の `dsound.h` / `mmreg.h`(**非公式**)から、HRESULT の値は Learn の「DirectSound Return Values by Number」から。

### デバイスの同一性・列挙

| 名前 | 値 | 理由 | 撤回条件 |
|---|---|---|---|
| S-ds-device-strings | 説明 "Primary Sound Driver"(既定)と "Browser Audio Output"(GUID つき)、モジュール名 "" | Learn に値が無い。前者は Windows が既定デバイスに付ける説明。ゲストは説明を一覧の文字列に使うだけ(読み)| 実機の列挙で別の文字列が分かったとき、またはページがデバイス名を渡すとき |
| S-ds-guid | `0F5D1E00-0001-4000-8000-464C53545544`(末尾は ASCII "FLSTUD") | Learn は GUID の値を決めない。固定で衝突しないもの | ブラウザの出力デバイス(`sinkId`)の識別と対応させるとき |
| S-ds-fail-null | 失敗時に `*ppDS = NULL` | COM の出力ポインタの慣習。Learn の `DirectSoundCreate` は書かない | 実機で失敗時に値が残ると分かったとき |
| S-ds-noiface | 未知の IID の `QueryInterface` は `E_NOINTERFACE` 0x80004002 | Learn の DS の表は `DSERR_NOINTERFACE` を 0x000001AE と書くが、HRESULT でない | 実機の戻りが違うと分かったとき |
| S-ds-caps | `GetCaps`: PRIMARYMONO/STEREO/8BIT/16BIT/CONTINUOUSRATE、最小 100・最大 200000 Hz、プライマリバッファ 1(Learn)、ほかは 0 | ページはメンバーとフラグ名を挙げるが、デバイスの値は書かない。ソフトウェアミキサーとして | 実エンジンが `GetCaps` を呼び、別の値を要求したとき(今は呼ばない) |
| S-ds-hwnd | `SetCooperativeLevel` の hwnd は仮想マシンの窓表にある生きた窓。無ければ `E_INVALIDARG` | ページは「アプリケーションの窓のハンドル」としか書かない | 実機で別の振る舞いが分かったとき |
| S-ds-coop-locked | ロック中のバッファがあるとき `SetCooperativeLevel` は `DSERR_INVALIDCALL` | Learn: 「バッファがロックされていたら呼ぶな」(戻りは書かない) | 同上 |
| S-ds-primary-default | `SetFormat` 前のデバイス形式は 44100 Hz・16 ビット・ステレオ PCM | Learn は NORMAL の出力を「既定の 8 ビット形式」と書くだけ | 実機の既定が分かったとき(実エンジンは必ず `SetFormat` する) |
| S-ds-formats | PCM 8/16 ビット、IEEE float 32 ビット、1〜2 ch、100〜200000 Hz。ほかは `DSERR_BADFORMAT`(WAVEFORMATEXTENSIBLE は SubFormat で PCM/float に畳む) | ページは対応形式を決めない | 実エンジンが別の形式を使うとき(今は 16 ビット PCM。float32 は設定で出るが未実行) |
| S-ds-avgbytes | `nAvgBytesPerSec` を検査しない | Learn: 「rate × block であるべき」(should) | 実機が検査すると分かったとき |
| S-ds-size-multiple | バッファのバイト数は `nBlockAlign` の倍数で `DSBSIZE_MIN`(4)以上。違えば `E_INVALIDARG` | ページは範囲(`DSBSIZE_MIN` / `DSBSIZE_MAX`)を挙げ、倍数性は WAVEFORMATEX の読みから | 同上 |
| S-ds-hw | `DSBCAPS_LOCHARDWARE` は `DSERR_INVALIDCALL` | Learn: ハード加速が無いと `CONTROLUNAVAIL` か `INVALIDCALL` のどちらか | 実エンジンが LOCHARDWARE を要求し、別の戻りを期待するとき(実エンジンは付けない: soft=1) |
| S-ds-guid3d-unread | `guid3DAlgorithm` を読まない | Learn: CTRL3D なしなら GUID_NULL でなければならないが、FLEngine はスタックの残りをそのまま渡す(audio-m5.md §3.3) | 実機が検査して失敗すると分かったとき |

### バッファの状態

| 名前 | 値 | 理由 | 撤回条件 |
|---|---|---|---|
| S-ds-buffer-init | 新しいバッファの内容は 0 | Learn: 「DirectSound は内容を初期化しない」 | 実機が別の値を返すと分かったとき(無音以外を前提にしたゲストが出たとき) |
| S-ds-primary-cursors | プライマリバッファの play / write は 0 | データを持たない | 実エンジンがプライマリの位置を読むとき(今は読まない) |
| S-ds-stopped-write | 停止中のバッファでは write カーソル = play カーソル | 何もコミットしていない | 実機が停止中にもリード分を返すと分かったとき |
| S-ds-pos-read-tick | `GetCurrentPosition` は仮想時計を `FLE_CLOCK_READ_TICKS` 進める | 位置だけを読むループを終わらせるため(決定 0017 の時計の読みと同じ) | 時計の読みの規則を変えるとき |
| S-ds-lock-range | オフセットが末尾以降・0 バイト・バッファより大きいは `E_INVALIDARG` | Learn: 「バッファは概念上円形」でバイト数は末尾までの距離を超えてよいが、範囲外の戻りは書かない | 同上 |
| S-ds-lock-twice | ロック中の `Lock` は `DSERR_INVALIDCALL` | Learn は書かない(戻り値の一覧に INVALIDCALL はある) | 実機で別と分かったとき |
| S-ds-primary-lock | プライマリの `Lock` は `DSERR_INVALIDCALL` | WRITEPRIMARY(未モデル)だけがプライマリをロックできる | WRITEPRIMARY を作るとき |
| S-ds-unlock-check | `Unlock` のポインタ・バイト数が `Lock` と違う(多い)は `E_INVALIDARG` | Learn: 「実際に書いたバイト数を指定」(少ないのは可) | 同上 |
| S-ds-notify-playing | 再生中の `SetNotificationPositions` は `DSERR_INVALIDCALL` | Learn: 「停止中に呼ぶ」 | 同上 |
| S-ds-notify-start | `Play` の開始位置と同じオフセットの通知は、そのときは立てない(カーソルが越えたときだけ) | Learn は書かない | 実機の挙動が分かったとき(実エンジンはポーリングで通知を使わない) |
| S-ds-restore | `Restore` は常に `DS_OK`(バッファは失われない) | 失われる契機がない | フォーカス喪失をモデルするとき |
| S-ds-primary-call | プライマリへの `SetCurrentPosition` / `SetFrequency` は `DSERR_INVALIDCALL` | Learn が「プライマリでは呼べない」(戻りは書かない) | 同上 |
| S-ds-format-nolevel | 協調レベル未設定の `SetFormat` は `DSERR_PRIOLEVELNEEDED` | Learn: NORMAL では失敗。「未設定」は同じとみなした | 実機で別と分かったとき |
| S-ds-secondary-setformat | セカンダリへの `SetFormat` は `DSERR_INVALIDCALL` | Learn: 「セカンダリでは使えない」(戻りは書かない) | 同上 |
| S-ds-late-join | 別のバッファが再生中のときに始めたバッファは、**すでにリングへ渡したフレームの後**から混ざる | すでに出力へ渡したフレームは変えられない(リードの範囲) | 実機が write カーソル内にも混ぜると分かったとき(この文書にだけ名前がある: コードの注釈 `restart_geometry`) |

### 出力・ミキサー

| 名前 | 値 | 理由 | 撤回条件 |
|---|---|---|---|
| **S-audio-lead**(旧 S-ds-lead-ms。PM の決定) | write − play の**既定 = 60 ms**(`FLE_AUDIO_LEAD_MS` = 60。環境変数 `FLE_AUDIO_LEAD_MS`(native)とページの `leadMs`(wasm)で 1〜1000 ms に変えられ、範囲外・数字でない値は既定)。44100 Hz で 2646 フレーム | Learn(`GetCurrentPosition`)は「通常は約 15 ミリ秒分」とだけ書く(15 は選べる値として残る)。60 ms は、スレッドのスライスの揺れに耐えるため(hog 60 ms で 15 ms 8.4% → 60 ms 0%)。**代償**: ミキサーが半分の入れ替わりから 92.9 − 60 = 32.9 ms 以内に書き終えないと、ゲストの write カーソルの検査が「アンダーラン」を数える(S2 の表)。**バッファが小さいと成立しない**: リードは 1 バッファより短くなければならない(1024 フレームのブロック = 2048 フレームのバッファでは 60 ms で波形が壊れる: `rt_mixer` の検査。実エンジンの 2 × 4096 フレームでは 90 ms まで一致) | 実機の write − play の値が取れたとき、または実エンジンが 60 ms で不具合(ゲストのアンダーランの計数が出る等)を起こしたとき。`FLE_AUDIO_LEAD_MS=15` で戻せる |
| S-ds-pcm16-scale | 16 ビット PCM は ÷ 32768(−32768 が −1.0) | audio-m5.md §7.2 の 7: FLEngine は float を 32767 倍で丸める。÷32767 だと +1.0 が満尺 | 実機の係数が分かったとき |
| S-ds-pcm8-scale(この文書で命名) | 8 ビット PCM は `(x − 128) / 128` | 符号なし 8 ビットの中心 128(Learn の WAVEFORMATEX の説明に基づく読み) | 同上 |
| S-ds-mix-sum(この文書で命名) | 再生中のバッファを**足すだけ**(クリップもリミッターも掛けない。float のまま) | リング・AudioWorklet が float | 実機が整数で飽和すると分かったとき(出力が ±1 を超えうる) |
| S-ds-gain-curve(この文書で命名) | 音量 `10^(vol/2000)`(vol は 100 分の 1 dB)、パン: 反対側を `10^(−abs(pan)/2000)`、±10000 で 0 | Learn の `SetVolume` / `SetPan`(100 分の 1 dB、パン 100 dB 減衰)。**曲線は dB の素直な読み** | 同上 |
| S-ds-mono-copy(この文書で命名) | モノラルのバッファは L と R に同じ値 | 慣習 | 同上 |
| S-ds-idle-gap | デバイスが空いていた間のリングは作らず、最初の `Play` でリングの書き込み位置を消費位置まで進める | 無音の間は出力が走り続け、リングに何も無い | 同上 |
| S-ds-stop-drain | `Stop` 後の play カーソルは、**リングへ渡し済みのフレームの直後** | Learn Stop: 「最後に再生したサンプルの次」。リードの分は再生されたとみなす | 実機で違うと分かったとき |

### 未モデル(名前つきで停止)

S-ds-no-3d(`CTRL3D`)、S-ds-no-fx(`CTRLFX`)、S-ds-no-voice-manager(`LOCDEFER` / `STATIC` / `Play` の voice フラグ)、S-ds-no-writeprimary(`DSSCL_WRITEPRIMARY`)、S-ds-no-resample(デバイスと違う rate のセカンダリ、`SetFrequency`)、S-ds-primary-twice(1 台に 2 つ目のプライマリ)、S-ds-rate-change(再生中のデバイス rate の変更)。実エンジンの呼び出し列にはどれも出ない(S1 の native 実測)。

### 環境の限界・ペース

| 名前 | 値 | 理由 | 撤回条件 |
|---|---|---|---|
| S-audio-ring | リング 32768 フレーム(0.74 秒)、ブロックログ 256 | 2 の冪。ブロック 4096 フレームの約 8 倍 | 実エンジンが大きいブロックを使うとき |
| S-audio-pace | 1:1 の実時間ペース(`fle_audio_wait_until`)。外部消費者があり再生中のときだけ | 位置の源が AudioWorklet なので、仮想時計を実時間に結ぶ必要がある | 時計の別の方式(B/C の混成)を採るとき |
| S-ds-limits(この文書で命名) | デバイス 8、バッファ 32、1 バッファ 64 MiB | 環境の上限 | 実エンジンが超えたとき(今は 1 デバイス・2 バッファ) |
| S-com-slot-range | COM スロットの擬似 VA は `0x7FF940000000 + 0x10 × id` | 応答表(`0x7FF900000010 + 0x10 × k`)と GetProcAddress の擬似 VA の上に置く。窓表等の範囲と重ならない | 擬似 VA の割り当てを一本化するとき |
| (PM の選択)ASIO を種付けしない | 仮想レジストリに `software\asio` を入れない | audio-m5.md §7.2 の 4(PM 承認済み) | ILWASAPI2ASIO 等が必要になったとき |

## 実装した場所

| ファイル | 行数 | 内容 |
|---|---:|---|
| `lift/runtime/fle_dsound.c`(書き換え。0126 の「0 台」を置き換える)/ `.h` | 1,487 / 35 | DirectSound のモデル、vtable、列挙、ミキサー、通知、スケジューラのフック(`fle_audio_poll`・`fle_audio_next_tick`) |
| `lift/runtime/fle_audio_ring.c` / `.h`(新規) | 405 / 115 | リング、消費源、EM_JS の運び(`fle_js_publish` ほか)、ペース、テレメトリ |
| `lift/runtime/fle_event.c` / `.h` | +11 / +3 | `fle_event_host_set`(通知イベントを立てる) |
| `lift/runtime/fle_win.c` | +6 | `fle_win_try_thunk` の先頭に `fle_com_try_slot` |
| `lift/runtime/fle_thread.c` | +31 | 弱い参照のフック: `fle_audio_poll`・`fle_audio_next_tick`・`fle_audio_paced`・`fle_audio_wait_until`・`fle_audio_switch_note`。**ペースの枝は G3 のアイドルフックの後**に置いた |
| `lift/crates/fe-lift/src/imports_table.rs` | +6 | 3 行(`DirectSoundCreate`・`DirectSoundCreate8`・`DirectSoundEnumerateW`)。末尾追記のみ。行 598〜600 |
| `lift/crates/fe-lift/tests/wall_fr15.rs`(新規)、`rows_0139/mod.rs`(新規) | 517 / 9 | 18 テスト、3 行の名前の一覧 |
| `lift/harness/audio/ds_scenarios.c`(新規) | 699 | C のゲスト(列挙・作成・エラー・再生・停止・通知・フォーマット・`rt_mixer`・リングのヘッダ・名前つき停止) |
| `lift/harness/web/audio/`(新規 9 ファイル) | 383 | ページ、AudioWorklet、Worker、ランナー、解析、wasm のシナリオのビルド |
| 既存テスト | | 下の「テストの変更」 |

`fe-lift` / `fe-decode` は触れていない。`fle_dispatch.h`、`app/` も触れていない。

## app/ に載せる口(読み取り専用の `app/` に載せるときの契約)

1. **ページ**: `AudioContext({ sampleRate: 44100 })`(ユーザー操作の後で `resume()`)、`audioWorklet.addModule('audio-worklet.js')`、`new AudioWorkletNode(ctx, 'fle-out', { outputChannelCount: [2], processorOptions: { sab, cap, dataOffset } })`、`node.connect(ctx.destination)`。**`SharedArrayBuffer` の要件 = COOP/COEP(cross-origin isolated)**。
2. **SAB のレイアウト**(`web/audio/ring.js`。version 1): Int32 の 32 語のヘッダ(0 RD、1 WR、2 STATE(0 閉・1 開・2 再生)、3 RATE、4 CAP、5 MAGIC 0x52414C46、6 VERSION、8 Q_TOTAL、9 Q_PLAYING、10 UR_QUANTA、11 UR_FRAMES、12 FILL_MIN、13 FILL_MAX、14 PUBLISHES、15 PUBLISHED_FRAMES、16 SKIPPED_FRAMES)、データは 256 バイト目から Float32 のステレオ交互 `cap` フレーム。`FLE_AUDIO_SAB.make(cap, rate)` が作る。カウンタは `Atomics`、フレーム数は mod 2^32。
3. **Worker へ**: `run` の**前に** `{ cmd: 'audio-init', sab, cap, dataOffset, leadMs }` を送る(`self.fleAudioSab` になり、エンジンの EM_JS が読む)。なければ(`fleAudioSab` なし)エンジンは**仮想時計**を消費源にして従来どおり動く(= native と同じ)。
4. **エンジンからのテレメトリ**: Worker は 500 ms の実時間ごとに `{ type: 'audio-pulse', real, virt, consumed, written, threads: [[total, max, slices] …], mixer, unlockCompute }` を `postMessage` する(スレッド切替から。Worker が長い計算中でも出る)。ページの問い合わせ `{ cmd: 'audio-state' }` は Worker のイベントループが回るときだけ答える。
5. **C のエクスポート**: `fle_audio_ring()`(C のリングのヘッダのアドレス。版 1。診断用。運びには使わない)、`fle_audio_pace_stat(k)`。リングのヘッダ(`fle_audio_ring_t`)は `fle_audio_ring.h` に項目ごとに書いてある。
6. **G3 のアイドルとの関係(要統合)**: G3 のページ主導のアイドル(`fle_idle_wait(now, next)` / `Module.fleIdleDeliver`)は、最初のスレッドが入力待ちで他に走れるスレッドがないときの時計の進め方を**ページが決める**。音声が再生中のとき、ページの方針は「**AudioWorklet の消費フレーム数に合わせて**仮想時計を進める」でなければならない(この差分のペースの枝は G3 のフックが適用されない場合だけ動く: G3 のフックが先)。入力イベントはペースの待ちの間は渡されない(F3 への注記)。

## 検証

### テスト(再ベース後の木)

| 項目 | 内容 |
|---|---|
| 新規 | `wall_fr15`: **18 テスト**(既定のリード 60 ms と環境変数・範囲外の値、行 598〜600 の位置、シナリオ一覧、列挙 2 回・NULL GUID、オブジェクトとエラー、全メソッドのエラー値、play / write カーソル(手計算: 44100 Hz・882 フレームのバッファ・リード 15 ms = 661 フレームで、`t+1 ms` の play = 176 バイト・write = 2820 バイト。**テストの走行は `FLE_AUDIO_LEAD_MS=15` で行い、既定の 60 ms は専用のテスト**で確かめる)、`Stop` の連続性、通知(+10、+20 … ms ちょうど)と待つスレッドの起床、仮想時計が次の通知へ進む、形式・音量・パン・ミキシング(サンプル値)、リングのヘッダ、未モデルの名前つき停止 2 テスト、実エンジンのポーリングミキサーが仮想時間でアンダーラン 0、フックの位置と生成 C に触れていないこと、モデルがホスト時計・スレッド・ファイルを使わないこと、全 vtable のスロットが埋まりエンジンが呼ぶオフセットが audio-m5.md の読みと一致)。 |
| 変えた既存テスト | `vcall_thunk_dll`(228 → 227)、`wall_fr4` / `wall_fr5`(`LATER_ROWS += rows_0139::ROWS_0139.len()`)、`wall_fr13`(`rows.len() − 10 − rows_0139::ROWS_0139.len()`)、`wall_fr14`(`GetTokenInformation` の位置で末尾を探す)、`wall_fr11`(dsound のテストを書き換え: callback → 名前つき停止 "indirect call to 0x12af280 … is not a known function entry"、NULL callback → `ret 80070057`)、`wall_r`(全ランタイムのリンクに `-lm`)。`wall_fr8/9/10/12` は G3 の名前ベースの版をそのまま使う(差分なし)。 |
| 全体テスト | **最終の木の通し結果は「全体テスト(最終)」の行**(下)。 |
| 変異検査 | 20 件中 20 件検出(M01 既定のリード 60 → 61 ms、M02 ÷32767、M03 Lock の wrap、M04 通知の開始位置、M05 Stop のカーソル、M06 列挙の 2 回目、M07 QueryInterface の戻り、M08 vtable の Play/Stop の入れ替え、M09 スケジューラが poll しない(時間切れ)、M10 窓表の確認、M11 2 回目の Lock、M12 idle gap、M13 Unlock のバイト数、M14 パン、M15 COM スロットのフック、M16 次の通知時刻、M17 write = play、M18 音量の曲線、M19 8 ビットの中心、M20 `DSERR_NODRIVER` の値)。 |
| 全体テスト(最終) | **136 result・2,002 passed・0 failed・0 ignored**(`cargo test --workspace --no-fail-fast --offline`、終了 0、**最終の木 = リード 60 ms の版での 1 回の通し**。23:56:42Z〜01:05:02Z、68 分。開始前の available 16,393 MB。`logs/fulltest-final/test-result-lines.txt` の 136 行の合算。基準 fa11947 は 135 result・1,984 passed・0 failed なので **+1 result(`wall_fr15`)・+18 passed**。**この走行の間、別の測定(a4 の独立 2 回、変異検査 20 件、native の再リンクと gdb の測定 3 本)を同時に走らせた**(PM の指示: 全体テストと同時に重い走行をしない → 違反。時間は延びたが、結果は 0 failed)。最初の通し(`wall_r` の `powf` で 1 failed)は `logs/fulltest-pass1/`) |

## 壁の表(この作業で見つけたもの)

| # | 壁 | 場所 | 対応 |
|---:|---|---|---|
| 1 | 実エンジンの 1 台目のデバイスが無い(`DirectSoundEnumerateA` が 0 台) | S0 | S1 のモデルで 2 回の callback |
| 2 | COM のメソッドを呼ぶ先がない(vtable の擬似 VA が応答表の外) | S1 | `fle_com_try_slot` を `fle_win_try_thunk` の先頭に。ヘッダ `fle_dispatch.h` のフックは**不成立**(「不備の自己申告」)|
| 3 | wasm: thread 2 の C スタック 1 MiB の canary 破損 | S2(s2d) | `FLE_THREAD_WASM_CSTACK_BYTES=8388608`(層状窓レーン 0140 が既定にする)|
| 4 | wasm: 実エンジンの実時間不足(アンダーラン 98%) | S2 | 壁ではなく限界。「遅いスライスの内訳」と PM の判断点 1・2 |
| 5 | 弱い参照の JS ライブラリ関数が wasm-ld で import されない | S2 | EM_JS / EM_ASYNC_JS を `fle_audio_ring.c` に埋め込む |
| 6 | `powf` が未解決(`wall_r` の全ランタイムのリンク) | 全体テスト | `-lm` を `wall_r.rs` に |

## 止まった理由 / 実エンジンの現状

- native: 300 秒の間に新しい壁なし(S1)。ミキサー(ポーリング)が約 92.4 ms ごとに無音のブロックを書き続ける。**ブロックの peak は 0**(無音)= G7 の「無音・アンダーラン 0」の native の形であって、G8(非ゼロ)ではない。
- wasm: PLAYING まで約 11.3 秒。その後は上のとおり実時間に対して約 0.5% の速さで、AudioWorklet はほぼ無音を再生する(アンダーランは**エンジンが遅いことの数え方**で、ブラウザの故障ではない)。

## 遅いスライスの内訳と強制 yield の見積もり(PM の追加依頼)

取れた範囲を書く。**測定の種類が違うものを混ぜない**: (A) wasm の実エンジンのスレッド別実行時間(S2 のテレメトリ、実測)、(B) native `-O0` の gdb による API 呼び出しの間隔(yield の候補点、実測)、(C) native `-O0` の gdb による標本抽出(どの関数に時間を使っているか、実測)。(B)(C) は wasm ではない(native `-O0`、gdb の下)ので、**絶対時間は wasm と一致しない。関数の比率は近い、と読む**(native の速さは wasm `-Oz` と同程度: 最初の `Play` の後の仮想時計の進みが native 約 3 ms/秒、wasm 5.5 ms/秒)。

### (A) wasm の実エンジンの時間の使い方(`logs/s2a/s2f.json.gz` の 500 ms ごとの `audio-pulse` 45 個)

| 区間(Worker の実時間) | メイン(thread 0) | thread 2 | ミキサー(thread 7) | 仮想時計 |
|---|---|---|---|---|
| 0〜14 秒(PLAYING は 11.3 秒) | 8.3 秒のスライス 1 本(5.9〜14.3 秒) | — | — | +8 ms |
| 14〜25.5 秒 | **11.26 秒のスライス 1 本(最大)** | — | 初回 | +15 ms |
| 30〜80 秒(定常) | **約 2.5 秒のスライスが約 3.0 秒ごと** | **約 0.5 秒のスライスが約 3.0 秒ごと** | 数 ms のスライスが 1 周期に 1 回 | **周期ごとに +16 ms**(= 3.0 秒の実時間で 16 ms: **190 : 1**) |
| 80〜88 秒 | 0.1〜0.4 秒に短くなる | 0.5 秒 | 数 ms | 5.6 秒で +128 ms(**44 : 1**) |

- 定常のとき、**16 ms の仮想時間(アプリの 16 ms タイマー = 1 フレームと読む)に 3.0 秒の実時間**がかかる(メイン約 2.5 秒 + thread 2 約 0.5 秒)。ミキサーが必要とするのは 92.9 ms の仮想時間ごとに約 6.5 ms の計算で、**仮想時間 92.9 ms を進めるのに実時間 18 秒**(6 周期)かかる。つまり実エンジンは定常で**実時間の約 0.5%**(1 : 190)で走る。thread 2 の「1 フレーム 0.46 秒」(PM の通知)は、この 0.5 秒のスライスと同じもの(平均 0.46 秒、最大 0.70 秒)。メインの「平均約 2 秒」は 35 スライスの平均(定常の 2.5 秒と、80 秒以降の短いスライスの混合)、最大 11.3 秒は起動の直後。
- **強制 yield だけでは足りない**(読み + 上の実測): スライスを 10 ms ごとに切っても、**仮想時計は「全スレッドが待つときだけ進む」設計なので、計算している間は 1 ms も進まない**。ミキサーの期限(仮想時刻)は来ないままで、切った先のスレッドも計算を続ける。**強制 yield が効くには、(a) 計算中にも仮想時計を実時間(= AudioWorklet の消費フレーム数)に合わせて進める、(b) 期限が来たミキサーを次の yield 点で優先する、の 2 つが要る**。(a) は「遅いマシンで動くアプリ」と同じ見え方になる(アプリのタイマーは遅れて発火する)。1 周期 3.0 秒の仕事は残るので、アプリのフレームは 3 秒に 1 回になる。**音声だけを実時間にする**ことはできるが、**アプリの描画を実時間にするには 1 周期の仕事を 2 桁以上減らす必要がある**(HLE の対象)。

### (B) API 呼び出しの間隔 = 強制 yield を API の入口に置いたときに切れる最短のスライス(`logs/prof2b/`、native `-O0`、gdb)

- 方法: `fle_imp_*`(約 700 の API 実装)のうち、純関数に近い 8 個(`GetCurrentThreadId`・`MultiByteToWideChar`・`CompareStringW`・`TlsGetValue`・`WideCharToMultiByte`・`SysFreeString`・`SysAllocStringLen`・`CharLowerBuffW`。全呼び出しの約 85%)を除いた全部に breakpoint を置き、**同じスレッドの連続する 2 回の呼び出しの間の時間**を取る(切り替えが入れば別スレッドの呼び出しが挟まるので区別できる)。**範囲は起動から最初の `Play` の前まで**(352 秒の gdb 実行。breakpoint の重さで `Play` に届かなかった。**`Play` の後は未確認**)。メイン 149,270 回、ほかのスレッド 5 本は合計 4,000 回強(最大の間隔 11.7 ms)。
- 結果(メインスレッド。間隔 ≥ x ms の本数 / その合計):

| 間隔 | ≥ 1 ms | ≥ 5 ms | ≥ 10 ms | ≥ 20 ms | ≥ 50 ms | ≥ 100 ms | ≥ 200 ms | ≥ 500 ms | ≥ 1 s | ≥ 2 s |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 本数 | 11,101 | 280 | 150 | 76 | 25 | 11 | 5 | 3 | 2 | 1 |
| 合計(秒) | 28.0 | 9.9 | 9.0 | 8.0 | 6.4 | 5.4 | 4.6 | 4.05 | 3.3 | 2.1 |

  (1 ms 前後の間隔には gdb の停止と再開のコスト(1 回約 0.5〜1.5 ms)が入っている。長い間隔は実際の計算時間。)
- 最長は 2.1 秒(`LeaveCriticalSection` → `RegOpenKeyExW`)、次が 1.16 秒(`VirtualFree` → `VirtualFree`)。**20 ms を超える間隔 76 本の端点**: 始点は `GetTickCount` 29、`LeaveCriticalSection` 11、`GetParent` 7、`HeapReAlloc` 6、`KillTimer` 5、`VirtualAlloc` 4。終点は `GetTickCount` 24、`FreeResource` 20、`EnterCriticalSection` 6、`RedrawWindow` 6。時間では始点が `LeaveCriticalSection` 2.5 秒、`GetTickCount` 1.4 秒、`VirtualFree` 1.2 秒、`KillTimer` 0.8 秒、`SetWindowLongPtrW` 0.8 秒。**`GetTickCount` から `GetTickCount` の間隔が ≥ 20 ms で 31 本・最大 104 ms**: アプリが「時計を見ながら一定時間だけ仕事をする」ループ(時間で区切った処理)を持つ、と読む(**ここは自然な yield 点**)。
- **見積もり**: 候補点 = 全 API の入口のうち、(1) `GetTickCount` / `timeGetTime` / `QueryPerformanceCounter`(時間で区切るループ。切っても意味が変わらない)、(2) `Enter`/`LeaveCriticalSection`・`WaitForSingleObject`・`SetEvent`(同期点。**ここでの切り替えはゲストが待ちを許す点**なので安全。ただしクリティカルセクションを持ったままの切り替えはほかのスレッドが待つ)、(3) `Sleep`・`GetMessage`・`PeekMessage`(すでに切り替え点)。**この 3 種だけで、起動〜最初の `Play` の区間のメインスレッドのスライスは、76 本(≥ 20 ms)を除いて 20 ms 以下に切れる**。残る 76 本(合計 8.0 秒)と、特に 11 本(≥ 100 ms、合計 5.4 秒)は、API の間に長い計算があるので、**API の入口では切れない**: 速度改善(HLE)か、生成 C の基本ブロック境界のチェック(生成器の変更)が要る。**`Play` の後(wasm の定常 2.5 秒のスライス)は測れていない**ので、この見積もりは起動区間のもの。
- 実装の見込み: `fle_imp_*` は 1 関数ずつ別なので、全部にチェックを入れるのは非現実的。**候補の関数(上の 10 個程度)にだけ**「前の切り替えから実時間 T ms 以上経っていたら `READY` にして `schedule()`」を入れる(協調スレッドの規則 0122 の拡張)。wasm の時計の読みは `performance.now()`(約 100 ns)なので、呼び出しごとの読みで足りる。T は `lead − ゲストの検査の余裕`(既定のリード 60 ms なら 10〜20 ms)。

### (C) どの関数に時間を使っているか(`logs/prof1e/`、native `-O0`、gdb の標本抽出。最初の `Play` の後の標本)

- 方法: 一次 `two-native`(native `-O0`、`-finstrument-functions` つき、FRUITY + SEH、fa11947 の木 + この差分の `nat3`)を gdb の下で 262 秒走らせ、**1 秒ごとにシグナルで止めて**、走っているゲストスレッドの番号(`fle_thread.c` の静的変数 `cur`)と C の呼び出し鎖を記録した(`tools/gdbsample.py`、`sigint-sender.py`。標本 219 個、最初の `Play`(実時間 45.9 秒)の後の 215 個を集計。`logs/prof1e/`)。止めている間(1 標本に約 1 秒未満)はゲストの時間が進まないので、比率は CPU 時間の比率の近似。
- **限界**: (1) native `-O0` + 関数入口の計装(`__cyg_profile_func_*`)のため、メインスレッドの leaf 標本の約 3 割は計装そのもの(`__cyg_profile_func_enter/exit` と wrap)で、**呼び出しの多い小さな関数の比率は wasm より大きく出る**。(2) 標本数が少ない(メイン 159、thread 2 は 52)ので、50% 前後の比率は ±8 ポイント程度の幅がある。(3) 最初の `Play` の後 262 秒の範囲で、wasm の「定常 30〜80 秒」に相当するかは**未確認**(native の仮想時計は `Play` の後 約 3 ms/秒、wasm は 5.5 ms/秒で、同程度の進み方)。(4) スレッド番号の対応(wasm のテレメトリの thread 0 / 2 と同じか)は、生成順が同じなので同じと読むが**未確認**。
- **メインスレッド(T0、標本の 74%)**: 最も外側のゲスト関数は `f_1375990`(`CreateFruityInstance`)= まだ初期化の中で、メッセージループには入っていない。ゲスト関数の所属は FLEngine 82%、Blend2D 9%、quickfontcache 6%、dsp_ippv2 3%、FreeType 1%。**`f_6a73a0` だけで T0 の標本の 53%**(84 / 159。呼び元 `f_6c01a0`)。生成 C を読むと 70 命令の SSE2 のルーチン(`movd` / `pshuflw` / `punpcklqdq` / `psllw` / `paddw` / `packuswb` / `unpack`: 8 ビットを 16 ビットに広げて重みをかけ、足して詰め直す = **画素のブレンド or 補間のカーネル**。`f_6c01a0` から 1 箇所で呼ばれ、行ごとのループの中と読む)。leaf は `fle_unpack`・`fle_szp`・`fle_packuswb`・`memcpy` などの SSE のエミュレーション helper。2 番目は `f_127c600`(14.5%、呼び元 `f_127c670`。40 行のスカラー関数で `0x28f` 倍の添字の表引き。呼び出し回数が多く、**計装の分が大きい**と読む)、3 番目は `f_40ff70`(3.8%)。上位 3 つで 71%。
- **thread 2(T2、標本の 24%)**: **すべて dsp_ippv2(IPP)**。`f_33d6e70` 46%(呼び元 `f_33e97c0`)、`f_27ad5f8` 27%(呼び元 `f_27ad5d6`)、`f_344fcf0` 27%(呼び元 `f_3476d30`)。leaf は MMX のエミュレーション helper(`fle_mx_unpack`・`fle_mx_add`・`fle_mx_arith`・`fle_mx_pack`)。最も外側は `f_412ae0`(TThread のラッパー)39 / 52。決定 0132 の gdb の計数(スレッド 2 が `ippiWarpAffineQuad_8u_C4R` の描画フレームを計算し続ける)と合う(読み。関数名の対応は取っていない)。
- **ミキサー(T7)**: 標本 2 個(1%)。呼び鎖に `f_d0ece0` → `f_12aeec0`(`TMixerThread_DS.Execute`)があり、時間は `f_6a1e50`(`maxps` を使う SSE の最大値/リミットの処理。呼び元 `f_6a45e0`)。**ミキサーは CPU 時間の約 1%**(wasm のテレメトリのミキサー 6.5 ms/ブロックと合う)。T13 の 2 標本は FLEngine の関数(意味のある比率は出ない)。
- **HLE の見込み(読み)**: `f_6a73a0` を等価な手書きの実装(wasm SIMD / スカラー)に置き換えれば、メインスレッドの約 53% が消える。Amdahl で、メインの 2.5 秒 → 約 1.2 秒(約 2.1 倍)。**定常で 190 : 1 の差を埋めるには、メイン・thread 2 とも仕事の 99.5% 以上を消す必要があり、上位 3 関数の HLE だけでは足りない**(メインは上位 3 つで 71%、thread 2 は 3 つで 100% だが 0.5 秒/フレーム)。仕事の構造(なぜ 16 ms の仮想時間ごとに 2.5 秒分の画素処理を繰り返すか: 全面の再描画を毎フレームやっていないか)を読む必要がある。**ここは未確認**。

summary の出力(`tools/profsample-sum.py`。`f_<VA>` はゲストの関数、モジュールは VA の範囲から):

```
samples 215 (after-play=True, first PLAY at wall 45.89 s)

== thread T0: 159 samples (74%)
  module of the nearest guest function: FLEngine 82%, Blend2D 9%, quickfontcache 6%, dsp_ippv2 3%, freetype 1%
  top nearest guest functions (samples; module; most frequent caller):
    f_6a73a0     84 (52.8%)  FLEngine    <- f_6c01a0
    f_127c600    23 (14.5%)  FLEngine    <- f_127c670
    f_40ff70      6 ( 3.8%)  FLEngine    <- f_410090
    f_1e7e100     4 ( 2.5%)  quickfontcache  <- f_1e802e0
    f_1e863f0     3 ( 1.9%)  quickfontcache  <- f_7cbf00
    f_48384d0     3 ( 1.9%)  Blend2D     <- f_4893e80
    f_6a74a0      2 ( 1.3%)  FLEngine    <- f_6a75b0
    f_1e7e490     2 ( 1.3%)  quickfontcache  <- f_1e802e0
    f_3845040     2 ( 1.3%)  dsp_ippv2   <- f_276dee0
    f_1100fa0     2 ( 1.3%)  FLEngine    <- f_12c17d0
    f_47edd30     2 ( 1.3%)  Blend2D     <- f_4840d30
    f_4839190     2 ( 1.3%)  Blend2D     <- f_4840740
  top leaf functions: __cyg_profile_func_exit 20, __wrap___cyg_profile_func_enter 17, __cyg_profile_func_enter 14, fle_unpack 13, __memcpy_avx512_unaligned_erms 11, fle_szp 10, fle_packuswb 8, fle_ld 7
  outermost guest function of the stack (a hint of what the thread is doing): f_1375990 146, f_1375950 4, f_52a570 3, f_807470 1

== thread T2: 52 samples (24%)
  module of the nearest guest function: dsp_ippv2 100%
  top nearest guest functions (samples; module; most frequent caller):
    f_33d6e70    24 (46.2%)  dsp_ippv2   <- f_33e97c0
    f_27ad5f8    14 (26.9%)  dsp_ippv2   <- f_27ad5d6
    f_344fcf0    14 (26.9%)  dsp_ippv2   <- f_3476d30
  top leaf functions: __cyg_profile_func_exit 10, __wrap___cyg_profile_func_enter 7, __cyg_profile_func_enter 5, fle_mx_unpack 5, fle_mx_add 4, fle_ld 4, fle_mx_arith 2, fle_mx_pack 2
  outermost guest function of the stack (a hint of what the thread is doing): f_412ae0 39, f_27ad5d6 11, f_6ca7b0 1, f_27ad5f8 1

== thread T13: 2 samples (1%)
  module of the nearest guest function: FLEngine 100%
  top nearest guest functions (samples; module; most frequent caller):
    f_5101a0      1 (50.0%)  FLEngine    <- f_50fa20
    f_439550      1 (50.0%)  FLEngine    <- f_9d0200
  top leaf functions: fle_rd 1, __cyg_profile_func_enter 1
  outermost guest function of the stack (a hint of what the thread is doing): f_412ae0 2

== thread T7: 2 samples (1%)
  module of the nearest guest function: FLEngine 100%
  top nearest guest functions (samples; module; most frequent caller):
    f_6a1e50      2 (100.0%)  FLEngine    <- f_6a45e0
  top leaf functions: __wrap___cyg_profile_func_enter 1, fle_sb 1
  outermost guest function of the stack (a hint of what the thread is doing): f_412ae0 2
```


## PM の未決の点

決定済み(この版に反映): (1) リードの既定は 60 ms(S-audio-lead)、(2) G7 の字義は実エンジンで未達、(3) 単一スレッド協調の制約は速度改善(HLE)と API 呼び出し点での強制 yield で解く。ミキサーの別 Worker は採らない。以下は**残る判断点**と、(3) の見積もり(前節)から出た注意。

1. **強制 yield は「仮想時計の進め方」の変更とセットでないと効かない**(前節 (A))。今の設計は「仮想時計は全スレッドが待つときだけ進む」なので、スライスを 10 ms で切っても、計算中は仮想時間が進まず、ミキサーの期限は来ない。必要なのは (a) 計算中も仮想時計を実時間(AudioWorklet の消費フレーム数)に合わせて進める(アプリのタイマーは遅れて発火する = 遅いマシンと同じ見え方)、(b) 期限が来たミキサーを次の yield 点で優先する、の 2 つ。この変更は `fle_thread.c` のスケジューラの規則(決定 0122)と、G3 のページ主導アイドル(決定 0136)の双方に触れる。**(a) を採るかどうかの判断が要る**(採らない場合、yield は UI の応答性にしか効かず、G7 には効かない)。
2. **実エンジンの定常速度は実時間の約 0.5%(1 周期 3.0 秒で仮想 16 ms)**。強制 yield で音声だけは実時間にできても、**アプリの描画は 3 秒に 1 フレーム**のまま。G7 の字義(実エンジンのブロックで連続アンダーラン 0)は yield + (a) + (b) で満たせる見込みだが、**G8 以降(再生中の UI、プラグインの描画)には 2 桁の速度改善(HLE)が要る**。HLE の対象は次の (C) の表(どの関数が時間を使うか)と perf2 のホットスポット表の突き合わせ。**この文書の (C) は native `-O0` の標本抽出で、perf2 の測定とは別物**(perf2 の結果は読んでいない)。
3. **リード 60 ms の代償**(S-audio-lead): ゲストの write カーソルの検査は、ミキサーが半分の入れ替わりから 92.9 − 60 = 32.9 ms 以内に書き終えないと「アンダーラン」を数える(hog 60 ms で 12 回)。強制 yield の T(切る間隔)は 10〜20 ms に収める必要がある。リードを 30 ms にすれば余裕は 62.9 ms、出力側の欠落への強さは落ちる(hog 60 ms で 5.0%: 再ベース前の測定)。**実エンジンでの検査の影響(ゲストの計数 `[0x15e16d0]` の値、UI の表示)は未確認**。
4. **ペースとアイドルの統合**(G3): ページ主導のアイドル(`fle_idle_wait(now, next)` / `Module.fleIdleDeliver`)は、音声が再生中のとき消費フレーム数に合わせて仮想時計を進める必要がある。この差分のペースの枝は G3 のフックが適用されない場合だけ動く(G3 が先)。ペースの待ちの間は入力イベントが渡されない(F3 への注記)。実装は `app/` 側。
5. **8 MiB の C スタック**を層状窓レーン(決定 0140)の既定にする(この差分は既定を変えない)。
6. **S-ds-pcm16-scale(÷32768)と float → PCM16 の係数(FLEngine は ×32767)**: ゲストが書いた PCM16 の戻し係数だけの問題(今は ÷32768)。
7. **実機の Windows の素材**: Learn に書かれない出典なしの選択 47 名は実機で確認できていない。デバイスの説明、`GetCaps`、境界の戻り値、write カーソルの実際の値の取得ができるなら、優先度が高い。

## 不備・規則違反の自己申告

- **作業のやり直し・失敗**(MERGE-NOTES にも同じ):
  - 最初に `fle_dispatch.h` の `fle_dispatch_miss`(インライン)にフックを置いたが、**すでに生成済みのオブジェクトは古いインライン関数を持つため効かなかった**。`fle_win_try_thunk` の先頭に変えた(`fle_dispatch.h` は無変更)。
  - 弱い参照の JS ライブラリ関数は wasm-ld で import されないと実験で分かり、EM_JS に埋め込む設計に変えた(1 回やり直し)。
  - wasm のシナリオのビルドで atom / `GLOBAL_BASE` の不整合を踏み、ビルドをやり直した。
  - 全体テストの 1 回目で `wall_r` が `powf` 未解決で失敗(`-lm` を足して直した)。**直した後の全体の通しは別の 1 回**(結果は MERGE-NOTES)。
  - ランナー(`run-audio.mjs`)を、`TMPDIR` 等の環境変数をエクスポートした同じシェルから起動すると**何も出さずに終了**する失敗が 2 回あった(環境を空けて起動し直した)。
  - wasm の S0 の走行で、段階 `idle` に届いた後も 661 秒(11 分)待った(停止の条件を決めておらず、待ちが長すぎた)。
  - 変異検査の最初の 1 回: M01 のパターンが木に無く(検出でも生き残りでもない)、M10 は生き残り(窓表が 0 も拒否するため等価)。テストを非ゼロの不正ハンドルで強め、やり直して 20/20。
  - `logs/s1a/rc.txt` に `rc=134` と `rc=4` の 2 行がある。`rc=134`(SIGABRT)は同じログディレクトリに書いた前の走行のもので、その stderr は次の走行で上書きされ、**134 の原因は記録がない(未確認)**。
  - `logs/s2a/s2a.json`・`s2a2.json` は PLAYING に届かなかった走行(state 0)で、その原因は**記録がない(未確認)**。`s2b` 以降の走行で PLAYING に届いた。`s2b`〜`s2d`・`s2g`・`s2h` は解析スクリプトがページのサンプルの欠落で止まる(JSON の生の値は読める)。
- **手順の逸脱**: 作業中に cwd を取り違えたことが数回(コマンドを木の外で実行しかけて気付いて直した)。大きな出力をそのまま画面に出したことが 1 回(`s0-oz.json` の RSS 記録)。`out-audio1` 内の自分の作業ディレクトリに対して `rm -rf` を 1 回使った(結果物のディレクトリには使っていない)。
- **再ベース前の実エンジンの測定**: S0 の native(gdb)と S2 の実エンジン(wasm)は ce16052 + 差分の木での測定。再ベース後に実エンジンの wasm を取り直していない(G3 の `fle_idle_wait(now, next)` の API が変わり、`harness/web/final0129/fle-final-worker.js` のアイドル処理が非互換)。再ベース後の確認は「再ベース(fa11947)の後の確認」に書いたとおり: シナリオ・テスト・a4・gencmp、native の実エンジン 1 回(`logs/nat2`)、wasm の C シナリオ 3 走行(`logs/s2b-rebased`)。
- 測っていないもの: wasm `-O2`、実機の Windows、native `-O2` の速さ、Safari(JSPI / AudioWorklet)、実際の出力デバイスでの聴取。

## 再現の手順

```
# 差分を当てる(fa11947 のリポジトリ根)
patch -p1 --dry-run < lift/out-git/audio1-results/audio1.diff && patch -p1 < lift/out-git/audio1-results/audio1.diff
# 新規テスト(native)
cargo test -p fe-lift --offline --test wall_fr15 --manifest-path lift/Cargo.toml
# C のシナリオ(native)は wall_fr15 が自分でビルドして走らせる(lift/harness/audio/ds_scenarios.c + runtime/fle_*.c、-lm)
# wasm のシナリオ(Chrome)
bash lift/harness/web/audio/build-scen-wasm.sh && node lift/harness/web/audio/run-audio.mjs name=s2-rt dist=<out> out=<logs> 'q=scen=rt_mixer&args=100,0,0&ms=9000&wait=30000'
python3 lift/harness/web/audio/analyze-audio.py <logs>/s2-rt.json
```

測定用の道具(gdb の API トレース `gdbaudio.py`、変異検査 `mutations.py`、`final-verify.sh` ほか)は `lift/out-git/audio1-results/tools-audio1/`(差分には含めない。パスは作業者の機械のもの)。

## 限界(確認できなかったこと)

- **実機の Windows との照合は 0 件**(列挙の説明・GUID、`GetCaps`、境界の戻り値、write カーソルの実測)。
- S2 は **Chrome 156 headless の 1 台**。**実際の音は聴いていない**。出力遅延(`outputLatency` 0.024〜0.032 s)はヘッドレスの値で、実機のオーディオスタックの値ではない(未確認)。
- 実エンジンの wasm の S2 は **`-Oz`**、**再ベース前の木**。`-O2` の速さ、FL の再生開始(G8: 非ゼロ)は未測定。
- `fillMin` の負の値(例: −384)は AudioWorklet が `RD` と `WR` を別々に読む競合の可能性がある(アンダーランのカウントと合わない走行がある)。**未確認**。
- ASIO・WASAPI・waveOut は作っていない(S0 で呼ばれないことを確認した範囲)。入力(録音)・MIDI の出力は範囲外。
- 900 秒より後の音声の呼び出し、プロジェクトを開いた後(G8)の呼び出しは見ていない。
