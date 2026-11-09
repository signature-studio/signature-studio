# mt3 段階 C: 音声の出力リングを wasm の共有メモリへ(設計。旗なし。決定番号は PM)

状態: **設計(未実装)**。前提: aud1(決定 0241)が `fle_audio_transport.c` に JS(EM_JS)を分離した後の木(origin/main 81f6638c で確認済み)。ここで書くのは、その分離された面を差し替える最小の形。

## 1. 問題(ブラウザの実測、0234k の 1-3)
出力リングの SharedArrayBuffer(SAB)はページが作り、`audio-init` メッセージで **最初の Worker の `globalThis.fleAudioSab` にだけ**渡る。`fle_audio_transport.c` の EM_JS(`fle_js_init/consumed/publish/rewrite/state/unlock/pulse/wait`)は、呼ばれた Worker の `globalThis.__fleAud`(その Worker だけの JS オブジェクト: SAB の view、`rdHi/rdPrev`〔読み位置の 64 ビット展開〕、`wr`、統計)を使う。ゲストのスレッドが Worker ごとに走ると、ミキサーの Worker にはそれが無く、`publish` も `consumed` も `state` も何もしない(Chrome 156 の実測: 再生中のアンダーラン 38312/38744、非無音 0)。Worker 側で `fleAudioSab` を受け取って `fle_js_init` を遅延実行する暫定(スパイクの glue の前置き)は、Worker ごとに JS の状態が分かれる(`rdHi`、`wr`、`unlocks`)ので、正式にはしない。

## 2. 設計: リングはエンジンが持つ。JS は一切要らない
- **置き場所**: ヘッダ(`int32[32]`、64 バイト境界)とデータ(`float[cap * 2]`)を、エンジンの静的領域(`.bss`、整列)に置く。wasm のメモリは共有メモリ(SAB)なので、**すべての Worker が同じ番地で同じ配列を見る**。
- **ページの受け取り**: 起動時に最初の Worker(1 回だけ)が `postMessage({ type: 'audio-ring', buffer: wasmMemory.buffer, hdr, data, cap })` をページへ送る(EM_JS 1 つ。`wasmMemory.buffer` は SAB で、そのまま AudioWorklet に渡せる)。ページは worklet の port へ転送する。**SAB の作成と `audio-init` で SAB を渡す経路をページから無くす**(`leadMs` と `reportMs` はヘッダの語へ書くか、従来どおりメッセージ)。メモリが伸びても(`ALLOW_MEMORY_GROWTH`)静的領域の番地は変わらず、古い SAB オブジェクトの範囲に収まる。
- **`fle_tp_*` を C にする**(JS を消す):
  - `consumed`: `__atomic_load_n(&hdr[0])` を読み、64 ビットへの展開は共有の `uint64_t rd_unwrapped` と `uint32_t rd_prev`(GIL の下。ランタイムの関数はすべて GIL の下で呼ばれる)で行う。
  - `publish`: `memcpy`(`float` の range を、リングの折り返しで 2 回)+ `__atomic_store_n(&hdr[1])` + `__atomic_fetch_add` で統計。ギャップの無音埋めも C。JS のループ(wasm→JS の同期呼び出し 1 回/公開と、`HEAPU8` からの複写)が**無くなる**(aud1 の表: 公開あたり 6.5〜7.0 ms の一部)。
  - `rewrite`、`state`、`unlock` の記録も同様(統計は C の配列。ページへは `pulse` の postMessage をそのまま最初の Worker から送るか、ページがメモリから読む)。
  - `external()`(= 外部の消費者がいるか): ヘッダの『attached』語(worklet が起動したときに 1 を立てる)。`fle_js_init` の起動時の 1 回の判定はこれに替わる。
  - `lead_ms`: ヘッダの語。
  - `wait(ms)`: 非 first-Worker で呼ばれる経路は無い(`fle_audio_wait_until` は `!RT_ON`〔1 Worker の協調モード〕のときだけ)。Worker ごとの実時間の待ちは `fle_thread_wasm_idle_wait` が持つ。
- **worklet の変更**(`fle-out.worklet.js`、`ring.js`): 読み取りは同じ(ヘッダの `Atomics`)。違いは、SAB を『ページが作ったもの』ではなく『エンジンの `wasmMemory.buffer` + 番地』から作ること、起動時の `hdr[attached] = 1`。
- **Worker ごとの JS 状態が無くなる**ので、段階 B の暫定(glue の前置きで SAB を子 Worker に配る、`fle_js_init` の遅延呼び出し)は不要。単体試験(`app/test/unit/audio-*.test.ts`)は『page が SAB を作る』前提を『engine のメッセージを待つ』に直す。`lift/harness/audio/sim_transport.c`(別の消費者の差し替え)は、C 実装を差し替える形のままで足りる。

## 3. 旗なし・順序
旗は付けない(常に有効)。外部の消費者がいなければ attached = 0 で、従来どおり仮想時計の経路(`fle_audio_external() == 0`)。node の協調モードの出力(仮想時計)は、リングの置き場所が変わるだけで同一(`cmp-short` 84 行・`ref1-perf-short` 55 行で確認する)。段階 B の実スレッド(Worker ごと)と独立にも入れられる(単一 Worker のブラウザでも、wasm→JS の複写 1 ループが消える)。

## 4. 規模と確認
`fle_audio_transport.c`(約 200 行)の置き換え + ページの音声(`app/src/audio/` の約 5 ファイル)+ 単体試験の修正で、1〜1.5 日。確認: ① node の協調モードの同一性、② `lift/harness/audio` の ds_scenarios(sim_transport)、③ Chrome 156 / Firefox 158 で 1 拍目の欠けが 0(0241 の試験)、④ 実スレッドのブラウザで再生の PCM と、5 分のアンダーラン数。
