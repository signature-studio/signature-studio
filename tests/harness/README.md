# tests/harness: ハーネスのテスト(S2)

`bash tests/harness/run.sh` で全部走る。依存は bash・jq・sqlite3・python3・shellcheck。node と opencode があれば、その分も走る(無ければ SKIP と表示される。SKIP は合格ではない)。
bats は使わない。テストは `$TMPDIR/opencode/s2-sandbox/tests/` の中だけで動く。実リポジトリで git コマンドを実行しない。実物の `gh`・`wrangler`・`opencode api` を呼ばない。セッションを作らない。モデルを呼ばない。

## 何を確かめるか(ファイル別)

| ファイル | 対象 | 確かめ方の段階 |
|---|---|---|
| `t01_config.sh` | `opencode.jsonc` が JSONC として正しい。**実物の opencode が**解決した agent 一覧とモデルが期待値と一致する。引用符 1 文字を消すと設定が黙って落ちる事実(fail-open)と、`preflight.sh` の `agents` 行がそれを FAIL にすること。`* deny` を消した場合も FAIL | **E1**(隔離した実物の opencode の `debug agents`) |
| `t02_permissions.sh` | 377 個の shell コマンド(`shell-cases.src`)と 79 個の非 shell 呼び出しの許可/拒否。wrangler・cf の全面 deny(`wrangler …`・`cf …`・`npx`/`bunx`/`pnpm`/`yarn` 経由・`*/wrangler`・`sudo wrangler`)と、文面に語が出るだけの読み取り(`grep wrangler …`)・`npm run test:worker` の許可。許可できてはいけない物(main の書き込み、リダイレクト、`preflight.sh` に引数)と、許可されるべき物(worker の通常作業)の両方 | **E2**(opencode 自身のスキャナでコマンドを分割し、評価は `lib/rules.py` による再実装。`t01` が「`rules.py` の規則 = 実物が読んだ規則」を確かめる) |
| `t03_plugins.sh` | 3 本の plugin(mock の ctx)。`worker-rules.ts` の自動付与・読めないときの拒否、`model-pin-gate.ts` のピン(設定から読む。`haiku` プロファイルでも追従)、`harness-guard.ts` の `model` 引数・末尾 `&`・接頭辞つきの禁止コマンド。各 40 行以内・状態なしを静的に確認。**実物の loader が 3 本を読み込む**こと | **E3**(mock)+ E1(loader) |
| `t04_scan.sh` | `scan-forbidden.sh`: 全規則が発火、近い形は発火しない、出力に値を出さない、許可リスト、終了コード、明示パス、未追跡ファイル | 実行 |
| `t05_hooks.sh` | `pre-commit`(ステージされた内容を検査。ワークツリーでなく)、`pre-push`(スキャンだけ。PUBLIC のリモートでも、スキャンが通れば通す。禁止文字列があれば拒否。**環境変数での回避が無い**ことを 6 種で確認。`gh` を呼ばないことを、呼ぶと失敗する偽物で確かめる。`--no-verify` は人間の道として通る)、`setup-repo.sh` の冪等性 | 実行(ローカルの bare repo と偽の `gh`) |
| `t06_audit.sh` | `audit-session.sh`: **前任のセッションに対する出力が前回の実測と完全一致**(`expected/audit-prev-session.txt`)。合成 DB で固定外モデル・禁止形のコマンド・DB を書かないこと。`--since "YYYY-MM-DD HH:MM"`(JST)で子を作成時刻で絞る(前後で分けて数える) | 実行(実 DB は読み取りのみ) |
| `t07_preflight.sh` | `preflight.sh` の各行(OK/WARN/FAIL)、**403 は WARN で FAIL にしない**(案内は作業ディレクトリ・`gh auth status`・`docs/ops/requests.md`)、`repo-api` は既定ブランチだけを取り、公開範囲・permissions を出さない、**wrangler を呼ばない**(呼べば記録して失敗する罠の偽物で確かめる)、トークン・アカウント ID・メールを出さない、`~/.config/opencode/` の他のファイルを読まない | 実行(偽の `gh`・`wrangler` の罠・`git push`・`opencode`・`df`) |
| `t08_scripts.sh` | `set-model-profile.sh`(split⇄haiku の往復がバイト単位で元に戻る。コメント維持。実物の resolver の答えが合わなければ元に戻す。セッション内では書かない)、`launch.sh`(偽の opencode だけ。順序・モデルが `opencode.jsonc` と同じ・登録簿の重複なし・失敗時に何が残るか)、`env.sh` | 実行(偽の opencode、`HOME` を差し替え) |
| `t10_consistency.sh` | `scripts/check-consistency.sh` を合成の文書(偽の木)で確かめる。**検出する**: 存在しないパス(バッククォート・リンク)、`docs/index.md` に載っていない `docs/core`・`docs/ops` の文書、README の表に載っていないテスト、どの文書にも載っていない `scripts/`、150 行の AGENTS.md。**検出しない**: 旧資料(`docs/archive`・`lift/`)のパス、他の場所を名指す行、作成予定の出力、コマンドの引数、裸の名前、ROADMAP の todo 行 | 実行 |
| `t09_provider.sh` | `set-model-profile.sh --provider gateway\|bedrock`(モデルの設定を切り替える。文字列は部品から組む)と、`preflight.sh` の agents 行(ローカルの上書きつき)。偽の opencode で確かめる | 実行 |

