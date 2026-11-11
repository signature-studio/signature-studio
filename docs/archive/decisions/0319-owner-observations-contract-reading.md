# 0319: 依頼者の観察(O7・O8・O9 と周辺)の『契約の読み』— ScrollWindowEx、文字、Debug タブのログの差、ログの出所

担当 alpha1。依頼は PM(2026-11-07 07:0x)。**先頭(何を実行したか)**: **実エンジンは実行していない。読みだけ**。使った物: Microsoft Learn(ScrollWindowEx・ScrollDC・GetTextExtentPoint32W・GetTextMetricsW の 4 ページ。取得は今回)、lift の runtime のソース(`lift/runtime/*.c`)、FLEngine_x64.dll の静的解析(pefile で import 表と文字列、objdump で数十命令)、FL64.exe の埋め込み manifest、app/src の該当箇所。依頼者の観察(`docs/feedback/2026-11-07-owner-observations.md`)は依頼者の機械の定性的な観察で、我々の計測ではない。数字は『読んだ値』で、挙動の観測ではない。推測は「読み」「仮説」と書く。方針は lift の忠実性(余計な処理を足さない、画面ごと・症状ごとの対症をしない)。各項に『(a) 余計な処理 / (b) 契約の不足』と『全体のどの仕組みを直すか』を書く。

## 0. 先に結論(各項の分類)

| 項 | 分類 | 全体のどの仕組みを直すか |
|---|---|---|
| ScrollWindowEx のあと『ホバーした要素だけ直る』(O9) | **(b)** ScrollWindowEx が、窓の『有効領域』の仕組み(決定 0214 の SetWindowPos 側にだけある)に載っていない。さらに ScrollWindow・ScrollDC は行そのものが無い(**§6: Wine 10 の測定で契約を確定し、差分を作成〔適用していない〕**) | 『窓の中身を動かす操作』(SetWindowPos・ScrollWindowEx・ScrollWindow・ScrollDC)を**一つの有効領域のモデル**にそろえる。ScrollWindowEx を契約どおり全体で実装(可視領域、更新領域の移動、SW_SCROLLCHILDREN)、ScrollWindow は ScrollWindowEx の別名、ScrollDC は契約どおり。Learn が黙る所は Wine を基準にする(alpha8 の先例) |
| 文字が横に潰れる(O9) | **未確定**。Win32 の文字 API(GetTextExtentPoint32 系・GetTextMetrics・ExtTextOut・DrawText)は **全部が名前つきの停止**で、誤った値を返す経路ではない。FL 自前の文字描画への入力(DPI、フォントの実体、SPI のフォント指標)の側を疑う | 入力の契約を揃える(DPI は下の O7/O8、SPI のフォント、フォントの実体) |
| Debug: 時刻 12:00:00 AM | **(b)** GetLocalTime・GetSystemTime が『現在の時刻』でなく仮想の起点(2021-01-01 00:00 UTC)を返す(実時刻を読まない設計) | 実時刻と時差を『run の値』としてページから渡す(画面の大きさの FLE_MONITOR_W/H と同じ形)。決定性は run の値の固定で保つ |
| Debug: SSE3/SSSE3/AVX no | **契約の範囲内**(仮想 CPU が lift の対応範囲を正直に申告している)。ただし native と同じ ISA の経路にはならない | CPUID の bit を『lift が実装した命令の表』から機械的に出す(一つの出所)。SSSE3→SSE3→SSE4.1→AVX/AVX2/FMA の順に lift を広げる |
| Debug: GUI scaling 1(96PPI) とぼやけ(O7) | **(b)** DPI は固定 96(`fle_gdc.h`)。GetProcessDpiAwareness は『非対応』を返すが、FL64.exe の manifest は `dpiAware True/PM`(per-monitor 対応)。ページは DPI を渡していない | DPI・画面の大きさを『デバイス画素と DPR』で run の値として渡し、GetProcessDpiAwareness は manifest から(per-monitor)、GetDpiForMonitor・GetDeviceCaps(LOGPIXELS)・画面の大きさを同じ出所から出す |
| Debug: Animation timer 8 ms(native 4 ms) | **(a)** ページ側の丸め表 `STANDARD_HZ = [60, 75, 90, 120]`(決定 0268)が測った 240 Hz を 120 Hz に切り詰める | 丸めの上限を外す(性能の理由で残すなら PM の決定として明記)。契約の鎖(rAF 測定 → FLE_DWM_HZ/FLE_DISPLAYSETTINGS_FREQUENCY → EnumDisplaySettingsW/DwmGetCompositionTimingInfo)は契約どおり |
| Debug: テンポラリが C:\Windows\Image-Line\… | **(b)** GetTempPathW 自体は Learn どおり(TMP→TEMP→USERPROFILE→Windows ディレクトリ)。**起動時の環境変数ブロックが空**なので Windows ディレクトリに落ちる | 初期の環境ブロックに実際の Windows が置く変数(TEMP・TMP・USERPROFILE・APPDATA・LOCALAPPDATA・windir・SystemRoot ほか)を、仮想プロファイルの定数から入れる |
| Debug: 言語が en-US(native は ja-JP) | **(b)+(c)** ロケールは en-US 固定(`fle_locale.c`、決定 0016)。ホストの言語を渡していない。他言語はデータが無い | ホストの言語を run の値で渡す。ただしロケール表(CT_CTYPE1、照合、GetLocaleInfoW)は en-US のみ。言語ファイルは Data(資材) |
| Debug: Loading tags finished が出ない | **未確定**。FL の関数(f_a3ae80)は『started』まで来て『finished』に着かない。原因(資材の欠落、スレッド、停止の握りつぶし)は静的には決まらない | 入口・出口を取る診断(TEST 専用)。資材(Data)が入れば確かめ直す |
| Debug の出所 | FL が**内部で文字列を組み立て、自前の一覧に入れて自前で描く**。OutputDebugStringW は『デバッガ無し=何もしない』(Learn どおり)で、エンジンの stderr/engineLog には届かない | 見るなら FL の関数にフックする TEST 専用の診断。製品に足さない |

## 1. ScrollWindowEx / ScrollDC / ScrollWindow(O9: Settings → General のスクロールで描画が崩れ、ホバーした要素だけ直る)

### 1.1 FLEngine が使う Scroll 系(import 表の実測と、行の有無)

| API | FLEngine が import | runtime の行 | 実装 |
|---|---|---|---|
| user32!ScrollWindowEx | ○ | ○(`ship2-imports.txt` 以降、`fle_wall_ia2w.c`、decision 0191 の壁 ia2-W23) | 部分的(下の差の表) |
| user32!InvalidateRgn | ○ | ○(壁 ia2-W31。FL の scroll ヘルパ f_825d20 が ScrollWindowEx の直後に呼ぶと注記) | 更新領域を『外接する 1 つの矩形』で足す |
| user32!ScrollWindow | ○ | **無し** | 呼ばれれば thunk で `fle_unsupported` |
| user32!ScrollDC | ○ | **無し** | 同上 |
| user32!GetScrollInfo / SetScrollInfo / ShowScrollBar | ○ | ○ | (本書では読まず) |
| user32!GetScrollPos / GetScrollRange / SetScrollPos / SetScrollRange / EnableScrollBar | ○ | **無し** | 同上 |
| comctl32!FlatSB_* 5 本 | ○ | ○ | (本書では読まず) |

### 1.2 契約(Learn)と runtime の実装の差

出典: Learn の ScrollWindowEx(winuser.h、今回取得)と ScrollDC(同)。runtime = `lift/runtime/fle_wall_ia2w.c` 25〜160 行、窓の有効領域まわり = `fle_win.c` の決定 0214 の節(2810 行〜)。

| # | 契約(Learn の要点) | runtime の実装 | 差 |
|---|---|---|---|
| S1 | 『窓のクライアント領域の内容』を dx, dy だけ動かす。prcScroll が NULL なら全体 | 一つの最上位窓の面(surface)の中で、scroll 矩形 ∩ クライアント ∩ 窓の clip 矩形のピクセルを `memmove`(決定 0126 の『最上位窓ごとに 1 面』の画素モデル) | 画素の移動は契約どおり。ただし『どの画素が動くか』の判断が矩形だけ(S2) |
| S2 | 窓が **WS_CLIPCHILDREN** を持つなら、返す領域(hrgnUpdate・prcUpdate)は『子窓の中で更新が要る領域を含む、更新が要る全体』。子窓・兄弟窓で隠れた部分は動かせない(実 Windows は可視領域で切る) | 『可視領域は持たない』(コメント: S-clip-rects)。面の中の矩形をそのまま動かすので、**重なった子窓・兄弟窓のピクセルも一緒に動く**。返す更新領域は『覆われなかった帯』だけで、**子窓の更新領域・隠れていた部分は含まない** | **差**。決定 0214 の SetWindowPos は `fle_win_dc_region`(CLIPCHILDREN|CLIPSIBLINGS の可視領域)と『有効な画素』の計算(`unc_*`)を持つのに、ScrollWindowEx はそれを使っていない |
| S3 | `SW_INVALIDATE`/`SW_ERASE` が無ければ、動かした元の領域は無効化されない。あれば無効化(`SW_ERASE` は WM_ERASEBKGND) | uncovered 帯の外接矩形を `InvalidateRect`(`flags & 6`)。`SW_ERASE` で erase フラグ | 帯の扱いは契約どおり。**更新領域は『窓ごとに外接矩形 1 つ』(S-updregion)**で、L 字などの正確な領域を持てない(過剰に描き直すだけで正しさには影響しないが、S4 と組み合わさる) |
| S4 | (Learn は黙る)実 Windows は、窓にすでにある**更新領域(まだ描かれていない無効な部分)も、動かす矩形の中では一緒に動かす**はず(動かす画素の中に『まだ古い無効な画素』があるため)。**→ §6 で確定: Wine 10 の測定と Learn ScrollWindow のページで『無効な部分も offset される』** | 既存の更新矩形に**触らない**。無効な部分の画素は動き、更新矩形は元の位置に残る | **仮説 H1(最有力の読み)**: 動かした画素が古い(まだ描かれていない)ものを含む場合、動いた先が『有効』と見なされ、元の位置だけが再描画される。結果は『縞・空白帯・行の重なり』で、**そこに触れる(ホバー等で FL がその要素を描き直す)まで直らない** = 依頼者の『ホバーした要素だけ直る』と整合。決定 0214 は同じ問題を SetWindowPos で既に直している(`unc_pending` を『コピーしてよい画素』から除く、`S-0214`)。ScrollWindowEx には同じ処理が無い |
| S5 | `SW_SCROLLCHILDREN`: 子窓を dx, dy だけ動かし WM_MOVE を送る | **`fle_stop`**(『子窓の移動と WM_MOVE はモデルに無い』) | FL がこのフラグを使えば**エンジンが止まる**(O1『激しく動かすと死ぬ』・O5 の候補。FL が使うかは未確認) |
| S6 | prcClip: 『クリップ矩形の中の画素だけが影響を受ける。外から中へ動いた分は描かれ、中から外へ動いた分は描かれない』。NULL の意味は ScrollWindowEx のページに明記が無い。ScrollDC のページは『NULL = クライアント領域全体』 | NULL なら scroll 矩形。コメントは別の版の文を引く(『同じ矩形』) | **契約の読みが揺れる**。FL が prcScroll ≠ prcClip で呼ぶ場合に効く。確認は Wine |
| S7 | 戻り値: SIMPLEREGION / COMPLEXREGION / NULLREGION、失敗は ERROR | 帯の数で 2/3/1。失敗は 0 | 契約どおり。ただし S2 のため『子窓を含む領域』の判定が欠ける |
| S8 | `hrgnUpdate` は『スクロールで無効になった領域』に更新される(矩形でない領域になりうる) | 外接矩形の矩形領域 | 近似(S-updregion) |
| S9 | `SW_SMOOTHSCROLL`: flags の上位語の時間をかけて滑らかに動かす | 即時に動かす(時間を無視) | 影響は小さい |
| S10 | 『入力・出力の座標はすべてクライアント座標』 | クライアント座標 | 契約どおり |
| S11 | **ScrollWindow**: ScrollWindowEx の古い形(矩形を動かし、動かして空いた所を無効化する) | **行が無い**(FLEngine は import) | 呼ばれれば停止 |
| S12 | **ScrollDC**: DC の画素を動かし、空いた領域を返す(無効化はしない) | **行が無い**(FLEngine は import) | 呼ばれれば停止 |

### 1.3 症状との対応と検証(読みと提案。実行していない)
- 仮説 H1(上の S4): Settings → General のスクロールで、スクロール中に直前の描画がまだ終わっていない無効領域が動く画素に混ざる。
- 仮説 H2(S2): General のページの部品が子窓で、窓が CLIPCHILDREN でないとき、子窓の画素が親のスクロールで一緒に動く(実 Windows では子窓の画素は子窓自身の可視領域で別に扱われ、動かし方も変わる)。
- 仮説 H3(S5): FL が `SW_SCROLLCHILDREN` を使うと止まる(エンジンが止まる症状 O1/O5 とは別に、描画が崩れる症状と結びつくのは H1/H2)。
- **検証案(Wine を基準、alpha8 の先例)**: 小さい Win32 のテストプログラム(`harness/alpha8/move_paint.c` の型)で、(1) 窓に無効領域がある状態で ScrollWindowEx を呼び、GetUpdateRgn/GetUpdateRect が動くか、(2) CLIPCHILDREN の子窓があるときに子窓の画素が動くか、(3) prcClip NULL の意味、を Wine 10 で実測し、runtime を同じテストで比べる。FL 側の確認は、TEST 専用の診断(ScrollWindowEx の引数・戻り・更新矩形を stderr に出す。再 build が要る)で Settings → General のスクロールが実際に何を渡しているかを取る。
- **直す仕組み(全体、(b))**: (i) ScrollWindowEx を、決定 0214 の窓の有効領域のモデル(可視領域 `fle_win_dc_region`、保留中の更新矩形 `unc_pending`)の上に載せる。動かす画素 = 可視領域 ∩ scroll 矩形 ∩ clip から『保留中の無効部分』を除いた物、保留中の無効矩形は動かして clip に切る(H1)、動かせなかった所(可視でない・無効な所)は更新領域に足す(S2)。(ii) `SW_SCROLLCHILDREN` を、決定 0214 の移動(`apply_pos`)と同じ道具で実装(子窓の移動と WM_MOVE)。(iii) ScrollWindow = ScrollWindowEx(SW_INVALIDATE|SW_ERASE 相当)の別名、ScrollDC は契約どおり(無効化しない、hrgnUpdate/lprcUpdate を返す)。(iv) FL のクラス名・窓の種類・位置による分岐は足さない。

## 2. 文字が横に潰れる(O9)— GetTextExtentPoint32 / GetTextMetrics / DrawText の契約と、ClearType・フォント代替

### 2.1 FLEngine の GDI 文字系の状態(import 表の実測)

| API | 行 | runtime の実装 |
|---|---|---|
| ExtTextOutW | ○ | **名前つきの停止**(`GDI_TEXT_STOP`、『グリフデータが要る: モデルに無い』、決定 0111) |
| GetTextMetricsW | ○ | **停止** |
| GetTextExtentPoint32W / GetTextExtentPointW | ○ | **停止** |
| DrawTextW / DrawTextExW(user32) | ○ | **停止** |
| TextOutW / SetTextAlign / GetTextAlign / CreateFontW / EnumFontsW | **無し** | thunk で `fle_unsupported` |
| CreateFontIndirectW / EnumFontFamiliesExW / SetTextColor | ○ | 実装(フォントのオブジェクトを作る。`EnumFontFamiliesExW` は**空の名前か『Segoe UI』のとき Segoe UI を 1 回だけコールバック、指標は 0。他の名前・文字集合は名前つきの停止**〔08:5x 訂正: 初版は『要求された名前をあると答えて』と書いたが不正確。§12〕) |

### 2.2 契約(Learn)
- GetTextExtentPoint32: 『選択中のフォントで文字列の幅と高さを論理単位で返す。クリッピングは考えない。カーニングのある装置では字ごとの幅の和と一致しないことがある。SetTextCharacterExtra と SetTextJustification を考慮する(ExtTextOut に lpDx がある場合を除く)』。
- GetTextMetrics: 『選択中のフォントの指標(高さ、平均幅など)を TEXTMETRIC に返す』。TrueType かは `tmPitchAndFamily & TMPF_TRUETYPE` で見る。DC の初期フォントは System(ビットマップ)。

