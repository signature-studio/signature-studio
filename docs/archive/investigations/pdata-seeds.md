# .pdata（例外ディレクトリ）の実測と、関数開始 seed としての性質

日付: 2026-10-20。対象 `binaries/win64/FLEngine_x64.dll`（読み取りのみ）。決定は `docs/decisions/0031-pdata-seeds.md`、実行記録は `docs/investigations/pdata-seeds-run.md`。生成物はすべて `lift/out-pdata/`（`tmp/` は TMPDIR）。

「実測」はツールの出力、「推測」は未検証の解釈。

## 1. 形式（Microsoft Learn「x64 exception handling」で確認、取得 2026-10-20）

URL: https://learn.microsoft.com/en-us/cpp/build/exception-handling-x64?view=msvc-170

- `RUNTIME_FUNCTION` は ULONG 3 個（関数開始、関数終了、UNWIND_INFO のアドレス）。すべてイメージ相対（RVA）。DWORD 境界。エントリは**ソートされて** `.pdata` に置かれる（ページの文面）。
- `UNWIND_INFO`: 先頭バイトは Version（下位 3 bit）と Flags（上位 5 bit）、次に SizeOfProlog、CountOfCodes、FrameRegister/FrameOffset（各 4 bit）。続く unwind code 配列は **2 バイト × 偶数個**（`CountOfCodes` が奇数なら 1 個の未使用スロットを足す）。その後ろが可変部。
- Flags: `UNW_FLAG_EHANDLER` 0x01、`UNW_FLAG_UHANDLER` 0x02、`UNW_FLAG_CHAININFO` 0x04。`CHAININFO` が立つときは EHANDLER/UHANDLER は 0 でなければならない（ページの文面）。
- 可変部: EHANDLER か UHANDLER なら ULONG の handler RVA（その後に言語固有データ）。**CHAININFO なら親の `RUNTIME_FUNCTION`（ULONG 3 個）**。
- チェーンの意味（ページの文面）: 「この unwind info は手続きの primary ではない。チェーンされた unwind info の中身は以前の `RUNTIME_FUNCTION` エントリである」。用途は 2 つ（非連続のコード片、揮発レジスタの保存のグループ化）。チェーンはさらにチェーンしてよく（multiple shrink-wrapping）、最終的に `CHAININFO` が 0 の項目に着く。それが primary で「実際の手続きの入口を指す」。
- したがって: **チェーンされた子のエントリは別の関数ではなく、親（primary）の一部**。関数開始は primary の BeginAddress。
- ページに x64 の unwind data アドレスの下位ビットの意味の記載は無い（IA64/ARM の別の規約）。fe-pe は下位ビットが立つ UNWIND_INFO アドレスを `Malformed` にしている（この DLL では 57,816 件すべて下位ビット 0）。

## 2. fe-pe の実装（`lift/crates/fe-pe/src/lib.rs`、既存コードは編集せず追記のみ）

- `dir::EXCEPTION`（= 3）。
- `Pe::runtime_functions()`: 配列を読む（ディレクトリサイズが 12 の倍数でなければ `Malformed`、4M 件超は `LimitExceeded`、範囲外は `Truncated`）。`RuntimeFunction { begin_rva, end_rva, unwind_rva }`。`end_rva` は排他的。
- `Pe::unwind_info(rva)`: `UnwindInfo { version, flags, size_of_prolog, count_of_codes, frame_register, frame_offset, handler_rva, chained }`。unwind code そのものは復号しない。
- `Pe::function_table()`: 各エントリのチェーンを辿り `FunctionEntry { function, parent, primary }`。チェーン長 64 超（循環を含む）は `LimitExceeded`。`is_chained()`。
- `Pe::function_starts()`（RVA）、`Pe::function_start_vas()`（VA）: 各エントリの primary の `begin_rva` を昇順・重複なしで返す（チェーン子は自分の開始でなく primary の開始に寄与）。
- CLI: `fe-pe pe-pdata-info FILE`（件数）、`fe-pe pe-pdata-seeds FILE`（VA を 1 行 1 件、`0x...`）。
- テスト: `lift/crates/fe-pe/tests/pdata.rs` 18 件（合成 PE 15 件: 順序、空、サイズ異常、範囲外、handler の位置（奇数個の code のパディング）、チェーン、2 段チェーン、循環、重複、切り詰めで panic しない。実 DLL 3 件）。`cargo test --offline -p fe-pe`: pdata 18、real_dll 9、resources 9、synthetic 19、すべて通過。

