# 決定 0247(alpha3): プラグインピッカーと Mixer の全面再描画 — ピッカーの 62.5 Hz は FL 自身の挙動(b)、Mixer の全パネル再描画の主因は runtime の単純化(a)(更新領域を外接矩形 1 つで持っていた)。更新領域を矩形の並びで持つ。仮の決定

状態: **仮の決定**。担当 alpha1(PM の指示 = 『alpha3』、2026-11-05、期限 15:30)。作業は HEAD **cbf9d1e5** の `git archive`(A1)と、その木 + `alpha3.diff`(A3b)。HEAD は 2eea92b1 まで進んだが(e52d024f 以降は文書だけ)、cbf9d1e5 からその間に変わった runtime は `fle_raster.c/.h`(alpha2b)だけで、本差分が触る 6 ファイル(`fle_win.c/.h`、`fle_gdi.c/.h`、`fle_gdc.c/.h`)は HEAD と A1 で同一(`git show HEAD:… | cmp` で確認)。凡例: **実測** = このセッションのコマンドの出力を見た事実(2026-11-05、このマシン、負荷平均 10〜23 の共有機)。**実機 Windows、依頼者の機械、実 GPU の値は 0 件**。入力はすべて G3 の模擬入力/CDP の模擬入力で、ユーザー操作ではない。

**追補(alpha3b2、§10)**: 上の『未確認 1』の F9 の古い帯と、§7.1 の Mixer 下端の帯は、**§10 で原因を確定して直した**(窓の移動で露出した部分のうち子窓の上にある部分を、子をクリップしない親の更新領域に入れていなかった。alpha3 が作った不具合ではなく HEAD の既存の不具合)。以下の本文(§1〜§9)は alpha3 の時点の記述で、F9 の帯・Mixer 下端の帯・(iii)(iv) の数字は §10 で更新される。

**何を実測したか**: 実エンジン(FLEngine と同梱 DLL を x86-64→C にした物)を (1) native -O2 の N1 に記録器を付けて G3 の模擬入力で gdb の C バックトレース・API の呼び出し数・BeginPaint の矩形・画面全体の画素を A1 と A3b で比べ、(2) 同じ G3 でプロセス CPU を 2 回ずつ測り、(3) headless Chrome 156(CDP の駆動)で A1 と A3b の wasm の bundle を交互に動かして Worker の CPU を測り、(4) Wine 10.0 と runtime に同じ C のプログラム(`harness/alpha3/win_upd2.c`)を動かして更新領域の戻り値を比べた。

**結論**: (a) **ピッカー(`TPlugListForm`)の 62.5 Hz の全面再描画は FL 自身の挙動(b)**: QFC の共有アニメーションのスレッドが 16 ms ごとに `PostMessageW(0x8ffc)` を投げ、主スレッドのピッカーの更新関数が末尾で無条件に Invalidate する(開いている間は『終わり』の枝に入らない)。周期 16 ms は我々の `EnumDisplaySettingsW` の 60 Hz の答えをゲストが丸めた値。契約のずれは見つからなかった(§1)。(b) **Mixer の移動の全パネル再描画の主因は runtime の単純化(a)**: 更新領域を外接矩形 1 つで持っていたため、FL が出す枠形の `RedrawWindow(hrgn)`(約 8 万画素)の足し込みが 75.7 万画素の全面になり、FL の WM_PAINT が全コントロールを描き直していた(§2)。更新領域を互いに素な矩形の並びで持つ(上限 32 で外接矩形に畳む)ようにした(`alpha3.diff`、runtime 6 ファイル)。Wine 10.0 との照合は 22 行中、A1 が 11 行、A3b が 1 行(40 個の小矩形を畳む既知の過大側)違う(§4)。(c) native -O2 の G3 のプロセス CPU は −14〜−62%(Mixer のドラッグ −51%、ピッカーのフェードイン −62%・定常 −14%・フェードアウト −46%、F8 −50%、Playlist −30%)、BitBlt 系は 0.3〜0.5 倍(定常のピッカーは 0.92 倍。§6.1)。**Chrome の Worker CPU は向きこそ下がる(ピッカーの定常 -14%、Mixer のドラッグ -2%)が、5 回の値が重なり有意な差とは言えない**(§6.2)。畳み(fold)は、起動(project の読み込みと最初の描画)で 52〜56 回、そのあとの Mixer の移動・ピッカー(フェードイン・定常・フェードアウト)・Playlist・F8・設定窓・F9 で **0 回**、File メニューの項目を選ぶ操作(pull1)で 5 回(§6.3)。

**未確認(重要な順)**:
1. **F9 での隠す→見せるのあとに、Mixer の右端の外の 10×470 画素の帯(x 1903〜1912、y 548〜1019)が、完全再描画の画面と違う(A3b 4,646 画素、A1 139 画素)**(§7.4)。この帯は Mixer をドラッグした直後に A1 にも A3b にも同じ内容で残っていて(帯の差 0)、A1 は F9 のあとにそれが消え、A3b は残る。BeginPaint の矩形・数は両者でほぼ同じ。帯の 1 画素に gdb のウォッチポイントを置くと、A1 では F9 の間にその画素を書く呼び出しが**1 回だけ**あり(FL の WM_PAINT の中の、宛先 (0,0)・幅 1047 の BitBlt が背景色を書く。`scripts/pk18.gdb`)、A3b には**無い**(その BitBlt のクリップ〔更新領域の矩形の並び〕に帯が入っていない)。幅 1047 の窓は Playlist と思われるが窓は未特定。つまり『A3b が新しく描き漏らす』のではなく、ドラッグの経路にある既存の描き漏らしを A1 の過大な矩形が後から隠していた可能性が高いが、**確認していない**。実機 Windows でも残るかは 0 件。PM の受け入れ条件(iv)『F9 で 0 画素』は **この 1 点で満たしていない**。
2. (iii)『一括で全窓を描き直した絵との差が A3b ≤ A1』は 9 本中 **4 本**(ui-menu、ui-pl、ui-win、pull1)で満たし、**5 本**(mxstart、mxmove、ui-f8、f8、mxrep)では満たさない。mxstart・ui-f8・f8 の超過は ≤3 段階の小さい差(4 段階以上は A3b 18〜26 画素 対 A1 0〜9)、mxmove は帯の画素(Mixer 下端の鍵盤状の帯)が +773 画素、mxrep は上の 1 の帯(+4,497 画素)(§7.3)。
3. node の仮想時計の wasm での 84 行・55 行の照合は **未実施**(native の同じ G3 の 94 行は clock_ms 以外 全行一致、§7.2)。ただし `g3-surface`(窓の面の SHA)の 34 行は 17 行が違う(画素が違うので当然。基準の表は取り込みの後に作り直しが要る)。起動の最初の絵(`wiz0`)は A1 と 15 万画素違う(A3b の Playlist に格子線が先に出る。§7.1)。Firefox 158 は未実施。実機 Windows 0 件。Sleep(1) だけの費用は分離できず未測定(§1.4)。Chrome の A3b の bundle は『非表示の窓への無効化を無視する』規則(S-0247-hidden)を入れる前の runtime で作った(§6.2)。
4. A1 の窓の一括再描画(`RedrawWindow(0x185)` を gdb から呼ぶ)が、ゲストが `FillRect` に DC でない hDC を渡して停止する(steady の終わり・minimize/restore のあと)。A3b では起きない。原因は未確認(§7.3)。


## 1. ピッカー(`TPlugListForm`)の 62.5 Hz の全面再描画の原因 — **FL 自身の挙動(b)**。契約のずれ(a)は見つからなかった

### 1.1 実行時の呼び出し鎖(native -O2 N1 を gdb で動かした C のバックトレース。`scripts/pk.gdb`、`runs/gdb1`)

定常 = F8 を押して 50 フレーム(16 ms)のフェードインのあとの 190 フレーム(3.04 仮想秒)。この間に **PostMessageW(0x8ffc) 190 回 = InvalidateRect(ピッカー) 190 回 = BeginPaint(ピッカー) 190 回(1 対 1 対 1)**(`scripts/pk3.gdb`、`runs/gdb3`)。

| 段 | 呼び出し鎖(関数 VA。FLEngine は 0x4xxxxx〜、QFC は 0x1d.. 〜 0x1e..) | 意味(読み) |
|---|---|---|
| 駆動(スレッド) | `thread_main → f_1d0fdb0 → f_1df0ee0 → f_1e85bd0`(`while !Terminated` のループ)→ `f_1e860a0` → PostMessageW(隠れ窓 0x7f990018, **0x8ffc**, wParam = 未処理 tick 数)/ `f_1e85b70 → f_1df1db0` = **Sleep(16)** | QFC の共有アニメーション周期(`AcquireSharedAnimTimer`)のスレッド。tick を窓へ投げ、登録された listener を呼び、16 ms 眠る |
| 周期の出所 | `f_1e857b0`(QFC): 『1 フレームの ms』= `N × 1000 × 分母 / 分子` を丸める。`DwmGetCompositionTimingInfo`(`GetProcAddress` は NULL を返す〔`fle_modules.c`〕)が無いので **EnumDisplaySettingsW の `dmDisplayFrequency`**(我々の答え = `fle_display_settings_hz()` = 60 Hz)を使う分岐 → 1000/60 を丸めて **16** | 周期 62.5 Hz = 我々の『60 Hz』をゲストが 16 ms に丸めた値。定常で `DwmFlush` は 0 回 |
| 受け(主スレッド) | `DispatchMessageW → call_wndproc → f_1e858e0`(QFC の `TSharedAnimTimerWindow` の窓プロシージャ)`→ f_1e86280`(listener へ配る)`→ f_ebfac0` | ピッカーの『アニメーション更新』(下) |
| 無効化 | `f_ebfac0 → f_617630`(TControl.Invalidate の仮想メソッド `[vtbl+0x178]`)`→ f_60c220`(Perform)`→ f_80c930 → f_85c6d0 → f_612e10 → f_60c3e0 → f_410090`(`TObject.Dispatch`)`→ f_6176e0`(CM_INVALIDATE = 0xb034 の処理)`→ InvalidateRect(ピッカー, NULL, TRUE)` | |

