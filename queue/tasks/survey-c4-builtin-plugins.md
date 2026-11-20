# survey-c4-builtin-plugins: C-4(100 以上の内蔵エフェクトとインストゥルメント)の現状を、リポジトリと runner の木で数える(測定のみ。製品は変えない)

## 報告の先頭の規則
- 報告の 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。実行していなければ「実エンジン: 実行なし(理由)」と書く。
- 数字は、この実行で得た実測だけを書く。各数字に、出典のコマンドと出力の行を付ける。推定は「推定」と書き、実測と分ける。
- 「達成」「揃った」と書かない。判定は「受理」「差し戻し」「未確認」のいずれかにする。

## 規則の要約(この task で守ること)
- 製品(app/・lift/・ci/・tools/・patches/・.github/)を変えない。変更は queue/findings/survey-c4-builtin-plugins.md だけ。
- Python は使わない。ベンダー名、Cloudflare のアカウント ID・ホスト名・ETag(32 桁の 16 進)、絶対パスを書かない。パスは repo の中の相対パスで書く。
- commit は環境の git の設定のまま行う(git config・--author・GIT_AUTHOR_*・git -c を使わない)。commit に Claude の行を付けない。ブランチを作らない。push はしない(判定の段が push する)。
- .git/config、opencode.jsonc、.opencode を読まない。git の内部命令(update-index・hash-object・update-ref・commit-tree・write-tree・bundle・for-each-ref を含む)を使わない。force は使わない。
- Secrets と Variables は読まない。wrangler と cf の CLI は使わない。R2 の鍵はこの実行には無い。
- sleep、timeout、nohup、disown、末尾の & は使わない。
- 一時ファイルは repo の中の .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。

## 目的と根拠
台帳 L-033 の (4): 「100 以上の内蔵エフェクトやインストゥルメントは 1 つも存在しない」。queue/plan.md の C-4 を参照。
是正(台帳 L-034、依頼者の申告): 内蔵のエフェクトは Plugins\Fruity\Effects に約 90 件、インストゥルメントは Plugins\Fruity\Generators に約 51 件ある。依頼者の手元の版の数で、lift 対象の版とは ±2 ほど違いうる。OpenGL を使う VFX 系は後回しにしてよい。それ以外の内蔵プラグインは、全て lift の対象。目標は約 141(90 + 51)。この数字は申告で、この task の実測ではない。今 lift の対象になっている 3 件は、目標ではなく進み具合である。
この task は、いま lift の対象になっている DLL の数と、seed の表にある内蔵の数を、リポジトリの表で数え、インストーラの全件はどこに在るか(runner に在るか)を確かめる。申告の約 141 を確かめられる範囲を測り、確かめられない分は未確認と書く。

## 読む物(実在を確認済み)
- lift/harness/b2d2/modules.tsv(lift の対象の DLL の表。8〜10 行が Fruity の 3 つ)
- lift/harness/hyb/hyb-args.tsv(hyb の対象の表)
- ci/extract-dlls.tsv(抽出する DLL の表。SHA-256 つき)
- lift/harness/seeds/vfs-seed-idle1.tsv(seed の表。Plugins\Fruity\Effects の 3 行、Plugins\Fruity\Generators の 1 行)
- lift/harness/hooks/fruity-delphi-effect-hook-points.txt(Fruity の効果の hook 点の表)
- lift/harness/hyb/lowered-all.txt(直接 wasm にした関数の一覧)と queue/inputs/lift-function-classes.tsv(モジュール別の関数の数)

## 先頭の注意(入力)
- インストーラ(FL Studio の exe)は、この runner には無い(extract ジョブだけが R2 から取って抽出する。作業ジョブは抽出の木だけを持つ)。したがって、インストーラの中の全 DLL の一覧は、この task では得られない。未確認として書く。
- 抽出の木が無い場合(lift/out-app/host が無い)も、ここで止めずに手順 1〜3 と 5 を行う。読みと表だけで答えられるため。

## やること
1. 表の行数を数える(リポジトリの表だけ。実行はしない)。
   - `grep -v '^#' ci/extract-dlls.tsv | grep -c .` の結果(DLL の行の数)
   - `grep -c . lift/harness/b2d2/modules.tsv` の結果(lift の対象の数)
   - `grep -c . lift/harness/hyb/hyb-args.tsv` の結果(hyb の対象の数)
   - `grep -c 'Plugins\\Fruity' lift/harness/seeds/vfs-seed-idle1.tsv` の結果と、その行を `grep -n 'Plugins\\Fruity' lift/harness/seeds/vfs-seed-idle1.tsv | cut -c1-160` で行番号つきで貼る。
