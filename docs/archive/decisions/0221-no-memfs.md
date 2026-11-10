# 決定 0221: Emscripten の MEMFS を実エンジンの起動経路から無くした(ファイルは『パックの読み出し口』=メモリ上のバイト列または OPFS の保存コピーだけ)— memfs1

状態: **仮**(コードは opt-in で、既定の入口のバンドルは旧形式のまま動く。PM は第 3 報への返信で『新形式を既定にする方針で進めてよい』〔旧形式は互換として残す、`?packsource=opfs` は opt-in のまま〕とした。既定の入口の切り替えは ship3 でページとバンドルを同時に出すときに行う)。作業者: memfs1。基準: HEAD 62ac3e9 の `git archive` で始め(wasm の再ビルドと node の検証)、HEAD 050fa43(screen1・cache1 を取り込み済み)に載せ替えてブラウザの検証を行い、最後に **HEAD 29b0b4b**(txt1・crash1・workers1 を取り込み済み)の `git archive` へ三方マージで載せ替えた(`tools/mkdiff3.sh 050fa43`。050fa43 → 29b0b4b で他の担当が触った `app/` のファイルのうち、この差分と同じなのは `app/src/main.ts` と `app/README.md` の 2 つで、衝突は 0 件。`memfs1.diff` は `patch -p1 --dry-run` が offset・fuzz・reject なしで当たる)。依頼者の指示(原文):「あと、memfsはもう捨てていますよね？OPFSのみ残し、memfsに関する情報は過去のドキュメントのみを残しプロジェクトから抹消してください。」

lift の忠実性の方針(`docs/policies/lift-fidelity.md`)との関係(足した物ごと): (1) **MEMFS の除去と、pack を MEMFS へ書き写す変換の除去**=(a) 我々の余計な処理の除去かつ変換の削減。(2) `hostfiles.ts` の syscall 9 本=(b) 外部 API(POSIX の `open/stat/pread/fopen` が wasm の syscall import を通る契約)の**読み取り専用の実装**。契約から外れる呼び出し(書き込みの open など)は黙って成功させず名前つきで停止する。(3) `legacy-glue-fs.ts`=旧形式のバンドルを読む互換の隔離(旧バンドルが無くなったら撤去。新しい特別扱いではない)。(4) first idle で pack を手放す処理=メモリの管理で、(a)(b)・通信/変換の削減のどれにも当たらない。FL の挙動には関わらないが、**方針の表の外にあることを明記し、PM の判断点にした**(`?keepfiles=1` で止められる)。FL の挙動を補う・代行する・特別扱いする処理は足していない。C・生成 C・ランタイム・ハーネスには触れていない。

## 冒頭三点

**何を実測したか。** 実エンジンだけを使った(代役・再生・事前生成は無い)。(1) 出荷中のバンドル(`fl-engine-ship2-F2xH2`)の実エンジンを node で起動し、glue の `FS.open/read/stat/close` を包んで、host の stdio がどのファイルをどれだけ読むかを first idle まで、および 16 スナップ(`cmp-short.g3`)の全期間で記録した。(2) HEAD 62ac3e9 から F2xH2 の構成で実エンジンの wasm を作り直し(gen→wasm 44 単位→リンク→wasm-opt -Oz)、**同じ objects を旧リンク(glue に FS がある)と新リンク(`-sFILESYSTEM=0`)の 2 通り**にして 2 つのバンドルにした。(3) その 2 つを隔離 Chrome 156 と Firefox 158(headless)で**交互に**起動し(毎回新しいブラウザ。1 回目の訪問=保存コピー無し、2 回目の訪問=OPFS に保存コピーあり)、first idle・CPU 秒・スレッドごとの CPU・起動後のメモリ(Chrome は `measureUserAgentSpecificMemory`、両方で /proc の RSS)を測った。(4) 新形式バンドルの実エンジンで 16 スナップ(node)、File > Open / Save / Export(Ctrl+R)(Chrome の e2e、WAV の SHA-256)、全体テスト・a4 FRUITY 無効・FLEngine の既定の生成 C の sha256・vitest・tsc を走らせた。値はすべてこの機械(16 コア、SwiftShader、load は 9〜45 で変動)のもので、依頼者の機械ではない。

**結論。** (a) 実エンジンが起動で host から読むのは 233 ファイル・98.48 MB で、DLL パックの 99.98% が読まれる(『必要なメンバーだけ遅延して読む』利得は無い)。FLEngine_x64.dll は 2 回(image_load と認証情報の検査)全部読まれる。読みは first idle より前に全部終わり、first idle 以後は 0 回。(b) MEMFS は『まだ起動経路で生きていた』。起動後の Worker の JS に pack のバイト列 76.3 MB が残り続けていた(二重保持。wasm メモリ側に同じバイトが image として載っている)。(c) MEMFS は**無くせた**: wasm を `-sFILESYSTEM=0` でリンクし、ページが libc の syscall import をパックのバイト列から答える関数に差し替える(案 D)。**C・ハーネス・生成 C は無変更**(native は実ディレクトリのまま)。`-sFILESYSTEM=0` のリンクは警告なしで通り、glue は 187,659 → 84,966 B。(d) 実エンジンは新形式のバンドルで first idle まで届き、**16 スナップが旧形式と 1 バイトも違わず**(84 行)、Chrome の起動後の Worker の JS は 91.5 → 18.2 MB。(e) 速度: 起動の速さは**変わらない**(first idle までの CPU 秒と main() から first idle で、Chrome 5+4 組・Firefox 20+3 組。差は測定のばらつきの中。§6.4)。MEMFS への書き込み 20〜38 ms が 1〜2 ms になるだけ。 (f) OPFS の保存コピーから読む配線は入れたが、Chrome では host の読みが約 0.4 s 長く(142 → 563 ms。first idle には差が出なかった)、Worker のメモリは『メモリ上のバイト列+手放し』と同じで利得が無いので、`?packsource=opfs` の opt-in にした(PM 承認。Firefox は窓 135 → 191 ms)。(h) **Firefox の first idle の外れ値(+10 s)は新の腕だけの現象ではない**: 順を交互にした 20 組で旧 3/20、新 1/20(3 つの測定の合計で 3/32 と 3/32)。外れ値の回は Worker のスレッドの CPU が +3.6 s、親が +1.3 s と全体が遅くなる形で、syscall 差し替えの窓は外れ値の回でも 112 ms(通常 117 ms)=原因は差し替えの外(この機械の CPU の取り合い=推測)。(g) File > Open / Save / Save as / Export の実エンジンの e2e は、新形式・旧形式(互換)・HEAD の 3 通りで **44/47 で同一**(3 件は HEAD でも落ちる既存の失敗)、Export の WAV は 3 つとも 353,904 B `f5c6a08b…6d8a`、操作の後でも host のファイルの読みは first idle のときと同じ(増えない)ので、first idle での手放しは安全(§6.5)。 `test/e2e/run.mjs` 全体(実エンジンのシナリオ付き)は新形式が 527/536、HEAD+旧形式が 525/536 で、**新だけが落ちる検査は 0 件**(共通の 9 件は以前からの失敗、§6.5)。

**未確認。** 依頼者の機械(Windows、Firefox 159)の値。Firefox の JS 側のメモリ(測る手段が無く、RSS では pack の解放が見えない)。native の 16 スナップの再実行(C 無変更なので同じはず=推測)。人の操作での Basic / Sampler の確認(CDP の合成入力のみ)。ゲストのスレッドは 1 つの wasm インスタンスの中の coroutine で別 Worker は無い(glue に `new Worker` が無い)ので、別スレッドからの host 読みは考えていない。

(本文は以下。数値の出所は `tables/` と `logs/`。)

## 1. 棚卸し(`tables/inventory-memfs.md`)

`grep -rIn -i memfs --exclude-dir=docs` は HEAD 62ac3e9 で 21 ファイル 40 行(050fa43 でも同じ 21 ファイル)。docs/ は 175 ファイル(050fa43 では 190。過去の文書。変えない)。