`shell-cases.tsv` は `shell-cases.src` から `lib/gen-shell-cases.ts` が作る(bun と opencode のソースの clone が要る)。テストの実行時に clone があれば、最新かどうかも確かめる。

## fail-open の検知(R3 §B.1)

`opencode.jsonc` の 1 文字の記述ミスで、opencode は文書全体を黙って却下し、全 agent が組み込みに戻る(deny 0 本・build のモデル未設定)。警告は 1 行で、tool は止まらない。
対策: `scripts/preflight.sh` の `agents` 行が、`opencode debug agents`(リポジトリ直下で実行)を `opencode.jsonc` と突き合わせる(agent の ID 一覧、build・worker・reviewer のモデル、build の `* deny`、worker の `sleep *` deny、reviewer の edit deny)。`t01` が「正常な設定 → OK」「引用符を 1 つ消した設定 → FAIL、終了コード 1」「`* deny` を消した設定 → FAIL」を実物の resolver で確かめる。

## 限界(防げない物。README を真に保つため `shell-cases.src` の `LIMIT-allow` が見張る)

規則を足して LIMIT-allow の形が止まるようになったら、`t02` が FAIL する(この表と `shell-cases.src` を更新する合図)。

| 防げない物 | 理由 | 代わりの手段 |
|---|---|---|
| `env sleep`・`FOO=1 sleep`・`command sleep`・`xargs sleep` | permission の glob は先頭語しか見ない | `harness-guard.ts` が接頭辞つきを拒否(`;` `&&` `\|\|` `\|` 改行で分けた各片の先頭)。`t03` |
| `bash -c "sleep 1"`・`eval`・`python3 -c "…time.sleep…"` | 別のプログラムの内部 | 監査(`audit-session.sh` の `sleep_bypass`) |
| worker の `rm -rf *`・`rm -rf ./*`・`rm -rf <リポジトリ外の任意のディレクトリ>` | glob の `*` は `/` と空白も含む。`rm -rf /*` と書くと全ての絶対パスに一致する(前任の約 11 万件で、正当な掃除 750 件を拒否した)ので、**危険な対象を名指し**した: `/`、`/?`、`~`、`~/*`、`$HOME`、`~/work`、`/home/*/work`、`*/work/signature-studio`、`*/signature-studio-?*`、`.`、`..`、`.git`。`rm -rf *`(リテラルの `*`)は、1 文字のファイル名(`rm -f a b`)と区別できず、前任の正当なコマンド 64 件を巻き込むので規則にしていない | reviewer の検査、git の差分 |
| main の許可リストの `git -C <dir> <動詞>` | `*` が空白を含むので、`git -C x commit -m status` は `git -C * status *` に一致する | 書き込む動詞を列挙して拒否(`git -C * commit *` など)。**列挙に無い動詞 + 後ろに status/diff/log という語**は通る |
| worker が shell で `opencode.jsonc`・`.opencode/`・`.githooks/` を書き換える | edit の deny は shell に効かない | git の差分(`audit`・reviewer) |
| shell 経由のファイル読み取り | パスの規則は shell に効かない(文書: best effort)。worker の shell には、`~/.config/opencode`・`opencode.db`・`~/.local/share/opencode/auth*`・`~/.config/botid`・`~/.local/share/botid`・`~/.config/gh/`・`.bash_history` の文字列を含むコマンドの拒否を入れた | **コマンド本文がその文字列を含むだけで拒否される**(コミットメッセージ・ヒアドキュメントも)。そういう本文は edit ツールで書く |
| `gh api` の書き込み | `-X`・`--method`・`-f`・`-F`・`--field`・`--raw-field`・`--input` を拒否 | `gh api` を別のプログラム経由で呼ぶ形は防げない |
| worker の `sh -c "wrangler …"`・`bash -c "wrangler …"`・`env wrangler …`・`python3 -c` の中の wrangler(wrangler・cf の全面 deny の限界) | 先頭語の規則は、別のプログラムに渡された文字列の中の wrangler を見ない | 監査(`audit-session.sh`)と reviewer。依頼者の決定 2026-11-10 |
| worker の `export -p`・`declare -x`(環境変数の列挙) | opencode のスキャナが、これらの形から command part を返さない(`no command part`)ため、規則が照合されない。規則は `opencode.jsonc` に書いてあるが効かない(S10 で再現) | 監査(`audit-session.sh`)と reviewer。`env`・`printenv` は規則で止まる |

