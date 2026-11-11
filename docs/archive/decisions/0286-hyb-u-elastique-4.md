# 0286(mt3w、仮): hyb の U リストに Elastique の 4 関数を足す(E10 の主部品) — 規則 0239 + 手順 3b の再実行で追加は 4 関数だけ

基準: **PM の e9-wip b0fe2c11**(main c74b9194 + E9。0270・0270b 入りの fe-wasmgen。`git archive b0fe2c11` の写し `lift/out-mt3y/w86`、PM の木には書いていない)。区分: **実測**＝このセッションの出力を見た事実、**読み**＝解釈、**未確認**。

## 冒頭の 3 点

1. **測ったこと(実測。実エンジン〔node・ブラウザの FL〕の走行は 0 件)**:
   - 規則 0239 + 手順 3b(B = 150 ms)の regen を、**b0fe2c11 のコードの fe-wasmgen(release)**で、E7b のとき(0270 の文書 §9)と**同じ引数**で再実行した: 実行された 15,412 関数(E-all)、`--atomic mt4-atomic-functions*.tsv`、`--startup-ms startup-big-ms.tsv --startup-budget-ms 150`、`--current lift/harness/seeds`、8 モジュールの census(`runcs.sh`)。結果: lowerable **14,992**、切り口(0.9999)**4,145**、startup-big **99**(手順 3b で 4 を再許可: 287,729 B、first idle の悲観 149 ms)→ **4,046 関数**。
   - **E7b のコード(REF2 = 0270 無し)との差は、追加が Elastique の 4 関数だけ(0xb1ce5e0・0xb25ce20・0xb2cf0c0・0xb337800)**。ほかのモジュールの追加は 0。**外れる(削除)のは 3 関数**: Elastique 0xb2c8090(`__security_check_cookie`。0270b の守りで C 経路に戻る)と fle の 0x4781f0・0xe936b0(切り口の移動)。REF2 の候補が 4,045、今回が 4,046(+4 −3)。
   - 差分 `0286-hyb-u-elastique-4.diff`(`hyb-U-elastique.txt` に +4 行と注記 1 行、`ci/test-hybrid.sh` の数 2 か所)。b0fe2c11 に `patch --batch -p1 --dry-run` が offset・fuzz なしで通る(`-` は 2 行 = test-hybrid.sh の数の更新だけ)。`ci/test-hybrid.sh` の hyb の検査は **3,875 関数・Elastique 165・oggio 46 で全部 ok**(私の写しでは `.github` が無く『deploy.yml: P_FELIFT lacks fe-wasmgen』の 1 件だけ FAIL = この差分と無関係の、写しの欠け)。
   - Elastique の本番形の wasm(`--hybrid-only` で U の関数だけ): **1,975,962 B → 2,286,607 B(+310,645 B、+15.7%)**。4 関数の wasm_code_bytes の合計 310,464 B(68,619 + 210,913 + 30,509 + 423)。
   - `boundary_regs.rs`(C ↔ 直 wasm の境界のテスト 2 件)は b0fe2c11 で通る。4 関数の caller / callee を静的に出し(§4)、守られた 5 関数との隣接は 0xb2c8090(cookie)だけ。
2. **結論**: **追加は予測どおり 4 関数だけ**で採れる。S1 は +310,645 B(E8 の wasm 142,586,692 B に対し +0.22%、線 154,885,888 B の内)。first idle の悲観(0.53 ms/KB)は +160 ms、E7b の ci1 の実測(Chrome −1,432 ms、Firefox −376 ms)の余裕の内。期待する利得は 0265 の 3 関数の C 経路 1,769 ms(ミキサースレッドの 11.7%)で、**推定で未測定**(opt1 の E10 後の確認で見る)。
3. **未確認**: 実エンジンでの 4 関数の挙動と CPU の効果(opt1 の E10 冷間ビルド後)。first idle の実測の増分。`cargo test -p fe-wasmgen`(b0fe2c11): 31 results・67 passed・0 failed(§5)。

## 1. 再実行の結果(`tables/0286-regen-NEW4-diff-vs-current.tsv`)

| 構成 | コード | lowerable | 切り口 | startup-big | U の関数 |
|---|---|---|---|---|---|
| REF2(E7b のとき) | 0270 無し | 14,987 | 4,143 | 98(手順 3b で 4 を再許可) | 4,045 |
| NEW2(0270 の文書) | 0270 | 14,994 | 4,146 | 99 | 4,047 |
| **NEW4(今回)** | **b0fe2c11 = 0270 + 0270b** | **14,992** | **4,145** | **99(再許可 4: 287,729 B、149 ms)** | **4,046** |

