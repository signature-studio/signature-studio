# 0250(仮): FL の定常 CPU(`f_863f60`)の出どころを外部 API の契約側から調べた(alpha4)

**依頼者の機械・実 GPU・実機 Windows・Chrome での測定は 0 件。** 以下の数値は、HEAD b42fad36 の生成 C から作った native N1(ランタイム -O2、生成 C -O1。FL 本体はリフト済みコードのまま)の G3 模擬入力(依頼者の操作ではない)で測った値だけ。代役・画像の再生・推測は使っていない。

## 0. 先頭の 3 点

**測ったこと**

- Basic.flp を開いた後の idle は 1 virtual 秒あたり **0.091 CPU 秒**(20 s)。自己時間は `f_410010` 23.8%、**`f_863f60` 13.1%**、`f_5287c0` 10.3%。`f_863f60` を含むサンプルは 73%(`tables/prof-basic-idle-20s.txt`)。
- 参照プロジェクト(serina1-serum2.flp。この build に Serum 2 のモジュールは無い。場面は開いてから Enter を 1 回押し、その後 10 s idle)の idle は 0.449 CPU 秒/virtual 秒、`f_863f60` は自己 16.7%・包含 76%。30 s 再生は 1.453 CPU 秒/virtual 秒(UI スレッド 0.958、スレッド 18 = ミキサー側 0.487)で `f_863f60` は自己 8.1%・包含 33%(`tables/prof-ref-*.txt`、`tables/cpu-per-virtual-second.tsv`)。
- `f_863f60` の入口は **SendMessageW(TApplication 窓, 0xB03F = CM_ACTIONUPDATE) 2002〜2005 回/s**。これは idle 周期 125.2/s × 16 個のアクション。1 周期は FL の `f_86a250`(VCL の HandleMessage の形: メッセージが無ければ `f_86b440` = Idle。Idle は OnIdle のあと、`Application.ActionUpdateDelay`(+0x164)が 0 以下なら直ちに `f_86b300` = DoActionIdle を呼ぶ。i03 で読んだ値は **0**、ActionUpdateTimer も 0。`logs/state-words-i03.txt`)の中の `f_86b300 → f_863c20 → f_863b50 → f_60e9a0 → f_528d60 → f_5de830 → f_604f60 → SendMessageW` で、受け側の窓手続きの中で `f_60c3e0 → f_410090 → f_864030 → f_863f20 → f_863f60`(制御ツリーの再帰走査)に入る(gdb の bt。`logs/bt-sendmessage-actionupdate.txt`)。1 回の CM_ACTIONUPDATE は native で約 32 µs(= f_864030 の包含 65 ms/virtual 秒 ÷ 2002)、1 idle 周期(16 個)で約 0.52 ms。**`f_863f60` は 102,000 回/s**(CM_ACTIONUPDATE 1 回あたり 51 回。同じ経路の `f_863f20` は 900,000/s、`f_410010`・`f_40fe90`・`f_6113f0` は各 約 1.2M/s。プロセス内の int3 カウンタで計数)で、自己時間は約 117 ns/回。
- idle 周期が 125/s になる起こし元は 2 つで、どちらも FL 自身: (a) QuickFontCache の TTimerThread(guest スレッド 6)が 16 ms ごとに `PostMessageW(0x8ffc)`(62.7/s)、(b) その 0x8ffc の処理が立てる旗を主フォームの OnIdle(`f_11f0270`)が見て、x=377 の 1 画素幅の線に `RedrawWindow(RDW_INVALIDATE)` し、WM_PAINT が 63.8/s 来る(`tables/message-rates-idle.tsv`、`logs/bt-redrawwindow-steady.txt`、`logs/bt-post8ffc-sleep16.txt`)。1 tick(16 ms)に idle 周期が 2 回、各回で 16 個の CM_ACTIONUPDATE が走る(同じ仮想 ms に 32 個、次の tick にまた 32 個)。

**結論**

