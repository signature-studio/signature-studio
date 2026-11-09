# FLEngine_x64.dll 抽出台帳

実測値のみを記す。推測は「推測」と明記する。実バイナリはリポジトリに含めない（`binaries/` は .gitignore 済み）。

## 1. 素材

| 項目 | 値 | 区分 |
|---|---|---|
| インストーラ | `flstudio_win64_26.1.7.5653.exe`（読み取り専用で使用） | 実測 |
| サイズ | 1,106,033,456 B | 実測（stat） |
| SHA-256 | `04393e7946c7369aaa5cd9ce1c755803e922146289e1ae3263629a44f4e5fc02` | 実測（sha256sum） |
| 形式 | NSIS（Unicode）。FirstHeader signature `0xDEADBEEF` + `NullsoftInst` | 実測 |
| FirstHeader のファイルオフセット | 0x10a00（68,096）。署名スキャンで 1 件のみ検出 | 実測 |
| FirstHeader flags | 0x30 | 実測 |
| inflated header length | 3,519,116 B | 実測 |
| total length（FirstHeader） | 1,105,955,042 | 実測 |
| ヘッダの圧縮 | LZMA（raw、props 5 B）。圧縮長 401,034 B | 実測 |
| 圧縮ヘッダ語の前の 0 パディング | 8 B | 実測 |
| データ領域の開始 | ファイルオフセット 469,170 | 実測 |
| ブロック表 | entries ブロック: offset 36,356、件数 59,904。strings ブロック: offset 2,192,900 | 実測 |
| 1 エントリのサイズ | 36 B（= (2,192,900 − 36,356) / 59,904） | 実測（導出） |
| EW_EXTRACTFILE（opcode 20）のエントリ | 27,574 件 | 実測 |

## 2. 抽出

### 2.1 前任スクリプト `tools/extract_nsis.py` の結果（未修正のまま実行）

- `/usr/bin/python3`（3.14.4）で `--list FLEngine` を実行 → `AssertionError: header not LZMA-compressed`（`read_header`、39 行目）で失敗。
- 原因（実測）: 圧縮ヘッダ語の前に 8 B のゼロパディングがあり、前任スクリプトは FirstHeader の直後 4 B をヘッダ語として読んでいた。ヘッダ語は FirstHeader 末尾から 8 B 後ろ（0x10a1c + 8 = 0x10a24）にあり、値は `0x80061e8a`（bit31 = 圧縮、長さ 401,034）。
- 元のファイルは上書きしていない。

### 2.2 修正版 `tools/extract_nsis2.py`（新規ファイル）

前任版からの変更点:

1. FirstHeader を固定オフセットではなく署名スキャンで探す。
2. FirstHeader 直後のゼロパディング（上限 64 B）を読み飛ばす。
3. エントリサイズを 28 B 固定にせず、ブロック表から導出する（実測 36 B）。28 B 固定では EXTRACTFILE のオフセット・名前フィールドの位置が合わず、名前解決が誤る。
4. 同一名のエントリが複数ある場合、データオフセットが同一なら曖昧とみなさない。
5. ヘッダの展開サイズ検証、EOF 検出を追加。

修正過程の実測:

- 修正 1・2 のみの段階: 3,140 件の EXTRACTFILE が出たが、`FLEngine` を含む名前は 0 件。`engine_x64.dll` 系は 3 件（498,176 B / 248,832 B / 313,344 B）だったが、いずれも FLEngine ではない。サイズ 22,227,016 B のエントリは 3,140 件中 0 件（全件ハッシュ走査）。
- この段階の 28 B 固定は誤り。strings ブロック内には `FLEngine_x64.dll` が 3 か所（string index 818、659421、659441）あり、エントリ表の構造から 36 B/エントリと判明。
- 修正 3 を入れた後: `--list FLEngine` で EXTRACTFILE 27,574 件中 3 件がヒット（エントリ 950、59704、59725）。3 件ともデータオフセット 0x2726ae で同一（インストーラ内で同じ実体を指す）。
- 補助スクリプト `tools/find_nsis_by_size.py` は、名前解決に失敗した場合の代替（サイズ指定で全件走査）として作ったが、上記のとおり名前解決が成功したため、FLEngine の抽出には使っていない。

### 2.3 照合と書き出し

| 項目 | 期待値（事前実測） | 今回の実測 | 結果 |
|---|---|---|---|
| `--hash` サイズ | 22,227,016 B | 22,227,016 B | 一致 |
| `--hash` SHA-256 | `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d` | `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d` | 一致 |
| `--write` 後の `ls -l` | 22,227,016 B | 22,227,016 B | 一致 |
| `--write` 後の `sha256sum` | 同上 | 同上 | 一致 |

