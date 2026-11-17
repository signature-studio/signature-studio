不一致: 4 組(wasm・glue・seed・dll。実測 4/4。生ファイルの大きさは 4 組とも manifest と一致)
原因: artifacts/ の .br 4 件は tools/verify-kit/tools/mkbr.mjs(brotli 品質 6)の出力とバイト一致(実測)。manifest の encodedSize は app/scripts/pack-assets.mjs の既定品質 11 と同じ引数で再圧縮した大きさと一致(実測。node の zlib で再現。pack-assets.mjs 本体は実行していない)。.br を作った実行者と時刻は未確認(記録なし)。
配信への影響: 読んだコードでは影響しない。encodedSize を読む行は検証・配信・進捗のどこにも無い(§3)。転送量は R2 上のオブジェクトの大きさで決まり、R2 上の中身は未確認(wrangler 禁止のため読んでいない)。実測は R0-2c で行う。

# R0-1: .br の実サイズと manifest の encodedSize の不一致の調査

## 1. 実測
コマンド: `stat -c '%n %s' artifacts/bundles/fl-engine-ci-0363b050/*`、`jq -r '[.engine.wasm.size,.engine.wasm.encodedSize,.engine.options.glue.size,.engine.options.glue.encodedSize,.seedPack.size,.seedPack.encodedSize,.dllPack.size,.dllPack.encodedSize] | @tsv' manifest.json`

| 資産 | 生ファイル(= manifest.size) | .br の stat | manifest.encodedSize | 判定 |
|---|---:|---:|---:|---|
| wasm | 143,087,706 | 19,795,997 | 15,697,877 | 不一致 |
| glue | 744,946 | 85,604 | 67,170 | 不一致 |
| seed | 7,248,243 | 4,446,700 | 3,929,989 | 不一致 |
| dll | 77,444,112 | 19,398,314 | 15,953,166 | 不一致 |

## 2. 作り方の出典
- mkbr.mjs:7: 品質 6・lgwin 24・size hint = 入力長。既存の NAME.br を書き換える。
- verify-bundle.sh:35: fetch の後に `node $K/tools/mkbr.mjs $d`。tools/verify-kit/README.md:10 は「`.br` は品質 6 でローカルに作る」。
- pack-assets.mjs:66: 品質の既定は 11。:82 で .br を作り、:85 で encodedSize に入れる。build-engine-bundle.sh:33 は `BROTLI_QUALITY=${BROTLI_QUALITY:-11}`、:441 で渡す。
- 再圧縮の実測(node の zlib。lgwin 24、size hint = 入力長):

| 資産 | q6 の sha256 = artifacts.sha256 | q6 の大きさ | q11 の大きさ = encodedSize |
|---|---|---:|---|
| wasm | 一致 | 19,795,997 | 15,697,877 一致 |
| glue | 一致 | 85,604 | 67,170 一致 |
| seed | 一致 | 4,446,700 | 3,929,989 一致 |
| dll | 一致 | 19,398,314 | 15,953,166 一致 |

- 旧ワークツリー側の kit の複製(同じ bundle 名の 9 ファイル。読み取りのみ)は `cmp` で artifacts と同一。
- artifacts/ は .gitignore:5 で git の外。git log に記録なし。
- archive の同じ説明: docs/archive/decisions/0330b-seed-only-change.md:107、0277-e7b-verification.md:167(どちらも局所の再圧縮)。

## 3. 配信側の出典
- manifest.ts:26: encodedSize は「informational」。:86 は型の検査のみ。:89 の required に入っていない。
- `grep -n encodedSize` は loader.ts・worker/index.ts・tools/assets-middleware.ts・shared/manifest-guard.ts で 0 件。読むのは app/test/unit/pack-assets-fruity.test.ts:104 だけで、pack-assets の出力と比べる(artifacts の .br とは比べない)。
- Content-Length: worker/index.ts:91-97 は R2 オブジェクトの size(manifest を使わない)。assets-middleware.ts:92・:105 はファイルの stat。
- 検証は復号後: loader.ts:156・:165・:378 が ref.size(復号後)と比べ、manifest.ts:18 の sha256 も復号後。進捗の分母も ref.size(loader.ts:159・:167)。
- 符号化の選択: assets-middleware.ts:75 は br を受けたとき NAME.br を選ぶ。worker/index.ts:82-83 は R2 に保存された contentEncoding をそのまま返す。

コードから言えること: encodedSize は配信・検証・進捗のどれにも使われない。読んだ行の中に、不一致が P1 の起動を失敗させる経路は無い。ただし遅さへの影響は別: R2 上が品質 6 のままなら wasm は q11 より 26%(19,795,997 ÷ 15,697,877 = 1.261、計算)多く転送される。R2 上の大きさは未確認。

## 4. 未確認
- R2 上の 4 オブジェクトの大きさと中身。
- artifacts/ の .br を誰がいつ作ったか(bundle-files.py は読んでいない)。
- 本番の配信元の応答ヘッダー。
- 不一致が P1 の起動時間に与える影響(R0-2c で実測)。
