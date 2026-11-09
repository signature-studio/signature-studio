# 0330b seed だけの変更一式 S(O3 のロゴ・staff roll、O16 の日本語を本番で出す)— push できる形(gpu1/gpu8、2026-11-07 10:5x JST)

**実際に実行した物**: PM の木は**書いていない**。`git archive HEAD`(d78f3c98)の部分複製を**私の木** `~/work/gpu8w/seedpush-tree`(新しい git repo)に展開して S を作り、(1)`patch --dry-run` / `git apply --check` を新しい `git archive HEAD`(`ci` `lift/harness`)と mt3 の `tree-rtb` の複写に実行、(2)`ci/plan.sh`・`ci/needs.sh` の純関数を base / S / 『S の後の seed だけの変更』で実行、(3)`ci/test-hybrid.sh`・`ci/test-needs.sh`・`ci/test-real-deploy.py` を実行(S に新しい検査 3b を足し、旧 `plan.sh` に対する**負の対照**で FAIL を確認)、(4)repo の `pack_seed.py`(S の重複検査つき)と S の tsv と **mt3 の抽出済みの host**(`ci-rt1s/host`、読み取りのみ)で seed pack を実際に作り、mt3 の native の loader(`vfs_seed.h`)に通した、(5)`.github/workflows/deploy.yml` の build job の各 step の `if:` を読んだ。**build(emcc・wasm)・deploy・実エンジン・ブラウザ・GitHub Actions の実行は 0**。installer からの抽出は私は走らせていない(mt3 の抽出結果を読み取りで使った)。
**push は私はしない**。PM が検証して push する。

## 0. 先頭の結論
1. 変更一式 S = 5 ファイルの差分(`diffs/0330b-seed-only-change.diff`、261 行)+ **バイナリ 1 つ**(TTC、下の表)。`git archive HEAD` に `patch -p1 --dry-run` rc 0・`.rej` 0・`git apply --check` rc 0。mt3 の `tree-rtb`(tsv の末尾に Fruity の DLL 行 3 本が増えた版)の対応する 5 ファイルにも rc 0。
2. **S 自身を push した時は冷間 build**(lift・wasm objects・link の key が変わる。README: 25〜45 分)。**S の後の seed だけの変更**(tsv の行や os-fonts の font)は extract / seed / bundle / prefix の key しか変えず、`needs.sh` は extract → seed → bundle だけを走らせる(実行して確認)。
3. seed の数や内容を数えている試験は**見つからなかった**(`git grep`、下 §3)。更新が要るもの 0。代わりに S に**新しい検査(`ci/test-hybrid.sh` の 3b)**を足した(『seed の資材だけの変更は lift・rt・link の key を変えない』を pin。旧 `plan.sh` に対しては FAIL する = 負の対照)。
4. S で作った seed pack は native の loader が受理(rc 0、243 files・47 directories、BigFruit.png・AboutTxt.txt・msgothic.ttc の 3 行が seed される)。
5. 未確認: GitHub Actions の実行(step の `if:` は読んで needs.sh の出力と 1 対 1 に対応することを確かめただけ)、installer からの本物の抽出(mt3 の抽出結果を使った)、本番の bundle との `compare-bundle.sh`(S の後の最初の push の後で)。

