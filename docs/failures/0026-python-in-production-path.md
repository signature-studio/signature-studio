# 0026: 本番の経路(deploy.yml・agent.yml)に Python を残し、Python を禁じる規則を検査で止めていなかった

- 日付: 2026-11-17
- 誰が: worker(R6-37。作業の記録は `evidence/R6-37.md`。経緯の作成者は未確認: `git log -S 'python3 ci/lib/extract_fl.py'` で導入の commit を見る)
- 依頼者の発言: 2026-11-17、依頼者の原文(main の指示に引用): 「確かに私はあなたにPythonの実行を特別に許可していますが、検証だけでなく本番のビルドフローにすらPythonを使用するような方だったのですね…恒久指示: 調査・検証を除き、一切のPythonの使用を禁止します。この指示は、既存のコードベース全てに適用されます。」(ci/lib/extract_fl.py（削除済み。決定 0415） を指して)。発言 ID は main の指示に無く、未確認

## 何が起きたか

- `.github/workflows/deploy.yml` が本番の配信の途中で `python3 9590e599^:ci/lib/extract_fl.py`(46 行)と `python3 $H/web/final0129/pack_seed.py`(84 行)を実行していた。`.github/workflows/agent.yml` の抽出のステップも `python3 9590e599^:ci/lib/extract_fl.py`(85 行)を実行していた。出典: `git grep -n python3 -- .github`(commit 4dd4f8c3 の時点)。
- `scripts/launch.sh` も Python を 3 箇所(19・30・59 行)で使っていた(人が起動する道具。JSONC の読み取りと、モデル名の分解)。
- 検査は無かった。決定 0400 の制約の文(`docs/decisions/0400-deploy-single-workflow.md` の「制約」の行)には「ci/lib の .py の実行を含む」と書かれていたが、deploy.yml の Python の行はその文より前から残っていた。文書だけで守る規則になっていた。

## 原因(その時の判断)

- 0400 の制約を文書に書いたが、既存の `python3` の行を持ち越す判断を、その制約と照らして確かめなかった(判断の時点は未確認: `git log -S`)。
- 検査(scan の規則・テスト)を置かなかった。AGENTS.md §11 の表に、Python の規則は無かった。

## 影響

- 本番のビルド・配信の経路が Python 3 に依存していた。依頼者の指示に反する。配信物の内容は、この件では変わっていない(移植は出力の一致を確かめてから入れる)。
- agent.yml の抽出のステップも同じ。

## 是正

- 27f3bcb4: deploy.yml の抽出の行と seed pack の行、agent.yml の抽出の行を Node(app/scripts/extract-fl.mjs・pack-seed.mjs)に替えた。run 44034012920 が success(鍵は移植の前と同じ)。
- 9590e599: 原本(ci/lib/extract_fl.py・lift/harness/web/final0129/pack_seed.py)を削除した。run 44044050468 が success。
- 0079e4d5(R6-37b): 製品ソースの生成元 3 件を削除し、規則の単語区切りの穴を直した(決定 0415 §8)。reviewer の検査は未了。
- scripts/launch.sh の Python 3 箇所は node と jq に替えた。scripts/ の 3 本の検査の道具の python3 は、行単位で理由つきに許可した。

## 再発防止

- 強制: `scripts/scan-forbidden.sh` の python 規則(pre-commit・pre-push で止まる。範囲は .github・ci・app・scripts・lift。許可は `scripts/forbidden-allowlist.txt`)。
- 検査: `tests/harness/t12_python.sh`(違反の例と許可の例、実リポジトリの範囲の件数)。
- 文書: AGENTS.md §6(恒久指示)と §11(検査の行)。

## 同じ型の探索

- 範囲: `git grep` で `.github` `ci` `app` `scripts` `lift`(docs・evidence・archive・patches・tests を除く。`.md` を除く)の `python3`・`python` の呼び出しと `.py` の呼び出し。
- 件数(移植の前): `.github/workflows/deploy.yml` 2 行、`.github/workflows/agent.yml` 1 行、ci/lib/extract_fl.py（削除済み。決定 0415）(定義と docstring)、`scripts/launch.sh` 3 行、`app/test/unit/pack.test.ts` の `python3` 1 件(テストの実行)。`scripts/preflight.sh`・`check-consistency.sh`・`audit-session.sh` は検証の道具として残し、許可リストに理由を書いた。
- 移植後の件数: 本ファイルの後の `evidence/R6-37.md` の §1 を見る。

## 未確認

- 導入の commit と判断(`git log -S`)。
- 依頼者の発言 ID。
- 本番の経路の Python を外した後の deploy の run(別の worker の push の後)。
