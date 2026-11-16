# 0398: 公開ホスト名が一覧にも Secret にも無いときは、配信の後の確かめ(probe)を飛ばし、配信は止めない。Secret PUBLIC_HOSTNAME は依頼者が登録しない
分類: (a) 我々の余計な処理の除去。0389 の決定 5 の (3)(公開ホスト名が無いときは Cloudflare に書く前に止める)を外し、配信を依頼者の Secret の登録に依存させない。probe を飛ばす分岐は、この除去に伴う 1 分岐。旧設計の同じ分岐は `archive/ci-old/ci/stage-deploy.sh:63`(PUBLIC_HOSTNAME が空なら「the Worker has no route, nothing to probe」と出して exit 0)。
撤回条件: (1) 依頼者が Secret PUBLIC_HOSTNAME を登録し、run の「公開ホスト名を決める」段がその値を使ったときは、probe が動く(本決定の分岐は残す。撤回ではない)。(2) 一覧からも Secret からも公開ホスト名が得られない配信の間に、Access の背後にない公開が外から確かめられたときは、本決定を撤回し、配信の前に公開の入口を確かめる形へ戻す。
日付: 2026-11-16   ROADMAP: R6-19(blocked)・R6-20(doing。done にしない)   一覧: なし(整備)   コミット: この文書・`.github/workflows/deploy-worker.yml`・`docs/decisions/0389-deploy-worker-action-and-access-probe.md`(追記)を含む commit
番号の理由: `docs/decisions/` の最後は 0397(2026-11-16 の ls)。0398 が最初の空き。

## 検索した範囲と件数(`docs/ops/intent.md` の「設計の前の確認」)