| 人間による TUI からのモデル切替 | 切替を禁じる設定が無い | `model-pin-gate.ts` が **tool 呼び出し**を止める。tool を使わない応答は止まらない。`audit-session.sh` が検知 |
| メインが `oc-msg.sh send` の本文に `>` を含める | main の許可は `oc-msg.sh *`(許可)の後に `*>*`(拒否)を置いた。そうしないと `oc-msg.sh send … > file` で任意のファイルを上書きできる。コマンド本文の `>` は区別できないので、本文に `>` を含む送信は拒否される | 本文に `>` を書かない(`to` か文字の矢印)。ヒアドキュメントの本文も同じ |
| main の `git diff/log/show` が、worker がリポジトリの設定に書いた外部コマンド(`diff.external`・textconv・pager)を実行しうる | git の読み取りの許可は、リポジトリ内の設定を尊重する。worker は `git config diff.external …` を拒否されていない | 未対策。reviewer が `git config --local -l` の差分を見る。**未検証の推測** |
| plugin が実セッションで効くか | 実セッションを作っていない | 下の Canary |

## Canary(親が行う。未検証の物を実セッションで確かめる)

**ここまでのテストは、実セッションを一度も作っていない。** 次の 4 点は mock と読み取りだけの確認で、実環境で効くかは未確認。親が使い捨てのセッション(Haiku で 1〜3 回の呼び出し)で確かめる。モデルを呼ぶので費用が出る(Haiku 5.5 の数回分)。

1. **plugin が読み込まれ、`execute.before` が呼ばれる**
2. **`model-pin-gate.ts` の `ctx.agent.get` と `ctx.session.get` が実セッションで `model` を返す**(隔離サーバーでは、`session.create` で作ったセッションの `get` が `model` を返し、`agent.get` が `{location, data}` を返すことを確認した。tool の `execute.before` の中からの呼び出しは未確認)
3. **`worker-rules.ts` が `subagent` の入力を書き換えて、worker が先頭のマーカーつきの指示を受け取る**(`execute.before` で `event.input` を差し替える方式。ソースでは `tool.ts` が `event.input` を実行に渡す。実行は未確認)
4. **`build` が edit/write/任意の shell を拒否され、worker だけが起動できる**(permission の実際の経路)

手順(`~/work/signature-studio` で、親が実行する)。**メッセージは `session.synthetic`(`oc-msg.sh send`)で送る**。`session.prompt` は使わない(人間の起動 `scripts/launch.sh` だけの物)。`oc-msg.sh send` は `resume: true` で実行を起こす。

```bash
cd ~/work/signature-studio
bash scripts/preflight.sh                       # agents 行が OK であること
# 使い捨てのメインセッション(agent build、model は opencode.jsonc の build と同じ。title に canary と入れる)
MODEL=$(python3 -c 'import re; print(re.search(r"^      \"model\": \"([^\"#]*)", open("opencode.jsonc").read(), re.M).group(1))')
CANARY=$(opencode api session.create --data "{\"agent\":\"build\",\"model\":{\"providerID\":\"${MODEL%%/*}\",\"id\":\"${MODEL#*/}\"},\"location\":{\"directory\":\"$PWD\"},\"title\":\"canary (delete me)\"}" | jq -r .data.id)
echo "$CANARY"

# (4) と (3): build に、わざと禁止された操作と、worker の起動を頼む(Sonnet を 1〜2 回呼ぶ)
~/.agents/skills/opencode-send-message/scripts/oc-msg.sh send "$CANARY" - <<'EOF'
Do exactly these two things and report the raw results, nothing else.
1) Try to create the file /tmp/opencode/canary-build.txt with the write tool (or say that you have no such tool).
2) Start a worker (subagent, agent=worker) with the prompt: Run git status --short and reply with the first line of your own instructions verbatim.
EOF
```