## 3. 実 DLL の実測

独立の Python パーサ（`lift/out-pdata/pdata_probe.py`）と fe-pe の両方で測り、一致した（開始 VA の一覧は `cmp` で同一）。

| 項目 | 値 |
|---|---|
| 例外ディレクトリ | RVA 0x1607000、サイズ 0xa9620（= `.pdata` セクション 0x1607000、仮想サイズ 0xa9620） |
| エントリ数 | **57,816**（0xa9620 / 12、余り 0） |
| BeginAddress が `.text`（RVA 0x1000〜0xfa44b0）内 | 57,816（**100%**）。BeginAddress/EndAddress の両方が内 57,816 |
| `.text` 外 | 0 |
| BeginAddress の重複 | 0 |
| 厳密に昇順 | **はい**（ソートの記述どおり） |
| End <= Begin（空・逆転） | 0 |
| 次のエントリとの重なり（End > 次の Begin） | 0 |
| UNWIND_INFO アドレスの下位ビット | 全件 0。distinct 57,816（エントリごとに別の UNWIND_INFO） |
| UNWIND_INFO の Version | 全件 1 |
| Flags 別 | 0（handler なし）46,979、2（UHANDLER）8,021、3（EHANDLER\|UHANDLER）2,816、**4（CHAININFO）0** |
| **チェーンされたエントリ** | **0 件** |
| チェーンの深さ > 1 | 0 |
| 関数開始（チェーン子を除く） | **57,816**（`.text` 内 57,816） |
| 最初のエントリ | begin 0x7380（VA 0x407380）、end 0x73ba |
| 最後のエントリ | begin 0xf768a0（VA 0x13768a0）、end 0xf768ae |
| 線形走査で最後の命令が EndAddress ちょうどに終わる範囲 | 57,811（残り 5 は §5 の jump table） |

- この DLL にはチェーンが無いので、チェーン処理は合成 PE のテストでのみ検証されている（実 DLL では未検証）。
- pdata は「スタックを使う、または別の関数を呼ぶ関数（非リーフ）」だけを持つ（ページの文面）。リーフ関数（スタックを使わず呼び出しもない）は pdata に無い。この DLL でどれだけあるかは §4 の閉包との差から見える。

## 4. 閉包との関係（実測。`lift/out-pdata/tool` の `closure`、`compare`）

seed 集合は 3 つ。

- 既定: entry と export の 3 件。
- 896: 既存の 896 件（`lift/out-pdata/seeds-896.txt`、`out-a2` の複製）。
- all-pdata: pdata の関数開始 57,816 件（`lift/out-pdata/seeds-pdata.txt`）。entry/export は常に加わる。

| 項目 | 既定（3） | 896 | all-pdata |
|---|---|---|---|
| seeds_code | 3 | 899 | 57,816（entry 等は pdata に含まれ重複） |
| 関数 | 6,969 | 8,427 | **58,695** |
| 基本ブロック | 67,399 | 73,234 | 345,466 |
| 命令 | 460,479 | 497,367 | **2,523,194** |
| decode 失敗 | 0 | 0 | **0** |
| out_of_text_flow | 0 | 0 | 0 |
| 直接 call | — | 43,016 | 226,117 |
| 条件分岐 | — | 34,559 | 153,525 |
| IAT 経由 jmp | — | 445 | 822 |
| 間接 call（IAT 以外、未解決を含む） | — | 6,901 | 40,747 |

