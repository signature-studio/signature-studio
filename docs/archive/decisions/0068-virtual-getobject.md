# 0068: 仮想 GetObjectW（作成したフォントだけ。複製の LOGFONTW 92 バイトを返す）

状態: 作業者（サブエージェント）が決定。方針（選択肢 A: フォントの種別だけ答える。B「全種別を決める」、C「停止のまま」は採らない）は依頼者（PM）。実装は `lift/runtime/fle_gdi.c`、`fle_gdi.h`（表に `GetObjectW` を追加）、`lift/crates/fe-lift/src/imports_table.rs`（1 行追記）、テストは `lift/crates/fe-lift/tests/gdi.rs`（§10、§11）、調査は `docs/investigations/getobjectw-caller.md`。「一次」は Microsoft Learn（2026-10-22 取得）、「読み」は DLL のバイト列の静的な読み（テストで固定したものは「固定」）、「実測」はコマンドの出力、「仮」は作業者の選択。

## 背景

harness は `fle: unsupported instruction at 0x424280`（gdi32!GetObjectW の thunk。IAT スロット 0x19719f0）で止まっていた（`docs/investigations/next-stop-getobjectw.md`）。呼び出し元は f_5c0bc0（0x5c0c0f。`f_5c1360(obj, hfont)` から呼ばれる。hfont は決定 0066 の CreateFontIndirectW が返したフォント、または f_8663b0 の呼び出し元が持つ stock のフォント）。ゲストは `GetObjectW(hfont, 0x5c, &lf)` を呼び、戻りが 0 なら既定値のまま、非 0 なら lf から値を読む。

## 一次資料で確認したこと（2026-10-22 取得。[GetObjectW](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getobjectw)、`updated_at` 2024-11-20、`ms.date` 2022-08-08）

- 書式 `int GetObjectW(HANDLE h, int c, LPVOID pv)`（パラメータの説明は空）。
- 戻り値: 「If the function succeeds, and *lpvObject* is a valid pointer, the return value is the number of bytes stored into the buffer. If the function succeeds, and *lpvObject* is **NULL**, the return value is the number of bytes required to hold the information the function would store into the buffer. If the function fails, the return value is zero.」
- Remarks: 「The buffer … must be sufficiently large to receive the information about the graphics object. Depending on the graphics object, the function uses a BITMAP, DIBSECTION, EXTLOGPEN, LOGBRUSH, LOGFONT, or LOGPEN structure, or a count of table entries (for a logical palette).」 フォント: 「If *hgdiobj* is a handle to a font, the **LOGFONT** that is returned is the **LOGFONT** used to create the font. If Windows had to make some interpolation of the font because the precise **LOGFONT** could not be represented, the interpolation will not be reflected in the **LOGFONT**.」 パレット: 「retrieves a 2-byte integer that specifies the number of entries in the palette」。
- **書かれていないこと**: どの形が「失敗」か（無効なハンドル、c が小さい、c が大きい）、c が構造体より小さいときの切り詰め、last error の値、ブラシ・ペン・stock のフォントの LOGFONT の値。

## 決定

### 答える形（1 つだけ）

`GetObjectW(h, c, pv)` は、次の**すべて**を満たすときだけ答える。

1. `h` が表の**生きている作成フォント**（`CreateFontIndirectW` が返し、`DeleteObject` していない。種別 `FLE_GDI_KIND_FONT`）。
2. `c`（`edx` の下位 32 ビット。`int`）が **0x5c** = sizeof(LOGFONTW) = 5×4 + 8 + 32×2 = 92（LOGFONTW のページの定義から計算）。
3. `pv`（`r8`）が非 NULL。

答え: 表が複製した LOGFONTW の 92 バイトを `pv` に書き、戻り値 **92**（Learn: 「the number of bytes stored into the buffer」）。複製は CreateFontIndirectW の時点の内容のままで（Learn: 「the LOGFONT used to create the font」）、書いた後も表は変わらない（何度聞いても同じ。テスト `each_font_gives_its_own_copy_and_the_table_is_not_changed_by_asking`、`getobjectw_adds_no_new_state_and_leaves_the_other_apis_alone`）。書き込みは `fle_st` で 4 バイトずつ 23 回（native では素の store、wasm では範囲検査付き）で、92 バイトの外は触らない（テスト `only_the_low_dword_of_rdx_is_c_and_only_92_bytes_are_written`）。**last error は触らない**（失敗しない呼び出しなので。0066 の CreateFontIndirectW と同じ）。戻り番地（スタック上）と、`rax` 以外のレジスタ（callee-saved の `rbx rbp rsi rdi r12–r15` を含む）は変えない（テスト `the_created_font_is_answered_with_its_92_byte_copy_and_92`）。スレッド規則は 0054 と同じ（最初の呼び出しのスレッドだけ。ハンドルの検査より先に確認する）。

### 停止する形（すべて `fle_stop`。値を創作して続行しない。理由文は形ごとに別）

検査の順は、ハンドル → 種別 → c → pv（先に当たったものを報告する。書き込みは全部の検査の後なので、**停止の前に pv には何も書かない**。テスト `nothing_is_stored_before_a_stop`）。

