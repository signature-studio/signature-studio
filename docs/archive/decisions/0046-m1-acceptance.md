# 0046: M1（FLEngine_x64.dll の完全 lift）の受け入れ基準 A1〜A6

状態: 文書化。PM の以前の決定の要旨を、依頼者経由の依頼文で受け取り、リポジトリの事実で現状を採点した。PM の確認待ち。「達成」は、定義どおりに実測で確認できたものにだけ付ける。実測は 2026-10-21、HEAD 31d0134（34 コミット、2026-10-20 が 23、2026-10-21 が 11）で行った。

## 0. 経緯と出典の扱い

- 依頼者の最終目標は「FL Studio がブラウザ上で実物と変わらず動作する」こと。M1 はその最初の節目で、「FLEngine_x64.dll の完全 lift（x86-64 → C → wasm）」と定義する。
- 以前の PM の記録は、この基準を「ADR-0005: A1〜A6」と呼んでいた。**リポジトリに該当する文書は無い**。`docs/decisions/0005-mxcsr.md` は「STMXCSR と LDMXCSR の応答」の決定で、「A1〜A6」も「A3」も含まない（全文を読んで確認した）。PM が「旧 0007/0009/0010」と呼んだ決定も、リポジトリの 0007（extra seed）、0009（CPUID）、0010（符号拡張、IDIV）とは別物で、原文はリポジトリに無い。0029 の「文脈」と `unsupported-inventory-pdata.md` §0 も、同じ不一致を記録している。
- 本書は、依頼文が伝えた要旨を A1〜A6 として書き直したものである。要旨の原文（PM の会話記録）はリポジトリに無いので、要旨の正確さは PM が確認すること。
- 番号の注意: 0036 は 2 本ある（`0036-a5-oracle-v2.md` と `0036-indirect-jmp.md`）。参照するときはファイル名で書く。
- 0046 は未使用の番号だった。README には ADR の索引が無いので、README は変更していない。

### PM の以前の決定の要旨（依頼文による）

1. A2 を緩めない。生成した stub（`fle_unsupported`）は 0 にする。
2. x87 は f80 の厳密な実装にする。
3. A4 は DllMain → InitUnits の完走で判定し、緩めない。
4. fail-soft（止まらずに続行する）での前進は、進捗として認めない。
5. 外部モジュール（プラグイン、道具の DLL）は M2 の対象にする。
6. 未知の外部 API は fail-closed（名前付きで abort）にする。
7. 名前付きの既定値で完走させない。
8. **M1 完了 = A2 + A4 + A5 + A6**。A1 は前提、A3 は wasm 化の確認で、M1 完了の条件ではない。

リポジトリ内の裏付け: 方針 4、6、7 に当たる運用は 0016 の決定 2（「決まらないものは abort。黙って値を作らない」）、0029 の判断 2（外部 DLL は M2 の判断で、未知は止める）にある。ただし 0029 は「依頼文の指示として扱った」と書いており、PM の決定の原文ではない。方針 2 は 0033 と 0043 が実装した。

## 1. 判定の一覧

| 基準 | 内容 | 現在の状態 | 一言の根拠 |
|---|---|---|---|
| A1 | 素材の SHA-256 が台帳と一致 | **達成** | `sha256sum` が台帳の値と一致（§2） |
| A2a | 命令と制御フローの完全性 | **未達（部分）** | 命令 stub 43、解決不能 jmp 1 が残る |
| A2b | import 応答の完全性 | **未達** | 応答の無い IAT thunk が 557 |
| A3 | emcc でリンクし node で native と同じ停止点 | **達成（定義の範囲）** | 199 / 1,386 / 9 が一致（限界あり） |
| A4 | DllMain → InitUnits が 895 unit 完走 | **未達** | LCType 0x1c で停止。実行中の unit は index 17（895 件中） |
| A5 | native オラクルで bit-exact | **部分** | 抽出 1,000 関数で core 差 0。ただし対象は純計算 9,901 件の約 10%、比較不能 15.4% |
| A6 | 実測値を docs に記録しコミット、push | **部分** | 記録とコミットは済み。remote 未設定のため push は保留 |

M1 完了（A2 + A4 + A5 + A6）は未達。

## 2. A1 素材

