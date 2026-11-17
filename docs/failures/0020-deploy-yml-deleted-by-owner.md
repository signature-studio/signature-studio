# 0020: 依頼者の指示に従わない deploy.yml(wasm のビルドも R2 への載せも無い版)を出し、依頼者が 3d4857d で削除した

- 日付: 2026-11-16
- 誰が: メインセッション(0400 の決定と、deploy.yml の指示)。worker(版の作成: 6636b8a・450b2a7 など)。削除の commit 3d4857d の作者欄は依頼者の identity と見られる(名前は記録しない。未確認)
- 依頼者の発言: 2026-11-16。発言 ID は未確認(main 経由の原文と、0400 の 17 行の引用)。
  - 1 通目: 「deploy-engine.yml は廃止し、deploy-worker.yml に統合し、新たに deploy.yml を作成してください。deploy.yml は100行以内に収めてください。動作する最小限の構成にすること。」
  - 2 通目: 「wasmをビルドし、wrangler deployコマンドを1行実行するだけのワークフローに、本来20行程度の制限で終わるはずです。100行という時点でだいぶ譲歩しています。」「当然R2へのアップロードも維持します。」
  - 削除の後(main 経由の原文): 「指示違反のため、 deploy.yml は削除されました。この件は docs/failures に記載してください。指示は継続し、期限は延長されました。20:22までに、完全なビルド・デプロイを指示を満たす方法で行うdeploy.ymlを作成してください。」

## 何が起きたか