| 分類 | ファイル | 内容 |
|---|---|---|
| 現役の経路 | `app/src/engine/fruity.ts` | `preRun` が DLL パック(70.6 MB)と seed パック(5.7 MB)を `writePackToFs` で glue の FS(MEMFS)の `/data` 以下へ書き、`memfs_ready` の印を出す |
| 現役の経路 | `app/src/bundle/manifest.ts`、`app/src/ui/phases.ts`、`app/src/main.ts`(コメント)、`app/README.md` | `options.root` の文言と検査、起動段階の文言、印の名前 |
| 現役の経路(ビルド) | `lift/harness/web/fruity/link-web.sh` | `-sFORCE_FILESYSTEM=1`、`-sEXPORTED_RUNTIME_METHODS=FS,ENV,…`(`app/scripts/build-engine-bundle.sh` の weblink 段階が呼ぶ) |
| 現役の道具 | `tools/prune2/node-bundle-run.mjs` | 出荷バンドルを node で走らせる道具(決定 0184)。ページと同じく FS へ書く |
| テスト | `app/test/unit/fruity-engine.test.ts`、`manifest.test.ts` | `FsApi` の偽物、`memfs_ready`、root の検査 |
| 歴史的なハーネス | `lift/harness/web/{fle-worker.js, jspi/, perf1/, perf2/, final0129/(3 ファイル), g3/, fruity/fle-fruity-worker.js, build/link.sh, README.md}` | 決定 0128〜0134 の測定用の Worker とページ。実バンドルは走らせない |
| 名前だけ(MEMFS の語は無いが同じ物) | `app/scripts/build-engine-bundle.sh` の `--args /data/...`、`app/scripts/pack-assets.mjs` の `root` | パックのファイルがエンジンから見える場所の名前 |
| 誤記 | `lift/harness/web/opfs1/fle_opfs_mount.h:18` | コメント「the volume (MEMFS) handles it」。実際は `fle_vfs.c` の木(wasm メモリ内)であって Emscripten の FS ではない |

## 2. 実エンジンが host の stdio で読むファイル(実測。`tables/trace-hostio-firstidle.txt`、`trace-hostio-16snap.txt`)

出荷バンドル(`fl-engine-ship2-F2xH2`)を node で、glue の `FS.open/read/stat/close` を包んで記録した。

| | 値 |
|---|---|
| 触れたファイル | 233(DLL 6、seed 222、標準入出力 3、テスト用の `script.g3` 1〔16 スナップの実行だけ〕) |
| open / stat / read(FS 呼び出し) | 232 / 218 / 1,049 |
| 読んだ合計 | 98,483,356 B |
| DLL パック 70,573,808 B のうち読まれた異なるバイト | **70,559,696 B(99.98%)**。PE の各セクションを `image_load` が `pread` で ImageBase へ(wasm メモリ内) |
| `FLEngine_x64.dll`(22,227,016 B) | **2 回全部**読まれる: `image_load` と、`auth_claims_from_env(path)` が認証情報の検査に全体を読む(1 MiB から倍々の `fread`)。合計 44.4 MB |
| seed パック 5,691,421 B | 全部(`vfs_seed.h` の `vs_read_host` が SHA-256 を検査してから `fle_vfs.c` の木へ) |
| 読みの時間窓 | main() の最初の約 0.45 秒(node)。Chrome では Worker 内で 67〜273 ms(新経路の統計: 最初の呼び出しから最後まで 206 ms) |
| first idle **以後**の host の open/read/stat | **0 回**(first idle まで、および 16 スナップの全期間。例外はテスト用フックの `script.g3` を開く 1 回だけ) |

結論: 『必要なメンバーだけ遅延して読む』利得は無い。全部を起動時に 1 回ずつ読む。読み出し口は『起動時に全部を速く読む』ことと『読んだあとバイト列を持ち続けない』ことが肝になる。コードの上でも、起動後に host のファイルを開く経路は `fle_filedlg.c` の `FLE_FILEDLG_PICK_FILE`(環境変数。ブラウザ版は使わない)だけ。

## 3. 二重保持(Chrome 156、起動後 5 秒。`tables/ab-chrome-cold-summary.txt`、`ab-chrome-warm-summary.txt`)

旧形式: Worker の JS(`JavaScript@DedicatedWorkerGlobalScope`)は GC 後に **91.4〜91.5 MB**(9 回すべて。表の MB は 2^20 B)。pack の ArrayBuffer 76.3 MB(70.57 + 5.69 MB)が MEMFS の node と `assets` から参照されて残っている。wasm メモリの中には同じバイトが image と `fle_vfs.c` の木として載っている=**二重保持あり**。新形式(first idle で pack を手放す。既定): **18.2〜18.4 MB**(9 回すべて)=73 MB 減。

## 4. 設計の比較と決定

| 案 | 内容 | C の変更 | native のテスト | JS に残るバイト | 実装の大きさ(読み。DOM 以外) |
|---|---|---|---|---|---|
| A | pack を JS から読む import(`fle_host_stat/read`)を C に足し、`vfs_seed.h` 等の stdio をそれに置き換える | `vfs_seed.h`、`image.c`、`reg_seed.h`、`authenticode_claims.h` の 4 ファイル+新 C+JS ライブラリ | native 側に 2 本目の実装(実ディレクトリ用)が要る | 持ち続ける(手放す口を足せば消える) | C 約 200 行+JS 約 100 行(読み。作っていない) |
| B | 起動時に必要なファイルだけ `fle_vfs.c` の木へ直接入れる | seed の 5.7 MB は可。**DLL の 70 MB は木ではなく ImageBase へ読む(`image_load` の `pread`)ので扱えない** | 同上 | seed 分だけ減る | — |
| C | OPFS に保存したパックを同期ハンドルで必要な部分だけ読む | A の読み出し口の裏側が OPFS | 同上 | 無い | §2 のとおり『必要な部分だけ』は全部なので遅延の利得は無い |
| **D(採用)** | **wasm を `-sFILESYSTEM=0` でリンクし、ページが libc の syscall import をバイト列(または OPFS の同期ハンドル)から答える** | **無し** | **無変更(POSIX そのもの=実ディレクトリ)** | 手放せる(`release()`)。OPFS 版は元々持たない | 454 行(hostfiles.ts)+旧形式の隔離 32 行 |

決めた理由: (1) C・ハーネスが無変更なので、生成 C・全体テスト・a4・16 スナップ・native のテストに影響しない。『native のテスト用ハーネスは同じ抽象で動く』は、POSIX そのものを使っているから自明に満たされる。(2) §2 のとおり全部を起動時に読むので、A/C の『読み出し口』の利点(遅延)が使われない。D は同じ読み出し口(`ByteSource.readInto`)を OPFS の同期ハンドル版にも使える。(3) 代償は Emscripten の syscall ABI(`struct stat` の配置、errno の番号、import の名前)への依存。単体テスト・ビルド時の記録(§5)・実エンジンの実行で守る。

`-sFILESYSTEM=0` のリンク(実測): HEAD 62ac3e9 の objects(F2xH2)で **リンクは通った**(`link.log` 空=警告なし、`-sEXPORTED_RUNTIME_METHODS=ENV,wasmExports,stackRestore`)。glue は 187,659 B → **84,966 B**(MEMFS・FS のコードが無い)。glue に残る syscall の既定の関数は、ストリームを操作されると名前つきで abort する(`it should not be possible to operate on streams when !SYSCALLS_REQUIRE_FILESYSTEM`、`fd_pread called without SYSCALLS_REQUIRE_FILESYSTEM`)。stdout/stderr は `fd_write` が glue の `printChar` 経由でページの `print`/`printErr` へ行く(`print` が呼ばれてエンジンのログがページに出ることを実エンジンで確認した)。stdio を使う他のコード(`fruity_g3.c` のダンプ、`fle_visrgn.c` 等)は環境変数で有効になる診断で、ブラウザ版では使わない。wasm 自体は 2 つのリンクで 78 B 違い(wasm-opt 後 106 B)で、それぞれ wasm-opt をかけた。

