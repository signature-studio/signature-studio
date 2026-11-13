実エンジン: 実行なし(草案。R2・Worker に書き込んでいない)
書き込み先: root の manifest.json と backup/ 以下だけ
草案の場所は .github/workflows/ の外(evidence/R6-4-workflow-draft.yml)。置けば push で走るため、本番の場所への移動は reviewer の検査の後

# R6-4 草案の読み合わせ(設計 `evidence/R6-4-design.md` §2 との対応)

## 1 段と step の対応(草案の行番号は `evidence/R6-4-workflow-draft.yml`)

| 設計の段 | 草案の step(行) | 失敗時 | 状態 |
|---|---|---|---|
| 段 0 アカウント ID | `0 …`(33 行)`id: acct` | 後続は `if` で止まる | 備えている |
| 段 1 root を読む・分岐・退避・書き込み前の確認 | `1 …`(56 行)`id: r1` | exit 1(root は書かれない) | 備えている。分岐(sha が E13 なら `skip=true`)は 93 行 |
| 段 2 E13 の 5 件の確認(書かない) | `2 …`(149 行)`id: r2` | exit 1(root は書かれない) | 備えている(下の注 1) |
| 段 3 E13 を root に書き、確かめる | `3 …`(226 行)`id: r3` | 段 5 で戻す | 備えている |
| 段 4 root を読み直し、判定 | `4 …`(266 行)`id: r4` | 段 5 で戻す | 備えている |
| 段 5 表と戻し | `5 …`(298 行)`if: always()` | 戻しも失敗なら退避のキー名と sha を出し exit 1 | 備えている |

追加した step: `Node 24 を入れる`(28 行)。段 2 と段 4 の判定関数を node 24 で呼ぶための準備で、設計の段の外の処理ではない(分岐は足していない)。

## 2 設計との照合(できている点)

- 起動・権限・並行・時間: `on: push` の `paths` は `.github/workflows/default-entry.yml` だけ。`permissions: contents: read`。`concurrency` は group 1 本で `cancel-in-progress: false`。`timeout-minutes: 30`。`workflow_dispatch`・`schedule`・入力・環境変数の旗は無い(grep 0 件)。
- 段 1 の sha の分岐: root の sha が E13 の値なら `skip=true` で段 2 以降が `if` で飛ばされる。
- 段 1 の退避: 退避先 `backup/manifest-root-<UTC>.json` の head が 404 であることを確かめてから put。put の後、退避の head と get で大きさ・sha が root と一致することを確かめる。
- 段 2 の 5 項目: (i) sha256、(ii) `engine.kind` と `judgeManifestBytes` の ok(node 24)、(iii) `engine.wasm.url` が `/assets/fl-engine-ci-0363b050/` で始まる、(iv) wasm の head が `br`・15,697,877 B、get して展開し 143,087,706 B・sha256 `58ad4a6a…`、(v) manifest の `/assets/` の URL 4 件の head。
- 段 3: 段 2 で R2 から読んだバイト列(`$W/e13.json`)を put。ローカルのファイルからは書かない。ContentType `application/json`、CacheControl `no-cache`、ContentEncoding は付けない。head と get で 3,098 B・E13 の sha を確かめる。
- 段 5: 段 3 か 4 が失敗していたら、退避を get して sha が root の元の値と一致することを確かめてから put し、head の大きさと get の sha で確かめる。

## 3 設計とずれた点・読み取れなかった点・未確認

- 注 1(段 2 の判定): 段 2 の判定は `judgeManifestBytes` を node 24 の `--experimental-strip-types` で呼ぶ。手元の node v24.19.0 で E13 の manifest に対して `{"ok":true,"kind":"emscripten-fruity-v1"}` を確かめた(`artifacts/bundles/fl-engine-ci-0363b050/manifest.json`、sha `57817472…` 一致)。GitHub の runner での実行は未確認。
- 注 2(段 5 の戻しの範囲): 戻しは段 3 か 4 が失敗したときだけ行う。段 1 の退避の後で段 2 が失敗した場合は root がまだ書かれていないため、戻しは行わず、退避は残る(設計どおり。退避のキーは表に出る)。
- 注 3(段 1 の退避が失敗した場合): 退避の put が失敗すると段 1 は exit 1 で、root は書かれない。退避の head・get のどちらかが失敗しても同様。
- 注 4(head の 404 判定): 退避先の確認は、head の stderr に `404|Not Found|NoSuchKey` が出ることで「無い」と判断する。aws CLI の実際の文言は未確認(R2 に接続していない)。
- 注 5(bash の空配列): 段 1 と段 5 で `"${META[@]}"` を使う。メタデータが空のときの動作は bash 5 の runner を前提にしている(未確認。ローカルの bash では `bash -n` のみ確認)。
- 注 6(ContentEncoding の再現): 段 1 の退避と戻しは、root の head の ContentType・ContentEncoding・CacheControl を有るものだけ付ける。root が現在どの値を持つかは未確認(R2 を読んでいないため)。
- 未確認(実行していない): YAML 全体の GitHub Actions 上の解釈、R2 への接続(どの段も実行していない)、Cloudflare API の応答の形(`result` 配列。前例 `engine-bundle.yml` の段 2 と同じ形を写した)、Node の展開の大きさ(143 MB は runner のメモリで足りると読むが未確認)。
- 公開ホストの確認(R4-5)は含まない。設計 §3 のとおり依頼者機の計測で行う。

## 4 検査の結果

- 構文: `yaml.safe_load` 解析 ok(8 step)。各 `run:` を取り出して `bash -n` で 6 本とも ok。
- 禁止形 `workflow_dispatch|schedule:|upload-artifact|wrangler|sleep|nohup|disown`: 0 件。
- Secrets: `CLOUDFLARE_API_TOKEN` 1 件、`R2_CACHE_ASSETS_ACCESS_KEY_ID` 5 件、`R2_CACHE_ASSETS_SECRET_ACCESS_KEY` 5 件。3 つの名前だけ。
- 書き込み系(`put-object` 3 行): 124 行(退避。キー `$BK` = `backup/manifest-root-…`)、241 行(root の `manifest.json`)、333 行(戻し。root の `manifest.json`)。root と backup/ 以外へ書く行は 0。`delete-object`・`copy-object`・`s3 rm|cp|sync` は 0 件。
- `add-mask`: 2 件。1 件は段 0 の `echo "::add-mask::$id"`(52 行)、もう 1 件は草案の先頭の注釈(3 行目)の文字列。
- 判定関数: 草案の段 2 と段 4 と同じ書き方で、手元の E13 の manifest に対して ok を確かめた。
