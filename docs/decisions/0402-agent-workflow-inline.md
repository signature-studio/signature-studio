# 0402: 作業用の workflow の取り込み・モデルの設定・検査を run の直書きにし、Variable OPENCODE_MODEL を読む(0399 を置き換える)
分類: (a) 除去(ci/agents の import-login.sh・runner-config.sh・check-agents.sh と、確かめる段を取り除く。異常系の検査を外す)。(b) 依頼者の指示(Variable OPENCODE_MODEL の値を設定に書く)
撤回条件: (1) 依頼者が、Variable の値がログに出ないことを求めたとき(Variable を読まない形に戻す。0399 の系)。(2) 依頼者が Variable OPENCODE_MODEL を取り除くまたは値を変えたとき(依頼者の操作。この決定は変えない)
日付: 2026-11-16   ROADMAP: R9-1   一覧: —   コミット: 本決定の commit(後続の commit で記録)

## 依頼者の指示(原文。main の転送のまま)
「この際なので、他ワークフローについても是正してください。(1) ci/agents/import-login.sh を取り除くしてください。ワークフロー内で認証情報の取り込みコマンド(今 import-login.sh が実行している物)を実行するだけの処理にシェルスクリプトは不要です。(2) ci/agents/runner-config.sh も不要です。variableを読み取りjsoncを記述する処理はそのjsoncの行数分のrun: | のみ必要であり、opencodeコマンドを叩く必要は全くありません。(3) ci/agents/check-agents.sh は不要で、なんならstepごと不要です。(4) ci/extract-cache.sh は不要です。なんなら .github/actions/extract-layer/action.yml ごと不要かもしれません。なぜ runs-on/cache を採用しないのか、理解に苦しみます。つまり ci/stage-cache.sh や .github/actions/stage-cache/action.yml も不要である可能性があります。(5) 上記処理全て、および類似ケース全てにおいて、過度な異常系の考慮は全く不要です。取り除くしてください。」

## 決定
- 決定: 取り込み(`opencode auth import --standalone`。標準入力に OPENCODE_AUTH_JSON を渡す)と、モデルの設定(`.opencode/opencode.jsonc` の JSON)を、agent.yml の run の直書きにする。根拠: 依頼者の指示(1)(2)。
- 決定(a 除去): ci/agents/import-login.sh(取り除く済み)・ci/agents/runner-config.sh(取り除く済み)・ci/agents/check-agents.sh(取り除く済み) を取り除くし、段「設定を確かめる」と、段「worker の終了を確かめる」を外す。根拠: 依頼者の指示(3)(5)。
- 決定(b): モデルは Variable `OPENCODE_MODEL` の値(依頼者が登録済み。Issue 0006)を、設定の JSON の 6 箇所(model と 5 つの agent)に書く。根拠: 依頼者の指示(2)。`opencode models` は呼ばない。
- 決定(置き換え): 決定 0399 を置き換える。0399 の「Variable を読まない」「2 件以上の辞書順の選択」「候補 0 件の停止」は、この決定で消える。0399 には 1 行の追記だけ。
- 決定: 依頼者の指示(4)(ci/extract-cache.sh・stage-cache・extract-layer を runs-on/cache に替える)は、この決定の範囲の外。後続の作業で、run が success になるまで HEAD を壊さない形で行う。

## ログに出ること(依頼者の指示の結果)
- GitHub は Variables の値を伏せない。段「実行用のモデルの設定を書く」の run の表示(heredoc の展開)に、Variable の値(`<提供元>/<モデル>`。ベンダー名を含む)が出る。実測の根拠: run 43804191981 の行 271(evidence/R9-3.md の 3 節)。
- この決定は、AGENTS.md §8(ログへの禁止文字列の表示を禁じる)の例外を、依頼者の指示で認めた形になる。例外の範囲: モデル ID の値を、この段のログに出すこと。依頼者の指示の文言の出典は上の原文。
- 値が出た run の取り除く: evidence/R9-3.md の 13 節で、6 件を取り除く済み。今回の run のログにも値が出る(依頼者の指示の範囲)。

## 残したもの(報告。直書きで 15 行以内にならない、または時間の都合)
- `ci/agents/plan.sh`(タスクの差分の一覧)・`ci/agents/run-task.sh`(プロンプトの組み立てと worker の実行)・`ci/agents/summary.sh`・`ci/agents/install-opencode.sh`(版の固定と sha512 の確認)。残す理由: 直書きすると 15 行を超える。異常系の分岐は、依頼者の指示(5)の対象として、次の作業で削る。
- `ci/extract-cache.sh`・ci/stage-cache.sh(取り除く済み)・ci/stage-keys.sh(取り除く済み)・`.github/actions/extract-layer`・.github/actions/stage-cache(取り除く済み): 指示(4)。後続。

## 未確認
- run の結果(この決定の commit の後の run。evidence/R9-1.md の 6 節に書く)。
- `opencode auth import --standalone` が背景のサービスを起動しないこと(取り込みの段の後の run の結果で確かめる)。
- 並列 60 の確認と reviewer の検査(未実施)。
- `.github/workflows/agent.yml` の `opencode auth import` の文字列(依頼者の指示の文言の取り込みコマンド。worker の禁止文の解釈を PM が確認する)。