### 2.3 判定(読み)
1. **Win32 の文字 API が『誤った幅』を返す経路は無い**(上の API はすべて停止か行なし)。FL がこれらを呼べばエンジンは**止まる**。『潰れる』が描画の崩れとして出ているなら、FL のその描画は Win32 の GDI の文字を使っていない = FL 自前の文字描画(FreeType の実モジュール `ft` と QuickFontCache)。
2. したがって『文字幅の計算と描画の幅の不一致』は、FL 自前の経路への**入力**の側にある可能性が高い(読み): (i) `GetDeviceCaps(LOGPIXELSX/Y)` が 96 固定(`fle_gdc.h`。native は 168 PPI)→ 文字の画素サイズの計算、(ii) `SystemParametersInfoW` のフォント指標(SPI_GETNONCLIENTMETRICS・ICONTITLELOGFONT・FONTSMOOTHING/TYPE、`fle_spi.c`)、(iii) **フォントの実体**: 仮想ボリュームの `C:\Windows\Fonts\` には、**代用品**として `arial.ttf` = Liberation Sans(Arial と幅の互換)、**`tahoma.ttf` = DejaVu Sans**(Tahoma より幅が広い)、`lucon.ttf` = DejaVu Sans Mono だけが置かれている(`vfs-seed-idle1.tsv`・`vfs-seed-vis1.tsv`、決定 0148 PROVISIONAL)。ほかの OS フォント(Segoe UI・Meiryo・Yu Gothic・MS UI Gothic など)の実体は無い。Artwork 側は Barlow・JosefinSans・OpenSans・PT Sans Narrow ほか。`EnumFontFamiliesExW` は何を要求しても『ある』と答える。**幅が native と違う面(Tahoma の代わりの DejaVu Sans など)で FL が配置すると、同じ箱に収まらず、FL が文字を箱に合わせて詰める(横に潰す)可能性がある**(読み。FL がそう詰めるかは未確認)。これは API の契約ではなく資材(C)の選択で、Tahoma・Segoe UI・日本語フォントは再配布できない。幅互換の自由なフォントを選ぶか、FL が要求する面と幅を実測する必要がある。
3. **ClearType**: Learn の GetTextExtentPoint32 は ClearType に触れない(幅は論理単位の整数)。ClearType/フォントスムージングは SPI の問いで、FL の QuickFontCache がグリフをどの形式で作るか(グレースケール/LCD)に効く。runtime は SPI_GETFONTSMOOTHINGTYPE に答える(`fle_spi.c`、値は未確認)。幅の不一致の原因としては二次。
4. **未確認**: FL が General のページで何の面・何の画素サイズを要求するか、フォントの実体を探して見つからない呼び(CreateFile の失敗)があるか。取る方法: 仮想ボリュームの『見つからない要求』を stderr に出す TEST 専用の診断(runtime の FindFirstFile/CreateFile の失敗の一覧)。再 build が要る。
- **直す仕組み(全体、(b)/(c))**: 入力の契約を揃える(DPI は O7 の項、SPI のフォント指標、フォントの実体は資材 C: 代用品 Tahoma→DejaVu Sans の幅が候補)。**文字 API の停止を『個別に』直さない**(Win32 の文字は FL 自前経路では使われない見込みで、使われる箇所があれば停止がその証拠になる)。GetTextExtentPoint32 など Win32 の文字 API を契約どおり実装する案は別途(フォントのグリフデータを持つ必要があり、FreeType の実モジュールと整合させる大きな仕事)。

## 3. Debug タブのログの違い(`tables/0319-debug-log-diff.tsv` に同じ表)

ログの文は `f_1265c30`(FLEngine、後述の 4 節)が組み立てる。項目ごとに『出所』『契約か余計な処理か』『直す仕組み』。

| 項目 | 製品(依頼者の観察) | native | 出所(runtime/ページ) | 分類 | 直す仕組み(全体) |
|---|---|---|---|---|---|
| Starting at | `12:00:00 AM` | `5:42:38` | FL: Delphi の `Now`(f_442c00)→ FormatDateTime(f_4441c0)。runtime: `fle_time.h`『時計は仮想。起点 2020-12-31T23:00Z + 仮想の ticks、ホストの時計は読まない。タイムゾーンは UTC』。仮想時計の開始 = 3,600,000 ms 後 → 最初の読みは 2021-01-01T00:00:00Z(`FLE_TIME_BOOT_FILETIME`) | (b) 契約の不足。Learn の GetSystemTime/GetLocalTime は『現在の時刻』。起点の固定は決定性のための設計の選択で、契約の範囲外 | 実時刻(FILETIME)とタイムゾーンの差を **run の値**としてページが渡す(FLE_MONITOR_W/H と同じ形、`Date.now()`・`getTimezoneOffset()`)。以後は仮想時計で進める。テスト・Export の再現は値の固定で保つ。時刻の 12 時間表記は FL のロケール書式(下の言語)に従う |
| 64Bit addressing mode / 3DNow! no | 同じ | 同じ(3DNow! no は最近の CPU と同じ) | CPUID 拡張葉は 0 を返す(`fle_cpuid.c`) | 契約の範囲内 | (必要なら拡張葉 0x80000001 の LAHF/LZCNT なども CPUID の表に載せる) |
| SSE yes / SSE2 yes | 同じ | 同じ | `fle_cpuid.c`: 葉 1 EDX = x86-64 の基準 (CMOV・CX8・FPU・FXSR・MMX・SSE・SSE2) | 契約の範囲内 | — |
| **SSE3 no / SSSE3 no / AVX no** | no | yes / yes / yes | `fle_cpuid.c`: ECX = 0。理由は『lift が対応しない機能を主張しない』(決定 0009/0032/0075/0118)。lift の対応(読み): SSSE3 は一部(`sse_ssse3.rs`: pshufb・palignr・pabsb あり、pmaddubsw・phaddw/d・psignb・pmulhrsw は無し)、SSE3 は float の水平演算ほか一部あり、SSE4.1(pmulld・ptest・blend・pextr/pinsr・pmovzx など)は**無し**、AVX は `VMOVUPS/VMOVAPS/VMOVNTPS/VZEROUPPER` だけ(`vex.rs`、決定 0032)、XGETBV は名前つきの停止 | **契約の範囲内**(仮想 CPU の能力を正直に答える)。『native と同じ経路』にしたいなら、lift を広げるのが筋(CPUID を偽って立てると、その命令で `fle_unsupported` に落ちる) | CPUID の bit を『lift が実装した命令の表』から機械的に出す一つの出所にする。広げる順: SSSE3 の残り(pmaddubsw・phaddw/d・phsub・psign・pmulhrsw など十前後。grep の概算)→ SSE3 完了 → SSE4.1 → AVX/AVX2/FMA(VEX の全面、XGETBV で XCR0 = 7 を返す)。必要な命令は『CPU 特徴で分岐する DSP の閉包』の census で数える |
| Touchscreen not available | 同じ | 同じ | — | — | — |
| **Main GUI scaling factor 1 (96PPI)** | 1 (96PPI) | 1.75 (168PPI) | `fle_gdc.h`: LOGPIXELSX/Y = 96 の定数。`fle_proc.c`: GetProcessDpiAwareness は **非対応(0)**。`fle_appfile.c`: GetDpiForMonitor = 96。`fle_screen.h`: `FLE_SCREEN_DPI` で変えられる(決定 0212)が、**ページは FLE_MONITOR_W/H しか渡さない**(`app/src/ui/screen-fit.ts` の `screenEnv`)。**FL64.exe の manifest は `dpiAware True/PM`(per-monitor 対応)**(実測) | (b)。(1) awareness は manifest に従うべき(runtime の注記『ホストが宣言しているかは分からない』は、manifest が手元にあるので根拠にならない)。(2) DPI と画面の大きさは同じ出所 | 画面の大きさを**デバイス画素**(部屋の CSS 画素 × DPR)で、DPI を `96 × DPR`(Windows の拡大率の段階に丸める)で run の値として渡し、GetProcessDpiAwareness は manifest(PM)から、GetDpiForMonitor・GetDeviceCaps(LOGPIXELS)・GetSystemMetrics・EnumDisplaySettingsW・モニタ矩形を同じ出所から出す。これで FL の GUI 倍率が native と同じ計算(dpi / 96)になり、ぼやけ(O7)も同時に直る見込み(ページの canvas の裏面が CSS 画素で、ブラウザが拡大している疑い。gpu1 の設計) |
| Temporary folder | `C:\Windows\Image-Line\4096-{…}\` | `…\Temp\Image-Line\90120-{…}` | `fle_vfs.c` GetTempPathW = Learn どおり(TMP→TEMP→USERPROFILE→Windows ディレクトリ)。**起動時の環境変数ブロックが空**(`fle_env.c`)なので Windows ディレクトリに落ちる。4096 は仮想の PID | (b)。API は契約どおりで、環境の初期値が実 Windows と違う | 初期の環境ブロックに TEMP・TMP(=`C:\Users\user\AppData\Local\Temp`)・USERPROFILE・APPDATA・LOCALAPPDATA・windir・SystemRoot・USERNAME・COMPUTERNAME ほかを、`FLE_VIRTUAL_USER_PROFILE_DIR` などの定数から入れる。ディレクトリは仮想ボリュームに実在させる |
| FL Studio language / Windows display language | English (US) / en-US | Japanese / ja-JP | `fle_locale.c`: ロケールは en-US 固定(決定 0016)。GetLocaleInfoW・照合・CT_CTYPE1 のデータは en-US の値のみ(決定 0042/0048/0087)。FL の言語は Data の言語ファイル(資材) | (b)+(c) | ホストの言語(`navigator.language`)を run の値で渡す。ただし他言語のロケールデータ(書式、照合、文字種)が要る。言語ファイルは Data 全体を使えるようにする方針(PM の決定 1)の中 |
| Loading tags started … **finished が出ない** | started のみ | started → finished | FLEngine: `f_a3ae80`(タグの読み込み、呼び元 f_a754b0・f_10aa600)の中に『started』(0xa3aeeb)と『finished』(0xa3b2e0、関数の終わり近く)の両方がある。『started』まで来て『finished』に着かない | **未確定**(E) | 入口・出口を取る TEST 専用の診断(f_a3ae80 と呼び先の戻り)。Data が無いと走査の結果が空になる可能性(C)と、スレッド・待ちで終わらない可能性を分ける。Data の遅延読み込み(PM の決定 1)後に確かめ直す |
| Browser caching took | 1 ms | 148 ms | FL: `f_b8d6c0`(Data のブラウザ索引の作成)。仮想時計の差。Data が無いので走査する物が無く 1 ms | (c) 資材。バグではない | Data の導入で自然に変わる |
| **Animation timer set to** | 8 ms, vsync enabled | 4 ms | FL: `f_135cdb0` が画面更新 Hz から計算(EnumDisplaySettingsW の dmDisplayFrequency〔`fle_monitor.c`〕/ DwmGetCompositionTimingInfo〔`fle_dwm.c`〕)。runtime はページが渡す `FLE_DWM_HZ`/`FLE_DISPLAYSETTINGS_FREQUENCY` を使う(既定 60)。ページ: `app/src/engine/hz.ts` が rAF を測り、**`STANDARD_HZ = [60, 75, 90, 120]` に丸める(上限 120。決定 0268)**。依頼者の画面は 240 Hz | **(a)** ページの丸め表が測定値を切り詰めている。runtime の鎖は契約どおり | 丸めの上限を外す、または上限を PM の決定として明記(エンジンが 240 Hz に追いつかない性能上の理由があるなら)。native の 4 ms と比べるなら 240 Hz を渡す |
| Mixer track map | 同じ形 | 同じ形(後半が長い) | FL: `f_12521b0` | — | (native の方が長いのは Data/プロジェクトの差の可能性。未確認) |

## 4. FL の Debug タブのログがどこから来るか(最終 07:2x。静的な読み)

**結論**: Debug タブのログは **FL の内部で閉じている**。Win32 の出力 API(OutputDebugString など)を経由せず、エンジンの stderr/engineLog には届かない。依頼者の問い『Worker が受けているか』への答えは『**受けていない**』。

根拠(FLEngine の静的な読み。実行はしていない):
1. **組み立て**: 関数 `f_1265c30` が startup ログの文を作る。時刻は Delphi の `Now`(f_442c00)→ `FormatDateTime`(f_4441c0)、CPU 特徴・GUI 倍率・テンポラリのパス・言語を `Format`(f_414c70)で連結する(『Starting at』『3DNow! support』『Touchscreen』『Main GUI scaling factor』『Temporary folder』『FL Studio language』の文字列がこの関数の中にある〔UTF-16〕)。
2. **追加**: 呼び元 `f_10ebce0`(0x10ebe4d で f_1265c30 を呼ぶ)は、その文字列をもう一度 `Format`(f_414c70)し、**`f_10eaea0(log オブジェクト, 文字列)`** に渡す。`f_10eaea0` は文字列を `TStringList` に入れて行に分け、**行ごとに log オブジェクトの仮想メソッド(vtable +0x28、引数 (行, -1, 0, …))を呼ぶ**(= 一覧への Add)。
3. **通知**: そのあと f_10ebce0 は log オブジェクトの窓ハンドル([obj+0x4c])へ **`PostMessageW`(thunk 0x425210、メッセージ 0x401 = WM_USER+1)**を送る(= 表示の更新の合図)。表示は FL 自前の描画(TVectorPanel 系)で、Win32 の ListBox/Edit ではない、と読む。
4. 他の行も同じ型: 『Loading tags started/finished』は `f_a3ae80`(呼び元 f_a754b0・f_10aa600)、『Browser caching took』は `f_b8d6c0`、『Animation timer set to … vsync』は `f_135cdb0`(呼び元 f_11a57c0・f_11d2070)、『Mixer track map』は `f_12521b0`。
5. **OutputDebugStringW**: FLEngine は import し(thunk 0x423a90)、呼び元は 11 関数(f_653dd0・f_6cc320・f_6cc9f0・f_706d30・f_707270・f_815e10・f_c3bbc0・f_c3bbe0・f_115b5f0・f_115be50・f_115c2b0)。**上のログの関数(f_1265c30・f_10ebce0・f_10eaea0・f_a3ae80・f_b8d6c0・f_135cdb0)は含まれない**(直接 call の範囲)。runtime の OutputDebugStringW は『デバッガが無いので何もしない』(Learn どおり、`fle_proc.c`)ので、仮に別の経路で呼ばれても stderr/engineLog には届かない。
- **見る方法**: FL の関数へのフック(TEST 専用の診断。lifted 関数の入口・出口は `-finstrument-functions` 相当が要り、wasm の製品 build では取れない)か、タブを開いた面の画像(PNG)。ログを engineLog に出す目的なら、f_10eaea0 の呼びを観測する TEST 専用のフックを足す案があるが、製品には足さない。
- 未確認: log オブジェクトの正確な型(vtable +0x28 の実体)。

## 5. 診断の方針(TEST 専用、製品に足さない)と mt3 の積み重ねとの関係

mt3 の `diag-g9-gui.diff` は、**プラグイン画像(0x6800000〜0x7400000)から呼ばれた**窓 API のログと、画像への間接呼びの数を取る(--wrap、fle_win.c に触れない)。alpha1 の診断は**重ならない**部分だけを足す(`impl/` の差分群を参照): (i)全ての呼び元の CreateWindowEx/DestroyWindow(class・title・親・rect・結果の lid・呼び出し元のアドレス)を fle_win.c の内部で、(ii)ScrollWindowEx の引数・戻り・更新矩形(`fle_wall_ia2w.c` の中)、(iii)仮想ボリュームの『見つからない要求』(`fle_vfs.c` の中)、(iv)待ち・スレッドのログ(f_a3ae80 が終わらない原因が待ちかを見る)。すべて env(`FLE_DIAG_*`)で有効化し、無ければ何も出さず、ゲストの見る値を変えない。

## 6. ScrollWindowEx の契約の実測(Wine 10)と、実装の差分(PM の許可 07:2x。適用・push はしない)

**先頭(何を実行したか)**: Wine 10.0(Ubuntu)の小さい Win32 テスト `tools/wine-scroll-contract.c`(mingw、専用 prefix `~/work/alpha22w/wine-scroll/prefix`、Xvfb、約 5 秒)を**実行した**(実エンジンではない。Wine の API の挙動の測定)。差分は clang の `-fsyntax-only` と `patch --dry-run` だけで確認し、**実行していない**(runtime の C の差分を走らせる検証は未了。下の 6.5)。

### 6.1 Wine 10 の測定(`tables/0319-wine10-scrollwindowex-probes.txt`)。窓は 300×300、色は 10 px の帯ごと
| テスト | 設定 | Wine 10 の結果 | 読み |
|---|---|---|---|
| T1/T4 更新領域 | 無効矩形 (0,200)-(300,230) を作り、ScrollWindowEx(dy=+40, prcScroll NULL, SW_INVALIDATE) | 戻り **2 SIMPLEREGION**、prcUpdate (0,0,300,40)(帯だけ)、hrgnUpdate の box **(0,0,300,270)**、窓の更新領域の box (0,0,300,270)・型 COMPLEX、点 (10,215)〔元の位置〕・(10,255)〔元 + 40〕・(10,20)〔帯〕**すべて領域の中** | **動かした無効矩形が更新領域に加わり、元の位置も残る**。Learn の ScrollWindow『invalid (unpainted) areas in the window are also offset』と整合。prcUpdate は帯だけ、hrgnUpdate は帯 + 動いた無効矩形の和 |
| T3 prcClip NULL | prcScroll = (0,50)-(300,150)、dy=+60、prcClip NULL、flags 0 | 画素 (10,180)〔scroll 矩形の外〕= **band(y=120)**(元は band(y=180))、画素 (10,130) = band(y=70) | **prcClip NULL = クライアント全体**(scroll 矩形ではない)。ScrollDC のページ『NULL = クライアント領域全体』と整合。以前の runtime(clip = scroll 矩形)は不一致 |
| T2a CLIPCHILDREN | 親 WS_CLIPCHILDREN、子 (100,100) 60×60 の赤、ScrollWindowEx(dy=+30, SW_INVALIDATE) | 画素 (130,170) = band(y=170)〔動いていない〕、(130,130) = 赤〔子は動かない〕、(20,130) = band(y=100)〔子の外の親の画素は動く〕。prcUpdate・hrgnUpdate の box (0,0,300,190)(帯 + 子に隠れていた source の行き先) | **子の画素は親の scroll で動かない。source が子に隠れていた行き先は無効**。報告される領域は『子の中の更新が要る所を含む全体』(Learn の WS_CLIPCHILDREN の注)。戻り値は 1(NULLREGION)で、領域が空でないのと食い違う(Wine の癖。真似ない) |
| T2b CLIPCHILDREN なし | 同じで子ありの親が WS_CLIPCHILDREN でない | 画素 (130,170) = **赤**(子の画素が親の bits と一緒に動く)、prcUpdate (0,0,300,30)(帯のみ) | 子の画素も動く |

### 6.2 差の確定(1.2 の S2・S4・S6)
- **S4(更新領域)**: 決定。無効矩形は動かした先にも加わる(Wine T1)。Learn の ScrollWindow にも『lpRect が NULL なら窓の無効領域も offset される。NULL でなければ offset されない』(今回 Learn で確認)。→ H1 は契約の読みから支持される。
- **S6(prcClip NULL)**: 決定。クライアント全体(Wine T3、ScrollDC のページ)。
- **S2(可視領域)**: 決定。WS_CLIPCHILDREN の窓は子の画素を動かさず、隠れた source の行き先を無効にする(Wine T2a)。

### 6.3 実装の差分(`impl/0319-scrollwindow-model.diff`、332 行、5 ファイル)
- `fle_win.c`(+≈170 行)に **`fle_win_scroll`**(窓の有効領域のモデルの上で、(1)窓のクリップ形式〔CLIPSIBLINGS、WS_CLIPCHILDREN なら CLIPCHILDREN も〕の可視領域 ∩ scroll 矩形 = 動く bits、(2)その行き先 = 移動後 ∩ clip 矩形〔NULL = クライアント全体〕∩ 可視領域、(3)覆われなかった領域 = (scroll 矩形 ∩ clip ∩ 可視) − 行き先、(4)bits の移動〔読んでから書く〕、(5)prcScroll が NULL のときだけ保留中の更新矩形を動かして無効化〔元の位置も残す〕、(6)SW_INVALIDATE/SW_ERASE で覆われなかった領域の外接矩形を無効化、(7)SW_SCROLLCHILDREN で子窓を `apply_pos`〔WM_WINDOWPOSCHANGED・WM_MOVE〕で動かし、動かない子にも WM_MOVE、CLIPCHILDREN の親では動かした子を無効化)。`fle_win.h` に結果の型と宣言。
- `fle_wall_ia2w.c`: `fle_imp_ScrollWindowEx` を新しいモデルの入口にし(戻り値は覆われなかった領域の矩形数で NULL/SIMPLE/COMPLEX、hrgnUpdate は矩形領域、prcUpdate は覆われなかった領域の外接矩形)、**以前の簡易版は `fle_win_scroll` が 0 を返す場合の fallback**として残す。新規 **`fle_imp_ScrollWindow`**(= ScrollWindowEx(SW_INVALIDATE | SW_ERASE | lpRect==NULL なら SW_SCROLLCHILDREN))。`fle_wall_ia2.h` に宣言。
- `harness/seeds/ship6-imports.txt` の**末尾**に `user32.dll ScrollWindow fle_imp_ScrollWindow` を 1 行(append only)。
- **ScrollDC は未実装**(FLEngine は import するが、DC の画素を動かす経路が fle_gdi.c のモデルに載る必要がある。本差分には含めない。契約は 1.2 の S12 のとおり: lprcClip NULL = クライアント全体、空いた領域を hrgnUpdate/lprcUpdate に返し、無効化はしない)。
- 確認: `clang -fsyntax-only` が 4 ファイルで通る(runtime の include の下)。`patch --dry-run -p1` が PM の木のコピー(HEAD 3969ad31)に通る。診断の差分(`diag-g9-fl.diff`)がこの差分の後にも当たる(offset 187 行つき)。fle_win.c に触れる mt3 の diff は無い(mt3 の積み重ねは fle_callback.c・fle_load.c・fle_modules.c・fle_dsound.c と ci/build 側)。ci1 の ①DefWindowProc(WM_ERASEBKGND)・②HTTRANSPARENT・③フックの差分(0323)と fle_win.c で重なる行は、重なったら ci1 から info を受けて合わせる。

### 6.4 モデルの限界(正直に)
更新領域は窓ごとに外接矩形 1 つ(S-updregion)なので、`hrgnUpdate` は帯 + 動いた無効矩形の外接矩形になる(Wine の COMPLEX の領域より広いが、広く無効にするだけで正しさには影響しない)。SW_SMOOTHSCROLL は即時。キャレットの扱いは無い。可視領域が `FLE_VISRGN_MAX` を超える窓は fallback(簡易版)に落ちる。子窓自身の更新領域は、SW_SCROLLCHILDREN の `apply_pos` が持つ座標と一緒に動く(窓の相対座標で持つため)と読むが、実機で確認していない。

### 6.5 検証(未了。許可があれば次)
alpha8 の `move_paint.c` の型の native テスト(runtime を直接呼ぶ)に T1〜T4 を足し、Wine の値(上の表)と並べる: 戻り値 2/1、prcUpdate・hrgn の box、(10,180) = band(y=120)、T2a の画素、更新領域の点。実エンジンでは、診断の差分(`diag-g9-fl.diff`)で Settings → General の ScrollWindowEx の引数を取り、同じ引数を native のテストに流す。

## 7. 時刻・環境ブロックの run の値の差分と、診断の差分(PM の許可 07:2x。適用しない)

**時刻**(`impl/0319-time-env-run-values.diff`、fle_time.h/c): `FLE_WALL_FILETIME`(エンジンが始まる時点の UTC の FILETIME、10 進)と `FLE_TZ_BIAS_MIN`(Windows の bias = UTC − local、日本は −540)を、画面の大きさ(FLE_MONITOR_W/H)や FLE_DWM_HZ と同じ『エンジンの環境に渡す run の値』として読む(`getenv`。製品でも読む配線値)。仮想時計は FLE_CLOCK_START_MS から進むので、起点 = 渡された時刻 − その分。**未設定・不正なら従来の固定起点と UTC のまま**なので、テスト・Export・決定性は変わらない(`FLE_TZ_BIAS_MINUTES` は関数呼び出しのマクロになり、GetLocalTime・GetTimeZoneInformation・SystemTimeToTzSpecificLocalTime・`fle_api2_str.c` が同じ値を使う)。ページ側(gpu1): `Date.now()` を FILETIME(ms × 10000 + 116444736000000000)に、`new Date().getTimezoneOffset()`(分。JavaScript も UTC − local で Windows の bias と同じ符号、日本は −540)をそのまま渡す。夏時間・時間帯名(「Tokyo Standard Time」など)は未対応。仮想ボリュームのファイル時刻は別の仮想時計(`FLE_VFS_CLOCK_START`)のまま。
**環境ブロック**(同じ diff、fle_env.c): `FLE_ENV_DEFAULTS=1` のとき、最初のアクセスで TEMP・TMP・USERPROFILE・APPDATA・LOCALAPPDATA・HOMEDRIVE・HOMEPATH・windir・SystemRoot・SystemDrive・ComSpec・Path・PATHEXT・OS・PROCESSOR_ARCHITECTURE・ProgramData・ProgramFiles・CommonProgramFiles・ALLUSERSPROFILE・COMPUTERNAME・USERDOMAIN・USERNAME を `fle_env.h` の仮想プロファイルの定数から入れ、TEMP・Roaming・ProgramData のディレクトリを仮想ボリュームに作る(`fle_vfs_make_dirs`)。未設定(テスト)なら空のまま(`fe-lift/tests/env.rs` などの期待を壊さない)。製品は manifest の engine.env に `FLE_ENV_DEFAULTS=1` を足す。→ `GetTempPathW` は Learn どおり `C:\Users\user\AppData\Local\Temp\` を返し、FL の temp は `…\Temp\Image-Line\<pid>-{GUID}` になる。
**診断**(`impl/diag-g9-fl.diff`、TEST 専用): `FLE_DIAG_WIN=1`(全ての CreateWindowEx/DestroyWindow の class・title・rect・style・親・呼び出し元、**DefWindowProc の WM_ERASEBKGND・WM_PAINT と祖先の class の鎖**〔ci1 の 0322 の項目①〕)、`FLE_DIAG_SCROLL=1`(ScrollWindowEx の引数・戻り・覆われた領域)、`FLE_DIAG_VFS_MISS=1`(仮想ボリュームの『見つからない要求』)、`FLE_FRUITY_THREAD_TRACE=1` に乗る待ちのログ(api・種類・thread・timeout・呼び出し元)。すべて観測だけで、ゲストの見る値を変えない。製品の wasm では `FLE_TEST_GETENV` が null 定数なので死んだコード。f_a3ae80 の入口・出口は、直接 call の関数に入口を差し込む手段(`-finstrument-functions` 相当)が wasm に無いため、**待ちとスレッドのログで代替**(『終わらない待ち』なら parked のまま出る)。

## 8. ページ側の差分(engine.env に実時刻・時差・既定の環境ブロックを渡す)と DestroyCursor の stop の診断(PM の許可 07:5x。適用しない)

**先頭(何を実行したか)**: vitest(app のコピー `~/work/alpha22w/app-copy` で、**単体テストの全部**〔69 ファイル、1322 件 passed・7 skipped、新規 9 件を含む〕)と `tsc --noEmit`(tsconfig.json・tsconfig.test.json とも rc 0)を実行した。ブラウザ・エンジン・Wine は実行していない。

**差分** `impl/0319-page-wall-clock-env.diff`(192 行、3 ファイル): 新規 `app/src/engine/wall-clock.ts`(純粋な関数: `filetimeOfUnixMs` は Unix ms を 10 進の FILETIME に〔2^53 を超えるので BigInt〕、`biasOfOffsetMin` は `getTimezoneOffset()`〔UTC − local。Windows の bias と同じ符号〕を整数で、`parseWallParam` は URL の `fle_wall`、`applyWallClock` は engine.env に `FLE_ENV_DEFAULTS=1`・`FLE_WALL_FILETIME`・`FLE_TZ_BIAS_MIN` を入れる)、`app/src/main.ts`(`applyWallClockParam(manifest)` を `applyScreenParam(manifest)` の直後、`client.init` の前に 1 行呼ぶ。import 1 行 + 関数 6 行)、新規 `app/test/unit/wall-clock.test.ts`(9 件)。HEAD の main.ts に `patch --dry-run` が通る(gpu1 が main.ts の同じ付近を変えると衝突しうる)。
**決定性の条件(PM)**: (1)**manifest がすでに持つ値は上書きしない**(固定した run、テストの run がそのまま残る)。(2) URL の `fle_wall=off` で時計の値を入れない(エンジンの固定起点のまま。既定の環境ブロックだけ入る)。`fle_wall=<unix ms>` で起点を固定。(3) Export の harness はこのページを通らないので変わらない。(4) **関門(ci1)**: 製品のページを (a) 既定〔実時刻〕と (b) `?fle_wall=off` で Export し、sha が基準(Balance なしの製品 = 381cdb5a…f979、G9-a の bundle なら 94c93666…4090)と一致するか。一致すれば『ページが実時刻を渡す』と『Export の sha が変わらない』は両立。**一致しない場合の疑い**: 時刻を使う FL の処理(乱数の種、ファイル名に時刻を使う保存、WAV の時刻つき chunk)。読みでは Export の WAV の内容に時刻は入らないと見ているが、未確認。runtime 側(`0319-time-env-run-values.diff`)が入っていない bundle では、渡した値は無視される(読む場所が無い)。
**時刻の渡し方の限界**: 夏時間と Windows の時間帯名は無い(起動時の bias のまま)。ブラウザの `Date.now()` が秒/ms に丸められる設定(privacy)は丸めたまま渡る。

**DestroyCursor の stop(Filter の V3、gpu1 の run)の診断の差分** `impl/0319-fle-stop-context.diff`(18 行、fle_env.c): `fle_stop` の 1 行目はそのままに、2 行目として `fle: stop context: guest return … rsp … rcx … rdx … r8 … r9 …` を出す(rsp が guest の番地に見えるときだけ戻り先を読む)。stop は元から致命なので意味は変わらない。原因は mt3 が読みで特定(fle_cursor.c の `off / STRIDE > 8` が RM_MAX 16 の Filter 差分で残り、Filter = 表の位置 8 = k = 9 の cursor が共有と認められない)。私の読みも同じ(符号化は `fle_loadimage.h`: BASE + STRIDE × (表の位置 + 1) + 4 × slot、表は [dsp, blend2d, qfc, freetype, iltools, oggio, elastique, balance, filter])。**node で hIcon を読む確認は、g9f のテスト bundle が製品の形〔FILESYSTEM=0、getenv なし〕で G3 を読まないため、Filter の木の TEST build が要り、していない**。値の確認は、上の診断を載せた次の build の実 Chrome の stop で出る。

## 9. 追補(08:3x): scroll model の native 検証、ScrollDC の差分、diag の置き場所の是正

**先頭(何を実行したか)**: native の harness(`tools/scroll0319/scroll_0319.c` + `build-native.sh`。1 つのソースを `-DC0323_WINE` で mingw にも、native では本物の runtime C〔fle_win.c・fle_gdi.c ほか〕にリンクして実行)を実行した。実エンジン(FL)・wasm・Chrome は実行していない。Wine 10 側の再実行は opt1 の窓(08:00〜08:35)の後(PM の許可待ち)。表はファイルに保存(`tables/0319-scroll-harness-native-*.out`)。

### 9.1 検証(native。HEAD ed125d09 の runtime 対 HEAD + `0319-scrollwindow-model.diff`。Wine 10 の値は 07:55 の実測 `tools/scroll0319/wine.out`)
| 行 | Wine 10(07:55) | HEAD の runtime | HEAD + scroll-model |
|---|---|---|---|
| U1(保留の無効矩形 (0,200,300,230)、dy +40、SW_INVALIDATE) | 戻り 2、prcUpdate (0,0,300,40)、hrgn / 更新領域の box (0,0,300,270) | 更新領域 (0,0,300,230)、hrgn (0,0,300,40) | **Wine と一字一句同じ** |
| U2(prcScroll = client 全体) | U1 と同じ | HEAD は U1 と同じ誤り | **同じ** |
| U3(保留なし、flags 0) | 戻り 1(NULLREGION)、prcUpdate (0,0,300,40)、更新領域は空 | 戻り 2 | **同じ** |
| C1(prcScroll (0,50,300,150)、dy +60、prcClip NULL) | (10,180) = band(y=120)、prcUpdate (0,50,300,110) | (10,180) = band(y=180) = clip を scroll 矩形にしていた | **同じ** |
| K2(CLIPCHILDREN なし、子 (100,100,60×60)、dy +30) | 親 DC (130,170) = RED(子の絵が動く) | RED | **RED(同じ)** |
| K1(WS_CLIPCHILDREN の親) | (130,170) = band(y=170)(子の絵は動かない)、prcUpdate (0,0,300,190) | (130,170) = **RED**(子の絵を動かしていた)、prcUpdate (0,0,300,30) | **band(y=170)、prcUpdate (0,0,300,190)(同じ)** |
- harness の誤りを 2 点直した。(a) runtime は WS_VISIBLE の窓を作った直後に update 領域を持たず、UpdateWindow が WM_PAINT を送らない(Wine は送る)ので、子の最初の絵は両側とも『InvalidateRect してから UpdateWindow』に揃えた。(b) WS_CLIPCHILDREN の親の DC で子の領域を GetPixel すると、runtime は **CLR_INVALID**(0xFFFFFFFF。Learn GetPixel「クリップ領域の外は CLR_INVALID」)を返す(契約どおり)。**Wine 10 は絵を返す**(Wine の逸脱。K1 の Wine の値は 07:55 の古い harness のもので、`(130,130) = RED(child)` は親の DC から読んだ値)。K の子の画素は子の DC でも読むようにした(`child DC (30,30)`)。Wine の再実行で確定する。
- 戻り値: Wine は K1 で NULLREGION(1)を返す癖がある(hrgnUpdate の box は (0,0,300,190) の非空)。runtime(model)は uncovered が複数の矩形なので COMPLEXREGION(3)を返す。Learn の『戻り値は無効化した領域の型』どおりで、Wine の癖は真似ない。

### 9.2 ScrollDC(`impl/0319-scrolldc-model.diff`、145 行、fle_gdi.c・fle_gdi.h・ship6-imports.txt。適用しない。PM 07:5x の (b))
- 契約(Learn ScrollDC、今回取得): 『dx, dy はデバイス単位』『lprcScroll の bits のうち lprcClip との交わりだけが影響を受ける。NULL = client 全体』『lprcClip: スクロール後に残る bits のうちこの矩形の中だけが描かれる。NULL = client 全体』『hrgnUpdate: 空いた領域(矩形とは限らない)』『lprcUpdate: それを囲む矩形、client 座標(マッピングモードに依らない)』『lprcUpdate が NULL なら矩形は計算しない。両方 NULL なら領域は計算しない』『戻り値は成功で非 0』。**無効化はしない**。
- model(ScrollWindowEx の `fle_win_scroll` と同じ式を DC の clip の上で): 存在する bits = DC の clip(window DC の可視領域 + SelectClipRgn);`src` = scroll 矩形 ∩ bits;`dst` = 移した `src` ∩ clip 矩形 ∩ bits;uncovered = (`src` ∩ clip 矩形) − `dst`。bits は読んでから書く(重なり対策)。window DC は client 座標(BeginPaint の DC は更新領域に制限)。memory DC は bitmap 全体が『client』。見えない窓は bits が無く uncovered も空。1 バイト未満/パレット形式の surface は名前つきの停止。hrgnUpdate は 64 矩形まで(超えたら停止)、生きていない領域 handle は名前つきの停止。
- 実行(native harness の D 面。`tables/0319-scroll-harness-native-stack-with-scrolldc.out`、runtime = stack + この差分): D1(prcScroll (0,50,300,150)、dy +60) → 戻り 1、lprcUpdate (0,50,300,110)、(10,180) = band(y=120)(ScrollWindowEx の C1 と同じ式);D2(prcClip (0,100,300,200)、dy +40) → clip 全体が補われ uncovered は空;D3(dx −20、dy +30) → L 字の uncovered の外接矩形 (0,0,300,300);D4(DC の clip 領域 (0,0,300,100)) → clip の外は触らない(uncovered (0,0,300,40));D5 → 更新領域は空のまま(無効化しない);D6(memory DC 100×100) → lprcUpdate (0,0,100,20)。`emcc -fsyntax-only -Wall`(fle_gdi.c): 警告 0。**Wine 10 の値は未取得**(08:35 以降。取れたら表にする)。
- 行(ship6-imports.txt の最後に 1 行): `user32.dll ScrollDC fle_imp_ScrollDC`。

### 9.3 診断の是正(`impl/diag-g9-fl-scroll.diff`)
`diag-g9-fl.diff` の FLE_DIAG_SCROLL は旧 path(`scroll_window_ex_simple`)にだけあり、scroll-model を積むと `fle_imp_ScrollWindowEx` が `fle_win_scroll` を通るので何も出ない(native で実測: 0 行)。診断を新しい経路の 2 関数に足す差分を出した。(b)でも(a)でもなく、置き場所の誤り(私の積み順の誤り)。

## 10. mt3 の質問(ii)(PM 08:3x): 『Multithreaded mixer / generator processing』の値はどこに保存され、既定は何か(読みだけ。08:4x)

**先頭(何を実行したか)**: 実エンジンは実行していない。FLEngine_x64.dll(opt1 の install、FL 2026 = 26.1)のバイト列の読み(Python の正規表現と DFM の読み)、opt1 の Wine の prefix の user.reg の読み、runtime の `fle_sys.c` / `fle_sysinfo.c` / `fle_reg.*` の読み、既存の docs(0173・registry-unit153)。以下、『読み』は未確認。

| 項目 | 読み | 根拠 |
|---|---|---|
| コントロール | Settings 窓(`TMIDIForm`、form 名 `MIDIForm`、Caption 'Settings')の `audiopage` の `cpupanel` 上の `TVectorCheckBox` 2 つ: `MultithreadGenBox`(Caption 'Multithreaded generator processing')と `MultithreadFXBox`(Caption 'Multithreaded mixer processing')。隣に `SmartDisBox`。Hint 'May increase CPU performance'。ハンドラ `MultithreadGenBoxClick` / `MultithreadFXBoxClick` / `SmartDisBoxClick` / `SafeOverloadBoxClick` | FLEngine の DFM(TPF0 ブロック。先頭 `TMIDIForm`/`MIDIForm` はファイル位置 0x141cc08、チェックボックスは 0x1422bd2 以降)、コンポーネント表(0xf50b96)、ハンドラ表(0xf52279) |
| **form の既定(DFM)** | **両方 `Checked = True`**(DFM に `Checked` の名前が書かれ、直後の値の型バイトが 0x09 = vaTrue。`MultithreadGenBoxClick … Checked \t`、`MultithreadFXBoxClick … Checked \t`)。= 値が保存されていない/読めない時の form の初期状態は**どちらもオン** | DFM のバイト列(`b'MultithreadGenBoxClick\x07Checked\t\x00\x00\x0fTVectorCheckBox\x10MultithreadFXBox…'`、`b'MultithreadFXBoxClick\x07Checked\t\x00\x00\x0fTVectorCheckBox\x0bSmartDisBox…'`) |
| 保存先 | FL は form ごとの状態を `HKCU\Software\Image-Line\FL Studio 26\General\<form 名>` の下に、**コントロール名を値名、'0'/'1' の文字列を値**として保存する(opt1 の Wine で FL 実機が書いた `General\WAVRenderForm` に `"PlugHQBox"="1"` など 30 値。実測)。Settings 窓の 2 つが同じ仕組み(`General\MIDIForm`、値名 `MultithreadGenBox` / `MultithreadFXBox`)かは**未確認**: Wine の prefix(prefix・prefix-e2・prefix-e3)の user.reg / system.reg に `MIDIForm` も `Multithread` も無い(Settings 窓を開いて閉じた run が無い)。FLEngine は `Software\Image-Line\FL Studio 26\General\`(末尾 \)を文字列として持つ(form 名を足して使う形)。**確かめる最短の方法**: opt1 の Wine で FL を起動 → Settings(F10)を開く → 何も変えずに閉じる → user.reg の `General\MIDIForm` を見る(1 run、約 2 分) | Wine の user.reg(実測)、FLEngine の UTF-16 文字列、docs 0173 §(c) の『設定の保存先は FL のレジストリ `Software\Image-Line\FL Studio 26\General`』 |
| 仮想 registry / 設定 | runtime の registry は**メモリ内で、起動時は 5 つの root が空**(`fle_reg.h`: 『host の registry は読まない。ゲストが作った物だけが見える』)。seed(`lift/harness/seeds/reg-seed-*.tsv`)に該当の値は無い(docs 0173 の grep 0 件)。FL が `General\…` を開けば ERROR_FILE_NOT_FOUND(2)→ **値が無い = form の既定(両方オン)で動く**。値を書くと同じ run の中では読めるが、run をまたぐ永続化は無い(page 側の保存の仕組みも未確認) | `fle_reg.h`、docs 0173 |
| 実際の並列度を決める別の入力 | **`GetSystemInfo` は `dwNumberOfProcessors` = 1、`dwActiveProcessorMask` = 1** を返す(`fle_sys.c:548` の注記: 『run が単一のゲスト thread だから 1 つ。ホストの core 数を使うと機械で出力が変わる〔選択〕』)。`GetLogicalProcessorInformation` も `FLE_SYSINFO_PROC_MASK`(単一)。FLEngine の Debug ログに『Threads: %d』『Mixer threading map rebuilt: 』がある(文字列。値の出所は未確認)。**FL のミキサー thread 数はプロセッサ数から決まるのが自然**(読み)なので、チェックが両方オンでも、プロセッサ数が 1 のままなら実 thread 版(B1/B2)に切り替えても FL が使う thread は 1 のまま(読み。未確認) | `fle_sys.c`、`fle_sysinfo.c`、FLEngine の文字列 |

**(b)(契約の不足)になる点**: 実 thread を本番に入れるなら、`GetSystemInfo` の `dwNumberOfProcessors` / `dwActiveProcessorMask` と `GetLogicalProcessorInformation` / `GetActiveProcessorCount` / `GetLogicalProcessorInformationEx` の返す数を、**実際に動く Worker 数に揃える**(run の値としてページが渡す。時刻・環境と同じ形:`FLE_NUM_CPU` のような run の値。ホストの core 数をそのまま使うと Export の出力が機械で変わるので、Export の harness は固定値、本番は渡す値)。Settings の 2 つのチェックは、値が無ければ既定でオン(読み)なので、追加の処理は要らない。**未確認の点を 1 run で潰せる**: (1) Settings 窓の保存先(上)、(2) FL のミキサー thread 数が `dwNumberOfProcessors` に従うか(Debug ログの『Threads』を、プロセッサ数を 2/4 にした TEST build で比べる)。

## 11. プロセッサ数の一つの出所 `FLE_NUM_CPU`(PM 08:3x。`impl/0319-num-cpu.diff`、149 行。適用しない。mt3 の `tree-rt1` に当てる形)

**先頭(何を実行したか)**: 実エンジン・Chrome・Wine は実行していない。native の harness(`tools/scroll0319/cpu_0319.c` + `build-cpu.sh`)を、mt3 の `tree-rt1`(RT の積み)+ この差分の runtime C にリンクして実行(非 FLE_GIL の native。`tables/0319-num-cpu-native-harness.out`)。`emcc -fsyntax-only -Wall`(fle_sys.c・fle_sysinfo.c・fle_cpuid.c、FLE_GIL あり/なしの両方): 警告 0。FLE_GIL の経路(EM_JS の hardwareConcurrency)は native では実行できない。

**問題(b)**: runtime の `GetSystemInfo` は `dwNumberOfProcessors` = 1 を返す(元の `fle_sys.c`)。mt3 の `tree-rt1` は `fle_sys_processors()` を足し、FLE_GIL では host の `hardwareConcurrency`(S3a の `FLE_PROCESSORS` が優先)、それ以外は 1 を返す。これで GetSystemInfo・GetLogicalProcessorInformation・affinity の mask は一つの関数に揃っていた。足りない物: (1) **run の値 `FLE_NUM_CPU`**(ページが実際に並列に走れる数を渡せる出所。時刻・環境・画面と同じ形)、(2) GetActiveProcessorCount 系・GetCurrentProcessorNumber(行も関数も無い)、(3) **CPUID の論理プロセッサ数**(leaf 1 の EDX bit 28 = HTT が 0、EBX = 0 のまま〔n > 1 でも『論理プロセッサは 1』と答える〕)。

**差分の内容**(`tree-rt1` に対して。dry-run と実適用で『適用した木 = 編集した木』を確認):
- `fle_sys.c`: `fle_num_cpu_env()`(`FLE_NUM_CPU` を 10 進の数字だけ、1..`FLE_SYS_MAX_PROCESSORS`〔32。mt3 の値〕で読む。範囲外・数字以外は無視 = 0)。`fle_sys_processors()` は FLE_GIL では **`FLE_NUM_CPU` > `FLE_PROCESSORS`(S3a) > hardwareConcurrency** の順、それ以外では `FLE_NUM_CPU` または 1。一度だけ読む。
- `fle_sysinfo.c/.h`: `GetActiveProcessorCount(group)` / `GetMaximumProcessorCount(group)`(group = 0 か 0xFFFF → n、他の group は『ページが誤りコードを書かない』ので名前つき停止)、`GetActiveProcessorGroupCount` / `GetMaximumProcessorGroupCount`(= 1、一つの group)、`GetCurrentProcessorNumber`(guest thread の番号 mod n。n = 1 なら 0)。いずれも `fle_sys_processors()` から。
- `fle_cpuid.c`: n > 1 のとき leaf 1 の EDX bit 28(HTT)を立て、EBX[23:16] = n 以上の最小の 2 の冪(SDM: 『package 内の論理プロセッサの最大 ID 数』)、EBX[31:24] = 0。n = 1 なら従来と同じ(EBX = 0、EDX bit 28 = 0)。leaf 4 / 0xB は最大 basic leaf = 1 のため leaf 1 のデータが返る従来の規則のまま(論理プロセッサ数は leaf 1 で答える)。
- `ship6-imports.txt`(最後に 6 行): 上の 5 つの行。**FLEngine と同梱 5 DLL の census では、これらを import するのは FLEX の helper DLL の `SetThreadAffinityMask` だけ**(FLEX は参照構成で読み込まれない)。上の 5 つは『互いに矛盾しない』ための契約の行で、今の closure に呼び手は無い。`SetThreadAffinityMask` / `SetThreadIdealProcessor` / `GetLogicalProcessorInformationEx` は呼び手が無く、thread handle の表と record の形を要するので足していない。
- GLPI と affinity は mt3 の版のまま(core record を n 個、node・cache・package を 1 つずつ。process affinity は遅延初期化)。

**実測(native harness、`tables/0319-num-cpu-native-harness.out`)**: `FLE_NUM_CPU` 未設定は tree-rt1 と **全行同じ**(新しい API の行だけ増える)。n = 4: GetSystemInfo 4 / mask 0xf、GLPI 10 records(core × 4、node、L1 × 2、L2、L3、package)で core の mask の和 0xf、GetProcessAffinityMask 0xf / 0xf、`SetProcessAffinityMask(0x3)` は成功して process mask 0x3(system 0xf)、GetActiveProcessorCount 4、CPUID leaf 1 EBX 0x00040000・HTT 1。n = 32 も同様。n = 33 / 0 / 'abc' は無視されて 1。

**Worker 数との関係(mt3 の 0312 との突き合わせ。要判断)**: `fle_thread.c` の `ensure_init` は Worker 数を **`fle_sys_processors()` から `n < 3 ? 2 : n > 9 ? 8 : n − 1`** で決める(`FLE_WORKERS` は S3a のみ)。つまり guest が見るプロセッサ数 P に対し Worker 数 W = clamp(P − 1, 2, 8)。ページが `FLE_NUM_CPU = W`(実際に動く Worker 数)を渡すと、`fle_thread.c` は W' = clamp(W − 1, 2, 8) < W にする(矛盾)。選択肢: (X) ページは P(guest に見せたい数)を渡し、W は mt3 の規則のまま(W = P − 1。guest は W + 1 個と答えられる = 少し多め);(Y) `FLE_NUM_CPU` が指定された時は `nworkers_cfg = clamp(FLE_NUM_CPU, 2, 8)` にする(guest の見る数 = Worker 数。PM の方針に厳密)。(Y) は fle_thread.c の 1 行だが mt3 の領域なので、この差分には入れていない。**Export の決定性**: 未設定 = 1 なら不変。n > 1 では CPUID の leaf 1 が変わる(HTT)ので、Export の sha が n で変わりうる(ci1 の G4 で測る)。

## 12. O16(日本語が『豆腐』)— フォント API の棚卸しと、日本語のグリフを持つフォントを仮想 Fonts に置いた時に FL が選ぶ契約(PM 08:3x。静的な読み。09:0x)

**先頭(何を実行したか)**: 実エンジン(FL)・Chrome・Wine の FL は実行していない。**静的な読みだけ**: (1) インストール済み FL 2026(26.1)の `FLEngine_x64.dll`・`speaker_x64.dll`・`FLReWire_x64.dll`・`Shared\QuickFontCache_x64.dll`・`freetype_x64.dll`・`Blend2D.dll`・`ILTools_x64.dll` の import 表(pefile、表 = `tables/0319-o16-font-imports-census-FLEngine-speaker-FLReWire.txt`)、(2) `QuickFontCache_x64.dll` の逆アセンブル(capstone。フォールバック一覧を作る関数 RVA 0x17f000〜、`.pdata` の関数境界と IAT の thunk の呼び元)、(3) runtime の C(`fle_gdi.c` の `EnumFontFamiliesExW`)と seed(`lift/harness/seeds/os-fonts/`)、(4) opt1 の Wine の prefix の報告(`C:\Windows\Fonts` は空、`Fonts` キーに host の 531 項目)。依頼者の観察は依頼者の機械のもの。

### 12.1 結論(先に)
- **FL の画面の文字は GDI でなく QuickFontCache(QFC)= AGG + FreeType で描かれる。** QFC は **`SHGetFolderPathW(CSIDL_FONTS = 0x14)` の下に、決まった 10 個のファイル名があるかを `CreateFileW` で調べ、あった物を『フォールバック一覧』に足す**(逆アセンブルで確認。順序は `msjh.ttf`・`msjh.ttc`・`msyh.ttc`・`malgun.ttf`・`batang.ttc`・`msgothic.ttc`・`arial.ttf`・`tahoma.ttf`・`seguiemj.ttf`・`segoeui.ttf`)。日本語向けの名前は **`msgothic.ttc`(MS ゴシック)**で、`msjh`(繁体字)・`msyh`(簡体字)・`malgun`/`batang`(韓国語)も CJK 字形を持つ。
- 仮想 `C:\Windows\Fonts` には `arial.ttf`(Liberation Sans)・`tahoma.ttf`(DejaVu Sans)・`lucon.ttf`(DejaVu Sans Mono)の代用 3 つしかない(決定 0148・0236)ので、一覧は [arial, tahoma] だけで **日本語を持つ face が一つも無い → 字形が無い → 豆腐**。= **(c) 資材の不足**。契約(API の応答)の不足ではない。
- **FONTSIGNATURE(NEWTEXTMETRICEX)・GetFontUnicodeRanges・FontLink の SystemLink は FL は使わない**(下の 12.2: どの DLL にも import / 文字列が無い)。フォントの被覆の判断は QFC が FreeType で自分でやる(`GetFontSignature`・`CalculateGlyphIdx`、読み)。契約として決める物は『ファイル名・場所・FreeType が開ける形式・cmap に字がある』であって、GDI のフォント列挙の応答ではない。

### 12.2 API の棚卸し(表 `tables/0319-o16-font-imports-census-…txt`、runtime の現状との差)
| 用途 | FL の側(import / 文字列。実測) | runtime の現状 | 差 |
|---|---|---|---|
| **フォントの場所** | QFC: `SHGetFolderPathW`(CSIDL_FONTS)、`FindFirstFileW`、`CreateFileW`/`CreateFileMappingW`/`MapViewOfFile`(フォントファイルを丸ごと読んで FreeType の `FT_New_Memory_Face` に渡す形、読み)。FLEngine・ILTools: `SHGetFolderPathW` | `C:\Windows\Fonts` = 代用 3 ファイル(決定 0148・0236、`os-fonts/`)。ファイル名が無ければ `FileExists` は偽 | **(c)** 日本語の face のファイルが無い |
| **フォントの一覧(名前→ファイル)** | ILTools: `RegEnumValueW`(`Software\Microsoft\Windows NT\CurrentVersion\Fonts`)と `EnumFontFamiliesExW`。FLEngine にも同じ key の文字列。QFC・ILTools・FLEngine: `...\CurrentVersion\FontSubstitutes` を開く | 仮想 registry は起動時に空(`fle_reg.h`)。key は無く open が 2。`EnumFontFamiliesExW` は**『Segoe UI』1 つだけ**を 1 回コールバック(`fle_gdi.c` 930〜、決定 0098。空の名前か Segoe UI 以外を指定すると名前つきの停止) | 一覧に日本語の face が出ない。FL がどこで一覧を使うか(起動時か設定画面か)は**未確認**(ILTools の `EnumFontFamiliesExW` の呼び元は未調査) |
| **文字の描画・幅** | QFC: AGG + FreeType(`TFontEngineFreetypeFast`・`GetFontSignature`・`CalculateGlyphIdx`・`FontFileContent`)。GDI の文字 API(`CreateFontIndirectW` 呼び元 1 箇所〔QFC RVA 0x111030〕、`ExtTextOutW` 2 箇所、`GetTextMetricsW` 1 箇所)は QFC 内の Delphi RTL(TFont/TCanvas 相当の領域 0x111000〜0x11a000)にある。FLEngine の GDI 文字 API(`ExtTextOutW`・`GetTextExtentPoint32W`・`GetTextMetricsW`・`TextOutW`・`DrawTextW/ExW`・`EnumFontsW`・`CreateFontW`・`wglUseFontBitmapsW`)も import | GDI の文字 API は**名前つきの停止**(`GDI_TEXT_STOP`)か行なし(`TextOutW`・`CreateFontW`・`EnumFontsW`)。実 run でこれらに止まっていない(= 画面の文字は QFC の経路) | 契約は足りているが、**GDI 経路が日本語で通るか(opt1 の問い b)は未確認**。通れば停止する(= 気づく)ので、画面は静かに豆腐にはならない |
| **GetFontData / GetFontUnicodeRanges / GetGlyphIndices / GetCharacterPlacement / Uniscribe(usp10)/ GDI+ / MLang / DirectWrite** | **どの DLL も import しない**(0 件) | 行なし(不要) | 差なし。『NEWTEXTMETRICEX の FONTSIGNATURE』『GetFontUnicodeRanges の応答』を決める必要は無い |
| **FontLink(`...\FontLink\SystemLink`)** | FLEngine・QFC・ILTools・FreeType・Blend2D に文字列が**無い** | 無し | 差なし。Wine の既定の SystemLink(MSGOTHIC.TTC など)は FL の挙動に効かない |
| **コードページ・ロケール** | `GetACP`・`GetOEMCP`・`GetCPInfo(Ex)`・`MultiByteToWideChar`・`WideCharToMultiByte`・`IsDBCSLeadByteEx`・`GetLocaleInfoW`・`GetThreadLocale`・`GetUserDefaultUILanguage`・`EnumSystemLocalesW` | 別調査(言語/ロケールは System\Languages の遅延 VFS 待ち、PM の未決) | ANSI の日本語(932)のファイル名・文字列の扱いは別件 |

### 12.3 契約案(『日本語のグリフを持つフォントを仮想 Fonts に置いた時に FL が選ぶ』)
1. **置き場所と名前**: `C:\Windows\Fonts\msgothic.ttc`(QFC が調べる名前)。内容は日本語を持つ face。フォールバック一覧は『存在する物だけを上の順に』なので、`msgothic.ttc` だけを足せば一覧は [msgothic, arial, tahoma]。
2. **FreeType が開ける形式**: TTC / TTF / OTF(CFF)。`freetype_x64.dll` は cff・truetype・sfnt・psnames を持つ(opt1 の読み)ので、Noto Sans CJK(CFF)も DroidSansFallback(TrueType)も開ける。拡張子 `.ttc` の名前に TTF を入れても FreeType は中身の magic で判断する(読み。`FT_New_Memory_Face` は拡張子を見ない)。
3. **cmap**: 字(U+3042 あ、U+30A2 ア、U+65E5 日 …)が Unicode の cmap にあること。QFC が OS/2 の署名(`GetFontSignature`)で被覆を前もって判断するなら `ulUnicodeRange` の Hiragana(bit 49)・Katakana(bit 50)・CJK Unified Ideographs(bit 59)と `ulCodePageRange1` の bit 17(日本語、932)も要る(読み。Noto Sans CJK JP・MS Gothic は立てている)。
4. **選ばれ方(読み、未確認)**: 主 face にその字が無い時に、一覧を順に見て最初に字を持つ face で描く。= 実機でも漢字は一覧の先頭の `msjh`(繁体字字形)が選ばれる並び(日本語 Windows でも)。我々の仮想 Fonts には `msgothic.ttc` だけを置くので、日本語の字形は置いた face になる。
5. **候補(資材)**: (i) Noto Sans CJK JP(OFL 1.1。この機械に `NotoSansCJK-Regular.ttc` 19.5 MB。そのままは大きいので Kana + JIS X 0208 に subset して 2〜4 MB〔fonttools が要る。この機械には未導入〕)、(ii) Droid Sans Fallback Full(Apache 2.0、4.0 MB、TrueType。この機械に有り、`fc-query` で kana・日本語の字・日・本・語 を持つと確認。字形は中国語寄り〔lang = ja|zh-tw|cmn|hak|lzh〕)。遅延 VFS の層に置く(PM の方針:フォントの出荷は遅延 VFS)。
6. **字形は実機の MS Gothic ではない**(代用。決定 0148 の S-font-… と同じ扱い。名前 `msgothic.ttc` は仮想ボリューム上の名前だけで、font の name table は変えない)。

### 12.4 副作用の確認項目(実装前)
- 一覧に足すと、**ラテン文字の幅は変わらない**(主 face が `arial`/`OpenSans`/`tahoma` で字を持つ)ので決定 0236 の『segoeui を足すと hint bar が折り返す』のような副作用は出ない見込み(読み。実測は未)。
- 一覧は起動時に作られる(`FontCache` の初期化、読み)。ファイルを足すと **VFS の読み込みが増える**(`msgothic.ttc` は存在確認のみ。字が要った時に丸ごと読む〔`MapViewOfFile`〕。4〜19 MB の遅延取得)。P1 の起動 10 秒に影響しないよう遅延取得にする。
- `FLE_DIAG_VFS_MISS`(cand1)の出力に `C:\Windows\Fonts\msjh.ttf` など 9 個の存在確認が `status 2` で出るはず(読み)。出れば 12.1 の逆アセンブルの読みが実 run で裏づく。

### 12.5 opt1 の問い(08:53)への答え
- (a) 仮想 Windows の中身: `C:\Windows\Fonts` = 代用 3 ファイル(arial/tahoma/lucon)。Fonts キー・FontLink・FontSubstitutes = **空(key が無い)**。`ExtTextOutW`・`GetTextMetricsW`・`GetTextExtentPoint32W` は『字形が face に無い時の代わりの face』を探す実装ではなく**名前つきの停止**。`CreateFontIndirectW` は LOGFONTW を保存した font object を作るだけ。`EnumFontFamiliesExW` は Segoe UI 1 つ。**訂正**: 0319 §2.1 の表(71 行)の『要求された名前を「ある」と答えて』は不正確。実際は『空か Segoe UI のとき 1 回、他の名前は停止』。
- (b) QFC は日本語で GDI の経路を通るか: 静的には**否定的**(QFC の主経路は FreeType/AGG。GDI の文字 API は RTL の領域にあって、`FontCache_AcquireFont` の経路から呼ぶ物は未確認)。確定は実 run で `ExtTextOutW` の停止が出ないこと(出ていない)と Wine の API ログで。
- (c) `EnumFontFamiliesExW` の用途: FLEngine・speaker・FLReWire・ILTools が import。**呼び元・時期は未調査**(FLEngine の呼び元は ILTools〔`RegEnumValueW` と組〕と同じ『フォント名の一覧』と読むが、起動時か設定画面かは未確認)。次の調査に回す。
- opt1 の 3 の『役割表』(segoeui・OpenSans・tahoma・consola/lucon …)は、`FontCache_AcquireFont` に渡すパスの選択(`Shared\Artwork\Fonts` の自前フォントか `C:\Windows\Fonts\...`)で、フォールバック一覧(今回の 10 個)とは別の表。決定 0148 の実測(`arial.ttf` 84 回・`tahoma.ttf` 26・`lucon.ttf` 7)は前者。

## 13. O1(clip のドラッグの絵が出ない)・O3(About のロゴが 4×3 のまま)— layered 窓・画面 DC・LockWindowUpdate の契約の読み(PM 08:5x。09:0x)

**先頭(何を実行したか)**: 実エンジン(FL)・Chrome・Wine は実行していない。(1) `FLEngine_x64.dll` の逆アセンブル(capstone。LockWindowUpdate・GetDCEx・ImageList_*Drag の呼び出し箇所と、直前の flags の即値)、(2) runtime の C の読み(`fle_layered.c`・`fle_desk.c`・`fle_gdc.c`・`fle_win.c`・`fle_gdi.c`・`fle_dwm.c`・`fle_osver.c`)、(3) **native の harness(`tools/scroll0319/scroll_0319.c` の面 L。本物の runtime C に clang でリンクして実行)**: LockWindowUpdate 中の GetDCEx の描画。数字は『読んだ値』と『この harness の実測』を分ける。gpu1 の観察(実 Chrome 156、E13)は gpu1 の物。

### 13.1 結論(先に)
1. **O1 の『ドラッグ中の絵が出ない』は、契約の不足を一つ実測で確定した: `GetDCEx(DCX_LOCKWINDOWUPDATE)` が LockWindowUpdate 中に描けない(b)**。Learn GetDCEx: DCX_LOCKWINDOWUPDATE「LockWindowUpdate が有効でもこの窓への描画を許す。tracking 中の描画に使う」。runtime は『ロックされた窓の DC は全て可視領域が空』(LockWindowUpdate のページの文)だけを実装し、**GetDCEx の例外が無い**ので、描画は捨てられる(`fle_gdc.c` の注記: 『DCX_LOCKWINDOWUPDATE は…状態だけの DC では何も変えない』)。FL の側: **`LockWindowUpdate` の呼び出し 7 箇所、`GetDCEx` の flags 0x402(DCX_CACHE | LOCKWINDOWUPDATE)・0x412(DCX_CACHE | CLIPSIBLINGS | LOCKWINDOWUPDATE)・0x12・0x10001 など**(FLEngine の逆アセンブル。RVA 0x20b803〔0x402〕、0x21757a〔0x12〕、0x21f67b〔0x412〕、0x3f26eb/0x405323〔0x10001〕、0x61c6d8)。= **FL は LockWindowUpdate を取って、ロック越しに描く定番のドラッグ(tracking)描画をしている**(読み)。
   - 実測(native harness、面 L、`tables/0319-scroll-harness-L-lockwindowupdate-before-after.out`): LockWindowUpdate(p) → 窓 DC を 3 通りで取って赤く塗る → 解除。**差分なしの runtime: 3 つとも変化なし**(DCX_CACHE | DCX_LOCKWINDOWUPDATE の描画も捨てられる)。**差分あり: DCX_CACHE | DCX_LOCKWINDOWUPDATE の描画だけが残り(RED)、GetDC と DCX_CACHE だけの描画は捨てられたまま**(Learn どおり)。U/C/K/D の面は差分の有無で出力が同一。
   - 差分 `impl/0319-dcx-lockwindowupdate.diff`(61 行、fle_win.c・fle_win.h・fle_gdi.c。適用しない): `fle_win_dc_query_ex(hwnd, dcx, out)` を足し(`fle_win_dc_query` は dcx = 0)、DCX_LOCKWINDOWUPDATE(0x400)の GetDCEx の DC は `is_locked_or_child` を無視。`fle_gdi.c` の窓 DC の target はその DC の flags で問い合わせる。stack(HEAD + 0319 の積み)と tree-rt1 の両方に dry-run で当たる。
   - まだ分からない点: クリップのドラッグが **この経路を通っているか**(読み)。確認は cand1 の run で、LockWindowUpdate/GetDCEx の呼び出しを数える診断が要る(FLE_DIAG_WIN は窓の create/destroy だけ。GetDCEx の診断は足していない)。
2. **VCL の TDragImageList 経路**: FLEngine は `comctl32!ImageList_BeginDrag / DragEnter / DragMove / DragLeave / DragShowNolock / EndDrag` を import し(呼び出し 20 箇所超。RVA 0x21bc26〜0x21bf16〔VCL の TDragImageList〕、0x2ff3c4〜0x30b254、0x612f90・0x61332d)、**runtime にはこの 6 個の行が無い**(`api-coverage.tsv`: state_default/ship/ship2 = none)。呼ばれれば『API 名なしの停止』になる(契約 b)。gpu1 の run が停止せずに進んだ(ドラッグが続いた)なら、この drag はそれらを呼んでいないか、停止が soft。**gpu1 に確認してほしい**: ドラッグの時点で `fle: unsupported` 系の行が出たか。
3. **画面 DC への描画が見えない(b の疑い。読み)**: `GetDC(NULL)` / `GetWindowDC(GetDesktopWindow())` の DC の描画は `lid 0` の『デスクトップの面』(`fle_raster_window_surface(0, …)`)に書かれるが、合成器 `fle_desk.c` の `gather()` は**最上位窓の面だけ**を集める(デスクトップの面は入力にならない)。窓の無い所は固定色(5,7,10)(S-desk-bg)。= 画面 DC への直接描画(ドラッグの輪郭・ゴースト)は画面に出ない。Learn GetDC: 『NULL → 画面全体の DC』、実機(DWM)では画面 DC の描画は他の窓が描き直すまで見える。**実行して確認したものではない**。FL が画面 DC にドラッグの絵を描いているかは 1 の診断で分かる。
4. **layered な子窓**: `ulw()` は `q.root != hwnd` の窓に**名前つきの停止**(『a child window with WS_EX_LAYERED (Windows 8): not modelled』)。合成器は最上位窓だけ。gpu1 の run に停止は報告されていないので、O1/O3 の原因ではない(今は潜在的な不足)。Learn Window Features: Windows 8 以降は子窓にも WS_EX_LAYERED が使える。
5. **FL が layered を選ぶ条件は満たされている**(読み): `DwmIsCompositionEnabled` = TRUE(FL64.exe の manifest が Windows 8/8.1 を宣言。`fle_dwm.c`)、OS = Windows 10 22H2 (`fle_osver.c`)。`TLightweightLayeredControl`(`f_71e570`)は `UpdateLayeredWindowIndirect`(GetProcAddress の cell)を優先し、無ければ `UpdateLayeredWindow`(docs の facts)。runtime は両方を持つ。
6. **ロゴが 4×3 の理由(O3)は未確定**。`ulw()` は psize が来れば窓の大きさを変える(`fle_win_layered_rect`)ので、小さいままなのは FL が大きさを渡す呼び出しに来ていないため(読み)。画像の読み込み(`Data\…`)が失敗して既定の 4×3 / 3×3 のまま、が第一候補だが**未確認**。確認用に TEST 専用の診断 `impl/diag-g9-fl-ulw.diff`(FLE_DIAG_WIN=1 で ULW/ULWI の全呼び出しの dst・size・hdcSrc・src・flags・blend・呼び元を stderr に。`fle_layered.c` に 1 ブロック。構文のみ確認、**未実行**)。`tools/a22-diag-read.py` が『2b』の節で窓ごとに size の遷移をまとめる(fixture で確認)。

### 13.2 まとめ表
| 項目 | 区分 | 状態 |
|---|---|---|
| GetDCEx(DCX_LOCKWINDOWUPDATE) がロック中に描けない | **(b) 契約の不足** | **実測で確定**(native harness)、差分あり |
| ImageList_BeginDrag/DragEnter/DragMove/DragLeave/DragShowNolock/EndDrag の行が無い | (b) | 事実(coverage 表)。ドラッグがこれを呼ぶかは未確認 |
| 画面 DC の描画が合成されない | (b)の疑い | 読み(未実行) |
| layered な子窓 | (b) 潜在 | 名前つき停止。報告された停止は無い |
| ロゴが 4×3 | 未確定 | 診断 diff を用意 |

## 14. comctl32 `ImageList_*` の drag 系(PM 09:1x の許可。`impl/0319-imagelist-drag.diff`、659 行。適用しない)

**先頭(何を実行したか)**: native の harness(`tools/scroll0319/scroll_0319.c` の面 I。本物の runtime C〔fle_imagelist.c・fle_gdi.c ほか〕に clang でリンクして実行)。`clang -fsyntax-only -Wall` と `emcc -fsyntax-only -Wall`(fle_imagelist.c・fle_gdi.c): 警告 0。stack(HEAD + 0319 の積み)と mt3 の tree-rt1 に dry-run で当たる。**FL・wasm・Chrome では実行していない**。Wine 10 での同じ面の実行は、Wine の窓が空いてから(PM の合図待ち)。

### 14.1 実装した物と、しない物の表(全 23 個 + SetDragCursorImage)
| API | FLEngine ほかの import | 状態 | 契約・補足 |
|---|---|---|---|
| ImageList_Create | ○(9 module) | **実装** | cx, cy > 0 でなければ NULL。flags を保持。背景色は CLR_NONE で始まる(Wine) |
| ImageList_Destroy | ○ | 実装 | 不正な handle は FALSE |
| ImageList_GetImageCount | ○ | 実装 | 不正な handle は 0 |
| ImageList_GetIconSize | ○ | 実装 | 不正な handle・両方の出力が NULL は FALSE |
| ImageList_SetIconSize | ○ | 実装 | Learn『全画像を消す』。不正は FALSE |
| ImageList_GetBkColor / SetBkColor | ○ | 実装 | 失敗は CLR_NONE、Set は直前の色を返す |
| ImageList_Add | ○ | **実装(drag の像の素)** | 画像数は bitmap の幅 ÷ cx(Learn)。画素は 32 ビット B,G,R,A に**コピー**(bitmap は呼び出し側が消す)。ILC_MASK の list は mask の 1 ビット = 透明。ILC_COLOR32 の 32 ビット DIB に alpha が有ればその alpha |
| ImageList_Replace | ○(FLEngine・Wrapper・Sytrus・Harmor・Edison・FLEX。ILTools 等は無し) | 実装(Add と同じ画素の入口) | i の範囲外は FALSE |
| ImageList_BeginDrag | ○ | **実装** | himlTrack・iTrack が不正か drag 中なら FALSE(Wine)。像を一時の list にコピー |
| ImageList_DragEnter | ○ | **実装** | hwndLock の窓を LockWindowUpdate して像を (x − dx, y − dy) に表示。座標は窓の左上基準(Learn)。NULL はデスクトップ |
| ImageList_DragMove / DragShowNolock / DragLeave / EndDrag / GetDragImage | ○ | **実装** | DragMove: 古い場所の画素を戻し、新しい場所の画素を保存して描く。DragShowNolock: 既にその状態なら FALSE(Wine)。DragLeave: 像を消して DragEnter が掛けた lock を外す。GetDragImage: 位置(= 最後の x, y)・hot spot・一時の list |
| ImageList_Remove / SetImageCount / GetIcon / Draw / DrawEx / Read / Write | ○ | **名前つきの停止**(行あり) | 『今は drag だけ』。FL が呼ぶと API 名つきで止まる(今までは API 名なしの abort) |
| ImageList_SetDragCursorImage | **どの module も import しない**(api-coverage に 0 件) | 行なし・未実装 | ImageList_Merge を要する(Wine)。呼び手が無い |

### 14.2 描画(O1 の穴との関係)
像は Wine と同じく `GetDCEx(hwnd, NULL, DCX_WINDOW | DCX_CACHE | DCX_LOCKWINDOWUPDATE)`(= 0x403)の DC に描く。**§13 の `0319-dcx-lockwindowupdate.diff` が前提**(この DC はロック中でも描ける)。像は alpha 192/255 で重ねる(Wine の `ILS_ALPHA`・Frame 192〔UNOFFICIAL。Learn は透明度を規定しない〕)。DC の解決は既存の `get_target`(窓/メモリ DC、クリップ、可視領域)を使い、書いた画素は `target_wrote` で合成器へ知らせる。`fle_gdi.c` に足したのは `fle_gdi_bitmap_view`(bitmap の画素の入口。mt3 の 0313 §11.3 の設計)、`fle_gdi_dc_blend32`(保存 + alpha 合成)、`fle_gdi_dc_put32`(保存した画素を戻す)の 3 関数。

### 14.3 実測(native harness、面 I。`tables/0319-scroll-harness-I-imagelist-drag.out`)
16×16 の赤い像、hot spot (3, 4)、窓 300×300(10 px の帯)。DragEnter(20, 20) で像は (17, 16)〜(32, 31): (25, 25) = 帯(y=20)に赤を 192/255 で重ねた色 `2e0ac6`(R 198・G 10・B 46 = (255·192 + 24·63)/255 …)、像の外 (33, 25)・(5, 5) は帯のまま。DragMove(60, 60): 古い場所 (25, 25) は帯に戻り、(65, 65) = `290fce`(帯 y=60 に重ねた色)。DragShowNolock(FALSE) = 1、もう一度 = 0(Wine)、TRUE = 1。**DragEnter の間は窓がロックされ、GetDC で塗っても捨てられる**((105, 105) は帯のまま。Learn)。DragLeave = 1 で像は消え(帯 y=60)、ロックが外れて GetDC の塗りが効く。EndDrag の後の GetDragImage は NULL。`GetDragImage` の位置は (60, 60)・hot spot (3, 4)。
- **これで O1 の絵が出るか**は、FL のドラッグがこの経路を通っているか(未確認)による。通っていなければ別の穴。gpu1 の既存の run のログには ImageList の行も停止も無かった(= 呼ばれていないか soft。§13)。cand2 の run で `FLE_DIAG_WIN`(+ 今回の診断)で確かめる。
- Wine 10 の同じ面の値は未取得(合図待ち)。

## 15. O9(絵が崩れる)— 子窓の移動と『親の空いた領域の無効化』の読みと実測、窓ごとの可視・描画の診断 `diag-g9-fl-paint.diff`(PM 09:2x。09:5x)

**先頭(何を実行したか)**: native の harness(`tools/scroll0319/scroll_0319.c` の面 M・N・P。本物の runtime C に clang でリンクして実行。スタックは HEAD d71efab5 + 0319 の 9 本 + scrolldc + dcx-lockwindowupdate + imagelist-drag + 本診断)。**FL・wasm・Chrome では実行していない**。Wine 10 での同じ面(M の 8 通り、N)は未取得(PM の合図待ち)。『FL の窓の style に WS_CLIPCHILDREN が付くか』の VCL の記憶(後述)は**読み/記憶であり未観測**。

### 15.1 結論(先に)
1. **子窓の単純な移動では崩れない**(PM の仮説〔SetWindowPos / MoveWindow / DeferWindowPos で子を動かしても親の空いた領域が無効化されない〕は、harness では再現しない)。親が WS_CLIPCHILDREN の有無にかかわらず、SetWindowPos・MoveWindow・BeginDeferWindowPos/DeferWindowPos/EndDeferWindowPos で子を 60 px 下げると、古い場所は親の帯が描き直され(M1〜M3、M5〜M7: 古い場所 = 帯、新しい場所 = 赤)、SWP_NOREDRAW を付けた時だけ古い絵が残る(M4・M8。契約どおり)。実装は決定 0214 の `apply_pos` / `unc_end` の一本の経路(3 つの API とも通る)。
2. **gpu1 のログにある FL の並び**(ScrollWindowEx(flags 0、hrgnUpdate)→ 返った領域を FL が無効化 → 子窓を SetWindowPos で動かす)を面 N で再現した。親が **WS_CLIPCHILDREN なし**の時だけ、子の新しい場所の上半分に『スクロール後の面の古い絵がコピーされたもの』が残る(N1: 子の DC (30,20) = 帯(y=90) / (30,50) = 赤)。理由: ScrollWindowEx(flags 0 = 子はスクロールしない)が親の DC(子の面を含む)の画素を 30 px 下にコピーし、続く SetWindowPos(子)は SWP_NOCOPYBITS が無いので**その時点の面の旧位置の画素を新位置へコピーし**、子は無効化されない。これは契約の読みどおりの動き(Learn SetWindowPos「bits をコピーする」、ScrollWindowEx「子はスクロールしない」)で、実装の抜けではない見込み(Wine での確認は未取得)。**親が WS_CLIPCHILDREN の時は崩れない**(N2: 子の DC は赤、親の DC は子を除くので CLR_INVALID)。
3. したがって『FL の画面が崩れる』が O9 の原因なら、候補は (i) 実機の FL のスクロール容器に WS_CLIPCHILDREN が**付いていない**(VCL の記憶: `TWinControl.CreateParams` は `csAcceptsControls` のコントロール〔TPanel・TScrollBox など〕に WS_CLIPCHILDREN を付ける。**未観測**)、(ii) 窓を持たない部品(TGraphicControl)の描画が親の WM_PAINT に載っていて、無効化された領域の外が古い絵のまま、(iii) FL 自身の描画順、のどれか。**harness だけでは決められない**。実行で見る物は 15.3。

### 15.2 実測(`tables/0319-scroll-harness-M-childmove.out`、`…-N-fl-sequence.out`、`…-diag-paint-sceneP.out`)
| 面 | 内容 | 結果 |
|---|---|---|
| M1〜M4 | 親 WS_CLIPCHILDREN なし。子 (100,100,60×60) を (100,160) へ。SetWindowPos / MoveWindow / DeferWindowPos / SetWindowPos+SWP_NOREDRAW | 古い場所 (130,110)・(105,155) = 帯、新しい場所 = 赤。NOREDRAW だけ古い場所が赤のまま |
| M5〜M8 | 同じ 4 通りで親が WS_CLIPCHILDREN | 古い場所 = 帯(NOREDRAW だけ赤)。新しい場所は親の DC から読むと CLR_INVALID(子を除く DC。契約) |
| N1 | 親 WS_CLIPCHILDREN なし。ScrollWindowEx(0,+30,NULL,NULL,hrgn,NULL,0)=1(hrgn の外接 (0,0,300,30))→ InvalidateRgn(hrgn)→ SetWindowPos(子 +30)→ UpdateWindow | 親の古い場所 (130,110)・(130,125) = 帯(描き直し)。**子の DC (30,20) = 帯(y=90)(スクロール後の古い絵のコピー)**、(30,50) = 赤 |
| N2 | 同じ並びで親が WS_CLIPCHILDREN | hrgn の外接 (0,0,300,190)。子の DC (30,20)・(30,50) = 赤。古い場所 = 帯 |

### 15.3 診断 `impl/diag-g9-fl-paint.diff`(TEST 専用。`FLE_DIAG_WIN=1` に乗る。製品では出ない)
窓ごと(lid)に stderr へ `fl-diag: …` を出す。**観測だけ**で、窓の挙動は変えない。
- `vis lid N … 0->1|1->0 by <API> own-bit B first-hidden-lid L …`: 窓の可視(`is_visible`)が変わった時(CreateWindowExW・ShowWindow・SetWindowPos・SetWindowLongPtrW(GWL_STYLE)・ShowOwnedPopups の後に全窓を走査して差を出す。親が出た時の子の変化も出る)。
- `show lid N … cmd C was W`: ShowWindow の呼び。
- `paint-msg lid N …`: WM_PAINT を取り出した(GetMessage/PeekMessage の合成)時。`BeginPaint lid N … rcPaint (l,t,r,b) erase E` / `EndPaint lid N`。
- `draw-discard lid N … first-hidden-lid L <API>`: 見えない窓への描画の破棄(S-draw-hidden)。L = その窓から親へたどって WS_VISIBLE を欠く最初の窓の lid(Balance の lid 171 のような『作られたまま見えない』窓が描画を受けているかを、窓単位で言える)。
- 各 kind は窓ごとに最初の 3 回と、その後は 2 のべき乗回だけ出す(`n` は正確な累計)。全体で 8000 行まで。
- 面 P(`tables/0319-diag-paint-sceneP.out`): 見えない子 lid 2 の下の見える孫 lid 3 に描くと `draw-discard lid 2 … first-hidden-lid 2`、`lid 3 … own-bit 1 first-hidden-lid 2`(孫自身は見える bit を持つが親が見えない)。ShowWindow(lid 2, 5) で `vis lid 2 0->1`、`vis lid 3 0->1`(by ShowWindow)、WM_PAINT の取り出し、ShowWindow(lid 2, 0) で両方 `1->0`。
- 読み方: FL の run で `FLE_DIAG_WIN=1` にして、(1) スクロール容器の `win create` の行の style の 0x02000000(WS_CLIPCHILDREN)の有無、(2) スクロール前後の `draw-discard`(崩れた部品が『見えない窓』への描画として捨てられていないか)、(3) `vis` の変化(出るはずの窓が出ているか)、(4) `paint-msg` / `BeginPaint` の rcPaint(スクロールで返した領域が実際に塗り直されているか)を見る。

### 15.4 残りの未確認と、次に要る物
- Wine 10 の M(8 通り)と N の値(合図待ち)。N1 のコピーが Wine でも同じなら、N1 は契約どおり = 実装の抜けではない。
- FL の run のログ(cand2 の run で `FLE_DIAG_WIN=1`+ 本診断)。
- 実装の差分は**出していない**(再現した抜けが無いので `0319-childmove-invalidate.diff` は作らない)。

## 16. O5(『callback nesting is deeper than FLE_CB_MAX_DEPTH (16)』)— 連鎖の実測、入れ子は正当か、スタックの実測、原因の分類と差分 `0319-callback-stack-limit.diff`(PM 09:2x。09:5x)

**先頭(何を実行したか)**: 実エンジン(wasm の TEST bundle = mt3 の tree-g9a + `diag-g9-cb.diff`、`fl-engine-test-g9a-cb`)を node の G3 で実行(`resize.g3`: 起動 → Welcome wizard が開いたまま 3 s → `display 1160 864`、`FLE_MONITOR_W=2060 FLE_MONITOR_H=1264`、`FLE_DIAG_CB=1`)。run = `~/work/alpha22w/runs/rz-4`。**7 秒で再現**(exit rc 0 の node が `fle: fle_call_guest: callback nesting is deeper than FLE_CB_MAX_DEPTH (16)` で止まる)。連鎖は `tables/0319-cb-chain-rz4.txt`。修正の差分は native の harness(面 Q、本物の runtime C に clang でリンク)で検証、wasm の host-stack の検査は **emcc -Wall -Wextra で警告 0 の構文確認まで**(wasm の実行確認は TEST bundle を作り直して rz-5 で行う:結果は別の info)。Chrome は使っていない。

### 16.1 連鎖(rz-4 の 17 段。表 `tables/0319-cb-chain-rz4.txt`)
| 段 | 窓(hwnd) | メッセージ | host stack の増分 | guest stack の増分 |
|---|---|---|---|---|
| 1–2 | (起動側の関数・0x2000000 への WM_MOVE) | — | (別の stack:thread 側) | — |
| 3 | 主窓 0x7f990038 | WM_DISPLAYCHANGE(apply_display が送る) | — | — |
| 4 | 主窓 | WM_WINDOWPOSCHANGED(主窓の WM_DISPLAYCHANGE の処理の中の SetWindowPos から) | **34,912 B** | 3,048 B |
| 5 | 主窓 | (DefWindowProc の中継) | 1,056 B | 1,512 B |
| 6 | 主窓 | WM_SIZE(SIZE_MAXIMIZED) | 1,072 B | 48 B |
| 7–9 | 子 0x7f9900c4 | WM_WINDOWPOSCHANGED → DefWindowProc → WM_SIZE | 34,160 / 1,056 / 1,072 | 2,088 / 1,080 / 48 |
| 10 | 兄弟 0x7f990040 | WM_WINDOWPOSCHANGING | 33,920 B | 2,840 B |
| 11–13 | 孫 0x7f9900d4 | WM_WINDOWPOSCHANGED → DefWindowProc → WM_SIZE | 34,176 / 1,056 / 1,072 | 1,344 / 1,512 / 48 |
| 14–16 | ひ孫 0x7f99016c | WM_WINDOWPOSCHANGED → DefWindowProc → WM_SIZE | 33,920 / 1,056 / 1,072 | 2,632 / 1,080 / 48 |
| 17 | ひ孫 | (DefWindowProc の中継の WM_SIZE) | 1,056 B | 992 B |

### 16.2 入れ子は正当か(フィードバックのループではないか)— **正当**
- 窓が 4 つ別々(主窓 → 子 → 孫 → ひ孫)で、**同じ窓の同じメッセージが 2 段以上に現れるのは、窓の手続き(0x52a570)と DefWindowProc の中継(0x7ff900000e30)の対だけ**(スクリプトの『繰り返し』の表示はこの対)。同じ窓へのループは無い。
- 形は VCL の配置の連鎖:親の WM_SIZE の処理が子を SetWindowPos する → 子へ WM_WINDOWPOSCHANGED が同期に送られる(Learn SetWindowPos)→ DefWindowProc が WM_SIZE を送る → 子の処理が孫を … 。**窓の木の深さ × 3 + 前後**がそのまま入れ子の深さ(木が 4 段 → 3 + 12 + 兄弟の WM_WINDOWPOSCHANGING 1 + 中継 1 = 17)。実 Windows でも同じ深さになる(SendMessage は同じスレッドでは直接の呼び出しで、入れ子の上限はスタックの大きさ:決定 0064 自身も『16 はこの環境の選択で、根拠のある値ではない』と書いている)。
- 通常の動作でも 9 段以上は出る(同じ run で 24 回、9 段の連鎖〔WM_WINDOWPOSCHANGED → DefWindowProc → WM_SIZE → WM_NCPAINT …〕が `deep` として出た)。16 は通常の倍にすぎない。

### 16.3 スタックの実測(rz-4 の値)
- **host(C)のスタック**:17 段で使ったのは **180,656 B**(8 MiB = 8,388,608 B のうち 2.2%)。1 段の最大は **34,912 B**(SetWindowPos から送る WM_WINDOWPOSCHANGED / WM_WINDOWPOSCHANGING の段。`apply_pos` と `unc_*` の C のフレーム)、DefWindowProc の中継と WM_SIZE の段は約 1.06 KB。平均は(34.9 + 1.06 + 1.07)/ 3 ≒ 12.3 KB/段。
- **guest のスタック**:17 段で **18,320 B**。1 段は 48〜3,048 B(最大 3,048 B)。主スレッドの guest スタックは 16 MiB、他のスレッドは 1 MiB(`FLE_THREAD_DEFAULT_STACK`)。
- 最悪の段(34.9 KB)だけが続くとして、8 MiB の shadow stack が尽きるのは約 240 段、平均の形なら約 680 段。**16 で止める根拠はスタックには無い**。
- 注意:TEST の診断が出す `host-stack-free` は **最初のスレッドでだけ正しい**(`emscripten_stack_get_free()` は主スレッドの stack の範囲で計算するため、他のスレッドの JSPI 領域では意味のない値になる。rz-4 の 1〜2 段の 805,892,064 や 42,103,136 がそれ)。新しい診断は `fle_thread_wasm_stack_left` で各スレッドの領域に対して出す。

### 16.4 分類と差分(`impl/0319-callback-stack-limit.diff`、114 行、fle_callback.c・fle_callback.h・fle_thread_wasm.c・fle_thread_wasm.h)
- **(a) 余計な処理**(我々の恣意的な上限):16 は Windows の契約ではない。**機構ごと直す**:数の上限をやめ、Windows と同じく**スタックの残りで止める**。
  1. guest のスタック:`rsp − StackLimit(TEB。fle_teb_set_stack が登録した、guest が走っているメモリの下端)< FLE_CB_GUEST_RESERVE(64 KiB)` で、数値つきの名前つき停止。
  2. host のスタック(wasm):走っているスレッドの shadow stack の残り(`fle_thread_wasm_stack_left`:第 1 スレッドは emscripten の stack の残り、他は領域 − canary)`< FLE_CB_HOST_RESERVE(512 KiB)` で名前つき停止。
  3. `FLE_CB_MAX_DEPTH` は **256 の最後の砦**(host の stack を問えない native のテストで、暴走を止めるため)。実測の通常の連鎖 17 に対し十分に深く、平均 12.3 KB × 256 = 3.1 MB は 8 MiB の範囲内。
- **検証(native harness、面 Q。`tables/0319-scroll-harness-Q-callback-nesting.out`)**:窓のメッセージが自分に SendMessage で N 回送る。修正前(t13): N = 100 は **16 段で停止**(`callback nesting is deeper than FLE_CB_MAX_DEPTH (16)`)。修正後(t14):**N = 100 は戻り値 107(= 全 101 段が戻った)**、N = 300 は 256 段で最後の砦が止める、guest の stack の残りを 72 KiB にした面では **8 段目で『the guest stack is used up』で停止**(数値つき)。
- 未確認:wasm の host 検査の実行(TEST bundle の作り直しと node G3 rz-5。別の info で結果を渡す)。**この修正で 17 段の連鎖が通っても、その先で別の止まりが出る可能性は残る**(rz-5 で確かめる)。
- **FL へのこの診断の足し方**:`diag-g9-cb.diff` は『修正の後ろ』に積む形に作り直した(host-stack-free をスレッドごとに出し、`LIMIT-guest` / `LIMIT-host` / `LIMIT-count` の 3 つの停止で連鎖を出す)。

### 16.5 mt3 の tree-rtb(実スレッド版、GIL=1)用の版 `impl/0319-callback-stack-limit-rtb.diff`(139 行。PM 10:0x の依頼)
- **実行した物**: tree-rtb の runtime のコピーに実適用(読み取りのみ)。FAILED 0・.rej 0・.orig 0。`emcc -fsyntax-only -Wall -Wextra -pthread -msimd128 -matomics -mbulk-memory` を、GIL なしと `-DFLE_GIL -DFLE_TLS_MORE` の両方で fle_callback.c・fle_thread_wasm.c とも警告 0。native の面 Q(rtb の runtime のコピーを本物の C にリンク、非 GIL): Q1 = 107、Q2 = 256 の砦で停止、Q3 = 8 段で guest stack の停止。**wasm の実行、GIL=1 の実 Worker での実行は未確認**。
- rtb で変えた所:(1) `FLE_TLS_MORE` では `fle_teb_cur` が `fle_ctx` のフィールドのマクロ(fle_ctx_more.h)なので、fle_callback.c はそれを extern 宣言せず、マクロをそのまま使う(`fle_teb` は共有の配列のまま)。(2) rtb の fle_callback.c に入っている旧 diag(FLE_DIAG_CB)の host-stack-free を `fle_thread_wasm_stack_left` に更新し、`LIMIT-guest/host/count` の 3 つの停止で連鎖を出す。
- **スレッドごとに host stack の残りが正しいことの読み**:`fle_thread_wasm_stack_left(idx)` の idx = `fle_thread_current()`。GIL 版では `cur` が `_Thread_local`(fle_thread.c:167)なので、その Worker で走っている guest スレッドの idx。idx ≥ 1 のスレッドは fle_thread.c:1290 の `fle_thread_wasm_stack_alloc(i)` で 8 MiB の領域(共有メモリ上の `region[i]`)を持ち、その Worker の JSPI stack の `__stack_pointer` は領域の top(`top_of`)に設定されるので、`sp − (region + canary)` が残り。idx = 0(Worker 0 の第 1 スレッド)は Worker 0 のインスタンスの emscripten stack 限界(`emscripten_stack_get_free()`)。idx ≥ FLE_THREAD_MAX(Worker の idle の BOOT 文脈)も `emscripten_stack_get_free()`。`wk_main` の pthread の C stack(512 KiB)は guest の callback には使われない(guest スレッドは JSPI stack + 領域で走る)。**読みであり、実行では未確認**。

## 17. G9 の残り(Delay 2・Parametric EQ 2・Frequency Shifter・Soundgoodizer・FLEX・Fruity Wrapper)の実モジュール化の見積り表と、足す順の提案(PM 10:1x の依頼。期限 10:40)

**先頭(何を実行したか)**: **読みだけ**(Chrome・Wine・wasm の実行は 0 本)。使った物: 原本の DLL(opt1 の prefix の `Plugins\Fruity\{Effects,Generators}\…`、`.off-0307` に退避した物と隠していない `Frequency Splitter\engine_x64.dll`。**読み取りのみ**)の pefile、`lift/out-git/alpha21-results/tools/a23-plugin-imports.py`(0309 の道具。ship5 の行 + fe-lift の表 = 819 行で突き合わせ。今の HEAD の出荷の行と同じ集合)、原 FLP(`serina1-serum2.flp`)の flp_model での読み、決定 0309(私の前の見積り)と 0320(opt1 の記録〔§8.1・§8.6〕)の引用。**重複を避ける**: 大きさ・lift の実測〔関数数・命令レベルの未対応〕・段階表の人日は 0309 §1.1・§1.3・§4、Wine の段ごとの効果量は 0320 §8.6 を見てください。ここに足したのは (a) 今回新しく読んだ物〔Frequency Splitter の engine_x64.dll の import、FLP の各 module の instance の場所、依存 DLL の大きさ、S1 の ×2.0〜2.1 の再計算〕、(b) 足す順の提案です。表の元データは `tables/0319-g9rest-estimate.tsv`、import の突き合わせは `tables/0319-g9rest-imports-{summary,none}.tsv`(`tools/a22-plugin-imports-ship6.py`)。

### 17.1 表(Fruity Delay 2 は mt3 が準備済み。参考として同じ表に載せる)
| module | native の大きさ(DLL) | 依存 DLL | FLP の instance | CPUID / XGETBV の箇所(opt1 0320 §8.1) | import(全 / 行なし / FLEngine に無い行なし) | Wine の効果(段の差。0320 §8.6) | S1 への寄与(native × 2.0〜2.1 の読み) |
|---|---:|---|---|---|---|---|---|
| Fruity Delay 2 | 3.73 MB | なし | 1(Insert 2 の slot 3) | 5 / 1 | 543 / 74 / 0 | RMS 1040.5、長さ同じ | +7.5〜7.8 MB |
| Fruity Parametric EQ 2 | 4.04 MB | **Frequency Splitter\engine_x64.dll**(3.35 MB、Intel IPP、IPPCODE・IPPDATA 節あり、export は VSTPluginMain・dispatch・main・mydispatcher = VST2 形式) | **8**(Insert 2 slots 1・2、5、11、12、16、17、18) | 本体 5 / 1 + engine 46 / 4 | 548 / 74 / 0(engine 77 / **0** / 0) | RMS 8058.8、**+138 フレーム** | +8.1〜8.5 MB(+ engine +6.7〜7.0 MB) |
| Frequency Shifter | 3.88 MB | **Engine_x64.dll**(263 KB、MSVC、VEX スカラー倍精度・FMA 692 箇所〔0309 §1.3〕、export は dispatch だけ) | 1(master の slot 4) | 5 / 1 + Engine 4 / 2 | 548 / 74 / 0(Engine 73 / **0** / 0) | **RMS 21867.7、+12,679 フレーム**(周波数が動く) | +7.8〜8.1 MB(+ Engine +0.5〜0.6 MB) |
| Soundgoodizer | 6.41 MB | `ildsp_x64.dll` 4.2 MB(文字列があるが呼ぶかは**未確認**)、`AudioRestore.dll` 0.95 MB・`iZAudioRestore.dll` 2.48 MB(文字列、**未確認**) | **12**(Insert 2、6、8、16 ×2、17、18、23、24、25、42、46) | 5 / 1 | 638 / 107 / 0 | FLEX と合わせて RMS 8913.6、+276 フレーム | +12.8〜13.5 MB(依存を lift するなら +15.3〜16.0 MB) |
| FLEX(generator) | 5.19 MB | **engine_x64.dll** 5.67 MB(MSVC、VEX 256 ビット float 18,772 箇所)、**engineV2_x64.dll** 8.12 MB(EVEX の AVX-512 190,302 箇所。0309 §1.3) | 1 | 5 / 1 + engine 48 / 5 + engineV2 47 / 4 | 618 / 88 / 0(engine 338 / 107 / **95**、engineV2 342 / 109 / **95**) | Soundgoodizer と合わせて(上) | +10.4〜10.9 MB(+ engines 約 27.6〜29.0 MB。0309 は engineV2 だけで wasm 44.5 MB と読んだ) |
| Fruity Wrapper(Effects・Generators の 2 ファイル) | 6.00 MB × 2 | 包む VST(Serum 2 ほか)。**この prefix には Serum が無い**(find で 0 件) | 約 21(master の slot 1・2・3・5、Insert 6・10・11・17・18・23・24・45、generator 8 チャネル) | 6 / 1 × 2 | 631 / 112 / **2**(CreateJobObjectW・SetInformationJobObject) | **RMS 29.3**(包む VST が無いので Export にほぼ出ない) | +12.0〜12.6 MB × 2(2 ファイルが共有できなければ) |
- 『S1 への寄与』は、依頼の ×2.0〜2.1 を native の大きさに掛けた**読み**(0309 の ×3.9 〔共有なし〕は古い。実際に wasm にした Balance の比が根拠のはず)。**MSVC・IPP の engine は Delphi の比と違う**(命令の密度・CPU 別の枝を全部抱える)ので、engines の行は粗い。
- **instance の場所は FLP の event 201(名前)と直後の 212(insert, slot)から読んだ**(generator の行は insert 0 / slot 0 として出るので〔FLEX・Wrapper の generator〕数は近似)。『信号がその instance を通るか』は FLP だけでは決まらない(Filter は Insert 47 に信号が来なかった)ので、**Export に効くかの根拠は Wine の段の差**(右から 2 列目)を使う。

### 17.2 import(行なし)の見立て — Balance / Filter との差分だけ
- **Delay 2・PEQ2・Frequency Shifter の本体は Balance / Filter と同じ 74 の集合**(0309。**FLEngine の import の部分集合**)。差分は 0。
- **PEQ2 の engine(IPP)と Shifter の Engine は import がすべて行あり**(77 と 73、行なし 0): KERNEL32 の CRT 系(Fls*・Tls*・Heap*・RtlVirtualUnwind・RtlLookupFunctionEntry・IsProcessorFeaturePresent・QueryPerformanceCounter・LoadLibraryExW・GetModuleHandleExW …)と USER32 の KillTimer(IPP の方だけ)。**新しい行は要らない**。`IsProcessorFeaturePresent` は SSE2 までを真にする実装(fle_sysinfo.c:78)で、IPP の CPU 別の枝(ippInit の `ippGetCpuFeatures` 相当)は CPUID を直接読むので runtime の CPUID(SSE2 固定)に従う。**SSE2 固定で足りるかの読み**: IPP は CPU 別の関数の表(px / w7 / v8 / p8 / g9 / h9 …)を CPUID の結果で選ぶので、SSE2 だと w7 / px 系の関数が選ばれて動く(Wine の参照も SSE2 に固定した run で正常に音が出た: elog22、効果 8058.8)。**VEX / AVX の関数は DLL に有るが選ばれない**ので lift の『未対応』は実行されない限り停止しない(0309 の PEQ2 の closure は命令レベルの未対応 0)。ippInit が呼ぶ API は上の kernel32 の範囲に収まる(import の一覧から。**実行はしていない**)。
- **Frequency Shifter の Engine**: VEX スカラー倍精度と FMA が 692 箇所。**CPUID 4 / XGETBV 2 の AVX 判定の分岐の先**にあるなら SSE2 固定では選ばれない。Wine の SSE2 固定の参照(elog23)で Shifter は大きな効果を出したので、**SSE2 の経路は有る**(読み)。ただし VEX の分岐が必須でなく『判定が AVX なしなら別の関数』なのか、それとも AVX なしだと Engine が何もしない〔音は別の理由で変わった〕のかは**未確認**(次の段で Engine の CPUID 判定の先を逆アセンブルして確かめる)。
- **FLEX の engines**: 行なし 107 / 109 のうち **FLEngine に無い物が 95**(OpenGL 36・d3d9・dbghelp 6・user32 27〜28 …。0309 §1.2)。= **FLEX は GUI / 描画の別の仕事**を連れてくる(音だけが目的なら engine の GL 呼び出しが実行されるかが分岐)。
- **Fruity Wrapper**: 行なし 112(Balance の 74 + 38)、FLEngine に無い 2(`CreateJobObjectW`・`SetInformationJobObject` = 子プロセスの管理。runtime には 0183 の wrp1 で job の行があるはずだが、HEAD の ship5 の集合には無いまま)。

### 17.3 難所の見立て(特に Fruity Wrapper)
- **Fruity Wrapper = VST ホスト**: (i) 包む DLL(Serum 2 ほか。**この機械の prefix には無い**)を LoadLibrary し、VST2 の `VSTPluginMain` / `main` を呼ぶ(VST3 なら IPluginFactory の COM 呼び出し)。**我々の側の穴**として読めるのは、(a) 実 DLL の読み込み(volume に DLL を置く = 利用者の VST は製品に無い。無ければ『Problems』の行 = 原本の挙動と同じ)、(b) VST の窓(親窓に子窓を作らせる〔effEditOpen に親の HWND を渡す〕。ホスト側 = FL の Wrapper は lift 済みの guest コード、VST は第三者の DLL で、user32 / gdi32 の網羅性が足りるか〔未確認〕)、(c) 子プロセスの橋(Wrapper は 32 ビットの VST を別プロセスで走らせる経路を持ち、CreateJobObject の 2 行がその印。64 ビットの VST では通らない見込み〔読み〕)。(d) TFruityPlugHost の契約(Dispatcher の ID・パラメータ・MIDI・オートメーション)は **FLEngine ⇔ Wrapper の guest どうし**で、runtime の実装は要らない(0309 §2)。したがって**我々が持つ穴は主に OS の API の網羅(user32 / gdi32 / ole32 / oleaut32)と、第三者 DLL が走る時の CPUID / x87 / SEH の流儀**。
- 原本の project では Serum 2 が音の本体(serina1-serum2)だが、**Export の参照(Wine)では VST が無いので無音**。製品でも利用者の VST が無ければ同じ。つまり Wrapper を実モジュール化しても **Export の WAV にはほぼ効かない**(RMS 29.3)。Problems 窓の行を 1 つ減らす以外の価値は小さい。

### 17.4 足す順の提案(次に足す順)
1. **Fruity Delay 2**(mt3 が準備済み)— Balance / Filter と同じ集合。FLP に 1 instance、Wine の効果 RMS 1040 で長さは同じ = **判定が一番きれい**(V0〜V4 の規則をそのまま使える)。+7.5〜7.8 MB。人日 +0.5〜1.5(0309)。
2. **Frequency Shifter** — instance は master の 1 つ、**効果が最大**(RMS 21867、+12,679 フレーム)で、判定の信号は強い。Engine(263 KB)は import が全部行あり。難所は **Engine の VEX / FMA 692 箇所**(SSE2 固定で実行されないかの確認が先:+0.5 d の読み)。Filter の判別が決着しない間は、**効果の大きい module で wasm と Wine の差が出るか**を見る第 2 の対照になる。+8.3〜8.7 MB(Engine 込み)。
3. **Fruity Parametric EQ 2** — instance が 8 で Export に効く(RMS 8058、+138)。IPP の engine(77 import がすべて行あり、VST2 形式)を『実モジュール』として足す手続きが要る(`LoadLibraryW` + `GetProcAddress("VSTPluginMain")` の経路。0183 の wrp1 の形)。+8.1〜8.5 + 6.7〜7.0 MB = 約 15 MB と **S1 の予算を一番食う**。ildsp の扱いが先に決まっていると良い。
4. **Soundgoodizer** — 12 instance で、効果は FLEX と合わせてしか分からない(elog20)。ildsp / AudioRestore の依存が呼ばれるかが分岐(**呼ばれるなら lift が要る**:ildsp は x87 944・AVX-512 23,929 命令〔g8 の読み〕)。分離した Wine 参照(Soundgoodizer のみ)が無いので**先に opt1 に頼む**(cumulative の run の順番を変えた版が 1 本)。
5. **FLEX** — 鳴る楽器(Agogo)の本体なので Export の中身は大きく変わるが、**一番重い**: 2 つの MSVC engine(VEX / EVEX、OpenGL・d3d9・dbghelp の 95 の行)。GL の呼び出しを音の経路から外せるか(engine の初期化で必須か)が最初の読み。S1 を一番食う(engines +27.6〜29 MB)。
6. **Fruity Wrapper** — 最後。Export にほぼ効かず(29.3)、第三者 VST の穴を連れてくる。Problems の行を 1 つ減らす目的だけなら、**Wrapper を『見えるだけ』にして VST が無い時の経路〔利用者の VST が無い〕だけを通す**最小の形を先に検討する。

### 17.5 未確認と、確かめる最も安い測定
- ildsp / AudioRestore / iZAudioRestore が Export の経路で**呼ばれるか**(Wine の `modload` の記録〔opt1 が run ごとに取っている『読み込まれた DLL』〕で、Soundgoodizer のみ・PEQ2 のみの run があれば直ちに分かる = 1 本ずつの Wine run)。
- Frequency Shifter の Engine の AVX 判定の先(逆アセンブル 1 時間)。
- 『instance の数が多い module(PEQ2・Soundgoodizer)の native の大きさの 2.0〜2.1 倍』が当てはまるか(Balance の実績だけが根拠。次の module を入れた時に測る)。
- 表の『FLP の instance』は読み(上の注)。

## 18. Filter の E4 用: Wine の参照 prefix の FL の設定 と wasm の registry の seed の差の表、『wasm と同じ設定』の .reg(PM 10:2x の依頼。期限 11:00)

**先頭(何を実行したか)**: **読みだけ**(実エンジン・Wine・Chrome の実行は 0 本)。入力 = opt1 の `prefix-e2/user.reg`(参照の Export が走る prefix。`prefix-e3` と Image-Line の部分は同一)、`lift/harness/seeds/reg-seed-{app,nsis}.tsv`(wasm の registry の起動時の中身)、ci1 の wasm の Export の画面(`export-cand1-ins1lp-r2/export-dialog.png`)、`~/work/opt1-wine/exports/trace/settings2-MIDIForm.txt`。分類は (b)(設定・契約の不足)の可能性の確認で、(a) ではない。

### 18.1 事実(読み)
1. **wasm の registry は seed の `Shared\Paths` の数値(6 行)だけで始まる**。`FL Studio 26\General\*` 以下は何も無い → FL は form の既定(DFM)で動く。
2. **Wine の参照 prefix(prefix-e2)も `General\MIDIForm`(Settings 窓の値。Smart disable・Multithread など)を持たない**(user.reg の Image-Line は 24 key・100 値で、`MIDIForm` も `Mixer` も無い)。`settings2-MIDIForm.txt` は Settings 窓を一度開いて閉じた別の run が書いた物(その値は form の既定をそのまま書き出した物と読める: SmartDisBox=1、MultithreadFXBox/GenBox=1、SafeOverloadBox=1、PlugResetBox=0 ほか)。**したがって、ミキサーの枝を決める Settings の値は、参照(Wine)も wasm も『form の既定』で同じ**(読み)。
3. Wine にあって seed に無い物(表 `tables/0319-reg-wine-vs-seed.tsv`、102 行): `General` の `Run counter`=24 と `Update notification`、**`General\WAVRenderForm` の 29 値(Export 窓の状態)**、`Shared\General` 4(Fancy animations・Smoothing factor・HSD・LastUpdateDate)、`ASIO` 4、MIDI の device 8 key、`Shared\Plugins\…` の VST の探索結果(Fruity Wrapper 用)と `VstDir64`。音声・ミキサー・遅延補償・スロットの有効化に直接関わる Settings の値は**この中に無い**。
4. 唯一、Export の結果に効きうるのは **`WAVRenderForm`**(PlugHQBox=1、NoPolyBox=1、IPBox=4、InputFXBox=1、MasterFXBox=1、DitherBox=0、SplitMasterBox=1 ほか)。wasm 側は ci1 の Export 窓の画面で、**WAV / 16Bit int / Stereo / Resampling 32-point sinc / HQ for all plugins ON / Disable maximum polyphony ON / Dithering OFF / Save playlist markers ON / Save loop markers ON / Save note markers OFF / Save tempo information OFF / Split mixer tracks OFF / Trim PDC silence ON / Enable insert effects ON / Enable master effects ON / Mode Full song / Tail Leave remainder**。Wine の値と対応づけられる項目(HQ・Disable polyphony・Dithering・markers・insert/master effects)は一致している(IPBox=4 が 32-point sinc かは**読み**)。Balance のみの Export が両側でほぼ bit 一致(2 フレーム ±1 LSB)なのもこれと整合する。
5. 『Run counter』: wasm は初回起動の扱い(Welcome wizard と『Help us improve』が開く)で、Wine は 24 回目。Export の経路への影響は読みでは見つからない(未確認)。

### 18.2 結論(読み)
**registry の差は『スロットが有効になる/ならない』の原因の候補としては弱い**: ミキサーの枝に効く Settings の値は両側とも未設定(form の既定)で、差は UI・デバイス・探索結果・Export 窓の状態に限られる。ただし**読みであって、E4 の実測で確定する**(下)。

### 18.3 E4(opt1 に頼む形)
`tables/0319-wasm-like-registry-minimal.reg`(`General\WAVRenderForm` は残し、`General` の 2 値・`Devices`・`ASIO`・`Shared\General`・`Shared\Plugins` を消す)と `…-full.reg`(`FL Studio 26` の木ごと消す = Export 窓も form の既定。**先に窓の画面で上の 4 の値になることを確かめる**。違えば elog16 と比べられない)。使い方: 参照の prefix のコピーに `wine regedit /S FILE.reg`(FL 停止中)→ ins1-lp と原 FLP(基準)の 2 本を Export → `tools/run-filter-cmp.sh`(wasm の cand1 の Export を相手に)。出力が wasm の形(4.778〜4.800 s の厳密な 0、100.58〜100.80 s の無音)に**ならなければ**、registry は原因ではない。

### 18.4 診断 diff `impl/diag-g9f-filter-calls.diff` の下書きを更新(build は RT-B の build のあと)
TEST の木(`~/work/alpha22w/g9f-test-tree`、cand1 のコピー、build 済みでない)に、Filter の image(0x7000000〜0x73B2000)への間接呼び出しの全列を `FLE_DIAG_G9F=1` の時だけ出す(`FLE_TEST_GETENV`)。上限: Eff_Render 型の呼び(引数が Length ≤ 16384・2 つの buffer ポインタの形)は累積 Length が 240,000 フレームを超えるまで(約 5.4 s 分 = 4.778〜4.8 s を含む)、それ以外は最初の 3,000 回。source / dest buffer の peak を呼びの前後で読む(Process が 0 を書いたか、素通しか、呼ばれていないかが分かる)。

### 16.6 訂正(自己申告): 初版の callback-stack-limit は『自分の差分の欠陥』で native のテストを壊した — 直した版と、実際に回したテスト(PM 10:36、mt3 の報告)
**欠陥は 3 つ、すべて私の差分**(分類: (a)/(b) でなく**自分の実装の誤り**):
1. `fle_callback.c` で `fle_teb_cur ? fle_teb_cur : fle_teb` と書いた。非 TLS_MORE では `fle_teb_cur` は**弱い変数**で、リンクされていないテスト(fle_callback.c を少数のファイルとだけリンクする callback・window・monitor・wall_r・wall_fr6 ほか)では address 0 を**値として読んで** SIGSEGV。弱い『配列』(`fle_teb[]`)は address が 0 になるだけで安全だが、変数は違う。→ 直し: `&fle_teb_cur != NULL` のときだけ読む(TLS_MORE は fle_ctx のフィールドなので従来どおり)。
2. guest stack の予約 64 KiB が、テストの guest stack(64 KiB)と同じ大きさで、最初の callback で止まった(wall_fr8・9・9t・10〜12・14〜17・19・20、wall_ia1w、wall_tbl1、present2_uncover、gpu7_ring_poll の 16 executable)。→ 直し: 予約 = **min(64 KiB, スタックの大きさの 1/4)**(スタックの大きさ = StackBase − StackLimit。engine の MiB 級の stack では 64 KiB のまま)。
3. **決定 0064 のテスト 3 件を更新していなかった**: 定数(MAX_DEPTH 16)・『限界ちょうどは通り、もう 1 つで止まる』・『fle_callback.c は素の C で、host に触れない』(`asm` を含む語 `wasm`、`emscripten`、`unsigned long long` の `long` を禁じる)。→ 直し: host の stack の問い合わせを `fle_callback.c` から外し、**`fle_cstack_left()`(弱い宣言。定義は host を知る fle_thread_wasm.c)**にした(fle_callback.c に `wasm`・`emscripten`・スレッドの語は無い)。64 ビットの表示は `PRIx64` / `PRIu64`(`long` を使わない)。テストは『決めた物を変える』形で更新(`impl/0319-callback-stack-limit-tests.diff`)。

**実際に実行した物(native のテスト、本物)**: `git archive HEAD` の複製(`~/work/alpha22w/cbtest/head`)に『0319-callback-stack-limit.diff + 0319-callback-stack-limit-tests.diff』を当て、`harness/test.sh full --jobs 1 --fresh --no-record --only '(callback|window|monitor|wall_r|wall_ref1|wall_fr|present2_uncover|gpu7_ring_poll|wall_tbl1|wall_ia1w|sysinfo|vfs|fruity)'`(CARGO_TARGET_DIR は私の別の場所):
- **初版**(欠陥つき): 30 executable、599 passed、**3 failed**(callback の 3 件〔定数・限界ちょうど・素の C〕)。[mt3 の木では 16 executable が SIGSEGV / 『stack を使い果たした』で落ちた]
- 途中(host の問い合わせを外し、定数を数値にする前): 601 passed、2 failed(新しいテストの期待値の誤り〔reserve 13312 でなく 13310: 登録した下端が page 境界でない〕と、`def_const` が `(64u * 1024u)` を読めない)。
- **直した版: 32 executable、642 passed、failed 0**(callback 19 件 〔新しい 1 件を含む〕、window 37、monitor 25、wall_r 31、wall_r2 17、wall_ref1 6、wall_fr3〜21 と wall_fr9t、wall_ia1w 7、wall_tbl1 8、present2_uncover、gpu7_ring_poll、sysinfo 16、vfs 23)。SKIPPED 2 件は入力の DLL が複製に無いだけ(wall_fr7 の quickfontcache、wall_fr9t の環境変数。差分と無関係)。
- tree-rtb(mt3 の最新の木のコピー)に『rtb 版の差分 + テストの差分』を当てた結果は報告に書く(下の run)。
- **tree-rtb(mt3 の最新の木のコピー。`cp -r` で mt3 の木には書いていない)**: 『rtb 版の差分 + テストの差分』を当てて同じ選択(`…|sysinfo|vfs`)を回した: **635 passed、8 failed**(`tables/0319-callback-native-tests-rtb-with-fix.log`)。比較のため**同じコピーから私の 2 つの差分を戻して**(fle_callback.c・fle_callback.h・fle_thread_wasm.c・callback.rs が mt3 の木と一致することを確認)同じ選択を回した: **634 passed、8 failed、落ちる 8 件も同じ**(`…-baseline-without-fix.log`)。差は『私の新しいテスト 1 件が通る』だけ。= **この 8 件は私の差分と無関係(mt3 の木の状態)**: `fe-lift/callback source_is_plain_c…`(`emscripten`: 旧い第 1 形の診断の `#include <emscripten/stack.h>`)、`vfs`・`wall_fr6`・`wall_fr9`・`window`・`wall_fr7`(`getenv`: 診断を getenv にしてある形。製品の形〔FLE_TEST_GETENV〕では通る、と mt3)、`wall_fr14`(`WM_SETCURSOR hit -2` の 3 行。ci1 の 0323b/c の pin)、`wall_fr9t`(scheduler の trace の assertion `left: 8002`。**私は原因を見ていない**。私の差分は native では fle_thread_wasm.c を含まない〔`#ifdef __EMSCRIPTEN__` の中〕ので無関係と読めるが、mt3 の GIL 版の native の挙動の問題かもしれない)。