- **こちらの外部 API の答えが FL に余計な仕事をさせている証拠は見つからなかった。** CM_ACTIONUPDATE の部分木に runtime API 呼び出しは無く(guest 関数 + SendMessageW の入口だけ)、頻度を決める入力(SetTimer、GetTickCount/QPC/timeGetTime、PeekMessage/GetMessage/WaitMessage、WM_PAINT の作り方、GetSystemMetrics)は 契約と合っているか、idle の経路で呼ばれていない(`tables/contract-vs-ours.tsv`)。したがって (a)「余計な処理の除去」にあたる物は無く、`alpha4.diff` は作っていない。
- 契約の範囲内で、こちらが**許される値の端**(または未実装)を選んでいる箇所が 2 つある。どちらも頻度を下げる方向の差で、決定的な計数と、CPU の上限見積もりを出した。
  1. `Sleep(n)` は我々は**ちょうど n ms**。Learn の Sleep は「1 tick 超 2 tick 未満なら 1〜2 tick の間」で、FL は `timeBeginPeriod(1)` を 2 回呼ぶ(QFC の `f_1e85e20`、FLEngine の `f_81c0d0`)ので tick は 1 ms、つまり `Sleep(1)` は 1〜2 ms、`Sleep(16)` は 16〜17 ms が契約。スレッド 18(FL 本体の DirectSound の再生位置のポーリング。`f_412ae0 → f_5251b0 → f_12aeec0 → f_5264e0 → Sleep(1)`、GetCurrentPosition 相当の仮想呼び出しを 1 ms ごと)が 1055 回/s で走るのは、この選び方による。使い捨ての実験(1 ms tick の次の境界に切り上げ)で Sleep は 8446 → 4482 回/8 s(-47%)。
  2. `GetProcAddress("DwmGetCompositionTimingInfo")` は NULL(response table に行が無い。決定 0122 の注と pick1 の capability 表 B31 が既に「Windows 10/11 では存在 = 差」と記録)。QFC の `f_1e857b0` は、これが成功すると正の遅延(`Sleep(遅延)` + `DwmFlush`)を選び、失敗すると EnumDisplaySettingsW の周波数(50 Hz 以上)から負の遅延(`Sleep` だけ。DwmFlush を呼ばない)を選ぶ。今は後者なので tick は `Sleep(16)` の 62.5/s で、`DwmFlush` は run の間 1 回も呼ばれない。実験(Sleep(16) のあとに次の 60 Hz 境界まで待つ): 0x8ffc は 500 → 479 回/8 s(-4%、59.9/s)。
- **CPU への効き**: 上の 2 つを合わせても、数えられる量の削減は スレッド 18 の Sleep が約半分(スレッド 18 は idle サンプルの約 10%)と tick の -4% で、**idle の約 9% 以下**の見積もり。CPU 時間そのものは、同じバイナリの 2 回が 0.1075 と 0.1177 CPU 秒/virtual 秒と 10% 近く揺れる(負荷平均 約 25 の機械)ので、実験版(0.093〜0.121)との差は**揺れの内**で、測れていない。
- Hz を上げると比例して増える(`FLE_DISPLAYSETTINGS_FREQUENCY=120`: 0x8ffc 999 回/8 s、SendMessageW 31968、CPU 0.1993 = 約 1.9 倍)。30 を与えても変わらない(FL のコードは 50 Hz 未満を無視して 16.67 ms にする)。つまりコストは tick 数に比例する FL 自身の仕事量で、**tick 数は実機の画面のリフレッシュレート(144 Hz なら 2 倍以上)と DwmFlush の実際の待ちで決まり、こちらの測定は 0 件**。

**未確認**

- 実機 Windows での Sleep(1) の実際の長さ、DwmFlush を含む tick の実周期、FL が実機で選ぶ音声出力(ASIO なら DirectSound のポーリング・スレッドは動かない。スレッド 18 の 10% は DirectSound を選ぶ我々の環境の事情かもしれない)、依頼者の画面のリフレッシュレート: すべて 0 件。
- Chrome(wasm)側の実回数: native は `f_86b300` 125/s(計数)。gpu1 の「約 61 回/s」はこれと合わず、上の突き合わせのとおりサンプル由来の回数と読んだが、Chrome 上の計数は 0 件。
- 旗 `g_16bca68` を毎 tick 立てる条件の値(`0x1403bd8`。`0x15e3e98` の指す先)は、i03 で **2**(PE ファイルの初期値と同じ)。つまり我々の答えで変わった値ではなく、実機でも ≥ 0 の見込みだが、この値が実機で別になる経路(他の書き手)は 全部は洗っていない(参照 65 か所のうち書き込みは未確認)。

## 1. 測定の方法(再現できる形で)