- 旧実装の実績「all-pdata で約 58,000 関数 / 250 万命令」と一致する（58,695 / 2,523,194）。
- **pdata の閉包は 896 の閉包を完全に含む**: 896 の関数 8,427 のうち all-pdata の閉包に無いもの 0、896 の命令のうち all-pdata の閉包に無いもの 0。既定の閉包に対しても同様（テスト `pdata_closure_contains_the_default_closure_and_the_896_seed_closure`）。
- **pdata の開始でない「既存の関数」は 463 件**（896 の閉包の関数のうち pdata 開始でないもの）。**463 件すべてが `ff 25 <rel32>` の 1 命令（`jmp [rip+disp]`、IAT 経由の import サンク）で、どの pdata 範囲にも含まれない**。VA の範囲は 0x406e70〜（`.text` の先頭のサンク領域）。
- all-pdata の閉包の関数で pdata 開始でないものは 879 件。内訳は 463（上と同じ）と、896 には無かった 416 件。**この 416 件もすべて `ff 25` のサンク**（`.text` 内の VA 0x406e60〜0x1375090、どの pdata 範囲にも含まれない）。つまり pdata の外の関数はサンクだけ（879 件すべてが `jmp [IAT]`）。
- pdata 開始 57,816 件はすべて閉包の関数（テスト `pdata_closure_has_no_decode_failures_and_contains_every_start`）。
- 閉包の命令のうち pdata 範囲の外にあるもの: 閉包の命令 2,523,194 のうち pdata 範囲内は 2,521,804、外は **1,390**。内訳（生成 C のラベルと pdata 範囲の突き合わせ、実測）: ①`ff 25` のサンク 879 命令（上の 879 関数）、②**511 命令**は、`ff 25` ではなく、pdata の範囲の外にあるコード。②は 58 個の閉包の関数（すべて pdata 開始の関数、VA 0x4071e0、0x407320、0x423070〜0x425a80、0x439190、0x4692d0〜0x4693d0、0x5933f0〜0x593990、0x5dc640〜0x5dcb90、0x5e1060、0x742000〜0x742040、0x80fa10、0x80fa30 など）に属し、VA は 0x407170〜0x80fa28。閉包の定義が「命令は最初に到達した関数に属する」ためで、直接 call でなく jmp で到達する pdata 外のコードと考えられる（推測。個々の命令列は確認していない）。これらは pdata 範囲外でも閉包に入っており、decode 失敗は 0。

## 5. decode の確かさ（実測）

- 再帰降下（閉包）の decode 失敗: **0 件**（all-pdata、`out_of_text_flow` も 0）。期待どおり。
- 参考（閉包とは別の確認。`pdata-seeds linear`）: 各 pdata 範囲 [Begin, End) を先頭から線形に decode した場合、57,811 範囲は命令境界がちょうど End に一致し、**5 範囲で decode 不能**（`invalid`）。
  - 0x40d268（範囲 0x40d210-0x40d31b）: バイトが `c7 d2 40 00 00 00 00 00 bd d2 40 00 ...`。8 バイトごとに 0x40d2xx を指す値の並びで、その直前の命令 `jmp [r10+rdx*8+0x40]`（0x40d253）の jump table。
  - 0x427011（範囲 0x426fe0-0x42d072）: `06 07 08 09 ... 14 11` の単調なバイト列で、index table。
  - 0x5e6d46、0x5eb35b、0x5f1664: `ff ff 6a f6 ...`、`e8 ff ff dd e8 ...`、`37 fe ff ff 5c fe ff ff ...` の 32 ビットの負の相対値の並びで、switch の相対 jump table。
  - いずれも関数内のデータ（jump table）で、関数開始が誤っているのではない（推測を含む: 5 件のバイトを手で確認した範囲での解釈。ここ以外の jump table は線形走査で偶然 decode できたため無害とは限らない）。
  - 閉包の命令（pdata 範囲内 2,521,804）のうち、線形走査の命令境界に乗っていないものは **0**。線形走査の命令数 2,566,540 は閉包の 2,521,804 より 44,736 多い（データ、および到達されない命令を含む。内訳は未確認）。

## 6. 再現

```
lift/target/release/fe-pe pe-pdata-info binaries/win64/FLEngine_x64.dll
lift/target/release/fe-pe pe-pdata-seeds binaries/win64/FLEngine_x64.dll > seeds-pdata.txt
lift/target/release/fe-decode closure binaries/win64/FLEngine_x64.dll --pdata
lift/out-pdata/target-tool/release/pdata-seeds compare FILE seeds-896.txt seeds-pdata.txt   # compare.txt
lift/out-pdata/target-tool/release/pdata-seeds linear FILE seeds-pdata.txt                  # linear.txt
```
