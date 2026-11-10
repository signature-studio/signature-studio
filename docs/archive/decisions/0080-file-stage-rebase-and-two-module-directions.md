# 0080: 2 つ目のモジュールの置き方（基底 0x2000000、ファイル段階のリベース、状態共有ほか）の方向の決定と、リベースツール `fe-pe rebase`（段階 0）

状態: PM の方針決定（2026-10-23）を、作業者（サブエージェント）が記録した。前半（「PM の決定」）は**方向の決定**で、実装はこの作業の範囲外（段階 1 以降）。後半（「リベースツールの仕様」以降）は段階 0 の実装と測定の記録。出典は `docs/investigations/two-module-design.md`（以下「設計文書」。節番号はその文書のもの）。決定 0073（DLL を 2 つ目のモジュールとして lift する）と 0074（実行範囲と reloc seed）の続き。この記録は設計文書に無い主張を足さない。

## PM の決定

| # | 決定 | 根拠（設計文書） | この作業での実装 |
|---|---|---|---|
| D1 | リベース先の基底は **0x2000000** | §3.2 の P1、§6 D1。FLEngine の終端 0x1cc0000 の直後で、`GLOBAL_BASE` 0x10000000 より下（DLL の終端は 0x461f000）。64 K の倍数（PE Format が ImageBase に課す条件。§3 冒頭） | ツールの引数として渡せる（既定値は持たない） |
| D2 | リベースは**ファイル段階**（生成前に `.reloc` の DIR64 を適用した別ファイルを作る。ロード時リベースは採らない） | §3.4 の (A)、§6 D2 | **実装した**（下） |
| D3 | 状態を共有する（`R`/`FL`/`X` と dispatch 表を 1 つ） | §4.3 の S、§6 D3 | なし（段階 2） |
| D5 | モジュールハンドルの値は基底（H1） | §5.3 の H1、§6 D5。DllMain の `hinstDLL` が基底アドレスであること（設計文書 §5.3 が引く Learn の DllMain） | なし（段階 3） |
| D7 | 当面は縮小 DLL（SSE2 系統の種）で進め、全部入りは後 | §4.6、§6 D7。全部入りは wasm として実行できない（`local count too large`。13 関数が 1 関数 50,000 locals の上限を超える）。全部入りは AVX の方針（W8）の決定後に再検討 | なし |
| D8 | CPUID は現行のまま | §6 D8（変えるなら XCR0 の値の決定が先。決定 0047）。「選択値が 0 になる」仮説は未確認で、最初の実行で確かめる | なし |
| D9 | スレッドは現行のまま。DLL が `DLL_THREAD_*` を受ける経路は作らない | §6 D9（wasm は単一スレッド前提。決定 0017、0045。DLL は `CreateThread` を持たない（実測）が `ippSetNumThreads` がある） | なし |

**D1 の値について（正直な記述）**: 0x2000000 は仕様や実機から導かれた値ではない。「FLEngine の終端（0x1cc0000）の直後の 64 K 整列の丸い値で、既知の使用領域（FLEngine のイメージ、`GLOBAL_BASE` 0x10000000 以上の wasm のデータ・スタック・ヒープ・アリーナ、0x7F990000 以上のトークン、0xCB000000 の番兵）と重ならない」という**衝突しない範囲の選択**で、他に 0x8000000 などでも同じ条件を満たす（設計文書 §3.2 の P2）。創作の値ではないが、この値でなければならない理由もない。DLL の実際の ImageBase は 0x180000000 で、この値は実機の ImageBase と違う。

**保留**（この記録では決めない。設計文書 §6 のとおり）:
- D4: DLL の CRT が引く `api-ms-win-*`（実測: `api-ms-win-core-synch-l1-2-0`、`api-ms-win-core-fibers-l1-1-1`）と `kernel32` への `LoadLibraryExW` の答え。DLL が `GetProcAddress` に渡す API 名を確認してから決める。**0081 で決定**（仮想モジュールとして「ある」と答える。仮の決定。実装は W6 の後）。
- D6: DLL の仮想パス（`GetModuleFileNameW`）。値は創作になるため PM の値が要る。**0082 で決定**（今は決めない。実在モジュールに対する `GetModuleFileNameW` は名前付き停止）。
- D10: メモリ（アリーナ 512 MiB、wasm の最大 2 GiB に対する DllMain 完走後の使用量）。段階 4 で測ってから決める。
- D11: `FreeLibrary` したときの扱い。段階 5 で決める。**0082 で決定**（名前付き停止のまま）。

## リベースツールの仕様（段階 0。`fe-pe` の `rebase` モジュールと `fe-pe rebase`）