| 項目 | 内容 |
|---|---|
| 対象 | HEAD b42fad36 の生成 C(`native-gen.sh`、CI_JOBS=3)から `tools/build-n1b.sh` で作った native N1(`work/n1-vH`。ランタイム -O2、生成 C -O1)。`tools/alpha1_rec.c` の記録器に SIGPROF サンプラ(`A4_PROF=<file>`、1 ms の CPU 時間ごと)を足した。perf は `perf_event_paranoid=4` で使えない |
| サンプラ | 割り込まれた PC、仮想 ms、guest スレッド、guest スタックから拾った CALL 直後の戻り番地 14 個。**戻り番地の列は走査による推測**で、正確な unwind ではない(自己時間は PC なので正確、包含は下限寄り)。`tools/a4-prof.py` が集計 |
| チェーン | gdb の `bt`(`scripts/pk32.gdb` `pk33.gdb` `pk35.gdb` `pk38.gdb`)。リフト済み関数は C の関数なので bt は実チェーン |
| 場面 | `scripts/a4-idle20.g3`(Basic.flp を開いて 20 s idle)、`scripts/a4-ref.g3`(参照プロジェクトを開き、Enter を 1 回(ref1 の場面と同じ。窓の中身は確認していない)、10 s idle、Space で 30 s 再生)。**依頼者の操作ではなく G3 の模擬入力** |
| 参照プロジェクトの実行 | N1 のリンクに `Shared\oggio_x64.dll` のパス表(決定 0183)が無く LoadLibraryW で停止したため、`tools/realmod_ref.c`(**ハーネスのみ**。本番は変更なし)で `shared\oggio_x64.dll` と `shared\elastique_x64.dll` の 2 行を足した |
| 名前 | FLEngine_x64.dll のエクスポートは 3 件(CreateFruityInstance、`__dbk_fcall_wrapper`、`dbkFCallWrapperAddr`)で、関数名の資料は無い。`f_` アドレスだけで書いた。読み取りの名前(HandleMessage、Idle、DoActionIdle 風)は機械語の形からの推測で、確認していない |

## 2. `f_863f60` の呼び出し元と頻度

| 項目 | 値 | 根拠 |
|---|---|---|
| 呼び出しのメッセージ | SendMessageW(hwnd 7f990014 = TApplication 窓, msg 0xB03F, wParam 0, lParam = アクション)。16 個の別々のアクション、同じ仮想 ms に 2 周期 = 32 個 | `logs/bt-sendmessage-actionupdate.txt` |
| 送り手のチェーン | `f_1375990 → f_86a660 → f_86a250 → f_86b440 → f_86b300 → f_863c20 → f_863b50 ×3〜4 → f_60e9a0 → f_528d60 → f_5de830 → f_604f60 → SendMessageW` | 同上 |
| 受け手のチェーン | `call_wndproc → … → f_60c3e0 → f_410090(TObject.Dispatch 風)→ f_864030 → f_863f20 → f_863f60` | サンプラ(推測の列) |
| 頻度 | CM_ACTIONUPDATE 2002〜2005/s = 周期 125.2/s × 16 | `tables/message-rates-idle.tsv`、`tables/idle-basic-api-rates.tsv` |
| 1 回あたり(native) | CM_ACTIONUPDATE 1 回 約 32 µs(`f_864030` 包含 1.30 s ÷ 20 s ÷ 2002/s)。周期 1 回(16 個)約 0.52 ms、tick 1 回(2 周期)約 1.04 ms | `tables/prof-basic-idle-20s.txt` |
| `f_863f60` 自体の呼び出し回数 | **102,000/s**(8 virtual 秒で 816,000 回。CM_ACTIONUPDATE 1 回(`f_864030` 1 回)あたり 51 回)。自己時間は約 117 ns/回(native -O1、サンプリングの概算)。再帰なので包含時間の「1 回あたり」は定義しない: `f_864030` 1 回の部分木全体で約 32 µs | `tables/call-counts-basic-idle.tsv`(プロセス内カウンタ `A4_COUNT=1`) |
| 同じ経路の他の関数 | `f_863f20` 900,000/s(450/entry)、`f_6113f0` 1,215,000/s、`f_410010` と `f_40fe90` 1,217,000/s(各 608/entry)。`f_86b300`(DoActionIdle)と `f_11f0270`(主フォームの OnIdle)は 125/s。`f_864030` は 2000/s | 同上 |
| スレッド | UI スレッド(guest 0)。idle のサンプルの 90%。スレッド 18 は 10%、スレッド 6(QFC)は 0.2% | `tables/prof-*.txt` |
| 呼び出しの呼び水 | `WaitMessage` 125/s、`PeekMessageW` 292/s。起こし元は 0x8ffc(62.7/s)と WM_PAINT(63.8/s) | 同上 |

