# R1 の記録: engine-bundle の実行 43485443530(GitHub の実行の読み取り)

- 実エンジン: 実行なし(GitHub の実行の読み取り。ビルドは手元で走らせていない)
- 実行 43485443530(headSha b6dd94378e6c4882acec558387c13c6df1548cf9、event push)は status completed・conclusion success。所要 1725 s(createdAt 18:43:50Z → updatedAt 19:12:35Z。28 分 45 秒)。ジョブ bundle は 1723 s(startedAt 18:43:52Z → completedAt 19:12:35Z)。
- 9 段の rc=0: 9 段全体で 1 回(ログの「9 段: 成功(rc=0)」行)。個別の rc が確認できた段は 6 段(gen・wasm・hybn0・weblink・wasmopt・hybrid。32 行の `stage-command` が全て rc=0)。resplit・seed・bundle の 3 段は個別の rc がログに無い(§3・§6)。
- wasm の SHA-256(ログの値): `7bf56507c6880f7422be07d821a1d3d3c904e8c55559516adb428e59ec998197`(engine.7bf56507c688.wasm)。大きさは未確認(段 7 は大きさを出さない)。
- 未確認: 成果物の中身と起動(手元の R1-5 の担当)。

## 1 実行の一覧

コマンド: `gh run list --workflow engine-bundle.yml --limit 5 --json databaseId,status,conclusion,headSha,createdAt,updatedAt,event`

```
[{"conclusion":"success","createdAt":"2026-11-13T18:43:50Z","databaseId":43485443530,"event":"push","headSha":"b6dd94378e6c4882acec558387c13c6df1548cf9","status":"completed","updatedAt":"2026-11-13T19:12:35Z"},{"conclusion":"success","createdAt":"2026-11-13T13:37:08Z","databaseId":43435220717,"event":"push","headSha":"066958a4f566dd87144c33555a731d82e17d83ec","status":"completed","updatedAt":"2026-11-13T14:10:15Z"},{"conclusion":"success","createdAt":"2026-11-13T13:30:37Z","databaseId":43434252094,"event":"push","headSha":"e78f667e99688d515cee2d625cfea6e0630612b5","status":"completed","updatedAt":"2026-11-13T14:13:25Z"}]
```

## 2 実行の詳細と所要

コマンド: `gh run view 43485443530 --json status,conclusion,headSha,createdAt,updatedAt,jobs --jq '{status,conclusion,headSha,createdAt,updatedAt,jobs:[.jobs[]|{name,conclusion,startedAt,completedAt}]}'`

```
{"conclusion":"success","createdAt":"2026-11-13T18:43:50Z","headSha":"b6dd94378e6c4882acec558387c13c6df1548cf9","jobs":[{"completedAt":"2026-11-13T19:12:35Z","conclusion":"success","name":"bundle","startedAt":"2026-11-13T18:43:52Z"}],"status":"completed","updatedAt":"2026-11-13T19:12:35Z"}
```

- 実行の所要: 19:12:35Z − 18:43:50Z = 1725 s
- ジョブ bundle: 19:12:35Z − 18:43:52Z = 1723 s

## 3 段ごとの結果

ログ: `gh run view 43485443530 --log > $TMPDIR/r15-eb.log`(rc=0、424 行。commit していない。§7 で消す)

段の時間は、ログの各行のタイムスタンプの差(秒)。

| 段 | 開始 | 終了 | 秒 | 結果(ログの行) |
|---|---|---|---|---|
| 2 アカウント ID | 18:43:57 | 18:43:57 | 0 | 実行時に 1 つ得た(値は出していない。行 136) |
| 3 インストーラの取得と照合 | 18:43:57 | 18:44:12 | 15 | 照合: 一致(大きさ 1106033456 B、SHA-256 一致)(行 165) |
| 4 抽出 | 18:44:12 | 18:44:13 | 1 | 抽出: 成功(rc=0)(行 182) |
| 5 Python・Node・emsdk・rust・cargo fetch | 18:44:13 | 18:44:30 | 17 | 各ステップ成功(行 194〜296) |
| 5 上流 Blend2D | 18:44:30 | 18:45:03 | 33 | 上流 Blend2D: 成功(rc=0)(行 318) |
| 6 9 段 | 18:45:03 | 19:12:31 | 1648 | 9 段: 成功(rc=0)(行 344) |
| 7 SHA-256 | 19:12:31 | 19:12:31 | 0 | バンドルのファイル数: 11(行 393) |
| Post・Complete | 19:12:31 | 19:12:32 | 1 | 成功 |

