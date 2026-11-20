実エンジン: `lift/target/release/fe-build` の engine と bundle を実行し、起動確認(boot-check の一時コピー、ヘッドレスのブラウザ)で engine-idle・終了コード 0 を観測した。engineLog は 322 行を取り、見つからない系の語は 0 件だった。

# survey-c3-fl-resources: C-3 の現状(この runner での測定)

判定: 未確認。インストーラの全件の一覧が runner に無く、「FL Studio 自体が必要とする全件」が分からないため。下の数字は、この実行で得た実測と、そこからの計算だけ。推定には「推定」と付けた。

## 実測

### 1. 版と抽出物
- `find lift/out-app/host -type f | wc -l` → `238`
- `test -f lift/out-app/host/binaries/win64/FLEngine_x64.dll` → 終了コード 0(あり)
- インストーラ(exe)は runner に無い。`find . -path ./app/node_modules -prune -o -path ./lift/target -prune -o -iname '*flstudio*' -print` の出力は 0 行。`ls lift/out-app/host` → `binaries`、`lift`
- 版: インストーラの名前は `flstudio_win64_26.1.7.5653.exe`(`.github/workflows/agent.yml` の 74 行)
- 238 は、seed 表のファイル 227 と DLL 表の 11 の和と一致(`grep -v '^#' ci/extract-dlls.tsv | grep -c .` → `11`)。件数の一致であり、名前の照合はしていない。

### 2. seed 表の行数(lift/harness/seeds/vfs-seed-idle1.tsv)
- 全体は 247 行。`awk -F'\t' 'NF<3'` で拾われる行は 1 行(1 行目の `#` で始まる注釈)。
- `awk -F'\t' 'NF>=3' lift/harness/seeds/vfs-seed-idle1.tsv | wc -l` → `246`(ディレクトリの行を含む)
- `awk -F'\t' 'NF>=3 && $2=="-"' lift/harness/seeds/vfs-seed-idle1.tsv | wc -l` → `19`(ディレクトリの行)
- ファイルの行は 246 − 19 = 227

### 3. 抽出の木との突き合わせ(ファイルの行)
- `awk -F'\t' 'NF>=3 && $2!="-" {print $2}'` で抽出先を取り、各行に `test -f "lift/out-app/host/$r"` を行った。
- 結果: `files_checked=227 missing=0`。無かったパスは 0 件で、最初の 10 件も無い。
- 抽出先が `lift/harness/seeds/os-fonts/` の下の 4 件は、リポジトリの複製で、`app/scripts/extract-fl.mjs` の 76〜77 行が抽出先に写す。

### 4. 分類ごとの件数(guest パスの `FL Studio 21\` の後ろの部分で数えた)
guest の場所は、導入先の下の相対パスで書く。

