# signature-studio-develop

`FLEngine_x64.dll`（FL Studio 26.1.7.5653 の Win64 エンジン）を x86-64 → C → wasm に lift する M1 の作業リポジトリ。

## 目的

`FLEngine_x64.dll` を x86-64 から C に変換（lift）し、その C を wasm にして動かす。lift ツールチェーンは Rust（`lift/` の Cargo workspace）、生成物のランタイムと実行環境（harness）は C。依頼者の最終目標は「FL Studio がブラウザ上で実物と変わらず動作する」こと。

M1 は「FLEngine_x64.dll の完全 lift」。完了の条件は A2 + A4 + A5 + A6（A1 は前提、A3 は wasm 化の確認）で、定義は [`docs/decisions/0046-m1-acceptance.md`](docs/decisions/0046-m1-acceptance.md)。

**M1 は完了していない。** 現在の到達点、各基準の判定、停止している場所は [`docs/progress.md`](docs/progress.md) が正本。数値はここに書かない（古くなるため）。

## 構成

```
tools/                   DLL 抽出のスクリプト（Python）
  extract_nsis.py        前任者のスクリプト（元のまま。NSIS ヘッダの 0 パディングに対応しておらず失敗する）
  extract_nsis2.py       修正版。NSIS インストーラからファイルを 1 つストリーム抽出（標準ライブラリのみ）
  find_nsis_by_size.py   名前解決できない場合の代替。サイズ・SHA-256 で NSIS エントリを特定
  count_rip_indirect.py  rip 相対の間接分岐を数える調査用
lift/                    Cargo workspace（members は crates/ の 3 つ）
  crates/fe-pe/          PE32+ リーダ。ヘッダ、セクション、エクスポート、インポート、RVA→オフセット、pdata、初期化 unit 表
  crates/fe-decode/      iced-x86 によるデコードと到達解析（関数、基本ブロック、間接 jmp、adjustor thunk）
  crates/fe-lift/        x86-64 → C の変換器（CLI は `fe-lift lift`）
  runtime/               ゲストが呼ぶ OS API の仮想実装（C、fle_*.c / fle_*.h）。ホストの実環境は見せない
  harness/               実行環境
    main.c, image.c, a4.c, trace.c   native の harness、DLL のイメージ読み込み、A4 の測定、停止報告
    a4-survey.sh                     A4 の測定（native、WASM=1 で wasm も）
    wasm/                            emcc による wasm ビルド（build.sh）、native 版のビルド（build_native.sh）、node の実行（run_node.cjs）
    oracle/                          A5 の native オラクルの照合
docs/
  README.md              文書の索引（決定 0001〜0070 の一覧を含む）
  progress.md            現状、受け入れ基準 A1〜A6 の判定、既知の限界
  decisions/             決定（番号つき）
  investigations/        調査と実行記録（抽出記録 win64-extraction.md、A4 の測定 a4-survey-tool.md ほか）
  milestones/            M1 の残作業
binaries/                実バイナリ置き場（.gitignore 済み）
```

`lift/out*/`（生成物の置き場）と `lift/target/` は `.gitignore` 対象で、リポジトリに入らない。

## 実バイナリは同梱しない

`FLEngine_x64.dll`（22,227,016 B）とインストーラはリポジトリに含めない。`.gitignore` で `binaries/`、`*.dll`、`*.exe`、`target/` を除外している。リポジトリに残すのは、抽出手順（スクリプト）と記録（サイズ、SHA-256、PE の実測値）だけ。

## 手順

### DLL の抽出

```
/usr/bin/python3 tools/extract_nsis2.py INSTALLER.exe --list FLEngine
/usr/bin/python3 tools/extract_nsis2.py INSTALLER.exe --hash FLEngine_x64.dll
mkdir -p binaries/win64
/usr/bin/python3 tools/extract_nsis2.py INSTALLER.exe --write FLEngine_x64.dll binaries/win64/FLEngine_x64.dll
```

