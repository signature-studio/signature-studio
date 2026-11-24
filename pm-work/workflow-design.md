# Issues 駆動の作業フロー(設計)

状態: 案。この文書は `.github/workflows/agent.yml`・`tools/select-targets.mjs`・`tools/task-accept.mjs` の設計の正本。
PM が決めた事項は「10」にまとめた。

## 1. 前提

- 待ち行列・閾値・生成器は無い。作業の単位は GitHub の Issue 1 件。
- タスクの全情報は Issue の本文に書く。受け入れコマンドは、本文の ` ```accept ` のコードブロック(1 つだけ。中身は 1 行のコマンド)。
- マイルストーンは GitHub Milestones。親子は sub-issue。依存は「ブロックされる側」の Issue に `blocked by`(blocked-by)として付け、ブロックする側は `blocking` として見える。
- 起動の判定は、Issue の状態・ラベル・依存関係を GitHub から読んで行う(ファイルの待ち行列は使わない)。

## 2. トリガー

| on | types | 対象にする Issue |
| --- | --- | --- |
| `issues` | `opened`・`edited`・`reopened`・`labeled`・`unlabeled` | そのイベントの Issue 1 件 |
| `issues` | `typed`・`untyped` | そのイベントの Issue 1 件。Type が付いた・外れたときに、起動の可否を「5」で再評価する |
| `issues` | `closed` | 閉じた Issue が `blocking` している Issue の全て(ブロッカーの解除を、ブロックされる側の起動に変える)。通った Issue は push の commit(`Closes #N`)で閉じるため、この事象は App の操作として起きる想定(未確認。「9」) |
| `issue_comment` | `created` | そのイベントの Issue 1 件。作者が Bot のもの、PR のコメントは対象外。本文が `/retry` で始まるなら再試行 |

置かないもの:

- `workflow_dispatch`: 依頼者の指示「検知する event があれば workflow_dispatch は許容しない」に従う。ブロッカーの解除は `issues: closed` で検知できるので、代わりに使う event が無い。
- `schedule`・`push`: 使わない。
- `pull_request`: PR を使う処理が無いので置かない(「10」の 2)。
- `blocked_by_added`・`blocked_by_removed`: Actions の `on:` には無い。下の「9」を見る。

## 3. run の流れ

```
select ──(対象が空なら終わり)──> start ──> extract ──> work(Issue ごとに並列) ──> report
```

| job | needs | if | 権限 | 使う token | 役目 |
| --- | --- | --- | --- | --- | --- |
| select | なし | なし | `contents: read`・`issues: read` | GITHUB_TOKEN(読むだけ) | イベントから候補を集め、「5」の条件で対象を決める(`tools/select-targets.mjs select`) |
| start | select | 対象が空でない | `contents: read`・`issues: write` | GITHUB_TOKEN(App の鍵は渡さない。例外: Project の In Progress の段だけ App の鍵。「17」) | 対象を同じ条件でもう一度確かめ、`running` を付ける。`/retry` なら `failed` を外す(`claim`)。起動した Issue の Project の Status を In Progress に移す(「17」) |
| extract | start | 起動された対象が空でない | 既定 | GITHUB_TOKEN なし(R2 の鍵のみ) | 抽出物を R2 のキャッシュから用意する(今の段をそのまま) |
| work | start・extract | 起動された対象が空でない | `contents: read`・`issues: read` | GITHUB_TOKEN は、本文の取得と判定の道具の取り直しの段だけ。push の段だけ App の token(「8」)。作業者の段には無い | Issue ごとに: 本文と題名を取る → 受け入れコマンドを取り出す → 作業者を動かす → 判定 → 通れば push(空の commit で `Closes #N` を付ける) |
| report | start・work | 常に(`always()`)。ただし start が成功し、起動された対象が空でない | `contents: read`・`actions: read`・`issues: write` | GITHUB_TOKEN(読みも書きも。App の鍵は渡さない。例外: Project の In Review の段と、再試行の起動の段。「17」「16」) | Issue ごとに結果をコメントし、状態を変える。Project の Status を In Review に移す(「17」) |

