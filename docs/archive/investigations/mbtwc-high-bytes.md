# MultiByteToWideChar の 0x80 以上のバイト: dsp_ippv2_x64.dll の呼び出し点、現行 runtime の停止、仕様、決めない選択肢

調査日: 2026-10-23〜24。HEAD 9ae09a0（main）。**調査のみ。この文書は何も決めない**。`lift/` 配下は 1 ファイルも編集していない（読み取りだけ。runtime のソースは `lift/out-mbtwc/tmp/` へ出したバイナリのために直接コンパイルしたが、変更していない）。別の作業者の未コミットの変更（`docs/README.md`、`lift/harness/two/patches_stage1.py`）には触れていない。DLL（`lift/out-ext/dsp_ippv2_x64.dll`、SHA-256 497dd4f77e859df9e86d999b3811be3c72526b41f86c38b0c3c6e395f691c352 を再計算して一致）は**実行していない**。

背景: W6（決定 0078 §8）は「ATTACH は IsValidCodePage と GetCPInfo の先で 256 バイトを MultiByteToWideChar に渡し、0x80 以上のバイトで既存の `fle_locale.c`（決定 0016）が止まるはず」と書いた。`stage23-two-module-real-dll.md` §5.1 段 2 は、DllMain(ATTACH) が GetStartupInfoW（決定 0083 で解消）の先で MultiByteToWideChar（cp1252、0x80 以上）で止まり、gdb の恒等変換で通したと実測している。この文書は、その停止を DLL 側と runtime 側と仕様の 3 方向から確かめ、決めない選択肢を並べる。

## 0. 凡例

- **実測** = このセッションで実行した出力（`lift/out-mbtwc/` にある）。
- **読み** = 逆アセンブル（capstone、objdump）、ソース、ページの文を読んだ結果。実行していない。
- **既存ログ** = 他の作業の記録（`stage23-two-module-real-dll.md` など）を読んだ。再実行していない。
- **推測**、**未確認** = そのとおり。
- 前回（途中終了した 1 回目）の成果物は読み取って使ったが、結論に使う事実は自分で再確認した。再確認の出力は `*-recheck.txt`、`redo-*.txt`、`riprefs-*.txt` など。**前回の `ref/ms-codepagefiles-compare.txt` の 437 の「差異 32 件」は読み違い**（zip の `437.txt` にある GLYPHTABLE を MBTABLE に混ぜていた）。今回の `tablecheck-result.txt` で訂正した（§5.2）。

## 1. 要約（事実と読みを分けて）

| 問い | 答え | 区分 |
|---|---|---|
| MultiByteToWideChar（MBTWC）の呼び出し点は | IAT スロット 0x1ebd230 を読む命令は DLL 全体で **1 つだけ**（`jmp [IAT]`、VA 0x1817d709b。`.pdata` の外にある 0x1817d7048 のフラグ調整ラッパーの末尾）。ラッパーの呼び出し点は **8 か所**（4 関数） | 実測（バイト走査と逆アセンブル） |
| そのうち ATTACH の直線上は | 4 か所（0x17d86c0 の 2 つ、0x17d9a94 の 2 つ）。実行回数は 2 + 2×2 = **6 回**。既存ログの実測（gdb で MBTWC 6 回）と一致 | 読み、既存ログと整合 |
| 0x80 以上を渡す箇所は | `0x17d674c`（CRT のロケール初期化の 256 バイト表。UCRT の `setSBUpLow` に当たる、は読み）が作る局所配列 `buf[256]`（`buf[i] = i`、`buf[0] = 0x20`）を、cp = 1252、dwFlags = 1、cbMultiByte = 256 で渡す。0x80〜0xFF は位置 128〜255 に 1 回ずつ入る | 読み（cp、flags、cb は既存ログの実測と一致） |
| 現行 runtime は | cp ∈ {0, 1, 3, 1252, 437} × dwFlags ∈ {0, 1, 8, 9} の 20 通りすべてで、**0x80〜0xFF の 128 バイトが全部停止**。0x00〜0x7F は全部通る。256 バイトの呼び出しは、サイズ問い合わせ（cch = 0）でも停止する | 実測（`sweep-final.txt`） |
| W6 の応答との整合 | GetACP 1252、IsValidCodePage(1252) = 1、GetCPInfo(1252) = {MaxCharSize 1、既定文字 `?`、先頭バイトなし} は連鎖として整合し、CRT を単一バイトの経路（0x17d674c）へ進める。GetOEMCP（437）は ATTACH で呼ばれない | 読み、実測 |
| 1252 の 0x80〜0x9F の Unicode 値は一次資料にあるか | Win32 の Learn のページ（MBTWC、GetCPInfo、CPINFO、Code Page Identifiers、Code Pages）には**無い**。Learn の旧版（previous-versions、アーカイブ）に 1252 の表の**画像**があるが、2008 年のもので、0x80・0x8E・0x9E を NOT USED とする（現行の Microsoft のデータファイルと違う）。Microsoft のデータファイル（Download Center の zip の `1252.txt`）には 256 件の表がある。Learn の MS-UCODEREF がこの zip を [CODEPAGEFILES] として参照する | 実測（取得と照合） |
| 0x80 以上を渡さずに済む分岐は | CRT 内にはある（ACP = 65001、GetCPInfo 失敗、IsValidCodePage 失敗など。§2.5）。ただしどれも、決定 0016・0078 の応答を変える（偽る）ことになる。1252 のままでは避けられない | 読み |
| 決定 0016 の範囲で実装できるか | **できない**。0016 自身が「0x80 以上は 1252、437 の表が MSDN のページにないので abort」と書いている。表の出典を「Microsoft のデータファイル」まで広げるかは、別の判断（§6、§7） | 読み、判断は PM |

停止の連鎖（表が 1252 だけで足りない）: MBTWC(1252) → GetStringTypeW（決定 0078 §3 の名前付き停止）→ LCMapStringW（lcid = 0。runtime は 0x400、0x800、0x409、0x7F だけ）→ WideCharToMultiByte(1252)。4 つとも別の表か規則が要る（§3.3）。

## 2. dsp_ippv2_x64.dll の MBTWC の呼び出し点

### 2.1 IAT スロットの参照は 1 つだけ

- PE: x64、ImageBase 0x180000000、エントリ RVA 0x17caca8（DllMain）。KERNEL32 の MultiByteToWideChar は IAT スロット RVA 0x1ebd230（`mbtwc_sites.py`。`sites-mbtwc-recheck.txt`）。
- `.text` と `IPPCODE` の全バイトから、rel32 が 0x1ebd230 に当たる位置を探した（`mbtwc_slotscan.py`、`slotscan-recheck.txt`）。当たりは 2 つ: `.text` RVA 0x17d709e（`48 ff 25 8e 61 6e 00` = `jmp [rip+0x6e618e]`）と、`IPPCODE` RVA 0x1e9e4a8。後者は命令ではない（`e9 04 0f 84 ed 01 00` の途中で、前後の命令に当該スロットを読むものがない。前回の `fp-check.txt` と今回の `riprefs-mbtwc-slot.txt` で確認）。
- 命令として復号して確かめた版（`mbtwc_riprefs.py`、`riprefs-mbtwc-slot.txt`）は、0x17d709b の `jmp [IAT]` の 1 つだけ（0x17d709c の行は同じバイトの誤った位置からの復号）。8 バイトの絶対ポインタ（イメージベース + RVA）は、`.text`、`.rdata`、`.data`、`.pdata` を含む全セクションに無い。
- したがって、MBTWC を呼ぶ経路は**すべて 0x17d7048 のラッパー**を通る。

### 2.2 ラッパー 0x17d7048

`.pdata` に無い（0x17d7020 のエントリは [0x17d7020, 0x17d7045) で終わる。`pdata-recheck.txt`）。dwFlags（edx）をコードページに応じて調整し、`jmp [IAT]` する（`redo-d7020.txt`）:

| コードページ | 調整 |
|---|---|
| 50220、50221、50222、50225、50227、50229、42、57002〜57011、65000 | dwFlags = 0 |
| 65001、54936 | dwFlags &= 8（MB_ERR_INVALID_CHARS だけ残す） |
| それ以外（1252 を含む） | そのまま |

