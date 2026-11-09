# 0031: pdata の関数開始を閉包の seed にする

状態: 作業者（サブエージェント）が決定。依頼者（PM）の判断は含まない。測定は `docs/investigations/pdata-seeds.md`、実行は `docs/investigations/pdata-seeds-run.md`。

## 背景

ネイティブ harness が 0x410320（`call [rax-0x30]`、vtable 風スロットの値 0x40f960）で `fle_dispatch_miss` した。0x40f960 は DLL 内の実コードだが閉包に入っておらず、`fle_dispatch_table` に無い。間接 call の呼び先を 1 件ずつ seed に足す方式は、vtable の要素が多数あるため終わらない。関数開始を網羅的に seed にする。

## 決定

1. **seed は `.pdata`（例外ディレクトリ、データディレクトリ 3）の RUNTIME_FUNCTION の関数開始とする。チェーンされた子エントリは除く。**
   - 根拠（Microsoft Learn「x64 exception handling」、https://learn.microsoft.com/en-us/cpp/build/exception-handling-x64）: `UNW_FLAG_CHAININFO` の UNWIND_INFO は primary ではなく、その中身は以前の RUNTIME_FUNCTION で、チェーンの終端（CHAININFO が 0）が「実際の手続きの入口を指す」primary。チェーン子は親の一部。
   - 実装: 各エントリのチェーンを辿り、primary の BeginAddress を開始とする。昇順、重複なし。チェーン長 64 超（循環を含む）は `LimitExceeded`。
   - この DLL にはチェーンが 0 件なので、57,816 エントリすべてが関数開始（チェーンの処理は合成 PE のテストで検証。実 DLL では未検証）。
2. **既定の閉包（entry と export の 3 seed）は変えない。** pdata seed は明示的な opt-in。
   - `fe_decode::pdata_seeds(pe)`（`.text` 内の開始 VA、昇順）と `fe_decode::analyze_with_pdata(pe)` を追加。既存の `analyze`、`analyze_with_seeds` は変更なし。
   - CLI: `fe-pe pe-pdata-seeds FILE`（1 行 1 件の `0x...`。既存の `seeds-table-895.txt` / `seeds-896.txt` と同じ形式）、`fe-pe pe-pdata-info FILE`、`fe-decode closure FILE --pdata`、`fe-lift lift FILE OUT.c [--pdata] [--seeds SEEDS.txt]`（`--seeds` は 1 行 1 件の hex VA のファイル。`--pdata` と併用可）。引数なしの `fe-lift lift FILE OUT.c` は従来どおり（既定の seed）。
   - `fe-lift lift ... --pdata` の出力は、`generate_with_seeds` に seeds-pdata.txt を渡した出力とバイト単位で同一（`cmp` で確認）。
3. **all-pdata の生成 C は 1 つの翻訳単位では clang が作れないので、関数境界で分割してビルドする。** `lift/out-pdata/split_c.py`（1 ファイル 3,700 関数、16 分割）と `build-parts.sh`（並列 clang）。生成器（fe-lift）自体には分割機能が無く、今回も足していない（範囲外。分割は out-pdata の外部スクリプト）。フラグは既存の `build.sh` と同じ（`-O0 -g -fno-builtin -ffp-contract=off -fPIE -finstrument-functions`）。
4. **harness の trace（`out-trace/trace.c`）は変えず、out-pdata に複製して `SEEN_SIZE` を 16384 → 262144 にして使う。** 理由: trace の関数集合（`lifted`、`seen`）は固定長 16384 の開番地ハッシュで、表の関数が 58,811 件あると `lifted` の構築が満杯のハッシュで無限ループし、最初の関数で 120 秒（`HARNESS_MAX_SECONDS`）止まった（実測、`run-pdata.txt` 1 回目の `limit: wall time 120s`、関数実行数 0）。`lift/harness/trace.c` と `lift/out-trace/trace.c` は触っていない。

## 検討した代替

- pdata の `EndAddress` を使った範囲の seed（範囲内の全命令）: 採用しない。範囲内に jump table（データ）が 5 件あり（pdata-seeds.md §5）、線形走査はデータを命令として読む。再帰降下なら 0 件。
- 関数開始に pdata 外のサンクを足す: 足さない。閉包は直接 call 先を辿るので、pdata 外の `jmp [IAT]` サンク 879 件は、それを呼ぶ関数から閉包に入る（896 の閉包の 463 件、all-pdata の閉包の 879 件。すべて閉包に入った）。

## 確認できていないこと

- pdata に無いリーフ関数で、どこからも直接 call されず間接 call の先にだけなるもの（閉包に入らない）の有無。pdata に無い開始の件数は閉包の範囲で数えた限り 879 件でサンクだけだったが、閉包の外にあるリーフ関数は数えていない。
- チェーンされたエントリを持つ DLL での動作（この DLL では 0 件）。
- 実 Windows との挙動比較。