## 19. 契約レビュー: mt3 の `0315-getmodulefilename-loaded-path.diff`(GetModuleFileNameW が『最初の LoadLibraryW のパス』を返す)— (b) の契約の実装として正しいか(PM 11:3x の依頼。期限 12:15)

**先頭(何を実行したか)**: **読みだけ**(実エンジン・Wine・Chrome・wasm・native の新しい run は 0 本)。読んだ物: mt3 の差分(119 行、fle_realmod.[ch]・fle_load.c・fle_vfs.c・tests/wall_wrp1.rs)、runtime の現在の答え(`fle_vfs.c` の `module_path` / `fle_imp_GetModuleFileNameW`、決定 0104・0118)、**Learn の GetModuleFileNameW のページ**(今回取得)、**Wine 10.0 のソース**(`dlls/kernelbase/loader.c` の `GetModuleFileNameW`、`dlls/ntdll/loader.c` の `LdrGetDllFullName`・`alloc_module`・`search_dll_file`。raw.githubusercontent.com の wine-10.0 タグ。UNOFFICIAL な基準)、gpu8 の cand1 の run の vfs-miss(`gpu8-results/0319/logs/cand1-*`)。分類: **(b) 契約の実装**(決定 0104 の `…\Shared\<export 名>` は出典の無い『選択』)。

