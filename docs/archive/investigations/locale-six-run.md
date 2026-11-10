# GetLocaleInfoW の en-US 6 種の実装と harness の再実行の記録

実行日: 2026-10-21（JST）。作業ディレクトリ `signature-studio-develop`、HEAD b7adccb（ワークツリーには他の担当の未コミット変更がある）。決定は `docs/decisions/0048-locale-six.md`、調査は `docs/investigations/locale-6-official-sources.md`。「実測」はツールの出力、「読み」は静的な読み、「推測」は未検証。実機の Windows では確認していない（値は仮）。

## 1. 結果

| 項目 | 実測 |
|---|---|
| 実装した LCType | 6 種: 0xc `,`、0x1f `M/d/yyyy`、0x1003 `h:mm:ss tt`、0x79 `h:mm tt`、0x20 `dddd, MMMM d, yyyy`、0x1c `0`（整数） |
| `tests/locale.rs` | 13 件 passed、0 failed（新規 `get_locale_info_enus_0048`。`undefined_cases_abort` を局所変更） |
| ワークスペース全テスト | **65 バイナリ、616 passed、0 failed、0 ignored**（`lift/out-loc4/test-ws.txt`。直前の `wasm-port-run.md` の記録は 615 で、差の 1 件は `get_locale_info_enus_0048`） |
| harness の停止点 | 0x1c の `GetLocaleInfoW` の abort → **`fle_unsupported` at 0x423220**（`Jmp`、IAT スロット経由の間接 jmp） |
| 関数実行数（distinct） | 199 → **281** |
| 入口数（`function entries (calls)`） | 1,386 → **5,904** |
| 停止時の呼び出し深さ | 9 → 7 |
| 生成の `unsupported_total` | **601**（10 種類。0042 後の記録 601 と同じ。生成 C は `out-wasm2/out/wasm-pdata.c` と `cmp` で同一） |
| wasm（node）の停止点 | native と同じ（`calling` 行の `rsp=` 以外は 1 行も違わない） |

「前」の数値（199 / 1,386 / 深さ 9）は `lift/out-wasm2/run-native.txt`（0x1c で停止した直前の native の実行）から。

## 2. 実装（`lift/runtime/fle_locale.c`、追記のみ）

- 定数 6 つ（`FLE_LC_SLIST`、`FLE_LC_INEGCURR`、`FLE_LC_SSHORTDATE`、`FLE_LC_SLONGDATE`、`FLE_LC_STIMEFORMAT`、`FLE_LC_SSHORTTIME`）と `fle_locale_info_value` の `case` を追加。文字列型 5 種は 0042 の文字列型と同じ形（`lcid != 0x409` なら 0 を返す = 呼び出し側で abort）、`INEGCURR` は `IOPTIONALCALENDAR`（0x100b）と同じ整数型（`*is_num = 1`、`*num = 0`）。`GetLocaleInfoW` 本体は変更なし（`LOCALE_RETURN_NUMBER` の扱いは既存の整数型と同じ）。
- 0042 のコメント「6 種は deliberately NOT here」を、0048 を指す文に更新（コメントのみ）。
- 他の担当の未コミット変更（`long` を固定幅に、`(uintptr_t)` を `fle_addr_opt()` に置換: `fle_locale.c` の 12 行）は混在していた。編集の直前に読み直し、そのまま残した（`git diff` で `fle_addr_opt` の 12 行が変わっていないことを確認）。

## 3. テスト（`lift/crates/fe-lift/tests/locale.rs`、局所変更）

- 追加 `get_locale_info_enus_0048`: 6 種 × LCID（0x409、0x400、0x800）で、サイズ問い合わせ、バッファ（余白 3 文字に 0x5555 の番兵）、1 文字足りない（ERROR_INSUFFICIENT_BUFFER）、NULL バッファ、負のサイズ（ERROR_INVALID_PARAMETER）。`LOCALE_NOUSEROVERRIDE` の 0x79。`LOCALE_RETURN_NUMBER` の 0x1c（DWORD 0、戻り値 2、cchData 1 は ERROR_INSUFFICIENT_BUFFER）。
- `undefined_cases_abort` の局所変更:
  - `gli-0042-slist`、`-inegcurr`、`-sshortdate`、`-slongdate`、`-stimeformat`、`-sshorttime`（en-US で abort を期待していた 6 行）を、**ドイツ語（0x407）で abort** の `gli-0048-german-*` 6 行に置き換え（en-US では答える期待へ）。
  - 追加: `LOCALE_RETURN_NUMBER` を文字列型に付けたら abort（`gli-0048-return-number-slist`、`-sshortdate`）、INEGCURR の `LOCALE_RETURN_NUMBER` と cchData 0 は abort（`gli-0048-return-number-inegcurr-size0`）。
  - `gli-lctype-unknown` は、0x1f を使っていたため、答えていない 0x1001（LOCALE_SENGLISHLANGUAGENAME）に変更（「未知の LCType は abort」を保つため）。0x1001 を選んだのは、閉包が要求せず、決定のどこにも出てこない LCType だから（読み）。

## 4. harness の再実行

