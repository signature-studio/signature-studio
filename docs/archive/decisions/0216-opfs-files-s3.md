# 0216 段階 3(追記): OPFS をゲストのファイルの実体にするための『同期アクセスの実測』と部品 — Chrome 156 / Firefox 158 で測り、ライブラリ(JSPI)とパス変換(C)を作ってテストした。エンジンの `fle_vfs.c` への配線はまだ。仮の決定

状態: **仮の決定**。段階 1(コミット dddad9c)・段階 2(a0d658a)に続く段階 3。基準は HEAD 0156065(`patch -p1 --dry-run` clean、`logs/patch-dry-run-s3.txt`)。成果は `opfs1-s3.diff`(**新規ファイル 15 個だけ、HEAD の既存ファイルは 1 行も変えない = 削除行 0**。MERGE-NOTES-S3.md)。

区分の凡例は段階 1・2 と同じ: **実測**＝このセッションのコマンドの出力を見た事実、**読み**＝コードや文書を読んだ解釈で未確認、**未確認**、**出典なしの選択**＝`S-opfs1-…`(`tables/s-choices-s3.tsv`)。速度の数字は**この機械(16 コア、他の担当が同時に走り、最終の 3 回走の開始時 load は 27〜35)の壁時計**で、依頼者の機械の値ではない。数字のぶれが大きいので、繰り返した回の最小・中央値・最大を出す(`tables/opfs-s3-measurements.tsv`)。

## 冒頭の三点

1. **実行した本物と範囲(実測)**:
   - **段階 3 では実エンジンを 1 回も動かしていない(0 件)**。エンジンのファイル I/O(`fle_vfs.c`)への配線をまだ作っていないので、『エンジンが OPFS をファイルシステムとして使えた』とは言えない。段階 2 の実エンジンテスト(Open/Save/Export、50/50)は既報のとおりで、そこではエンジンの仮想ボリュームにページがファイルをコピーして渡している(エンジンは OPFS を直接は触っていない)。
   - **本物のブラウザで測ったもの**: 隔離 Chrome 156.0.8078.47(headless)と Firefox 158.0(headless、geckodriver)の**専用 Worker から本物の OPFS を `createSyncAccessHandle` で同期アクセス**した(`opfs-bench.mjs`、`opfs-lib-test.mjs`)。ファイルを開く・読む・書く・flush・サイズ・ロックの挙動・Worker を殺しての永続、JSPI の中断の費用、SharedArrayBuffer + `Atomics.wait` で別 Worker に橋渡しする費用。
   - **ライブラリのテスト(模擬あり)**: 作ったライブラリ `fle_opfs_lib.js`(emscripten の JSPI ライブラリ)を、**エンジンではない C のテストプログラム `opfs_test.c`**(emscripten、`-sJSPI -sSHARED_MEMORY=1`、エンジンと同じ作り方)から呼び、Chrome と Firefox で各 3 回、**68 件を全回通した**(`logs/s3/lib-*.txt`)。ここでファイル操作を出しているのはテストの C プログラムで、エンジンの実際のアクセスの型(順次かランダムか、件数、大きさ)は**観測していない**(模擬)。
   - **パス変換のテスト(native -O0)**: `fle_opfs_mount.c`(Windows のパス → OPFS のパス、純粋な C)を `wall_opfs1_map.rs` の **125 件**でテスト(模擬: テストの C が UTF-16 のパスを出す)。gcc の ASan + UBSan 付きでも 125/125、指摘 0。変異検査 9 件は全て落ちる(`tables/opfs-s3-mount-mutations.tsv`)。2 つのファイルは `runtime/` ではなく `lift/harness/web/opfs1/` に置いた: `build_nmod_wasm.sh` が `runtime/fle_*.c` を全部エンジンにコンパイルし、多くのテストも全部リンクするので、`runtime/` に置くと配線の前にバンドルが変わる。配線のときに `runtime/` へ移す。(`runtime/` に置いた版で、全部リンクする既存のテスト 4 本〔call_iat 3、k32_dsp 14、modules 16、hle_entries 10〕と noreturn_audit〔8〕が通ることは、移す前に確かめた。)
