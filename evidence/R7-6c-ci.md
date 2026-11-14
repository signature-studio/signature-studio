# R7-6c: engine-bundle の run の状態(事実のみ)

取得日: 2026-11-15(JST)。対象: `.github/workflows/engine-bundle.yml` の run。ROADMAP の状態は変えていない。

## 1. 取得したコマンドと出力の要点

- `gh auth status`: GitHub にログイン済み(Active account: true)。
- `git log --oneline` の先頭: `4de7546`(origin/main と HEAD が一致)。`33b676a`(R7-6: 到達しない DPI 分岐の除去)は `659ff0e` の祖先。
- `git ls-remote origin main`: `4de754647054e0f04da8bea3c6af34e84f37b01d refs/heads/main`(取得時点)。
- `gh run list --workflow engine-bundle.yml --limit 100 --json ...`: 件数 19(この workflow の run の総数。上限 100 に届いていない)。
- 各 run の headSha と `33b676a` の関係は `git merge-base --is-ancestor` で確かめた。

## 2. 33b676a 以降の push に対する run

`headSha` が `33b676a` を含む run は 0 件(19 件中 0)。

| run id | status | conclusion | headSha | createdAt(UTC) | `33b676a` との関係 |
|---|---|---|---|---|---|
| 43608277966 | completed | success | 659ff0e | 2026-11-14T15:05:31Z | 子孫(33b676a の後) |

`33b676a` より前の run は 18 件(すべて completed・success)。

| run id | headSha | createdAt(UTC) |
|---|---|---|
| 43584338146 | b25aa33 | 2026-11-14T10:14:36Z |
| 43584013233 | d4be398 | 2026-11-14T10:10:35Z |
| 43582706673 | e4dc679 | 2026-11-14T09:54:03Z |
| 43572936481 | a5f2d5a | 2026-11-14T07:50:11Z |
| 43563420664 | 237dfd4 | 2026-11-14T05:50:08Z |
| 43562729285 | d3d543f | 2026-11-14T05:41:18Z |
| 43561440913 | 282b855 | 2026-11-14T05:25:13Z |
| 43560713147 | ffe1c2c | 2026-11-14T05:16:15Z |
| 43555481169 | 8542c49 | 2026-11-14T04:12:35Z |
| 43534630085 | 623b9f8 | 2026-11-14T00:30:21Z |
| 43533370038 | 39ad948 | 2026-11-14T00:19:05Z |
| 43532853927 | 2eb7980 | 2026-11-14T00:14:41Z |
| 43521749811 | 0e28308 | 2026-11-13T22:44:41Z |
| 43511106808 | fd14816 | 2026-11-13T21:29:13Z |
| 43501646405 | 6da2208 | 2026-11-13T20:26:56Z |
| 43485443530 | b6dd943 | 2026-11-13T18:43:50Z |
| 43435220717 | 066958a | 2026-11-13T13:37:08Z |
| 43434252094 | e78f667 | 2026-11-13T13:30:37Z |

- 19 件すべてに job は `bundle` が 1 つあり、status は completed、conclusion は success(`gh run view <id> --json jobs`)。
- 進行中の run は 0 件。
- 43608277966 の最新の status は completed、conclusion は success(updatedAt 2026-11-14T15:44:59Z)。

## 3. 保存したハッシュ行

`gh run view <id> --log` の出力のうち `sha256` を含む行だけを `/tmp/opencode/r7-6c-log.txt` に保存し、そこから読んだ(57 行)。`SHA256SUMS` の行は step 7 の `sha256sum *` の出力。`SHA256SUMS` 自体のハッシュ値を run ごとに抜粋する。

| run id | 行の種類 | SHA256SUMS のハッシュ(step 7) |
|---|---|---|
| 43608277966 | step 7 | 84ddd49bf34b6054e8c1df167d48ee4d5d3eac81e7a0ff3d8fa2a4f2f1e7a3b8 |
| 43584338146 | step 7 | 1d8af68b0d170e421b16c4c85d2107b9cd7f50a3979541b1060d09afa4f29af3 |
| 43584013233 | step 7 | 70469d696d4bc6337e0369887362a6d0d406520a58cb206c0d17fdb7d99ab92e |
| 43582706673 | step 7 | 1148957043d996fcaf868496e97e54f43a16ebae11f2930edd38b81b8af17342 |
| 43572936481 | step 7 | 9d8de7ccfcc41536a1448f077463e84eb814e9f4f8cf536ea21683fd598c350b |
| 43563420664 | step 7 | 05a9e77f38df143a4e110f08b6931c1cca4aab9320e050d2d22ca05ef189ddf5 |
| 43562729285 | step 7 | 3e194ea086273bfa4e816b0e08d607e7aac9d5508b1779d41193869a386819e5 |
| 43561440913 | step 7 | fac93620aec69dd89b1b01df5fff4f1ef3301247fba50f86fdedb83ccc50147c |
| 43560713147 | step 7 | 5b86ad02551183acb81141e6ef43e19cd0c2717b9973074ddf5039029aa8f0bf |
| 43555481169 | step 7 | e2b852ed826c5f1edb479cd51d37fca431f6a359e9539fb77ea601e51268e294 |
| 43534630085 | step 7 | 32e51171c460cb302e3060b13a4a8daa031f5be14c1f83421262c09bd6c12e14 |
| 43533370038 | step 7 | 85f937d7607f11bd30c498bff11bf94f685ba84bcf8265cf89d1ed8bfef38067 |
| 43532853927 | step 7 | be216f517fe822c9a6835ea22a31282eb97c22786baff636772f5b505088f6f9 |
| 43521749811 | step 7 | e95eed46cdd3851f73dfd586a341d6d886cac3239f0c247285758f4032ac9482 |
| 43511106808 | step 7 | d0de2f0fc44441b570090f6604f17ca55732fdbb7801f81fd656a6bc2a510a13 |
| 43501646405 | step 7 | 7572c871a6edc29204e7b8d44381bba3fad50e87d4a64e35bb50f7578fcaacc8 |
| 43485443530 | step 7 | f1adbca1e6342e8baa724816d8ab37cc8311f1c04ed0880211e477ffc0e4ac1a |
| 43435220717 | step 7 | 88010189ac393fde8143b5730a0b5787ed32d0792aec5fa45d4a0ec5b1b57b6c |
| 43434252094 | step 7 | 163e92fd15fea8e256de0c87069811ae1c62d5325a50f3dc318668eb967eb87b |

- 各 run のログには、インストーラの照合のステップのコマンド行(`got_sha=$(sha256sum "$INST" ...)`)も含まれる。そのステップの出力値(インストーラの SHA-256 の値)は、`sha256` を含まない行にあり得るため今回の抽出では拾っていない。値は未確認。
- 43608277966 のログの `sha256` を含む行は 3 行(コマンド行 2 と SHA256SUMS の行 1)。

## 4. 未確認

- 33b676a 自体の push に対応する run が無い理由は確かめていない。
- 19 件はこの workflow の一覧に限る。他の workflow・他の headSha の run は見ていない。
- 43608277966 がどの push の commit で起動したかは、取得したデータからは判定していない。
