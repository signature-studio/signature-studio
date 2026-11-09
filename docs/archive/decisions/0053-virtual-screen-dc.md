# 0053: 仮想スクリーン DC（GetDC(NULL)、GetDeviceCaps の 7 index、ReleaseDC）を固定値で答える

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。**数値の多くは仮の決定**（下の「出典の確認状況」の表で、一次資料が値を決めないものと、非公式の出典に基づくものを分けた）。実装は `lift/runtime/fle_gdc.h`、`fle_gdc.c`、`lift/crates/fe-lift/src/imports_table.rs`（3 行追記）、テストは `lift/crates/fe-lift/tests/gdc.rs`、調査と実行記録は `docs/investigations/getdc-caller.md`。「一次」は Microsoft Learn（2026-10-21 取得）、「非公式」はそれ以外。

## 背景

harness は `fle: unsupported instruction at 0x424c50`（`user32.dll!GetDC` の thunk）で止まっていた（`virtualalloc-top-down-run.md` §4）。呼び出し元 f_5d19e0 は `GetDC(NULL)` → `GetDeviceCaps(hdc, 90 = LOGPIXELSY)` → `ReleaseDC(NULL, hdc)` で、戻り値をグローバル 0x15f2f74 に保存する（フォントの高さの計算に使われると読める）。続く f_5c4dc0 は `GetDC(NULL)` → `GetDeviceCaps(hdc, 104 = SIZEPALETTE)` → `ReleaseDC` → `CreatePalette`（`getdc-caller.md` §2）。

## 原則

1. ホストの実画面・実 DPI・実環境値は見せない（再現性）。値は仮想で、この決定で固定する。
2. ページから決まらない値は、選択肢・採用理由・撤回条件とともにここに書く。非公式の出典に基づくものは「仮の決定」と明記する。
3. ゲストが使わない index、hWnd が NULL 以外の GetDC、ウィンドウ実体が要る呼び出しは、名前付きの停止（`fle_stop`）にする。値は作らない。

## 一次資料で確認した規則

