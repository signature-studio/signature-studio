# 0401: エンジン bundle の作成を Cargo の fe-build に作り直し、deploy.yml は cargo の 1 行で呼ぶ(0400 の「置かない」の bundle・R2・root を入れる)
分類: 整備(a の除去を伴う。シェル・Python のつなぎを消す)。出力が同じことは sha256 で確かめる。
撤回条件: (1) 同じ commit の木で作った最終 engine wasm の sha256 が 0286bdf1827f…(基準。evidence/R6-23.md §3)と一致しない。(2) 同じ入力の bundle の段が現行の 2 倍を超える。(3) 依頼者が構成を変える指示をしたとき。
範囲と件数(evidence/R6-23.md §0・§1): 経路の 32 項目。手書きのつなぎ 1,443 行(build-engine-bundle.sh 456 行を含む)、入力と R2 の層 1,109 行、TS 555 行(消費側)、C++ 529 行(入力)。依頼者の発言は前任 263 件・本セッション 84 件を読んだ。

## 1 候補と判定
| 候補 | 内容 | 判定 |
|---|---|---|
| A | fe-build(lift/ の workspace の bin と lib)。fe-pe・fe-lift・fe-wasmgen は lib の関数で呼ぶ。emcc・wasm-opt・wasm-merge・aws だけ外部プロセス | 採る |
| B | CMake・Ninja | 却下。CMake 自身がスクリプト。Python のつなぎが残る |
| C | fe-wasmgen で wasm を直接作り emcc を外す | 長期の別の行。出力が変わり、同一性の検査が効かない |
| D・E | 現状の bash・Python、または Makefile・npm scripts | 却下(依頼者の禁止。シェル・100 行・Makefile) |

決定: A。根拠: (1) 4 クレートは lib の公開 API を持つ(`fe_pe::rebase::rebase`・`fe_lift::generate_full`・`fe_wasmgen::compile`)。(2) 基準の 1,116 秒の 96% は emcc・fe-wasmgen・wasm-opt の計算(evidence §2)。つなぎを換えても時間は減らず、減るのは行数・重複・検査の食い違い。(3) 依頼者は lift を Rust で作ると決めた(msg_15034d596001HdCrYgl5mxPtfQ)。

## 2 段の順序と同一性(evidence §3・§5)
S0 基準 → S1 入力(R2 の抽出キャッシュ)→ S2 gen・resplit → S3 .o → S4 hybn0〜hybrid → S5 seed・bundle → S6 R2 の層・root・deploy.yml → S7 旧の削除。各段の旧ファイルは同じ依頼で消す(ラッパーにしない)。最終の engine wasm・glue・seed・DLL pack の sha256 を基準と一致させる。manifest は `bundle.created` を除き、.br は展開して比べる(バイト一致は未確認)。

## 3 0400 との差
- bundle・R2 の載せ・root を入れる。根拠: 依頼者の 2 通目「当然 R2 へのアップロードも維持します」(0400 の撤回条件 (1))。
- `timeout-minutes` 40 → 120(基準の段の合計 1,116 秒に、emsdk の導入と未測定の bundle が加わる)。`paths` に `lift/**`・`ci/**`・`.github/**` を足す(bundle の入力)。

## 4 捨てる物と根拠
- build-engine-bundle.sh・Python のつなぎ・stage-keys.sh の関数本文のハッシュ: 依頼文「スクリプト禁止」と msg_1b7bb7b92001i7T2kIeDM1AT7O(100 行近くの記述は間違い)。
- 「あるから飛ばす」10 箇所: 古い出力を使う危険。R2 の内容の鍵で代える(0395)。出典: evidence §4。
- RT_O2_FILES(未使用)・systemd の注記(実装に無い): 出力に影響しない。
- 残す根拠: msg_1e28f3a20001GYppSYhLxztoMh(R2 からの入力と抽出キャッシュ)、msg_1e298741b0011ilEMoBvRoXd5n(捨てる物は探して根拠を書く)。

## 5 deploy.yml の下書き(0400 の制約: 100 行・80 文字・直接の run・公開 action 3 つ)
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
    - name: 入力・bundle・R2 の載せ・root(R2 の Secrets はこの段だけ)
      working-directory: lift
      env:
        CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
        CACHE_ID: ${{ secrets.R2_CACHE_ASSETS_ACCESS_KEY_ID }}
        CACHE_SECRET: ${{ secrets.R2_CACHE_ASSETS_SECRET_ACCESS_KEY }}
        ARTIFACT_ID: ${{ secrets.R2_ARTIFACT_ACCESS_KEY_ID }}
        ARTIFACT_SECRET: ${{ secrets.R2_ARTIFACT_SECRET_ACCESS_KEY }}
      run: cargo run --release --locked -p fe-build -- publish
    - id: cfg
      working-directory: lift
      env: {CLOUDFLARE_API_TOKEN: "${{ secrets.CLOUDFLARE_API_TOKEN }}"}
      run: cargo run --release --locked -p fe-build -- worker-config
    - id: up
      uses: cloudflare/wrangler-action@953926a2e2182532811c01a25e53647d93bf07c0
      env: {WRANGLER_OUTPUT_FILE_PATH: "${{ runner.temp }}/upload.ndjson"}
      with:
        apiToken: ${{ secrets.CLOUDFLARE_API_TOKEN }}
        accountId: ${{ steps.cfg.outputs.account }}
        workingDirectory: app
        command: >-
          versions upload --config wrangler.deploy.json
          --message ci-${{ github.run_id }}
    - id: ver
      working-directory: lift
      run: cargo run --release --locked -p fe-build -- version
    - id: sw
      uses: cloudflare/wrangler-action@953926a2e2182532811c01a25e53647d93bf07c0
      with:
        apiToken: ${{ secrets.CLOUDFLARE_API_TOKEN }}
        accountId: ${{ steps.cfg.outputs.account }}
        workingDirectory: app
        command: >-
          versions deploy ${{ steps.ver.outputs.new }}@100%
          --config wrangler.deploy.json --message ci-${{ github.run_id }} --yes
    - id: check
      if: steps.sw.outcome == 'success'
      working-directory: lift
      env: {CLOUDFLARE_API_TOKEN: "${{ secrets.CLOUDFLARE_API_TOKEN }}"}
      run: cargo run --release --locked -p fe-build -- check-live
    - if: failure() && steps.up.outcome == 'success'
      uses: cloudflare/wrangler-action@953926a2e2182532811c01a25e53647d93bf07c0
      with:
        apiToken: ${{ secrets.CLOUDFLARE_API_TOKEN }}
        accountId: ${{ steps.cfg.outputs.account }}
        workingDirectory: app
        command: >-
          versions deploy ${{ steps.cfg.outputs.old }}@100%
          --config wrangler.deploy.json --message ci-rollback --yes
```
