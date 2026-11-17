# 0422: emsdk を 7.0.0 へ、Node を 26(active LTS)へ上げる(R6-42 で実施。engine の sha は変わる)
分類: (a) 除去(古い版の固定を外す。版の固定は ci/versions.env と .github/workflows/deploy.yml の 2 箇所)。
撤回条件: (1) 新しい版の engine で、R6-34 の観測(起動・About・再生・Export)が、今の版より悪くなったとき。(2) 新しい版の deploy の run が失敗したとき(失敗したステップを見て、版を戻す)。(3) 依頼者が版の変更を禁じたとき。
日付: 2026-11-17   ROADMAP: R6-42(doing)   一覧: なし   証跡: この文書(調査の出力を下に書いた)。
番号の理由: docs/decisions の最後は 0421(2026-11-17 の d1dda509 の時点)。0422 が最初の空き。

## 0 調べたこと(公式の一覧。記憶で書かない。2026-11-17 に取った)

| 対象 | 取得コマンド | 結果 |
|---|---|---|
| emsdk の release | `gh api repos/emscripten-core/emsdk/releases` | 0 件(GitHub の release は作られていない) |
| emsdk の tag | `gh api repos/emscripten-core/emsdk/tags` | 先頭が 7.0.0、次に 6.0.13、6.0.12、6.0.11 … |
| 7.0.0 の commit | `gh api repos/emscripten-core/emsdk/git/refs/tags/7.0.0` → commits | 1693ce7359fc、2026-11-06T20:42:08Z、"Release 7.0.0 (#1814)" |
| 6.0.13 の commit | 同 | 35659964bc27、2026-10-19T19:48:59Z、"Release 6.0.13 (#1811)" |
| 6.0.12 の commit(今の固定) | 同 | 35ff8a6d1505、2026-10-08T22:42:54Z、"Release 6.0.12 (#1807)" |
| Node の一覧 | `curl -s https://nodejs.org/dist/index.json` | 全体の先頭は v27.0.0-alpha.0.1.1(alpha。LTS でない)。LTS の先頭は v26.13.1 (Lithium)、v24 の最新は v24.22.3 (Krypton) |
| Node の予定 | `curl -s https://raw.githubusercontent.com/nodejs/Release/main/schedule.json` | v26: LTS 2026-10-28、maintenance 2027-10-20、end 2029-04-30。v24: LTS 2025-10-28、maintenance 2026-10-20、end 2028-04-30。v22: end 2027-04-30 |

今の固定: emsdk 6.0.12(ci/versions.env の EMSDK_VERSION と EMSDK_REV = 35ff8a6d…。deploy.yml の setup-emsdk の version: 6.0.12)。Node 24(ci/versions.env の NODE_VERSION = 24。deploy.yml の setup-node の node-version: 24)。

## 1 決定

1. emsdk は 7.0.0 へ上げる。根拠: 公式の tag の先頭で、"Release 7.0.0" の commit(2026-11-06)。6.0.12 より新しい release である(6.0.13 も 6.0.12 より新しいが、7.0.0 が最新)。
2. Node は 26(active LTS。2026-10-28 から)へ上げる。根拠: v24 は maintenance LTS(2026-10-20 から。終了は 2028-04-30)で、active LTS は v26。app の build・test・pack-assets(node)・extract(node)に使う。
3. 実施(2026-11-17。証跡 evidence/R6-42.md)。版の変更は ci/versions.env と deploy.yml の 2 箇所(emsdk 7.0.0・Node 26)。フルビルドで engine.wasm の sha256 は変わり得る(emsdk の emscripten が変わるため)。新しい sha は記録し、verified.txt には足さない(検証の前なので root は切り替えない)。
4. 実施の確かめ: deploy の run が success か、gate の copy が 0 件か、root への upload が 0 件か。検証(起動・About・再生・Export・Problems。R6-34 の手順)は別の依頼。

## 2 未確認

- 実施の結果: engine の sha は 5029a700(165755648 B)に変わった(b440 は 165776779 B)。build は壊れなかった(R6-42 §C)。
- Node 26 で app の build と test は通った(npm ci・build・test・test:worker が rc 0。R6-42 §A)。
- ci/versions.env の冒頭の注は、build-engine-bundle.sh(今は存在しない。0405 の記録)を名指ししている(古い。実施の作業で直すかどうかを決める)。
5. 結果(2026-11-17): engine の sha は 5029a7008bc7bc8a2c44aa24fe708d402897a444dbe7daeb1b77e01d5817a6a1(verified.txt には足さない)。deploy の run の結論は R6-42 §D(push 後に記録)。
