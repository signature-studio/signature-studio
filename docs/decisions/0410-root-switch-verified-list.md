# 0410: deploy.yml に入口(R2 の root の manifest.json)の切り替えを戻す。検証済みの wasm の一覧に最終 wasm が在るときだけ(2026-11-17 に 1 回実施。§7)
分類: (b) 外部(R2 の公開の入口)の契約の実装。切り替えの条件は一覧の 1 ファイルだけで決まる。
撤回条件: (1) 依頼者が入口の切り替えを deploy.yml から外すよう指示したとき(0400 の撤回条件 (1) の逆)。(2) 一覧に載せた wasm の実測(起動 n=3・Export・Problems・P1)が条件を外れたとき、その sha を一覧から 1 行消す commit で戻す。
日付: 2026-11-17   ROADMAP: R6-33   一覧: 1 件(b440e13a623a。commit b8d2d416。切り替えの run 44014410031 は success)   コミット: fa751e1a..16fe90e8(fb7a3ef8 = deploy.yml と一覧、11f40f0e = pack.args の説明 1 行、db00513d = 決定と記録、d0d91b30 = endpoint の統合、16fe90e8 = add-mask の除去)

## 0 範囲と件数(設計の前の確認)

| 範囲 | 検索・確認 | 件数と ID |
|---|---|---|
| 依頼者の指示(2026-11-16。main 経由の原文) | 本文 | 入口の切り替えを deploy.yml に戻す。100 行・80 字・ブロック形式。発言 ID は未確認 |
| 依頼者の過去の記録(`~/work/handover/setup/S16-intent/catalog.md`) | 全 84 行を読んだ。`grep -n -i "root\|manifest\|切り替\|差し替\|verified\|既定\|入口\|本番"` | 一致 0 行。型 P9(指定の形・大きさを超えて足す)に当たる恐れがあるため、2 行の指定を 1 行超えた点を §2 の 2 に書いた |
| 旧の入口のステップ(`git show 5b4f57c5^:.github/workflows/deploy-engine.yml` の 815〜935 行) | 全行を読んだ | 121 行。機能 8 件(§1 の表) |
| 旧の gate と差し替え | 同じファイルの 833〜838 行、903〜908 行 | 2 件 |
| 0400・0403 | `grep -n root docs/decisions/0400-*.md docs/decisions/0403-*.md` | 0400 の撤回条件 (1) が今回の指示に当たる。0403 の 50 行目は『本番の root は読まず、書いていない』 |
| 鍵の形 | `app/scripts/pack-assets.mjs` の 77〜83 行 | `<prefix>/<base>.<sha の先頭 12 字>.<ext>`(sha を含む) |

## 1 入れた物と、置かなかった物(旧の入口のステップの 8 件)

| 旧の機能 | 行き先 | 理由 |
|---|---|---|
| ① 一覧の gate(wasm の sha256 が一覧に在るとき) | 入れた: `grep -qx $SHA lift/harness/web/verified.txt &&` | 依頼者の指示 |
| ② 参照先 5 件の存在の確認(head) | 置かない。切り替えは `aws s3 sync` の後だけ(sync が失敗すると step は止まる) | 100 行の制約。sync の終了で代える |
| ③ 入口の読み・同じ bytes なら何もしない | 置かない | 同じ内容を書くだけ |
| ④ 退避(put・get と sha の確認) | 置かない | 100 行・スクリプト禁止。§5 の未確認 |
| ⑤ 書き込み(content-type application/json・no-cache) | 入れた: `aws s3 cp`(同じ 2 つの属性) | 依頼者の指示 |
| ⑥ 書き込み後の head(大きさ)と get(sha) | 置かない | 100 行。cp の終了で失敗は見える |
| ⑦ 失敗時の戻し(restore) | 置かない | ④ と同じ |
| ⑧ 出力(GITHUB_OUTPUT・結果の表) | 置かない。`echo "verified: N"` の 1 行で代える | 0400(結果の表は run が出す) |