### 操作

PE のバイト列と新しい ImageBase を受け取り、新しいバイト列と報告を返す（`fe_pe::rebase::rebase(data, new_base)`）。

1. `.reloc`（データディレクトリ 5）の各ブロック（4 KiB ページごと）の各エントリを見る。
2. **IMAGE_REL_BASED_DIR64（10）**: その位置の 8 バイトに（新基底 − 旧基底）を加える。加算は 2^64 を法とする巻き戻り（`wrapping_add`）で、PE Format は「差を 64 ビットのフィールドに適用する」と書くだけで巻き戻りは規定していない。この点は**実装上の選択**。実 DLL で新基底が旧基底より小さい（差が負）ことを使う（0x180000000 → 0x2000000）。
3. **IMAGE_REL_BASED_ABSOLUTE（0）**: 何もしない（PE Format: 「skipped。ブロックのパディングに使える」）。数えて報告する。
4. **上記以外の種別（HIGHLOW(3)、HIGH、LOW、HIGHADJ など）はエラー**（`UnsupportedType { ty, rva }`）。黙って無視しない。理由: x64 の DLL に現れるのは DIR64 と ABSOLUTE のはずで、他の種別があるなら「差を適用すべき位置を見落とす」ことになる。既存の `Pe::dir64_relocation_vas`（決定 0051）は DIR64 以外を読み飛ばす作りで、**それは変えていない**。`rebase` は別に走査して、読み飛ばさない。
5. ヘッダの ImageBase（PE32+ のオプションヘッダのオフセット 24）を新基底に書き換える。
6. 上記以外のバイトは 1 つも変えない（テスト `nothing_else_changes`。CheckSum、DllCharacteristics、Characteristics、セクション、`.reloc` の中身を含む）。実 DLL の出力と元の差は 31,206 バイト（`cmp -l`）で、15,602 スロット × 8 バイト + ImageBase 8 バイトの上限（124,824）の内側。

### エラーにするもの（設計文書の実測を踏まえた扱い）

設計文書 §3.1 は、実 DLL の 15,602 スロットがすべて `[ImageBase, ImageBase + SizeOfImage)` を指し、ファイルの裏付けのない位置が 0 件だったと実測している。そのため、裏付けのない位置は**エラー**にした（黙って飛ばさない）。

- DIR64 の位置の 8 バイトが、1 つのセクションのファイル上のデータ（SizeOfRawData の内側）に収まらない（`NotFileBacked`）。`.bss` 側（VirtualSize が SizeOfRawData を超える部分）、セクションの外、境界をまたぐ 8 バイトを含む。
- DIR64 の位置がヘッダの中（`InHeaders`）。
- 2 つの DIR64 のバイト範囲が重なる（`Overlap`。同じ位置の重複を含む）。
- ブロックの形が不正（サイズが 8 未満か奇数、ディレクトリの外へ出る、最後のブロックの後に 8 バイト未満の余り）（`Malformed`）。余りをエラーにするのは `dir64_relocation_vas` と異なる（あちらは余りを無視する）。
- 新基底が 64 K の倍数でない（PE Format: ImageBase は 64 K の倍数）、または 新基底 + SizeOfImage が 2^64 に達する（`BadNewBase`）。
- 基底を変えるのに `.reloc` が無い（ディレクトリなし）か、Characteristics に IMAGE_FILE_RELOCS_STRIPPED（0x0001）がある（`NoRelocations`）。絶対アドレスの位置が分からないため。
- 新基底が旧基底と同じなら、出力は入力と同じバイト列（ただし上の位置の検査は行う。検査の結果はエラーでありうる）。`.reloc` が無くてもこの場合は成功する。

上の 4 番目以降のエラー（`Overlap`、`InHeaders`、余りバイト、64 K、桁あふれ、`NoRelocations`）は、依頼文にはなかった検査で、作業者の判断で足した。実 DLL と FLEngine では 1 件も出ない（出力が通る）。

### DllCharacteristics、Characteristics、`.reloc`

- **DllCharacteristics は変更しない**（実 DLL は 0x0160 = HIGH_ENTROPY_VA、DYNAMIC_BASE、NX_COMPAT のまま。出力でも 0x0160）。理由: このツールの出力は Windows のローダに渡さず、生成器（fe-pe → fe-decode → fe-lift）が読むだけ。DYNAMIC_BASE（PE Format: 「DLL can be relocated at load time」）を外す根拠も、外して得られるものもない。変えるなら理由が要るが、ここでは無い。
- Characteristics（0x2022）も変更しない。
- **`.reloc` は残す**（中身も変えない）。出力のあとの `fe-pe` の読み取りは、元と同じ位置を DIR64 として返す（テストで確認）。

