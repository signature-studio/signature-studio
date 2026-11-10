# CreateFruityInstance を偽のホストオブジェクトで呼ぶ native ハーネス（第 1 段階: 設計と実装。コンパイルも実行もしていない）

> **更新（第 2 段階の測定後。f5e9552）**: 以下の本文は**第 1 段階を書いた時点の記録**で、書き換えていない（§6 の `+0x34〜0x37` の行だけ、第 2 段階で訂正済み）。第 2 段階は `create-fruity-instance-measurement.md` がビルド・実行した。その実測（節番号は同文書のもの）:
>
> - **自己テスト**（§3）: コミットされたままでは `ok` 46 行 + FAIL 1 件（`+0x34` を「パディング（0）」と期待した検査の誤り。`fruity_host.h` の実装は誤っていない）。1 件の検査を直して **47/47、0 failure**。直しは f5e9552 で `lift/harness/seeds/fruity_selftest.c` に入っている。ASan／UBSan は環境の都合で使えなかった。
> - **FRUITY 無効の基準**（§2）: 再現した（TAG `t2331d00e8ebb`、`init_count=942 of 942`、functions 5,306、entries 1,007,942、終了コード 0）。ただし native の SHA-256 は、本文の `9c76bc6a…a23785` ではなく、d7336ad 以降は **`9c3244281cd3696c065ebb41ea674b67605c5b8433c8310d0f13719a3ba32f37`**（`9c76bc6a…` は d7336ad より前の木の値。原因は d7336ad の `a4.c` の注記の文言）。0020f8e（このハーネスのコミット）の FRUITY 無効のビルドは、228c78a のものとビット単位で同一。
> - **FRUITY 有効**（§4）: `CreateFruityInstance(0xC0, 0xEDE95A, host, &instance)` は**戻らなかった**。`fle: GetFileAttributesW: empty path is not defined`（`lift/runtime/fle_vfs.c:303`）で、呼び出しの 4,693 関数入口目に SIGABRT（ハーネスの終了コード 134）。`instance` は 0 のまま。独立の 3 回のビルドで同一。
> - **設計の予想は外れた**（§7、§9）: 本文の「最初の停止は WinVerifyTrust」（§0 の 6(c)、§10 の H1、§12 の J13）。実際の停止は WinVerifyTrust の手前で、署名検査（WinVerifyTrust、J5）、署名後の未応答 API（J6）、メッセージループ（J7）には到達していない。
>
> 本文の「何も実行していない」「第 2 段階で測る」「予想される」は、この更新の時点では上のとおり実行済みまたは外れた記述。

作業日: 2026-10-28。基準: HEAD **86439bc**（作業中に別の作業者の docs だけのコミット 228c78a が入り、HEAD は 228c78a に進んだ。差分は両方の clean な `git archive` に当たることを確認した。§9）。ワークツリーは `git archive 86439bc` の展開（`/tmp/opencode/fruity/src/`、RAM 上）だけを変えた。ワークツリーと git は読み取りだけ（commit なし、`git status` は作業の前後で同じ未追跡 2 件）。

**この文書は何も決めない**。決める必要がある点は §12 に列挙した。凡例: **実測** = 今回のコマンドの出力を見た事実／**読み** = 命令やコードを読んだ上での解釈（実行していない）／**未確認** = 調べていない、または調べられない／**引用** = 既存文書の記述（再測定していない）／**出典なし** = 出典がなく、この作業の選択。

**第 1 段階の範囲**: 設計、実装（コード）、構文確認（`clang -fsyntax-only`、`bash -n`、python の `ast.parse`）、python だけで動く合成データの検査、`patch --dry-run`。**cargo、clang、gcc、emcc のビルド・リンク・実行はしていない**。したがって「この C コードが動く」ことは**何も測っていない**。動作の記述はすべて読み（予想）である。**【第 1 段階の時点の記述。第 2 段階でビルド・実行した。冒頭の更新を参照】**

## 0. 要約

1. **何を作ったか**: DllMain が戻った後に、FL64.exe と同じ形の偽のホストオブジェクト（3 スロットの vtable、7 データフィールド）をゲストのメモリに置き、`CreateFruityInstance(0xC0, 0xEDE95A, host, &instance)` を `fle_call_guest` で呼ぶ native ハーネス。環境変数 `FLE_CALL_FRUITY=1` で有効になる。戻り値と `instance` を `fruity: rax=… instance=…` で出す。停止したときは停止メッセージ全文、シャドウコールスタック、ゲストのレジスタ、直前までの import thunk 呼び出しの列（引数つき）、ホストオブジェクトへの呼び出しの記録、C スタックを出す（§7）。
2. **`CreateFruityInstance` の VA は 0x1375990**（PE の export テーブルの序数 3。**実測**: pefile で `binaries/win64/FLEngine_x64.dll`（SHA-256 `b2c64da4…9496d`）を読んだ。ハーネスも実行時にマップした画像の export テーブルから同じ値を引く）。
3. **FLEngine の生成 C は変わらない**。差分は `lift/harness/` の 10 ファイルだけで、`lift/crates/`、`lift/runtime/` には触れていない（**実測**: 差分のファイル一覧、§9）。応答表の行は足していない。偽ホストの 3 つの関数をゲストが呼べるようにするため、**結合済みディスパッチ表（common.c）にだけ**3 行を足す（`FLE_CALL_FRUITY=1` のビルドの common.c だけ。§5）。
4. **既定のビルドは変わらない（読み。バイナリの SHA は測っていない）**: 新しいコードは `#ifdef FLE_FRUITY` と、`FRUITY=1` のときだけ動く分岐の中にある。`main_two.c` の `#ifdef FLE_FRUITY` ブロック 7 個を取り除くと HEAD の `main_two.c` と**行単位で一致する**（**実測**、python）。`merge_tables_n.py` は `--extra-rows` を付けなければ、合成した表で common.c も出力行も**バイト単位で一致する**（**実測**）。基準の native の SHA-256 `9c76bc6a…a23785` が再現するかは第 2 段階で測る（§10）。**【第 2 段階の結果: 既定のビルドのビットは変わっていないが、基準の値は `9c3244281cd3696c065ebb41ea674b67605c5b8433c8310d0f13719a3ba32f37` に変わっている（d7336ad の `a4.c` による）。冒頭の更新を参照】**
5. **署名検査（WinVerifyTrust）は何も答えない**。応答表に行がないので、ゲストが `wintrust.dll!WinVerifyTrust` の thunk（VA 0x4256d0）に入ると `fle: unsupported instruction at 0x4256d0` で止まる（読み）。停止レポートがその引数（GUID、`WINTRUST_DATA`、検査対象のパス）を出す。「検証成功」を返す実装は足していない（§8）。
6. **見つかったこと（静的な読み。§3）**: (a) FL64.exe のスロット 2 の比較は `index` の **64 ビット符号なし**比較（`cmp r8, 1; ja`）。以前の文書の「`index <= 1`」はこの意味で、負の index は 0。(b) FL64.exe は `instance` 変数を呼ぶ前に 0 にする。(c) `CreateFruityInstance` の最初の `WinVerifyTrust` 呼び出し（0x1375bce）より前に、直接の `call` で届く import のうち応答表に行がないのは `advapi32!RegDeleteKeyExW` だけ（下限）。したがって最初の停止は WinVerifyTrust になる**可能性が高いが、確定ではない**。(d) 最初の WinVerifyTrust より後に、直接の `call` で届く未応答の import が **113 個**ある（`PeekMessageW`、`TranslateMessage`、`DispatchMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects`、`ShowWindow`、`DirectSoundCreate`、`CreateThread` など。下限）。**署名検査に答えるだけでは先へ進めない**。
7. **PM の判断が要る点は §12**（スタブの受け方、署名検査への答え、ブロッキングの扱い、既定バイナリ不変の解釈など）。