加えて、`CF_ACCOUNT` の 3 重複(キャッシュ・載せ・載せ直し)を、GITHUB_ENV への書き込み `AWS_ENDPOINT_URL` と `RUNS_ON_S3_BUCKET_ENDPOINT`(cache ステップの env を消した)に統合した。分類 (a) 除去(同じ値の重複。実行の値は変わらない)。`::add-mask::` は入れていない(d0d91b30 で入れ、main の決定で 16fe90e8 により外した)。

## 2 決定

1. 入口の切り替えは `aws s3 sync`(bundle/br・bundle/raw)の後に、一覧の gate と `aws s3 cp` で行う。根拠: 依頼者の指示(位置と条件)。
2. ステップの末尾に確かめの 1 行(`echo "verified: N"`)を置く。依頼の 2 行に対し、この行で 3 行になった。根拠: 末尾が `grep … && aws …` の列だと、一覧が空のとき step が rc=1 で失敗する(§3 の (c))。
3. 一覧は改行 1 つだけ。切り替えは、検証した wasm の sha256 を 1 行足す commit で行う。根拠: 依頼者の運用。
4. `AWS_ENDPOINT_URL` と `RUNS_ON_S3_BUCKET_ENDPOINT` を GITHUB_ENV に書く。後のステップ(キャッシュ・載せ・wrangler-action)が同じ値を受け取る。根拠: (a) 除去。アカウント ID を含む endpoint がログに出ることは、main の決定により許容する(app/wrangler.jsonc の account_id として依頼者が許可した範囲。evidence/R6-33.md §4)。
5. 旧 gate の残り(②③④⑥⑦⑧)は置かない。根拠: 100 行・80 字の制約と、依頼の 2 行の指定。残る危険は §5 に書いた。
6. 一覧の運用(依頼者の運用。main の指示): 検証した wasm の sha256 を 1 行足す commit で切り替える。条件: 起動 n=3 以上・About を閉じた後に止まらない・Export の決定性 n=2 以上・Problems の件数・P1 が 10 秒未満。

## 3 実測(コマンドと出力。この機械)

- `wc -l .github/workflows/deploy.yml` → 100(fa751e1a)、100(fb7a3ef8)、100(d0d91b30)、99(16fe90e8。add-mask を外した後)。`wc -L` → 80(いずれも)。`awk 'length > 80'` → 0 行。
- `yaml.safe_load` → steps 10(変更前も 10)。
- (c) bash `-eo pipefail` で、末尾が `grep -qx X 空のファイル && echo cp` → rc 1。末尾に別のコマンドを足すと rc 0。
- (d) 偽の aws(呼び出しを記録するだけ)で、deploy.yml の該当ステップの本文を抜いて実行。一覧が空: `verified: 0`、rc 0、aws は sync の 1 件。一覧に在る: `verified: 1`、rc 0、aws は sync と `s3 cp` の 2 件。cp が rc 7: step の rc 7。GITHUB_ENV: `AWS_ENDPOINT_URL` に `https://` とテストの値 `0123` を並べた行が書かれた(ホストの接尾辞は文書に書かない)。
- `scripts/scan-forbidden.sh` → deploy.yml・verified.txt で 0 件、pack.args で 0 件。

## 4 読み(実行していない)

- 鍵は sha を含む。未検証の deploy は新しい engine の鍵を増やすだけで、既存の鍵は上書きしない(同じ sha なら同じ bytes)。毎回上書きされるのは `fl-Oz/manifest.json`(0023)。
- 入口(root)は、一覧に在る sha の deploy でだけ変わる。一覧が空の今は変わらない。
- 切り替えは PUT の 1 件。中間の状態を作らない、と読む(R2 の挙動は未確認)。

## 5 未確認

- 退避と戻しが無い。今の入口(R6-7 の `fl-engine-ci-d5d440fa` を指す)を戻すには、R2 の版の履歴が要る。R2 の版管理は未確認(R2 は読まない)。
- 参照先の存在を cp の前に見ていない。sync の終了に依る。
- wrangler-action が AWS_ENDPOINT_URL を読むか。run のログで確かめる。
- 切り替え後の公開の挙動(P1〜P3)は、sha を一覧に足す commit で測る。