### CheckSum

PE Format（Microsoft Learn、2026-10-23 取得）の CheckSum の記述: 「IMAGHELP.DLL にアルゴリズムがある。ロード時に検証されるのは、すべてのドライバ、ブート時にロードされる DLL、重要な Windows プロセスにロードされる DLL」。

- **据え置き**（コピー。変えない）。`fe-pe rebase` は値を報告に出す。
- 実測: `dsp_ippv2_x64.dll` の CheckSum は **0**（計算されていない）。FLEngine_x64.dll は 0x153cf0c。
- 理由: 出力は Windows のローダを通らず、仕様が検証を求める 3 種（ドライバ、ブート時の DLL、重要プロセスの DLL）のどれでもない。DLL の値が 0 なので、リベースで「合わなくなる」値もない。
- **未確認**: FLEngine のように 0 でない CheckSum を持つファイルをリベースした出力は、IMAGEHLP のアルゴリズムで計算した値と一致しなくなる（作ったファイルの内容が変わるため）。アルゴリズムを実装していないので、実際に何が一致しなくなるかは確かめていない。ローダを使わない限り影響は無いはずだが、これは仕様の記述からの推論で、検証していない。
- 証明書テーブル（データディレクトリ 4。署名）は扱わない。実 DLL では 0（実測）。署名のあるファイルをリベースすると署名は無効になるはずだが、確認していない。

### CLI

```
fe-pe rebase FILE NEWBASE OUT
```

- NEWBASE は `0x` 付きの 16 進（`0x` が無いとエラー）。既存の `pe-init-table FILE [OUT.tsv]` の流儀に合わせ、OUT は新規ファイルのみ（存在すればエラー。`create_new`）。
- 標準出力: `old_base`、`new_base`、`delta`（2^64 を法とした値）、`reloc_blocks`、`dir64`、`absolute`、`checksum`、`out`（バイト数）。
- 既存のサブコマンドの引数と出力は変えていない（`main` の先頭で `rebase` だけ別に分岐）。

## 検証（実 DLL: `lift/out-ext/dsp_ippv2_x64.dll`、SHA-256 497dd4f7…c352。作業物は `lift/out-p0/work/`）

| 項目 | 結果 |
|---|---|
| (f) `.reloc` の件数 | ブロック 81、**DIR64 15,602、ABSOLUTE 22**（ツールの出力。設計文書 §3.1、§1 と一致） |
| (a) 決定的 | 0x2000000 へ 2 回リベースした `a1.dll` と `a2.dll` の SHA-256 は同じ: `4fd255027525f2a2e56a93fe1cc7f95442861f788179a23ac8b0401b8f4fefc5`（`cmp` でも一致） |
| (b) 恒等 | 基底 0x180000000（旧と同じ）の出力は元と全バイト一致（`cmp`、SHA-256 497dd4f7…c352） |
| (c) 往復 | 旧 → 0x2000000 → 旧 の出力は元と全バイト一致（`cmp`） |
| 上書き拒否 | OUT が存在すると `error: ...: File exists`、終了コード 1 |
| (d) fe-lift の統計 | 同じ `fe-lift lift FILE /dev/null --pdata` を元とリベース後（0x2000000）の両方に実行。出力 8 行目の `c_bytes` を除き**全行が同一**（`diff`）。**functions 32,648**、closure_instructions 6,464,799、emitted_instructions 6,511,234、bad_nodes 0、tail_calls 22,594、gotos 450,886、c_lines 6,707,823、unsupported_total 974,582。差は `c_bytes` のみ: 566,289,705 → 536,731,303（−5.2%。VA の即値の桁が短くなるため。設計文書 §3.4 の傾向と同じ） |
| (e) `0x18…` の即値 | 生成 C を書き出していないので**再測定していない**（任意。設計文書 §3.4 の実測: 1,135,257 → 0） |

functions は設計文書の 32,162 でなく 32,648。ワークツリーの他の変更（決定 0075、0079 の SSE と表の解決など）で閉包が変わったためで、元とリベース後を同じツールで測って比較した（依頼文の想定どおり）。

### 実 DLL の副次的な確認

- `.reloc` に載らないがイメージ範囲に入る 2 つの値（`.rdata` の 0x1ec7070 と 0x1ec7110。値 0x180000029。設計文書 §3.1）は、出力でも 0x180000029 のまま（変わらない）。定数かアドレスかは**未確認**のまま。
- 0x2000000 の出力でも、セクション、データディレクトリ、`function_starts`、export 446 件の RVA と名前は元と同じ。export の VA は 0x2000000 + RVA。DIR64 の各スロットの値は「元の値 − 0x180000000 + 0x2000000」になる（テスト `real_dsp_ippv2_structure_follows_the_new_base`）。