2. 種類ごとの内訳を書く。lift/harness/b2d2/modules.tsv の 1 列目(DLL の名前)を見て、次の 3 つに分けて件数を書く。(i) FL の内蔵のエフェクト(Fruity で始まる名前)は 3 件(Fruity Balance・Fruity Filter・Fruity Delay 2)。(ii) FL の内蔵のインストゥルメント(Plugins\Fruity\Generators の DLL)は 0 件。lift/harness/b2d2/modules.tsv にも ci/extract-dlls.tsv にも、Generators の DLL の行は無い。seed の Plugins\Fruity\Generators は GM CCs.txt の 1 件だけ(その件数も書く)。(iii) それ以外(dsp・Blend2D・QuickFontCache・FreeType・ILTools・oggio・elastique など、エンジンが使うライブラリ)の件数。最後に、申告の目標 約 141 に対する進み具合を「3 / 約 141」の形で書く(約 141 は申告の数で、実測ではないと書く)。
3. 直接 wasm にした関数の数を書く。`wc -l < lift/harness/hyb/lowered-all.txt` の結果と、モジュール別の関数の数(`awk -F'\t' 'NR>1 {c[$2]++} END {for (k in c) print k, c[k]}' queue/inputs/lift-function-classes.tsv` の結果)。
4. 抽出の木にある DLL を数える。`find lift/out-app/host -type f -iname '*.dll' | wc -l` の結果。抽出の木が無ければ「実行なし(抽出の木が無い)」と書く。
5. UI から選べるかを確かめる(手段の有無だけを書く)。内蔵の効果・インストゥルメントの一覧は、エンジンの「プラグインの一覧」の画面で見える想定だが、その一覧を開く手段(メニュー)は、C-6(トップバーの右クリックメニュー)が機能しないため、この runner では開けない。これを「未確認(メニューが出ない: survey-c6-topbar-menus を参照)」と書く。
6. インストーラの全件の一覧について: 抽出の木に無いこと、作業ジョブから R2 の鍵が使えないことを、行の番号つきで書く(`grep -n 'R2_ARTIFACT' .github/workflows/agent.yml` の結果。鍵が使われるのは extract ジョブの段だけ)。
7. 1 件を足す手数を、既存の履歴から数える(読みのみ)。`git log --oneline -- ci/extract-dlls.tsv lift/harness/b2d2/modules.tsv lift/harness/hyb/hyb-args.tsv lift/harness/hooks/` の件数を書く。Fruity Delay 2 の行を足した commit を `git log --format=%h -n 1 -G 'delay 2_x64' -- ci/extract-dlls.tsv` で探し、その commit の変更ファイルを `git show --stat --format='%h %s' <探した commit>` の結果で書く(変更ファイルの数と、どの表が変わったか)。commit の件名にベンダー名が出る場合は、書かない。
8. 1 プラグイン 1 wasm の実現の仕方を、読みで調べる(配信の方針 L-035。読みのみ。結果は `## コードの読み` に、パスと行番号つきで書く)。(a) Fruity の 3 件の大きさ: `find lift/out-app/host -iname '*fruity*' -printf '%s\t%p\n'` の結果(抽出の木の DLL の大きさ。無ければ「未確認」)。3 件のうち最も小さい物を、最初の 1 件の候補として書く。(b) モジュール別の lowered の一覧の件数: `wc -l lift/harness/hyb/lists/*.txt` の結果。Fruity の一覧が無いことを書く。(c) real module の定義: lift/runtime/fle_realmod.h の 1〜12 行。表(fle_realmod_table)が、どこで作られるかを `grep -rn 'fle_realmod_table' lift --include=*.c --include=*.h --include=*.rs --include=*.inc` の結果で書く(ビルドの段か、手で書いた表か)。(d) 起動引数: lift/harness/web/pack.args の --args の行(起動引数に並ぶ DLL)と、lift/crates/fe-build/src/main.rs の pack_step_for(459〜463 行)。(e) wasm を分けるなら、どの段を変えるか(lift/crates/fe-wasmgen と lift/crates/fe-build。lift/ の変更になることを書く)。(f) 先例: app/src/plugins/plugin-client.ts(1 インスタンス 1 Worker の口)と app/src/plugins/ott-host.ts(先頭 1〜3 行)。

## 出力
- queue/findings/survey-c4-builtin-plugins.md に、次の見出しで日本語で書く。
  - `## 実測`: 手順 1・3・4 の数字(出典の行つき)。表で書く: 対象・件数・出典。
  - `## コードの読み`: 手順 2・5・6 の結果。Fruity の 3 つが、どの表と hook 点に入っているか(パスと行番号つき)。
  - `## 未確認`: インストーラの全件の数(runner に一覧が無い)、UI から選べるか(メニューが出ない)、内蔵の数が 100 を超えるかどうか。
  - `## 次の手順の案`: (a) 共通の前提の inventory(C-3 と同じ一覧)から、内蔵の DLL の表を作る。Effects の約 90 と Generators の約 51 を、一覧の件数で確かめる。(b) 1 DLL につき 1 タスクにする方式(型を queue/kinds/ に足し、生成器の入力に一覧を置く。lower と同じ形)。1 件を足す手数は、手順 7 の履歴から見積もる(推定と明記)。(c) 後回しの VFX 系(PE の import に OpenGL の DLL を含む物)の判定を、inventory の import の表で行い、名前の一覧を計画に書く。import は fe-pe の pe-info で読める(lift/crates/fe-pe/src/main.rs の 67〜105 行)。(d) 論点: 約 141 件を 1 つの wasm に入れると、大きさと起動(P1)に効く。全件を 1 つに入れるか、必要になった時に読み込むかを決める材料(今のエンジンの wasm の大きさと起動の実測)。(e) lower の進みとの関係(lowered-all.txt の行数と、モジュール別の関数の数から見た、残りの量。推定と明記)。
- commit する(push はしない)。
- 最終の応答の 1 行目は「実エンジン: …」で始める。

accept: test -s queue/findings/survey-c4-builtin-plugins.md && grep -q '^## 実測' queue/findings/survey-c4-builtin-plugins.md && grep -q '^## 未確認' queue/findings/survey-c4-builtin-plugins.md && grep -q '^## 次の手順の案' queue/findings/survey-c4-builtin-plugins.md
