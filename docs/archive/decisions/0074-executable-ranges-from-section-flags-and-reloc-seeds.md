# 0074: 実行範囲を「名前が .text」ではなくセクション特性で決める（複数範囲）。reloc seed（pdata 開始と一致するものだけ）を足す

状態: 作業者（サブエージェント）が記録した（2026-10-23）。決定 0073 の後続作業 W3(a)(b)（`docs/investigations/dsp-ippv2-lift-feasibility.md` §9）の実装。値を決める決定ではなく、fe-pe / fe-decode の定義の変更と、その測定結果の記録。**FLEngine_x64.dll の結果は変わらない**（下の実測）。

## 状況

`fe-decode` の `Text::from_pe` は、名前が `.text` のセクションだけを実行範囲として扱っていた。`dsp_ippv2_x64.dll` はコードが `.text`（0x17de140 B）と `IPPCODE`（0x6dc771 B）の 2 セクションにあり、そのままでは `IPPCODE` が範囲外になる（実測。下の表の「変更前」）。

## 変更したこと

1. **fe-pe**（`src/lib.rs`）: `IMAGE_SCN_CNT_CODE`（0x20）と `IMAGE_SCN_MEM_EXECUTE`（0x20000000）の定数（出典: Microsoft Learn「PE Format」の Section Flags の表。2026-10-23 に読んだ）と、`Section::is_executable()`（MEM_EXECUTE）、`Section::has_code_content()`（CNT_CODE）を追加。
2. **実行範囲の定義**: 実行範囲 = **`IMAGE_SCN_MEM_EXECUTE` を持つ全セクション**（名前は見ない）。`IMAGE_SCN_CNT_CODE` だけで MEM_EXECUTE の無いセクションは**範囲に入れない**（「実行できる」と PE 仕様が言うのは MEM_EXECUTE。CNT_CODE は「コードを含む」という内容の印）。CNT_CODE は `has_code_content()` として別に読めるが、閉包は使わない。
3. **fe-decode**（`src/lib.rs`）: `Text` を複数範囲（`CodeRange { va, bytes }` の VA 昇順の列）にした。`Text::from_pe`（MEM_EXECUTE のセクションごとに 1 範囲）、`Text::single(va, bytes)`（テスト用の手作りバイト列）、`ranges()`、`range_at(va)`、`slice(va, len)`（1 つの範囲に収まるものだけ）、`contains`、`decode` を持つ。`Text` の公開フィールド `va` と `bytes` は無くなった。使っていた `adjustor.rs`、`dataflow.rs`（`leaders` は範囲ごとに線形走査）、`jumptab.rs`、`main.rs`、テスト `tests/adjustor.rs` を合わせた。`indirect.rs` と `fe-lift/src/lib.rs` は `contains` と `decode` だけを使うので変更なし。
   - 細部: セクションの `VirtualSize` が `SizeOfRawData` を超えるときは、ファイル上にある長さで切る（従来は `slice_at_rva` が失敗していた）。`SizeOfRawData` が 0 のセクションは範囲に入れない。実行セクションが 1 つも無いと `no executable section` のエラー（従来は `no .text section`）。
