# ci1 → PM 下書き: E14（present Worker P。bundle は E13 のまま、ページだけの差）の first idle 手順

## 構成（同じ bundle を 2 つのページで配信し、交互に first idle を取る）
- bundle: E13 の本番 `fl-engine-ci-0363b050`（`kit/bundles/…`、`kit/assets-fl-engine-ci-0363b050/{manifest.json, fl-engine-ci-0363b050→bundles/…, proj}`）。**manifest は 1 つ・bytes は同一**。両方のサーバーが同じ `assets-…` を `/assets/` に出す。
- ページ A（= 現行の本番のページ）: `page d0447902`（`kit/page-d0447902/app`、作成済み）。ページ B（= P）: git の ref があれば `./verify-bundle.sh page <ref>`、無ければワークツリーで `./verify-bundle.sh pagedir pageP <ワークツリーのディレクトリ>`（`<DIR>/app` を `kit/page-pageP/app` にコピーして同じ vite build。`node_modules` は PM の木への symlink、vite のキャッシュは kit の中）。
- ポート: A = 5060（比較の相手 = prev）、B = 5061（新 = new）。`exec ./serve.sh 5060 fl-engine-ci-0363b050 d0447902` と `exec ./serve.sh 5061 fl-engine-ci-0363b050 pageP`（どちらも背景）。perf の beacon は `out/perf-5060.jsonl`、`out/perf-5061.jsonl` に分かれる。
- 環境変数: `export NEW=fl-engine-ci-0363b050+P PREV=fl-engine-ci-0363b050 REF=pageP WORKER=<E13 の本番の Worker 版> NEWPORT=5061 PREVPORT=5060 MAXWAIT=15 QUIET_LOAD=8`（`NEW`/`PREV` は表の見出しだけに使われる。`REF` は `entry-check` が使う e2e の道具の置き場 = 作った page の `app` なので、B の名前でも A の名前でもよい）。

## 手順
1. 作成: サーバー 2 本が 200 を返すこと（`curl`）、`./verify-bundle.sh preflight`（NEWPORT = B で Firefox の `webglRenderer` 非 null・`desk.mode = gl`。P が present の経路を変えるなら、Chrome の beacon の `desk.mode` も記録）。
2. `./verify-bundle.sh idle chrome 10`（A と B を交互、開始 load ≤8・I/O 圧 0 を待つ、上限 15 分、各 run の他の Chrome の数は `chrome-count.mjs` の記録から表に付ける）。基準は E13 比 +100 ms（PM の指定があれば従う）。10 組で外れ値が出たら E12 のときと同じに追加 6 組と段階時刻（main_start → idle）を見る。
3. 続けて `idle firefox 6`、`scenes chrome|firefox`（停止なし、File ×100）、`export`（381cdb5a…f979）、`alpha7`（画素差）。pure（`pure chrome 6`、A と B の交互。仮想 s/実 s・presents/s・書き込み率・音楽位置の窓）と lat（`lat chrome 3`）は counted の窓で。
4. 報告: 表は全 run の load・I/O 圧・他の Chrome の数。ページ B だけが違う（bundle の bytes が同じ）ので、engine の stage（main_start → idle）の差が出れば page の作業（Worker P の起動など）として読める。

## 注意
- E14 が**新しい冷間 bundle**の場合は通常の手順（`fetch`・`page`・サーバー 2 本）。この下書きは『bundle が同じでページだけが違う』場合。
- 実行中の `verify-bundle.sh` は編集しない。窓の受け渡しは info 1 行、待機は背景の待機プロセス。

## 準備の状態（23:10 machine date、ブラウザなし・CPU ほぼ 0 の作業だけ）
- ページ A（E13 の本番 = d0447902）+ E13 の assets は、**5053 で既に起動している**（`serve.sh 5053 fl-engine-ci-0363b050 d0447902`、これが 5060 と同じ内容）。E14 で A を使う間は `PREVPORT=5053` でそのまま使える（5060 を新たに立てる必要は無い。perf の beacon は `out/perf-5053.jsonl`）。ポート 5060・5061 は空き（`ss -ltn` で確認）。
- ページ B（P）は **未作成**（P の ref もワークツリーも、まだ PM から指定されていない）。指定を受けたら `./verify-bundle.sh page <ref>` または `pagedir pageP <DIR>`（vite build 1 回、nice 19、数十秒〜数分の CPU）→ `exec ./serve.sh 5061 fl-engine-ci-0363b050 pageP`。
- **P が B1 以降の別の bundle（例 `fl-engine-ci-8301687b`）を前提にする場合**（0312f の S0+S1+P は『B1 の bundle + 積み重ね + P の page』）: その bundle は R2 に無く、`~/work/mt3w-s1/ci-b1/engine/bundle/…` にある。`assets-<NAME>` に manifest と bytes を置く手順（`fetch` ではなく、gpu1 の `mk-assets.sh` の出力か、同じ形の hardlink）が別に要る。その場合は bundle が同一でないので、この計画の『bundle が同じ』の前提は成り立たず、通常の E 系の手順（scenes・File ×100・Export・alpha7 を含む）に切り替える。PM の指定を待つ。
- 起動後の窓は **PM の info のみで始める**（自動開始はしない）。
