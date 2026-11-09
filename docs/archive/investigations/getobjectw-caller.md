# GetObjectW の呼び出し元（f_5c0bc0）と lfPitchAndFamily の調査

実行日: 2026-10-22。作業ディレクトリ `signature-studio-develop`、HEAD e3f54dc（決定 0066 を含む）に未コミットの変更を載せた状態。決定は `docs/decisions/0068-virtual-getobject.md`（GetObjectW）と `0067-system-font-fields.md`（フォントのフィールド）。前の作業者の調査 `next-stop-getobjectw.md` と重複する事実（停止点、f_5c0bc0 の概要、26 か所の c の表）は参照だけにし、ここでは**確認し直した点とテストで固定した点**を書く。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` と DLL のバイト列の静的な読み、「固定」は `tests/gdi.rs` §11 がバイト列を assert するもの、「非公式」は Learn 以外。成果物は `lift/out-go/`（gitignore 済み）。

## 1. Learn の GetObjectW（2026-10-22 取得、`updated_at` 2024-11-20、`ms.date` 2022-08-08）

URL: https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getobjectw

- 書式 `int GetObjectW(HANDLE h, int c, LPVOID pv)`。**パラメータ h、c、pv の説明は空**。
- 戻り値（引用）: 「If the function succeeds, and *lpvObject* is a valid pointer, the return value is the number of bytes stored into the buffer.」「If the function succeeds, and *lpvObject* is **NULL**, the return value is the number of bytes required to hold the information the function would store into the buffer.」「If the function fails, the return value is zero.」
- Remarks（引用）: 「The buffer pointed to by the *lpvObject* parameter must be sufficiently large to receive the information about the graphics object. Depending on the graphics object, the function uses a BITMAP, DIBSECTION, EXTLOGPEN, LOGBRUSH, LOGFONT, or LOGPEN structure, or a count of table entries (for a logical palette).」「If *hgdiobj* is a handle to a font, the LOGFONT that is returned is the LOGFONT used to create the font. If Windows had to make some interpolation of the font because the precise LOGFONT could not be represented, the interpolation will not be reflected in the LOGFONT.」「If *hgdiobj* is a handle to a logical palette, GetObject retrieves a 2-byte integer that specifies the number of entries in the palette.」
- **書かれていない**: どの入力が「失敗」か（無効なハンドル、削除済み、c が小さい、c が大きい）、c が構造体より小さいときに何バイト書くか、last error、ブラシ・ペン・stock のフォントの値。パラメータ名がページで `h/c/pv` と本文の `hgdiobj/lpvObject` で食い違う（ページの不整合。意味は同じ）。
- 判断（0068）: ページが書く形（フォント、LOGFONT、戻り値はバイト数）だけ答え、書かない形は停止する。

## 2. f_5c0bc0 が lf を読む箇所（読み。バイト列は `tests/gdi.rs` §11 で固定）

f_5c0bc0(rcx = 出力オブジェクト `[rbp+0x28]`、rdx = hfont `[rbp+0x30]`)。lf は `[rbp+0x144]`（92 バイト）。**前の作業者の表（`next-stop-getobjectw.md` §2）は、バイト列と照合して全項目が正しかった**。テスト `f_5c0bc0_asks_for_0x5c_bytes_and_reads_only_these_fields_of_the_logfont` が次を固定している。

| 箇所 | 命令（バイト列） | 意味 |
|---|---|---|
| 0x5c0bf2 | `48 8b 45 30 48 85 c0 0f 84 01 01 00 00` | hfont が NULL なら 0x5c0d00（GetObjectW を呼ばない） |
| 0x5c0bff〜0x5c0c1b | `48 8b 4d 30 ba 5c 00 00 00 4c 8d 85 44 01 00 00 e8 6c 36 e6 ff 85 c0 0f 84 e4 00 00 00` | `GetObjectW(hfont, 0x5c, &lf)`、戻りが 0 なら 0x5c0d00（**既定値のまま**） |
| 0x5c0c1c | `8b 85 44 01 00 00 48 8b 4d 28 89 41 08` | lfHeight（+0x00）→ `[obj+8]` |
| 0x5c0c29〜0x5c0c3c | `81 bd 54 01 00 00 bc 02 00 00 7c 08` + `48 8b 45 28 80 48 11 01` | lfWeight（+0x10）`>= 0x2bc`（700）なら `[obj+0x11] \|= 1` |
| 0x5c0c3d / 0x5c0c4e / 0x5c0c5f | `80 bd 58 01 00 00 01`（+0x14）、`80 bd 59 01 00 00 01`（+0x15）、`80 bd 5a 01 00 00 01`（+0x16） | lfItalic、lfUnderline、lfStrikeOut が `== 1` なら `\|= 2 / 4 / 8` |
| 0x5c0c70 | `48 0f b6 85 5b 01 00 00 48 8b 4d 28 88 41 12` | lfCharSet（+0x17）→ `[obj+0x12]` |
| 0x5c0c7f | `48 8d 4d 38 48 8d 95 60 01 00 00 41 b8 20 00 00 00 e8 ab 3c e5 ff` | lfFaceName（+0x1c、0x20 ユニット）→ f_414940（→ f_41c570 → f_413940 で `[obj+0x13]` へ 0x7c バイト） |
| 0x5c0cb6〜0x5c0cdf | `48 0f b6 85 5f 01 00 00 24 0f 2c 01 84 c0 74 10 2c 01 84 c0 75 14` と 3 つの枝 | lfPitchAndFamily（+0x1b）`& 0xf`: **1 → `[obj+0x10] = 2`**、**2 → `1`**、**それ以外（0 を含む）→ `0`** |
| 0x5c0cf3 | `8b 85 50 01 00 00 48 8b 4d 28 89 41 0c` | lfOrientation（+0x0c）→ `[obj+0xc]` |

- **読まないフィールド**（固定）: lfWidth（`[rbp+0x148]`）、lfEscapement（`[rbp+0x14c]`）、lfOutPrecision（`[rbp+0x15c]`）、lfClipPrecision（`[rbp+0x15d]`）、**lfQuality（`[rbp+0x15e]`）**。f_5c0bc0 のコード範囲に、これらのオフセットの 4 バイト（disp32）が無いことをテストで確認した。**lfQuality の更新（0067）はゲストの今の分岐を変えない**。
- **分岐を決める値**（0066/0067 の値で）: lfWeight 400 → 太字なし。lfItalic/Underline/StrikeOut 0 → フラグなし。lfPitchAndFamily 0 → `[obj+0x10] = 0`。lfCharSet 1 → `[obj+0x12] = 1`。lfHeight -12 → `[obj+8] = -12`。lfFaceName `Segoe UI` → 変換して `[obj+0x13]` へ。**lfPitchAndFamily を 0x22 にすると `[obj+0x10]` は 1 になる**（低位 4 ビット = 2）。つまり、PM の問い（0x22 にするか）はゲストの分岐に**効く**。§3 で調べた結果、0x22 の裏付けは見つからなかった。

## 3. lfPitchAndFamily（と他のフィールド）の調査

### 3.1 Learn（2026-10-22 再取得）

[LOGFONTW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-logfontw)（`updated_at` 2025-08-15）。lfPitchAndFamily: 「The pitch and family of the font. The two low-order bits specify the pitch of the font and can be one of the following values. DEFAULT_PITCH, FIXED_PITCH, VARIABLE_PITCH. Bits 4 through 7 of the member specify the font family … FF_DECORATIVE, FF_DONTCARE, FF_MODERN, FF_ROMAN, FF_SCRIPT, FF_SWISS. The proper value can be obtained by using the bitwise OR (\|) operator to join one pitch constant with one family constant.」表: 「FF_DONTCARE: Use default font.」「FF_SWISS: Fonts with variable stroke width (proportional) and without serifs. MS Sans Serif is an example.」 lfQuality: 「DEFAULT_QUALITY: Appearance of the font does not matter.」「CLEARTYPE_QUALITY: If set, text is rendered (when possible) using ClearType antialiasing method.」

- **数値（DEFAULT_PITCH 0、FIXED_PITCH 1、VARIABLE_PITCH 2、FF_DONTCARE 0、FF_SWISS 0x20、DEFAULT_QUALITY 0、CLEARTYPE_QUALITY 5）は Learn のこのページに無い**。mingw-w64 の `wingdi.h`（非公式。`https://raw.githubusercontent.com/mingw-w64/mingw-w64/master/mingw-w64-headers/include/wingdi.h`、2026-10-22 取得）に `#define DEFAULT_PITCH 0`、`FIXED_PITCH 1`、`VARIABLE_PITCH 2`、`FF_DONTCARE (0<<4)`、`FF_ROMAN (1<<4)`、`FF_SWISS (2<<4)`、`DEFAULT_QUALITY 0`、`CLEARTYPE_QUALITY 5`、`DEFAULT_CHARSET 1`。
- **SPI_GETNONCLIENTMETRICS、SPI_GETICONTITLELOGFONT、NONCLIENTMETRICSW、LOGFONTW のどのページも、システムフォントの lfPitchAndFamily、lfQuality（ほか）の値を書かない。一次資料は無い。**