(NEW4 = NEW2 − 2 は 0270b の守りが 2 関数〔Elastique 0xb2c8090・dsp 0x37ca8f0、どちらも cookie〕を C 経路に残した分。)

**NEW4 対 REF2(U のリスト、モジュール別)**:

| モジュール | REF2 | NEW4 | 追加 | 削除(外れる) |
|---|---|---|---|---|
| fle | 2,929 | 2,927 | 0 | 0x4781f0・0xe936b0(切り口の移動。0270 の文書 §9 のとおり) |
| dsp | 159 | 159 | 0 | 0 |
| b2d | 75 | 75 | 0 | 0 |
| qfc | 359 | 359 | 0 | 0 |
| ft | 177 | 177 | 0 | 0 |
| ilt | 134 | 134 | 0 | 0 |
| oggio | 46 | 46 | 0 | 0 |
| **elastique** | 166 | 169 | **0xb1ce5e0・0xb25ce20・0xb2cf0c0・0xb337800** | **0xb2c8090**(cookie。0270b の守りで C 経路に残る) |
| 合計 | 4,045 | 4,046 | 4 | 3 |

**コミット済みのリスト(3,871)に対して**: regen の出力との差は『only in current 18・only in regenerated 193』(REF2 では 18・192)。193 の内訳は fle 180・elastique 8・ilt 3・dsp 1・ft 1(REF2 は fle 182・elastique 5)で、**この差のうち 0270 に由来するのは Elastique の 4 関数の追加と、REF2 にあった 3 候補〔0xb2c8090、fle の 2 関数〕が消えたことだけ**。ほかは E7b からある差で、『追加のみ』の約束どおり反映しない。取り込むのは 4 関数だけ(リストの末尾に追記)。

## 2. 差分の中身と、4 関数の判定(`0286-hyb-u-elastique-4.diff`)

| 関数 | wasm_code_bytes | 実行(命令。18 run: start+par-boot / ref1-perf / ref1-ui / 合計) | 判定 | S1 の増分 |
|---|---|---|---|---|
| 0xb1ce5e0 | 68,619 | 0 / 444.5 M / 158.4 M / 602.9 M | 起動で動かない(startup-big でない)。切り口内 → U | +68,619 B |
| 0xb25ce20 | 210,913 | 0 / 36.9 M / 8.8 M / 45.8 M | 同上 → U。最大。`status ok` = MAX_FN_COST 75,000,000 の内 | +210,913 B |
| 0xb2cf0c0 | 30,509 | 0 / 61.8 M / 2.3 M / 64.1 M | 同上 → U | +30,509 B |
| 0xb337800 | 423 | 0 / 12.8 M / 2.0 M / 14.8 M | 同上 → U | +423 B |
| 合計 | **310,464** | | | モジュールの wasm 全体では **+310,645 B**(1,975,962 → 2,286,607) |

- **MAX_FN_COST との関係**: `ci/test-hybrid.sh` の『cost limit is 75,000,000』は変えない。4 関数の census の status は全部 `ok`(上限内。上限を超えると『cost over the limit』になる)。最大の 0xb25ce20(210 KB)も `ok`(E7b で 30,000,000 → 75,000,000 にした理由の『Elastique の大きい 2 関数』の 1 つ)。
- **startup-big の判定**: 4 関数とも start / par-boot の命令が 0 なので startup-big(≥ 20,000 B で start・par-boot で動く物)に当たらない。手順 3b の B = 150 ms は『startup-big の再許可』にかかる予算で、この 4 関数は予算の外(0270 の文書 §9 と同じ判断)。
- **S1(B 単位)**: モジュール +310,645 B。E8 の engine wasm 142,586,692 B に足すと 142,897,337 B(+0.22%)、線 154,885,888 B まで 11,988,551 B 残る。
- **first idle**: 悲観(0.53 ms/KB × 303 KB)で +160 ms。E7b の ci1 の実測(Chrome −1,432 ms、Firefox −376 ms、各 n=6)の余裕の内。node の 1 関数ずつの実測の 1/9〜1/20 を当てた概算は +8〜18 ms(推定)。
- `ci/test-hybrid.sh`: `nu` 3,871 → **3,875**、Elastique 161 → **165**、メッセージの文面も。ほかの検査(重複なし、8 リストだけ、oggio 46、cost limit)は変えない。
- リストの注記 1 行(`# decision 0286: …`)をヘッダに足した(`fn` は `#` の行を数えない)。

## 3. PM が検証できる形

