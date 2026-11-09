# VerifyVersionInfoW の size 0 を検査しない変更（0040、B-narrow）後の harness 再実行の記録

日付: 2026-10-21。決定は `docs/decisions/0040-versioninfo-size0.md`（**仮の決定。非公式の出典に基づく。実機 Windows は未確認。実機で FALSE などと分かれば撤回する**）。調査は `docs/investigations/versioninfo-size0-spec.md`。手順は `pdata-seeds-run.md`、`indirect-jmp-run.md`（split ビルドの方式）。成果物は `lift/out-vi3/`（`tmp/` は TMPDIR、cargo の target は `out-vi3/target`）。

「実測」はツールの出力、「推測」は未検証の解釈。

**混在の注意**: 作業ツリーには x87 担当の未コミット変更（`fe-lift/src/x87.rs`、`runtime/fle_x87.c/.h`、`lib.rs` の x87 部分ほか）が混在している。今回は `runtime/fle_x87.c` が作業ツリーに存在し、ビルドできたので、前任のスタブ（`out-ijmp/fle_x87_stub.c`）は**使わなかった**（実際の `fle_x87.c`、`fle_x87ctl.c` をリンクした）。生成・ビルド・実行・テストはすべてその混在状態（作業開始時の HEAD は ab58ed4。作業中に x87 担当が 8ccbfa5 をコミットして HEAD が進んだ。他担当の未コミット + 私の変更）で行った。どの時点のツリーで生成・テストしたかは、コミットの前後を区別して記録していない。また、作業ツリーには x87 以外の担当（SSE、mxcsr、prefetch など）の未コミット変更も多数あり、`git status` で `fe-lift/tests/*.rs` の多くが M になっている。

## 1. 変更（実測）

- `lift/runtime/fle_osver.c` の `fle_osver_check`: `dwOSVersionInfoSize` が **0 のときは検査せず**、0 でも 284 でもないときは従来どおり名前付き abort（メッセージは `dwOSVersionInfoSize is neither 0 nor sizeof(OSVERSIONINFOEXW)`）。階層メンバ（major、minor、SP major、SP minor）の読み取りは、`dwTypeMask` のビットが立っているものだけに限定した（以前は 4 つとも常に読んでいた。size 0 のとき構造体の他のメンバを読まないため）。product type は `fle_emulated_os.product_type`（= 1）に対して `fle_ver_cmp` で比較する。結果は比較の計算で決まり、固定していない。
- 既存の size 284 の挙動と、テスト `verify_version_info_matches_spec_rules_on_emulated_os`（ほか既存）は変わらず通過。
- `lift/crates/fe-lift/tests/osver.rs`:
  - `undefined_cases_abort` の `size-0-as-f_454030`（abort 期待）を外し、`size-1`、`size-0x11d`、`size-high-bit`（size = 0x80000000）の 3 行（abort 期待）を追加した。**0x114（OSVERSIONINFOW 276）は既存の `bad-size` 行（`b[0] = 0x14`、b[1] は初期化子の 0x01 で 0x114）が既に abort を保証しているので重複させていない**（コメントで明記）。
  - 新規 `size_0_is_not_checked`: size 0、構造体の size を除く +4〜+281 を 0xFF で埋め（TypeMask が示さないメンバを読めば結果が変わる）、wProductType だけ設定。4 行（`f_454030` の呼び出しそのもの = product 1 / VER_EQUAL、product 3 / EQUAL、product 1 / GREATER_EQUAL、product 2 / LESS）。期待値は既存の `expect`（仕様の規則を Rust の式で書いたもの）で計算し、戻り値と GetLastError（TRUE なら 0 のまま、FALSE なら 1150）を比べる。`f_454030` の期待が TRUE、全体に TRUE と FALSE の両方が含まれることもテストで確かめている。
  - 変異による検出力の確認は**行っていない**。
- `lift/crates/fe-lift/tests/versioninfo.rs`: 冒頭コメントの注記だけ更新（逆アセンブル検証は変更なし）。

## 2. harness（all-pdata）の再生成・再実行（実測）

コマンド（`lift/out-vi3/` で）: `fe-lift lift ../../binaries/win64/FLEngine_x64.dll out/vi3-pdata.c --pdata`（`gen-pdata.txt`）→ `split_c.py out/vi3-pdata.c out/vi3-pdata-part 3700`（16 分割）→ `EXTRA_RT="…" JOBS=8 sh build-parts.sh vi3-pdata`（`out-ijmp` の複製。runtime の全 `fle_*.c`（30 個）をリンク）→ `./bin/flengine-vi3-pdata ../../binaries/win64/FLEngine_x64.dll`（`run.txt`、`run.exit` = 134）。

| 項目 | 前（`out-ijmp/run.txt`） | 後（`out-vi3/run.txt`） |
|---|---|---|
| 停止点 | `VerifyVersionInfoW`（`fle_osver.c`、size 不正の abort） | **`GetLocaleInfoW`**（`fle_locale.c:283`、`LCType value for en-US is not on the MSDN pages`） |
| 関数実行数（distinct） | 147 | **189** |
| 関数入口（calls） | 736 | **957** |
| 呼び出し深さ | 7 | 10 |
| 生成の `unsupported_total` | 698（19 種類） | **667**（19 種類） |
| 閉包の命令 / 関数 | 2,526,826 / 58,695 | 2,526,826 / 58,695（不変） |
| `c_lines` | 2,880,335 | 2,880,419 |