これは Learn の文と同じ規則（§4）。1252 では dwFlags は変わらない。

### 2.3 ラッパーの呼び出し点（8 か所）

`xrefs_to.py`（`xrefs-recheck-wrapper.txt`、`xrefs-recheck-graph.txt`）で 8 か所。引数は逆アセンブル（`redo-*.txt`）の読み。

| # | 呼び出し点 | 関数 | 役割（読み。CRT のどの処理か） | CodePage | dwFlags | cbMultiByte | 入力の出所 | ATTACH の直線上か |
|---|---|---|---|---|---|---|---|---|
| 1 | 0x17d4029 | 0x17d3f3c | mbtowc 相当（1 バイト、または先頭バイト + 次の 1 バイトを 1 文字にする）。65001 は別経路（0x17d8a30） | ロケールの cp（`[[ロケール+0x18]+0xc]`） | 9（PRECOMPOSED｜ERR_INVALID_CHARS） | 先頭バイトのとき `[mb情報+8]`（mb_cur_max） | 呼び出し側の文字列（ユーザーのデータ） | **いいえ**（§2.6） |
| 2 | 0x17d4080 | 0x17d3f3c | 同（1 バイト） | 同 | 9 | 1 | 同 | いいえ |
| 3 | 0x17d58b7 | 0x17d57f8 | argv のワイルドカード展開の中の、狭→ワイド変換（サイズ問い合わせ） | 引数（呼び出し元は未追跡） | 9 | -1（NUL 終端） | argv の文字列 | **いいえ**（§2.6） |
| 4 | 0x17d593d | 0x17d57f8 | 同（変換） | 同 | 9 | -1 | 同 | いいえ |
| 5 | 0x17d8733 | 0x17d86c0 | GetStringTypeA 相当のラッパー（MBTWC でワイドにして GetStringTypeW。サイズ問い合わせ） | `[0x17d674c の mbcinfo+4]`（設定中のコードページ。ATTACH では 1252） | 1 | 256 | **0x17d674c の局所配列 `buf[256]`** | **はい** |
| 6 | 0x17d87dc | 0x17d86c0 | 同（変換。cch = 手前の戻り値） | 同 | 1 | 256 | 同じ `buf[256]` | はい |
| 7 | 0x17d9b1b | 0x17d9a94 | LCMapStringA 相当（MBTWC → LCMapStringW → WideCharToMultiByte）。サイズ問い合わせ。0x17d9db4 が薄い入口 | 同（`[mbcinfo+4]`） | 1 | `strnlen(buf, 256)`。`buf[0]` = 0x20 なので 256 | 同じ `buf[256]` | はい（0x17d674c が 0x17d9db4 を 2 回: LCMAP_LOWERCASE 0x100 と LCMAP_UPPERCASE 0x200） |
| 8 | 0x17d9bc1 | 0x17d9a94 | 同（変換） | 同 | 1 | 同 | 同 | はい |

- dwFlags の 1 は、`0x17d86c0` と `0x17d9a94` の中で `neg [引数]; sbb edx,edx; and edx,8; inc edx`（引数 0 のとき 1、非 0 のとき 9）と、2 回目の `mov edx, 1` で作られる。0x17d674c は引数に 0 を渡す（`and dword [rsp+0x30], 0`、`and dword [rsp+0x40], 0`）。既存ログの実測（`stage23` §5.1 段 2: cp = 1252、flags = 1、cb = 256、最初が cch = 0、次が 256）と一致する。
- 実行回数: 0x17d86c0 が 2 回（#5、#6）、0x17d9a94 が 2 回呼ばれて各 2 回（#7、#8）= 6 回。`stage23` §4 の表（MultiByteToWideChar 6）と一致（既存ログ）。
- ラッパーから見えるもう 1 つの出口、WideCharToMultiByte のラッパー 0x17d70a4 の呼び出し点は 9 か所（0x17d43db、0x17d4845、0x17d5a3b、0x17d5acb、0x17d62ba、0x17d6313、0x17d71d0、0x17d721e、0x17d9d2c）。ATTACH の直線上で 0x80 以上を含みうるのは **0x17d9d2c**（0x17d9a94 の中。LCMapStringW の結果の 256 ワイド文字をバイトへ戻す）。0x17d71d0、0x17d721e（環境ブロック）と 0x17d62ba、0x17d6313（モジュールのパス）は、仮想環境が空（NUL 1 つ。決定 0078）、パスが仮想パス（決定 0018）なので ASCII の想定（値は今回確認していない）。

### 2.4 ATTACH の直線（呼び出しの連鎖）

読み。各辺は `call`／`jmp` の位置を確かめた（`redo-attach-front.txt`、`xrefs-recheck-*.txt`）。

```
0x17caca8 DllMain        (reason==1 のとき call 0x17cb22c = クッキー初期化。そのあと jmp 0x17cab74)
 → 0x17cab74             (0x17cabe3 で call 0x17ca988。reason ごとの分岐)
 → 0x17ca988 → jmp 0x17ca9d8   (ATTACH)
 → 0x17ca9d8             (0x17ca9f5 で call 0x17cb484)
 → 0x17cb484             (0x17cb4b3 で call 0x17d2484。CRT の C ランタイム初期化)
 → 0x17d2484 → jmp 0x17d77d4   (初期化表 0x1ec8360〜0x1ec8460 を歩く。16 バイトの組の先頭を昇順に呼ぶ)
 → 表の index 10（0x1ec8400）の init = 0x17d6c5c   [項目は call [0x1ebd260]（CFG のディスパッチ）で呼ばれる = 間接呼び出し]
 → 0x17d6c5c             (0x17d6ca9 で call 0x17d6934。ecx = -3、dl = 1)
 → 0x17d6934             (0x17d6a9c で call 0x17d6cd8。ecx = GetACP の結果)
 → 0x17d6cd8             (0x17d6d80 で call 0x17d674c)
 → 0x17d674c             (0x17d680e で 0x17d86c0、0x17d6841 と 0x17d687a で 0x17d9db4)
```

- 表の項目の呼び出しは間接なので、直接辺の探索（`mbtwc_attach.py`）は表の項目を根にしている。これは読みの仮定で、`stage23` §4 の実測の呼び出し順（… GetStartupInfoW → GetStdHandle → GetCommandLineA/W → GetACP → IsValidCodePage → GetCPInfo → MultiByteToWideChar …）が、表の index 8（0x17d36ac。lowio）、index 9（0x17d7020。GetCommandLineA/W）、index 10（0x17d6c5c）の順と合う（既存ログ）。
- 0x17d6c5c の呼び出しは ATTACH で 4 経路ある（表の index 10、スロット 0x1ebd298 の 0x17d9e4c、0x17d1d74、0x17d1efc）。本体は先頭の `cmp byte [0x2561e54], 0; jne` で 1 回しか走らない（フラグは BSS で 0 から始まり、書くのは 0x17d6cae だけ。`riprefs-flags.txt`）。最初に走るのは表の index 10。
- `0x17d6934`: ecx = -3 は `0x17d6634` が GetACP に直す（`[0x2561e50]` = 1、0x17d666b）。ACP が 1252 なら、初期の mbcinfo（`0x2543970`。cp = 0）と違うので 0x228 バイトを割り当てて続行する。
- `0x17d6cd8`: 組み込みの 5 エントリの表（0x2543dc0。各 0x30 バイト。cp は 932、936、949、950、1361）に 1252 は無い。IsValidCodePage(1252) → 1、65001 ではない、GetCPInfo(1252) → 成功、`MaxCharSize == 2` ではない（単一バイト）ので 0x17d6e59 → `[mbcinfo+8] = 0` → 0x17d6d7d で **0x17d674c を呼ぶ**。

### 2.5 0x17d674c が作る 256 バイトの表（0x80 以上を渡す箇所）

読み（`redo-d674c.txt`、`crt-table-semantics.txt`）:

1. `cp == 65001` なら 0x17d68d0（ASCII の算術だけ。MBTWC なし）へ。
2. `GetCPInfo(cp, &cpinfo)` が 0 なら同じ 0x17d68d0 へ。
3. `buf[0..255]` に 0、1、2、…、255 を入れ（0x17d679c〜0x17d67b1）、`buf[0] = 0x20`（0x17d67bc）。CPINFO の LeadByte（`rsp+0x56`）の各組 [lo, hi] の範囲を 0x20 で塗る（1252、437 は先頭バイトが無いので塗らない）。結果は `0x20, 0x01, 0x02, …, 0xFF`。
4. 0x17d86c0（#5、#6）で `CT_CTYPE1` を取る（GetStringTypeW は `[0x1ebd020]`、0x17d87f9）。
5. 0x17d9db4 を 2 回（LCMAP_LOWERCASE、LCMAP_UPPERCASE）。中の LCMapStringEx は `GetProcAddress` が NULL（決定 0081）なので、`LocaleNameToLCID`（同じく NULL）→ DLL 内の 0x17d9f1c（名前が NULL なら 0 を返す）→ LCMapStringW(**lcid 0**, 0x100 / 0x200, 256 ワイド)（0x17d76b9）。そのあと WideCharToMultiByte(1252, 0, 256 ワイド)（0x17d9d2c）。
6. 256 回のループ: `ctype1[i] & 1`（C1_UPPER）なら `flags[i+1] |= 0x10`、小文字側のバイトを `lower[i]` から。`& 2`（C1_LOWER）なら `|= 0x20`、`upper[i]` から。それ以外は 0。

`stage23` §5.1 段 4 の「ロケール名が NULL になる理由は読んでいない」への答え（読み）: 名前は `[mbcinfo+0x220]` で、0x17d6e10〜0x17d6e50 が cp 932、936、949、950 のときだけ `ja-JP`、`zh-CN`、`ko-KR`、`zh-TW`（`.rdata` 0x1ec9060〜0x1ec9078 の指す文字列。実測）を入れ、それ以外（1252 を含む）は 0。LCID 0 はそこから来る。

### 2.6 DETACH、argv、エクスポートからは

- 0x17d57f8（#3、#4）: `0x17d5ecc` → `0x17d621c`（`0x17d1d74` の中、0x17d1e84）からだけ呼ばれる。`0x17d1d74` の引数は `0x17cb9c4`（`mov eax, 1`）の 1 で、`cmp r14d, 1; jne 0x17d1e79` の手前（0x17d1e75〜0x17d1e77）で終わるので、ワイルドカード展開は走らない（読み。決定 0078 §1 と同じ読み）。
- 0x17d3f3c（#1、#2）: `mbtwc_attach.py` の ATTACH の根（DETACH とスレッドの根を止めた探索）からは**届かない**（`attach-recheck.txt`）。止めずに DllMain だけを根にすると、DETACH 経由（0x17ca988 → 0x17caaf0 → 0x17cb618 → 0x17d24bc → … → 0x17d49f8 → 0x17d40b4 → 0x17d3f3c。`roots-dllmain.txt`）で届く。**この探索には、ノーリターンの呼び出し（0x17d6c56 の `call 0x17d25a0`）の後ろへ落ちる偽の辺がある**（0x17d6cbc → 0x17d6934 は、0x17d6c5c の本体に落ちた偽の辺。0x17d6934 の呼び出し元は 0x17d6ca9 の 1 つだけ。`xrefs-recheck-graph.txt`）。`roots-detach.txt` の 0x17d674c などへの鎖は信用しない。
- エクスポート 446 名から直接辺で届く 496 関数に、ここまでの関数（0x17d3f3c、0x17d57f8、0x17d674c、0x17d86c0、0x17d9a94、0x17d6c5c）は**入らない**（下限。`export-reach-recheck.txt`）。IPP の関数は、この表を直接は使わない（読み）。

### 2.7 0x80 以上を渡さずに済む分岐（読み）

| 条件 | 場所 | 結果 | W6 までの応答では |
|---|---|---|---|
| GetACP が 65001 | 0x17d677d（0x17d674c）、0x17d6d58（0x17d6cd8） | 0x17d674c が ASCII の算術だけ。MBTWC なし。GetCPInfo も呼ばない | 起きない（決定 0016: 1252） |
| GetCPInfo(cp) が 0 | 0x17d6794（0x17d674c）、0x17d6d97（0x17d6cd8） | 0x17d674c は同じく ASCII の算術。0x17d6cd8 は `[0x2561e50]`（OS 由来の cp のフラグ）≠ 0 なら 0x17d6f65（静的な ASCII の既定表を写す。UCRT の `setSBCS` の読み）で戻り 0 | 起きない（W6 の GetCPInfo は 1252 で成功） |
| IsValidCodePage(cp) が 0 | 0x17d6d4b | -1 を返し、0x17d6934 が errno = 22 にして -1。0x17d6c5c は戻り値を捨てて続行する（0x17d6cae）ので、静的な初期表のまま | 起きない（W6 は 1252 を 1） |
| GetACP が 0 | 0x17d6d09 | 0x17d6f65（既定表） | 起きない |
| cp が 932、936、949、950、1361 | 0x17d6d21 | 組み込みの表を使うが、0x17d674c は呼ばれ、256 バイトは MBTWC に渡る（先頭バイトの範囲は 0x20） | 起きない |
| 同じ cp が設定済み | 0x17d6980 | 0 を返して終了 | ATTACH の 1 回目は初期の cp が 0 なので起きない |

つまり、**1252 のまま**では 256 バイトの MBTWC は避けられない（読み）。避ける分岐は、応答を偽るか（GetCPInfo / IsValidCodePage / GetACP）、ACP を 65001 に変える（決定 0016 の変更）かのどちらか。

## 3. 現行 runtime（`lift/runtime/fle_locale.c`、決定 0016）の挙動

対象: HEAD の `fle_locale.c`（SHA-256 先頭 b0a4539f3535430c。W6 の 7b693f5 以降変わっていない。`fle_imports.c`、`fle_locale.h` も HEAD と同一）。調べ方: コードを読み、`probe_runtime`（前回のビルド）を `probe-results-recheck.txt` で再実行（前回の出力と差なし）、今回の掃引は現行ソースから `mbtwc_sweep.c` をビルドして実行（`sweep-final.txt`）。

### 3.1 MultiByteToWideChar

検査の順（`fle_imp_MultiByteToWideChar`、537〜586 行）: コードページの解決（CP_MACCP で停止）→ UTF-8 でも ANSI/OEM でもなければ停止 → dwFlags の検査 → cbMultiByte < -1 で停止 → 引数の検査 → 変換（サイズ問い合わせの 1 回目から全バイトを走査する）。

| 入力 | 結果 | 区分 |
|---|---|---|
| cp = 0（= 1252）、1（= 437）、3（= 1252。スレッドのロケールが既定のとき）、1252、437 | 単一バイト・ASCII 互換として扱う（決定 0016 の制限） | 読み |
| cp = 2 | 停止（CP_MACCP） | 実測 |
| cp = 65001 | UTF-8 の規則（flags 0 と 8 だけ。不正な列は、8 なしで停止） | 読み |
| 他の cp（1251、932、42、65000 など） | 停止 | 実測 |
| 単一バイトの cp で flags に 0x10 以上のビット、または 1 と 2 の同時（3、7） | 0 と ERROR_INVALID_FLAGS（1004） | 実測 |
| 同 flags に 2 または 4 を含む（上を通ったもの。2、4、5、6、0xa、0xc） | 停止（MB_COMPOSITE、MB_USEGLYPHCHARS の効果がページに無い） | 実測 |
| cb < -1 | 停止 | 実測 |
| src NULL、cb = 0、cch < 0、src と dst が同じ | 0 と 87 | 読み |
| バイト 0x00〜0x7F（どの flags でも） | b → U+00bb（128 バイトとも ret = 1） | 実測 |
| **バイト 0x80〜0xFF** | **停止**。メッセージ `byte of 0x80 or above in a single-byte code page: table not on the MSDN pages`（503 行） | 実測 |
| 256 バイト（CRT の呼び出し。cb = 256、cch = 0 と 256） | 停止（サイズ問い合わせでも） | 実測 |
| 128 バイト（0x20、0x01〜0x7F、cb = 128）は通る。129 バイト（0x80 を含む）は停止 | | 実測 |

