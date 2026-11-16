# 0404: 抽出の層のキャッシュは R2 に置く。実装は runs-on/cache(S3 互換の R2 を endpoint にする)。0402 の「キャッシュをやめる」を取り消す
分類: (b) 依頼者の指示の実装(キャッシュは GitHub Actions Cache ではなく R2 に置く)。(c) 通信の削減(R2 から取る 1.1 GB のインストーラを、キャッシュが有るときは取らない)。(a) 除去は extract-layer と ci/extract-cache.sh の削除(キャッシュの本体は runs-on/cache の 1 step に替える)
撤回条件: (1) runs-on/cache が GitHub のホストランナーで R2 の endpoint を使えないと、run のログで確かめられたとき(その事実を main に返し、次の策は main が決める)。(2) 依頼者が R2 へのキャッシュを求めなくなったとき
日付: 2026-11-16   ROADMAP: R9-1   一覧: —   コミット: 本決定の commit(後続の commit で記録)

## 依頼者の発言(原文。main の転送と、evidence/R6-9.md の記録から)
- msg_1a2ccc2c3001RSHoExRGznB5pH(2026-11-04): 「インストーラーからの展開や抽出に時間がかかる場合、それらをキャッシュできるようにします。キャッシュは、GitHub Actions Cacheではなくキャッシュ用R2に保持します。そのため、actions/cacheではなくruns-on/cacheを使用します。」
- msg_1e28f3a20001GYppSYhLxztoMh(2026-11-16 13:13:19): 「ワークフロー側のエージェントはR2からアーティファクトを読み取り、実際のインストーラーから抽出を行い、抽出物はR2にキャッシュする、前任者と同じことをやっているのですよね？」
- 2026-11-16 の依頼者の指摘(main の転送): 「deploy.yml に、アーティファクトのダウンロード、R2 へのキャッシュがありませんが」。

## 決定
- 決定: 抽出物のキャッシュは R2(バケット fl-web-cache。鍵は R2_CACHE_ASSETS_*)に置き、runs-on/cache@v5 で取り出す・保存する。根拠: 依頼者の発言(上)。
- 決定(取り消し): 0402 の「抽出のキャッシュをやめる」は取り消す。根拠: 0402 は、時間が減らないことを理由に、依頼者の発言を確かめずに機能を捨てた(誤り)。キャッシュの効果は時間ではなく通信(取得 1.1 GB を省く)。
- 決定: endpoint のアカウント ID は、実行時に API(CLOUDFLARE_API_TOKEN)で 1 つ得て、GITHUB_ENV に入れ、伏せる。リポジトリには書かない。根拠: アカウント ID は scan-forbidden の cfhex 規則と AGENTS.md §8(コミットに Cloudflare の識別子を書かない)に当たる。依頼者の指示の「app/wrangler.jsonc の account_id を env に置く」は、この理由で採らない。
- 決定: runs-on/cache の環境変数は、RUNS_ON_S3_BUCKET_CACHE=fl-web-cache、RUNS_ON_S3_BUCKET_ENDPOINT=https://<アカウント>.r2.cloudflarestorage.com、RUNS_ON_S3_FORCE_PATH_STYLE=true、AWS_REGION=auto、AWS_ACCESS_KEY_ID・AWS_SECRET_ACCESS_KEY(R2_CACHE_ASSETS_*)。根拠: 読んだ dist/restore/index.js の実装(下の実測)。
- 決定: 抽出は、キャッシュが無いときだけ。インストーラの取得(R2_ARTIFACT_*)と抽出(ci/lib/extract_fl.py)は run の直書き。根拠: 依頼者の指示(直書き)。

## 実測(数字は read と gh api の出典)
- runs-on/cache の dist/restore/index.js(gh api の raw。4,266,321 B)。`RUNS_ON_S3_BUCKET_CACHE` が設定されていると `canSaveToS3` が真になり、S3 の経路を使う(ランナーの種類の判定はない)。`RUNS_ON_RUNNER_NAME` が非空のときだけ AWS の資格情報の環境変数を消す(ホストランナーでは RUNS_ON_RUNNER_NAME が空)。リージョンは `RUNS_ON_AWS_REGION` → `AWS_REGION` → `AWS_DEFAULT_REGION` の順。
- 抽出の層(evidence/R6-8.md の 3.1 節): 保存 39 秒(run 43804191943)、復元 9 秒(run 43806250386)。取得と抽出は 9〜12 秒(同 49 行)。インストーラは 1.1 GB(同 76 行)。tar 87,121,920 B、232 件(同 70 行)。
- 時間は減らない(9 秒の復元 対 9〜12 秒の取得と抽出)。減るのは取得の通信 1.1 GB(同 76 行)。
- runs-on/cache の README の記述: S3 の経路は RunsOn のランナー向けと書かれている。GitHub のホストランナーで使えることは、この決定の run で確かめる(未確認は下)。

## 未確認
- GitHub のホストランナーで、RUNS_ON_S3_BUCKET_ENDPOINT の R2 に保存・復元できること(この決定の commit の run で確かめる)。
- 1 回目の run の保存と、2 回目の run の復元(hit の語と秒)。
- runs-on/cache@v5 の版(ブランチ v5 を使う。main の値のまま)。

## 出典
- 依頼者の発言: msg_1a2ccc2c3001RSHoExRGznB5pH、msg_1e28f3a20001GYppSYhLxztoMh。evidence/R6-9.md(C2・C3)。
- 決定 0386(抽出のキャッシュ。R2 の鍵の渡し先)、0402(取り消し)。evidence/R6-8.md(数字)。