**gpu1(Chrome)との突き合わせ** PM の報告は「`f_86b300` の部分木は idle の約 42%(56 ms/s)、約 61 回/s・1 回 1.73 ms」。native の計数では `f_86b300` は **125/s ちょうど**(8 virtual 秒で 1000 回)で、61/s ではない。56 ms/s ÷ 125/s = 約 0.45 ms/回は、native の 1 回(16 個 × 約 32 µs = 約 0.52 ms、-O1・サンプラ有り)と同じ桁で、wasm が 1 回あたり特に遅いとは読めない。「61 回/s × 1.73 ms = 105 ms/s」は 56 ms/s と合わない(積が 2 倍近く違う)ので、gpu1 の 61 回/s は「プロファイルのサンプルから数えた回数」で実回数ではない可能性が高い。Chrome 側の実回数は未計測(0 件)。**コストは 回数(2 周期 × 62.5 tick × 16 アクション × 約 51 ノード)× 1 ノードあたりの小さい関数の連なり**で、runtime API の寄与は無い。

## 3. 周波数を決める入力と契約値(全表は `tables/contract-vs-ours.tsv`)

| 入力 | FL の使い方 | Windows の契約 | 我々の答え | 差 |
|---|---|---|---|---|
| SetTimer / WM_TIMER | run 全体で 2 回(600000 ms、500 ms) | uElapse < 10 → 10 ms(Learn) | 10 ms に丸め済み(`fle_win.c`) | なし。125/s の原因ではない |
| GetTickCount / timeGetTime / QPC | 2/s、0/s、22/s。ペース取りに使っていない | 分解能は システム timer(Learn) | 仮想時計(1 ms 刻み、読むたび +1 µs) | 粒度は違うが効かない |
| Sleep(n) | スレッド 18 が Sleep(1) 1055/s、スレッド 6 が Sleep(16) 62.5/s | 1〜2 tick(Learn)、FL は timeBeginPeriod(1) | **ちょうど n ms** | 契約内。許される最短 |
| DwmGetCompositionTimingInfo | 成功なら `Sleep(遅延)+DwmFlush`、失敗なら EnumDisplaySettingsW から Sleep だけ | Windows 10 に存在(Learn) | **NULL** | **差**(未実装) |
| EnumDisplaySettingsW | 起動時 6 回。周波数 ≥ 50 から tick を決める | 現在のモード | 60 Hz | 60 Hz の画面なら同じ。実機の Hz は 0 件 |
| PeekMessage / GetMessage / WaitMessage | VCL 形のループ。1 起床に 1 回の Idle | 送信 → 投函 → 入力 → WM_PAINT → WM_TIMER の順(Learn) | 同じ順 | 差は見つからず |
| WM_PAINT の作り方 | FL の OnIdle が 1 画素線を RedrawWindow。主フォームは WS_CLIPCHILDREN なし(スタイル 0x94030000)、線は子 TWPControl(x=378〜)の左隣 | 更新領域が空でなければ WM_PAINT(Learn) | 空でない更新領域 | 差なし |
| GetSystemMetrics ほか | 起動時のみ | - | - | idle で効かない |

## 4. 実験(使い捨て。`tables/experiment-sleep-dwm.diff` は **alpha4.diff ではない**)

| 版 | 内容 | Sleep 回数 / 8 s | 0x8ffc 回数 / 8 s | CPU 秒/virtual 秒 |
|---|---|---|---|---|
| vH | HEAD | 8446 | 500 | 0.1075、0.1177 |
| vS | Sleep を次の 1 ms tick に切り上げ | 4482 | 500(スレッド 6 は Sleep(16) が 16.000 ms で始まるので丸めが効かない) | 0.1022、0.1210 |
| vD1 | Sleep(16) のあと 次の 60 Hz 境界まで待つ(DwmFlush 経路の模擬)・Sleep は ちょうど n | 8423 | 479 | 0.0964、0.1165 |
| vD2 | vS + vD1 | 4461 | 478 | 0.0931 |
| hz-30 | `FLE_DISPLAYSETTINGS_FREQUENCY=30`(テスト用の探り) | 8446 | 500 | 0.1059 |
| hz-120 | 同 120 | 8924 | 999 | 0.1993 |