2. **結論**: (a) **OPFS の同期アクセスは Chrome 156 と Firefox 158 のどちらでも Worker から使える**。一方、**開く操作は非同期**(1〜5 ms)なので、エンジンの Win32 呼び出しの中で使うには JSPI の中断が要る。**JSPI の中断そのものの費用は小さい(Chrome 約 0.2 µs、Firefox 約 1 µs。測定表)**。エンジンはすでに JSPI で作られ、ゲストのスレッドも『1 つの Worker の中の JSPI スタック』で動く(決定 0124、`fle_thread_wasm.h`〔読み〕)ので、**ゲストのスレッドから呼べるか〔`Atomics.wait` の橋渡しが要るか〕という懸念は、読みでは当たらない**(実エンジンの 2 本目のスレッドからは未実行)。(b) **1 回の呼び出しの費用は Chrome が Firefox の約 4〜15 倍**(3 回の中央値: ランダム 4 KiB の読み 630 µs 対 42 µs〔15 倍〕、書き 1.9 ms 対 0.46 ms〔4 倍〕、順次読み 506 対 3,397 MiB/s)。Chrome ではランダムな小さい読みが 5,000 回あると約 3 秒かかる(630 µs × 5,000)。ライブラリのブロックキャッシュは**順次の小さな読みで約 18 倍、小さな書きで約 23 倍速い**(Chrome の中央値 996 → 57 µs、3.0 ms → 0.13 ms)が、**ランダムな読みでは Chrome でも Firefox でも逆に遅い**(Chrome 867 → 1,601 µs、Firefox 38 → 86 µs)。(c) **同期ハンドルは排他的**(2 つ目の `createSyncAccessHandle`・`createWritable`・`removeEntry`・`move` は `NoModificationAllowedError`、`getFile()` の読みは通る。Chrome と Firefox で同じ)ので、エンジンが開いている間は Files 窓から消せず、他のタブとも競合する。(d) 以上から、**最初の配線は『開くときにバイト列を読み込み、最後に閉じるときに書き戻す』方式(設計 B)を勧める**(§5)。ボリュームの node がキャッシュのままで、`ReadFile`/`WriteFile`/`CreateFileMappingW` の今の動きが変わらず、OPFS のハンドルを開きっぱなしにしない。(e) 部品を作った: パス変換(`fle_opfs_mount.*`)、OPFS ライブラリ(大文字小文字を無視した名前の解決、ハンドル共有、ブロックキャッシュ、置換つき rename、`flush_all`)、**act1 の R4 のための『全体を原子的に置き換える状態ファイル』(`fle_opfs_state_put/get`: `.tmp` に書いて fsync して `move` で置換。書きかけで Worker を殺しても古い内容が残ることをテストした)**。
3. **未確認・未着手**: **エンジンへの配線(`fle_vfs.c`)**: 段階 3 は『設計と測定』の指示で、5 時間の枠の中では着手していない(dec1〔決定 0206〕は merge 済みなので、次の差分で配線できる)。エンジンの実際のファイルアクセスの型。設定(ini など)とレジストリを永続させたとき**Welcome の窓が出なくなるか**(配線が要る)。2 つのタブで同時に開いたときの挙動(同一タブ内のロックだけ測定)。ゲストの 2 本目のスレッドから OPFS を呼ぶ実エンジンテスト。電源断・ブラウザの強制終了でのデータ(テストは Worker の `terminate` とページの再読み込みまで)。`navigator.storage.persist()` が許可されなかったときの OPFS の消去(ブラウザの判断)。依頼者の Firefox 159 / Windows 10 での測定(0 件。数字はこの機械のもの)。人による実際の操作(0 件)。

## 1. 同期アクセスの比較(実測、Chrome 156 と Firefox 158、各 3 回の最小 / 中央値 / 最大)

出典: `tables/opfs-s3-measurements.tsv`(`tools/opfs-s3-summary.mjs` が `logs/s3/` の生の出力から作る)。同じ Worker 内で `createSyncAccessHandle` を使う直接の呼び出し、JSPI で中断する呼び出し、別 Worker に SAB + `Atomics.wait` で頼む橋渡しの 3 つを比べた。

**direct sync access (1 MiB blocks)**

| 項目 | Chrome 156(最小 / 中央値 / 最大) | Firefox 158(最小 / 中央値 / 最大) | 単位 |
|---|---|---|---|
| open of a new file, median | 0.890 / **1.090** / 2.965(n=3) | 1.700 / **1.740** / 3.000(n=3) | ms |
| open of a new file, p95 | 2.630 / **6.045** / 10.7(n=3) | 2.900 / **7.100** / 8.980(n=3) | ms |
| re-open, median | 0.495 / **0.885** / 0.960(n=3) | 0.740 / **1.340** / 1.640(n=3) | ms |
| sequential write 64 MiB | 36.0 / **138** / 157(n=3) | 719 / **916** / 1138(n=3) | MiB/s |
| sequential read 64 MiB | 170 / **506** / 542(n=3) | 1515 / **3397** / 4015(n=3) | MiB/s |
| random 4 KiB read | 564 / **630** / 792(n=3) | 40.0 / **41.9** / 49.7(n=3) | us/call |
| random 4 KiB write | 1676 / **1908** / 1947(n=3) | 325 / **457** / 463(n=3) | us/call |
| getSize | 281 / **442** / 637(n=3) | 31.8 / **38.0** / 209(n=3) | us/call |

**direct sync access (64 KiB blocks)**

