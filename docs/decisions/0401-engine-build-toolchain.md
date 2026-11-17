# 0401: エンジン bundle の作成を fe-build の publish 1 つにまとめ、Worker の配信は wrangler deploy 1 回にする。設定はツリーに置く
分類: 整備(除去 a を伴う。設定の生成・版の戻し・確かめの段と、シェル・Python のつなぎを消す)。出力が同じことは sha256 で確かめる。
撤回条件: (1) 依頼者が、Worker の設定をツリーに置くことを禁じたとき(§4 (i))。(2) publish の最終 engine wasm の sha256 が 0286bdf1827f…(基準。evidence/R6-23.md §3)と一致しないとき。(3) 依頼者が、fe-build の内部の段を迂回と判定したとき(§5)。
改訂(2026-11-16): 依頼者の発言により、83ae203 の 0401 の下書きと fe-build の subcommand(toolchain・publish・worker-config・version・check-live)を置き換えた。

## 0 旧版から消した主張(理由つき)
- 下書きの worker-config・version・check-live と、実行時の設定の生成(app/wrangler.deploy.json): 依頼者「デプロイ用の wrangler.json などというものは存在せず、全てのデプロイに必要な記述はツリーに含めることができます」(本日)。失態 0016。
- 版の upload → 100% → 確かめ → 戻し: 依頼者「過度な異常系の考慮は全く不要」(本日)。Worker の配信は wrangler deploy 1 回。
- S1 の R2 の抽出キャッシュ・S6 の段キャッシュ(ci/extract-cache.sh・.github/actions・stage-cache・stage-keys): 依頼者「不要かもしれません」(本日)。runs-on/cache の件は §4 (ii) で未決。
- publish 内の root の退避と戻し: 依頼者「過度な異常系」(本日)。root の差し替え自体は残す(main の指示)。
- 旧の「残す根拠」(msg_1e28f3a20001GYppSYhLxztoMh: R2 の入力と抽出キャッシュ): 抽出キャッシュを外すため撤回。残すのは R2 からの入力の取得だけ。
- 旧 §3 の timeout(40 → 120)・paths の追加: 下書きの値(§7)として残す。根拠は基準の段の合計(evidence §2)と、bundle の入力(lift・ci)。

## 1 候補と判定
A(採る): fe-build(lift/ の workspace の bin と lib)。fe-pe・fe-lift・fe-wasmgen は lib の関数で呼ぶ。emcc・wasm-opt・wasm-merge・aws だけ外部プロセス。B(CMake・Ninja)却下: CMake 自身がスクリプト。C(fe-wasmgen で emcc を外す)長期の別の行。出力が変わる。D・E(現状の bash・Python、Makefile・npm scripts)却下: 依頼者の禁止。
決定: A。根拠: (1) 4 クレートは lib の公開 API を持つ(`fe_pe::rebase::rebase`・`fe_lift::generate_full`・`fe_wasmgen::compile`)。(2) 基準 1,116 秒の 96% は emcc・fe-wasmgen・wasm-opt の計算(evidence §2)。(3) 依頼者は lift を Rust で作ると決めた(msg_15034d596001HdCrYgl5mxPtfQ)。

## 2 段の順序(evidence §5 の改訂)
S0 基準 → S1 入力(R2 の成果物を取って展開。キャッシュは持たない)→ S2 gen・resplit → S3 .o → S4 hybn0〜hybrid → S5 seed・bundle → S6 publish の R2 載せ・root → S7 旧の削除(app/scripts/・lift/harness の shell と Python のつなぎ、ci/stage-*・extract-cache・actions)。ci/agents と agent.yml は本決定の対象外(依頼者の「不要」の指示は別の作業。ワークツリーに別の作業の未コミットの削除がある)。最終の engine wasm・glue・seed・DLL pack の sha256 を基準と一致させる(manifest は bundle.created を除いて比べる。.br は展開して比べる。詳細は evidence §3)。

## 3 Worker の配信
deploy.yml は cloudflare/wrangler-action で `command: deploy`、`workingDirectory: app` の 1 回だけ。設定は `app/wrangler.jsonc`(ツリー。決定 0403 のとおり、account_id・bucket 名・Access の値を含む)。with に accountId は書かない(0403)。engine の R2 の書き込みの宛先(アカウント ID)は、現行どおり API で 1 つ得る。0403 に合わせ、ツリーの account_id から読む方へ寄せるかは未着手。