### 1.2 『終わった』の判定は永遠に満たされないのか — いいえ(開いている間は **終わりが無い**コード)

`f_ebfac0`(0xebfac0〜0xec00d7)は、(1) 経過時間 dt を `[0x15e4c00]` のオブジェクトの仮想メソッド(`+0x10`)から得て `[this+0x808]`(経過 ms)に足し、(2) `[this+0x810]`(**フェードアウトの経過。開いている間は −1 = 『開いた状態』**)が負なら『開いた』の枝に入り、(3) 3 項目(`[this+0x7f4]` の個数)の状態(バネ/イージング)を更新し、(4) **末尾 0xec00a1〜0xec00ab で無条件に `call [this->vtbl+0x178]`(Invalidate)**する。関数を無条件の Invalidate より前で抜ける道は 0xebfbf6 → 0xec00b1 の 1 つだけで、そこに入る条件は [this+0x810] × 定数 > 閾値(= フェードアウトが終わった)。入る前に listener を登録解除する(`[0x15e4c00]` の仮想メソッド `+0x50`)。実測: [this+0x810] = −1 のまま、[this+0x808] が 16 ms ずつ増え続ける(816 → 3680、`scripts/pk2.gdb`、`runs/gdb2`)。Escape 後にだけ [this+0x810] が 0 から進み、終了の枝に入る。

つまり FL は『ピッカーが開いている間は tick ごとに全面を Invalidate する』コードで、**我々の答えの値が『終わり』の判定を妨げているのではない**。表示のリフレッシュ(tick)ごとに同じ再描画をするのは、実機の 60 Hz でも同じに読める(**実機 Windows での実測は 0 件**)。

### 1.3 契約との照合(周期を決める唯一の入力 = EnumDisplaySettingsW の周波数)

| 項目 | 内容 |
|---|---|
| Learn `EnumDisplaySettings` / `DEVMODE.dmDisplayFrequency` | 『表示周波数(Hz)』。我々の答え 60(`FLE_DWM_HZ` 既定 60〔`fle_thread.c: fle_display_hz`〕、ページが測った値を渡す opt1 の仕組みがある)。ゲストは 1000/60 を 16 ms に丸める(ゲストの算術)。**ページが 144 Hz を伝えれば周期は 6〜7 ms になり、再描画も 144 Hz になる**(実機の FL と同じ向き) |
| `DwmGetCompositionTimingInfo` | `GetProcAddress` は NULL(`fle_modules.c` の dwmapi の行)。ゲストは EnumDisplaySettingsW の分岐を使う。契約どおり(関数が無い環境の扱いは FL 自身が持っている) |
| Learn `Sleep` | 『指定した ms 間実行を中断する』。ゲストが `Sleep(16)` を 1 周に 1 回呼ぶ。我々の仮想時計で正確に 16 ms 進む |

### 1.4 付随の観察(ピッカーの定常 1 フレームあたり。`runs/api1`、`runs/gdb3`、`runs/gdb4`)

| 観察 | 内容 | 費用への読み |
|---|---|---|
| **別スレッドの `Sleep(1)` × 約 16 回/フレーム**(3,024 回/3 s) | `thread_main → f_412ae0 → f_5251b0 → f_12aeec0 → f_5264e0 → Sleep(1)`。FL 自身のポーリングループ(`[this+0x1a]` = Terminated を見る。`[0x15d3294]` が 0 のあいだ Sleep(1) を繰り返す) | `tools/alpha1_rec.c` の `A1_TIMEALL=1`(import ごとのスレッド CPU 時間。表 `tables/native-api-time-steady-A1.tsv`)で 190 フレーム(`st` 区間)の Sleep は **3,214 回(16.9 回/フレーム)**。ただしゲストのスレッドは協調スレッド(`fle_thread.c`、決定 0122: 複数スレッドのとき Sleep は待って他のスレッドが走る)なので、Sleep の『CPU 時間』は他のスレッドの実行時間を含み(`st` 区間で 4,740 ms は同区間のプロセス CPU 4,678 ms を超える)、**Sleep(1) だけの費用は分離できない(未測定)**。変更はしない(FL のポーリングそのもの。`timeBeginPeriod` を 2 回呼ぶので契約どおり Sleep(1) ≒ 1 ms) |
| IntersectRect 1,454 回/フレーム、SetWindowOrgEx 53、SelectClipRgn 47、GetClipBox 44、IntersectClipRect 42 | VCL の描画(コントロールごとのクリップの判定) | `st` 区間で IntersectRect **276,174 回(1,453 回/フレーム)= 8.2 ms**(スレッド CPU。プロセス CPU 4,678 ms の 0.18%)、SelectClipRgn 8,877 回 = 1.9 ms、BitBlt 283 回 = 240.6 ms(A1_NOBB)。IntersectRect 自体は費用の主因ではない。変更なし |
| GetTickCount 37 回/フレーム | QFC スレッドと FL のポーリングが時計を読む | 小さい |

## 2. Mixer の移動(と同種の部分更新)の全パネル再描画 — **原因の一部は runtime の単純化(a)だった: 更新領域を『外接矩形 1 つ』で持っていた(S-updregion、決定 0126)**

### 2.1 実行時の呼び出し鎖(`scripts/mxmove.g3` = gpu1 の Mixer ヘッダードラッグ〔guest (715,564)→(1015,564)〕を 40 ステップ、`scripts/pk5〜pk13.gdb`、`runs/gdb6〜gdb12`)

1 ステップ(16 ms = マウス移動 1 件)の中身(lid 64 = Mixer の `TFXForm` 1514×500):

| # | 呼び出し | 内容 |
|---|---|---|
| 1 | `f_410090(Dispatch) → f_60d520 → f_80bb30 → f_80c010 → f_616c10 → SetWindowPos(lid 64, x+15, y, 1514×500, flags 0x14)`(1 回) | FL が自分で子窓を動かす(`WM_NCLBUTTONDOWN`/`SC_MOVE` は来ない: gpu1)。runtime はビットをコピーし、露出した **15×500 の帯だけ**を無効にする。続く `BeginPaint` の rcPaint は (46,466,61,966)(最小。契約どおり) |
| 2 | **QFC の tick**(§1 と同じ鎖: `f_1e858e0 → f_1e86280 → f_80a7a0 → f_8090d0 → f_809030 → f_820f10`)→ `RedrawWindow(lid 64, hrgn, RDW_INVALIDATE|RDW_NOCHILDREN)` と `RedrawWindow(lid 64, rect (4,4,1510,27), 0x41)` ×2、`InvalidateRect(子 lid 89, NULL, TRUE)` | FL が tick ごとに出す無効化(Mixer の『ドラッグ中のアニメーション』。静止時は約 10 tick に 1 回〔ピッカーの定常 190 フレームで lid 64 の BeginPaint 18〜21 回〕) |
| 3 | `GetUpdateRgn → BeginPaint → …描画… → ValidateRgn`(1 ステップに BeginPaint 5.2、GetUpdateRgn 5.2、ValidateRgn 9.3、GetRegionData 5.6、SelectClipRgn 308、IntersectRect 2,628) | FL の WM_PAINT は **GetUpdateRgn で更新領域の矩形の並びを読み(GetRegionData)、その各矩形に重なるコントロールだけを描く**。ValidateRgn で描いた部分を除く |

手順 2 の `hrgn`(`bounds = (0,0,1514,500)`)は**矩形ではなかった**: `fle_gdi_region_covers(bounds) = 0`。25 画素刻みの標本(`scripts/pk12.gdb`)で、上の帯(y 0〜約 40 の全幅)と左の縁(x 0〜約 49)だけを覆う**枠形**(約 8 万画素)で、内部(x ≥ 50, y ≥ 50)は含まない。ところが runtime は更新領域を外接矩形 1 つで持つ(S-updregion)ので、これを足した結果は (0,0,1514,500) = **75.7 万画素の全面**になり、BeginPaint の rcPaint もそのクリップも全面になる。FL の WM_PAINT はその全面の中のコントロールを毎回すべて描き直す(ゲストの IntersectRect 2,628 回/ステップ、SelectClipRgn 308 回/ステップ)。

### 2.2 Learn との照合(契約はどう言っているか)

| Learn | 内容 | 旧 runtime(S-updregion) | 本変更 |
|---|---|---|---|
| The Update Region (WM_PAINT) | 『システムは窓の無効な部分だけを更新領域に足す』(領域であって矩形ではない) | 外接矩形 1 つ(過大) | 矩形の並び(互いに素)。`UPD_PIECES_MAX` = 32 を超えたら外接矩形に畳む(S-0247-fold。数える) |
| InvalidateRect / RedrawWindow(hrgnUpdate, RDW_INVALIDATE) | 矩形/領域を更新領域に『足す』 | 領域は外接矩形に畳んで足していた | 矩形ごとに足す(領域の矩形は `fle_gdi_region_pieces` で取る) |
| ValidateRect / ValidateRgn / RedrawWindow(RDW_VALIDATE) | 矩形/領域を更新領域から『除く』 | 外接矩形を全部覆うときだけ空にし、一部の除去は何もしなかった | 除く(`fle_visrgn_subtract`) |
| GetUpdateRect | 更新領域を『囲む最小の矩形』 | 外接矩形 | 外接矩形(変わらない) |
| GetUpdateRgn | 更新領域を引数の領域へコピー。戻り値は NULLREGION/SIMPLEREGION/COMPLEXREGION | 常に SIMPLEREGION(外接矩形) | 矩形の並びを Y-X の帯で入れ、2 つ以上なら COMPLEXREGION(3) |
| BeginPaint | PAINTSTRUCT.rcPaint = 更新矩形(外接矩形)、DC のクリップは『更新領域の外を除く』。BeginPaint は更新領域を全部検証する | rcPaint も DC のクリップも外接矩形 | rcPaint = 外接矩形(同じ)、**DC のクリップ = 更新領域**(`fle_gdc_set_paint_pieces` → `fle_gdi.c: get_target0` が可視領域を矩形の並びで切る) |
| ShowWindow(隠れた窓が見える) | 見えるようになった窓は client 全体を描く(Learn BeginPaint: 更新領域は『作成・サイズ変更・移動・スクロール』のあとシステムが設定する。『表示』は S-0214 の選択) | 更新領域が**空のときだけ**全体にしていた(外接矩形が隠していた) | 更新領域に何があっても全体を足す(S-0247-show) |

