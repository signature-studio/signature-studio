# mt3: ミキサー(FL の音を作って DirectSound に書くスレッド)の見分け方(gpu1 へ。PM 指示 5)

## どのスレッドか
- FL のスレッドはすべて開始アドレス `0x412ae0`(ゲストのスレッド関数)から始まる。**ミキサー = DirectSound のバッファへ Lock / Unlock し、`Sleep(1)` で `GetCurrentPosition` を見に行くスレッド**。実エンジンのログで次の 3 点が同時に当てはまる:
  1. `parked thread N (start 0x412ae0, Worker W)` の行で `Sleep` が 1 万〜2 万回/130 s(1 回 約 1.1 ms)、`EnterCriticalSection` の待ちがある。
  2. クリティカルセクション **0x16b6aa8** を、Enter の戻り先 **0x12aefcd**、Leave の戻り先 **0x12af25a** で、**1 回 150〜240 ms、約 5 回/秒** 保持する(再生中の 10 秒のうち 8.4〜9.0 秒。`longest hold: cs 0x16b6aa8 thread N ... enter-ret 0x12aefcd`)。保持の 94〜95% がゲストコード、runtime の import は 3.5〜5%。
  3. `fle_dsound` の Unlock を呼ぶスレッド(`unlock_bytes` が増える)。
- 番号は作成順で走行ごとに変わる: 実スレッド版(`FLE_WORKERS=12`)の参照プロジェクトの走行では thread 17(Worker 1)または 18、協調版では thread 18(起動の直後は thread 7 が同じ役)。**番号でなく上の 3 点で見分けること**。
## 実スレッド版では 1 Worker = 1 スレッド
- ミキサーのスレッドは第 2 の `DedicatedWorker`(`ps -L` や thr-sample で、UI の Worker の次に CPU を使う Worker)で、その Worker の V8 CPU プロファイルがミキサー単独の内訳になる: `FL_WORKER_CONSOLE=1 FL_PROFILE=1` で `prof-N.json`(N は attach の順: W0 の次の 1、2 …)。最適化版の wasm は関数名が `wasm-function[N]`(名前セクションが無い)なので、読むには `--profiling-funcs` つきのリンクか、生成 C の関数 `f_<VA>` の名前つきのビルドが要る(`tools3/prof-sum.py` は名前をそのまま出す)。
- 負荷の見積り(この機械、ハイブリッド + GIL、再生中): ミキサー Worker の CPU 55〜67%、書き込み率(エンジンが DirectSound に書いた量 / 実時間)0.42〜0.54 → 1 秒の音に 1.0〜1.6 CPU 秒(静かな回)〜 2 CPU 秒(負荷 16 以上の回)。**この数は CPU の割合と書き込み率の比であって、直接測った『CPU 秒/周期』ではない**。
## 取り方(ミキサー・スレッド別の上位 30 関数)
- 実スレッド版(GIL)の bundle をブラウザで走らせ、参照プロジェクトを開いて再生(`tools/scn-ref3-tempo-relaxed.json`)、再生中の 25 秒の窓で第 2 Worker のプロファイルを取る。協調版(1 Worker)では UI とミキサーが同じスレッドなので、プロファイルは混ざる(ミキサーだけの内訳は `FLE_CS_HOLD_STATS` 版の `longest hold` の保持区間で見るしかない)。
