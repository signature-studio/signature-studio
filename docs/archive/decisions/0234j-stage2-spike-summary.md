# 0234j(mt3): B′ 第 2 段スパイクの総括(GO 判断のあとの整理。11:20 JST)

状態: 作業木の試作(旗つき足場)と、その上で測った事実。PM の判断(10:50): **GO**。成果物の差分(段階 B)は未完成のため取り込まない(規則 4)。段階 A(lifter の LOCK 付き命令のアトミック化)は別差分 `mt4-atomics.diff` で先に取り込み可能(`lift/out-git/mt4-results/`)。

## 1. 事実(この機械、node 24、最適化なしの wasm、他の担当による負荷 11〜33)
| 項目 | 結果 |
|---|---|
| 入れ子 Worker の JSPI | Chrome 156 / Firefox 158 / node 24 で動作(玩具)。エンジン本体の第 2 Worker は emscripten の pthread(JSPI ビルドでは開始ルーチンが JSPI スタック上で走る)で、node では worker_threads の薄い Worker クラスで動いた。ブラウザでのエンジン本体は未確認。 |
| GIL の費用 | 1 Worker・競合なし: 全体 −0.1%(t=−0.2)、再生区間 −0.9%(t=−0.6)。2 Worker 以上(5 分再生): 競合した取得 532,601 回/412 秒(約 1,300 回/秒)、スピン 1.19 億回(競合 1 回あたり約 224 回)。 |
| 協調モードの同一性(84/55 行) | GIL 入り・残りの文脈の TLS 入りのバンドル(1 Worker)で全行一致。 |
| 本物のスレッドでの起動(cmp-short) | 1 スレッド 1 Worker(最大 11 Worker)+ アトミック版で 4/4 完走(約 20 秒)。大域インターロック版は 0/3。1 Worker に 2 本以上: 常に止まる(FastMM のスピンロックの追い出し)。 |
| 参照プロジェクト | 30 秒再生、5 分再生(再生区間 312 秒、実時計)を完走。名前つきの停止 0。`late_frames` 0。ミキサーの Lock/Unlock 452 → 2263 回。スレッドは終了時に 23 本(Worker は 11 本なので、12 本目以降は Worker を共有した)。 |
| ミキサーのテンポ | 再生区間 5.80 ブロック/秒(テンポ 0.54)、起動後〜アイドル 10.8(1.0)。この測定は最適化なしの wasm・load 22〜27 で、**P3 の判定には使えない**。スレッドごとの CPU(top -H): 再生中の第 1 Worker 68%、ミキサーの Worker 47.5%、他は 0〜3% = ミキサーは専用の Worker で走っている。 |

## 2. 見つけた原因(4 件。根拠付き。docs/0234i と 第 7 報)
1. 第 2 Worker の stdio の同期 proxy(NO_FS=0 の glue)と第 1 Worker の同期 GIL 待ちのデッドロック → 第 1 Worker の GIL 待ちを非同期に。不変条件 I1。
2. GIL の起床漏れ(寝た後は 2 で取る)。
3. 1 Worker に 2 本以上のスレッドは FastMM のスピンロックで進まない → 1 スレッド 1 Worker。
4. lifter の LOCK 付き命令・XCHG[mem] が非アトミック → mt4 段階 A。

## 3. 段階 B を成果物にするために残る作業(見積り、読み)
| 作業 | 内容 | 規模 |
|---|---|---|
| 旗の除去 | `FLE_GIL`、`FLE_TLS_MORE` は `tools5/strip_ifdef.py` で機械的に畳める。環境変数 `FLE_WORKERS` は定数(1 スレッド 1 Worker)に。診断(fle_dbg、ノートのリング、DBG1、スライスごとの JS スタック、ラッパの `D()`、`FLE_RT_*`)は成果物に入れない | 半日 |
| 動的な Worker 割り当て | CreateThread のたびに pthread を 1 本(最大 31)。プールの考え方(終了した Worker を再利用)、Worker 数の上限に達したときの扱い(いまは巡回で共有)。`T[BOOT]` の idle 文脈を Worker ごとに | 1 日 |
| runtime の GIL の外の共有状態 | `fle_swap_ctx_*`(Blend2D のアダプタ)、`fle_hle_*`、`fruity_host_slot*`、`fle_hook_*`、`fle_site_store32` の入口に GIL(データ駆動の wrap を『生成 C が直接呼ぶ runtime の関数』へ拡張。x87 と mxcsr は TLS だけなので除く) | 1 日 |
| export の GIL | `fle_desk_*`、`fle_surface_*`、`fle_av_poll`、`fle_app_*`、`fle_filedlg_*`、`fle_host_post_input` に `fle_gil_enter/leave`(`fle_g3_*` は済み)。I1 の CI の検査(glue に pthread からの同期 proxy が無い) | 半日 |
| ブラウザ(Chrome 156 / Firefox 158) | 入れ子の pthread のエンジン、`Atomics.waitAsync` の可否、COOP/COEP、起動時間(Worker 11 本の起動と 128 MB の wasm の第 2〜12 インスタンス化)、テンポ(静かな機械・最適化版) | 1〜2 日 |
| audio ring(段階 C) | 共有メモリ化(aud1 の差分の後)。いまの node 試験は ring を使わない | 別 |
| 参照プロジェクトの PCM 照合・テンポ 1.0 | ミキサーの出力 PCM の hash の安定、FLP との対応(tools/tempo) | 半日 |
合計 4〜6 日(読み)。