### 3.2 非公式の出典（検索と取得。2026-10-22。詳細な引用と OS の区別は決定 0067 の表）

実 Windows の値は、`HKCU\Control Panel\Desktop\WindowMetrics` の `CaptionFont`、`IconFont`、`MenuFont`、`MessageFont`、`StatusFont`、`SmCaptionFont`（各 92 バイトの LOGFONTW。REG_BINARY）に保存され、SPI はこれを読んで返す。次の出典で `.reg` のバイト列を読んだ。SPI の戻りの直接のダンプ（NONCLIENTMETRICS を printf したもの）は、検索で**見つからなかった**。

| 出典 | OS | 結果（lfCharSet / lfOutPrecision / lfClipPrecision / lfQuality / lfPitchAndFamily / face） |
|---|---|---|
| Super User 954408（2015）の 2 つの回答（Wayback 2024-04-15） | Windows 10 | 1 / 0 / 0 / **5** / **0** / Segoe UI（height -15 = 120 DPI と、-12 = 96 DPI） |
| PC Astuces フォーラム「Forum Windows 10」2019-03-04（投稿者自身の export 6 フォント） | Windows 10 | 同じ。6 フォントが同じ LOGFONT |
| AskVG（Wayback 2025-01-02）のコメント（J.Stanley の「default “Segoe UI”」の .reg） | Windows 8/8.1/10 | 同じ |
| Windows 11 Forum 2026-03-03 | Windows 11 | 同じ 8 バイト `00 00 00 01 00 00 05 00` |
| PTT 2009-07-03 | Windows Vista | 同じ 8 バイト |
| Wine `dlls/user32/tests/sysparams.c` の `test_LOGFONT_charset`（実 Windows で通る前提のテスト） | Windows（版の記述なし） | SPI_GETICONTITLELOGFONT と SPI_GETNONCLIENTMETRICS の戻りで `lfCharSet == DEFAULT_CHARSET`（**SPI の戻りの値**についての表明） |