## 1. 入力と手順

- 読んだもの: `docs/investigations/host-engine-boundary.md`（全体）、`lift/harness/a4-multi.sh`、`two/main_two.c`、`two/gen_two.sh`、`two/build_two_native.sh`、`two/merge_tables_n.py`、`two/two-native-run.sh`、`main.c`、`a4.c`／`a4.h`／`boot.h`、`image.c`、`trace.c`、`wasm/trace_native.c`、`wasm/trace.c`、`census/census.c`／`census.h`、`seeds/vfs_seed_selftest.c`／`vfs_seed.h`、`runtime/fle_dispatch.h`、`fle_callback.h`／`.c`（`fle_call_guest`）、`fle_runtime.h`、`fle_modules.h`、`fle_env.h`、`fle_host.h`、`fle_sys.h`、`crates/fe-lift/src/imports_table.rs`（応答表）、`dispatch.rs`。決定は、全文: 0015、0045、0077。一部だけ: 0101（冒頭と表）、0100（`fle_dispatch.h` のコメント経由）、0097（`census.c`／`census.h` のコメント経由）、0110（`vfs_seed.h`、自己テストの流儀、決定の検証表）、0113（`c1-results/MERGE-NOTES.md` 経由）。そのほか `lift/out-git/c1-results/MERGE-NOTES.md`、`lift/out-git/v1-results/`（`verification.md`、`tools/run6.sh`、`tools/wrap/`）。
- FL64.exe の追加の読み: `tools/extract_nsis2.py` でインストーラ（`flstudio_win64_26.1.7.5653.exe`）のエントリ 948 を**メモリ上にだけ**展開し、SHA-256 `4c839f24da13885fef7190d81c792fe2a2e0248dcfce9cb5f400a48ae0520c65` を確認（**実測**。既存文書の値と一致）、capstone で必要な範囲を逆アセンブルした（`tools/fl64_read.py`。**FL64.exe はディスクに書いていない**。出力のテキストは RAM 上の作業ディレクトリにだけ置き、この成果物には抜粋以外を入れていない）。
- FLEngine の読み: `binaries/win64/FLEngine_x64.dll` を pefile と capstone で読んだ（`tools/cfi_calls.py`、`tools/reach.py`）。

## 2. 環境の制約（遵守したこと）

`/` への書き込みは成果物のテキストだけ（`lift/out-git/fruity-results/`。§14 の表のとおり約 0.3 MB）。作業は RAM 上 `/tmp/opencode/fruity/`（最大 34 MB）。ビルド・リンク・実行は 0 回。許可された構文確認だけ（§9）。ホストの実ファイルをゲストに見せていない（偽ホストの文字列はハーネスの定数と `FLE_VIRTUAL_DLL_PATH` から作る）。

## 3. FL64.exe の追加の読み（偽ホストの根拠。**読み**）

アドレスは FL64.exe の VA（ImageBase 0x140000000）。抜粋は 7 行（命令にして約 20 個）。

```
140007520: mov rax,[rcx] ; movsxd r9,r9d ; movsxd r8,r8d ; movsxd rdx,edx ; jmp [rax+0x10]     スロット 0
140007530: ret 0                                                                              スロット 1
140007540: sub rdx,2 ; je 140007555 ; cmp rdx,1 ; jne 140007555 ; cmp r8,rdx ; ja 140007555    スロット 2
14000754c..: mov rax,rdx ; ret    /    140007555: xor eax,eax ; ret
140069d00: 20 75 00 40 01 00 00 00  30 75 00 40 01 00 00 00  40 75 00 40 01 00 00 00          vtable の 3 スロット
14000764d: mov [rbp-0x21],rsi   (rsi = 0: 1400075fa xor esi,esi)                             instance 変数 = 0
1400076a3: lea rdx,[rbp-0x21] ; ... 1400076a7: call 1400062a0                                  Run(host, &instance, ...)
```

| 項目 | 読み | 以前の文書との関係 |
|---|---|---|
| vtable（0x140069d00） | 3 スロット = 0x140007520、0x140007530、0x140007540（バイト列から） | 一致 |
| スロット 0 | `edx`、`r8d`、`r9d` を 64 ビットに符号拡張し、**オブジェクト自身の vtable の +0x10（スロット 2）へ jmp**。スタックには触れない | 一致 |
| スロット 1 | `ret 0`。何も変えない（rax も） | 一致 |
| スロット 2 | `id - 2 == 0` なら 0。`id - 2 != 1`（= `id != 3`）なら 0。`cmp r8, 1; ja` で **`index`（r8）が 64 ビット符号なしで 1 より大きければ 0**。それ以外は 1。`value`（r9）は読まない | **細部が増えた**: 「`index <= 1`」は符号なし比較。負の `index` は 0 になる（スロット 0 経由なら `r8d = -1` が 0xFFFF…FFFF に拡張される） |
| 製品名（0x140069710） | `"FL Studio\0"` | 一致 |
| 空文字（0x140069820） | 1 バイトの NUL。+0x20 と +0x28 は**同じ**これを指す | 一致 |
| ベースディレクトリ | `0x140001330` は `std::wstring::find_last_of(L"\\/")` の後に `substr(0, pos + 1)`（区切りを**含む**）。区切りがなければ空文字。これを UTF-8 にして `strcpy_s(buf, 0x8000, …)` で 32 KB のグローバルにコピー（0x140006f00〜0x140006f0c）。UTF-16→UTF-8 の変換関数（0x140002940）の中身は読んでいない（**未確認**。このパスは ASCII） | 「末尾が区切り」が確定 |
| `instance` | 呼ぶ前に 0（0x14000764d） | **新規**。ハーネスも FL64.exe と同じく 0 から始める |
| 戻り値の振り分け | `test ebx,0xfffffffd; je` で 0 か 2 なら `instance->vtable[0](instance)`。−2 なら `MessageBoxA`。他は何もしない | 一致（記録だけ。§8） |