選択(S-): **S-0247-pieces** = 互いに素な矩形で持つ(領域の代数は `fle_visrgn` の既存の subtract/intersect)。**S-0247-fold** = 32 矩形を超えたら外接矩形(過大なのは従来と同じ向き)。**S-0247-show** = 上の表。UNOFFICIAL は無い(Windows の実測は 0 件)。


## 3. 何を直したか(`alpha3.diff`。コードとテストの差分は 1300 行〔文書・表・道具を含めた全体は約 1.9 万行〕、`-` の行は 49 行で、すべて旧い外接矩形の直接代入・拡張の置き換え)

| ファイル | 変更 |
|---|---|
| `lift/runtime/fle_win.c` | 窓の更新領域を**互いに素な矩形の並び**で持つ(`tab[i].updr` = `fle_visrgn*`、矩形 1 つのときは NULL。`upd_l..upd_b` は外接矩形のまま)。補助関数 `upd_clear_rect/upd_set_rect/upd_set_full/upd_get/upd_put/upd_minus_rect/upd_add/upd_bounds/upd_rgn_ok`。`InvalidateRect`/`RedrawWindow(hrgn)`/`unc_*`(窓の移動・隠す・重なりの変化で露出した部分)は矩形を**足し**、`ValidateRect`/`ValidateRgn`/`RDW_VALIDATE` は矩形を**除く**。`GetUpdateRgn` は矩形の並びを返す(戻り値 COMPLEXREGION = 3)。`BeginPaint` の `rcPaint` は外接矩形のまま、DC のクリップは更新領域(矩形の並び)。窓の破棄で `updr` を解放。`fle_win_update_folds()` を公開 |
| `lift/runtime/fle_gdi.c`, `fle_gdi.h` | `fle_gdi_region_pieces`(領域の矩形を取る)、`fle_gdi_region_set_pieces`(Y-X の帯で領域へ入れる)、`FLE_GDI_COMPLEXREGION`、`get_target0` が可視領域を『更新領域の矩形の並び』で切る(`fle_visrgn_intersect`) |
| `lift/runtime/fle_gdc.c`, `fle_gdc.h` | `fle_gdc_set_paint_pieces`(`FLE_GDC_PAINT_PIECES` = 32)。EndPaint で解除 |
| 新規 `lift/harness/alpha3/upd_region.c`(30 項目)、`win_upd2.c`(Wine と runtime で同じ 22 行を出す)、`lift/crates/fe-lift/tests/alpha3_updregion.rs`(`n >= 30`) | §4、§5 |

選択(S-): **S-0247-pieces** = 互いに素な矩形で持つ(領域の代数は `fle_visrgn` の既存の subtract/intersect)。**S-0247-fold** = 32 矩形を超えたら外接矩形に畳み、`fle_win_update_folds()` が数える(従来と同じ向きの過大)。**S-0247-show** = 見えるようになった窓は更新領域に何があっても client 全体(従来は更新領域が空のときだけ。外接矩形が空でないと省いていた)。**S-0247-hidden** = 見えない窓の無効化は更新領域に入れない(Wine 10.0 の行 18)。S-0247-show と S-0247-hidden は Learn に記述が見つからず **UNOFFICIAL**(出典: Wine 10.0 の実行結果 `tables/wine-upd2.tsv` 行 18・19)。他に UNOFFICIAL は無い。

`fle_visrgn` の関数(weak)が無い runtime では従来の外接矩形の動作に戻る(ビルドの組み合わせの保険。`fle_gdi.c` のリンクエラー対策)。生成 C・lifter・`app/`・G3 の実装は変えていない。

## 3.1 余計な処理ではない理由(3 行)

1. **外部 API の契約の実装(b)であり、我々の単純化(a)の除去**: Learn の更新領域は『領域』で、矩形 1 つに潰す規則は契約にない。潰していたのは決定 0126 の S-updregion という我々の選択だった。FL の描画を代行・補正する処理は足していない。
2. クラス名・窓の種類・位置・大きさによる分岐は 0、新しいオプション・環境変数・フラグは 0(`docs/policies/no-options.md`、`lift-fidelity.md` の点検表)。
3. 畳み(S-0247-fold)は安全側(外接矩形 ⊇ 正確な領域)で、起動以外の操作では 0 回。性質は §5 の乱数テストで確認。

## 4. 契約との照合(Learn と Wine 10.0)

### 4.1 Learn(§2.2 の表)と Wine 10.0 の実行結果

`harness/alpha3/win_upd2.c` は同じ C のプログラムを (1) `x86_64-w64-mingw32-gcc` でビルドして Wine 10.0 で動かし、(2) runtime のホスト版(`tools/build-wine2.sh`)にリンクして動かして、22 行の出力を比べる。範囲: 矩形 2 つの無効化、重なる無効化、ValidateRect(一部)、ValidateRgn(一部)、ValidateRect(NULL)、RedrawWindow(hrgn、INVALIDATE|NOCHILDREN / VALIDATE)、BeginPaint の rcPaint、EndPaint の後、隠れた窓の無効化、ShowWindow(SW_SHOWNA) の後、40 個の小さい離れた矩形。子/兄弟の関係(ScrollWindowEx、ExcludeUpdateRgn、RDW_ALLCHILDREN の子の扱い)は**この表に入れていない**(未実施)。

| 結果(`tables/wine-upd2.tsv`) | Wine 10.0 と同じ行 |
|---|---:|
| A3b(本変更) | **21 / 22** |
| A1(HEAD) | 11 / 22 |

A1 が違う 11 行: GetUpdateRgn の戻り値(常に SIMPLEREGION)と領域の中身(隙間も覆う)、ValidateRect/ValidateRgn の一部除去が効かない、隠れた窓の無効化が残る、ShowWindow の後に client 全体にならない。A3b で違う 1 行: 40 個の小矩形(畳みの過大。Wine は正確な領域を持つ)。**注意**: Wine は Windows の実測ではない(出典としての強さは Learn より下)。

### 4.2 『畳みは安全側』の性質(`upd_region.c` の乱数テスト)

6 回の試行(各 16 または 400 回の InvalidateRect/ValidateRect を乱数で。`ValidateRect` は 10% で最大 60×40)。厳密な画素のモデルと比べる: (1) どの試行でも**更新領域は厳密な和が持つ画素を 1 つも落とさない**(0 画素)、(2) 16 回の 3 試行は畳まれず**領域が和と完全に一致**(違い 0 画素)、(3) 400 回の 3 試行は畳みが走る。実行: A3b 30 項目 0 失敗。HEAD の runtime では 30 項目中 13 が失敗(§5)。

## 5. テスト

- `lift/harness/alpha3/upd_region.c`(窓 1 つに InvalidateRect・ValidateRect・ValidateRgn・RedrawWindow の 4 つの経路、GetUpdateRgn/GetUpdateRect/BeginPaint の rcPaint とクリップの画素、畳み、乱数、ShowWindow): **A3b 30 項目 0 失敗**、HEAD の runtime 13 失敗(変更が効いている証拠)。`cargo test -p fe-lift --test alpha3_updregion`(`n >= 30`)。
- `test.sh full`(本変更の木): **202 results / 2,401 passed / 0 failed / 1 ignored**(`harness/test.sh full --jobs 3`、rc=0、176 executables 実行 + 22 cached。新しい `alpha3_updregion` を含む。`logs/test-full-t3b-final.log`)。
- 本変更の前の最初の 2 回の失敗は自己原因: (1) `fle_gdi.c` の `fle_visrgn_*` のリンクエラー(weak 保護の追加で解消)、(2) `wall_fr9/fr10` が null の weak 関数の呼び出しで segfault(保護で解消)。`x87`/`osver`/`lib` の失敗は走行中の別の `test.sh` と重なった実行で、単独では通る。

## 6. 測定(**実エンジン**。入力は G3/CDP の模擬入力で、ユーザー操作ではない)

### 6.1 native -O2 の N1(記録器つき)のプロセス CPU(`tables/native-cpu-ab.tsv`、2 回ずつの平均、ms)

負荷平均 10〜23 の共有機なので**絶対値ではなく比**で読む(2 回の値は表に併記)。区間の定義は表のとおり(g3-cpu の user+sys の差)。

| 区間 | A1 | A3b | 変化 | BitBlt 系 A1 → A3b |
|---|---:|---:|---:|---|
| Mixer のヘッダーのドラッグ 40 ステップ(e03..n020) | 1,128 | 551 | −51% | 33 → 9 |
| ピッカー フェードイン 49 フレーム | 4,004 | 1,507 | −62% | 207 → 75 |
| ピッカー 定常 189 フレーム(3.0 s) | 4,922 | 4,234 | −14% | 266 → 245 |
| ピッカー フェードアウト 74 フレーム | 2,449 | 1,333 | −46% | 113 → 58 |
| F8 の開閉(通し) | 8,094 | 4,016 | −50% | 457 → 206 |
| F8 の開閉(ui-f8) | 2,883 | 1,142 | −60% | 151 → 59 |
| File メニュー(pull1) | 2,420 | 1,850 | −24% | 19 → 15 |
| メニュー(ui-menu) | 688 | 684 | −1% | 8 → 9 |
| 設定窓(ui-win) | 1,070 | 1,074 | 0% | 12 → 12 |
| Playlist(ui-pl) | 6,795 | 4,772 | −30% | 475 → 152 |

ピッカーの定常(3.0 s)は −14% にとどまる: 定常の 190 フレームは、FL が 62.5 Hz で出す『全面の Invalidate』(§1)そのものなので、更新領域を正確に持っても全面は全面(残るのは A1 の外接矩形が余計に含めていた部分だけ)。