- ビルド: 24.07 秒、最大 RSS 1,203,628 KB、EXIT=0。生成 C は 216,076,847 バイト。
- **`unsupported_total` が 698 → 667 に減ったのは、この変更の効果ではない**（runtime の変更は生成に影響しない）。作業ツリーの x87 担当の未コミットの lift 変更（x87 の命令が lift されるようになった）による変化と読める（推測。内訳の一覧の差は取っていない）。前の 698 は x87 を含まない時点の生成。
- 生成の一覧に残る未対応（実測）: `Jmp: indirect through IAT slot (not lifted)` 557、`Mulpd` 25、`Addpd` 13、`Psubw` 10、`Cvtdq2ps` 9、`Pcmpgtd` 8、`Rcpps` 8、`Subpd` 8、`Bswap` 7、`Test: high-byte register` 7、`Prefetcht0` 5 ほか。x87 は `F2xm1` 1、`Fsincos` 1 が残る。

### VerifyVersionInfoW の通過の確認（gdb、`gdb-vi4.cmds`、`gdb-vi4.txt`）

- 停止点の手前の VerifyVersionInfoW は 2 回。1 回目: 呼び出し元 f_409b30（size 284、R2 = 0x23、R8 = 0x1801b）で戻り値 1（前から変わらない）。2 回目: `f_454030` の中。**R2 = 0x80、R8 = 0x200000、`dwOSVersionInfoSize` = 0、wProductType = 1、戻り値 R0 = 1（TRUE）**。0039 の実測の入力と一致する。
- 戻り値 1 → f_454030 は 0 を返す → f_454090 は OS 名に "Windows 10" を選ぶ（0040 の影響、`versioninfo-size0-spec.md` §3.2。選ばれた文字列そのものは今回の gdb では読んでいない。命令の読みと戻り値からの推論）。

## 3. 次の停止点の原因（実測の範囲）

- **`GetLocaleInfoW`、lcid = 0x409、LCType = 0x100b、cchData = 256**（gdb `gdb-vi5.txt` で `fle_locale_stop` の上のフレームの R[1]、R[2]、R[9] を読んだ）。`fle_locale.c:283` の `fle_locale_info_value(base, ...)` が 0x100b を知らず、名前付き abort。
- 呼び出しの鎖: `f_423560`（`jmp [IAT 0x19716f8]`。0x423560 の 1 命令）← `f_446370`（0x446392 の `call 0x423560`。r8 = `[rbp+0x20]`、r9d = 0x100）← `f_447860` ← `f_446400` ← `f_44bb70` ← `f_455790` ← `f_4120c0`（表のループ）← `f_412160` ← `f_41d8f0` ← 入口 `f_1376820`。
- LCType 0x100b がどの LOCALE_ 定数かは**確認していない**（0x1000 台は LOCALE_SENGLISH... などの名前を持つ値があるが、対応は Microsoft Learn で確認していない。推測にとどめる）。参考実験で出た停止点（前任の参考実験の `GetLocaleInfoW LCType 0x100b @ fle_locale.c:283`）と**同じ**。範囲外のため実装していない（`fle_locale.c` は変更していない）。
- 停止の手前で `fle_uipref_locale_string` 経由の GetLocaleInfoW（LCType 92 = 0x5c、89 = 0x59）が複数回呼ばれ、通過している（gdb `gdb-vi3.txt`）。

## 4. テスト

- 変更に関わるテスト（`cargo test --offline -p fe-lift --test osver --test versioninfo`）: osver **7 件**（既存 6 + 新規 `size_0_is_not_checked`）と versioninfo **3 件**、すべて passed。
- ワークスペース全体（`cargo test --offline --workspace --no-fail-fast`、`out-vi3/test-ws.txt`、`test-ws.exit` = EXIT=101）: 62 バイナリ、**589 passed、1 failed**、0 ignored。
  - 失敗 1 件は `fe-lift/tests/null0.rs` の `runtime_answers`（`Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }`）。既知の競合。**単独再実行（`test-null0-rerun.txt`）は 6 件すべて passed**。再実行を含めると失敗は 0。
  - x87 担当の未完成分の失敗は、今回は**出なかった**（`fle_x87.c` が存在し、x87 のテストは通過した。前任の記録の x87 の失敗 8 件とは状況が違う）。
  - 前任（`indirect-jmp-run.md` §4.3）の 531 件・8 失敗とは作業ツリーの状態が違うので、件数の比較は意味がない（他担当のテストが増えている）。

## 5. 制約の遵守と残置物

- `rm`、`>` による既存ファイルの上書き（新規ファイルへのリダイレクトと、`cp` による新規コピーは使った）、git 操作、`/tmp`、sleep/timeout は使っていない。TMPDIR は `lift/out-vi3/tmp`。権限拒否はなし。外部通信なし。2 GB 級の中間ファイルなし（最大は `out/vi3-pdata.c` 216 MB、`parts/*.o` の合計約 500 MB）。`lift/out-vi3/` 全体は約 2.9 GB（cargo の `target` を含む。`out-pdata` の大きなファイルの複製はしていない）。
- 後始末できていないもの（削除禁止）: `out-vi3/gdb-vi*.cmds`、`gdb-vi*.txt`、`gdb-vi*-stdout.txt`（gdb の作業ログ）、`out-vi3/out/vi3-pdata.c` と分割 16 個、`build.err`（`time -v` の出力）。
- 触ったファイル（`lift/out-vi3/` の生成物を除く）: `lift/runtime/fle_osver.c`（M）、`lift/crates/fe-lift/tests/osver.rs`（M）、`lift/crates/fe-lift/tests/versioninfo.rs`（M、コメントのみ）、`docs/decisions/0040-versioninfo-size0.md`（新規）、`docs/investigations/versioninfo-size0-run.md`（新規）。