### 19.1 契約(Learn と Wine 10)
- Learn: 『指定したモジュールを含むファイルの**完全修飾パス**』。hModule が NULL なら現在のプロセスの実行ファイル。『**返す文字列は、モジュールを読み込んだ時に指定された形式と同じ形式**を使う。したがって長い名前・短い名前のどちらにも、`\\?\` の接頭辞が付くこともある』『別のプロセスでは同じ DLL の名前の大文字小文字が違うことがある』。バッファが小さい時は nSize 文字(終端の NUL を含む)に切り詰め、戻り値は nSize、`ERROR_INSUFFICIENT_BUFFER`(runtime の `fle_imp_GetModuleFileNameW` は Learn どおり)。
- Wine 10: `GetModuleFileNameW` → `LdrGetDllFullName` → モジュールの `FullDllName`。`FullDllName` は `alloc_module` が **`nt_name`(`RtlDosPathNameToNtPathName_U_WithStatus` が作った NT パスの先頭 `\??\` を落とした物)**から作る。つまり『読み込みの時に渡された名前を GetFullPathName の規則で正規化した DOS パス』で、(i) 相対名は検索したディレクトリ + 名前の絶対パス(`search_dll_file`)、(ii) `/` は `\`、(iii) `.`・`..`・連続した区切りは畳まれ、(iv) 大文字小文字は渡された通り、(v) 入力が `\\?\C:\…` でも `\??\` を落とすので `C:\…` になる(Learn の『同じ形式』とは違う Wine の癖。FL は使わない)。切り詰めは Wine は NUL なしで nSize 文字(Learn と違う Wine の癖。runtime は Learn どおりで良い)。hModule == NULL は `ImageBaseAddress`。

### 19.2 mt3 の案は正しいか — **方向は正しい(b)。ただし記録する前に 3 つ直す必要がある**
正しい点: (1) Learn の『読み込み時に指定された形式』『大文字小文字は渡された通り』と一致する。(2) 『最初の要求だけ数える』は、Windows が同じファイルを 2 度目に読む時に同じモジュール(最初のパスのまま)を返すのと整合。(3) FL は plug-in フォルダを列挙して**完全修飾パスで** LoadLibraryW する(vfs-miss の `Plugins\Fruity\Effects\<Name>\<Name>_x64.dll` の形)ので、実運用の入力は mt3 の想定どおり。(4) `hModule == NULL`(FL64.exe)と `FLE_EXE_HANDLE`(= lifted FLEngine。`FLE_VIRTUAL_DLL_PATH`)の答えは mt3 の差分で**変わらない**(`module_path` の第 3 の分岐だけ変わる)。Delphi の DLL の `HInstance` はその DLL 自身の handle なので、実モジュールの経路に入る。

直す点(Learn / Wine との差):
1. **相対名・名前だけの要求を記録してはいけない**: path table は『tail が要求の末尾と一致し、その前が区切りか**無い**』で一致する(fle_realmod.h のコメント)ので、`LoadLibraryW(L"Fruity Wrapper_x64.dll")` のような名前だけも一致して、**完全修飾でないパスを GetModuleFileNameW に返す**(Learn は完全修飾パス、Wine は検索したディレクトリの絶対パス)。→ ドライブ付き(`X:\` か `X:/`)か UNC(`\\server\`)の絶対パスだけ記録する。それ以外は記録せず、今までの答えに戻る。
2. **`.`・`..`・連続した区切りを畳む**(Wine は畳む。Windows の `RtlGetFullPathName_U` も同じ)。畳めない/ルートより上に出る `..` は記録しない。今の差分は `/` → `\` だけ。
3. **接頭辞**: `\\?\C:\…` は Learn では『同じ形式』= そのまま返す。mt3 の ASCII のコピーはそのまま返すので Learn どおり(Wine の `\??\` を落とす癖には合わせない)。UNC `\\?\UNC\…` も同様。→ 追加の処理は要らないが**テストの期待に 1 行**(接頭辞つきがそのまま返ること)。
副作用・確認事項:
- **Balance / Filter / Delay 2 が見るパスが変わる**(今は `…\FL Studio 21\Shared\<export 名>`、新しくは `…\Plugins\Fruity\Effects\<Name>\<Name>_x64.dll`)。Export の WAV に効く可能性は読みでは低いが(音声の経路はパスを使わない)、**Delphi の RTL / VCL の初期化は `GetModuleFileNameW(HInstance)` を ModuleName として保存し、`ExtractFilePath` で自分のフォルダの資源〔Artwork・`Plugin.nfo`・プリセット〕を探すことがある**ので、新しいパスでは読み取りの先が変わる。volume に在るファイルは読まれて(実機と同じ)、無いものは新しい vfs-miss になる。→ **確認(安い)**: ci1 の cand1 の原 FLP の Export に mt3 の差分を足した build の sha256 が `94c9366687076a41…`(Balance のみ・Filter 入りと全バイト一致、ci1 の 10:2x)と**同じか**。同じなら Export への影響は 0。違えば差の最初のフレームを見る。
- gpu8 の cand1 の vfs-miss に **名前が空の探索**がある: `…\FL Studio 21\_x64.dll`(220 回)、`…\Fruityplug_x64.dll`、`.nfo`(187 回)。FL が plug-in の名前から `<名前>_x64.dll` / `<名前>.nfo` を組み立てる時に**名前が空**になっている形で、**原因は未確認**(GetModuleFileNameW が返すパスと関係するか、FL の plug-in database の走査の別の入力か)。mt3 の差分の前後で**この数が変わるか**を見ると、関係の有無が分かる(vfs-miss の集計だけで済む)。
- 返す文字列の長さ: mt3 の記録は 519 文字まで、`module_path` の呼び出し側の `char full[512]` は loaded path を**コピーせずポインタで返す**ので問題ない。`fle_imp_GetModuleFileNameA`/`W` の切り詰めは Learn どおり(NUL つきで nSize、`ERROR_INSUFFICIENT_BUFFER`)。

### 19.3 テストの追加(mt3 の `wall_wrp1.rs` に足すと良い物。runtime の差分と一緒に)
| 要求した名前 | 記録するか | GetModuleFileNameW の答え |
|---|---|---|
| `C:\Program Files\Image-Line\FL Studio 21\Plugins\Fruity\Generators\Fruity Wrapper\Fruity Wrapper_x64.dll` | する | そのまま |
| `C:/Program Files/…/Fruity Wrapper_x64.dll` | する | `/` → `\` |
| `C:\Program Files\Image-Line\FL Studio 21\Plugins\..\Plugins\Fruity\…\Fruity Wrapper_x64.dll` | する(畳む) | `.` / `..` を畳んだ形 |
| `Fruity Wrapper_x64.dll`(名前だけ) | **しない** | 今までの答え(`…\Shared\<export 名>`) |
| `Fruity\Generators\Fruity Wrapper\Fruity Wrapper_x64.dll`(相対) | **しない** | 同上 |
| `\\?\C:\Program Files\…`(接頭辞つき) | する | そのまま(Learn) |
| 2 度目に別の文字列で要求 | しない(最初が残る) | 最初のパス |
native の面で見る物: 私の harness は path table を持たないので足していない。mt3 の `wall_wrp1.rs` の `REALMOD_PRE` に上の 7 行を足すのが最短(私は書かない: mt3 の所有物)。

## 20. Filter の診断 build(1 本で足りるように)の中身を詰める — cand1 の既存ログの読み、追加で記録する観測、『957 フレーム』の正体(PM 11:4x の依頼。期限 12:15)

**先頭(何を実行したか)**: **読みだけ**(Chrome・Wine・wasm の新しい run は 0 本。native でも走らせていない)。読んだ物: ci1 の cand1 の ins1-lp の Export の engine-log(`lift/out-ci1/kit/out/export-cand1-ins1lp-r2/engine-log.log` 2,399 行と原 FLP の同じ log)、既存の WAV 4 本(Wine の ins1-lp ・ins1-bp・E4 の `e4-reg`、wasm の ins1-lp・ins1-bp)の数、FLP の playlist の tick。分類: **まだ (a)/(b)/参照のどれとも言えない**(原因は未特定。下は『何を見れば分かるか』の設計)。E4 の結果(opt1、PM も確認): Wine に minimal `.reg` を入れても素通し/無音の形は出ない → registry は否定。

### 20.1 (1) cand1 の既存ログで読めること — **ほとんど何も読めない**
- engine-log にある診断は mt3 の `g9_hook`(0x6800000〜0x7400000 = **Balance と Filter の image の合算**への間接呼び)だけ: 最初の 64 回、最初の 16 個の distinct な target、2 のべき乗回の個数。**最初の 64 回の target はすべて 0x68xxxxx(Balance)で、Filter の image〔0x7000000〜0x73B2000〕への呼びは 0 件**。つまり『Filter の image に Process が入ったか』はこのログからは言えない。合計は 262,144 回台(ins1-lp も原 FLP も同じ。べき乗の行が 2^18 で止まる)で、ins1-lp と原 FLP の差を言える粒度ではない。
- **sample 位置・時刻・API 名のある行が無い**(`vfs seed` / `real module` / `win create` / `vfs-miss` / `g9-diag` だけ)。DirectSound・Sleep・タイマー・latency に関わる host call の行は 0 件(`timer` を含む行は window class 名 `TSharedAnimTimerWindow` / `TTimerWindow` の create の 2 行だけ)。**『出ていない』のでなく『記録していない』**: host call の trace が無いので、4.778 s 付近の呼びは分からない。
- よって診断 build で足す(下)。

### 20.2 (2) 診断 build で記録する観測(`impl/diag-g9f-filter-calls.diff`、131 行に更新。`FLE_DIAG_G9F=1`、TEST の木のみ。build はしていない)
| 観測 | 何のため | 出力 |
|---|---|---|
| A. Filter の image への **Eff_Render 型**の呼び(Self, Source, Dest, Length の形: Length ≤ 16384、2 つの buffer が guest の heap の範囲)ごとに、呼び番号・target・a1(Self)・Source/Dest・Length・**累積フレーム**・呼びの**前の source の peak と後の dest の peak** | [210723,211680) の間に Process が呼ばれているか、呼ばれて 0 を書くのか(dst_peak = 0 で src_peak > 0)、呼ばれないのか(累積フレームが飛ぶ/この範囲の行が無い)、素通し(dst = src)か | `g9f: #n va … a1 … a2 … a8 … a9 … src_peak … dst_peak … cum_frames …`。累積 240,000 フレーム(約 5.4 s)まで |
| B. **Filter → 外**の呼び(返り番地が Filter の image の中で、target が外。= プラグインがホストに頼む呼び: Dispatcher の FHD_* など)の **target ごとの表**(最大 64、回数、最初と最後の累積フレーム) | 『ホストが返す値』がどれか(例: FHD_GetInBuffer / GetOutBuffer / GetMixingTime / PDC 関連)を累積フレームの区間と対応づける | 累積 1 / 205,000 / 211,000 / 213,000 / 240,000 フレームを越えた時に表を出す(`g9f: back-calls … target … xN first_cum … last_cum …`) |
| C. **オブジェクトの dump**: 最初の Eff_Render と、累積 210,000 を越えた最初の Eff_Render で、a1(プラグインのオブジェクト)の 192 byte と、その中の guest heap を指す最初の 3 つの pointer(HostTag = FL のスロット側の構造体を含む)の 128 byte を 16 進で | スロットの enable / bypass / ready に相当するフィールドが、wasm の 2 つの時点で変わるか(Wine との比較は後) | `g9f: object dump …` |
| D. Filter の image への**それ以外**の呼び(Dispatcher・ProcessParam・Idle など)の最初の 3,000 回の引数 | ID(rdx)から FPD_* を引ける | A と同じ行 |
読み方の決め手(予告):
1. [210723,211680) に対応する累積フレームの区間で Eff_Render が**呼ばれ、src_peak > 0 で dst_peak = 0** → プラグインが 0 を書いた(プラグイン側の状態 → B の表で、その区間の直前に何を頼んで何を受けたか、C の差で状態を探す)。
2. **呼ばれない**(累積フレームがその区間で進まない、または Length が 0/小さい) → FL のミキサーがスロットを呼ばない判断(FL の構造体のフィールド → C の差)。
3. 呼ばれ、dst_peak = src_peak(素通し) → wasm の出力が 0 になるのはプラグインの後(ミキサー側の合成)。
- **Wine で同じ点を見る方法(PM の許可後)**: opt1 の ELOG のフック(Wine の FL64 に入れている物。cpuid の UD2 shim と同じ型)を、Filter の Eff_Render の入口(wasm の診断で target VA が分かる。Wine の module の base と RVA = target − 0x7000000)に入れて Length と peak を出す。`WINEDEBUG=+relay` は Filter が叩く Windows API だけで、host 内部の呼びは見えない(重い割に決め手にならない)。
- 診断の足し方の注意: 1 本の build で足りるように、A〜D は同じ run で出る(上限つき)。出力は 1 MB 以内の見込み(A は 240,000 フレーム ÷ block 長の行)。

