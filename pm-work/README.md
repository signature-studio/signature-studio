# pm-work/

PM(計画・監査・受付)の資料を置く場所です。依頼者の指示により、PM 資料は全てここに格納します。

| パス | 中身 |
|---|---|
| `plan.md` | 旧計画(参考)。`queue/plan.md` から移動した |
| `findings/` | survey の実測(`survey-c1`〜`c6`) |
| `inputs/` | 旧入力の表(`lift-function-classes.tsv`、`lower-rejected.tsv`) |
| `old-tasks/` | 旧タスク本文。Issue の下書きの材料 |
| `old-tasks/queue-tasks/` | 旧 `queue/tasks/`(旧タスク本文) |
| `old-tasks/queue-done/` | 旧 `queue/done/`(完了した survey の記録) |
| `old-tasks/ci-tasks/` | 旧 `ci/tasks/`(計測タスクの本文) |
| `ledger/` | 依頼者の指示の台帳(`instructions.tsv`) |
| `issue-drafts/`・`milestones.md` | 承認待ちの下書き(Issues・Milestones)。承認が出るまで GitHub へ書き込まない。別の担当が書く |

- マイルストーンは GitHub Milestones に、タスクは Issues とその Sub-Issues に格納する。
- Issue の中に、タスクに関する全ての情報を含める。
- 旧 `queue/`・`driver/`・`ci/tasks/` は削除した(下の「削除した物」を参照)。