| 項目 | 内容 |
|---|---|
| 判定コマンドと数値 | `sha256sum binaries/win64/FLEngine_x64.dll` が `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`、`stat -c %s` が 22,227,016 |
| 現在の状態 | **達成** |
| 根拠と出典 | 2026-10-21 に上記 2 コマンドを実行し、台帳 `docs/investigations/win64-extraction.md` §2.3 と一致した。台帳はインストーラ `flstudio_win64_26.1.7.5653.exe`（1,106,033,456 B、SHA-256 `04393e79…fc02`）から抽出した値を記す |
| 未確認・限界 | `binaries/` は `.gitignore` 対象で、実バイナリはリポジトリに無い（台帳の値だけが残る）。インストーラは今回再ハッシュしていない。抽出手順を最初から再実行してはいない |

## 3. A2 lift 完全性

判定対象は all-pdata の生成 C: `fe-lift lift binaries/win64/FLEngine_x64.dll OUT.c --pdata`。

A2 は 2 つに分ける。

- **A2a 命令と制御フローの完全性**: 生成 C に命令未実装の stub が 0、解決不能な間接 jmp が 0。
- **A2b import 応答の完全性**: 閉包の IAT thunk のうち、実行で踏まれるものが全て、仕様確認済みの応答を持つ。応答の無い API を「名前付き abort」で止めるだけでは A2b は満たさない（abort は fail-closed として許容するが、進捗とは数えない。方針 4、6）。

### 現状の数（実測。lift/out-doc/gen-0046.txt）

`lift/out-wasm2/target/release/fe-lift`（2026-10-21 03:38 ビルド）で再生成した。

| 項目 | 値 |
|---|---|
| 関数 / `c_bytes` | 58,695 / 215,539,555 |
| `unsupported_total` | **601**（生成 C の `fle_unsupported` 行も 601 行で一致） |
| 内訳: IAT 経由 jmp | 557（`Jmp: indirect through IAT slot`） |
| 内訳: 解決不能な間接 jmp | 1（`f_4126f0` の `jmp [rcx]`、0x412752。setjmp/longjmp 風。W3） |
| 内訳: 命令未実装 | **43**（Psubw 10、Pcmpgtd 8、Rcpps 8、Bswap 7、Test high-byte 7、And high-byte 1、Andnpd 1、Xgetbv 1） |
| x87（F 系）の stub | 0（f2xm1、fsincos も 0043 で実装） |
| 遅延 import の `jmp reg`（tail dispatch） | 57 occurrence（20 か所）。`reg_dispatch` 57、表 jmp 3、解決不能 0 |

経過: 2,991（`gen-pdata-b.txt`）→ 698（0036 indirect-jmp）→ 667（0033 x87）→ 603（0038）→ 601（0043）。出典: `indirect-jmp-run.md` §3、`x87-run.md` §4、`locale-100b-run.md`、`x87-trans-run.md` §6。

### 判定表

| 項目 | A2a 命令と制御フロー | A2b import 応答 |
|---|---|---|
| 判定コマンドと数値 | 生成 C の `unsupported_total` から IAT 経由 jmp を除いた数が 0（現在 44 = 命令 43 + jmp 1）。`grep -c fle_unsupported OUT.c` も同じ数を出す | 閉包の IAT thunk 557 のうち、実行で踏まれるものが 0 件の応答なし。応答は `imports_table.rs` の TABLE（201 行）と `runtime/fle_*.c` が持つ |
| 現在の状態 | **未達（部分）**。x87 と大半の命令、解決可能な間接 jmp は完了（stub 0）。ただし 44 か所が残る | **未達**。557 thunk、536 API、23 DLL に応答が無い |
| 根拠と出典 | 上の表（実測）。x87: `x87-trans-run.md`。間接 jmp: `indirect-jmp-run.md`（62 → 1） | 実測: user32 221 thunk（209 API）、gdi32 129（126）、kernel32 58（55）、comctl32 31（30）、shell32 24（22）、ole32 19、wininet 15、msacm32 14、winmm 13、crypt32 7、他。分類と実装単位 W0〜W12 は `unsupported-inventory-pdata.md` §2〜§6（推測を含む） |
| 未確認・限界 | 44 か所のうち、InitTable とエントリの直接閉包（U、6,297 関数）に入るのは Xgetbv などごく一部（`unsupported-inventory-pdata.md` §2.5、§4。生成 C が古い時点の測定で、再計測していない）。Xgetbv は CPUID ECX=0 で守られているとされる（0032、未検証） | 実行で踏まれる thunk の数は未測定（静的閉包は過大見積もり。`unsupported-inventory-pdata.md` §4.1）。遅延 import の 27 API（9 DLL）は仮想モジュール表に無い DLL で、M2 の判断が要る。「仕様確認済み」の判定には、仮の決定（A4 の表）が入る |

