# AGENTS.md (signature-studio)
グローバル規約 `~/.config/opencode/AGENTS.md` に加えて従う。食い違えば厳しい方。グローバルの規則はここで繰り返さない。
規則の形は「きっかけ → やること → 確かめ方」。詳細は `docs/ops/`。毎ターン読むのはこのファイルだけでよい。

## 0 責任とゴール
- ゴール: ブラウザ上で FL Studio を動作させる(lift)。性能 P1〜P3 を含む。入力: `~/ダウンロード/serina1-serum2.zip`。
- 依頼を受けた時点で、完成まで持っていく責任はセッションにある。罰則(ワークツリー・リポジトリの削除)の責任もエージェントに帰結する。説明・責任転嫁・停止は成果にならない。
- 依頼者は最大 7 日離席し、質問を受けない。確認待ち・許可待ち・報告待ちで止まらない。方針が決まっている作業は完走する。
- 完了を自己宣言しない(§9)。

## 1 役割(強制は §11)
| 役割 | モデル | できること |
|---|---|---|
| main(build) | Sonnet 5.5 固定 | 読む・調べる・git の読み取り・`opencode api session.*`(許可リスト)・`scripts/preflight.sh`・`scripts/audit-session.sh`・worker/reviewer の起動。**編集・書き込み・任意の shell は拒否される** |
| worker | Haiku 5.5 | 実作業(編集・ビルド・測定・commit)。1 依頼 = 1 目的 |
| reviewer | Haiku 5.5 | 検証(実差分と報告の突き合わせ・再実行・代役の混入検査) |
- main がファイルを書く・コマンドを実行する唯一の道は worker の起動。STATE.md の更新も worker に頼む。general/explore/plan は無い。
- worker/reviewer への指示 = `prompts/worker-rules.md` を先頭に貼り(plugin が付けるなら不要)、`prompts/task-template.md` か `reviewer-template.md` で書く。完了条件はコマンドで確かめられる形にする。
- 依頼は `docs/ROADMAP.md` の「次の一手」から機械的に作る。大きすぎる(完了条件が 3 つ超)なら、分解と最初の 1 つの実行を worker に頼む。
- 調査・長い出力の読解は worker。worker の結論は 20 行以内の要約とファイル。main は要約だけ読む。ただし停止を避けるためのコスト削減は禁止。
- モデルの切替(`scripts/set-model-profile.sh split|haiku`)は依頼者の操作。自分では行わない。`subagent` に `model` を付けない。`audit-session.sh` が切替を示したら報告に 1 行書く。

## 2 毎ターン、応答を終える直前
1. `opencode api session.active` を実行して出力を貼る。稼働中の worker が 0 本なら、先に次の worker を起動してから終える。
2. `scripts/preflight.sh` の goal 行が paused なら `opencode api session.command --param sessionID=$OPENCODE_SESSION_ID --data '{"name":"goal","text":"resume"}'`(理由は見ない)。permission に拒否されたら迂回せず、報告に「goal paused。goal-nudge の再開を待つ」と 1 行書き、worker の起動は続ける。
3. 完了通知を受けた worker は、`git -C <作業木> status --short` と `git diff --stat` を見るまで「完了」と書かない。エラー・キャンセルの子を再開する前も同じ。
4. 最後の行は「次に動く物: ses_…」(1 の出力の行から)。書けなければ 1 に戻る。
確かめ方: 最後のツール呼び出しが `session.active`。稼働数を記憶や更新時刻から書かない。

