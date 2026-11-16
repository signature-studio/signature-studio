# 0369: seed の資材(BigFruit.png・AboutTxt.txt・msgothic.ttc)の有無で、About と日本語の表示を実エンジンで観測する(R5-3 の第 1 段)

分類: (b)(契約の実装の補い。資材の欠落であり、API の契約ではない。旧記録 O3・O16 の分類 C「資材の欠落」と同じ)。理由の 1 行: ゲストが `LoadImage`・`CreateFile` 等で要求するファイルが仮想ボリュームに無い、という外部の契約(ファイルの存在)の不足を、seed の行で埋める。FL の挙動を補う・代行する処理は足さない(ページ・ランタイムのコードは変えない)。

日付: 2026-11-15   依頼: R5-3 の第 1 段(main が付けた名。ROADMAP の行は変えない)   前コミット: `8379215`

## 決定
決定: 観測は下の定義どおり行う。判定語は書かない。資材は現行の seed の行をそのまま使う(行は足さない・外さない)。根拠: 行は現行 source(`lift/harness/seeds/vfs-seed-idle1.tsv`)に既にあり、本依頼の目的は、その seed で作った bundle の About と日本語の表示を実エンジンで見て記録すること。

## 問い
現行 source の seed から作った work-r25(seed に 3 行がある)と、seed に 3 行の無い E13(対照)で、実エンジンの About 画面(ロゴ・staff roll)と日本語の表示、engine log の vfs 行を並べる。

## 資材の出典(確かめた範囲)
- `BigFruit.png`: TSV(`lift/harness/seeds/vfs-seed-idle1.tsv`)の 24 行目の 2 列目に、旧ツリーのパスで書かれている。3 列目(sha256)`0a8dab68…e108`。元は FL Studio 21 のインストール先のファイル。そのパスの入力はリポジトリに無い(`lift/out*/` は gitignore)。この clone では存在を確かめられていない(未確認)。
- `AboutTxt.txt`: TSV の 240 行目。2 列目 `lift/out-idle1/inputs/install/System/Texts/About/AboutTxt.txt`、sha256 `2e59358c…813c`。同上(未確認)。`~/ダウンロード/` には FL Studio の installer があるが、この 2 つがそこから取られたかは未確認。
- `msgothic.ttc`: TSV の 241 行目。元は `lift/harness/seeds/os-fonts/NotoSansCJKjp-subset-msgothic.ttc`、sha256 `1ab00fd8…ea69`。**MS Gothic ではない。** Noto Sans CJK JP Regular 2.004 の subset(OFL 1.1。`lift/harness/seeds/os-fonts/COPYRIGHT-fonts-noto-cjk-jp.txt` の License 欄は SIL-OFL-1.1、Copyright 2014-2021 Adobe。README.md の表の 12 行目)。Reserved Font Name は宣言されていない。出典の記述はあり(URL は本書には書かない)。

## 再配布の扱い
- `BigFruit.png`・`AboutTxt.txt` は FL Studio の資材。seed pack には入るが、bundle は公開しない(`artifacts/` は gitignore。AGENTS §4 により bundle・wasm・DLL は commit しない)。本依頼の commit には bundle を含めない。
- `msgothic.ttc` は OFL 1.1 の下での配布。COPYRIGHT ファイルを同梱して扱う。

## 撤回条件
- work-r25 の About でロゴが出ない・staff roll が出ない・日本語が豆腐のまま、のいずれかが観測された場合、または work-r25 の run で `BigFruit`・`AboutTxt`・`msgothic` のいずれかの vfs-miss が 1 件でも出た場合、seed の 3 行の足し方(行・元ファイル)の再点検を次の依頼にする。
- seed の行を外す場合は、`lift/harness/seeds/vfs-seed-idle1.tsv` の 3 行を外す差分を別の依頼で作る(本書では作らない)。

## 観測の定義(観測の前に固定。事実だけを書く)
1. 「About でロゴが出る」= About の shot(`about-28s`)に、フルーツのロゴ(旧記録 O3: 246×345 の layered control に画像が描かれる)が見える。目視。見えない(3×3 の点・空白)なら「出ない」。
2. 「staff roll が出る」= About の shot(`about-28s`)に、スクロールする本文が見える。目視。
3. 「日本語が字形で出る」= 所定プロジェクト(`~/ダウンロード/serina1-serum2.zip`)を開いた画面(`ja-main-20s`)で、日本語の名前が文字(字形)で描かれる。四角(□)なら「豆腐」。
4. vfs-miss = 各 run の engine log(`log-all.log`)のうち、`vfs` と `miss` を含む行の件数を、`BigFruit`・`AboutTxt`・`msgothic` のそれぞれについて数える。`vfs seed:` 行の件数も併記する。
- 判定語は書かない。E13 は seed に 3 行が無い bundle(同じ page・同じ台本の対照)。

