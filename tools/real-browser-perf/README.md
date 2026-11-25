# 実ブラウザでの再生計測

ローカル開発サーバーを browse 経由の接続済みブラウザで開き、評価プロジェクトを読み込んで Song Mode で再生する。同一区間を比較する場合は停止ボタンを二度押して先頭に戻す。対象タブを表示状態に保ち、ウインドウの前面・背景や画面ロックの条件も記録する。

```sh
node tools/real-browser-perf/measure.mjs signature-perf none 30000 1
node tools/real-browser-perf/measure.mjs signature-perf center 30000 1250
node tools/real-browser-perf/measure.mjs signature-perf pointer-toolbar 30000 1250
node tools/real-browser-perf/interaction.mjs signature-perf 700 560 1000 560 20 40
```

第1引数は browse の自分のタブID、第2引数は `none`・`center`・`toolbar`・`pointer-center`・`pointer-toolbar`、第3引数は計測時間(ms)、第4引数は入力頻度(Hz)。結果は標準出力のJSON。接続されたブラウザに対して実行するため、同じタブの他の操作は計測完了まで待つ。

- `center` と `toolbar` は共有入力リングへの合成入力であり、物理マウスの計測ではない。実際に送信できた頻度も結果に含む。
- `pointer-center` と `pointer-toolbar` はページの入力処理を通るDOM合成PointerEventを使う。`dispatchedDomHz` はDOM送出数、`deliveredRawHz` は入力処理からエンジンへ送ったイベント数の増分による頻度。同じ座標のMOVEは入力処理でまとめられる。`isTrusted=false` であり、物理マウスの計測ではない。
- `displayFrames` は描画WorkerのrAF周期ごとに重複排除したcanvas更新数。GPUの完了やディスプレイへの物理的な表示回数を証明しない。未対応の場合は未測定。
- `meanCanvasFps` は全区間の更新率、`minOneSecondCanvasFps` は1秒以上の観測窓の最低更新率。各窓は実経過時間で計算し、短い最終窓は最低値から除く。直近60窓と窓総数も保存する。連続した観測窓の値であり、すべての開始位置での移動窓の最低値ではない。
- `endState` は終了時の状態。終了時にエンジンや音声が停止していたりファイルダイアログが開いていたりする場合は `ok=false` を返す。`ok=true` 自体は120fps・入力1000Hz・正常なSong再生の合格を意味しない。
- 音声はバッファ枯渇、欠落、遅延書き込み、非無音の処理回数を別々に読む。枯渇ゼロだけで連続した正しい音声を保証しない。
- `audioProducer` は新規書き込み量と消費量、バッファ長、書き込み位置を記録する。同一サンプルレートで比較する。元からループ再生する音源もあるため、この比率だけで音声の正しさは判定しない。累積フレーム数のない旧ビルドではログが欠けると比率を未測定にする。
- `clock` はidle報告とは独立した音声パルスから仮想時間の進行を読む。連続入力でidleへ戻らない場合も測定できる。
- 10秒以上の計測では、入力の開始と同じ処理で8秒の実出力を取り込む。`waveform` は非無音部分の4096/8192フレーム周期などでの完全一致率と、近隣周期の対照値を返す。ループの意図や元の音源も考慮する。元の音声はブラウザ外へ送らず、解析後にこの計測で作ったコピーを解放する。まだ取り込めていない場合は未測定。
- スレッド別の時間は切替間の経過時間で、ホスト側での待機を含む。CPU時間やCPUプロファイルではない。
- 不足プラグイン・不足サンプルがある場合は、完全なプロジェクトの合格判定に使わない。
- 同じビルド・解像度・再生区間で複数回比較し、起動直後の測定と定常再生を分ける。結果にはビルド識別子・画面サイズ・計測条件を含む。

`interaction.mjs` は `tab fromX fromY toX toY reps steps` を取り、measure と同じ precondition で通常の browse drag を交互方向に反復する。各 pointerdown/up 区間の `__app.deskStats.displayFrames` 差分、経過時間、その区間あたりの canvas 更新率を drag ごとに報告し、CLI call 間の待ち時間を平均へ混ぜない。counter が未対応または不正な場合は `null` と警告を返す。開始後の `__app.latency.records` だけを集計する。records は最大400件のため、入力数との差があれば警告する。

browse drag は CDP 経由の DOM pointer 入力であり、物理 mouse 入力ではない。canvas 更新率は物理表示 FPS を示さない。UI 要素が実際に動いたかは画面で別途確認する。

個人のプロジェクト、音声、ブラウザの保存データはリポジトリに保存しない。