| 項目 | Chrome 156(最小 / 中央値 / 最大) | Firefox 158(最小 / 中央値 / 最大) | 単位 |
|---|---|---|---|
| sequential write 32 MiB | 25.4 / **32.8** / 45.3(n=3) | 17.4 / **27.6** / 59.3(n=3) | MiB/s |
| sequential read 32 MiB | 69.4 / **80.3** / 109(n=3) | 46.9 / **150** / 1201(n=3) | MiB/s |
| random 4 KiB read | 559 / **598** / 714(n=3) | 41.6 / **47.5** / 76.5(n=3) | us/call |

**JSPI**

| 項目 | Chrome 156(最小 / 中央値 / 最大) | Firefox 158(最小 / 中央値 / 最大) | 単位 |
|---|---|---|---|
| one suspension that resolves at once | 0.175 / **0.252** / 0.287(n=3) | 0.930 / **1.160** / 1.570(n=3) | us |
| open through JSPI, per call | 1.499 / **3.052** / 4.776(n=3) | 1.758 / **2.190** / 4.163(n=3) | ms |

**SAB + Atomics.wait bridge (helper Worker)**

| 項目 | Chrome 156(最小 / 中央値 / 最大) | Firefox 158(最小 / 中央値 / 最大) | 単位 |
|---|---|---|---|
| no-op round trip, median | 5.000 / **10.000** / 10.000(n=3) | 20.0 / **20.0** / 20.0(n=3) | us |
| no-op round trip, p95 | 25.0 / **215** / 1990(n=3) | 40.0 / **80.0** / 1000(n=3) | us |
| open through the bridge, median | 0.755 / **5.105** / 5.170(n=3) | 1.320 / **1.840** / 1.980(n=3) | ms |
| read through the bridge, median | 205 / **245** / 270(n=3) | 60.0 / **160** / 2000(n=3) | us |

**library with block cache (fle_opfs_lib.js)**

| 項目 | Chrome 156(最小 / 中央値 / 最大) | Firefox 158(最小 / 中央値 / 最大) | 単位 |
|---|---|---|---|
| cached 4K write | 128 / **129** / 182(n=3) | 36.4 / **45.1** / 53.1(n=3) | us/call |
| cached 4K sequential read | 41.8 / **56.7** / 68.6(n=3) | 18.7 / **20.1** / 25.5(n=3) | us/call |
| uncached 4K write | 1842 / **3020** / 3094(n=3) | 330 / **508** / 718(n=3) | us/call |
| uncached 4K sequential read | 907 / **996** / 4239(n=3) | 76.1 / **110** / 183(n=3) | us/call |
| cached random 4K read | 1322 / **1601** / 2166(n=3) | 47.3 / **85.5** / 107(n=3) | us/call |
| uncached random 4K read | 763 / **867** / 1825(n=3) | 36.7 / **37.7** / 76.0(n=3) | us/call |
| 64 MiB write | 57.6 / **70.1** / 93.4(n=3) | 191 / **282** / 358(n=3) | MiB/s |
| 64 MiB read | 187 / **234** / 385(n=3) | 236 / **661** / 767(n=3) | MiB/s |
| 64 KiB reads | 73.1 / **147** / 317(n=3) | 203 / **472** / 534(n=3) | MiB/s |
| create+write+close of a file | 20.3 / **21.3** / 25.0(n=3) | 6.693 / **7.712** / 9.277(n=3) | ms/file |
| list of 300 files (names, sizes, dates) | 69.7 / **98.0** / 149(n=3) | 86.0 / **91.3** / 181(n=3) | ms |
| list of 300 files, names only | 19.6 / **20.5** / 32.1(n=3) | 4.240 / **4.560** / 7.500(n=3) | ms |
| open + close of an existing file | 2.427 / **3.865** / 6.309(n=3) | 3.565 / **4.765** / 5.157(n=3) | ms |
| state put of 3 MiB (tmp, fsync, move) | 26.5 / **29.6** / 44.7(n=3) | 7.180 / **8.460** / 10.2(n=3) | ms |
| state get of 3 MiB | 6.490 / **7.420** / 21.9(n=3) | 3.620 / **4.100** / 6.300(n=3) | ms |
| create + write 10 bytes + close, no fsync at close (default) | 7.323 / **11.4** / 18.3(n=3) | 6.416 / **6.826** / 7.159(n=3) | ms/file |
| create + write 10 bytes + close, fsync at close | 23.6 / **26.1** / 35.6(n=3) | 5.935 / **11.2** / 13.3(n=3) | ms/file |
| open + close of a file that was only read (no fsync, nothing written) | 2.154 / **2.323** / 2.465(n=3) | 2.359 / **5.106** / 5.491(n=3) | ms/file |
| stat of a missing name in a 300-file folder, casefold on (lists the folder) | 24.4 / **28.6** / 33.3(n=3) | 3.300 / **3.380** / 9.220(n=3) | ms |
| stat of a missing name in a 300-file folder, casefold off | 1.260 / **6.335** / 12.4(n=3) | 0.500 / **0.600** / 7.660(n=3) | ms |

