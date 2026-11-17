# 0022: pack.args が pack-assets.mjs の引数を 4 件欠いていた(--description・--stop-note 2 件・--brotli-quality)。manifest の停止時の案内と説明が消えたまま deploy した

- 日付: 2026-11-17
- 誰が: worker(`lift/harness/web/pack.args` を 7f6a4cba で作り、7306a58d で変えた者。worker ID は未確認)。是正と本記録は、main のレビューを受けた worker。
- 依頼者の発言: 依頼者の直接の発言はない。main のレビューの差し戻し(2026-11-17、main の発言。ses_e2b1511aeffe5dF7dYQK544FTz): 「旧スクリプトが pack-assets.mjs に毎回渡していた --stop-note の 2 件と --description が、新しい pack.args に無い。新しい manifest に stopNotes が入らず、app/src/main.ts:415 が出す案内が消える」。

## 何が起きたか

- 旧(`git show 0ee94698:app/scripts/build-engine-bundle.sh`): 停止時の案内は `STOPNOTE_ARGS` の 2 件(426・428 行)、説明は `--description`(437 行)、`--brotli-quality`(445 行)。
- 新(`lift/harness/web/pack.args` と `.github/workflows/deploy.yml` の 91〜92 行): この 4 件が無かった。`git log 7f6a4cba..HEAD -- lift/harness/web/pack.args` は 7306a58d だけ。
- 結果: 基準の manifest(`~/.cache/signature-studio/work-r55b/bundle/fl-Oz/manifest.json`)には `engine.options.stopNotes` の 2 件と `bundle.description` があり、この機械の再現(`work-r31/bundle-r31`。修正の前)には無い。app は `app/src/main.ts:415`(停止時の案内の照合)と `:681`(bundle-info の説明の表示)でこの値を使う。
- 実行: この pack.args で deploy した run は 43901558294(b4e1e601。success)と 43920582268(d557c42。success)。両方が同じ pack.args を使った。R2 に載った manifest の中身は、この機械からは確かめていない。

## 原因(その時の判断)

- pack.args を旧の呼び出しから写したとき、旧の呼び出しの引数を 1 件ずつ一覧にして比べなかった。`STOPNOTE_ARGS` は配列で別の行に組み立てられ、`--description` は長い 1 行の中にあり、読み落とした。
- 確かめたのは engine・glue・seed・DLL の鍵(R6-30・R6-31)だけだった。鍵は pack-assets の入力の鍵で、manifest の本文(stopNotes と description)は鍵に入らない。鍵の一致で、manifest の本文を見たことにした。
- ローカルの再現の出力を、基準の manifest とキーごとに比べなかった(R6-31 §6 は起動の確かめで、manifest の比較はしていない)。

## 影響

- 停止時の案内(2 件)と説明文が、deploy された manifest から消えた。案内が出ないまま、停止の時の表示は元の文面と違う。
- 依頼者の目に入る範囲(案内の文面と説明)の欠落。公開側の参照は未確認。

## 是正

- 2026-11-17: `lift/harness/web/pack.args` に 4 件を戻した。
  - `--description`: 旧の文から `HEAD 9135826` の表記だけを除いた静的な文(`always-inline-units=15 prune-list=prune-SM.tsv` は、旧の値と同じ 15 件・prune-SM.tsv から確かめた)。
  - `--stop-note` 2 件: 旧と同じ文面。基準の manifest の stopNotes と、キーごとの比較で一致を確かめた。
  - `--brotli-quality 11`: 既定(pack-assets.mjs の `Number(args['brotli-quality'] ?? 11)`)と同じ値を明示した。
- 確かめ(この機械。ワークツリーの `$O` の入力は R6-31 の再現の物で、engine は f6ceeee8eca8): 修正後の pack.args で pack-assets.mjs を走らせ、出た manifest を基準の manifest とキーごとに比べた。`created` を除く 44 キーのうち、差は次の 6 件だけ。engine の 5 件は入力の鍵(f6ce)による。`description` は HEAD の表記だけの差。

| キー | 基準(work-r55b) | 修正前(bundle-r31) | 修正後(ローカル。入力は f6ce) |
|---|---|---|---|
| bundle.version | e5fecd8d93d5 | f6ceeee8eca8 | f6ceeee8eca8(入力由来) |
| engine.wasm.size | 158,494,081 | 158,281,333 | 158,281,333(入力由来) |
| engine.wasm.sha256 | e5fecd8d…83602 | f6ceeee8…867f3 | f6ceeee8…867f3(入力由来) |
| engine.wasm.encodedSize | 17,017,790 | 16,997,776 | 16,997,776(入力由来) |
| engine.wasm.url | …engine.e5fecd8d93d5.wasm | …engine.f6ceeee8eca8.wasm | …engine.f6ceeee8eca8.wasm(入力由来) |
| engine.options.stopNotes[0].contains・note | 2 件 | 無し | 一致 |
| engine.options.stopNotes[1].contains・note | 2 件 | 無し | 一致 |
| bundle.description | `…variant Oz; HEAD 9135826; …` | 無し | 一致(HEAD の表記だけが違う。除外) |