- `if:` を書いたジョブは、既定の `success()` の判定が外れるので、`start`・`extract`・`work` の `if` には `success() &&` を付ける(上流が落ちたときに、空の対象で後続が走らないため)。
- work は `strategy.matrix.issue` で Issue の番号ごとに 1 ジョブ。`fail-fast: false` で、1 件の失敗が他を止めない。
- work の `concurrency` は `group: task-<番号>`、`cancel-in-progress: false`。同じ Issue の作業は直列になる。
- 対象が空なら start 以降は skip される。runner を使うのは select(数十秒)だけ。
- 抽出物の接頭辞の一覧は、リポジトリのファイル(`ci/resources-prefixes.txt`・`ci/plugin-builtin-prefixes.txt`。1 行 1 接頭辞)に置き、extract の取り出しが読み、extract と work の鍵の hashFiles に入れる。一覧を変えると鍵が変わり、接頭辞の足りない古いキャッシュは使われない(fl-exe の 2 つの完全なパスは、鍵の文字列 `FL64.exe-FL64-scaled.exe` に入れる)。新しい seed の行を足す Issue は、実物の接頭辞が `ci/resources-prefixes.txt` に在ることを、起票の前に確かめる(無ければ、先に足す。足す前に、作業者は受け入れを通せない)。
  - #1386・#1387 で足した 6 件(`ci/resources-prefixes.txt` の 9〜14 行目。どの接頭辞も `\` で終わる folder 単位。ファイル単位の指定は無い)。大きさは、一覧の bytes(dll・exe と Data\ の物だけ在る)の合計。
    - `System\Tools\Chord progression\`: 59 件。大きさは一覧に無い。FL 内蔵のコード進行ツールの Python のスクリプト(#1386)。
    - `Shared\Python\`: 37 件。dll・exe の合計は 14,939,080 B。他の 27 件は大きさが一覧に無い。FL 内蔵の Python の実行環境(`Python\Python312.dll` を読む物。#1386)。接頭辞はこの folder 全体になる。
    - `Data\Patches\Scores\Arpeggiator\Riff machine\`: 40 件・8,093 B(#1387)。
    - `Data\Patches\Scores\Quantization\Riff machine\`: 9 件・3,908 B(#1387)。
    - `Data\Patches\Scores\Chopping\Riff machine step 1\`: 24 件・6,311 B(#1387)。
    - `Data\Patches\Scores\Chopping\Riff machine step 2\`: 19 件・2,807 B(#1387)。
  - 接頭辞で取った物は、作業ツリーの `lift/out-app/host-extra/resources/` に入るだけ。エンジンの VFS に載るのは、起動の seed 表(`lift/harness/seeds/vfs-seed-idle1.tsv`)か、pack の lazy 指定に行が在る物だけ。接頭辞を足した Issue は、その行も足す。
- 抽出の表は、`ci/extract-dlls.tsv` と `lift/plugins/*/plugin.tsv` の行の和で、後者は `app/scripts/extract-fl.mjs` が `app/scripts/lib/plugin-rows.mjs` で読んで足す。抽出の鍵(agent.yml の extract-tree-・deploy.yml の host-)は、その plugin.tsv を入力に含む。

- 実機のビーコン(ラベル `perf-beacons` の作業だけ)。extract が R2 の `fl-web-cache` の `perf/` から、直近 3 日(UTC の今日と前の 2 日)の `*.json` を、最大 1000 件・合計 100 MB で取り、`$RUNNER_TEMP/perf` に置いて、鍵 `perf-beacons-${{ github.run_id }}` でキャッシュに保存する。work は同じ鍵で復元し、`lift/out-app/host-extra/perf/` に置く(commit はしない)。鍵を run ごとにするのは、ビーコンが増え続けて内容で決まる鍵が使えず、同じ run の extract と work だけが同じ鍵を読むため。公開の artifact は使わない。リポジトリは公開で、artifact も公開されるため、依頼者の実機のビーコン(個人を特定しうる値を含む)を置けない。R2 の鍵は extract だけが持ち、作業者の段には渡さない。ラベルの無い run では、これらの段は動かない。

## 4. 作業の段の中身

1. 本文と題名を取る: `gh api repos/<repo>/issues/<番号>` の結果を `$RUNNER_TEMP/issue.json` に置き、本文を `task.md`、題名を `title.txt` に取り出す。
2. 受け入れコマンドを取り出す: `node tools/task-accept.mjs < task.md > task-accept.md`。` ```accept ` ブロックが 1 つで中身が 1 行のときだけ成功。無ければ、理由を `out/judge.txt` に残して失敗する(report が Issue にコメントする)。
3. 作業者のプロンプト: `AGENTS.md` + `---` + 「## タスク(Issue #番号)」+ 本文。本文以外の情報は渡さない。
4. 作業者を 1 回動かす(opencode の導入・認証・起動は今の段のまま)。作業者の段には token が無い。
5. 判定: `guard/judge.sh`(取り直した main の物)に `task-accept.md` を渡す。
6. 判定に通ったときだけ push する。push の段は、題名と `(Closes #番号)` を付けた空の commit(`git commit --allow-empty`)を作ってから、pull → push する(衝突時は中断して再試行)。push の token は App のもの(「8」)。push できた SHA を `out/commit.txt` に残す。
7. 成果物 `agent-<番号>`(`out/`: 報告・判定の出力・SHA)を 7 日保存する。
8. 押せなかった作業を保存する(push の段が 3 回とも落ちたときだけ。判定が落ちたときは、次の「判定が落ちた作業を保存する」が保存する): 基点は、push の前に残した HEAD(`RUNNER_TEMP/work-tip.txt`)と `refs/judge/main` の merge-base。`git diff --binary <基点> <push 前の HEAD>` を `out/work.diff` にし、7 の artifact に入れる。pull で取り込んだ main の変更は含めない(含めると、再試行で他人の変更を再適用し、後から戻された変更を巻き戻す)。差分が空なら作らず、50 MB を超えるなら作らない(警告)。
- 再試行の作業の前に「前回の作業を取り込む」段がある: 同じ番号の artifact のうち、期限内で `work.diff` を持つ最新を取り、`git apply --3way --index` で載せる。きれいに載れば commit し、作業ツリーのルートに `.prior-work.md` を書く(「衝突なく」)。衝突すれば、衝突の印を残し、`.prior-work.md` に衝突したファイルを書く(commit はしない。作業者が解消して commit する)。`.prior-work.md` は `.gitignore` の対象で、commit しない。作業者は `AGENTS.md` の指示で、これを最初に読む。取り込みは、キャッシュの復元(runs-on/cache/restore)と取った物の配置の後、タスクの段の前に置く(鍵の hashFiles は取り込み前の main のファイルで計算され、取り込んだ作業が鍵の入力を変えてもキャッシュは外れない)。そのため、取り込んだ作業が `ci/versions.env`・`app/package*.json`・`lift/Cargo.*` を変えていても、この run の版の読み込みとビルドは取り込み前の版で行われ、作業者が必要なら自分で再ビルドする。
- 保存の基点を「push の前の HEAD」にするのは、push の段の pull が、成功すると HEAD に main の merge を足すため。merge の後の HEAD を基点にすると、差分に main の変更が混ざる。
- 判定が落ちた作業の保存と続きからの取り込み: 判定の段が落ちて push の段が飛んだときは、作業者が動いた後(`out/report.md` が在る)に限り、作業ツリーの全差分(commit 済み・未 commit・未追跡。`.gitignore` の物を除く。`git add -A` の後の `git diff --cached --binary <基点>`)を、基点(`refs/judge/main` との merge-base)からの差分として `out/work.diff` に残す(50 MB を超えるなら作らない)。判定の出力は `out/judge.txt` として既に artifact に在るので、別に残さない。次の再試行は、`out/judge.txt` に `judge: 通った` の行が無い作業(判定を通らなかった作業)を、commit せずに作業ツリーに載せ、`.prior-work.md` に判定の理由(末尾 10 行。長い行は 200 字で切る)と、続きから行う旨を書く。作業者は、残りを行い、受け入れを通して commit する。判定を通った作業(push だけ落ちた)は、これまでどおり commit してから始める。作業者は、ターンを終えない(バックグラウンドのコマンドの完了の通知は、ヘッドレスの実行では届かない)。長いコマンドは、フォアグラウンドで行い、シェルツールの `timeout` 引数に十分な値を渡す(`AGENTS.md` の「作業者の手順」の 2 行)。

## 5. 起動してよい条件

Issue N が対象になるのは、次をすべて満たすとき(select の判定。start は同じ判定をもう一度行う):

- 状態が `open`。PR ではない。
- Issue の Type が Task。
- ラベル `pm-hold`・`running`・`needs-owner` が無い。
- sub-issue を持たない(Type が Feature の親は起動しない)。
- `blocked by` の Issue のうち、`open` のものが 0 件。
- ラベル `failed` が無い。ただし、本文が `/retry` で始まる `issue_comment` のときだけ、`failed` は無視される(start が `failed` を外す)。

判定の理由は select の標準エラーに 1 件ずつ出す(「起動しなかった理由」を Actions のログで見られる)。

## 6. ラベル(承認後に PM が作る。エージェントは作らない)

| ラベル | 意味 | 付ける人・物 |
| --- | --- | --- |
| `pm-hold` | 依存関係を付けている途中。起動しない | PM |
| `running` | 作業中 | start が付け、report が外す |
| `failed` | 落ちた。再試行は `/retry` か、このラベルを外す | report が付ける |
| `needs-owner` | 依頼者の判断待ち。起動しない(駆動の判定には残す。PM は使わない。「10」の 3) | 人が付ける |

旧ラベル `ready` は使わない。

## 7. 失敗・再試行・取り消し

- 通った: push の段が作った `Closes #N` の空の commit で、GitHub が Issue を閉じる(未確認。「9」)。report は「通った: <run の URL>」「commit: <SHA の URL>」「受け入れの出力(末尾 8 行)」をコメントする。Issue がまだ open のときは、「close を commit のキーワードで行えなかったので、後続の Issue は起動しない。PM が再評価する」を追記し、GITHUB_TOKEN で閉じる。そのあと `running` を外す(開いたまま外すと、同じ Issue への別の run が、`running` の無い状態を見て再起動し得るため)。
- 落ちた: 「落ちた: <run の URL>」「作業の状態」「失敗した段」「判定の出力(末尾 8 行)」をコメントし、`failed` を付けてから `running` を外す(逆順だと、外した直後の別の run が、`failed` の無い状態を見て再起動する)。
- 取り消された(cancelled)・ジョブが動かなかった: 落ちたのと同じく `failed` を付ける。時間切れで取り消された作業は、取り消しの後の保存の段が `work.diff` に残し、コメントに「時間切れで取り消された」と「作業を保存した」(または「作業を保存できなかった」)を書く(「13」)。
- push の段で落ちた(判定は通った): 作業は「4」の 8 の `work.diff` として artifact に残る。落ちた旨のコメントに「作業を保存した」を書く。次の再試行は、その差分を作業の前に取り込んでから始める。main との衝突で落ちたときは、「16」の自動の再試行の対象になる(拒否で落ちたときは対象外)。
- 判定の段で落ちた(判定は通らなかった): 「4」の「判定が落ちた作業の保存と続きからの取り込み」のとおり、作業ツリーの差分を `work.diff` に残し、コメントに「作業を保存した」を書く。次の再試行は、それを commit せず続きから取り込む。
- 再試行: Issue に `/retry` と書く、または `failed` ラベルを外す。`/retry` のときは start が `running` を付けてから `failed` を外す(逆順だと、外した直後の別の run が、`running` の無い状態を見て二重に起動し得る)。

## 8. トークン

- ラベル(`running`・`failed`)とコメントは、start と report が GITHUB_TOKEN(`issues: write`)で行う。GITHUB_TOKEN で行った変更は次の run を起動しない(`workflow_dispatch`・`repository_dispatch` を除く)。そのため、start と report の書き込みで run が連鎖しない。
- 通った Issue は、push の段が作る空の commit(`Closes #N`)で閉じる。push は App の token で行うので、この close は App の操作になり、`issues: closed` の run を起こすと想定する(未確認。「9」)。report は、Issue がまだ open のときだけ GITHUB_TOKEN で閉じる。GITHUB_TOKEN の close は run を起こさないため、後続は起動せず、PM が再評価する。
- App の鍵(`GH_APP_CLIENT_ID`・`GH_APP_PEM`、変数 `GH_APP_INSTALLATION_ID`)は、work ジョブの「名義を設定する」段と push の段にだけ渡す。select・extract には渡さない。例外: report の自動の再試行の起動の段(「自動の再試行を起動する」)だけは、再試行の Issue があるときに限り、鍵を受け取る(running を外す App の操作の 1 回。「16」)。同じく、Project の Status を動かす段(start の In Progress・report の In Review。「17」)だけは、鍵を受け取る。
- 作業者の段(opencode)には、どの token も鍵も無い(今と同じ)。
- 本文の取得と、判定の道具の取り直しの段にだけ、GITHUB_TOKEN(`issues: read`・`contents: read`)を渡す。

## 9. 既知の限界

- `blocked_by_added` は Actions の event に無い。依存を後から付けた Issue は、付ける前に起動していれば止まらない。PM は `pm-hold` を付けて作り、依存を付けてから外す(`unlabeled` で検知する)。
- `blocked_by_removed` も無い。依存を手で外しただけでは起動しない。Issue にコメントするか、本文を `edited` にすると、再評価される。
- 保存と取り込みは、衝突の損失を減らすだけで、衝突を無くさない。衝突は作業者が解消する(自動では解消しない)。ジョブが取り消された(cancel・時間切れ)ときは、保存の段が `(failure() || cancelled())` で動く。猶予(約 5 分)の中で差分が作れなければ保存されない(「13」)。
- 同じ Issue への run が 2 つほぼ同時に来ると、start の再確認の間に両方が通ることがある。残りの窓は数秒。work の `concurrency` で作業は直列になるが、0 にはならない。
- start が途中で落ちると、それまでに付いた `running` が残る。PM が外す。report は動かない(start が成功していないため)。
- 作業ジョブの鍵の一時ファイル(`$RUNNER_TEMP/botid-config.json`)は、作業の段から読める(旧設計から同じ)。
- **未確認**: `Closes #N` の空の commit で Issue が閉じるか。pull で merge commit ができたときも閉じるか。閉じなかった場合は、report が GITHUB_TOKEN で閉じ、後続は起動しない(「7」)。
- **未確認**: App の操作で閉じた Issue が `issues: closed` の run を起こし、ブロックされていた後続を select が選ぶか。起きなければ、後続の Issue の本文を編集して `edited` を起こし、再評価する(上の `blocked_by_removed` と同じ手順)。
- **未確認**: App の操作で外した `running`(`issues: unlabeled`)が run を起こし、select が Issue を選ぶか(「16」の再試行の起動)。起きなければ、再試行は起動せず、Issue に `running` が残る。PM が外す。

## 10. PM の決定

依頼者に判断を求めるのは、依頼者だけが決められる事項だけにする。それ以外は PM が決め、ここに書く。

1. **起動する数**: ブロッカーの無い task を常に 60 件以上起票して、60 の作業を動かし続ける。起票は PM が `issue-drafts/WAVES.md` の波で行う。閾値(`queue/threshold`)と生成器は作らない。60 の同時実行は GitHub の上限に任せる。
2. **`pull_request`**: 置かない。PR を使う処理が無いため。
3. **ラベル**: 運用は `pm-hold`・`running`・`failed` の 3 種。種別は Issue の Type(Task・Feature)、規模はプロジェクトの Size、優先度は組織の Issue field の Priority で指定する。PM は `needs-owner` を使わない(依頼者の判断待ちで起動を止める Issue は作らない)。表示用のラベルは PM が作る(`issue-drafts/LABELS.md`)。
4. **App の権限**: App の token で書くのは push の段だけ(鍵は名義の設定の段と push の段にだけ渡す)。start・report は GITHUB_TOKEN(`issues: write`)で行う。ワーカー用の App には Issues の書き込みの権限が無いため、ラベル・コメント・close をこの形にする。
5. **受け入れコマンドの出所**: 本文の `accept` から取る(今の設計のまま)。本文を編集できるのは PM と依頼者だけにする。
6. **依存の手動の変更**: 「9」の既知の限界として扱う。
7. **機械的な作業**: 単独の Issue にしない。(a) 関数の一覧に 1 行ずつ足す作業は、M7 の一括の task の中の手順として、エージェントがスクリプトを書いて行う。(b) プラグイン 1 件につき表に 4 行を足す作業は、M5 の束ねた task の中の手順として、同じく行う。スクリプトだけの専用の経路は作らない。
8. **測り方と定義**: P1 は、ページを開いてから最初の idle まで 10 秒以内。P2 は、120 fps と、入力から画面の p95 が 17 ms 以下。P3 は、Song Mode の再生で underrun が 0。定義の出典の文書 `docs/ops/fidelity-and-performance.md` は、リポジトリに無い。120 fps は、表示の上限を外す起動の引数で測る(作業者が選び、報告に書く)。
9. **起動時の同意ダイアログ**: 測定の道具の中で「No thanks」で閉じる。製品の既定の動作は変えない。
10. **P3 の入力**: インストーラに含まれる demo・テンプレートのプロジェクトで、Song Mode の playlist を持つ物を、作業者が一覧から選び、道具に記録する。依頼者の個人のファイルは使わない。
11. **外部プラグイン**: Serum 2・OTT・Vital の 3 件を全て対象にする。取得は FL のインストーラと同じ経路(extract の段)で、作業の段は鍵を持たない。
12. **「100 以上」の数え方**: DLL のファイル数で数える。
13. **VFX 系(OpenGL)**: 後回しの範囲は、023 の判定(import に opengl32 を含む物)で決める。039 は全群の後に自動で起動する。
14. **抽出の段の鍵**: 変えてよい。全件の抽出し直しは許容する。
15. **インストーラの一覧**: `inputs/fl-installer-files.tsv` に置き、作業の段が commit する。013 の閾値(Effects の DLL 85 以上、Generators の DLL 45 以上)は、実数が下回る場合に作業者が実数に合わせて直してよい(理由を報告に書く)。
16. **完成の判定**: underrun の数など実測の値だけで判定する。耳の確認は行わない。
17. **エンジンの大きさ**: engine wasm の大きさの目標は無い。単一の wasm ファイルの上限は、ブラウザが読み込める約 1GB。超える場合は分割する。起動前に取る量は、P1 の実測で決める。
18. **M7 の目標**: P3 の再生中の命令数のうち、直接 wasm の割合が 99% 以上。
19. **依存の変更**: 外部プラグインの 3 件(027・028・029)は、音声の経路の修正(040)の後に入れる。
20. **同じファイルを変える Issue は `blocked by` でつなぐ**(PM の起票の規則)。同じ生成物の入力(`lift/harness/web/pack.args`・`lift/harness/web/seed.args`・`lift/crates/fe-build/tests/plan/*.txt` など)を変える Issue が並行すると、push の段の pull で衝突し、作業が失われる(#1341 は #1319 と衝突して 3 回失敗した)。後から起票する側を、先に入る側の `blocked by` にする。

## 11. 依頼者の指示との対応

| 指示 | 設計 |
| --- | --- |
| Issues・PR の作成/更新、コメント作成を認める。workflow_dispatch は Issue 番号のみ、検知 event があれば不要 | `issues`・`issue_comment` の types を「2」の通り置く。`workflow_dispatch` は置かない(検知 event `issues: closed` がある) |
| Issue created で起動し、ブロッカーがあれば run は即座に終わる | opened の run は select だけ走り、対象が空なら以降は skip される |
| タスクの全情報を Issue に含める | 本文と ` ```accept ` ブロック。作業者には本文だけ渡す |
| `queue/`・`ci/tasks/`・`driver/` を削除し、マイルストーンは GitHub Milestones、タスクは Issues と sub-issues に | 削除は別の担当。この設計はこれらに依存しない |
| 依存は blocking/blocked-by で示し、ブロッカーの無いタスクは即座に起動 | `blocked by` の open が 0 の Issue を、起動の条件にする。ブロッカーが閉じた `closed` で後続を起動する(未確認。「9」) |
| PM 資料は `pm-work/` | この文書を含め、PM 資料は `pm-work/` に置く |
| Issues・Milestones・ラベル・PR への書き込みは承認まで禁止 | この文書は書き込みを行わない。ラベル・Issue は承認後に PM が作る |

## 12. 本番のデプロイ前のスモーク(deploy.yml)

`fe-build bundle` の後、`fe-build upload`・wrangler の前に、2 つの検査の段を置く。(1) `tools/boot-check/boot-bundle.mjs` で起動を確かめ、状態が `engine-idle`・`standIn` が偽・`error` が null のときだけ通す(boot-bundle の `error` が null でも、状態が idle とは限らないので、状態を別に見る)。(2) `tools/smoke/welcome-flow.mjs` で Welcome 窓の 4 つの閉じ方(「No thanks」→ Default、Escape、「No thanks」→ Default の 1 回クリック、「I'm in」→ Default)を、流れごとに別の起動で走らせ、4 件とも `error` が無く、trap せず、idle に戻り、プロジェクトが開き、窓が閉じたときだけ通す。判定は、deploy.yml の中の `jq -e` の段が行う(道具の終了コードは流れの成否を表さないため)。どちらかが落ちれば、後の upload・wrangler は飛び、本番は更新されない(既定の `success()`)。判定の式は `spec/workflow.test.sh` の (m) が、fixture で実際に動かして確かめる。`HostFilesUnsupported` の件数は、正常時の値が未確認のため判定に入れない。時間の上限は、起動の段 15 分、流れの段 30 分。Chrome は runner の画像の `google-chrome` を使う(作業ジョブの boot-check と同じ)。作業ジョブの判定(judge)には、まだ入れていない。判定には取り直した main の engine と bundle を作り直す段が要り、作業ジョブの時間が大きく増えるため、deploy のログで `fe-build engine` の所要時間を見てから決める。

## 13. 時間切れと待ちの固まり

ジョブの上限(work は 350 分)に達すると、ジョブは取り消され、`failure()` は動かず、`cancelled()` の段だけが猶予(約 5 分)の中で動く。#1342 の 2 回目の試行(run 45197326869)では、作業者が背景のビルドを `tail --pid` で待ち、その待ちが戻らず、ジョブが時間切れで取り消され、作業が全て失われた(作業の保存の段が動かなかった)。そこで、押せなかった作業と判定が落ちた作業の保存の条件を `(failure() || cancelled())` にし、保存の段は通信をせず、取り消しで残った `.git/index.lock` を外してから差分を作る。報告の段は、取り消された作業を `failed` にし、保存の有無をコメントに書く。次の再試行は、判定の出力が無い保存を、判定の前の取り消しとして続きから取り込む。

## 14. 本番の engine の門(廃止)

`fe-build upload` は、engine の sha256 が `lift/harness/web/verified.txt` の行に在るときだけ、本番の `manifest.json` を差し替えていた(`upload_gated_steps`)。門の行は人が足す約束だったが、誰も足さなかったため、門は静かに閉じたままだった。手順は落ちず、deploy は success のままだった。その結果、2026-11-22 18:53Z から 11-24 00:40Z まで約 3 日間、全ての engine の変更(HiDPI・音声の補充・メニューなど)が本番に出ず、本番は `b440e13a623a` の版のままだった(#1363 の実機のビーコンで確かめた)。940e08fd は、スモークが通った sha を deploy が門の行に足す暫定の形にしたが、門そのものが deploy の迂回なので、門を廃止した(#1378)。

今の形: `fe-build upload` は、常に bundle のファイルを上げ(`upload_sync_steps`)、最後に `manifest.json` を差し替える(`upload_manifest_step`)。順は変えない(ファイル → manifest)。main の deploy は、スモーク(起動の確認と Welcome 窓の流れ)が通った engine を、常に本番に出す。スモークが落ちれば deploy は失敗し、本番は替わらない(既定の `success()`)。upload の後の「本番の manifest を確かめる」は、本番の manifest とこの build の manifest を `cmp` で比べ、違えば deploy を失敗にする。deploy が success でも、engine が替わったかは実機のビーコンの版で確かめる。

抽出の木の鍵(`host-`)は、インストーラの名前と抽出の引数(`lift/crates/fe-build/src/main.rs`)と、extract-fl が repo から読む `lift/harness/seeds/os-fonts/` の 4 件を含む。含まないと、これらを変えても古い木が使われ、変更が本番に出ない(#1378 で足した)。

本番の bundle の入力(作業者の受け入れと同じにする)。`fe-build bundle` が読むのは、seed 表の行の host ファイル(`pack-seed.mjs` が SHA-256 を照合して入れる)と、`pack.args` の lazy 指定、それに `pack_step_for` が見つけた `error-offline.html`(`lift/out-app/host-extra/resources/System/Internet/`)だけである。seed 表の行は、言語・テーマ・テンプレート・demo・Blocks・内蔵プラグインの DLL の多くを含み、deploy は `extract-fl` で host に取り出す。作業者の resources の木(`ci/resources-prefixes.txt`)は、deploy の「resources を取り出す」の段も取り出すので、オフラインの頁が本番の bundle に入る(それまでは、deploy の木に無いので黙って省かれた)。木の中の余分な物は、seed 表に無いため bundle に入らない(写しの要らない理由)。内蔵プラグインの木(`host-extra/plugins`)と起動用の実行ファイル(`host-extra/exe`)は、bundle の入力ではなく、道具(tools/plugin-hooks・tools/feat-c1-hidpi)だけが読む。bundle の中身は、「bundle の中身を確かめる」の段が、オフラインの頁と、言語・テーマ・テンプレートの代表 1 件ずつを manifest で確かめる。

作業者の受け入れの連鎖で作られる物は、deploy の連鎖でも作られる(`spec/workflow.test.sh` が、`pack.args` の参照を検査する。#1337 の OTT の wasm は `fe-build bundle` が作り、その入力は `fe-build external` が取る。deploy.yml は 100 行以内で、シェルスクリプトを run に書かないため、作る段は fe-build に置く)。

外部プラグインの wasm(`pack.args` の `lift/out-app/plugins/` の lazy 指定)は、インストーラが `lift/out-app/external` に在る時だけ bundle に入る。作業者の run では、ラベル `plugin-external` の Issue だけがインストーラを置くので、他の Issue の受け入れの bundle は、その lazy 指定を省き、省いた物を標準エラーに 1 行ずつ出す。`fe-build bundle` は、3 件が 1 つも無ければ省き、一部だけ在る・一致しないときは止まる。deploy は「resources を取り出す」の段で 3 件を必ず取るので、wasm は必ず入り、「bundle の中身を確かめる」の段が、`pack.args` の外部プラグインの lazy の全てが manifest の lazy に在ることを確かめる(無ければ deploy を失敗にする)。将来、Serum 2・Vital の build が重くなったら、build の結果を R2 のキャッシュに置く(鍵は入力のハッシュ)。

deploy で `[skip ci]` を使わない(依頼者の指示。main の push が deploy を迂回しないように)。judge が、作業の commit の題名と本文に、skip ci などの印が在れば落とす。push の段は、Issue の題名から印を落としてから空の commit を作る。pm-work だけの push は、paths の filter で deploy が起動しないので、印を付ける理由が無い。

待ちが 1 つでジョブの上限の全てを使わないよう、作業者への規則を `AGENTS.md` の「作業者の手順」に置く。ビルド・測定などの長いコマンドは、背景で起動して終了を待たず、フォアグラウンドで直接実行する。シェルツールの `timeout` 引数は最大 5400000(90 分)とし、90 分で戻らないコマンドは固まったとみなして状態(ログの末尾と `ps`)を確かめる。作業は開始から 4 時間 30 分を過ぎたら、受け入れが通っていなくても commit して報告を書いて終える(ジョブの上限 5 時間 50 分に対し、報告を書く余裕を残す)。

## 15. 公開のログに出さない値(endpoint とアカウント ID)

このリポジトリのログは公開される。runner は `GITHUB_ENV` に書いた値を後続の全段の env 表示に出すため、endpoint とアカウント ID は `GITHUB_ENV` に書かず、runs-on/cache の段の `env:` には秘密(secrets)の値として渡す。runs-on/cache は endpoint を環境変数でしか受けないので、この移行は秘密の登録の後に行う(登録の前に移すと cache が壊れる。現行の書き方は、登録まで残る)。

## 16. push の衝突の自動の再試行

作業ジョブの「通った変更を main に push する」の段が、main との衝突(`git pull` の merge の衝突)で 3 回とも落ち、最後の試行が衝突だったときは、同じ Issue を自動で再試行する(手で `/retry` しなくてよい)。push の拒否(`remote rejected`・権限・保護による失敗)は対象外で、従来どおり `failed` を付ける(再試行しても同じ結果になるため)。

- 印: push の段は、衝突のときだけ、GITHUB_OUTPUT に `push_conflict=true` を書き、衝突したファイルのうち `pm-work/` の外の名前を `out/push-conflict.txt` に残す(成果物 `agent-<番号>` に入る)。最後の試行が拒否なら、前の試行が衝突でも印を残さない。
- 判定: 報告の「結果を集める」段が、作業ジョブの結論が failure で、Issue が open で、印(`out/push-conflict.txt`)が在るとき、Issue のコメントのうち印の行 `<!-- auto-retry:push-conflict -->` を含む物の数 k を数える。`tools/auto-retry.mjs decide` が、k が 2 未満なら `retry`、2 以上なら `fail` を出す。
- retry(k が 2 未満): 報告が、印の行と「自動で 1 回やり直す(k+1/2)」を含むコメントを書く(GITHUB_TOKEN)。`failed` は付けない。`running` の除去は、報告の最後の段(App の token)で行う。
- fail(k が 2 以上): 従来どおり `failed` を付け、「自動の再試行を 2 回行ったが、衝突が続く。人手(PM)が見る」と書く。
- 上限: 1 つの Issue につき自動の再試行は 2 回まで(初回と合わせて最大 3 回の push)。数えるのは印の行の入ったコメントだけなので、人の `/retry` は数えを増やさず、上限も変わらない。
- 起動の方式: GITHUB_TOKEN で行ったコメント・ラベルの変更は、次の run を起こさない。そこで、再試行の起動の段は App の token で `running` を外す。外すと `issues: unlabeled` のイベントが起き、select が Issue を選ぶ(`failed` が無く、`running` も無いため)。同じ Issue の run は concurrency(`agent-<番号>`、`cancel-in-progress: false`)で直列になるので、再試行の run は、今の run(報告まで)が終わってから始まる。再試行の run は、前回の作業(`work.diff`)を「前回の作業を取り込む」の段で載せてから始め、受け入れを通して push する。
- 鍵: App の鍵を受け取るのは、報告の「自動の再試行を起動する」の段だけで、再試行の Issue があるときにだけ動く(「8」)。その段は、botid の設定を作り、`gh` の設定ディレクトリを使って `running` を外し、使い終えた鍵の一時ファイルと `hosts.yml` を消す。
- 確認の状態: 印の数え方と判定と、`running` の除去の呼び出しは `spec/auto-retry.test.mjs` と `spec/workflow.test.sh` の (u) で確かめる。App の操作が run を起こすことと、報告の段が App の名義で `gh` を使えることは、実際の run で確かめるまで未確認(「9」)。

## 17. Project の Status(In Progress・In Review)の更新

- 対象: 組織のプロジェクト `signature-studio/3` の Status(単一選択)。start の段が、起動した Issue を `In Progress` に移し、report の段が、結果を書いた後に `In Review` に移す。道具は `tools/project-status.mjs set <選択肢の名前> <番号>...`。選択肢は名前で引き、大文字・小文字の違いは無視する(実物の選択肢は `In progress`・`In review`)。
- 順序: In Progress は claim の後(エージェントの開始の直前)。In Review は、結果を書く段(閉じる処理を含む)の後で、自動の再試行の起動より前。再試行の run は、その後に起動するため、最後の Status は再試行の作業の In Progress になる。
- 閉じた Issue は触らない。閉じた Issue は、Project の組み込みの自動化が Done にする。移すのは、Issue が open のときだけ。
- 権限: 組織のプロジェクトは GITHUB_TOKEN では読み書きできないため、App の token を使う。App に組織のプロジェクトの読み書きの権限が要る(PM が App の設定で足す)。ローカルの gh の資格では、Status の選択肢(`In progress`・`In review` を含む)を読めた。App の権限は、run でしか確かめられないため未確認。
- 失敗: 権限が無い・Project が読めない・選択肢が無い・gh が失敗したときは、理由を標準出力に 1 行ずつ出し、段は 0 で終わる(作業は止めない。`continue-on-error` は使わない)。権限が無いときは、Issue に注記を 1 回だけ書く(印 `<!-- project-status:no-access -->` のコメントが在れば書かない)。App の token を取れないときも、理由を出して 0 で終わる。
- 鍵: App の鍵を受け取るのは、start と report の Project の段だけ。作業者の段と作業ジョブには渡さない。
- 試験: `spec/project-status.test.mjs`(合成の応答と偽の gh で道具を試し、agent.yml の段の置き場所・条件・鍵を確かめる)。