エンジン側の `CreateFruityInstance` の準備（FLEngine の逆アセンブル。**読み**）:

- 0x1375ab5〜0x1375bce: `GetModuleFileNameW(HInstance /* [0x15ec298] */, buf, 0x104)` の結果のパスを `WINTRUST_FILE_INFO`（`cbStruct = 0x20`、`pcwszFilePath`、`hFile = 0`、`pgKnownSubject = 0`）に入れ、`WINTRUST_DATA`（`cbStruct = 0x58`、`dwUIChoice = 2`、`fdwRevocationChecks = 0`、`dwUnionChoice = 1`、`pFile`、`dwStateAction = 1`）を作って、`WinVerifyTrust(0, [0x15e25c8 → 0x13a96bc の GUID], &data)` を呼ぶ。GUID のバイト列は `{00AAC56B-CD44-11D0-8CC2-00C04FC295EE}`（= `WINTRUST_ACTION_GENERIC_VERIFY_V2`。**実測**: 0x13a96bc を読んだ）。`test eax,eax; sete al` なので、戻り 0 を成功として扱う。続けて同じ引数で `dwStateAction = 2` の呼び出し（0x1375bf8）。この構造体のオフセット（`dwUIChoice` は +0x18、`pFile` は +0x28、`dwStateAction` は +0x30）は、ハーネスの停止レポートが読むオフセットと一致する。
- `WinVerifyTrust` の thunk は VA 0x4256d0、バイト列 `ff 25 3a c1 54 01` = `jmp [0x1971810]`、IAT スロット 0x1971810 = `wintrust.dll!WinVerifyTrust`（**実測**）。

## 4. 全体の流れ

```
main_two.c (-DFLE_FRUITY のときだけ):
  画像を読むたびに fruity_add_image(base,size)
  vfs seed の後:     fruity_precheck()           環境変数と、3 つのスタブ行が表にあることを調べる（不正なら exit 2。ゲストは動かない）
  DllMain を呼ぶ     (従来どおり)
  trace_report("returned from entry"); printf("rax=…")      ← 従来の出力。a4: の行はここで 1 回だけ出る
  if (fruity_requested()) return fruity_run(entry_sp);     ← FLE_CALL_FRUITY=1 のときだけ
```

`fruity_run`: ① マップした FLEngine の PE export テーブルから `CreateFruityInstance` の VA を引く ② 偽ホストオブジェクトを作る ③ 上限と停止レポートを仕掛ける ④ `fle_call_guest(va, 0xC0, 0xEDE95A, host, &instance)` ⑤ 戻ったら `fruity: rax=… instance=…` と記録を出す。

**選択（出力の順序）**: DllMain の報告（`== stop: returned from entry` と `a4:` の行）を**先に、従来どおり**出し、その後に `fruity:` の行を足す。理由: (a) `a4:` の行を変えない（依頼）、(b) `trace_report` は 1 度しか出さない作りで（`reporting` フラグ。2 度目は何も出さずに exit 5）、停止の報告に使えない。そこで fruity の停止は自前の報告（`fruity:` で始まる行だけ。`a4:`、`runtime message:`、`== stop:` などの行頭を使わない）を出す。`a4-survey.sh` の要約は先頭の報告だけを読むので変わらない。

## 5. 偽ホストの関数をゲストに見せる方法（選択肢と理由）

ゲストは `call [vtable+8*n]` を `fle_dispatch(<スロットの値>)` として実行する（`dispatch.rs`: `{ t = target; R[4] -= 8; fle_st(R[4], 8, next); fle_dispatch(t); R[4] += 8; }`）。`fle_dispatch` は VA を `fle_dispatch_table` の二分探索で引き、なければ `fle_dispatch_miss`（abort）。ホストの関数の VA はイメージの外にあるので、**表に行がなければ呼べない**。リフトしたコードはメモリ上の機械語を実行しない（決定 0105 の記述と、`fle_dispatch` の作り）ので、「ゲストメモリ上にスタブの命令を置く」ことはできない。擬似 VA と表の行が、既存の唯一の道である。

| # | 方法 | FLEngine の生成 C | runtime | 判断 |
|---|---|---|---|---|
| A | 応答表（`imports_table.rs`）に行を足す。決定 0015 §4 の仕組みそのもの | 変わる（応答表、ディスパッチ表、個数。SHA が変わる） | 変わらない | **不採用**: 方針（生成 C を変えない）に反する。全モジュールの表に入る |
| B | `fle_dispatch.h` の「見つからない VA」で呼ぶ弱い NULL フックを足す（census の `fle_dispatch_hook` の「見つかった場合」版） | 変わらない | **変わる**（全生成 C が再コンパイルされ、基準のバイナリが変わる） | 不採用: runtime を変える |
| C | 既存の `fle_dispatch_hook`（census）で、**ゲストの実在関数の VA**（呼ばれないと思われるもの）を横取りする | 変わらない | 変わらない | 不採用: 「呼ばれない」を証明できない。実在の関数の名前を偽る |
| **D** | **結合済み表（common.c）にハーネスの行を足す**: `merge_tables_n.py --extra-rows lift/harness/fruity-rows.txt`。VA は `0x7FFB00000010/20/30`（擬似 VA の領域。**出典なしの選択**）、関数は `fruity_host_slot0/1/2` | 変わらない（`flengine.c` は同一） | 変わらない | **採用**。応答表の行と同じ「擬似 VA を表の行にする」道で、`realmod_table_gen.c`（ハーネスが TAG ごとに作る）と同じ前例がある。生成器を触らない |
| E | `abort` を `--wrap` で横取りして miss から戻る | — | — | 不可能（`abort` は noreturn。呼び元の C フレームに戻れない） |

D の制約: 結合済み表は実モジュールが 1 つ以上のビルドにしかない（FLEngine 単独の構成は `flengine.c` の表をそのまま使うので行を足せない）。`a4-multi.sh` は `FLE_CALL_FRUITY=1` かつモジュール 0 個を名前付きで拒否する。