| ページ | 確認した内容 |
|---|---|
| [GetDC](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdc)（`updated_at` 2024-02-22） | 「If this value is NULL, GetDC retrieves the DC for the entire screen」。成功すれば DC のハンドル、失敗は NULL。共通 DC は `ReleaseDC` を呼ぶ。「The number of DCs is limited only by available memory」。ハンドルは同時に 1 スレッドだけが使え、`ReleaseDC` は `GetDC` を呼んだスレッドから呼ぶ。 |
| [ReleaseDC](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-releasedc) | 「If the DC was released, the return value is 1. If the DC was not released, the return value is zero」。`GetDC` が返した共通 DC は `ReleaseDC` で解放する（`CreateDC` のものは `DeleteDC`）。 |
| [GetDeviceCaps](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getdevicecaps) | 戻り値は要求した項目の値。`BITSPIXEL` は 15 または 16 bpp のとき 16。`SIZEPALETTE`、`NUMRESERVED`、`COLORRES` は「valid only if the device driver sets the RC_PALETTE bit in the RASTERCAPS index」。`NUMCOLORS` は 8 ビット以下で色表の項目数、それより深いと -1。注記: ドライバが情報を出さなければ GetDeviceCaps は計算し、そのとき「fixed DPI (96 DPI)」を仮定する。 |
| [DPI and device-independent pixels](https://learn.microsoft.com/en-us/windows/win32/learnwin32/dpi-and-device-independent-pixels) | 「The base value of DPI is defined as USER_DEFAULT_SCREEN_DPI which is set to 96」。標準の DPI 設定は 96、120、144。`GetDeviceCaps(hdc, LOGPIXELSX)` を DPI の取得に使う例がある。 |
| [Interprocess communication](https://learn.microsoft.com/en-us/windows/win32/winprog64/interprocess-communication)（64 ビット Windows） | 「only the lower 32 bits are significant, so it is safe to truncate the handle … or sign-extend」。対象にウィンドウ（HWND）と GDI オブジェクト（HBRUSH、HPEN）が挙がる。HDC は挙がっていない。 |

## 出典の確認状況（値ごと）

| 項目 | 値 | 出典 | 状態 |
|---|---|---|---|
| index 名 → 数値 | HORZSIZE 4、VERTSIZE 6、HORZRES 8、VERTRES 10、BITSPIXEL 12、PLANES 14、LOGPIXELSX 88、LOGPIXELSY 90、SIZEPALETTE 104 | Learn の GetDeviceCaps は**名前だけ**で数値を書かない。数値は mingw-w64 の `wingdi.h`（`/usr/share/mingw-w64/include/wingdi.h` の 1516〜1541 行付近を読んだ）と、docs.rs の windows-sys の定数ページ 9 件（それぞれ `GET_DEVICE_CAPS_INDEX = <値>u32`）。検索結果の抜粋では ReactOS と Wine の `wingdi.h` も同じ値だった（抜粋を見ただけで、ファイル全体は読んでいない）。 | **非公式。仮の決定**（独立した複数の出典が一致している） |
| DPI | LOGPIXELSX = LOGPIXELSY = 96 | 一次: 上の DPI ページの USER_DEFAULT_SCREEN_DPI = 96、GetDeviceCaps ページの 96 DPI の仮定。ただし「この仮想ディスプレイが 96 でなければならない」とは書かれていない（ユーザー設定で 120、144 になる）。 | 値の 96 は一次資料に基づく**選択** |
| 解像度 | HORZRES 1920、VERTRES 1080 | **なし**。ページは「画面の幅（ピクセル）」「高さ（ラスタ行）」としか書かない。 | 選択（出典なし。**仮**） |
| 色深度 | BITSPIXEL 32 | なし。ページは「隣接する色のビット数」と書く。15/16 bpp で 16 を返す規則だけがある。 | 選択（出典なし。**仮**） |
| 色プレーン | PLANES 1 | なし（ページは「Number of color planes」だけ）。 | 選択（出典なし。**仮**） |
| システムパレット | SIZEPALETTE 0 | 一次: 「valid only if … RC_PALETTE」。パレット装置でない画面での戻り値はページに**無い**（0 とも書かれていない）。 | 選択。ページが決めない値を 0 にした（**仮**）。0 は「ゲストが 16 未満をパレットなしと扱う」ことの裏付けにはなる（`getdc-caller.md` §2 の読み）が、実 Windows の戻り値は未確認 |
| DC ハンドルの値 | `0x7FA90000 + 4 * (i + 1)`（i 番目の GetDC）、領域 1 MiB | なし（ページは値を書かない）。一次: 上の 64 ビットのページ（下位 32 ビットが有効、符号拡張）。 | 選択。32 ビットに切り詰めても符号拡張しても元の 64 ビット値に戻る（0x80000000 未満）ことを条件にした。HDC がページの列挙に無いので、同じ規則が HDC に当てはまるかは**推測** |

## 決定

1. **GetDC**: hWnd が NULL のときだけ。新しいハンドルを返す（呼ぶたびに別の値。再利用しない）。hWnd が NULL 以外は `fle_stop`（"hWnd is not NULL: the DC of a window needs a window object"）。last error は変えない（ページは変えるとは書かない）。
2. **ReleaseDC**: hWnd が NULL のときだけ。生きている DC は解放して 1。NULL の hDC と解放済みの DC は 0（ページの「解放されなかった」）。hWnd が NULL 以外、この実行時が発行していない値（別種のハンドル、領域内の未発行の値、領域のずれた値）は `fle_stop`。
3. **GetDeviceCaps**: 生きている DC に対して、次の 7 index だけ。HORZRES 8 → 1920、VERTRES 10 → 1080、BITSPIXEL 12 → 32、PLANES 14 → 1、LOGPIXELSX 88 → 96、LOGPIXELSY 90 → 96、SIZEPALETTE 104 → 0。
   - HORZSIZE 4、VERTSIZE 6（ゲストの呼び出し点にあるが、物理画面の大きさ（mm）で、値の根拠がない）は、番号と名前を出して `fle_stop`（"not decided"）。
   - それ以外の index は `fle_stop`（"index N is not answered"）。index は `int` なので rdx の下位 32 ビットだけを見る。
   - 生きていない DC（NULL、解放済み、別種のハンドル）は `fle_stop`（ページは失敗値を書かない）。
4. **スレッド**: 最初の呼び出しのスレッドだけ（ページ: DC は 1 スレッドが使う、ReleaseDC は GetDC のスレッド）。ほかのスレッドから呼ぶと `fle_stop`。`fle_host_tid()`（0045）を使う。
5. ホストの画面・DPI・環境・時計・ファイルは読まない（テスト `source_uses_no_host_display_window_clock_thread_process_or_file`）。

## 選択肢

- 解像度: A. 1920×1080（採用）。B. 1024×768 や 800×600。C. 640×480（最小）。**採用理由**: 値はページから決まらない。ゲストが解像度を使う箇所は、この初期化 unit では見つかっていない（HORZRES と VERTRES は他の呼び出し点で使われるが、閉包の外）。再現性だけが要件なので、現行の一般的な値で固定する（根拠にならない選択であることは上に書いた）。
- DPI: A. 96（採用）。B. 120 や 144（ユーザー設定で実在する値）。**採用理由**: 一次資料が「基準値 96」「ドライバが答えなければ 96 DPI を仮定」と書く唯一の値。
- 色深度: A. 32（採用）。B. 24。C. 8（パレット装置）。**採用理由**: 8 ビット以下にするとゲストが `GetSystemPaletteEntries` の経路（SIZEPALETTE ≥ 16）へ入り、パレット装置の実体が要る。32 なら通らない。24 と 32 の差は、ゲストが BITSPIXEL を使う箇所がまだ見つかっていないので、決める根拠がない。
- SIZEPALETTE: A. 0（採用）。B. 256。**採用理由**: B にすると RC_PALETTE の応答（RASTERCAPS、index 38）を一緒に決める必要がある。ページは非パレット装置の戻り値を書かないので、0 は選択。
- 窓付きの GetDC: A. 名前付きで停止（採用）。B. 仮想ウィンドウ表を作って対応。**採用理由**: 仮想 OS（0014）にウィンドウ実体がない。閉包の中の GetDC の呼び出し 5 箇所は hWnd を別の経路で作る（`getdc-caller.md` §2）ので、実行で届くかは見えるまで分からない。
- DC ハンドル: A. `0x7FA90000` 付近（採用）。B. 他のハンドル領域と同じ `0x7FFAxxxxxxxx`。**採用理由**: 64 ビットのページに従い 32 ビット内に収める。他の領域（ファイル、イベント等）と値が重ならない。

## 撤回条件

次のどれかが見えたら見直す。

- 実 Windows、または Microsoft の別の文書が、index の数値（Learn の SDK ヘッダの引用）や、非パレット装置の SIZEPALETTE の戻り値、画面の既定値を示し、上の表と食い違う（特に SIZEPALETTE が 0 以外、index の数値が違う）。
- ゲストが HORZRES、VERTRES、BITSPIXEL、PLANES、LOGPIXELSX を使う箇所に届き、その動作が選択した値に依存する（レイアウトの崩れ、停止）。そのときは、その箇所の読みに基づいて値を決め直す（解像度、色深度）。
- ゲストが HDC を 32 ビットに切り詰めて使う、または HDC を別の値の範囲（ポインタらしい値）として検査する。ハンドルの領域を変える。
- HORZSIZE または VERTSIZE に実行が届く。物理寸法を決めるか（例: 96 DPI から逆算）、その箇所の用途を読んで別の扱いにする。
- hWnd が NULL 以外の GetDC に実行が届く。窓の表（0014 の範囲を超える）を設計する。

## 確認できていないこと

- 実 Windows が GetDC(NULL)・GetDeviceCaps で返す値（比較していない）。
- 上の index の数値が、現行の Windows SDK のヘッダと同じか（Learn のページは数値を書かない。mingw-w64 と windows-sys の一致だけ）。
- 閉包の外のゲストのコードが GetDeviceCaps の値をどう使うか（HORZRES、VERTRES、BITSPIXEL、PLANES、LOGPIXELSX の使われ方）。
- 値を変える決定（解像度、色深度）が後の API（CreateCompatibleBitmap、CreateDIBSection、BitBlt）の仮想実装に与える影響。