読み: (1) 計数(Sleep、0x8ffc)は決定的で、実験どおりに動く。(2) CPU は 揺れが大きく(同じ vH で 0.1075 と 0.1177)、-4%〜-9% は検出できない。(3) 120 Hz は 約 1.9 倍で、コストが tick 数に比例することは確か。(4) 仮想時計は『Sleep から戻って次の Sleep を呼ぶまでの時間が 0』なので、tick に切り上げる定式化は 起動後 +1 µs を読む スレッド 18 には効くが、スレッド 6 には効かない(実機では実行時間があるので 17 ms になる)。**この非対称は 決定的な契約の実装として一意に書けない**。実機の測定が 0 件のままでは、Sleep の切り上げは入れる根拠が弱い。

## 5. 判断と提案

- **`alpha4.diff` は作らない。** 理由: (1) 余計な処理(a)は見つからない。(2) (b) の差は 2 つ(Sleep の端、Dwm の NULL)あるが、見込みの効果は idle の約 9% 以下で、CPU では測れない。しかも (3) Dwm の行を足すには lifter の response table(`imports_table.rs`)に行を足して全ての生成 C と wasm を作り直す必要があり、84/55/87 行と a4 の全面更新が付く。(4) 切り上げは 実機の測定が無く、スレッド 6 に効かない。
- もし PM が (b) を入れると決めるなら 最小の形: `DwmGetCompositionTimingInfo(HWND, DWM_TIMING_INFO*)` の行(rateRefresh = fle_display_hz():1、qpcRefreshPeriod = 10^7/hz、cbSize 0x124 を確認して S_OK)を `imports_table.rs` と `fle_dwm.c` に足す。効果は 0x8ffc の 62.5 → 59.9/s(-4%)。
- 効果が大きいのは契約側ではなく FL 側の仕事量(1 tick に 2 周期 × 16 アクション)と、wasm の生成コードの質(native 比約 3.3 倍)。ここは (a)(b) の範囲外。

## 6. 副課題: A1 の全窓一括再描画の `FillRect`(DC でない hDC)停止(最初の切り分け表。**調査は未実施、仮説のみ**)

HEAD(alpha3b2 を含む)では**再実行していない**。alpha3 の記録では「A3b では起きない」「A1+fix(unc_end の子をクリップしない増加分)でも止まらない」。停止は gdb から `RedrawWindow(0x185 = INVALIDATE|ERASE|UPDATENOW|ALLCHILDREN)` を全 top-level 窓へ注入した場面(A1 の steady の終わり、minimize/restore のあと)に出る。BeginPaint/EndPaint/ReleaseDC のどの組が壊れたかは見ていない。次の表は、見るべき点を並べた仮説で、**どれも測っていない**。

| 仮説 | 予想される証拠 | 見る方法 | 契約の問題か |
|---|---|---|---|
| H1: 注入した RedrawWindow がゲストの GetDC〜ReleaseDC の間(DC を保持している最中)に入れ子で走り、保持中の DC を入れ子の描画が ReleaseDC/EndPaint で解放する | 停止した FillRect の hDC が、直前の別の描画が Release した値と同じ | 停止点で `bt`、直前 200 件の `GetDC`/`BeginPaint`/`ReleaseDC`/`EndPaint` の戻り値・引数を並べる(`scripts/pk33.gdb` の形) | いいえ(ハーネスの入れ子。FL は入れ子の WM_PAINT を想定しない) |
| H2: A1 の外接矩形の更新領域が大きく、入れ子の WM_PAINT が多くの窓を一度に描き、DC のキャッシュ(FL 側の 1 つの変数)を破壊する | A3b(矩形の並び)で WM_PAINT の窓数が少なく起きない | 注入時の WM_PAINT 配送数と窓の一覧を数える | いいえ(H1 と同じ原因が起きる確率の差) |
| H3: BeginPaint が返す DC を EndPaint の後に使う(FL の描画関数が DC を保持する)。EndPaint が DC を無効にする契約(Learn)に runtime が従った結果 | hDC が EndPaint の引数と同じ値 | 同上 | いいえ(契約どおり) |
| H4: runtime が GetDC/ReleaseDC で同じ値を再利用し、解放済みを新しい値と取り違える | 停止した hDC が 後の GetDC の戻り値と同じで、世代が違う | DC 表の世代番号を表示 | **はい**(あれば我々の不具合) |

