実エンジン: 実行なし(製品のエンジンは起動しておらず、wasm も作っていない。この task は、リポジトリの表と抽出の木の数え上げ、git の読み、コードの読みだけを行った。このマシンには wasm のファイルが無い)

# survey-c4-builtin-plugins: C-4(内蔵のエフェクトとインストゥルメント)の現状の測定

- 判定: 未確認。約 141 の申告は、この runner では確かめられない。数えられたのは Fruity の効果 3 件だけで、インストゥルメントの DLL は 0 件。
- 数字の扱い: 「実測」は、この実行で打ったコマンドと出力の値。「推定」と書いた物は推定。約 141 は依頼者の申告で、実測ではない。
- 変更: 本ファイルだけ。製品(app/・lift/・ci/・tools/・patches/・.github/)は変えていない。

## 実測

### 手順 1: 表の行数(リポジトリの表だけ。実行はしない)

| 対象 | 件数 | 出典 |
| --- | --- | --- |
| ci/extract-dlls.tsv の DLL の行(# を除く) | 11 | `grep -v '^#' ci/extract-dlls.tsv` を `grep -c .` で数えた。結果 11 |
| lift/harness/b2d2/modules.tsv(lift の対象) | 10 | `grep -c . lift/harness/b2d2/modules.tsv`。結果 10 |
| lift/harness/hyb/hyb-args.tsv(hyb の対象) | 8 | `grep -c . lift/harness/hyb/hyb-args.tsv`。結果 8 |
| lift/harness/seeds/vfs-seed-idle1.tsv の Plugins\Fruity の行 | 4 | `grep -c 'Plugins\\Fruity' lift/harness/seeds/vfs-seed-idle1.tsv`。結果 4 |

seed の Plugins\Fruity の行(`grep -n 'Plugins\\Fruity' lift/harness/seeds/vfs-seed-idle1.tsv | cut -c1-160` の出力。1 列目は、インストール先の接頭辞(製品の版の名前とベンダー名を含む)を省いた。2 列目は repo の中の相対パス):

```
4:    Plugins\Fruity\Generators\MIDI Out\Data\GM CCs.txt   → lift/out-fruity11/inputs/GM_CCs.txt
245:  Plugins\Fruity\Effects\Fruity Balance\Fruity Balance_x64.dll   → lift/out-idle1/inputs/install/Plugins/Fruity/Effects/…
246:  Plugins\Fruity\Effects\Fruity Filter\Fruity Filter_x64.dll       → lift/out-idle1/inputs/install/Plugins/Fruity/Effects/…
247:  Plugins\Fruity\Effects\Fruity Delay 2\Fruity Delay 2_x64.dll   → lift/out-idle1/inputs/install/Plugins/Fruity/Effects/…
```

### 手順 3: 直接 wasm にした関数の数

| 対象 | 件数 | 出典 |
| --- | --- | --- |
| lift/harness/hyb/lowered-all.txt | 4374 | `wc -l < lift/harness/hyb/lowered-all.txt`。結果 4374。`grep -c .` も 4374(末尾に改行あり) |
| queue/inputs/lift-function-classes.tsv(見出しを除く) | 12087 | `wc -l` は 12088(見出しを含む) |

モジュール別(`awk -F'\t' 'NR>1 {c[$2]++} END {for (k in c) print k, c[k]}' queue/inputs/lift-function-classes.tsv` と、`grep -c . lift/harness/hyb/lists/lowered-<モジュール>.txt`、および両者の共通の行の数。共通は `comm -12` で数えた):

| モジュール | lists の行数(直接 wasm) | class 表の行数 | うち class 表にある |
| --- | --- | --- | --- |
| b2d | 82 | 189 | 50 |
| dsp | 172 | 294 | 115 |
| elastique | 165 | 0 | 0 |
| fle | 3158 | 8991 | 2422 |
| ft | 189 | 328 | 146 |
| ilt | 184 | 1341 | 178 |
| oggio | 46 | 0 | 0 |
| qfc | 378 | 944 | 219 |
| 計 | 4374 | 12087 | 3130 |

- lists の 8 件の合計は 4374 で、`lowered-all.txt` と一致する。
- `ls lift/harness/hyb/lists` は 8 ファイル(b2d・dsp・elastique・fle・ft・ilt・oggio・qfc)。Fruity の一覧は無い(`ls lift/harness/hyb/lists | grep -ci fruity` → 0)。
- class 表には elastique と oggio の行が無い(`grep -c 'elastique\|oggio' queue/inputs/lift-function-classes.tsv` → 0)。

### 手順 4: 抽出の木にある DLL

抽出の木は在った(lift/out-app/host)。`find lift/out-app/host -type f -iname '*.dll' | wc -l` の結果は **14**(`find lift/out-app/host -type f | wc -l` は 238)。

`find lift/out-app/host -type f -iname '*.dll' -printf '%s\t%p\n'` の結果(パスは lift/out-app/host/ の後の部分)と、`grep -a -c -i 'opengl32'` の結果(DLL の中に opengl32 の文字列がある行の数。PE の import 表の解析ではない):

| バイト | opengl32 の行数 | 抽出の木の中の相対パス |
| --- | --- | --- |
| 22227016 | 1 | binaries/win64/FLEngine_x64.dll |
| 2065920 | 0 | lift/out-blend2d/Blend2D.dll |
| 39942656 | 0 | lift/out-ext/dsp_ippv2_x64.dll |
| 3718656 | 0 | lift/out-plugins/fruity_balance_x64.dll |
| 3731968 | 0 | lift/out-plugins/fruity_delay2_x64.dll |
| 3780608 | 0 | lift/out-plugins/fruity_filter_x64.dll |
| 3662336 | 0 | lift/out-inventory/extracted/ILTools_x64.dll |
| 2067968 | 0 | lift/out-qfc/quickfontcache_x64.dll |
| 4997632 | 0 | lift/out-sampledec/elastique_x64.dll |
| 1872384 | 0 | lift/out-sampledec/oggio_x64.dll |
| 607032 | 0 | lift/out-wallQ/freetype/freetype_x64.dll |
| 3718656 | 0 | lift/out-idle1/inputs/install/Plugins/Fruity/Effects/Fruity_Balance_x64.dll |
| 3731968 | 0 | lift/out-idle1/inputs/install/Plugins/Fruity/Effects/Fruity_Delay_2_x64.dll |
| 3780608 | 0 | lift/out-idle1/inputs/install/Plugins/Fruity/Effects/Fruity_Filter_x64.dll |

- 14 = FLEngine 1 + 他のモジュール 7(dsp・Blend2D・qfc・freetype・ILTools・oggio・elastique)+ Fruity 3 件 × 2 か所(lift/out-plugins と lift/out-idle1)。
- `git ls-files lift/out-app` の結果は 0 件。`.gitignore` の 11 行(`lift/out*/`)で除外されている。抽出の木は、このマシンだけの作業用のデータで、git の管理の外。

## コードの読み

### 手順 2: 種類ごとの内訳

| 種類 | 件数 | 出典 |
| --- | --- | --- |
| (i) 内蔵のエフェクト(名前が fruity で始まる DLL) | 3 | `grep -ci fruity lift/harness/b2d2/modules.tsv` の結果 3。行 8〜10 |
| (ii) 内蔵のインストゥルメントの DLL(Plugins\Fruity\Generators) | 0 | `grep -ci generator lift/harness/b2d2/modules.tsv` の結果 0。`grep -v '^#' ci/extract-dlls.tsv` の `grep -ci generator` の結果 0 |
| (ii') seed の Plugins\Fruity\Generators の行(データ。DLL ではない) | 1 | seed の行 4(GM CCs.txt) |
| (iii) それ以外(エンジンのライブラリ) | 7 | modules.tsv の行 1〜7(dsp_ippv2・Blend2D・quickfontcache・freetype・ILTools・oggio・elastique) |

- ci/extract-dlls.tsv の 11 行は、FLEngine 1 + (iii) 7 + (i) 3。(ii) は 0。
- 進み具合は「3 / 約 141」(分母の約 141 は申告の数。実測ではない)。

Fruity の 3 件が、どの表と hook 点に在るか:

| 件 | ci/extract-dlls.tsv | lift/harness/b2d2/modules.tsv | seed(lift/harness/seeds/vfs-seed-idle1.tsv) | hook 点(lift/harness/hooks/fruity-delphi-effect-hook-points.txt) | lift/harness/hyb/hyb-args.tsv | lift/harness/web/pack.args |
| --- | --- | --- | --- | --- | --- | --- |
| Fruity Balance | 15 行(抽出先 lift/out-plugins/fruity_balance_x64.dll) | 8 行(base 0x6800000) | 245 行 | modules.tsv の 8 行が、このファイルを使う(--hook-points)。ファイルの 2 行目に Balance の大きさと hook 構築子 | 無し | 14 行の --args(fruity_balance_x64.6800000.dll) |
| Fruity Filter | 16 行(lift/out-plugins/fruity_filter_x64.dll) | 9 行(0x7000000) | 246 行 | 同上。ファイルの 3 行目 | 無し | 14 行(fruity_filter_x64.7000000.dll) |
| Fruity Delay 2 | 18 行(lift/out-plugins/fruity_delay2_x64.dll。直前のコメントは 17 行) | 10 行(0x7800000) | 247 行 | 同上。ファイルの 4 行目に「Fruity Delay 2 も同じ 3 つの RVA」と書いてある(この task では確かめていない) | 無し | 14 行(fruity_delay2_x64.7800000.dll) |

- hook 点の 3 つの RVA は、ファイルの 12〜14 行(rva:0x17bc0・rva:0x31d70・rva:0x31d90)。3 件で共通。
- ci/extract-dlls.tsv の 13〜14 行は、Fruity 2 件の追加の記録(2026-11-16、R6-12)。17 行は Delay 2 の追加の記録(2026-11-17、R5-6a)。
- hyb-args.tsv は 8 行(行 1〜8)。Fruity を含まない。

### 手順 5: UI から選べるか

- 未確認(メニューが出ない: survey-c6-topbar-menus を参照)。内蔵の一覧を開く手段は、トップバーのコンテキストメニュー(C-6)で、C-6 は機能していない(queue/plan.md 25 行と 95 行)。
- queue/findings/ は、この task の作成の前は無かった。survey-c6-topbar-menus の結果も、このリポジトリには無い。

### 手順 6: インストーラの全件と R2 の鍵

- インストーラの全件の一覧は、このリポジトリにも、この runner の抽出の木にも無い。抽出の木の 14 件は、ci/extract-dlls.tsv の 11 行(FLEngine 1 + 10)と、seed の行から作られる。
- R2 の鍵の所在(`grep -n 'R2_ARTIFACT' .github/workflows/agent.yml` の結果は 69 行と 70 行。`.github/workflows/agent.yml` で R2 の鍵の行を全部並べた結果は 60・61・69・70・150・151 行):
  - 69〜70 行: R2_ARTIFACT の鍵。job extract(35〜77 行)だけで使う。
  - 60〜61 行: R2_CACHE の鍵。job extract の、抽出物のキャッシュ(50 行)。
  - 150〜151 行: R2_CACHE の鍵。job run(作業ジョブ。78〜236 行)の 141〜151 行「抽出物を R2 のキャッシュから取る」で使う。キーは抽出の木の版(hashFiles。`fail-on-cache-miss: true`)。
- 訂正: 依頼の文の「作業ジョブから R2 の鍵が使えない」は、そのままでは当てはまらない。作業ジョブは R2 の鍵(キャッシュ用)を持ち、抽出の木を 141〜151 行で取る。ただし、取るのは抽出の木の版で、インストーラの全件ではない。R2 の成果物の鍵(ARTIFACT)は extract だけ。

### 手順 7: 1 件を足す手数(履歴)

- `git log --oneline -- ci/extract-dlls.tsv lift/harness/b2d2/modules.tsv lift/harness/hyb/hyb-args.tsv lift/harness/hooks/` の件数: **1**。4 つの表と hook 点の履歴は、1 つの commit(f4cffe7)に潰れている。全体の commit は 1052 件。
- `git log --format='%h %ad %s' --date=short -G 'delay 2_x64' -- ci/extract-dlls.tsv` の結果: f4cffe7 だけ(2026-11-20)。Delay 2 の行は、この commit で入った。
- `git show --stat --format='%h %s' f4cffe7` の結果: 1383 files changed, 418143 insertions(+)。このうち、ci/extract-dlls.tsv(18 行)・lift/harness/b2d2/modules.tsv(10 行)・lift/harness/hyb/hyb-args.tsv(8 行)・lift/harness/seeds/vfs-seed-idle1.tsv(247 行)・lift/harness/hooks/ の 7 件(fruity-delphi-effect-hook-points.txt を含む)が変わっている。
- 結論: 履歴からは、1 件を足す手数は測れない(履歴が 1 commit で、Fruity の 1 件ごとの commit が無い)。
- 推定: 表の構造から見ると、Fruity の 1 件を足すのに、表の行は 4 つ(ci/extract-dlls.tsv の 1 行、lift/harness/b2d2/modules.tsv の 1 行、seed の行 1 つ以上、pack.args の --args の 1 語)。新しい hook 点が要る場合は、hook 点のファイルが 1 つ増える(Fruity は既存のファイルを使う)。

### 手順 8: 1 プラグイン 1 wasm の読み(配信の方針 L-035。読みのみ)

(a) Fruity 3 件の大きさ: 上の表(手順 4)の通り。最も小さいのは **Fruity Balance**(3718656 バイト。lift/out-plugins/fruity_balance_x64.dll)。Delay 2 は 3731968、Filter は 3780608 バイト。hook 点のファイル 2〜3 行(Balance 3,718,656、Filter 3,780,608 バイト)と一致する。最初の 1 件の候補は Balance で、queue/plan.md の 75 行と 144 行の案と一致する。

(b) モジュール別の lowered の一覧: `wc -l lift/harness/hyb/lists/*.txt` の合計は 4374(手順 3 の表)。Fruity の一覧は無い(lists は 8 件、Fruity 0 件)。最初の 1 件は、直接 wasm 化の一覧を作るところから始まる(queue/plan.md 75 行)。

(c) real module の定義と表の作られ方:
- lift/runtime/fle_realmod.h の 5 行: 実モジュールは、リンクが fle_realmod_table で与える名前と base。6 行: 表が無いときは、lift/runtime/fle_realmod.c の弱い定義(20 行: `fle_realmod_table[] __attribute__((weak)) = { { NULL, 0 } }`)で、実モジュールは無い。
- 表は、ビルドの段で作られる。lift/crates/fe-lift/src/gen_all.rs の 279〜288 行(fn realmod。realmod.c を書く。書く行は 109 行)。入力は lift/harness/b2d2/modules.tsv(lift/harness/b2d2/gen.args の --modules の行)。lift/crates/fe-build/src/main.rs の 242〜243 行(gen 段の gen-all)が呼ぶ。
- 手で書いた表は、試験と harness にだけ在る: lift/harness/two/realmod_table.c の 14 行、lift/crates/fe-lift/tests/wall_q.rs の 99 行など。

(d) 起動引数: lift/harness/web/pack.args の 13 行(--args)と 14 行(値)。値は FLEngine と 10 のモジュール(dsp・Blend2D・quickfontcache・freetype・ILTools・oggio・elastique・Fruity 3 件)。wasm は 1 つ(35 行 --engine-wasm の値は lift/out-app/web/engine.wasm)。lift/crates/fe-build/src/main.rs の pack_step_for は 459〜463 行: argv に `node app/scripts/pack-assets.mjs --version <sha の先頭 12 桁>` と、pack.args の行を並べる。

(e) wasm を分けるなら、変える段: lift/crates/fe-build/src/main.rs の stage。gen(246 行)・compile(290 行。main_two.c に -DFLE_FRUITY=1 が 260 行で既に在る。新しい #ifdef は足さない)・hybrid-n0(295 行。lift/crates/fe-wasmgen)・link(306 行。emcc を 1 回。h_objs・mod・fle・rt の 4 群をまとめる)・wasm-opt(310 行)・hybrid(314 行。lift/crates/fe-wasmgen)。lift/harness/web/fruity/link-web.sh は、1 つの flengine6.wasm を作る別の link(20〜28 行)。どの段を変えても lift/ の変更になる。lift/ に触れる push は main の deploy.yml を動かす(報告に書く)。

(f) 先例: app/src/plugins/plugin-client.ts の 1〜2 行(1 つのプラグインの Worker の口)。39 行 class PluginInstance。56 行 static start(makeWorker: () => WorkerHandle, cfg)。1 インスタンス 1 Worker の形。app/src/plugins/ott-host.ts の 1〜2 行: 「lift/harness/vst の、lift 済みのプラグインの emscripten 版の VST3 harness wasm」の host。プラグインごとに 1 つの wasm の先例(外部プラグインの側)。

## 未確認

- インストーラの全件の数と一覧: runner に無い。抽出の木は 14 件、ci/extract-dlls.tsv は 11 行、seed は 247 行(ゲストが要求したファイル。全件ではない)。作業ジョブの R2 のキャッシュも、同じ抽出の木の版。
- 内蔵の数が 100 を超えるか: 未確認。この実行で数えられたのは Effects 3 件(Fruity Balance・Filter・Delay 2)と、Generators の DLL 0 件。申告の約 141(約 90 + 約 51)は、実測ではない。
- UI から選べるか: 未確認(メニューが出ない: survey-c6-topbar-menus を参照。C-6 は機能していない)。
- 大きさと起動(P1): 未確認。この runner には wasm のファイルが無い(`find` で、.wasm の該当は 0 件)。queue/plan.md 72 行の記録(エンジン本体の wasm は 165848536 バイト、圧縮 br で 17510392 バイト。runner の判定の出力の記録)は、この task の実測ではない。
- Delay 2 の hook 点: 「Fruity Delay 2 も同じ 3 つの RVA」(hook 点のファイル 4 行)は、この task では確かめていない。
- OpenGL を使う VFX 系の判定: 文字列の有無の結果だけ(FLEngine で 1 行、Fruity 3 件で 0 行)。PE の import 表の解析ではない。動的に読む DLL は見えない(推定)。
- class 表に elastique と oggio の行が無い理由: 未確認。
- lowered の数: queue/plan.md 67 行は 4327 件(2026-11-21 の push の時点)。実測は 4374 件。差の 47 件の理由は未確認。

## 次の手順の案

(a) 共通の前提の inventory(C-3 と同じ一覧)から、内蔵の DLL の表を作る。Effects の約 90 と Generators の約 51 を、一覧の件数で確かめる。extract の段は、インストーラのファイルの表(ゲストのパス、抽出先、大きさ、SHA-256、import した DLL の名前)を出す必要がある(queue/plan.md 123 行の定義)。この一覧は、作業ジョブのキャッシュには無いので、extract の段で作る。ci/ の変更になり、push は main の deploy.yml を動かす(報告に書く)。

(b) 1 DLL につき 1 タスクにする。型を queue/kinds/ に足し(lower.md と同じ形。{{module}} と {{va}} の代わりに DLL の名前を使う)、生成器の入力に一覧を置く(queue/inputs/ に、lift-function-classes.tsv と同じ形の tsv)。1 件を足す手数(推定): 表の 4 行(ci/extract-dlls.tsv、lift/harness/b2d2/modules.tsv、seed、pack.args)。履歴からは測れない(手順 7)。

(c) 後回しの VFX 系(PE の import に OpenGL の DLL を含む物)の判定は、inventory の import の表で行い、名前の一覧を計画に書く。import は、fe-pe の pe-info で読める(lift/crates/fe-pe/src/main.rs の 67〜105 行。import_dlls は 99 行)。このマシンでは fe-pe を作らない(製品の作業は Actions で行う)。今の文字列の判定は目安にしかならない。

(d) 論点: 約 141 件を 1 つの wasm に入れると、大きさと起動(P1)に効く。全件を 1 つに入れるか、必要になった時に読み込むかを決める材料は、今のエンジンの wasm の大きさと起動の実測。この実測は、Actions でビルドして取る(この runner には wasm が無い)。方針は P3・P2・P1・サイズの順。L-035 の lazy-load では、プラグインを起動後に取るので、起動前の wasm には入らない。

(e) lower の進みとの関係(推定を含む):
- 直接 wasm の関数は 4374 件。class 表の 12087 行のうち、3130 件が直接 wasm 済み(25.9%)。
- class 表の残りは 8957 件(推定。class 表が lower の候補の全件だと仮定した場合。モジュール別: fle 6569・ilt 1163・qfc 725・dsp 179・ft 182・b2d 139)。
- class 表に無い直接 wasm の関数は 1244 件(elastique 165・oggio 46 を含む。class 表に無い理由は未確認)。
- 内蔵を増やすと、lift と lower の段の時間が増える(推定。queue/plan.md 70 行)。
