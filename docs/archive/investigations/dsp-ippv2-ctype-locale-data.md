# dsp_ippv2_x64.dll の DllMain(ATTACH) が GetStringTypeW → LCMapStringW → WideCharToMultiByte で要るデータ: 入力、出力先、ゲストの使い方、値の出典（調査のみ。決めない）

調査日: 2026-10-24（JST）。作業の基準は `git archive` で展開した **9b65b6696af583bbd5edfa50bda693bdc32e0dba**（作業開始時の HEAD。決定 0084 まで）。記録時の HEAD は `c75acc48727a83d94a482d21ca9304be8f509eab`（別の作業が 3 コミット進めた）。`git diff 9b65b66 HEAD` は `lift/runtime/fle_locale.c`、`fle_locale.h`、`fle_imports.c`、`fle_realmod.c`、`tests/locale.rs`、`tests/k32_dsp.rs`、`lift/crates/fe-lift/src`、`lift/harness/two`、`a4-survey.sh` について空（この調査が使ったファイルは HEAD でも同じ）。差が出たのは `fle_reg.c`、`reg.rs` など決定 0085 の実装で、この調査と無関係。**この文書は何も決めない**。ソース、決定、既存の docs は編集していない。

作業物は `lift/out-ctype/`（gitignore 対象。`src/` が `git archive` の展開）。runtime、fe-lift、fe-decode、fe-pe は `lift/out-ctype/src/` のものだけを使い、作業ツリーの `lift/runtime`、`lift/crates` は読んでいない。

## 0. 凡例

- **実測** = このセッションでコマンドを実行して得た値。ログは `lift/out-ctype/run/`。
- **読み** = capstone の逆アセンブル、生成 C、ソース、ページの文を読んだ結果。実行して確かめていない。
- **仮の値** = gdb が止まった所に入れた値。**決定ではなく、runtime の挙動でもない**。runtime のソースとバイナリは変えていない。
- **非公式** = Microsoft の資料ではないもの（Wine、unicode.org の UCD、mingw-w64 のヘッダ）。
- **未確認** / **意見** = そのとおり。
- アドレス: `f_37d…` は基底 0x2000000 の VA。本文の `0x17d…` は RVA（元の DLL の VA は 0x180000000 + RVA。リベース後の VA は 0x2000000 + RVA）。

## 1. 要約

| # | 項目 | 結果 | 区分 |
|---|---|---|---|
| 1 | 再ビルド | 2 モジュール構成（`tbl2`、種 452 個）を `git archive` の runtime で再ビルド。FLEngine の C は SHA-256 `43985d62…37137a1`（期待値と一致）。縮小 DLL の C `6035da1d…279dbf`、`common.c` `33779308…`、リベース済み DLL `4fd25502…efc5`、種 3 ファイルは `out-two-post0083/`、`out-mbtwc-impl/` の同名ファイルと全バイト一致（`cmp`） | 実測 |
| 2 | 最初の停止 | `GetStringTypeW`、**882 / 15,937**、init_count 145/942。`two-module-after-0084.md` と同じ。gdb 読み取りのみの run でも同じ（127 回目の import） | 実測 |
| 3 | 3 つの呼び出しの入力 | **GetStringTypeW は 1 回、LCMapStringW は 4 回（サイズ問い合わせ 2 + 変換 2。flags 0x100 と 0x200 の 2 パス）、WideCharToMultiByte は 2 回（cp=1252 の 256 単位。ほかに ASCII の cp=0 が 4 回）**。入力はどれも固定の 256 単位（GetStringTypeW と LCMapStringW は 1252 の MBTABLE を `[0x20] + 表[1..255]` で変換したもの）。**U+0000 は渡されない**（先頭は U+0020）。異なる単位は 255 個、U+00FF 以下が 228 個、U+0100 以上が 27 個 | 実測 |
| 4 | 出力先 | すべて `f_37d674c`（CRT のマルチバイト初期化 `setSBUpLow` 相当）のスタック上の配列 3 つ（C1 の 256 WORD、小文字 256 B、大文字 256 B）。それを 256 回のループで、ヒープの 0x228 B のブロック（`mbcinfo`）の `+0x19`（`_mbctype`）と `+0x119`（`_mbcasemap`）へ畳む。ATTACH の後、ブロックの内容は DLL の静的なグローバル表 RVA 0x2543ba0、0x2543cb0 にも写される | 実測（ダンプ）＋読み |
| 5 | ゲストが読むビット | C1 の 16 ビットのうち、ループが読むのは **C1_UPPER（0x1）と C1_LOWER（0x2）だけ**（各 WORD の下位バイトの bit 0 と bit 1）。ほかのビットは書かれたあと使われない | 読み＋実測（C1 を全ゼロ／別の値にしても結果は上の 2 ビットだけで決まる） |
| 6 | 値が ATTACH の分岐に効くか | **効かない**。§4.3 の表の仮の値の組 8 通り（`stg3`、`msdata`、`ucdall`、`wineall` の値 4 通りと、`fgstw`、`flcmap2`、`fwctmb2`、`allfail` の失敗の 4 通り）のすべてで DllMain は eax=1。import の列は、`stg3`（`s3zero` と同じ規則）を基準に、`wineall`、`msdata`、`ucdall`、`fgstw`、`fwctmb2` の 5 通りが 160 回すべて名前、引数、戻りアドレスまで同一（`flcmap2` は 156 回で、LCMapStringW の 2 回目以降で MBTWC、WCTMB の組が 1 パス分出ない分だけ短い。`allfail` も 156 回）。ATTACH の間に `_mbctype` を引く関数 `f_37d6f98`（入口 `f_37d700c`。`_ismbbtype` 相当で、mask は 4 固定）は **116 回入られる**（実測。入口カウンタを付けた 4 run（`cnt`、`cnt_zero`、`cnt_msdata`、`allfail`）で 116）。この関数が `_mbctype` に対して調べるマスクは 4（`mbctype` の値 0x04。`f_37d700c` が `r9d = 4` を渡す）で、`f_37d674c` が立てるビット 0x10、0x20 とは別のビット。DLL の `f_37d6cd8` が先頭バイトの範囲に立てるビットも 0x04 だが、1252 は先頭バイトを持たないので、**この run で 0x04 が立つバイトは無い**（`mbctype` のビット数の集計: 0x10 と 0x20 だけ。実測）。`_mbcasemap` を文字で引く命令は DLL 内に**見つからなかった**（読み。走査は `[base+index+0x19/0x119]` 形のみで、下限の保証にはならない） | 実測（入口の回数、import の列）＋読み（走査） |
| 7 | 失敗を返したとき | 3 つの呼び出しのどれが 0 を返しても、`f_37d674c` は戻り値を調べず、DllMain は継続して eax=1 で戻る。ただし GetStringTypeW が失敗すると C1 の配列は書かれず、**スタックの残りの値**から表が作られる（実測: 3 回の run で同一のバイト列） | 実測（仮の値）＋読み |
| 8 | 値の出典 | **Microsoft の資料に C1 の文字ごとの値は無い**（Learn の GetStringTypeW、MS-UCODEREF、Code Page Data Files の zip、Sorting Weight Table）。LCMapStringW の大文字化は MS-UCODEREF 3.1.5.3 と Windows 8 Upper Case Mapping Table（973 組）が Microsoft の出典。**小文字化は表が無い**（大文字表の逆引きが 255 単位で UCD と Wine に一致）。**Locale=0 は Learn にも MS-LCID にも無い**。WideCharToMultiByte(1252) は WCTABLE（決定 0084 の zip の `1252.txt`）が出典で、ATTACH の入力はすべて WCTABLE にある（best-fit に頼るのは U+0191 の 1 単位） | 実測（照合）＋読み |
| 9 | 影響範囲 | 同じ CRT 部分は **Blend2D.dll にもある**（GetStringTypeW 1 か所の `call [IAT]` が同じ `setSBUpLow` 相当の関数から。読み）。FLEngine と QuickFontCache は GetStringTypeW を import しない。LCMapStringW は FLEngine 2 か所、QFC 1 か所、Blend2D 3 か所、dsp 3 か所（フラグは FLEngine、QFC が 0x01000100／0x01000200、dsp が 0x100／0x200） | 実測（静的）＋読み |