### 6.2 headless Chrome 156 の Worker の CPU(`tables/chrome-ab.tsv`。このマシン、負荷平均 10〜20、CDP の模擬入力)

Worker の CPU(ms/s。区間の壁時間あたりのエンジン Worker のスレッド CPU)。A1 = HEAD の runtime の bundle、A3b = 本変更の bundle。ポート 5311/5312 の `vite preview` を交互に動かした(`tools/run-chrome4.sh`)。ピッカーは `picker-cost2`(F8 でフェードイン 0.8 s、定常 3 s、Esc でフェードアウト 1.2 s)、Mixer は gpu1 の `move-1920`/`move-3240`(ヘッダーを 20 ステップで 300 px、8 s 測定。guest の画面 1920×1080 と 3240×1946)。**5 回ずつ**(3240 は 2 回ずつ)。

| 区間 | A1 平均(各回) | A3b 平均(各回) | 平均の変化 | 中央値 A1 → A3b |
|---|---|---|---:|---|
| ピッカー フェードイン | 427(188 200 506 605 635) | 396(165 395 412 488 518) | -7% | 506 → 412 |
| ピッカー 定常 3 s | 467(180 187 630 663 673) | 403(177 357 450 513 520) | -14% | 630 → 450 |
| ピッカー フェードアウト | 430(168 256 536 592 600) | 369(152 376 421 424 472) | -14% | 536 → 421 |
| Mixer のドラッグ 1920(ms/s) | 163(136 149 175 175 182) | 160(146 161 164 165 166) | -2% | |
| Mixer のドラッグ 1920(ms/move) | 2.72 | 2.67 | -2% | |
| Mixer のドラッグ 3240(ms/move、2 回) | 2.70 | 2.94 | +9% | |

presents/s(定常のピッカー): A1 5.9 → A3b 1.1。**読み**: 向きは下がる(平均 -2〜-14%)が、各回の値が A1 と A3b で大きく重なり(負荷平均 10〜20 の共有機で、値は 170〜670 ms/s の 2 つの山に分かれた)、**この測定では有意な差とは言えない**。Mixer のドラッグは Worker CPU が 1 ステップ約 2.7 ms で、アイドル(約 110〜190 ms/s)との差が小さく、A1 でも既にほぼ頭打ち。native の -14〜-62% が Chrome の Worker で見えないのは、wasm 側の費用の内訳が native と違う(コンパイル済みの C の比率、compose と put の比率)ためと思われるが、**内訳は未測定**。Firefox 158 は未実施。

**注意**: Chrome の A3b の bundle は S-0247-hidden を入れる前の runtime(t3b の 14:03 版)で作った。入れたあとの runtime は native の N1 でだけ測った(§6.1 は入れた後)。A1 の bundle は HEAD cbf9d1e5(alpha2b を含まない)。

### 6.3 畳み(fold)の回数(`tables/folds.txt`、`tools/folds.py`)

`fle_win_update_folds()` を G3 の各 probe で記録(A3b の native)。起動(`wiz0` → `e03`/`opened`)で 52〜56 回(project の読み込みと最初の全面の描画で、離れた小矩形が 32 個を超える)。**そのあと**:

| 操作(G3 の区間) | 畳み |
|---|---:|
| Mixer のドラッグ 40 ステップ(`e03`..`end`) | **0** |
| ピッカー F8 のフェードイン 50・定常 190・Esc・フェードアウト 75 フレーム(`steady`) | **0** |
| F8 の開閉(`f8`、`ui-f8`) | **0** |
| Playlist のドラッグ・ホイール・ズームなど(`ui-pl`、`opened` 以降) | **0** |
| 設定窓(`ui-win`)、メニュー(`ui-menu`) | **0** |
| F9 の隠す・見せる(`mxrep`) | **0** |
| File メニューの項目を選ぶ(`pull1` の `fm_open`→`fm_sel`→`fm00`) | 5 |

(`ui-pl` などの `e03`→`opened` の +4 は project の読み込み。)

### 6.4 付随の観察(表のみ。変更なし。`tables/native-api-time-steady-A1.tsv`)

Sleep(1)×16.9/フレーム、IntersectRect 1,453/フレーム、SelectClipRgn 47/フレームは §1.4。

## 7. 画素の違い — 受け入れ条件(i)〜(iv)(PM の返信に対する)

**画素の完全一致は条件ではない**(更新領域を契約どおり持つので、FL がどこまで描くかが変わる)。実エンジンの FL の描画は**冪等ではない**(A1 で main form 全体を完全再描画すると絵が毎回変わる: e03 のスナップで測定)。A3b は『描かれる回数が減る』分だけ、半透明の縁取りなどの小さい差が残る。

### 7.1 (i) G3 の各スクリプトの最後の絵で A1 と A3b が違う場所(`tables/pixdiff-classes.tsv`、`tools/classify.py`)

違う画素を y の帯に分け、各ビルドが**自分の完全再描画の絵**と一致しているかで読みを付ける(`A3b_conv` = A3b の絵が完全再描画の絵と同じ画素、`A1_conv` = A1 が同じ画素)。

| スクリプト | 帯(y、x) | 画素 | 最大の差 | 読み |
|---|---|---:|---:|---|
| 全スクリプト共通 | y4〜77、x300〜1120(ツールバー: 転送ボタン、テンポ、パターン、時計の周り) | 2.5k〜8.5k | 37(ui-f8・f8・pull1・mxrep には 154 の画素もある。4 段階以上は 28〜222 画素) | 半透明の縁の混ぜの収束の差(下の 7.3)。時計・メーターは走るたびに変わる帯 |
| 全スクリプト共通 | y86〜268、x377〜866 と x440〜563(チャンネルラックの行・PAT/SONG) | 0.6k〜5.9k | 3 | ≤3 段階の混ぜの差。見て分かる差ではない |
| wiz0(起動の最初の絵。ウィザードが出ている。全スクリプトの最初の probe) | 画面全体(窓の縁の 1 段階の差の枠、ツールバー、Playlist の格子) | 152,830(4 段階以上 75,364、16 段階以上 66,815) | 166 | 起動中の途中の絵。**A3b の Playlist に縦の格子線(小節線)が出ていて、A1 には無い**(`tables/wiz0-A1-A3b.png`。線の列 x 約 1290〜1880)。同じ格子は e03 では両方にある。つまり描く順序・タイミングの差で、描かれていない領域ではない。e03 では 2,538 画素(76 が ≥4 段階)に縮む |
| mxstart(Mixer のドラッグ前) | y575〜1041、x1675〜1896 | 3,412 | 3 | A1 は完全再描画の絵と同じ(`A1_conv` 3,412)、A3b は ≤3 段階違う(Mixer 右のスロット欄の混ぜ) |
| mxmove(ドラッグのあと) | **y1022〜1040、x599〜1676(Mixer 下端の鍵盤状の帯)** | 6,422(4 段階以上 6,327) | 46 | どちらにも帯が残り(A1 は約 120 画素の長さ、A3b は約 300 画素。1 ステップ 15 px のドラッグの約 8 ステップ分と約 20 ステップ分)、どちらも完全再描画の絵では消える。`A1_conv` 3,382 画素(A3b の長い部分)と `A3b_conv` 2,717 画素(A1 にだけ帯がある部分)の両方がある。読み(推測): FL が各ステップで描き足していく帯を、A1 は外接矩形の消去で多く消し、A3b は消す回数が少ない。**これが ≥16 段階の差の 1 つ(最大 46)**。完全再描画の絵はどちらも同じ(21 画素) |
| mxrep(F9 のあと) | y548〜1047、x1675〜1912(Mixer 右端) | 6,374(4 段階以上 4,984) | 165 | 7.4 |
| pull1 | y132〜248、x440〜563 | 3,454 | 2 | ≤2 段階 |

**読み**: 大きい差(≥16 段階)は mxmove の下端の帯と mxrep の右端の帯の 2 か所だけ。残りは ≤3 段階の混ぜの差で、見て区別できない。

### 7.2 (ii) 非画素の行(native)

`cmp-short.g3`(16 スナップ)の `g3-snap`/`g3-win`/`g3-wintable`/`g3-layered`/`g3-timers` の **94 行**を A1 と A3b で比べた(`tables/g3-lines-native-A1-vs-A3b.txt`): **clock_ms を除いて 94 行全一致**(`g3-wintable` の SHA、窓の数・種類・enabled、layered の更新数を含む)。違うのは仮想時計 `clock_ms` の 1〜4 ms の差のみ(A1 は 3 回とも同じ値、A3b も 3 回とも同じ値で、決定的。runtime が同じで記録器だけ違う 2 つの A3b のビルドの間にも 1〜5 ms の差があるので、更新領域の動作の差ではなく、仮想時計が『時計を読む回数』で進むことの差と読む。確認はしていない)。**ただし `g3-surface`(窓の面の SHA-256)の 34 行は 17 行が違う**(`tables/g3-surface-lines-cmp-short.txt`: main form の面〔lid 14〕が 16 スナップのすべてで違い、ウィザードの面〔lid 48〕は最初の 1 つだけ違う。画素が違うので当然)。基準の表(`g3-lines-base-84.txt` など)の `g3-surface` 行は取り込みの後に作り直しが要る(MERGE-NOTES)。**node の仮想時計の wasm での 84 行・55 行(参照 project)は未実施**。

### 7.3 (iii) 操作のあとの絵と『全 top-level 窓を 1 回完全に描き直した絵』の差(`tables/rp-incr-vs-full.tsv`、`scripts/pk15.gdb`、`tools/rp-cmp.py`)

G3 の最後(`snap fin`)で gdb から全 top-level 窓へ `RedrawWindow(0x185)` を 1 回呼び、呼ぶ前の絵と後の絵を画素で比べる(1920×1080)。