手順は `docs/investigations/wasm-port-run.md` の native と wasm（`build_native.sh`、`build.sh`）。出力は `lift/out-loc4/` 内（`TMPDIR=lift/out-loc4/tmp`、`EM_CACHE=lift/out-loc4/wasm/emcache`）。

1. 生成: `out-wasm2/target/release/fe-lift lift FLEngine_x64.dll lift/out-loc4/nb/out/wasm-pdata.c --pdata`（2.5 秒）。出力は `out-wasm2/out/wasm-pdata.c` と `cmp` で同一（runtime だけの変更なので生成は変わらない、という予想と一致）。生成 C の分割 16 本は、2 GB 級の複製を避けるため `out-wasm2/out/` への**シンボリックリンク**（`lift/out-loc4/nb/out/`、`lift/out-loc4/wasm/out/`）。
2. native: `OUT=lift/out-loc4/nb bash lift/harness/wasm/build_native.sh`（1 分 5 秒。16 本のオブジェクト、runtime 32 本、harness）→ `lift/out-loc4/nb/native/flengine-native`（約 258 MB）。実行: `run-native.txt`、`run-native.err`。
3. wasm: `OUT=lift/out-loc4/wasm bash lift/harness/wasm/build.sh rt gen-c harness link`（55 秒、emcc 同時 2 本）。runtime 32 本のコンパイルは errors 0、warnings は `fle_proc.c` の 1 件のみ（本作業の外）。生成 C 16 本は errors 0。`flengine.wasm` は 115,724,550 B。実行: `node bin-base/run_node.cjs binaries/win64/FLEngine_x64.dll`（`run-node-base.txt`）。
4. 比較: `diff <(grep -v '^calling' run-native.txt) <(grep -v '^calling' run-node-base.txt)` は差なし。`calling … rsp=` の 1 行だけが違う（native `0x713b207fffc0`、wasm `0x11c49bd0`）。

## 5. 次の停止点

- `fle: unsupported instruction at 0x423220`（`abort kind: fle_unsupported`）。呼び出しスタック（native と wasm で同じ）: `f_423220` ← 0x524c69（`f_524c40`）← `f_525600` ← `f_4120c0` ← `f_412160` ← `f_41d8f0` ← エントリ `f_1376820`。
- 0x423220 は `jmp QWORD PTR [rip+0x154e5ba]`（スロット 0x19717e0）。インポート表を PE のヘッダから引くと、スロット 0x19717e0 は **kernel32.dll の `CreateEventW`**（実測。Python で IAT を読んだ）。生成 C のこの行は `fle_unsupported(0x423220)`（`Jmp: indirect through IAT slot (not lifted)` 557 件のうちの 1 件）。
- 呼び出し元 0x524c40: 0x524c59 〜 0x524c64 で `rcx = 0`、`edx = -1`（`or edx, 0xffffffff`）、`r8 = 0`、`r9 = 0` を置いて呼ぶ。`CreateEventW(lpEventAttributes, bManualReset, bInitialState, lpName)` の引数としての読み: 属性なし、手動リセット（非ゼロ）、初期状態は非シグナル、名前なし（読み）。
- `lift/runtime` と `lift/crates` に `CreateEvent` の文字列は無い（実測。`grep`）。実装は本作業の範囲外なので、広げていない。

## 6. 範囲外・限界

- 値はすべて仮（決定 0048）。0x20 は `d` か `dd` か、0x1c は `0` か `1` かが、実機で変わり得る。
- 日付書式と INEGCURR の後段の使われ方、ゲストが 0x1f 以降（0x1003、0x79、0xc）を呼んだあとの経路は、harness が 0x423220 で止まるまでに 0x1c の次の 5 種を通過した（停止点が進んだこと）まで確認した。`obj+0x10`、`+0x18`、`+0x30`、`+0x38`、`+0x17e` の使用箇所は追っていない。
- ワークスペース全テストは他の担当の未コミット変更を含むワークツリーで実行し、0 failed だった。失敗がなかったので、HEAD に自分の変更だけを重ねたコピーでの切り分けは行っていない。
- wasm の確認は停止点（1 経路）まで。停止点より先の実行（SSE/x87 の演算の wasm と native の一致など）は未確認。

## 7. 触ったファイルと制約の遵守

- 編集: `lift/runtime/fle_locale.c`、`lift/crates/fe-lift/tests/locale.rs`（`fle_locale.c` には他者の未コミット変更が混在していた: `fle_addr_opt` への置換 12 行。`locale.rs` は `git diff` が 2 ハンクで、どちらも本作業の変更だけ = 他者の変更の混在なし）。
- 新規: `docs/decisions/0048-locale-six.md`、本書、`lift/out-loc4/`（`tmp`、`emcc-cache`、`nb`、`wasm`、`target`、`test-ws.txt`。`target` は cargo のビルド出力、`nb` と `wasm` はビルド成果物と実行記録）。
- `rm`、`>` による既存ファイルの上書き（新規ファイルへのリダイレクトは使用）、git 操作、`/tmp`、コマンド文字列の `sleep`/`timeout` は使っていない。`out-wasm2` の大きなファイルは複製せず参照（シンボリックリンク）。権限拒否はなし。