### 20.3 (3) 『957 フレーム』の正体 — Filter の block 長でも PDC の latency でもなく、**tick の端数**
- 数: wasm の変種が厳密に 0 の区間 [210723, 211680) の長さ 957。**211,680 = 1536 tick**(この project は PPQ 96・200 BPM で 1 tick = 3.125 ms = 137.8125 フレーム。1536 tick = 16 拍 = 4/4 の 4 小節ぶん。= 4.8000 s)。区間の始まり 210,723 は基準の最初の非ゼロのフレーム = playlist のチャネル 7 の clip の開始(4.778125 s = tick 1529.06)。**957 フレーム = 1536 − 1529.06 = 6.94 tick = 『clip が始まってから、次の 16 拍の境界までの端数』**。Filter の処理単位(block 長)や PDC の latency とは関係なく、project の時間の構造から決まる数。
- **両側が 1536 tick(211,680/211,681)で一致**: wasm の 0 の区間はそこで終わり、Wine の効果の開始もそこ。= その tick に FL 側の事件がある(多数の clip が一斉に始まる〔チャネル 0xb・0xc・0x23・0x2d・0x36・0x38・0x39・0x3b とパターン 5001/5002〕)。同じ型が 2 つ目の区間にもある: wasm の厳密な 0 の終わり 4,445,142 は tick 32,255.9 ≈ **32,256 tick(= 336 拍)**、チャネル 7 の 2 つ目の長い clip の終わりは tick 32,254。
- **PDC との関係**: Filter の latency が 0 でないなら『Trim PDC silence』ON の Export は先頭を詰めて全体を ずらすが、**Wine の変種は基準と最初の非ゼロのフレームが同じ(210,723)で、素通しの区間は bit 一致** → Wine の Filter の latency は 0 か、効いていない(読み)。wasm も『最初の効果のフレーム』が基準の最初の非ゼロと同じ。PDC の仮説は支持されない。
- **Export の長さはどれも整数 tick**: 基準 5,715,911 フレーム = 41,475 tick。変種との差は、Wine の ins1-lp が +102 tick(14,057 フレーム)、wasm が +101 tick(13,919)、ins1-bp は Wine が +3 tick(414)、wasm が +2 tick(276)。**Wine と wasm の差はどの変種でも『ちょうど 1 tick』(138 フレーム)**。FL は『出力が無音になってから 1 tick 単位で止まる』ので、減衰の尾の違い(フィルタの状態の違いの結果)が tick 1 つの差になっていると読める。= 長さの差は独立の問題でなく、フィルタの出力の違いの結果(読み)。
- 補正: 私の 10:0x の report で『4.8 s = tick 1536 = 3 小節目の頭』と書いたのは誤り(PPQ 96・200 BPM では 16 拍 = 4 小節ぶん = 5 小節目の頭)。数字(1536 tick)は正しい。

## 21. 『名前が空の探索』の読み(a)/(b)、Eff_Render の呼び元の静的な読み、診断 build 最終版 — RT-B の木に当たる形(PM 12:0x の依頼)

**先頭(何を実行したか)**: 読み(FLEngine_x64.dll〔binaries/win64〕の capstone による逆アセンブルと call 元の探索。Chrome・Wine・wasm の run は 0 本)+ **本物の cargo の native のテスト**(下)。**空の探索の原因は『FL 自身の通常の否定の探索』と読める(未確認。Wine の file trace で確かめる)**。Eff_Render の呼び元は**静的には 1 つに絞れていない**(下に正直に)。

