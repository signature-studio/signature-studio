# 次の停止点: user32!GetKeyboardLayout（0x424d60）の調査と判断材料（実装していない）

実行日: 2026-10-22。作業ディレクトリ `signature-studio-develop`、HEAD cf46bbb に、決定 0062 の変更（`fle_loadimage.c/.h`、テスト）を載せた状態。DLL は `binaries/win64/FLEngine_x64.dll`。「一次」は Microsoft Learn（2026-10-22 取得）、「実測」はコマンドの出力、「読み」は `lift/objdump-text.txt` の静的な読み、「推測」は未検証。成果物は `lift/out-nc/`（gitignore 済み）。

**実装していない。** 理由は §6。決定 0062（LoadImageW の 32649 が NULL）は完了している。

## 1. 停止点（実測。native と wasm が一致）

`OUT=$PWD/lift/out-nc WASM=1 bash lift/harness/a4-survey.sh`（ログ `lift/out-nc/a4/20261021T182650Z-*`、`lift/out-nc/survey1.txt`）。

```
stop   : fle: unsupported instruction at 0x424d60        (user32!GetKeyboardLayout の thunk、IAT スロット 0x1971120)
depth  : init_count=115 / 942   last unit: index=114 run_rank=111/895 proc=0x623840 in_progress
         started=111 completed=110 in_progress=1 of 895 runnable   all_units_done=no
funcs  : executed (distinct)=581  entries=13330   unsupported_total=541（生成 C は変わらない）
```

- 呼び出しスタック: `f_424d60` ← `f_8651a0`（return 0x865222）← `f_620cd0` ← `f_623840`（unit 114 の proc）← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← エントリ。深さ 8。
- 前回（LoadImageW の 32649 で停止）から 関数実行数 580 → 581、入口数 13,250 → 13,330（+80）、unsupported_total 541 のまま。a4 の値は前回と同じ（停止は同じ unit 114 の中）。
- native と wasm: `diff <(grep -v '^calling' native) <(grep -v '^calling' node)` は差なし、stderr も差なし。全体の差は `calling entry … rsp=` の 1 行だけ。終了コードはどちらも 134（SIGABRT）。
- LoadImageW の 21 回は全部通った（gdb で `fle_imp_LoadImageW` を数えた。`lift/out-nc/gdb-lasterror.txt`: idx -22〜-2 の 21 回、2 回目の 32649 が NULL）。停止は f_8651a0 の中、f_8659f0（ループ）から戻った直後の 0x86521d。**PM が予想した SetCursor、DestroyCursor、GetCursor、ShowCursor、DeleteObject はまだ届いていない**（f_8659f0 の後ろで先に別の API に当たった）。

## 2. 呼び出し点（読み）と戻り値の使われ方

`GetKeyboardLayout` の呼び出しは 3 か所。**3 か所とも引数 idThread は 0**（`xor ecx, ecx`: 0x84364a、0x86521b、0xa16500）。

| 呼び出し点 | 関数 | 戻り値（HKL、64 ビット）の使われ方 |
|---|---|---|
| 0x84364c | f_843630（呼び出し元は 0x5de3d6、0x843969） | 先に `GetKeyState(0xa5)`（thunk 0x424d50）が負のときだけ呼ぶ。`[0x14075a0]`（前回の HKL）と比べ、違えば f_843390（`GetKeyboardLayoutNameW`、レジストリ、`[0x14075a8]` のフラグ）で再計算して `[0x14075a0]` に保存。戻り値は `[0x14075a8]` の 1 バイト。 |
| **0x86521d（今の停止点）** | f_8651a0 | **64 ビット全体を `[obj+0x90]` に保存**するだけ（0x865226）。読み出しは 0x865e2f（f_865c90 の中）の `cmp rax, [rcx+0x90]` で、`GetKeyboardLayoutList` が返した HKL の配列の各要素と比べる。 |
| 0xa16502 | f_a164f0（呼び出し元 0xa165e7） | **下位 16 ビット（LANGID）だけ**使う（`and edx, 0xffff`）。f_a16460 が `GetLocaleInfoW(LANGID, 0x1004 = LOCALE_IDEFAULTANSICODEPAGE, buf, 6)`、`GetACP`、数値化でコードページを出し、`MultiByteToWideChar(cp, 4, …)` に渡す。 |