負荷(開始時の load average、16 コアで他の担当と共有): chrome: 28.52, 34.75, 27.05; firefox: 34, 32.44, 29.07

読み(測定の解釈):
- **直接の同期呼び出しが最も安い**。橋渡しは、空の往復だけで Chrome 10 µs(最小 5)・Firefox 20 µs(中央値)かかり、さらに橋の向こうの Worker が OPFS を呼ぶ費用がそのまま載る(橋経由の読みは Chrome 245 µs、Firefox 160 µs〔最小 60、最大 2,000〕)。**橋渡しを使う利点は、呼び出し側が JSPI を使えないとき(JSPI の無い Worker や pthread)だけ**。
- **JSPI の中断は、開く操作(1〜5 ms)に比べて無視できる**(0.2〜1 µs)。開く操作は直接でも JSPI でも橋渡しでも 1〜5 ms の範囲(測定表の『open』の行:直接 Chrome 1.1 ms・Firefox 1.7 ms、JSPI 経由 3.1 ms・2.2 ms、橋経由 5.1 ms・1.8 ms)で、3 回の範囲は重なり、機械の負荷によるぶれが方式の差より大きい。方式の違いが決めているのではなく、ブラウザの `getFileHandle` + `createSyncAccessHandle` の費用である〔読み〕。
- Chrome の 1 呼び出しの重さ(ランダム 4 KiB の読み 630 µs、書き 1.9 ms)は、Firefox(42 µs、0.46 ms)の約 15 倍と約 4 倍。**順次の大きな読み書きは両方とも速い**(ブロック 1 MiB のとき、Chrome の 64 MiB の順次読みは 506 MiB/s〔170〜542〕、書きは 138 MiB/s〔36〜157〕。Firefox は読み 3,397 MiB/s〔1,515〜4,015〕、書き 916 MiB/s〔719〜1,138〕)。ブロックを 64 KiB にすると落ちる(Chrome の読み 80 MiB/s〔69〜109〕、書き 33 MiB/s。Firefox の読み 150 MiB/s〔47〜1,201〕、書き 28 MiB/s は、ぶれが大きい)。
- **`getSize` は同期だが 1 回 0.28〜0.64 ms(Chrome。Firefox は約 0.04 ms)かかる**ので、ライブラリは自分でサイズを覚えている(`GetFileSize` ごとに呼ばない)。

ライブラリ(`fle_opfs_lib.js`)の効果(`opfs_test.c`、Chrome と Firefox、各 3 回の最小 / 中央値 / 最大。同じ表の『library』の行):
- 順次の 4 KiB 読み: キャッシュあり Chrome 約 57 µs、なし 約 1.0 ms(約 18 倍)、Firefox 20 対 110 µs。小さな書き: Chrome 約 0.13 ms 対 約 3.0 ms、Firefox 45 対 508 µs。
- **ランダムな 4 KiB 読み: キャッシュありの方が Chrome で遅い**(中央値 1.6 ms 対 0.87 ms。ミスのたびに 16 KiB の窓を読むため)。Firefox も遅い(86 対 38 µs)。つまり**キャッシュは順次・書きのためで、ランダムな読みの対策にはならない**。ランダムな小さな読みが多いファイルは、設計 B(開くときに全部読んでヒープから答える)の方が合う(S-opfs1-block-cache)。
- ファイルを開いて閉じる(読むだけ): Chrome 約 2.3 ms、Firefox 約 5 ms。**閉じるときの fsync を既定でやめた**: 小さなファイルを作って閉じる費用が、Chrome は中央値 26.1 → 11.4 ms(3 回の範囲は重ならない)、Firefox は 11.2 → 6.8 ms(範囲は重なる: 5.9〜13.3 対 6.4〜7.2)になった(Windows の `CloseHandle` も fsync しない。S-opfs1-no-fsync-on-close)。
- フォルダの一覧(300 ファイル): 名前だけ Chrome 約 20 ms・Firefox 約 5 ms、名前+大きさ+日時は 約 98 ms・約 91 ms(ファイルごとの `getFile()` が費用の大半)。`FindFirstFileW` は**名前だけ取ってワイルドカードで絞ってから、合ったものだけ `fle_opfs_stat`** にする(`fle_opfs_list_names`)。
- 存在しない名前の問い合わせ(エンジンはよく探す): 大文字小文字を無視するための一覧が加わり、300 ファイルのフォルダで Chrome 約 29 ms・Firefox 約 3 ms(無効にすると 6 ms・0.6 ms)。Windows の名前は大文字小文字を区別しないが OPFS は区別するので、**まず完全一致を 1 回試し、見つからないときだけ一覧で探す**(S-opfs1-casefold-fallback)。フォルダのハンドルのキャッシュは、Files 窓がフォルダを消して作り直すと古くなるので作っていない。
- 状態ファイル 3 MiB の置換(`.tmp` に書く → fsync → 閉じる → `move` で置換): Chrome 約 30 ms、Firefox 約 8 ms。読み出しは Chrome 約 7 ms、Firefox 約 4 ms。