## 1. S の中身
| ファイル | 変更 | 出どころ |
|---|---|---|
| `ci/plan.sh` | +8 −3: `H_EXTRACT_DLL`(tsv 抜き)から `KEXD` を作り `k_lift` はそれを使う。`H_GEN` と `H_WASM` から seed の資材 6 つ〔`vfs-seed-idle1.tsv` `reg-seed-nsis.tsv` `reg-seed-app.tsv` `authenticode-claims.tsv` `os-fonts/` `web/final0129/pack_seed.py`〕を `:(exclude)` で外す | 0330 の差分(gpu1) |
| `ci/test-hybrid.sh` | +19: 検査 3b(新規。seed の資材を変えた時の key の変わり方を、git repo にした複写で pin) | gpu1 |
| `lift/harness/seeds/vfs-seed-idle1.tsv` | +3 行: `Artwork\Skins\Default\BigFruit.png`、`System\Texts\About\AboutTxt.txt`(どちらも installer の中、host path は `lift/out-idle1/inputs/install/…`)、`C:\Windows\Fonts\msgothic.ttc`(os-fonts の font)。**Fruity microfont.ttf は入れていない**。msgothic の行は `arial.ttf` の**前**に置いた(RT-B のように tsv の末尾に行が増えた木にも hunk が当たるため) | 行の SHA は mt3 の抽出(installer の stream から読んで検証済み)と opt1 の font |
| `lift/harness/seeds/os-fonts/COPYRIGHT-fonts-noto-cjk-jp.txt` | 新規(+105 行、Debian copyright 形式、OFL 1.1 の全文入り) | opt1(`opt2-results/0320/font-seed/`) |
| `lift/harness/seeds/os-fonts/README.md` | 表に msgothic の行、注意 4 点(modified file、起動時に全部読まれる、Wine で日本語が出る、`make-msgothic.py` で再現可) | opt1 の `os-fonts-README.md.diff` |
| `lift/harness/web/final0129/pack_seed.py` | +8: seed の表に同じ guest path が 2 回あれば pack を作る前に loader と同じ文言で止まる | mt3 の `0315-pack-seed-duplicate-guest-path.diff` |
- **S にバイナリは入れない**(diff に入らない)。次の表の TTC を push 時に置く。
- **重複して当てない**: mt3 の `0315-pack-seed-duplicate-guest-path.diff` を別に当てる場合、S の `pack_seed.py` の hunk は 『previously applied』になる。どちらか一方にする。
- `os-fonts/OFL.txt` は入れない(opt1 の指示: `COPYRIGHT-fonts-noto-cjk-jp.txt` に OFL 1.1 の全文が入っている)。

### バイナリの表
| repo に置く path | 正本のある所 | 大きさ | SHA-256 |
|---|---|---|---|
| `lift/harness/seeds/os-fonts/NotoSansCJKjp-subset-msgothic.ttc` | `~/work/signature-studio-develop/lift/out-git/opt2-results/0320/font-seed/NotoSansCJKjp-subset-msgothic.ttc`(opt1。作り方は `font-seed/tools/make-msgothic.py`、`head.created/modified` を元から保つので 2 回の build が同じ bytes) | 2,123,288 B | `1ab00fd8ffe15553c16ef441b6a63e539159d560491c70fbc598d26f470eea69` |
- **mt3 の seed2b は別の TTC を使った**: `fonts-pkg/msgothic.ttc`(SHA-256 `daa1948f…80b2f`、同じ 2,123,288 B、head.modified だけが違う〔opt1 の README〕)。S は正本(`1ab00fd8…`)に揃えてある。seed2b の O16 の確認(日本語が出た)は `daa1948f…` の font での結果で、**正本の font で Chrome の実表示を見るのは未実行**(別の合図で)。
- 置く前後の確認: `sha256sum lift/harness/seeds/os-fonts/NotoSansCJKjp-subset-msgothic.ttc` が上の値と一致すること(`extract_fl.py` が tsv の第 3 列と照合し、不一致なら exit 2 で止まる)。

## 2. 検証 — 当たること
| 当てる先 | コマンド | 結果 |
|---|---|---|
| 新しい `git archive HEAD`(`ci` `lift/harness`) | `patch -p1 --dry-run -s < 0330b-seed-only-change.diff` | **rc 0、`.rej` 0、`.orig` 0** |
| 同上 | `git apply --check` | **rc 0** |
| mt3 の `tree-rtb` の複写(`ci/plan.sh` `ci/test-hybrid.sh` `os-fonts/README.md` `vfs-seed-idle1.tsv` `pack_seed.py`) | `patch -p1 --dry-run -s` | **rc 0**(tsv の末尾に Fruity の DLL 行が増えていても当たる。初版は msgothic を末尾に置いたため hunk 2 が FAILED になり、`arial` の前に移して直した) |