## 検証していないこと（未確認）

- リベース後の DLL から生成した C を、コンパイルも実行もしていない（統計だけ）。wasm と native での実行（DllMain の途中まで）は設計文書 §3.2 が P1 で行った測定で、今回は再現していない。
- `0x18…` 形の即値の再測定（上の (e)）。
- FLEngine 以外のファイルでの `.reloc` の実形（FLEngine と DLL の 2 つだけを使った）。
- CheckSum と署名（上記）。
- 他の基底（0x8000000 など）で `fe-lift` の統計が同一になること。設計文書 §3.2 では実測済みだが、今回は 0x2000000 だけを測った。

## テスト

- 新規 `lift/crates/fe-pe/tests/rebase.rs`（15 件）。合成 PE32+（`.text`、`.data`（`.bss` 側を含む）、`.reloc`）で、DIR64 の加算、ImageBase の書き換え、他のバイトが変わらないこと（CheckSum、DllCharacteristics、Characteristics、`.reloc`）、`.reloc` に載らない値が変わらないこと、同じ基底で恒等、往復と決定性（3 基底を経由する往復を含む）、DIR64 と ABSOLUTE 以外の種別（1〜9、11、15）がエラー、`.bss` 側・境界またぎ・セクション外・ヘッダ内の位置がエラー、重なりがエラー、不正ブロックがエラー、64 K と桁あふれ、`.reloc` なし・RELOCS_STRIPPED、PE でない入力。実 DLL（`$DSP_IPPV2_PE`、無ければ `lift/out-ext/dsp_ippv2_x64.dll`。無ければ SKIPPED で通る。環境変数があって無ければ失敗する既存の流儀）で (a)〜(c) と件数 15,602 / 22 と構造の保存、FLEngine（`$FLENGINE_PE`）で件数 278,531 / 993 と往復。(d) は fe-lift の統計を呼ぶため fe-pe のテストには入れず、上の表の手順で測った。
- 既存のテストの表明は弱めていない。既存ファイルの変更は `lib.rs`（`pub mod rebase;` の 1 行）と `main.rs`（`rebase` の追加）だけ。
- 作業中に 1 度、テスト自身の誤り（桁あふれの境界値）で落ちたので、テストの値を直した（実装は変えていない）。

### 全体テスト

`cargo test --workspace --no-fail-fast`（`FLENGINE_PE` 指定。ログ `lift/out-p0/test-workspace.log`）: 結果行 93（`Running` 90 + Doc-tests 3）、通過 **1,075**、失敗 **1**。基準（92 バイナリ・1,061 件）に対し、新規 `rebase.rs` の 1 バイナリ・15 件を足すと 93・1,076 で、失敗 1 を引いた 1,075 と合う。

失敗した 1 件は fe-pe と無関係で、他者の変更によるもの（読み）:

- `fe-lift` の `tests/sysinfo.rs` の `source_uses_no_host_processor_memory_time_or_file_state`。`fle_sysinfo.c`（runtime。ワークツリーで変更中）が `environ` を使うとして落ちる（`sysinfo.rs:953`）。
- 原因の読み: `fle_sysinfo.c:103` の `snprintf` の文字列リテラル「…this environment decides it」の `environment` が、テストの禁止語 `environ` に部分一致する（テストは `/* */` のコメントだけを除くので、文字列リテラルは対象に残る）。他の `environ` の出現（2、39、171 行）はコメント内。テストを読み、`fle_sysinfo.c` を検索して得た読みで、HEAD の状態でこのテストが通るかは確かめていない（git 操作をしていない）。
- 他者のファイルなので直していない。

### FLEngine の生成 C が変わらないこと

fe-pe を変更したあとのバイナリで `fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata`: 215,700,923 B、関数 59,297、命令 2,528,030、`bad_nodes` 0、SHA-256 **67827424301459e6f2da1ccb46a5fd081b7de9a38c64e1bca44ba9bf5d15584d**（基準と同一）。fe-pe の変更が既存の読み取りに影響しないことの確認（`rebase` は読み取り側を変えていない）。

## 撤回条件

- D1 の基底が、段階 2 以降の実測（wasm の配置、FLEngine のメモリ使用量。D10）で衝突すると分かったとき。その場合は設計文書 §3.2 の P2 などから選び直す（ツールは基底を引数にとるので、ツールの変更は要らない）。
- PM が方針を変えたとき。