| 分類 | guest の場所 | ファイル | ディレクトリ行 | seed pack 内の大きさ(B) |
|---|---|---|---|---|
| 言語パック | `System\Languages\` | 1(`en.svg`) | 0 | 899 |
| テーマ(スキン) | `Artwork\Skins\` | 3(`Default` の 3 件) | 0 | 343,555 |
| プロジェクトテンプレート | `Data\Templates\` | 0 | 1 | 0 |
| デモのプロジェクト | `Data\Demo projects\` | 1(`NewStuff.flp`) | 1 | 313,356 |
| 既定のプロジェクト(.flp) | `Data\System\` の .flp | 1(`Empty.flp`) | — | 46,361 |
| 補足 | `Data\Patches\` | 130 | 10 | 2,937,873 |
| 補足 | `System\Config\` | 57 | 3 | 20,730 |
| 補足 | `Shared\Artwork\Fonts\` | 11 | 0 | 1,674,816 |

- 出典: `awk` で guest パスを分類した出力(`FILE	1	System\Languages\`、`FILE	3	Artwork\Skins\`、`DIR	1	Data\Templates\`、`FILE	1	Data\Demo projects\`、`FILE	1	Data\System\` の .flp は `Data\System\Empty.flp` の 1 件、`FILE	130	Data\Patches\`、`DIR	10	Data\Patches\`、`FILE	57	System\Config\`、`DIR	3	System\Config\`、`FILE	11	Shared\Artwork\`。Fonts の 11 件はすべて `Shared\Artwork\Fonts\` の下)。大きさは 6 節の seed pack のヘッダから取った。
- その他(表の中の残り): `Plugins\Fruity\` の 4 件(3 DLL と `GM CCs.txt`)11,232,666 B。guest の Windows の `Fonts` に当たる os-fonts の 4 件 3,636,968 B。`System\Internet\` の 6 件 434,552 B。`Data\System\` の .flp 以外の 3 件 5,466 B。`Artwork\Wallpapers\` の 2 件 14,645 B。`System\Texts\` の 1 件 9,228 B。`Help\` の 2 件 10,949 B。`Data\Demo projects.nfo` の 1 件 22 B。
- 件数の合計: 1 + 3 + 0 + 1 + 1 + 130 + 57 + 11 + 4 + 4 + 6 + 2 + 3 + 1 + 2 + 1 = 227(ファイル行と一致)。大きさの合計は 20,682,086 B。
- ディレクトリ行 19 の内訳: 導入先の直下 1、`Data\Patches\` の下 10、`Data\System\Articulator\` 1、`Data\Templates\` 1、`Data\Demo projects\` 1、`System\Config\` の下 3、`System\Hardware specific\` 1、`System\Internet\About\` 1。

### 5. 起動の記録(エンジンを実際に動かした)
ビルドの段(すべて終了コード 0):
- `npm ci --prefix app` → 0(`added 74 packages`)
- `npm run build --prefix app` → 0(型検査と vite の build)
- `cargo build --release --locked --manifest-path lift/Cargo.toml` → 0(警告のみ)
- `lift/target/release/fe-build engine` → 0(末尾に `hybrid: 167414828 bytes, 8 direct modules`)
- `lift/target/release/fe-build bundle` → 0(`engine wasm  167414828 B`、`seed pack    20780949 B`、`dll pack     88675800 B  11 files`、`stored as br (17717323 B)`)

起動の確認:
- `tools/boot-check/.tmp-survey-c3.mjs` は `tools/boot-check/boot-bundle.mjs` の main の一時コピー。変えたのは 1 箇所で、`engineLog30: a ? a.engineLog().slice(0, 30)` を `engineLogAll: a ? a.engineLog()` にし、全件を取れるようにした。
- 実行: `node tools/boot-check/.tmp-survey-c3.mjs <bundle>/raw/fl-Oz` → 終了コード 0、`error: null`
- 結果(JSON): `state: "engine-idle"`、`standIn: false`、`engineKind: "emscripten-fruity-v1"`、`timelineIdle: 8748.705078125`、`elapsedMs: 9365`(1 回の測定)、`documentTitle: "Welcome to FL Studio - in ?trial mode?"`(原因は未確認)
- engineLog は 322 行(`engineLogAll` の要素数)。
- 見つからない系の語の件数(`grep -c` を engineLog の全文に行った): `見つから` 0、`not found` 0、`ENOENT` 0、`no such` 0、`not exist` 0
- seed の読み込み: `vfs seed: done, 246 files, 51 directories created`(1 行)。guest パスの行は 246(`grep -c '^vfs seed: C:'`)、ディレクトリの行は 19(`grep -c '(directory)$'`)。
- 失敗らしい行は 3 種類だけ:
  - `[err] fle: act1-net` の `InternetOpenUrlW` が 2 件、`-> NULL, last error 12007`(ニュースと版の XML の取得)。推定: 起動確認の隔離フラグ `--host-resolver-rules=MAP * ~NOTFOUND`(`tools/boot-check/boot-bundle.mjs` の 30 行)による。
  - `first drawing call discarded because its window is not visible`(1 件。描画の破棄)
  - `== stop: returned from entry`(1 件。entry の戻り)
- DLL の読み込み: `real module table` が 10 件(dsp_ippv2・blend2d・quickfontcache・freetype・iltools・oggio・elastique・Fruity の 3)。エンジン本体を加えて 11 件で、`ci/extract-dlls.tsv` の 11 件と同じ数。

### 6. 起動前に取る物の大きさ(seed pack)
- 方法: `node -e` で `lift/out-app/seed/seed.bin` を読んだ。先頭 8 バイトは `FLSEEDP1`、続く u32(LE)がヘッダの長さ、その後が JSON のヘッダ。ヘッダの `files` の `path`(`seed/<抽出先>`)と `size` を使い、表の各行を抽出先で引いた。表のファイル 227 件はすべて引けた(`unmatched file rows: 0`)。
- 出力(実測): `magic FLSEEDP1 header_length 43869 file bytes 20780949 header.count 231 header.data_bytes 20737068`。`lift/out-app/seed/seed.bin.manifest.json` の `bytes` は 20780949、`files_checked_against_list_sha256` は 227。
- seed pack の全体: 20,780,949 B(= 12 + ヘッダ 43,869 + データ 20,737,068)。判定の出力の seed pack 20780949 B(run 44673024703)と同じ。
- ヘッダの件数 231 = 表のファイル 227 + extras 4(`seed/vfs-seed-idle1.tsv` 50,734 B、`seed/reg-seed-nsis.tsv` 1,346 B、`seed/reg-seed-app.tsv` 817 B、`seed/authenticode-claims.tsv` 2,085 B)。
- 言語(`System\Languages\`): 899 B
- テーマ(`Artwork\Skins\`): 343,555 B
- テンプレート(`Data\Templates\`): 0 B(ファイルは 0 件)
- プロジェクト: 既定の `Data\System\` の .flp 46,361 B、デモの `Data\Demo projects\` 313,356 B
- 言語・テーマ・テンプレート・プロジェクトの合計: 704,171 B(seed pack の約 3.4%。実測の和からの計算)
- 参考: `Plugins\Fruity\` の 4 件 11,232,666 B も seed pack に入る。そのうち 3 DLL の SHA-256 は、`ci/extract-dlls.tsv` の 15・16・18 行と一致する(`grep -c -F` で seed 表に 3、DLL 表に 3)。つまり、同じ 3 DLL が seed pack と dll pack(88,675,800 B、11 件)の両方に入る。

### 7. エンジン本体の大きさ(参考)
- この実行の engine wasm は 167,414,828 B(br は 17,717,323 B)。`queue/plan.md` の記録は 165,848,536 B(run 44673024703、br 17,510,392 B)。差は +1,566,292 B。原因は未確認(推定: 版の違い)。

## コードの読み

### app/scripts/extract-fl.mjs(抽出の仕組み)
- 25〜29 行 `rows()`: TSV を読み、空行と `#` の行を飛ばし、TAB で割る。
- 52〜68 行: DLL 表。名前の一致で候補の流れを引き、SHA-256 が一致したものを `HOST_ROOT/<rel>` に置く(`place` は 31〜35 行で `.part` に書いてから rename)。一致しなければ終了コード 2(63〜66 行)。
- 69〜70 行: NSIS スクリプトのインストール木を、guest パス(小文字)から流れの位置への表にする。
- 72〜94 行: seed 表。73 行で、欄が 3 未満の行と、抽出先が `-` の行(ディレクトリ)を飛ばす。76〜77 行: 抽出先が `lift/harness/seeds/os-fonts/` の下なら、リポジトリのファイルを写す。79〜83 行: それ以外は guest パスでインストール木を引き、無ければ終了コード 2 で止まる(81〜82 行)。
- 86〜90 行: 取り出した中身の SHA-256 と表の 3 列目を比べ、違えば終了コード 2。
- 91 行: `place(path.join(root, rel), data)` で抽出先に置く。
- 95〜98 行: `--manifest` が指定されたときだけ、書いたファイルの一覧(SHA-256 と相対パス)を書く。インストーラの全件の一覧は書かない。
- 2 行目と 12 行目の注釈のとおり、インストーラは読むだけ。