## 3. 検証 — seed の数や内容を数えている試験(更新が要るか)
- 探したもの(`git grep`、私の木): `vfs-seed-idle1` `os-fonts` `pack_seed` `seedpack` `extract_fl` を `lift/crates`・`ci`・`app/test`・`app/scripts`・`tools`・`lift/harness/*.sh`・`lift/harness/web` で、数 `218 222 229 231 232 234 241 243 248` を `seed|guest file|tsv|row` と同じ行で。
- **tsv の行数・seed の file 数・seed の大きさを pin した試験は無かった**。出た物: ① `ci/README.md` と `lift/harness/a4-fruity.sh` の**コメント**『218 guest files』(文書で、試験ではない。数は S の後で変わる〔installer から +2、repo の font +1〕が、コメントなので S は触っていない)、② `lift/crates/fe-lift/tests/wall_fr12.rs`(loader は 2048 entries まで受け、2049 で名前つきで止まる。tsv は 244 行で余裕)、③ `app/test/unit/pack.test.ts`(『pack_seed.py が書いた pack を読む』。合成の list を使う)、④ `wall_dlg1.rs`(loader の UTF-8 の扱い)。
- 『222 ファイル』は PM の記憶の数字と思われ、repo 内では見つからなかった。**更新が要る期待値は 0**。
- 実行した試験(S の木): `ci/test-hybrid.sh`(`.github` を足して 46 ok + 新しい 3b の 12 ok = 全部 ok)、`ci/test-needs.sh`(128 通り、all invariants hold)、`ci/test-real-deploy.py`(全 ok)。Rust・vitest・cargo は**実行していない**(S は runtime・crates・app を触らない)。
- **新しい検査 3b**(`ci/test-hybrid.sh`): git repo にした複写で `vfs-seed-idle1.tsv` に 1 行と `os-fonts/` に 1 file を足し、`k_extract` `k_seed` `k_bundle` `prefix` が**変わる**こと、`k_lift` `k_rt` `k_link` `k_felift` `k_b2d` `k_emsdk` `k_vst3` `k_app` が**変わらない**ことを pin。**負の対照**: 旧 `ci/plan.sh` に戻した複写で実行すると `FAIL k_lift / k_rt / k_link depends on the seed data`(= 検査は旧い挙動を検出する)。注意: 既存の検査 3 の複写は `.git` を持たず内容の hash になり、`:(exclude)` が効かない(`find` で代用される)ので、3b は専用の git repo の複写で行った。

## 4. 検証 — key の表(`ci/plan.sh` を実行、私の木)
| 比較 | extract | vst3 | felift | b2d | lift | rt | link | seed | bundle | prefix | app |
|---|---|---|---|---|---|---|---|---|---|---|---|
| HEAD → **S**(S 自身を push) | CHANGED | same | same | same | **CHANGED** | **CHANGED** | **CHANGED** | CHANGED | CHANGED | CHANGED | same |
| **S → S2**(S の後に、tsv に 1 行と os-fonts に 1 file を足しただけ) | CHANGED | same | same | same | **same** | **same** | **same** | CHANGED | CHANGED | CHANGED | same |
- 1 行目: S 自身は冷間 build(lift + wasm objects + link + seed + bundle。README: 標準 runner で 25〜45 分)。felift・b2d・vst3・app は cache の hit。
- 2 行目: **seed だけの変更は extract・seed・bundle・prefix だけ**。
`ci/needs.sh` の出力(cache の hit/miss を与えた):
| 状態 | 与えた hit | 走る stage |
|---|---|---|
| S 自身 | bundle・link・seed・lift・extract が miss、felift・b2d が hit | `need_link=true`、`run_lift=true`、`run_extract=true`、`need_b2d`・`need_emsdk=true`(wasm objects と link を走らせる)= 冷間 |
| S の後の seed だけの変更 | bundle・seed・extract が miss、link・lift・felift・b2d が hit | `need_link=false`、`run_lift=false`、`restore_lift=true`、`restore_link=true`、`run_extract=true`、`need_seed=true`、`need_felift/b2d/emsdk=false`(= extract → seed → bundle だけ) |