`sweep-final.txt` の要点: cp ∈ {0, 1, 3, 1252, 437} × dwFlags ∈ {0, 1, 8, 9} の 20 通りで、0x00〜0x7F は 128 返り（ret ≠ 1 は 0）、0x80〜0xFF は 128 停止（返り 0）。MB_ERR_INVALID_CHARS（8）付きでも「無効な文字」の判定をせずに停止する。

### 3.2 W6 の応答との整合

| API | 現行の応答 | CRT の 256 バイトの段との関係 |
|---|---|---|
| GetACP | 1252 | 0x17d6634（-3 の解決）で使われる。整合 |
| GetOEMCP | 437 | ATTACH では**呼ばれない**（0x17d665e は ecx == -2 のときだけ。ATTACH は -3）。437 の表は ATTACH に不要 |
| IsValidCodePage | 1252、437 は 1。ほかは停止（0、1、3、65001、932、99999 など。実測） | ATTACH が問うのは 1252 だけ。整合 |
| GetCPInfo | cp 0、1、3、1252、437 は ret = 1、`CPINFO: 01 00 00 00 3f 00 00 …`（MaxCharSize 1、DefaultChar `?`、LeadByte 全部 0）。65001、932、1251、2 は停止。NULL は 87 | CRT は単一バイトの経路（0x17d6e59）へ進む。Learn の CPINFO のページ（§4）と、Microsoft のデータファイルの `CPINFO 1 0x3f 0x003f`（§5）の両方と一致する |
| WideCharToMultiByte | 単一バイトの cp は 0x00〜0x7F のワイド文字だけ。0x80 以上（0x00e9、0x20ac、0x00ff）は停止。flags は 0 と 0x400 だけ | 0x17d9d2c の 256 ワイド文字の変換で止まる |
| GetStringTypeW | 常に停止（決定 0078 §3: 文字ごとの種類の表がページに無い） | 0x17d87f9。MBTWC が通ったら次にここで止まる |
| LCMapStringW | lcid 0x400、0x800、0x409、0x7F は ASCII の大文字小文字だけ。**lcid 0 は停止**。0x80 以上のワイド文字は停止（256 ワイドは lcid に関係なく停止） | 0x17d76b9。lcid = 0（§2.5）で止まる |

### 3.3 決定 0016 が決めていること、既存の表

- 決定 0016 §1: 環境の設定として ACP = 1252、OEM = 437（「**これは仕様ではない**」）。コードは 2 つのコードページが単一バイトで ASCII 互換であることを前提にする（一般知識で、MSDN の記述ではない）。§2: 「仕様のページで規則が確認でき、en-US の範囲で実装できるものだけ実装した。範囲外（…ページが結果を定めない入力、表が要るバイト値）は `fle_locale_stop`」「黙って値を作らない」。限界: 「MultiByteToWideChar、WideCharToMultiByte は UTF-8 と、ANSI/OEM の ASCII（0x80 未満）だけ。0x80 以上は 1252、437 の表が MSDN のページにないので abort」。
- `import-locale-spec.md` §4: 「コードページ変換の 0x80 以上 | 1252、437 の表がページにない（Code Page Identifiers のページは識別子の一覧だけ）」。
- W6 以前に `fle_locale.c` が持つ表は**無い**（`git show 7b693f5~1:lift/runtime/fle_locale.c` に表なし。`lift/` と `docs/` に `gen_cp…` 系の名前も無い）。既存のテスト `locale.rs` の 852 行は、1252 の 0x80 が停止することを表明している（`mbtwc-high-byte-1252`）。
- 先例: 決定 0041 は、ページに値が無い LCType に非公式の出典（Wine）を使い、「仮」「非公式」と明記して採った。決定 0072 は、一次（Learn の 1 文）だけを根拠にして、Wine のテストを「根拠にしないもの」とした。どちらも仮の決定（撤回条件付き）。**出典の格を決めるのは PM**（この文書は決めない）。

## 4. 仕様として言えること（Microsoft Learn。引用は 2026-10-24 に再取得したページの原文）

取得したページは `lift/out-mbtwc/learn/live-*.md`（Markdown 版）。引用 49 件が当該ページに実在することを `mbtwc_quotecheck.py` で機械的に確かめた（`quotecheck-result.txt`: total 49、fail 0。空白と Markdown の強調記号だけ正規化）。ページの日付（`ms.date`）は各項に書く。

### 4.1 MultiByteToWideChar（https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-multibytetowidechar 、ms.date 2024-02-05）

- CodePage: 「This parameter can be set to the value of any code page that is installed or available in the operating system.」CP_ACP は「The system default Windows ANSI code page.」、CP_OEMCP は「The current system OEM code page.」。いずれも「This value can be different on different computers…」と注記する（値は定めない）。
- dwFlags: MB_PRECOMPOSED が既定。「MB_PRECOMPOSED and MB_COMPOSITE are mutually exclusive.」（1 と 2 の同時は ERROR_INVALID_FLAGS の読み）。「For the code pages listed below, dwFlags must be set to 0. Otherwise, the function fails with ERROR_INVALID_FLAGS.」（50220、50221、50222、50225、50227、50229、57002〜57011、65000、42）。「For UTF-8 or code page 54936 (GB18030, starting with Windows Vista), dwFlags must be set to either 0 or MB_ERR_INVALID_CHARS.」（0x17d7048 のラッパー（§2.2）はこの規則と同じ形）。1252 に対する flags の制限は**書かれていない**。
- MB_PRECOMPOSED の効果: 「The use of the MB_PRECOMPOSED flag has very little effect on most code pages because most input data is composed already.」「The default behavior of this function is to translate to a precomposed form of the input character string.」
- サイズ: 「If this parameter is set to a positive integer, the function processes exactly the specified number of bytes.」「If this value is 0, the function returns the required buffer size, in characters, including any terminating null character, and makes no use of the lpWideCharStr buffer.」（CRT の 256 バイトは cb = 256 の明示長で、戻り値は 256 になる、は読み。NUL を特別扱いしない前提）。
- 無効な文字: 「Fail if an invalid input character is encountered.」（MB_ERR_INVALID_CHARS）。無効な文字の定義: 「A character that is not the default character in the source string, but translates to the default character when MB_ERR_INVALID_CHARS is not set.」と DBCS の先頭バイトだけの文字。「Starting with Windows Vista, the function does not drop illegal code points if the application does not set this flag, but instead replaces illegal sequences with U+FFFD (encoded as appropriate for the specified codepage).」
- **1252 の 0x80〜0xFF の Unicode 値は、このページに無い**。ページに表も、1252 の個別の記述も無い（`grep` で 20AC、0x80 とも 0 件。`live-win32-multibytetowidechar.md`）。

### 4.2 IsValidCodePage、GetCPInfo、CPINFO、GetACP

- IsValidCodePage（…/winnls/nf-winnls-isvalidcodepage、ms.date 2018-12-05）: 「A code page is considered valid only if it is installed on the operating system. Unicode is preferred.」「Starting with Windows Vista, all code pages that can be installed are loaded by default.」どのコードページが入っているかは書かれていない。
- GetCPInfo（…/winnls/nf-winnls-getcpinfo、ms.date 2018-12-05）: 「Returns 1 if successful, or 0 otherwise.」エラーは ERROR_INVALID_PARAMETER だけ。
- CPINFO（…/winnls/ns-winnls-cpinfo、ms.date 2018-12-05）: 「If the code page has no lead bytes, every element of the array is set to NULL.」「The default is usually the "?" character for the code page.」MaxCharSize は SBCS のとき 1（原文: 「The length can be 1 for a single-byte character set (SBCS)…」）。
- GetACP（…/winnls/nf-winnls-getacp）: 「Returns the current Windows ANSI code page (ACP) identifier for the operating system.」値は書かれていない。
- これらと W6 の応答（§3.2）は矛盾しない。「1252 と 437 は入っている」は環境の設定（決定 0016、0078。ページに無い）。

### 4.3 Code Page Identifiers と Code Pages（…/windows/win32/intl/code-page-identifiers、ms.date 2018-05-31。…/intl/code-pages、ms.date 2018-05-31）