### .github/workflows/agent.yml(extract ジョブと run ジョブ)
- extract ジョブ(35〜76 行)
  - 50〜64 行: 抽出の木のキャッシュ(R2 のキャッシュ)。パスは `${{ runner.temp }}/host`。鍵(64 行)は `ci/artifacts.lock`、`ci/extract-dlls.tsv`、`app/scripts/extract-fl.mjs`、`app/scripts/lib/*.mjs`、`lift/harness/seeds/vfs-seed-idle1.tsv` の hashFiles。
  - 66〜76 行(キャッシュが無いときだけ): 74 行でインストーラを R2 の artifact から取り、75 行で extract-fl.mjs を実行し、76 行でインストーラを消す。キャッシュに残るのは抽出の木だけ。
  - 鍵: 60〜61 行は R2 のキャッシュの鍵、69〜70 行は artifact(インストーラ)の鍵。
- run ジョブ(78〜161 行)
  - 141〜155 行: 抽出の木を R2 のキャッシュから取る(155 行の `fail-on-cache-miss: true`)。
  - 157〜161 行: `mv` で `lift/out-app/host` に置き、161 行で `FLEngine_x64.dll` の有無を確かめる。
- 訂正: 入力の注記は「R2 の鍵も extract ジョブにだけある」だが、インストーラの鍵(`R2_ARTIFACT_*`)は extract ジョブ(69〜70 行)だけにある。キャッシュの鍵(`R2_CACHE_*`)は run ジョブ(150〜151 行)にもある。

