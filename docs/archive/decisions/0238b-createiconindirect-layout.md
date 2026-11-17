# 0238b: Chrome の起動直後の `CreateIconIndirect: hbmColor is NULL and the height of hbmMask is odd` の名前つき停止は、ハイブリッドの直接 wasm に焼き込んだ番地が C 部分の番地とずれていたビルドの事故(hyb/ を共有した同時ビルド)であり、実行時の欠陥でも『ゲストが静的配置に依存する』のでもない — 原因確定、ビルドの安全網を追加(load2)

状態: **決定**(原因は実測で確定した。4 本のバンドルで予測が当たり、対照実験が通った。残る未確認は §6)。決定 0238 の結論(API セットの契約)はこの調査に依存しない。

## 1. 冒頭の三点

1. **何を実測したか**: 実エンジン(F2xH6、HEAD ba85d14b + ship4-h6.diff + load2、CI の段スクリプトで build、出荷形 `NO_FS=1`)を隔離 Chrome 156 で起動し(ページだけ、プロジェクトなし。`tools/boot-n.sh`)、エンジンが止まるかを数えた。この機械、load 2〜37、依頼者の機械ではない。
2. **結論**:
   - 停止したバンドル c4 は、**直接 wasm(hybrid の direct modules)に焼き込まれた `fle_teb`・`fle_mxcsr`・`fle_hyb_buf`・`fle_progress_count` の 4 つの番地が、同じバンドルの C 部分(`flengine6.c.opt.wasm`)が使う番地と違っていた**。ハイブリッド段(`stage_hybrid`、build-engine-bundle.sh)は 4 つの番地を `$HYB/link.map` から読んで直接 wasm に焼き込む。c4 の直接 wasm は兄弟ビルド(tAfix)の番地(c4 の C 部分より +0x40)を持っていた。直接コードは C 部分とは別の変数を読み書きする(MXCSR 1,234 回、hyb_buf 2,333 回)。
   - **対照実験**: c4 と**同一の C 部分**に、C 部分が実際に使う番地で直接 wasm を作り直して統合し直したバンドル c5(`tools/rehybrid.sh`、hyb/ と web/ は私有コピー)は、**Chrome で 8/8 起動して停止 0**(load 12.2〜17.4)。c4 は同じ C 部分で **6/6 停止**(load 9.5〜10.4)。差は直接 wasm の番地だけ。
   - 原因は**ビルド方式**: 診断用に作った `tools/iter.sh` は作業ディレクトリ `ciH6base` を `cp -al`(ハードリンク)で複製し、`hyb/`(`link.map`、`dir/n-*.wasm`、`direct.wasm`)が全ビルドで**同じ inode**(リンク数 5)だった。重い build を 2 本同時に走らせたので、兄弟が `link.map` と `n-*.wasm` を上書きし、先に走っていたビルドが兄弟の `link.map` を読んだ。CI の段スクリプトは 1 ビルド 1 ワークツリーなので、この事故は起きない(§5)。ランタイムの欠陥ではなく、**ゲストが配置に依存するわけでもない**(決定 0238 の本文と、コミット 4977340a のメッセージの『the guest depends on the static layout of the runtime』は誤り。PM が訂正する)。
3. **安全網**を足した(§5): 直接 wasm の番地が C 部分でも同じ番地として使われていることと、link.map が C 部分を作った link のものであることを、hybrid 段で検査し、食い違えばビルドを失敗させる。**c4 を実際に失敗させる**(§5 の表)。出荷 bundle の語義は変えない(検査だけで、wasm のバイトは変わらない: §5)。

## 2. 実測(Chrome 156、ページだけ、`tools/boot-n.sh`、`tables/boot-*.tsv`、`tables/icon-stop.tsv`)

| バンドル | 起動数 | 停止 | 焼き込んだ fle_teb | C 部分が使う fle_teb | 備考 |
|---|---|---|---|---|---|
| c3 | 20 | 0 | 0x1091ba50 | 0x1091ba50(一致) | クリーン。MXCSR の参照: C 部分 823、統合後 2,057 |
| tAfix | 6 | 0 | 0x1091ba90 | 0x1091ba90(一致) | tA 木 + アラインメント修正。823 / 2,057 |
| **c4** | 6 | **6** | 0x1091ba90 | **0x1091ba50**(**不一致**) | クリーン + アラインメント修正。C 部分で焼き込んだ番地の参照は 0 回。統合後の参照は MXCSR(0x10205534)1,234 回・hyb_buf(0x1091daa0)2,333 回 = **直接コードだけが使う** |
| **c5** | 8 | **0** | 0x1091ba50 | 0x1091ba50(一致) | c4 と同じ C 部分(`cmp` で同一)、直接 wasm だけ作り直し。823 / 2,057 |

