# docs/investigations の履歴スクリプトについて

`docs/investigations/*/tools/` のスクリプトは、その調査を行った日に実行した物の記録である。当時のビルド変数(`SHIP_PRESET`、`HOOKS=0`、`JSPI=1`、`VAR=force` など)と作業木(`lift/out-*`)を前提にしており、現行では動かない。
製品の入口ではなく、動く状態に保たれてもいない(決定 0243 層 e)。そのまま実行せず、ここから変数を現行のスクリプトへ写さないこと。
現行のビルドは `app/scripts/build-engine-bundle.sh` と `ci/` で、挙動を選ぶ設定変数を持たない。
