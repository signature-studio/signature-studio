# 0312j: 『Split mixer tracks が WriteFile: out of memory で失敗する』(線形メモリ 2 GiB の上限)の設計の読み — 書いたファイルをどこに持つか(mt3。PM 指示 00:41。仮。**読みだけ。実装・build・Chrome は 0**)

## 0. 冒頭

1. **事実(file:line、実測 = コードを読んだ)**: 仮想 volume のファイルの中身は**C の heap(wasm の線形メモリ)にある**。`fle_vfs.c:59〜70` の `fle_node` が `uint8_t *data; uint64_t size, cap;`、書き込みは `fle_imp_WriteFile`(`:668〜700`)→ `node_reserve`(`:162〜176`)で `realloc` し、**容量は倍々**(`nc = cap*2 > need ? cap*2 : need`)で、**新しい領域を `memset` で 0 にする**。1 ファイルの上限は 256 MiB(`:52` `MAX_FILE_BYTES`)。`realloc` が NULL なら `fle_stop(api, "out of memory")`(`:172`)= 報告された『WriteFile: out of memory』。**線形メモリの上限は `-sMAXIMUM_MEMORY=2147483648`(2 GiB)、初期 640 MiB**(`lift/harness/web/fruity/link-web.sh:23`)。wasm の線形メモリは縮まない。
2. **算術(読み)**: 47 トラック × 22.8 MB = **1,072 MB**(実際の大きさ)。ところが容量は倍々なので、容量は実際の大きさの 1〜2 倍(書き方次第。**64 KB 刻みで書くなら 22.8 MB のファイルの容量は 64 KB × 2^9 = 32 MiB = 33.55 MB**、1.47 倍)= 47 × 33.55 MB = **1,577 MB**(FL の書き方は未確認で、この数は 64 KB 刻みの場合の読み)。基底 約 1 GB(PM の数値)+ 1,577 MB = **約 2.58 GB > 2.147 GB** で失敗する。**実際の大きさのままでも 約 2.07 GB で、2 GiB の余白は 約 70 MB しかない**(= 倍々の無駄を除いただけでは足りない。記憶の上限を変えるか、メモリの外に出す必要がある)。
3. **推薦(1 つ)**: **(b) 外部 API の拡充として、仮想 volume のファイルの中身を host の store(OPFS)に持てるようにし、最後の `CloseHandle` で線形メモリから追い出す**(方式 A1)。**既存の host files の境界(POSIX の syscall をページが答える層、`app/src/engine/hostfiles.ts` の読み取り専用 pack)を書き込みに拡げる**。memory64(A3)は保留、wasm32 の 4 GiB(A2)は主案にしない。先に**小さな安全策 A0**(書き終えたら容量を実際の大きさに縮める、倍々を止める)を入れてよい。
4. **区分**: 『FL の挙動の代行・補正』ではなく、**kernel32 のファイル I/O の契約(`CreateFileW`/`WriteFile`/`ReadFile`/`SetFilePointer`/`CloseHandle`…)の実装の裏の記憶領域の置き場所**の拡充 = **(b) 外部 API の拡充**。guest から見える戻り値・副作用・エラーは変えない。新しい旗・環境変数なし。**閾値 1 つ(下限)は『実装の定数』であって利用者の選択肢でない**(`MAX_FILE_BYTES` と同じ種類)。

## 1. 現状の事実(file:line)