### 21.1 gpu8 の cand1 の vfs-miss『名前が空の探索』(`…\FL Studio 21\_x64.dll`、`…\Fruityplug_x64.dll`、`.nfo`)
読みで分かったこと(FLEngine のアドレスは image の VA):
1. **`Fruityplug` と `_x64.dll` を組み立てるのは FL 自身の関数 `f_abeb30`**(Delphi の文字列連結。文字列 `Fruityplug`・`.dll`・`_x64` を持つ)。引数は (出力、**ディレクトリ**、**名前**、フラグ byte)。作るパスは `<ディレクトリ>\Fruityplug_x64.dll`(= FL の plug-in フォルダの『Fruityplug_x64.dll』という名前の規約のファイル)と `<ディレクトリ>\<名前>_x64.dll`。つまり **`Fruityplug_x64.dll` と `<名前>_x64.dll` は FL の『このフォルダに plug-in の DLL が有るか』の探索の組**で、どちらも `FileExists` 型の確認(GetFileAttributesW)。我々の runtime の返す値(GetModuleFileNameW など)から作られた名前ではない。呼び元は `f_abfa70`(リストを走査する仮想関数。vtable 0xabe8f0)と `f_132c680`(別の入口)。
2. **ディレクトリは FL の exe のフォルダ**: `f_652670`(FL 自身の『アプリのフォルダ』を一度だけ作って全域に保存する関数)が `GetModuleFileNameW(NULL, …)` を呼び、`ExtractFilePath` してキャッシュする(call 元 `0x6526b8`、hModule = 0 = exe)。runtime の答えは `…\FL Studio 21\FL64.exe`(決定 0118)で正しいので、`Program Files\Image-Line\FL Studio 21\` が出るのはそのため。**mt3 の 0315 の差分が変えるのは『実モジュールの hModule』の答えだけで、この hModule = 0 の経路は変えない**(読み)。
3. **名前が空になる**のは、リストの項目(名前の文字列)が空の時(`f_abfa70` の中で `list[i]` を取って `<ディレクトリ>` と連結する)で、実 Windows でも同じ関数が同じ探索をするはず。数が多い(o1 の run で 44 回、scroll の run で 220 回)のは、探索がリストの走査ごとに起きるため(時間に比例)と読める。**FL が最初から持つ空の項目への否定の探索で、結果は status 2 = 無い(FileExists が偽)= 実 Windows でも偽**。
4. **分類**: (a)(余計な処理)でも (b)(契約の不足)でもなく、**FL 自身の通常の動作の副産物(診断の vfs-miss が『無い』を全部数えるから見える)**と読める。実機との同一性は**未確認**(opt1 の P0〔起動時の file trace〕か E-series の Wine の file trace で、Wine の FL も `\_x64.dll` を探すか確認できる = 1 run)。mt3 の差分の前後でこの数が変わるかは RT-C の bundle で見る(変わらないはず: 経路が別)。
5. 関連: `Plugins\Fruity\Effects\<Name>\Fruityplug_x64.dll` と `<Name>_x64.dll`(Fruity Wrapper・Soundgoodizer・Parametric EQ 2・FLEX・Delay 2 の status 3)も同じ関数が作る探索で、**plug-in の種類 1 つにつき DLL の有無を 2 つ探す**(Fruityplug が先、`<Name>_x64.dll` が後)。status 3 = 途中のディレクトリ(plug-in のフォルダ)が仮想ボリュームに無い、が理由で、G9 の module を足すと(フォルダと DLL を volume に置くと)この探索は成功に変わる = 期待どおり。

### 21.2 Eff_Render の呼び元を静的に読む試み(結果: **絞れていない**。正直に)
- plug-in の vtable の並び(FL SDK の TFruityPlug。Delphi のクラスで 8 バイトずつ): 0x00 DestroyObject、0x08 Dispatcher、0x10 Idle、0x18 SaveRestoreState、0x20 GetName、0x28 ProcessEvent、0x30 ProcessParam、**0x38 Eff_Render**、0x40 Gen_Render。FLEngine で『`call [reg+0x38]` の直前に rdx・r8・r9d を作る』形は **114 の関数**にあり、同じ関数が 0x28・0x30・0x38・0x40 の 4 つを全部呼ぶ関数が約 10 個(`f_f09150`・`f_c93549`〔`f_c93580` を含む〕・`f_bf7090`・`f_b38ef0`・`f_908d00` ほか)。
- 最有力の 1 つ `f_c93580`(`call [rax+0x38]` at 0xc93693): スロットの配列を `[obj+0xc0]`(範囲検査 `f_47a250` つき)から引き、項目の `[item+0x64]`(plug-in の側の object)の vtable の +0x38 を **(rdx = [msg+8]、r8 = [msg+0x18]、r9d = esi〔-1。msg の種別 0x103 の時だけ 0〕)**で呼ぶ。**r9d が -1/0 で、長さではない**ので、これは Eff_Render ではなく『メッセージ型の呼び』(Dispatcher/ProcessEvent の仲間)の可能性が高い。**Eff_Render の呼び元は、診断 A の `caller` 列(Filter への呼びの返り番地 = FL 側の呼び元の VA)で 1 回で確定する**(静的には他の約 100 か所と区別できない)。
- **診断 C の dump の対象は合っている**: Filter の `CreatePlugInstance(Host rcx, Tag rdx)` は基底の構築子へ Tag を渡し、plug-in の object の +0x10 に `TFruityPlugInfo` の組(SDKVersion = 1、名前の pointer、…)を書く(読み: Filter の 0x72cbb10)。TFruityPlug の先頭は [+0 vtable, +8 HostTag, +0x10 Info, …] なので、**a1 の 192 バイト(+0x08 の HostTag と Host の pointer)と、guest の heap を指す最初の 3 pointer の 256 バイト**に、FL が渡した『スロットの tag』(= FL 側のスロットの構造体を指すなら pointer)が入る。HostTag が整数の id なら pointer として追えない(その場合は dump の差が出ない)ので、**診断 A の `caller` で FL の関数が分かった後に、その関数の `[item+…]` の読みで構造体のフィールドを決めて dump を足す**のが二段目(runtime のみの増分 build 1 本)。
- 『clip の開始のたびにミキサーのスロットが再構築される経路』の有無: **静的には確認できていない**(上の約 10 の関数のうち、スロットの enable・再構築を扱う物を特定するには A の caller が先)。

### 21.3 診断 build の最終版 `impl/diag-g9f-filter-calls.diff`(123 行。RT-B の木に当たる)と、本物の native のテスト
- 当て先: **mt3 の tree-rtb と tree-rtb-build(`FLE_DIAG_*` が getenv の形)の両方に `patch --dry-run` で clean(FAILED 0、offset なし)**。診断の読みは `FLE_TEST_GETENV("FLE_DIAG_G9F")`(tree 側の形)。mt3 の `rtb-tree.sh diag` の `sed 's/FLE_TEST_GETENV("FLE_DIAG_/getenv("FLE_DIAG_/'` が fle_callback.c を含むので、**そのまま getenv に変換される**(build の木で当たるかは、tree-rtb-build が既に変換済みの形でも dry-run で確認した)。plain-C の守り(callback.rs の `long`・emscripten・asm・malloc ほかの禁止語、標準の PRIx64 / PRIu64 だけ)を満たす(コメントを剥がして確認)。
- **変更点(10 分前の版から)**: 呼び元の VA(`caller`)を A の行に足した。Workers の共有状態を spin lock(`__atomic_exchange_n`)で守る(real-thread 版では複数 Worker から呼ばれる)。出力の上限を 20,000 行に固定。B の表は 5 つの区切り(1・205,000・211,000・213,000・240,000 累積フレーム)で出す。C の pointer 先の dump を 256 バイトに。
- **本物の cargo の native のテスト**(tree-rtb のコピーに『この診断』を当てた状態。`--only '(callback|window|monitor|wall_r|wall_ref1|wall_fr|present2_uncover|gpu7_ring_poll|wall_tbl1|wall_ia1w|sysinfo|vfs|time|env|wall_ia2|dwm_timing)'`、--jobs 2): **36 executable、702 passed、0 failed**。`emcc -fsyntax-only -Wall -Wextra -pthread -msimd128 -matomics -mbulk-memory` を GIL なしと `-DFLE_GIL -DFLE_TLS_MORE` の両方で警告 0。
### 21.4 診断が出す量(Export 1 本 = 約 5.73M フレーム、約 6 分)
- 上限: A と D を合わせて **20,000 行**(1 行 約 190 B)= **最大 3.8 MB**。A は累積 240,000 フレームで打ち切る: block 長 64〜1024 フレームの範囲で 234〜3,750 行(0.04〜0.71 MB)。D は 3,000 行まで。B は 5 つの区切りで 64 行以下の表(各 約 7 KB)= 35 KB 以下。C は 2 回の dump = 約 8 KB。**合計は最悪でも約 4 MB**で、wasm の stderr は Export の log(cand1 の engine-log は 2,399 行・約 0.3 MB)に足されても Export の長さ・速度に影響しない量(読み。FL の mixer が呼ぶ度に fprintf するのは最初の 240,000 フレームの間だけで、以降は呼びごとの判定のみ)。ただし **A の `g9f_peak` は 1 呼びごとに Length × 2 個の float を読む**(最大 16384 × 2 × 2 回 = 約 65k 回の読み/呼び)ので、240,000 フレームの間は遅くなる(約 4 秒分の音声の間だけ。以降は `blk` の判定と lock のみ)。
### 21.5 出力の読み方 — `tools/a22-diag-read.py` の節 10(A: 前後の peak と ZERO/THROUGH/FILTERED/SILENT-IN の区間、210723〜211680 の近傍の呼び、B: 表の増分と NEW の target、C: dump の差、D)
固定の見本(手書きのテストデータ。**実ログではない**)で出力の形を確認済み。空のログでは 1 行で終わる(壊れない)。

## 22. Eff_Render の呼び元は FL の『Smart disable』の経路に見える — 二段目の診断の候補(どの構造体のどのオフセット)と、Wine 側の観測の手順書(PM 12:1x の依頼。期限 12:45。読み)

**先頭(何を実行したか)**: **読みだけ**(FLEngine_x64.dll の capstone による逆アセンブルと call 元の探索。wasm・Wine・Chrome の run は 0 本)。§21.2 で『静的には絞れていない』と書いた呼び元が、**このあと別の読み方(FL の自分のログ文字列『Mixer threading map rebuilt』からの逆引き)で 1 つの有力な候補に絞れた**: 下の事実はすべて**読みであり未観測**。診断 A の `caller` 列が `0xcf2df9`(= `f_cf2d00` の `call [rbx+0x38]` の返り番地)なら確定。

### 22.1 読めたこと(FLEngine の VA)
- `f_cf2a90`(タスクの 1 本の処理)→ `f_cf2f90`(『このスロットを今回走らせるか』)→ `f_cf2d00`(**`call [rbx+0x38]` at 0xcf2df6 = スロットの plug-in の vtable +0x38 を `rdx = [task+8]`〔Source〕、`r8 = [task+0x54]`〔Dest〕、`r9d = [task+0x48]`〔Length〕で呼ぶ**)。この呼びは長さを r9d に取るので**Eff_Render の形**(§21.2 の f_c93580 の呼びは r9d = -1/0 のメッセージ型で、これではない)。呼び元の連鎖は `f_cf2d00` ← `f_cf2a90`(call 元 0xcf2b0f)。plug-in の object は `slot = [task+0x38]` → `f_1331340(slot, &holder)` → `holder` の vtable +0x20(`GetPlugin`相当)の答え。
- **`f_cf2f90` は FL の『Smart disable』そのもの**に読める: スロット `s = [task+0x38]` について、`s+0xf7`(byte)が 0 なら『走らせる』(戻り 0)。非 0 の時、挿入 index(`s+0x130`)が 0 でなければ、source の buffer(`[task+8]`、`[task+0x48] × 2` 個の float)を **`f_6a2180` で『全部の絶対値が閾値以下か』を調べ**(閾値は `xmm2 = 1.5849e-5` ≒ −96 dB、整数の絶対値の比較)、**無音なら、スロットの無音カウンタ `s+0xa0` が大域の限界 `*[0x196ec10]`(ポインタの置き場 `[0x15e4e78]`)以上になったら『走らせない』(戻り 1)、未満ならカウンタに Length を足して走らせる。無音でなければカウンタを 0 に戻し、フラグ byte を 1 にして走らせる**。つまり『無音が一定フレーム続いたスロットを止め、音が来たら再開する』経路。
- **限界の値**: `f_12c6aa0`(call 元 0x11acc59 と 0x135d99c。音声エンジンの初期化/設定の側)が `*[0x15e4e78] = *[0x15e1f40] << 2` を書く(0x12c6b7b〜0x12c6b8a)。`[0x15e1f40]` は同じ関数が `cvtsi2sd` で倍精度にしてから別の関数へ渡す整数(**サンプルレートに見える**。読み。`/5` の割り算も同じ変数から)。= **限界 ≒ サンプルレート × 4 フレーム(44.1 kHz なら 4.0 s)**。この project では Filter の入力(Insert 1 の Balance の後)は 0 〜 4.778 s の間無音なので、**4.0 s(frame 176,400)で Smart disable が掛かり、4.778 s(frame 210,723)に音が来る**。これは今回の差の位置と一致する(読み)。
- **Wine と wasm で見える差の候補(診断 A で決まる)**: 音が来た最初の 957 フレームで Wine は入力をそのまま通し(基準と bit 一致)、wasm は厳密な 0。Smart disable で『走らせない』時の出力は FL 側の buffer の扱い(in-place なら入力のまま、別 buffer なら以前の内容)に依る。→ (i) wasm では Filter が呼ばれて 0 を書く(診断 A: src_peak > 0、dst_peak = 0、caller 0xcf2df9)、(ii) 呼ばれず(Smart disable の戻り 1)出力 buffer が 0 のまま(診断 A に行が無く、累積フレームがこの区間で進まない)、(iii) 呼ばれて素通し。**(ii) なら、その buffer の初期値(新しく確保されたメモリが 0 か、Windows のヒープの古い内容か)や、Smart disable の設定〔`SmartDisBox` の既定。DFM に `Checked` が無い = False、Wine の設定ファイルの値は '1'〕の差が候補**。
- 補足(registry): `SmartDisBox` の DFM には `Checked` の property が**無い**(Caption・Hint『May increase CPU performance』・Options・Spacing のみ。直前の `MultithreadFXBox` の `Checked = True` とは別。私の §10 は Multithread の 2 つの `Checked = True` を読んだが、**SmartDisBox は既定が偽**)。Wine の参照の user.reg に `MIDIForm` の key が無い(§18)ので、**両側とも『保存された値なし』= FL のコードが決める既定**で、E4 の否定とは矛盾しない。実行時にどちらが有効かは、`f_cf2f90` の `[s+0xf7]` の値(下)で直接見える。

### 22.2 二段目の診断(caller が `0xcf2df9` と確定した後に足す。runtime のみの増分 build 1 本。**どの構造体のどのオフセットを見るか**)
`f_cf2d00` の入口(task = rcx)と、その中の `call [rbx+0x38]` の前後で次を出す(TEST の診断、`FLE_DIAG_G9F`):
| 対象 | 見る物 | 意味 |
|---|---|---|
| task(`rcx`) | `+0x08` Source ptr、`+0x20`、`+0x28`(int。0 に落とされる)、`+0x38` スロット ptr、`+0x48` Length、`+0x54` Dest ptr(8 バイト、unaligned) | 呼びの引数の元。Source == Dest か(in-place か) |
| slot(`[task+0x38]`) | `+0x64`(int。≥ 0 = 使用中、`f_132f780`)、**`+0xa0` 無音カウンタ**、**`+0xf7` Smart disable のフラグ byte**、`+0xfc`(byte)、`+0x130`(挿入 index)、`+0x88` | Smart disable の状態の遷移(カウンタが 176,400 に達したか、いつ 0 に戻ったか、`+0xf7` の値) |
| 挿入レコード(大域の配列 `[…] + index × 0x28f × 8` = 0x1478 byte) | `+0x148`(dword。`≥ 2` で `f_cf2a90` が『無音検査あり』にする)、`+0x4c`(クリティカルセクション)、`+0x370`、`+0x380`(byte)、スロット表 `+0x134c + 8k`(k < 10) | 『この挿入に Smart disable が掛かるか』と、スロットの並び |
| 大域 | `*[0x15e4e78]` の指す dword(**限界**)、`*[0x15e1f40]` の指す dword(**サンプルレートに見える**) | 限界が `rate << 2` か、wasm と Wine の値が同じか |
| 戻り | `f_cf2f90` の戻り(0 走らせる / 1 走らせない)と `[rbp+0x4f]`(検査あり) | (ii) の直接の証拠 |
+ 同じ区間で Dest の peak(診断 A の dst_peak)。**Wine でも同じ点を見られる**(22.3)ので、同じ表を両側で作って比べる。

### 22.3 Wine 側の同じ点の観測の手順書(opt1 の ELOG のフック。**PM の許可後**。Wine の窓が要る)
- 仕組み: opt1 の `stub/wintrust_hook.c`(INT3 + 例外ハンドラ。module の RVA の関数の先頭 1 バイトを 0xCC にして、入口で ELOG 行を出し、必要なら戻りのパッドで出口の行も出す。`K_PUSH` など先頭命令を emulate)。**FLEngine_x64.dll は Wine でも wasm でも同じ image**なので、VA は共通: **`f_cf2d00`(RVA = 0xcf2d00 − 0x400000 = 0x8f2d00、先頭命令 `push rbp` = `K_PUSH`)**に入口と出口のフックを置けば、Filter の base に依らず『Filter(や他の plug-in)を呼ぶ側』が観測できる。
- 出す物(ELOG 行の案、行の形式は 0308b §7 の規約): 入口 `ELOG cf2d00 <seq> task=<rcx> slot=<[rcx+0x38]> len=<[rcx+0x48]> src=<[rcx+8]> dst=<[rcx+0x54]> cnt=<[slot+0xa0]> flag=<byte [slot+0xf7]> ins=<[slot+0x130]> t=<ms>`、出口 `ELOG cf2d00x <seq> cnt=… flag=… dstpeak=<float の peak>`(peak は Source/Dest の length × 2 個の float の絶対値の最大。opt1 の `fnv_buf` と同じ型の `ReadProcessMemory` の読み)。Filter の slot だけに絞る(slot の `+0x64` の値は plug-in の ID でなく slot の番号なので、**Filter の Insert 1 の slot 1 = `ins = 1` かつ slot index 1**で絞る。`+0x130` が挿入 index)。件数の上限: 最初の 40 秒分(累積 Length)か 20,000 行。
- 限界の値(`*[0x15e4e78]`)と `[0x15e1f40]` の指す値は、Wine の FL のメモリを ELOG の最初の呼びで 1 回読んで出す(`ELOG globals limit=… rate=…`)。
- 順序: wasm の診断 A の結果(caller と区間の振る舞い)を先に読み、**caller が `0xcf2df9` でなければこの手順書の VA を直す**。Wine の run は『ins1-lp の原本の prefix-e2〔config A〕+ ELOG shim』で 1 本(約 3 分)。比較は『wasm の診断 A + 二段目の表』と Wine の ELOG の行を、累積フレーム(= Σ len)で突き合わせる。


## 23. 子窓に WM_PAINT が届かない — gpu1 の RT-B のログ(Balance・Settings のスクロール)との対応、契約(Wine 10 / Learn)との対照、直しの差分 `0319-show-descendants-paint.diff`(PM 12:2x〜12:5x。期限 12:50→12:55)

**先頭(何を実行したか)**: 読み(gpu8 の `0331/logs/rtb-balance` と `rtb-scroll` の実 Chrome のログ、Wine 10.0 の `server/window.c`〔raw.githubusercontent の wine-10.0〕と `dlls/win32u/window.c`、`lift/runtime/fle_win.c`)+ **native の harness**(面 R。本物の runtime C)+ **本物の cargo の native のテスト**(HEAD の複製に私の全差分を当てた全 215 本、結果は下)。**実エンジン・wasm・Chrome は私は走らせていない**。

### 23.1 (1) ログとの対応 — 直しが当たる状況かどうか
gpu8 の `rtb-balance/log-10s.log` の行(行番号つき):
| lid | クラス | create(行、style) | WS_VISIBLE が付いた行 | 付けた API | その窓への ShowWindow | WM_PAINT |
|---|---|---|---|---|---|---|
| 171 TPluginForm(親 15) | 44 行、0x46000000(WS_VISIBLE なし) | 54 行 0→1(`show lid 171 … ShowWindow cmd 1 was 0` が 53 行) | **ShowWindow(この窓に 1 回)** | 16 回 |
| 174 TVectorSheet(親 171) | 48 行、0x46000000 | 55 行 0→1、style 0x56000000 | **無し(ShowWindow の呼びが 0)。by の欄は走査が拾った API 名なので `ShowWindow` と出るが、この窓への呼びではない** | 0 |
| 175 TWPControlForm(親 174、呼び元 0x6973abe = プラグイン自身) | 50 行、0x46000000 | 56 行 0→1、style 0x54000000 | 無し | 0 |
| 176 TNewCaption(親 171) | 51 行 | 57 行 | 無し | 0 |
| 177 TWinControlProxy(親 171) | 52 行 | 58 行 | 無し | 0 |
- **create の style は 0x46000000 = WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN で WS_VISIBLE(0x10000000)が無い**(0x44000000 の 173・177 も同じ)。**それなのに 55〜58 行で 0→1 になる**。ShowWindow が呼ばれたのは 171 だけ(53 行、親 15 の下で、15 は可視)。→ 174・175・176・177 は**『作られた時は不可視で、親 171 が ShowWindow された時に初めて可視になった』窓**。これは **直しが当たる状況**: 174〜177 は作られた後(52〜54 行の間)に `WS_VISIBLE` を付けた(`SetWindowPos(SWP_SHOWWINDOW)` か `SetWindowLongPtrW(GWL_STYLE)`。create の style から見て **create の後の style の変更**。診断の `vis` の `by` は走査した API 名で、付けた API ではない。**どの API かは確定できていない**)。**付いた時点では親 171 が不可視**なので、runtime の `upd_add`(`fle_win.c`: `if (!is_visible(i)) return;` = S-0247-hidden、Wine 10.0 の実測に基づく)で**更新領域は作られない**。その後 171 が ShowWindow(53 行)され、`fle_imp_ShowWindow` は **171 にだけ** `upd_set_full` を掛ける(HEAD の `fle_win.c` の ShowWindow の最後:`upd_set_full(i); tab[i].upd_erase = 1;`)。174〜177 は可視になっても更新領域が空のまま → **WM_PAINT が配られない**(`paint_pending_window` は更新領域が空の窓を飛ばす)。**ログの 171 の 16 回の WM_PAINT・174〜177 の 0 回と一致**。
- Wine 10.0 では: `server/window.c` の `set_window_pos`(1909〜)が、表示された窓の露出領域を `redraw_window(win, exposed_rgn, 1, RDW_INVALIDATE | RDW_ERASE | RDW_FRAME | RDW_ALLCHILDREN)`(2059・2084 行)に渡す。`redraw_window`(1671〜1692 行)は子窓へ再帰し、止まるのは `(win->style & WS_CLIPCHILDREN) && !(flags & RDW_ALLCHILDREN)` と、子が `WS_VISIBLE` でない場合だけ(`ALLCHILDREN` が立っているので、親が WS_CLIPCHILDREN でも子の更新領域は作られる)。**= Wine では 171 が表示された時、可視な子孫(174〜177)全部に全面の更新領域ができ、WM_PAINT が届く**。runtime の欠けているのはこの再帰。
- **O9(Settings の TMIDIForm の子)にも同じ仮説が当たるか: 当たらない**。`rtb-scroll/log-all.log` で、TMIDIForm(lid 21、style 0x86000000)の子 22・23・135・138〜143 は **WM_PAINT を受けている**(lid 138 TVectorSheet が 32 回、23 が 1 回など)。同ログで『create の style に WS_VISIBLE が無いのに可視になり、その窓への ShowWindow が 0』の窓は 83 あるが、**ほとんどが WM_PAINT を 1〜4 回受けている**(例: lid 15 TWPControl 4 回、48〜51 各 1 回)= **主窓(lid 13)の ShowWindow は、子の上へ再帰する別の経路で子孫に更新領域が作られている**(主窓の表示は create の時の WS_VISIBLE 経路ではなく、`SetWindowPos`/`RedrawWindow(RDW_ALLCHILDREN)`/`UpdateWindow` などの別経路で全体が invalid になっていると読める。**この読みは未確認**)。**O9 の崩れは『更新領域が作られない』ではなく、スクロール後の面**(`ScrollWindowEx` flags 0 → FL 自身が子を `SetWindowPos`/`MoveWindow` で動かす)なので、**この直しとは別の穴**(面 N の読み、§15)。
- **TWPControlForm の親 174 TVectorSheet も 0 なので**、PM の問い『`TVectorSheet` 以下の子窓は更新領域が作られない』か『配達が 171 の直下に限られる』か → **後者に近いが配達(`paint_pending_window`)は z 順の全窓を見る(171 に限らない)ので、原因は『配達』ではなく『更新領域が作られない』**。`paint_pending_window` は `live && owner == t && !in_paint && 更新領域あり && is_visible && !iconic` の窓をそのまま返す(`fle_win.c`)。

### 23.2 契約(Learn / Wine)と runtime の対照 — (b) 契約の不足
- **Learn**: BeginPaint『update region is set by the system after sizing, moving, creating, scrolling, or any other operation that affects the client area』、ShowWindow は表示で『まだ描かれていない領域』を持たせる(Learn の文面は子孫には触れない)。**Wine 10(UNOFFICIAL)** が子孫へ再帰する(上)。
- runtime(HEAD の `fle_win.c`): 窓が可視になった時に更新領域を作るのは (i) `fle_imp_ShowWindow`(`upd_set_full(i)` を **その窓だけ**)、(ii) `apply_pos` の `sized && is_visible(i)`(**SWP_SHOWWINDOW だけで寸法が変わらないなら作らない**)、(iii) `SetParent`。**子孫を見ない・SetWindowPos(SWP_SHOWWINDOW) では更新領域を作らない**。= **(b) 契約の不足(Wine の `redraw_window(RDW_ALLCHILDREN)` の子孫への再帰が無い。FL のクラス名や窓の種類の条件は使っていない)**。(a)(余計な処理)ではない。

### 23.3 直し `impl/0319-show-descendants-paint.diff`(fle_win.c、37 行。HEAD に clean。tree-rtb・tree-rtb-build には offset 9 で当たる)
1. 新しい関数 `upd_show_descendants(i)`: 窓 i の**可視な子孫すべて**(`live`・`WS_VISIBLE`・非 iconic・大きさあり)に `upd_set_full` と `upd_erase = 1` を、**再帰**で(Wine の `redraw_window(RDW_ALLCHILDREN)`。親の `WS_CLIPCHILDREN` の有無に依らない)。
2. `fle_imp_ShowWindow` の『不可視 → 可視』で `upd_set_full(i)` の直後に呼ぶ。
3. `apply_pos` で `SWP_SHOWWINDOW` かつ `vis0 == 0`(呼びの開始時に不可視)かつ `NOREDRAW` でなく、窓が今可視なら、その窓の `upd_set_full` と `upd_show_descendants`(**今までは SWP_SHOWWINDOW だけでは更新領域を作らなかった**)。
- 分岐は窓の可視・親子・`SWP_NOREDRAW` だけ。クラス名・窓の種類・位置の条件は無い。

### 23.4 検証(本物)
- **native の harness**(`tools/scroll0319/scroll_0319.c` の面 R。5 通り: 孫が親の不可視の間に `SetWindowPos(SWP_SHOWWINDOW)` / `ShowWindow` で可視になる、親が plain / WS_CLIPCHILDREN、親を `ShowWindow` / `SetWindowPos(SWP_SHOWWINDOW)` で表示): **修正前は 5 通りとも孫に更新領域が無く『NOT painted』(`tables/0319-scroll-harness-R-show-descendants-before.out` に 4 通り + R5 は面の追加後)、修正後は 5 通りとも孫が WM_PAINT を受けて赤く塗られる**(`…-after.out`)。
- **permanent な native のテスト**: `harness/alpha3/upd_region.c`(`alpha3_updregion` の本体)に『show-descendants』の節を足した(`impl/0319-show-descendants-paint-tests.diff`)。**修正なしの fle_win.c では 52 件中 6 件が FAIL(『表示された h の可視な子 g と孫 gg が全面の更新領域を持つ』と『`SetWindowPos(SWP_SHOWWINDOW)` で h を出した時に h・g・gg が全面を持つ』が plain と WS_CLIPCHILDREN の各 3 件)、修正ありでは 52 件中 0 件 FAIL**(13:0x に実測: 同じ `upd_region.c` と同じ他の .o を、HEAD から私の diff を `patch -R` で外した `fle_win.c`〔`upd_show_descendants` 0 件〕と、head4 の `fle_win.c` の両方にリンクして走らせた)。**以前の本節・報告に書いた『50 件・4 つ FAIL』は、`SetWindowPos(SWP_SHOWWINDOW)` の check を足す前の数字で、今のテストの数ではない**。
- **本物の cargo の native 全体**(HEAD の `git archive` の複製 + 私の差分〔callback-stack-limit+tests、scrollwindow-model、time-env、run-values-and-scroll-model-tests、show-descendants-paint〕+ 上のテスト): 結果は下(§23.6)。

### 23.5 副作用
- **コスト**: 可視な子孫の**全面**を無効化する。Wine でも同じ(`redraw_window` が子孫に全面を足す)ので『Windows と同じ量』。FL では『ウィンドウを開く/見せる』時の 1 回だけ(Balance の form で 171〜177 の 6 窓)。ただし**主窓(lid 13)の ShowWindow は子孫が 100 以上**(rtb-scroll のログで 125 の子窓)なので、**起動時の 1 回の全面再描画が『全子孫への WM_PAINT』になる**。FL は元々 1,000 回級の WM_PAINT(lid 13 が 2048 回)を受けているので量の桁は変わらない見込みだが、**P2(起動と UI の fps)への影響は未測定**。
- **Wine との差**: (1) Wine は子の `window_rect` と redraw 領域の交差を取り、親の region に入る子だけ再帰する(`rect_in_region`)。私の版は『可視な子はすべて』で、親の可視領域外に出ている子にも全面を足す(子の `upd_set_full` は子自身の大きさなので、親の外にはみ出した分は後の描画でクリップされるだけ)。(2) Wine は **WS_CLIPCHILDREN の親には、親自身の更新領域から子の領域を引く**(`crop_children_update_region`)。runtime は別の機構(`fle_win_dc_region` の可視領域・`upd_children`)で、**親の更新領域には子の領域が残る**ことがある(親の WM_PAINT の rcPaint が子の上に及ぶ。親の DC のクリップが子を除くので、描画は子に及ばない)。(3) 子孫の `upd_erase = 1`: Wine の `RDW_ERASE` と同じ。

### 23.6 全 215 本の結果(実測。`/home/ubuntu/work/alpha22w/cbtest/run14-head4-show.log`、12:57 完了)
- 木: `git archive HEAD`(d78f3c98)の複製 `cbtest/head4` に、callback-stack-limit(+tests)、scrollwindow-model、time-env-run-values、run-values-and-scroll-model-tests、**show-descendants-paint(+tests)** を当てた物。`fle_win.c` に当てた行(28 行)と `upd_region.c` に足した行(38 行)が head4 に全部入っていることを 13:0x に確認(欠け 0)。
- コマンド: `bash lift/harness/test.sh full --jobs N --fresh --no-record`、`FLE_INPUT_ROOT=/home/ubuntu/work/signature-studio-develop`、私用の `CARGO_TARGET_DIR`。
- 結果: **215 test executables(+ doc-tests。results=219)、passed=2417、failed=1、ignored=1**。failed の 1 件は `fe-lift/dwm_timing: the_import_row_is_one_appended_row_after_the_rows_of_the_earlier_decisions_and_the_runtime_defines_it`(行数 211 対 210。mt3 が更新した `ship6` の pin で、私の show-descendants とは無関係。show-descendants を除いた run9〔12 時前〕でも同じ 1 件)。SKIPPED 15 行は入力 DLL が無い物(従前どおり)。
- `alpha3_updregion`: passed=1(中の check は 52 件、failed 0)。


## 24. 変種 FLP の Insert 1 / Insert 2 の表 — 『変種に Delay 2 の効果が出ない』の原因(PM 12:4x の依頼)

**先頭(何を実行したか)**: **読みだけ**(原 FLP `serina1-serum2.flp` と `variant-flp/ins1-lp.flp` を `flp_model` で読み、ci1 の RT-B・cand1 の WAV と opt1 の Wine の WAV を numpy で比べた。実エンジン・Wine・Chrome の run は 0 本)。

### 24.1 結論(先に): **変種 FLP は Delay 2 を置き換えていない。Delay 2 は Insert 2 の slot 3 にあり、変種で変わったのは Insert 1 に slot 1 の Filter が 6 イベント足されただけ**
- 原 FLP の Delay 2 は **Insert 2 の slot 3 の 1 つだけ**(`(insert 2, slot 3)`)。変種でも同じ(`(2, 3)`)。**Insert 2 のイベント 48 個は原と変種で完全に同一(byte 一致)**、**Insert 1 以外のすべての Insert も byte 一致**(Delay 2 の slot のイベント 201・212・203・155・128・41・213・98 の 8 つも byte 一致、状態 `80ffffff45000000…`)。
- Insert 1 の変更: 原 = `(1, 0, Fruity Balance)` のみ(27 イベント)→ 変種 = `(1, 0, Fruity Balance)` + `(1, 1, Fruity Filter)`(33 イベント = +6、私が足した Filter の 6 イベント)。Insert 1 の非 slot のイベント(235 = 有効、165/166 = ルーティング、49 など)は byte 一致。
- したがって『変種で Delay 2 が外れた/置き換わった』ではない。

### 24.2 では『なぜ変種の RT-B の Export に Delay 2 の効果が出ない(cand1 と 3 フレームしか違わない)のか』— **信号の経路の読み(確認済みの数値つき)**
- 数値(ci1 の RT-B と cand1 の WAV を私が直接読んだ。ci1 の値と一致): RT-B 原 FLP 対 cand1 原 FLP は **4,136,261 フレームが違う(RMS 1040.5284 = Wine の elog17 − elog16 の差の RMS 1040.5284 と同じ値)**。**RT-B 原 FLP 対 Wine elog17(Delay 2 あり)は 19 フレーム ±1 LSB(RMS 0.0013)** = **wasm の Delay 2 は Wine と一致している(Delay 2 の lift は正しい)**。一方 RT-B ins1-lp 対 cand1 ins1-lp は 3 フレーム ±1 LSB(RMS 0.0005)。
- **効果が 10 秒ごとに全域で 0**: 『RT-B − cand1』の RMS を原の組と ins1-lp の組で区間ごとに比べると、原の組は 0〜10 s 765.6、10〜30 s 892.3、30〜60 s 948.1、60〜100 s 1444.5、100〜130 s 523.4 に対し、**ins1-lp の組は全区間で 0.0**。原の組の効果は **frame 220,666(5.004 s)から始まる**。
- **読み(未確認)**: Delay 2 の slot(Insert 2 slot 3)の**効果は『Insert 2 の出力 → Insert 1 以外のどこかへ流れる』経路で Export に載る**。ins1-lp の Filter は **Insert 1 の slot 1**(= Master → Insert 1 → …の Balance の後)にあり、Insert 1 の後を通るすべての信号に掛かる。Wine の elog17 は『Delay 2 あり・Filter なし』、elog16-ins1-lp は『Delay 2 なし・Filter あり』で、**私の変種では Delay 2 と Filter の両方が有効になる組は Wine にも wasm にも無い**。RT-B ins1-lp で Delay 2 の差が 0 になる理由は、**Insert 1 の Filter(強い低域通過、cutoff 128)が Insert 1 を通る信号の 100% に掛かり、Delay 2(Insert 2)が足す差〔RMS 1040〕を Insert 1 の後で取り除く(低域通過より上の帯域の差が消える。Delay 2 の効果は主に中高域)**ためと読める。**この読みは検証していない**(Filter ありの組で Delay 2 の有無だけを変えた WAV が必要)。**もう 1 つの読み**: Wine の elog16-ins1-lp も『Delay 2 の DLL を隠した prefix』で取ったので、両側とも Delay 2 を使っていない run と、wasm の RT-B は Delay 2 を実モジュールとして走らせた run で、**比較の意味は同じではない**(RT-B の ins1-lp に Delay 2 が入っていても出力が cand1 と同じなので、Filter の比較には影響しない = ci1 の『Filter の比較として見てよい』のとおり)。
- 『今後 Delay 2 あり(elog17 系)で Filter を比べる』には: **Insert 2 の経路が Insert 1 を通らない信号を作る**ので、変種 FLP の作り直しは**必須ではない**が、(i) Wine 側を『Delay 2 あり + ins1-lp』の prefix(opt1 の R3 の構成 + 変種 FLP)で取り直し、(ii) wasm は RT-B の ins1-lp の Export(既にある)と突き合わせる、という 1 本の Wine run で足りる。この比較で『Filter の差が Delay 2 の有無で変わるか』も分かる。**その Wine run を頼むなら、元の変種 FLP のままでよい**。

## 25. Filter の診断(RT-D の ins1-lp の Export の log)を読む前提 — どの行・値をどう読むか、Eff_Render 型の呼びが出ない場合
**読みだけ。出たら `tools/a22-diag-read.py <log> --max-rows 40` の節 10 を見る。見る順序と判定(出る前に固定する)**:
1. **まず `g9f:` の行が出ているか**(`grep -c 'g9f:'`)。0 行なら `FLE_DIAG_G9F=1` が engine の環境に渡っていない(manifest の `engine.env`)か、bundle が診断入りでない。→ 結論を出さず ci1 に確認。
2. **節 10 の A の『Eff_Render-shaped calls printed』が 0 で、他の呼び(D)だけ出る場合**: Filter は呼ばれているが引数が `Eff_Render(Self, Source, Dest, Length)` の形(Length ≤ 16384、buffer が guest の heap 0x1000000〜0x30000000 の範囲、4 バイト整列)でない = 呼び出し規約の読みが違う。→ D の行(target・a1/a2/a8/a9・caller)を見て、**どの target がどの引数の形か**(`a9` が Length 候補か)を決め直す(診断の `blk` 条件の修正 = runtime のみの増分 build)。『Filter が呼ばれていない』とは言わない。
3. **A も D も 0 行(Filter の image への呼びが 0)**: Filter のどの関数も呼ばれていない = FL が Filter を走らせていない(Smart disable 以前に、スロットが存在しない/無効)。この場合は **B の表(Filter → 外の呼び)も 0**。→ 『呼ばれていない』を確定。原因は FL 側(§22 の `f_cf2f90` の戻り、スロットの有無)。
4. **A が出る場合**の読み方(`first call with src_peak > 0` を基点にする。`cum` は『Filter が受けたフレームの累積』で、Smart disable でスキップされると Export の位置より遅れる): 
   - 基点(最初に音が来た呼び)の前の呼びがすべて `SILENT-IN`(src_peak = 0): FL は無音の間も Filter を走らせている(または走らせた記録がある)。
   - 基点から +0〜+957 フレーム(wasm の厳密な 0 の区間)の呼びの種別: **ZERO(src_peak > 0 かつ dst_peak = 0)= Filter が 0 を書いた**(プラグイン側の状態。B の表と C の dump の差を見る)/ **THROUGH(dst = src)= 素通し**(wasm の出力が 0 になるのは Filter の後 = FL のミキサー側の合成)/ **FILTERED = 通常の処理**(区間が 0 になるのは Filter の後)/ **呼びが無い(基点の次の呼びが +958 以降)= Filter が呼ばれていない**(§22 の Smart disable の戻り 1 = FL の判断)。
   - Wine との比較: Wine は同じ区間で入力をそのまま出す(基準と bit 一致)= THROUGH 相当。wasm が ZERO なら『Filter の中の状態の差』、THROUGH でも出力が 0 なら『ミキサー側』、呼びなし なら『FL のスロット判断の差』。
5. **caller 列**(Filter を呼んだ FL 側の VA): `0xcf2df9`(= `f_cf2d00` の `call [rbx+0x38]` の返り番地)なら §22 の読み(Smart disable の経路)が確定。別の VA なら §22 の対象関数を差し替える。
6. **B の表**: 累積 1 / 205,000 / 211,000 / 213,000 / 240,000 の各表の増分で、**Filter が FL に頼む呼び(FPD_/FHD_ 系の Dispatcher)の種類と回数が 205,000 → 211,000 の間に変わる target**(NEW の行)があれば、その時点で Filter が何かを要求し始めた = 『音が来た』ことの Filter 側の反応。
7. **C の dump の差**: 最初の Eff_Render と累積 210,000 後の最初の Eff_Render の plug-in の object の 192 バイトの `changed dwords`。差が 0 なら状態は dump の範囲に無い(= Filter の内部の状態は別の場所)。**差があるオフセットが、Filter の enable / ready に相当するか**は、そのオフセットの値の遷移(0 → 1 など)で人が読む。
- **読んだら言うこと**(PM の指示どおり): 『Filter が呼ばれていない / 呼ばれて 0 を書く / 素通し / 通常の処理』のどれか、caller の FLEngine の関数(静的な読みで関数名と役割)。**確認できない時は『確認できない』と言い、推測を事実のように書かない**。


## PM の決定と取り込み(2026-11-07 08:10、docs のみ。製品の差分なし)

取り込み: alpha1 の 0319(依頼者の観察 O7・O8・O9 の契約の読み)と差分群(`docs/investigations/alpha22/impl/`)、Wine 10 の小テストの測定表、道具。判断は `docs/feedback/2026-11-07-owner-observations.md` の PM の決定の追記に記した(ScrollWindowEx = (b) で有効領域のモデルに揃える、時刻と Temp = (b)、DPI = (b)、CPUID の SSE2 固定と Hz 上限 120 は意図した差、Debug タブのログは FL 内部で閉じて engineLog に届かない)。**PM の検証**: 差分 3 本(scrollwindow-model、time-env-run-values、fle-stop-context)と診断 2 本(diag-g9-fl、diag-g9-fl-scroll)を HEAD の git archive に順に実適用して FAILED 0・.rej 0、`emcc -fsyntax-only -Wall -Wextra` で警告 0。実機(実エンジンの wasm)での効果は cand1 の bundle ができてから測る。依頼者の機械は 0 件。


### PM の追記(2026-11-07 08:58、再取り込み: §12 O16 の棚卸し、ページ側の積み順、Filter 変種の道具。docs のみ)

- **O16(日本語が豆腐)**: 原因は (c) 資材の不足(alpha1 の静的な読み。逆アセンブルによる; 実エンジン・Wine の実行は 0)。FL の画面の文字は GDI でなく QuickFontCache(AGG + FreeType)で、QFC は SHGetFolderPathW(CSIDL_FONTS)の下の**固定 10 個のファイル名**(msjh.ttf、msjh.ttc、msyh.ttc、malgun.ttf、batang.ttc、msgothic.ttc、arial.ttf、tahoma.ttf、seguiemj.ttf、segoeui.ttf)の存在を調べ、あった物をフォールバック一覧に足す。仮想 `C:\\Windows\\Fonts` は代用 3 ファイルなので一覧が [arial, tahoma] だけになり、日本語の字形が無い。**PM の決定**: 契約案(`msgothic.ttc` に日本語を持つ face を置く)を採る方向。実装は未着手。フォントは遅延 VFS の層(日本語の字が要った時だけ取得)に置く。フォント候補(OFL): opt1 の 0320 §9 の Noto Sans CJK JP のサブセット(かな・記号・ASCII で WOFF2 53,740 B、+JIS 第一水準で 538,872 B、JIS X 0208 全部で 1,135,112 B)。ライセンスと字形(MS Gothic の代用であり実機と同じではない)を決定文書に残す。確定は Wine の API ログ(opt1 の J0〜J3、Chrome 0 本と PM の合図)と cand1 の `FLE_DIAG_VFS_MISS` で msjh.ttf など 9 個の存在確認が出るか。
- **訂正(alpha1 の自己申告)**: 0319 §2.1 の表の『EnumFontFamiliesExW は要求された名前を「ある」と答える』は不正確。実装は『空の名前か Segoe UI のとき Segoe UI を 1 回、他の名前・文字集合は名前つきの停止』(`fle_gdi.c`)。PM の依頼文の同じ記述も同じ元の誤り。
- **ページ側の積み順(PM が実適用で確認)**: `0319-page-wall-clock-env` → `0319-page-num-cpu` → `0318-hidpi-page` → `0318-hidpi-page-test`。新しい `git archive HEAD`(`app/` のみ)に順に `patch --batch -p1`、FAILED 0、fuzz なし、.rej 0。`screen-fit.test.ts` は runtime 側の `fle_screen.h` を読むので、ページ側だけを先に push すると vitest が 2 件落ちる(想定。ページと runtime は同じ push)。`FLE_NUM_CPU` は manifest の `engine.env` に `"FLE_NUM_CPU": "auto"` を書いた bundle(RT-A)か URL `fle_cpu=auto` にだけ入り、協調版には何も足さない。
- **Filter 変種の比較(PM の追記)**: Wine の `elog16-ins1-lp` は 5,729,968 フレーム(elog16 より +14,057)。PM が numpy で確認: **先頭 4.8 s(frame 211,681)までは elog16 と 1 サンプルの差もなく、そこから差が出る**(1 s・2 s の窓で差 0、5 s の窓で 6,991 フレームが非零、最大差 28,413)。前の記述『先頭 5 s は同一』は 4.8 s が正しい。比較の道具は `docs/investigations/alpha22/tools/a22-filter-variant-compare.py`(自己テストの出力に `SELFTEST` とある物は実 Filter の結果ではない)。


### PM の追記(2026-11-07 11:55、docs のみ)
- **E4(Wine に wasm 相当の registry)は否定**: minimal `.reg` を入れた ins1-lp は対照と同じ(5 フレーム ±1 LSB)。alpha1 が独立に WAV で確認(e4-reg は 08:28 の ins1-lp と 4 フレーム違うだけ、[210723,211680) の厳密な 0 は 0/957、[4435770,4445142) は 0/9372)。PM も numpy で同じ値を確認(rms 1773 対 wasm の exactly 0、99.77〜100.8 s の rms 9955 対 1582)。registry は Filter の差の原因ではない。
- **§19 の GetModuleFileNameW の契約レビュー**(alpha1): mt3 の案の方向は (b) として正しいが、記録の前に 3 点の直し(絶対パスだけ記録、`.`/`..` を畳む、`\\?\` はそのまま)が要る。RT-B には入れず、Frequency Shifter の RT-C と一緒にする(Delay 2 の V3/Export を Wine の elog17 ときれいに比べるため)。
- **§20 の診断**: 957 フレームは tick の端数。cand1 の既存の log では Filter への呼びが読めない(g9_hook は Balance と Filter の合算で最初の 64 回は全て Balance)。診断 A〜D は RT-B の build の後に 1 本。


<!-- 2026-11-08 07:3x PM: alpha1 の §26(Insert/右クリック/fhook9)と §27(O20 delta)を末尾に追記。上の PM 節は変更していない。 -->

## 26. Insert 1 の出力を作る側の診断(RT-D4 の増分に入る差分) — `impl/diag-g9mix-slot.diff`

**先頭(何を実行したか。実エンジン・実 Chrome・Wine・native の cargo は走っていない)**: 静的な読み(FLEngine_x64.dll の capstone と生成 C の読み)+ RT-D2 の log と fhook4 の表の読み。差分への検証は `clang -fsyntax-only -std=gnu11 -Wall -Wextra` で `lift/runtime/fle_diag_g9mix.c` と `lift/runtime/fle_callback.c` の両方が警告 0、mt3 の `tree-rtd3t` と `tree-rtd3` の両方に `patch --dry-run -p1` で rc=0。**native の cargo テストは PM の許可が無いので走っていない**(mt3 の `rtd3-build.sh addpatch` が、native テストが通った後に build の木に当てて FLE_DIAG_* の読みを getenv に変える)。

これまでの Eff_Render 診断では、区間の Filter の入力が Wine と wasm で 0 だという所まで分かった(私の 14:07 report の訂正 A を別として)。何が分からないか:

- **区間の音は、Filter の入力に入らない。** Balance の出力が伏せられるのか、Balance を通る前に伏せられているのか、Balance の Eff_Render 自体が呼ばれていないのか、Insert 1 の slot 0 の Eff_Render の前後にどこで落ちるのか、**Insert 1 以外の経路で master に来るのか**。
- **入力が同じ**なのに区間の後の `src_peak` の比の中央値が約 0.58 だという事実は、音を作る側の処理が既に区間の前から違うことを示すが、どこからかは Eff_Render の結果では言えない。
- **1 slot の 1 tick の処理 `f_cf2a90(task, Length)` の前後に、task+0x54(slot が出した最終の buffer)、task+0x4c(dry)、task+0x18(in) の peak/sum/first8 を見たい。** f_cf2d00 の slot 状態(slot、si、+0xa0、+0xf7、in-place)も取る。

### 26.1 観測点(静的な読みの結果)

`f_cf2a90(task rcx, Length edx)` は vtable 0xcf26e0 の関数で、間接 call が `fle_dispatch` を通るので既存の `fle_dispatch_hook` で見られる。内部の 3 つの direct call(`f_cf2f90`、`f_cf2d00`、`f_cf3020`)は直接 C で呼ばれるので **見られない**(生成 C で確認した。既存の hook にするか、hook-points(前に lift した時から行う必要がある)を使う形にしない限り無理)。そのためこの診断は、slot の文脈を入口で取り、出口で task の最終状態と slot+0xa0 の前後を読む(間接の確認)。
- **task+0x54(slot 最終出力)** = 入口で task+0x18 から導いた in_ptr に対する f_cf2a20 の出力合成・ミックスの後。slot+0x13c|slot+0x140 が非 0 の時は `[task+0x2c]+0x68`、否则は入力と同じ(in-place)。
- **task+0x4c(dry)** = `task+0x2c+0x60`(`[slot+0x154] != 0`) else in_ptr。
- **task+8(in)** = `task+0x18`(f_cf2a20 が task+8 に代入する前の場所)。
- **f_cf2f90** = `skip` としてのみ(入口前後の task+0x28 = 0)。`slot+0xa0` の遷移で整合確認。
- **f_cf2d00 = Eff_Render** = in/out の前後。**f_cf3020** = silence pre/post(`f_cf3020` が task+0x54・len を見て slot+0xa0 を書くので、post が 0 なら間接的に確認)。

### 26.2 露出開始の印(expos)

Wine hook と揃えるため、Export 開始は **slot ごとに Length 137/138 が 24 回連続する最初の呼びの先頭 scum を expos 0 とする**傾向に直した(観測した 2 例の規則と明記。fhook4 の Wine scum 1,007,104 と整合する)。reader(a22-diag-read.py)は log の expos 欄があればそれを優先する形に直してある。

### 26.3 窓と上限

**expos 210,000〜212,499 は全呼び**、それ以外は **1/16**。出力の行は g9m 自体で 19,000 行まで(既存の g9f の 20,000 行とは別)。観測のみ。`FLE_TEST_GETENV("FLE_DIAG_G9F")` で開く。

### 26.4分類

診断である(観測のみ。ランタイムの動作は変えない)。出た行は `g9m:`。

### 26.11 Insert の出力と master の和を作る関数(VA の特定と hook 方法)

> **訂正(2026-11-08 06:5x、私の誤り。結論は §26.12)**: この節の次の 4 点は誤りか根拠が無い。(1)「関数 index 112491、file offset 0x7c92499、RVA 0x7c92499」の **0x7c92499 は wasm の code section の中のファイル offset で、RVA ではない**(`wasm-objdump -d` の左の列)。FLEngine の RVA は **0x8f2a90**(VA 0xcf2a90)。(2)「insert レコード(`imul 0x28f`)を使う関数は 416 本」は**再現できない**。`.pdata` の関数境界で数え直した結果は **388 本**(opt1 の 388 と一致。416 の数え方は私の記録に無く、再現できなかった)。(3)「Eff_Render を呼ぶのは 11 本」は**根拠が無い**。`f_cf2d00` の呼び元は `f_cf2a90` の 1 か所だけ(`tools/a22callers.py`、opt1 の `callers.out` と一致)。(4)「Insert 0 は別経路の可能性が高い」は、§26.12 の静的な読みで**弱まった**(同じ per-insert 関数 `f_1276ea0` が record 0 も通る形)。本文の `이며` は文字化け(「であり」)。

**先頭(何を実行したか。静的な読みのみ。Chrome・Wine・cargo は実行していない。実機の run は 0 本)**。

##### name section で名前が特定できた関数(RT-F bundle、engine.15a5342cd9f7.wasm、wasm-objdump)
- wasm の name section には function entry 名 `f_cf2a90` があり、関数 index `112491`、file offset `0x7c92499`、RVA `0x7c92499`（表記）にある（f_cf2a90 の名前のエントリは 9faa8b4 付近の name section dump でも確認した）。`f_cf2d00` は index 112495、file offset `0x7c93452`、RVA `0x7c93452` である。周辺の名前:`f_cf2a20`、`f_cf2f90`、`f_cf2c60`、`f_cf2cb0`、`f_cf3020`、`f_cf3200`、`f_cf3230`、`f_cf2e0b`。
- **Insert 出力と master の sum を作る本体は name section で特定できなかった**。name section にある `f_cf3790` / `f_cf4250` / `f_cf42f0` / `f_cf4430` は、insert レコードの `imul 0x28f` を含むが、Eff_Render（`f_cf2d00`）を呼ぶ経路ではなく、master sum を作る形ではない(静的な読み)。

##### 二分探索の hook 点として確実に言える点
- **`f_cf2a90` の入口と出口**。RVA `0x8f2a90`（Wine の image 0x400000 + 0x8f2a90 = `0xcf2a90`; name section では `f_cf2a90` index 112491）。既存の RT-E の wasm 診断はこの関数を既に対象として `fle_dispatch_hook` で捉えている。Wine の hook も同じ VA で入っている(opt1 の fhook5 / fhook8)。
- task の読み方：`task = rcx`、`Length = edx`（ローカル変数では `local[0]=rcx`, `local[1]=rdx`, `edx` の上位ビット)。入口はまだ `f_cf2a20` 前で、入口後に `task+0x38` で slot を、`task+0x18` で input buffer を、`task+0x2c` で insert record の元を、`slot+0x130` で insert index を、`slot+0x64` で si を、`slot+0xa0` で silence counter を、`slot+0xf7` で Smart disable flag を、`slot+0x13c/slot+0x140` で out が in-place か insert の別 buffer かを、`slot+0x154` で dry active を読む。
- **f_cf2a90 の戻りが本命の二分探索の hook 点である**（呼び出し側は `r9` 等でなく、`task+0x54` のポインタで戻る）。この戻りの時点で `task+0x54` の peak/sum が slot の最終出力이며, task+0x4c = dry、task+0x20 = input、task+0x28 は [task+0x48] と同じなら加工済み（Eff_Render を通過した）、0 な`f_cf2f90` の bypass。fhook5/fhook8 の Wine 表はこの形。
- 別候補（未確認）:
  1. `f_cf2d00`（RVA 0x8f2d00、name section f_cf2d00 index 112495）。Eff_Render。
  2. `f_cf2f90`（RVA 0x8f2f90）。Smart disable。
  3. `f_cf3020`（RVA 0x8f3020）。停止判定。
  4. `f_cf3790`（RVA 0x8f3790？短縮。正確には name section でも `f_cf3790` func[4509] の RVA がない。function の最初の命令として a2c/c38 が `f_cf3790` ではないか疑問で、name section に RVA はないので不明）。候補として列挙。

##### なぜ Insert 0(master)が出てこないか(静的な読み。opt1 の fhook8 の表と照合)
- f_cf2a90 は vtable の 1 エントリ(`TProcessingModule` / `TPluginModule` / `TPluginRackModule` 系の mix 対象側の slot 処理)。Insert 0(master)は、センド先の insert としては結局見えるが、**master 自身の insert スロット処理は `f_cf2a90` を呼ばない別経路**の可能性が高い(観測: fhook8 で f_cf2a90 の entry が出たのは Insert 1 と Insert 47 の 3 個だけ)。Insert 0 の slot が f_cf2a90 を呼ばない理由として、静的には「master はプラグインスロットの mix 対象ではない、master 固有の別経路」で説明できるが、実機(未確認)。
- 候補の 416 本からも落ちるべきではない: insert レコード(`imul 0x28f`)を使う関数は 416 本あるが、その末端のうち Eff_Render（`f_cf2d00`）を呼ぶのは 11 本(「call [reg+0x40] を持つ」)だけ。Eff_Render までに必要な処理は、直接 call する「f_cf2f90 / f_cf2d00 / f_cf3020」であり、その前段(`f_cf2a20`、「f_cf2c60」と「f_cf2cb0」)が関数ポインタ呼びをするため、狭い関数は「slot 処理本体と混ぜ」に絞るには f_cf2a90 の役割が最適。

##### master sum の二分探索の手順(opt1 の Wine hook と wasm の診断の両方に渡す形)
1. Wine 側: opt1 の既存の hook に `f_cf2a90` の入口/出口(表の f2a90 entry pairs)に、`task+0x54` / `task+0x4c` / `task+0x18` / `slot+0x130` / `slot+0x64` / `slot+0xa0` / `slot+0xf7` を頂き、master を同定するのに「slot ごとの insert index が同じの Insert の task を 1 台に持たないリンク」をもう 1 行とする。
2. wasm 側: 同じ VA `f_cf2a90` で既存の診断(fe_diag_g9mix.c)が既に捉えている。Insert 出力と master の sum を作る関数が未特定のため、二分探索は **`f_cf2a90` の戻り(task+0x54). 使用**とする。

### 26.5 RT-E 統合(2026-11-07 15:53)

`impl/0319-rte-combined.diff` は、次の一本の順で接続した統合 diff である(各 `.diff` の書式は当てる樹の形に合わせてあり、tree-rtd3 と tree-rtd3t の両方で `patch --dry-run -p1` が rc=0):

1. `impl/0319-show-descendants-paint.diff` + `impl/0319-show-descendants-paint-tests.diff` (RT-E 候補。PM の承認は未取得)。
2. `impl/0319-o20-virtual-pointer-contextmenu.diff` + `impl/0319-o20-virtual-pointer-contextmenu-tests.diff`。
3. `impl/diag-g9mix-slot.diff`(今回の診断。記事は `tools/a22-diag-read.py` 節 11)。

native テストの手順は `impl/README-RTE1-native-tests.md` にある。実測済みの内訳は: wall_fr11 passed=49 failed=0、sweep1 passed=4 failed=0、callback passed=19 failed=0。

#### 26.5.1 O20 の追加の修正点(2026-11-07 17:25 以後、静的な読み)

- **WM_CONTEXTMENU の wParam（修正済み）**: Learn の WM_CONTEXTMENU には「wParam は右クリックされた window の HWND で、子 window のことがある」とある。子 window の context menu が parent に送られる場合でも、**wParam はクリックされた子 window**でなければならない。初版は子 window なら parent を wParam にしていたため FL 側が child を同定できずメニューが出ない可能性があった(実機未確認)。現在の diff では受信先は parent、wParam は original hwnd に直した。
- **drag3/4 の乖離の読み**: `fle_win.c` の `set_pointer` は offset だけを更新し、host の最新絶対位置をそのまま残す。wrap の下で SetCursorPos(450,17) が来た直後、host latest は 450,1079 のままで offset y = -1062 になる。SetCursorPos を呼び出した後、ブラウザの次 host move(450,20)は OS 側でカーソルが 17 に動いた後の絶対座標なので、guest = clamp(20 − 1062) に飛ぶ。さらにその後 guest が 0 で張り付き、drag 3・4 の host には反応しなくなる(読み)。
- **修正案**: `set_pointer` で guest 位置だけでなく **host latest を指定された位置へ再アンカーし offset を 0**にする(SetCursorPos の契約どおり「cursor を指定位置へ動かす」)。これにより後続の host 絶対座標は新しい位置からの絶対座標として扱われる。子 window の位でクラス・窓の分岐なし。詳しくは差分 `impl/0319-o20-virtual-pointer-contextmenu.diff`。

### 26.5.2 右クリックの別経路の静的な読み(2026-11-07 17:25)

> **訂正注(2026-11-08 07:0x)**: 本節の次の記述は誤りまたは根拠不足。正しくは §26.13。(1) `f_845950` の呼び元を `f_8459ed` とした箇所は誤り(`0x8459ed` は `f_845950` の内側の番地で、`f_845950` への直接 call は 0 件)。(2) `f_81d2a0` の呼び元 `f_108e0d0` は誤りで、正しくは `f_108e120`。(3) `f_845950` を「自前の TrackPopupMenu への 2 つ目の経路」とした読みは、仮想呼びの経路に改め、呼び元は未確認。(4) 旧 `a22callers.py` の関数名は CC 走査による推定で誤りがあった(新ツール `a22callers-pdata.py` で再検証)。

**先頭(何を実行したか。静的な読みのみ。Chrome・Wine・native の cargo は走っていない。実機の run は 0 本。以下は仮説として置く、未検証)**。

##### TrackPopupMenu / CreatePopupMenu の呼び出し元
- `user32.dll` の import の thunk 側は `CreatePopupMenu 0x4248f0`、直接 call 2 本(`0x81d0ec`、`0x83d6a6`)。`TrackPopupMenu 0x425620`、直接 call 2 本(`0x81d1b5` は関数 `f_81d040`、`0x845a49` は関数 `f_845950`)。
- `f_81d040`(VA 0x81d040): general な "object から menu を作り、CreatePopupMenu → InsertMenuW → TrackPopupMenu(idfirst=-1)"。呼び出し元は `f_81d2a0`(0x81d2a0)だけ(`0x81d2f5` から一度)、その呼び元は `f_108e120`(0x108e120)など Delphi UI コントロールクラスの vtable 関数(`0x106b648` 等)。**汎用の menu 表示経路が UI コントロールの仮想メソッドから**呼ばれる。
- `f_845950`(VA 0x845950): 別のコントロール handler 側で、関数内に TrackPopupMenu(`0x845a49`)がある。**自前の TrackPopupMenu への経路が 2 つある**。
- **WM_CONTEXTMENU を送っても出ない理由(仮説)**: 上の 2 経路はどちらも「コントロール自身が menu を用意できる時」に直接 TrackPopupMenu を呼ぶ形で、**WM_CONTEXTMENU を待たない**。FL のコントロールは DefWindowProc の生成した WM_CONTEXTMENU を拾う経路と、「WM_RBUTTONUP の wParam(ボタンの状態)を見て自前で TrackPopupMenu へ行く」経路がある(後者の可能性がこの静的な読みでは優先する。未検証)。

##### GetKeyState / SetCapture / ReleaseCapture
- `GetKeyState` thunk `0x424d50` の直接 call 21 本。直前の `ecx` の値を生成 C から読んだものは **VK_SHIFT(0x10)、VK_CONTROL(0x11)、VK_MENU(0x12)、0xa5** だけ。**`VK_RBUTTON(0x02)` で呼ぶ箇所は無い**。右ボタンの現在状態は直接の GetKeyState では読んでいない。
- `SetCapture`(0x4253d0)8 本、`ReleaseCapture`(0x4252b0)29 本、`GetCapture`(0x424b90)57 本。このうち `SetCapture` の `0x72a57b`(`f_72a310`)は drag 用の capture の経路である。右クリックの capture の直接の証拠は無い。

##### 候補(未検証)
1. (A) コントロールが WM_CONTEXTMENU を受けずに自前の OnMouseUp 等で直接 TrackPopupMenu へ行っている(静的に見える 2 経路 `f_81d040`、`f_845950`)。
2. (B) DefWindowProc が生成した WM_CONTEXTMENU の wParam が parent で、handler が child の同定に失敗した。先記の diff では wParam=original hwnd に直してあるので、残す価値は下がった(実機未確認)。
3. (C) Capture 中のメッセージの再送が効かない。
4. (D) WindowProc が DefWindowProc を呼ばずに自前で完了している。
次に要る観測(診断): `CreatePopupMenu` / `TrackPopupMenu` / `DestroyMenu` / `GetCapture` が実際に呼ばれたか、WM_CONTEXTMENU 受け側の回数、生成後のゲストの処理を出す。それが A-D を分ける。この観測は実機(run)が要る。

### 26.8 RT-F bundle(18:0x)

mt3 が `fl-engine-test-rtf-d45afb0e` を出した(RT-D3 素の木 + alpha1 の再構築版 RTE 差分。wasm 170,925,746 B、SHA256SUMS ok=9、name section 5,184,047 B、FLE_DIAG_ATOM/GIL/G9F の文字列あり、engine.env に FLE_DIAG_* なし。dll/seed/glue は RT-E と同一)。manifest コピーの手順は 0315/tools/mk-rtf-assets.sh BUNDLE_DIR [NAME=rtf-diag](FLE_DIAG_ATOM=1 FLE_DIAG_GIL=1)。

### 26.7 RT-E bundle(17:06)

mt3 が `fl-engine-test-rte-87af8f55` を出した(RT-D3 + alpha1 の 0319-rte-combined 統合。wasm 170,925,731 B、SHA256SUMS ok=9、name section 5,184,034 B / 関数名 126,910 件、FLE_DIAG_ATOM/GIL/G9F の文字列あり、engine.env に FLE_DIAG_* なし。dll/seed/glue は RT-D3 と同一)。manifest コピーの手順は `0315/tools/mk-rte-assets.sh BUNDLE_DIR [NAME=rte-diag]`(FLE_DIAG_ATOM=1 FLE_DIAG_GIL=1)。

### 26.8 Wine 側の slot hook の観測(fhook5)

**先頭(何を実行したか。opt1 が実 Wine 10 で実行した観測で、私は走らせていない)**。opt1 の fhook5 は、FLEngine `f_cf2a90` の入口/出口の slot ごとの観測で、WAV は `elog16-ins1-lp` と 2 フレームだけ ±1 LSB の差(`fhook4` とは 4 フレーム)。`fhook5-slots.tsv` の事実:
   - window 内(expos 210,000..212,500)Insert 47 の slot は 18 calls 全て入力 0・出力 0。
   - Insert 1 の 2 個の slot は、expos 211,542 を境に入力が非ゼロで出力も非ゼロ。ただしこの expos の定義は fhook5 側の Export 開始で、Filter の Eff_Render の expos とは差が 138 フレームほどある。`si=2` の 2 行で Balance の同定は不可(Balance の Eff_Render の RVA は次の Wine で)。
   - `in_peak>0` の 7 rows で Filter の動きと一致する(138 フレームずれているが、区間の外で初めて音が入る形)。

### 26.9 Wine 側の Balance の Eff_Render の観測(fhook6)

**先頭(何を実行したか。opt1 が実 Wine 10 で実行した観測で、私は走らせていない)**: `exports/fhook6.out`、`exports/elog/fhook6.elog`、`exports/trace/hookx-fhook6.txt`、`tables/fhook6-balance.tsv`。WAV は frames 5,729,968、elog16-ins1-lp と 4 フレーム ±1 LSB。hook 行: `# hook bal installed at 0000000079b6efc0 (Fruity Balance_x64.dll base 00000000798b0000, first instruction kind 0 len 2)`。
- Balance の Eff_Render に入ったのは区間の 1 本だけ(seq 2a8f、expos 210578): 入力 peak 0・sum 0、出力 peak 0・sum 0。前後の間引き行(expos 206168・208372)も入力 0。
- Balance の入力が非ゼロになる expos 212,782(seq 2a9f)では、出力 peak = 入力 peak(0.37556)で素通し。
- つまり区間 [210723, 211680) の Balance の出力は 0。**この区間の WAV の音は Insert 1 を通らない**(区間へのボーカルの入力が既に 0)。
### 26.10 Wine 側の Balance の Eff_Render（fhook7。修正版 hook）