4. **reloc seed**（`fe-decode/src/relocseeds.rs`、新規）: `.reloc` の DIR64 が指す 8 バイトのスロットの値のうち、**実行範囲内で、`.pdata` の関数開始（連鎖エントリを除く。`Pe::function_starts`）と一致するものだけ**を seed にする。実行範囲内で `.pdata` 開始と一致しない値は種にせず、数えて（`not_function`）報告する。決定 0051 の補正 thunk seeds（`adjustor.rs`）とは別の集合: 0051 は `.pdata` 開始でも `.pdata` の範囲内でもない値の中から `add/jmp` の形のものだけを取る。reloc seed は `.pdata` 開始そのものだけを取る。重ならない。`Pe::dir64_relocation_vas` は 0051 のものを再利用し、fe-pe には足していない。
5. **CLI**:
   - `fe-decode closure FILE ... --reloc-seeds`（`--pdata`、`--adjustors` と組み合わせられる。reloc seed の件数を出力する）。出力に `text_ranges` と各 `range` の行を足した（`text_va` と `text_size` は最初の範囲のまま残してある）。
   - `fe-decode reloc-seeds FILE [--not-function]`: seed の VA を 1 行 1 つ（0x 付きの 16 進）で標準出力、件数を標準エラー出力。`--not-function` は seed の代わりに「関数開始でない値」を出す（調べる用）。出力はそのまま `fe-lift lift ... --seeds FILE` に渡せる。
   - **`fe-lift` の CLI（`main.rs`）は変えていない**（編集してよい範囲が `fe-lift/src/lib.rs` だけだったため、`lib.rs` も変えていない）。`fe-lift` で使うときは `fe-decode reloc-seeds` の出力を `--seeds` に渡す。`fe-lift lift --reloc-seeds` を足すかは未決（PM が `main.rs` の編集を許すなら小さい変更）。
6. 既定の挙動: 実行属性を持つセクションが `.text` だけのイメージ（FLEngine など）では従来と同じ。「従来動作を既定、新動作をオプション」にする必要は無かった（下の実測）。

## FLEngine_x64.dll は変わらない（実測）

`FLEngine_x64.dll` のセクション特性（`fe-pe pe-info`）: 実行属性（0x20000000）を持つのは `.text`（flags 0x60000020）だけ。`.data`、`.bss`、`.idata`、`.didata` は 0xc…、`.edata`、`.rdata`、`.pdata`、`.rsrc` は 0x4…、`.reloc` は 0x42000040。したがって範囲は 1 つのまま。

変更前に作った実行ファイル（`lift/out-w3/base/`）と変更後で、`fe-lift lift FLEngine_x64.dll OUT.c --pdata`（README の手順。all-pdata の閉包）を比べた:

| 項目 | 変更前 | 変更後 |
|---|---|---|
| 関数 | 59,297 | 59,297 |
| 命令（`closure_instructions`） | 2,528,030 | 2,528,030 |
| `unsupported_total` | 531 | 531 |
| `bad_nodes` | 0 | 0 |
| 生成 C | 215,700,923 B | 215,700,923 B |
| 生成 C の SHA-256 | 67827424301459e6f2da1ccb46a5fd081b7de9a38c64e1bca44ba9bf5d15584d | 同じ（`cmp` で全バイト一致） |

`fe-lift` の統計出力は全行が一致（`diff` で差なし）。`fe-decode closure --mnemonics --pdata --adjustors` の出力の差は、足した `text_ranges 1` と `range` の 2 行だけ。`decode_failures`、`out_of_text_flow`、`out_of_text_calls` は 0 のまま。

FLEngine での reloc seed（参考。`--pdata --adjustors --reloc-seeds` を合わせても閉包は 59,297 関数 / 2,528,030 命令のまま増えない）: DIR64 スロット 278,531（`tests/reloc.rs` の値と一致）、読めないスロット 0、実行範囲内の distinct な値 80,188、うち `.pdata` 開始 24,592、開始でない 55,596。reloc seed だけを種にした閉包は 41,407 関数 / 2,214,158 命令（参考）。

## dsp_ippv2_x64.dll の数（実測。`lift/out-ext/dsp_ippv2_x64.dll`、SHA-256 497dd4f7…c352、ImageBase 0x180000000 のまま）

セクション: `.text`（0x60000020）と `IPPCODE`（0x60000020）が実行。`.rdata`、`.pdata`、`_RDATA`、`.rsrc`、`.reloc` は 0x4…、`.data` と `IPPDATA` は 0xc…。範囲は 2 つ（`0x180001000` 長さ 0x17de140、`0x1817e0000` 長さ 0x6dc771）。