## 5. 検証 — `deploy.yml` の build job の `if:` が needs.sh の出力どおりか(読み。**未実行**)
`.github/workflows/deploy.yml` を読んだ。plan job は各 stage の cache lookup(`lk_bundle` `lk_link` `lk_seed` `lk_lift` `lk_extract` `lk_felift` `lk_b2d` …)の `cache-hit` を `H_*` として `ci/needs.sh` に渡し(308〜314 行)、build job の step の `if:` は `needs.plan.outputs.<needs.sh の出力名>` と 1 対 1 に対応する:
| step(build job) | `if:` | S の後の seed だけの変更での値 |
|---|---|---|
| restore emsdk / emsdk / rust toolchain | `need_emsdk` / `run_felift` | false → 走らない |
| restore extract / run extract | `restore_extract` / `run_extract` | false / **true**(installer を artifact bucket から取って展開) |
| restore felift / run felift、restore b2d / run b2d | `restore_*` / `run_*` | false → 走らない |
| restore lift / run lift | `restore_lift` / `run_lift` | **true** / false |
| the generated C(SHA-256) | `need_lift_out` | true(軽い) |
| keys of the object groups、restore objects ×7、wasm objects、save objects ×7 | `need_link` | **false → 走らない** |
| restore link / run link | `restore_link` / `need_link` | **true** / false |
| restore seed / run seed | `restore_seed` / `need_seed` | false / **true** |
| bundle(brotli・manifest・SHA256SUMS)/ save bundle | (build job が走る時) | 走る |
- 結論: step の条件は needs.sh の出力に従っており、『extract → seed → bundle』だけで通る**はず**。**未確認**: ① 実際の Actions の実行、② restore lift が seed の stage に必要な出力(`$ONAT/v-*`)を持っているか、③ extract stage の所要。S の後の最初の seed だけの push で確かめる。
- deploy は bundle の key が変わるので `deploy` job が `prefix`(= `fl-engine-ci-<k_bundle の末尾 8 桁>`)の新しい bundle を上げ、既定の manifest を最後に切り替える(0330 §2)。

## 6. 検証 — seed pack を実際に作って seed2b と比べる
作り方: `lift/harness/web/final0129/pack_seed.py`(S の版)+ S の tsv + reg-seed 2 本 + authenticode-claims を、**root = 私が作った木**(`lift/out-*` は mt3 の `ci-rt1s/host/lift/out-*` への symlink、`lift/harness/seeds/os-fonts/` は S の木の複写)で。比較の対照(同じ root・HEAD の tsv・HEAD の pack_seed.py)も作った。圧縮は `app/scripts/pack-assets.mjs` と同じ streaming の brotli(`LGWIN 24`、size hint)。品質 1 は mt3 の `.br` のサイズを**そのまま再現**した(8,240,342 B と 9,826,130 B)ので方法は合っている。**本番の既定は品質 11**(`BROTLI_QUALITY` の既定)。
| pack | files | raw | SHA-256(先頭) | br 品質 11 | br 品質 1 |
|---|---|---|---|---|---|
| 対照: HEAD の tsv(同じ root) | 225 | 7,248,243 | `f7726113ae34d52e` | 3,929,989 | 5,189,703 |
| **S**(同じ root) | **228** | **9,548,386** | `7cd2eaecbdba9f71` | **5,230,435** | 6,757,486 |
| S − 対照 | +3 | **+2,300,143** | | **+1,300,446** | +1,567,783 |
| (参考)RT-A の実 bundle の seed | 227 | 14,748,394 | `8e8bbd1ea7fe7821` | 5,187,014 | 8,240,342(mt3 の `.br`) |
| (参考)seed2b の実 bundle の seed | 232 | 17,089,558 | `7d762565049176a2` | 6,475,926 | 9,826,130(mt3 の `.br`) |
- S が足す 3 file(対照との差、header で確認): `seed/lift/out-idle1/inputs/install/Artwork/Skins/Default/BigFruit.png` 166,508 B(`0a8dab68…`)、`…/System/Texts/About/AboutTxt.txt` 9,228 B(`2e59358c…`)、`seed/lift/harness/seeds/os-fonts/NotoSansCJKjp-subset-msgothic.ttc` 2,123,288 B(`1ab00fd8…`)。変わった既存 file は `vfs-seed-idle1.tsv` だけ。消えた file 0。
- 対照と RT-A の差(7.5 MB)は、RT-A の tsv に Fruity Balance/Filter の DLL(3,718,656 + 3,780,608 B)の行があるため(mt3 の real-module の変更。HEAD の tsv にはまだ無い)。RT-A + S の差分 = 14,748,394 + 2,300,143 = 17,048,537 に対し seed2b は 17,089,558(差 41,021 B ≈ Fruity microfont の複写 2 つ + tsv の行。S は microfont を入れないのでその分小さい)。
- **本番の起動で増えるのは seed の br で +約 1.30 MB(品質 11)**(0330 §4.4 で品質 1 の 1.59 MB を使っていたのを訂正する値)。
- **native の loader に S の seed を通した**(mt3 の `seedpack-unpack.py` + `vfs_seed_check.c`、`lift/harness/vfs_seed.h` をそのまま): pack を展開(228 files、各 SHA-256 が header と一致)→ loader `rc=0`、『vfs seed: done, 243 files, 47 directories created』、stderr 0 行、BigFruit.png・AboutTxt.txt・`C:\Windows\Fonts\msgothic.ttc`(`1ab00fd8…`)の 3 行が `vfs seed:` に出る。
- mt3 の `test-pack-seed-dup.sh`(重複検査の試験)を S の木に実行: (2)(3)(3b)(4) は通り、**(1)は『line 237』を固定した期待値のため FAIL**(S では BigFruit の行が前に入って 238 になる)。これは results の tool であり repo の試験ではない。期待値を 238 にするか行番号を見ない形にする。**注意(自己申告)**: この script は作業用の場所に `~/work/mt3w-s1/tmp/packdup` を固定で使い(`rm -rf` してから作る)、私の実行でその中身を上書きした(mt3 の同名の場所)。