| 項目 | 場所 | 内容 |
|---|---|---|
| ファイルの node | `fle_vfs.c:59〜70` | `data`・`size`・`cap`。volume の木は heap の malloc |
| 成長 | `fle_vfs.c:162〜176` `node_reserve` | 倍々 + `memset(nd + n->cap, 0, nc - n->cap)`(**新しい領域を全部触る**)、256 MiB 上限(`:52,165,170`)、`realloc` 失敗で `fle_stop("… out of memory")`(`:172`) |
| 書き込み | `fle_vfs.c:668〜700` `fle_imp_WriteFile` | `node_reserve` → `memcpy(node->data + at, …)`。位置は `e->pos`(Seek で戻れる= **WAV の頭の size 欄の書き戻し**はここ) |
| 読み | `fle_vfs.c:657` | `memcpy(…, node->data + pos, k)` |
| 切り詰め | `fle_vfs.c:181〜183,752〜753` | `node_truncate`、SetEndOfFile の 0 埋め |
| コピー | `fle_vfs.c:1426` | `CopyFileW`: `memcpy(dn->data, sn->data, size)`(**ファイル 2 つ分の記憶**) |
| **`data` ポインタを外へ渡す所(中身が線形メモリにあることを前提)** | `fle_vfs.c:723` `fle_vfs_file_info` → **`fle_vmap.c:721`(`CreateFileMappingW`: 読み取り専用の map〔`PAGE_READWRITE` は未実装で stop〕)**、`fle_vol.c:83`(GetFileType)、`fle_wall_ia2v.c:175,322`(ini の読み)。`fle_vfs.c:2594` `fle_vfs_peek_file` → **`fle_load.c:210`(`LoadLibrary` が画像の中身を見る)**。`fle_vfs.c:2651〜2690` `fle_vfs_put_file`/`fle_vfs_file_state` → **`fle_filedlg.c:895`(Save 対象をページに渡す。`fruity.ts:201〜215` `pollSaved` が `heap.slice(at, at+size)` で複写)** | 15 か所の `->data`(`grep -c`) |
| 線形メモリ | `link-web.sh:23` | `-sINITIAL_MEMORY=671088640 -sMAXIMUM_MEMORY=2147483648`、`-sFILESYSTEM=0` |
| host files の境界 | `app/src/engine/hostfiles.ts:1〜15` | 『エンジンは -sFILESYSTEM=0。**ページが POSIX の syscall 〔`__syscall_openat`・`fd_pread`・`fd_seek`・`fd_close`・stat 系〕に自分で答える**』。**読み取り専用 pack**。それ以外(書き込みの open など)は『エンジンを名前つきで止める』(`HostFilesUnsupported`) |
| 既存の『出す』道 | `worker.ts:245〜251` `pollSavedFiles`、`fle_filedlg.c:860〜906` | Save の対象は idle で複写して page に渡す(ダウンロード)。volume には残る |

## 2. 案の比較

| 案 | 内容 | 容量への効き | 契約・区分 | No-options | P1 | Export の sha | 費用・リスク |
|---|---|---|---|---|---|---|---|
| **A0** | 最後の `CloseHandle` で `realloc(data, size)` に縮める。倍々をやめ(係数 1.25 または固定チャンク)、`memset` は新規部分だけ | **約 −0.5 GB**(1,577 → 1,072 MB)。**2 GiB の余白は約 70 MB に増えるだけで足りない可能性**(基底が 1 GB 超なら失敗) | (b) 実装の記憶の効率。契約不変 | 旗なし | 影響なし | 影響なし(中身は同じ) | 小(`fle_vfs.c` の 2 関数)。**単独では不十分**でも、他案の前提(1 つの開いたファイルが倍々で 2 倍になる無駄を消す) |
| **A1(推薦)** | **host の store(OPFS の `FileSystemSyncAccessHandle`)にファイルの中身を持つ**。`fle_node` の `data` を『store の key + 小さな書き込みキャッシュ』にする。開いて書いている間は線形メモリ(または chunk)、**最後の `CloseHandle` で host に書き出して `free`**。Read/Seek/Truncate/Copy/Delete は store を通す。map(`CreateFileMappingW`)・LoadLibrary・Save の複写は、必要なときだけ host から線形メモリに読み戻す(map 中は pin) | **線形メモリには開いている 1 ファイル分しか載らない**(47 トラックでも約 + 33.55 MB)。ファイルの上限 256 MiB も外せる(host の容量まで) | (b)。Win32 の契約は同じ。**host files の POSIX 境界の書き込み版**(`__syscall_openat` の `O_CREAT`、`fd_pwrite`、`ftruncate`、`unlinkat`) | 旗なし。**下限(これより小さいファイルは追い出さない)は実装の定数**で、0 にすれば完全に一様(**一様の方が分岐が少ない**。小さいファイルの I/O の費用が出るので測ってから) | **I/O Worker を最初の追い出しまで作らない**(起動の経路に載らない)。first idle に影響しない見込み | 影響なし(下の検証) | 中〜大: ① `OPFS` の `SyncAccessHandle` は**作った Worker の中でしか使えず、同じ file に同時に 1 つ**(FL の export は guest スレッドのいる Worker 上で走る)→ 専用の I/O Worker に集約し、guest スレッドは JSPI で await する RPC(`EM_ASYNC_JS`)。② fle_vfs.c の 15 か所の `->data` を store 経由に(`vfs_store_read/write/truncate/free` の薄い層)。③ native の試験は memory 実装のまま(層の裏を差し替える)。④ 後始末(OPFS の掃除、容量の上限・quota)。⑤ ページへ渡す経路は、**host の File を直接 `Blob`/stream にでき、23 MB の `heap.slice` の複写も無くなる** |
| **A2** | wasm32 のまま `-sMAXIMUM_MEMORY=4GB` | 2 GiB → 4 GiB。基底 1 GB + 1,577 MB(A0 なしでも)は収まる | 契約不変。ただし区分は『資源の上限を上げるだけ』 | 旗なし | 影響なし(初期メモリを変えなければ) | 影響なし | **JS 側の pointer の符号付きシフト**(`HEAPF64[(tot >> 3) + i]`〔`fle_audio_transport.c` の `fle_js_pulse`〕、`Int32Array(…, addr >> 2)`〔`fle_thread_wasm.c:135〜`〕ほか EM_JS/lib の全部)が 2 GiB 以上で壊れる → 全部の監査が要る。**RAM を 4 GiB 近く使える機械が前提**(依頼者の機械は 0 件)。guest 側の 4 GiB 近くの address は FL の 32 bit 前提(`(uint32_t)` のキャスト)に当たる可能性 |
| **A3** | memory64(`-sMEMORY64`) | 2 GiB 超(16 GiB まで) | 契約不変 | 旗なし | wasm のコードが大きくなる(64 bit のポインタ)、**bounds check が guard page で省けなくなり遅くなる〔読み〕**。first idle・P2/P3 に響く危険 | 影響の確認が全体の再 build | 大: 全 runtime の `uintptr_t` が 64 bit、`R[]` の layout、pthread + JSPI の組み合わせの実績なし。ブラウザの対応(Chrome 133+、Firefox 134+、Safari は未〔2026-11 時点の私の知識では不確実〕)。**保留** |
| A4 | File System Access API(`showDirectoryPicker`)で利用者のフォルダへ直接書く | host に出る | 契約不変。ただし『利用者のフォルダ』は Chrome 系のみ | 旗なし(利用者の操作は要る) | 影響なし | 影響なし | OPFS より可搬性が低い。A1 の出力の先として後で足せる |

