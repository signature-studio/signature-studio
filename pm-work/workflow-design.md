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
| `issues` | `closed` | 閉じた Issue が `blocking` している Issue の全て(ブロッカーの解除を、ブロックされる側の起動に変える) |
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
| start | select | 対象が空でない | `contents: read`・`issues: write` | App の token(名義の設定の段) | 対象を同じ条件でもう一度確かめ、`running` を付ける。`/retry` なら `failed` を外す(`claim`) |
| extract | start | 起動された対象が空でない | 既定 | GITHUB_TOKEN なし(R2 の鍵のみ) | 抽出物を R2 のキャッシュから用意する(今の段をそのまま) |
| work | start・extract | 起動された対象が空でない | `contents: read`・`issues: read` | 本文の取得だけ GITHUB_TOKEN。作業者の段には無い | Issue ごとに: 本文を取る → 受け入れコマンドを取り出す → 作業者を動かす → 判定 → 通れば push |
| report | start・work | 常に(`always()`)。起動された対象が空でない | `contents: read`・`actions: read`・`issues: write` | 読みは GITHUB_TOKEN、書きは App の token | Issue ごとに結果をコメントし、状態を変える |

- `if:` を書いたジョブは、既定の `success()` の判定が外れるので、`start`・`extract`・`work` の `if` には `success() &&` を付ける(上流が落ちたときに、空の対象で後続が走らないため)。
- work は `strategy.matrix.issue` で Issue の番号ごとに 1 ジョブ。`fail-fast: false` で、1 件の失敗が他を止めない。
- work の `concurrency` は `group: task-<番号>`、`cancel-in-progress: false`。同じ Issue の作業は直列になる。
- 対象が空なら start 以降は skip される。runner を使うのは select(数十秒)だけ。

## 4. 作業の段の中身

1. 本文を取る: `gh api repos/<repo>/issues/<番号> --jq '.body // ""'` を `$RUNNER_TEMP/task.md` に置く。
2. 受け入れコマンドを取り出す: `node tools/task-accept.mjs < task.md > task-accept.md`。` ```accept ` ブロックが 1 つで中身が 1 行のときだけ成功。無ければ、理由を `out/judge.txt` に残して失敗する(report が Issue にコメントする)。
3. 作業者のプロンプト: `AGENTS.md` + `---` + 「## タスク(Issue #番号)」+ 本文。本文以外の情報は渡さない。
4. 作業者を 1 回動かす(opencode の導入・認証・起動は今の段のまま)。作業者の段には token が無い。
5. 判定: `guard/judge.sh`(取り直した main の物)に `task-accept.md` を渡す。
6. 通ったときだけ push(今の形: pull → push。衝突時は中断して再試行)。push できた SHA を `out/commit.txt` に残す。
7. 成果物 `agent-<番号>`(`out/`: 報告・判定の出力・SHA)を 7 日保存する。

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

- 通った: 「通った: <run の URL>」「commit: <SHA の URL>」「受け入れの出力(末尾 8 行)」をコメントし、Issue を close してから `running` を外す(close より前に外すと、外した直後の `unlabeled` で、まだ open のまま再起動してしまう)。
- 落ちた: 「落ちた: <run の URL>」「失敗した段」「判定の出力(末尾 8 行)」をコメントし、`failed` を付けてから `running` を外す(逆順だと、外した直後の run が `failed` の無い状態を見て再起動する)。
- 取り消された(cancelled)・ジョブが動かなかった: 落ちたのと同じく `failed` を付ける。
- 再試行: Issue に `/retry` と書く、または `failed` ラベルを外す。`/retry` のときは start が `running` を付けてから `failed` を外す(その逆順だと、外した直後の `unlabeled` が `running` の無い状態を見て二重に起動する)。

## 8. トークン

- ラベル・コメント・close は、名義の設定の段で作る App の token で行う。GitHub の仕様では、GITHUB_TOKEN で行った変更は次の run を起動しない(`workflow_dispatch`・`repository_dispatch` を除く)。GITHUB_TOKEN で close すると、ブロックされていた Issue が起動しない。
- start と report の書き込みに使う App の token は、push の段と同じ仕組み(`botid`)で発行する。
- 作業者の段(opencode)には、どの token も無い(今と同じ)。
- 本文の取得だけ、GITHUB_TOKEN(`issues: read`)を、その 1 段にだけ渡す。

## 9. 既知の限界

- `blocked_by_added` は Actions の event に無い。依存を後から付けた Issue は、付ける前に起動していれば止まらない。PM は `pm-hold` を付けて作り、依存を付けてから外す(`unlabeled` で検知する)。
- `blocked_by_removed` も無い。依存を手で外しただけでは起動しない。Issue にコメントするか、本文を `edited` にすると、再評価される。
- 同じ Issue への run が 2 つほぼ同時に来ると、start の再確認の間に両方が通ることがある。残りの窓は数秒。work の `concurrency` で作業は直列になるが、0 にはならない。
- start が途中で落ちると、それまでに付いた `running` が残る。PM が外す。report は動かない(claimed が空のため)。
- 作業ジョブの鍵の一時ファイル(`$RUNNER_TEMP/botid-config.json`)は、作業の段から読める(旧設計から同じ)。

## 10. PM の決定

依頼者に判断を求めるのは、依頼者だけが決められる事項だけにする。それ以外は PM が決め、ここに書く。

1. **起動する数**: ブロッカーの無い task を常に 60 件以上起票して、60 の作業を動かし続ける。起票は PM が `issue-drafts/WAVES.md` の波で行う。閾値(`queue/threshold`)と生成器は作らない。60 の同時実行は GitHub の上限に任せる。
2. **`pull_request`**: 置かない。PR を使う処理が無いため。
3. **ラベル**: 運用は `pm-hold`・`running`・`failed` の 3 種。種別は Issue の Type(Task・Feature)、規模はプロジェクトの Size、優先度は組織の Issue field の Priority で指定する。PM は `needs-owner` を使わない(依頼者の判断待ちで起動を止める Issue は作らない)。表示用のラベルは PM が作る(`issue-drafts/LABELS.md`)。
4. **App の権限**: start・report・push は、既存の App の token を使う。Issues への書き込みの権限は、初回の起動の結果で確かめる。
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

## 11. 依頼者の指示との対応

| 指示 | 設計 |
| --- | --- |
| Issues・PR の作成/更新、コメント作成を認める。workflow_dispatch は Issue 番号のみ、検知 event があれば不要 | `issues`・`issue_comment` の types を「2」の通り置く。`workflow_dispatch` は置かない(検知 event `issues: closed` がある) |
| Issue created で起動し、ブロッカーがあれば run は即座に終わる | opened の run は select だけ走り、対象が空なら以降は skip される |
| タスクの全情報を Issue に含める | 本文と ` ```accept ` ブロック。作業者には本文だけ渡す |
| `queue/`・`ci/tasks/`・`driver/` を削除し、マイルストーンは GitHub Milestones、タスクは Issues と sub-issues に | 削除は別の担当。この設計はこれらに依存しない |
| 依存は blocking/blocked-by で示し、ブロッカーの無いタスクは即座に起動 | `blocked by` の open が 0 の Issue を、起動の条件にする。ブロッカーが閉じた `closed` で後続を起動する |
| PM 資料は `pm-work/` | この文書を含め、PM 資料は `pm-work/` に置く |
| Issues・Milestones・ラベル・PR への書き込みは承認まで禁止 | この文書は書き込みを行わない。ラベル・Issue は承認後に PM が作る |