**先頭(何を実行したか。opt1 が実 Wine 10 で実行した観測で、私は走らせていない)**。WAV frames 5,729,968、elog16-ins1-lp と 4 フレーム ±1 LSB、`tables/fhook7-balance.tsv`。hook は `hooktables/balance-effrender.tbl` の修正版(key=rcx、len=r9、win=210000-212500。sha cf407d69d02e017b)。
- Balance の Eff_Render の窓内(210,000〜212,500)18 呼出のうち、expos 210,026..211,542 の 12 呼出は入力 peak 0・sum 0、出力 peak 0・sum 0。
- 入力が非ゼロになるのは **expos 211,680**(seq 2a67、pos 1225440)で、その呼びの入力 peak は 0.566111・sum 57.6837358、**出力も peak 0.566111・sum 57.6837358 で完全一致(素通し。Balance volume = 1.0)**。その後も出力 peak は入力 peak と同じ。
- つまり区間 [210723, 211680) の Balance の出力ピークは 0。**「Balance が 0 を出した」か「入力が 0」かは、依然として区間の入力自体が 0 なので割り切れない**。この run だけでは Balance が 0 を「出した」ことは分からない。
- slot_idx は全件 2 のため、Insert 1 の slot を個別同定する材料は未提供(opt1 の次段で進める)。



## 27. O20 Tempo: delta 方式の仮想ポインタ(2026-11-08 06:2x、`impl/0319-o20-delta-pointer.diff` + `-tests.diff`)

### 27.1 実際に実行した物
- 静的な読み: FLEngine_x64.dll の drag 関数 3 本(VA 0x7f4ca0 / 0x7f4ec0 / 0x7f4e20)を capstone で読み直し(`tools/a22dis.py`)、rip 相対のグローバルの番地を計算した。
- native テスト(cargo、実機ではない): mt3 の tree-rtd3t のコピー(`/home/ubuntu/work/alpha22w/tree-o20d`)に実装を入れ、`bash lift/harness/test.sh full --jobs 1 --no-record --only 'wall_fr11|sweep1|callback'`(nice 19、FLE_INPUT_ROOT=/home/ubuntu/work/signature-studio-develop、私用 CARGO_TARGET_DIR)を回した。
- **Chrome・Wine は実行していない**。実ブラウザでの 522 と drag 3/4 の確認は gpu1 の次の build 後の run が要る(未確認)。

### 27.2 FL の drag 関数の読み(静的。生の命令の読み)
グローバル(image VA。rip 相対の命令から計算した番地。意味づけは命令の使われ方からの読み): 0x15f6e0c/10 = 開始点 x/y(**0x15f6e0c は `0x7fffffff` なら「drag 非アクティブ」の目印も兼ねる**。begin が qword で書き、end が `[0x15f6e0c]` を ecx、`[0x15f6e10]` を edx にして 0x425490 を呼ぶ)、0x15f6e14/18 = prev.x/prev.y、0x15f6e1c/20 = 出力 dx/dy、0x15f6e24 = ボタン mask、0x15f6e28 = scale、0x15f6e2c/30/34/38 = 縮めた矩形の left/top/right/bottom。(訂正: 先の下書きは 0x15f6e0c を「開始 flag」と書いたが、開始点 x と目印の兼用が正しい。)
- **begin 0x7f4ca0**: 非アクティブなら、`call 0x425580`(import 表で `user32!ShowCursor` と確認)を ecx=0 で呼ぶ → カーソルを隠す。開始点を保存し、矩形の 4 辺を 0x20 だけ内側へ縮める(`add [..],0x20` ×2、`add [..],-0x20` ×2)。
- **move 0x7f4ec0**:
  1. `cur` = `call 0x424c40` の結果(rbp+0x38/0x3c の POINT)。
  2. `dx = (prev.x − cur.x) × scale`、`dy = (prev.y − cur.y) × scale` を出力。
  3. **`prev = cur`**(qword 1 本の mov)。
  4. ボタン mask が一致するときだけ: `cur.x < left` なら `prev.x += (right − left)`、`cur.x > right` なら `prev.x −= (right − left)`。y も同様(top/bottom)。
  5. `prev != cur` なら `call 0x425490(prev.x, prev.y)` でカーソルを prev へ動かす。
- **end 0x7f4e20**: ボタン mask が一致するなら `call 0x425490(開始点)` → `call 0x425580` を ecx=0xffffffff で呼ぶ(= `ShowCursor(TRUE)`)→ 開始 flag を 0x7fffffff に戻す。
- 注意(読みの訂正): 私の過去の要約に「prev が矩形の外なら wrap」とあったが、実際は手順 3 の後なので **cur(= 更新後の prev)が縮めた矩形の外のとき**である。
- `0x424c40` と `0x425490` の jmp thunk の行き先(IAT の 0x1976340 / 0x1976308)は、pefile の名前付き import 表(`DIRECTORY_ENTRY_IMPORT`)の番地に**載っていなかった**(`0x425580` = ShowCursor、`0x4254a0` = SetPropW は載っていた)。delay import か別の表かは**未確認**で、名前は**未確定**。呼び方(POINT の出力先を渡す / 2 整数を渡す)と、PM・gpu1 の先行の読み(0x425490 → SetPhysicalCursorPos)から GetPhysicalCursorPos / SetPhysicalCursorPos と読んでいるだけ(仮説)。

### 27.3 この読みから、仮想ポインタに要る性質(静的な推論。実機未確認)
FL の move は「**前回の自分の SetPhysicalCursorPos の結果をそのまま次の cur として読み、そこからの動きだけを delta にする**」という Windows の契約(カーソルは状態を持ち、デバイスは相対の動きを報告する)に依存している。
1. 手順 5 の直後の GetPhysicalCursorPos は、**guest が set した位置**を返すこと(= guest を host の絶対位置で上書きしない)。上書きすると、次の move の dy = (prev − cur) × scale が (wrap した prev − ブラウザの端の座標) になり、画面の高さ級の値になって **522 に張り付く**(RT-B)。
2. その後の host の動きは、**set した位置からの相対**として効くこと(RT-F が失ったもの。RT-F は host 最新を guest に再アンカーするので、ブラウザのポインタが端で動かない限り、guest は host の絶対座標へ戻る)。
3. drag の end で `SetPhysicalCursorPos(開始点)` を呼んだ後、ブラウザの実ポインタは別の場所にある。**次の host move は host の絶対点を guest にしなければ**、(RT-E のように)開始点 + (host − 古い host) = 画面外になり、次の drag が効かない。

### 27.4 設計(delta + 表示中は host の絶対点)
- 状態: `ptr`(guest)、`host_ptr`(host の前回の点)。**offset は持たない**(RT-E の誤りの根)。
- host move `fle_win_host_pointer_set(x, y)`(fle_input.c:146): `n = clamp(x, y)`。
  - ShowCursor の display count が負(カーソルが隠れている。FL の drag 中)なら `ptr = clamp(ptr + (n − host_ptr))`。
  - そうでなければ `ptr = n`。
  - どちらでも `host_ptr = n`。