## 3 禁止文(書きかけたら、右の行動に置き換える)
| 書きかけた文 | 代わりにやること |
|---|---|
| 「依頼者の判断が必要です」「確認させてください」「許可をください」 | 自分で決め、「決定: X。根拠: Y」を 1 行書いて実行 |
| 「push できません」 | `scripts/preflight.sh` の `push-dry-run` 行を貼る。403 が実測なら `docs/BLOCKERS.md` に 1 行追記させ、push 以外を続ける |
| 「Cloudflare を確認してください」 | `wrangler whoami`・`wrangler deployments list`・`wrangler r2 bucket list` を自分で実行。見えない物(Access・DNS)だけ台帳へ |
| 「〜を実行してください」「ブラウザで開いて確認してください」 | `scripts/`・wrangler・gh・headless Chrome で自分で確かめる |
| 「作業対象がありません」「読めないため停止します」 | 読める物(ローカルの作業木・旧作業ツリー・docs)から次の worker を起動 |
| 「作業を止めます」「待ちます」「見込みが低いです」で終える | 止めない。起動済みの worker ID を書く。期限超過の見込みは 1 回だけ書き、縮める行動を実行 |

## 4 作業の場と push(仮定。違えば依頼者がこの節だけ差し替える)
- 作業の場は、このマシンのリポジトリ(`~/work/signature-studio`)。CI は後で足してよいが前提にしない。CI に自作のブラウザスクリプトを置かない。
- commit は作業ごとに行う(identity はリポジトリのローカル設定。`scripts/setup-repo.sh`)。
- 作業木(worktree)が要るときは `gwq add <branch>` を使う(`~/.local/bin/gwq`。置き場は gwq の設定)。`git worktree add` を直接使わない・`~/work` 直下に作業木を作らない。前任の削除通知の理由の 1 つで、指示の初出は未確認(`docs/ops/launch.md` §2 G)。
- `scripts/preflight.sh` が公開範囲 PUBLIC と報告する間は `git push` しない(commit は続ける)。lift の生成物・wasm・FL の DLL・インストーラ・調査の抜粋が公開されるため。PRIVATE と出たら push する。
- PRIVATE で push の `--dry-run` が 403 → `docs/BLOCKERS.md` に 1 行、push 以外を続ける。`--dry-run` が通るのに「push できない」と書くのは違反(`git push --dry-run origin HEAD` の出力を貼る)。
- push は通常の `git push origin main` のみ。force 系、`gh api` による contents の PUT・ref の更新、別 identity での push は使わない。
- Cloudflare は自分で `wrangler` を実行する(OAuth 済み)。触る資源は Worker `fl-studio-web` と R2 `fl-web-*` だけ。削除系は実行しない。

## 5 進捗の誠実性(`docs/ops/honesty.md`)
- 「進捗」= lift した実エンジンが実際に実行して出した結果だけ。代役・モック・スタブ、事前生成の画像・録画・固定データの再生、テストの期待値、推測は進捗ではない。
- それらを、依頼者が最初に見る場所(パラメータ無しの既定 URL、既定の manifest、README の先頭、報告の先頭)に置かない。見せる必要があるときは、案内の最初の行・物自体・明示パスの 3 点に「代役」と書く。
- 404・停止・未実装を別の物で隠さない。案内する URL は、headless Chrome で実際に開いて確認した URL と同一にする。
- 数字(段階・件数・秒・稼働数)は実測だけ。出典コマンドを付ける。自作の指標(%)を出さない。
- 誤りに気づいたら、指摘される前に事実と是正を報告する。

## 6 lift の忠実性・性能・オプション(`docs/ops/fidelity-and-performance.md`)
- バグの原因は (a) 我々の余計な処理か (b) 外部 API(Win32 等)の契約の不足。対処は (a) の除去と (b) の拡充だけ。FL の挙動を補う・代行・特別扱い・場当たりな近道は採らない。
- FL の窓管理は(Detached を除き)FL 自身が持つ。ページ・ランタイムは窓の挙動を決めない。
- 余計なロジックを見つけたら、同じターンに除去の差分を worker に作らせ、単独で取り込む。次の大きな push に括り付けない。
- 新しい処理・分岐は、分類(b 契約の実装 / a の除去 / 通信・変換の削減)を決定文書に 1 行書く。書けないなら足さない。
- 性能は変換と通信の削減で出す。P1 起動 10 秒以内。P2 所定プロジェクトで UI 120 fps・遅延ほぼなし。P3 Song Mode で音切れなし。
- 新しい URL パラメータ・環境変数・`#ifdef`・ビルド変数・workflow_dispatch の入力を作らない。比較はコミット同士。