領域の確認（**実測**: `grep` で runtime の `UINT64_C(0x7F…)` を調べた）: ヒープ 0x7FF700000000、モジュールハンドル 0x7FF800000000、応答表 0x7FF900000000 + 0x10×(k+1)、ハンドル群 0x7FFA00000000〜0x7FFA8….。0x7FFB00000000 台はどの runtime ヘッダも使っていない。`VirtualAlloc` は上位（0x7FFFFFFEFFFF）から下へ取るので、4 GiB 以上取らなければ届かない（読み）。

## 6. 偽ホストオブジェクトの仕様

ゲストのメモリ = ホストのメモリ（native は恒等写像。`HeapAlloc` のブロックも `malloc`）。1 回の `calloc`（0x8080 バイト）に全部を置く。配置はハーネスの選択（出典なし）で、FL64.exe では vtable と文字列が `.rdata`、基底ディレクトリのバッファが `.data`、オブジェクトが `operator new` のヒープ。

| オフセット | 内容 | 値 | 根拠 |
|---|---|---|---|
| +0x00 | vtable へのポインタ | ブロック内 +0x40 | FL64.exe は `.rdata` 0x140069d00 |
| +0x08 | u32 HostVersion | **1** | 読み（0x14000761c `mov qword [rdi+8],1`）。エンジン側が `!= 1` で拒否する点は引用（host-engine-boundary.md §6.1） |
| +0x0c | u32 Flags | **0** | 同上（上の `mov` の上位 32 ビット）。ビット 0 が 0 = ホスト exe の署名検査もエンジンが行う |
| +0x10 | u64 | 0 | 読み（0x140007649） |
| +0x18 | `char*` | `"FL Studio"` | 読み（0x140069710） |
| +0x20 | `char*` | `""` | 読み。+0x28 と同じ文字列 |
| +0x28 | `char*` | `""` | 読み |
| +0x30 | `char*` | `"C:\Program Files\Image-Line\FL Studio 21\"`（32 KB のバッファ） | 形は読み（§3）。**値は環境の主張**: エンジンのパスを `FLE_VIRTUAL_DLL_PATH`（`fle_env.h`、決定 0018 の PM の値）とし、そのディレクトリ部（区切りを含む）。ゲストが `GetModuleFileNameW(HInstance)` で得る値と同じ文字列から作るので、FL64.exe の振る舞い（エンジンのパスから作る）と整合する |
| +0x34〜0x37 | （パディングではない）+0x30 のポインタの上位 32 ビット | ポインタの上位半分 | 第 2 段階で訂正: +0x30 は 8 バイトのポインタで、オブジェクトの大きさ 0x38 ちょうどで終わる。パディングは無い（第 1 段階の記述と自己テストの 1 件がゼロを期待していた誤り） |
| vtable[0] | `Dispatch(int,int,int)` | 擬似 VA `0x7FFB00000010` → `fruity_host_slot0` | §3 のスロット 0 |
| vtable[1] | 何もしない | `0x7FFB00000020` → `fruity_host_slot1` | §3 |
| vtable[2] | `Dispatch(intptr,intptr,intptr)` | `0x7FFB00000030` → `fruity_host_slot2` | §3 |

スタブ（`fruity_host.h`）: スロット 0 は `R[2]`、`R[8]`、`R[9]` を 32→64 ビットに符号拡張し、**`R[1]`（this）が指すオブジェクトの vtable の +0x10 から VA を読んで `fle_dispatch`**（FL64.exe の `jmp [rax+0x10]` と同じ）。スロット 1 は何も変えない。スロット 2 は `R[0] = (id == 3 && index <= 1) ? 1 : 0`（`index` は `uint64_t`）。スロット 1 と 2 の呼び出し（先頭 128 件）と回数を記録し、停止レポートと戻りの後に出す。

**Dispatch の `id` の意味（0、1、4 など）は不明のまま**（U2）。FL64.exe の振る舞い（`id == 3 && index <= 1` だけ 1）をそのまま再現する。ふり（成功の偽装）はしていない。

## 7. 打ち切りと停止レポート（選択肢と理由）

| 仕組み | 既存か | 判断 |
|---|---|---|
| 関数入口の回数の上限 | あり（`trace_set_call_limit` と `trace_limit_hook`。census が使う） | **採用**。`FLE_FRUITY_MAX_CALLS`（既定 20,000,000。呼び出し開始からの追加分） |
| 実時間の上限 | あり（`trace_init` の `alarm(120)`。全体で 120 秒） | **採用**。開始時に `alarm` を張り直す。`FLE_FRUITY_SECONDS`（既定 60 秒）。SIGALRM は自前のハンドラで受ける（trace のハンドラは理由に「120s」と書いてしまうため） |
| 最初のウィンドウ作成 API での名前付き停止 | **ない**。`CreateWindowExW` は応答表にある（仮想ウィンドウ、決定 0071）ので、止まるのは新しい動作 | 任意（既定は無効）: `FLE_FRUITY_STOP_AT=CreateWindowExW,…`。lifted な `jmp [IAT]` thunk に入った時点（答えが走る前）で止まる。任意の API 名に使える。**依頼の「最小」を超える追加**（§12 J10） |
| census の「unit を放棄して続ける」 | あり | 不採用: 放棄しても続ける先がない。放棄した結果は A4 の進捗でもない（決定 0097） |

ブロッキング（メッセージループ）について: 現状の応答表には `PeekMessageW`、`GetMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects` の行がない（**実測**: `imports_table.rs`）。ループに入る前に別の未応答 API で止まる可能性が高く（読み）、ループの実時間はこの段階の問題にならない。行が足された後の段階で上の 2 つの上限が効く。

停止レポート（`fruity:` で始まる行。実装は `fruity.c` の `report`）:

1. `== stop: <理由>`、`runtime message:`（**全文**。`trace_native.c` の 511 文字の切り詰めではなく 4096 文字の `trace_full_message`）、`abort kind:`、`dispatch target:`（ミスのとき）。
2. 停止が import thunk の中なら、その thunk の VA と import 名（画像のバイト列 `ff 25 disp32` から IAT スロットを引き、`image_import_name` で名前にする）。
3. 経過時間、呼び出し開始からの関数入口数、新しく実行された関数数。
4. シャドウコールスタック（最大 40 フレーム。DllMain の longjmp が残した 12 個の古いフレームは数えるが表示しない）。
5. ゲストのレジスタ 16 個、`[rsp]` から 8 語、`rcx rdx r8 r9` の指す先（32 バイトの 16 進と、UTF-16LE／ANSI の文字列らしさ）。`process_vm_readv` で読むので、不正なポインタでも落ちない（読めなければ「読めない」）。
6. `WinVerifyTrust` の thunk で止まったときは、`GUID`、`WINTRUST_DATA` の 0x58 バイトと主なフィールド、`WINTRUST_FILE_INFO` と**検査対象のパス**（§3 のオフセット）。
7. **直前までの import thunk 呼び出し 48 件**（新しい順）: 名前、ゲストの戻りアドレス、`rcx rdx r8 r9`。`-finstrument-functions` の入口フックの前に `-Wl,--wrap=__cyg_profile_func_enter` で自前の関数を挟んで記録する（`trace_native.c` は変えない）。見えるのは lifted な `jmp [IAT]` thunk への入口だけ（擬似 VA 経由の呼び出しや `call [IAT]` は見えない。FLEngine の `call [IAT]` は 0 件: 引用、決定 0077 §3）。
8. 直前の lifted 関数入口 48 件（戻りアドレスつき）。
9. 偽ホストへの呼び出しの記録。
10. C スタック: glibc なら `backtrace()` + `backtrace_symbols_fd`（PIE のオフセット。`addr2line` に渡す）。**gdb で取る場合**: SIGABRT／SIGSEGV は gdb が先に止まるのでそのまま `bt`。自前の上限での停止は `FLE_FRUITY_TRAP=1` で報告の後に `raise(SIGTRAP)` する（gdb 配下なら止まる）。

終了コードは `trace_native.c` と同じ（134、139、135、132、136、4＝実時間、3＝入口数）、名前付き停止は 6。

## 8. 署名検査の扱い（依頼どおり）

- **何も答えない**。`WinVerifyTrust` に応答表の行を足していない。ゲストが `wintrust.dll!WinVerifyTrust` の thunk（VA 0x4256d0）に入ると `fle_unsupported(0x4256d0)` → `fle: unsupported instruction at 0x4256d0` で SIGABRT（決定 0077 §2 の挙動。読み）。停止レポートの 2 と 6 が「どこで・何の引数で」を出す。
- **呼ばれる場所（読み）**: 自己検査が 0x1375bce（`dwStateAction = 1`）と 0x1375bf8（`= 2`）。`Flags` のビット 0 が 0（FL64.exe の値）なので、ホスト exe の検査が 0x1376023 と 0x137604d。これらの前後に `CryptQueryObject`、`CryptMsgGetParam`、`CertFindCertificateInStore`、`CertGetNameStringW`、`CertFreeCertificateContext`（いずれも応答表に行なし）が続く。
- **`Flags` を変えて検査を避けることはしていない**（FL64.exe は 0。ビット 0 でも自己検査は残る）。
- 「検証成功」を返すかどうかは**環境の主張**で、PM の判断（§12 J5）。

## 9. 変更したファイル（`fruity.diff`。HEAD の clean な archive に `patch -p1` で当たる）

新規 5、変更 5（計 10 ファイル）。`lift/crates/`、`lift/runtime/`、`lift/harness/` の既存の C（`a4.c`、`image.c`、`trace*.c`）は触っていない。

| ファイル | 行 | 内容 |
|---|---:|---|
| `lift/harness/fruity_host.h`（新規） | 210 | 偽ホストの配置、3 つのスタブ、ログ。単一ヘッダ（`FRUITY_HOST_IMPL` を定義した 1 つの .c でスタブを定義） |
| `lift/harness/fruity.c`（新規） | 767 | export の引き、呼び出し、上限、停止レポート、環境変数、入口フック |
| `lift/harness/fruity.h`（新規） | 31 | `main_two.c` から呼ぶ 4 関数と環境変数の説明 |
| `lift/harness/fruity-rows.txt`（新規） | 8 | 結合表に足す 3 行 |
| `lift/harness/seeds/fruity_selftest.c`（新規） | 206 | `fruity_host.h` の自己テスト（47 件の検査。第 2 段階で実行） |
| `lift/harness/two/main_two.c` | +21 | `#ifdef FLE_FRUITY` のブロック 7 個 |
| `lift/harness/two/build_two_native.sh` | +18 | `FRUITY=1` のときだけ: `-DFLE_FRUITY=1` の main_two.o、fruity.o、`--wrap` つきのリンク |
| `lift/harness/two/gen_two.sh` | +11 −2 | `FRUITY_ROWS`（既定は空 = 従来どおり） |
| `lift/harness/two/merge_tables_n.py` | +35 −3 | `--extra-rows`（付けなければ出力は不変） |
| `lift/harness/a4-multi.sh` | +31 −2 | `FLE_CALL_FRUITY=1` の処理、`---- fruity` の出力、stamp に `fruity-rows.txt`、TAG の鍵 |

**a4-multi.sh の stamp が変わる**: stamp は `harness/*.c`、`harness/*.h` と `harness/fruity-rows.txt` の SHA-256 を含む。新規ファイルが入るので、**この差分を当てる前に作った OUT は a4-multi.sh が拒否する**（新しい OUT を作る）。MERGE-NOTES に同じことを書いた。

**ビルドの分け方（選択。J2）**: `FLE_CALL_FRUITY=1` のとき `a4-multi.sh` は (1) gen に `FRUITY_ROWS=…` を渡し、(2) build に `FRUITY=1` を渡し、(3) TAG の鍵に `fruity-rows` の SHA を足す（= 既定の TAG と別になる）。未設定のとき TAG の鍵は従来と同じなので、6 モジュール構成の TAG は `t2331d00e8ebb` のまま（読み。TAG の計算式を変えていないことから）。fruity 版は**別の TAG のモジュールのオブジェクトを再度ビルドする**（OUT を共有すると RAM が増える。§10 の手順は OUT を分ける）。

## 10. 第 2 段階の手順書（別の作業者がビルド・測定する）

前提: ビルドと測定の規則は `lift/out-git/c1-results/MERGE-NOTES.md`、`docs/investigations/a4-six-module-completion-verification.md`、`lift/out-git/v1-results/`（RAM の管理）に従う。**`/tmp/opencode` の使用量は V1 の測定で最大 2,081 MB、C1 で 2,693 MB まで上がった**。OUT は 1 つずつ作り、検査が済んだら消す。`/tmp/opencode` 全体が 2.3 GB を超えないよう `du -sm /tmp/opencode` を見る。

```
R=/home/ubuntu/work/signature-studio-develop ; W=/tmp/opencode/<自分の名前>            # RAM 上。OUT は $W/o1, $W/o2 のように o で始める
mkdir -p $W/src && git -C $R archive 86439bc | tar -x -C $W/src                          # 228c78a でも当たる（確認済み）
cd $W/src && patch -p1 < $R/lift/out-git/fruity-results/fruity.diff                       # 10 ファイル。.rej が出ないこと
export FRUITY_W=$W S=$W/src/lift                                                           # tools/run6-fruity.sh が読む
```

