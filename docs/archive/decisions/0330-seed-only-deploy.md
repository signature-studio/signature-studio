# 0330 seed の行を足すだけの改善(BigFruit.png・AboutTxt.txt・msgothic.ttc)を本番に早く出す方法 — 調査(gpu1/gpu8、2026-11-07 10:2x JST)

**実際に実行した物(読みのみ + repo の CI script の純関数の実行)**: PM の木は**書いていない**。`git grep` / `sed` で `.github/workflows/deploy.yml`、`ci/*`、`app/scripts/build-engine-bundle.sh` を読んだ。実行したのは(1)`git archive HEAD`(ci・app・tools・lift/harness・lift/runtime・lift/crates ほか 25 MB)を**私の作業用の複製**(`~/work/gpu8/ciplan`、新しい git repo)に展開して、`ci/plan.sh`(各 stage の cache key を『入力のハッシュ』から作る純関数)を 4 つの状態で走らせ、key が変わる stage を比べた、(2)`ci/needs.sh`(cache の hit/miss から『どの stage を走らせるか』を出す純関数)を hit/miss の組で走らせた、(3)`ci/compare-bundle.sh` を mt3 の RT-A と seed2b の bundle の対に走らせた。**build・deploy・実エンジン・ブラウザは 0**。数字のうち『25〜45 分』は `ci/README.md` の記述(標準 runner で測った値)の引用で、私は測っていない。

## 結論(先頭)
1. **通常の push + CI の deploy で出せる。ただし今の `ci/plan.sh` のままだと『seed だけの変更』にならず、lift・wasm の objects・link を含む全段の冷間 build(README: wasm の入力が変わると 25〜45 分)になる。** 理由は key の入力の取り方が粗いこと(下 §1、実行して確認済み)。
2. **`ci/plan.sh` の 3 行(差分 `diffs/0330-plan-seed-only-keys.diff`、32 行、未適用)で、seed の資材だけの変更なら extract・seed・bundle・deploy だけが走り、lift・wasm・link は cache の hit になる**(私の複製で key を実行して確認。§1.3)。ただしその差分を入れる push 自体は **1 回だけ冷間 build**(key の入力の集合が変わるため)。**次に lift/runtime を変える push(RT-A/RT-B)が冷間 build になる予定なら、その push に差分と seed の行を一緒に入れれば追加の費用は 0**。
3. 本番がどの bundle を使うかは **delivery bucket の `manifest.json`(既定の manifest)**。workflow が『bundle の files → Worker の新 version → 既定の manifest を最後に』の順に自動で上げ、手動の手順は無い(§2)。
4. wasm・glue・DLL pack が同一で seed pack と manifest だけが違うことの検証は `ci/compare-bundle.sh` + manifest の平坦化の差分(mt3 の RT-A ↔ seed2b の対で実行して確認、§3)。
5. 資材: `BigFruit.png`・`AboutTxt.txt` は installer の中のファイル(今の 218 個と同じ扱い)。`msgothic.ttc` は Noto Sans CJK JP の subset(OFL 1.1)で、repo の `lift/harness/seeds/os-fonts/` に既存の DejaVu・Liberation と同じ形(font + COPYRIGHT/OFL + README の表)で置く(§4)。**`Fruity microfont.ttf` は入れない方がよい**(§4.3)。