## 2. 手順（実測。実行した順）

`lift/` から実行。TMPDIR は `lift/out-ctype/tmp`（`/tmp` は使っていない。§5.1 の Wine も `TMPDIR` と `WINEPREFIX` を `lift/out-ctype/` の下へ向けて実行し、`/tmp` に新しいファイルが無いことを確認した）。

```
git archive HEAD | tar -x -C lift/out-ctype/src            # 9b65b66 の展開
cd lift/out-ctype/src/lift
CARGO_TARGET_DIR=…/out-ctype/target cargo build --release --offline -p fe-pe -p fe-lift
# 種 3 ファイル: out-mbtwc-impl/two/dll/ からコピーし SHA-256 を out-two-post0083/dll/ と照合（一致）
OUT=…/out-ctype/two FEBIN=…/out-ctype/target/release TAG=tbl2 SEEDS=…/seeds-idx0-plus-initterm2.txt bash harness/two/gen_two.sh rebase gen split merge
TAG=tbl2 OUT=…/out-ctype/two bash harness/two/build_two_native.sh          # two-native 571,228,344 B
DLL=…/binaries/win64/FLEngine_x64.dll OUT=…/out-ctype/out-tbl2 NATIVE_BIN=harness/two/two-native-run.sh \
  TWO_NATIVE=…/v-tbl2/native/two-native TWO_REAL=…/dsp_ippv2_x64.2000000.dll FEPE=…/fe-pe bash harness/a4-survey.sh
bash lift/out-ctype/gdb/run_ct.sh NAME "gstw=…,lcmap=…,wctmb=…"           # gdb の測定（§3、§4）
```

- a4-survey（`run/a4-survey-1.txt`）: `stop: fle: GetStringTypeW: the character classes of the page's C1_*, C2_*, C3_* flags are Unicode data that no MSDN page gives (…)`、top of stack `0x37d86c0 f_37d86c0`、init_count 145/942、functions 882、entries 15,937。
- gdb は `lift/harness/two/gdb_imports.py` と同じ作り（`fle_imp_*` に breakpoint、DllMain 入口の後だけ記録）の新しいコピー `lift/out-ctype/gdb/ct_drv.py`、`ct_rules.py`。**値を入れない run（`r0`、`r0b`）が既定**。仮の値は `CT_RULES` を指定した run だけで、`fle_imp_*` の入口で出力と R[0] を書き、`return` で本体を飛ばす。読み取りだけの入口カウンタ（`CT_COUNT`）の breakpoint は止まらない。
- runtime の 4 つの `fle_imp_*` の停止条件（`fle_locale.c`。HEAD でも同じ。SHA-256 `c4118af5…`）:
  - `fle_imp_GetStringTypeW`（:150–154）: 常に `fle_locale_stop`（:152）。
  - `fle_imp_LCMapStringW`（:746–792）: `fle_locale_resolve` が知るのは 0x400、0x800、0x409、0x7F だけ。**Locale=0 は :756 で停止**。フラグは LOWER または UPPER 単独（LINGUISTIC_CASING は付いてよい）、それ以外は :769 で停止。0x80 以上の単位は :776 で停止（256 単位の入力はこれにも当たる）。
  - `fle_imp_WideCharToMultiByte`（:658–712）: 単一バイトの cp は dwFlags が 0 か 0x400 だけ（:684）、**0x80 以上の単位は :611 で停止**。
  - `fle_imp_MultiByteToWideChar`（:546–596）: 決定 0084 の cp1252 の表（`fle_cp1252_hi[32]` :495）。flags 4 または 2 は :571 で停止（バイトの値に関係なく）。
- `lift/crates/fe-lift/tests/locale.rs` は、LCMapStringW を ASCII だけで検査（`lc_map_string` :779。lcid は 0x409、0x400、0x800、0x7f）、`undefined_cases_abort`（:945）が `wctmb-non-ascii-1252`、`lcmap-non-ascii` などの停止を表明する。`k32_dsp.rs` は GetStringTypeW を `STOPPED` に入れる（:179）。**GetStringTypeW を実装するときは、この 2 つのテストの行が変わる**（読み）。

## 3. 停止点ごとの実際の引数と出力先（実測）

gdb の読み取り（`run/r0b-gdb.log`）と、仮の値を入れて進めた run（`run/s3zero-gdb.log` ほか。引数は run によらず同じ）。ゲストのスタックのアドレスは run によらず同じだった。

### 3.1 GetStringTypeW（127 回目の import。停止点）

| 項目 | 値 |
|---|---|
| 呼び出し元 | `f_37d86c0`（0x17d87f9 の `call [0x1ebd020]`。戻りアドレス 0x37d87ff）← `f_37d674c`（戻り 0x37d6813） |
| dwInfoType | **0x1（CT_CTYPE1）** |
| lpSrcStr | 0x7ffff7bfecc0（スタック。126 回目の MultiByteToWideChar の出力） |
| cchSrc | **256** |
| lpCharType | 0x7ffff7bff290（スタック。`f_37d674c` のフレームの `rbp+0x270`。512 B） |
| 入力の 256 単位 | `tables/in-units.txt` と**全単位一致**（`run/measured-inputs-check.txt`）。先頭 0x0020、0x0001〜0x001F、0x0020〜0x007F、0x20AC、0x0081、0x201A、0x0192、…、0x0178、0x00A0〜0x00FF |
| 内訳 | 異なる単位 255 個（U+0020 が 2 回: バイト 0x00 の位置と 0x20 の位置）。U+00FF 以下 228 個（U+0001〜U+00FF のうち 0x80〜0x9F の 27 個を除く）、U+0100 以上 27 個（0152 0153 0160 0161 0178 017D 017E 0192 02C6 02DC 2013 2014 2018 2019 201A 201C 201D 201E 2020 2021 2022 2026 2030 2039 203A 20AC 2122） |

バイト 0 が 0x20 なのは、`f_37d674c` が `buf[i] = i` のあと `mov byte [rsp+0x70], 0x20`（0x17d67bc）で上書きするため（読み。UCRT の `setSBUpLow` と同じ形）。

### 3.2 続く呼び出し（仮の値を入れたとき。GetStringTypeW 以降は 2 パス）

| # | 回 | 呼び出し | 引数（実測） | 出力先 |
|---|---:|---|---|---|
| 1 | 128、129 | MultiByteToWideChar | cp 1252、flags 0x1、src 0x7ffff7bfef90（256 B）、cb 256、dst NULL/cch 0 → 0x7ffff7bfec60/cch 256。戻り 0x37d9b20、0x37d9bc6（呼び出し元 `f_37d9a94`） | 256 WORD |
| 2 | 133 | **LCMapStringW**（サイズ問い合わせ） | **Locale 0x0**、dwMapFlags **0x100**（LCMAP_LOWERCASE）、lpSrcStr 0x7ffff7bfec60、cchSrc 256、lpDestStr NULL、cchDest 0。戻り 0x37d76bf（`f_37d75e8`） | — |
| 3 | 134 | LCMapStringW | 同上で lpDestStr 0x7ffff7bfea50、cchDest 256 | 256 WORD（スタック。`f_37d9a94` のフレーム） |
| 4 | 135 | **WideCharToMultiByte** | CodePage **1252**、dwFlags **0**、lpWideCharStr 0x7ffff7bfea50（上の LCMapStringW の出力）、cchWideChar 256、lpMultiByteStr 0x7ffff7bff090、cbMultiByte **256**、lpDefaultChar **NULL**、lpUsedDefaultChar **NULL**。戻り 0x37d9d31。サイズ問い合わせは無い | 256 B（`f_37d674c` の `rbp+0x70`。小文字の表） |
| 5 | 136〜140 | 同じ順を dwMapFlags **0x200**（LCMAP_UPPERCASE）で（MBTWC ×2、LCMapStringW ×2、WCTMB ×1） | WCTMB の lpMultiByteStr 0x7ffff7bff190 | 256 B（`rbp+0x170`。大文字の表） |

