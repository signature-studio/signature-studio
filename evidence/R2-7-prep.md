1. 実エンジン: 実行なし(ファイルと git の読み取り、`gh run list` の読み取り 1 回のみ。`boot-bundle.mjs`・headless Chrome・9 段のビルドは実行していない)
2. 結果: work-r25(`4ba7ce1`)は、R2-1〜R2-6 の取り込み commit を全部祖先に持つ(表 c)。HEAD(`0d778ae` の時点)は work-r25 より 41 commit 先で、lift・app・ci を変えたのは `33b676a`(R7-6)の 1 件だけ。R2-7 の定義の範囲では、新しい bundle を作る必要は無い。HEAD の bundle は GitHub に上がっていない(engine-bundle.yml に upload の段は無い)ので、HEAD を測るなら手元で 9 段を回すしかない。
3. 判定は書かない: 基準 +100 ms の超過・不超過は、この記録では書かない(測定は未実施)。
4. ※ 完了ではない: P2 の 120 fps は未確認、P3 は音切れが残る(`evidence/R4-3.md:2`)、O9 は未特定、所定プロジェクトを開くと 8 件のプラグインが開けない(`evidence/R4-2b.md:1`)、依頼者機の値は 0 件、reviewer の最終検査は未了。この記録は R2-7 の完了ではない。

## 開始時(読み取り)
- `git status --short`(作業開始時): ` M docs/ROADMAP.md`、` M docs/decisions/0364-remove-unreachable-dpi-branch.md`、` M evidence/R2-7b.md`、`?? evidence/R0-6-failfast.txt`、`?? evidence/R7-6b-grep.txt`、`?? evidence/R7-6b-named-tests.txt`。自分の作業と無関係の物は触っていない(他の worker の変更の一覧。パスのみ)。
- `git log --oneline -n 5`(作業開始時): `0d778ae` R2-7e 文面の直し、`6d8779f` R5-2 文面の直し、`febe237` R2-7c 文面の直し、`4e30975` R2-7h 文面の直し、`a797ab5` ROADMAP R2-5 の注。
- `ls docs/decisions` の最大番号: 0364。次は 0365。

## (a) R2-1〜R2-6 の取り込み commit(出典つき)
| 段 | commit | 内容 | 出典 |
|---|---|---|---|
| R2-1 | `46e2d1a` | L08(診断の文字列を describe 時に作る) | `evidence/R2-1.md:38` |
| R2-1 の木に入った別の commit | `8466e21` | `fle_input.h` の `FLE_HTERROR` の重複行の削除(値は同じ) | `evidence/R2-1.md:7` |
| R2-2 1 本目 | `7780300` | ScrollWindowEx のモデル | `evidence/R2-2.md` の表 1 行目 |
| R2-2 2 本目 | `2eb445d` | ScrollDC のモデル | `evidence/R2-2.md` の表 2 行目 |
| R2-2 3 本目 | `5cc3fbd` | DCX と LockWindowUpdate | `evidence/R2-2.md` の表 3 行目 |
| R2-2 4 本目 | `1651842` | ImageList のドラッグ | `evidence/R2-2.md` の表 4 行目 |
| R2-2 5 本目 | `2ad0f8a` | 子孫窓の描画 | `evidence/R2-2.md` の表 5 行目 |
| R2-3 | `6926320` | L14(右クリック・仮想ポインタ。delta は当てない) | `git log`(`lift/runtime/fle_win.c` ほか 3 files) |
| R2-3c | `527cb67` | delta-pointer | `evidence/R2-3c.md:2` |
| R2-6 | `6d95158` | L07(読む側の無い FLE_SCREEN_REMAXIMIZE の送る行の除去) | `evidence/R2-6.md:2` |
| R2-6 の追記 | `2e5e0c9` | `fle_win.c` のコメントから名前と経緯を除く(1 行の差し替え) | `evidence/R2-6.md:84`・`git log` |
| R2-5 | (取り込み commit は無い) | blocked。`docs/ROADMAP.md` の R2-5 の行。R2-6 は R2-5 を待たない(R2-6 の行) | `docs/ROADMAP.md` |
| 参考(R7-2) | `623b9f8` | FLE_CALL_FRUITY の getenv を FLE_TEST_GETENV に替える(製品では読まない) | `git log`(R7-2 の commit) |

## (b) 手元の bundle の manifest(`guilt_read` で読んだ。読みのみ)
| bundle | manifest の description の HEAD | created | wasm の sha256(頭 16 桁) | glue の sha256(頭 12 桁) |
|---|---|---|---|---|
| base(`~/.cache/signature-studio/work/bundle/fl-Oz`) | `818b5b0` | 2026-11-13T16:35:11.303Z | `0286bdf1827f5930` | `07e7646f280b` |
| work-r25(`~/.cache/signature-studio/work-r25/bundle/fl-Oz`) | `4ba7ce1` | 2026-11-14T10:44:22.156Z | `921bb7e2ed1ad326` | `b063ddc0ad2f` |
| work-r24(`~/.cache/signature-studio/work-r24/bundle/fl-Oz`) | `f1635bf` | 2026-11-14T08:18:48.162Z | `619954341286f4a8` | `b063ddc0ad2f` |

- base の HEAD は、0351 の未確認の節が書く `d0447902` と違う(`docs/decisions/0351-engine-bundle-workflow.md` の 49 行目。理由は未確認)。
- 3 つの bundle の wasm は、どれも同じではない(sha の頭 16 桁が 3 種)。

## (c) 取り込み commit が work-r25 の HEAD `4ba7ce1` の祖先か
コマンド: `git merge-base --is-ancestor <commit> 4ba7ce1; echo rc=$?`(2026-11-14 に実行)