エンジンの wasm が syscall の層から import するのは `__syscall_openat/stat64/fcntl64/ioctl` と `fd_close/read/pread/seek/write` だけ(`lstat64/newfstatat/fstat64` は wasm-opt で消える)。

## 5. 実装

- 新規 `app/src/engine/hostfiles.ts`: `ByteSource`(`readInto(dst, at)`)、`MemorySource`、`OpfsSource`(同期ハンドル。SharedArrayBuffer の view へ直接読み、拒まれたら 1 MiB の私的バッファ経由)、`openPackSource`、`openHostPack`(保存コピーを優先、開けなければバイト列)、`readPackIndex`(先頭 12 B と索引だけ読む)、`HostFiles`(パス→(source, offset, size) の表と syscall 9 本)、`unsupportedImports`(ABI の検査)、`HostFilesUnsupported`、統計 `HostFileStats`(open/stat/read の回数とバイト、窓、`callMs`)。
- 新規 `app/src/engine/legacy-glue-fs.ts`: 旧形式のバンドル(glue に FS がある)を読む互換を 1 箇所に隔離(32 行)。旧バンドルが無くなったらこのファイルと `fruity.ts` の分岐 1 箇所を消す。
- `fruity.ts`: `options.hostFiles === 'packs-v1'` なら新経路(`installHostFiles` を `instantiateWasm` の imports に当てる)、無ければ旧経路。印の名前は中立な `files_ready`(`how: 'packs-v1' | 'glue-fs'`)。first idle で `host_files_released`(既定。`?keepfiles=1` で保つ)。ABI の検査: manifest の `hostFilesAbi`(ビルドの `emcc --version` から書く)が `SUPPORTED_EMSCRIPTEN`(`6.0.12`)に無い、または wasm が未知の syscall import を持つバンドルは、glue を読む前に名前つきの文で止める。
- `manifest.ts`: `options.hostFiles?: 'packs-v1'` と `hostFilesAbi`(どちらか一方だけは検査で拒む)。`pack-assets.mjs --host-files emscripten-X.Y.Z`。`build-engine-bundle.sh` の opt-in `NO_FS=1`(weblink を `link-web.sh` の `WEB_NO_FS=1` で `-sFILESYSTEM=0` に、bundle 段階で `--host-files`)。**既定の動作は不変**。
- `worker.ts`/`protocol.ts`/`client.ts`/`main.ts`: pack を `HostPack` で渡す、`BundleAssets.stored`(OPFS の保存コピーの場所。新形式のとき Worker が優先して開く)、idle の報告に `hostFiles`(`window.__app.hostFiles`)。cache1(決定 0220)がマージされたので `LoadedPack.stored` を使う配線を入れたが、**既定では渡さず `?packsource=opfs` のときだけ**(§6.4: Chrome では保存コピーから読むと host の読みが約 0.4 s 長く、メモリの利得が無い。PM 承認済み)。`?keepfiles=1` は手放しを止める。
- 停止の文面(PM の条件 2): 書き込みの open、相対の open、誰にも渡していない/閉じた fd、手放したあとの open は `HostFilesUnsupported` を投げ、wasm が停止して「The engine asked its host files for something they do not do: opening /data/x for writing (open flags 0x41). The host files are the read-only packs of the bundle」がページの停止の文面になる(例外の文面は単体テストで確認。**実ブラウザでこの停止がページに出る所までは確認していない**: 今のエンジンはこの呼び出しをしない)。存在しないパスは普通の ENOENT、ディレクトリの open は EISDIR。

## 6. 検証

### 6.1 単体・型(最新の HEAD 29b0b4b に `memfs1.diff` を当てた木)

- `npm run typecheck`(`tsc --noEmit` 4 設定: `tsconfig.json`、`.webworker.json`、`.test.json`、`worker/tsconfig.json`): 0 件(`logs/typecheck-final.txt`)。
- vitest: **1,589 passed + 7 skipped**(HEAD の素の `git archive` は 1,551 + 7。+38。`logs/vitest-final.txt`、`logs/vitest-head-final.txt`)。新規 `hostfiles.test.ts` 32 件: syscall 9 本の挙動(読み・`pread`・`seek`・`stat` の配置〔mode、size、blksize〕・ENOENT/EISDIR・fd の再利用)、書き込み/未知の fd/相対 open/手放した後の open が名前つきで停止すること、`readPackIndex`(flpack1/flseedp1、先頭 12 B と索引だけ読む)、`OpfsSource`(短い読み、SAB の拒否→私的バッファ)、`openHostPack`(保存コピー優先、ロック/サイズ違い/索引不正でバイト列へ)、ABI の拒否(未知の Emscripten、未知の syscall import)、`FruityEngine` の新経路(FS 無しの glue、imports の差し替え、環境、統計、first idle の手放し)。`manifest`・`pack-assets`・`fruity-engine`・`filedlg` の既存テストは新形式/旧形式の両方を検査するよう更新した。

### 6.2 実エンジン(出荷バンドルの wasm と glue をそのまま使い、ファイル呼び出しの層だけを差し替えた。node。`tables/hostfiles-layer-vs-glue-fs-shipped.txt`、`tables/g3/`)

glue には FS が残っているが、manifest の写しに `hostFiles: 'packs-v1'` を足して、ファイル呼び出しを `hostfiles.ts` に答えさせた。first idle まで: host の読み 98,483,356 B(旧経路の FS 呼び出しの合計と**同じ値**)、open 229(旧は stdin/stdout/stderr を含めて 232)、stat 218、first idle 後の呼び出し 0。**16 スナップ(`cmp-short.g3`)の g3 行 84 行が旧経路と完全に同一**(`diff` の出力なし)、`g3cmp.py` で `expected/g3-lines-SHIP2-baseline.txt` と `IDENTICAL (82 lines: g3-snap 16, g3-progress 16, g3-surface 34, g3-wintable 16)`。エンジンは vfs_seed の SHA-256 検査を通って起動している(host のファイルの中身が違えば `vfs-seed: ... SHA-256 mismatch` で停止する)。

### 6.3 作り直した F2xH2 の旧/新バンドル(HEAD 62ac3e9 の objects を `-sFILESYSTEM=0` ありとなしでリンク)

node で `tools/g3-bundle.sh`(`cmp-short.g3` の 16 スナップ)。旧形式(glue に FS あり)も新形式(`-sFILESYSTEM=0`)も g3 行は **84 行で、互いに 1 バイトも違わない**(`tables/g3/rebuilt-F2xH2-old-format.txt` と `…-new-format.txt`)。どちらも `lift/harness/b2d2/expected/g3-lines-present2-u1.txt` に対し `g3cmp.py` が `IDENTICAL (82 lines: g3-snap 16, g3-progress 16, g3-surface 34, g3-wintable 16)`。新形式の host の読みは open 230(229 ファイル+テスト用の `script.g3`)、stat 218、read 556、98,483,939 B(= 98,483,356 + script の 583 B)。stdout は旧と同じ 808 行: `-sFILESYSTEM=0` の glue でもエンジンの標準出力は `print` に届く。終了コード 134(スクリプトの正常な終わり)も同じ。

### 6.4 ブラウザでの起動と、メモリの前後(`tables/ab-{chrome,firefox}-{cold,warm}-summary.txt`、`tables/ab-*.jsonl`、`tables/ab-tables.md`)