## 1. cache key: seed だけの変更はどの stage の key を変えるか
### 1.1 コードの事実(`ci/plan.sh`、`ci/README.md` の表)
`hashpaths` は `git ls-files -s -- <paths>` の blob id のハッシュ(`ci/lib/common.sh`)。各 key の入力(抜粋):
| key | 入力 | seed の資材が入る所 |
|---|---|---|
| `k_extract` | installer の SHA-256 + `H_EXTRACT_TOOLS`(`extract_fl.py` `extract-dlls.tsv` `extract_nsis2.py` `nsis_script_dump.py` `gen_install_tree.py` **`lift/harness/seeds/vfs-seed-idle1.tsv`**) | tsv を直接 |
| `k_lift` | **`$KEX`** + `$KFE` + `H_GEN`(**`lift/harness` 全体**〔`web/` 除く〕+ 2 つの docs/investigations) + `SP_LIFT` | KEX 経由で tsv、`H_GEN` 経由で `seeds/` の全部 |
| `k_rt`(wasm objects)、`k_link` | `$KLIFT` + `H_WASM`(`lift/runtime` + **`lift/harness` 全体**〔`web/` 含む〕+ tools)+ `SP_WASM` + tools | KLIFT 経由、`H_WASM` 経由で `seeds/` と `web/final0129/pack_seed.py` |
| `k_seed` | `$KEX` + `$KLIFT` + `H_SEED`(`pack_seed.py` + `lift/harness/seeds`)+ `SP_SEED` | 直接 |
| `k_bundle` | `$KLINK` + `$KSEED` + `H_BUNDLE`(`pack-assets.mjs`)+ `SP_BUNDLE` | |
**lift と wasm の stage が実際に読む seed 系のファイル**は `ship6-imports.txt`・`hyb-U-*.txt`・`flengine-seeds-thunks.txt`・`ship-hot-units-F2.tsv` などで、`vfs-seed-idle1.tsv` `reg-seed-*.tsv` `authenticode-claims.tsv` `os-fonts/` を読むのは **`stage_seed`(`pack_seed.py`、`build-engine-bundle.sh` の 399〜400 行)と extractor(`extract_fl.py`)だけ**(`git grep` で確認)。key の入力がこれより粗い。

### 1.2 実行して確認(私の複製、`ci/plan.sh` をそのまま)
seed の行 2 本(BigFruit.png と msgothic.ttc)を `vfs-seed-idle1.tsv` に足し、`os-fonts/` に `msgothic.ttc` と OFL・COPYRIGHT を足しただけの commit(コードは無変更)で比べた:
| stage の key | 今の plan.sh での変化 |
|---|---|
| extract / lift / rt(wasm objects)/ link / seed / bundle / prefix | **全部 CHANGED** |
| felift / b2d / app / vst3 | same |
`ci/needs.sh` に『bundle・link・seed・lift・extract が全部 miss』を入れると `run_lift=true`、`need_link=true`(wasm objects と link を走らせる)、`need_b2d`、`need_emsdk`。= **冷間 build**(README: 標準 runner で 25〜45 分、2 vCPU・8 GB)。

### 1.3 差分を入れた場合(`diffs/0330-plan-seed-only-keys.diff`、`ci/plan.sh` だけ、32 行)
内容: (a)`H_EXTRACT_DLL`(tsv を除いた extract の入力)から `KEXD` を作り、`k_lift` は `KEX` ではなく `KEXD` を使う、(b)`H_GEN` と `H_WASM` から seed の資材(`vfs-seed-idle1.tsv` `reg-seed-nsis.tsv` `reg-seed-app.tsv` `authenticode-claims.tsv` `os-fonts/` `web/final0129/pack_seed.py`)を `:(exclude)` で外す。`KEX`(tsv を含む)は extract stage の key として残す(guest files は seed の入力だから)。
実行した結果(私の複製):
| 比較 | extract | felift/b2d | lift | rt | link | seed | bundle / prefix |
|---|---|---|---|---|---|---|---|
| 今の plan.sh → 差分を入れた plan.sh(資材は無変更) | same | same | CHANGED | CHANGED | CHANGED | CHANGED | CHANGED |
| **差分を入れた plan.sh で、seed の資材だけ変更** | **CHANGED** | same | **same** | **same** | **same** | **CHANGED** | **CHANGED** |
`ci/needs.sh` に『bundle miss・link hit・seed miss・lift hit・extract miss』を入れると: `need_link=false`、`run_lift=false`、`restore_lift=true`、`restore_link=true`、`run_extract=true`、`need_seed=true`、`need_felift/b2d/emsdk=false`。= **extract(installer を artifact bucket から取って展開)→ seed → bundle → deploy だけ**(needs.sh の既存の規則『seed hit and link hit → only the bundle step runs』の seed miss 版)。
- 1 行目の通り、差分を入れる push は **1 回だけ冷間 build**(lift・rt・link の key が変わる)。以後の seed 資材だけの push が軽くなる。
- **未確認**: deploy.yml の build job の各 step の `if:` が needs.sh の出力に正しく従って、extract・seed・bundle だけで通るか(workflow は実行していない。コメントの規則は読んだが、step の定義は全部は読んでいない)。差分を入れる push の後の最初の seed-only push で確かめる。extract stage の所要時間(installer の取得 + 展開)は未測定。