**手順 A: 自己テスト（数秒。ゲストなし）**

```
cd $S && clang -std=gnu11 -O0 -g -Wall -Wextra -fPIE -I runtime -I harness harness/seeds/fruity_selftest.c -o $W/fruity_selftest && $W/fruity_selftest harness/fruity-rows.txt
```
期待: `ok` が 47 行（rows 行を含む）、最後に `0 failure(s)`、終了コード 0。FAIL が出たら、その行の名前が壊れた仕様を指す。

**手順 B: 既定のビルドが変わっていないことの確認（安いものから）**

1. `main_two.c` のオブジェクト比較（秒）: `mkdir -p $W/base0 && git -C $R archive 86439bc lift/harness/two/main_two.c lift/harness/two/merge_tables_n.py | tar -x -C $W/base0` で取った HEAD の `main_two.c`（3 の比較は同じ展開の `merge_tables_n.py` を使う）と、パッチ後の `$S/harness/two/main_two.c` を、`build_two_native.sh` と同じ `clang -std=gnu11 -O1 -Wall -Wextra -fPIE -I $S/runtime -I $S/harness -c`（`-DFLE_FRUITY` なし、`-g` なし）でコンパイルし、`cmp` で一致を確かめる（期待: 一致。静的には §0 の 4 のとおり）。
2. 既定のビルドで a4（`FLE_CALL_FRUITY` なし）: `bash $R/lift/out-git/fruity-results/tools/run6-fruity.sh $W/o1`。期待（V1／C1 と同じ）: TAG `t2331d00e8ebb`、`a4:` の 7 行が同一、`== stop: returned from entry`、functions 5,306、entries 1,007,942、harness 終了コード 0、**native の SHA-256 `9c76bc6a9270bd99e874cfd2633ca99334ba1a276d2337db235ecf7a25a23785`**（`sha256sum $W/o1/v-t2331d00e8ebb/native/two-native`）。**一致しなければ「既定のビルドが変わらない」は成り立たない**（報告する）。
3. 結合表の比較（2 の gen が済んだ後、秒）: `bash $R/lift/out-git/fruity-results/tools/merge_check.sh $W/base0/lift $S $W/mc $W/o1/src/fle-tables.txt $W/o1/v-t2331d00e8ebb/src/m1-tables.txt … m5-tables.txt`（表の順は `gen_two.sh` の `stage_merge` と同じ: FLEngine、m1〜m5。`*-tables.txt` はラッパーに消されない）。期待: `common.c identical without --extra-rows`、`printed lines identical…`、`diff lines … 8`。済んだら `rm -rf $W/o1`。
4. 環境変数の拒否（ビルドの前に止まるので OUT は作られない）: `FLE_FRUITY_SECONDS=5 bash …/run6-fruity.sh $W/ox` は `a4-multi: FLE_FRUITY_SECONDS is set without FLE_CALL_FRUITY=1`（終了コード 2）、`FLE_CALL_FRUITY=2 bash …/run6-fruity.sh $W/ox` は `a4-multi: FLE_CALL_FRUITY must be unset, empty, 0 or 1 (got '2')`（読み。コードの `die`）。

**手順 C: fruity 版の測定**

```
FLE_CALL_FRUITY=1 bash $R/lift/out-git/fruity-results/tools/run6-fruity.sh $W/o2 > $W/o2.stdout 2> $W/o2.stderr
```
上限を変えるとき: `FLE_FRUITY_SECONDS=120 FLE_FRUITY_MAX_CALLS=50000000`。名前で止めるとき: `FLE_FRUITY_STOP_AT=CreateWindowExW`。`FEBIN` を空にすると cargo で `fe-lift`／`fe-pe` を作る（V1 の `/tmp/opencode/v1/target-rel/release` が残っていれば既定で使う）。

stdout の見方: `---- a4-survey.sh` と `---- a4 lines` は DllMain の結果（V1 と同じはず）。**`---- fruity` ブロックが今回の結果**。`harness exit code` は fruity の停止コード（134 など）。ログは `$W/o2/runs/<時刻>-<pid>/a4/*-native.txt`（全文。backtrace の行は `fruity:` を持たないのでブロックに出ず、このログにだけある）。

**期待される出力（読み。実行していない）**

```
fruity: DllMain returned (the report above is its report). Stage 1: calling CreateFruityInstance as FL64.exe does (…)
fruity: CreateFruityInstance: export ordinal 3, va 0x1375990 (PE export table of the image at 0x400000)
fruity: host object at 0x… (0x38 bytes): vtable=0x… [slot0=0x7ffb00000010 slot1=0x7ffb00000020 slot2=0x7ffb00000030] version=1 flags=0 +0x10=0
fruity:   +0x18 name=0x… "FL Studio"  +0x20=0x… ""  +0x28=0x… ""  +0x30 base directory=0x… "C:\Program Files\Image-Line\FL Studio 21\" (buffer 0x8000 bytes)
fruity: before the call: functions executed (distinct) 5306, entries 1007942, shadow depth 12 (…)
fruity: calling f_1375990(rcx=0xc0, rdx=0xede95a, r8=host 0x…, r9=&instance 0x…) with rsp=0x…
```
その後は次のどれか。