## 7. 手順(PM が push する時)と注意
1. 新しい `git archive HEAD`(または作業木)に `patch -p1 < 0330b-seed-only-change.diff`。
2. `cp ~/work/signature-studio-develop/lift/out-git/opt2-results/0320/font-seed/NotoSansCJKjp-subset-msgothic.ttc lift/harness/seeds/os-fonts/` し、`sha256sum` が `1ab00fd8ffe15553c16ef441b6a63e539159d560491c70fbc598d26f470eea69` であることを確かめる。
3. `bash ci/test-hybrid.sh`・`bash ci/test-needs.sh` が通ること。
4. commit・push。**この push は冷間 build**(§4)。RT-A/RT-B の runtime の変更と一緒に出すなら同じ冷間 build で足りる(0330 の案 C)。
5. 注意: ① S は mt3 の `0315-pack-seed-duplicate-guest-path.diff` を含む。別に当てるなら片方に。② RT-B の `tree-rtb` の tsv(Fruity の DLL 行 3 本が末尾)にも当たる。③ `os-fonts/README.md` の『The four rows at the end』は、msgothic の行を `arial` の前に置いたため『末尾の 4 行』が少し不正確(文言だけの問題)。④ **正本の font(`1ab00fd8…`)で実 Chrome の表示は未確認**(seed2b は `daa1948f…`)。⑤ 1 回目の push の後、extract の所要・`restore lift` の出力・`compare-bundle.sh`(本番の bundle との比較)を確かめる。