### ci/extract-dlls.tsv
- 有効な行は 11 件(`grep -v '^#' ci/extract-dlls.tsv | grep -c .`)。15・16・18 行の 3 件は Fruity の DLL で、seed 表の `Plugins\Fruity\Effects\` の 3 DLL と SHA-256 が同じ。

## 未確認
1. インストーラ(exe)の全件の一覧。runner に無く、作業ジョブは抽出の木(238 件)だけを持つ。extract-fl.mjs は表の行だけを書き、全件の一覧を出さない(95〜98 行)。インストーラに入っているファイルの総数と大きさは、この実行では得られない。
2. 「必要な全件」。FL Studio が起動と動作に要る言語・テーマ・テンプレートの全件は、この記録からは分からない。起動の記録が示すのは、表の 246 行を読み込んだことと、engine-idle に至ったことまで。表に無いファイルを要求したかどうかは確かめられない。
3. 起動の記録の限界
   - `見つからない` などの語が 0 件でも、欠落の証明にはならない。engine log に、ファイルを開いた記録(成功も失敗も)が出るかは、この実行で確かめていない。`CreateFile`・`FindFirst`・`GetFileAttributes` の語も 0 件。
   - 起動は 1 回だけ(elapsedMs 9365 ms)。10 秒との比較は 1 回の値。
   - 表題の `in ?trial mode?` の原因は未確認。
4. 言語とテーマ。表にある言語は `en.svg` の 1 件、テーマは `Default` の 3 件だけ。他の言語とテーマの有無は、インストーラが無いため確かめられない。
5. engine wasm の大きさ。`queue/plan.md` の記録と違う(7 節)。原因は未確認。
6. 起動前に取る物の区分(配信の方針 L-035)。どの物を起動前に取るかの判定は、この測定の範囲外(survey-c4・c5 の範囲)。

## 次の手順の案
1. インストーラの全件の一覧を作る道具を extract ジョブに足す(推定)。`app/scripts/extract-fl.mjs` に一覧を書く指定を足し、インストーラのインストール木の全行(guest パス・大きさ・SHA-256)を出す。キャッシュが無いときに一覧を作り、中身は含めない一覧だけを artifact に上げるかどうかは依頼者の判断(`agent.yml` の 6 行の方針に照らす)。中身は従来どおり R2 のキャッシュにだけ置く。この変更は `app/`(`app/scripts/`)と `.github/`(`agent.yml`)に触れるので、main の `deploy.yml` が動く。報告に書く。
2. 一覧から、言語・テーマ・テンプレートの全件の表を作る(推定)。seed 表を全件に広げると、起動前に取る seed pack が増える。増える量は一覧ができるまで分からない。その上で L-035 の区分(起動前に取る物と、起動後に取る物)に分ける。
3. 観測: seed pack に入る `Plugins\Fruity\` の 3 DLL(約 11.2 MB)は、dll pack と重複している。L-035 では、プラグインは起動後に取るとされているので、起動前の pack に入れるかどうかは survey-c4・c5 で扱う(推定)。