- 予測『焼き込み = C 部分の番地 ⇔ 止まらない』は、**今も残っている 4 本(c3・tAfix・c4・c5)で 4/4 当たる**。c4 は『ほぼ同じ木(c4 = c3 + アラインメント修正〈停止と無関係な小さな変更〉)で c3 は 20/20 通り、c4 は 6/6 止まる』という矛盾に見えた対で、差の原因が番地だった。
- 測定の道具: `tools/chk-baked.sh WORKDIR…`(バンドルの `engine/logs/hyb-addresses.txt` の番地が C 部分で `i32.const` として何回使われるか。読み取りだけ)、`tools/scan-addr.py`、`tables/baked-check.txt`。リポジトリの他の作業ディレクトリで hyb-addresses.txt と C 部分が揃っていた 12 本(out-rt1 ciA5・ciA5-n・ciA7〈最初の測定〉・ciA7-n・ciB7・ciB7-n、out-ship6 ci9・ci9fs・ci9m・ci-h6m・ci-h7m〈F2xH7〉、out-mt2 b3-s2)は**全て一致**(焼き込んだ番地が C 部分で 1〜828 回使われる。`tables/baked-check.txt`、`tables/baked-check-more.txt`)。ci9・ci9fs と ciA5-n・ciA7・ciA7-n・ciB7-n は hyb/link.map を共有している(リンク数 2〜3)が、食い違いはなかった(理由は調べていない: 配置が同じだったか、同時に走らなかったか)。単独ビルドの CI は問題ない。
- 停止時の ICONINFO(tA の `fle_stop` で 4 語を出した): `{ fIcon = 0, hot = (0, 1), hbmMask = 0x7fba00bc, hbmColor = NULL }` = 幅 0・高さ 0 のカーソルの形。直接コードが壊れた変数(hyb_buf・MXCSR)の値で、ILTools_x64.dll の 0x625ee30 のカーソル作成に渡るグリフの境界 RECT が 0 のままだった、という読みと矛盾しない(RECT が 0 になる経路は**直接は追っていない**: §6)。

## 3. 以前の観察が説明できるか

| 以前の観察 | 説明 |
|---|---|
| 走らない経路(`fle_stop` の先頭)に stderr の 1 行を足すだけで 5/5 停止(tA) | 1 行でランタイムのデータ配置が 0x40 動く(tAfix の C 部分で実測: fle_teb が +0x40)。tA が読んだ link.map は兄弟(配置が違う)のもので、焼き込みが C 部分と食い違った(推定。tA は消してあり、C 部分を確認できない) |
| 診断を足した別の bundle(t5〜t7)は止まらない | 兄弟と配置が同じだったか、同時ビルドでなかった(推定) |
| tP1(+40 B)・tP2(+4104 B)の未使用大域変数でも止まらない | 2 本は同時に build、焼き込みは両方とも 0x1091ba50(c3 と同じ)。配置が実際に動いたか C 部分を確認できない(消してある) |
| ほぼ同じ木で c3 は 20/20 通り、c4(= c3 + アラインメント修正)は 6/6 止まる | **c4 は実測で不一致、c5 で解消**(§2) |
| タイミング・CPU 固定・スタックのゼロ埋め・ランタイム変数の初期化では変わらない(tZ0/tZ1/tCp は 0/5、tUz は 5/5) | 全てバンドル単位で決定的(実測)。tUz と tCp は兄弟(焼き込みは両方 0x1091ba10)で、どちらか片方の C 部分が違ったと考えれば合う(推定) |

つまり『バンドルごとに決定的で、ソースの小さな変更で入れ替わる』という観察は、『ビルドごとに、兄弟が上書きした link.map を読んだか』で説明できる(c4 は実測、他は推定)。**消してあるバンドル(t3・tA・tUz・tP1・tP2・tZ0・tZ1・tCp・t5〜t7)の C 部分の番地は確認していない**(推定)。

## 4. 副産物: 契約の点検で見つけた違反(停止の原因ではない)