| 出力 | 意味（読み） | 次に見るもの |
|---|---|---|
| `== stop: SIGABRT …`、`runtime message: fle: unsupported instruction at 0x4256d0`、`the stop is in the lifted thunk f_4256d0 … of import wintrust.dll!WinVerifyTrust`、`WinVerifyTrust arguments: hWnd=0 …` | **予想される最初の停止（H1）**。条件: 0x1375990〜0x1375bce の間で他に止まらないこと。そのとき `guest-return` は 0x1375bd3、GUID は GENERIC_VERIFY_V2、`WINTRUST_DATA` は cb=0x58 UI=2 revocation=0 union=1 state=1、パスは `C:\Program Files\Image-Line\FL Studio 21\FLEngine_x64.dll`（読み。§3） | `last … import-thunk` の列（その前に呼ばれた API。`GetModuleFileNameW` と `LocalAlloc` などが見えるはず）、`calls the engine made into the fake host object`（空かもしれない） |
| 別の `unsupported instruction at 0x<thunk>` と `import X` | 先に別の未応答 API（候補: `advapi32!RegDeleteKeyExW`）。H1 の条件が崩れた | その API の呼び出し元（`guest-return`、シャドウスタック） |
| `runtime message: fle: <API>: <理由>` | 応答表に行はあるが、その入力は名前付き停止にしてある（Learn が値を定めない場合など） | 該当 API の決定 |
| `fle: indirect call to 0x… is not a known function entry`（VA が 0 でも 0x7ffb… でもない） | ゲストが表にない VA を呼んだ | `dispatch target` に `import …` が付けば、IAT スロットを間接 call した |
| `fle: indirect call to 0 is not a known function entry`（VA が 0）、または文字列の断片のような VA | エンジンが**ホストの vtable のスロット 3 以降**を呼んだ可能性（FL64.exe の vtable は 3 スロットで、ハーネスのブロックでは直後が 0 のパディング、その後が文字列。読み: エンジンは 0〜2 だけ使う）。`guest-return` で呼び元を見る | もしそうなら**新しい発見**として報告 |
| `limit: … lifted function entries` ／ `limit: wall time` | ループ（`TApplication.Run` 相当の可能性。0x86a660）または長い処理 | シャドウスタックの下位、入口の列 |
| `SIGSEGV fault address …` | ゲストのポインタの誤りか runtime／ハーネスの欠陥 | C スタックと、手順 D |
| `fruity: rax=… instance=…` と `RETURNED` | 戻った。`eax` が 0 か 2 なら FL64.exe は `instance->vtable[0]` を呼ぶ（今回は呼ばない。記録だけ） | `instance->vtable=… slot0=…` の行、host の呼び出しの記録 |

**手順 D: gdb で C スタックを取る**（`$TN = $W/o2/v-<TAG>/native/two-native`。TAG は stdout の `a4-multi: real modules` の行）

```
export FLE_CALL_FRUITY=1 FLE_VFS_SEED=$S/harness/seeds/vfs-seed-fonts.tsv FLE_VFS_SEED_ROOT=$R FLE_FRUITY_TRAP=1
D=$W/o2/v-<TAG>/dll ; REAL=$D/dsp_ippv2_x64.2000000.dll:$D/Blend2D.4700000.dll:$D/quickfontcache_x64.1d00000.dll:$D/freetype_x64.4b00000.dll:$D/ILTools_x64.6000000.dll
ulimit -c 0 ; gdb -q -batch -ex 'set pagination off' -ex run -ex 'bt 40' --args $TN $R/binaries/win64/FLEngine_x64.dll $REAL
```
SIGABRT／SIGSEGV は gdb が先に止まる。上限での停止は `FLE_FRUITY_TRAP` の SIGTRAP で止まる。`-g` なしのビルド（`tools/wrap/clang`）では関数名（`f_<va>`）は見えるが行番号はない。

**手順 E: 報告に入れるもの**: 停止（または戻り）の `fruity:` 行全文、`harness exit code`、`du -sm /tmp/opencode` の最大値、手順 A〜C の結果（特に native の SHA）、host の呼び出しの記録（`slot2 id=… index=…` の列。**`id` が 3 以外で何が呼ばれたか**）。

## 11. wasm への展開に必要な変更の見積もり（**読み。作っていない**）

`fruity.c` は native 専用（`#error` で wasm のコンパイルを止める）。node の wasm で同じ測定をするには:

| 項目 | 変更 | 量（見積もり） |
|---|---|---|
| 停止の道 | wasm にはシグナルも `alarm` もない。`-Wl,--wrap=abort` の `__wrap_abort`（`harness/wasm/trace.c`）から `report()` を呼ぶ。実時間の上限は入口フックの中で `emscripten_get_now()` を見る | `fruity.c` の約 80 行の分岐 |
| 読み取り | `process_vm_readv` は使えない。`FLE_CHECKED_MEM` の領域判定か、`emscripten_get_heap_size()` で範囲を見る | 約 20 行 |
| `harness/wasm/trace.c` | `trace_depth`、`trace_frame`、`trace_full_message`、`trace_message_reset`、`trace_set_call_limit`、`trace_limit_hook` を足す（`trace_native.c` にある 6 つ。約 35 行） | `trace.c` に約 40 行 |
| ビルド | `build_nmod_wasm.sh` の `stage_harness` に `main_two.c`（`-DFLE_FRUITY=1`）と `fruity.c` を足し、リンクに `-Wl,--wrap=__cyg_profile_func_enter`。`gen_two.sh` の `FRUITY_ROWS` はそのまま使える | 約 10 行 |
| 実行 | `run_node_nmod.cjs` は変更不要（環境変数は NODERAWFS で `getenv` に届く。VFS seed と同じ） | 0 |
| 前提 | 完走には `VAR=force`（`-DFLE_LANDING_FORCE`、決定 0115）のビルドが要る。既定の NOJUMP は QFC の DllMain の setjmp 着地（159/942）で止まる。`FLE_WASM_FAST_RETADDR=1` なしの実行は約 33 分（引用: 決定 0115） | — |
| 別の問題 | `CreateFruityInstance` のブロッキングのメッセージループを**ブラウザのイベントループにどう接続するか**は設計の判断（host-engine-boundary.md §9 の 3）。この段階では扱わない。なお wasm のハーネスは node 専用（`-sENVIRONMENT=node`、`NODERAWFS`。決定 0045 の e）で、ブラウザ（`web`）はその時点で未対応 | — |
| ゲストのアドレス | 偽ホストの `calloc` ブロックは 32 ビットの線形メモリ内に入るので、`fle_addr_hi32` の検査（決定 0045 の d）に掛からない（読み）。`FLE_CHECKED_MEM` を使う場合も、ブロックはヒープ（`__global_base` 以上）に入る | — |

合計で数百行の変更。時間の見積もりは、測っていないので書かない。

## 12. PM の判断が要る点・解釈が分かれた点（決めていない）