- Code Page Identifiers の表: 「1252 | windows-1252 | ANSI Latin 1; Western European (Windows)」「437 | IBM437 | OEM United States」。名前だけで、バイトと文字の対応は無い（1252 と 437 の行には 20AC など値の語が無い。`live-win32-code-page-identifiers.md`）。
- Code Pages: 「The usual OEM code page for English is code page 437.」「Windows code page 1252 and OEM code page 437 are generally used in the United States.」「Characters represented by the remaining codes, 0x80 through 0xff, vary among character sets.」また 「Windows code page 1252, the code page commonly used for English and other Western European languages, was based on an American National Standards Institute (ANSI) draft.」「but Windows code page 1252 was implemented before the standard became final, and is not exactly the same as ISO 8859-1.」（= 1252 は Latin-1 と同一ではない、という文だけ。差の内容は書かれていない）。
- 「0x00 through 0x19 and 0x7F always represent standardized control characters」の文は、1252 の 0x00〜0x7F が ASCII であることの根拠になるが、0x1A〜0x1F の扱いまでは言っていない（ページは 0x19 と書いている。原文のまま）。

### 4.4 Code Page 1252 の表（一次資料があるか）

**Win32 の Learn のページには無い**。次の 3 つの種類がある。出典の格が違うので分ける。

1. **Learn の旧版（previous-versions、アーカイブ）**: 「Code Page 1252 Windows Latin 1 (ANSI)」（https://learn.microsoft.com/en-us/previous-versions/cc195054(v=msdn.10) 、`ms.date` 2008-02-06、`is_archived: true`、`ROBOTS: NOINDEX`）。**本文は画像 2 枚だけ**（`cc195054.h0005(en-us,msdn.10).gif` = 16×16 の格子、`.h0006` = 縦の一覧。取得した gif の SHA-256: c2f29f5617ec…、c87751ddbd0e…）。目で読んだ結果（`learn/cc195054_h0006_*.png`、`tablecheck-result.txt`）:
   - 0x82〜0x8C、0x91〜0x9C、0x9F（24 行。目で転記した 24 件）は Microsoft のデータファイルの MBTABLE と**一致**した。0xA0〜0xFF は 00xx の列を目で見ただけ（恒等と読める。転記して比べてはいない）。
   - **0x80、0x81、0x8D、0x8E、0x8F、0x90、0x9D、0x9E は「NOT USED」（Unicode 欄 FFFE）**。現行の Microsoft のデータファイルは 0x80 = U+20AC、0x8E = U+017D、0x9E = U+017E を持ち、0x81、0x8D、0x8F、0x90、0x9D は C1 制御（U+0081 …）に写す。**2008 年の旧版と現行のデータファイルで、3 バイト（0x80、0x8E、0x9E）が食い違う**。
   - 同じ旧版の付録 H（https://learn.microsoft.com/en-us/previous-versions/cc195051(v=msdn.10) 、ms.date 2008-02-06）は、「Each of the Windows code pages, 1250 through 1257, is followed by a list showing the individual characters, the corresponding Unicode IDs, and the Unicode descriptive names.」と書き、MS-DOS の 437 を「MS-DOS Latin US」として一覧に載せる（表は別ページ。**437 の表のページは取得していない**）。
2. **Microsoft のコードページデータファイル**（Microsoft Download Center id=10921、「Windows Supported Code Page Data Files.zip」、2009 年。取得した zip の SHA-256 5074e6dd253056ba61fc6c870c9a955467855129c6ad3a51761c386b301b125a。`download.microsoft.com` から再取得して一致）。`CodpageFiles/1252.txt`（SHA-256 72ea23c9…）に `MBTABLE 256` と `WCTABLE 698`、`CodpageFiles/437.txt`（45305eac…）に `MBTABLE 256`、`GLYPHTABLE 32`、`WCTABLE 729`。**Learn の MS-UCODEREF が、このファイル群を仕様の入力として参照する**:
   - [MS-UCODEREF] 2.2.2「Supported Codepage Data Files」（…/openspecs/windows_protocols/ms-ucoderef/d167d756-0c58-4564-a0de-77c647d367fa）: 「The data files of supported system codepages are published as specified in [CODEPAGEFILES], [UNICODE], and [UNICODE-BESTFIT]. … bestfit1252.txt contains the data for codepage 1252.」
   - 1.2.1「Normative References」: 「[CODEPAGEFILES] Microsoft Corporation, "Windows Supported Code Page Data Files.zip", 2009」。
   - 3.1.5.1.1.3「Pseudocode for Mapping a Codepage String to a UTF-16 String」（…/5d543f48-e18b-4828-91d4-69b1488748cf）: 「OPEN SECTION SingleByteMapping where section name is MBTABLE from file with the name of CodePageFileName」。SBCS では MBTABLE の第 1 欄の一致する行の第 2 欄を出力する（行が無ければ既定文字）。疑似コードはコードページ名を `CodePage as a string, and ".txt"`（= `1252.txt`、zip のファイル名）と書く。逆向きの 3.1.5.1.1.2 は「Bestfit」+ 番号 + `.txt`（unicode.org の `bestfit1252.txt`）と書き、**両者のファイル名の規則が揃っていない**（ページの記述どおり。なお 1252 では 2 つの MBTABLE と WCTABLE が同一。下の 3）。
   - 2.2.2.1.2「MBTABLE」（…/c1c1ac17-8415-4bae-9fd0-6f979a4722ef）: 「The MBTABLE tag marks the start of the mapping from single-byte bytes to Unicode UTF-16.」
   - 2.2.1「Supported Codepage in Windows」: 「The usual ANSI codepage ID for US English is codepage 1252.」（437 は OEM の行）。
   - MS-UCODEREF は Open Specifications で、概要ページの知財の通知に「you can make copies of it in order to develop implementations of the technologies that are described in this documentation … This permission also applies to any documents that are referenced in the Open Specifications documentation.」とある。**これが zip のデータの再利用に及ぶかの法的な判断は、この調査ではしない**（原文を引くだけ。ライセンスの解釈は PM）。zip のファイル自体にはライセンス文が無い（`1252.txt` に "copyright" の語は、`;Copyright Sign` の文字名だけ）。
3. **unicode.org が公開する同じ Microsoft のファイル**: `VENDORS/MICSFT/WindowsBestFit/bestfit1252.txt`（37057 バイト、SHA-256 72ea23c9…。**zip の `1252.txt` と同一**。再取得で確認）と `VENDORS/MICSFT/WINDOWS/CP1252.TXT`（1998 年版 2.01。0x81、0x8D、0x8F、0x90、0x9D を**載せない**。載せた 251 件はすべて zip の MBTABLE と同じ）、`PC/CP437.TXT`（256 件。zip の 437 の MBTABLE と**完全に一致**）。MS-UCODEREF の [UNICODE-BESTFIT]、[UNICODE-README] が、この場所を参照している（1.2.1）。

つまり、**「一次資料にあるか」への答え**:
- Win32 の API ページ（MBTWC、GetCPInfo など）と Code Page Identifiers: **無い**。
- Microsoft が公開し、Learn の仕様（MS-UCODEREF、normative reference）が参照するデータファイル: **ある**（1252 も 437 も、MBTABLE 256 件ずつ）。ただし**これが「MultiByteToWideChar の仕様」として Win32 のページにリンクされているわけではない**（MS-UCODEREF の 3.1.5.1.1.3 は、旧来の Windows の動作を表す疑似コードで、MBTWC 自体は名指ししない。これは読み）。
- Learn の旧版（アーカイブの画像）: 1252 の表があるが、**0x80、0x8E、0x9E が現行のデータファイルと食い違う**（旧版は NOT USED）。

### 4.5 非公式の参考（根拠にしない）