## 8. type B の試験 bundle `fl-engine-test-e13-seed3-7cd2eaec`(PM の指示、10:5x)
**実際に実行した物**(Chrome・build・deploy は 0): 本番の E13 bundle(`lift/out-ci1/kit/bundles/fl-engine-ci-0363b050`)の wasm・glue・DLL pack(と各 `.br`)を **hard link** し、seed pack と manifest だけを差し替えた。置き場 `~/work/gpu8w/bundles/fl-engine-test-e13-seed3-7cd2eaec/`(README-TEST-BUNDLE.txt、SHA256SUMS〔`sha256sum -c` 全部 OK〕入り。TEST、push しない)。手順の要点は `tools/mk-seed3-bundle.sh`。
| 見た物 | 結果 |
|---|---|
| 私の手順(`pack_seed.py` + **HEAD の tsv** + mt3 の抽出済み host)で作った seed pack と本番の `seed.f7726113ae34.flpack` | **`cmp` で完全一致**(7,248,243 B、SHA-256 `f7726113ae34d52e0712fff5e893ae6a61f1eaecc72545e82eb27b4ebd6d8984`)= 手順は本番の seed を再現する |
| type B の seed = 同じ手順 + **S の tsv** | `seed.7cd2eaecbdba.flpack`、228 files、raw 9,548,386 B、SHA-256 `7cd2eaecbdba9f712f7366a76280d2099363aeacf7a2aea7637f3caf238247f6`、`.br`(品質 11・window 24)5,230,435 B |
| wasm・glue・DLL pack(と `.br`)と本番 | `cmp` 同一、`ci/compare-bundle.sh` で role ごとに yes(wasm `58ad4a6aa720375a`、glue `07e7646f280b1b79`、dll `2f494419c1adec95`、各 br も一致) |
| manifest と本番の差 | `bundle.created`、3 つの URL の prefix、`seedPack.{url,size,sha256,encodedSize}` だけ。`engine.env` は本番のまま(FLE_DIAG_* は足していない: 本番の wasm は読まない) |
| native の loader に bundle の seed を通す | rc 0、『vfs seed: done, 243 files, 47 directories created』、stderr 0、3 行(BigFruit・AboutTxt・msgothic〔`1ab00fd8…`〕)が seed される |
- 注意: kit の本番 bundle の `seed.*.flpack.br`(4,446,700 B)は ci1 の局所の再圧縮で、本番 CI の manifest の `encodedSize`(3,929,989 B = 品質 11)とは違う。type B の `.br` は品質 11 で作ったので manifest の `encodedSize` と一致する。
- Chrome の確認(About・日本語名の FLP・起動)は ci1 の計時の窓の後。assets は `lift/out-gpu2/assets-seed3`、対照(本番の seed のまま日本語名の FLP)は `assets-e13ja`。

## 9. type B の実 Chrome の確認(11:40〜11:51、PM の合図の窓)
**実際に実行した物**: 実 Chrome 156 headless(私の 1 つずつ、4 run)、ci1 の E13 page(`kit/page-d0447902`、協調版、vite preview)、**4996 = type B `fl-engine-test-e13-seed3-7cd2eaec`**(本番の E13 の wasm・glue・DLL pack + S の seed〔正本 TTC `1ab00fd8…`〕。manifest は本番の形のまま)、**4997 = 本番の seed のまま**(`fl-engine-ci-0363b050`)。HTTP で取った seed pack・wasm の SHA-256 が期待値と一致することを先に確認。同じ project(`ja-names.flp` を `serina1-serum2.flp` の名で置いた `proj`)。非計時(first idle は参考値)。保存: `0330b/logs/{e13s3-about,e13s3-ja,e13ja-control,e13-control-about}/`、切り抜きの比較 `0330b/logs/ja-compare-seed3-vs-control.png`。
**運用の条件(PM の指定: MemAvailable ≥ 12 GB かつ swap si/so < 100 KB/s)**: `tools/run-e13s3.sh` が確認。実測: about(seed3)= MemAvailable 15.5 GB、si/so 0・0(`FORCE=1` で始めたが条件は満たしていた)。ja(seed3)は 1 回目に swap 572 KB/s の標本で起動を拒否 → 再試行で 20 GB・si 0・so 0(vmstat の最後の標本)。ja(対照)28.9→25 GB・swap 0・0。about(対照)24 GB・si 3,720 KB/s の読み(block in)だけで swap si/so は 0・0。Wine の E4 と RT-B の build が並走。
| run | port | 結果 |
|---|---|---|
| About(type B) | 4996 | **ロゴ・歯車・staff roll が出る**(`about-28s.png`)。layered 窓(lid 141)は 3 s で **247×346**、28 s で 239×338。first idle 4.73 s。unsupported 0、abort 0。`vfs seed:` の行に BigFruit.png(166,508 B)・AboutTxt.txt(9,228 B)・`C:\Windows\Fonts\msgothic.ttc`(2,123,288 B、sha256 `1ab00fd8ffe15553c16ef441b6a63e539159d560491c70fbc598d26f470eea69`) |
| 日本語名(type B) | 4996 | Channel rack 1 行目『サンプ‥リナ』、Playlist 見出し『ぷにぷに電機　日本語ボリューム（Ｓｏｌｏ）』が**日本語の字形**で出る(正本の TTC)。first idle 4.98 s、unsupported 0、`vfs seed: … msgothic.ttc … 1ab00fd8…` の行あり |
| 日本語名(対照・本番の seed) | 4997 | 同じ 2 か所が**豆腐(□)**(`ja-compare-seed3-vs-control.png` の下の段)。first idle 4.72 s、unsupported 0、msgothic の行 0 |
| About(対照・本番の seed) | 4997 | layered 窓(lid 141)は **3×3**(ロゴなし)。first idle 5.07 s |
- 日本語の字形は seed2b の確認(`daa1948f…`)と同じ見え方で、正本の TTC(`1ab00fd8…`)でも出ることが分かった。
- 起動の first idle は 4.72〜5.07 s で seed の有無に有意な差は見えない(各 n=1、load・並走が違う。参考値)。seed が +2.3 MB(raw)でも first idle に出ていない、とは言えるが、差の大きさは測れていない。
- 本番の wasm は診断の env を読まないので、engine log に msgothic.ttc の open/read の行は無い(seed の行だけ)。読み込みの経路は見えない。
- 対照の唯一の違い = seed pack(正本の TTC・BigFruit.png・AboutTxt.txt が入った版 / 入っていない版)。wasm・glue・DLL pack・page・project・ブラウザは同じ。→ O3(ロゴ・staff roll)と O16(日本語)の原因は seed の資材の欠落(C)という仮説が、**本番の wasm そのもの**で確認できた。