- 修正前の比較は 43 キー(created・description を除く)で、差は上の 10 件(version・size・sha256・encodedSize・url と stopNotes 4 件)。修正後の比較は 44 キーで、差は上の 6 件。残りの 38 キーは基準と一致(args・env・hints・hostFilesAbi・glue・seedPack・dllPack など)。
- ローカル確認の誤り: `pack-assets.mjs` の出力先は、`--out` を後から付けても効かなかった。`xargs` は入力の引数を、指定した引数の後ろに付けるため、pack.args の `--out $O/bundle` が後ろで勝った。出力はワークツリー `r31-deploy` の `$O/bundle` に書かれ、同じ場所の以前の出力(R6-31 の作業の出力)を置き換えた。`~/.cache/signature-studio/work-r31/bundle-r31` は触っていない。
- push: commit e63f7770。run 43933717646(e63f7770)は success(`gh run view 43933717646 --json conclusion` → success)。upload 行の engine は `engine.e5fecd8d93d5.wasm`(基準と一致)、glue・seed・DLL も基準の鍵と一致。CI が R2 に載せた manifest の中身は、この機械からは読んでいない(ローカルの比較で代えた)。

## 引数の一覧(旧と新を 1 件ずつ比べた結果)

旧: `git show 0ee94698:app/scripts/build-engine-bundle.sh` の 426〜445 行。新: `lift/harness/web/pack.args`(修正後)と `.github/workflows/deploy.yml` の 91〜92 行。

| 引数 | 旧 | 新 | 結果 |
|---|---|---|---|
| --out | `$BUNDLE` | `$O/bundle` | 同じ出力先 |
| --prefix | `fl-Oz` | `fl-Oz` | 一致 |
| --name | `fl-engine-Oz` | `fl-engine-Oz` | 一致 |
| --version | engine の sha 12 桁 | deploy.yml の sha 12 桁 | 一致(e5fecd8d93d5) |
| --description | 文(437 行。HEAD を含む) | 無し → 戻した | 欠けていた。修正 |
| --engine-wasm | `flengine6.opt.wasm` | `$O/web/engine.wasm` | 同じ wasm(鍵が基準と一致) |
| --glue | `flengine6.js` | `$O/web/engine-glue.js` | 同じ(鍵 b063ddc0ad2f) |
| --seed-pack | `seedpack.bin` | `$O/seed/seed.bin` | 同じ(鍵 f6526748a0c3) |
| --dll-dir | `$WORK/in/dll` | `$O/dll` | 同じ(鍵 da039e8017d7) |
| --args | `/data/FLEngine_x64.dll` と 9 件の DLL(5 件と EXTRA 4 件) | 同じ 10 件 | manifest の args が一致 |
| --env(6 件) | 同じ 6 件 | 同じ 6 件 | 一致 |
| --fruity-hints | `first=53,stop=59,variant=Oz` | 同じ | 一致 |
| --stop-note(2 件) | `STOPNOTE_ARGS`(426・428 行) | 無し → 戻した | 欠けていた。修正 |
| --host-files | `emscripten-6.0.12` | 同じ | 一致 |
| --brotli-quality | `11`(445 行) | 無し → 戻した | 既定と同じ値。明示のため戻した |
| --bucket | `${BUCKET:-YOUR-BUCKET}` | `fl-web-assets` | 値は違うが、pack-assets.mjs は読まない(出力に出ない) |

## 再発防止

- 文書: 旧の呼び出しを新しいデータファイルへ写すときは、上の表(旧・新・結果)を evidence に作り、1 件ずつ比べる。配列で組んだ引数と、長い 1 行の中の引数を必ず含める。
- 検査(未実施): deploy の後に、新しい manifest を基準の manifest とキーごとに比べる検査。案のみで、実装していない。

## 同じ型の探索

- 範囲: 旧の pack-assets 呼び出し(0ee94698 の 436〜445 行)の引数(表の 16 行)と、`lift/harness/web/pack.args` と deploy.yml の 91〜92 行。上の表の全件を 1 件ずつ比べた。欠けた 4 件(--description・--stop-note 2 件・--brotli-quality)を戻した。
- 出力の鍵が基準と一致した物(engine e5fecd8d93d5、glue b063ddc0ad2f、seed f6526748a0c3、DLL da039e8017d7。R6-30 と R6-31 の確認)は、その引数が出力を変えていないことを鍵で確かめた。seed の引数(`lift/harness/web/seed.args`)と wasm・link・hybrid の引数の 1 件ずつの比較は、表にしていない(未確認)。
- コマンド: `git show 0ee94698:app/scripts/build-engine-bundle.sh | sed -n 420,450p`、`grep -n -i bucket app/scripts/pack-assets.mjs`(読むのは説明のコメントだけ)。

## 未確認

- 公開側(R2 と Worker)が、この修正の前の manifest を参照していたか。
- 同じ時期に、別の worker の commit 6fb75fbe(R5-6a。決定 0408)と 0322bfec が origin の main に入った。6fb75fbe は `pack.args` の `--args` に `fruity_delay2` の DLL を 1 件足した(この記録の run 43933717646 の時点では無い。9 件の版で走った)。1df9827b の push で deploy の run 43944303574 が始まった。その run の結果: success。engine は `b440e13a623a`(e5fe ではない)、DLL は 11 件。詳細は `docs/failures/0023-docs-push-triggered-unverified-deploy.md`。
- 修正後の run の manifest の中身(R2 からは読んでいない。ローカルの再現の比較で代えた)。
