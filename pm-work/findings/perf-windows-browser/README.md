# perf/windows-browser（Discussion #1356）から拾った知見

依頼者の指示（2026-11-25）で、作業ブランチ `perf/windows-browser` は打ち切られ、マージされず削除される。ここにはブランチから使える知見（測定値・仮説・測定の方法・反省）と、コードの写し（patch）を残す。全文の記録は `discussion-1356.md`。

- 対象: `origin/perf/windows-browser` = `a40fc7e3`（merge-base は `999bc651`。32 commit・46 ファイル）。main は `87660bb0`。
- 注意: 終了報告が挙げる 5 commit（`573e57af`・`f95bf134`・`0e475c70`・`7b032631`・`86db2369`）は origin に無い（報告の上ではローカルだけに在る）。取得できないので、この 5 件の内容は報告の記述だけで、コードは無い。
- 結論（報告の自己評価）: P1・P2・P3 の達成は無い。最低 fps は 4.95 fps（目標 120）。音声の欠落は 0 だが、PCM の反復で「正常な音」は確認できていない。

## 1. 実機の測定値（報告の値。個人の評価用データはリポジトリに無い）

| 指標 | 20 秒の窓（最後の有効値） | 10 秒の窓（その前） |
| --- | --- | --- |
| 実配送の入力 Hz | 1169.34（DOM 送出 1249.30） | 1172.84（送出 1248.76） |
| 平均の canvas 更新 fps | 7.65 | 10.69 |
| 1 秒窓の最低 fps | 4.95 | 9.90 |
| 音声の欠落増分（underrun・gap・skip・late） | 0 | 0 |
| PCM の 8192 フレーム前との完全一致率 | 0.000517（約 0.05%） | 0.5989 |

- 条件: DPR 1.75、ゲスト 3247×1764（5.73 Mpx）、ブラウザの rAF 約 240 Hz、エンジン設定 120 Hz、Song Mode。欠落プラグイン 5 種（Fruity Wrapper・FLEX・Frequency Shifter・Soundgoodizer・Fruity Parametric EQ 2）あり、完全な再生ではない。
- 入力は CDP の DOM 合成 PointerEvent（`isTrusted=false`）。物理マウスではない。
- エンジンは Wasm `f3e6a0068bef`（main の #1351 の描画面積の縮小などを取り込む前）。**今の main の実機の値ではない**。
- 2 つの窓は条件を揃えた比較ではない（窓・再生位置が違う）。改善率や退行率は出せない。
- 起動（新版・既定プロジェクト・ローカルの機能確認用 bundle）は最初の画面まで 7.875 秒。旧版の 136.7 秒とは条件が違う。

## 2. 原因の仮説と修正：ブランチの状態と main（87660bb0）との比較

「根拠」は報告の記述。実測の数字が在るものを「実測」、推定だけのものを「推定」とした。

