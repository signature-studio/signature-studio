# 次の停止点: gdi32!GetObjectW の調査と判断材料（実装していない）

実行日: 2026-10-22。作業ディレクトリ `signature-studio-develop`、HEAD d660dc5 に決定 0066 の変更を載せた状態。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の静的な読み、「推測」は未検証。成果物は `lift/out-sf/`（gitignore 済み）。

**実装していない。** 理由: PM の指示（追加してよい API は SPI の 2 つの uiAction と CreateFontIndirectW まで。GetObjectW は次の停止点として出てきても実装しない）。

## 1. 停止点（実測。native と wasm は rsp= の行を除いて完全一致）

```
stop   : fle: unsupported instruction at 0x424280      (gdi32!GetObjectW の thunk。IAT スロット 0x19719f0)
depth  : init_count=115 / 942   last unit: index=114 proc=0x623840 in_progress   started=111 completed=110
funcs  : executed (distinct)=606  entries=13545
```

スタック: f_424280（thunk、guest-return 0x5c0c14）← f_5c0bc0（0x5c1378）← f_5c1360（0x86644f）← f_8663b0（0x865379）← f_8651a0 ← f_620cd0 ← f_623840 ← f_4120c0 ← f_412160 ← f_41d8f0。

つまり、決定 0066 で SPI の 2 つの uiAction と CreateFontIndirectW を越えて、f_8663b0 の最初の `f_5c1360([obj+0x104], hfont)`（アイコンタイトルのフォント。0x866447）に入り、その中の f_5c0bc0 が GetObjectW を呼んで止まった。a4 は 115 / 942 のまま（unit 114 の中）。

## 2. f_5c0bc0 の読み（0x5c0bc0〜0x5c0d18）

f_5c1360(rcx = フォント情報オブジェクト、rdx = hfont) は、f_5c0bc0(&local, hfont) を呼び、その結果を f_5c0fc0 に渡す。f_5c0bc0:

1. 18 個の QWORD + 1 個の DWORD（0x94 バイト）を静的な 0x13d6a74 から rcx のオブジェクトにコピー（既定値）。
2. hfont（rdx）が NULL なら終わり。
3. `GetObjectW(hfont, 0x5c, &lf)`（0x5c0c03〜0x5c0c0f。**c = 0x5c = sizeof(LOGFONTW)**、lf は `[rbp+0x144]`）。戻り値が 0 なら終わり（既定値のまま）。
4. 戻り値が非 0 のとき、LOGFONTW から次を**読む**（lf = `[rbp+0x144]`）:

| LOGFONTW のフィールド | lf のオフセット | ゲストの使い方 |
|---|---|---|
| lfHeight | +0x00（0x144） | `[obj+8]` に写す |
| lfOrientation | +0x0c（0x150） | `[obj+0xc]` に写す |
| lfWeight | +0x10（0x154） | `>= 0x2bc`（700）なら `[obj+0x11] \|= 1`（太字） |
| lfItalic | +0x14（0x158） | `== 1` なら `[obj+0x11] \|= 2` |
| lfUnderline | +0x15（0x159） | `== 1` なら `\|= 4` |
| lfStrikeOut | +0x16（0x15a） | `== 1` なら `\|= 8` |
| lfCharSet | +0x17（0x15b） | `[obj+0x12]` に写す |
| lfPitchAndFamily | +0x1b（0x15f） | **低位 4 ビットが 1（FIXED_PITCH）なら `[obj+0x10] = 2`、2（VARIABLE_PITCH）なら `1`、それ以外は `0`** |
| lfFaceName | +0x1c（0x160）、0x20 ユニット | f_414940 / f_41c570 / f_413940 で `[obj+0x13]`（最大 0x7c バイト）へ変換して写す |

lfWidth、lfEscapement、lfOutPrecision、lfClipPrecision、lfQuality は読まない（この関数では）。

## 3. 判断材料

### 3.1 決定 0066 の仮の値が分岐に効く箇所（実測ではなく読み）

- **lfPitchAndFamily の低位 4 ビット**: 0066 は 0（DEFAULT_PITCH \| FF_DONTCARE）にした。ゲストは 0 を `[obj+0x10] = 0`（不明）に、2 を `1` に写す。**実 Windows 10 の Segoe UI は VARIABLE_PITCH \| FF_SWISS（0x22）と記憶するが出典なし**で、そのとき `[obj+0x10] = 1` になる（推測）。つまり、0066 の仮の値 0 は、ゲストが読む分岐の結果を実機と変えうる。**0x22 に変えるかは未決**（Learn の LOGFONTW は VARIABLE_PITCH と FF_SWISS の定義は書くが、システムフォントの値は書かない）。
- **lfWeight 400**: 700 以上でないので太字フラグは立たない（caption が太字でないとして実 Windows と同じ可能性が高いが、出典なし）。
- **lfCharSet 1**: `[obj+0x12]` に入る。DEFAULT_CHARSET の数値 1 は非公式（mingw）。
- **lfFaceName**: `Segoe UI` を変換して `[obj+0x13]` へ。
- lfItalic、lfUnderline、lfStrikeOut の 0 は、立つフラグが無い形になる。

