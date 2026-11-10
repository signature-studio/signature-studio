# 0284(mt3w、仮): `CompareStringW` の名前つき停止 — 停止の診断を良くし、Microsoft の Sorting Weight Table から導ける範囲(ASCII の制御文字、ハイフン/アポストロフィと任意の文字)を既定で実装した

基準: **e8-wip a1e96d92 + 0279**(ワークツリー `lift/out-mt3y/w79`)。差分は main c74b9194 にも `patch --batch -p1 --dry-run` が通る。PM の木には書いていない。区分: **実測**＝このセッションの出力を見た事実、**読み**＝解釈、**未確認**、**出典なしの選択**＝`S-…`。

## 冒頭の 3 点

1. **測ったこと(実測)**:
   - **停止の正体は、本番でも『制御文字(ESC など)を 1 つでも含む最初の違い』**: 実際の `fle_locale.c`(+ 本番と同じ ia2 の重み表が有効な構成、`fle_wall_ia2.c` + 表の標識行)に 1 単位ずつの文字列の全ての組(ASCII の 128 + 非 ASCII の代表 23 = 151 単位 → 22,650 組)を `NORM_IGNORECASE` ありで比べさせた。**停止 8,800 組のうち、制御文字(U+0000〜U+001F、U+007F)を含むものが 7,894 組**(NUL を除いて 7,658 組。制御文字 × 英字 2,808、× 記号 1,620、× 数字 540、× 制御文字 972 …)。**印字可能な ASCII 同士(記号・空白・英数)で停止するのは、ハイフンとアポストロフィの 1 組(順序つき 2)だけ**(『ハイフンを除くと等しく数も同じ』= 規則が無い)。ほかの ASCII の記号同士・記号対英数・空白は 1 組も停止しない(既存の S- 規則と表がすでに答える)。PM の読み『ASCII の記号が停止する』は、このコードにはほぼ当たらない。
   - **d11 の停止の最有力は Esc 文字(U+001B)**: 決定 0136 の行 5 は『Enter で `CompareStringW("\r", "O", NORM_IGNORECASE)`(キー文字とボタンのアクセラレータ)』、行 6 は『エラーダイアログへの Esc で `CompareStringW` が停止(追っていない)』。ci1 の検証で不正な FLP を開いて TMsgForm(エラー窓)が出た状態で Escape を押すと、同じ比較が ESC 文字(制御文字、空白類ではない)で起きる。**ESC 対 'O' は旧コードで停止 → 新コードで 1(ESC が小さい)**(`enum_cmp`、実測)。**読み**(ci1 の実際の文字列は未入手)。
   - **導出して実装した**(`0284-compare-string.diff`、既存の `fle_ia2_on()` の内側。新しい旗なし): (a) **ASCII の制御文字** U+0001〜U+001F・U+007F に Microsoft の表(`Windows 10 Sorting Weight Table.txt`、SHA-256 `81fcfa1e…`、決定 0191 の入力と同一)の主要な重み(SM 6 または 7、AW)を使う。表は NUL 以外の全ての制御文字に重みを与える。`gen_ia2_sortw.py` を拡張して `ia2_ctl[32]` を生成し、**既存の配列は 1 バイトも変わらない**(`diff` は追加 34 行だけ)。(b) **ハイフン/アポストロフィ**: Learn の『coop と co-op が並ぶ』の文(決定 0178/0229 が読んだ規則)は相手の単位の種類に依らないので、印字可能な ASCII に限っていた制限を外した。
   - **結果(実測、本番と同じ構成、NORM_IGNORECASE あり/なし)**: 停止 **8,800 → 1,502 / 8,852 → 1,554**(組)。**停止 → 答えになった組 7,298。旧コードが答えていた組で答えが変わったもの 0**。残る停止 1,502 組の中身: NUL との全ての組(296。NUL は表で『重み無し』= NLS が無視する文字)、U+00AD・U+FEFF・U+00DF を含む組(882。前 2 つは重み無し、ß は主要な重みが別の単位と等しい)、その他 330(同じ script member の非 ASCII 記号 対 ASCII 記号 — 表の AW をビルドに持たない: U+00A0・U+20AC・U+2013 など、同じ主要な重みの別の単位)、ハイフン対アポストロフィ(2)。制御文字を含む停止は 656 組残る(NUL 296 と、重みの無い/AW の無い相手との組)。