期待される結果と確かめ方(`~/.agents/skills/opencode-send-message/scripts/oc-msg.sh log "$CANARY" 20`、子は `opencode api session.message.list --param sessionID=<子の ID>`、`scripts/audit-session.sh "$CANARY"`)。**下の期待はソースからの予測で、実測していない**:

| # | 期待 |
|---|---|
| 4 | build は `write` を実行できない。**2 通りのどちらでもよい**: (a) `write` が tool 一覧に無い(`No tool named "write"`)、(b) `error.type = permission.rejected`(`Permission denied: write`)。どちらでも `/tmp/opencode/canary-build.txt` は存在しない。`audit-session.sh` の「3b. main: direct work total」が **0** |
| 3 | worker の最初のメッセージ(子セッションの `session.message.list` の最初の user)が、`You are a subagent spawned by another session.` の次の行で `<!-- worker-rules v1 -->` で始まる |
| 1 | 3 が成り立てば、plugin は読み込まれ、`execute.before` が呼ばれている |
| 2 | 子の tool 呼び出し(`git status`)が `Rejected: session model …` で**拒否されず**、worker が実行して報告する(ピンと一致している) |
| モデル | `audit-session.sh "$CANARY"` の「2.」で、child が Haiku、main が Sonnet だけ。「5.」(固定外の行)が **0 行** |

`model-pin-gate.ts` が**拒否する**側の確認(切替を起こす。**使い捨てセッションに対してだけ**):

```bash
opencode api session.switchModel --param sessionID="$CANARY" --data "{\"model\":{\"providerID\":\"${MODEL%%/*}\",\"id\":\"claude-haiku-5-5\"}}"
~/.agents/skills/opencode-send-message/scripts/oc-msg.sh send "$CANARY" "Read the file AGENTS.md with the read tool and say its first line."
```

期待: build の `read` が `Rejected: session model <provider>/claude-haiku-5-5 is not the model <provider>/claude-sonnet-5-5 pinned for agent build. Tool calls stay blocked until the session's model is switched back.` で失敗する。応答は tool なしで返る(モデルの応答自体は止まらない)。
**終わったら canary を削除する**: `opencode api session.remove --param sessionID="$CANARY"`(親の shell から)。費用は Sonnet 数回 + Haiku 数回。

結果が期待と違う場合:
- 3 が成り立たない(マーカーが付かない): `worker-rules.ts` の書き換えが効いていない。S1 の文書は「先頭に貼る(plugin が付けるなら不要)」と書いてあるので、**メインが貼る運用に戻せば成立する**。plugin を外す必要は無い(付かないだけで害は無い)。
- 2 が成り立たない(`Rejected: agent … has no model` や `session.get` の例外): `model-pin-gate.ts` は `build`・`worker`・`reviewer` の**全 tool 呼び出しを拒否する**(fail closed)。**直ちに退避して復旧**する: `.opencode/plugins/model-pin-gate.ts` を `.opencode/model-pin-gate.ts.off` に移し、`opencode reload`。原因を調べてから戻す。
- 4 が成り立たない(`write` が実行できてしまう): **最重要**。build の `* deny` が効いていない。`scripts/preflight.sh` の `agents` 行と、`opencode debug agents` の build の規則を確認する。

## 実環境の確認(読み取りのみ)

`opencode debug agents`(リポジトリ直下、実サービス)の結果は、実環境のグローバル設定の影響を受ける。`~/.config/opencode/opencode.jsonc` は R3 の時点で不正な JSONC として却下されている。グローバル設定は、このハーネスの作者(S2)は読まない・変えない・scripts に読ませない。結果が期待と違う場合は、`scripts/preflight.sh` の `agents` 行が FAIL になる。**設定を迂回しない**。