### 3.2 GetObjectW の呼び出し点（実測。`call 0x424280` は 26 か所）

c（`mov edx,imm`）の値で分けた（読み。バイト列の固定はまだしていない）:

| c | 呼び出し点 | 想定される構造体（Learn の GetObjectW の Remarks が列挙: BITMAP、DIBSECTION、EXTLOGPEN、LOGBRUSH、LOGFONT、LOGPEN、パレットのエントリ数） |
|---|---|---|
| 0x5c | 0x5c0c0f（今回の停止） | LOGFONTW（92） |
| 0x68 | 0x5c5b26、0x5cb2e8、0x5cbafe、0x5cbc23、0x5cbd0d、0x5cd74f、0x5cd96a、0x5cdc03、0x5ce5d5、0x5ce8e9、0x9f2489 | DIBSECTION（x64 で 0x68 = 104 か。計算で確認していない） |
| 0x20 | 0x5c185e、0x5c4504、0x5c5893、0x5c58ab、0x5d043d、0x6b67f9、0x98b4f6 | BITMAP（x64 で 32） |
| 0x10 | 0x5c1757、0x5c1de4 | LOGBRUSH か LOGPEN（x64 の sizeof は 16 と 16 か。要確認） |
| 0x4 | 0x5c511c、0x5cbc60 | パレット（Learn: 2 バイトの整数。c = 4 の意味は不明） |
| 変数 | 0x5c1739、0x5c17dd | 不明（`mov edx,[rbp+0x44]` など） |
| 0x400 | 0x6ba762 | 不明（大きなバッファ） |

つまり、**GetObjectW を実装するなら、ハンドルの種別（フォント、パレット、ブラシ、ペン、ビットマップ）で分かれる**。0066 の表は LOGFONTW の複製を持つので、フォントへの GetObjectW(h, 0x5c, buf) は表から答えられる（Learn: 「If hgdiobj is a handle to a font, the LOGFONT that is returned is the LOGFONT used to create the font」）。stock のフォント（`GetStockObject(13)`）への GetObjectW は、LOGFONT の値が決まっていない（0054 のスロットには複製が無い）。

### 3.3 Learn の GetObjectW（2026-10-22 取得、`updated_at` 2024-11-20）

- 書式 `int GetObjectW(HANDLE h, int c, LPVOID pv)`（パラメータの説明は空）。
- 戻り値: 「If the function succeeds, and lpvObject is a valid pointer, the return value is the number of bytes stored into the buffer. If the function succeeds, and lpvObject is NULL, the return value is the number of bytes required … If the function fails, the return value is zero.」
- Remarks: 「The buffer … must be sufficiently large」。フォント: 「the LOGFONT that is returned is the LOGFONT used to create the font. If Windows had to make some interpolation … the interpolation will not be reflected in the LOGFONT.」パレット: 「retrieves a 2-byte integer that specifies the number of entries in the palette」。
- **c がバッファより小さいとき、c が構造体より大きいときの結果は書かれない。** 戻り値はバイト数。ページは c の扱い（c が 0x5c より小さい場合に切り詰めるか）を書かない。ゲストは c = 0x5c で、LOGFONTW は 92 なので、一致する形だけなら c の規則は要らない。

### 3.4 選択肢（判断は PM）

- **A. フォントの種別だけ答える**: `GetObjectW(h, c, pv)` を、h が表の作成フォントで c == 0x5c（かつ pv が非 NULL）のときだけ答える（複製の 92 バイトを pv に書いて 92 を返す）。ほかは名前付き停止（パレット、ブラシ、ペン、ビットマップ、stock、c が違う形、pv が NULL）。**実装は小さく、0066 の複製を使う**。次の停止点（f_5c0bc0 の後の f_414940 などのゲスト関数と、CreateHalftonePalette ほか）を見るために必要。
- **B. すべての種別を決める**: ブラシ、ペン、ビットマップは表に内容が無く、決定が要る（0054 の「CreatePen、CreateSolidBrush などは作らない」）。不要な範囲を作ることになる。
- **C. 停止のまま**。

### 3.5 その先の見通し（推測）

GetObjectW を越えると、f_5c0bc0 の 0x5c0c90 の f_414940（lfFaceName の変換）、f_41c570、f_413940、そして f_5c0fc0（f_5c0830、f_5c03d0、f_5c0850）。その後、f_8663b0 は status、menu、message、caption の 4 つで同じ f_5c1360 を呼ぶ（0x8664bd、0x8664df、0x866501、0x866520）。続いて `f_5c1030`（0x8665b2 など）と `f_86b240`。その先は `CreateHalftonePalette`（0x1971ba0、thunk 0x423fd0）と GetSystemMetrics の止まっている 22 index（0057）。

## 4. 限界

- GetObjectW の呼び出し点 26 か所のうち、c の値を読んだのは `mov edx,imm` の直前の命令だけ。c が変数の 2 か所と、構造体の種別は、呼び出し元の hfont の出どころを追っていない。
- f_5c0bc0 の後の関数（f_414940、f_41c570、f_413940、f_5c0fc0 の先）は読んでいない。
- DIBSECTION、LOGBRUSH、LOGPEN の x64 の sizeof（0x68、0x10）は Learn のページから計算していない。