- WideCharToMultiByte の入力は LCMapStringW の出力で、**値は仮の値に依存する**。MS8 の表（大文字）と逆引き（小文字）で与えると、入力は 256 単位。うち U+0100 以上は小文字側 23 種、大文字側 24 種（U+0191 を含む）、和集合 28 種。全て `1252.txt` の WCTABLE に行がある（`run/measured-inputs-check.txt`）。
- ほかに WideCharToMultiByte が 4 回（151、152、155、157。cp=0、flags 0、ASCII。モジュールのパス、環境ブロック）あり、runtime は止まらず通る（`s3zero` で実測）。
- LCMapStringW の Locale が 0 になる理由（読み）: `f_37d75e8` は GetProcAddress("LCMapStringEx") が NULL（決定 0081）のとき `f_37d76d4` → DLL 内の `f_37d9f1c`（名前 NULL で `xor eax,eax`）で LCID を作る。`f_37d674c` が渡すロケール名は `[mbcinfo+0x220]`で、932、936、949、950 のとき以外は 0（0x17d6e10〜0x17d6e50）。1252 は NULL。**この GetProcAddress／API セットの側は別調査（`dsp-ippv2-apiset-gpa.md`）の範囲。この文書は触れず、結論を出さない**。

## 4. ゲストが結果をどう使うか

### 4.1 `f_37d86c0` 周辺は何か（読み。逆アセンブルは `lift/out-ctype/asm/`）

| 関数 | 役割（読み） | 根拠 |
|---|---|---|
| `f_37d6c5c` | **マルチバイトのロケールの初期化**（UCRT の `__acrt_initialize_multibyte` 相当）。初期化表 index 10 の項目（`mbtwc-high-bytes.md` §2.4）。BSS のフラグ `[0x2561e54]` で 1 回だけ。静的な表 `0x2543cb0`（`_mbcasemap`）、`0x2543ba0`（`_mbctype`）、`0x2543970`（既定の `mbcinfo`）のアドレスを `[0x2561e40]`、`[0x2561e38]`、`[0x2561e48]` に置いたあと `f_37d6934` を `ecx = -3` で呼ぶ | 0x17d6c5c〜0x17d6cb5 |
| `f_37d6934` | **`_setmbcp` 相当**。`f_37d6634` で -3 を GetACP に直し、現在の cp と同じなら 0 で戻る。違えば 0x228 B をヒープに取り、既定のブロックを写し、`f_37d6cd8` を呼び、成功ならグローバルへ差し替える | 0x17d6934〜0x17d6ba2 |
| `f_37d6cd8` | `setmbcp_internal` 相当。組み込みの 5 エントリの表（932、936、949、950、1361）に cp が無ければ IsValidCodePage → GetCPInfo。MaxCharSize が 2 でなければ（1252）`[ブロック+8] = 0` にして **`f_37d674c`** を呼ぶ | 0x17d6cd8〜0x17d6f65（0x17d6d80 の呼び出し） |
| **`f_37d674c`** | **`setSBUpLow` 相当（これが 3 つの API の呼び出し元）**。256 バイトの `buf`（`buf[i] = i`、`buf[0] = 0x20`）を作り、`f_37d86c0`（GetStringTypeA 相当）で C1 を取り、`f_37d9db4`（LCMapStringA 相当）を 0x100 と 0x200 で呼び、256 回のループで `mbcinfo` の表を埋める | 0x17d674c〜0x17d6932 |
| `f_37d86c0` | GetStringTypeA 相当。MultiByteToWideChar（flags 1 → 戻り値 = 個数 → 2 回目で変換）→ **GetStringTypeW** | 0x17d86c0〜0x17d8858 |
| `f_37d9db4` → `f_37d9a94` | LCMapStringA 相当。MBTWC（2 回）→ `f_37d75e8`（LCMapStringEx の呼び出し。GetProcAddress が NULL なら LCMapStringW）を **サイズ問い合わせ 1 回 + 変換 1 回** → **WideCharToMultiByte**（`f_37d70a4`。サイズ問い合わせ無しで直接 cb=256 の 1 回） | 0x17d9a94〜0x17d9db2、`run/args-dsp.txt` |

`_setmbcp`、`__acrt_initialize_multibyte` などの名前は **UCRT のソースの形との類似による読みで、DLL のシンボルではない**（DLL にシンボルは無い）。`__acrt_locale_initialize_ctype`（ロケール構造体の `ctype1` 表の初期化）や `__init_ctype` に当たる関数は、この経路に**無い**（読み。`f_37d674c` が書くのは `mbcinfo` の `_mbctype`/`_mbcasemap` だけで、`ctype1` 配列は書かない）。

### 4.2 格納先（実測。gdb で `f_37d674c` の最後の 0x17d690f に breakpoint し、ダンプした）

`mbcinfo`（0x228 B）の書式: `+0 refcount、+4 cp、+8 ismbcodepage、+0xc ushort[6]（先頭バイトの範囲）、+0x18 mbctype[257]、+0x119 mbcasemap[256]、+0x220 ロケール名のポインタ`。ループ（0x17d689b〜0x17d68cc。**C1 の各 WORD の下位バイトだけを調べる**）:

```
for i in 0..255:
    if C1[i] & 1 (C1_UPPER):  mbctype[i+1] |= 0x10;  mbcasemap[i] = lower[i]        # lower = 小文字の 256 バイト（flags 0x100 の結果）
    elif C1[i] & 2 (C1_LOWER): mbctype[i+1] |= 0x20; mbcasemap[i] = upper[i]        # upper = 大文字の 256 バイト（flags 0x200 の結果）
    else:                     mbcasemap[i] = 0
```

| 表 | 置き場所 | 内容 |
|---|---|---|
| C1 の配列 | `f_37d674c` のスタック、`rbp+0x270`（0x7ffff7bff290。512 B） | GetStringTypeW の出力 |
| 小文字の表 | 同、`rbp+0x70`（256 B） | WideCharToMultiByte（flags 0x100 のパス）の出力 |
| 大文字の表 | 同、`rbp+0x170`（256 B） | 同（flags 0x200 のパス） |
| `mbcinfo` | ヒープ（ホスト上のアドレス 0x555565d0bb30 など。run ごとに違う）。`[0x2561e48]`（リベース後 0x4561e48）がポインタ | `_mbctype`（+0x18。index = バイト + 1）、`_mbcasemap`（+0x119） |
| グローバル `_mbctype`、`_mbcasemap` へのポインタ | `[0x2561e38]` = 0x4543ba0、`[0x2561e40]` = 0x4543cb0（リベース後。ファイル上の静的表 RVA 0x2543ba0、0x2543cb0）。`f_37d6934` の 0x17d6b67 の `call f_37d6464` が `mbcinfo` の内容をここへ写す（`[0x2561e38]` が指す領域へ +0x18 から 0x101 B、`[0x2561e40]` が指す領域へ +0x119 から 0x100 B。読み。スレッドのロケールのフラグ `[0x2543f50]` が立つときは写さない分岐がある） | 実測: ATTACH の後、グローバルの表は `mbcinfo+0x18` と `+0x119` に**一致**（`msdata`、`wineall`、`s3zero`、`flcmap2` の 4 run） |

静的表のファイル上の初期値（RVA 0x2543ba0。257 B）は ASCII の英字だけ（非 0 が 52 B。0x10 が 26、0x20 が 26、0x80 以上は 0）。ATTACH の後は、仮の値によって 52（値 0 のとき）〜71 B（`msdata`）が変わる（実測）。

### 4.3 ゲストの表への効果（実測。仮の値の組ごと。`run/stage-table.txt`、`ct_analyze.py`）