| 形 | 理由文（`fle: GetObjectW: …`） | 根拠 |
|---|---|---|
| NULL、DC、他の領域、領域内の未発行の値、ずれた値 | `handle was not issued by this runtime's GDI table (NULL, a DC, another object kind or a number never issued): the page says only that failure returns zero` | ページは「失敗は 0」としか書かず、どのハンドルが失敗かを書かない。0 を返さず停止する（0054 の規則「ページが書かない形は停止」） |
| 削除済みのフォント | `handle was deleted by DeleteObject: the page does not say what GetObject returns for it` | DeleteObject のページは削除後のハンドルを「no longer valid」とするが、GetObject の結果は書かない |
| stock の SYSTEM_FONT（`GetStockObject(13)`） | `handle is the stock SYSTEM_FONT: its LOGFONT is not decided (decision 0066 gives a value only to the fonts CreateFontIndirectW made)` | 表の stock の slot に LOGFONT の複製は無く、値が決まっていない。決めると 0066 の仮の値に別の仮を重ねる |
| パレット | `handle is a palette: the page says GetObject stores a 2-byte entry count, the guest's calls (c = 4) are not decided` | ページは 2 バイトの個数とだけ書く。ゲストの c = 4 の意味（2 バイトなのに 4）は分からない。パレットは別の判断 |
| stock のブラシ | `handle is a stock brush: its LOGBRUSH is not decided` | NULL_BRUSH の LOGBRUSH の値はページに無い |
| stock のペン | `handle is a stock pen: its LOGPEN is not decided` | BLACK_PEN の LOGPEN の値はページに無い |
| アイコン | `handle is an icon: it is not an object kind the GetObject page describes` | アイコンは GDI オブジェクトの一覧（BITMAP…LOGPEN）に無い |
| `c != 0x5c`（`c is 0x<c>, not sizeof(LOGFONTW) = 0x5c: the page does not say what is stored for another c`） | c の値を理由文に入れる（`snprintf`）。ゲストの他の 25 の呼び出しの c（0x68、0x20、0x10、0x4、0x400、0、変数）、0x5b、0x5d、極値 | ページは c が構造体より小さい・大きいときを書かない。切り詰め（`min(c, 92)`）を**創作しない** |
| `pv == NULL` | `pv is NULL: the page returns the byte count for it, the guest never passes NULL for a font` | ページは必要バイト数（92）を返すと書くが、ゲストはフォントにそう呼ばない（c が 0x5c のとき pv は常に `lea r8,[rbp+0x144]`。固定）。答えられるが、使われない形は作らない |

ゲストの `GetObjectW(h, 0, NULL)`（0x5c1739。サイズ問い合わせ）と `c = 0x10` の呼び出しなどは、h がフォントでもパレットでもブラシでも、`c` の検査（または種別の検査）で止まる。**それらが次の停止点になる**（`docs/investigations/next-stop-*.md` に判断材料だけ書く）。

### 複製の扱い

0066 の表の LOGFONTW 複製を、そのまま `pv` に返す。**複製は書き換えない**（`lfFaceName` の整形も、DPI の写しもしない。Learn: 「the interpolation will not be reflected」）。5 つのフォントがすべて同じ LOGFONTW（0066、0067）なので、f_5c0bc0 が読む値は決定 0066/0067 の値と同じになる。ゲストが CreateFontIndirectW に渡した LOGFONTW が別の値なら、その値が返る（ホストの値は混ざらない）。

## 選択肢

- **A. フォントの種別だけ答える（採用。PM の決定）**。0066 の複製を使う。実装は小さい。
- B. すべての種別を決める。ブラシ、ペン、ビットマップ、DIBSECTION（c = 0x68）の内容を決める。表に内容が無く、0054 の「CreatePen、CreateSolidBrush などは作らない」と矛盾する。不採用。
- C. 停止のまま。f_5c0bc0 の後（f_414940 など）を見られない。不採用。
- c の扱い: A. `c == 0x5c` だけ（採用）。B. `c >= 0x5c` で 92 を返す。C. `min(c, 92)` を切り詰めて返す。**採用理由**: ページが c の規則を書かない。ゲストが使う c は 0x5c だけ。
- 戻り値: A. 92（採用。Learn の「the number of bytes stored」）。B. 構造体の実サイズ。同じ値。
- pv が NULL: A. 停止（採用）。B. 92 を返す（ページの規則）。**採用理由**: ゲストが使わない形を創作せず、使うときに見る（Learn は答えを書くので、後で答えてよい）。

## 撤回条件

- ゲストが c = 0x5c で、パレット、ブラシ、ペン、stock のフォントに GetObjectW を呼ぶ（名前付きの停止が出る。そのとき決める）。
- ゲストが `GetObjectW(hfont, 0, NULL)`（0x5c1739）や c ≠ 0x5c の呼び出しに届く（同上。ページがバイト数を返すと書く形は、判断が要る）。
- Microsoft の文書が、c が構造体より小さい・大きいときの結果、または失敗の条件を書いた。
- 0066/0067 の LOGFONTW の値が更新された（複製は CreateFontIndirectW に渡された値なので、SPI の値の変更は自動で追従する）。

## 限界

- 実 Windows の GetObjectW とは比べていない（Windows 10 の戻り値、last error の副作用）。
- 同一のスレッドからの呼び出しだけを検査する（0054 の規則）。
- LOGFONTW だけを返す。DIBSECTION、BITMAP の呼び出し点（25 か所のうち 20 か所は c = 0x68 と 0x20）は止まる。
- pv の範囲検査は wasm の `fle_addr` に任せる（native は素のポインタ）。
- 呼び出し点のうち、c が変数の 0x5c17dd と、c = 0 の 0x5c1739 は、実行で届いていない（静的な読みだけ）。

## 0054、0066、0067 との関係

0054 の該当節に「0068 で GetObjectW を追加」を追記した（本文は消さない）。0066 の複製と表をそのまま使う。0067 の値の更新は複製に自動で反映される。