A1+fix で止まらなくなった事実(露出の領域が入ると止まらない)は H2 と合うが H4 とは合いにくい。ただし、決め手は 停止点の `bt` と DC の履歴で、未取得。

## 7. 逸脱の開示

- 1 回、末尾に `&` を付けたコマンド(gdb の背景実行)を実行した。禁止されていた形で、その場では弾かれず実行された。以後は `background: true` のみ。第 1 報で報告済み。
- 1 回、`sleep` と `&` を含むコマンドを送って弾かれた(実行されていない)。
- 負荷平均 約 25 の機械で測ったため、CPU 時間は ±10% 揺れる。決定的な数(メッセージ・Sleep・Post の回数)を主にした。
- gdb の ignore count による呼び出し回数の計数(`scripts/pk34.gdb` 5 関数・2 s、`scripts/pk37.gdb` 3 関数・1 s)は、どちらも 14 分以上かかり完了しなかったので kill した(結果なし)。代わりに記録器へプロセス内の int3 カウンタ(`A4_COUNT=1`)を足して数えた。
- 基準: 測定は b42fad36 の生成 C。PM の通知で HEAD は 2c1029ed(層 e: fle_gdi.c / fle_gdc.c の `REUSE_AFTER == 0` の死んだ枝の削除、fle_audio_ring.c の lead の定数化)に進んだが、idle の API 経路には関係しないと読むだけで、再測定はしていない。差分を出していないので 2c1029ed 基準の dry-run は不要。
- PM の tree には書いていない(`lift/out-alpha3` と `lift/out-git/alpha4-results/` だけ)。

## 8. 成果物

`docs/`(この文書、`README-0250-line.txt`)、`tables/`(`contract-vs-ours.tsv`、`cpu-per-virtual-second.tsv`、`prof-*.txt`、`message-rates-idle.tsv`、`idle-basic-api-rates.tsv`、`hz-sleep-dwm-experiments.tsv`、`experiment-sleep-dwm.diff`)、`logs/`(gdb の bt)、`scripts/`(G3 場面と gdb スクリプト)、`tools/`(記録器、サンプラ、ビルド・実行スクリプト、`realmod_ref.c`)。

## PM の決定と取り込み(2026-11-05、docs のみ)

取り込み: alpha1 の最終報告(18:45)の文書・表・道具を `docs/investigations/alpha4/` に、決定文書を `docs/decisions/0250-idle-cpu-contract.md` に置いた。**差分(alpha4.diff)は無い**。コードは変更しない。測定は alpha1 のもの(native N1、b42fad36 の生成 C、G3 の模擬入力。実機 Windows・依頼者の機械・Chrome の値は 0 件)で、PM は再測定していない。

判断:
1. **採る: `f_863f60` の定常 CPU は FL 自身の動作。** 1 tick に idle 周期 2 回 × 16 アクション(CM_ACTIONUPDATE 2002/s)という FL の設計で、runtime API の寄与は無い。(a) 除去すべき我々の余計な処理は無し。
2. **入れない: Sleep(n) の 1〜2 tick 化、GetProcAddress(Dwm*) の実アドレス化。** 見込みは idle の約 9% 以下で CPU では測れず(同一バイナリの揺れが差より大きい)、Dwm の行は lifter の response table 変更を伴い 84/55/87 行と a4 が全面に動く。仮想時計は「戻ってから次の Sleep までの時間が 0」なので tick 切り上げは決定的に一意な定式化が書けない。実機の Sleep(1) の長さ・DwmFlush を含む tick の周期の測定が 0 件のうちは、契約が許す最短を選んでいる現状を差として記録するに留める(pick1 B31 と同じ扱い)。
3. gpu1 の「f_86b300 約 61 回/s」は alpha1 の native の計数(125/s ちょうど)と合わない。gpu1 の値はサンプル由来の可能性が高い。Chrome 上の実回数は未測定。
4. 残る課題: 副課題の FillRect(DC でない hDC)停止は仮説表のみ(H4 = runtime の DC 再利用の取り違えが唯一の我々の不具合の候補)。決め手は停止点の bt と DC 履歴。次の担当に回す。