| run | 仮の値（**決定ではない**） | 結果 | import 回数 | `_mbctype` の 0x10（_SBUP） | 0x20（_SBLOW） | `_mbcasemap` 非 0 |
|---|---|---|---:|---:|---:|---:|
| `r0b` | なし（読み取りのみ） | GetStringTypeW で停止（882 / 15,937） | 127 | — | — | — |
| `stg1` | GetStringTypeW = 全 0（stage23 と同じ） | LCMapStringW で停止（889 / 15,950） | 133 | — | — | — |
| `stg2` | + LCMapStringW = ASCII のみ | WideCharToMultiByte で停止（890 / 15,957） | 135 | — | — | — |
| `stg3` | + WideCharToMultiByte = 恒等（`?`） | **DllMain が eax=1** | 160 | 0 | 0 | 0 |
| `msdata` | C1 = UCD 由来の私の規則、LCMap = MS8 の表と逆引き、WCMB = WCTABLE | **eax=1** | 160 | 60 | 63 | 123 |
| `ucdall` | C1 = UCD、LCMap = UCD の単純な大文字小文字、WCMB = WCTABLE | **eax=1** | 160 | 60 | 63 | 123 |
| `wineall` | **非公式** Wine 10.0 en-US の値（§5.1） | **eax=1** | 160 | 60 | 65 | 125 |
| `fgstw` | GetStringTypeW が **0 を返す**（LastError 87。出力は書かない）。LCMap = ASCII、WCMB = 恒等 | **eax=1** | 160 | 12 | 3 | 15 |
| `flcmap2` | LCMapStringW が **0 を返す**。C1 = UCD、WCMB = WCTABLE | **eax=1** | 156 | 60 | 63 | 0 |
| `fwctmb2` | WideCharToMultiByte が **0 を返す**。C1 = UCD、LCMap = MS8 | **eax=1** | 160 | 60 | 63 | 0 |
| `allfail` | 3 つとも **0 を返す** | **eax=1** | 156 | 12 | 3 | 0 |

- 全 run で、DllMain の戻りは `fle_realmod.c:144`（`ST_LOADED`）に到達した。GetLastError/SetLastError は 3 つの API の呼び出しの前後に増えない（`GetLastError` の 32 回目、`SetLastError` の 41 回目は ATTACH の前半で、値を入れた run でも同じ位置）。
- 値の 3 通り（`msdata`、`ucdall`、`wineall`）の差は 3 つの単位だけ（§5）。`msdata` と `ucdall` の差は 1 バイト（バイト 0xB5 の `_mbcasemap`: `msdata` は 0xB5、`ucdall` は 0x3F）。`wineall` は 0xAA と 0xBA を LOWER にする（`_SBLOW` が 2 増える）。
- **GetStringTypeW が 0 を返すと**、`f_37d86c0` は C1 を書かないまま戻り、`f_37d674c` は戻り値を調べない。C1 の配列にはスタックの前の内容（呼び出しの前の別の関数のフレームの残り）が残り、その下位バイトの bit 0、1 から表が作られる。`fgstw` の 3 回の run（`fgstw`、`fgstw_b`、`fgstw_c`）で C1 配列の SHA-256 が同一（`8dcc54639ef5…`）。**ゲストの CRT は失敗を検出せず、未初期化に近い値で表を作る**（実測。実機で同じ残りになるかは未確認）。
- LCMapStringW が 0 を返すと、`f_37d9a94` は 0 を返し、`f_37d9db4` は呼び出し元に戻る。`f_37d674c` はその結果を使わず、小文字、大文字の配列はスタックの残りのまま。`_mbcasemap` は C1 のビットに従って**配列の残りの値**で埋まる（`flcmap2` ではすべて 0 だった）。

### 4.4 読む側（読み、静的な走査と実測の入口カウンタ）

- **`_mbctype`（`mbcinfo+0x19+c`）を引く関数は DLL 内に 4 つ**（`f_37d6f98` = `_ismbbtype` 相当の mask 4 固定は 0x17d6fc4、`f_37d9a00`（入口 `f_37d99f8`）は 0x17d9a5a の mask 4、それに表を埋める `f_37d6cd8` の 0x17d6de8 と 0x17d6ed5。そして `f_37d674c` 自身）。**`_mbcasemap` を文字で引く命令は走査で見つからなかった**（`[base+index+0x119]` の形、byte 幅。0x17d672b の `mov al, [rcx+0x119]` は表の写し（`f_37d66b4`）で、文字による参照ではない）。
- そのうち ATTACH で入られるもの（実測、`CT_COUNT`）: `f_37d700c`（→ `f_37d6f98`）**116 回**。`f_37d99f8` は **0 回**。呼び出し元は `f_37d1b54`（コマンドラインの解析。ATTACH で 2 回入られる）の 0x17d1bc7 と 0x17d1cae で、`movsx esi, byte [rdi]` の文字を渡す形（`_ismbblead` 相当。読み。`_mbctype` の 0x04 は `f_37d6cd8` が先頭バイトの範囲に立てるビット）。**mask 4 だけ**を調べる。つまり ATTACH で読まれるのは**ビット 0x04 だけ**で、`f_37d674c` が書くビット 0x10、0x20 と `_mbcasemap` は ATTACH では読まれない（読み）。
- 通常の実行（IPP/QFC の関数）で読まれるか: **DLL の 446 個のエクスポートからの直接の呼び出しで `f_37d674c`、`f_37d86c0`、`f_37d9db4`、`f_37d6f98` などには届かない**（`mbtwc-high-bytes.md` §2.6。`mbtwc_exports.py` を今回も再実行し、`f_37d700c`、`f_37d1b54`、`f_37d99f8`、`f_37d5ecc`、`f_37d674c`、`f_37d86c0`、`f_37d9db4` が「どのエクスポートからも直接の辺で届かない」ことを確認した）。ただし関数ポインタ経由は追っていない（下限）。
- IPP の関数（`.text` の 0x600000 以降、IPPCODE）には、byte 幅、base+index、disp が 0x18/0x19 の操作が 26 関数ある（`run/readers2-dsp.txt`）が、これらは `mbcinfo` の写しではなく SIMD の構造体の添字の可能性が高く（`vpinsrb`、`movzx r14d, [r9+rbx+0x18]` ほか）、27 関数すべてで `f_37d674c` が埋めるグローバル（`[0x2561e38/40/48]`、静的表）への参照も、ロケールのゲッタ（`f_37d2504`、`f_37d30a8`、`f_37d6cbc`）への call も無い（実測の走査）。**よって IPP の通常の実行が `_mbctype`、`_mbcasemap` を読むという根拠は見つからなかった**（読み。断定ではない。DLL は 2.5 万行規模の IPP コードで、間接参照は追っていない）。
- したがって、**ATTACH の分岐（成功か失敗か）に効かない。表に格納されるだけ**（実測 §4.3 と読み）。値が効くのは `isalpha`、`isupper`、`tolower`、`_mbctoupper` などの CRT の関数をゲストの CRT 経由で通常の実行が呼ぶ場合で、DLL 内にこれらの呼び出しは見つからなかった（読み）。

## 5. 値の出典

### 5.1 (a) GetStringTypeW(CT_CTYPE1)、U+0000〜U+00FF の 256 値

**Microsoft の資料に値は無い**:

> [Learn: live-getstringtypew.html] "These types support ANSI C and POSIX (LC_CTYPE) character typing functions."
（ページは C1_UPPER 0x0001 … C1_DEFINED 0x0200 の名前と値と「Meaning」の列を挙げるだけで、文字ごとの値を書かない。表の行（例: C1_DEFINED の Meaning は「A defined character, but not one of the other C1_* types」）は引用の機械照合の対象にしていない。ページの先頭の Note は `This API may have incomplete/outdated information for certain Unicode characters, particularly those in the supplementary range.` と書き、ICU の `u_charType` などを推奨する。ページ日付 `ms.date` 2024-10-23。）