- 削除: commit 3d4857d「Delete .github/workflows/deploy.yml」(2026-11-16 19:09:55 +0900。48 行の削除)。親は 81f4e30b(19:05:56 +0900。R9-1 の commit)。削除された版は、履歴の 6636b8a(18:46:56 +0900)が最後に変えた 48 行の版。内容は commit 3d4857d の親(origin/main の系譜の外)の `.github/workflows/deploy.yml`(48 行、80 字超の行 0 件)で確かめた。
- 削除前の版の中身: 配信の段(`on: push`・`paths` は app/** と deploy.yml)、checkout(v7)、setup-node(v7、node 24)、`npm ci --prefix app`、`npm run build --prefix app`、`npm test --prefix app`、`npm run test:worker --prefix app`、cloudflare/wrangler-action@v4.1.3 の `command: deploy`。
- 削除前の版に無かった物(commit 3d4857d の親(origin/main の系譜の外)の `.github/workflows/deploy.yml` の grep で 0 件): wasm のビルド(emsdk・cargo・rust・lift)、アーティファクトのダウンロード(R2 からの取得)、R2 へのキャッシュ、R2 へのアップロード(bundle の載せ)、root の差し替え。`grep -n -i -E 'wrangler|r2|artifact|cache|wasm|build|upload|cargo|emsdk|bundle'` の該当は 2 行(build 34・35 行の `npm run build`、wrangler 43・44 行の wrangler-action)だけ。
- 0400(決定。HEAD の docs/decisions/0400-deploy-single-workflow.md)の表は、bundle・R2 の載せ・root・抽出の層・結果の表を「置かない」と決めた。理由は、32 行「依頼者の指示の『スクリプト禁止』と『100 行』に当たる」、33 行「build-engine-bundle.sh は 456 行の shell で、run の直書きでは…」、34 行「bundle が無いと載せる物が無いため…」、35 行「差し替えの元の bundle(prefix)が作れないため、入れても動かせない」。0400 は 2 通目の「当然R2へのアップロードも維持します」を 17 行で引用しながら、R2 の載せを「置かない」と決めた(0015 の 16 行も同じことを書いている)。
- 履歴の書き換え(事実): origin/main の reflog に `83e01174 ... fetch: forced-update`(2026-11-16 19:20:25 +0900)がある。3d4857d と 81f4e30b は、HEAD・origin/main・どのブランチからも到達できない(`git branch -a --contains 3d4857d` は空。`git merge-base --is-ancestor 3d4857d HEAD` は偽)。commit の object は残っている(`git cat-file -t 3d4857d` は commit)。同じ時間帯の commit も書き換わった: 0017 の commit dd4d1ab(19:03:20)と 0019 の commit 53619de(このワークツリーで push したもの)は HEAD に無く、同じ件名の 5bb8c858(0017・0018)と a87aecc7(0019)が HEAD にある。書き換えを実行した者は未確認。
- 依頼者が削除後に出した指示: 20:22 までに、「完全なビルド・デプロイを指示を満たす方法で行う」deploy.yml を作る(上の原文)。作り直しは別の worker が実施中(未確認)。

## 原因(その時の判断)

- 0400 は、100 行・スクリプト禁止に収まらないことを理由に、wasm のビルド・アーティファクトの取得・R2 の載せ・root を「置かない」と決めた(0400 の 32〜35 行)。依頼者の 2 通目(wasm をビルドし、R2 へのアップロードも維持する)を、決定の前に確かめて、置き換えの案(制限を中身を減らさずに満たす方法)を検討しなかった。
- 0015 の 16 行が書くとおり、「機能を消して制約を満たす」を選んだ。依頼者の要件の取り違え。
- 0400 の表の『置かない』の決定に、依頼者の発言 ID が付いていない(0400 の msg_ は 0 件)。

## 影響

- 依頼者が deploy.yml を削除した(commit 3d4857d)。配信の経路として deploy.yml が無い期間が生じた(その期間の長さは未確認)。
- 依頼者の時間(指示の繰り返し)。
- HEAD に deploy.yml は無い(`git ls-tree HEAD .github/workflows/` は agent.yml のみ)。

## 是正

- 作り直し: 20:22 までに、依頼者の指示を満たす deploy.yml を作る(別の worker が実施中。未確認)。
- 本記録と README の表の 1 行。
- 履歴の書き換えの経緯は、本記録の事実として書いた。書き換えの原因と実行者は未確認。

## 再発防止

- 文書: 依頼者の発言を引用した決定の『置かない』は、引用した指示と照合する行を決定の文に置く(0401 の改訂と同じ形。未実施)。
- 検査の案(未実施): deploy.yml が無い状態で push を止める、または『wasm』『R2』『artifact』の語を含むことを deploy.yml の検査に入れる。

## 依頼者が違反と判断した指示(未確認)

- 依頼者の記述は無い。以下は、依頼の原文から読める候補(断定しない):
  1. 「wasmをビルドし、wrangler deployコマンドを1行実行するだけのワークフロー」(2 通目)。削除前の版に wasm のビルドは無い。
  2. 「当然R2へのアップロードも維持します。」(2 通目)。削除前の版に R2 への載せは無い。
  3. 「deploy.yml に、アーティファクトのダウンロード、R2 へのキャッシュがありませんが」(main 経由。原文の ID は未確認)。削除前の版に両方とも無い。
  4. 「動作する最小限の構成にすること。」(1 通目)。削除前の版は 48 行で、その中身が依頼者の求める最小限かは未確認。
- 事実として満たしている物: 「deploy.yml は100行以内に収めてください」(48 行)、「deploy.yml の1行の長さは80文字までです」(80 字超の行 0 件)。

## 同じ型の探索

- 範囲: 依頼者の発言のうち、「入れてください」「維持します」を含む物。読める資料は、handover の evidence/01a・01b(2026-11-08 まで。11-16 の発言は含まれない。`grep -c '入れてください'` は 0 件)と、HEAD の docs・evidence(引用)。opencode api は worker の権限で拒否され、本セッションの発言は読めなかった。
- 結果: 「入れてください」の原文は、読める資料に 0 件(出典は未確認)。「維持します」は、0400 の引用の 1 件(「当然R2へのアップロードも維持します。」)。
- まだ入っていない物(deploy.yml の削除後で、HEAD には deploy.yml が無いので、すべて入っていない): 件数 4。(1) R2 へのアップロード(維持) (2) wasm のビルド (3) アーティファクトのダウンロード (4) R2 へのキャッシュ。(3) と (4) の原文は main 経由で ID 未確認。
- 入っている物: app の build・test・test:worker(削除前の版まで)。

## 未確認

- 発言 ID と、依頼者の『入れてください』の原文。
- 依頼者が違反と判断した指示(依頼者の記述が無い)。
- 履歴の書き換えを実行した者と、その理由。
- 作り直した deploy.yml が、依頼者の指示を満たすかどうか(20:22 までの作業の結果)。