| スクリプト | A1: 全 / ≥4 段階 / ≥16 段階 | A3b: 全 / ≥4 / ≥16 | A3b ≤ A1 | A1 対 A3b(完全再描画後) |
|---|---|---|---|---:|
| mxstart(ドラッグ前) | 7 / 6 / 0 | 6,511 / 18 / 4 | **NO** | 1,473 |
| mxmove(Mixer のドラッグ) | 10,897 / 10,717 / 5,680 | 14,654 / 11,490 / 6,405 | **NO**(+773 が ≥4) | 21 |
| steady(ピッカー) | A1 は停止(下) | 2,784 / 24 / 4 | 比較できない | - |
| ui-menu(メニュー) | 15,716 / 125 / 112 | 2,786 / 26 / 4 | yes | 28 |
| ui-pl(Playlist) | 11,030 / 140 / 113 | 4,285 / 40 / 4 | yes | 42 |
| ui-win(設定窓) | 15,715 / 125 / 112 | 2,785 / 25 / 4 | yes | 14 |
| ui-f8 | 15 / 9 / 0 | 2,786 / 26 / 4 | **NO**(≤3 段階) | 80 |
| f8 | 0 / 0 / 0 | 2,781 / 21 / 4 | **NO**(≤3 段階) | 75 |
| pull1 | 15,724 / 156 / 147 | 8,961 / 23 / 4 | yes | 77 |
| mxrep(F9) | 8,493 / 149 / 139 | 9,708 / 5,007 / 4,646 | **NO**(§7.4) | 77 |

読み: (1) **完全再描画の絵は A1 と A3b でほぼ同じ**(14〜80 画素。mxstart だけ 1,473)。つまり『描かれ方の履歴は違うが、収束先は同じ』。(2) A3b の差の大半は ≤3 段階(ツールバーなどの半透明の縁取り): FL の描画が冪等でなく、A1 は全面を何度も描いて収束し、A3b は少ない回数で止まる。(3) 『A3b ≤ A1』を満たさない 5 本のうち、mxstart・ui-f8・f8 の超過は ≤3 段階で ≥4 段階は 18〜26 画素。mxmove の超過は下端の鍵盤状の帯(7.1)、mxrep の超過は 7.4 の帯。(4) steady の A1 は、完全再描画の途中でゲストが `FillRect` に DC でない hDC(NULL)を渡して停止した(minimize/restore のあとでも A1 は同じ停止)。A3b は停止しない。A1 側の原因は未確認。

### 7.4 (iv) 隠す・見せる、minimize/restore、窓を動かして露出

| 操作 | 結果 |
|---|---|
| minimize → restore(main form。`rp-minres.g3`、`pk16.gdb`、`tables/minimize-restore.tsv`) | A3b: 前と後の絵の差 **65 画素**(A1 は 10,359)。restore のあとの絵と完全再描画の絵の差 2,479 画素(≥4 段階 20、最大 37)。**0 件の描き漏らし**(大きい差なし)。A1 は完全再描画で停止(7.3 の 4) |
| 窓を動かして露出(Mixer のドラッグ、設定窓の開閉、メニューの閉) | A3b は mxmove の下端の帯(7.1)と ≤3 段階の差のみ。設定窓・メニュー・Playlist は A1 より小さい(7.3) |
| **F9 の隠す→見せる(`mxrep.g3`)** | **未達**: 完全再描画の絵との差が A3b 4,646 画素(≥16 段階)、A1 139。場所は x1903〜1912、y548〜1019 の 10×470 の帯(Mixer の右端の外) |

F9 の帯の調べ(`tools/run-n1.sh` + `A1_CROP="1880,548,40,472"`、`scripts/pk17.gdb`、`tables/st-strip.png`):
- 帯の 40×472 画素は、ドラッグ直後(`end`)に A1 と A3b で**全く同じ**(差 0)。Mixer の右端の外に、Mixer の古い像(閉じるボタンの X の一部)が残っている。完全再描画の絵では消える(mxmove でも A1・A3b ともに 4,720 画素の差)。
- F9 で隠したあと(`f9off`)、A1 では帯が消えて main form の地(Playlist の格子と暗い面)になり、A3b では古い像が残る(A1 と A3b の帯の差 9,912 画素)。見せたあと(`f9on`)は A1 は完全再描画の絵と一致(0 画素)、A3b は 5,181 画素違う。
- F9 のあとの `BeginPaint` の矩形は、A1 と A3b でほぼ同じ 521 回ずつ(`mx_end` 以降。A1 に `(4,4,1510,353)` が 1 回、A3b に `(0,466,1020,966)` が 1 回、`(0,0,1514,500)` が 34 回対 35 回などの違い)で、**どちらも x ≥ 1903 を含まない**。BeginPaint の数も同じ(`f9off` 区間 161/161、`f9on` 区間 289/288)。→ A1 で帯が消える経路は BeginPaint ではない(消去背景か、窓の隠す/見せるのビットのコピーか。**特定していない**)。
- 帯の 1 画素(main form の面の (1910,606))へのウォッチポイント(`scripts/pk18.gdb`、`runs/wp-mxrep-a3/gdb.txt`〔logs/ に抜粋〕): **A1 は F9 の間にその画素へ書く呼び出しが 1 回**(`fle_r_blit` ← `fle_imp_BitBlt` ← ゲストの `f_6bb1b0` ← `f_8261a0` ← `f_806b70` ← `f_410090`〔Dispatch〕。宛先 (0,0)・幅 1047、値 0x384146 = 背景)、**A3b は 0 回**。つまり A1 では BitBlt を出した窓の更新領域(外接矩形)に帯が入っていて、A3b では更新領域の矩形の並びに帯が入っていない。帯は Mixer の右に覗く別の窓(幅 1047、Playlist と思われる)の領域で、ドラッグ中にその窓の露出した帯が再描画されないことが両方のビルドに共通の原因に見える(露出の無効化〔`unc_*`〕が帯を含まないのか、FL がその帯を描かないのか。**未特定**)。
- `mx_start` 以降に右端が 1000〜1050 の BeginPaint の矩形は A1・A3b とも同じ 3 回(`scripts/pk19.gdb`、`runs/pw-mxrep-*`): (1041,466,1047,966)・(0,466,1020,966)・(0,0,1047,998)。つまり幅 1047 の窓は両方で窓全体(0,0,1047,998)の WM_PAINT を 1 回受けるのに、A3b だけ帯の画素に書かない。rcPaint は外接矩形なので、**A3b の更新領域は窓全体を囲む矩形の並びだが帯を含まない**(帯が途中の `ValidateRect`/`ValidateRgn` で除かれた)と読むのが筋が通る。A1 は『一部の除去は何もしない』(旧実装)ので帯が残って描かれた。この読みなら A3b は契約どおり(Wine 10.0 と同じ行 7〜10)で、帯が古いまま残るのは FL がドラッグ中に露出した帯を描かないことによる。**ただしその窓の ValidateRect/ValidateRgn の呼び出しは記録していない(未確認)**。
- `S-0247-hidden` を外した ビルド(`work/n1-a3x`)でも同じ(4,647 画素)なので、その規則が原因ではない。
- 読み: ドラッグで露出する側の帯が再描画されない既存の欠陥(A1 にもある)を、A1 は F9 の過大な更新で後から隠していた可能性が高い。A3b は新しい描き漏らしを作ってはいないが、**隠していた物を隠さなくなった**。修正は行っていない(原因の位置が特定できていない。推測で FL の窓の名前や位置の特別扱いを足さない)。

## 8. 違反・逸脱の開示

- 作業中に 1 度、バックグラウンドのコマンドの末尾に `&` を付けた(`run-ab.sh` の起動、14:19)。規則違反で、以後は `background: true` だけを使った。
- 重い処理は 2 プロセスを超えた時間帯があった(wasm の bundle 2 本の `wasm-opt` + native の G3 実行 1 本)。
- 以前(12:33)の『`tar` の失敗で `patch` が PM の木に当たった』件は既報・是正済み。
- PM の木には書いていない。パッチの確認は自分の木(`lift/out-alpha3`)だけ。`git commit`/`push` はしていない。

## 9. 成果物(`lift/out-git/alpha3-results/`)

`alpha3.diff`(HEAD に当てる)、`docs/0247-update-region-pieces.md`(本書)、`docs/README-0247-line.txt`、`MERGE-NOTES.md`、`tables/`(上の表)、`tools/`(記録器、ビルド、駆動、集計)、`scripts/`(G3 のスクリプトと gdb のスクリプト)、`logs/`。


## 10. 決定 0247 追補(alpha3b2): F9 の古い帯と Mixer 下端の帯は**我々の欠落**だった — 窓の移動で露出した部分のうち、子窓の上にある部分を、子窓をクリップしない親(Playlist)の更新領域に入れていなかった。入れるようにした。仮の決定

状態: **仮の決定**(0247 の追補節。番号は増やさない)。担当 alpha1(PM の指示 = 『alpha3b』『alpha3b2』、2026-11-05)。基準は HEAD **84e17206**(runtime は e840e4d4 = 6eb1737d〔opt1 の層 b・c・d + ship6〕と同一。c5ed023b も ci/ だけの変更で runtime は同一)に `alpha3r.diff`(alpha3 の再ベース)を重ねた木。**この節の native の表は、HEAD の生成 C(`fe-lift lift --pdata` の sha256 は 34a376db…78062a〔PM の値と一致〕)から作り直した N1 で測った**(runtime は HEAD の物〔A1〕、alpha3r〔A3b〕、alpha3r + 本修正〔A3b+fix〕、HEAD + 本修正〔A1+fix〕の 4 通り)。**実機 Windows、依頼者の機械、実 GPU の値は 0 件**。入力は G3 の模擬入力で、ユーザー操作ではない。

**何を実測したか**: 実エンジン(FLEngine と同梱 DLL)の native -O2 の N1 に、窓の移動(`SetWindowPos`)・露出の無効化・`BeginPaint`・`WM_PAINT` の配送・窓/DC を引数に取る API の呼び出しを時系列で出す一時の記録(成果物には入れない)を足して、Mixer のヘッダーを G3 でドラッグしたあと F9 で隠して見せる一連(`mxrep.g3`)を走らせ、帯の 1 画素へのウォッチポイントと突き合わせた(alpha3b)。決定 0214 の `present2_uncover` の 3 つの場面と、親・子・兄弟の場面(`win_unc.c`)を Wine 10.0 でこの機械で実走し、期待と比べた。直しの前後で、全 G3 スクリプトの最後の絵と『全 top-level 窓へ RedrawWindow を 1 回呼んだあとの絵』を画素で比べた。