- HeapAlloc は 16 バイト整列を約束する(Learn: 『The alignment of memory returned by HeapAlloc is MEMORY_ALLOCATION_ALIGNMENT』、x64 で 16)。ランタイムは host の malloc を使い、Emscripten の malloc は 8 か 0 mod 16 を返す(**2,000 ブロックのうち 986 が 16 整列でない**、`tools/align-wasm.sh`、`fle_guest_alloc` は 0/2000)。HeapAlloc・HeapReAlloc・GlobalAlloc・LocalAlloc・CoTaskMemAlloc を `aligned_alloc(16)` に直した(`align.diff`、テスト `load2_align.rs`、全体テスト 195 結果 / 2,382 通過 / 0 失敗)。**これは停止の原因ではない**(c4 = クリーン + この修正は止まり、c5 = c4 と同じ C 部分で直接 wasm の番地だけ直したものは通る)。別の差分(`align.diff`)にして、load2.diff に混ぜていない。
- 点検して違反が見つからなかったもの: VirtualAlloc の 64 KB 粒度とゼロ埋め、スレッドスタックのゼロ埋め(tZ0 と tZ1 は同じ結果)。

## 5. 安全網(`load2-guard.diff`。HEAD 4977340a に dry-run 通過。HEAD 52034ca9 の checkout には実際に当てて `ci/test-hybrid.sh` と `bash -n` が通る。未コミットのワークツリーにも dry-run 通過)

`app/scripts/build-engine-bundle.sh` に関数 `hyb_guard`(record / map / cpart / merged の 4 つの検査。中は python)を足し、weblink 段と hybrid 段が呼ぶ。`ci/plan.sh` の SP_WASM の関数の一覧に `hyb_guard` を足した。**ファイルでなく関数にした理由**: `lift/harness` に新しいファイルを足すと `H_GEN`(`git ls-files -s -- lift/harness`)が変わり、**lift 段の鍵(`k_lift`)が変わって CI の lift 段が作り直しになる**。スクリプトの先頭のコメントも全部の鍵の前置き(`script_preamble`)に入るので触っていない。

| 段 | 検査 | 失敗の条件 |
|---|---|---|
| `weblink`(link の直後) | `record`: link.map の sha256 を `$WEB/link.map.sha256`(C 部分の隣。ハードリンクされる `hyb/` の外)に記録 | — |
| `hybrid` の開始 | `map`: 今の link.map の sha256 が記録と同じか | 違う(別のビルドが上書きした) |
| 同 | `cpart`: 4 つの番地が C 部分(`flengine6.c.opt.wasm`)の `i32.const` として 1 回以上使われるか | 1 つでも 0 回 |
| 直接 wasm を作った後、統合の前 | `map` をもう一度 | 作っている間に link.map が変わった |
| 統合の後(`merged.wasm`) | `merged`: 統合後の module が fle_mxcsr と fle_hyb_buf を C 部分より多く使うか(直接コードは 1,234 / 2,333 回使う) | 増えていない(`hyb/dir/n-*.wasm` が別の番地のものに差し替わった) |

記録がないとき(古い作業ディレクトリ、weblink を飛ばした部分実行)は『検査できない』という注記を出し、失敗にしない(`cpart` は必ず効く)。

**効くこと(実測)**:
- `ci/test-hybrid.sh` に 13 のテストを足した。合成した wasm(`0x41 + sleb128` を 16 進で書き下し、guard の関数に頼らない)で: 通る場合、兄弟の配置(+0x40)の C 部分、逆の食い違い、1 つの番地が使われていない場合、直接コードが別の配置のもの、link.map の上書き、記録なし、build-engine-bundle.sh が 4 つの検査を呼んでいること(hybrid 段に 4 回、weblink 段に 1 回)、`ci/plan.sh` の SP_WASM に `hyb_guard` が入っていること、`lift/harness` に新しいファイルがないこと。**全て通る**(`bash ci/test-hybrid.sh` → all checks hold)。
- **変異 4/4 を殺す**: `cpart` が失敗しない(3 テストが落ちる)、`map` が失敗しない(1)、`merged` が失敗しない(1)、hybrid 段が `cpart` を呼ばない(1)。`tables/guard-mutation.tsv`。
- **実物に対して**: c4 の C 部分に対する `cpart` は **失敗**(4 つとも 0 回)。c3・tAfix・c5 は通る。他の 6 本(out-rt1 ciA5・ciA7-n・ciB7、out-ship6 ci9・ci9m・ci-h7m〈F2xH7〉)は `cpart`・`merged` とも**通る**(偽陽性 0。ciA7 は持ち主が作り直し中で測れなかった)。`tables/guard-on-bundles.txt`、`tables/guard-on-other-bundles.txt`。
- **ビルドの段で**(`tools/rehybrid.sh`、ガード入りの木 `src-g`〈最終の load2-guard.diff を当てた木〉、c4 と同じ C 部分): 兄弟の番地(0x1091ba90…)を与えると数秒で `hybrid failed`(c9。`cpart` が 4 つとも 0 回と報告)。記録を別の link.map のものにすると `map` で失敗(c10)。正しい番地と正しい記録では最後まで通る(c11: §5.1)。ログは `logs/rehybrid-c*.log`。
- **CI の鍵**(`ci/plan.sh` を F2xH7 で、ガードのない HEAD 4977340a の木とガード入りの木で走らせて比べた): `k_lift`・`k_extract`・`k_felift`・`k_seed`・`k_b2d`・`k_app`・`k_vst3`・`k_emsdk`・`cfg_sha` は**変わらない**。`k_rt`・`k_link`・`k_bundle` と bundle の接頭辞(`fl-engine-ci-F2xH7-…`)は**変わる**(SP_WASM がスクリプトの関数 `stage_weblink`・`stage_hybrid`・`hyb_guard` の本文を含むため。CI では rt の objects・link・bundle の段が 1 回作り直しになる。ガードを足せば必ずそうなる)。link.map の内容そのものは鍵の入力ではない(link の出力で、鍵は入力から決まる)。P_LINK のキャッシュは `flengine6.opt.wasm`・`.js`・`.wasm` の 3 つで、link.map と `hyb/dir` は入っていない。キャッシュヒットなら hybrid 段は走らず、ミスなら weblink から全部走る = link.map と C 部分は同じ実行で作られる。**部分実行**(weblink を飛ばして hybrid だけ)だけが事故の入口で、そこを `map` と `cpart` が守る。

