# 0371: seed pack だけを入れ替えた bundle で、About を閉じた後の停止を実エンジンで観測する(R5-3d)

分類: 測定(lift の処理を足さない。(a)(b)(c) の対象外)。撤回条件: この観測の結果を、停止の原因の切り分けの次の作業の選択にだけ使う。別の窓・別の道具・別の bundle の結果(`evidence/R5-3c.md` の値など)と混ぜて判断しない。

日付: 2026-11-15   依頼: R5-3d(main が付けた名。ROADMAP の行は変えない)   前コミット: `2fa5dd6`(0370 の結果の追記まで)

## 決定
決定: seed pack だけを入れ替えた 4 つの変種(V1〜V4)を作り、本書の定義どおりに台本 A で観測する。判断の規則は置かない(観測の記録)。根拠: 結果を見てから基準を決めると、後から基準を動かす余地が生まれるため。次の作業の選択は、結果の後に別の決定文書で、規則を先に書いてから行う。

## 問い
R5-3c(`evidence/R5-3c.md`、0370)で、台本 A の停止は work-r25 など 7 bundle で 3 回中 3 回、E13 で 3 回中 0 回だった。両者の違いの 1 つは seed pack で、E13 は 225 件(sha の頭 `f7726113`)、work-r25 は 228 件(sha の頭 `7cd2eaec`)。wasm・glue・dll pack を変えずに seed pack だけを入れ替えた bundle で同じ台本 A を走らせ、停止が seed pack に従うかを並べる。原因は断定しない。

## 事実(観測の前に確かめた読み。出典付き)
- seed pack の header(`FLSEEDP1`、`version` 1)を両方読んだ。E13: `count` 225・`data_bytes` 7205523。work-r25: `count` 228・`data_bytes` 9505110。
- 2 つの seed pack の差(file 単位の比較): `seed/vfs-seed-idle1.tsv` の中身が違う(E13 49445 B、work-r25 50008 B)。work-r25 だけに `seed/lift/out-idle1/inputs/install/Artwork/Skins/Default/BigFruit.png`(166508 B)・`seed/lift/out-idle1/inputs/install/System/Texts/About/AboutTxt.txt`(9228 B)・`seed/lift/harness/seeds/os-fonts/NotoSansCJKjp-subset-msgothic.ttc`(2123288 B)の 3 件がある。
- manifest の差は 11 キー: `bundle.created`・`bundle.description`・`bundle.version`・`engine.wasm.{url,size,sha256,encodedSize}`・`engine.options.glue.{url,sha256,encodedSize}`・`dllPack.url`・`seedPack.{url,size,sha256,encodedSize}`(`diff` の出力は `evidence/R5-3d.md` の手順 3)。`engine.env`・`engine.args`・`dllPack.sha256` は同じ。
- `.br` の扱い: `app/tools/assets-middleware.ts` は `Accept-Encoding: br` のときだけ `NAME.br` を `Content-Encoding: br` で配る。`.br` が無ければ raw で配る。両 bundle の seed の `.br` は、中身を展開すると manifest の `sha256` と一致する(`node` の展開。`evidence/R5-3d.md` の手順 2)。
- `encodedSize`: `app/src/bundle/loader.ts` と `assets-middleware.ts` は読まない。`app/src/bundle/manifest.ts` は型と検査の定義だけ。E13 の manifest の `seedPack.encodedSize` 3929989 は、実際の `seed.f7726113ae34.flpack.br` の大きさ 4446700 と合わない(work-r25 は合う)。この変種では `encodedSize` を書いた `.br` の実際の大きさにした。

## 変種(4 つ。wasm・glue・dll pack は symlink、seed pack と manifest は新しく置く)
`tools/verify-kit/tools/mk-r53d-variants.sh`(新規。引数なし。`rm` を含まない。既存の `work-r53d` があれば止まる)で作る。`~/.cache/signature-studio/work-r53d/<v>/<ディレクトリ名>/`。

