# mt3: ハイブリッド + GIL で見つかった GIL の護送(convoy)と、その直し(PM 2026-11-05 17:20 の指示)

状態: **作業木 h6 での実装・試験・測定。納品物(旗なしの差分)にはまだしない**。測定は §4 に追記する。

## 1. 症状(ハイブリッド + GIL、旧 GIL、Chrome 156、負荷 3〜6、再生中の 10 秒あたり、2 回)
- GIL の保持: W0(UI)約 1.0 s、W1(ミキサー)約 0.25 s = 合計 約 13%。待ち: W0 5.8〜6.3 s(60%)、W1 2.8〜4.6 s(28〜46%)。Worker の CPU は UI 57〜59%、ミキサー 41〜54%。ミキサーの書き込み率 0.27〜0.50。
- 全体(80 s): W0 の取得 1,667 万回(平均保持 0.5 µs)のうち遅い取得(待った取得)20.8 万回、待ち合計 21.7 s。そのうち 10 ms 以上の待ちが 662 回。W1 は取得 69 万回のうち 41% が遅い取得、待ち合計 13.2 s、10 ms 以上が 385 回。
- 非ハイブリッド(旧測定): W0 の待ち 16〜21%、W1 8〜12%。ゲストコードが速くなって runtime の呼び出しが密になると顕在化する。

## 2. 見立て(確定ではない。A/B で確かめる)
- 旧 GIL: 速い経路は『待ち手が一人もいない』ときだけ(`nwait == 0`)。待ち手がいれば全員が遅い経路に入り、**優先度 + 到着順の先頭の待ち手だけ**が取れる。先頭の待ち手が眠っている(W0 は JSPI の `Atomics.waitAsync` で、再開にイベントループ 1 周かかる)と、ロックが空いていても、起きて走っている待ち手も取れない。ロックの『渡し』ごとに起床の遅れが入る護送。
- ロックが空いている間にも誰も取れない時間があれば、『保持 13% なのに待ち 50%』が説明できる。保持の長さ(最長 W0 45.8 ms、W1 16.6 ms)は別の問題として残る(§5)。

## 3. 直し
(1) 速い経路を `nwait` と無関係な CAS にする(割り込み: barging)。(2) 『先頭』の判定は**起きている**待ち手だけで行う(眠る前に自分の鍵を 0 にする。眠っている待ち手は誰も止めない)。(3) 眠っている待ち手がいるときだけ起こす(`fle_gil_nsleep`)。眠る前に `nsleep` を足してからもう一度ロック語を見る(その間に解放されていれば眠らない: 起床漏れを作らない)。
GIL は Windows の契約ではなく runtime を安全にするための装置なので、公平さ(優先度順・到着順)は性能と安全の道具で、契約ではない(PM)。優先度は『起きている待ち手の間』の順序として残る。
- **barging でも守られる不変条件**(`fle_gil_assert` の表明は残す): (I-a) 同じ Worker が持っているのに取らない(取ると名前つきの停止)。(I-b) 持っていないのに放さない(名前つきの停止)。(I-c) 解放は必ず `nsleep` を見て起こす(眠っている待ち手が残らない)。(I-d) JS のイベントループを同期で待つ Worker は GIL を持たない(I1: 0234i §2.3。変わらない)。
- GIL を取らないと壊れる状態の表(0234i §4)は変わらない。変えたのは**取り方**だけで、取る場所(`fle_imp_*` の入口、`fle_progress_slice`、export)は同じ。