## 4. A3 wasm

| 項目 | 内容 |
|---|---|
| 判定コマンドと数値 | `bash lift/harness/wasm/build.sh` で runtime と生成 C を emcc でコンパイル、リンクし、`node lift/out-wasm2/bin-base/run_node.cjs binaries/win64/FLEngine_x64.dll` を実行。同じ runtime と生成 C の native（`build_native.sh`）と、停止点、関数実行数（distinct）、入口数、深さを比べる |
| 現在の状態 | **達成（上の定義の範囲）** |
| 根拠と出典 | `wasm-port-run.md` §1、§5。runtime 32/32、生成 C 16/16 が `emcc -c` に成功、リンク成功（`flengine.wasm` 115,721,618 B）。停止は `GetLocaleInfoW` LCType 0x1c 付近、関数 **199**、入口 **1,386**、深さ **9**。native との差は `calling entry … rsp=` の 1 行（スタックのアドレス）だけ。本書の作業で、既存の `flengine.wasm`（03:42 ビルド）と native バイナリを再実行し、`calling` 行を除いて全文一致することを再確認した（`lift/out-doc/node-rerun.txt`、`native-rerun.txt`）。native のワークスペース全テスト: 65 バイナリ、615 passed、0 failed（`lift/out-wasm2/test-ws.txt`） |
| 未確認・限界 | (1) 再実行は既存バイナリの再実行で、再ビルドはしていない。作業ツリーに他担当の未コミット変更がある（`lift/runtime/fle_runtime.h`、`fle_sse.h`）。(2) 停止点より先の実行は未確認。この経路で SSE/x87 の演算は 0 回で、wasm 上の演算の等価性の証拠にならない。(3) **fscale の native と wasm の不一致が未修正**: `fle_x87.c` の `long`（wasm32 で 32 bit）が原因で、probe の出力 18,822 行が不一致（`long` → `int64_t` で 0、`wasm-port-run.md` §7）。(4) ブラウザは未対応（`NODERAWFS` は node 限定）。(5) ページ保護の非強制、`fle_ldx` 系が上位ビット検査の外、アリーナ 512 MiB の上限（0045） |

**訂正（事実）**: 以前、A3 をネイティブ clang の確認だけで「達成」と記録していた。この記録の原文は PM の会話記録で、リポジトリには無い（`wasm-port-run.md` §6 も同様に記す）。emcc でのコンパイル、リンク、node 実行は、2026-10-21 の `wasm-port-run.md` が最初の実測である。

## 5. A4 起動

| 項目 | 内容 |
|---|---|
| 判定コマンドと数値 | native と wasm の harness を実行し、`returned from entry`（`lift/harness/wasm/main.c`）で終了コード 0。かつ、InitTable（構造体 0x13768b0、件数 942）の runnable 895 件が全て実行される（survey ok ≥ 895）。外部 API の応答は仕様確認済みのみ |
| 現在の状態 | **未達** |
| 根拠と出典 | 停止は `GetLocaleInfoW` LCType 0x1c（`LOCALE_INEGCURR`）、lcid 0x409。関数 199、入口 1,386、深さ 9（`locale-enus-run.md` §7、§8。本書の再実行も同じ）。スタックは f_423560 ← f_446370 ← f_446400 ← f_44bb70 ← f_455790 ← f_4120c0 ← … ← エントリ。本書の実測: DLL のバイト列から表を読むと、runnable は 895 件（proc≠0 かつ先頭 8 バイト≠0）。スタックの f_455790（0x455790）は表の **index 17** で、f_4120c0 のループが index 0〜16 を過ぎて index 17 の unit を実行中に止まった。入口ログの 256 件には index 0〜12 が出る（13〜16 は 256 件の後ろで、ループの順序からの読み） |
| 未確認・限界 | (1) `init_count` という名前の計数はリポジトリに無い。「survey」の測定ツールも無い（旧リポジトリの調査記録 `2026-10-19-old-repo-survey.md` は本リポジトリに無い。0035 の冒頭に記載）。unit 完走数を直接数える計数器は harness に未実装で、現在は入口ログ（先頭 256 件）の突き合わせだけ。A4 の判定には、unit ごとの実行回数を数える harness の追加が要る（課題）。(2) **仕様確認済みでない応答が既にある**（下表）。(3) 6 種の LCType（0xc、0x1c、0x1f、0x20、0x1003、0x79）は出典が食い違い、abort のまま。この先に `GetDateFormatW`、`EnumCalendarInfoW` などが続く見込み（`locale-enus-run.md` §5） |