### 1.4 mt3 の seed2 の作り方との対応
mt3 の `seed2-build.sh` は RT-A の work dir のコピー(wasm・glue・DLL pack は hard link)に対して CI の `seed` と `bundle` の段だけを走らせた(= 上の『軽い経路』を手で再現)。**これは workstation のテスト版で、本番に出す経路ではない**(`ci/README.md`: 『There is no deploy from a workstation: the workflow is the one path (decision 0243)』)。

## 2. 本番がどの bundle を使うか、新しい bundle が出るまで
- 既定は **delivery bucket(`fl-web-assets`)の key `manifest.json`**(Worker は `/assets/*` を R2 から返す)。bundle の files は prefix `fl-engine-ci-<k_bundle の末尾 8 桁>/` の下(`plan.sh` の `prefix=`)。manifest の URL がその prefix を指す。
- 手順(`.github/workflows/deploy.yml` の `deploy` job、全部 `github.ref == main` の時だけ実行): ① bundle を cache から restore → ② `stage-deploy.sh plan`(SHA256SUMS・大きさ・manifest が名指す key を検査)→ ③ `upload … objects`(**既定の manifest 以外の全部**を S3 API で)→ ④ lifecycle(perf/)→ ⑤ Worker の新 version(`wrangler versions upload` + `versions deploy @100%`)→ ⑥ **`upload … switch`(既定の manifest.json を最後に上げる)**→ ⑦ Access の probe。
- 起動: `push` to `main`(`docs/**` と `**/*.md` だけの push は除く)か、`workflow_dispatch`(入力なし。main なら deploy、他の branch は dry run)。**自動**で、手動の切り替えは無い。concurrency は `deploy`(直列)。
- bundle が cache の hit なら build job は走らない(`bundle hit → no build job`)。seed だけの変更は §1.3 の通り bundle が miss。
- 1 回の deploy で bundle の全 files を新しい prefix の下に上げ直す(br で wasm 約 39.5 MB・DLL pack 約 33.9 MB・seed 約 8〜10 MB)。content-hash 名の file も prefix が違うので重複して上がる(R2 の保管量は増える。古い prefix の掃除は deploy.yml では見ていない = 未確認)。

## 3. 『seed と manifest だけが違う』ことの検証(ハッシュ)
- **`bash ci/compare-bundle.sh <旧 bundle の dir> <新 bundle の dir>`**(既存): wasm・glue・DLL pack・seed pack と、それぞれの brotli の SHA-256 を role ごとに比べる。私が mt3 の RT-A(`fl-engine-test-rt1-91bf78f6`)と seed2b(`fl-engine-test-rt1-seed2-8128f5be`)に実行した結果: dll・dll.br・glue・glue.br・wasm・wasm.br は **yes**(16 桁の先頭まで一致: `da039e8017d7ffc3` `3fd846d8ba531826` `7f41b3a3474dc94a` `32a1c7fb98a59a68` `090789381f146cee` `0a5a00696f8850d2`)、seed と seed.br は NO(`8e8bbd1ea7fe7821 → 7d762565049176a2`、`d3d1312daee96a9c → 914e95216a4300a6`)、manifest は NO。
- manifest の中身の差(同じ 2 つを平坦化して比べた): `bundle.created`、3 つの URL の prefix(`dllPack.url` `engine.options.glue.url` `engine.wasm.url`)、`seedPack.{url,size,sha256,encodedSize}` だけ。`bundle.version`(= wasm の hash の先頭)と `engine.args` `engine.env` は同じ。
- 本番(`fl-engine-ci-0363b050`)に対して同じ検査をするには、本番の bundle dir と、新しい CI の bundle(restore したもの)を渡す。**期待は『wasm・glue・DLL pack は yes』だが、冷間 build をやり直す経路(今の plan.sh のまま)では wasm の再現性(同じ入力で同じ bytes か)が前提で、それは未確認**。差分を入れた経路なら wasm は cache から restore なので同一が保証される(同じ key の物をそのまま使う)。
- 補助: mt3 に『本番(E13)と同じ入力で seed の行だけを足した bundle』を作ってもらい、本番の bundle との `compare-bundle.sh` を取っておくと、deploy 前に PM が差を確認できる。