腕: **旧** = HEAD のページ(cache1 込み)+旧形式のバンドル(glue に FS。pack を FS へ書く)。**新** = memfs1 のページ+新形式のバンドル(同じ objects を `-sFILESYSTEM=0` でリンク。既定=メモリ上のバイト列を使い、first idle で手放す)。**新・opfs** = 新 + `?packsource=opfs`。1 回目の訪問 = 新しいブラウザ(保存コピー無し)。2 回目の訪問 = 同じブラウザで 1 回目の保存が終わるのを待って、about:blank を挟んでもう一度開いた回(cache1 の保存コピーがあり、ページがそこからバイト列を読む)。腕は交互(順は毎回入れ替え)、毎回新しいブラウザ。『host 読みの窓』= エンジンの最初の host ファイル呼び出しから最後まで(新形式のみ。括弧内は syscall の関数の中で使った時間で、`callMs` を足した後の測定だけにある)。値は**この機械**のもの。load は他の担当の仕事で 9〜45 を動いたので、壁時計は CPU 秒と『main() から first idle』を併記した。

**Chrome 156・HEAD 050fa43 のページ(測定した時点の最新)・1 回目の訪問(保存コピー無し=ネットワーク経路)**(中央値〔最小〜最大〕)

| 腕 | n | first idle(ナビゲーションから)s | main() から first idle s | host 読みの窓 ms(呼び出しの中 ms) | first idle までの CPU 秒 | 開始時の load | 最大プロセスの RSS MB | Worker の JS MB(GC 後)| uASM 合計 MB |
|---|---|---|---|---|---|---|---|---|---|
| 旧形式(glue に FS)+旧ページ | 5 | 10.63 〔9.59〜12.59〕 | 8.62 〔7.61〜10.25〕 | (旧は FS 内) | 17.4 | 19.7 | 894 | 91.5 | 113.4 |
| 新形式(`-sFILESYSTEM=0`)・既定(メモリ上のバイト列+first idle で手放す) | 5 | 10.96 〔8.76〜13.13〕 | 9.36 〔6.95〜10.37〕 | 176 | 17.2 | 20.0 | 861 | 18.2 | 40.2 |
(中央値の 1.4 倍 = 15.1 s を超えた回 — old: 0/5、new: 0/5)

**Chrome 156・HEAD 050fa43 のページ(測定した時点の最新)・2 回目の訪問(OPFS に保存コピーがある)**(中央値〔最小〜最大〕)

| 腕 | n | first idle(ナビゲーションから)s | main() から first idle s | host 読みの窓 ms(呼び出しの中 ms) | first idle までの CPU 秒 | 開始時の load | 最大プロセスの RSS MB | Worker の JS MB(GC 後)| uASM 合計 MB |
|---|---|---|---|---|---|---|---|---|---|
| 旧形式(glue に FS)+旧ページ | 4 | 7.91 〔7.83〜10.27〕 | 7.02 〔6.75〜9.36〕 | (旧は FS 内) | 13.1 | 17.9 | 929 | 91.5 | 113.4 |
| 新形式(`-sFILESYSTEM=0`)・既定(メモリ上のバイト列+first idle で手放す) | 4 | 8.05 〔7.64〜9.79〕 | 7.12 〔6.80〜8.80〕 | 142 | 12.9 | 17.7 | 877 | 18.4 | 40.3 |
| 新形式・`?packsource=opfs`(保存コピーから読む) | 4 | 7.96 〔7.23〜8.43〕 | 7.13 〔6.39〜7.38〕 | 563 | 12.8 | 17.5 | 805 | 18.3 | 40.2 |
(中央値の 1.4 倍 = 11.1 s を超えた回 — old: 0/4、new: 0/4、newopfs: 0/4)

**Firefox 158・HEAD 050fa43 のページ(測定した時点の最新)・1 回目の訪問**(中央値〔最小〜最大〕)

| 腕 | n | first idle(ナビゲーションから)s | main() から first idle s | host 読みの窓 ms(呼び出しの中 ms) | first idle までの CPU 秒 | 開始時の load | 最大プロセスの RSS MB |
|---|---|---|---|---|---|---|---|
| 旧形式(glue に FS)+旧ページ | 20 | 8.20 〔6.71〜13.19〕 | 6.38 〔5.31〜10.11〕 | (旧は FS 内) | 16.2 | 13.2 | 1305 |
| 新形式(`-sFILESYSTEM=0`)・既定(メモリ上のバイト列+first idle で手放す) | 20 | 7.77 〔6.80〜11.70〕 | 6.12 〔5.36〜9.62〕 | 117 | 16.1 | 13.8 | 1297 |
(中央値の 1.4 倍 = 11.0 s を超えた回 — old: 3/20、new: 1/20)

**Firefox 158・HEAD 050fa43 のページ(測定した時点の最新)・2 回目の訪問**(中央値〔最小〜最大〕)

| 腕 | n | first idle(ナビゲーションから)s | main() から first idle s | host 読みの窓 ms(呼び出しの中 ms) | first idle までの CPU 秒 | 開始時の load | 最大プロセスの RSS MB |
|---|---|---|---|---|---|---|---|
| 旧形式(glue に FS)+旧ページ | 3 | 9.06 〔8.35〜11.74〕 | 7.17 〔6.59〜9.25〕 | (旧は FS 内) | 13.0 | 17.3 | 1363 |
| 新形式(`-sFILESYSTEM=0`)・既定(メモリ上のバイト列+first idle で手放す) | 3 | 9.27 〔8.52〜10.64〕 | 7.10 〔6.65〜7.99〕 | 135 | 13.1 | 21.9 | 1391 |
| 新形式・`?packsource=opfs`(保存コピーから読む) | 3 | 9.34 〔7.92〜10.79〕 | 6.84 〔6.36〜8.60〕 | 191 | 13.7 | 18.0 | 1382 |
(中央値の 1.4 倍 = 13.0 s を超えた回 — old: 0/3、new: 0/3、newopfs: 0/3)

**Chrome 156・HEAD 62ac3e9 のページ(最新に載せ替える前)・load 12**(中央値〔最小〜最大〕)

| 腕 | n | first idle(ナビゲーションから)s | main() から first idle s | host 読みの窓 ms(呼び出しの中 ms) | first idle までの CPU 秒 | 開始時の load | 最大プロセスの RSS MB | Worker の JS MB(GC 後)| uASM 合計 MB |
|---|---|---|---|---|---|---|---|---|---|
| 新形式(`-sFILESYSTEM=0`)・既定(メモリ上のバイト列+first idle で手放す) | 6 | 8.93 〔8.00〜11.16〕 | 6.50 〔5.81〜8.77〕 | 118 | 15.5 | 11.9 | 785 | 18.2 | 40.9 |
| 旧形式(glue に FS)+旧ページ | 6 | 9.04 〔8.46〜15.43〕 | 6.55 〔6.17〜11.77〕 | (旧は FS 内) | 15.5 | 12.4 | 868 | 91.5 | 115.1 |
(中央値の 1.4 倍 = 12.6 s を超えた回 — new: 0/6、old: 1/6)

**Chrome 156・HEAD 62ac3e9 のページ・load 25〜44**(中央値〔最小〜最大〕)

| 腕 | n | first idle(ナビゲーションから)s | main() から first idle s | host 読みの窓 ms(呼び出しの中 ms) | first idle までの CPU 秒 | 開始時の load | 最大プロセスの RSS MB | Worker の JS MB(GC 後)| uASM 合計 MB |
|---|---|---|---|---|---|---|---|---|---|
| 旧形式(glue に FS)+旧ページ | 6 | 14.31 〔9.67〜18.41〕 | 10.12 〔7.09〜14.37〕 | (旧は FS 内) | 17.8 | 33.2 | 884 | 91.4 | 114.1 |
| 新形式(`-sFILESYSTEM=0`)・既定(メモリ上のバイト列+first idle で手放す) | 6 | 13.56 〔9.49〜17.86〕 | 10.14 〔6.91〜13.42〕 | 167 | 18.0 | 36.0 | 748 | 18.2 | 32.9 |
(中央値の 1.4 倍 = 19.5 s を超えた回 — old: 0/6、new: 0/6)

**Firefox 158・HEAD 62ac3e9 のページ(2 組の測定の合算)**(中央値〔最小〜最大〕)