### 仕様確認済みでない応答（仮の決定。A4 の「仕様確認済みのみ」を満たしていない）

| 決定 | 内容 | 根拠の性質 |
|---|---|---|
| 0040 | `VerifyVersionInfoW` の dwOSVersionInfoSize=0 を通す | 非公式（ReactOS、Wine）。実機 Windows 未確認 |
| 0041 | LCType 0x100b に "0" | 非公式（Wine ほか）。実機未確認 |
| 0042 | en-US の 51 LCType の値（0041 と合わせて 52） | 非公式（Wine 5.0 と現行、CLDR）。実機未確認。出典 2 つ以上の一致を基準にしたが、公式ではない |
| 0030 | `timeGetDevCaps` の wPeriodMax=1000000 | 記憶による値（非公式、確認していない）。環境の設定 |

これらは「名前付きの既定値」ではなく、非公式な出典の一致に基づく値である。方針 7 との整合は PM が判断する（実機または公式資料で確認するまで A4 の「仕様確認済み」に数えない、が本書の扱い）。

## 6. A5 bit-exact 差分

| 項目 | 内容 |
|---|---|
| 判定コマンドと数値 | `lift/harness/oracle/run2.sh`（v2）、現在の最新は `run5.sh`（`lift/out-oracle/results/run5/a5-summary.json`）。native オラクル（`lift/harness/oracle/`）と lift を同じ入力で走らせ、GPR、XMM、メモリ、EFLAGS、MXCSR、x87 を比較する |
| 現在の状態 | **部分** |
| 根拠と出典 | run5: 純計算 9,901 件から 1,000 関数 × 16 ケース = 16,000 ケース。**bit_exact 13,535**、**mismatch_core 0**、**mismatch_aux 0**、**lift_unsupported 0**、lift_stop_other 0、**native_fault 2,465**（15.4%）。関数単位: bit_exact_all_comparable 963、native_fault_all 37（`mxcsr-flags-run.md` §2.4、`a5-summary.json` を本書で確認）。run4 は bit_exact 12,805、mismatch_aux 717（MXCSR 状態ビット）、lift_unsupported 13 で、0038 が解消（`a5-oracle-run2.md` §2） |
| 未確認・限界 | (1) 対象は純計算 9,901 件のうち 1,000 件（約 10%）。選別から外した 47,915 件（候補 57,816 の 82.9%）は、IAT 呼び出しや間接 call を持つ関数と、その呼び元で、対象外（`a5-oracle-run2.md` §3、§7）。(2) native_fault 2,465 ケース（15.4%）は比較不能。(3) ネガコン: run5 は 203 件中、出力の差で検出 123（**60.6%**）、native fault も含めて 187（92.1%）、未検出 16。run4 は 127/203（62.6%）だった。run5 の検出率は run4 より低い。(4) 1 関数 16 ケースでは全経路を通らない（run4 で全命令を実行した関数は 808/1,000）。乱数 seed は 1 のみ。(5) EFLAGS の未定義フラグは比較していない（AF は run4 で 2,842 ケース）。(6) **run5 は MXCSR の 0038（ホスト実行版）の時点の結果**。0042（ロケール）、0043（x87 の f2xm1 と fsincos）、0044（MXCSR の純 C 版）の後で A5 は再実行していない。0044 の根拠は別の突合（166 バリアント × 100,000 = 16,600,000 ケース、ホスト版との差 0）である。(7) f2xm1 と fsincos は「正しく丸めた値」で、ホスト（AMD Ryzen 9 9950X）と 280,000 回中 4,254 件が 1 刻み違う（0043）。Intel 実機は未確認で、x87 の超越関数が実機と bit-exact とは言えない |

## 7. A6 記録と公開

