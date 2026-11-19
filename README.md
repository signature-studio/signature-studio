# signature-studio

FL Studio(Windows 版)のエンジンを x86-64 から C / wasm へ変換し、ブラウザで動かすプロジェクトです。目標と役割は `AGENTS.md` にあります。

| 場所 | 中身 |
|---|---|
| `app/` `lift/` `tools/` `patches/` `ci/` | 製品(前の世代の main から、そのまま運んだ物) |
| `guard/` | 判定の道具。作業ジョブの最後の段で `guard/judge.sh` を呼びます |
| `spec/` | テスト(`bash spec/guard.test.sh`) |
| `ci/tasks/` | タスク(前の世代の 6 件がある) |
| `ledger/` | 依頼者の指示の台帳 |
| `opencode.jsonc` | エージェントの設定(作業者は Haiku 5.5、監査・受付は Sonnet 5.5) |

駆動とワークフローは、まだありません。