| 項目 | 結果 | 根拠 |
|---|---|---|
| `ci/test-hybrid.sh`(hyb の検査) | 3,875 / 165 / 46 で ok(私の写しでは deploy.yml の 1 件が FAIL = `.github` の欠け。b0fe2c11 の完全な木では ok のはず〔読み〕) | 実測 |
| `cargo test`(fe-wasmgen / fe-lift) | **変わらない**: どの cargo のテストも `hyb-U-*.txt` を読まない(grep: 参照は `ci/test-hybrid.sh`・`app/scripts/build-engine-bundle.sh`・`lift/harness/hyb/filter-hybrid-list.py` だけ)。fe-wasmgen 全部の結果は下 | 実測 + 読み |
| `test.sh full` | **変わらない**: `test.sh` の対象のテストは上のとおり `hyb-U` を読まない | 読み |
| 既定の FLEngine C の sha | **変わらない**: `hyb-U-*.txt` は fe-wasmgen(`--hybrid-only`)と strip(`strip_units.py`)だけが読む。fe-lift の生成 C には入らない(`gen-direct.sh`・`gen_two.sh` は参照しない) | 読み(grep) |
| wasm の差 | **8 モジュールのうち Elastique だけ**(リストを変えたのは `hyb-U-elastique.txt` だけ)。+310,645 B | 実測(本番形の単体ビルド。`tables/elastique-prod-like-wasm.txt` に sha と大きさ) |
| C の部分 | Elastique の C 部分から 4 関数の本体が strip で消える(`strip_units.py`、リストにある関数の本体を除く)。その分 C の部分は小さくなる | 読み |

## 4. 境界 — 4 関数の caller / callee と、守られた 5 関数(`tables/elastique-callgraph-direct-edges.txt`)

静的な呼び出しグラフ(`callgraph`、direct call / jmp のみ。8,035 辺)から、4 関数の形:

| 関数 | caller(直接) | callee(直接) | 新しく直になって増える境界 | 守られた 5 関数との関係 |
|---|---|---|---|---|
| 0xb1ce5e0 | 1: 0xb1baec0(**U**。署名が 0270 で変わった: sig_in 0x0f07 → 0x2f86 = 0xb1ce5e0 の新しい署名と一致) | 3: 0xb1cfb60・0xb2840c0(C 経路)・**0xb2c8090(cookie、守りで C 経路)** | 直 → C(2 本)。caller は U なので C → 直 は無し | 0xb2c8090 を呼ぶ: 呼んだ後の `je 0xb1cf210` は**要約の定数 ZF=1**を読む(`FL` ではない)。status は 0270b の下で `ok` |
| 0xb25ce20 | 3: 0xb1bb100・0xb1bb120・0xb1bb140(**C 経路**。署名が 0x0f07 → 0x2fc6 に変わった = 0xb25ce20 の新しい署名と一致) | 3: 0xb260080・0xb2840c0(C 経路)・**0xb2c8090** | **C → 直 が 3 本**(wrapper)、直 → C が 3 本 | 0xb2c8090 を呼ぶ: 呼んだ後の `jne 0xb25dd5d` が要約の ZF を読む。同上 |
| 0xb2cf0c0 | 19: **U 6**・**C 経路 13**(0xb289c10 など) | 2: 0xb2d82c4・0xb2d8440(C 経路) | C → 直 が 13 本、直 → C が 2 本 | 無し |
| 0xb337800 | 2: U 1・C 経路 1(0xb006d20) | 0(leaf) | C → 直 が 1 本 | 無し |

- 守られた 5 関数(Elastique 0xb2c8090・0xb3aa660・0xb3ae300・0xb3b26e0、dsp 0x37ca8f0)のうち、4 関数の隣にあるのは **0xb2c8090 だけ**(0xb1ce5e0 と 0xb25ce20 の callee)。0xb3aa660・0xb3ae300・0xb3b26e0・dsp 0x37ca8f0 は 4 関数の caller にも callee にも無い。
- **0xb2c8090 の扱い**: 守りが C 経路に残すのは『**C 経路の caller** が戻りのフラグを読む』形で、直 wasm の caller が**要約**(cookie の戻りの ZF=1 という既知値)から読む形は止めない(0270b §6.5)。4 関数のうち 2 本の caller は直 wasm で、要約から読む側。この形は `boundary_regs.rs` の 2 件目(**`the_registers_the_stack_pointer_and_xmm6_to_15_come_back_and_the_je_reads_the_callees_zf`**: 直 wasm の caller が C 経路の callee を呼び、callee-saved・R[4]・xmm6〜15 が戻り、`je` が callee の ZF を読む)が覆い、1 件目(`the_wrapper_of_a_direct_function_leaves_the_flags_of_the_c_path_as_they_were`)が C → 直の wrapper の形を覆う。**b0fe2c11 で 2 件とも通る(実測)**。
- **C → 直の caller(0xb1bb100 など 17 本)の署名**: 0270 で変わった署名(sig_in)は、新しく直になる callee の署名と一致している(上の表)。署名は fe-wasmgen の 1 回の実行の中で一貫するので、4 関数を足しても署名の食い違いは起きない(読み。実エンジンで見るのは opt1)。
- **このテストが覆わない物**: 4 関数そのものを使った境界のテストはない(`boundary_regs.rs` は 7 つの実行される関数の形の合成)。C ↔ 直の全レジスタテストを 4 関数の形(特に 0xb25ce20: 3 本の C 経路 caller と cookie 呼びが同居)で実エンジンで確認するのは opt1(E10 の冷間ビルド後)。