| 変種 | host(wasm・glue・dll pack) | donor(seed pack) | ディレクトリ名 | 意味 |
|---|---|---|---|---|
| V1 | E13 | work-r25(228 件、sha 頭 `7cd2eaec`) | `fl-engine-ci-0363b050` | seed だけを work-r25 に替えた E13 |
| V2 | work-r25 | E13(225 件、sha 頭 `f7726113`) | `fl-Oz` | seed だけを E13 に替えた work-r25 |
| V3 | E13 | E13 | `fl-engine-ci-0363b050` | V1 と同じ手順の対照 1(seed は元と同じ) |
| V4 | work-r25 | work-r25 | `fl-Oz` | V2 と同じ手順の対照 2(seed は元と同じ) |

- manifest の書き換えは `seedPack` の 4 キー(`url`・`size`・`sha256`・`encodedSize`)だけ。`url` は host のディレクトリ名に合わせる。他のキー(`bundle`・`engine`・`dllPack`)は host の値のまま。
- V3 の manifest は元の E13 の manifest と `seedPack.encodedSize` だけが違う(3929989 → 実際の `.br` の大きさ 4446700)。V4 の manifest は元の work-r25 の manifest と同一(`seedPack.encodedSize` は元から実際の大きさと一致している)。V3 の違いは手順の対照に含める。
- 各変種の seed pack の `.br` は donor の `.br` をそのままコピーした物(展開して sha を照合済み)。

## 定義(観測の前に固定)
0370 の定義 1〜4 と同じ。
1. 「停止した」= その run の `report.json`(配列の要素 0)が `stopped: true` かつ `state: exited` かつ `beaconExit.code` が 134。
2. 「同じ停止」= `beaconExit.message` が `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]` と一致。
3. 「止まらなかった」= `stopped: false` かつ `state: engine-idle` かつ `opErrors` が空(台本が最後まで届く)。
4. それ以外(別の message・別の code・台本の失敗)は「別の結果」とし、message と code をそのまま書く。bundle が配れない・manifest の検証で失敗するなどで台本が走らない場合は「配れなかった」とし、原因を書いて、その変種の観測を止める(手順 6 の直しを除く)。
- この決定は停止の有無だけを対象にする。ロゴ・staff roll・日本語の目視は本書の対象外。

## 台本と道具
- 台本 A: `tools/verify-kit/tools/scn-r53-about-ja.json`(閉じる → F10 → About タブ → Escape → 所定プロジェクトを開く)。
- 道具: `node tools/verify-kit/serve-once-r53-bundle.mjs <変種のディレクトリの絶対パス> <label>`(0370 と同じ。出力は `tools/verify-kit/out/r53-<label>-chrome-<N>`)。

## 組(各 n=3。直列。下の順に、1 組ずつ 3 回続けて)

| 順 | 変種 | label | wasm(sha 頭 16 桁) | seed pack(sha 頭・件数) | 予測 |
|---|---|---|---|---|---|
| 1 | V3 | `r53dv3` | E13 `58ad4a6aa720375a` | E13 `f7726113`(225 件) | 止まらない |
| 2 | V4 | `r53dv4` | work-r25 `921bb7e2ed1ad326` | work-r25 `7cd2eaec`(228 件) | 停止する |
| 3 | V1 | `r53dv1` | E13 `58ad4a6aa720375a` | work-r25 `7cd2eaec`(228 件) | 下の 4 通りのどれか |
| 4 | V2 | `r53dv2` | work-r25 `921bb7e2ed1ad326` | E13 `f7726113`(225 件) | 下の 4 通りのどれか |

- 件数は、この観測で seed pack の header を読んで数えた値(225・228)。
- 前の観測(0370)の対照: 台本 A の work-r25 は 3/3 停止、E13 は 0/3(`evidence/R5-3c.md`)。