| 項目 | 内容 |
|---|---|
| 判定コマンドと数値 | 実測値を `docs/` に記録し、`git log` にコミットがある。push 先の remote が設定され、push が成功する |
| 現在の状態 | **部分**（記録とコミットは済み。push は未） |
| 根拠と出典 | 34 コミット（2026-10-20 に 23、2026-10-21 に 11）。各実測は `docs/investigations/*-run.md` と `docs/decisions/` に記録済み。`git remote -v` は出力なし（remote 未設定、2026-10-21 実測） |
| 未確認・限界 | **push は、依頼者が remote を設定するまで保留**。ユーザーの資格情報には触れない。0046 と本日分の追記はコミット前（コミットは PM が行う）。作業ツリーに未コミットの他担当の変更（`lift/runtime/fle_runtime.h`、`fle_sse.h`）と未追跡の `lift/iat-unsup-closure.txt` がある。`docs/progress.md` は古く（lift 本体は未着手と書く）、現状を反映していない |

## 8. 影響

- M1 の完了判定は、本書の A2、A4、A5、A6 の判定コマンドと数値で行う。「達成」は定義どおりに実測した項目に限る。
- 今後の記録は、各基準について「達成 / 部分 / 未達」と実測値、出典を、本書の表と同じ形で書く。
- 優先課題は `docs/milestones/M1-remaining-work.md` の §9 に書いた。

## 9. 2026-10-22 の更新

状態: 追記。**§0〜§8 は書いた時点（2026-10-21、HEAD 31d0134 の実測と、判定一覧は e84e99d 時点）の記録として、消さずに残している**。**判定基準（A1〜A6 の定義、判定コマンド、M1 完了 = A2 + A4 + A5 + A6）は変更しない**。以下は、同じ基準を現在の HEAD 92b9e50（2026-10-22 20:52）に当てはめた判定である。実測の出典は各行に書いた。PM の確認待ち。

§0〜§8 の次の記述は、現在は事実と合っていない（本節で訂正する）。

- §0 の「README には ADR の索引が無いので、README は変更していない」: `docs/README.md` を作った（決定 0001〜0070 の索引。ルートの `README.md` は未変更で、本書の時点と同じく ADR の索引を持たない）。
- §7 の「`docs/progress.md` は古く（lift 本体は未着手と書く）」: `docs/progress.md` を現状に合わせて書き直した（旧版は同文書の「履歴」に残した）。
- 0046 の本文が「`pm/` ディレクトリは無い」と報告していた点: リポジトリ直下は変わらず `README.md binaries docs lift tools` で、`pm/` は無い（2026-10-22 に `ls` で確認）。
- §5 の「`init_count` という名前の計数はリポジトリに無い。『survey』の測定ツールも無い」: 決定 0060 と `bash lift/harness/a4-survey.sh` が入った。現在の測定は下の A4。
- §7 の「作業ツリーに未コミットの他担当の変更」: この更新の時点の `git status` は、未追跡 3 件（`lift/harness/wasm/trace.c.new`、`trace_native.c.new`、`lift/iat-unsup-closure.txt`。最後は 0 行）のみ。

### 9.1 判定の一覧（現在）

| 基準 | §1 の判定（2026-10-21） | **現在の判定（2026-10-22）** | 一言の根拠 |
|---|---|---|---|
| A1 | 達成 | **達成** | `sha256sum`、`stat -c %s` が台帳と一致（再実行） |
| A2a | 未達（部分） | **未達（部分）** | 命令 stub 43 → **0**。解決不能な間接 jmp 1 が残る。Xgetbv は名前付き停止 |
| A2b | 未達 | **未達** | 応答の無い IAT thunk 557 → **533** |
| A3 | 達成（定義の範囲） | **達成（定義の範囲）** | native と wasm の a4 測定が一致。限界は §4 のまま、ブラウザ未検証 |
| A4 | 未達 | **未達** | init_count **115 / 942**（0046 の時点は index 17）。完走（completed 895）には遠い |
| A5 | 部分 | **部分** | run5 のまま。**再実行していない** |
| A6 | 部分 | **部分** | 記録とコミットは済み。remote 未設定のため push は保留 |

M1 完了（A2 + A4 + A5 + A6）は**未達**。

### 9.2 A1 素材

- 実測（2026-10-22）: `sha256sum binaries/win64/FLEngine_x64.dll` = `b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d`、`stat -c %s` = 22227016。台帳と一致。判定: **達成**。
- 限界は §2 のまま（`binaries/` は `.gitignore` 対象、インストーラは再ハッシュしていない、抽出手順の再実行はしていない）。