f_865c90（`obj+0x80` の表の遅延構築。`[obj+0x80] != 0` なら作らずに返す）の読み:
- `GetKeyboardLayoutList(0x40, buf)`（0x865cff、thunk 0x424d70）で HKL の配列を取り、要素ごとに f_620bd0 を呼び、真のものだけレジストリ `HKLM\System\CurrentControlSet\Control\Keyboard Layouts\%.8x`（0x865f08 の UTF-16 文字列）を開いて `layout text`（0x865f76）を読む。`RegOpenKeyExW` が失敗（非 0）なら次の要素へ（0x865da0 の `jne 0x865e65`）。要素が `[obj+0x90]`（f_8651a0 が保存した HKL）と等しければ名前をコピー。
- f_865c90 の呼び出し元は f_865f90（0x865f9f。その呼び出し元は 0x614ea8、0x614fee）と 0x614ed3、0x614f0d、0x614f39（`[rax+0x28]`、`[rax+0xb0]`、`[rax+0x30]` の仮想呼び出し。`ActivateKeyboardLayout` の呼び出し 0x614f54 の直前）。**f_8651a0 の中からは呼ばれない**（f_8651a0 が呼ぶのは f_8659f0、0x424d60、0x40f9e0 など）。

HKL を使う周辺の API（呼び出し点の数。読み、`lift/out-nc` の thunk 解決）: `GetKeyboardLayoutList` 1、`GetKeyboardLayoutNameW` 1（0x8433cd）、`LoadKeyboardLayoutW` 2（0x867647、0x8676ab）、`ActivateKeyboardLayout` 4（0x614f54、0x615021、0x8676e2、0x867702）、`GetKeyState` 21、`GetKeyboardState` 6、`SetKeyboardState` 1、`MapVirtualKeyW` 4、`GetKeyNameTextW` 1。すべて未対応（imports_table.rs に行が無い）。

## 3. Learn で確認できたこと（一次。2026-10-22 取得）

- **GetKeyboardLayout**（`updated_at` 2025-07-01）: `HKL GetKeyboardLayout([in] DWORD idThread)`。idThread は「The identifier of the thread to query, or 0 for the current thread.」。戻り値は「the input locale identifier for the thread. The low word contains a Language Identifier for the input language and the high word contains a device handle to the physical layout of the keyboard.」。Remarks: 入力ロケール ID は「a broader concept than a keyboard layout」（IME、音声変換も含む）。「Since the keyboard layout can be dynamically changed, applications that cache information … should process the WM_INPUTLANGCHANGE message」。KLID は `GetKeyboardLayoutName` で取る。**失敗時の戻り値と GetLastError は書かれていない**（ページに Return value の失敗の記述が無い）。**HKL の具体的な値（en-US の high word）は書かれていない**。
- **GetKeyboardLayoutList**: 「the number of input locale identifiers copied」、nBuff が 0 なら必要な要素数。失敗は 0 と GetLastError（値は無し）。
- **GetKeyboardLayoutNameW**: 「The buffer (of at least KL_NAMELENGTH characters)」。名前は「a copy of the string provided to the LoadKeyboardLayout function, unless layout substitution took place」。失敗は 0 と GetLastError（値は無し）。KL_NAMELENGTH の数値はこのページに書かれていない。
- **LoadKeyboardLayoutW**: 名前は「the hexadecimal value of the Language Identifier (low word) and a device identifier (high word). For example, U.S. English has a language identifier of 0x0409, so the primary U.S. English layout is named "00000409". Variants … "00010409", "00020409"」。戻り値は「the input locale identifier corresponding to the name specified … If no matching locale is available, the return value is the default language of the system」。失敗は NULL と GetLastError（値は無し）。**Windows 8 以降は入力ロケールがシステム全体**（「Beginning in Windows 8: The input locale identifier is loaded for the entire system」）。
- **Language Identifiers**: LANGID は 16 ビット（上位 6 ビットがサブ言語、下位 10 ビットが主言語）。
- これらのページから **決まらないこと**: en-US の HKL の数値そのもの（`0x04090409` か `0x00000409` か）。KLID（`00000409`）と HKL の high word の関係（ページは「device identifier (high word)」と「device handle」と、別の言葉で書く）。システムが持つ配列のリスト（`GetKeyboardLayoutList` が何個返すか）。`GetKeyState(0xa5)` の戻り値（本調査では GetKeyState のページを取得していない）。
- 非公式の出典（検索で見つかった範囲）: サードパーティの記述に `HKL 0x04090409`、`0x04090409 (Switch to English keyboard)` が現れる（`wine-assembly.berrry.app` の設計メモ、QKeyMapper README）。**どれも Microsoft の文書ではない。値の裏付けにしない。**