| 資料 | 内容 | C1 の値があるか | 区分 |
|---|---|---|---|
| Learn の GetStringTypeW（上） | ビット名と意味 | **無い** | Microsoft |
| MS-UCODEREF v20240423 の本文（PDF のテキスト `ref/learn/ms-ucoderef.txt`） | 「C1_」「GetStringType」「ctype」の語が**無い**（`grep` で 0 件） | **無い** | Microsoft |
| Windows Supported Code Page Data Files.zip（決定 0084 の zip）の 111 ファイル | 先頭トークンの集計: CODEPAGE、CPINFO、MBTABLE、WCTABLE、ENDCODEPAGE、DBCSRANGE、DBCSTABLE、GLYPHTABLE だけ | **無い**（文字分類のタグが無い） | Microsoft |
| Sorting Weight Table（Windows 10。同じ Download Center id=10921。取得 21,156,033 B、SHA-256 `81fcfa1e…`） | 文字ごとに SM/AW/DW/CW の 4 バイト。CW の bit 0x10 を「Upper Case」と定義。U+0000〜U+00FF の 256 行すべてに行がある | **C1 そのものではない**。CW の 0x10 と UCD の `General_Category = Lu` は 255/256 一致（不一致は U+00DF: CW に 0x10 が立つ）。C1_LOWER、DIGIT、PUNCT などに当たる記述は無い | Microsoft（C1 の出典ではない。間接的な手掛かり） |
| Unicode の UCD（UnicodeData 16.0.0、DerivedCoreProperties 16.0.0。unicode.org） | General_Category、Uppercase、Lowercase など | **Microsoft が C1 の規範として参照していることは確認できなかった**。Learn の GetStringTypeW のページ自体が ICU（Unicode 準拠）を代替に挙げるが、C1 と UCD の対応は書いていない | 非公式（Microsoft の規範ではない） |
| Wine 10.0（ホストの `/usr/bin/wine`。en-US = `LC_ALL=en_US.UTF-8`） | `GetStringTypeW(CT_CTYPE1)` を U+0000〜U+00FF と ATTACH の 256 単位で実行（`run/wine-enUS-out.txt`、`wine/ct_wine.c`） | **ある**（下表） | **非公式。Windows の実測ではない**。Wine は Windows の挙動の再実装 |

Wine の C1（U+0000〜U+00FF。連続した区間にまとめた 36 区間。全 256 の一覧は `run/wine-c1-runs.txt`、`tables/candidates.tsv`）:

| 区間 | 値 | 内訳 |
|---|---|---|
| U+0000〜0008、000E〜001F、007F〜0084、0086〜009F | 0x0220 | CNTRL\|DEFINED |
| U+0009 / 000A〜000D / 0085 | 0x0268 / 0x0228 / 0x0228 | SPACE\|CNTRL(\|BLANK)\|DEFINED |
| U+0020 / 00A0 | 0x0248 | SPACE\|BLANK\|DEFINED |
| U+0021〜002F、003A〜0040、005B〜0060、007B〜007E、00A1〜00A9、00AB〜00AC、00AE〜00B1、00B4、00B6〜00B8、00BB〜00BF、00D7、00F7 | 0x0210 | PUNCT\|DEFINED |
| U+0030〜0039 | 0x0284 | DIGIT\|XDIGIT\|DEFINED |
| U+0041〜0046 / 0047〜005A / 00C0〜00D6、00D8〜00DE | 0x0381 / 0x0301 / 0x0301 | UPPER\|(XDIGIT\|)ALPHA\|DEFINED |
| U+0061〜0066 / 0067〜007A / 00DF〜00F6、00F8〜00FF | 0x0382 / 0x0302 / 0x0302 | LOWER\|(XDIGIT\|)ALPHA\|DEFINED |
| U+00AA、00B5、00BA | 0x0312 | LOWER\|PUNCT\|ALPHA\|DEFINED |
| U+00AD | 0x0230 | PUNCT\|CNTRL\|DEFINED |
| U+00B2、00B3、00B9 | 0x0214 | DIGIT\|PUNCT\|DEFINED |

- **ATTACH が実際に読むのは C1_UPPER（0x1）と C1_LOWER（0x2）だけ**（§1 の 5）。255 個の異なる単位のうち、Wine は UPPER 60、LOWER 65、どちらも無し 130。
- Wine の UPPER/LOWER と UCD の対応（`run/c1-upper-lower-compare.txt`）: **UPPER は `General_Category = Lu` と一致（56 個。U+0000〜U+00FF）**。LOWER は `General_Category = Ll`（59）に **U+00AA と U+00BA**（`Lo`。UCD 16.0 では `Other_Lowercase`、派生 `Lowercase` の true）を足した 61 個と一致。つまり **`Lu` と派生プロパティ `Uppercase`/`Lowercase`（DerivedCoreProperties）で Wine の UPPER/LOWER は全 256 単位とも再現できる**。`General_Category` だけだと U+00AA、U+00BA の 2 単位が合わない。UCD の版による違い: `Ll` だった U+00AA、U+00BA が 6.1.0 で `Lo` になる（5.2.0、6.0.0 は `Ll`、6.1.0、8.0.0、16.0.0 は `Lo`。`ref/UnicodeData-*.txt`）。**Windows 10 22H2 がどの版を使うかは未確認**。
- C1_ALPHA は UCD の `General_Category = L*` と 117 個すべて一致。C1_DIGIT は 0x30〜0x39 のほか U+00B2、U+00B3、U+00B9 を含む（UCD では `No`）。C1_PUNCT は 65 個で、UCD の `P*`（30 個）より多い（Wine は記号 `S*` も PUNCT にしている）。C1_CNTRL は 66 個（UCD の `Cc` 65 + U+00AD）。C1_DEFINED は 256 個すべてに付く。**これらは Wine の値で、Microsoft の資料から確認できない。**ATTACH はこの残りのビットを使わない。
- ATTACH の入力のうち U+0100 以上の 27 単位の Wine の値: 0x0152、0x0160、0x0178、0x017D = UPPER|ALPHA|DEFINED（0x0301）、0x0153、0x0161、0x017E = LOWER|ALPHA|DEFINED（0x0302）、0x0192 = LOWER|ALPHA|DEFINED（0x0302）、U+02C6 = 0x0300、U+02DC = 0x0200、ほかは 0x0210（PUNCT）、U+20AC、U+2122 は 0x0200。U+0192 が **LOWER**（`Ll`）である点が、バイト 0x83 の `_mbcasemap` に効く（§5.3）。

C2、C3（CT_CTYPE2、CT_CTYPE3）は ATTACH では呼ばれない（`GetStringTypeW` の import は 1 か所、dwInfoType は 1 固定。実測）。

### 5.2 (b) LCMapStringW(LCMAP_LOWERCASE / UPPERCASE) と Locale=0

**大文字化**:

> [PDF: ms-ucoderef.txt] "To map a UTF-16 string to upper case, each UTF-16 code point is looked for in an upper casing table [MSDN-UCMT/Win8]. If an entry is found, the input code point is changed to the output code point."

- `[MSDN-UCMT/Win8]` は 1.2.1 の Normative References に載る「Windows 8 Upper Case Mapping Table」（Download Center id=10921。取得 35,546 B、SHA-256 `76a66e29…`。973 行。`0xLOWER  0xUPPER` の形）。U+0000〜U+00FF の行は 57 行（ASCII a〜z の 26、U+00E0〜U+00FE から U+00F7 を除く 30、U+00FF の 1）、U+00B5 と U+00DF と U+00AA、U+00BA は**表に無い**（大文字化されない）。
- 255 単位に対する **MS8 の大文字は Wine と全単位一致**。**UCD の `Simple_Uppercase_Mapping` との差は 1 単位**（U+00B5: UCD は U+039C、MS8 と Wine は U+00B5 のまま）。MS8 と UCD は BMP 全体では 222 コードポイントで違う（`run/ms8-vs-ucd-fullbmp.txt`。U+0131、U+017F、U+0252 など）が、**ATTACH の入力に関係するのは U+00B5 だけ**。
- 大文字化の出力のうち U+0100 以上になるのは U+0152、0160、0178（U+00FF から）、017D、0191（U+0192 から）。うち **U+0191 だけが 1252 の MBTABLE に無く**、WCTABLE の best-fit の 1 件（→ 0x83。§5.3）。

**小文字化**: **Microsoft の資料に表が無い**（MS-UCODEREF の目次は 3.1.5.3 を大文字化だけにしている。「lower」の語は表の説明「bit 3,4 => upper/lower case」と Sorting Weight Table の「CW」だけ）。MS8 の大文字表は**逆引きが関数になる**（973 行で、異なる大文字が 973 個＝異なる小文字の写像が重ならない）。この逆引きを小文字化とみなすと、255 単位に対して **Wine と UCD の `Simple_Lowercase_Mapping` に全単位一致**（`run/compare-sources.txt`）。ただし**逆引きを小文字化とみなすのは Microsoft の記述ではなく、この調査の読み**。BMP 全体では UCD と 205 コードポイントで違う（U+0130 など。ATTACH の入力には無い）。

**Locale=0**:

> [Learn: live-lcmapstringw.html] "Locale identifier that specifies the locale. You can use the MAKELCID macro to create a locale identifier or use one of the following predefined values."

- Learn の LCMapStringW のページ（`ms.date` 2025-08-08）が挙げる定義済みの値は LOCALE_INVARIANT、LOCALE_SYSTEM_DEFAULT、LOCALE_USER_DEFAULT と、カスタムの LOCALE_CUSTOM_DEFAULT、LOCALE_CUSTOM_UI_DEFAULT、LOCALE_CUSTOM_UNSPECIFIED。**0 は挙がっていない**。
- 数値: LOCALE_USER_DEFAULT は 0x0400、LOCALE_SYSTEM_DEFAULT は 0x0800、LOCALE_INVARIANT は 0x007F:

> [Learn: live-locale-user-default.html] "The default locale for the user or process. The value of this constant is 0x0400."
> [Learn: live-locale-system-default.html] "The default locale for the operating system. The value of this constant is 0x0800."
> [Learn: live-locale-invariant.html] "The value of LOCALE_INVARIANT IS 0x007f."

- MS-LCID の LCID 構造のページ（2.2。`ms.date` 2024-04-23）の Language ID の表に **0x0000 の行は無い**（取得した表 482 行を機械的に確認: `0x0000` の行 0 件、0x0001 から始まる）。0x007F は「Reserved for invariant locale behavior」。語「neutral」はそのページに 0 回。mingw-w64 の `winnt.h`（**非公式**）は `LOCALE_NEUTRAL` を `MAKELCID(MAKELANGID(LANG_NEUTRAL,SUBLANG_NEUTRAL),SORT_DEFAULT)`（= 0）と定義する。**Microsoft の資料では 0 を LOCALE_NEUTRAL と呼ぶ根拠を、この調査では見つけていない**（Learn の Locale Identifiers のページにも `LOCALE_NEUTRAL` の語は無い）。
- 非公式（Wine 10.0、en-US）: Locale=0 と 0x409、0x7F、0x400、0x800 の 5 通りで、**255 単位に対する LCMAP_LOWERCASE と LCMAP_UPPERCASE の出力が同一**（`run/compare-sources.txt`。ret 256、LastError 0）。Windows での値は未確認。runtime の `fle_locale_resolve` は 0x400、0x800、0x409、0x7F だけを持つ（`fle_locale.c:61–73`）。0 を「ユーザー既定」に解釈するかは決定が要る（§8 の 5）。

### 5.3 (c) WideCharToMultiByte(1252) と、ATTACH が渡す入力の範囲

**出典は決定 0084 の zip の `CodpageFiles/1252.txt` の WCTABLE**（698 行。`CPINFO 1 0x3f 0x003f`。SHA-256 `72ea23c9…76a9`）。MS-UCODEREF 3.1.5.1.1.2 の疑似コードが WCTABLE の第 1 欄が一致する行の第 2 欄を出力し、行が無ければ lpDefaultChar（NULL なら CPINFO の既定文字）にする:

> [PDF: ms-ucoderef.txt] "The Unicode UTF-16 code point for the character being converted."
> [PDF: ms-ucoderef.txt] "The single byte that this UTF-16 code point maps to. This can be a best-fit mapping."

- この疑似コードに **dwFlags（WC_NO_BEST_FIT_CHARS など）と lpUsedDefaultChar は出てこない**（`grep` で 0 件）。WC_NO_BEST_FIT_CHARS の効果は Learn の WideCharToMultiByte のページに文章がある（「if translating from Unicode to multibyte and back to Unicode again does not yield the same Unicode character, the function uses the default character」）が、**どの文字が best-fit かの表は WCTABLE の「厳密な逆引き 256 件」との差**から導くしかない（読み）。
- **ATTACH が渡す入力**（実測。`run/measured-inputs-check.txt`）: 呼び出しは 2 回（256 単位。LCMapStringW の小文字出力と大文字出力）、cp 1252、dwFlags **0**、lpDefaultChar と lpUsedDefaultChar は **NULL**、cb 256。入力はこのうち**大文字小文字の写像を通った後の単位**で、仮の値（MS8 の大文字表と逆引き）では、1 回目は 195 種、2 回目は 195 種（和集合 256 種）。**U+0100 以上は和集合で 28 種**（小文字側 23 種 + 大文字側の 5 種 U+0152、0160、0178、017D、0191。残りは共通）。**全 256 種が WCTABLE に行を持つ**。
- WCTABLE の 698 行のうち U+0000〜U+00FF の行は 229 行、MBTABLE の厳密な逆引きは 256 件。ATTACH の入力のうち厳密な逆引きに入らないのは、**大文字側の U+0191（U+0192 の大文字化）の 1 単位だけ**で、WCTABLE の best-fit（→ 0x83）。flags = 0 では best-fit が使われうる。**ATTACH の結果としてこの 1 バイトが効くのは `_mbcasemap[0x83]` だけ**:
  - バイト 0x83 は U+0192（`ƒ`）。C1_LOWER を立てれば `mbcasemap[0x83] = upper[0x83]`（大文字の表の位置 0x83）で、その値は WCTABLE で 0x83（best-fit を採る場合）、採らない場合は既定文字 0x3F（非公式 Wine は flags 0 で 0x83。WC_NO_BEST_FIT_CHARS を付けると 0x3F）。C1_LOWER を立てなければ 0。
  - C1_LOWER は MS 資料に値が無い（§5.1）。Wine は U+0192 を LOWER にする（UCD は `Ll`）。
- 非公式 Wine（flags 0）の WideCharToMultiByte(1252) の出力は、WCTABLE から作った表と**全 256 バイト一致**（小文字、大文字とも）。
- lpDefaultChar、lpUsedDefaultChar が NULL なので、runtime の `*used = 0`（:710）は呼ばれない。runtime が現状「`dwFlags` が 0 か 0x400」と「ASCII 以外は停止」としているのを、WCTABLE 全域に広げる場合は、**best-fit（WCTABLE の 698−256 行）をどう扱うかが決定点**（§8 の 4）。
- ATTACH の外では、FLEngine の WCTMB は dwFlags 0 または 0x400（WC_NO_BEST_FIT_CHARS）、cp は 0xFDE9（UTF-8）、1252（CP_ACP の値、固定、または変数）が静的に見える（§6）。

### 5.4 (d) 最小の実装範囲（選択肢。決めない）

ATTACH が実際に渡す入力は**固定の 256 単位**（§3、実測）。実装範囲の選択肢と、その事実:

| # | 範囲 | 内容 | 事実 |
|---|---|---|---|
| A | **渡される 256 値だけに限定し、それ以外の入力は従来どおり名前付き停止** | GetStringTypeW(CT_CTYPE1) は「256 単位の ATTACH の入力と一致する入力」だけ、LCMapStringW は Locale 0 とその単位だけ、WCMB(1252) はその出力だけ | ATTACH は通る（仮の値で実測）。出典は 5.1〜5.3 の各行。**ATTACH 専用の特殊ケースで、API の仕様ではない**。入力が 1 単位でも違えば停止する。**入力が `f_37d674c` の 256 単位ではなくなる呼び出し（Blend2D、FLEngine の LCMapStringW、QFC）は別に要る** |
| B | **U+0000〜U+00FF の全域 + ATTACH が渡す U+0100 以上の 27 単位** | GetStringTypeW は C1 のビットをこの 283 単位で持つ。LCMapStringW は LOWER、UPPER でこの範囲を持つ。WCMB(1252) は WCTABLE 全体 | C1 の出典は Microsoft に無い（§5.1）。LCMap は MS8（大文字）＋逆引き（小文字）を使える。WCMB は WCTABLE（決定 0084 と同じ zip） |
| C | **Unicode のデータを出典に全域（BMP 以上）** | UnicodeData（General_Category、Simple_Case_Mapping）で C1、LCMap を決める | **UCD と MS8 は BMP で 222（大文字）、205（小文字）コードポイントで違う**（§5.2）。Microsoft の動作とは一致しない部分がある。出典の版（5.2〜16.0）で U+00AA、U+00BA の C1 が変わる |
| D | **停止のまま** | 実装しない | ATTACH は GetStringTypeW で止まる（882 / 15,937）。決定 0073 の目的（IPP の関数の実行）に届かない |
| E | **GetStringTypeW が 0 を返す（LastError 87）** | 失敗を返す | **応答を偽る**。ゲストは失敗を検出せず、スタックの残りで表を作る（§4.3 の `fgstw`。実機でも同じとは限らない）。ATTACH は通る。決定 0007、0016 の「黙って値を作らない」と反する方向 |