- バイト列の読み方の検算（Python で `struct.unpack('<5l8B64s')` した。`lift/out-go/` のシェルで実行）: 出典 1 の 90 バイトを解くと `h=-12 w=0 esc=0 ori=0 wt=400 it=ul=so=0 cs=1 op=0 cp=0 q=5 pf=0 face='Segoe UI'`（出典 3 も同じ）。
- **反対の出典**: Tatsh の gist `font-entry.py`（https://gist.github.com/Tatsh/1d90ab3736f1eae6987015a2fcfe8c98）は引数の既定値が `pitch_and_family = 0b00100010`（FF_SWISS \| VARIABLE_PITCH）、`quality = 0`。これは「別のフォントの .reg を作る」スクリプトの作者の既定で、OS のダンプではない。採らない。
- **取得できなかったもの**: Stack Overflow、Super User、AskVG の直接取得は 403（Wayback 経由で取得）。Reddit の取得は本文が無い。tenforums.com の取得は 403（検索結果の断片だけ。`CaptionFont=hex:f4,ff,ff,ff,...,90 ... 53,00,65,00,67,00,6f,00,65,00,20,00,55,00,49`）。Chromium、Firefox、Qt のソース、テストの期待値に、lfPitchAndFamily、lfQuality の実 Windows の値を書いたものは見つからなかった（検索のみ。ソースを読んでいない）。ReactOS は検索に出たが読んでいない。

### 3.3 結論

- **lfPitchAndFamily は 0 のまま（変更しない）。** PM の仮説（0x22）を支える出典は見つからず、6 つの実 Windows のバイト列は 0 を示す。
- **lfQuality を 0 から 5 に更新**（非公式、仮。決定 0067）。6 出典が一致。
- lfCharSet 1、lfOutPrecision 0、lfClipPrecision 0、-12、400、`Segoe UI` は変更なし。非公式の裏付けが付いた。
- **限界**: すべて「レジストリに保存された値」で、SPI の戻りのダンプではない。「独立」と言えるのは Super User の export、PC Astuces の export、Windows 11 Forum の 3 件程度。AskVG と Super User の回答 2 は写しの可能性がある。

## 4. GetObjectW の 26 か所の呼び出し点（読み。**バイト列を固定**）

`next-stop-getobjectw.md` §3.2 の表を、`call rel32` の走査（.text の `E8` で target が 0x424280 のもの）で再現した。**26 か所が完全に一致**し、c は次のとおり（`tests/gdi.rs` の `GETOBJ_SITES`。各点の直前 16 バイトを固定）。前の作業者が「c が変数」「不明」とした 3 か所を、バイト列で確定した。