## 4. この lift 環境で既にある決定（関係するもの）

- ロケール: en-US、LCID 0x0409（決定 0016。「この lift 環境の設定」。仕様ではない）。`fle_emulated_locale` 1 か所。`GetLocaleInfoW(0x409, LOCALE_IDEFAULTANSICODEPAGE = 0x1004)` は `fle_locale.c` が既に答える（`FLE_LC_IDEFAULTANSICODEPAGE`）。3 番目の呼び出し点（0xa16502）は、HKL の下位 16 ビットを 0x0409 にすれば既存の応答で通る（読み）。
- スレッド: 単一スレッド規則（決定 0017）。GetCurrentThreadId は固定の仮想値 0x1001（決定 0056、`FLE_EMULATED_TID`）。idThread が 0 以外の GetKeyboardLayout は、スレッド表が無いので答えられない（`GetCurrentThreadId` の値 0x1001 を渡された場合を含む。3 か所とも 0 を渡すので当たらない）。
- レジストリ: 仮想レジストリ（決定 0018）は起動時に空。`HKLM\System\CurrentControlSet\Control\Keyboard Layouts\%.8x` は無いので、f_865c90 のレジストリ読みは `RegOpenKeyExW` の失敗で要素を飛ばす（読み。実装の挙動は未実測）。

## 5. 選択肢

GetKeyboardLayout 単体（idThread = 0 だけ答える）:
- **A. 停止のまま**（今の状態）。
- **B. HKL の定数を返す（状態なし）**。下位 16 ビットは `fle_emulated_locale.lcid & 0xFFFF`（0x0409）で決まる（Learn の low word + 決定 0016）。**上位 16 ビット（device handle）は Learn が決めない**。候補は B1 `0x04090409`（実機の慣習。非公式のみ）、B2 `0x00000409`（LoadKeyboardLayout の「00000409」から読む案。ページは HKL の値とは書かない）。どちらも仮の決定になる。idThread が 0 以外は停止。
- **C. 仮想キーボード状態を設計する**（HKL、`GetKeyboardLayoutList` の配列、`GetKeyboardLayoutNameW` の KLID、`LoadKeyboardLayoutW`、`ActivateKeyboardLayout`、スレッドごとの入力ロケール、レジストリの `Keyboard Layouts` キー、`GetKeyState`/`GetKeyboardState` との関係）。

## 6. 実装しなかった理由