2. **結論**: (1) **診断**は入れた(メッセージの 2 行目に 2 つの文字列・flags・lcid・長さ・最初の違い・ゲストの戻りアドレス)。E9 に入れれば、次に出たとき文字列が分かる。(2) **制御文字の穴は表から導けて、旧答えを 1 組も変えずに塞がった**。(3) 既存の S- の記号の順(コード位置)は表と **870 組中 106 組で食い違う**(記号対記号)。空白対記号・記号対英数・数字対英字は 0 組で一致(表 §4)。置き換えるかは PM の判断(実装していない)。(4) `fle_ia2_on()` の『標識行』は実質いつも有効(ship の 3 つの構成に標識行がある)ので、no-options の 2(a) に従えば旗と無効側を消す整理ができる(§5、実装していない)。
3. **未確認**: ci1 の実際の 2 つの文字列(診断が出す)。FL の TMsgForm が Esc でどの文字列を比べるか(読み)。実機の `CompareStringW` が表の主要な重みの順と一致するか(決定 0191 の注: 実機との照合 0 件)。残る停止(NUL・重みの無い単位・非 ASCII 記号)が実際に出るか。

## 1. 診断(挙動は変えない)

`fle_locale_stop` のメッセージの 1 行目は従来のまま(テストが `contains` で見る)。**2 行目**に、`CompareStringW` の最も外側の呼び出しの文脈が出る(ハイフンの規則が尾を再帰的に比べても、元の文字列が出る)。非印字 ASCII・非 ASCII・引用符は `\uXXXX`、バックスラッシュはそのまま、各 160 単位まで。例(実測、`tests/wall_ia2.rs` の新しい 2 件が検査):

```
fle: CompareStringW: character other than an ASCII letter or digit in unequal strings (not defined by the MSDN page or outside the emulated locale; not implemented)
fle: CompareStringW: lcid=0x409 flags=0x1 len1=4 len2=3 string1="\u001B[\u0022q" string2="O\x" first-difference=0 units=U+001B/U+004F guest-return-address=0x0
```

- **ゲストの戻りアドレス**は `CompareStringW` の呼び出しの次の命令(ゲストの関数の中)。FLEngine で `CompareStringW`(thunk `f_4231c0`)を呼ぶのは **6 か所**(静的、§6): `0x4399ca`(f_439970)、`0x439a3d`(f_4399e0)、`0x44ecfb`(f_44ec40)、`0x4558f3`(f_455890)、`0xc0708b`(f_c07030)、`0xc07112`(f_c070a0)。出た値でどの比較かが決まる。
- 同じ文脈が**全ての** `fle_locale_stop`(case-only、prefix、`SORT_DIGITSASNUMBERS`、ハイフン、ロケール違い …)に付く(`CompareStringW` の実行中に呼ばれた物だけ)。他の API の停止は変わらない。
- 実装: `fle_cmp_body`(従来の本体)+ 外側の `fle_imp_CompareStringW`(文脈を保存して `fle_cmp_body` を呼ぶ)。`fle_api2_str.c`・`fle_wall_dec1.c` の呼び出しも通る。

## 2. 何が停止し、何を実装したか(`tools/enum_cmp.c`、`tables/enum-single-unit-pairs-summary.txt`)

**方法**: 実際の `fle_locale.c`・`fle_wall_ia2.c` を含む runtime を native でリンクし、1 単位の文字列の全ての順序つきの組(ASCII 128 + 非 ASCII 23: U+00A0 U+00AD U+00B7 U+00E9 U+00C4 U+00DF U+2013 U+2014 U+2018 U+2019 U+201C U+2022 U+2026 U+20AC U+2122 U+3042 U+30A2 U+4E00 U+FF21 U+FF10 U+E5A0 U+FEFF U+200B)を子プロセスで比べて、答えか停止かを記録した。4 構成(重み表あり=本番と同じ / なし=ia2 の標識行なし、旧コード / 新コード、`flags` 0 / 1)。

