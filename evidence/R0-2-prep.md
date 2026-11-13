# R0-2 の準備: E13 bundle を headless Chrome で起動するまでの手順(2026-11-13)

## 先頭
- bundle の実体: **在る**。`artifacts/bundles/fl-engine-ci-0363b050/` に 9 ファイル(`ls -la`、2026-11-06 付け)。`sha256sum -c artifacts.sha256` が 10 行すべて OK(本作業で実行)。
- この機械で今すぐ起動できるか: **いいえ**。理由: `app/node_modules` が無く(`ls` で No such file)、ページのビルドと配信は未実行。`npm` の到達は 200 なので `npm ci` で作れる見込みだが、**未実行・未確認**。headless Chrome の起動だけ確認した(1 節)。

## 1 実測(この作業で実行したもの)
| 確認 | コマンド | 出力 |
|---|---|---|
| headless Chrome | `google-chrome --headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage --no-first-run --no-default-browser-check --user-data-dir=$TMPDIR/… --dump-dom about:blank` | rc=0、`<html><head></head><body></body></html>`。stderr は dbus の 1 行(`Request ended (non-user cancelled)`、無害) |
| Chrome の版 | `google-chrome --version` | 156.0.8078.47 |
| bundle の整合 | `sha256sum -c artifacts.sha256` | 10 行 OK(bundle 9 + DLL 1) |
| npm の到達 | `curl -sS -o /dev/null -w '%{http_code}' https://registry.npmjs.org/vite` | 200 |
| app の依存・ビルド | `ls app/node_modules app/dist` | 両方とも無い |
| 旧ツリーの kit | `ls -la`(旧ツリーの `lift/out-ci1/kit/bundles/fl-engine-ci-0363b050`) | 同名の dir がある。中身は未列挙。複製していない |

## 2 bundle の入手経路ごとの可否
| 経路 | 状態 | この機械で今 | 根拠 |
|---|---|---|---|
| a. 手元の実体 `artifacts/bundles/` | 在る | **できる**(実体と sha256 は実測) | 先頭の実測 |
| b. R2 からの取得(旧 `verify-kit` の fetch) | 旧本番バケットを `wrangler` で読む | **しない**(P4。`wrangler` は deny) | `tools/verify-kit/verify-bundle.sh` の fetch 行、`docs/ops/environment.md` §2 |
| c. GitHub の成果物・CI キャッシュ | `engine-bundle` の workflow は未作成 | **できない**(まだ無い) | `.github/workflows/` は `cloudflare-check.yml` の 1 つだけ。ROADMAP R8-4、`environment.md` §1 |
| d. 手元ビルド `app/scripts/build-engine-bundle.sh` | 入力は揃う: DLL 22,227,016 B(sha 一致)、`lift/data/prune-SM.tsv`・`lift/harness/seeds/*`・`build_upstream.sh` が在る | **未確認**(実行していない。R0-5 の範囲) | `build-engine-bundle.sh` の `HOST_ROOT`・`B2D_UPSTREAM_LIB/SRC` は必須。旧 CI の冷間所要 17〜22 分は [文書] |
| e. 旧ツリーの kit の実体 | 同名の dir がある | **使わない**(読み取りのみ。複製しない) | `AGENTS.md`・`docs/ops/environment.md` §5 |

## 3 起動の手順(パラメータ無しの URL)
各行の出典は括弧内。**未実行の行は全て未確認**。
- S1 ページのビルド: `cd app && npm ci` の後 `node_modules/.bin/vite build`(`app/package.json`、`verify-bundle.sh` の `page` 行)。
- S2 bundle の配置: `A=$TMPDIR/assets-e13`。`$A/fl-engine-ci-0363b050` を bundle へのシンボリックリンク、`$A/manifest.json` を実体の copy(`verify-bundle.sh` の fetch の `ln -sfn` と `cp` の 2 行。manifest の `url` は `/assets/fl-engine-ci-0363b050/…` の形)。
- S3 配信: `app/` で `FL_ASSETS_DIR=$A node_modules/.bin/vite preview --host 127.0.0.1 --port P --strictPort`(`app/vite.config.ts` の `configurePreviewServer` と preview の CSP、`serve.sh` の exec 行)。**長時間動くため、起動方法を決める必要がある**。末尾の `&` と `nohup` は禁止、`background` は 15 分以上の処理の指定(worker-rules)。
- S4 確認: `/` と `/assets/manifest.json` の HTTP が 200 であること(`verify-bundle.sh` の「サーバーが落ちていると no-app で 10 分待つ」の注記)。
- S5 Chrome: `google-chrome --headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage --remote-debugging-port=<空きポート> --user-data-dir=$TMPDIR/prof-<n>`。隔離の引数(`--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1`、`--no-proxy-server`、`--disable-ipv6`、`--disable-quic`、`--disable-background-networking`、`--metrics-recording-only` ほか)は `archive/e2e-old/e2e/browser.mjs` の 9〜22 行。起動は `cdp.mjs` の 42〜49 行。プロファイルは毎回新しく作る(冷間)。
- S6 読む: CDP の `Page.navigate` で `http://127.0.0.1:P/` を開く(パラメータ無し)。1 秒ごとに `window.__app.state`・`engineKind`・`timeline.idle` を読み、`idle` が数値、または `state` が `error`・`exited` になったら止める。その後 `window.__app.engineLog().slice(0, 30)` と `document.body.innerText.includes('STAND-IN')`(false であること)を取る。出典: `app/src/main.ts` の 51 行(AppState)、155〜165 行(app オブジェクト)、182 行(`window.__app`)、579〜582 行(idle の stamp と `engine-idle`)。
- S7 期待(未確認): `engineKind=emscripten-fruity-v1`(manifest の `engine.kind`。`app/shared/manifest-guard.ts` の 6 行の許可リスト)。ログの `vfs seed:` 行の文字列は `lift/harness/vfs_seed.h` が出す(grep で確認。実行時の出力は未確認)。

## 4 既存の `entry-check.mjs` をそのまま使えない理由
- `tools/verify-kit/tools/entry-check.mjs` は `app/test/e2e/browser.mjs` を import する。現在の `app/test/` には `helpers`・`unit`・`worker` しか無い(`ls`)。
- 同じ STATE の式に `engineLog` が無い(`grep -c engineLog` = 0)。

## 5 新しく書く必要がある物(R0-2 の範囲。本作業では作らない)
1. 起動スクリプト(`tools/` に 1 本): S2・S3・S5・S6 を行い、5 回分の結果を `evidence/R0-2.md` に出す。
2. 隔離 Chrome の部品: `archive/e2e-old/e2e/` の `browser.mjs`・`cdp.mjs`・`netlog.mjs` を、`tools/` の下へ写すか、import を直す(この repo の中のコピー。旧ツリーからは複製しない)。
3. `app/` の依存: `npm ci`(package-lock あり。npm の到達は 200)。
4. S3 の長時間プロセスの起動方法(規則の中で決める)。

## 6 完了条件の確認
- `wc -l evidence/R0-2-prep.md`: 本ファイルの行数は 1 節の後に記録(commit 前に実行)。
- この文書は手順と入手経路の記録。bundle の実行・ページのビルド・配信は未実行。
