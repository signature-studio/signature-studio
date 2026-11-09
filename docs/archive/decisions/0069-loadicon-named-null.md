# 0069: LoadIconW(hInstance = NULL, lpIconName = L"MAINICON") は NULL を返し、last error には触れない（仮の決定）

状態: **仮の決定**。PM が A 案を仮の決定として採用し、作業者（サブエージェント）が実装した。0054 は変更しない（0054 の LoadIconW の節に「0069 で追記」と 1 行足しただけ。本文は消していない）。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」は DLL のバイト列・gdb・コマンド出力、「読み」は `objdump` の静的な読み、「仮」は根拠となるページが無い選択。実装は `lift/runtime/fle_gdi.c` の `fle_imp_LoadIconW`、`fle_gdi.h`（`FLE_GDI_ICON_NAME_MAX_UNITS`）。テストは `lift/crates/fe-lift/tests/gdi.rs`（5b 節）。

## 背景

`docs/investigations/hinstance-caller.md` §0〜§5。harness は `fle: LoadIconW: lpIconName is not IDI_APPLICATION (MAKEINTRESOURCE(32512)): not answered` で止まっていた。引数は `LoadIconW(hInstance = NULL, lpIconName = L"MAINICON")`（0x867934、`f_867710` の中）。hInstance の 0 は `[[0x15e15c8]] = *(u64*)0x15e6078 = 0` で、これは Delphi の `MainInstance`（DLL では 0 が設計どおり）。本物の `HInstance`（0x15ec298）は 0x400000。lift、harness、runtime のどの欠陥でもない（分類 (c)）。判断が要るのは LoadIconW の答え方だけだった。

## 決定

1. `hInstance == NULL` かつ `lpIconName` が整数識別子でなく（IS_INTRESOURCE でない: 上位 48 ビットが非 0）、文字列が UTF-16 の `MAINICON` にちょうど一致するときだけ、**戻り値 NULL（`R[0] = 0`）を返す。last error には触れない**。戻り番地（`[rsp]`）と R[0] 以外のレジスタは変えない。
2. 文字列は `fle_ld` で 2 バイトずつ最大 16 ユニット読む。NUL が 16 ユニット以内に無ければ停止（"longer than 16 UTF-16 units"）。NUL より先は読まない。比較は 1 ユニットずつ完全一致（大文字小文字を区別する。`mainicon` や `MAINICON ` は別の名前）。
3. ほかはすべて従来どおり: `hInstance` が非 NULL は停止（"hInstance is not NULL"。0x400000 の形も同じ）、別の文字列名は停止（"lpIconName is a string other than L\"MAINICON\""）、整数識別子は 32512 だけ（IDI_APPLICATION の形は変えていない）、他の識別子と 0 は停止（"not IDI_APPLICATION"）。スレッドの規則（0054 の 5）は先に効く。
4. 変更前に停止していた `rdx = 0x100000000 + 32512` は、上位ビットが非 0 なので文字列ポインタとして読む。読めないアドレスは文字列かどうかを区別できない（IS_INTRESOURCE の定義どおり）。既存テストの 1 件（"high-bits-set"）はこの理由で外した（置き換えは「別の文字列」のケース）。

## 選択肢

- A. NULL を返し、last error に触れない（**採用**）。
- B. 前の調査の「HInstance が設定されない理由を調べる」: 成り立たない（HInstance は設定されている。分類 (c)）。
- C. 停止のまま: 次に進めない（次の停止点まで進めることは探りで確認済み）。
- D. DLL の `.rsrc` の MAINICON（決定 0025 の RT_GROUP_ICON）を仮想アイコンとして返す: **不採用**。hInstance = NULL は Learn が「定義済みのシステムアイコン」の意味と書くので、DLL のリソースを返すのは規則に合わない。ゲストの意図はホスト exe の MAINICON で、DLL のものではない。アイコン表とリソース読み込み（0054 と 0061 にまたがる）が要る。

## 値と出典の区別

