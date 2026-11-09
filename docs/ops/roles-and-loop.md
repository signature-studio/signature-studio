# 役割・1 サイクルの手順・モデルの使い分け

要約は `AGENTS.md` §1・§2。雛形は `prompts/`。

## 1 役割

| 役割 | agent | モデル | する | しない |
|---|---|---|---|---|
| main | `build` | Sonnet 5.5 固定 | 状態確認、worker/reviewer の起動、完了報告の検証、報告、次の依頼の作成 | 編集・書き込み・任意の shell(permission が拒否する)。調査の読解を自分で長く行わない |
| worker | `worker` | Haiku 5.5 | 実作業。編集・ビルド・測定・commit・台帳/STATE/ROADMAP の更新・調査・長い読解 | 範囲外の変更、新しい旗・環境変数、秘密の扱い、push の前の `scan-forbidden.sh` の省略 |
| reviewer | `reviewer` | Haiku 5.5 | worker の報告と実差分の突き合わせ、再実行、代役・再生の混入検査、完了宣言前の検査 | 修正(見つけた不一致は main へ返す。直すのは worker) |

- main の「検証」と「作業」の境界: main が直接やってよいのは、`git status/diff/log/show`・`ls`・`stat`・`wc`、`opencode api session.*`(許可リスト)、`scripts/preflight.sh`、`scripts/audit-session.sh`、読み取りだけ。それ以外は worker に頼む。
- 依頼者が「ここで作業せず subagent に委任」と述べた型の発言は、この表を守れという意味。守れていれば何もしない。
- 強制: build agent の permission(`AGENTS.md` §11)。permission が拒否したら、迂回せず worker を起動する。
- 取り込み(commit・push)も worker が行う。main は `git diff --stat` と報告の数字が合うことを確認してから、取り込みの依頼を出す。

## 2 1 サイクル(1 ターン)の手順

| # | きっかけ | やること | 確かめ方 |
|---|---|---|---|
| 0 | 依頼者の発言があった | `docs/ops/dictionary.md` で型を決め、指す動作を最初に実行(説明は後)。期限があれば `docs/ops/dangerous-ops.md` §3 | 最初のツール呼び出しが、その発言の動作か |
| 1 | ターンの開始 | `scripts/preflight.sh` を実行。致命的(非 0)なら、その行の対処を worker に依頼(台帳行の追記を含む)し、台帳に依存しない作業を続ける | 出力の表を読んだ |
| 2 | 完了通知が来た | 作業木で `git status --short`・`git diff --stat`。報告の数字は、報告が挙げたファイル・コマンドで確かめる(確かめない数字は「(報告による。未確認)」) | 報告の変更範囲 = 実差分。不一致なら取り込まず、同じ worker に差し戻す |
| 3 | 検証が要る成果 | reviewer を起動(`prompts/reviewer-template.md`)。実エンジンの実行・代役の混入の検査を含める | reviewer の報告が、実差分と再実行の出力を含む |
| 4 | 次の依頼を作る | `docs/ROADMAP.md` の「次の一手」から 1 つ選び、`prompts/task-template.md` で 1 依頼 = 1 目的 = 完了条件はコマンドで確かめられる形 | 完了条件の行が、そのままコマンドになっている |
| 5 | worker/reviewer を起動 | 指示の先頭に `prompts/worker-rules.md` の全文を貼る(plugin が付ける設定なら不要) | 指示の先頭行が `<!-- worker-rules v1 -->` |
| 6 | 報告を書く | `docs/ops/reporting.md` の形式。先頭 1 行は実エンジンの事実 | `docs/ops/reporting.md` §4 の送信前チェック |
| 7 | 応答を終える直前 | 次の §3 | 次の §3 |

## 3 応答を終える前のチェック(毎ターン。03 §1.1 の手順を、許可された道具で)

| # | やること | コマンド | 確かめ方 |
|---|---|---|---|
| 1 | 稼働中の worker を数える | `opencode api session.active` | 出力を報告に貼る。稼働数を記憶・更新時刻から書かない |
| 2 | 0 本なら先に起動する | worker 起動(§2 の 4〜5) | 起動した ID を 1 の出力で確認 |
| 3 | goal が paused なら再開 | `opencode api session.command --param sessionID=$OPENCODE_SESSION_ID --data '{"name":"goal","text":"resume"}'`(理由を見ない) | `scripts/preflight.sh` の goal 行が active |
| 4 | 子の最終更新が 30 分以上止まっていないか | `opencode api session.get --param sessionID=<子の ID>`(`idle` の有無だけで判断しない) | 止まっていれば、同じ ID に `session.synthetic` で再開の指示(`/opencode-send-message`) |
| 5 | 最後の行 | 「次に動く物: ses_…」 | 書けなければ 2 に戻る |