- SetCursorPos / SetPhysicalCursorPos(`set_pointer`): `ptr = clamp(x, y)` のみ。**host_ptr は触らない**。
- apply_display の丸め直し(`fle_win_pointer_reclamp`、fle_input.c:750): `host_ptr` と `ptr` を新しい画面へ別々に clamp(offset が無いので、再計算する物が無い)。
- カーソル状態の取り込みは、既にある弱シンボル `fle_gdi_cursor_display_count()`(fle_win.c:53、GetCursorInfo が同じ物で flags を決める)をそのまま使う。**窓名・クラス名・FL 固有の分岐は無い**。リンクされていなければ(弱シンボルが NULL)従来どおり絶対(sweep1 の stub もこの形で通る)。
- 契約との関係: SetCursorPos の page は「カーソルを指定の座標へ動かす」(fle_win.c 先頭のコメント)。ホストが報告できるのは絶対座標だけだが、**カーソルが隠れている間は、ホストの見える実ポインタが guest のカーソルではない**ので、ホストの動きを「動き」として受ける。表示されている間は、見えているブラウザのポインタが guest のカーソルなので絶対(S-pointer-shown-absolute、**選択**。page がこの区別を書いているわけではない。契約の読みではなく、2 つの観測(522 と drag 3/4)を同時に満たす最小の設計として私が選んだ物)。
- SetCursorPos が一度も呼ばれなければ、guest = host(隠れている間でも delta の和 = 絶対位置、clamp で端に当たる場合を除く)で、従来(RT-B)と同じ。この一致はテストの `l[0]`・`l[2]`・`l[4]`〜`l[6]` と、offset=0 のテストで確認した。

### 27.5 variant 比較(native テスト。`tools/run-variants.sh`。実機ではなく、私のモデルを C のテストにした物)
同じ新しい 3 テスト(wall_fr11 の `the_virtual_pointer_*`)を、`fle_win.c` の 4 つの形に当てた。各形のポインタ部分だけを差し替え、他は同一。`wall_fr11` の passed/failed(1 テストは最初に外れた assertion で止まるので、下の「最初の不一致」はそのテストの最初の 1 件だけ):

| 形 | wall_fr11 | 最初の不一致(left = 実測、right = 期待) |
|---|---|---|
| RT-F 形(set で host を guest へ再アンカー、offset 0) | passed=48 failed=3 | wrap 後(set 450,1036)の host move 450,30 で **guest = 450,30**(期待 1035)。FL の次の move は prev = 1036 と cur = 30 の差を取るので、**522 の張り付きが再現する形**。 |
| RT-E 形(set で offset を保持) | passed=48 failed=3 | 表示中の host move(20,10)で **guest = 110,200**(期待 20,10)。drag 終了後の host move 450,20 で **guest = 450,0**(期待 450,20)。**gpu1 が実 Chrome で見た「次の host move(450,20)で guest が 0」と同じ値**。 |
| 純 delta(隠れている/表示中を区別しない) | passed=48 failed=3 | RT-E 形と同じ 2 件(110,200 と 450,0)。 |
| **最終形(delta は隠れている間だけ、表示中は host の絶対点)** | **passed=51 failed=0** | なし |

- 純 delta が S2(drag の end で SetCursorPos(450,17)、ブラウザの実ポインタは y=1079 のまま、次に y=20 へ)で guest = 0 になることは、この節の作業の最初に、私が python の算術(エンジンの測定ではない)で出していた。ここで C のテストになり、gpu1 の RT-E の観測値(0)と一致した。**一致したのは「同じ値が出る」ことまでで、原因が同じだと実機で確認したわけではない**(未確認)。
- 純 delta が表示中の set(「set → 100,200」の後の host 20,10 で 110,200)を外すのは、私が選んだ規則 S-pointer-shown-absolute に反するから。FL の実際の挙動の誤りを示す物ではない。純 delta を実機で外す決め手は S2(450,0)の側である。

### 27.6 テスト(wall_fr11 の 4 本。実装の最終形で 52 件すべて通過。全セットの結果は 27.8)
1. `the_virtual_pointer_moves_by_the_host_delta_while_the_cursor_is_hidden`: 表示中は host の絶対点(set の後でも host 20,10 → 20,10、GetCursorInfo の flags 1・pos 20,10)。ShowCursor(FALSE) の後は host の動きが delta になる(20,20 → 21,19 → 450,40)。その後 host 450,32 → 450,31 の後に **SetPhysicalCursorPos(450,1036)**(FL の wrap)。次の host move 450,30 / 29 / 28 で guest = **450,1035 / 1034 / 1033**(= FL の次の move が読む dy は 1 ずつ。RT-B の 522 に当たる「画面の高さ級の dy」は出ない)。ShowCursor の count が -1 のままなら(FALSE・TRUE の組を挟んでも)delta が続き(1032)、count が 0 に戻ると host の絶対点(450,26)。
2. `the_virtual_pointer_is_the_host_point_again_when_the_cursor_is_shown_after_a_set_cursor_pos`(**RT-E で drag 3・4 が止まったシナリオ**): host (450,1079) で隠し、SetPhysicalCursorPos(450,17)(drag 2 の end)→ ShowCursor(TRUE) → guest 450,17。host move 450,20 → **450,20**(RT-E 形は 450,0)、450,21 → 450,21。続けて 3 回目の drag(隠す → host 22/23 → wrap(1036)→ host 24 = **450,1037** → 終了の set 450,23 → 表示 → host 25 = **450,25**)。
3. `the_virtual_pointer_is_brought_inside_a_smaller_screen_and_keeps_moving_by_the_delta`(apply_display の丸め直し): 隠れている間に set(1500,900)、fle_screen_set(1024,768) + reclamp → guest 1023,767、host の次の動き 21,19 は host の前回の点(22,20、画面内)からの差 (−1,−1) → **1022,766**、表示後の host 23,21 → 23,21。
4. `the_virtual_pointer_equals_the_clamped_host_point_when_the_guest_never_sets_the_cursor`(**offset=0 の従来動作との一致**): set を一度も呼ばず、画面外の点を含む 13 点(2500,-9 / -30,1500 / 1920,1080 など)を、表示中と隠れている間の両方で流し、guest が `clamp(host)` と全点一致(bad 0)。

### 27.7 3 経路の扱いと、分かっていないこと
- 3 経路: (1) host move は `fle_win_host_pointer_set`(fle_input.c:146 は**無変更**)、(2) SetCursorPos/SetPhysicalCursorPos は `set_pointer`(guest のみ)、(3) apply_display の丸め直しは `fle_win_pointer_reclamp`(fle_input.c:750 は**無変更**。ptr と host_ptr を別々に clamp)。変更ファイルは `fle_win.c` 1 つ(65 行の diff)。fle_input.c/h、sweep1 の stub、sweep1.rs は無変更(新しい公開記号が無いため)。
- **実機で未確認**(gpu1 の実 Chrome が要る): (a) 522 が消え、drag 1〜4 すべてが追随すること。(b) FL が drag の途中で ShowCursor の count を負のまま保つ前提(begin で FALSE、end で TRUE。27.2 の読み)。FL の他の場所で count が負になり、かつ SetCursorPos を呼ぶ経路があれば、そこで delta 動作になる(その場合も SetCursorPos が無ければ guest = host で従来と同じ。テスト 4)。(c) `0x424c40` / `0x425490` が GetPhysicalCursorPos / SetPhysicalCursorPos であること(27.2。名前付き import 表に無い)。
- **delta 方式が解決しない物**: ブラウザの実ポインタは画面の端で止まるので、wrap の後にユーザーがさらに同じ向きへ動かし続けても、host の delta が出なくなる端(0 や 1079)に着いたところで drag は止まる。これは Pointer Lock(`docs/decisions/0319-pointer-lock-stage2.md`、第 2 段)の領分で、この diff は 522 の張り付きと drag 3・4 の停止の 2 つだけを直す。限界の長さは gpu1 の実測が要る(私は測っていない)。
- S-pointer-shown-absolute(表示中は host の絶対点)は、私が選んだ規則で、SetCursorPos の page が書いている事ではない。ShowCursor の count を使わない形(純 delta)は、表示後の最初の host move で guest が 0 になる(上の表の 450,0)ので採れない。他の選び方(たとえば表示に戻った最初の host move だけを絶対にする)でも 2 観測は満たせるが、「最初の」という状態が増えるので、count だけで決まる今の形を選んだ。

### 27.8 分類と成果物
- 分類: (b) 外部 API の契約の実装(SetCursorPos の後の入力は新しい位置からの相対、ホストが絶対しか渡せないので隠れている間は delta で受ける)。ただし表示中の絶対は上記の**選択**。(a) 余計な処理の除去を含む: RT-E の offset と RT-F の再アンカーという「guest の位置を host の絶対点で補正・上書きする処理」を両方除いた。クラス・窓名・FL 固有の分岐は無い(`fle_gdi_cursor_display_count()` は GetCursorInfo が既に使う API 状態)。
- native テスト(tree-o20d = mt3 の tree-rtd3t のコピー + 本差分): `bash lift/harness/test.sh full --jobs 1 --no-record --only 'wall_fr11|sweep1|callback'`(nice 19): **results=7 passed=75 failed=0**(wall_fr11 52/0、callback 19/0、sweep1 4/0)。
- 差分(RT-F の tree 形が基準。tree-rtd3t と tree-rtd3 の両方に `patch --batch -p1` をコピーへ実適用して rc=0。tree-rtd3t は結果ファイルがテストしたファイルと cmp 一致、tree-rtd3 は FLE_TEST_GETENV→getenv の差だけを除いて一致):
  - `impl/0319-o20-delta-pointer.diff`(65 行、3,502B。fle_win.c)
  - `impl/0319-o20-delta-pointer-tests.diff`(198 行、10,534B。wall_fr11.rs。**旧テスト `the_virtual_pointer_keeps_set_cursor_pos_as_a_relative_offset` を置き換え**、新テスト 4 本を入れる。runtime の diff と**必ず一緒に**当てること。片方だけだと旧テストか新テストが外れる)
- 先に出した `0319-o20-virtual-pointer-contextmenu.diff`・`0319-rte-combined.diff` のポインタ部分(offset 保持、再アンカー)は、この diff で置き換わる。WM_CONTEXTMENU の wParam 修正と apply_display 以外の部分には触れていない。

### 26.12 Insert 出力と master の和: 静的な読み(2026-11-08 06:3x–06:45)と、§26.11 の訂正

**先頭(実際に実行した物)**: 静的な読みのみ。FLEngine_x64.dll を capstone と pefile で読み(`tools/a22dis.py`、`tools/a22refs.py`、`tools/a22callers.py`、新規 `tools/a22-insert-fns.py`)、結果を `tables/insert-fns-0319.tsv`(388 本)と `tables/hook-insert-sum-0319.tbl`(提案の hook 表。未検証)に出した。Wine・Chrome・cargo は実行していない(この節の作業中)。opt1 の fhook9(v10 shim、06:40 開始)の結果は含まない。

##### 0. §26.11 の訂正(私の誤り)
- 「index 112491 / file offset 0x7c92499 / RVA 0x7c92499」の **0x7c92499 は wasm の code section 内の file offset で、RVA ではない**。FLEngine の RVA は **0x8f2a90**(VA 0xcf2a90)。
- 「insert レコードを使う関数は 416 本」は再現できなかった。`.pdata` の関数境界で数えた結果は **388 本**(opt1 の 388 と一致)。
- 「Eff_Render を呼ぶのは 11 本」は根拠が無い。`f_cf2d00` の呼び元は `f_cf2a90` の 1 か所だけ(`a22callers.py`、opt1 の `callers.out` と一致)。
- 「Insert 0 は別経路の可能性が高い」は、下の 3 で**弱まった**(record 0 も同じ per-insert 関数を通る)。

##### 1. insert 単位の処理の入口: `f_1276ca0`(RVA e76ca0)→ `f_1276ea0`(RVA e76ea0)
- `f_1276ca0`(19 B、`sub rsp,0x28` で始まる)は `[rcx+8] = edx` を書き、`f_1276ea0` を呼ぶ。`f_1276ea0`(1,094 B)が、insert 1 つの 1 tick の処理本体。
- `f_1276ca0` の VA は、`.text` 内の 1 本の vtable(`TMixerTrackModule` の名前文字列が `0x1276850` 付近に隣接)から `abs64` で参照される(`0x1276828`)。**つまり `f_1276ca0` は `TMixerTrackModule` の仮想メソッド**。直接 call は無い(vtable 経由)。
- `f_1276ea0` の内部(読み): `rcx+0x1c`(insert index)で record を `[G15e06a0] + idx×0x28f×8` から引く → 送り先(`+0x1344`)を回す → 出力を `+0x34` に組む → **slot 配列(`+0x13ec`)を回し、各 slot の vtable[0] = f_cf2a90 を呼ぶ(0x12770a3 `call [rax]`)**。したがって f_cf2a90 は `f_1276ea0` の内側にある(`a22callers.py`: `f_cf2a90` の vtable 参照は `0xcf26e0` の 1 件のみ)。
- slot ループに入る条件(0x1276ff3〜0x127700f の読み): `record+0x150 != 0` または `record+0x36c != 0` または `bl != 0`(`bl` は `f_1276890` の戻り)。外れると `f_1277c80` と `f_1277730` が走り、f_cf2a90 は呼ばれない。slot 配列が空(`+0x13ec`)でも呼ばれない。

##### 2. 「加算」を担う関数: `f_1277940`(RVA e77940、`push rbp`、586 B)= 送り(send)
- `obj+0x1c` の insert の送り配列(`record+0x1344`、24 B 要素: dword 送り先 index、float gain L/R)を回し、送り先 record の `+0x34`(バッファ)へ、グローバルの関数ポインタ(`0x15e4b38`、`0x15e2fd8`、`0x15e4db0`、`0x15e3968` など)で書く。
- ただし、**関数ポインタの指す先は `.bss`**(0x15f3468、0x15f4a10 など)で、import 名は付いていない。つまり「加算の primitive の正体」は**静的には確定できない**(実行時に別の関数が入るはず)。「加算」は実行時の登録を見ないと言えない(未確認)。
- 送り先が 0(master)の時、その `+0x34` は master の和。よって **master の和が作られる点は `f_1277940` の終了時点**(送り先 0 の要素を処理した後)と読める(仮説。opt1 の `snd` hook と同じ VA)。
- `f_1277940` への直接 call は `0x1277272` の 1 件で、呼び元は `f_1276ea0`(その後で `f_1277b90` を呼ぶ)。

##### 3. `f_d175b0`(opt1 の `ins0`)の参照
- `a22refs.py` で、`0xd175b0` を指す rip 相対の `lea` は `f_d18130` の中(VA `0xd1a1ad`)の 1 件だけ。`lea rcx,[rip-0x2c04]` の後に `mov [rax+0x74], rcx`(`rax` = `[rbp+0x330]`)が続くので、**オブジェクトのフィールドへの関数ポインタの代入**と読める(vtable ではない)。
- `f_d18130` の直接 call は `0xd1d656`(in `f_d1d3d0`)と `0xd1f026`(in `f_d1efd0`)の 2 件。`f_d175b0` の直接 call は無い。
- 結論: **`f_d175b0` が実際に呼ばれるかは、静的には確定できない**(フィールドに入ったポインタが呼ばれる経路の先まで追っていない)。opt1 の `ins0` が発火するかは本番 run で見るしかない。
- `.rdata`/`.data` に `0x9175b0`(RVA 形)の dword は、`.pdata` の中の 1 件だけ(関数ポインタ表ではない)。

##### 4. 提案の hook(`tables/hook-insert-sum-0319.tbl`、未検証)
- Rank 1: `f_1276ca0` の EXIT(insert 単位の 1 tick の出力。`record(idx)+0x34` が N フレーム)。
- Rank 2: `f_1277940`(send。master = 送り先 0 の `+0x34`)。opt1 の `snd`(VA 0x1277940)と同じ点。
- Rank 3: `f_1276540`(master の tap。`f_1276ea0` の `obj+0x1c == 0` の時だけ呼ばれる、2 箇所)。**master 固有の経路の候補**。
- Rank 4: `f_1276ea0` の slot 入口の条件(`record+0x150` と `+0x36c`)。Insert 0 の slot が呼ばれない理由を、条件の値で切り分ける。

##### 5. 未確認と限界
- 「f_1276ea0 が master(index 0)にも使われる」は、静的には `f_1276ca0` の呼び元(vtable 経由)が全 insert かどうか未確認のため、確定していない。
- 「f_1277940 の関数ポインタの先が加算」は、`.bss` 経由の間接呼びなので未確認。
- 実機(Wine)の値は無い。Insert 0 が f_cf2a90 を呼ばない理由は、この読みでは「record 0 の条件値」か「slot 配列の空」のどちらかで、どちらかは観測していない。

### 26.13 右クリックの別経路: 静的な再検証(2026-11-08 06:5x–07:0x)と §26.5.2 の訂正

**先頭(実際に実行した物)**: 静的な読みのみ。Chrome・Wine・cargo は走らせていない。実機の run は 0 本。道具は `tools/a22callers.py`、`tools/a22refs.py`、`tools/a22dis.py`、新規 `tools/a22fnof.py`(VA を .pdata の関数範囲に写す。読み取りのみ)。結論は仮説を出る範囲に限る。

##### 1. §26.5.2 の訂正(私の誤り)
- 「`f_845950` の呼び元は `f_8459ed`」は**誤り**。`0x8459ed` は `f_845950`(範囲 [0x845950,0x845a59))の**内側**の番地で、関数ではない。`f_845950` への直接 call は **0 件**。
- 「`f_81d040` の呼び元は `f_81d2a0`(0x81d2f5 から一度)」は正しい。`f_81d2a0` の呼び元は **`f_108e120`**(0x108e1b1 の 1 件。`0x108e1b1` は pdata で見ると `f_108e120` の中)。旧記述の `f_108e0d0` は誤り。`f_108e120` の直接呼びは 0 件(vtable 経由と読む。未確認)。
- 原因: 旧の `a22callers.py` は関数の開始を「CC の後ろ向き走査」で推定していて、誤った名前を出した。新しい `tools/a22callers-pdata.py`(pdata 基準)で再検証した。上の 3 点はすべて新ツールの出力で確認した。
- `f_83d680`(0x83d6a6 の CreatePopupMenu 呼び)は 107 B の小関数で、`f_81d040` とは別。
- `f_8459ed` の文字列は §26.5.2 の 1 つ目の記述の誤写。TrackPopupMenu の直接呼びは `0x845a49`(= `f_845950` の中)と `0x81d1b5`(= `f_81d040` の中)の 2 件。`0x845a49` の呼び元は関数 `f_845950` 自身(と `f_8459ed` の表記は誤り)。

##### 2. `f_845950` の呼ばれ方(新しい読み)
- `f_845950` の abs64 参照は 2 箇所: `0x83b2e0` と `0x83b4a5`。
- `0x83b2e0` は **8 バイト境界の code pointer 表(37 エントリ、先頭 `0x83b1c0`)の最後のエントリ**。近傍に Delphi の型名らしい文字列 `TPopupMenu` 系(`0x83b3ea` の `TPopupMenuF`、`0x83b501` の `TPopupMenu8`)がある。**ただし、この表が `TPopupMenu` の vtable だと確定する証拠(表の先頭の vtable 判定、クラス名との対応)は、まだ取っていない**。したがって「`f_845950` は `TPopupMenu` の仮想メソッド」は**仮説**(強いが未確定)。表の 36 番目のスロットという番号も、その仮説の上での数え方。
- `0x83b4a5` は 8 バイト境界からずれた位置で、`0x845950` の下位 5 バイトが読めているだけ。vtable ではなく、パック型の RTTI か何かの断片と読めるが、**確定していない**(未確認)。
- したがって、§26.5.2 の「`f_845950` が自前で TrackPopupMenu を呼ぶ 2 つ目の経路」は、**`TPopupMenu` の仮想メソッドとして呼ばれる経路**に読み替える。その呼び元(仮想呼びの相手)は、静的には追えていない。

##### 3. 静的に確定したこと・確定していないこと
- 確定(静的): `TrackPopupMenu` の直接呼びは 2 件(`f_81d040` 内 `0x81d1b5`、`f_845950` 内 `0x845a49`)。`CreatePopupMenu` の直接呼びは 2 件(`f_81d040` 内 `0x81d0ec`、`f_83d680` 内 `0x83d6a6`)。`VK_RBUTTON` を直接に GetKeyState へ渡す箇所は無い(21 件の ecx は 0x10/0x11/0x12/0xa5)。
- 未確定: どの経路が**実機で右クリックメニューを出すか**。WM_CONTEXTMENU を受けたかどうかは、静的には分からない。仮想呼びの相手が実際に呼ばれるかは、実行の観測が要る。
- 候補(改訂): (A') `TPopupMenu` の仮想メソッド(`f_845950`)が、FL のコントロールの mouse-up 処理から仮想呼びで呼ばれ、そこで TrackPopupMenu へ行く。(A'') `f_81d040` 経路(UI コントロールの vtable 関数 → `f_81d2a0` → `f_81d040`)。(B)(C)(D) は §26.5.2 の旧記述のまま(仮説)。
- 次の観測(実機が要る): `CreatePopupMenu` / `TrackPopupMenu` / `DestroyMenu` の呼び出しを、呼び出し元の VA と一緒に出す(Wine の hook。opt1 の表の形で、`0x425620`(TrackPopupMenu の thunk)と `0x4248f0`(CreatePopupMenu の thunk)に、入口で ra(return address)を出す)。その ra で、A' と A'' を分ける。

### 26.14 fhook9 の expos と WAV の frame の対応(2026-11-08 06:56–07:0x、PM の依頼)

**先頭(実際に実行した物)**: Wine・Chrome・cargo は使っていない。既存データ(opt2 の `fhook9-ins0.tsv` 5,313 行、`fhook9-snd.tsv` 5,637 行、`fhook9.wav` 5,729,968 frame)の読みと集計だけ。道具: 新規 `tools/a22-expos-wav.py`(表 `tables/fhook9-expos-wav-0319.tsv` を出す)。WAV は numpy で読み、区間の peak と非 0 frame 数を測った。数値は測ったもの。

##### 1. expos の定義(確定 / 仮説 / 未確認)
- **確定(表から測った)**: ins0 では、全 18 窓行で `expos == pos`。`pos` は ins0 の**累積 Length**(seq 0〜2999 の 3,000 呼びでは、`pos` が「それまでの全呼びの Length の累積」と 3000/3000 で一致)。
- **確定(表の形)**: ins0 の表は全呼びではない。seq 0〜2999 は全件、seq 3000 以降は 16 呼びに 1 行(gap 16)、seq の上限 39,999 で cap。よって seq 3000 以降の `pos` の増分 2,205 は 16 呼び分の Length の和(16 × 約 138)。
- **仮説(数値で支持)**: `expos` は WAV の frame と同じ基準。根拠は次の 3 点(§3)。
- **未確認**: `expos` の定義が Export 開始(fhook4 の 24 連続の 137/138)と一致するか。表の seq 0〜22 は `expos = -1` の 23 行。これが 24 連続の条件に満たないためかは、確かめていない(私は fhook9 の expos の算出コードを読んでいない。表の値だけで言う)。窓 210,000 以前の expos の付け方は本表の範囲では確かめられない。

##### 2. WAV の区間(測った値。PM の依頼の書き方の訂正)
- `[210342, 210723)`: 381 frame、**全部 0**(非 0 は 0 frame、最大 0)。
- `[210723, 211680)`: 957 frame、**全部非 0**(最大 8417、最初の非 0 は 210723)。
- `[211680, 212500)`: 820 frame、全部非 0(最大 32767 = クリップ)。
- したがって「区間 [210342, 211680) は 1,338 frame の 0」は**誤り**。前半 381 frame だけが 0。私の前の報告(§26.14 ではなく 06:5x 以前の記述)の「1,338 frame の 0」も同じ誤り。
- 補足: WAV の最初の非 0 frame は 172955(|x|≤3 の ±1 ノイズ)。意味のある音(|x|>100)の最初は 175189。210342 より前にも音が 37,164 frame ある。

##### 3. expos を WAV frame と見なす検査(ins0 の 18 窓行)
表 `tables/fhook9-expos-wav-0319.tsv`(ins0 18 行)の要点: 各行で `ins_in_pk` と、WAV の `[expos, expos+len)` の peak を比べた。

| expos | ins0 入力 peak | WAV peak | 非 0 frame |
|---|---|---|---|
| 210026 | 0.19178 | 0.19177 | 138/138 |
| 210302 | 0.0044952 | 0.00449 | 40/138 |
| 210440 | 0.0 | 0.0 | 0/138 |
| 210578 | 0.0 | 0.0 | 0/137 |
| 210715 | 0.00642786 | 0.00644 | 130/138 |
| 211680 | 0.589702 | 0.58969 | 138/138 |
| 212093 | 1.15816 | 0.99997(クリップ) | 138/138 |

- 18 行中 **16 行で入力 peak が非 0**(0.0 は 210440 と 210578 の 2 行)。PM の数え方と一致。
- 非 0 の 16 行は、WAV の peak と 0.0001 以内で一致(クリップの 3 行を除く)。0 の 2 行は WAV も全 frame 0。**ins0 の窓の expos は、WAV の frame と同じ基準と読める**(仮説として強い)。ただし、一致の根拠は「ins0 の入力 peak と WAV の peak が同じ値」という**数値の一致だけ**で、どの処理が WAV を作ったかは示していない。一致する確率が低いことは言えるが、因果は未確認。
- 212093 以降で入力 1.158 に対して WAV が 0.99997 なのは、WAV の 16 bit クリップ(32767)。入力 peak が 1 を超える点は、Wine 側の浮動小数で、WAV は頭打ちになる(仮説。クリップの扱いは未確認)。
- ins0 の master_before_pk と master_after_pk は、全 18 行で入力と同じ(pass-through)。つまり、この窓では **ins0(master 段)の入力 = 出力 = WAV**。
- したがって、§26.12 の「ins0 は master の段」は、この窓の中では WAV と一致する。

##### 4. snd の窓(324 行)は、expos の対応がそのまま使えない
- snd の `pos` は **ins0 の累積とは別の基準**(窓の行では pos 1,217,642 や 1,837,674 など、expos 210,026 と一致しない)。`pos` は snd 側の累積で、WAV frame ではない。`expos` の列だけを WAV frame として使う。
- snd の窓行で `src_in_pk` が非 0 の 272 行のうち、WAV の `[expos, expos+len)` の peak と 0.01 以内で一致したのは **29 行だけ**。
- 不一致の主な形: `src_insert = 501` と `0` の行は、入力 peak が ins0 の入力と同じ(例: 212093 の 1.15816)で、WAV は 0.99997 に頭打ち。`src_insert = 1` の行は WAV と一致しない(211680 で 0.235 対 WAV 0.5897)。Insert 1 の出力と master の出力の関係は、この表からは読めない。
- 結論: snd の表を WAV の frame に置き換えて master の和を判定するのは、**今の表では不可**(送り元ごとに同じ expos が付き、送り元の出力と WAV の関係が一様でない)。

##### 5. 確定 / 仮説 / 未確認(まとめ)
- 確定(測定): ins0 の全 18 窓行で expos == pos。ins0 の表は seq 3000 以降が 1/16 の間引き・cap 40,000。WAV の [210342,210723) は全 0、[210723,211680) は全非 0。
- 仮説(強い): ins0 の窓の expos は WAV frame と同じ基準(16 の非 0 行が WAV の peak と一致)。
- 未確認: (a) expos の定義が fhook4 の Export 開始(24 連続の 137/138)と一致するか。(b) snd の送り先の対応(どの送り元が master に入るか)。(c) 212093 以降のクリップの扱い。
- 注意: ins0 の入力 = WAV は、**この窓の master の出力が WAV と一致する**ことを示すだけで、master の和が「f_1277940 の送り」か「ins0 の入力」かは、これでは決まらない。

### 26.15 右クリックの ra 観測の番地(opt1 の hook-ra-0319)と、IAT の名前の確認(2026-11-08 07:00)

**先頭(実際に実行した物)**: 静的な読みのみ(pefile と a22callers-pdata.py)。Chrome・Wine は使っていない。opt1 の fhook10 の結果は、この時点で未実行(opt1 が 07:00 に、preflight の swap 条件で止まったと報告)。

- thunk の番地(opt1 の報告と一致): `CreatePopupMenu` thunk VA `0x4248f0`(RVA `0x248f0`)→ IAT `0x1970ae8` → **`user32.dll!CreatePopupMenu`**。`TrackPopupMenu` thunk VA `0x425620`(RVA `0x25620`)→ IAT `0x1970cf8` → **`user32.dll!TrackPopupMenu`**。
- opt1 は「IAT の名前は pefile で解決できなかった(`?`)」と書いたが、通常の import 表(`DIRECTORY_ENTRY_IMPORT`)で解決できる。私の §26.13 の読みと一致する。(私は以前、この 2 本を delay import と誤って書いていた。`0x1976340`/`0x1976308` の GetPhysicalCursorPos/SetPhysicalCursorPos は、別の番地で delay import として解決されるので、混同しないこと。)
- 直接 call 元(pdata 基準の再検証): `0x4248f0` は `f_81d040`(0x81d0ec)と `f_83d680`(0x83d6a6)。`0x425620` は `f_81d040`(0x81d1b5)と `f_845950`(0x845a49)。
- ra 観測の読み方(opt1 の `hook-ra-0319.tbl` の `s0` = 入口の [rsp] = 戻り番地): `s0` が `0x81d0ec` / `0x83d6a6` なら CreatePopupMenu の呼び元、`0x81d1b5` / `0x845a49` なら TrackPopupMenu の呼び元。**この 4 つの戻り番地で、どの関数経路から来たかが決まる**(観測は未実行)。
- 注意: 戻り番地は直接 call の 4 箇所しか指さない。`f_845950` への経路(vtable 経由)の入口は、ra = `0x845a49` になるのは `f_845950` の内側で TrackPopupMenu を呼ぶ時だけで、`f_845950` に入った事実は ra では分からない。`f_845950` に入ったことを見るには、`f_845950` の入口 hook が要る(静的には `0x845950` が vtable(0x83b2e0)に入っていることまで)。

### 26.16 RT-H への取り込みの照合(2026-11-08 07:25、読み取りのみ)

**先頭(実際に実行した物)**: 読み取りだけ(`cmp`、`diff`、`sed` で build 形に変えた比較、`strings`、`ls`)。Chrome・Wine・cargo は走らせていない。mt3 の木とビルドには触れていない。mt3 が示した木: `mt3w-s1/tmp/ri-pre/tree-rtd3/fle_win.c`(06:41 の写し、RT-H の build 開始 06:46:35 より前)と `mt3w-s1/tree-rtd3/lift/runtime/`(現在。07:18 の 0335 を含む)。

- **fle_win.c**: 06:41 の写しと、私のテスト形(tree-o20d の `FLE_TEST_GETENV` → `getenv` に変えたもの)は**完全一致**(diff 0 行)。delta 方式の関数と ShowCursor の条件はこの写しに入っている。
- **fle_diag_g9mix.c**: 現在の木と、私のテスト形(build 形)は**一致**(same)。expos 修正(`g9m_runlen` / `g9m_run_start`)が入っている。
- **fle_win.c の現在の木との差**: 0335(gpu8 のフォーカス診断)の追加行だけ。私の delta の行は、どちらにも同じ形で残っている。
- **fle_callback.c**: 現在の木は私のテスト形と**違う**。違いは mt3 の RT-H の変更(g9_hook を診断 switch(FLE_DIAG_G9F / ATOM / GIL / CENSUS)が 1 のときだけ設置する `g9_diag_on`)。私の差分は `fle_callback.c` を変えていない。この違いは mt3 の (1) に当たる。
- **wasm**: `engine.498095c419b0.wasm` のサイズは 170,941,858 B(mt3 の値と一致)。`fle_gdi_cursor_display_count` の文字列は 1 件(私の delta の参照と整合。ただし文字列の有無だけで、関数本体は確認していない)。`fle_win_host_pointer_set` の文字列は 0 件(名前は残っていない)。

**結論**: RT-H に、私の 2 本(expos 修正の `diag-g9mix-expos-fix.diff`、O20 delta の `0319-o20-delta-pointer.diff`)が入っていることは、build 形の比較で確認した。ただし、wasm の中身の確認は名前の有無と大きさだけで、関数本体の一致は未確認。RT-H の実機(Chrome)での挙動は、gpu1 の run が要る。