**意見**（事実ではない）: A は ATTACH の通過という目的に対して最小で、出典の足りない範囲（C1）を最小にできる。ただし A は「API の実装」ではなく「呼び出し点の再現」であり、決定 0016 の「ページに書かれた規則で実装できるものだけ」の枠から外れる。**どの範囲にするかは、C1 の出典の格（§5.1 の Wine、UCD を決定 0041 の先例と同じ「非公式・仮」で採るか）が決まってからでないと決められない**。

## 6. 影響の範囲（静的に数えた。実測）

IAT スロットへの `call`/`jmp [rip+disp]` の参照を、capstone で復号して数えた（`ct_xref.py iat`、`ct_sites.py`、`ct_args.py`。`run/sites-*.txt`、`run/args-*.txt`）。同じ API でも、スロットを `jmp` するだけの**薄い入口**（`.pdata` の外）と、そこへの `call` の数（実際の呼び出し点）を分けて書く。引数は直前の定数代入を拾う読みで、分岐は追っていない（「定数が見つからない」は変数）。

| API（IAT スロット） | FLEngine（ImageBase 0x400000） | dsp_ippv2（0x180000000） | QuickFontCache（0x400000） | Blend2D（0x180000000） |
|---|---|---|---|---|
| GetStringTypeW | **import 無し** | スロット参照 1（`call`。`f_37d86c0`）。呼び出し点 1 | **import 無し** | スロット参照 1（`call`。0x1ade74）。呼び出し点 **2**（0x1aaf2e = 下の `setSBUpLow` 相当（dwInfoType = 1、cch = esi）、0x1abbb4 = 1 文字ずつ（edx = r10d））。cchSrc = 256 は 0x1aaf2e だけ |
| GetStringTypeA、GetStringTypeExW | import 無し | import 無し | import 無し | import 無し |
| LCMapStringW | スロット 1（`jmp`。薄い入口 0x23970）。呼び出し点 **2**（0x505f4、0x506b4。flags **0x01000100**、**0x01000200** = LCMAP_LINGUISTIC_CASING + LOWER／UPPER。Locale = `ebx`（変数）） | スロット 1（`call`。`f_37d75e8`）。`f_37d75e8` の呼び出し点 **3**（0x17d9bfb、0x17d9c53、0x17d9cfd）。ATTACH では 4 回呼ばれる（Locale 0。flags 0x100 ×2、0x200 ×2） | スロット 1（`jmp`。薄い入口 0x1d8f0）。呼び出し点 **1**（0x40104。flags 0x01000100） | スロット 1（`call`。0x1a9b54）。`0x1a9b54` の呼び出し点 **3**（0x1af49a、0x1af4f2、0x1af59c。flags は変数 `r13d`）。そのうち `setSBUpLow` 相当（`0x1af64c`）が flags 0x100、0x200 の 2 回 |
| LCMapStringEx、LCMapStringA | import 無し | import 無し | import 無し | import 無し |
| WideCharToMultiByte | スロット **2**（薄い入口 0x70e0 ← 1 呼び出し、0x23d70 ← 8 呼び出し）。cp: 0xFDE9（UTF-8）が 3、1 が 1、`ecx=<変数>` が 4。dwFlags: 0 が多数、**0x400（WC_NO_BEST_FIT_CHARS）が 1**（0x589351） | スロット 1（`f_37d70a4` の末尾の `jmp`。呼び出し点 **9**。cp は定数 0 が 2（0x17d71d0、0x17d721e。環境ブロック）、0xFDE9 が 1（0x17d4845）、変数が 6。ATTACH で入られるのは 6 回: **0x17d9d2c（`f_37d9a94` 内）が 1252 の 256 単位で 2 回**、cp = 0 の ASCII が 4 回（0x17d62ba、0x17d6313 = モジュールのパス（cp は変数で実測 0）、0x17d71d0、0x17d721e）） | スロット **2**（薄い入口 0x6ec0 ← 1、0x1db20 ← 2）。cp は 1、`ecx=<変数>` | スロット 1（0x1ab710 の末尾の `jmp`。呼び出し点 **10**）。cp は 0xFDE9 が 1、0 が 2、変数が 7 |
| MultiByteToWideChar | スロット **2**（薄い入口 0x70d0 ← 4、0x23a40 ← 7）。cp: 0xFDE9 が 2、0 が 1、`ecx=<変数>` が多数。dwFlags: 0、**4（MB_USEGLYPHCHARS）が 1**（0x616539） | スロット 1（`f_37d7048`（`.pdata` の外）の `jmp`）。呼び出し点 **8**（ATTACH の 6 回が 1252、flags 1、cb 256。ほか 2 つは `f_37d3f3c`（mbtowc 相当）で ATTACH では入られない） | スロット 1（薄い入口 0x6eb0 ← 4） | スロット 1（0x1ab6b4 の末尾の `jmp`）。呼び出し点 **8**（flags 0x9、0x1 など。0x1adee7 は cp 変数、0x1adf8d は flags 1） |

- **FLEngine の MultiByteToWideChar の呼び出し点のうち、dwFlags が 4（MB_USEGLYPHCHARS）のもの**（0x616539。cp は直前の `call 0xa16460` の結果）は、決定 0084 の範囲では停止する（flags の 2、4 は :571 で停止）。**実行が届いたときの cp と入力は未確認**（読み）。
- Blend2D のスロット参照 **GetStringTypeW 1 か所**の呼び出し元 `0x1aae6c`（Blend2D の `setSBUpLow` 相当）は、`mov byte [rsp+0x70], 0x20`（0x1aaedc）、`dwInfoType = 1`（0x1aaf1b）、cch = 0x100（`esi = 0x100`、0x1aaec3）で、**dsp の `f_37d674c` と同じ形**（C1 のビットの読み方も同じ: `test byte [rdx], 1` → `or byte [..+0x18], 0x10`、`test byte [rdx], 2` → `or … 0x20`。0x1aafab〜0x1aafc0）。呼び出し元の連鎖: `0x1ab3ea` ← `0x1ab12e` ← `0x1ab315` ← `0x1a5082`（3 つの呼び出し元）← `0x19df3b`（読み）。**Blend2D を実在モジュールとして載せる（決定 0086）と、同じ 256 単位、同じ値が要る**。ほかに 1 文字ずつの GetStringTypeW（0x1abbb4）があり、これは ATTACH の `setSBUpLow` ではなく通常の実行（`isalpha` 相当）で入力が変わる。**GetStringTypeW を「256 単位だけ」にすると、Blend2D の 0x1abbb4 には効かない**。
- QuickFontCache は **GetStringTypeW を import しない**。LCMapStringW は 1 か所（flags 0x01000100）、WideCharToMultiByte は 3 か所（cp は 1 と変数）。dsp と同じ ATTACH の `setSBUpLow` は無い。
- FLEngine は GetStringTypeW を import しない。LCMapStringW を 2 か所（LINGUISTIC_CASING 付き LOWER／UPPER。Locale = 変数。入力は文字列）、WideCharToMultiByte を 2 つの薄い入口から計 9 か所、MultiByteToWideChar を計 11 か所持つ。**これらの呼び出し点の入力の範囲は、この調査では測っていない**（FLEngine は ATTACH の `setSBUpLow` を持たない。`import-locale-spec.md` §3 の範囲）。
- 参考（**LCMapStringEx への呼び出しは 4 つのモジュールのどれにもスロットとして無い**）。dsp の CRT が `GetProcAddress("LCMapStringEx")` で動的に引く点は別調査の範囲（`dsp-ippv2-apiset-gpa.md`）。

## 7. 限界、未確認