1. **値が Learn から決まらない**。B の high word は B1、B2 のどちらも一次の裏付けが無い。0059、0061 は「ハンドル」で、ゲストがポインタとして検査しない値の仮の決定だった。HKL は **ゲストが `%.8x` でレジストリのキー名に使い（0x865c90）、GetKeyboardLayoutList の要素と等値比較する**（0x865e2f）値で、値を間違えると別の経路に分かれる。
2. **状態と複数の API が必要になる見込み**。`[obj+0x90]` の HKL は `GetKeyboardLayoutList` の要素との一致で使われるので、B 単体で入れた値は、次に `GetKeyboardLayoutList` を答えるときに同じ値を要素に含める必要がある（2 つの API の値の整合）。f_865c90 の先には `GetKeyboardLayoutNameW`、`LoadKeyboardLayoutW`、`ActivateKeyboardLayout` が続き、`GetKeyState`（21 か所、0x84364c の先頭にある）も f_843630 の先頭で必要。
3. **次の停止点が決められない**。GetKeyboardLayout を答えたあと、f_8651a0 の続き（0x865222 以降の `0x40f9e0` による生成の連続）のどこで次に止まるかは、実行しないと分からない（読みでは、f_8651a0 の後ろの 0x865226 以降は `[obj+0xa4]`、`[obj+0xac]` … への生成の繰り返しで、API を直接呼ぶ箇所は見えない。ただし生成関数 0x40f9e0 の先は追っていない）。生成 C を作り直す実験（imports_table.rs の 1 行と runtime の関数）は共有ファイルを変えるので、この範囲では行わなかった。
4. この判断は 0059、0061 と同じ原則（原則 3「ゲストが使わない形は止める。値は作らない」、原則 2「ページから決まらない値は仮の決定として書く」）に沿う。B は決められるが、PM が high word の値（B1、B2）と、続く API（GetKeyboardLayoutList など）までの範囲を選ぶのが先。

## 7. 必要な状態と規則（C を選ぶ場合の材料）

- 状態: 入力ロケールの配列（HKL の列と並び。`GetKeyboardLayoutList` の順序）と現在の HKL。スレッドごと（Learn: GetKeyboardLayout は「for the thread」）か、システム全体（Windows 8 以降の LoadKeyboardLayout。Learn の文言）か。この lift は単一スレッドなので、1 つの値で足りる。ホストのキーボード配列は読まない（原則 1）。
- スレッド規則（0017、0056）: idThread = 0 だけ答える。0 以外は `fle_stop`。
- レジストリ: 仮想レジストリは空。`Keyboard Layouts\%.8x` を作るかどうかは、ゲストが `layout text` の値をどう使うか（f_865c90 の `[obj+0x88]` の文字列。後段は未追跡）次第。作らなければ RegOpenKeyExW の失敗で要素を飛ばす経路になる（読み）。
- GetKeyState: 21 か所（0x6062d2、0x8188a2 …）。キー状態の仮想化（すべて押されていない、など）は別の決定が要る。0x843630 の 0xa5 の呼び出しは、戻り値が負（上位ビット）のときだけ GetKeyboardLayout に進む（`test ax, ax; jge`、0x843645〜0x843648）ので、**GetKeyState が「押されていない」を返す設計なら 0x84364c は実行されない**（読み）。

## 8. その先の見通し

- B を選んだ場合: 3 つの呼び出し点のうち、0x86521d と 0xa16502 は通る見込み（0xa16502 は既存の GetLocaleInfoW で通る。読み）。0x84364c は GetKeyState の設計次第。次の停止点は f_8651a0 の続き（読んでいない）か、GetKeyState、GetKeyboardLayoutList など。**実行して確かめる必要がある**。
- LoadImageW の 21 回は終わったので、SetCursor（31 か所の呼び出し、`unsupported-inventory-pdata.md` の数）、DestroyCursor、GetCursor、ShowCursor は、この先でまだ届いていない。順序は GetKeyboardLayout の先の実行で分かる。
- 前回までの見通しの訂正: `loadimage-caller.md` §7 は「次の停止は SetCursor か DestroyCursor、GetCursor、ShowCursor、DeleteObject のどれか」と読んでいた。実測は **GetKeyboardLayout**（f_8659f0 から戻った直後の f_8651a0）だった。

## 9. 未確認、限界

- 実 Windows の GetKeyboardLayout(0) の値、GetKeyState(0xa5) の戻り値は実測していない（Learn に値が無い）。
- f_8651a0 の 0x865226 以降を実行したときの停止点は未測定（GetKeyboardLayout が答えられない）。
- f_620bd0（f_865c90 の述語）の中身、`[obj+0x88]`（layout text）の後段の使われ方、0x5de3d6/0x843969 の呼び出しの前後は読んでいない。
