# lowerb-{{k}}: 関数のバッチ {{k}} を直接 wasm にする

関数の表 `queue/inputs/lift-function-classes.tsv` から atomic(`lift/harness/seeds/mt4-atomic-functions.tsv`)を除き、表の順に 40 件ずつ分けた塊の、番号 {{k}} です(台帳 L-037)。

mode: script
バッチ: 40
run: bash driver/lowerb.sh {{k}}

## やること(スクリプトが行う。エージェントは動かない)
1. 塊の関数のうち、`lift/harness/hyb/lowered-all.txt` にも `queue/inputs/lower-rejected.tsv` にも無い物を、`lift/harness/hyb/lists/lowered-<module>.txt` と `lowered-all.txt` の末尾に 1 行ずつ足す。
2. `driver/lower-check.sh` で、ビルドと起動確認を行う(error が無く、state が engine-idle、standIn が false)。
3. 通らなければ、`listed but not emitted` の関数を外して `lower-rejected.tsv` に記録し、起動確認が通らないときは、二分探索で通る最大の前半を残して、後半を記録する。ビルドは合計 6 回まで。
4. 通った物を 1 つの commit にする。通る物が無ければ、変更を戻して失敗する。

accept: bash driver/lowerb.sh {{k}} check && bash driver/lower-check.sh