## 4. 葉の import は GIL を取らない(同期の通信の削減。(a) の除去でも (b) の契約の実装でもない)
- 方針: runtime の共有状態を読みも書きもしない import だけを、`runtime/fle_gil_lockfree.inc`(X マクロの一覧。fle_imp 側のデータ)に置く。`harness/gil/gen_wrap.py` がその一覧を読み、一覧の名前は GIL を取らない通過の包みにする。FL のクラス名・関数アドレスでの分岐は無い。
- 一覧と、関数ごとに読んだ理由(表):
| import | 触る物 | 共有状態か | 判断 |
|---|---|---|---|
| IntersectRect、UnionRect、SubtractRect、OffsetRect、InflateRect、SetRect、EqualRect、PtInRect(`fle_user_nowin.c`) | R[]・スタック引数・引数が指すゲストメモリ(`fle_ld/st`)。停止経路は `fle_stop` | 無い | 取らない |
| GetTickCount、timeGetTime、QueryPerformanceCounter、QueryPerformanceFrequency(`fle_sys.c`) | 実時計の読み: `performance.timeOrigin + performance.now()` と、Worker 間で単調にするアトミックな最大値(`fle_clock_real_now`)。実時計の起点(`real_origin_ms`、`real_base_elapsed`)は第 2 の Worker が起動する前に書かれる。出力はゲストメモリ | 読みが共有、書きは CAS のみ | 取らない。仮想時計(`clock_real == 0`、1 Worker だけ)では読みが時計を進める(`fle_clock_elapsed` への書き)が、その時は他の Worker が無い |
| GetCurrentThreadId(`fle_imports.c`) | 自分の Worker の `cur`(thread-local)と `nthr`(増えるだけ。32 ビットの読み) | 無い | 取らない |
| (取るままのもの) | EnterCriticalSection / LeaveCriticalSection(CS の表とスケジューラ)、Sleep、GetThreadPriority(スレッド表)、SysAllocStringLen(ヒープ)、MultiByteToWideChar(最終エラー)ほか | あり/未確認 | 取る |
- 試験: `crates/fe-lift/tests/gil_lockfree.rs`(2 件): 一覧の各名前が imports_table の行で、runtime/ の 1 か所で定義されること; **各関数の本体(同じファイルの呼ばれる関数を再帰して)が、読んだ葉の呼び出し(`fle_ld/st`、`fle_stop`、`fle_arg`、アトミック、`fle_clock_real_now`、`fle_thread_current` …)だけを呼ぶこと**(新しい呼び出しが入れば落ちる = 見直しが必要)。実際の多スレッド負荷はブラウザの実エンジンの走行(IntersectRect 約 950 万回/80 s、GetTickCount 約 207 万回)で掛かっている。
- 協調モード(a4 / 84 / 55 行の同一性): この変更は `GIL=1` のビルドの包み(`fle_gil_wrap.c`)にだけ入り、協調モードのコードには入らない(`FLE_GIL` が無ければ包みは空)ので、それらの同一性は変わらない。再確認は納品物にするときに行う。

## 5. 残る問題
- 保持の長さ: W0 の最長 45.8 ms、W1 の最長 16.6 ms。どの import が長く持つかは、まだ測っていない(import ごとの最大保持を次のビルドの計測に足す)。

