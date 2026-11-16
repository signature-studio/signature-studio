# R5-4: reviewer の検査の報告(コピー)

この節の下の全文は、R5-4 の reviewer の検査の報告のコピー。このコピーを作った worker は、この検査を再実行していない。報告の本文の改変は 1 箇所だけ(reviewer の報告にあった、用語表で禁止されている語を、用語表の語に置き換えた)。判定と数字は変えていない。報告に、セッション ID やホームの絶対パスは含まれていなかった(コピーの時点で確認)。

## reviewer の報告(全文)

## 1. 判定

**R5-4 を `done` にする判定材料としては受理。** 対象は完了条件「sha256 の一致/不一致が `evidence/R5-4.md`」です。差し戻しは 0 件です。数値・時刻・組の順・label と bundle の対応に、記録と実物の不一致はありませんでした。

- 未確認 4 件: reviewer の道具では再現できない記録の主張です。(a) `cmp` による「同じバイト」の確認、(b) 各回の直前の `free -m`・`/proc/loadavg`・`ps` の値、(c) 各回の rc=0 と stdout の終わり、(d) 組の後の `ps`。`done` の文面では記録の主張として残してください。
- 判定の材料: `export.json` 5 件の sha256 は、期待値と 64 桁すべて一致しました(literal 検索 5/5)。size は 5 件とも 22,863,828 B。`report.json` は 5 件とも `stopped:false`・`state:engine-idle`・`opErrors:[]`・`error:null`・`ok:true`。組の順は E13 → work-r53k → E13 → work-r53k。0377 の本文(c85d953)と測定前の補足(adee702)は、どちらも組より前に commit されています。
- netlog: E13 の 2 回は `fl-engine-ci-0363b050` が各 24 件、`fl-Oz` は 0 件。work-r53k の 2 回は逆です。
- 目視: 組の 8 枚をすべて開き、記録の記述と一致しました。
- 道具 3 ファイルは c85d953 以降に変更がありません。0377 の 1〜44 行は不変です。
- ROADMAP: R5-4 の行(52 行目)は `todo` のままです。HEAD までの ROADMAP の変更は 3226007 の R5-3 行(`done`→`todo`)の 1 hunk だけです。
- 「達成」などの肯定の判定は 0 件。出典のない断定は 0 件。
- ワークツリーに未 commit の変更は無し(`git status --short` は `evidence/R0-6-failfast.txt` の未追跡 1 件のみで、R5-4 とは無関係)。

## 2. 項目ごとの結果

**項目 1 変更範囲: 一致**
- `git show --numstat --format=%h adee702` → `8 0 docs/decisions/0377-export-sha-observation.md`
- `git show --numstat --format=%h 5477576` → `17 0 …0377…md`、`98 0 evidence/R5-4.md`(新規)
- `git diff c85d953 HEAD -- docs/decisions/0377-…md` → `@@ -42,3 +42,28 @@` の追加のみ

**項目 2 時系列: 一致**(`git log` と `ls -la --time-style=full-iso` の読み。推定開始 = `report.json` の更新時刻 − `seconds`。`seconds` の定義は記録自身も未確認としている)

| 出来事 | 時刻(+0900) | 出典 |
|---|---|---|
| c85d953(0377 本文・道具) | 2026-11-15 16:45:54 | git log |
| 道具の確認の run(`r54-r54shake-chrome-1`、数えない) | フォルダ 16:48:17 | ls |
| 3226007(R5-3 を todo に) | 16:54:20 | git log |
| 数えない E13 の run(`r54-r54e13-chrome-1`) | 最初の成果物 16:55:28、推定開始 16:55:03 | ls・report |
| adee702(測定前の補足) | 2026-11-16 09:04:57 | git log |
| 組 1 E13(`-chrome-2`) | 最初の成果物 09:06:09、推定開始 09:05:42 | ls・report |
| 組 2 work-r53k(`-chrome-1`) | 09:09:07、推定開始 09:08:41 | 同上 |
| 組 3 E13(`-chrome-3`) | 09:11:42、推定開始 09:11:16 | 同上 |
| 組 4 work-r53k(`-chrome-2`) | 09:14:17、推定開始 09:13:51 | 同上 |
| 5477576(結果) | 09:21:06 | git log |