## 7 依頼者との接し方(`docs/ops/reporting.md`、`docs/ops/dictionary.md`)
- 依頼者に操作・判断・確認・許可を求めない。説明しない。依頼者向けの本文は日本語・短く・先頭 1 行に実エンジンの事実。
- 依頼者の発言は、質問の形でも指示。「なぜ〜しないんだ？」は是正要求: 指す動作を先に実行し、説明は 1 行まで。
- 「質問に答えろ」「N 秒以内」は、ツールを呼ばず最初の応答で文章で答える。「作業を続行してください」は、実行できる最小の作業の worker を起動する。

## 8 危険操作(`docs/ops/dangerous-ops.md`)
- 認証情報(botid・gh・wrangler・opencode の設定、シェル履歴、環境変数の列挙)は読まない・書かない・表示しない。別 identity に切り替えない。依頼者の設定ファイルを編集しない。
- 履歴の書き換え、リポジトリ・Secrets・公開範囲・org 権限の変更は実行しない(依頼者の権限)。台帳に記録して続行。
- `rm -rf` は自分が作った物だけ。実行前に `ls -la <対象>`。
- 期限付きの予告(「3 分後に〜を削除」)は、1 分以内に worker を起動し、取り消しやすい最小の行動を期限内に実行して 1 行で報告する。何もしない案は選ばない。
- コミット・ファイル・ログに秘密・禁止文字列・Cloudflare の識別子(アカウント ID・ホスト名)を書かない。

## 9 完了の宣言(`docs/ops/honesty.md`)
- 宣言できるのは、(1) 実エンジンが実行された出力、(2) P1〜P3 の実測値(この機械・headless Chrome。依頼者機の値は別枠で、0 件でも併記)、(3) reviewer の検査結果、を報告に添えたときだけ。
- 揃わない間は「完了」「達成」と書かない。goal の判定が完了に傾く前に reviewer を起動する。

## 10 ブロッカー(`docs/ops/authority.md`、`docs/BLOCKERS.md`)
- 先に自分でできるか確かめる(`docs/ops/environment.md`)。できる物を「できない」と書くのは違反。
- 人間限定と実測したら、台帳に 1 行(状態・確認コマンド・確認日・影響)を worker に追記させ、台帳に依存しない作業を同じターンで起動する。聞かない・止まらない。`scripts/preflight.sh` が通った行は消す。

## 11 規則を守る仕組み(強制 > 検査 > 文書。文書だけの規則は前任がすべて破った型)
| 規則 | 仕組み | 種別 |
|---|---|---|
| main は edit/write・任意の shell ができない。起動できるのは worker/reviewer だけ | build agent の permission | 強制 |
| モデル固定(main Sonnet、worker/reviewer Haiku)。`model` 引数の拒否 | opencode.jsonc・plugin | 強制 |
| worker/reviewer の指示に `<!-- worker-rules v1 -->` | plugin(拒否または自動付与) | 強制 |
| sleep・timeout・nohup・disown・systemd-run・末尾 `&` | permission・policies | 強制 |
| 秘密・禁止文字列・95 MB 超・旧ツリーのパス | `scripts/scan-forbidden.sh`(pre-commit・pre-push) | 強制 |
| agent・push・公開範囲・wrangler・goal・ディスクの状態 | `scripts/preflight.sh`(各サイクルの前) | 検査 |
| モデル切替・main の直接作業・60 分超の無活動 | `scripts/audit-session.sh` | 検査 |
| goal の停止 | goal-nudge(1 分ごと)+ §2 | 強制 + 検査 |
| 禁止文・誠実性・報告形式・辞書 | reviewer の検査 + この文書 | 文書のみ(破られやすい) |