## 6. 測定(Chrome 156、ハイブリッド + GIL、参照プロジェクト、再生中の 10 秒の区間。**この機械の負荷は 6〜24(他の担当のビルドが同居): 比と構造だけを読むこと**)
表: `tables/ab1-gil-fix123.txt`(A 旧 GIL / B 直し 1〜3)、`tables/ab2-A-B-E.txt`(A / B / E 各 2 回)、`tables/ab3-E-B-with-schedstat.txt`、`tables/lat1-input-latency.txt`。E = B + 『全 Worker が同期で待つ』+ 『葉の import は GIL なし』。
| 構成 | W0(UI)の GIL 待ち/10 s | W1(ミキサー)の GIL 待ち/10 s | UI の WaitMessage(空き)/10 s | ミキサーの書き込み率(密な区間) | Worker CPU(UI / ミキサー) |
|---|---|---|---|---|---|
| A(旧 GIL) | 5.7〜6.4 s | 2.2〜4.4 s | 0.5〜0.9 s | 0.27〜0.34 | 57〜60% / 46〜54% |
| B(直し 1〜3) | 4.4〜6.2 s | 0.2〜0.6 s | 0.3〜0.6 s | 0.36〜0.59 | 51〜71% / 46〜59% |
| E(B + 同期待ち + 葉の import) | **0.2〜0.8 s** | **0.2〜0.8 s** | **1.6〜5.8 s** | 0.24〜0.54(負荷 17〜23 の回は 0.24〜0.45) | 53〜69% / 42〜67% |
- 読み: **GIL の待ちは A/B の 60% から E の 2〜8% へ下がり、UI は飽和しなくなった(空きが 0.5 s → 2〜5 s/10 s)。しかし書き込み率は 0.95 に届かない(E 0.24〜0.54)。**
- スケジューラの待ち(ab3、負荷 19〜23): 各 Worker が『走れるのに CPU が無い』時間は 3〜6% で、OS に割り当てられていないわけではない。ミキサー Worker の内訳(E、10 秒あたり): CPU 67%、走れるが CPU 無し 5%、Sleep(1) 14〜24%、GIL 待ち 3〜5%、残り(CS 待ち・JS 呼び出し等)約 8%。= **ミキサーは CPU を使い切っておらず(67%)、GIL に待たされてもいない。それでも書き込み率が 0.4〜0.5 なのは、1 コアの 100% を使っても 0.64 倍の実時間(0.43 / 0.67)しか出ないこと = 1 CPU 秒あたり 0.64 秒分の音しか作れないこと**を意味する。ミキサーの 1 本のスレッドの速さが上限で、GIL を直しても届かない。
- 入力→応答(マウス移動 93 回を再生の直後に送る。`lat-sum.py`): 配送(ページのリスナー → エンジン Worker がメッセージを受け取る)の中央値は A 31〜41 ms、B 34〜35 ms、E 39〜63 ms。応答(最初の答えのフレームまで)の中央値は A 61〜67 ms、B 73〜76 ms、**E 113〜150 ms**(p95 は A 161〜171、B 145〜171、E 254〜468)。**E は悪化している**(負荷が荒れていて 2 回ずつなので確度は低い)。機構の仮説(未検証): E ではエンジンの UI が飽和しなくなり、時間あたりのフレーム数が増え、ページの主スレッド側の present が詰まるのではないか。(a)(b)のどちらの影響かは分けていない(D のビルドが無い)。
- I1(`check_i1.py`、E のビルド): 通過(`tables/check_i1-after-sync-wait.txt`)。W0 が同期で待っている間に他の Worker が W0 のイベントループを必要とする経路は無い(proxy される import は列挙済みで、すべて『起動時・終了時・NO_FS の stub』)。

## 7. プロセッサ数 > 1 の実験と、x87 の共有 static(18:45)
- 5 分(E 相当、Chrome 350 s 生存): エンジンの書き込み率 0.21〜0.30(10 秒ごと、負荷 16〜26 で他のビルドが同居。ミキサー Worker は CPU 55%・走れるが CPU 無し 22%)。表: `tables/gate-300s-hybrid-gil-E-*.txt`。ゲート(≥0.95)には届かない。
- 静かな回の読み: ミキサー Worker の CPU 62〜67%、GIL 待ち 2〜5%、Sleep 14〜24%で書き込み率 0.4〜0.5。= 1 本のスレッドが 1 コアを使い切っても 0.5〜0.65 倍の実時間(この参照プロジェクトは 1 秒の音に 1.5〜2 CPU 秒)。(この見積りは CPU の割合と書き込み率の比で、直接測った『CPU 秒/周期』ではない。)
- → FL 自身の複数スレッドのミキシングを使う実験: `FLE_PROCESSORS=n`(作業木の足場。製品ではページが `navigator.hardwareConcurrency` を事実として渡す)で GetSystemInfo の `dwNumberOfProcessors` / `dwActiveProcessorMask`、GetLogicalProcessorInformation の RelationProcessorCore の記録数、GetProcessAffinityMask のシステムマスクを n にした(`fle_sysinfo.c`、`fle_sys.c`)。セマフォは h6 に実装済み(0234o)。
- n = 4: FL は起動途中で `fle: x87: FSINCOS: cosine rounding is not decided at 256 bits [operand se=3ffe m=c90fdaa22168c000]`(= 53 ビットに丸めた π/4)で停止。同じ演算数を native と emcc の -O0/-O2/-Oz の単独のプログラムで流すと正しく丸められる(cos b504f333f9de6614)。= 演算でなく**同時実行**の問題。`fle_x87.c` の `static unsigned X`(その命令の例外ビット)と `keep_snan` が全 Worker 共有で、超越関数の丸めの窓が『`X = 0` にして 2 回丸め、結果と例外ビットまで比べる』ので、間に別の Worker の x87 命令が `X` を書くと 2 つが食い違う。**`_Thread_local` にした**(2 行)。n = 1 でも UI とミキサーの x87 の例外ビットが混ざり得た。