## 3. A1 の設計の読み(差分の単位)

- **U1 `vfs_store` の層**(`fle_vfs.c`): `store_read(node, off, buf, n)`・`store_write`・`store_truncate`・`store_release(node)`・`store_materialize(node, &data)`。memory 実装は今の `data` のまま。**native と試験は memory 実装**。
- **U2 追い出しの規則**: `CloseHandle` で `opens == 0` かつ書き込みがあった node を host に flush して `free(data)`。**map 中・LoadLibrary 中・`put_file` の seed はそのまま(線形メモリ)**。読み戻し(`store_materialize`)は map/Save のときだけ。
- **U3 host 側**(`app/src/engine/hostfiles.ts` の拡張 + I/O Worker): `__syscall_openat`(`O_CREAT|O_WRONLY`)・`fd_pwrite`・`__syscall_ftruncate64`・`__syscall_unlinkat`・`fd_close` を OPFS に。host files の既存の方針(『知らない呼びは名前つきで止める』)を保つ。
- **U4 Save の経路**: `pollSavedFiles`(`worker.ts:245`)が host の File を直接 `transfer` する(線形メモリを経由しない)。
- **順序**: A0(単独で入れられる)→ U1(memory 実装のまま層だけ。既定出力は不変、native 試験が通ること)→ U2/U3 → U4。

## 4. 検証(実装後)

1. **Export の sha**: alpha1 の node harness で同じ FLP の Export を、変更の前後で取り、**WAV の sha256 が `381cdb5acf891a160e0286aab6b4822d08f661bcfd4f44f4f7184eda00d1f979` のまま**(`FLE_PROCESSORS=1` で n を固定)。Export の書き込みは `WriteFile` 経由で、書き戻し(WAV の頭の size 欄)を含む → **Seek して頭を書き換える経路**が host store で同じか。
2. **Split のトラックの sha**: 収まる数(例 10 トラック)で A1 なしと A1 ありの全ファイルの sha256 が一致する。47 トラックで A1 ありが完走する。
3. **差分試験(native)**: 同じ Win32 のファイル操作の列(Create/Write/Seek/Read/SetEndOfFile/Copy/Move/Delete/Map)を memory 実装と host store の模擬(一時ファイル)の両方で走らせ、戻り値・GetLastError・読み戻しが一致する。既存の `fe-lift/tests/vfs.rs`(23 test)・`vol.rs`・`load2_files.rs` をそのまま両実装で。
4. **メモリ**: `worker_done` の mark の `memoryBytes`(`worker.ts:283`)で、線形メモリの高水位が 47 トラックで下がることを見る。
5. **P1**: first idle(A1 前後)。I/O Worker は最初の追い出しまで作らない。起動時に FL が書く小さなファイルは下限未満(または追い出し前)で影響しない。

