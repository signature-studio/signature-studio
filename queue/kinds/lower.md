# lower-{{va}}: 関数 {{va}}({{module}})を直接 wasm にする

関数の表 `queue/inputs/lift-function-classes.tsv` で、クラス b(直接 wasm に移せる候補)の関数です。実行された命令の数は {{instructions}} です。

## やること
1. `{{va}}` を、`lift/harness/hyb/lists/lowered-{{module}}.txt` と `lift/harness/hyb/lowered-all.txt` の末尾に 1 行ずつ足す。
2. 受け入れコマンドを実行する。エンジンを作り直し、起動を確かめる。
3. 通らなければ、原因を直して、もう一度実行する。直せなければ、足した 2 行を戻し、原因を報告に書く。
4. 変更を commit する。

accept: grep -qx '{{va}}' lift/harness/hyb/lists/lowered-{{module}}.txt && grep -qx '{{va}}' lift/harness/hyb/lowered-all.txt && cargo build --release --locked --manifest-path lift/Cargo.toml && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json"