## 2. 部品(新規ファイルだけ。`opfs1-s3.diff`)

| ファイル | 内容 |
|---|---|
| `lift/harness/web/opfs1/fle_opfs_lib.js`・`fle_opfs.h` | emscripten の JS ライブラリ(`-sJSPI`)と C の宣言。open/close/read/write/flush/truncate/size(同期)、stat/list/list_names/mkdir/remove/rename/set_root(非同期 = JSPI)、`flush_all`。**パスごとに同期ハンドルを 1 つだけ開き、ゲストのハンドル全部で共有**(参照数、最後の close で解放)。名前の解決は『完全一致 → 一覧で大文字小文字無視』。書きは最大 1 ブロックまで溜めて(write-behind)、読み・サイズ・close の前に書き出す。close は書き出して閉じるだけ(fsync は `FlushFileBuffers` と `flush_all` と `fle_opfs_flush`)。rename は置換の旗つき(既定は存在すれば EEXIST、`a.txt` → `A.txt` は許す)。 |
| `lift/harness/web/opfs1/fle_opfs_state.c`・`.h` | 状態ファイルを**全体で原子的に置き換える** `fle_opfs_state_put`(`.tmp` → fsync → close → `move`)と `fle_opfs_state_get`。act1 の R4(レジストリの部分木 `registrations.bin`)と R3/R5(`/fl/state`、`/fl/license`)の置き場。 |
| `lift/harness/web/opfs1/fle_opfs_mount.c`・`.h` | **ゲストのパス(UTF-16)→ OPFS のパス(UTF-8)** の変換と、マウント表(最大 16、最長一致)。純粋な C(OS もブラウザも要らない)。`fle_vfs.c` には**まだつないでいない**。規則は §3。 |
| `lift/harness/web/opfs1/opfs_test.c`・`opfs-test.html`・`opfs-test-worker.js` | ライブラリのテスト(68 件)と計測。 |
| `lift/harness/web/opfs1/bench.html`・`bench-worker.js`・`jspi.wat` | 同期アクセス・JSPI・SAB 橋渡しの計測。 |
| `app/test/e2e/opfs-lib-test.mjs`・`opfs-bench.mjs` | 上の 2 つを Chrome(CDP)と Firefox(geckodriver)で動かす。 |
| `lift/crates/fe-lift/tests/wall_opfs1_map.rs` | パス変換の native テスト(125 件)。 |

ライブラリのテスト 68 件の内訳(Chrome と Firefox で同じ): 順次・ランダムの読み書きのバイト一致、ブロックを跨ぐ読み、1 ハンドルの共有と参照数、開いたままの rename/remove が EBUSY、truncate、300 ファイルの作成と一覧、日本語のパス、大文字小文字を無視した stat/open/list/remove/rename、置換つき rename、`flush_all`、状態ファイル(置換・空・開いているファイルへの put が EBUSY で古い内容を保つ・3 MiB)、Worker を殺しての永続(下の §6)。

## 3. パス規則(`fle_opfs_mount.h` に書いた Windows の規則。Learn『Naming Files, Paths, and Namespaces』による**モデル**で、実 Windows との照合は 0 件)

- 区切りは `\` と `/` の両方。連続した区切りは 1 つ。`.` と `..` は**マウントの照合の前に**解決する(`C:\a\..\b` は `C:\b`)ので、パスはマウントの外へ出られない(外へ出たら別のパスになるだけ)。
- マウントの照合は**成分ごと・大文字小文字を無視**(`Image-Line2` は `Image-Line` に合わない)。最長のマウントが勝つ。文字の折りたたみは ASCII・Latin-1・ギリシャ・キリル・全角ラテンだけ(S-opfs1-casefold-table)。
- 成分の終わりの `.` と空白は捨てる(Win32)。`...` や空白だけの成分は不正。`< > : " | ? *` と制御文字は不正。1 成分は 255 UTF-16 単位まで。パス全体は 259 まで。`\\?\` の接頭辞があれば 32767 まで(この形では `.`・`..`・終わりの `.` は文字どおり。ただし `.` と `..` は OPFS に無い名前なので拒否)。
- デバイス名(`CON PRN AUX NUL COM1-9 LPT1-9`、拡張子つきでも)は、**マウントの下にあれば**ファイルではないと答える。マウントの外はボリュームの側で扱う(不正な文字や長すぎるパスも、マウントの外なら『マウントではない』を返す)。
- 孤立したサロゲートは不正。サロゲート対は 4 バイトの UTF-8 になる。
- **ルートをマウントしたとき、最上位の `fl` は見えない**(`FLE_OPFS_E_HIDDEN`)。`/fl/state`・`/fl/license`(act1 R3/R5)をエンジンやプラグインから触れなくするため。
- 逆変換 `fle_opfs_guest_path`(一覧の表示用)は、OPFS のパス → ゲストのパス。`Image-Line2` のような前方一致だけのものは拒否。