| 範囲 | 検索の語 | 件数と ID |
|---|---|---|
| 依頼者の発言 `~/work/handover/evidence/01a-user-messages-full.md`(`grep -c '^## seq'` = 263。最後の発言は seq 114889、UTC 2026-11-08 01:28:01。それ以降は範囲外) | `\bAccess\b`(ASCII)| 0 件 |
| 同 | Access(CF Access・アクセス制御の方針) | 2 件: seq 29271 `msg_1982c56e0001L9vl9HVSX0PeIB`(CF Access で私のみアクセス可能。一般公開されない)、seq 45011 `msg_1a2cf988f001wutV6ek5oxPQTW`(Access のポリシーを利用) |
| 同 | Secret・Variable・シークレット | 5 件(配信の文脈): seq 44954 `msg_1a2ccc2c3001RSHoExRGznB5pH`、46959 `msg_1a3555a3a001pbPcxQ5Dc9M4I1`、48017 `msg_1a3964d54001FVQ76lEhAA3IYQ`、48103 `msg_1a3aec610001K648g07q3mu1fu`(Secrets は設定済み。#3 はクローズ)、113522 `msg_1b8621b57001hbV0Pz7JvaFjPl`。モデル認証の 2 件(seq 112530 `msg_1b8440ead001Ch5z3WFTvMbxa6`、113113 `msg_1b85393e8001S42niXMrcL0DQh`)は Cloudflare と無関係 |
| 同 | Cloudflare・wrangler・デプロイ・配信・Workers | 5 件: seq 44954(cloudflare/wrangler-action によるビルド・デプロイ)、48307 `msg_1a3c4f66d001910yclWtESfUQ5`、49921 `msg_1a42a812f001p8DEs4kippIVJh`(同じ語の再質問)、49993 `msg_1a42ca996001av82HqWfxJSiES`(本番デプロイを手で行う依頼は断る)、96034 `msg_1b2ef9ea9001lKkgk7g1xMhzWB`(手動デプロイは不要ではないか) |
| 同 | 公開ホスト名・ホスト名・hostname・PUBLIC_HOSTNAME・カスタムドメイン | 0 件。「ホスト」は wasm のホストの 1 件のみ(seq 131 `msg_15034d596001HdCrYgl5mxPtfQ`) |
| 同 | 公開 | 3 件: seq 20782・20794(公開リポジトリの話)、29271(一般公開されない) |
| 依頼者の発言の要約 `~/work/handover/setup/S16-intent/catalog.md` | Access・公開・Secret・ホスト・配信・Cloudflare・wrangler・deploy・probe | 0 件(0389 の表の「2 行」とは一致しない。未確認: その行の出典を再確認していない) |
| Issue 1〜8(`gh issue list --state all`、`gh issue view 8 --json …`、`gh api repos/signature-studio/signature-studio/issues/8/events`) | 閉じた理由・ラベル・コメント | 下の表 |
| 依頼者の 2026-11-16 の回答(「#7, #8 に回答しました」) | 01a の範囲外。Issue の状態で確かめた | — |

Issue の状態(2026-11-16 に確かめた値。コメントの本文と投稿者は記録しない):

| Issue | 状態 | 閉じた理由 | 閉じた時刻(UTC) | ラベル | コメント |
|---|---|---|---|---|---|
| 8(公開ホスト名の Secret の登録) | CLOSED | NOT_PLANNED | 2026-11-16T08:20:14Z | slop(付与 08:20:09Z。主体は種別 User) | 0 件。08:20:19Z にロック(主体は種別 User) |
| 7(提供元の Variable の登録) | CLOSED | DUPLICATE | 2026-11-16T08:20:58Z | slop | 1 件 |
| 6(モデルの Variable の登録) | CLOSED | COMPLETED | 2026-11-16T04:11:21Z | なし | — |
| 5(console のログイン情報の Secret) | CLOSED | COMPLETED | 2026-11-16T04:11:19Z | なし | — |
| 4 | `gh issue list --state all` の出力に無い(未確認: 削除か PR か) | — | — | — | — |
| 3(依頼者機での計測) | OPEN | — | — | なし | — |
| 2(公開リポジトリの履歴のホスト名) | CLOSED | NOT_PLANNED | 2026-11-16T00:40:29Z | slop | — |
| 1(Cloudflare の資格情報) | CLOSED | COMPLETED | 2026-11-11T19:40:43Z | なし | — |

## 依頼者の回答の読み(決定)

依頼者は、公開ホスト名の Secret `PUBLIC_HOSTNAME` の登録を実施しない。根拠: Issue 8 は、依頼者の回答として「実施しない」(NOT_PLANNED)で閉じられ、slop のラベルが付き、コメントは 0 件(上の表)。依頼者の「#7, #8 に回答しました」(2026-11-16。main の依頼)と合う。依頼者に別の方法で再び頼まない(新しい Issue を出さない)。配信を止める原因にしない。

## 決定

- 決定 1: 「公開ホスト名を決める」段は、カスタムドメインの一覧(0389 の決定 5 の (1))→ Secret `PUBLIC_HOSTNAME` の順に得る。どちらも得られないとき(無い、または形が違う)は、ホストを空にし、「得られない。probe を飛ばす。配信は続ける」と出し、成功で抜ける。分類: (a)。根拠: 依頼者の回答(上)。0389 の決定 5 の (3) は、依頼者の登録に配信を依存させていた。
- 決定 2: 形が違う Secret の値は、得られないものとして扱う(値は出さない)。分類: (a)。根拠: 形の違う値で probe を続けると、確かめる宛先が誤る。依頼者に直させない(登録は頼まない)。
- 決定 3: probe 段は、ホストが空のとき飛ばす(`if: ${{ success() && steps.host.outputs.public_host != '' }}`)。結果の表の probe の行は「飛ばした(公開ホスト名が得られない)」と出す。分類: (a)。根拠: `archive/ci-old/ci/stage-deploy.sh:63`(PUBLIC_HOSTNAME が空なら probe を飛ばして exit 0)と同じ分岐。
- 決定 4: `secrets.PUBLIC_HOSTNAME` の参照は残す。登録されれば、それを使う。分類: 整備。根拠: 0389 の決定 5 の (2)。
- 決定 5: 依頼者の発言 msg_1982c56e0001L9vl9HVSX0PeIB(一般公開されない)の趣旨のうち、Access の設定の検査は、`ci/lib/wrangler_config.py` と gen 段の検査(ACCESS_TEAM_DOMAIN・ACCESS_AUD の値の有無)が引き続き担う。ただし、この検査は Access が公開の入口に適用されていることを見ない。適用の確かめは、probe が動いたときだけ行う。分類: 整備。
- 決定 6: 0389 に追記(決定の文は変えない): 0389 の決定 5 の (3)、および deploy-worker.yml の冒頭の「どちらも無ければ配信の前に止める」は、本決定の決定 1 と 3 で置き換えた。
- 決定 7: 同じ型の探索。依頼者の登録に依存して停止する段は、3 本のワークフローで 3 段(段名と停止の分岐)。`deploy-worker.yml`: 「公開ホスト名を決める」(Secret PUBLIC_HOSTNAME。本決定で解消)。`agent.yml`: 「依頼者の opencode の認証情報を取り込む」(Secret OPENCODE_AUTH_JSON。`ci/agents/import-login.sh:7-9` で空なら停止)、「実行用のモデルの設定を書く」(Variable OPENCODE_PROVIDER。HEAD の `ci/agents/runner-config.sh:11-12` で空なら停止。Issue 7 は依頼者が DUPLICATE で閉じた)。`deploy-engine.yml`: 0 段(R2 の鍵は「在れば」で段を飛ばすため停止しない)。agent.yml の 2 段は本変更の範囲外(agent.yml と ci/agents は変えない)。未是正。
- 決定 8: 読み取り段で、Worker の binding の値(ACCESS_TEAM_DOMAIN・ACCESS_AUD・R2 のバケット名)を、全体に加えて先頭の各長さ(8 文字以上)でも伏せる。分類: (b)(外部の出力の契約の実装)。根拠: 実行 43833129108(commit d620584)の upload の段の出力(wrangler-action が出す binding の表)に、ACCESS_AUD の値の先頭 37 文字と「…」が 1 行出た。表の値は全体を伏せる仕組み(`::add-mask::`)では、切られた値に当たらない。この workflow の冒頭の「アクセス設定はログに出さない」の契約を守るため。Access の AUD は秘密の鍵ではないが、公開リポジトリのログに Cloudflare の識別子の一部を出さない規則(AGENTS §8)に従う。
- 追記(置き換え): 決定 1〜8 は、置き換えられた(deploy.yml に統合。公開ホスト名の段は無い)。依頼者の新しい指示(main 経由)により、`deploy-engine.yml` と `deploy-worker.yml` は廃止し、100 行以内の 1 本の `deploy.yml` に統合する(別の worker が作る)。この文書の決定の文は変えない。

## 実測(この機械。Cloudflare には触れていない)

- `.github/workflows/deploy-worker.yml` を `yaml.safe_load` で読んだ: 15 段。probe 段の `if` と、結果の表の probe の式を確かめた。
- host 段の分岐(偽の環境変数。`FROM_LIST` と `FROM_SECRET` を変え、`GITHUB_OUTPUT` と `GITHUB_ENV` を一時ファイルに向けた。リポジトリの外で実行): 一覧のみ → rc 0、出力 `public_host` に 1 行。Secret のみ → rc 0、1 行。両方無し → rc 0、「得られない。probe を飛ばす」、出力 `public_host=`(空)。形が違う Secret → rc 0、「得られなかったものとして扱う」、空の出力。両方 → 一覧を使う、rc 0。

## 未確認

- 実行(run)の結果。wrangler-action の versions upload・versions deploy が、この Worker の資格情報で通るか。結果は `evidence/R6-20.md` の実行 3 に書く。
- 飛ばした回の配信で、公開の入口が Access の背後にあるか。確かめていない(決定 5)。
- GitHub の式の評価(probe の `if` と結果の表の式)。run で確かめる。
- `actionlint`: この機械に無い(未確認)。
- 0389 の表の「catalog の 2 行」(31・32 行)と、この検索の 0 件の食い違い(未確認)。
- agent.yml の 2 段(決定 7)。

## 出典

- `docs/requests/0008-public-hostname-secret.md`(依頼の本文と、確認の記録)、`gh issue view 8`、`gh api repos/signature-studio/signature-studio/issues/8/events`(2026-11-16)
- `archive/ci-old/ci/stage-deploy.sh:63`(probe の飛ばし)
- `ci/lib/wrangler_config.py`(route は PUBLIC_HOSTNAME がある時だけ付く。gen 段は PUBLIC_HOSTNAME を渡さないので route は付かない)
- `docs/decisions/0389-deploy-worker-action-and-access-probe.md`(決定 5・6・7)、`evidence/R6-19.md`(実行 2)、`evidence/R6-9.md`(G9)
- 依頼者の発言: msg_1982c56e0001L9vl9HVSX0PeIB(01a 928 行付近。seq 29271)、msg_1a2ccc2c3001RSHoExRGznB5pH(seq 44954)
- 同じ型: HEAD の `ci/agents/import-login.sh:7-9`、`ci/agents/runner-config.sh:11-12`、`.github/workflows/agent.yml`(「依頼者の opencode の認証情報を取り込む」「実行用のモデルの設定を書く」)