## 4. 資材・licence・risk
### 4.1 資材の出どころ
| file | 出どころ | repo に置くか |
|---|---|---|
| `Artwork\Skins\Default\BigFruit.png`(166,508 B)、`System\Texts\About\AboutTxt.txt`(9,228 B) | installer の中(extractor が `vfs-seed-idle1.tsv` の行〔guest path・host path・SHA-256〕で取り出す。今ある 218 個と同じ。mt3 の実行: 『9 DLL + 229 seed file written, all SHA-256 verified』) | **置かない**(tsv に行を足すだけ。host path は `lift/out-idle1/inputs/install/…`) |
| `C:\Windows\Fonts\msgothic.ttc`(2,123,288 B、SHA-256 `daa1948fcd6cddc9549249d79ef075f6001c3693bf2a61ad8717679d7fb80b2f`) | Noto Sans CJK JP Regular の subset(7,171 glyph、cu2qu で TrueType 輪郭に変換、1 face の TTC に包む)。**MS Gothic ではない**(opt1 の `fonts-pkg`、FreeType 2.13.2 で開いて確認済み) | **置く**: `lift/harness/seeds/os-fonts/msgothic.ttc`(既存の DejaVu・Liberation と同じ置き場。`extract_fl.py` が `os-fonts/` の行を checkout からコピーし SHA-256 を検査する) |
### 4.2 licence(OFL 1.1)
- Noto Sans CJK JP は SIL OFL 1.1。opt1 の `fonts-pkg/COPYRIGHT.txt` に、元の font(Version 2.004、Adobe)・Reserved Font Name は**宣言なし**・変更点(subset、outline 変換、hinting と layout tables の削除、TTC 化)・商標(Noto は Google)が書いてある。font の name table の nameID 0・7・9・10・13・14(著作権・商標・ライセンス)は**変えていない**(同 doc)。
- 置き方(既存の `os-fonts/README.md` の形): ① `os-fonts/msgothic.ttc`、② `os-fonts/COPYRIGHT-msgothic-substitute.txt`(opt1 の `COPYRIGHT.txt`)、③ `os-fonts/OFL-noto-sans-cjk-jp.txt`(opt1 の `OFL.txt`、Sans/LICENSE の verbatim)、④ README の表に 1 行(guest path・file・font・SHA-256・選択のラベル。『substitute であり MS Gothic ではない』を明記。DejaVu・Liberation と同じ書き方)。
- **配布する copy(seed pack で利用者のブラウザに届く font)にも licence を付けるか**は PM・依頼者が決める事項: OFL は『each copy contains the above copyright notice and this license』を求め、機械可読の metadata(name table)でよいとされる〔読み。法務の判断ではない〕。念のため seed pack に OFL 全文を guest から見えない path(例: seed の中の `licenses/`)で同梱するか、サイトの licence 表示に載せる案がある。
### 4.3 `Fruity microfont.ttf` は入れない方がよい
- Image-Line の font(installer の中)。seed2 の実験で入れた 3b・3c(`Shared\Artwork\Fonts\` と `C:\Windows\Fonts\` に同じ bytes を置く)は **repo の `os-fonts/Fruity_microfont.ttf` のコピー**を経由した(mt3 の doc)= **Image-Line の font を repo に入れることになる**(既存の `os-fonts/README.md` は『`Fruity microfont.ttf` はあえて代用しない』と書いている)。
- **O3・O16 の解決にはこの 3 行は要らない**(O3 = BigFruit.png + AboutTxt.txt、O16 = msgothic.ttc。私の確認では 3 つ全部を入れた bundle で両方直ったが、microfont の寄与は切り分けていない)。効果が測れていない行は本番の変更から外すのが安全。入れるなら、installer の path(3a)だけを使い、別 path へのコピーは extractor に『同じ install tree の member を別の guest path に置く』行を足してから(repo に font を置かない)。
- もう一つ: mt3 の seed2 の最初の版は `Default news.rss`・`Layout Piano.scr` の guest path を重複して足し、loader が拒否した(engine が exit 2)。**mt3 の `0315-pack-seed-duplicate-guest-path.diff`(pack_seed.py に重複検査、8 行)を seed の変更と一緒に入れる**のがよい。§1.3 の差分は `pack_seed.py` を lift/wasm の key から外してあるので、その差分を入れても冷間 build にならない(差分を入れない場合は `pack_seed.py` の変更も H_WASM に入っていて冷間 build になる)。
### 4.4 P1(first idle)への影響の見積り
- **訂正(0330b §6)**: 下の +1.59 MB は mt3 の品質 1 の値。**本番の既定は品質 11**で、S の seed は +1,300,446 B(品質 11、実際に pack を作って測定)。
- **転送量**(mt3 の実測、br): RT-A の起動時の br = wasm 39,486,162 + DLL pack 33,862,064 + glue 106,995 + seed 8,240,342 = 81,695,563 B。seed2b は seed が 9,826,130 B(+1,585,788 B)→ 83,281,351 B(**+1.94%**)。raw の seed は +2,341,164 B(14,748,394 → 17,089,558)。
- **算術**(推測): 追加の 1.59 MB は、1 Gbps で約 13 ms、100 Mbps で約 127 ms、50 Mbps で約 254 ms。seed は wasm の download と並行なので、最大の download(wasm、39.5 MB)より短い間は first idle に出ない可能性が高い(page の download は並行、`timeline` の download_start → download_done)。CPU 側: page が seed pack の SHA-256 を検証(+2.3 MB で約 10 ms 以下)、brotli の復号(ブラウザ)+ MEMFS への展開(数 ms)、`msgothic.ttc` を FL が開く時の FreeType(未測定)。メモリ: +2.3 MB(wasm の約 1 GB に対して無視できる)。
- **実測**: 私の実 Chrome の first idle は seed2b の About の run 8.24 s、ja の run 7.79 s(診断入り・別の page・load 違い、RT-A の seed なしの対照は同じ条件で測っていない)で、**+2.3 MB の影響の目安にはならない**。測るなら ci1 の静かな窓で、同じ page・同じ engine(RT-A)の seed なし/ありを交互に n ≥ 5 ずつ、first idle の中央値と p90、MemAvailable と swap の記録つきで。
- 他の risk: seed の loader は同じ guest path の重複を拒否する(上)。`msgothic.ttc` の guest path は Windows の Fonts にあり、FL の font 選択が拾うかは O16 の run で『日本語の字形が出る』ことは確認したが、**拾った経路(CreateFileW/ReadFile)は engine log で未確認**(診断は miss しか出さない)。負の対照(同じ ja FLP を seed なしの RT-A で開く)も未実行。

## 5. 選択肢と推奨
| 案 | やること | 初回の費用 | 以後の seed 資材だけの push |
|---|---|---|---|
| A. 何も変えずに push | tsv の行 + `os-fonts/` の font と licence + README の表 | 冷間 build(25〜45 分、標準 runner。失敗の risk〔memory・time〕は README の範囲) | 毎回冷間 build |
| **B. `plan.sh` の差分 + seed の行を同じ push に** | A + `diffs/0330-plan-seed-only-keys.diff` | 冷間 build 1 回(A と同じ) | extract + seed + bundle + deploy(§1.3、所要は未測定) |
| C. RT-A/RT-B の push に相乗り | B を、次に冷間 build になる push(runtime の変更)に入れる | **追加の費用 0** | B と同じ |
推奨は **C**(それまで待てるなら)、急ぐなら **B**。A は『一度きり』で足りる時だけ。どの案でも、deploy の前に `compare-bundle.sh` を本番の bundle と取ること(§3)と、mt3 の重複検査(§4.3)を入れること。