`--hash` の出力が `22227016 b2c64da40921cf262c15c5b25c2b0422e4d1851e953c87e0bc38ed2c19f9496d` と一致することを確認してから `--write` する。`--write` は既存ファイルを上書きしない。

### ビルドとテスト

```
cd lift
cargo build --offline
cargo test --workspace --offline
./target/debug/fe-pe pe-info    ../binaries/win64/FLEngine_x64.dll
./target/debug/fe-pe pe-exports ../binaries/win64/FLEngine_x64.dll
```

実 DLL のテストは環境変数 `FLENGINE_PE` でパスを指定できる。未設定の場合は `binaries/win64/FLEngine_x64.dll`（リポジトリ直下基準）を使う。

- `FLENGINE_PE` が設定されていてファイルが読めない場合、実 DLL のテストは失敗する。
- 未設定でデフォルトのパスにファイルがない場合、実 DLL のテストは `SKIPPED` を出力して成功扱いになる。この場合は実 DLL を検証していない点に注意。
- fe-lift のテストには、生成した C を `clang` でビルドして実行するものがある（テストのソースが `clang` を呼ぶ）。

### C の生成（lift）

```
cd lift
cargo build --release --offline -p fe-lift
./target/release/fe-lift lift ../binaries/win64/FLEngine_x64.dll OUT.c --pdata
```

使い方は `fe-lift lift FILE OUT.c [--pdata] [--seeds SEEDS.txt]`。全関数を対象にするときは `--pdata` を付ける（`harness/wasm/build.sh` の `gen` 段も同じ引数）。生成 C は約 200 MB になる。

### wasm のビルドと A4 の測定

- wasm: `bash lift/harness/wasm/build.sh [gen split rt gen-c harness link]`（既定は全段）。emcc が要る。生成物は `OUT`（既定 `lift/out-wasm2`）の下に作られ、既にある出力の段は飛ばす。実行は `node run_node.cjs <dll>`（`bin-base/` にコピーされる）。ビルド時のメモリ、環境変数は `build.sh` の冒頭のコメントを参照。
- native 版: `OUT=... bash lift/harness/wasm/build_native.sh`（`build.sh gen split` の生成 C を再利用）。
- A4 の測定（DllMain → InitUnits がどこまで進むか）:

```
bash lift/harness/a4-survey.sh            # native だけ
WASM=1 bash lift/harness/a4-survey.sh     # wasm(node) も実行し、a4 の行が native と同じか確認する
```

前提は `$OUT/native/flengine-native`（既定 `OUT=lift/out-surv`）。`WASM=1` は `$OUT/bin-base/flengine.js` も要る。停止点は結果であってエラーではない（終了コード 0）。ログは `$OUT/a4/` に毎回新規に書かれる。詳細は [`docs/investigations/a4-survey-tool.md`](docs/investigations/a4-survey-tool.md)、測定の定義は `docs/decisions/0060-a4-measure.md`。

## 読む順

1. [`docs/progress.md`](docs/progress.md): 現状、受け入れ基準 A1〜A6 の判定、既知の限界
2. [`docs/README.md`](docs/README.md): 決定の索引（0001〜0070）
3. [`docs/decisions/0046-m1-acceptance.md`](docs/decisions/0046-m1-acceptance.md): 受け入れ基準の定義
4. `docs/investigations/`: 調査と実行記録。残作業は `docs/milestones/M1-remaining-work.md`

## 運用上の注意

- 仮想環境（OS は Windows 10 22H2 と定義、0014）の応答の値の多くは「仮の決定」で、**実機 Windows で確認していない**。内容と撤回条件は [`docs/investigations/provisional-decisions.md`](docs/investigations/provisional-decisions.md)。
- ホストの実環境（実ファイル、実レジストリ、実ディスプレイなど）はゲストに見せない。ゲストが見るのは `lift/runtime/` の仮想の値だけ。
- 未知の外部 API は名前付きで abort する（fail-closed）。止まった場所は失敗ではなく測定結果として記録している。