| 資料 | 内容 | 現行のデータファイルとの比較 |
|---|---|---|
| WHATWG Encoding Standard、`index-windows-1252.txt`（Date 2024-09-18） | 128 件（0x80〜0xFF）。未定義の 5 バイトは同じ数値の C1 制御（U+0081、U+008D、U+008F、U+0090、U+009D） | 0x80〜0xFF が**完全に一致**（`whatwg-recheck.txt`） |
| Wine 10.0 の `c_1252.nls`、`c_437.nls`（ホストにインストール済み） | 256 件の表（MB → WC） | zip の 1252、437 の MBTABLE と**完全に一致**（`wine-nls-recheck.txt`） |
| CPython の `cp437` コーデック | 256 件 | 437 は完全に一致。`cp1252` は 0x81、0x8D、0x8F、0x90、0x9D を未定義とする（zip は C1 制御に写す）。差はそこだけ |
| IANA の議論（Microsoft の Shawn Steele の 2006 年のメール。`ref/IANA_msg01660.html`） | Windows-1252 の best-fit 表の公開 | 読んだだけ。結論には使わない |

どれも**根拠にしない**（決定 0072 と同じ扱い）。ただし、Microsoft のデータファイル（上の 2）と独立の 3 つの資料（WHATWG、Wine、CPython）が 1252 と 437 の 256 件で一致している（1252 の未定義 5 バイトの扱いだけは WHATWG と Wine が Microsoft に合わせ、CPython は違う）ことは、**転記の誤りではなさそうだ**、という参考になる。

## 5. 表を決めるときの事実（実装の手前まで。この調査では実装していない）

### 5.1 0x80〜0xFF の 128 件（1252）

Microsoft のデータファイル（`1252.txt` の MBTABLE。zip、`bestfit1252.txt`、WHATWG、Wine、旧版の画像の 24 件が一致）:

| 区間 | 値 |
|---|---|
| 0x80〜0x9F | 80=U+20AC 81=U+0081 82=U+201A 83=U+0192 84=U+201E 85=U+2026 86=U+2020 87=U+2021 88=U+02C6 89=U+2030 8A=U+0160 8B=U+2039 8C=U+0152 8D=U+008D 8E=U+017D 8F=U+008F 90=U+0090 91=U+2018 92=U+2019 93=U+201C 94=U+201D 95=U+2022 96=U+2013 97=U+2014 98=U+02DC 99=U+2122 9A=U+0161 9B=U+203A 9C=U+0153 9D=U+009D 9E=U+017E 9F=U+0178 |
| 0xA0〜0xFF | byte == Unicode の値（恒等。96 件。`tablecheck-result.txt`） |

- 0x00〜0x7F も恒等。したがって、**1252 の表の自由度は 0x80〜0x9F の 32 件だけ**。内訳（`tablecheck-result.txt`、`casecheck-detail.txt` の再計算）: 恒等でない 27 件（Microsoft のデータファイル、unicode.org の同じファイル、WHATWG、Wine が一致。**うち 24 件は旧版の画像とも一致し、0x80、0x8E、0x9E の 3 件は旧版が NOT USED**）と、**恒等の 5 件（0x81、0x8D、0x8F、0x90、0x9D）が「未定義」の扱い**: Microsoft のデータファイルは C1 制御（同じ数値）に写し、WCTABLE も逆向きに同じ（byte → Unicode → byte の往復は 256 件とも成立。`tablecheck-result.txt`）。旧版の画像は NOT USED、1998 年の CP1252.TXT は行を持たない。
- 0x80、0x8E、0x9E は、旧版の画像（NOT USED）と現行のデータファイル（U+20AC、U+017D、U+017E）で食い違う。**どちらが「Windows 10 22H2（決定 0014）の実際の動作」か**は、この調査では確認できない（ホストは Linux。実機が無い）。MS-UCODEREF 2.2.1 の「Windows NT … Windows 11」を対象とする記述と、zip の日付（2009 年）、WHATWG が 2024 年版も同じ表であることから、現行の表の方が自然に思えるが、これは**推測**。

### 5.2 437

- Microsoft の `437.txt`: MBTABLE 256 件、WCTABLE 729 件、**GLYPHTABLE 32 件**（0x01〜0x1F、0x7F を絵文字の字形 U+263A… に写す、別の表。MBTABLE の 0x00〜0x7F は恒等）。MS の MBTABLE と unicode.org の `PC/CP437.TXT`（256 件）は**完全に一致**（`tablecheck-result.txt`。前回の記録の「差異 32 件」は GLYPHTABLE を MBTABLE に混ぜた読み違いで、誤り）。
- 0x80〜0x9F は 80=U+00C7、81=U+00FC、82=U+00E9、83=U+00E2、… 9E=U+20A7、9F=U+0192（全 128 件は `tablecheck-result.txt`）。
- **ATTACH は 437 を使わない**（§3.2: GetOEMCP は呼ばれない）。437 の表が要るのは、ゲストが CP_OEMCP（1）で変換したときだけ（FLEngine 側の使用は `import-locale-spec.md` §3 の範囲）。

### 5.3 MBTWC を通した先（256 バイトの連鎖。`stage23` §5.1 段 2〜5）

- **GetStringTypeW(CT_CTYPE1, 256 ワイド)**: Learn のページ（https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-getstringtypew）は C1_UPPER、C1_LOWER など**ビットの名前と値**を挙げるが、文字ごとの値は書かない（原文: 「These types support ANSI C and POSIX (LC_CTYPE) character typing functions.」に続くビットの表だけ）。256 のワイド文字（U+0000〜U+00FF）の分類は、Unicode のデータ（UnicodeData.txt の一般カテゴリなど）か、Microsoft の別のデータが要る。**今回の調査では、その出典を確定していない**。Download Center の zip（111 ファイル）は、タグが CODEPAGE、CPINFO、MBTABLE、WCTABLE、ENDCODEPAGE、DBCSRANGE、DBCSTABLE、GLYPHTABLE だけで、文字の分類（ctype）のデータは**無い**（全ファイルの先頭トークンを数えた）。MS-UCODEREF の保存したページにも C1_ や ctype の語は無い。Download Center の同じ ID の「Sorting Weight Table」（10 ファイル。中身は調べていない）は照合の重みで、分類ではないはず（**未確認**）。決定 0078 §3 の停止のとおり。
- **LCMapStringW(lcid = 0, 0x100 / 0x200, 256 ワイド)**: Learn のページ（…/winnls/nf-winnls-lcmapstringw、ms.date 2025-08-08）は Locale に LOCALE_INVARIANT、LOCALE_SYSTEM_DEFAULT、LOCALE_USER_DEFAULT とカスタムの 3 つ（LOCALE_CUSTOM_DEFAULT、LOCALE_CUSTOM_UI_DEFAULT、LOCALE_CUSTOM_UNSPECIFIED）を挙げる。**0 は挙がっていない**（`LOCALE_NEUTRAL` の語は取得した全ページに無い）。runtime は 0 で停止する（§3.2）。大文字化は MS-UCODEREF 3.1.5.3（「each UTF-16 code point is looked for in an upper casing table [MSDN-UCMT/Win8]」）が、Windows 8 の大文字化表（Download Center の同じ ID の `Windows 8 Upper Case Mapping Table.txt`、973 組、SHA-256 76a66e29…。再取得で一致）を使うと書く。**小文字化の表は Microsoft のダウンロードに無い**（大文字化の表だけ）。小文字は大文字表の逆引きで 60 件（1252 の 256 文字のうち）が引けるが、これは**逆引きという読み**で、MS-UCODEREF は小文字化を書かない。
  - 1252 の 256 文字に対する大文字化: 61 件（ASCII の a-z 26 件を含む）。大文字が 1252 の外になるのは 0x83（U+0192 → U+0191。1252 の WCTABLE の best-fit は 0x83 に戻す）だけ。U+00B5、U+00DF、U+00AA、U+00BA は表に無い（大文字化されない）。0xFF は U+00FF → U+0178 → 0x9F。