## 4. 配置の提案(`tables/guest-path-map-s3-proposal.tsv`。段階 2 の暫定対応を引き継ぐ)

- **`C:\Users\user\Downloads` = OPFS のルート `/`**(`fl` は見せない)。段階 2 の『OPFS の `/x/y` ↔ `C:\Users\user\Downloads\x\y`』と同じ対応なので、段階 2 の窓がエンジンに渡すパスは変わらず、**コピーが要らなくなる**(段階 2 は Open で隣のファイルを仮想ボリュームへコピーし、Save でページが書き戻していた)。act1 の R2(エンジンの既定の開始フォルダが `Downloads`)とも合う(S-opfs1-root-is-downloads。別のドライブ文字の `D:\` にもできる)。
- `C:\Users\user\Documents\Image-Line` → `/fl/C/Users/user/Documents/Image-Line`、`…\AppData\Roaming\Image-Line` → `/fl/C/Users/user/AppData/Roaming/Image-Line`(永続。設定・プリセット・プラグインデータベース。後者の実際のファイルは**エンジンの実行から観測していない**)。`Local` と `Temp` と `C:` のそれ以外は**メモリのボリュームのまま**(zip の一時展開などが OPFS に残らない)。
- ゲストのファイルではないもの: レジストリ `HKCU\Software\Image-Line\Registrations` は `/fl/state/registrations.bin`(`fle_opfs_state_put/get`)、認証のファイルは `/fl/license/`。書き出しのタイミング(`RegFlushKey`、終了時)は act1 の R4 のとおりで、`fle_reg.c` への配線は act1 側(未実施)。

## 5. 配線の設計(`tables/vfs-opfs-wiring.tsv`:Win32 の関数ごと)

エンジンの `fle_vfs.c` は**メモリ内の node の木**で、各 node が連続したバイト列 `data` を持つ(`CreateFileMappingW` が `fle_vfs_file_info` でその領域を受け取る)。OPFS をつなぐ 2 つのやり方:

- **設計 A(ストリーム)**: マウントの下の node は `data` を持たず、`ReadFile`/`WriteFile` ごとに `fle_opfs_read/write`(ブロックキャッシュつき)。メモリは軽いが、(1) Chrome のランダムな小さな読みは 1 回 約 0.63 ms(3 回の中央値。範囲 0.56〜0.79 ms)、(2) `CreateFileMappingW` は連続した領域を要るので結局まるごと読む、(3) 同期ハンドルを開きっぱなしにするので Files 窓から消せず他のタブとも競合する、(4) `DeleteFileW` が使用中のファイルで失敗する。
- **設計 B(開くときに読み込み、閉じるときに書き戻す)**: node が OPFS の『キャッシュ』。最初の `CreateFileW` で OPFS からバイト列を全部読んで `data` に置き、最後のハンドルを閉じたら(変更があれば)書き戻して `data` を捨てる。`ReadFile`/`WriteFile`/`SetFilePointer`/`CreateFileMappingW`/共有モードの今の動きは**変わらない**。同期ハンドルは読み書きの間だけ開くので、ロックの問題がほとんど消える。費用は、開くときに『ファイルの大きさ ÷ 順次読みの速度』(62.7 MB の zip なら、測定した Chrome の順次読み 506 MiB/s〔最小 170〕で 0.12〜0.35 秒)と、開いている間ファイルの大きさ分のヒープ。**今のメモリボリュームは書いたファイルを閉じても残すので、B は閉じたファイルのメモリが戻る分、メモリは今より減る**〔読み〕。大きすぎるファイル(限界値は決めていない)だけ A にする余地を残す。

**勧め: B から始める**(S-opfs1-design-B)。理由は上の 4 点と、`fle_vfs.c` の変更が『node の生成・開く・閉じる・列挙・作成・削除・名前変更の 7 か所に OPFS 呼び出しを足す』に収まること。`FindFirstFileW` は名前だけを取って絞る。`GetFileAttributesW` はまず node のキャッシュ、無ければ OPFS に聞く(§1 の費用: 存在しない名前が多い)。OPFS の操作は全部、Win32 呼び出しの中で JSPI の中断になる(段階 2 のファイルダイアログが同じ仕組みで実エンジン上で動いている〔実測〕)。タブを閉じるとエンジンが終了処理に入らないので、**タイマーで `fle_opfs_flush_all` と変更のある node の書き戻しを定期的に呼ぶ**(間隔は未決定。S-opfs1-flush-timer)。

## 6. 永続(実測、Chrome と Firefox で同じ)

- `write` → `flush` したが `close` していないハンドルを持つ Worker をページから `terminate()` し、ページを再読み込みしても、**中身は残る**。`write` → `close`(fsync なし)も残る。**ライブラリの書き溜め(write-behind)バッファにある分は失われる**(このテストで確認:ディスク上の大きさ 0)。=『タイマーの flush』が要る理由。
- 状態ファイルの `put` が **`.tmp` を書いたあと `move` の前**で Worker を殺されても、**古い状態が残る**(テスト)。
- **テストしていない**: 電源断・ブラウザ自体の強制終了・ディスクが一杯のとき(`QuotaExceededError` は ENOSPC に写すがエンジンへ流していない)・`persist()` が拒否されたときのブラウザによる消去(act1 の R5 の想定。Files 窓は `/fl/state`・`/fl/license` の削除に確認を出す〔段階 2 で実装〕が、ブラウザが消すのは止められない)。

## 7. 同期ハンドルの排他(実測)と複数タブ

同一ページ内(Chrome と Firefox): ハンドルを開いている間、同じファイルの 2 つ目の `createSyncAccessHandle`・`createWritable`・`removeEntry`・`move` は `NoModificationAllowedError`、`getFile()` は読める。close の後は全部通る。**設計 B なら、エンジンが読み書きしている短い間だけ**この状態になる。2 つのタブが同じ OPFS を開いたときの挙動は**測っていない**(同じ起源の別タブも同じロックを見るはずだが、未確認)。

## 8. スレッド

エンジンのゲストのスレッドは、『1 本目以外は自分の JSPI スタックで動く協調スレッド』で、全部 **1 つの Worker の中**にある(決定 0124、`fle_thread_wasm.h` を読んだ〔読み〕)。したがって OPFS のライブラリはその Worker で動き、`Atomics.wait` の橋渡し(測定表の費用)は**要らない**見込み。プラグインの Worker(別の Worker で動く)が自分のファイルを OPFS から読む場合は別の話で、今回は扱っていない。ライブラリを『ゲストの 2 本目のスレッド』から呼ぶテストは、配線が要るので**未実施**。

## 9. メモリ

ライブラリは開いているファイル 1 つにつき、JS 側に最大 256 KiB の書きバッファと 256 KiB の読みの窓を持つ(wasm のヒープではない)。設計 B ではライブラリのキャッシュを使わず(ファイルを全部ヒープに置く)、それはボリュームの今の方式と同じで、**ヒープの使用量は『開いているファイルの合計』になる**。上限(`fle_vfs.c` の 256 MiB/ファイル、384 MiB)は今のまま。

## 10. act1 の要件 R1〜R6 との対応(`lift/out-git/act1-results/MERGE-NOTES.md`)

| 要件 | 状態 | 根拠 |
|---|---|---|
| R1 保存モードでエンジンの値をそのまま使い、保存したファイルを取り出せる | 段階 2 で実装(提案名・フィルター・既定の拡張子・上書き確認。Files 窓から Download)。**実エンジンの『Unlock with file』の保存は未実行**(実鍵・実フローは依頼者が要る) | 段階 2 の Save as(実エンジン)と同じ経路〔実測〕 |
| R2 開くモードで OPFS のファイルとローカルのアップロードの両方から選べる | 段階 2 で実装(窓の中に Upload。取り消しはエンジンに『取り消し』を返す) | 段階 2 の Open / Import(実エンジン)〔実測〕 |
| R3 検証ファイルと鍵をサーバー・ビーコン・ログに載せない | ビーコンの `files` ブロックは数と固定の失敗コードだけ(名前・パス・内容なし)。ページのログも名前を書かない。`/fl/license` は同期もプレビューもしない | 段階 2 の `files-usage` テスト(プライバシーの検査)〔実測〕 |
| R4 レジストリの永続(`registrations.bin`、`/fl/state/`) | **置き場と読み書き(`fle_opfs_state_put/get`)を用意し、Worker 殺しのテストまで通した。`fle_reg.c` への読み込み(入口の前)と書き出し(`RegFlushKey`・定期)の配線は未実施**。対象の部分木の確定は実鍵の通しが要る(act1) | 本段階のテスト〔実測〕 |
| R5 『全消去』等が `/fl/state`・`/fl/license` を黙って消さない、`persist()` を要求 | 段階 2 で実装(削除・名前変更・移動で警告つきの確認)。`persist()` の要求は段階 1 から。**ブラウザが許可しない場合の消去は止められない**(§6) | 段階 1・2 のテスト〔実測〕 |
| R6 同名の保存で上書きの確認 | 段階 2 で実装(`FOS_OVERWRITEPROMPT` → 『Replace the file?』、実エンジンで確認) | 段階 2 の Save as〔実測〕 |

## 11. 未確認と次の手

1. **`fle_vfs.c` への配線**(設計 B の 7 か所)。dec1(決定 0206)が同じファイルを触ったので、その merge 後に別の差分で。実エンジンで『Open → 隣のファイルが D 無しで見える』『Save がコピー無しで OPFS に出る』を確認する。
2. エンジンの**実際のアクセスの型**を観測する(`CreateFileW`/`ReadFile` の順序と大きさ)。A か B かの最終判断はこれが要る。
3. 設定とレジストリを永続させて **Welcome の窓が出なくなるか**(`FLE_REG_SEED` による機械識別の固定が前提:act1)。
4. 2 タブ、ディスク一杯、`persist()` 拒否の挙動。
5. 大文字小文字の折りたたみを NTFS の `$UpCase` に合わせるか(今は部分)。
6. `zip-route-comparison.tsv` の DelZip64 の列は、dec1 が merge された(9a7975a)ので**dec1 の native -O0 の測定を引用**して更新した(私は測っていない。ブラウザのバンドルに DelZip64 が入るのは ship3 の後)。名前の復元は、dec1 が Python で『zip の 0x7075 の名前 = FLP の名前』をコードポイント単位で確認しており、私の結論(48/48)と一致する。

## 12. 再現

```
# ブラウザで測る(Chrome は CDP、Firefox は geckodriver)
cd app; export APP_WORK=<HOME 以下の作業ディレクトリ>
node test/e2e/opfs-bench.mjs --browser chrome|firefox --out bench.json
# ライブラリのテスト(emscripten でモジュールを作ってから)
cd lift/harness/web/opfs1 && emcc opfs_test.c fle_opfs_state.c -O1 -sJSPI -sSHARED_MEMORY=1 -sINITIAL_MEMORY=268435456 -sENVIRONMENT=worker -sMODULARIZE -sEXPORT_NAME=createOpfsTest --js-library fle_opfs_lib.js -o <build>/opfs_test.js
OPFS_BUILD=<build> node app/test/e2e/opfs-lib-test.mjs --browser chrome|firefox --out lib.txt
# パス変換(native)
cd lift && cargo test --offline -p fe-lift --test wall_opfs1_map
# 集計
node tools/opfs-s3-summary.mjs logs/s3 > tables/opfs-s3-measurements.tsv
```

## 13. 出典なしの選択

`tables/s-choices-s3.tsv`(12 件:S-opfs1-root-is-downloads、data-folders、design-B、no-fsync-on-close、casefold-fallback、casefold-table、reserved-fl、block-cache、state-put、device-names、long-paths、flush-timer)。

---

## PM の決定と取り込み(2026-11-04)

- **取り込み**: `opfs1-s3.diff`(新規 15 ファイル、`-` 行 0、どのビルドも変えない)を HEAD 0235dff に当てた。clean(offset・fuzz なし)。PM 実測: `wall_opfs1_map` と `wall_opfs1` の Rust テスト 通過(4 + 1)。段階 3 は `app/src` に触れないので vitest・e2e は再実行していない。担当の訂正どおり `wall_opfs1`(4 件)は担当が配置を移した後に再実行しておらず、PM の再実行で通った。
- **この段階で実エンジンは動かしていない(担当の申告どおり)**: Chrome 156 と Firefox 158 の専用 Worker で本物の OPFS を同期アクセス(createSyncAccessHandle)で測り、作ったライブラリを emscripten(-sJSPI)の C テストプログラムから 68 件呼んだ。エンジンの実際のアクセスの型は観測していない。したがって『エンジンが OPFS をファイルシステムとして使える』とはまだ言えない。
- **設計 B を採る**: 『開くときに全部読み、閉じるときに書き戻す』。`fle_vfs.c` のノード(キャッシュ)はそのままで、ReadFile / CreateFileMappingW は変わらず、同期ハンドルを開きっぱなしにしない(他タブ・Files 窓と競合しない)。配置は `C:\Users\user\Downloads` = OPFS のルート(最上位の `fl` は見せない。`/fl/state`・`/fl/license` は保護)。Chrome の 1 回の呼び出しが Firefox の 4〜15 倍重い(ランダム 4 KiB 読み 630 µs 対 42 µs)ことが、設計 B の根拠。
- **配線(次の作業)**: `fle_vfs.c` への接続、`lift/harness/web/opfs1/` から `lift/runtime/` への移動(移動すると次のバンドルが変わるので ship の流れで行う)、act1 の R4(設定・レジストリ・ライセンスファイルの永続化)の `fle_reg.c` への接続。エンジンの実際のファイルアクセスの型の観測が先。
- **未確認**: 電源断・ブラウザの強制終了、ディスク一杯、persist() 拒否時の消去、2 タブ、ゲストの 2 本目のスレッドからの OPFS 呼び出し、設定を永続させたとき Welcome ウィザードが出なくなるか、依頼者の Firefox 159 / Windows 10(0)。