**結論**: (1) 帯の原因は**我々の側の欠落**(ゲストが描かないのではない)。ドラッグの最後の一歩(Mixer `lid 64` が x=32→16)で、Mixer の下から現れた Playlist(`lid 53` TEventEditForm、WS_CLIPCHILDREN なし)の右端の帯のうち、**子窓 `lid 93`(TNumBar)の上の幅 10 の部分は `lid 93` にだけ無効領域が入り、Playlist 自身には入っていなかった**。TNumBar は `WM_PAINT` に `ValidateRgn(hwnd, NULL)` で答えるだけで描かない(BeginPaint も DefWindowProc も呼ばない。Mixer の子 86・83 も同様)。Playlist が TNumBar の見た目を描く作りと読めるが、誰もその 10×500 画素(と Mixer 下端の鍵盤状の帯)を描かず、古い像が残った。HEAD(A1)にも同じ帯が最初からあり、A1 は F9 の外接矩形が偶然覆っていた(alpha3 が作った不具合ではない)。(2) 直し: `unc_end` で、**子をクリップしない窓(WS_CLIPCHILDREN なし)の更新領域に、『子をクリップしない可視領域(兄弟はクリップ)』の旧→新の増加分も足す**(動いた窓自身と、その窓と一緒に動く子孫には、コピーで残るビットを引いた増加分)。これまでの『子をクリップした増加分』は残す。`unc_invalidate` は**変えない**(領域の外接矩形を入れる。矩形ごとに入れる版は起動直後の絵を壊した。§10.5.3)。(3) 効果: 全 top-level 窓へ RedrawWindow を 1 回呼んだ前後の絵の差(画素 全 / ≥16 段階 / 最大)は、Mixer のドラッグのあと(mxmove)A1(HEAD)10,897 / 5,679 / 165、A3b(alpha3r)13,308 / 6,406 / 165 → **A3b+fix 3,778 / 5 / 37**(帯 4,485 画素 → **0**)。F9 のあと(mxrep)A3b 8,362 / 4,647 / 165(帯 4,485)→ **A3b+fix 3,362 / 5 / 37**(帯 0)。HEAD の A1 の mxrep は F9 の外接矩形が覆うので 241 / 0 / 15 で帯は出ない。Mixer 下端の鍵盤状の帯(y1022〜1040)も同じ原因で消えた。他の G3(ui-pl・ui-menu・ui-win・ui-f8・steady)は A3b と A3b+fix が一致し、**A3b+fix の ≥16 段階は全スクリプトで 7 画素以下**(最大差 37、上の帯のメーター・時計の周り)。起動の 16 スナップは A3b と A3b+fix で g3 の 94 行も g3-surface 34 行も合成画面 15 点も**全て一致**(§10.5.4)。 (4) テスト: `present2_uncover` 16/16(期待は変更なし)、`upd_region.c` 40 項目、`win2_act`、`window`、`wall_ia1w` が通る(§10.4)。

**未解消の差・未確認**: Wine 10.0 との差が 3 点残る(PM の決定で、描き残しが出ない向きの 0214 の期待を保った。§10.2.2): (a) sibling を HWND_BOTTOM にしたとき、Wine は下から現れた窓に無効領域を渡さない(0214 は渡す)、(b) Wine は最上位窓にも露出の矩形を渡す(こちらは渡さない)、(c) Wine は WS_CLIPCHILDREN ありの親にも子の上の露出を渡す(こちらは渡さない)。実機 Windows で同じか(Wine は Windows の実測ではない。UNOFFICIAL)。Playlist の `WM_PAINT` が TNumBar の見た目を描く、という読みの直接確認(Playlist の BitBlt の宛先と中身を窓ごとに見ていない)。Chrome・Firefox・node の 84 行・55 行(FS 版の bundle が無い)と CPU は alpha3b2 では**測っていない**。`a4` は走らせていない(PM が取り込み時に回す)。

### 10.1. 時系列(alpha3b。`tables/trace-last-drag-step-A3b.txt`、`scripts/pk20.gdb`、`pk21.gdb`)

窓の木(`mxrep.g3` の起動後。`wintable`): main form `lid 14` TFruityLoopsMainForm(-5,-6 1930×1092)→ `lid 16` TWPControl(378,88 1546×998、style 0x54000000 = CHILD|VISIBLE|CLIPSIBLINGS)→ { `lid 64` TFXForm "Mixer - Master"(16,466 1514×500、同 style)、`lid 53` TEventEditForm "Playlist"(499,0 1047×998、style 0x54000000 = **CLIPCHILDREN なし**) } → `lid 93` TNumBar(53 の中の 1020,48 21×944、style 0x56000000 = CLIPCHILDREN あり)。画面では `lid 16` の x 1530〜1546 が 1903〜1919、`lid 53` の x 1031〜1041 が TNumBar のクライアント x 11〜21 = 画面 x 1903〜1912、y 466〜966 が画面 y 548〜1019(= 帯)。

ドラッグの最後の一歩(`SetWindowPos lid 64 x=32→16`、flags 0x14)のあと:

| 順 | 記録 | 意味 |
|---|---|---|
| 1 | `unc_invalidate lid=53 … bbox(client)=(1041,466,1047,966)` | Playlist 自身に入った露出は右端の幅 6(TNumBar の右、x 1041〜1047)だけ |
| 2 | `unc_invalidate lid=93 … bbox(client)=(11,418,21,918)` | TNumBar の露出した部分(幅 10)は TNumBar にだけ入る |
| 3 | `unc_invalidate lid=64 / 83 / 86 …` | Mixer 自身と、その子 83・86 の下端 (0,446,…,465) の露出も、それぞれの窓にだけ |
| 4 | `paint_pending lid=64` → `BeginPaint lid=64` → `paint_pending lid=53` → `BeginPaint lid=53 bbox=(1041,466,1047,966)` | Mixer と Playlist は描く |
| 5 | `paint_pending lid=86` → `upd_clear lid=86`、`lid=83` も同じ、`paint_pending lid=93` → **`API ValidateRgn(hwnd=lid93) rdx=0`** → `upd_clear lid=93` | 子窓 86・83・93 は `WM_PAINT` を受けても **BeginPaint も DefWindowProc も呼ばずに領域を空にする**。93 は API を記録して `ValidateRgn(NULL)` だと分かった(86・83 は呼び出した API を記録していない)。何も描かない |

→ 露出した 10×500 画素(TNumBar の上)も、Mixer の下端(86・83 の上)も、**誰の `WM_PAINT` でも描かれない**。A1(HEAD)も同じ。F9 のあとの A1 では、同じ画素に書く呼び出しが 1 回だけあった(gdb のウォッチポイント。FL の `WM_PAINT` 内の宛先 (0,0)・幅 1047 の BitBlt、値 0x384146 = 背景。`logs/wp-mxrep-A1-watch.txt`)。A1 の更新領域は外接矩形なので、F9 でできる窓全体の更新が帯を含んだ。alpha3 の更新領域は正確になったので含まず、既存の欠落があらわになった。

### 10.2. 契約と Wine 10.0

#### 2.1 契約

- Learn(Window Styles): WS_CLIPCHILDREN は『親窓の中に描くとき、子窓が占める領域を除く』。付いていない窓は子窓の上にも描ける(Playlist が TNumBar の見た目を自分で描く、という FL の作りと合う)。Learn(SetWindowPos): SWP_NOREDRAW の説明は『動かした結果、露出した親窓の部分』を挙げる。**子窓の上の露出を誰の更新領域に入れるか**は Learn に書かれていない。

#### 2.2 Wine 10.0 の実走(この機械。`harness/alpha3/win_unc.c`、`scripts/win_p2.c`)と食い違いの表(`tables/win-vs-0214-vs-alpha3b2.tsv`)

決定 0214 の `present2_uncover` の場面(S1 移動・NOREDRAW・NOCOPYBITS、S2 sibling を HWND_BOTTOM、S3 孫が動く)と、親・子・兄弟の場面(U1: 最上位窓 T の中の親 P と P の右端の子 C、T の子 M が P の上から退く。CLIPCHILDREN なし/ありの 2 通り)を Win32 API で書いて Wine 10.0 で実走した。結果:

| scene | window | Wine 10.0 (this machine) | decision 0214 test (HEAD) | alpha3b2 (runtime) | Wine = 0214 = alpha3b2 ? |
|---|---|---|---|---|---|
| S1.0 child moves, SWP_NOREDRAW | parent | 0 | parent 0 0 0 0 0 | pass (same as 0214) | same |
| S1.0 | kid | 0 | kid 0 0 0 0 0 | pass | same |
| S1.1 child moves, SWP_NOCOPYBITS | parent | (10,20,110,70) | parent 1 10 20 110 70 | pass | same |
| S1.1 | kid | (0,0,100,50) | kid 1 0 0 100 50 | pass | same |
| S1.2 child moves (plain, no overlap) | parent | (10,20,110,70) | parent 1 10 20 110 70 (a_child_that_moves_without_overlap…) | pass | same |
| S1.2 | kid | 0 | kid 0 0 0 0 0 | pass | same |
| S2 sibling B goes to HWND_BOTTOM | parent | 0 | parent 0 0 0 0 0 | pass | same |
| S2 | a (was covered by B) | 0 | a 1 40 40 100 100 | pass (= 0214) | **Wine differs from 0214 and alpha3b2** |
| S2 | b | 0 | b 0 0 0 0 0 | pass | same |
| S3 grandchild moves inside its parent | top | 0 | top 0 0 0 0 0 | pass | same |
| S3 | kid2 | (5,5,55,35) | kid2 1 5 5 55 35 | pass | same |
| S3 | gk | 0 | gk 0 0 0 0 0 | pass | same |
| U1 win_unc, no WS_CLIPCHILDREN on T and P | P: part beside C [150,250) | in | (no test) | in | same |
| U1 | P: part over C [250,300) | in | (no test) | in | same |
| U1 | C | (0,0,50,150) | (no test) | (0,0,50,150) | same |
| U1 | T (top-level) | (150,0,300,150) | (no test) | 0 | **Wine differs from alpha3b2** (decision 0214 gives a window beneath only) |
| U2 win_unc, WS_CLIPCHILDREN on T and P | P: part beside C | in | (no test) | in | same |
| U2 | P: part over C [250,300) | in | (no test) | NOT in | **Wine differs from alpha3b2** (P clips its child: its own visible region does not include C) |
| U2 | C | (0,0,50,150) | (no test) | (0,0,50,150) | same |
| U2 | T | (150,0,300,150) | (no test) | 0 | **Wine differs from alpha3b2** |