| c | 点 |
|---|---|
| 0x5c | 0x5c0c0f（**今回答える 1 か所**） |
| 0x68 | 0x5c5b26、0x5cb2e8、0x5cbafe、0x5cbc23、0x5cbd0d、0x5cd74f、0x5cd96a、0x5cdc03、0x5ce5d5、0x5ce8e9、0x9f248d（11 か所） |
| 0x20 | 0x5c185e、0x5c4504、0x5c5893、0x5c58ab、0x5d043d、**0x6b6800**、0x98b4f6（7 か所。前の表の 0x6b67f9 は 0x6b6800 が正しい） |
| 0x10 | 0x5c1757、0x5c1de4 |
| 0x4 | 0x5c511c、0x5cbc60 |
| 0x400 | 0x6ba762 |
| **0（`xor edx,edx`、`xor r8,r8`）** | **0x5c1739**（サイズ問い合わせ `GetObjectW(h, 0, NULL)`。前の表は「変数」としていた） |
| 変数（`mov edx,[rbp+0x44]`） | 0x5c17dd |

- 前の表の「0x9f2489」は 0x9f248d が正しい（call 命令のアドレス）。「0x6b67f9」も 0x6b6800 が正しい。
- 分かれ方: 答える 1 か所（c = 0x5c）、止まる 25 か所（`count("0x5c") = 1`、0x68 が 11、0x20 が 7、0x10 が 2、0x4 が 2、0x400 が 1、0 が 1、変数が 1）。
- thunk 0x424280 は `ff 25 6a d7 54 01`（`jmp [rip+0x154d76a]`。0x424286 + 0x154d76a = 0x19719f0）で slot 0x19719f0 に飛び、その名前は `gdi32.dll!GetObjectW`（`imports` を DLL から読んで固定）。DLL に GetObject の名前を持つ import は 1 つだけ（GetObjectA は無い）。thunk と slot の絶対 VA、RVA を持つ 4/8 バイトの値は DLL のどこにも無い（間接参照の表は無い）。

## 5. 次の停止点の見通し（推測。実行して確認するのは §6）

GetObjectW を越えると f_5c0bc0 の 0x5c0c90 の f_414940（lfFaceName の変換）、f_41c570、f_413940、そして f_5c0fc0（f_5c0830、f_5c03d0、f_5c0850）。f_8663b0 は status、menu、message、caption の 4 つで同じ f_5c1360 を呼ぶ。続いて f_5c1030 と f_86b240。その先に CreateHalftonePalette（thunk 0x423fd0）と GetSystemMetrics の止まっている index（0057）。

## 6. 再実行（a4-survey。`OUT=lift/out-go WASM=1 bash lift/harness/a4-survey.sh`。生成 C は `lift/out-go/out` に再生成）

実測（`lift/out-go/a4/20261022T043649Z-*`、`out-go/a4-run.txt`）:

```
stop   : fle: LoadIconW: lpIconName is not IDI_APPLICATION (MAKEINTRESOURCE(32512)): not answered
         top of stack: 0x4250f0 f_4250f0
depth  : init_count=115 / 942   last unit: index=114 run_rank=111/895 proc=0x623840 in_progress
         started=111 completed=110 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=627  entries=13850
wasm   : same a4 lines as native
```

- GetObjectW の停止（0x424280）は越えた。スタックは f_4250f0 ← f_867710 ← f_620cd0 ← f_623840 ← … ← 入口。次の停止点は **`LoadIconW(NULL, L"MAINICON")`**（判断材料は `next-stop-loadiconw-mainicon.md`）。GetObjectW の別の形（パレット、c = 4 など）では**止まっていない**。
- native と wasm(node): 出力全文が `calling entry … rsp=` の 1 行を除いて一致（`diff <(grep -v 'rsp=' native) <(grep -v 'rsp=' node)` が空）。
- 生成 C: `unsupported_total` 537 → 536（GetObjectW の thunk が `fle_imp_GetObjectW` を呼ぶ形になった）。
- ビルド: 生成 C 17 部すべて `errors=0 warnings=0`（emcc）。runtime は `fle_gdi.c`、`fle_spi.c` とも `-Werror` の clang と emcc で警告なし。`fle_proc.c` の 1 件の warning（`fle_thread_prio` set but not used）は今回の変更と無関係（最初のコミット 31d0134 からある）。

## 7. 限界

- f_5c0bc0 の読みは静的（lf の各フィールドの分岐）。実行は GetObjectW を越えた後にしか分からない。
- 26 か所のうち、c の値を確認したのは直前のバイト列だけで、h（hfont など）の出どころは追っていない。0x5c1739、0x5c17dd、c = 0x10、0x20、0x68 の呼び出し先の h がフォントかパレットかブラシかビットマップかは分からない。
- lfPitchAndFamily、lfQuality の値は SPI の戻りのダンプではない（§3.3）。