書き出し先: `binaries/win64/FLEngine_x64.dll`（新規作成、中間ファイルなし）。

## 3. PE 実測値（`fe-pe pe-info` の出力。独立に書いた Python パーサでも import/IAT を照合）

| 項目 | 値 |
|---|---|
| Machine | 0x8664 |
| Optional header magic | 0x20b（PE32+） |
| Characteristics | 0x2022 |
| TimeDateStamp | 0x6abbb7e3 |
| ImageBase | 0x400000 |
| SizeOfImage | 0x18c0000 |
| SizeOfHeaders | 0x400 |
| EntryPoint RVA | 0xf76820 |
| SectionAlignment / FileAlignment | 0x1000 / 0x200 |
| Subsystem / DllCharacteristics | 2 / 0x0160 |
| e_lfanew / SizeOfOptionalHeader | 0x100 / 240 |
| データディレクトリ数 | 16 |
| セクション数 | 10 |

セクション（VA はいずれも RVA）:

| 名前 | RVA | VirtualSize | RawOffset | RawSize |
|---|---|---|---|---|
| .text | 0x1000 | 0xfa34b0 | 0x400 | 0xfa3600 |
| .data | 0xfa5000 | 0x240008 | 0xfa3a00 | 0x240200 |
| .bss | 0x11e6000 | 0x388e9c | 0 | 0 |
| .idata | 0x156f000 | 0x670e | 0x11e3c00 | 0x6800 |
| .didata | 0x1576000 | 0x1460 | 0x11ea400 | 0x1600 |
| .edata | 0x1578000 | 0x94 | 0x11eba00 | 0x200 |
| .rdata | 0x1579000 | 0x45 | 0x11ebc00 | 0x200 |
| .reloc | 0x157a000 | 0x8c590 | 0x11ebe00 | 0x8c600 |
| .pdata | 0x1607000 | 0xa9620 | 0x1278400 | 0xa9800 |
| .rsrc | 0x16b1000 | 0x20e400 | 0x1321c00 | 0x20e400 |

エクスポート（3 件、すべて名前付き、フォワーダなし）:

| Ordinal | 名前 | RVA | VA |
|---|---|---|---|
| 1 | dbkFCallWrapperAddr | 0x11ec2b0 | 0x15ec2b0 |
| 2 | __dbk_fcall_wrapper | 0x1d5a0 | 0x41d5a0 |
| 3 | CreateFruityInstance | 0xf75990 | 0x1375990 |

インポート: 24 DLL、738 名前インポート、序数インポート 0。

| DLL | 件数 | 先頭 IAT RVA |
|---|---|---|
| shlwapi.dll | 1 | 0x15709c8 |
| opengl32.dll | 2 | 0x15709d8 |
| version.dll | 3 | 0x15709f0 |
| user32.dll | 227 | 0x1570a10 |
| oleaut32.dll | 19 | 0x1571130 |
| advapi32.dll | 14 | 0x15711d0 |
| kernel32.dll | 184 | 0x1571248 |
| wintrust.dll | 1 | 0x1571810 |
| SHFolder.dll | 1 | 0x1571820 |
| crypt32.dll | 7 | 0x1571830 |
| gdi32.dll | 127 | 0x1571870 |
| mpr.dll | 4 | 0x1571c70 |
| winmm.dll | 24 | 0x1571c98 |
| oleacc.dll | 1 | 0x1571d60 |
| wininet.dll | 15 | 0x1571d70 |
| winspool.drv | 5 | 0x1571df0 |
| comdlg32.dll | 2 | 0x1571e20 |
| comctl32.dll | 30 | 0x1571e38 |
| shell32.dll | 22 | 0x1571f30 |
| IMAGEHLP.DLL | 3 | 0x1571fe8 |
| ole32.dll | 29 | 0x1572008 |
| msacm32.dll | 14 | 0x15720f8 |
| api-ms-win-crt-string-l1-1-0.dll | 1 | 0x1572170 |
| dsound.dll | 2 | 0x1572180 |

事前に聞いていた値（サイズ、SHA-256、machine、ImageBase、SizeOfImage、EntryPoint、エクスポート 3 件の VA、24 DLL・738 名・序数 0、セクション 10）はすべて一致した。

## 4. 補足（推測）

- 推測: `.pdata`（0xa9620 B）は x64 の例外ハンドリング用 RUNTIME_FUNCTION 表と思われる。未検証（今回は解析していない）。
- 推測: `.rdata` が 0x45 B しかなく、`.edata`・`.idata`・`.didata`・`.reloc` の並びも含め、Delphi/Embarcadero 系リンカの出力に見える。確認していない。
- 推測: DLL に `api-ms-win-crt-string-l1-1-0.dll!memset` が 1 件だけある理由は未確認。