## 静かさ
- 各回の直前に `node tools/verify-kit/tools/wait-quiet.mjs 8 10` を実行し、rc を記録する。道具の実行後に `/proc/loadavg` を記録する。
- 基準は 0367・0370 と同じ(swap の入出力が 0 でない・1 分 loadavg が 8 超・I/O some avg10 が 0 でない回は数えず、同じ位置で撮り直す。最大 3 回の試行)。rc が 1 の回は道具を起動せず、同じ位置で撮り直す。
- 未確認: 停止の有無が静かさに左右されるか。本書は静かさに敏感だと仮定していない。

## 予測(観測の前に書く)
- V3(E13 の wasm・E13 の seed。手順の対照): 止まらない(E13 と同じ。0370 で 3/3 止まらなかった)。外れたら、手順が結果を変えている(V1・V2 の読みに使えない)と書く。
- V4(work-r25 の wasm・work-r25 の seed。手順の対照): 停止する(work-r25 と同じ。0370 で 3/3 停止)。外れたら、手順が結果を変えている(V1・V2 の読みに使えない)と書く。
- V1 と V2 は、次の 2 通りのうちどちらかとは予言しない(main は seed が引き金かを知らない)。観測の前の読み:
  - seed が引き金なら、V1(E13 の wasm + work-r25 の seed)は停止し、V2(work-r25 の wasm + E13 の seed)は止まらない。
  - seed が引き金でなければ(wasm 側に原因があれば)、V1 は止まらず、V2 は停止する。
  - どちらでもない組合せ(両方停止・両方止まらない)なら、seed と wasm の相互作用か、別の要因。
- 開示: main は R5-3c の結果(E13 のみ止まらない)と seed pack の件数(E13 225・work-r25 228)を見ていた。予測は外れ得る。

## 参考(旧の別の観測。判定に使わない)
- `evidence/R5-3c.md` の台本 A の work-r25 3/3 停止・E13 0/3(同じ道具・同じ台本)。

## 未確認(観測の前の時点)
- 停止の原因(API・ゲストの関数・seed 資材との関係)。
- seed pack の差のどのファイル(`vfs-seed-idle1.tsv`・`BigFruit.png`・`AboutTxt.txt`・`msgothic.ttc` のどれか)が停止に関わるか。本書はファイル単位の切り分けをしない。
- E13 の manifest の `seedPack.encodedSize`(3929989)が実際の `.br`(4446700)と合わない理由。runtime では読まれない(手順 2 の読み)。
- 実機との比較は 0 件(H6)。

## 出典
- `evidence/R5-3c.md`・`docs/decisions/0370-about-close-abort-bisect.md`(台本・定義・対照)。
- `evidence/R2-7c.md`(seed pack の 4a・4b。件数の出典)。
- `docs/decisions/0369-seed-assets-about-ja.md`・`docs/decisions/0367-p3-e13-vs-r25.md`(書式と静かさ)。
- 各 bundle の `manifest.json`、`app/src/bundle/manifest.ts`・`loader.ts`・`app/tools/assets-middleware.ts`(manifest と `.br` の扱い)。

## 結果の追記(2026-11-15、`evidence/R5-3d.md`)

- 定義の適用(組ごと。n=3。`evidence/R5-3d.md` の手順 7 の表):
  - V3(E13 の wasm・E13 の seed。手順の対照 1): 止まらなかった 3/3。
  - V4(work-r25 の wasm・work-r25 の seed。手順の対照 2): 停止 3/3(code 134・定義 2 の message と一致)。
  - V1(E13 の wasm・work-r25 の seed): 停止 3/3(code 134・定義 2 の message と一致)。
  - V2(work-r25 の wasm・E13 の seed): 止まらなかった 3/3。
- 「別の結果」「配れなかった」は 0 件。
- 予測との比較: 手順の対照 V3・V4 は当たり。V1・V2 は「seed が引き金なら V1 は停止し V2 は止まらない」に当たった(4 通りのうちこの 1 つ)。
- 「n=3。この機械だけ(headless Chrome)。実機との比較は 0 件(H6)」。
- 判断の規則は置いていない。次の作業の選択は別の決定文書で行う。