| 構成 | 停止(旧) | 停止(新) | 停止 → 答え | 旧の答えが変わった組 |
|---|---|---|---|---|
| 本番と同じ(表あり)`flags` 1 | 8,800 | **1,502** | **7,298** | **0** |
| 本番と同じ(表あり)`flags` 0 | 8,852 | 1,554 | 7,298 | 0 |
| 表なし(標識行なし)`flags` 1 | 12,244 | 12,072 | 172 | 0 |
| 表なし `flags` 0 | 12,296 | 12,124 | 172 | 0 |

(表なし = `fle_ia2_on()` が偽の構成。本番は偽にならない: ship2/4/5 の構成が標識行を持つ。§5。)

**旧停止の分類(本番と同じ構成、`flags` 1、順序つきの組)**: 制御文字 × 英字 2,808、× 記号 1,620、× 数字 540、× 制御 702、× 空白類 270、× 一般句読点 432、× 非 ASCII 756、× ハイフン 108、× 空白 54、空白類 × 非 ASCII 140、空白類 × 一般句読点 80、空白類 × 空白類 20、ハイフン × 非 ASCII 56、NUL × 全部 296 …。印字可能な ASCII だけの組の停止はハイフン対アポストロフィの 2 組だけ。

**実装(導出の根拠)**:

| 変更 | 根拠 | 出典 |
|---|---|---|
| ASCII 制御文字の主要な重み | 表の行(`0x0001`〜`0x0008` SM 6 AW 3〜10、`0x0009`〜`0x000D` SM 7 AW 5〜9、`0x000E`〜`0x001F` SM 6 AW 11〜28、`0x007F` SM 6 AW 29)。『SM → AW の数値順』は決定 0191 の規則そのもの | Microsoft の表(SHA-256 `81fcfa1e…`)。**S- を足していない** |
| 共通の接頭辞の制御文字は決まりに影響しない | 決定 0126(S-cmp-equal-prefix)。表に重みがある単位だけ許す(NUL は重み無しで停止のまま) | 同上 |
| ハイフン/アポストロフィ × 任意の単位 | Learn『Handling Sorting in Your Applications』:『co-op と coop が並ぶ』。決定 0178/0229 の読み(ハイフンは他の単位の比較に参加せず、残りが等しければ多い方が大きい)を、相手の種類に限らず適用 | Learn(既存の読み) |

**実装していない物(停止のまま)**: NUL(表は『重み無し = 無視』と書くが、無視した後の等価/長さの規則は表にない)、重みの無い単位 U+00AD・U+FEFF・U+200B 系、SM が同じ非 ASCII 記号 対 ASCII 記号(ビルドに AW が無い。ジェネレータの拡張で足せる = データの追加、§7)、2 つのハイフン/アポストロフィの数が等しい場合、同じ主要な重みの別の単位(副次・三次の重みが要る)。

## 3. テスト(`tests/wall_ia2.rs`、実測)