### 9.3 A2 lift 完全性

出典: `lift/out-cl/gen/gen-pdata.txt`（2026-10-22 15:21 に生成した all-pdata の統計）と、生成 C `lift/out-cl/out/wasm-pdata-part-*.c`。

| 項目 | 2026-10-21（§3） | 2026-10-22（現在） |
|---|---|---|
| 関数 / `c_bytes` | 58,695 / 215,539,555 | 59,297 / 215,700,245（0051 の adjustor thunk 602 を seed に追加） |
| `unsupported_total` | 601 | **534**（生成 C の `fle_unsupported` 行も 534 行） |
| 内訳: IAT 経由 jmp | 557 | **533** |
| 内訳: 解決不能な間接 jmp | 1（`f_4126f0` の `jmp [rcx]`、0x412752） | **1**（同じ。W3） |
| 内訳: 命令未実装 | 43 | **0** |

`unsupported_total` の推移（各調査記録の「前 → 後」の値）: 2,991 → 698 → 667 → 603 → 601（§3）→ 558（0047。命令 43 件を実装。`a2a-remaining-run.md`）→ 554（0050 のイベント 4 thunk。`event-sync-run.md` §4）→ 554（0051、0052。変化なし。`adjustor-thunks-run.md`、`virtualalloc-top-down-run.md`）→ 551（0053。`getdc-caller.md`）→ 547（0054。`gdi-objects-caller.md`。0055 の msctf も 547 のまま、`msctf-caller.md`）→ 546（`getsystemmetrics-caller.md`）→ 543（`spi-caller.md`）→ 542（`loadcursor-caller.md`）→ 541（`loadimage-caller.md`）→ 541（0062、0063。`next-stop-getkeyboardlayout.md`）→ 540（`callback-caller.md` の開始時点の基準）→ **534**（`gen-pdata.txt`。0064〜0070 の後の最新の生成）。中間の値は各記録の記述で、本更新では再生成していない。

**A2a**（生成 C の `unsupported_total` から IAT 経由 jmp を除いた数が 0）: 現在 534 − 533 = **1**（解決不能 jmp）。命令 stub は 0 になった。判定: **未達（部分）**。残る 1 か所は `f_4126f0` の `jmp [rcx]`（setjmp/longjmp 風、呼び出し元は終了またはエラー経路に見える。**推測**、`unsupported-inventory-pdata.md` §6 W3）。

- **Xgetbv の扱い**: Xgetbv は生成 C で `fle_xgetbv_stop`（名前付きの停止。`runtime/fle_cpuid.c`）として出力され、`unsupported_total` と `fle_unsupported` の計数には**含まれない**（`gen-pdata.txt` の `handled` に `xgetbv_stop 1`）。0047 は「A2a の判定にこの除外を使うかは PM が決める」と書いた。**本書は、Xgetbv を『命令 stub 0』の数に含めない扱い（= 0 件）で現在の数字を書いたが、その除外が A2a の定義に合うかは PM の判断である**。根拠（0047 §5）: SDM は XCR0 の値を決めない（値を返す実装を作れない）、ランタイムの CPUID が OSXSAVE を主張しないので `xgetbv` の手前の分岐を必ず通らせない（単体実行で確認）、到達すれば命令と VA を名前に出して止まる。ただし、harness の実行でこの経路が実際に実行されるかは確認していない。除外を認めない場合、A2a の「命令 stub」は 1（Xgetbv）で、判定は同じく未達である。
- 0046 の A2a の「解決不能な間接 jmp が 0」は満たしていない（1 か所）。したがって A2a が**達成**になる日は、この 1 か所の扱い（W3）が決まるまで来ない。

**A2b**（閉包の IAT thunk のうち、実行で踏まれるものが全て仕様確認済みの応答を持つ）: 応答の無い IAT thunk は **533 thunk、514 API、23 DLL**（0046 の時点は 557 thunk、536 API）。判定: **未達**。

