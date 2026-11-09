# 0057: 仮想システムメトリクス（GetSystemMetrics の 14 index）を固定値で答える

状態: 作業者（サブエージェント）が決定。方針（ホストの実画面・実マウス・実 DPI・実テーマをゲストに見せず、値は仮想で固定し、決定として記録する。仕様から決まらない値は「仮の決定」と明記する）は依頼者（PM）。**値の多くは仮の決定**（下の表で、一次資料に基づくもの、一次資料から導いたもの、仕様から決まらない選択を分けた）。実装は `lift/runtime/fle_sysmetrics.h`、`fle_sysmetrics.c`、`lift/crates/fe-lift/src/imports_table.rs`（1 行追記）、テストは `lift/crates/fe-lift/tests/sysmetrics.rs`、調査と実行記録は `docs/investigations/getsystemmetrics-caller.md`。「一次」は Microsoft Learn（2026-10-21 取得）、「非公式」はそれ以外。

## 背景

harness は `fle: unsupported instruction at 0x424ee0`（`user32.dll!GetSystemMetrics` の thunk）で止まっていた（`msctf-caller.md` §6）。呼び出し元は f_620860 の `GetSystemMetrics(19 = SM_MOUSEPRESENT)` で、結果を `[obj+0x10]` にブール値で保存する。隣の f_620880 は `GetSystemMetrics(0x4b = 75 = SM_MOUSEWHEELPRESENT)` で、結果を `[obj+0x28]` に保存し、真のとき `SystemParametersInfoW(0x68 = SPI_GETWHEELSCROLLLINES)` を呼ぶ。DLL 全体の呼び出し点は 54 か所、静的な index は 36 種（`getsystemmetrics-caller.md` §2）。

## 原則

1. ホストの実画面・実マウス・実 DPI・実テーマは見せない（再現性）。値は仮想で、この決定で固定する。
2. 一次資料に書かれていない値は、事実としては書かない。実装に入れる必要があるものは「仮の決定、出典なし、作業者の選択」と明記する。必要がないものは実装せず、名前付きの停止にして理由を書く。
3. ゲストが使わない index、値を決めていない index は、`fle_stop` で止まる。値は作らない。
4. 画面の値は決定 0053 の仮想画面と名前で結びつける（`FLE_GDC_HORZRES`、`FLE_GDC_VERTRES`）。数字の複製を持たない。

## 一次資料で確認した規則

