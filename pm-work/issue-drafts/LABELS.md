# ラベル(依頼者の承認の後に PM が作る)

運用に使うラベルは 5 種。設計は `pm-work/workflow-design.md` の「6」に従う。既存の旧ラベル `ready` は使わない(設計の「6」)。

## 運用に使う 5 種(起動の判定に使う)

| ラベル | 色(案) | 意味(設計の「6」) | 付ける人・物 |
|---|---|---|---|
| `task` | `1d76db` | 作業の対象。作業者が 1 つの成果を終える Issue(`accept` を持つ) | PM |
| `epic` | `5319e7` | 子(sub-issue)を持つ親。起動しない | PM |
| `pm-hold` | `fbca04` | 依存関係を付けている途中。起動しない。起票の時に全ての task に一時的に付け、依存を付けてから外す | PM |
| `running` | `0e8a16` | 作業中 | start が付け、report が外す |
| `failed` | `b60205` | 落ちた。再試行は `/retry` か、このラベルを外す | report が付ける |

`needs-owner` は作らない(依頼者の判断待ちで起動を止める Issue は、今は無い)。

## 表示用のラベル(起動の判定には使わない)

本文の探しやすさのためのラベル。PM が作る。

| ラベル | 色(案) | 説明 |
|---|---|---|
| `priority-p3` | `e99695` | P3(Song Mode で音が途切れない)に関係する |
| `priority-p2` | `f9d0c4` | P2(UI の 120 fps)に関係する |
| `priority-p1` | `c5def5` | P1(起動 10 秒以内)か、依頼者の課題 C-3・C-4・C-5 に関係する |
| `render` | `c2e0c6` | 描画の合成と提示(描画の Worker) |
| `audio` | `bfe5bf` | 音声の実測と道具 |
| `hidpi` | `bfd4f2` | HiDPI(物理画素と DPI) |
| `topbar-menu` | `f9d0c4` | トップバーのコンテキストメニュー |
| `delivery` | `c5def5` | 起動前と起動後の配信(lazy-load) |
| `resources` | `d4c5f9` | 言語・テーマ・テンプレート |
| `inventory` | `fef2c0` | インストーラの全件の一覧 |
| `plugin-builtin` | `e4e669` | 内蔵のエフェクトとインストゥルメント |
| `plugin-external` | `0052cc` | 外部プラグイン(Serum 2・OTT・Vital) |
| `perf` | `d4c5f9` | エンジンの性能(直接 wasm 化) |
| `startup` | `c5def5` | 起動の時間(P1)とサイズ |
| `verify` | `ededed` | 測定と検証(道具を含む) |