| 項目 | 内容 | 出典 | 状態 |
|---|---|---|---|
| 失敗時の戻り値が NULL | LoadIconW ページ「If the function fails, the return value is NULL」 | **一次**（Learn） | 確認済み |
| hInstance NULL のとき lpIconName は IDI_ の識別子 | 同ページ「If hInstance is NULL, lpIconName specifies the identifier (beginning with the IDI_ prefix) of a predefined system icon to load」 | **一次** | 確認済み |
| hInstance NULL で lpIconName が文字列のときの結果 | ページに書かれていない | — | **未確認**（推測: NULL） |
| ゲストの引数 | `LoadIconW(NULL, L"MAINICON")`（0x867934。名前は 0x867ba0 の UTF-16。gdb で `rcx=0 rdx=0x867ba0`） | **実測** | 確認済み |
| ゲストが NULL を許容する | `f_5d03e0` の `cmp [rbp+0x98],0; je 0x5d0470`（0x5d0411）で hIcon が NULL なら設定を飛ばす | **読み** | 確認済み |
| NULL を返すこと | 実機の戻り値は未確認 | **仮** | **仮の決定** |
| last error に触れないこと | Learn に失敗時の値がない。値を作らない選択 | **仮** | **仮の決定** |
| 呼び出し直後の経路が GetLastError / SetLastError を読まない | 下の「採用条件の確認」 | 読み（静的）と実行時の確認（gdb） | 確認済み（限界あり） |

## 採用条件の確認（ゲストが GetLastError / SetLastError を読まないこと）

条件: LoadIconW(NULL, "MAINICON") の戻り（0x867939）から、結果を受ける経路で GetLastError（thunk 0x407010、0x41cf80、0x423540。IAT スロット 0x19713c0）と SetLastError（IAT スロット 0x19713d0、thunk 0x423bd0）を読まない。

### 静的な読み

- 戻り 0x867939: `mov rcx,rbx; mov rdx,rax; call 0x5d03e0`（`f_5d03e0(self = rbx, hIcon = rax)`）。
- `f_5d03e0`: `call 0x5d0340`（`rcx` の `[+0x48]` の下に 0x20 バイトのオブジェクトを作り、`[+0x20] = hIcon` を入れる。実際は 0x5d0373 で hIcon を `[obj+0x20]` へ、0x5d037f で r8 = 0 を `[obj+0x10]` へ書くだけ）→ `cmp [rbp+0x98],0; je 0x5d0470`（hIcon = NULL ならここへ）→ 0x5d0470: `call [rbx+0x20]`（仮想呼び出し、rbx = `[self]` の vtable）→ ret。
- 直接 call の閉包（`lift/out-nc/work/closure.py`。.pdata の関数境界で、各関数の直接 call と関数をまたぐ jmp をたどる。IAT の thunk は `jmp [IAT]` の形で IAT スロットを解く）: `f_5d03e0` は 43 関数、thunk 12 個。**GetLastError、SetLastError は無い**（`lift/out-cl/closure-5d03e0.txt`）。`f_5d0340` は 39 関数、thunk 9 個で、同じく無し。
- 0x867939 以降の `f_867710` の直接 call（0x86793f から終わりまでの 18 か所。`lift/out-cl/after-loadicon-calls.txt`）のうち、閉包に GetLastError（kernel32 の thunk 0x423540、0x407010）が入るのは **0x867a78 の `call 0x867e10` だけ**。ほかの 14 個の呼び先（`0x5d03e0` を含めて調べた 16 個から、`0x5d03e0` と `0x867e10` を除いたもの）の閉包には無い（`lift/out-cl/closure-after-loadicon.txt`）。0x867e10 の経路は `f_867e10 → f_869680（0x868023）→ f_5cfff0（0x86968b）→ f_5d0090 → f_5c5500 → f_5c43d0（0x5c43e4: call 0x423540）`。これは **ゲストの `TApplication` の別の構築部で、0x86968b の `f_5cfff0` が `[self+0x130]`（0x86791c で書いたオブジェクト）の `[+0x48][+0x20]` を読む**。`f_5d0090` は先頭で `rsi = [self+0x48]` を取り、`cmp [rsi+0x20],0; jne 0x5d012e`（**hIcon が非 NULL なら何もしない**）、`mov rax,[rsi+0x10]; test rax,rax; je 0x5d012e`（アイコンのデータ元が NULL でも何もしない）。**hIcon が NULL かつ `[rsi+0x10]` が非 NULL のときだけ**下りて `f_5c5500` を呼ぶ（読み）。`[rsi+0x10]` は `f_5d0340`（0x5d037f: `mov [rax+0x10],rcx` の rcx = r8 = 0。`f_5d03e0` が 0x5d0409 で `xor r8,r8` を渡す）が 0 に初期化した値で、**0x86791c〜0x86793f の経路では `[rsi+0x10] == 0` のまま**なので、LoadIconW が NULL でも `f_5d0090` は 0x5d012e へ飛んで `f_5c5500` へ下りない（読み。`[rsi+0x10]` を非 NULL にする書き込みが別の経路に無いことまでは追っていない: 限界）。0x5c43e4 の GetLastError は、`f_5c5500` が 0x5c59a1 で `CreateIcon`（thunk 0x4248b0）を呼び、戻り値を `[rcx]` に書いて 0 のときに `f_5c43d0` を呼ぶ形（0x5c59bd）。つまり **GetLastError を読むのは「CreateIcon が失敗したとき」の分岐**で、`f_5c5500` へ下りること自体が `[rsi+0x10]` 非 NULL を要する。LoadIconW の NULL だけでは入らない。
- 閉包の間接呼び出し（追えない）: `call [rbx+0x20]`（0x5d0488、仮想呼び出し。`f_5d03e0` の閉包に 20 か所の間接呼び出しがある）、ほか `[rip+...]` の関数ポインタ（0x15e6030、0x15e6040、0x15e6048、0x15e6068、0x15e63d8、0x15eaca8。0062 と同じ表）。**静的には先を追えない**。