| # | 仮説・修正 | ブランチ（commit・patch） | main | 根拠 | 扱い（Issue 案） |
| --- | --- | --- | --- | --- | --- |
| 1 | ゲストが実行を続けると Worker のイベントループへ戻らず、描画要求が届かない。要求をスケジューラ地点で受ける | `d0bb1004`・patches/01 | 無い | 実測（小規模で連続入力の描画が大きく改善。数値の記録は無い） | 181 に含める |
| 2 | 描画通知を 100 ms まとめない（入力→rAF の値に通知待ちが加算された） | `a83bb56a`・patches/01（render-worker.ts） | 有（100 ms の集約が残る） | 実測（入力→rAF の中央値 6〜7 ms、p95 は 177・37・36.5 ms。長い停止は残る） | 181 に含める |
| 3 | 連続入力中、仮想時計を実行中のスケジューラ地点でも実時間に合わせる | `eb571d4c`・patches/01（idle-driver.ts・fruity.ts） | 無い | 実測（仮想時間／実時間 ≈ 1。数値の記録は無い） | 181 に含める |
| 4 | ファイル選択の待ちを、ゲスト全体を止めない協調的な待ちにする | `a566f0e4`・`f7fd1987`・`4e67f253`・patches/04 | 無い（JSPI で同期的に待つ） | 実測（旧版: 97.8 MiB の回答が約 544 秒後に処理され、転送が 1.3／97.8 MiB で停滞。修正後は全量読込と非無音出力を確認） | 185 |
| 5 | 再生状態を、音声データの準備の後に公開する | `99cb6e35`・patches/02（fle_dsound.c の m_b_play） | 無い（ring_set_state → audio_sync の順） | 推定（起動時の欠落 4 quanta との関係は未確認） | 184 に含める |
| 6 | DirectSound の通知位置を、サンプル境界だけでなくバイト単位にする | `556d5cc9`・`bba9a7e1`・patches/02 | 無い（align の倍数だけ受理） | ネイティブ試験のみ。指定の FLP への影響は未確認 | 184 |
| 7 | 深度 2 の WndProc／TIMERPROC だけ切り替えを許し、スレッド優先度を既定で有効にする | `83a5b3d3`・patches/03 | 無い（既定は優先度無効・深度 1 だけ） | 探索（単発）: 深度 2 を全面的に許した実験は、無入力で新規書き込み／消費 0.4991→0.9990、8192 一致 47.17%→1.63%。限定版は未測定 | 183 |
| 8 | 保留中の連続した MOVE を 1 件にまとめる（満杯で入力が落ちるのを防ぐ） | `ce2f23ce`・patches/05 | 無い（EV_MAX=512 で満杯なら捨てる） | 推定。ネイティブ試験のみ | 182 |
| 9 | 切り替えを延期した経路でも、公開済み音声をポーリングする | `2666b5e4` | 有（fle_thread.c の fle_audio_poll_in_callback） | — | 対象外（main に在る） |
| 10 | JSPI のスレッド切り替えが microtask だけを連鎖し、ホストの通常の message を占有する（4 ms ごとに返す） | 終了報告の `f95bf134`（origin に無い） | 無い | 実測（旧版: 空の guestText 要求が 1 秒以内に返らない。新版は約 1.91 ms だが別条件） | 候補。コードが無いので Issue にしない |
| 11 | rAF を測れない起動時に、エンジンの Hz が 60 のまま固定される（120 の目標を保つ） | 終了報告の `f95bf134`（origin に無い） | 無い（hz.ts は未測定なら 60） | 観測（engineEnvHz が 60 に固定。表示が高 Hz になった後も） | 候補。headless の runner では再現しない（実機が要る） |
| 12 | 32bit・一定色の矩形の塗りを、行ごとに直接書く | `75a5f538`・patches/06 | 無い | ネイティブ microbenchmark で約 52% 短縮。製品の fps は未測定 | 候補（次の波） |
| 13 | 初画素の診断から全画素の集計を外す | `74a5f953` | 有（初画素の 1 回だけ） | 1 回限り。効果は小さい | 対象外 |
| 14 | 遅延読込のプラグインの呼び出しを、JS の中継を通さず Wasm の中継で行う（JSPI を保つ） | `56609226`・patches/07 | 無い | 正しさの修正（非同期待機中の SuspendError の回帰試験）。性能の効果は未測定 | 対象外（正しさ） |
| 15 | VCL の CM_ACTIONUPDATE が子コントロールを再帰的に巡回する経路の費用が大きい | 報告 2〜4（CPU プロファイル。コードは無い） | 関数名表との照合は未了 | 実測（CPU プロファイル）。関数の特定は未完 | 候補（次の波。関数名表と照合してから） |

## 3. 測定の方法と指標（再利用するもの）

- **PCM の異常な反復**（`patches/` と `tools/spec/real-browser-waveform.test.mjs`、`tools/real-browser-perf/waveform.mjs`）: 無音近傍（最大振幅 < 1e-5）を除いて、左右の両サンプルが lag 前と完全一致する割合。lag は 4095・4096・4097・8191・8192・8193・12288・16384・32768 で、隣の lag は対照。8192 だけが高いと、古いバッファの循環を疑う。**ただし、ループ音源は正しく反復する。反復率だけで音の正しさを判定しない**（報告の注意）。
  - 発見: 音切れのカウンターが 0 でも、古いバッファを循環していれば正常ではない（報告の表: 無入力で新規書き込み／消費 0.4991、一致 47.17%）。
- **1 秒窓の最小 fps**（`3f8a98bc`・`measure.mjs` の `minOneSecondCanvasFps`）: 平均でなく、1 秒以上の観測窓の最小を見る。短い最後の窓は外す。rAF の回数を UI の fps と呼ばない（canvas 更新は描画 Worker の rAF で重複排除した数。GPU の完了や物理表示は示さない）。
- **入力の実配送 Hz**（`measure.mjs`・`interaction.mjs`）: DOM 送出数と、入力処理がエンジンへ送った数を別々に数える。
- **音声の書き込みと消費**（`bba9a7e1`）: 書き込み量／消費量の比。unlock の bufferFrames・offset を記録する。ただし C ランタイムの変更（patches/02）が要る。
- **仮想時間／実時間**（`clock`）: 音声のパルスから読む。idle に戻らない連続入力でも測れる。
- **評価用フォルダの取り込み**: OPFS へ Upload folder で丸ごと入れ、全ファイルのサイズと SHA-256 を照合した（76 ファイル・107,503,126 バイト）。プロジェクトはリポジトリに入れない。
- **browse での測定**（`tools/real-browser-perf/`）は実機の Windows Chrome 専用。runner（GitHub Actions）では `tools/p2-frames/`・`tools/p3-audio-probe/` を使う。両方の差は、Issue 180 で移植する。