段 6 の内訳(ログの `stage-command` 行 345〜376。32 行、全て rc=0。`app/scripts/build-engine-bundle.sh` の `capped` が出す行):

| 段(DEFAULT_STAGES の順) | capped コマンドの wall(秒) | 小計 |
|---|---|---|
| gen | gen 25 | 25 |
| resplit | (capped なし。時間の行が無い) | — |
| wasm | rt 15・harness 1・units 433 | 449 |
| hybn0 | fle 82・dsp 93・b2d 1・qfc 5・ft 1・ilt 11・oggio 4・elastique 11 | 208 |
| weblink | weblink-Oz 14 | 14 |
| wasmopt | wasmopt-Oz 335 | 335 |
| hybrid | gen・opt の 18 行(fle 80+6、dsp 91+1、b2d 0+1、qfc 4+1、ft 1+1、ilt 10+0、oggio 3+1、elastique 11+7、merge1 1、merge2 12) | 231 |
| seed | (capped なし) | — |
| bundle | (capped なし) | — |

- 合計: 1262 s(32 行の wall を awk で足した値)。段 6 の 1648 s との差 386 s は未確認。capped の無い段の時間と、段の間の時間が入るが、標準出力は `$RUNNER_TEMP/build.txt` に入りログに出ないため、内訳は確かめられない。

段 7 の出力(SHA-256 のみ。ログ 394〜404 行。中身と大きさは出していない):

```
f1adbca1e6342e8baa724816d8ab37cc8311f1c04ed0880211e477ffc0e4ac1a  SHA256SUMS
2f494419c1adec95aad70e0beed730a6ed5e9cba695a7b72efca3cdb18fa7d39  dll.2f494419c1ad.flpack
3a9645ab01725069bd080cb1f93247e07cf64463cf89c998da1d806584dc3843  dll.2f494419c1ad.flpack.br
07e7646f280b1b7963c74457eacc7ef050c47682cc0911b373c5ae5575cfd074  engine-glue.07e7646f280b.js
e112e9581bc46e8759649d7b4a58ab22662a721e71db6c3a19658b44dad4478b  engine-glue.07e7646f280b.js.br
7bf56507c6880f7422be07d821a1d3d3c904e8c55559516adb428e59ec998197  engine.7bf56507c688.wasm
ad42f9693e5264d5040478eddcfe12b488d1191b7dcd51501c28bc29889dbb8a  engine.7bf56507c688.wasm.br
5810797395c198ef2c57d15e40ab57fdae658499dc50bf5e21765691a66d4e74  manifest.json
7cd2eaecbdba9f712f7366a76280d2099363aeacf7a2aea7637f3caf238247f6  seed.7cd2eaecbdba.flpack
918eeca6b85ce8467ce8568f7df773beec9eb0db51117a2be4fce27e81b87f82  seed.7cd2eaecbdba.flpack.br
34920409430e0b2db0989f9cba80268442dffc54e46a25698d2eb8dc95116fec  upload-commands.txt
```

## 4 手元の bundle・前回の実行・今回の実行の並置(事実の記録。一致は要求しない)

| 項目 | 今回 43485443530(headSha b6dd943) | 前回 43434252094(headSha e78f667) | 手元 E13(9 ファイル) |
|---|---|---|---|
| wasm | `7bf56507c688`(大きさ未確認) | `0286bdf1827f`(`evidence/R8-4.md` 128 行) | `58ad4a6aa720`(143,087,706 B) |
| wasm(手元 R0-5、取り込み前の木) | — | 手元 R0-5 は `0286bdf1827f`・143,087,622 B(`evidence/R0-5.md` 178 行) | — |
| wasm.br | `ad42f969…` | `45f79be9…` | `5a0e32cb…` |
| manifest.json | `58107973…` | `64fb82b2…` | `57817472…` |
| dll.flpack | `2f494419…`(前回・E13 と一致) | 同左 | 同左 |
| engine-glue.js | `07e7646f…`(前回・E13 と一致) | 同左 | 同左 |
| seed.flpack | `7cd2eaec…`(前回と一致) | 同左 | `f7726113…` |