- **WideCharToMultiByte(1252, 0, 256 ワイド)**: 戻す表は `1252.txt` の WCTABLE（698 件。256 件の厳密な逆引き + best-fit）。MS-UCODEREF 3.1.5.1.1.2 が WCTABLE を使う疑似コードを持つ（原文: 「OPEN SECTION WideCharMapping where section name is WCTABLE from file with the name of CodePageFileName」。名前は「Bestfit」+ 番号）。runtime の WideCharToMultiByte のフラグの扱い（WC_NO_BEST_FIT_CHARS）は、WCTABLE の best-fit 部分の扱いに直結する（Learn: 「Translate any Unicode characters that do not translate directly to multibyte equivalents to the default character specified by lpDefaultChar.」）。ATTACH の呼び出し（0x17d9d2c。flags = 0）の入力は、**LCMapStringW の出力**（256 ワイド文字。大文字化は 0x200、小文字化は 0x100）。大文字化の出力のうち U+00FF 超の 24 種（U+0152、U+0160、U+0178、U+017D、U+0191、U+02C6、U+02DC、U+2013…U+20AC、U+2122。`casecheck-detail.txt`）は、**U+0191 を除いて 1252 の MBTABLE の値域に入る**（逆引きで直接のバイトになる）。**U+0191（0x83 の U+0192 の大文字）だけは MBTABLE に無く、WCTABLE の best-fit の 1 件**（U+0191 → 0x83。`casecheck-strict.txt`）。flags = 0 では best-fit が使われうる（Learn の WC_NO_BEST_FIT_CHARS は「flags を付けたとき best-fit を使わない」と読める）ので、**ATTACH の結果の 1 バイト（`upper` の表の 0x83 の位置）は、best-fit を採るか（0x83）、採らないか（既定文字 0x3f）で変わる**。ただし、その位置が表に使われるのは、CRT のループが U+0192 を C1_LOWER とみなしたとき（GetStringTypeW の分類次第。出典未確定）。

要するに、**MBTWC の 1 つの停止を外しても、ATTACH はあと 3 つの別の API の表（GetStringTypeW の分類、LCMapStringW の lcid 0 と大文字小文字、WideCharToMultiByte の 1252）が要る**（実測では 3 つの仮の値を入れると WCTMB まで進み、その後は §8 の「ATTACH の完走」の項。`stage23` §5.1）。

## 6. 結論: 決めない選択肢

この調査は何も決めない。事実の上で、次の選択肢が考えられる（互いに排他ではない）。

| # | 選択肢 | 内容 | 長所 | 短所、決めるのに必要なもの |
|---|---|---|---|---|
| A | 決定 0016 の範囲のまま（停止を維持） | 何もしない | 「黙って値を作らない」（0007、0016）を保つ。実装が増えない | ATTACH は 1252 の MBTWC で止まり、DllMain が戻らない。dsp_ippv2 の実行（決定 0073）に進めない |
| B | 1252 の 0x80〜0xFF を実装する。出典は「Microsoft のコードページデータファイル（MS-UCODEREF の [CODEPAGEFILES]）」 | 表は §5.1 の 128 件（実質 0x80〜0x9F の 32 件）。5 件の未定義は C1 制御（Microsoft のデータファイルの値）。MB_ERR_INVALID_CHARS（8）のとき、その 5 バイトを無効にするかは**ページが定めない**（§4.1: 無効な文字は「default character に変換されるもの」。MBTABLE ではその 5 件は C1 制御に写り、既定文字ではない。つまり無効にならない、が文面どおりの読み） | 一次資料（Microsoft が公開し、Learn の仕様が参照）に基づく。独立の 3 つの非公式資料と一致。0016 の方針「仕様のページで規則が確認でき…」の**範囲を、Win32 のページから MS-UCODEREF + データファイルへ広げる**判断だけで済む | 0016 の改訂（または追加の決定）が要る。出典の格（0041 の Wine の先例より強い）と、ライセンス（§4.4 の 2）の判断は PM。0x80、0x8E、0x9E は旧版の画像と食い違う（§5.1）。**MBTWC だけでは ATTACH は進まない**（§5.3） |
| B' | B と同時に、WideCharToMultiByte(1252) も実装する | WCTABLE（698 件）か、MBTABLE の逆引き 256 件 + best-fit の扱いの決定 | 往復が揃う | best-fit（WCTABLE の 442 件。698 − 256）を採るか、逆引きだけにするか。flags（WC_NO_BEST_FIT_CHARS）との関係。ATTACH だけでも best-fit が 1 件（U+0191 → 0x83）効く（§5.3）。決定が増える |
| B'' | 437 も同時に | MBTABLE 256 件（§5.2） | CP_OEMCP の変換に備える | ATTACH には不要。出典は B と同じ（unicode.org の `CP437.TXT` と一致） |
| C | 一次資料が足りないので、別の決定として「出典の拡張」を先に決める | 0016 の「表がページに無い」を、「Win32 のページに無い、MS-UCODEREF のデータファイルにある」に書き換える（または、非公式の資料 3 つの一致を、仮の決定として採る） | B の前提を明示できる | 未決（PM の決定が要る）。決定の撤回条件（実機で値が違う、など）を書く |
| D | ACP の応答を 65001 に変える（決定 0016 の変更） | CRT は 0x17d674c で ASCII の算術だけ（§2.7）。MBTWC を呼ばない | 表が要らない。一次の記述がある（Learn の「Use UTF-8 code pages in Windows apps」: 「CP_ACP equates to CP_UTF8 only if running on Windows Version 1903 (May 2019 Update) or above and the activeCodePage property described above is set to UTF-8.」） | 実機の DLL は activeCodePage の manifest が無ければ ACP = 1252 で動く（決定 0014: Windows 10 22H2、en-US）。ACP が変わると FLEngine の GetACP 依存（`import-locale-spec.md` §3: IsDBCSLeadByteEx など）が変わる。**環境の定義を変える**ので未決（PM の決定が要る）。0017 の「en-US、1252」との整合も壊す |
| E | GetCPInfo を失敗させる、IsValidCodePage を 0 にする | CRT は ASCII の算術へ（§2.7） | 表が要らない | **応答を偽る**（実機は成功する）。0016、0078 の応答（1252 を valid）と矛盾する。0007（fail-soft な既定値を入れない）に反する。非推奨 |
| F | 恒等変換（gdb の仮の値）を runtime に入れる | b → U+00bb | 実装が極小 | **cp1252 の表ではない**（0x80〜0x9F が違う）。「黙って値を作る」。0016 に反する。`stage23` §5.1 の仮の値としては使われたが、決定ではない |
| G | CRT の関数（0x17d674c など）をゲスト側で置換・スキップする | ゲストの関数を runtime が差し替える | MBTWC の表が要らない | 決定 0073（DLL をそのままリフトする方向）の外。ゲストの CRT の挙動を変える（`setSBUpLow` の結果が空になる。後続の `isupper`、`toupper` が変わる）。別の決定が要る |
| H | ATTACH の完走を「未達」として受け入れる | DLL の実行は ATTACH の途中までとし、IPP の関数は呼ばない | 表の問題を先送りできる | 決定 0073 の目的（IPP の関数の実行）に届かない |

**ATTACH を完走させるのに必要な表は MBTWC の 1252 だけではない**（§5.3）。B を採っても、GetStringTypeW（C1 の分類）、LCMapStringW（lcid = 0 の扱い、0x80 以上の大文字小文字）、WideCharToMultiByte（1252）の 3 つに別の決定が要る。MBTWC の出典の判断（B、C）は、残りの 3 つに同じ出典の格を使えるかの試金石になる（WCTABLE、大文字化の表は Microsoft のダウンロードにある。C1 の分類と小文字化の表は無い）。

## 7. 推奨（意見。事実ではない）

1. **まず C（出典の拡張を 1 つの決定として決める）を勧める**。論点は、(a) Microsoft のコードページデータファイル（MS-UCODEREF が [CODEPAGEFILES] として参照するもの）を、決定 0016 の「ページに書かれた規則」と同じ格の一次資料として扱うか、(b) 扱うなら、ライセンス（§4.4 の 2 の引用。解釈は PM）、(c) 食い違い（旧版の画像と現行のデータファイル）の扱い（現行のデータファイルを採り、旧版との差を「未確認の差」として決定に書く、を勧める）。
2. (a) が通れば **B（1252 の MBTABLE 128 件）だけを先に実装**し、0x80、0x8E、0x9E の食い違いと、未定義 5 バイトの MB_ERR_INVALID_CHARS の扱い（ページが定めない。文面どおりなら無効にしない）を「仮」として決定に書く。出典は、データファイルの SHA-256 と、MS-UCODEREF のページ（3.1.5.1.1.3）を書く。
3. **E、F は採らない**（応答を偽る、または黙って値を作る）。**D は環境の定義の変更**なので、PM が望むときだけ（FLEngine への影響の調査が先）。
4. B の後、次の停止（GetStringTypeW）の出典（C1 の文字分類）が無いので、**停止の先を先に見積もる**ことを勧める: B を採っても ATTACH は完走しない（§5.3）。GetStringTypeW に使える Microsoft の資料の有無の調査（この調査では見つかっていない）と、LCMapStringW の lcid = 0 の扱い（Learn に記述が無い。`LocaleNameToLCID` が NULL 名で 0 を返す DLL 内の分岐 0x17d9f1c の結果）が、実際に ATTACH を止める項目になる。順序は、これらを先に PM に見せ、B だけを実装しても進まないことを前提に決めるのがよい。
5. 437 は ATTACH に不要なので、B の最初の範囲に入れず、必要になった時点（ゲストが CP_OEMCP で 0x80 以上を変換したとき）で足す。出典は同じ（MBTABLE が unicode.org の `CP437.TXT` と一致）。