## 6 出典

- 依頼者の指示(2026-11-16。main 経由。発言 ID は未確認)。
- 決定 0400(撤回条件 (1))、0403 の 50 行目、失態 0023(fl-Oz/ の書き込みの経緯と追記)。
- `5b4f57c5^:.github/workflows/deploy-engine.yml`(入口のステップ 815〜935 行、gate 833〜838 行、差し替え 903〜908 行)。
- `app/scripts/pack-assets.mjs` の 77〜83 行(鍵の形)。
- `evidence/R6-33.md`(run の結果)。

## 7 実施(2026-11-17)と戻し方の限界

1. 決定: 候補 b440e13a623aa949…e043(R6-34 の観測の対象)を一覧に足し、切り替えを 1 回実施した。根拠: main の決定(evidence/R6-34.md)と、run 43999015862 のログの pack-assets の sha256 が一致したこと。
2. 実測: run 44014410031(headSha b8d2d416。success)で、`s3://fl-web-assets/manifest.json` への upload が 1 件、`verified: 1`、wrangler 段 success(evidence/R6-33.md §6)。
3. 戻し方の限界(事実): 一覧から 1 行を消しても root は戻らない。root を戻す CI の段は無い。戻しの候補は、(b) 旧版の lift で c963715a4410… を作り直して一覧に足す(再現は未確認)、(c) fl-engine-ci-d5d440fa/manifest.json を root に書き戻す段の追加(決定が要る)。
4. 退避: 切り替えの前の root を退避する段は無い。R6-7 の退避(backup/manifest-root-20261116T021018Z.json)は R6-7 の前の root。

## 8 戻しの段(2026-11-17。main の決定 (c) の変形)

1. 決定: `lift/harness/web/root.txt`(空で作る)が空でないとき、中のキー(fl-web-assets の下の manifest.json のキー)を `fl-web-assets/manifest.json` へ `aws s3 cp` する 2 行を、gate の後に置く。root.txt が最後に勝つ。空(今)は何もしない(`[ ! -s ] ||` で rc 0)。根拠: main の決定。
2. 行数: deploy.yml は 99 → 100 行。100 行に収めるため、確かめの echo の行(`verified: N`)を外した。gate の結果は、ログの `upload -> s3://fl-web-assets/manifest.json` の行の有無で確かめる。
3. 検査(偽の aws。`bash -eo pipefail`。deploy.yml の該当 6 行を抜いた。`/tmp/opencode/rootsw`。後で消した): T1 一覧の空・root 空 → aws 0 件、rc 0。T2 一覧に在る・root 空 → gate の cp 1 件、rc 0。T3 一覧の空・root 在り → root の cp 1 件(`fl-engine-ci-d5d440fa/manifest.json` を入れたテスト用の値)、rc 0。T4 両方 → gate → root の順、rc 0。T5 root の cp が失敗 → rc 7。T6 sha 不一致・root 空 → aws 0 件、rc 0。
4. 未確認: S3 → S3 の cp(R2 上の CopyObject 相当)が R2 で動くか。root.txt が空の今は実行されない。最初に使うときがテストになる。R2 は読んでいない。
5. 戻しの材料(R6-7 の記録。現在の R2 は未確認): 切り替え前の root は `fl-engine-ci-d5d440fa`(R6-7 §3: root の後の sha256 `1c78c19e…e885`、3,097 B)。その prefix の 5 件(manifest.json を含む)は R6-7 §2・§4 で head・get により確認された。R6-7 の退避は `backup/manifest-root-20261116T021018Z.json`(sha256 `27b81add…`。R6-7 の前の root で、切り替え前の root ではない)。
6. 戻す手順: root.txt に `fl-engine-ci-d5d440fa/manifest.json` の 1 行を書く commit。一覧から b440 の行を消す commit は任意(root.txt が後に勝つため、なくても戻る)。
7. 限界: root.txt のキーが R2 に無いと cp は失敗し、step が失敗する(T5 と同じ形。実 R2 では未確認)。戻し先の manifest が参照する wasm・glue・seed・dll が R2 に在るかは、この記録では確かめていない。