- (a) c85d953 は 5 つの run のどれより前 → 一致
- (b) adee702 は組の 4 回より前(最初の成果物の時刻 09:06:09 で言える。推定開始でも同じ) → 一致
- (c) 組の順は E13 → work-r53k → E13 → work-r53k → 一致
- 0377 補足 (c) が予測した出力フォルダ名(`-chrome-2`・`-chrome-3`・`r53k-chrome-1`・`-chrome-2`)と実物は一致

**項目 3 数値: 一致**(`export.wav` は `ls -la` で 5 件とも 22863828 B)

| フォルダ | 組の順 | export.json の sha256 = 期待値(literal) | seconds | stopped | state | opErrors | error | ok | openedTitle |
|---|---|---|---|---|---|---|---|---|---|
| r54-r54e13-chrome-1 | 数えない | 一致 | 112 | false | engine-idle | [] | null | true | serina1-serum2.flp - FL Studio 2026 |
| r54-r54e13-chrome-2 | 1 | 一致 | 125 | false | engine-idle | [] | null | true | 同じ |
| r54-r54r53k-chrome-1 | 2 | 一致 | 125 | false | engine-idle | [] | null | true | 同じ |
| r54-r54e13-chrome-3 | 3 | 一致 | 125 | false | engine-idle | [] | null | true | 同じ |
| r54-r54r53k-chrome-2 | 4 | 一致 | 124 | false | engine-idle | [] | null | true | 同じ |

- 期待値 `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` は、`tools/verify-kit/README.md` 22 行・0377・evidence の表と 64 桁一致。
- 「4 回の WAV は同じバイト」について: sha256 の記録が 4 回とも同じなので、SHA-256 の衝突がなければ同じバイトと言えます。ただし reviewer は WAV の sha256 を再計算しておらず、`cmp` も実行できないため、「同じバイト(cmp)」は未確認です。

**項目 4 label と bundle: 一致**(`grep` の件数。各行は netlog の URL パスを読んだ。host は 127.0.0.1)

| フォルダ | 組の順 | `fl-engine-ci-0363b050` | `fl-Oz` |
|---|---|---|---|
| r54-r54e13-chrome-2 | 1(E13) | 24 件 | 0 件 |
| r54-r54r53k-chrome-1 | 2(work-r53k) | 0 件 | 24 件 |
| r54-r54e13-chrome-3 | 3(E13) | 24 件 | 0 件 |
| r54-r54r53k-chrome-2 | 4(work-r53k) | 0 件 | 24 件 |

**項目 5 目視: 一致**(組の 8 枚をすべて開いた。例: `tools/verify-kit/out/r54-r54e13-chrome-2/missing-plugins-dialog.png`、同 `export-dialog.png`。他の 3 回も同じ 2 枚)
- `missing-plugins-dialog.png`: 「Problems loading the project」「8 plugin(s) could not be opened」「and the plugin(s) will be ignored.」。一覧 8 件(Fruity Wrapper (FL)、FLEX (FL)、Frequency Shifter (FL)、Fruity Balance (FL)、Soundgoodizer (FL)、Fruity Parametric EQ 2 (FL)、Fruity Delay 2 (FL)、Fruity Filter (FL))、すべて Error。Ok は画面上でおよそ (983, 683)
- `export-dialog.png`: 「Rendering to serina1-serum2.wav」、Full song、Length 108 bars、Total time 2'10''、Disk space 21.80 MB、16Bit int・Stereo、Start は画面上でおよそ (1134, 832)。Problems の窓は無い
- 同じ回の 2 枚の PNG のバイト数は、E13 の 2 回目(-chrome-2)と work-r53k の 1 回目(-chrome-1)が同じ(353396 / 382860)。work-r53k の 2 回目(-chrome-2)だけ少し違う(353244 / 382713)が、目視では差は見えない