### PM の追記(2026-11-07 10:57、docs のみ)
- **PM の検証**: `diffs/0330b-seed-only-change.diff`(6 ファイル: ci/plan.sh、ci/test-hybrid.sh、os-fonts の COPYRIGHT と README、vfs-seed-idle1.tsv の +3 行、pack_seed.py の重複検査)を新しい `git archive HEAD` に実適用して rc 0(FAILED なし。見つかった .rej 3 件は HEAD に追跡済みの docs の既存の物)。正本の TTC(`lift/out-git/opt2-results/0320/font-seed/NotoSansCJKjp-subset-msgothic.ttc`、2,123,288 B)の SHA-256 は 1ab00fd8ffe15553c16ef441b6a63e539159d560491c70fbc598d26f470eea69 で tsv の行と一致。TTC を `lift/harness/seeds/os-fonts/` に置いた木で `ci/test-hybrid.sh` は『all checks hold』。mt3 の別の重複検査の差分(0315-pack-seed-duplicate-guest-path.diff)は 0330b の pack_seed.py と重なるので、どちらか一方だけを適用する。
- **gpu1 の型 B 検証 bundle**(`~/work/gpu8w/bundles/fl-engine-test-e13-seed3-7cd2eaec`、gpu1 の報告。PM は bundle の実体を未再検証): HEAD の tsv で作り直した seed pack は本番の `seed.f7726113ae34.flpack` と cmp で完全一致(7,248,243 B)。wasm・glue・DLL pack は本番 bundle `fl-engine-ci-0363b050` の hard link で、違うのは seed pack と manifest だけ。seed は 228 files、raw 9,548,386 B(+2,300,143 B)、br(品質 11)5,230,435 B。native の loader は rc 0、243 files、47 directories。**実 Chrome での確認(About のロゴ、日本語名の FLP、first idle)は、ci1 の計時の窓の後。未実施**。
- **判断(変更なし)**: 0330b を単独の push として、G9 の push より先に出す。push は PM が行い、deploy の CI が走っていない時に限る。この push 自体は plan.sh の key が変わるため 1 回の冷間 build(約 60 分)。依頼者の機械の値は 0 件。