- 今回の wasm は、前回と手元 R0-5 のどれとも違う。前回(e78f667)と今回(b6dd943)の間は 88 commit（出典: `git rev-list --count e78f667e99688d515cee2d625cfea6e0630612b5..b6dd94378e6c4882acec558387c13c6df1548cf9` の出力 88）で、そのうち `app/`・`ci/`・`lift/` を変えたのは 32 ファイル(`git diff --stat e78f667 b6dd943 -- lift app ci` の出力: 32 files changed, 1830 insertions(+), 108 deletions(-))。wasm の差がこの変更による物かは未確認(commit ごとの比較はしていない)。
- 手元 R0-5 の bundle は取り込み前の木。前回の実行も取り込み前(e78f667)。今回の実行は取り込み後の木(L01〜L04 を含む)。

## 5 伏せ検査(ログに対して実施。ログは commit していない)

| 検査 | コマンド | 結果 |
|---|---|---|
| アカウント ID(32 桁の単独) | `grep -c -E '\b[0-9a-f]{32}\b'` | 0 件 |
| 32 桁を含む行 | `grep -c -E '[0-9a-f]{32}'` | 28 件。うち 11 件は段 7 の SHA-256(64 桁)の行。残り 17 件は 40 桁の識別子(checkout の commit、action の取得先の commit、emsdk の版の識別子。いずれも公開情報)。40 桁以外で 32 桁以上の並びは 0 件(`grep -v -E '\b[0-9a-f]{40}\b'` の後の件数 0)。32 桁の単独は無いので、アカウント ID の当たりではない |
| メール形式 | `grep -c -E '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}'` | 0 件 |
| R2 のホスト名(解決後の形) | ログ内の、ホスト名の形の検査（語は記録しない） | ホスト名の形: 3 件。変数参照と API の URL の形で、解決した値ではない(語の列挙は省略) |
| トークン・鍵の語 | `grep -c -i -E 'token|secret|bearer|AKIA'` | 7 件。すべて値が `***` で伏せられた行(行 39・134・163・187・204 など)かヘッダ語(行 22・26)。生の値は 0 件 |

伏せ検査で当たった値は無いので、`***` に置き換えた値は無い。

## 6 未確認

- 成果物の中身と起動(手元の R1-5 の担当。この記録の対象外)。
- GitHub 側の wasm の大きさ(段 7 は大きさを出さない。artifact は上げていない)。
- 段 6 の 1648 s と内訳の合計 1262 s の差 386 s。
- 前回(段 6 が 2452 s)と今回(段 6 が 1648 s)の差の原因。ワークフローに cache の段は無い(`.github/workflows/engine-bundle.yml` の 1〜156 行を読んだ)が、速くなった理由は未確認。
- 段 5 の上流 Blend2D の時間は、今回 33 s・前回 55 s。差の原因は未確認。
- resplit・seed・bundle の 3 段は capped で囲まれず、個別の rc と時間がログに出ていない。9 段全体の rc=0 だけが確かめられた。
- 今回の wasm と前回の wasm が違う原因(§4)。

## 7 コマンドと後始末

- `gh run list --workflow engine-bundle.yml --limit 5 --json ...`(§1)
- `gh run view 43485443530 --json ... --jq ...`(§2)
- `gh run view 43485443530 --log > $TMPDIR/r15-eb.log; echo rc=$?; wc -l`(rc=0、424 行)
- `grep -c 'stage-command'` = 32、`grep 'rc=[1-9]'` = 0 件、`grep -o 'wall=[0-9]+s'` の合計 = 1262 s
- 後始末: `$TMPDIR/r15-eb.log` は、`ls -la` の後に削除する。
