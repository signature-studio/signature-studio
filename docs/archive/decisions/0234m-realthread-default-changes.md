# mt3: 実スレッド版を既定にする場合の変更の全体と、壊れるもの/保証の試験の見積り(PM 求め (5)。決定番号は PM)

状態: **見積り(読み)**。測定したものは 0234k と下の 3 に分けて書く。単一 Worker の協調構成を残さない(経路を 1 本にする)場合を前提にする。

## 1. 変更の全体(段階 B の成果物にあたるもの)
| 層 | 変更 | 規模(行) | 状態 |
|---|---|---|---|
| runtime: giant lock | `fle_gil.h/.c`(ロック、再入、W0 優先、診断の表明)、`fle_imp_*` の包み(`gen_wrap.py` の生成、`--wrap`)、ランタイム入口の `fle_gil_assert` | 約 250 + 生成 | スパイクで動作(段階 2)。非 `fle_imp_*` の入口(`fle_swap_ctx_*`、`fle_hle_*`、`fruity_host_slot*`)は未 |
| runtime: スケジューラ | Worker ごとの `cur`/`pending_free`、wake word、`pick_worker`(動的割り当て、上限 32)、`wk_start`(pthread の依頼は最初の Worker だけが `pthread_create`)、idle 待ち、READY の通知(export 中も) | 約 400 | 動作(node 5 分、Chrome 数分) |
| runtime: 時計 | 実時計(Worker 間で単調)、`fle_sys_clock_real_enable` | 約 60 | 動作 |
| runtime: アトミック | LOCK 付き命令を本物のアトミックに(mt4 段階 A) | 差分 3,824 行 | 完了(PM 取り込み中) |
| runtime: ローダーロック | `call_dllmain` を CS の鍵で囲む | 約 20 | Chrome で起動時の ExitProcess 1/4 → 0/6 |
| runtime: 音声 | **段階 C: リングを共有メモリへ**(0234l)。DS 通知の予測時刻(`fle_audio_tick_of_frame` の実時計版)は A/B で効果が見えず(FL は通知を使わずポーリング: 通知 0 件)**削除した** | 約 200 | 段階 C は設計のみ |
| runtime: 入出力 | `fd_write__proxy:'none'`、`pthread_create` の非同期依頼、keep-alive(`fle_idle_lib.js`、`fle_filedlg_lib.js`) | 約 40 | 動作 |
| ビルド | `-pthread -matomics -sPTHREAD_POOL_SIZE=0` 等、`GIL=1` の包み生成、`check_i1.py` を CI に | 約 80 | スパイクのスクリプトで動作 |
| ハイブリッド(直 wasm) | 直 wasm が呼ぶ C 関数(`--c-imports`)を GIL の包みへ向ける(merge の export 名を `__wrap_` に) | 約 100(推定) | **未**。これが無いと CPU が 2.1 → 1.3 CPU 秒/秒(−39%)の利得を使えない(下の 3) |
| ページ | `audio-ring` を待つ一本道、glue の Blob を pthread Worker にも渡す(`importGlueFromBlob`)、idle driver は入力の配送だけ(時計の pacing は実時計では不要)、診断の取得 | 約 150 | 暫定がスパイクにある |
| ヘッダ | COOP same-origin / COEP require-corp / CORP は**既に**全応答に付く(`shared/headers.ts`、`worker/index.ts`)。ページの beacon が `window.crossOriginIsolated` を読む(`src/perf/browser-info.ts`、パネルに『cross-origin isolated』) | 0 | 依頼者の Firefox 159 での値は**私は見ていない**(beacon で読める) |

## 2. 壊れるもの / 保証の試験(経路を 1 本にする場合)
| 保証 | 今の仕組み | 実スレッド版のみにしたときの扱い | 見積り |
|---|---|---|---|
| 起動 16 スナップ 84 行の同一性、参照プロジェクト 55 行 | 仮想時計 + 協調スケジューラで決定的 | **同一にならない**(lid 14 のオーバーレイの動き、スレッドの順序)。node では 84 行のうち時間に依存する行が出る(段階 2 で確認: lid 14 の行が変わる、ダイアログ lid 48 は 14/16 一致) | 比較を『画素のハッシュ』から『時間に依存しない行 + 許容つき』へ作り直す: 2〜3 日 |
| a4 FRUITY off 942/942、895/895 | ネイティブの協調実行 | ネイティブ(`FLE_GIL` なし)の試験はそのまま。wasm の実スレッドとは別物 | 変更なし(ただしネイティブは経路が 2 本のまま: ネイティブ試験用) |
| パリティ 10 セッション 552 行 | 仮想時計 | 同一性は崩れる。実時計のもとで『落ちない・画面の主要要素が出る』の試験に変える | 3 日 |
| e2e(ページ) | idle driver + 仮想時計の pacing | 実時計では pacing の試験が不要になり、入力の配送の試験は残る | 2 日 |
| フル試験(cargo、2,152 件通過) | 影響なし(lifter 側) | 変更なし | 0 |
| 音(1 拍目の欠け、0241) | 単一 Worker | 段階 C の後に Chrome/Firefox で再試験 | 0.5 日 |
| 性能(P1 起動 10 秒、P2 120 fps、P3 音切れ無し) | 単一 Worker | 下の 3 | — |

**判断の材料**: 経路を 1 本にすると決定的な試験が使えなくなる(協調モードの 84 行同一性が最大の損失)。決定的な試験を残すには『ネイティブ/node の協調モード』(本番の wasm とは別の経路)が要り、PM の無オプションの方針と衝突する。選択肢: (a) 実スレッド版を既定にし、試験は実時計用に作り直す(上の表の合計 約 8〜9 日)、(b) 本番は単一 Worker のまま、実スレッドは別の成果物にしない(P3 は別の手段)。

## 3. 測定で分かっていること(Chrome 156、この機械、他の担当の負荷 5〜28)
- 非ハイブリッド(GIL)の実スレッド版: 参照プロジェクトを開いて再生でき、音が出る。テンポ(PCM と FLP のクリップ位置で当てはめ)は 0.29〜0.32(残差最大 0.4〜0.9 s。負荷 10〜28)。ミキサーの Worker の CPU は 57%、UI の Worker は 94%(いずれも 1 本の Worker)。ミキサーは CPU を使い切っておらず、待ち(GIL、イベント)が約 4 割。UI は描画で 9 割。
- つまり『ミキサーを専用 Worker に置く』だけでは P3 は満たせない。理由は 2 つ: ① 非ハイブリッドの CPU は 2.1 CPU 秒/仮想秒(UI + ミキサーの合計)で、1 本のミキサーでは実時間に届かない。② FL の複数コアのミキシングは、ランタイムが 1 プロセッサを報告し、`CreateSemaphoreW/ReleaseSemaphore` が未実装のため使われない(プロセッサ数を増やすと `unsupported instruction ... CreateSemaphoreW` で止まる: 実測)。
- 次の手: **ハイブリッド + GIL**(CPU −39%)、**セマフォの実装**(再 lift が要る: 取り込みの import 表に無い API)、GIL の保持時間の短縮(UI の描画で GIL を握る時間)。