## 5. 未確認・リスク(正直に)

- **FL が Split で書いたファイルを後で読み戻すか**、map するか(読み戻すなら host から読む費用が出る)。トラックのファイルが `CreateFile` を再 open するか(WAV の頭を最後に直す)は **コードを読んだだけで未確認**。
- 基底の『約 1 GB』の内訳(FL の heap、plugin の cache、sample の cache、ほか)は PM の数値で、私は測っていない。A0 が単独で足りるかは基底次第。
- OPFS の quota、Firefox/Safari の `createSyncAccessHandle` の可否(Safari 16.4+、Firefox 111+ の記憶〔不確実〕)、private browsing の制限。
- 利用者が 47 個のファイルをどう受け取るか(ダウンロード 47 回、zip の streaming、A4)は別の設計。
- pthread Worker が複数ある構成で、guest スレッドの JSPI の RPC が I/O を直列化して Export の時間が延びないか(未測定)。
- 倍々の無駄は `fle_vfs.c` 以外(`fle_node` の `kids` など)にもあるが、MB の桁ではない。


## PM の判断と取り込み(2026-11-07 00:47、docs のみ。製品の差分なし)

取り込み: mt3 の 0312j(「Split mixer tracks」の `WriteFile: out of memory` の設計の読み)。**コードを読んだだけで、実装・build・Chrome・依頼者の機械は 0 件**。事実(file:line、mt3 の読み): 仮想 volume のファイルの中身は線形メモリの heap(`fle_vfs.c:59〜70`、`node_reserve` が倍々 + memset〔`:162〜176`〕、上限 256 MiB〔`:52`〕、`:172` の `fle_stop("out of memory")`)。線形メモリは `-sINITIAL_MEMORY=640 MiB -sMAXIMUM_MEMORY=2 GiB`(`link-web.sh:23`)。47 トラック × 22.8 MB に倍々の余白を足すと約 2.58 GB で 2.147 GB を超え、倍々を除いても約 2.07 GB で余白は約 70 MB(読み)。

判断:
1. **方針は A1 を採る**: 外部 API(kernel32 のファイル I/O)の契約の裏の記憶領域を host の store(OPFS の SyncAccessHandle、専用 I/O Worker)に置き、最後の CloseHandle で線形メモリから追い出す。lift の忠実性の方針の区分は (b) 外部 API の拡充(guest から見える戻り値・副作用・エラーは変えない)。新しい旗・環境変数・FL の class/窓の分岐は足さない。A2(wasm32 の 4 GiB)は JS 側の符号付きシフトの全監査が要り、A3(memory64)は遅さと pthread + JSPI の実績なしで保留、A4(File System Access API)は A1 の出力先として後から。
2. **A0(閉じたら容量を実際の大きさに縮める・倍々を止める)は、次に runtime の C を触る版(E15 以降)に同乗させる**。単独では出さない(runtime の C の変更は全部 cold CI で約 60 分)。A0 だけで Split が通るかは 2.07 GB の見積りが余白 70 MB で確かでないので、通ると言わない。
3. **A1 の着手は G9-a の後**(G9 が G 段階表の次の段階で、Split は 8 plugin を鳴らした後も通る必要がある。plugin の分で基底が増えるので、先に A1 を入れる意味もある)。U1(`vfs_store` の層だけ、既定の出力不変)は低リスクなので、mt3 の空き時間に先に設計の差分まで。
4. 検証の条件: Export の sha(`381cdb5a…f979`)が不変(`FLE_PROCESSORS=1` で n を固定、Seek して WAV の頭の size 欄を書き戻す経路を含む)、Split の全ファイルの sha を収まる数で A1 の有無で比較、native の `fe-lift/tests/vfs.rs`(23 件)・`vol.rs`・`load2_files.rs` を両実装で、first idle(P1)が不変〔I/O Worker は最初の追い出しまで作らない〕。
5. 未確認: FL が Split のファイルを後で読み戻す/map するか、基底の約 1 GB の内訳、OPFS の quota と Firefox/Safari の `createSyncAccessHandle`、利用者が 47 個のファイルをどう受け取るか、複数 Worker の I/O 直列化による Export の時間。