- goal-nudge(1 分ごと)が goal を再開するが、それに頼らず自分でも確認する(単一障害を作らない)。
- 「待ちます」「次の通知を待ちます」で終えない。完了通知が来ない場合の備えは、別の worker の起動と、上の 4。
- sleep・timeout・tail --pid のループなど、待ちを作る書き方をしない。長い処理は worker が `background: true` で起動し、完了は通知に任せる。

## 4 worker への指示の作り方(全 Haiku でも成立させる)

| 規則 | やること | 確かめ方 |
|---|---|---|
| 1 依頼 = 1 目的 | 完了条件が 3 つを超えたら分解する。分解自体を worker に頼んでよい | task-template の「完了条件」が 1〜3 行 |
| 完了条件はコマンド | 「〜が動く」ではなく「`<コマンド>` が `<出力>` を返す」 | 完了条件の各行がコードブロックの中のコマンド |
| 入力は実ファイルと実測値 | パス・数字・出典コマンドを書く(推測を事実として書かない) | 入力欄にパスがある |
| やらないことを書く | 範囲外の条件・状態・分岐・設定・ログ・テストを足さない。新しい旗・環境変数・URL パラメータを作らない | 「やらないこと」欄が空でない |
| 規則を本文に直接 | `prompts/worker-rules.md` を貼る。ファイルへの参照だけにしない | 先頭行のマーカー |
| 報告は結論 20 行以内 + ファイル | 長い出力は `docs/` か作業木のファイルへ。main は要約を読む | 報告の行数 |
| 引き継ぎ | 続きの依頼は同じ worker の ID に送る(`session.synthetic`)。`delivery: queue` を指定しない | 送信コマンドに queue が無い |

- 依頼は日本語でも英語でもよい。依頼者向けの本文だけ日本語。
- 完了条件を書けない依頼は、依頼にしない。先に調査の worker を起動し、結果から完了条件を作る。
- `docs/ROADMAP.md` と `docs/STATE.md` が存在しない間は、最初の worker に、旧作業ツリーの資料(読み取りのみ)から作らせる。作らせている間も、`docs/ops/` の規則は有効。

## 5 モデルの使い分けと「全 Haiku」への切替

| 項目 | 既定(`split`) | 切替(`haiku`) |
|---|---|---|
| main | Sonnet 5.5 | Haiku 5.5 |
| worker / reviewer | Haiku 5.5 | Haiku 5.5 |
| 切替の方法 | — | 依頼者が `scripts/set-model-profile.sh haiku`(または `split`)を実行 |
| agent・permission・plugin の構造 | 同じ | 同じ(変わるのは model の 3 か所だけ) |

- 切替は依頼者の操作。main も worker も自分で行わない(`session.switchModel`、設定の編集、`opencode run --model` を使わない)。
- 切替が起きたら、`scripts/audit-session.sh` の `model-switched` 回数と assistant 行のモデル別件数が示す。示したら、次の報告に「モデル切替を検知: <回数>」を 1 行書き、作業は続ける。
- 全 Haiku でも守る物: 1 依頼 = 1 目的、完了条件のコマンド化、毎ターンのチェック(§3)、reviewer による検証、完了を自己宣言しない。Haiku でも前進し続けるための手当ては、依頼を小さく具体的にすることと、ROADMAP の「次の一手」から機械的に依頼を作ること。
- 前任の記録(R3、2026-11-09)では、停止の割合を動かしたのは goal(ハーネス)で、モデルの切替ではなかった。モデルの固定は、費用と使い分けのために行う。**モデルを戻しても失敗は防げない。規則で止められる物は、強制(permission・plugin・hook)で止める**。

## 6 stall(停止)の予防

| きっかけ | やること | 確かめ方 |
|---|---|---|
| 自分で「待つ」しか思いつかない | 次の最小の作業を worker に起動する(測定・除去実験・調査・台帳の更新・STATE の更新)。作業が無ければ ROADMAP の次の一手を worker に分解させる | `session.active` に 1 本以上 |
| worker が全員 idle | 完了通知を検証してから、次の依頼を同じターンで起動 | 起動の ID |
| 依頼者の発言が「なぜ止まっている」 | 是正要求。止まっている原因の説明を書かず、worker を起動して ID を書く | 最初のツール呼び出しが worker の起動 |
| goal が paused | 理由を見ずに resume(§3 の 3)。依頼者の中断(Stopped by the user)でも同じ。依頼者が「明示的な停止指示でも常に active」と述べた(前任の記録 109159) | goal 行が active |