| ページ | 確認した内容 |
|---|---|
| [GetSystemMetrics](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmetrics)（`updated_at` 2025-07-01） | **表が各 SM_* の名前と十進の数値を併記している**（依頼文は「Learn は数値を書かないことが多い」としていたが、このページは書く）。0x4b = 75 = **SM_MOUSEWHEELPRESENT**（「Nonzero if a mouse with a vertical scroll wheel is installed; otherwise 0」）を公式で確認。戻り値: 成功で要求した値、失敗で 0 で「GetLastError does not provide extended error information」。ページは**既定の寸法（スクロールバー、枠、アイコンなど）の値を書かない**。 |
| [The Virtual Screen](https://learn.microsoft.com/en-us/windows/win32/gdi/the-virtual-screen) | 「The bounding rectangle of all the monitors is the virtual screen」「The primary monitor contains the origin (0,0)」。 |
| [About Icons](https://learn.microsoft.com/en-us/windows/win32/menurc/about-icons)（`updated_at` 2026-09-21） | 「System icon sizes」の表: 96 DPI で SM_CXSMICON／SM_CYSMICON は 16x16、SM_CXICON／SM_CYICON は 32x32。ページは OS を限定せず、DPI だけで書く。 |

## 値ごとの決定（14 index）

「出典」: **一次**＝ページが値を書く。**一次から導出**＝ページの規則と、この環境の別の決定から値が決まる。**選択**＝ページが値を決めない。

| index | 名前（Learn の表の番号） | 値 | 根拠 | 出典 | 仮の決定か | 選択肢 | 撤回条件 |
|---|---|---|---|---|---|---|---|
| 0 | SM_CXSCREEN | 1920 | 「GetDeviceCaps(hdcPrimaryMonitor, HORZRES) と同じ値」。0053 の HORZRES | 番号は一次。**1920 という値は 0053 の選択（出典なし）**。一致の規則は一次 | **仮**（0053 を引き継ぐ） | 1024x768 など。0053 と同じ理由で 1920x1080 | 0053 の解像度が変わったら自動で追従。ゲストがレイアウトでこの値に依存して崩れたら見直す |
| 1 | SM_CYSCREEN | 1080 | 同上（VERTRES） | 同上 | **仮** | 同上 | 同上 |
| 78 | SM_CXVIRTUALSCREEN | 1920 | 仮想画面は全モニタの外接矩形。モニタは 1 つなので画面と同じ | 定義は一次。値は一次から導出 | 導出（モニタ数 1 が選択） | モニタを複数にする（0053 の単一画面の変更が前提） | 0053 が複数モニタを導入したとき |
| 79 | SM_CYVIRTUALSCREEN | 1080 | 同上 | 同上 | 同上 | 同上 | 同上 |
| 76 | SM_XVIRTUALSCREEN | 0 | 「The primary monitor contains the origin (0,0)」。唯一のモニタが外接矩形の左端 | 一次から導出 | 導出 | 負の座標のモニタ配置 | モニタを複数にするとき |
| 77 | SM_YVIRTUALSCREEN | 0 | 同上（上端） | 同上 | 導出 | 同上 | 同上 |
| 80 | SM_CMONITORS | 1 | 0053 の仮想画面は 1 つ（単一モニタ） | 定義は一次。**1 は 0053 の選択** | **仮** | 2 以上 | 0053 が複数画面を導入したとき |
| 19 | SM_MOUSEPRESENT | 1 | ページは「Nonzero if a mouse is installed… This value is rarely zero, because of support for virtual mice」としか書かない。0 にすると、ゲストはマウスなしの経路（`[obj+0x10] = 0`）へ進み、読み手が未追跡なので、どの分岐へ入るか分からない | 定義は一次。**値の 1 は選択（出典なし）。「rarely zero」の一文が 1 を支える傍証にとどまる** | **仮の決定、作業者の選択**（PM の方針でも「マウスあり = 1」は仮） | 0 | `[obj+0x10]` の読み手に実行が届き、1 でレイアウトが崩れる、または別の経路が要る。実 Windows 10 が仮想環境（マウスなし）で 0 を返すと分かった |
| 75 | SM_MOUSEWHEELPRESENT | 1 | ページは既定を書かない。1 にすると f_620880 は SPI_GETWHEELSCROLLLINES（0x68）を呼ぶ。0 だと呼ばずに通る（読み。ただし 0 にして得るものは「停止点を先送りする」だけで、根拠がない） | 定義と 0x4b = 75 は一次。**値の 1 は選択（出典なし）** | **仮の決定、作業者の選択** | 0 | 同上。`SystemParametersInfoW` の実装時に、スクロール行数の既定値（ページは 3 と書く）と整合しない結果が出たとき |
| 11 | SM_CXICON | 32 | About Icons: 96 DPI で 32x32。仮想画面の DPI は 96（0053） | **一次**（表の値）。**「Windows 10 22H2 の値」とまでは言えない**（ページが OS を限定しない） | 一次の値を 96 DPI に当てはめた選択 | 48（150%）など | DPI を 96 以外にしたとき。実 Windows の値と食い違ったとき |
| 12 | SM_CYICON | 32 | 同上 | 同上 | 同上 | 同上 | 同上 |
| 49 | SM_CXSMICON | 16 | 同上（16x16） | 同上 | 同上 | 24（150%）など | 同上 |
| 50 | SM_CYSMICON | 16 | 同上 | 同上 | 同上 | 同上 | 同上 |
| 89 | SM_SERVERR2 | 0 | 「The build number if the system is Windows Server 2003 R2; otherwise, 0」。エミュレート OS は Windows 10（0014） | 一次から導出 | 導出（OS は 0014 の PM の決定） | なし | エミュレート OS を Windows Server 2003 R2 にしたとき |

- 戻り値は `int`。index は `int` なので `rcx` の下位 32 ビットだけを見る。
- last error には触れない（ページ: 「GetLastError does not provide extended error information」）。状態を持たないので、スレッドの検査も要らない。
- テスト `the_decided_values`、`the_screen_agrees_with_the_virtual_display_of_decision_0053`（SM_CXSCREEN と GetDeviceCaps(HORZRES) などの一致）が、画面の整合を固定する。

## 値を決めない（名前付きで停止する）22 index

ゲストの呼び出し点が使う index のうち、一次資料が値を書かず、この環境で決める根拠もないもの。`fle: GetSystemMetrics: index N (SM_xxx) is used by the guest but no value is decided (decision 0057)` で止まる。

| index | 名前 | 使われ方（`getsystemmetrics-caller.md` §2.1） | 値を決めない理由 |
|---|---|---|---|
| 2、20 | SM_CXVSCROLL、SM_CYVSCROLL | スクロールバーの幅・矢印の高さ | 値は Learn のページに無い。**17 が既知の値であることは経験的に知られているが、一次資料で確認できないので使わない**。実装に入れる必要も（この停止点では）ない |
| 3、21 | SM_CYHSCROLL、SM_CXHSCROLL | 同上 | 同上 |
| 10 | SM_CXHTHUMB | スクロールバーのつまみ | 同上 |
| 6 | SM_CYBORDER | 余白の計算 | 同上 |
| 7、8、45、46 | SM_CXFIXEDFRAME、SM_CYFIXEDFRAME、SM_CXEDGE、SM_CYEDGE | 枠の厚さ | 同上 |
| 32 | SM_CXSIZEFRAME | リサイズ枠の厚さ（フレーム幅） | 同上。**テーマと DPI に依存する値**（ページ: 「This API is not DPI aware」） |
| 13、14 | SM_CXCURSOR、SM_CYCURSOR | カーソルの大きさ | 同上 |
| 15 | SM_CYMENU | メニューの高さ | 同上 |
| 61 | SM_CXMAXIMIZED | 最大化ウィンドウの幅 | ページは「primary display monitor」としか書かない。タスクバーの有無などの環境が絡む |
| 68、69 | SM_CXDRAG、SM_CYDRAG | ドラッグ閾値 | 値はページに無い（SPI で設定する値） |
| 86 | SM_TABLETPC | タブレット PC か | ページの定義は Windows XP Tablet PC edition か Windows Vista／7 で Tablet PC Input service が始動している場合。Windows 10 の扱いは書かれていない |
| 94、95 | SM_DIGITIZER、SM_MAXIMUMTOUCHES | タッチ・ペンの有無 | SM_DIGITIZER は「Windows 7 or Windows Server 2008 R2」の定義でビットマスク。Windows 10 の値の規則が書かれていない |
| 0x2003 | SM_CONVERTIBLESLATEMODE | ラップトップ／スレートのモード | ゲストは `sete`（値が 0 のとき 1 を保存）で使う。ページは 0 が Slate Mode で、デスクトップ PC には適用されないと書く。0 を返すとスレートモードの経路へ入る。値が挙動を分けるので、選択の根拠が要る |
| 0x2004 | SM_SYSTEMDOCKED | ドッキングモード | ページは「0 for Undocked Mode and non-zero otherwise」。仮想デスクトップ PC に当てはまる値の根拠が無い |

SM_CXICON など 4 つのアイコン値は、About Icons ページの表が値を書くので答える（上の表）。同じページの「SHIL_*」の値は使っていない。

## 環境状態を問う index

SM_REMOTESESSION（0x1000）、SM_REMOTECONTROL（0x2001）、SM_SHUTTINGDOWN（0x2000）は、ゲストの呼び出し点に**静的には現れない**（54 か所の immediate に無い）ので**実装しない**（番号で停止する。テスト `the_environment_state_indexes_are_not_answered_because_the_guest_does_not_ask`）。ゲストが使う環境状態の index は SM_CMONITORS（80）、SM_SERVERR2（89）、SM_TABLETPC（86）、SM_DIGITIZER（94）、SM_MAXIMUMTOUCHES（95）、SM_CONVERTIBLESLATEMODE、SM_SYSTEMDOCKED の 7 つ。このうち 0053 と整合する固定値を決められるのは SM_CMONITORS = 1 と、OS から決まる SM_SERVERR2 = 0。残りは上のとおり止まる。

## 選択肢（全体）

- A. 14 index に答え、ゲストが使う 22 index と未使用 index は止まる（採用）。
- B. ゲストが使う 36 index 全部に値を決める（Windows 10 の既定値の記憶を使う）。**不採用**: 値の多くは一次資料に書かれておらず、「Windows 10 の既定値は X」と言える根拠がない。画面と同様、値が根拠なしに実装に入ると、後で外せない。
- C. 未使用・未決の index に黙って 0 を返す。**不採用**: 値を創作して続行することになる（0 は「失敗」の戻り値でもある）。
- D. ホストの値を読む。**不採用**: PM の方針（ホストの環境を見せない）に反する。

## 撤回条件

- 実行が、止まった 22 index のどれかに届いた（そのたびに、その index の用途を読み、一次資料が値を書くなら答える。書かないなら、選択として決定に追記する）。
- ゲストが SM_MOUSEPRESENT = 1 または SM_MOUSEWHEELPRESENT = 1 のときの経路（`SystemParametersInfoW(0x68)` ほか）で、仮想マウス・ホイールの実体を要求する。または実 Windows 10 の対応する値が示されて、食い違った。
- 0053 が解像度、DPI、モニタ数を変えた。SM_CXSCREEN などは名前でつながっているので自動で追従するが、アイコン 4 値（96 DPI の値）、SM_CMONITORS、仮想画面の原点は手で見直す。
- Microsoft の別の文書が、既定の寸法（スクロールバー、枠など）を Windows 10 の値として示した。止まっている index を答える側に移す。
- エミュレート OS（0014）が変わった。SM_SERVERR2、SM_TABLETPC などを見直す。

## 確認できていないこと

- 実 Windows が GetSystemMetrics で返す値（比較していない）。
- 14 index の数値は Learn の表で確認した。mingw-w64 の `winuser.h` との照合（参考）で、87 名が一致、不一致 0（4 名は名前の定義が無かった）。値の根拠には使っていない。
- アイコンの 4 値が Windows 10 22H2 の標準かどうか（ページは OS を限定しない）。
- `[obj+0x10]`（マウスの有無）、`[obj+0x28]`（ホイールの有無）、`[obj+0x14]`（スクロール行数）の読み手。
- 間接呼び出し経由の GetSystemMetrics の呼び出し先、動的な index（f_859470、f_859610 の表の添字 b の範囲）。
- 値を変える決定が、後のウィンドウ・レイアウト系 API の仮想実装に与える影響。
