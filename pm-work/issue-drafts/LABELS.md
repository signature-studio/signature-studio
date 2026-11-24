# ラベル(依頼者の承認の後に PM が作る)

運用に使うラベルは 3 種。設計は `pm-work/workflow-design.md` の「6」に従う。既存の旧ラベル `ready` は使わない(設計の「6」)。

種別は Issue の Type(Task・Feature)、規模はプロジェクトの Size、優先度は組織の Issue field の Priority で指定するため、`task`・`epic`・`priority-*` のラベルは作らない。

## 運用に使う 3 種(起動の判定に使う)

| ラベル | 色(案) | 意味(設計の「6」) | 付ける人・物 |
|---|---|---|---|
| `pm-hold` | `fbca04` | 依存関係を付けている途中。起動しない。起票の時に全ての Task に一時的に付け、依存を付けてから外す | PM |
| `running` | `0e8a16` | 作業中 | start が付け、report が外す |
| `failed` | `b60205` | 落ちた。再試行は `/retry` か、このラベルを外す | report が付ける |

## 表示用のラベル(起動の判定には使わない)

本文の探しやすさのためのラベル。PM が作る。

| ラベル | 色(案) | 説明 |
|---|---|---|
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
| `perf-beacons` | `ededed` | 実機のビーコン(依頼者の実機の測定の記録。`agent.yml` が R2 から取り、作業に置く) |
| `startup` | `c5def5` | 起動の時間(P1)とサイズ |
| `verify` | `ededed` | 測定と検証(道具を含む) |
| `clipboard` | `bfdadc` | FL のクリップボード(ページの Clipboard API と結ぶ) |
| `cursor` | `f7c6c7` | マウスカーソル(標準のカーソル・窓の端・FL 自身の資源のカーソル) |
| `input` | `d1ddf0` | 入力(マウス・キー)の経路 |
| `topbar` | `fde7a3` | トップバー(上部の表示の値) |
| `fl-exe` | `c7d7e8` | 起動用の FL の実行ファイル(FL64.exe)を、作業の段で取り出す |