手順（`fe-lift` は README のとおり。`CARGO_TARGET_DIR=lift/out-w3/target`、release ビルド。統計だけで足りるので生成 C は書かず `/dev/null` に出力した）:

```
fe-lift lift lift/out-ext/dsp_ippv2_x64.dll /dev/null --pdata
fe-decode closure lift/out-ext/dsp_ippv2_x64.dll --mnemonics --pdata --adjustors
fe-decode reloc-seeds lift/out-ext/dsp_ippv2_x64.dll > lift/out-w3/dll/reloc-seeds.txt
fe-lift lift lift/out-ext/dsp_ippv2_x64.dll /dev/null --pdata --seeds lift/out-w3/dll/reloc-seeds.txt
```

| 項目 | 調査文書の「そのまま」（`.text` のみ。変更前のバイナリで再測定し一致） | 調査文書の merged コピー（実在の DLL ではない） | 今回（変更後。実在の DLL そのまま） |
|---|---|---|---|
| 関数 | 25,401 | 32,162 | **32,162** |
| 命令（閉包） | 4,855,751 | 6,458,203 | **6,458,203** |
| 誤デコード（`decode_failures`） | 0 | 0 | **0** |
| 範囲外の流れ / 範囲外の呼び | 35 / 2,429 | 0 / 0 | **0 / 0** |
| 生成後の `bad_nodes` | 35 | 0 | **0** |
| `emitted_instructions` | 4,902,066 | 6,504,638 | **6,504,638** |
| `unsupported_total` | 898,560 | 1,011,928 | **1,011,928** |
| 生成 C（書かず、`c_bytes` の計数） | 432,619,203 B | 565,582,720 B | **565,582,720 B**（6,698,311 行） |

- 変更前のバイナリでの再測定は、関数 25,401、命令 4,855,751、`out_of_text_flow` 35、`out_of_text_calls` 2,429、`bad_nodes` 35、`unsupported_total` 898,560、生成 C 432,619,203 B で、調査文書の値と一致した。
- 変更後の実在の DLL の統計は、merged コピー（`lift/out-ipp/merged-text.dll`。同じ手順で再測定）と**全項目が一致**した。`fe-lift` の所要時間は約 5.6 s、最大 RSS 約 728 MB（`/usr/bin/time -v`。出力は `/dev/null`）。
- `fe-lift lift --pdata` は補正 thunk（決定 0051）を既定で seed に含める（`--no-adjustors` で外す）。DLL の補正 thunk は 3 個で、`fe-decode closure --pdata`（`--adjustors` なし）は関数 32,159・命令 6,458,197・`seeds_code` 31,254、`--pdata --adjustors` は 32,162・6,458,203・31,257（差は thunk 3 個 = 関数 3、命令 6）。上の表の 32,162 / 6,458,203 は thunk を含む値で、merged コピーの値（同じ手順）と同じ。

### DLL の reloc seed

| 項目 | 値 |
|---|---|
| DIR64 スロット | 15,602（調査文書 §2.3 と一致） |
| 読めないスロット | 0 |
| 実行範囲内を指す distinct な値 | 10,481 |
| うち `.pdata` 開始（seed） | **7,731** |
| うち `.pdata` 開始でない値（種にしない。数えただけ） | **2,750** |
| `--pdata` に reloc seed を足した閉包 | `fe-lift lift --pdata --seeds reloc-seeds.txt`: 関数 32,162、命令 6,458,203、`unsupported_total` 1,011,928（**増えない**。7,731 個は `--pdata` の seed にすでに含まれる）。`fe-decode closure --pdata --reloc-seeds` も `--pdata` のみ（32,159 / 6,458,197）と同じ |
| reloc seed だけを種にした閉包（`--pdata` なし） | 関数 16,304、命令 5,077,318、`unsupported_total` 754,350、`bad_nodes` 0 |