- **実機の Windows は無い**（ホストは Linux）。Windows 10 22H2（決定 0014）の GetStringTypeW(CT_CTYPE1)、LCMapStringW、WideCharToMultiByte(1252) の実際の値は、どれも確認していない。Wine（10.0）は非公式。
- C1 の値の Microsoft の出典は**見つからなかった**。探した範囲は、Learn の GetStringTypeW、MS-UCODEREF v20240423 の本文、Code Page Data Files の zip の全 111 ファイル、Sorting Weight Table（Windows 10 版）。**Windows 8 以外の Sorting Weight Table（Vista、7、11、Server 2022 など 7 ファイル）は取得していない**（Windows 10 版だけ調べた）。MS-UCODEREF の付録、図は PDF のテキストの範囲で調べた。
- `f_37d674c` を `setSBUpLow`、`f_37d6934` を `_setmbcp` などと呼ぶのは**形の類似による読み**。
- `_mbcasemap` を文字で引く命令は走査で見つからなかったが、**走査は byte 幅、`[base+index+0x19/0x119]` 形のみ**で、間接参照、レジスタに加算済みのアドレス、`rep` 系の読みは拾えない。
- IPP の通常の実行が `_mbctype`、`_mbcasemap` を読むか: エクスポートからの直接の辺では届かず、IPP の 27 関数にグローバルへの参照が無いことだけを確認した。**関数ポインタ経由と、gdb でのエクスポートの実行は未実測**。
- 失敗を返したときの「スタックの残り」は、この run のこのビルドでの値（実機の CRT の呼び出しの前後の内容）ではない。
- gdb の仮の値は ATTACH の通過だけを測った。**DllMain が戻った後に IPP の関数を実行して表の値が効くかは測っていない**。
- UCD の版: 取得した 16.0.0 を主に使った。5.2.0、6.0.0、6.1.0、8.0.0 は U+00AA、U+00BA の General_Category の確認だけ。
- Blend2D の `GetStringTypeW` の 2 つ目の呼び出し元（0x1abbb4）の呼び出し元は 1 つ（0x1a5df1）のみ確認し、その先は追っていない。
- 決定 0085、api-set の調査の作業ツリーの変更には触れていない。`dsp-ippv2-apiset-gpa.md` は読んだ（HEAD c75acc4 の追跡下のファイル。この調査の範囲の外の判断は引いていない）。

## 8. 決定が要る点（決めない。選択肢と事実）

1. **GetStringTypeW(CT_CTYPE1) の値の出典の格**。Microsoft の資料に値は無い（§5.1）。選択肢: (a) 非公式の Wine の値を「仮」として採る（決定 0041 の Wine の先例と同じ形。Windows の実測ではない。ATTACH が読むのは UPPER、LOWER の 2 ビットだけで、Wine と UCD の `Lu`/`Lowercase` で再現できる）、(b) UCD（General_Category / Uppercase / Lowercase）を出典にする（Microsoft が規範として参照している確認は無い。U+00AA、U+00BA の扱いが UCD の版と派生プロパティで変わる）、(c) 停止のまま、(d) 失敗を返す（応答を偽る。§4.3 の `fgstw` はスタックの残りで表が作られる）。
2. **実装範囲**（§5.4 の A〜E）。**呼び出し点が Blend2D（決定 0086 で実在モジュールにする対象）にもある**ことが範囲の判断に効く（§6）。
3. **LCMapStringW の小文字化の出典**。Microsoft に表が無い。選択肢: (a) MS8 の大文字表の逆引き（255 単位で Wine、UCD と一致。**逆引きを使うのはこの調査の読み**）、(b) UCD の Simple_Lowercase_Mapping（255 単位で一致。BMP 全体で MS8 の逆引きと 205 点で違う）、(c) 停止のまま。大文字化は MS-UCODEREF 3.1.5.3 と MS8 の表が出典（ATTACH の入力では UCD との差は U+00B5 の 1 単位）。
4. **WideCharToMultiByte(1252) の best-fit と dwFlags**。出典は WCTABLE（決定 0084 の zip）。ATTACH の入力で best-fit が効くのは U+0191 → 0x83 の 1 単位（`_mbcasemap[0x83]` への効果は C1_LOWER の扱い次第）。選択肢: (a) WCTABLE の 698 行を全部採る（best-fit を採る。flags 0 のとき。WC_NO_BEST_FIT_CHARS（0x400）は MBTABLE の逆引き 256 件に限る、は読み）、(b) 厳密な逆引き 256 件だけ（best-fit は停止）、(c) ATTACH の入力に限る。MS-UCODEREF の疑似コードは dwFlags と lpUsedDefaultChar を扱わない。
5. **LCMapStringW の Locale=0**。Learn にも MS-LCID にも無い（§5.2）。選択肢: (a) 0 をユーザー既定（0x400）と同じに扱う（非公式 Wine はそうなる）、(b) 0 を別の値として stop のまま（`fle_locale_resolve` に 0 を足さない。ATTACH で LCMapStringEx が NULL のときに必ず入る点は、別調査の GetProcAddress の判断次第）、(c) ATTACH の入力に限って通す。**どちらの選択でも、Learn に書かれた根拠は無い**。
6. **失敗を返す選択肢の扱い**。§4.3 のとおり、ゲストは失敗を検出しない。GetStringTypeW を失敗させると DllMain は通るが、`_mbctype` はスタックの残りで作られ、実機と一致する保証が無い。
7. **仮の決定にするなら撤回条件**: 実機の Windows 10 22H2 で値が違うと確認されたとき（決定 0084 と同じ形）。

## 9. 作ったファイル

- 本文書 `docs/investigations/dsp-ippv2-ctype-locale-data.md`（追跡下。新規。**編集した唯一の追跡下のファイル**）。
- `lift/out-ctype/`（gitignore 対象。2.6 GB のうち最大の単一ファイルは `two/v-tbl2/native/two-native` 571 MB、`real.c` 266 MB。2 GB 級の単一ファイルは作っていない）:
  - `src/`（`git archive HEAD` の展開）、`target/`（fe-lift、fe-pe）、`two/`（2 モジュール構成。`dll/`、`gen/`、`src/`、`native/`、`v-tbl2/`）、`out-tbl2/a4/`（a4-survey のログ）。
  - `gdb/ct_drv.py`、`gdb/ct_rules.py`（仮の値の規則。**測定用で決定ではない**）、`gdb/run_ct.sh`。`run/`: 各 run の `*-gdb.log`、`*-dir/`（ダンプ）、`stage-table.txt`、`compare-sources.txt`、`measured-inputs-check.txt`、`c1-upper-lower-compare.txt`、`wine-c1-runs.txt`、`ms8-vs-ucd-fullbmp.txt`、`xref-*.txt`、`sites-*.txt`、`args-*.txt`、`readers*.txt`。
  - `ct_pe.py`、`ct_xref.py`、`ct_sites.py`、`ct_args.py`、`ct_readers.py`、`ct_readers2.py`、`ct_inputs.py`、`ct_tables.py`、`ct_compare.py`、`ct_analyze.py`、`ct_wineparse.py`、`ct_quotecheck2.py`（標準ライブラリと capstone だけ。`PYTHONDONTWRITEBYTECODE=1` で実行し、`__pycache__` は作っていない）、`asm/`（逆アセンブル）。
  - `wine/ct_wine.c`、`wine/ct_wine.exe`（mingw のクロスコンパイル。**非公式の参照測定用**）。`wineprefix/`（Wine の設定。`lift/out-ctype/` の下）。
  - `ref/`: Microsoft の zip（SHA-256 `5074e6dd…`）、Windows 8 Upper Case Mapping Table（`76a66e29…`）、Windows 10 Sorting Weight Table（`81fcfa1e…`）、MS-UCODEREF の PDF（`a8473d1d…`）とそのテキスト、Learn と MS-LCID のページの保存（`ref/learn/live-*.html`。取得 2026-10-23T23:51Z）、UnicodeData 5.2.0〜16.0.0（unicode.org。**非公式**）、PropList、DerivedCoreProperties（16.0.0）。
  - `tables/`: `in-units.txt`（ATTACH の 256 単位）、`candidates.tsv`（各出典の値の対照表）、`c1-ucd.txt` ほか。
- 再現: `lift/out-ctype/` の `gdb/run_ct.sh` と §2 の手順。