## 5. 実行したテスト

- `cargo test -p fe-wasmgen --no-fail-fast`(b0fe2c11、`CARGO_BUILD_JOBS=3`、nice 15): **31 results・67 passed・0 failed**(`tables/fe-wasmgen-tests-0286.log`)。fe-lift と `test.sh full` は走らせていない(`hyb-U` を読むテストが無いので結果は変わらない: §3)。
- `boundary_regs`: 2 passed / 0 failed。

## 6. 限界・自己申告

- 実エンジンの走行 0 件。4 関数が速くなる量は未測定(1,769 ms は 0265 の 45 s のプロファイルの C 経路の費用で、wasm 化後の値ではない)。
- caller / callee は direct call / jmp の静的グラフ(間接呼びは数えていない)。
- census は 18 run の実行関数(E-all)に基づく。
- 私の `cargo`・regen は `taskset -c 8-15`・`nice -n 15`(regen の census は fe-wasmgen の 2 本並列、cargo は `CARGO_BUILD_JOBS=3`)。`sleep`・`timeout`・末尾 `&` は呼んでいない。

## PM の決定と取り込み(2026-11-06 11:40、docs のみ)

取り込み: mt3 の第 1 報(12:30 の最終で変われば更新)の文書・表・道具を `docs/investigations/mt3w/0286/` に、差分 `0286-hyb-u-elastique-4.diff`(33 行)を `docs/investigations/mt3w/patches/` に置いた。実エンジンの走行は 0 件、4 関数の CPU の効果・first idle の増分は未測定(mt3)。PM の確認: 差分は E9(`e9-wip` b0fe2c11)に `patch --batch -p1` で rc=0、適用後の `ci/test-hybrid.sh` は 'all checks hold'(hyb-U 合計 3,875 = fle 2758 + dsp 158 + b2d 76 + qfc 364 + ft 176 + ilt 132 + oggio 46 + elastique 165)。
判断:
1. **4 関数(Elastique 0xb1ce5e0・0xb25ce20・0xb2cf0c0・0xb337800)の hybrid への追加を E10 に載せる**(E9 の次の冷間ビルド)。E9 には入れない(0279 の仮想時間の較正と L1c の入力リングの効果を切り分けるため)。regen の再実行では、追加は Elastique のこの 4 関数だけ、削除は 3 関数(0xb2c8090 = 0270b の守りで C 経路に残る cookie、fle の 0x4781f0・0xe936b0 = 切り口の移動)。
2. S1: Elastique の wasm +310,645 B(+15.7%)、engine wasm に対し +0.22%(142,586,692 → 142,897,337 B)、線 154,885,888 B まで約 12.0 MB 残る。first idle の悲観見積り +160 ms は E7b の ci1 実測(Chrome −1,432 ms〔静かな窓 −593 ms〕、Firefox −376 ms)の余裕の内。**E10 のデプロイ後に実測する**(悲観値を採用値にしない)。
3. 完了条件(E10): opt1 が node 84/55/87 行を E9 と比べて種類と原因で説明、Export sha256 `381cdb5a…f979`、60 s 再生が止まらない、Elastique 3 関数の自己時間を 0265 の 1,193/299/277 ms と比較、first idle を ff5ee429/E7b/E8 と比較。mt3 の `boundary_regs` は 4 関数そのものを使っていない(7 関数の形の合成)ので、4 関数を使った境界の確認は実エンジン(opt1)で行う。
4. 未確認(mt3): 4 関数の CPU の効果、署名が変わった 3 関数(0xb1baec0・0xb005900・0xb2c51e0)の実エンジンでの意味、守られた cookie の C 経路の caller が実在すること(Elastique 6/15、dsp 5/72)の影響。