- **reloc seed を `--pdata` に足しても、この DLL の閉包は変わらない**。このオプションの効果が出るのは `--pdata` を使わない（調査文書 §5.5 の `--seeds` で系統を絞る）ときだけ。表の添字に入る関数を種にするための道具として入れた。
- 調査文書 §5.3 の「関数でない表エントリ 507 件」と、今回の「関数開始でない値 2,750」は**別の数え方**（507 は表エントリの位置。2,750 は DIR64 スロット全体の実行範囲内の distinct な値）。507 件との対応は未確認。`0x1800018a0`（調査文書が「多くの表に共通」とした値）は 2,750 の中にある（`reloc-not-function.txt` に 1 行）。
- 参考（採っていない）: 2,750 を全部 seed にすると、閉包は関数 34,844、命令 6,572,807、`unsupported_total` 1,048,471、`bad_nodes` 0（`--pdata --seeds reloc-not-function.txt`）。これらは `.pdata` に無く、関数として入口かどうかは確認していないので、採らない（指示と同じ方針）。

## テスト

- 新規 `lift/crates/fe-decode/tests/multisec.rs`（7 件。合成 PE。`.text`、`IPPCODE`、CNT_CODE だけの `NOEXEC`、`.pdata`、`.reloc`）: セクション特性で決まること（名前ではない）、範囲が実行セクションごと・VA 昇順であること、2 つ目の実行セクションの関数が閉包に入り、実行属性の無いセクション（CNT_CODE のみ、または `.text` という名前でも MEM_EXECUTE なし）が入らないこと、実行セクションが無いとエラー、`pdata_seeds` が実行範囲内の開始だけであること、reloc seed が「`.pdata` 開始と一致するもの」だけを加えること（一致しない値は seed にならず `not_function` に数えられる。関数にもならない）、再配置ディレクトリが無いときは seed 0。
- 既存テストの表明は弱めていない（`tests/adjustor.rs` は `Text { va, bytes }` を `Text::single(va, bytes)` に置き換えただけ）。
- `cargo test --workspace --no-fail-fast`（`FLENGINE_PE` を指定、`CARGO_TARGET_DIR=lift/out-w3/target`）: **失敗 0、通過 1,003 件、`Running` 83 個 + Doc-tests 3 個 = 結果行 86 個**。数の上では基準（996 件）+ 本件の新規 7 件 = 1,003 だが、実行時点の作業ツリーには他の subagent の未コミットの変更（`sse_legacy.rs`、`tests/sse_legacy.rs` など）もあったので、差分の内訳が全部本件の 7 件だと確かめてはいない（バイナリ数の数え方も基準と同じか未確認）。落ちたものは無く、切り分けは不要だった。ログは `lift/out-w3/test-workspace.log`。

## 確認できていないこと

- DLL を `fe-lift` の `--pdata` で lift した生成 C を、コンパイルしても実行してもいない（統計だけ。`/dev/null` に出力）。
- `fe-lift` の CLI に `--reloc-seeds` を足していない（上記）。
- 実行属性を持つが `.text` と同種ではないセクションを持つ他の PE は、FLEngine と DLL の 2 つしか確認していない。MEM_EXECUTE のデータ領域（あるとしたら）は命令として読まれ、誤デコードや範囲内の誤った関数になりうる。この 2 つの DLL では、MEM_EXECUTE を持つのはコードのセクションだけ（`pe-info` の flags の読み）。
- 507 件の「関数でない表エントリ」の内訳と、2,750 個の「関数開始でない値」の内訳（表エントリ、`.text` 内の 456 件のスロット、その他）。調査文書 §10 の未確認が残る。
- DLL の `.pdata` に無い入口（`0x1800018a0` など）を、実行時の `fle_dispatch` が引くかは、実行して確かめていない。引かれると `fle_dispatch_miss` で止まる（調査文書 §5.5 の読み）。
- ImageBase 0x180000000 のままの生成 C は、`fle_addr` の上位 32 ビット検査で止まる VA を 117 万か所持つ（調査文書 §2.3 の実測）。今回はリベースを実装していない（W1）。