### 実行時の確認（gdb。`lift/out-cl/gdb-lasterror.txt`、`gdb-class2.txt`）

native の `flengine-native` に `fle_imp_GetLastError`、`fle_imp_SetLastError`、`fle_imp_LoadIconW`、`fle_imp_GetClassInfoW`、`fle_imp_RegisterClassW` のブレークポイント（出力を出して continue）。起動から停止点（CreateWindowExW）までの順序:

```
GetLastError      (f_407010 ← f_4099c0)           LoadIconW より前
GetLastError      (f_407010 ← f_4104f0)           LoadIconW より前
LoadIconW rcx=0 rdx=0x7f00                        IDI_APPLICATION（従来の形）
LoadIconW rcx=0 rdx=0x867ba0                      MAINICON（今回の形。NULL を返す）
GetClassInfoW rcx=0x400000 rdx=0x52a7c8
RegisterClassW rcx=0x13be810
（停止: CreateWindowExW）
```

LoadIconW(NULL, MAINICON) の呼び出しの**直後から次の停止点まで、GetLastError も SetLastError も呼ばれない**。GetLastError が呼ばれるのは LoadIconW より前の 2 回だけ。

結論: **この形の LoadIconW の戻り（NULL）から結果を受ける経路は、静的な直接 call の閉包にも実行時にも GetLastError / SetLastError を読まない**。A 案の条件を満たす。

限界: 間接呼び出しの先（仮想呼び出し `[rbx+0x20]` など）は静的に決められない。実行時の確認は 1 回の実行（この harness の現在の停止点まで。CreateWindowExW より先は未確認）に限る。0x867a78 の `f_867e10` は今回の実行では未到達（停止点より先）。

## 撤回条件

- 実 Windows で `LoadIconW(NULL, 文字列)` の戻りが NULL でないと分かったとき（その値にするか、停止に戻す）。
- ゲストが NULL の hIcon を持つノードの経路で GetLastError を読むと分かったとき（A 案をやめて停止に戻す。停止点より先の実行で見つかる可能性がある）。
- Learn が hInstance NULL で文字列名の結果、または失敗時の GetLastError の値を書いたとき（その値にする）。
- ゲストの DLL が変わり、LoadIconW(NULL, ...) の引数が変わったとき（MAINICON 以外の文字列が来れば停止に出る）。

## 確認できていないこと

- 実 Windows の `LoadIconW(NULL, L"MAINICON")` の戻り値と last error。
- ほかの呼び出し点（0x87fc44、0x9919ba）が渡す文字列。これらは MAINICON 以外なら停止に出る（確認できていない形は答えない）。
- 停止点（CreateWindowExW）より先の実行で、NULL のアイコンを持つオブジェクトが別の経路で扱われるか。