## 4 食い違いと未決
- (i) AGENTS §8(識別子をコミットに書かない)と依頼者の発言(ツリーに含めてよい)。決定 0403 が、依頼者の指示により app/wrangler.jsonc の 1 ファイルだけを例外とした(account_id・bucket 名・Access の値)。0401 はこれに従う。例外の範囲をほかのファイルへ広げないこと(0403 の scan 例外は 1 行のみ)。
- (ii) runs-on/cache。依頼者「なぜ runs-on/cache を採用しないのか理解に苦しみます」(本日)。2026-11-04 にも指示(msg_1a2ccc2c3001RSHoExRGznB5pH)。しかし 0400 の公開 action の限定(checkout・setup-node・wrangler-action)と食い違う。本決定は段キャッシュを持たない。採否は未決(main)。
- (iii) 過度な異常系: root の差し替えに退避・戻しが無い。差し替えが途中で失敗した場合の戻しは持たない(未確認の危険。依頼者の指示による)。

## 5 リスク: fe-build が内部で段を持つことは、依頼者の禁じた迂回か
- 当たらないと読む根拠: 禁じられたのは、deploy.yml の直書きの長い run、シェルスクリプトの実行、旧スクリプトを呼ぶラッパー(0400 の制約)。fe-build は旧スクリプトを呼ばず、段の計算は Rust の関数で持つ。[読]
- 当たる恐れ: 依頼者が「wasm のビルドは 1 コマンド」と期待するなら、内部の段の数が、制限を避けるための別の制御に見える可能性がある。判定は依頼者の指示でしか決まらない(未確認)。
- 対処: publish の段を関数として 1 つの Rust binary に収め、deploy.yml からは 1 行で呼ぶ。迂回と判定されたら、撤回条件 (3) で本決定を撤回する。

## 6 捨てる物と根拠
- build-engine-bundle.sh・Python のつなぎ・stage-keys.sh の関数本文のハッシュ: 依頼文「スクリプト禁止」と msg_1b7bb7b92001i7T2kIeDM1AT7O(100 行近くの記述は間違い)。
- 「あるから飛ばす」10 箇所: 古い出力を使う危険。決定 0395 の内容の鍵で代える。出典: evidence §4。
- 設定の生成・版の戻し・R2 の段キャッシュ: §0 の依頼者の発言(本日)。
- RT_O2_FILES(未使用)・systemd の注記(実装に無い): 出力に影響しない。

## 7 deploy.yml の下書き(0400 の制約: 100 行・80 文字・直接の run・公開 action は 3 つ。Secrets の名前は既存のもの)
```yaml
name: deploy
on:
  push: {branches: [main], paths: [app/**, lift/**, ci/**, .github/**]}
permissions: {contents: read}
concurrency: {group: deploy, cancel-in-progress: false}
env: {WRANGLER_SEND_METRICS: "false"}
jobs:
  deploy:
    runs-on: ubuntu-24.04
    timeout-minutes: 120
    steps:
    - {uses: actions/checkout@v4, with: {persist-credentials: false}}
    - {uses: actions/setup-node@v4, with: {node-version: "24"}}
    - run: npm ci --prefix app --ignore-scripts --no-audit --no-fund
    - run: npm run build --prefix app && npm test --prefix app
    - run: npm run test:worker --prefix app
    - working-directory: lift
      run: rustup show && cargo run --release --locked -p fe-build -- toolchain
    - working-directory: lift
      env:
        CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
        ASSETS_ID: ${{ secrets.R2_CACHE_ASSETS_ACCESS_KEY_ID }}
        ASSETS_SECRET: ${{ secrets.R2_CACHE_ASSETS_SECRET_ACCESS_KEY }}
        ARTIFACT_ID: ${{ secrets.R2_ARTIFACT_ACCESS_KEY_ID }}
        ARTIFACT_SECRET: ${{ secrets.R2_ARTIFACT_SECRET_ACCESS_KEY }}
      run: cargo run --release --locked -p fe-build -- publish
    - uses: cloudflare/wrangler-action@953926a2e2182532811c01a25e53647d93bf07c0
      with:
        apiToken: ${{ secrets.CLOUDFLARE_API_TOKEN }}
        workingDirectory: app
        command: deploy
```