- **S1・S3 は Wine = 0214 の期待 = 本修正の結果**。S1.1(NOCOPYBITS)で Wine の親は動いた子の旧い場所 (10,20,110,70) だけを受け取り、子の新しい場所は受け取らない — 『移動した窓の新しい場所も祖先に入れる』広い版(alpha3b)は Wine と違ったため捨てた。
- **食い違い 3 点(未解消)**: (a) **S2**: Wine は sibling を HWND_BOTTOM にしても下から現れた窓 a に無効領域を渡さない(0214 の期待は a に (40,40,100,100) を渡す。0214 のテストの注は Learn を挙げ、Wine とは書いていない)。(b) **U1/U2 の T**: Wine は最上位窓にも露出の矩形 (150,0,300,150) を渡す(本修正は渡さない)。(c) **U2 の P**: Wine は WS_CLIPCHILDREN ありの親にも子の上の露出を渡す(本修正は渡さない)。PM の決定(『過剰な無効化は契約上安全、渡さない側は描き残しの危険』)で、今回は 0214 の期待を保ち、(a) は渡す側のまま、(b)(c) は渡さない側のままにした。最上位窓や CLIPCHILDREN ありの親に渡さないことで描き残しが出る場面が実エンジンで見つかったら、渡す側に変える。実エンジンの G3 では描き残しは出ていない(§10.5)。

### 10.3. 直し(`alpha3b2.diff`。`lift/runtime/fle_win.c` 1 ファイル + テスト)

`unc_end` / `unc_begin`: 窓ごとの『新しく見えるようになった部分』(子をクリップした可視領域〔CLIPCHILDREN | CLIPSIBLINGS〕の旧→新の増加分。決定 0214)はそのまま。**さらに、WS_CLIPCHILDREN の無い窓について、子をクリップしない可視領域(CLIPSIBLINGS だけ)の旧→新の増加分も足す**(`v0n`)。動いた窓自身と、その窓と一緒に動く子孫(Mixer と、その子 86・83 など)では、増加分から『コピーで残るビット』(`valid0n` を動かし量だけずらして `copied` と重ねた物)を引く。`unc_invalidate` は変えない。クラス名・窓の種類・位置による分岐 0、新しいオプション 0。S-0214-erase(露出は背景の消去つき)はそのまま。

PM の指示は『各窓の更新領域に足すのは、その窓自身の可視領域〔自分の旗で計算〕の旧→新の増加分』だった。**そのままでは足りない理由**(確認した): 動いた子の親(CLIPCHILDREN なし)の可視領域は子の上を含むので、子が動いても変わらず、親は動いた子の旧い場所を受け取れない。`present2_uncover` の swp_nocopybits(期待 `parent 1 10 20 110 70`)と、Wine 10.0 の同じ場面の実走(`parent 1 10 20 110 70`)の両方がそれを要求する。そこで『現行(子をクリップした増加分)∪ 自分の旗で求めた増加分』とした。Playlist(`lid 53`、CLIPCHILDREN なし)は後者で TNumBar の上の帯を受け取る。最上位窓は(子をクリップしない可視領域が変わらないので)後者では何も受け取らず、孫が親の中で動くとき最上位窓に何も入らない(0214 の期待どおり)。

この直しが **(b)(外部 API = SetWindowPos/WM_PAINT の契約の実装)に当たる理由**: 移動で露出した部分の `WM_PAINT` が誰にも行かないのは、我々が窓ごとの露出を『その窓だけが見える領域』で数え、子をクリップしない窓が子の上にも描ける分を数えていなかったため。FL の挙動の代行・補正ではない(FL は TNumBar の `WM_PAINT` に何もせず、Playlist の `WM_PAINT` で描いている)。

### 10.4. テスト

- **`test.sh full`(最終の木: HEAD 84e17206 + alpha3r.diff + 本修正、`harness/test.sh full --jobs 2`、`logs/test-full-t4-final.log`)**: **200 results / 2,347 passed / 0 failed / 1 ignored、SKIPPED-lines=1(既知の `FLE_FR9T_CLEAN_RT` の 1 件)、rc=0**。(最初の実行は `binaries/` の無い木で 94 行が skip されたので、`binaries` を足してやり直した値。)alpha3r.diff だけの `test.sh full` はこの HEAD では走らせていない。
- 個別: `present2_uncover` **16/16(期待は変更なし)**、`alpha3_updregion`(`upd_region.c` 40 項目)、`win2_act` 2/2、`window` 37/37、`wall_ia1w` 7/7 が通る。
- `upd_region.c`(40 項目。`uncover_scene` を CLIPCHILDREN の有無の 2 通り × 5 項目): 修正後 **0 失敗**、修正前(alpha3r のみ)**1 失敗**(CLIPCHILDREN なしの親が子の上の露出 [250,300) を持たない。`logs/upd-region-alpha3b2-*.txt`)。
- Wine 10.0 の実走(`win_p2.c`、`win_unc.c`)と表: §10.2.2。
- **生成 C の sha**: `fe-lift lift FLEngine_x64.dll OUT.c --pdata`(同じ木の lifter。HEAD e840e4d4 + alpha3r + alpha3b の時点で実行)の sha256 は **`34a376dbf23a30c62df5ce6d04c07ed9beb5b651dd82d608cf5bcc59af78062a`、215,740,414 B**(PM の値と一致)。本差分は lifter のソースを 1 つも変えず、HEAD 84e17206 の `lift/crates`・`lift/runtime` は e840e4d4 と同一。
- **a4 は走らせていない**(PM が取り込み時に回す)。

### 10.5. 測定(native -O2 の N1、G3 の模擬入力。**すべて HEAD の生成 C から作り直した N1**)

#### 5.1 全 top-level 窓へ RedrawWindow を 1 回呼んだ前後の絵の差(`tables/alpha3b-rp.tsv`、`tools/rp-cmp2.py`、`scripts/pk15.gdb`)

各スクリプトの最後で全 top-level 窓へ `RedrawWindow(0x185)` を 1 回呼び、その前後の絵の差(画素: 差のある画素 / ≥4 段階 / ≥16 段階 / 最大の差。strip = x 1895〜1919、y 548〜1019 の ≥16 段階の画素)。A1 = HEAD の runtime、A3b = HEAD + alpha3r、A3b+fix = + 本修正、A1+fix = HEAD + 本修正。

| script | A1 all/>=4/>=16/max | A1+fix all/>=4/>=16/max | A3b all/>=4/>=16/max | A3b+fix all/>=4/>=16/max |
|---|---|---|---|---|
| mxmove | 10897/10717/5679/165 (strip 4485) | 2030/6/0/15 (strip 0) | 13308/11491/6406/165 (strip 4485) | 3778/20/5/37 (strip 0) |
| mxrep | 241/11/0/15 (strip 0) | 8492/146/139/22 (strip 0) | 8362/5008/4647/165 (strip 4485) | 3362/24/5/37 (strip 0) |
| ui-pl | 11030/140/111/22 (strip 0) | (not run) | 4285/41/7/37 (strip 0) | 4285/41/7/37 (strip 0) |
| ui-menu | 15716/125/111/22 (strip 0) | (not run) | 2786/27/5/37 (strip 0) | 2786/27/5/37 (strip 0) |
| ui-win | 15715/124/111/22 (strip 0) | (not run) | 2785/26/6/37 (strip 0) | 2785/26/6/37 (strip 0) |
| f8 | 0/0/0/0 (strip 0) | (not run) | (none: guest stopped inside the repaint) | (none: guest stopped inside the repaint) |
| ui-f8 | 14/11/0/15 (strip 0) | (not run) | 2786/27/5/37 (strip 0) | 2786/27/5/37 (strip 0) |
| pull1 | (none: guest stopped inside the repaint) | (not run) | 8961/24/5/37 (strip 0) | 8961/24/5/37 (strip 0) |
| mxstart | (none: guest stopped inside the repaint) | (none: guest stopped inside the repaint) | 5165/19/5/37 (strip 0) | 5165/19/5/37 (strip 0) |
| steady | 13/10/0/12 (strip 0) | 13/10/0/12 (strip 0) | 2784/25/5/37 (strip 0) | 2784/25/5/37 (strip 0) |

読み: (1) mxmove と mxrep で、A3b にあった帯(4,485 画素、最大差 165)と Mixer 下端の帯が消えた。(2) A3b+fix の ≥16 段階は mxmove・mxrep・ui-pl・ui-menu・ui-win・ui-f8・pull1・steady・mxstart で 5〜7 画素(最大差 37)。A1 との比較(≥16 段階): ui-pl・ui-menu・ui-win は A3b+fix が小さい(7・5・6 対 111)、mxmove も小さい(5 対 5,679)。mxrep・ui-f8・steady は A1 の方が 5 画素ほど小さい(0 対 5。上の帯のメーター・時計の周り)。≥4 段階では A3b+fix 20〜41 画素 対 A1 0〜11(mxrep・ui-f8・steady)で、これは ≤3 段階の混ぜの差(FL の描画が冪等でなく、A3b は A1 より描く回数が少ない)が主。(3) **全 top-level 窓への RedrawWindow がゲストの `FillRect`(DC でない hDC)で停止する場面が、ビルドごとに別のスクリプトで出る**: A1 は pull1・mxstart、A3b と A3b+fix は f8(A1 は f8 を完了する)。この停止は gdb から窓全体を描き直す測定の限界で、原因は未確認(alpha3b の報告の steady・minimize/restore の A1 の停止と同じ型)。停止した組は表で『none』。(4) A1+fix(HEAD に本修正だけ)は mxmove で帯が消える(10,897 → 2,030、帯 4,485 → 0、≥16 段階 0)が、**mxrep では A1 の 241 / 0 より悪い 8,492 / 139**(A1 の外接矩形の更新領域と本修正の相互作用と思われる。原因は未調査。A1 は取り込み対象でない)。