- **旧い 1 件を直した**: `compare_string_leaves_the_pairs_the_table_does_not_decide_to_the_stop` の末尾『ハイフン × かな = 停止』(`-` 5 行)は新しい規則で答えになった(『"-" は空として 'あ' より小さい』)。下の新しいテストに移した。
- **新しい 3 件**(期待値は表の行から手で導出した。理由はテストのコメントに): ① `compare_string_orders_the_ascii_control_units_by_the_primary_weights_of_the_table`(ESC 対 'O' → 1/3、LF < CR、BEL < ESC、DEL > ESC、TAB > ESC、ESC > かな、共通の接頭辞の ESC。標識行なしでは従来どおり停止)。② `compare_string_takes_the_hyphen_out_against_any_unit`(`-` 対 かな、`-x` 対 ESC `x`、`a-` 対 `a` かな、`'` 対 ESC、`-` 対 `-` は 2)。③ `a_stop_of_compare_string_names_the_two_strings_the_flags_and_the_first_difference`(2 行目の形、160 単位で切る、引用符・ESC の表示)。
- **結果(実測)**: `wall_ia2` 20 passed(旧 17 + 新 3)。既存の CompareStringW のテストが全部通った: `locale` 35、`wall_dlg1` 29、`wall_fr11` 47、`wall_fr12` 32、`wall_fr14` 26、`wall_ia1` 19、`wall_crash1` 14、`wall_r2`(`tables/compare-string-tests-first-run-before-test-fix.log` は旧い `wall_ia2` の 1 件が落ちた最初の実行。直した後に再実行)。
- **`test.sh affected`(a1e96d92 + 0279 + 0284、`--jobs 3`、nice 15、`--fresh`)**: `140 of 212 test executables selected`、**`results=140 passed=1242 failed=0 ignored=1 SKIPPED-lines=1`**(SKIPPED は `fe-lift/wall_fr9t` の `FLE_FR9T_CLEAN_RT` 未設定の 1 件。`tables/test-affected-0284.log`)。最初の実行は SKIPPED 7 件(`out-qfc`・`out-blend2d`・`out-ext` の実 DLL の入力が無い)だった: 0279 の後片付けでワークツリーの入力のシンボリックリンクを消していたため(自分の不備)。`test-env.sh --link` で戻して再実行し、1 件になった。a4 は走らせていない(`fle_locale.c` のみ。FRUITY 無効の a4 が `CompareStringW` の新しい経路に入るかは未実測 = 影響範囲外と読むが、必要なら走らせる)。

## 4. 表と既存の S- 選択の食い違い(実装していない。PM の判断材料。`tables/ascii-table-vs-s-choices.txt`)

ASCII の 1 単位同士の最初の違いで、現在のコードが S- 選択(出典なし)で決めている規則を、Microsoft の表(SM → AW)と比べた:

| 現在の規則 | 組(順序つき) | 表と食い違う組 |
|---|---|---|
| S-cmp-symbol-vs-symbol(記号 2 つ: コード位置の順) | 870 | **106**(例: `+`〔SM 8〕は表では他の記号〔SM 7〕より後。コード位置では `,` `.` `/` `:` ` ;` `?` `@` `[` `\` `]` `^` `_` の前) |
| S-cmp-space-as-symbol(空白 対 記号: コード位置) | 30 | 0 |
| 記号 対 英数(ページ: 記号が先) | 1,860 | 0 |
| 空白 対 英数 | 62 | 0 |
| S-cmp-control(TAB〜CR 対 印字可能: コード位置、制御文字が先) | 465 | **5**(`\t \n \v \f \r` 対 空白: 表は空白〔SM 7 AW 2〕が先) |
| S-ia1-cmp-digit-letter(数字が先) | 520 | 0 |
| 数字同士・英字同士 | 全部 | 0 |

= 表で置き換えれば出典なしの S- 選択 2 つ(記号の順・TAB〜CR)が出典つきになり、答えが変わるのは**記号 2 つの最初の違い 106 組と、空白類 × 空白 5 組**に限られる(Browser のリストなど、記号の位置で並びが変わる所がありうる)。**推奨**: 置き換える(S- の削除)が、実エンジンの行(node 84/55/87)の再基準化が要りうるので別の冷間ビルド。実装していない。

## 5. `fle_ia2_on()` の整理案(no-options。PM の判断。実装していない)

- 実態: `fle_ia2_on()` は『応答表に標識行 `CompareStringW::ia2-sort-weights` がある構成だけ真』。**ship2/ship4/ship5 の imports に標識行がある**(`lift/harness/seeds/ship{2,4,5}-imports.txt`)ので、**本番は常に真**。偽は標識行の無い構成(テストの合成 PE と、旧い ia1 だけの構成)にだけ存在する。no-options の 2(a)『有効側を唯一の経路にして旗と無効側のコードを削除する』に当たる。
- 触る所: `fle_locale.c`(`fle_ia2_on` と呼び出し 4 か所)、`fle_wall_ia2.c`(`fle_ia2_cmp_active`、標識の `fle_imp_ia2_marker`)、`fle_gdi.c:4079〜4091`(領域の `fle_ia2_cmp_active` 検査)、`fle_user_nowin.c:26〜30`(PUA)、imports の seeds 4 ファイル(標識行の削除)、テスト: `wall_ia2.rs` の標識行なしの 4 件(`marker = false`: 旧い停止を確かめる)は削除か『答える』への書き換え、標識行を前提にするハーネス(`wall_*` の合成 PE)は `fle_wall_ia2.c` を一緒にリンクする必要。
- 影響: 本番の挙動は変わらない(常に真だったので)。テストとハーネスのリンク構成が変わる。実エンジンの行は変わらない見込み(読み)。

## 6. 静的な見当 — ゲストで `CompareStringW` を比べる関数と、d11 の経路で比べうる文字列(読み)

`FLEngine_x64.dll`(image base 0x400000)の `call/jmp rel32` の走査(`tools/callers.py`、実測)で `CompareStringW`(thunk `f_4231c0`)を呼ぶのは次の 6 関数:

| 関数 | 戻りアドレス(診断に出る値) | lcid / flags | 読み | 直接の呼び出し元の数 |
|---|---|---|---|---|
| `f_439970` | 0x4399ca | 0x400 / 0 | Delphi の文字列比較(大文字小文字を区別、`AnsiCompareStr` 相当) | 7 |
| `f_4399e0` | 0x439a3d | 0x400 / **1**(`NORM_IGNORECASE`) | `AnsiCompareText`(wall_r2 のコメントが『`CompareText` の `sub eax,2`』と書く関数) | **37** |
| `f_44ec40` | 0x44ecfb | 引数で渡る | `TStringList` 系の比較(flags を呼び出し側が決める)と読む | 3 |
| `f_455890` | 0x4558f3 | 0x400 / 1 | `AnsiCompareText` の別の経路 | 1 |
| `f_c07030` | 0xc0708b | 0x400 / 1 | 同上(FL 側のコード) | 2 |
| `f_c070a0` | 0xc07112 | 0x400 / 1 | 同上 | 1 |

(間接呼び出し・仮想メソッド経由は数えていない。)

**d11 の経路で比べうる文字列(読み、未確認 — ci1 の診断の出力で確定する)**:

| 場面 | 比べる 2 つの文字列(見当) | 根拠 |
|---|---|---|
| 不正な FLP(404 の本文 35 バイトを `Sampler.flp` として)を開く | エラー窓 `TMsgForm` のボタンの見出し・アクセラレータ文字。FL の通知文は『タイトル|本文』形式(`Error|…`、例: `Error|An error occured while opening the zipped project.`、`Warning|This project contains invalid data and may cause the program to crash.`、`It may be corrupted, or some plugin caused an error while opening.`、`Error loading sample...`、`File is not a RIFF file`)。FLP が壊れているときの正確な文言は静的には特定していない(文字列表は FLEngine の 0x1062000〜0x1087000 付近に並ぶ) | `FLEngine_x64.dll` の文字列 |
| エラー窓で Enter | `"\r"` 対 `"O"`(OK のアクセラレータ)`NORM_IGNORECASE` — 決定 0136 行 5。**S-cmp-control で既に答える** | 決定 0136 |
| エラー窓で Esc | `"\x1b"`(ESC)対 ボタンのアクセラレータ文字('O' '&C' など)`NORM_IGNORECASE` — 決定 0136 行 6 が『追っていない』と書いた停止。**この決定で答える**(ESC < 英字) | 決定 0136、実測(ESC 対 'O') |
| File メニューの項目のホバー、Escape でメニューが閉じる | VCL のメニュー項目のアクセラレータ文字(`&Open` の 'O' など)とキー文字。単一文字の比較が主 | VCL の `TMenuItem`/`TPopupList` の一般的な構造(読み。FL 固有の経路は未解析) |
| 窓クラス名・ファイル名 | 『名前に記号を含む 2 つ』(ファイル名 `image-line\\…`、`\Backup\` 対 `-serum2\` など)は、決定 0126/0129/0178/0229 の規則(共通の接頭辞、記号の位置、ハイフン)で既に答える | 既存の決定 |

**ci1 への依頼(診断の読み方)**: 次の停止で `string1=…` `string2=…` `guest-return-address=0x…` の 2 行目をそのまま渡してほしい。`0x4399ca/0x439a3d/…` のどれかなら上の表で比較の種類が決まる。

## 7. 次の候補(実装していない)

1. **SM が同じ非 ASCII 記号の AW**(U+00A0 など): ジェネレータの `FULL_SM` を SM 6〜12 に広げる(データ追加。`.inc` が増える)。表の順を既定で使えるようになる。
2. **『重み無し』の単位の無視**(U+00AD・U+FEFF・NUL など): 表のヘッダは『Those characters are effectively ignored by NLS』と書く。両方の文字列から取り除いて比べる規則を足せるが、等価/長さの扱いが表に無い。
3. §4 の置き換え(S- の記号の順・TAB〜CR → 表)と §5 の整理。

## 8. 限界・自己申告

- 実エンジンの `CompareStringW` の呼び出しではなく、**runtime の関数を native で直接呼んだ**(1 単位 × 1 単位)。実機 Windows との照合は 0 件(決定 0191 と同じ限界: 表の主要な重みの順が実機の `CompareStringW` と一致するかは未確認)。ESC が d11 の停止の原因であることは**読み**(ci1 の文字列を待つ)。
- 列挙は 1 単位の文字列だけ。複数単位で最初の違いが後ろにある組は、最初の違いの 1 単位対と同じ規則を通る(共通の接頭辞の規則は別にテストした)。
- 旧い停止 8,800 組の多くは制御文字 × 何か。ゲストが実際にそれらを比べるのは Esc などのキー文字が主、と読むが未確認。
- 規則: `sleep` を 1 回呼んだ(0279 で申告)。列挙の最初の版は `abort()` の core dump が apport を起こして 1 件 1.8 秒かかったので止め、`abort` を子プロセスの `_exit(99)` に差し替えた(`/var/crash` に自分の物は残っていない)。cargo は `nice -n 15`、`CARGO_BUILD_JOBS=3`、`--jobs 3`。

## PM の判断と取り込み(2026-11-06 12:30、docs のみ)

取り込み: mt3 の第 1 報(診断 + ASCII 制御文字の重み + ハイフン/アポストロフィ × 任意の単位)の文書・表・道具と、差分 `0284-compare-string.diff`・`0284-gen_ia2_sortw-docs.diff` を `docs/investigations/mt3w/` に置いた。実行したのは runtime の `fle_locale.c` を native で直接呼ぶ列挙(1 単位 × 1 単位の全組 22,650 × 4 構成)・cargo のテスト・`test.sh affected`(140/212)。**実エンジン(ゲスト)の走行・実機 Windows・ci1 の実際の文字列は 0 件**。『d11 の停止は TMsgForm への Esc 文字(U+001B)』は読みで、検証していない(ci1 の d11 の停止は、キットの proj に Sampler.flp が無く 404 本文を FLP として開いた入力欠落が原因だった)。
判断:
1. **診断(停止メッセージの 2 行目に lcid・flags・両方の長さ・2 つの文字列・最初の違いの位置・ゲストの戻りアドレス)と、表(Windows 10 Sorting Weight Table、決定 0191 の入力と同一 SHA)から導ける ASCII 制御文字・ハイフン/アポストロフィの規則を E10 に載せる**。旧コードが答えていた組の答えは 0 組変わらず、停止 8,800 → 1,502 組。新しい旗・S- 選択なし(lift 忠実性の (b) 契約の表の実装)。
2. A(表で S-cmp-symbol-vs-symbol と S-cmp-control を置換、111 組の最初の違いが変わる)は P2/P3 に直接効かないので、0285 の実装の後に回す。B は 0284c(別文書)。
3. 未確認: ゲストでの実際の比較の文字列(次の停止の 2 行目を ci1 経由で受ける)、残る停止(NUL・U+00AD・U+FEFF・U+00DF を含む 882 組ほか)。