## 8. 限界、未確認

- **DLL は実行していない**。呼び出しの引数と到達は読み（実行の裏付けは、`stage23` §5.1 の gdb の実測: cp = 1252、flags = 1、src = 256 バイト、cb = 256、cch = 0 → 256。呼び出し回数 6）。
- 表の出典は、Microsoft のダウンロード（zip）、unicode.org（Microsoft のファイル）、Learn の旧版の画像（目で読んだ 24 件のみ照合）。実機の Windows（決定 0014）では確認していない。0x80、0x8E、0x9E の食い違いのどちらが実機かは未確認。
- 1252 の 5 バイトの未定義（0x81、0x8D、0x8F、0x90、0x9D）が実機で C1 制御に写るかは、Microsoft のデータファイルの MBTABLE が根拠（Microsoft が公開した表。実機の動作そのものではない）。
- MS-UCODEREF の疑似コード（3.1.5.1.1.3）は、旧来の動作を表すもので、MBTWC の現在の実装そのものとは書かれていない。flags（MB_PRECOMPOSED、MB_COMPOSITE）の効果、MB_ERR_INVALID_CHARS の判定、先頭バイトの扱いは疑似コードに無い。
- GetStringTypeW の C1 分類と、LCMapStringW の小文字化の表は、Microsoft の資料から確認できていない（探索は限定的。Code Page Data Files の zip は全 111 ファイルのタグを数え、分類のデータが無いことは確認した。同じ Download Center ID の Sorting Weight Table 群と、Unicode Consortium 側の資料（UnicodeData.txt など）は調べていない）。
- 呼び出し点 #1〜#4（0x17d3f3c、0x17d57f8）の「ATTACH の直線上ではない」は、直接辺の探索と読み。`mbtwc_attach.py` の根の選び方（初期化表の項目と 4 つのスロット）が前提。間接呼び出しは追っていない（下限）。
- ホストは Linux。runtime の掃引は、現行ソースから `lift/out-mbtwc/mbtwc_sweep.c` を直接コンパイルして実行したもの（子プロセスで abort を受ける。コアダンプは出さない設定）。**既存のテスト（`cargo test`）は実行していない**。
- `ATTACH が完走するか` は、この調査の範囲外。`stage23` §5.1（仮の値 5 段）の後、DllMain は eax = 1 で戻る（既存ログ）が、それは仮の値を入れた結果。

## 9. 作成ファイル、再現の手順

- この文書: `docs/investigations/mbtwc-high-bytes.md`（新規。編集した唯一のリポジトリ内のファイル）。
- 作業物（すべて `lift/out-mbtwc/`。gitignore 済み）:
  - スクリプト（今回の新規）: `mbtwc_slotscan.py`（IAT スロットの全バイト走査）、`mbtwc_disasm.py`（RVA の範囲の逆アセンブル）、`mbtwc_pdata.py`（`.pdata` の範囲）、`mbtwc_riprefs.py`（rip 相対の参照の命令単位の確認）、`mbtwc_roots.py`（任意の根からの直接辺の到達）、`mbtwc_tablecheck.py`、`mbtwc_casecheck.py`（表の照合）、`mbtwc_quotecheck.py`（引用の機械的確認）、`mbtwc_sweep.c`（現行 runtime の掃引）。前回から: `mbtwc_sites.py`、`mbtwc_attach.py`、`mbtwc_exports.py`、`xrefs_to.py`、`probe_runtime.c`、`probe_driver.py` ほか。
  - 今回の出力: `sites-mbtwc-recheck.txt`、`slotscan-recheck.txt`、`riprefs-*.txt`、`pdata-recheck.txt`、`redo-*.txt`（逆アセンブル）、`xrefs-recheck-*.txt`、`attach-recheck.txt`、`export-reach-recheck.txt`、`roots-dllmain.txt`、`roots-detach.txt`（偽の辺あり。§2.6）、`probe-results-recheck.txt`、`probe-rebuilt-recheck.txt`、`sweep-final.txt`、`tablecheck-result.txt`、`casecheck-result.txt`、`casecheck-strict.txt`、`casecheck-detail.txt`、`cp437-glyph-check2.txt`、`whatwg-recheck.txt`、`wine-nls-recheck.txt`、`quotecheck-result.txt`。
  - 取得物（Learn。引用の根拠）: `learn/live-*.md`（2026-10-24 に取得した Markdown 22 ファイル。`mbtwc_quotecheck.py` の入力）、`learn/prev-*` と `learn/cc195054_*.gif/.png`（旧版 1252 の画像と、目で読むための切り出し）。前回取得の `learn/*.html`、`learn/*.txt` は読み取りで再利用しただけ（引用の根拠は上の `live-*.md`）。
  - 取得物（Microsoft のデータ。表の根拠）: `ref/Windows_Supported_Code_Page_Data_Files.zip`（+ `.sha256`。Download Center から再取得して一致）、`ref/Windows_8_Upper_Case_Mapping_Table.txt`（同じ ID。再取得して一致）。`ref/WindowsBestFit_bestfit1252.txt`、`ref/WINDOWS_CP1252.TXT`、`ref/PC_CP437.TXT`、`ref/WindowsBestFit_readme.txt` は unicode.org が公開する Microsoft のファイル（再取得して一致を確認。`bestfit1252.txt` は zip の `1252.txt` と同一）。**`ref/WindowsBestFit_bestfit437.txt` は 404 のエラーページ（HTML）で、データではない**（unicode.org に `bestfit437.txt` は無い。437 は zip の `437.txt` と `PC_CP437.TXT` を使う）。
  - 非公式（根拠にしない）: `ref/whatwg_index-windows-1252.txt`、`ref/wine_nls_compare.py` と `ref/wine-*`（ホストの Wine 10.0 の `c_1252.nls`、`c_437.nls`）、`ref/IANA_msg01660.html`、`ref/dl-10921.html`（Download Center のページの HTML。ファイル一覧と URL の確認用）。
  - 前回の途中成果物のうち、読み違いを含むもの: `ref/ms-codepagefiles-compare.txt` の 437 の「MB table vs unicode.org PC/CP437.TXT: differing bytes」（32 件）。GLYPHTABLE を MBTABLE に混ぜた誤りで、**採らない**（§5.2。`tablecheck-result.txt` が訂正）。`ref/cp437-compare.txt` と `ref/cp1252-compare.txt` は unicode.org のファイルと CPython の比較で、読みは変わらない。
  - `tmp/`: 再ビルドした `probe_runtime_rebuilt`、`mbtwc_sweep`、再取得物（`*.fetched.*`）、W6 以前の `fle_locale_preW6.c`。
- 再現: `cd lift/out-mbtwc && python3 mbtwc_slotscan.py ../out-ext/dsp_ippv2_x64.dll`、`python3 mbtwc_disasm.py ../out-ext/dsp_ippv2_x64.dll 0x17d674c 0x17d6934`、`clang -O1 -w -I../runtime -o tmp/mbtwc_sweep mbtwc_sweep.c ../runtime/fle_locale.c ../runtime/fle_imports.c && ./tmp/mbtwc_sweep`、`cd ref && python3 ../mbtwc_tablecheck.py`、`cd learn && python3 ../mbtwc_quotecheck.py`。