#### 5.2 A1 対 A3b+fix(同じ 2 時点の絵。`tables/alpha3b-rp-cross.tsv`)

| script | A1 vs A3b+fix, incremental (all/>=4/>=16/max) | A1 vs A3b+fix, after one complete repaint |
|---|---|---|
| mxmove | 14497/10789/5735/165 | 72/62/52/154 |
| mxrep | 3123/15/4/37 | 10/3/0/6 |
| ui-pl | 9272/139/115/37 | 41/24/0/10 |
| ui-menu | 13651/139/115/37 | 27/16/0/9 |
| ui-win | 13703/187/167/154 | 77/62/52/154 |
| f8 | (A3b+fix: the guest stopped inside the complete repaint) | (none) |
| ui-f8 | 2848/85/43/154 | 78/64/39/154 |
| pull1 | (A1: the guest stopped inside the complete repaint) | (none) |
| steady | 2796/37/17/37 | 24/17/13/28 |
| mxstart | (A1: the guest stopped inside the complete repaint) | (none) |

#### 5.3 『移動した窓の部分木を除く版で mxmove の絵が壊れる』件(alpha3b で未解明だった物)

**再現した**。alpha3b の途中で作った版(子孫の露出を足し、さらに `unc_invalidate` を領域の外接矩形でなく**矩形ごと**に入れる版)を HEAD の生成 C の N1 で動かすと、Mixer の右側(画面 x 約 1140〜1905)に Playlist の画素が出て、mxmove は全窓描き直しとの差が 476,739 画素(≥16 段階 173,801、最大 202)、**ドラッグ前の mxstart も 472,280 画素**になる(表 `tables/mxrepro-variants.tsv`)。切り分け: (a) alpha3r だけ(A3b)は壊れない(mxmove 13,308、mxstart 5,165)。(b) **`unc_invalidate` だけを外接矩形に戻した版は壊れない**(mxstart 5,165 = A3b と同じ、mxmove 8,308)。(c) 最終形(外接矩形のまま + 『子をクリップしない可視領域の増加分』を足す)も壊れない(mxstart 5,165、mxmove 3,778)。→ 原因は『露出した領域を正確に(矩形ごとに)入れる』と、外接矩形が偶然覆っていた古い画素が覆われなくなることにある。つまり**ある窓の『新しく見えるようになった部分』の計算(`v1 − コピーで残るビット`)が、実際に古い画素が残っている部分の一部を落としている**(落ちているのは Mixer の子 86・83 の右側と読めるが、どの窓のどの矩形かは特定していない)。alpha3(A3b)の `unc_invalidate` も外接矩形のままなので、alpha3r は影響を受けない。起動直後の Mixer(`lid 64`、Playlist の下にある時期)の『新しく見える部分』は 6 個の矩形に分かれる(`logs/uncmulti-mxstart-alpha3b2.txt`)、というところまで。**この穴の原因は未解明で、外接矩形で隠したまま**(最終形は `unc_invalidate` を変えない)。

#### 5.4 起動の 16 スナップ(`cmp-short.g3` に `cpu` を足した `cmp-short-c.g3`、native)

- 非画素の行(`g3-snap`〔clock_ms などを除く〕/`g3-win`/`g3-wintable`/`g3-layered`/`g3-timers`)**94 行**: A1 対 A3b、A3b 対 A3b+fix、A1 対 A3b+fix のどれも**違い 0 行**。
- `g3-surface`(窓の面の SHA-256)34 行: A1 対 A3b は 17 行(alpha3 の報告のとおり。main form の面が 16 スナップのすべてで違う)。**A3b 対 A3b+fix は 0 行**(34 行すべて一致)。合成画面の 15 点(`cpu` を足した各 probe)も A3b と A3b+fix で**全点一致**(`tables/cmp-short-surface-A3b-vs-A3bfix.txt`)。つまり本修正は起動の基準の表を変えない(alpha3 の分の 17 行の作り直しだけが要る)。node の wasm の 84 行・55 行は**未実施**(FS 版の bundle が無い)。

#### 5.5 `fle_win_update_folds`(A3b+fix、native。`tables/folds-A3b-fix.txt`)

起動(`wiz0` → `e03` / `opened`)で **52〜56 回**。**そのあと**: Mixer のドラッグ(`mxmove`)**0**、F9(`mxrep`)**0**、ピッカー F8 のフェードイン・定常・フェードアウト(`steady`)**0**、F8(`f8`、`ui-f8`)**0**、Playlist(`ui-pl`)**0**、メニュー・設定窓(`ui-menu`、`ui-win`)**0**。File メニューの項目を選ぶ(`pull1` の `fm_open`→`fm_sel`→`fm00`)**5**(alpha3 と同じ)。→ PM の条件『Mixer 移動・ピッカー・Playlist で 0』を満たす。

### 10.6. 違反・逸脱

- 重い処理(`native-gen.sh` の 2 ジョブ、`test.sh full` の 2 ジョブ、cargo のビルド)が同時に走った時間帯があった(2 本までの指示の範囲内の本数だが、機械の負荷平均は 20〜35)。末尾 `&`・`pkill -f`・`sleep`・`while` は使っていない。前景の `tail --pid` は自分のジョブの PID を確認してから。PM の木には書いていない(自分の `lift/out-alpha3` の下だけ。`git archive` した木 `t4` に `lift/out-*` の入力を symlink した)。
- 前回(alpha3b)の後に `lift/out-alpha3` を 2.9 MB まで清掃したため、native N1 の生成 C を HEAD から作り直した(約 15 分)。作業の最後に再び清掃する。

### 10.7. 成果物(`lift/out-git/alpha3b2-results/`)

`alpha3b2.diff`(**alpha3r.diff の上に**、HEAD 84e17206 基準。alpha3b.diff を置き換える)、`docs/0247-update-region-pieces.md`(alpha3 の本文 + 追補の注 + §10 追補〔番号は増やさない〕)、`docs/README-0247-line.txt`、`MERGE-NOTES.md`、`tables/`、`tools/`、`scripts/`、`logs/`。


## PM の決定と取り込み(2026-11-05)

取り込んだ基準: HEAD 26a7cf4e に、`alpha3r.diff`(107 ファイル。`-` は fle_win.c の旧い外接矩形の代入・拡張 49 行だけ)を `patch -p1`(offset・fuzz・reject なし)で当ててコミット 2bb41c27(第 1 部)にし、続けて `alpha3b2.diff`(54 ファイル。`-` は 3 行で全部テスト: `upd_region.c` の `mk`→`mkp` 2 行と `alpha3_updregion.rs` の `n >= 30` 1 行)を `patch -p1`(offset・fuzz・reject なし)で当てた(第 2 部)。`alpha3b.diff`(第 1 案)は取り込まない。それが 3 件の `present2_uncover` を落とした(決定 0214 の S-p2-9 との衝突)ので、alpha1 に作り直させたのが `alpha3b2`(各窓の更新領域に足すのは、その窓自身の可視領域〔自分の WS_CLIPCHILDREN 旗で計算〕の増加分、という定式化。クラス名・窓種・位置の分岐 0、新しいオプション 0)。

PM が測った値(この機械、`lift/out-pmchk-a3/`):
- FLEngine の既定の生成 C(`fe-lift lift … --pdata`)の sha256 は `34a376db…78062a`(215,740,414 B)のまま(lifter 不変)。
- `ci/test-hybrid.sh`・`test-needs.sh`・`test-lifecycle.sh`・`test-real-deploy.py`・`test-wrangler-config.sh` すべて rc=0、shellcheck -x -S warning rc=0。app: typecheck rc=0、vitest 1,274 passed + 7 skipped、worker 通過、e2e(エンジン無し)172/172。
- `test.sh full`: **200 results / 2,347 passed / 0 failed / 1 ignored**(alpha1 の報告と同数)。
- a4(FRUITY 無効): **init_count=942/942、run_rank=895/895(started=895 completed=895 in_progress=0)、functions_distinct=5,310、function_entries=1,009,737、started_but_not_entered=0**(基準どおりで不変)。

判断:
1. **採る。** F9 の古い帯と Mixer 下端の帯は我々の欠落(b)だった。HEAD にも最初からあった既存の不具合で、alpha3r が作ったのではない(旧い外接矩形が F9 の全窓更新で偶然覆っていた)。これは依頼者が報告した「窓移動の残像」の候補の 1 つだが、依頼者の画面での確認は 0 件で、同じ現象かは分からない。
2. 契約の根拠は Wine 10.0 の実走(UNOFFICIAL。Windows の実測は 0 件)。Wine との差 3 点(sibling を HWND_BOTTOM へ、最上位窓への露出、CLIPCHILDREN ありの親への露出)は §10.2.2 に未解消として残す。過剰な無効化は契約上安全で、渡さない側が描き残しの危険なので、0214 の期待を保った。
3. 効果は native の測定(N1 の生成 C は cbf9d1e5 と HEAD 84e17206 の物)。Chrome の Worker CPU は向きが下がるだけで有意とは言えず、**P2 の改善として数えない**。Mixer ドラッグの native CPU −51%、ピッカーのフェードイン −62% などは、wasm と実ブラウザでの確認を取り込み後の本番 bundle で ci1 に取らせる。
4. 未確認(alpha1 の報告のまま): node の 84 行・55 行、Firefox、A1/A3b が全窓再描画で FillRect(DC でない hDC)に止まる原因、`unc_invalidate` を矩形ごとにすると起動時の絵が壊れる原因(外接矩形が隠していた)。
