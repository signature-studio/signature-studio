# 0381: エンジンの bundle を GitHub Actions(deploy-engine)で作って R2 の新しい prefix に載せ、一覧に在る wasm のときだけ root の manifest.json を差し替える
分類: (b 契約の実装)。R2 に bundle を載せる経路と、root を検証済みの bundle に向ける経路が無かったので足す(旧 `engine-bundle.yml` は作るだけで載せなかった)。(c 1 件のデータの差し替え)。root の `manifest.json` 1 件を、一覧に在る bundle に退避つきで向ける。(a の除去)は無し。
撤回条件: 差し替えた後の既定の入口で、実エンジンの停止(About を閉じた後など)が出たら、退避 `backup/manifest-root-<UTC>.json` から root を戻す。この workflow を捨てるのは、9 段が CI で再現せず、修正を重ねても直らないとき(push 6 回の上限で判断)。
日付: 2026-11-16   ROADMAP: R6-4 の「実エンジンの実行が確認できた bundle だけを既定の入口にする」(ROADMAP の行は変えない)   一覧: `ci/verified-bundles.tsv`(1 行)   コミット: この文書・`.github/workflows/deploy-engine.yml`・`ci/verified-bundles.tsv` を含む commit(push で deploy-engine が動く)

## 決定
- 決定 1: 起動は `.github/workflows/deploy-engine.yml`(`on: push`、`main`)。`paths` は旧 `engine-bundle.yml` の入力(lift・app の bundle 部分・ci の入力・抽出の tools)。`ci/lib/**`・`ci/tasks/**`・`ci/agents/**` は含めない。例外は `ci/lib/extract_fl.py` 1 件(インストーラから入力を抽出する段の実行ファイルで、bundle を変える入力)。含めるのは、一覧 `ci/verified-bundles.tsv` と自分自身。`workflow_dispatch`・入力・環境変数の旗は作らない。
- 決定 2: 9 段は `app/scripts/build-engine-bundle.sh` を既定の順で回す。インストーラの照合と抽出、上流 Blend2D は旧 `engine-bundle.yml` と同じ手順。ビルドの段には書き込みの Secrets を渡さない。
- 決定 3: prefix は `fl-engine-ci-<8 桁>`。8 桁は、build の入力の `git ls-files -s` の sha256 の頭 8 桁。同じ入力なら同じ名前になり、再実行で既に載っているかを確かめられる。根拠: 旧 CI の `archive/ci-old/ci/plan.sh` は prefix を段の鍵の末尾 8 桁から作っていた。build-engine-bundle.sh の既存の変数 `PREFIX` で渡す(新しい変数は作らない)。prefix は manifest の URL とキーにだけ入る(`app/scripts/pack-assets.mjs`)ので、wasm の内容は変わらない。
- 決定 4: 載せる。prefix の `manifest.json` が無い(404)ときだけ 5 件を載せる(順は `upload-commands.txt` のとおり。`manifest.json` は最後)。1 件ごとに head と get で大きさ・ContentEncoding・sha256 を確かめ、4 件の確認が済んでから `manifest.json` を載せる。途中で失敗しても `manifest.json` が無いので、次の実行で載せ直せる。`manifest.json` が既に在るときは載せず、4 件の sha256 と、manifest の参照先(sha と大きさ)を比べる。
  - 注(分類: 整備・記録の訂正): 実装(`.github/workflows/deploy-engine.yml` の `verify_row`、l.303)は、prefix の `manifest.json` を get で確かめ(head を省く)。root の `manifest.json` は head と get の両方で確かめる(l.408・l.451〜461・l.462)。決定: ワークフローは変えない(再実行に約 40 分と R2 への書き込みが要るため)。この差は記録として残す。
- 決定 5: 検査は deploy-engine.yml の run の中にインラインで書く(専用の検査スクリプトや追加のワークフローは作らない)。`sha256sum -c`、`.br` の展開と元との一致、manifest の `engine.kind` と参照先の prefix、`app/shared/manifest-guard.ts` の `judgeManifestBytes`(node 24)、`upload-commands.txt` の形(put の 5 件だけ・キーは `fl-web-assets` の prefix の下だけ・削除と同期の指定は無し)。
- 決定 6: root の差し替えは、展開後の wasm の sha256 が一覧(`ci/verified-bundles.tsv` の 1 列目)に在るときだけ。順に、(1) 参照先の 5 件を head で確かめる、(2) root を get し、新しい manifest と同じバイト列なら何もしない、(3) 退避 `backup/manifest-root-<UTC>.json`(head の ContentType・CacheControl・ContentEncoding を写す。get で元と一致を確かめる)、(4) root を書く(ContentType `application/json`、CacheControl `no-cache`、圧縮なし。`app/worker/index.ts` が求める形)、(5) head の大きさと get の sha256 を確かめる。(4)(5) が失敗したら退避を get して root に書き戻し、sha256 で確かめて失敗にする。
- 決定 7: 一覧 `ci/verified-bundles.tsv` の列は 2 つ(wasm の sha256 全 64 桁、根拠の evidence のパス)。一覧は wasm の sha256 だけで bundle を決める。行を足すのは、手元で実行を確かめた記録を commit した後。行を足した commit では deploy-engine が再び走り、同じ prefix は載せ直さず、root を差し替える。
  - 注(分類: 整備・記録の訂正): 一覧の根拠 R5-3l・R5-4 は、R6-2 の条件 1 の 5 項目のうち engineKind・standIn・vfs seed の行を記録していない(`grep -c` で各 0 件)。後追いの記録は `evidence/R6-2e.md`(手元の preview)。この決定の文は変えない。