| ID | 内容 | 採った暫定（この実装） | 他の選択 |
|---|---|---|---|
| J1 | ゲストが偽ホストの関数を呼ぶ道（§5） | D: 結合表に 3 行（生成 C・runtime 不変） | A: 応答表に行（生成 C が変わる）、B: runtime に NULL フック |
| J2 | 「native の SHA を変えない」の解釈。基準のバイナリ（`9c76bc6a…`）を**ビット単位で変えない**ことか、a4 の結果と FLEngine の生成 C の SHA を変えないことか | 前者を満たすよう、新コードを `-DFLE_FRUITY`／`FRUITY=1` のときだけ入れた（既定のビルドの入力は同一）。代償: fruity 版は別の TAG（モジュールのオブジェクトを再ビルド）で、`FLE_CALL_FRUITY=1` と別の経路 | 常にリンクして実行時の環境変数だけで切り替える（簡単だが、基準のバイナリの SHA が変わる） |
| J3 | 基底ディレクトリの値 `C:\Program Files\Image-Line\FL Studio 21\`（`FLE_VIRTUAL_DLL_PATH` の親） | 環境の主張（決定 0018 の PM の値から導出） | 空文字（エンジンは自分の DLL のディレクトリを使う。host-engine-boundary.md §6.1） |
| J4 | スタブの擬似 VA `0x7FFB000000[1-3]0` | 出典なしの選択 | 別の領域 |
| J5 | 署名検査（§8）: 何も答えないで止める。**先へ進むには少なくとも WinVerifyTrust と crypt32 の 6 API の答えが要る**（`Image Line NV` との署名者名の比較までエンジンが行う） | 止める | (a) 成功を返す（環境の主張: この DLL は署名済みで署名者は Image Line NV）、(b) 署名の中身を持つ仮想の証明書ストアを作る、(c) `Flags` のビット 0 を立てる（FL64.exe と形が変わる。自己検査は残る） |
| J6 | 署名の後にも未応答の API が 100 個以上ある（`logs/unanswered-reachable-after-first-WinVerifyTrust.txt`）。user32 の API 83 個（ウィンドウ、メッセージ、メニューなど）、crypt32 7、kernel32 7（スレッドなど）、DirectSound など | 扱わない | A4 のような「壁を 1 つずつ直す」ループ（census 的な洗い出しを先にするか） |
| J7 | ブロッキングのメッセージループの扱い。今は回数と時間の上限で打ち切る | 打ち切り | ループを分割する、`PeekMessageW` に仮想の空キューを答える、など（host-engine-boundary.md §9 の 3） |
| J8 | `fle_call_guest` を使った（呼び出しの前後で `R[4]`、戻りアドレス、被保存レジスタを検査し、崩れたら名前付きで止まる）。生の呼び出し（`main_two.c` が DllMain を呼ぶ形）にすると検査がなくなる | `fle_call_guest` | 生の呼び出し |
| J9 | DllMain 完走後の状態から始める。ゲストの終了経路（`f_4128b0` → `f_412770` → longjmp）が `InitCount`、`InitContext+0` を 0 にしている（V1 の引用）。実機で同じ状態になるかは**照合していない**（実機なし）。シャドウスタックには 12 個の古いフレームが残る | そのまま | `InitContext` を戻して始める（ゲストの状態を書き換えることになる） |
| J10 | 任意機能（`FLE_FRUITY_STOP_AT`、`FLE_FRUITY_TRAP`、`WinVerifyTrust` の引数の解読、引数が指す先のプレビュー）は依頼の「最小」を超える | 入れた | 外す（`fruity.c` の `stop_named`、`decode_wvt`、`describe_value` と `cfg_trap`） |
| J11 | `fruity` の停止報告を `trace_report` と別にした（`trace_report` は 1 度しか出せず、`a4:` の行を 2 度出す） | 別 | `trace_native.c` に再出力の口を足す（a4 の行が重なる） |
| J12 | ホストオブジェクトは DllMain の**後**に作る（FL64.exe は LoadLibrary の前に作る）。DllMain は host に触れない（読み: host ポインタは 0x1375a22 の `CreateFruityInstance` の中でだけ保存される）ので、結果は同じと読む | 後 | 前（`malloc` の並びが変わる） |
| J13 | 第 2 段階の期待値（H1: 最初の停止は WinVerifyTrust）は静的な読みの**下限**に基づく（直接の `call` だけをたどった）。外れても欠陥ではない | — | — |

## 13. 限界・未確認

- **【第 1 段階の時点の記述。第 2 段階でビルド・リンク・実行した。冒頭の更新と `create-fruity-instance-measurement.md` を参照】** **何も実行していない**: コンパイル、リンク、実行、`--wrap` の効き、`process_vm_readv` が許されるか、`backtrace` の出力、gdb の挙動は**すべて未確認**。構文確認（§9）は型と文法だけを見ており、論理の誤りは見つけられない。自己テストの 47 件は期待値を文書から手で書いたもので、第 2 段階で初めて動く。
- `-finstrument-functions` が static inline の補助関数（`fle_ld` など）も計装する点は読んだ（`trace_native.c` の `is_lifted` の存在から）。入口フックは「ディスパッチ表にある関数だけ」を記録するようにしたが、これも実行して確かめていない。
- FL64.exe の読みは線形逆アセンブルの範囲だけ。UTF-16→UTF-8 の変換（0x140002940）は読んでいない。`Run` の他の分岐（`_Copy`、`ShowEngineChoice`）は今回の対象外。
- `WINTRUST_DATA` のオフセットは、エンジン自身が書く構造体の命令（0x1375b48〜）と合っていることを確かめた（読み）が、Microsoft Learn の定義そのものは今回確認していない。
- エンジンが `Dispatch` の戻り値を使うか（U2）、`Flags` のビット 1〜3（U3）、`+0x10`／`+0x20`／`+0x28` の用途（U4）は未解明のまま。
- 静的な到達可能性（§0 の 6、`logs/reach-*.txt`）は直接の `call`／tail `jmp` だけをたどった**下限**。間接呼び出し（vtable、`call reg`）は追っていない。
- FLEngine 単独（0 モジュール）の構成、wasm、ブラウザ、実機 Windows は対象外。
- `FLE_CALL_FRUITY=1` を、`FRUITY=1` なしで作った `two-native`（従来のバイナリ）に渡すと、コードがないので**黙って無視される**（`a4-multi.sh` 経由なら fruity 版が作られる）。直接実行するときは `---- fruity` 相当の `fruity:` 行があることを確かめる。

## 14. 成果物（`lift/out-git/fruity-results/`）

| ファイル | 内容 |
|---|---|
| `fruity.diff` | 本体（79,614 バイト）。`git archive 86439bc` と `228c78a` の両方に `patch -p1` が当たる。適用した木とワークツリーの `diff -r` は空（`logs/patch-and-static.txt`） |
| `create-fruity-instance-harness.md` | この文書（`docs/investigations/` に置く内容） |
| `MERGE-NOTES.md` | 統合の注意、stamp、規則違反の自己申告 |
| `tools/` | `fl64_read.py`（メモリ上でだけ FL64.exe を読む）、`cfi_calls.py`、`reach.py`（静的な到達可能性）、`merge_check.sh`、`run6-fruity.sh`、`wrap/clang`、`wrap/python3` |
| `logs/` | `syntax-checks.txt`、`patch-and-static.txt`、`merge_check-synthetic.txt`、`cfi_calls-0x1375990-0x13765f0.txt`、`reach-*.txt`、`unanswered-reachable-after-first-WinVerifyTrust.txt` |