| 腕 | n | first idle(ナビゲーションから)s | main() から first idle s | host 読みの窓 ms(呼び出しの中 ms) | first idle までの CPU 秒 | 開始時の load | 最大プロセスの RSS MB |
|---|---|---|---|---|---|---|---|
| 旧形式(glue に FS)+旧ページ | 11 | 11.96 〔10.79〜13.15〕 | 6.61 〔5.58〜7.55〕 | (旧は FS 内) | 20.0 | 17.7 | 1225 |
| 新形式(`-sFILESYSTEM=0`)・既定(メモリ上のバイト列+first idle で手放す) | 11 | 11.71 〔10.65〜21.66〕 | 6.40 〔5.68〜13.93〕 | 124 | 19.6 | 17.1 | 1220 |
(中央値の 1.4 倍 = 16.7 s を超えた回 — old: 0/11、new: 1/11)


**Firefox の外れ値(PM の条件 2)**: 外れ値 = その測定の first idle の中央値の 1.4 倍を超えた回。

| 測定 | しきい値 | 旧の外れ値 | 新の外れ値 |
|---|---|---|---|
| A(62ac3e9 のページ、旧→新の順) | 17.7 s | 0/7 | 2/7 |
| B(62ac3e9 のページ、新→旧の順) | 15.6 s | 0/5 | 0/5 |
| C(最新のページ、順を交互に) | 11.0 s | 3/20 | 1/20 |
| 合計 | | 3/32 | 3/32 |