| commit | 内容 | rc(0 なら祖先) |
|---|---|---|
| `46e2d1a` | R2-1 L08 | 0 |
| `8466e21` | FLE_HTERROR の重複 | 0 |
| `623b9f8` | R7-2 FLE_TEST_GETENV | 0 |
| `7780300` | R2-2 1 本目 | 0 |
| `2eb445d` | R2-2 2 本目 | 0 |
| `5cc3fbd` | R2-2 3 本目 | 0 |
| `1651842` | R2-2 4 本目 | 0 |
| `2ad0f8a` | R2-2 5 本目 | 0 |
| `6926320` | R2-3 L14 | 0 |
| `527cb67` | R2-3c delta-pointer | 0 |
| `6d95158` | R2-6 L07(app と lift) | 0 |
| `2e5e0c9` | R2-6 の追記(コメント) | 0 |
| `33b676a` | R7-6(HEAD 側。work-r25 には無い) | 1 |

同じ確認で、`818b5b0`(base の HEAD)と `f1635bf`(work-r24 の HEAD)は `4ba7ce1` の祖先(rc=0 と rc=0)。

HEAD との差(`git log --oneline 4ba7ce1..HEAD -- lift app ci` の出力):
```
33b676a R7-6: 到達しない DPI 分岐の除去(分類 a。fle_proc.c の三項演算子と FLE_PROCESS_PER_MONITOR_DPI_AWARE。応答は 0 のまま。決定 0364。ROADMAP の状態は変えない)
```
`git log --oneline 4ba7ce1..HEAD | wc -l` の結果は 41(docs・evidence を含む全 commit)。`33b676a` の lift・app の変更は `lift/runtime/fle_appfile.c`・`fle_proc.c`・`fle_proc.h` の 3 files(2 insertions(+)、3 deletions(-)。`git show --stat 33b676a` の lift 側の行。同じ commit の決定文書の 36 行は除く)。

## (d) 結論(事実の文)
- work-r25 は、R2-1〜R2-6 の取り込み commit を全部含む(表 c の rc 0)。ROADMAP の R2-7 の行の「R2-1〜R2-6 を入れた bundle」は、work-r25 がその定義に当たる。R2-5 は blocked で入っていない(R2-5 の行)。
- HEAD は work-r25 より 41 commit 先。lift・app・ci を変えたのは `33b676a` の 1 件。
- `git grep -n FLE_SCREEN_DPI 4ba7ce1 -- lift/runtime` は、`fle_appfile.c:239`・`fle_proc.c:408` 等に一致する(R2-5 の取り込みが work-r25 に無い事実と整合。読みで、getenv の経路は未確認)。
- GitHub の engine-bundle の run は、headSha `659ff0e`(`33b676a` の子孫)で `completed` / `success`(`gh run list --workflow engine-bundle.yml --limit 10` の出力。createdAt 2026-11-14T15:05:31Z)。この run の bundle の sha は取っていない(未確認)。`.github/workflows/engine-bundle.yml` に upload の段は無い(`grep -n upload` の rc 1)ので、bundle は GitHub から取れない。
- 新しい bundle の要否: R2-7 の定義の範囲では不要(work-r25 で足りる)。HEAD を測るなら必要。その場合の手順は `evidence/R0-5.md` の 9 段の手順(`PATCH="$PWD/lift/harness/b2d1/b2d3-compop.patch" bash lift/harness/b2d1/build_upstream.sh` を先に、続いて `WORK=<既存の bundle と別の名前の場所> HOST_ROOT=<ワークツリー>/artifacts/host B2D_UPSTREAM_LIB=<上流の lib> B2D_UPSTREAM_SRC=<上流の src> bash app/scripts/build-engine-bundle.sh`)。この依頼では実行しない。`evidence/R0-5.md:42`・`:63`〜`:69` が出典。

## 未確認
- R2-7 の定義の「R2-1〜R2-6」に、どの commit を数えるかの最終の決め方(R2-2 の 5 本と R2-3 の 1 本の数え方は、ROADMAP の行の文だけから読んだ)。
- `4ba7ce1` の `lift`・`app`・`ci` の中身と、work-r25 の wasm の一致(ビルドの再現は確かめていない。manifest の description の読みのみ)。
- `33b676a` が GitHub の run でどう扱われたか(run の conclusion だけ読んだ。bundle の sha は未取得)。
- work-r25 の wasm と `4ba7ce1` の木の一致(再ビルドしていない)。
- base の manifest の HEAD(818b5b0)と 0351 の `d0447902` の食い違いの原因。

## 変えたファイル
- `docs/decisions/0365-r2-7-baseline-definition.md`(新規)
- `evidence/R2-7-prep.md`(新規)
- ROADMAP・R2-7 の行・他の決定文書は変えていない。

## 出典
`docs/ROADMAP.md`(R2-1〜R2-7、R7-2 の行)、`docs/decisions/README.md`、`docs/decisions/0351-engine-bundle-workflow.md`、`docs/decisions/0364-remove-unreachable-dpi-branch.md`(存在の確認のみ)、`evidence/R0-5.md`、`evidence/R2-1.md`、`evidence/R2-2.md`、`evidence/R2-3c.md`、`evidence/R2-6.md`、`evidence/R2-7a.md`、`evidence/R4-2b.md`、`evidence/R4-3.md`、`.github/workflows/engine-bundle.yml`、`git log`・`git merge-base`・`git grep`・`gh run list`(いずれも読み取り)、`~/.cache/signature-studio/{work,work-r24,work-r25}/bundle/fl-Oz/manifest.json`(読みのみ)。