## 4. 反省（報告から。やってはいけないこと）

1. 目標を時間で区切った判断基準を持たず、最低 fps が一桁のまま、テスト通過と統合の進捗で作業を続けた。
2. 同じ FLP・同じ窓・同じ入力の比較を後回しにし、準備操作（Open・警告）に時間を使った。
3. 仮説を一度に複数動かした（main 統合・優先度・yield・120 Hz）。効果の因果を切り分けられなかった。
4. 35 分の再ビルドの前に、期待できる改善幅と棄却条件を狭めなかった。
5. ビルド成功・テスト通過を、性能の合格として報告した。

## 5. 保存したもの

- `discussion-1356.md`: Discussion の本文と全 3 コメント（作成・更新の時刻つき。個人の評価用のファイル名・パスは伏せた）。
- `patches/`（テーマ別。`git diff 999bc651 origin/perf/windows-browser -- <paths>`。各 300 kB 未満）:
  - `01-scheduler-point-requests-clock-canvas-count.patch`: 描画要求の受付・描画通知・仮想時計・canvas 更新の数（app/src/engine・app/src/perf・テスト）。
  - `02-audio-unlock-dsound-notify-play-order.patch`: 音声の unlock の情報・DirectSound の通知・再生の公開順（lift/runtime の音声・ds_scenarios・app の audio）。
  - `03-preempt-depth2-dispatch-priority-default.patch`: 深度 2 の許可・WndProc／TIMERPROC・優先度の既定（fle_callback・fle_thread・fle_win）。
  - `04-filedlg-nonsuspending-poll.patch`: ファイル選択の非同期化（fle_filedlg.c・fle_filedlg_lib.js・spec）。
  - `05-input-pending-move-coalesce.patch`: 保留中の MOVE の統合（fle_input.c・g3 の試験）。
  - `06-raster-constant-fill-row-path.patch`: 32bit の一定色の矩形の行塗り（fle_raster.c・raster.rs）。
  - `07-xmod-wasm-relay-jspi.patch`: 遅延読込の Wasm 中継（xmod.ts・ott-host.ts・試験）。
  - `08-fe-lift-wall-tests.patch`: fe-lift の壁の試験の追随（wall_*.rs）。
- `tools/`: `real-browser-perf/`（measure・interaction・interaction-aggregate・waveform・README の写し）と `spec/real-browser-waveform.test.mjs`（写し）。

## 6. この知見から作った Issue（下書き 180〜185）

- 180（計測）: `tools/p2-frames` に 1 秒窓の最小 fps（`minWindowFps`）と入力の落ち数（`inputDropped`）、`tools/p3-audio-probe` に PCM の 8192 反復（`pcmRepeat8192`）を足す。
- 181（P2）: 描画要求をスケジューラ地点で受ける・描画通知を 100 ms まとめない・仮想時計を実時間に合わせる（#2・#1・#3）。
- 182（P2）: 保留中の連続した MOVE を 1 件にまとめる（#8）。
- 183（P3）: 深度 2 の限定的な切り替えと、優先度の既定（#7）。
- 184（P3）: DirectSound のバイト単位の通知と、再生の公開順（#5・#6）。
- 185（P3）: ファイル選択を、ゲストを止めない待ちにする（#4）。

取り込まない候補: #10（コードが origin に無い）・#11（runner の headless では再現しない）・#12（製品の fps 未測定。次の波）・#15（関数の特定が未了）。

## 7. main との突き合わせ（確認した箇所）

- `lift/runtime/fle_input.c`: `if (ev_n >= EV_MAX) return 0;`（MOVE の統合は無い）。
- `lift/runtime/fle_thread.c`: `static int prio_on;`（既定は優先度無効）。`fle_progress_slice` は深度 2 を許さない。
- `lift/runtime/fle_win.c`: `dispatch_message` に深度 2 の許可は無い。
- `lift/runtime/fle_dsound.c`: `m_b_play` は `fle_audio_ring_set_state` の後に `audio_sync`。通知は `off % align` で弾く。
- `lift/runtime/fle_filedlg.c`・`lift/harness/web/filedlg/fle_filedlg_lib.js`: `fle_filedlg_host_pick__async: true`（JSPI で同期的に待つ）。
- `app/src/engine/render-worker.ts`: `flushDesk` に 100 ms の集約が残る。`ask()` は `gate.request()` を持たない。
- `app/src/engine/hz.ts`: 未測定なら `HZ_DEFAULT = 60`。
- `tools/p2-frames/measure.mjs`: 1 秒ごとの `presents` を持つが、最小窓の出力は無い。`tools/p3-audio-probe/measure.mjs`: PCM の取り出しは無い（`armCapture` は `app/src/audio/page-audio.ts` に在る）。
