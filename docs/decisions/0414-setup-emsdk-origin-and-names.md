# 0414: deploy.yml の emsdk の表記を転送先の emscripten-core/setup-emsdk に直す。変数名は今回直さない(100 行・80 字と両立しないため)
分類: 整備(workflow の記法。処理の増減は無い。`AGENTS.md` §6 の (a)(b)(c) には当たらない)
撤回条件: 転送先の v16 で run が success にならない、emsdk が 6.0.12 で入らない、またはログが mymindstorm 版の run(R6-30 の 43862867326)と違う。
日付: 2026-11-17   ROADMAP: R6-36a(doing)   依頼: 2026-11-17 の依頼者の発言(commit 4dd4f8c3 の 48 行・14〜16 行への指摘)
注: 当初は 0412 と付けたが、0412 は R6-36b の決定(95628ace)と番号が重複したため 0414 に改めた(次の空き番号。0413 は R6-38 が使用)。

## 0 範囲と件数(置き換える前の確認)

- 置き換えた物: `.github/workflows/deploy.yml` の `uses:` の 1 行(48 行目)。`grep -rn 'uses:' .github/workflows/` は 11 行(deploy.yml 5、agent.yml 6)。uses の名前は 6 種類(actions/checkout・actions/setup-node・actions/upload-artifact・runs-on/cache・cloudflare/wrangler-action・mymindstorm/setup-emsdk)。各々を `gh api repos/OWNER/REPO --jq .full_name` で照合すると、転送されるのは `mymindstorm/setup-emsdk` の 1 件だけ(他の 5 件は full_name が同じ)。
- 直さなかった物: deploy.yml の 1〜2 字の変数の参照 73 件(HEAD の実測: `git show HEAD:.github/workflows/deploy.yml | grep -o ...` で O 33・H 26・F 4・A 2・U 3・Q 3・S 2)。agent.yml の 1〜2 字の変数は 0 件。
- 依頼者の過去の発言の記録: `~/work/handover/setup/S16-intent/catalog.md` に、emsdk・action・変数名の指示は無い(`grep -n -i -E 'ツール|emsdk|アクション|action|公式|ビルド'` の該当 0 件)。

## 1 出所の実測

- `gh api repos/emscripten-core/setup-emsdk --jq '{full_name,archived,pushed_at}'`: emscripten-core/setup-emsdk、archived false、pushed_at 2026-09-13T10:39:54Z。repo id は 203863565。
- `curl -s -o /dev/null -w '%{http_code} %{redirect_url}\n' https://github.com/mymindstorm/setup-emsdk`: `301 https://github.com/emscripten-core/setup-emsdk`。API も 301 で、転送先は repositories/203863565(同じ id)。よって mymindstorm は改名前の名前で、別の action ではない。
- `gh api repos/emscripten-core/setup-emsdk/releases/latest`: v16(published 2026-04-08T21:52:43Z、prerelease false)。タグ v16 は commit 4528d102f7230f0e7b276855c01ea1159be0e984(`git/refs/tags/v16`)。
- v16 の action.yml の入力: version(既定 latest)、emsdk-version、no-install、no-cache、actions-cache-folder(既定は空)、cache-key、update、update-tags。deploy.yml が使うのは version(6.0.12)だけ。actions-cache-folder は未設定で、この action のキャッシュは使わない。
- 選んだ理由の記録: `git log --reverse -S 'mymindstorm'` の最初は 8f2e4ec6(R6-25 §13、2026-11-16)。evidence/R6-25.md・docs/decisions の 0400・0405・docs/failures に理由は無い(0405 は対応表だけ)。`git log -S 'emscripten-core/setup-emsdk'` は 0 件。事実: 理由は未記録。最新・公式かを調べた記録は無い(0017 と同じ型)。
- emsdk 本体の最新タグ: `gh api repos/emscripten-core/emsdk/tags` の先頭は 7.0.0、6.0.13、6.0.12。プロジェクトの 6.0.12 は ci/versions.env の固定値。今回は変えない。版の更新は、出力の同一性の検査とセットの別の依頼(0017 の「版の更新の影響」)。

## 2 決定 1(action の表記): 変える

決定: `uses: mymindstorm/setup-emsdk@v16` を `uses: emscripten-core/setup-emsdk@v16` にする。
根拠: 同じ repo(id 203863565)への転送。タグ v16 は最新リリース。入力 version: 6.0.12 は同じ。キャッシュは使わない(同等)。
実測: `wc -l -L`(前)100 80、(後)100 80。`grep -c mymindstorm .github/workflows/deploy.yml` 0。

## 3 決定 2(変数名): 今回は変えない

依頼された対応(案): O→BUILD_DIR、H→HARNESS、F→FE_BIN、S→ENGINE_SHA、A→ACCOUNT_ID、U→R2_ENDPOINT、Q→MANIFEST_OPTS。
実測(一時ファイルで置換して数えた。deploy.yml は変えていない。スクリプト: /tmp/opencode/r636a/sim2.py):

| 案 | 名前 | steps のインデント | 80 字超の行 | 最長 |
|---|---|---|---|---|
| 1 | 上の案 | 据え置き | 13 行 | 107 字 |
| 2 | 上の案 | 2 字減らす | 10 行 | 105 字 |
| 3 | O=OUT・H=HARN・F=BIN(他は上の案) | 2 字減らす | 4 行(66・78・82・93) | 91 字 |

案 3 の 66 と 82 は 1 命令だけの行。折り返すと 2 行増え、100 行を超える(102 行)。78 と 93 は複数行の命令の中なので、組み直せば行を増やさずに済む可能性があるが、未確認。

行を減らす手段の検討(実測):
- (i) GITHUB_ENV への 2 行の echo を printf の 1 行にする → 88 字(超過)。
- (ii) A と U の 2 行を 1 行にする → 102 字(超過)。
- (iii) 命令を `;` でつなぐ → 80 字に収まる組が無く、読みにくくなる。`&&` でつなぐと、失敗時の動きが変わる(不可)。

決定: 変数名は今回直さない。根拠: 意味の分かる名前にすると、100 行・80 字の両方を満たせない。満たすには run の命令の並びを変える必要があり、それは依頼者の別の依頼(ビルドの作り直し)の範囲。
未達: 完了条件の「旧名の参照 0 件」は未達(旧名は残っている。HEAD の実測 73 件)。

## 4 検査

- `scripts/scan-forbidden.sh`: 結果は evidence/R6-36a.md の §4 に貼る。
- 変更範囲: deploy.yml・この文書・evidence/R6-36a.md・ROADMAP の 1 行だけ。

## 5 結果(run)

- run 44024013509(headSha 2170029d)は completed・success。`emscripten-core/setup-emsdk@v16` のステップは success で、ログに `SHA:4528d102f7230f0e7b276855c01ea1159be0e984` と `version: 6.0.12` が出ている。詳細は evidence/R6-36a.md の §5。
- ログの該当: `[command]…/emsdk install 6.0.12`(250 行)と `Done installing SDK '…'`(263 行)。build の出力(wasm の鍵)は変えていない。鍵の一致は、この run では検査していない(R6-34・R6-35 の対象)。
