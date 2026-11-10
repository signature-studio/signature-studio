# signature-studio

FL Studio をブラウザ上で動かす(Windows 版 FL Studio のエンジンを x86-64 → C → wasm に lift する)プロジェクト。
ゴールには性能 P1〜P3 を含む。所定のプロジェクト(`~/ダウンロード/serina1-serum2.zip`)を使って測る。

## エージェント(後任セッション)が最初に読む物
1. `AGENTS.md` — 規約(1 ページ。毎ターン読む)
2. `docs/ops/lessons.md` — 前任の失敗(1 ページ)
3. `prompts/START.md` — 最初の 10 分の手順

## 人間(依頼者)が起動前に読む物
- `docs/ops/launch.md` — 起動前の確認項目と起動手順(`scripts/launch.sh`)
- `docs/requests/` — 依頼者に頼む作業の Issue(形式は `docs/ops/requests.md`)

## 構成
| パス | 内容 |
|---|---|
| `AGENTS.md` | 規約 |
| `docs/ops/` | 運用規約(権限・環境・役割と手順・報告・誠実性・忠実性と性能・危険操作・辞書・教訓・起動) |
| `docs/requests/` | 依頼者に頼む作業の Issue(形式は `docs/ops/requests.md`) |
| `prompts/` | 起動プロンプト、worker/reviewer への指示の雛形と規則ブロック |
| `scripts/` | preflight・audit・scan-forbidden・env・set-model-profile・launch・setup-repo(ハーネス担当が作成) |
| `docs/ROADMAP.md`、`docs/STATE.md`、`docs/core/` | ロードマップ・現在の状態・技術文書(存在しなければ worker に旧資料から作らせる) |

## 状態
- 新リポジトリでの再出発。旧リポジトリは削除済み。旧ワークツリーは参照用に残っている(読み取りのみ)。
- PUBLIC は意図的(GitHub Actions の並列利用のため)。commit したら push する。

## 注意
- 成果物(wasm、FL の DLL、インストーラ、調査の抜粋)をこのリポジトリに入れない。入れる前に `scripts/scan-forbidden.sh` を通す。
- 前任の記録(`~/work/handover/`)は禁止文字列を含む。このリポジトリへコピーしない。