**項目 6 道具: 一致**
- `git log --oneline -- tools/verify-kit/tools/sdriveg2-export.mjs tools/verify-kit/serve-once-r54-bundle.mjs tools/verify-kit/tools/scn-r54-export.json` は c85d953 が最新で、以後の変更は 0 件
- `git show --numstat --format=%h c85d953`: `sdriveg2-export.mjs` 41 追加・0 削除、`serve-once-r54-bundle.mjs` 63・0、`scn-r54-export.json` 28・0

**項目 7 記録の誠実性: 一致**(指摘は軽微 2 件)
- grep `達成|直った|効いた|問題ない|整合|到達`: evidence/R5-4.md 3 行は否定(「宣言ではない」)、0377 の 4 行も否定(「根拠に使わない」)。肯定 0 件
- 出典のない原因の断定: 0 件。Mem total の違い(14647 対 25425)は「未確認」と明記(R5-4.md 38・91 行)。E13 1 回目の停止は「依頼文の記述によれば」と出典付き(67 行)
- 先頭の「※ 完了ではない」(3 行)、「ROADMAP の状態は変えない」と「検証用サブエージェントによる検査は未実施」(96・97 行)あり
- 数えない回の扱い: 補足 (b)(0377 の 50 行付近)と一致し、2 節に別記されている。ただし、この区分は 0377 本文の「数えない回」の定義(WAV の有無が読めなかった回)に無い新しい区分。決定の commit は組より前。除外した 1 回は一致だったので、除外で結論が有利になってはいない
- 用語: 「前の作業用サブエージェント」は glossary の「区別が要るときだけ使う語」(25・28 行)の用途に当たる(受理)。「依頼文」は、用語表で「依頼」を「エージェントから依頼者への頼み」と定めているため紛らわしい(軽微な指摘)

**項目 8 秘密・ID: 一致**
- grep `ses_|/home/|[A-Za-z0-9]{32,}`: evidence/R5-4.md 7 件、0377 1 件。すべて sha256 の 64 桁。`ses_`・`/home/` は 0 件

**項目 9 ROADMAP: 一致**
- `git diff c85d953 HEAD -- docs/ROADMAP.md` は 1 hunk(`@@ -48,7 +48,7 @@`)。R5-3 行が `done`→`todo`。3226007 の commit 内容と一致
- `git log -- docs/ROADMAP.md`: c85d953 より後の変更は 3226007 のみ
- grep `^| R5-4 |`: 52 行 `todo`。R5-4 の行は変更なし
- 注: `git cat-file -t 3226007` は shell で `permission.rejected`。他の git 読み取りは通った

**項目 10 `done` の括弧書き案**(追加のみ・`|` を含まない・3 文)

（組の 4 回の sha256 は期待値と一致し、E13 と work-r53k で sha256 が同じ(直前の記録がない 1 回は組に数えない)。この機械・headless Chrome・n=2 で、依頼者機・Wine との比較は 0 件。G6〜G8 の達成ではなく、P2・P3 の実測が揃うまで G7 以降は達成と書かない。）

`done` にする場合は、evidence/R5-4.md 97 行の「検証用サブエージェントによる検査は未実施」を実施済みに直すことが要ります(main か worker の作業)。

## 3. 不一致の一覧

0 件。

## 4. 出典のない断定の数

0 件。再現できず、記録の主張としてだけ残るもの: 4 件(上の未確認 (a)〜(d))。

## 5. 直すとよい点(差し戻しには当たらない)

1. evidence/R5-4.md の 11 行・60 行、および 0377 の結果追記 68 行の「同じバイト(cmp)」は、「sha256 が 4 回とも同じ」に言い換えるか、「cmp は未確認」と書く。
2. 「依頼文」(R5-4.md 67 行、0377 補足 (a))は、用語表の「依頼」と紛れる。別の語にするか、用語表の扱いを確認する。
