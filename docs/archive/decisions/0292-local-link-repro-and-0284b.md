# 0292(仮): (1) E11 の hyb_guard / link-map の新しい行のローカル再現、(2) 0284 の A、(3) 後片付け(mt3w)

状態: **仮**。作業者 mt3w、2026-11-06 JST。基準: PM の `e11-wip`(a203e65e = E10 bb05de13 + 0285)と `e10-wip`(bb05de13)。区分: **実測**=出力を見た値/**読み**=コードを読んだ推論/**未確認**。

## 0. 冒頭の三点(正直さ)

1. **実エンジンが実行されたか**: (1) は CI の stage スクリプトそのもの(`ci/stage-wasm.sh link` = hybn0 → weblink → wasmopt → hybrid)を E11 の木で 1 回走らせた(emscripten の link、wasm-opt -Oz、fe-wasmgen、wasm-merge の実物。C 部の objects は E10 の冷間ビルドの物を再利用: C のソースは E10 と E11 で同じ)。(2) は native の C 経路(`enum_cmp` = ランタイムの `fle_imp_CompareStringW` を 1 対ずつ呼ぶ駆動)と `test.sh full`・a4。**ブラウザ・node での実エンジンの走行は 0 件**。代役・再生・推測の画面や音は使っていない。
2. **(1) の結論**: **PM の e11-wip は冷間 CI の link 段で落ちない**。link map の 3 つの新しい行の形式・export 一覧(`fle_hyb_slice` の 1 行増)・`hyb_guard` の 5 検査(record / map / cpart / map / merged)は、実物の stage スクリプトを E11 の木で走らせて全部通った(§1)。最終の `flengine6.opt.wasm` は E10 の 142,776,280 B から **+310,318 B**(143,086,598 B)で、**0285 の報告の +197,374 B より大きい**(§1.5 で訂正)。`hyb_guard cpart` の 4 アドレスを 7 アドレスに広げる差分 `0285b` も作り、実物の走行で通した(採否は PM)。
3. **(2) の結論**: A は答えが変わる順序対を 116(記号 106 + TAB〜CR 対 空白 10)に限り、停止↔答えの増減は 0。A は fle_locale.c +20 行・wall_ia2.rs +97 行・`-` 0 行。新しいテストは突然変異の確認(e10-wip の runtime では落ちる)つき、`test.sh full` と a4 は §2.5。一般句読点の置換は別の差分 `0284b-gp-extension.diff`(156 順序対が変わる)にして、採否を PM に任せる。

## 1. E11 の冷間 CI で初めて通る物のローカル再現(依頼 (1))

**手順(実測)**: `lift/out-opt2/x16`(opt2 の E10 の冷間ビルドの作業ディレクトリ)から objects・lift の出力を `rsync`(web・hyb の wasm・link.map・logs・timing は除く)で自分の作業ディレクトリへ複製し、`git archive e11-wip` の木に対して `lift/out-gpu1/tools/run-ci-build.sh <木> <作業> F2xH7 felift link` を 1 回。`link` = CI の `stage-wasm.sh link` = `run_bb hybn0 weblink wasmopt hybrid`(PM の CI と同じ段)。手順は `tools/README.txt`。C 部の objects は E10 の物をそのまま使った(C のソースは E10 と E11 で同じ。E11 は hyb-U の wasm と段階のスクリプトだけが違う)。

### 1.1 link map の 3 つの新しい行の形式(実測)
`weblink` の `link.map` の 3 つの新しい行は、既存の 4 つと同じ形式(列: アドレス、LMA、align、名前。名前の行の直前に `<object>:(.data|.bss.<名前>)` の行が付く)で、`build-engine-bundle.sh` の正規表現 `^\s*[0-9a-f]+\s+[0-9a-f]+\s+[0-9]+\s+<名前>$` が **名前の行を 1 つだけ**捕まえる(section の行は末尾が `)` なので捕まえない):
```
10209708  a656ec3        8                 fle_progress_limit      (a4.o の .data: 初期値 UINT64_MAX を持つので LMA の列が 0 でない)
1030cc40        0        4                 fle_dispatch_hook       (a4.o の .bss)
1030cc48        0        8                 fle_progress_count      (既存)
1030cc54        0        4                 fle_landing_top         (a4.o の .bss)
```
`hyb-addresses.txt`: `fle_teb 0x109202a0 fle_mxcsr 0x10209958 fle_hyb_buf 0x109222b0 fle_progress_count 0x1030cc48 fle_progress_limit 0x10209708 fle_landing_top 0x1030cc54 fle_dispatch_hook 0x1030cc40`、`tls: fle_ctx at TLS offset 0x40`。3 つの新しい番地は 32 bit に収まる(`i32.load` の offset 即値にできる)。サイズは `fle_progress_limit` 8 byte(u64)、`fle_landing_top` と `fle_dispatch_hook` が 4 byte(wasm32 のポインタ)で、0285 の補助関数の `i32.load` と一致。

### 1.2 export 一覧(実測)
hybn0 が fe-wasmgen を `--hybrid-dispatch 0x3308 0x3310 0x3318`(仮)で走らせ、各 module の wasm の `c.*` の import から作る `exports.rsp` は **2,269 行**(E10 の 2,268 行より `-Wl,--export-if-defined=fle_hyb_slice` が 1 行増えただけ。`diff` で確認)。weblink の出力 `flengine6.wasm` に export `fle_hyb_slice` が **1 つある**(`wasm-objdump -x -j Export`)。= hybn0 と hybrid で『`fle_hyb_slice` を import する module』が同じ(アドレスに依らず適合する関数の集合は同じ)ことが、export 一覧に出ている。

### 1.3 hyb_guard の検査(実測)
2 回目(清潔な走行)の出力(`tables/e11-run2.log`)から:
- `hyb_guard record`: weblink が `link.map.sha256`(`8f821884…`)を書いた。
- `hyb_guard map`(hybrid の前): `link map is the one the weblink stage wrote (sha256 8f82188432a14cb7)`。
- `hyb_guard cpart`(**私が 7 アドレスに広げた版**。`0285b-…diff` を当てた木): `uses in the C part: fle_teb 0x109202a0: 7; fle_mxcsr 0x10209958: 828; fle_hyb_buf 0x109222b0: 60; fle_progress_count 0x1030cc48: 14; fle_progress_limit 0x10209708: 7; fle_landing_top 0x1030cc54: 23; fle_dispatch_hook 0x1030cc40: 5` — 7 つとも 1 回以上 → 通る。
- `hyb_guard map`(merge の前): 通る。
- `hyb_guard merged`: `fle_mxcsr 0x10209958: C part 828, merged 56806; fle_hyb_buf 0x109222b0: C part 60, merged 31255` → 通る。
- hybrid の段階の自己検査: 各 module で `listed N, emitted ok N, listed but not emitted 0, emitted but not listed 0`(fle 2,758、dsp 158、b2d 76、qfc 364、ft 176、ilt 132、oggio 46、elastique 165。計 3,875)、`stubs removed 3875, import entries removed 3875`、『merged module still imports functions of module c』= 0(さもなければ段階が落ちる。通った)。
1 回目(私の編集で壊れた走行。§1.6)でも、PM の e11-wip そのままの 4 アドレスの cpart(`fle_teb … fle_progress_count 0x1030cc48: 14`)・map・merged は通った(`tables/e11-run1.log`)。
**PM の e11-wip(4 アドレスの guard)は、このまま冷間 CI で落ちない**(link map の 3 行の形式と export 一覧と guard の 4 検査を、実物の stage スクリプトで通した)。

### 1.4 見つけたこと(PM の懸念への答え)
- **`hyb_guard cpart` は新しい 3 つのデータの番地も、最適化後の C 部(`flengine6.c.opt.wasm`)の中に `i32.const <番地>` の形で持っている**: `fle_progress_limit` 0x10209708: **7** 回、`fle_landing_top` 0x1030cc54: **23** 回、`fle_dispatch_hook` 0x1030cc40: **5** 回(既存の 4 つ: fle_teb 7、fle_mxcsr 828、fle_hyb_buf 60、fle_progress_count 14)。= 0285 の実装で私が『`i32.const` の数え方では 0 になって誤って落とす恐れがあるため広げなかった』と書いた懸念は、**最適化前の wasm(`flengine6.wasm`)では本当に 0**(memarg の offset で参照される: limit 53、landing 119、hook 51、progress_count 106)だが、**ガードが読む最適化後の C 部では 0 ではない**。⇒ ガードを 7 つに広げても冷間 CI で落ちない(この C 部で確認した)。
- そこで `0285b-hyb-guard-cpart-seven-addresses.diff`(基準 e11-wip、`patch --batch -p1 --dry-run` が offset・fuzz なしで通る、`-` は 4 行: ガードの docstring・名前の組・長さの条件・呼び出しの引数): `hyb_guard cpart` が 4 つまたは 7 つを受け、hybrid の段階が 7 つ渡す。`ci/test-hybrid.sh` に合成の C 部で 3 項目(7 つ全部使う C 部は通る、`fle_landing_top` を使わない C 部は落ちる、4 つだけの C 部に 7 つを渡すと落ちる)を足した(all checks hold)。**採否は PM の未決**(E11 に載せるなら 0285 の差分の後ろに)。

### 1.5 大きさ(実測。E10 = opt2 の冷間ビルド x16、E11 = この再現。`wasm-opt -Oz` の後の各 module と、最終の `flengine6.opt.wasm`)
| module | E10 | E11 | 増分 B |
|---|---|---|---|
| fle | 5,452,534 | 5,590,141 | +137,607 |
| dsp | 508,248 | 508,248 | 0 |
| b2d | 133,213 | 135,719 | +2,506 |
| qfc | 670,051 | 695,689 | +25,638 |
| ft | 233,183 | 242,436 | +9,253 |
| ilt | 201,136 | 205,489 | +4,353 |
| oggio | 430,361 | 430,947 | +586 |
| elastique | 2,158,064 | 2,179,356 | +21,292 |
| 8 module の計 | 9,786,790 | 9,988,025 | **+201,235** |
| `direct.wasm`(merge1 の後)| 10,759,175 | 11,012,469 | +253,294 |
| **`flengine6.opt.wasm`(最終)** | **142,776,280** | **143,086,598** | **+310,318(+0.217%)** |
最終の増分の内訳(`wasm-objdump -h`): Code セクション +295,855 B、Elem +13,598 B(7 つの表の要素)、Function +806 B、Table +27 B、Type +30 B。Function の数は 104,586 → 105,364(+778)、表は 1 → 8(fle・b2d・qfc・ft・ilt・oggio・elastique の 7 つ。dsp は間接呼びが無いので無し)。
- **0285 の報告との差(訂正)**: 私は 0285 の実装の報告で『-Oz の後で +197,374 B』と書いた。それは各 module の `wasm-opt -Oz` だけを単独で走らせた値(上の表の 8 module の計 +201,235 B は CI の fe-wasmgen のフラグ〔`--static-calls --sync-saved --sse-int` ほか〕での値で、私の値 +197,374 B は onez.sh の引数での値。同じ種類の量で、フラグの違いで 4 KB ずれる)で、**CI の段階で merge した最終の wasm は +310,318 B**(module の計より +109 KB 多い。多い分の原因は未確認: merge の後の Code の増えかたで、adapter の本体が wasm-merge で重複を失う等の読みはあるが、確かめていない)。**S1 の判定は +310,318 B を使う**: 143,086,598 B は線 154,885,888 B の内(残り 11,799,290 B)。first idle の悲観(0.53 ms/KB × 303 KB)は **+161 ms**(0.57 で +173 ms)、node の eager Liftoff の実測比(読みの 1/9〜1/40)で **+4〜18 ms**(推定)。0286 の +310,645 B と同じ大きさで、0286 は同じ扱い(悲観値は実測の ci1 の余裕の内)。**ブラウザの first idle は未測定**。
- 再現性: 1 回目と 2 回目(やり直し)の `flengine6.opt.wasm` の sha256 が一致(`49bbf63d…eac`)。
### 1.6 1 回目の走行が『失敗』と出た理由(私の操作)
1 回目の `run-ci-build.sh`(felift link)は、全部の段階が終わった後に `build-engine-bundle.sh: 行 453: 予期しないトークン ')' 周辺に構文エラー` で rc=1 になった。**原因は私が走行中に同じファイルを書き換えたこと**(`hyb_guard` を 7 アドレスに広げる編集を、実行中の `build-engine-bundle.sh` に加えた。bash はスクリプトを読み進めながら実行するので、バイト位置がずれて末尾の解析が壊れ、段階が一部 2 回走った)。成果物は 1 回目の時点で出来ていた。**2 回目(木を触らずに `link` だけ、hybn0 → hybrid、wasmopt と weblink は成果物があるので省かれる)は rc=0、`link done in 529 s`**。この 2 回目が、広げた 7 アドレスの `cpart` を含む本物の走行。

## 2. 0284 の A(`0284b`): S-cmp-symbol-vs-symbol・S-cmp-space-as-symbol・S-cmp-control を表の重みで置換

差分: `0284b-table-replaces-s-choices.diff`(基準 e10-wip bb05de13、`patch --batch -p1 --dry-run` が offset・fuzz なしで通る。`fle_locale.c` +20 行と `wall_ia2.rs` +97 行、**`-` の行は 0**)。拡張: `0284b-gp-extension.diff`(A の上に載せる 15 行。§2.6)。

### 2.1 何を変えたか(読み)
`fle_locale.c` の `CompareStringW` の最初の差の位置で、**S-cmp-symbol-vs-symbol(記号 対 記号の code point 順)・S-cmp-space-as-symbol(空白 対 記号)・S-cmp-control(TAB〜CR 対 印字可能 ASCII)の 3 つの規則の前に**、`fle_ia2_on()`(= 本番の全 build)で、その対(記号/空白どうし、TAB〜CR 対 0x20〜0x7e)なら Microsoft の Sorting Weight Table の primary weight(SM、AW)で決める 1 つのブロックを置いた。表が決められない対(U+0000、primary weight が等しい 2 単位)は**元の規則がそのまま答える**。3 つの規則の本体は消していない(§2.7)。symbol 対 英数字(ページの規則)・ハイフン/アポストロフィ(ページの規則)・一般句読点(§2.6)・英字どうし/数字どうしは触っていない。

### 2.2 表と code point 順が違う対(実測 + 表の行から)
`enum_cmp single`(1 単位の文字列どうしの全対: ASCII 128 + 非 ASCII の代表 32 の 22,650 順序対、flags 0 と 1)を e10-wip と A で走らせて比べた(`tables/enum-single-*.txt.gz`、`tables/0284b-changed-ordered-pairs-enumerated.tsv`):
- **答えが変わる順序対は flags 0・1 のどちらも 116**。内訳: **記号 対 記号 106**(= 53 の無順序対)+ **TAB(0x09)〜CR(0x0d) 対 空白 10**(= 5 の無順序対 × 2 向き)。PM の依頼の『111 組』は 106 + 5(後者を 1 向きだけ数えた値)。
- **停止 → 答え、答え → 停止はどちらも 0**(A は答えられなかった対を増やさず、答えていた対を停止にしない)。他の 22,534 対は完全に同一。
- 原因は表の構造: ASCII の記号は SM 7 に `!"#$%&()*,./:;?@[\]^_` `{|}~`、**SM 8 に `+ < = >`**(数学記号の群)。SM 8 は SM 7 のどれよりも大きいので、`+` `<` `=` `>` が code point で小さい側にある対(`+` 対 `,` `.` `/` `:` `;` `?` `@` `[` `\` `]` `^` `_` `` ` `` `{` `|` `~`)が全部逆になる。SM 7 の内側でも AW の順が code point と違う対がある。**TAB〜CR は SM 7 で AW 5〜9 、空白は AW 2**(表の `0x0009〜0x000d` と `0x0020` の行)なので、空白のほうが小さい(旧: TAB〜CR が空白より小さい)。
- 全 58 の無順序対と、表の (SM, AW) は `tables/0284b-changed-pairs.tsv`(表の行から作った。ランタイムの出力ではない)。例: `+`(8,3)対 `,`(7,47) は 旧 1(CSTR_LESS_THAN)→ 新 3、`\t`(7,5)対 ` `(7,2) は 旧 1 → 新 3。

### 2.3 現れうる画面(読み。実エンジンの確認なし)
ゲストで `CompareStringW` を呼ぶのは 6 関数(0284 の第 1 報 §6、`f_4231c0` の thunk の caller の静的な走査): `0x4399ca`(f_439970: lcid 0x400 / flags 0 の Delphi の文字列比較)、`0x439a3d`(f_4399e0: `AnsiCompareText` = NORM_IGNORECASE、**直接の呼び出し元 37**)、`0x44ecfb`(f_44ec40: TStringList 系、flags は呼び出し側)、`0x4558f3`、`0xc0708b`、`0xc07112`(`AnsiCompareText` の別の経路。FL 側のコード)。**変わる対は『最初に違う位置の 2 単位が、上の 53 + 5 の対のどれかである 2 つの文字列』の順序だけ**で、現れうるのは次の種類(いずれも読み):
- ソートされる一覧(Browser の項目名・プラグイン名・ファイル名・プリセット名など `AnsiCompareText` と TStringList.Sort に渡る名前): 『`Name+1`』対 『`Name,1`』のように、共通の接頭辞の後が `+ < = >` 対 他の記号、または SM 7 の内側で順が違う 2 記号の対。ハイフンとアポストロフィはページの規則(別)で、この対に入らない。
- エラー窓の Enter/Tab(決定 0136): キー文字 `\r`(0x0d)と `&` 以外のアクセラレータ文字の比較は、`\r` 対 英字なので 旧と同じ(変わるのは `\r` 対 空白だけ)。
- ファイルパスの比較: `\`(SM 7)と `.` `/` などの記号どうし。パスの区切りの `\` 対 `+ < = >` の対は変わる。
**数えられていない物**: 実際の FL の一覧(FL のデータフォルダの名前)の中に、最初の差が上の 53 対になる 2 名があるか。インストール済み製品のファイル名の一覧を持っていないので、この値は**未測定**。

### 2.4 node の行への影響の予測(読み)
| 行 | 動くか | 理由 |
|---|---|---|
| progress / slices / snap clock_ms(g3-progress)| **動かない見込み** | 比較の回数も、間接呼びの回数も、答えを使う分岐の回数も、答えが変わる 116 対に当たらなければ同じ。当たれば、一覧の並びが変わり、その後の描画・走査の経路(間接呼びの数)が変わりうる |
| g3-surface(Browser・ListView の画素)| 上の 53 対が当たる一覧が画面にあるときだけ | 並びが入れ替わる = 画素が変わる。起動 16 スナップ(84 行)と参照プロジェクト 55 行の画面は、記号で始まる項目が少ないので当たらない見込み(未確認)|
| g3-wintable / lid の大きさ | 動かない | 窓の構成は比較の答えに依らない |
| Export の sha(音声)| 動かない | 音声の経路は文字列の比較を使わない |
| first idle | 動かない | 比較は起動の主な経路ではない(A は条件が 1 つ増えるだけ)|
opt1 が E-次で上の 5 行を E10 と比べる。**動いた行があれば、その行の画面の一覧に 53 対のどれかが入っている**。

### 2.5 テスト(手導出の再基準化)
- 新しいテスト `wall_ia2.rs` `compare_string_orders_the_ascii_symbols_by_the_table_and_not_by_the_code_point_with_the_marker`(マーカーあり): 表と code point 順が違う **53 + 5 の対**(全部、1 向きずつ)と、両方の順が一致する 12 対の期待値を、`Windows10-SortingWeightTable.txt`(SHA-256 81fcfa1e…)の (SM, AW) の行から導いた(python、ランタイムの出力は使っていない)。マーカー無しでは旧(`+` 対 `,` = 1、TAB 対 空白 = 1)。
- **突然変異の確認**: `FLE_IA2_RT_DIR` で e10-wip の `fle_locale.c` に差し替えると、このテストは落ちる(`u2b-u2c` で 1 と 3 が逆)。A の下では通る。
- **既存のテストは期待値の書き直しが 0 件**(A は ia2 が有効な構成でだけ答えを変え、その構成のテストは `wall_ia2.rs` だけで、そこの既存の記号対は表と code point が一致する対だけを使っていた): `wall_ia2` 21 passed(新 1 + 既存 20)。`test.sh full`: (キャッシュなしの走行は進行中。結果は最終報告)。経緯: 1 回目は /tmp の tmpfs が満杯(私の作業ファイルが 2.3 GB)で `Disk quota exceeded`(24 件の失敗、`tables/testfull-A-run1-tmpfull.log`)、2 回目は私の部分的な export(lift だけ)に repo の `tools/` が無くて authenticode_check が落ちた(14 件)。どちらも A と無関係の環境の問題で、/tmp を空け `tools/` を足した 3 回目は results=217 passed=2405 failed=0
- a4: (進行中)

### 2.6 一般句読点(S-cmp-general-punctuation)の扱い — 拡張を別の差分に
依頼は記号対記号と control の 2 つだったので、**一般句読点(U+2000〜U+206F)対 ASCII 記号の規則は A に入れていない**。表と比べると、この規則も code point 順で、**156 順序対(78 の無順序対)で表と逆**(en/em ダッシュ U+2013/U+2014 は SM 6 で、空白と SM 7 の記号より小さい。`+` 対 U+200B なども)。`0284b-gp-extension.diff`(15 行、A の上)がその置換で、答えが変わる順序対は A と合わせて 272(= 116 + 156)。既存のテストの期待値(`wall_fr12` の 4 行: `-` 対 U+2013 = 1、` ` 対 U+2026 = 1、`'` 対 U+2019 = 1 ほか)は拡張でも変わらない(実測: `./enum_A one` で確認)。**採否は PM の未決**。

### 2.7 3 つの旧規則の本体の扱い
A の下で、本番(`fle_ia2_on()` が常に真)では 3 つの規則に到達する対は『表が決められない対』(U+0000 と、primary weight が等しい 2 単位)に限られる。ASCII の記号・空白・TAB〜CR の全対では表が決めるので(§2.2 の 0 件の増減)、**実質は死んだコード**になる。本体を消すのは、0284c(B = ia2 の無条件化)の後が妥当(マーカー無しの構成が消えるため)。今は消さない(消すと B 無しの構成のテストが停止を期待する形に変わる)。

## 3. 後片付け(依頼 (3))

- 削除済み(`readlink -f` で確認してから): N1 のバイナリ(188 MB。`tools/build85.sh` で再生成可)、`w86`、`ct86`(`~/work/mt3w-ct86` 854 MB)、`tmp/regen`、w87 の複製 docs(192 MB)、生成 wasm(`a85/impl/*.wasm`)。
- 作業の終わりに削除(実施済み、`readlink -f` で確認してから): `~/work/mt3w-ci11`(CI の作業ディレクトリ、約 1.9 GB)、`~/work/mt3w-e11`、`~/work/mt3w-w88`、`~/work/mt3w-ct88`・`ct87`・`ct79`(cargo の target、計 4.3 GB)、`~/work/mt3w-gen`(1.7 GB)、`lift/out-mt3y/{w79,w87,a73,a75,a84,a85,a92,tmp,dll}` とそれらへのリンク(ct87・w88)。結果は `lift/out-git/mt3w-results/` にある。`/tmp/opencode/m85` は 8 MB(私の作業ファイルが /tmp の tmpfs を 2.3 GB 使い、他の担当のテストが `Disk quota exceeded` になりうる状態だった〔私の A のテストが 1 回それで落ちた: §2.5〕ので、早めに空けた)。
- `lift/out-mt3y` は現在 140K(上限 500 MB 内)。

## 4. 未確認
1. 冷間の C 部の再 link(`objects` を含む)。C のソースが E10 と同じことに頼って、E10 の objects を再利用した。冷間 CI でだけ起きる物(objects の key、cache の復元)は見ていない。
2. 実エンジン(node・ブラウザ)での動作: 0285 の progress/slices/snap の行、Export の sha、60 s 再生(opt1 の E11)。この再現は『段階が通ること、guard が通ること、wasm が V8 で受理されること』まで。
3. A の実エンジンの行への影響(§2.4 は読み)。インストール済み製品のファイル名の一覧が無く、53 対に当たる 2 名があるかは数えていない。
4. 最終の wasm の増分が module の計より 109 KB 多い原因(§1.5)。

## PM の決定と取り込み(2026-11-06 13:50、docs のみ)

取り込み: mt3 の決定 0292(first report を兼ねた最終)の文書・表・道具と、差分 `0284b-table-replaces-s-choices.diff`・`0284b-gp-extension.diff`(`0285b` は決定 0285-impl の側に取り込み済み)を `docs/investigations/mt3w/` に置いた。実行したのは CI の `stage-wasm.sh link`(実物のスクリプト)を e11-wip の木で 1 回(C 部の objects は E10 の冷間ビルドを再利用)と、native の C 経路(`enum_cmp`、test.sh full、a4)で、**ブラウザ・node の実エンジンは 0 件**。依頼者の機械は 0 件。

判断:
1. (1) **E11 の冷間 CI で初めて通る段(link map の新 3 行、exports.rsp の `fle_hyb_slice`、hyb_guard)はローカルで通った**。最終 wasm は E10 + 310,318 B(143,086,598 B、+0.217%)。0285 の報告の +197,374 B は module 単独の −Oz の合計で、CI の merge を通すと +310 KB(Code +295,855、Elem +13,598)。S1 の線の内。`0285b`(hyb_guard を 7 番地に)は採用済み(e11-wip aa56ac4c)。
2. (2) **0284b(A)を採る**: 表(Windows 10 Sorting Weight Table)の primary weight で決めるブロックが `fle_ia2_on()` かつ記号どうし/空白どうし/TAB〜CR 対 ASCII の対だけに働き、表が決められない対は旧規則が答える。順序が変わる対は 116(記号 106 + TAB〜CR 対 空白 10)、停止 → 答え / 答え → 停止は 0。期待値は表から手で導出し、突然変異(旧 fle_locale.c に差し替え)で落ちる。test.sh full 217 results/2,405 passed/0 failed(キャッシュなし、213 実行体)、a4 942/942・895/895・5,310・1,009,737。**E12 に 0284c と一緒に載せる前に、opt1 が FS 付きテストビルドで 0284c と 0284b を別々に node の行へ当て、動く行の原因を分ける**。
3. **`0284b-gp-extension`(一般句読点、表と逆の順序対 156)は保留**(E13。A の node 行が安定した後)。事実として記録: en/em ダッシュは SM 6 で空白より小さい、など、code point 順の source 無しの選択が表と逆。
4. 順序: E10(0286 + 0284、opt1 の実測待ち)→ E11(0285 + 0285b)→ E12(0284c + 0284b)→ E13(gp-extension)。
5. 未確認: 冷間の C 部の再 link(objects の key、cache の復元)、実エンジンでの 0285 の行(opt1 の E11)、0284b の node 行(g3-surface が「一覧に 53 対のどれかがある」ときだけ動きうる、という予測は読み)、最終 wasm の増分が module の −Oz の合計(+201,235 B)より 109 KB 多い原因。