- 決定 8: Secrets の渡し先。`CLOUDFLARE_API_TOKEN` はアカウント ID を得る段だけ。`R2_ARTIFACT_*`(fl-web-artifacts の読み取り)はインストーラの段だけ。`R2_CACHE_ASSETS_*`(fl-web-assets の読み書き)は、載せる段と root の段だけ。アカウント ID は ::add-mask:: し、エラーの出力はアカウント ID と R2 のホスト名を伏せてから出す。
- 決定 9: AWS CLI の環境変数 `AWS_REQUEST_CHECKSUM_CALCULATION=when_required` と `AWS_RESPONSE_CHECKSUM_VALIDATION=when_required`(R2 が既定の checksum を受けない場合に備える。ツールの設定で、製品の挙動の旗ではない)。
- 決定 10: 結果の表(`if: always()`)に、各段の結果・prefix・wasm の大きさと sha256・載せたか・一覧に在るか・root の sha256 の前後・退避のキー名だけを出す。

## 手元の検証(実 R2 には触れていない)
- YAML の構文: `python3 -c "import yaml; yaml.safe_load(...)"`、通った。
- shellcheck(run の全段): 指摘 0 件。`scripts/scan-forbidden.sh .github/workflows/deploy-engine.yml ci/verified-bundles.tsv`: `0 findings in 2 file(s)`、rc 0。
- 偽の `aws`(ローカルのディレクトリを R2 に見立てる。テストの台本は commit していない)で、workflow の run を取り出して流した。手元の work-r53k の bundle を使い、結果は次のとおり。
  - T1 新規: 5 件を載せ、確かめた件数 5。一覧に在る wasm(`c963715a…`)で root を差し替え、退避を作った。root の sha256 は新しい manifest と一致。
  - T2 再実行: `manifest.json` が在るので載せ直さず、確かめは 4 件の sha256 と manifest の参照先で通る。root は同じバイト列なので何もしない。
  - T3 一覧に無い sha(`000…0`): 載せるところまで行い、root は変えない(`listed=no`)。
  - T4 root の書き込みを 1 回壊す: 大きさの確認で失敗し、退避から戻して、sha256 が元と一致(失敗で終わる)。
  - T5 glue の大きさを壊す: 確かめで止まり、`manifest.json` は載らない。T6 の再実行で載せ直して通り、root を差し替えた。
  - テストは偽の aws の制御の流れだけを確かめる。R2 の ETag・checksum・権限は確かめていない(下の未確認)。

## 未確認
- CI の環境での 9 段の再現。根拠: 旧 `engine-bundle` の run 43687019712(headSha `96107088…`、success)で、CI の wasm の sha256 が手元の work-r53k と全桁一致(`evidence/R5-3l.md` §7)。この workflow の実 run で改めて確かめ、結果は、作成する `evidence/R6-7.md` に書く。
- R2 への put の挙動(aws-cli の既定の checksum、R2 の ETag)。実 run で確かめる。
- 一覧は wasm の sha256 だけを見る。同じ wasm で seed・dll が違う bundle は区別できない。About の停止は、E13 の wasm に別の seed を組んだときに観測された(`docs/decisions/0371-seed-swap-about-close.md` の V1)。現在の行 `c963715a…` の bundle の seed は `7cd2eaecbdba`(`evidence/R5-3l.md`)。
- 公開ホストでの `engine-idle`。Access の配下で、匿名の headless Chrome では開けない。確かめられるのは Actions の出力と R2 の読み直しまで。
- 9 段の所要時間。見込みは 40〜70 分(依頼の見込み)。実測は run のログに残る。

## 出典
- 旧 `engine-bundle.yml`: `git show 7157394^:.github/workflows/engine-bundle.yml`。
- R2 の退避・書き込み・確かめの形: `evidence/R6-4-workflow-draft.yml`(段 1・3・4・5。reviewer は受理。固定値を使わず、ビルドした manifest を使う形に直した)。
- prefix の末尾 8 桁: `archive/ci-old/ci/plan.sh`(`prefix=fl-engine-ci-${KBUNDLE: -8}`)。
- Worker の /assets/ は許可リストを持たない、既定の manifest は圧縮なしを要求する: `app/worker/index.ts`(`judgeDefaultManifest`)。
- 同じ wasm の 9 段の再現と検証: `evidence/R5-3l.md` §7、`evidence/R5-4.md`。
- 形式: `docs/decisions/0379-deploy-worker-workflow.md`。
