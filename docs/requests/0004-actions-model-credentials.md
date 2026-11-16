# Issue 0004: Actions でエージェントを動かすためのモデルの認証情報

状態: 取り下げ(2026-11-16)。この依頼は、依頼者の AWS の認証情報を作らせる内容で、誤りだった。依頼者のモデルの認証は、opencode の console に登録した BYOK だけで、AWS の認証情報は使わない。GitHub の Issue 4 は、2026-11-16 の確認の時点で GitHub から消えていた(`gh issue view 4` は解決できない。消した人と経緯は未確認)。旧本文は git の履歴(`git log -- docs/requests/0004-actions-model-credentials.md`)に残る。決定 0382。

補足: 依頼者に AWS の認証情報を求めない規則は、AGENTS.md の依頼の節と docs/ops/requests.md に置く。