- 内訳（DLL ごと。生成 C の `Jmp ... fle_unsupported` 533 行を、DLL の `jmp [rip+rel32]` と `lift/out-imports/imports.txt` で DLL と API に引いた。スクリプトと出力は `lift/out-doc/remaining-iat-by-dll.py`、`remaining-iat-by-dll.txt`）: user32 207（197 API）、gdi32 123（120）、kernel32 54（51）、comctl32 31（30）、shell32 24（22）、ole32 19、wininet 15、msacm32 14、winmm 13、crypt32 7、winspool 5、mpr 4、version 3、advapi32 2、opengl32 2、imagehlp 2、dsound 2、wintrust、shfolder、oleaut32、crt、oleacc、shlwapi 各 1。0046 の時点の user32 221、gdi32 129、kernel32 58、ole32 19 から、user32 −14、gdi32 −6、kernel32 −4。
- **「実行で踏まれるもの」の数は、今回も測っていない**（静的な閉包は過大見積もり。§3 の限界のまま）。`import 応答の完全性` が満たされたとは言えず、満たされるまでの距離も、この数からは言えない。
- 「仕様確認済みの応答」に数えられないもの: 今日までに追加した応答の大半が仮の決定（§9.5）。**応答が増えたことを A2b の前進と書かない**。0046 の A2b の定義は「仕様確認済みの応答」で、仮の決定を含むものは、実機または公式資料で確認するまで数えない（§5 の扱いを引き継ぐ）。

### 9.4 A3 wasm

- 現在の確認: `bash lift/harness/a4-survey.sh` の `WASM=1` 実行（`lift/out-cl/a4/20261022T101528Z-node.txt`）が、同時刻の native（`-native.txt`）と、`rsp=` の行を除いて全文一致する（`diff <(grep -v rsp= native) <(grep -v rsp= node)` の出力が空。本更新で再実行して確認した）。停止点、関数実行数 645、入口数 13,910、呼び出し深さ 10、a4 の行がすべて同じ。`flengine.wasm` は 115,894,302 B。判定: **達成（定義の範囲）**。
- §4 の訂正と限界（4 の (2)〜(5)）は残る。(3) の fscale の不一致は 0049 で直した（プローブ 60,002 行で native と wasm の不一致 0）。(4) ブラウザは未検証（`-sENVIRONMENT=node`、`-sNODERAWFS=1`）。(5) ページ保護の非強制、アリーナ 512 MiB の上限は変わらない。今回、`-O1` 以上、ブラウザでは確認していない。
- native のワークスペース全テスト: 84 バイナリ、**958 件成功、失敗 0**（`lift/out-cl/test-ws.txt`、2026-10-22 20:19）。§4 の 65 バイナリ、615 件から増えた。同日 19:49 の別の実行（`test-ws-run1.txt`）は 957 件成功、1 件失敗（msctf のテスト `the_row_is_in_the_table_and_declared_in_the_header`）。失敗の原因は確認していない（再実行で解消したと読めるが、推測）。

### 9.5 A4 起動

**測定の定義は決定 0060**（`docs/decisions/0060-a4-measure.md`。判定基準は変えていない。InitCount は「実行中の index + 1」で完了数ではなく、完了数は completed = started − in_progress）。測定は `bash lift/harness/a4-survey.sh`（使い方と静的表は `docs/investigations/a4-survey-tool.md`）。

| 項目 | 実測（`lift/out-cl/a4/20261022T101528Z-native.txt`、HEAD 92b9e50 の直前の状態の実行。wasm も同じ a4 行） |
|---|---|
| 静的な unit 表（DLL のバイト列） | 942 件（runnable 895、empty 46、noproc 1）。§5 の記録と一致（差 0） |
| **init_count / 942** | **115 / 942**（最後の unit は index 114、run_rank 111 / 895、proc 0x623840、in_progress） |
| started / completed（895 runnable 中） | started 111、**completed 110**、in_progress 1。all_units_done = **no** |
| 停止 | `fle: unsupported instruction at 0x425780`（user32!CreateWindowExW の thunk）。`f_52a7f0` の 0x52a8be、深さ 10 |
| 関数実行数 / 入口数 | 645 / 13,910（§5 の時点は 199 / 1,386） |

判定: **未達**。`completed 895 / 895` と `returned from entry` での終了コード 0 は満たしていない。