## 予測(観測の前に書く)
- E13: ロゴが出ない・staff roll が出ない・日本語が豆腐・vfs-miss に 3 件が出る(旧記録 O3・O16 のとおり。参考)。
- work-r25: ロゴが出る・staff roll が出る・日本語が字形で出る・3 件の vfs-miss は無い。
- 外れ得る。開示: main は旧記録 O3・O16 の症状を見ていた。

## 道具(観測の前に決める)
- `refproj-names.json`(旧ツリーの `out-gpu2` 配下。この clone には無い)は現在のツリーに無い。そのため `openProject`(旧台本 `scn-g6-d3.json`・`scn-g6-ja-d3.json` の操作)は使わず、P3 の台本と同じ `openZipProject`(`serina1-serum2.zip` を `/assets/proj/` から読む)を使う。
- `serve-once-p3-bundle.mjs` は台本を `scn-p3-mixmove-300.json` に固定しているため、同じ中身で台本だけを差し替えたラッパー `tools/verify-kit/serve-once-r53-bundle.mjs`(新規)を作る。台本は `tools/verify-kit/tools/scn-r53-about-ja.json`(新規)。`tools/verify-kit/serve-once-p3-bundle.mjs` と `tools/boot-check/screen-observe.mjs` は変えない。
- 台本は再生しない(P3 の測定ではない)。

## 未確認(観測の前の時点)
- `msgothic.ttc` の seed が work-r25 の実エンジンで実際に読まれるか。
- 今の page で About を開く操作(F10 の後の About タブの座標 1222,95 は旧台本からの写し)が動くか。
- 所定 zip の FLP に、豆腐と字形の差が出る日本語の名前があるか(旧記録の `ja-names.flp` は別の FLP)。
- `BigFruit.png`・`AboutTxt.txt` の元の所在(上記)。
- 実機との比較は 0 件(H6)。耳での確認は本書の対象外。

## 出典
- `lift/harness/seeds/vfs-seed-idle1.tsv`(24・240・241 行目)。
- `lift/harness/seeds/os-fonts/README.md`・`COPYRIGHT-fonts-noto-cjk-jp.txt`。
- `docs/archive/feedback/2026-11-07-owner-observations.md`(O3・O16。参考)。
- `evidence/R2-7c.md`(4a・4b。work-r25 の seed pack 228 件と 3 行)。
- `evidence/R5-2.md`(E13 の初期画面の観測。参考)。
- `docs/decisions/0367-p3-e13-vs-r25.md`(書式の雛形)。

## 結果の追記(2026-11-15、`evidence/R5-3.md`)

- 定義の適用(事実だけ):
  - E13 の 3 回(位置 1 の `r53-r53e13-chrome-2`、位置 3 の `chrome-3`、位置 5 の `chrome-4`): 目視の判定は未確定。貼られた画像と PNG のパスの対応が取れていない。vfs-miss: `^vfs` の 242 行はすべて `vfs seed:`。`miss` を含む行 0。`BigFruit`・`AboutTxt`・`msgothic` を含む行 0。
  - work-r25 の 3 回(位置 2 の `chrome-1`、位置 4 の `chrome-2`、位置 6 の `chrome-3`): 3 回とも About を閉じた後に engine が abort(code 134、`fle: indirect call to 0 ...`)で止まり、日本語の表示(`openZipProject`)に届いていない。vfs は log 未取得。`lid 141` は 245〜249 × 343〜347(3×3 ではない)。ロゴ・staff roll の目視は未確定。
- 「n=3。目視。実機との比較は 0 件」: E13 は 3 回、work-r25 は 3 回。ただし目視の判定は未確定、work-r25 の日本語と vfs は未観測。
- 撤回条件に当たるか: 判定できない。work-r25 の日本語とロゴ・staff roll の目視、vfs-miss の件数が揃っていない。
- 台本の直し: 位置 1 の初回 run の About の座標を (1222,95) から (1221,137) に直した(直した後の台本で E13 は止まらなかった)。予め書いた「道具」の節の座標は、この直しの前のもの。
- 未確認: work-r25 の abort の原因。`msgothic.ttc` が work-r25 で読まれるか。`BigFruit.png`・`AboutTxt.txt` の元の所在。

注(2026-11-15): 14 行と 39 行の旧ツリーのパスの書き方を、`scripts/check-consistency.sh` の指摘(この clone に無いパス)に合わせて直した。意味は変えていない。元のパスは commit `90dafcf` の版にある。

注(2026-11-16、R5-3 の 5 回目の検査): 上の『結果の追記』(2026-11-15)の『ロゴ・staff roll の目視は未確定』は、後の目視で置き換わる(`evidence/R5-3.md` の補い。reviewer の目視 3 枚も同じ結果)。E13(`evidence/R5-3/r53e13-chrome-2/about-28s.png`)は、ロゴ・staff roll が見えない。work-r25(`evidence/R5-3/r53wr25-chrome-1/about-28s.png`)と work-r53k(`evidence/R5-3l/r53knew-chrome-1/about-28s.png`)は、フルーツのロゴと開発者名の列が見える(各 1 枚。n=1)。撤回条件の「判定できない」は、定義 3(日本語)と work-r25 の vfs-miss が未観測のため、そのまま残る。