### 5.1 出荷 bundle の語義は変わらない

ガードは検査の呼び出しだけで、生成物を作る行は変えていない。実測: ガード入りのスクリプト(c11)とガードなしのスクリプト(c5)で、同じ入力(c4 の C 部分、正しい番地)から hybrid 段 + seed + bundle をやり直し、**`flengine6.opt.wasm` が byte 単位で同一(sha256 40e325e8af5b…、146,033,664 B)**、グルー `flengine6.js` も同一、bundle の `SHA256SUMS` は `manifest.json` を除いて全行同一(manifest.json は作成時刻・HEAD・鍵が変わった接頭辞の URL が違うだけ)。c11 は `link.map.sha256` を記録し、`map`(2 回)・`cpart`・`merged` を通った(`logs/rehybrid-c11.log`)。**F2xH7 の wasm の sha256 8b2f575c… との比較は、F2xH7 を作り直していないのでしていない**(重い build で load の条件に合わない。PM が F2xH7 の CI で確認すること)。代わりに F2xH7 の作業ディレクトリ(out-ship6 ci-h7m、sha256 7503c58c…)に対してガードが通ることを実測した(`tables/guard-on-other-bundles.txt`)。

## 6. 未確認

- 消してあるバンドル(§3)が、実際に焼き込みと C 部分で食い違っていたか。**c4 だけ**を実測した(+ 通るバンドルの一致 3 本)。
- RECT が 0 になった経路をゲストの側で追っていない(直接コードが壊れた変数を読んだため、と読むのが自然だが、追っていない)。
- Firefox、node(JSPI)での再現(この停止はビルドの事故なので、期待はしない)。
- F2xH7 の再ビルドで sha256 8b2f575c… が変わらないこと(§5.1)。

## 7. 再現と確認

- **5 秒で確認**: `tools/chk-baked.sh WORKDIR…`(読み取りだけ)。出力の `refs in the C part` が 0 なら、そのバンドルの直接コードは別の配置の番地を使っている。
- **作り直し**: `tools/rehybrid.sh SRC_WORK NEW_WORK TREE TEB MXCSR HYBBUF PROGRESS`(hyb/ と web/ を私有コピーにして hybrid 段 + seed + bundle をやり直す)。
- **iter.sh の方式を使うなら**: `hyb/` と `emcache/` を `cp -a`(ハードリンクにしない)、重い build を同じ `hyb/` 起点で 2 本同時に走らせない。
- **対照実験 c5 の再現**: (1) 停止するバンドル(c4)の作業ディレクトリから `tools/rehybrid.sh c4 c5 TREE 1091ba50 102054f4 1091da60 10308188`(番地は `tools/scan-addr.py` で C 部分の参照が最も多い値を探して決める。`tools/chk-baked.sh` の `refs in the C part` が 0 でない番地)。(2) `FL_ASSETS_DIR=assets-c5` で `vite preview --outDir dist-c5`(dist-c5 = `app-run/dist` + assets のシンボリックリンク)を起動し、`tools/boot-n.sh PORT c5 8`(Chrome 156 を 8 回、`tables/boot-c5.tsv`)。
- テスト: `bash ci/test-hybrid.sh`(`load2-guard.diff` を当てた木で、13 テストが足される)。