- §5 の停止（`GetLocaleInfoW` LCType 0x1c、unit index 17）から、停止点は大きく進んだ（ロケール 6 種 → VirtualAlloc 0x101000 → GetDC → CreatePalette → LoadLibraryW(msctf) → GetSystemMetrics → SystemParametersInfoW → LoadCursorW → LoadImageW → GetKeyboardLayout → EnumDisplayMonitors → SystemParametersInfoW(0x1f) → GetObjectW → LoadIconW(MAINICON) → GetClassInfoW → CreateWindowExW）。**ただし、A4 の指標である init_count は、測定ツールが入った後（2026-10-22 00:32 以降）は 115 のまま動いていない**。GetSystemMetrics 以降の停止点の移動は、すべて同じ unit 114 の内側にある（呼び出しスタックに `f_623840` がある。`getsystemmetrics-caller.md`、`spi-caller.md`、`loadcursor-caller.md`、`next-stop-*.md` の記録）。**この unit は GDI・入力・ウィンドウの初期化で、停止点が進むことと完走が近づくことは同じではない**。
- **仕様確認済みでない応答が大きく増えた**。§5 の表（0030、0040、0041、0042）に、その後の 0048、0053〜0057、0059、0061〜0063、0065〜0067、0069、0070（決定ファイルが「仮」と書く 18 件のうち、0040〜0042 を除く 15 件）が加わる。完走を A4 の「外部 API の応答は仕様確認済みのみ」で判定するなら、これらは数えない。一覧と撤回条件は `docs/investigations/provisional-decisions.md`。「ホストの実環境を見せない」「値を創作しない」という方針（0053 などが書く）は守っているが、**実 Windows との照合は 1 件も無い**。
- 完走までの量は、測っていない。残る runnable は 895 − 110 = 785 件（算術）だが、そのうち何件が未対応の API に当たるかは未測定。**完走の時期は断定しない**。

### 9.6 A5 差分実行

- **再実行していない**。最新は run5（`lift/out-oracle/results/run5/a5-summary.json`、2026-10-21 00:29）のまま: 16,000 ケース（1,000 関数 × 16）、bit_exact 13,535、native_fault 2,465（15.4%）。mismatch_core 0。
- run5 の後に、0042（ロケール。A5 の対象関数に影響しないと見るが未確認）、0043（x87 の f2xm1/fsincos）、0044（MXCSR の純 C 版）、0047（Psubw、Pcmpgtd、Rcpps、Bswap ほかの命令）、0049（ゲストアドレスの検査、fscale の `long`）が入った。これらの後の A5 は未測定。§6 の限界（対象は純計算の約 10%、native_fault、ネガコン 60.6%、Intel 実機は未確認）は変わらない。判定: **部分**。

### 9.7 A6 記録と公開

- `git log`: 58 コミット（2026-10-20 が 23、2026-10-21 が 25、2026-10-22 が 10。`git log --format=%ad --date=format:%Y-%m-%d` の集計）。各実測は `docs/investigations/*.md` と `docs/decisions/` に記録済み。
- `git remote -v` は出力なし（remote 未設定、2026-10-22 実測）。**push は、依頼者が remote を設定するまで保留**。ユーザーの資格情報には触れない。判定: **部分**。
- 本更新（0046 の本節、`docs/progress.md`、`docs/README.md`、`docs/milestones/M1-remaining-work.md` の追記、`docs/investigations/provisional-decisions.md`）は、コミット前。コミットは PM が行う。

### 9.8 この更新の限界

- 数値は、2026-10-22 15:21 生成の `gen-pdata.txt`（`unsupported_total` 534）と、同日 19:15 の a4 測定（`20261022T101528Z`）、20:19 の `test-ws.txt` による。**HEAD 92b9e50（20:52、0069、0070 の実装）の後に、生成 C と a4 を再実行していない**。0070 が `GetClassInfoW`、`RegisterClassW` を TABLE に足した後の生成 C が 15:21 の生成に含まれるか、含まれないかは、`gen-pdata.txt` の `jmp_imp:GetClassInfoW 1`、`jmp_imp:RegisterClassW 1`（応答表に行がある）から含まれると読んだ。停止点 0x425780 と関数実行数 645、入口数 13,910 は、`getclassinfo-caller.md` の実装後の実測（同ファイルの「実装後の実測」の段落）と一致する。
- 「実機 Windows 未確認」の仮の決定は、A2b と A4 の「仕様確認済み」を満たさない可能性がある。この更新は判定基準を変えず、判定の根拠を書き直しただけである。
- 別の作業者が `lift/` と `docs/decisions/0071-*.md`、`docs/investigations/window-design.md` を同時に編集している。本更新はそれらを読んでいない（この時点で `window-design.md` と 0071 は存在しなかった）。