- **新の腕だけに出るのではない**。最新のページで順を交互にした 20 組(各腕 20 回)では**旧 3/20(15%)、新 1/20(5%)**。3 つの測定を合わせると旧 3/32、新 3/32 で同じ。第 1 報の『新の 2/11』を出した測定 A は常に旧→新の順で、順の効果と偶然を区別できなかった(B は逆順で 0/5、C は順を交互にして新の方が少ない)。
- 外れ値の回は **Worker のスレッド(`DOM Worker`)の CPU が +3.6 s(6.6 → 10.2 s)、親プロセスの主スレッドが +1.3 s、他のスレッドが少しずつ**増える(`tables/ff-outliers-cold.txt`)。wasm の tier-2 コンパイル・GC・IO のスレッドが増えたのではなく、**同じ仕事に全体が遅くなる**形(この機械の他の仕事との CPU の取り合いと読んだが、**原因は確定していない**: Gecko profiler は使っておらず、スレッド別 CPU までしか見ていない。外れ値の回の load は 12〜19 で特別高くはない)。
- **syscall の差し替えは遅い経路ではない**: 新の 20 回で、host の読み(最初の呼び出しから最後まで。554 回の呼び出し、98.48 MB のコピー)は中央値 117 ms〔98〜152〕で、`main()` から first idle までとの相関は 0.17。**外れ値の新の回(#5)でも窓は 112 ms、main() から first idle が 9.6 s(通常 6.4 s)**=遅れは差し替えの外(エンジン自身)で起きている。
- エンジンの Worker スレッドの CPU の中央値: 旧 7.65 s、新 7.28 s(第 1 報の 8 組)。差は無い。


**読み方**
- **起動の速さは変わらない**: Chrome の 1 回目の訪問で main() から first idle は旧 8.62 s / 新 9.36 s(5 組、組ごとの差は −0.7〜+1.5 s で向きが一定でない)、2 回目の訪問で 7.02 / 7.12 s(4 組)。first idle までの CPU 秒は 17.4 / 17.2、13.1 / 12.9。Firefox は 20 組で first idle 8.20 / 7.77 s、CPU 16.2 / 16.1 s。HEAD 62ac3e9 のページでの Chrome の 2 つの測定(load 12 と 25〜44)も同じ結論(main() 以後 6.55 / 6.50 s と 10.12 / 10.14 s)。MEMFS への書き込み(`files_ready` の ms)は旧で 20〜38 ms、新で 1〜2 ms で、起動全体の 0.3% 程度。
- **host の読み**: 98.48 MB・554 回の呼び出しが、新形式では Chrome で 120〜180 ms、Firefox で 117〜135 ms の窓に収まる。**syscall の関数の中で使った時間(`callMs`)は Firefox で 21〜26 ms(4 回、中央値 23.5 ms。窓は 97〜117 ms)、Chrome で 42 ms(e2e の 1 回、窓 181 ms)**。残りはエンジン自身の処理(SHA-256 の検査、`fle_vfs` への書き込み)。差し替えの JS は 554 回・98.48 MB のコピーで 25〜42 ms しか使わない=遅い経路ではない。
- **メモリ**: Chrome の Worker の JS(GC 後。`measureUserAgentSpecificMemory`。表の MB は 2^20 B)は旧 **91.4〜91.5 MB**(9 回すべて)→ 新 **18.2〜18.4 MB**(9 回すべて)で、**pack の 73 MB(76.3 MB のバイト列から glue などを引いた分)が消える**。uASM の合計は 113.4 → 40.2 MB(回ごとに 105〜115 → 32〜42 MB)。最大プロセスの RSS は雑音が大きく(同じ腕の中で ±60 MB)、中央値で 894 → 861 MB(1 回目)、929 → 877 MB(2 回目)、HEAD 62ac3e9 のページでは 868 → 785、884 → 748 MB。**RSS の差は 30〜140 MB の範囲で、符号は一定だが大きさを言い切れない**。Firefox は JS 側のメモリを測る手段が無く、RSS は旧 1305 / 新 1297 MB(1 回目)と**差が見えない**。新の既定(手放す)と `?keepfiles=1`(手放さない)を Firefox で 3 組比べても、最大プロセスの RSS は 5 秒後に 1326 / 1312 MB、30 秒後に 1425 / 1453 MB(中央値、keep / release。load 50 前後)で差が無い=**Firefox では pack の手放しが RSS に出ない**(解放されないのか、アロケータが OS へ返さないのか、GC が回っていないのかは未確認)。
- **OPFS の保存コピーから読む(`?packsource=opfs`、2 回目の訪問)**: Chrome で host 読みの窓が 142 → **563 ms**(約 +0.42 s。大きい読みは 330 MB/s、小さい読みの 1 回ごとの費用が大きい)、first idle には差が出ない(7.96 s 対 8.05 s、ばらつきの中)。Worker のメモリは『メモリ上のバイト列+手放し』と同じ(18.3 MB)ので**メモリの利得は無い**。Firefox は窓 135 → 191 ms で差は小さい。→ 既定をメモリ(+手放し)にした PM の判断の裏づけ。

### 6.5 File > Open / Export の OPFS 窓(Chrome の e2e、`test/e2e/files-engine-dialog.mjs`)と WAV

`test/e2e/files-engine-dialog.mjs`(実エンジンの File > Open / Save / Save as / Export〔Ctrl+R〕/ MIDI / zip / cancel を OPFS の窓で答える。隔離 Chrome 156。クリックとキーは CDP の合成入力=人の手の代わり)を 3 通りの組み合わせで走らせた(`tools/e2e-all.sh`、`tables/e2e-engdlg-compare.txt`、`logs/engdlg-{new,compat,base}.txt`):

| 組み合わせ | ページ | バンドル | 結果 |
|---|---|---|---|
| new | memfs1 のページ | 新形式(glue に FS 無し) | **44/47** |
| compat | memfs1 のページ | 旧形式(旧バンドルを読む互換の経路) | **44/47** |
| base | HEAD のページ | 旧形式 | **44/47** |

- **3 つの間で結果の違う検査は 0 件**。3 件の失敗は 3 つの組み合わせで同じ(HEAD で既に落ちる既存の失敗):

| 失敗した検査 | 原因 | この差分での扱い |
|---|---|---|
| `the project folder was put into the OPFS with the window's Upload folder` | cache1(決定 0220)が OPFS の根に `fl/`(検証済みの保存コピー)を作るようになり、『根にはプロジェクトの 3 項目だけ』が合わなくなった(テストは cache1 より前に書かれた) | **検査を弱めず仕様変更に追従**: `fl/` 以下を除いた項目がちょうどプロジェクトの 3 項目であることを要求する(`app/test/e2e/files-engine-dialog.mjs`) |
| `the root lists the folder (the files are filtered by the engine's type)` | 同上(根の一覧が `["fl/","Sampler-project/"]` になる) | `fl/` を除いた一覧がちょうど `["Sampler-project/"]` であることを要求する形に書き換えた |
| `File > Import > MIDI file reaches the page as the same kind of window` | HEAD でも落ちる(原因は調べていない。この変更とは無関係) | 触っていない |

書き換えた 2 検査は、負荷が下がるまで再実行を控えたため、**書き換え後の実行での PASS は未確認**(§6.5 の実行は書き換え前のテストによる)。

- **Export の WAV**: 3 つとも OPFS に書かれた `Sampler.wav` は 353,904 B、SHA-256 **`f5c6a08b1dede642f91026e26939ca99cbea9e20f21bef0c2742c2d5dc3f6d8a`**(テストの検査 `equals its Export (353,904 B, f5c6a08b…)` が PASS。PM の依頼文の値は最後の 1 文字 `a` が欠けている)。File > Open(窓・Save-changes の確認・キャンセル)、Save、Save as(Save 窓・MIDI の型)、zip の展開、Escape も 3 つとも PASS。
- **手放しの前提(host のファイルは起動後に開かれない)**: 新形式の実行の最後(Open・Save・Save as・Export・MIDI・zip・cancel の後)に `window.__app.hostFiles` を読むと、first idle の時点と**完全に同じ**: open 229、stat 218、read 554、98,483,356 B、seek 0、failedOpens 0。操作の間に host の読みは 1 回も無い。`HostFilesUnsupported` による停止も起きていない(3 つの組み合わせとも、エンジンは最後まで普通に動いた)。syscall の関数の中で使った時間(`callMs`)は 42.4 ms(host 読みの窓 181 ms の 23%。残りはエンジン自身の SHA-256 検査や `fle_vfs` への書き込み)。

**e2e 全体**(`test/e2e/run.mjs --target preview`、実エンジンのシナリオ付き。`tools/e2e-main.sh` / `e2e-main-base.sh`、`tables/e2e-main-compare.txt`、`logs/e2e-main-{new,base}.txt`): memfs1 のページ+新形式のバンドルが **527/536**(9 失敗)、HEAD のページ+旧形式のバンドルが **525/536**(11 失敗)。**新が落ちて base が通る検査は 0 件**。9 件の失敗は両方で同じ: `engine-desktop` の『決定 0136 の画素ハッシュ(1223c73f…、f285760a…、7bf902bd…)に一致する』系 8 件と『scenario ran to completion』(この wasm〔HEAD 62ac3e9 から作り直した F2xH2〕はウィザードの描画が決定 0136 の時点と違う=以前から。バンドル形式とは無関係)。base だけが落ちる 2 件(`the screenshot region of the wizard … drawn in front`、`perf-beacon display: the rAF meter ran …`)は時間に敏感な検査で、load 50〜70 の下で base の側が落ちた。両方に無い名前の 2 + 2 件は『STORED brotli bytes (N B)』の N が違うだけ(新形式の glue は 23,956 B、旧は 47,301 B)。

### 6.6 OPFS の保存コピーから読む(Worker 側)

- **単体**(`hostfiles.test.ts`): `openHostPack` は保存コピーが開いてサイズが合えばそれを使い、ロック(別タブが同期ハンドルを持っている: `NoModificationAllowedError`)・サイズ違い・索引不正のときはページが渡したバイト列に落ちて理由を返す(Worker はそれを `warn` に出す)。`OpfsSource` は SharedArrayBuffer の view を拒まれたら 1 MiB の私的バッファ経由で読む。
- **実物のパック**(`tools/opfs-pack-test.mjs`、`tables/opfs-pack-test.txt`): DLL パック 70.6 MB と seed パック 5.7 MB を OPFS に 1 ファイルずつ保存(cache1 と同じ形)、同期ハンドルで開き、全メンバー(6 + 222)を syscall 層(`fd_pread` 1 MiB ずつ)で wasm 共有メモリへ読み、メンバーごとの SHA-256 が索引と一致(**bad 0**、Chrome・Firefox とも)。2 本目の同期ハンドルは `NoModificationAllowedError`(排他)、`release()` で閉じると他から開ける。DLL パックの全メンバーを読む時間(SHA-256 込み)は Chrome 353 ms、Firefox 139 ms。最大のメンバー(39.9 MB)は Chrome 120 ms、Firefox 18 ms。`FileSystemSyncAccessHandle.read()` は SharedArrayBuffer の view へ直接読めた(Chrome・Firefox とも。70 MiB で Chrome 149 ms、Firefox 24 ms〔私的バッファ経由は 7 ms〕)。
- **実エンジン**(Chrome の 2 回目の訪問、`?packsource=opfs`、4 回): Worker は DLL パックも seed パックも OPFS から開き(`pack_source_dll`/`pack_source_seed` の印が `opfs`)、エンジンは first idle に届いた(state `engine-idle`)。host 読みの窓は 563 ms(中央値。メモリ上のバイト列だと 142 ms)。first idle に差は出なかった(§6.4)。Firefox(3 回): 窓 191 ms(メモリで 135 ms)。
- 保存コピーを開いている間、cache1 の側は書き込みに『保存失敗(locked)』を記録して続行する設計(cache1 の回答)。手放し(first idle)で同期ハンドルを閉じるので、ロックの時間は起動の間だけになる。

### 6.7 native 側(C を変えていないことの確認)

- FLEngine の既定の生成 C(`fe-lift lift FLEngine_x64.dll out.c --pdata`): sha256 **`cd9107cef4cbef14f154645749957b7067be7ed7c2b93f5bbe1f72fd47029e50`、215,811,678 B のまま**(`tables/gen-default.sha256`)。
- 全体テスト `bash lift/harness/test.sh full --jobs 3`(**HEAD 62ac3e9 の木にこの差分を当てたもの**。`lift/runtime/`・`lift/crates/` は無変更。最新の HEAD での再実行は、他の測定と重なって load が 50 を超えたので途中で止め、**完了していない**): **185 results / 2,315 passed / 0 failed**、181 実行ファイル+doc-test 4、キャッシュ 0(`logs/fulltest.txt`)。その後に説明だけを直した 2 ファイル(`pack_seed.py`、`fle_opfs_mount.h`)に関わる `opfs1|pack|seed` の 6 実行ファイルを `--fresh` で再実行: 24 passed / 0 failed。
- a4 FRUITY 無効 `lift/harness/a4-fruity.sh off`(同じく 62ac3e9 の木。最新の HEAD での再実行は同じ理由で中止): init_count **942/942**、run_rank **895/895**、functions **5,310**、entries **1,009,731**、exit 0(`logs/a4off.txt`)。
- 16 スナップの **native 側は再実行していない**: native が使う C・ハーネスは無変更(この差分は `lift/runtime/`、`lift/harness/*.c|*.h`、`lift/crates/` を含まない)なので、native の表は変わらないはずだが、これは**推測**。

### 6.8 最新の HEAD に差分を当てたページ(`dry/` 相当の木)の smoke(実エンジン・Chrome 156・1 回)

HEAD 29b0b4b の `git archive` に `memfs1.diff` を当て、`vite build` したページ(`logs/vite-build-final.txt`)を `vite preview` で出し、新形式のバンドル(`fl-memfs1-new`)を `tools/measure-start.mjs` で 1 回起動した(`tables/smoke-final-head-new.jsonl`、`logs/smoke-final-head-new.txt`)。結果: first idle に到達(state `running`)、`files_ready` は `how: packs-v1`(DLL 6 ファイル 70,572,928 B、seed 222 ファイル 5,691,421 B)、first idle の `hostFiles` は open 229、stat 218、read 554、98,483,356 B、`failedOpens` 0(他の木と同じ値)、`host_files_released` が出た、Worker の JS は GC 後 19.1 MB、console に error/warn 無し。**これは 1 回の smoke であって性能の測定ではない**(load 48 の下で first idle まで 12.1 s)。AB 測定・e2e・OPFS のテストは 050fa43 の木(§6.4〜6.6)で行い、最新の HEAD の木では繰り返していない。

## 7. 互換と移行

- **旧形式のバンドル**(`options.hostFiles` が無い)は memfs1 のページで今までどおり動く(`legacy-glue-fs.ts`)。File > Open / Save / Export の e2e で新形式・HEAD と同一(§6.5 の compat)。
- **新形式のバンドルを HEAD の(memfs1 を含まない)ページで開く**と、スキーマの検査で止まり、名前つきの文が出る: `manifest does not match schema 1: /engine/options/hostFiles: unknown property`(0.15 s。`tables/ab-oldpage-newbundle.jsonl`)。つまり新形式のバンドルはこの変更を載せたページとセットで出す。PM の回答: ship3 でページとバンドルを同時に出すので問題なし。
- 新しい Emscripten で作ったバンドル(`hostFilesAbi` が `emscripten-6.0.12` でない)や、未知の syscall import を持つ wasm は、glue を読む前に止まる(単体テスト 2 件)。
- 旧形式を捨てる手順: 旧バンドルを配らなくなったら `legacy-glue-fs.ts` と `fruity.ts` の `else` の枝 1 つ(preRun の FS 経路)を消し、`manifest.ts` の `hostFiles` を必須にする。

## 8. 抹消の結果(`grep -rIn -i memfs --exclude-dir=docs` が残す所と理由)

`grep -rIn -i memfs --exclude-dir=docs` は 40 行・21 ファイル → **22 行・10 ファイル**(すべて歴史的ハーネス。`tables/grep-memfs-remaining.txt`)。現役のコード・スクリプト・テスト・app の README には 0 件。旧形式のバンドルを読む互換(`legacy-glue-fs.ts`)も『MEMFS』の語を使わず『the glue's file system』と書いた。

| ファイル(行数) | 残す理由 |
|---|---|
| `lift/harness/web/README.md`(3) | 歴史的ハーネスの説明。先頭に『歴史的(決定 0128〜0134)。出荷ページはもう FS を使わない』と書いた。本文は当時していたことの記録 |
| `lift/harness/web/{fle-worker.js(2), jspi/fle-worker-jspi.js(2), perf1/fle-perf-worker.js(2), perf2/fle-perf2-worker.js(3), final0129/fle-final-worker.js(3), g3/fle-g3-worker.js(1), fruity/fle-fruity-worker.js(2)}` | 決定 0128〜0134 の測定用 Worker。当時の動作の記録で、`memfs_ready` の印を出す。実バンドルは走らせない |
| `lift/harness/web/final0129/summarize-final.mjs`(3) | 上の Worker が出した過去の記録(`memfs_ready`)を読む集計。語を変えると過去の結果ログが読めなくなる |
| `lift/harness/web/build/link.sh`(1) | 歴史的なリンク(`MODE=web`)のコメント。出荷ビルドは `fruity/link-web.sh` |

『語だけ消す』差分(PM の提案どおり): **`memfs1-hist-words.diff`**(`memfs1.diff` の上に当てる任意の差分。10 ファイル、`memfs_ready` → `files_ready`、『MEMFS』→『the glue file system』の機械的な置換)。当てると `grep -rIn -i memfs --exclude-dir=docs` は **0 件**、`harness-main-lid.test.ts`・`pack.test.ts` は通る(`logs/apply-histwords.txt`)。引き換えに `summarize-final.mjs` は `files_ready` を読むようになるので、過去の結果ログ(`memfs_ready` の印)は集計できなくなる。

歴史的ハーネスの扱い(PM の判断。提案): **削除せず `lift/harness/web/README.md` の先頭に『歴史的』と書いた**(この差分に入っている)。消す場合に壊れる物: (1) `final0129/` は丸ごとは消せない — `final0129/pack_seed.py` は `build-engine-bundle.sh` の seed 段階が使う現役の道具、`app/test/unit/pack.test.ts` と `app/src/bundle/pack.ts` の形式の出典。(2) `web/g3/` は `lift/crates/fe-lift/tests/api2.rs`、`wall_fr14.rs`、`lift/harness/api2/win_*.c`、`testfast/deps.tsv` が参照する。(3) `web/jspi/` は `two/build_nmod_wasm.sh` と `web/fp1/build_nmod_wasm_fp1.sh` が参照する。(4) `web/perf2/` は `noreturn-audit/noreturn_audit.py` と `fp1/build_nmod_wasm_fp1.sh`、`app/test/unit/harness-main-lid.test.ts` が参照する。(5) `web/perf1/` は `harness-main-lid.test.ts` だけが参照する。(6) `fruity/fle-fruity-worker.js` は `app/src/engine/fruity.ts` の冒頭コメントが『移植元』として挙げる。(7) `web/fle-worker.js`、`web/fruity/main.js`、`web/fruity/run-chrome.mjs` は参照する物が無い。したがって『MEMFS の語が残る 11 ファイルのうち、参照されずに消せるのは `web/fle-worker.js` と `web/perf1/fle-perf-worker.js`〔テスト 1 件が名前を挙げるだけ〕程度』で、他は参照を直さないと壊れる。語だけを消すなら `memfs_ready` → `files_ready`、『MEMFS』→『the glue's file system』の置換で足りる(機械的、約 30 行)。

## 9. PM の判断点

1. **新形式のバンドルを既定の入口にするか**(PM 回答: **新形式を既定にする方針で進めてよい**。旧形式は互換として残す。切り替えは ship3 でページとバンドルを同時に出すとき): 既定のバンドルは旧形式のまま(この差分は既定の動作を変えない)。新形式にするには `NO_FS=1` でバンドルを作り直し(wasm は同じ構成で再リンク+wasm-opt。リンク 4 s、wasm-opt 約 5〜8 分)、マニフェストを差し替える。新形式のバンドルは memfs1 のページとセットで出す(旧ページはスキーマの検査で止まる。§7)。Firefox の外れ値は新の腕だけの現象ではないと分かった(§6.4)ので、既定にしない理由は残っていない。
2. **first idle で pack を手放す既定**(`host_files_released`、`?keepfiles=1` で保つ): Chrome の Worker の JS が 91.4 → 18.2 MB(9 回すべて)。引き換えに『起動後に host のファイルを開く機能』は名前つきで停止する。File > Open / Save / Save as / Export / MIDI / zip の e2e(§6.5)の後でも host の読みは first idle から増えなかった(open 229、read 554、98,483,356 B のまま)ので、現状は安全。ただしこれは**方針の表(a)(b)・通信/変換の削減の外**のメモリ管理(冒頭)。既定にしないなら `FruityHost.keepHostFiles` の既定を反転する 1 行。
3. **`?packsource=opfs`**(OPFS の保存コピーをエンジンが直接読む): opt-in にした(PM 承認済み)。Chrome で窓 +0.42 s、メモリの利得なし(§6.4)。cache1 が『ページがバイト列を保持せずに SHA-256 だけ確かめる』形に変われば、Worker が保存コピーを読む価値が出る(そのとき再測定)。
4. **歴史的ハーネス**: 削除せず README に『歴史的』と書いた(PM 承認済み)。『語だけ消す』任意の差分 `memfs1-hist-words.diff` を用意した(§8)。
5. **e2e の既存の失敗 3 件**(`files-engine-dialog.mjs`): cache1 が OPFS の根に `fl/` を作るので『根にフォルダだけ』の 2 検査が落ちる(HEAD で既に)。PM の指示(検査を弱めず、仕様変更への追従として `fl/` を許す形に)で**この差分で書き換えた**(§6.5。書き換え後の実行での PASS は未確認)。`Import > MIDI` の窓の 1 検査は HEAD でも落ちるまま。
6. **全体テスト・a4 は PM の取り込み時に最新 HEAD で実行する**(PM の指示。この作業では 62ac3e9 の木での値)。

## 10. 出典のない選択(S-)

| 番号 | 選択 | 理由・代替 |
|---|---|---|
| S-memfs1-1 | 案 D(syscall import の差し替え) | §4。代替 A/B/C は C を変える/DLL を扱えない/遅延の利得が無い |
| S-memfs1-2 | stat の時刻を 0、mode を 0444(ディレクトリ 0555)、dev 1、nlink 1 | エンジンは見ない(vfs_seed は `S_ISREG` と `st_size` だけ)。MEMFS は書き込み時刻を入れていたが観測されない。代替: bundle の作成時刻 |
| S-memfs1-3 | fd は 3 から最小の空き、上限 1024 | musl の挙動に合わせた。実測の最大同時 open は 1〜2 |
| S-memfs1-4 | 書き込みの open・未知の fd・相対 open は errno でなく `HostFilesUnsupported`(停止) | PM の条件 2(黙って成功させない)。代替: EROFS/EBADF を返す(エンジンが黙って続ける) |
| S-memfs1-5 | 存在しないパスは ENOENT、ディレクトリの open は EISDIR | POSIX の普通の答え。エンジンの `fopen` の失敗の扱いはそのまま |
| S-memfs1-6 | first idle で pack を手放す(既定)、`?keepfiles=1` で保つ | §9-2。代替: 手放さない(JS に 76 MB 残る)、OPFS 版だけ手放す |
| S-memfs1-7 | `HARMLESS_IMPORTS`(fd_write、fcntl64、ioctl、proc_exit、clock_time_get、environ_*) | glue の既定の関数が答え、host のファイルに触れない物。これ以外の `__syscall_*`/`fd_*`/`path_*`/`sock_*` の import を持つ wasm は名前つきで拒む |
| S-memfs1-8 | `SUPPORTED_EMSCRIPTEN = ['6.0.12']`(manifest の `hostFilesAbi`) | このリポジトリの emsdk。新しい版に上げるときは stat の配置・errno・import 名を見直して追加する(PM の条件 1) |
| S-memfs1-9 | opt-in の名前 `NO_FS=1`(PM の例は `NO_MEMFS=1`) | 『MEMFS』の語をスクリプトに残さないため |
| S-memfs1-10 | `OpfsSource` は SAB の view へ直接読み、拒まれたら 1 MiB の私的バッファ経由 | Chrome 156・Firefox 158 とも直接で動いた(§6.6)。Firefox は直接が私的より遅い(70 MiB で 24 ms と 7 ms)が絶対値は無視できる。代替: 常に私的バッファ経由 |
| S-memfs1-11 | 旧形式の判別を『`options.hostFiles` が無い』とする | 旧バンドルのマニフェストを書き換えずに読める。代替: スキーマ番号を上げる(`schema: 1` の検査が全体に及ぶ) |
| S-memfs1-12 | 計測の方法: 交互、毎回新しいブラウザ、first idle はページの `timeline.idle`、CPU は /proc の utime+stime、Chrome のメモリは `measureUserAgentSpecificMemory`(GC 後) | load が 9〜45 のため壁時計は参考値。CPU 秒と main() 以後の時間を併記。回数は指示の下限(6)以上: Chrome 5+4+6+6 組、Firefox 20+5+6+3 組 |
| S-memfs1-13 | Firefox の外れ値 = その測定の中央値の 1.4 倍超 | 出典なし。代替: 2 倍(外れ値が 0〜1 になる)。比較は腕ごとの割合なので倍率に依らない |
| S-memfs1-14 | `?packsource=opfs` は URL パラメータの opt-in(既定はメモリ) | PM 承認。代替: manifest の欄、既定で OPFS |
| S-memfs1-15 | `HostFileStats.callMs`(syscall の関数の中の時間)を常に数える | 呼び出しごとに `performance.now()` 2 回(1,100 回で約 0.1 ms)。遅い経路でないことの確認用。代替: 診断のときだけ |
| S-memfs1-16 | 歴史的ハーネスは削除せず、語だけ消す任意の差分を別に出す | PM 承認。`memfs_ready` → `files_ready` の置換で過去の結果ログが集計できなくなる点を明記 |
| S-memfs1-17 | 全体テスト・a4・FLEngine の既定の生成 C は HEAD 62ac3e9 の木(lift 無変更)で測り、最新の HEAD 29b0b4b では全体テストと a4 の再実行を始めたが load 50 超のため中止した(未完了。最新の HEAD で測ったのは vitest・tsc・`vite build`・実エンジンの first idle の smoke 1 回〔§6.8〕) | 差分は lift/runtime・crates・生成 C に入る C を含まないので、最新の HEAD でも同じはず(推測)。最新の HEAD の `lift/` は他の担当の変更(txt1 の `fle_win.c` 等)で、この差分とは無関係 |

## PM の判断と取り込み(2026-11-04)

取り込んだ基準: HEAD 29b0b4b に `memfs1.diff`(10:11 版、23 ファイル、`-` 79 行を全部読んだ)を `patch -p1` で当てた(offset・fuzz・reject なし)。PM が測った数字(この機械、`lift/out-pmchk-m2/`): `npm run typecheck` rc=0、vitest **1,589 passed + 7 skipped**(素の HEAD は 1,551 + 7)、Worker 50/50、e2e **1,262/1,262**(files-engine-dialog の 2 検査は『`fl/` を除いた集合が元の期待と等しい』形で PASS)。差分は `lift/runtime`・`lift/crates`・生成 C に入るハーネスを含まないので、FLEngine の生成 C と全体テスト・a4 は再実行していない(担当は 62ac3e9 の木で 185/2,315/0・942/942・895/895・5,310・1,009,731 と測った。**最新の HEAD での全体テスト・a4 は ship3 で行う**)。

1. **新形式(`NO_FS=1`)を既定にする方針を確定する。** 切り替えは ship3 で、ページと新形式のバンドルを同時に出す(新形式のバンドルを旧ページで開くと manifest の検査で名前つきで止まる。ページと bundle は同じコミットから作って一緒に出す、という ci1 の設計と整合する)。現在の既定の入口(SHIP2、旧形式)は変えない。
2. **`legacy-glue-fs.ts`(旧形式の互換)は、ship3 の切替後に撤去する**(依頼者の指示『MEMFS は過去のドキュメントのみを残し、プロジェクトから抹消』に従う。この差分は撤去の前段で、旧バンドルを読める間だけ互換を残す)。撤去の差分は ship3 の後の別作業で、`grep -rIn -i memfs --exclude-dir=docs` が歴史的ハーネスの注記だけになることを確かめる。
3. **first idle で pack を手放す既定(`host_files_released`)は採る。ただし方針の表の外にあるメモリ管理**(冒頭に担当が明記)。『起動後に host のファイルを開く機能』は名前つきで停止するので、File>Open / Save / Export(OPFS の窓を通る経路)の e2e が first idle 以後に host の読みを増やさないことを確認済み(open 229・read 554・98,483,356 B のまま)。参照プロジェクトや dec1 のデコーダ DLL など、起動後に host の DLL を開く経路が増えたら、このときに止まるので ship3 以降の確認項目に入れる(`?keepfiles=1` で保てる)。
4. **`?packsource=opfs` は opt-in のまま**(Chrome で host 読みの窓 +0.42 s、メモリの利得なし)。cache1 が『ページがバイト列を保持せず SHA-256 だけ確かめる』形に変わったら再測定する。
5. **歴史的ハーネスは削除せず README に『歴史的』と書いた形で取り込む。** 語だけを消す `memfs1-hist-words.diff` は `docs/investigations/memfs1/` に置き、適用は撤去(2)と同時に判断する。
6. 起動時間・メモリの数字は『この機械・依頼者の機械ではない』測定。Firefox の外れ値(旧 3/32、新 3/32)の原因は『この機械の CPU 競合』と読むが**確認していない**(Gecko のプロファイラは使っていない)。依頼者のブラウザの first idle は cbeacon の beacon で見る。
