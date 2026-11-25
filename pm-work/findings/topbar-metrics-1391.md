# topbar-metrics-1391: 上部バーの Memory Usage(段階 1)

実エンジン: `lift/target/release/fe-build engine`・`fe-build bundle`・`tools/boot-check/boot-bundle.mjs`・`tools/topbar-metrics/measure.mjs`(2 回。2 回目は `--shots` で画面を保存)・`tools/smoke/welcome-flow.mjs`(流れ A〜D)・`npm test --prefix app`・`cargo test --workspace --no-fail-fast`(作業ツリーと HEAD)を実行し、出力を観測した。測定の値は runner の値。この機械とは別枠。nproc 4、loadavg 2.60 1.88 1.92(測定の後)。

判定: **受理**(受け入れの式が通った)。ただし (ii)(画素の変化の時刻の対応)は、方式が band のため判定に入っていない。画面との照合は 1 回だけ(下の 4)。

## 1. 直す前の値の出どころ(前回までの記録)

| 値 | FL が呼んだ API と戻り値 | 画面の表示 |
|---|---|---|
| Memory Usage | `GetProcAddress(kernel32.dll, "K32GetProcessMemoryInfo")` が 0(無い) | 上段右「0 MB」 |
| CPU Load | `GetSystemTimes`・`GetProcessTimes`・`GetThreadTimes` は名前で引かれていない | 下段右「0」(段階 2) |
| Frame Rate | `DwmFlush` が仮想の 60 Hz で呼ばれる(仮想の時計) | 「120」(段階 2) |

## 2. 製品の変更

- `lift/runtime/fle_sysinfo.c`(前回までの差分のまま): `K32GetProcessMemoryInfo`。使用量 = コミットした VirtualAlloc のページ + 生きている HeapAlloc のブロック(`fle_sys_in_use_bytes`)+ 読み込んだ像の SizeOfImage の合計(`fle_vmap_image_bytes`。定数は無い)。呼ぶたびに engine の log へ `memory-usage: K32GetProcessMemoryInfo WorkingSetSize <B>` を 1 行出す(測定の道具だけが読む)。
- `lift/crates/fe-decode`・`lift/crates/fe-lift`(前回までの差分のまま): 書き込み可能な領域の内容を定数としない(遅延読み込みの枠が自分自身を呼ぶ閉路を止めた)。K32 の 1 行は `imports_table.rs` の末尾。
- `app/src/engine/worker.ts`(`guestText`。読み取りの道具): 近傍の文字列を、場所(`NEAR_PLACES` = 400)まで、文字コードごとに数える(前回は 24 件で、UTF-16 の表示に届かなかった)。最良の文字列は従来どおり先頭 `TEXT_HITS`(64)の場所から取る(重い走査を増やさない)。
- 仮想の時計・画面の周期は変えていない。音声への影響は無い。製品に、新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数は足していない。

## 3. 測定(`tools/topbar-metrics/measure.mjs`)

- 標本: Welcome 窓が閉じた後から、1 秒ごと 40 個。各標本で、K32 の回数と最後の値(engine の log)、「N MB」の候補(ASCII と UTF-16LE)、上部バーの帯の画素(desk の上端から 96 CSS px の PNG の sha256)。
- `memory.candidates`: 全標本と、K32 の後の最後の読み取りの和(重複なし・整数)。
- `memory.zeroMbNew`: 「0 MB」の候補で、K32 の最初の呼び出しの後の標本に現れた場所の数(その前の標本には無いもの)。
- `memory.shownMb`: 最後の K32 の MiB の floor が候補に在ればその値。無ければ、±1 の候補がちょうど 1 つの時だけその値。どちらでも無ければ null(推定しない)。
- `memory.hash`: `method` は "band"(上部バーの帯。矩形は取っていない)。`series`・`floors` は 40 個ずつ出す。

## 4. 結果(runner の値)

| 実行 | K32 の回数 | 最後の K32 | floor | candidates | shownMb | zeroMbNew | 窓の中の floor |
|---|---|---|---|---|---|---|---|
| 1(受け入れ) | 98 | 173,790,612 B | 165 | 165, 166, 167 | 165 | 0 | 165(40 個とも) |
| 2(`--shots`) | 96 | 173,790,612 B | 165 | 165, 167 | 165 | 0 | 167(40 個とも)。最後の読み取りは 165 |

- 画面(実行 2 の、標本の後の画面): 上部バーの右上に「165 MB」、その下に「0」(CPU Load。段階 2)。K32 の最後の値の floor(165)と一致した。
- 参考(判定に入れない): `memory.guess`(最後に動いた候補)は 166。画面と一致しない。`memory.pageMeasure` は 5,666,018,378 B(`performance.measureUserAgentSpecificMemory`)、比は 0.031。

## 5. 未確認

- (ii) 画素の対応: 方式が band のため判定に入っていない。両実行とも、40 秒の窓の中で floor が 1 回も変わらなかったので、時刻の対応は確かめていない(rect の方式でも同じ)。実行 2 では、窓の後に floor が 167 から 165 へ変わった(変わった時刻は未確認)。
- 表示の時刻: 実行 2 では、標本の最後の K32 は 167 だが、標本の後の画面は 165 だった。K32 の変化が標本の後に起きたのか、ページの log の見え方が遅れたのかは分からない。
- `shownMb` の規則: 候補に floor が在れば floor を返す。過去の表示の写し(166・167)が残るため、「候補に在る」は「表示している」の証拠としては弱い。画面との照合は 1 回だけ(実行 2)。
- 起動: `boot-bundle` の `elapsedMs` は 12,255 ms(P1 の 10 秒は超えている。流れ A〜D の上限 12,500 ms は満たす)。

## 6. 回帰

- `npm test --prefix app`: 71 files passed・1 skipped、1406 tests passed・10 skipped。
- 流れ A〜D(`welcome-flow.mjs`): 4 流れとも engine-idle・standIn false・trapped false・hostErrors 0・opened true。welcomeClosed(B・D)true。最大 elapsedMs 11,364(≤ 12,500)。preBootBytes 311,108,406(≤ 314,385,095)。
- `cargo test --workspace --no-fail-fast`(lift):
  - 作業ツリー: 2023 件通過・439 件失敗。
  - HEAD(作業を stash で退避し、同じ試験を実行。後で戻した): 2021 件通過・439 件失敗。
  - 失敗の名前の一覧は両方で同一(diff 0 件)。増えた失敗は無い。
  - 失敗の原因(両方の共通): 試験の link に無い runtime のファイル(`fle_win_api_trace_on`・`fle_clip_host_read`・`fle_clip_host_write` の未定義参照)、mingw が無い、python が無い、ファイルが無い。
  - 差分の 2 件増(通過)は、作業ツリー側の新しい試験。
- 前回記録の「HEAD 270 件」とは数が違う(この runner では 439 件が HEAD で落ちる。環境の差。未確認)。

## 7. 注意

- この作業では Python を使っていない(node・シェル・cargo のみ)。画面は保存した PNG を目で確かめた(OCR は使っていない)。
- 一時ファイルは `.tmp-*`(リポジトリの中)に置き、終わったら消した。受け入れの出力と stash の退避は `$RUNNER_TEMP` に置いた。
