# Sytrus の止まり(#1429): Reverb_x64.dll を実モジュールにし、データ表の関数を seed にした

判定: **受理**(受け入れコマンドが 0 で終わった。道具の riff-machine は止まらず、窓が出た)。前回の記録 `sytrus-1429-reverb.md` の原因(`Shared\Reverb_x64.dll` が読めず `LoadLibraryW` が NULL を返す)は、直した。直した後に出た次の止まり(Reverb の表の関数が lift の関数の一覧に無い)も、直した。Reverb の他の関数の欠落が、後の操作で出る可能性は未確認。

runner の値。この機械とは別枠。nproc 4。riff の JSON の loadavg は 1.93 1.57 1.87。作業の開始 18:54:25 UTC。engine の作り直し 20:02 → 20:54(約 52 分)。

## 1. 原因の連鎖(実測・逆アセンブル。番地は Reverb の VA と guest の両方)

Reverb_x64.dll の像: ImageBase 0x180000000(`objdump -p`)、SizeOfImage 0x7e000。guest の base は 0xcba0000(`lift/out-app/plugins/plugins.tsv`)。guest = 0xcba0000 + (VA − 0x180000000)。

| 段 | 結果 | 根拠 |
|---|---|---|
| 1. 前回まで | `LoadLibraryW 126 "Reverb_x64.dll"`(NULL)。Sytrus の `+0x28` が 0 のまま読まれる | `sytrus-1429-reverb.md` の §3(前回の実測) |
| 2. 直し(1) | `lift/plugins/reverb/plugin.tsv`(key `reverb_x64.dll`・補助・`Shared\Reverb_x64.dll`)。ci の Reverb の行を外す。`validate`(plugins.rs・plugin-rows.mjs)が `Shared\file` を許す。seed の先が `inputs/install/` の直下になる。Plugin database は補助の folder を取らない | `lift/out-app/plugins/vfs-seed-idle1.tsv`・`modules.tsv` に `reverb_x64.dll`・`shared/reverb_x64.dll` が出た |
| 3. 次の止まり(直し(1)の後) | `fle: indirect call to 0xcbb7ef0 is not a known function entry (caller ?, near ?)`(終了コード 134)。`LoadLibraryW 126` は無い | 実測(この作業の run 2)。RVA 0x17ef0 |
| 4. 直し(2) | `lift/plugins/reverb/seeds.tsv`(28 RVA)。生成の段が base + RVA の VA にして `--seeds` で lift・list・hybrid に渡す | 実測(run 3)で止まりが消えた |

### 3.1 次の止まりの番地(実測と objdump)

- 0xcbb7ef0 = 0xcba0000 + RVA 0x17ef0(VA 0x180017ef0)。Reverb の像の中。値は 0xcc の埋めでも 0 でも NULL の import でもない。**前回の分類 (i)〜(iii) のどれにも当たらない。**
- VA 0x180017ef0 の関数(objdump):
  ```
  180017ef0: lea    0x5b709(%rip),%rax    # 0x180073600
  180017ef7: mov    %rax,0x5cb62(%rip)    # 0x180074a60
  180017efe: mov    $0x1,%al
  180017f00: ret
  ```
- この番地の参照(実測):
  - `.pdata` に、この関数の行は無い(RVA 0x17ef0 を始まりに持つ行が 0 件)。
  - 直接の `call`・`lea`(`# 0x180017ef0`)の参照は無い。
  - 8 バイトの絶対値として `0x180017ef0` が在るのは 1 か所だけ。ファイル位置 0x61a90(VA 0x1800628d0)。
- 読み元の表(実測): VA 0x1800628d0 から 16 バイト刻みの 16 件(0x1800629d0 まで)。各件は `{関数の番地, 0}`。先頭は 0x180017ef0、次は 0x180017f48(`.pdata` の行あり)。
- 表を指す命令(実測): `0x1800180b7`・`0x18001810d` の `lea %rcx,0x1800628d0(%rip)`(直前の `lea %rdx,0x1800629d0(%rip)` と組)。前者の関数(0x1800180b0)は `jmp 0x180045acc`、後者の関数(0x180018106)は `jmp 0x180045b60`(回る関数の違い。未確認)。
- 表を回す関数 0x180045acc(実測): `[rcx, rdx)` を 0x10 刻みで回し、先頭の値が 0 でないものごとに、`.rdata` の 0x1800602e0 の関数ポインタを間接に呼ぶ。最後の `call *%rax` のどの経路で 0x180017ef0 が呼ばれたかは、**未確認**(推定: この回る関数の先のどこかで、表の値を `call` する)。

### 3.2 分類(実測と推定の区別)

- (i) lift が 0xcc の埋めを読んだ: **該当しない**(値は Reverb の有効な関数の番地)。
- (ii) 初期化されていないデータ: **該当しない**(値は表に実際に書かれている)。
- (iii) 解決されていない外部(NULL): **該当しない**(前回の直しで読み込みは通った)。
- **(iv) 新しい分類(この作業で見つけた): lift の関数の一覧に、データの表からだけ参照される関数が無い。** 関数に `.pdata` の行が無く、直接の `call` も無いため、fe-lift・fe-wasmgen の入口(`.pdata`・呼び出し先・thunk の形)から漏れた。実測の証拠: `fe-wasmgen` の報告(`lift/out-app/plugins/work/reverb_x64.dll.tsv`、直し(2)の前の版)に 0xcbb7ef0 の行が無い。直し(2)の後は、同じ報告で `listed 1630, emitted ok 1630`。
- 比べ: `.rdata`・`.data` の中で `.text` を指す 8 バイトの値は 88 件。`.pdata` の関数の先頭に無いものは 33 件(重複を除いて 28 の RVA)。この 28 件を seed にした。

## 2. 直し方(既定の動作。オプトインを作らない)

- `lift/plugins/reverb/plugin.tsv`: `reverb_x64.dll`、`Shared\Reverb_x64.dll`、補助、sha256 892423bc…、492544 B、hook `none`、args `--pdata`、omit `-`。
- `lift/plugins/reverb/seeds.tsv`(新しい形式。key と RVA)。28 行。各行は、§3.2 の表・vtable から参照される小さな関数の先頭(番地の前が `ret`・`int3`・0x00 の埋め)。
- `lift/crates/fe-build/src/plugins.rs`:
  - `parse_seeds`・`read_seeds_from`(seeds.tsv の検査: 見出し・2 列・16 進)。
  - `generate(true)`: 各 DLL の base が決まった後、RVA + base の VA を `OUT_DIR/seeds/<key>.txt` に書き、args に `--seeds <file>` を足す。その args は modules.tsv・hyb-args.tsv・plugins.tsv の全てに入る。
  - seeds の key が plugin の key に無ければ停止。
  - 検査: `validate` が `Shared\file`(2 部分)を許す。`seed_file` の親が無い時に `//` を作らない。`plugin_db_names` は補助だけの folder を飛ばし、本体の guest に親の folder が無ければ停止。
- `app/scripts/lib/plugin-rows.mjs`: 同じ検査と、seed の先(`inputs/install/<key>`)。
- `ci/extract-dlls.tsv`: Reverb の 2 行を外した(同じ DLL の二重の抽出を避けた)。
- `inputs/plugin-image-sizes.tsv`: Reverb の 1 行を足した(SizeOfImage 516,096 B。PE ヘッダから読んだ値)。このファイルは `tools/plugin-layout/image-sizes.mjs` が `inputs/plugin-classes.tsv` から作る。Shared の DLL は後者に無いので、手で足した(記録しておく)。
- テスト: `fe-build` の単体テスト 28 件、`spec/plugin-rows.test.mjs`、`spec/workflow.test.sh`(r8 を plugin.tsv 側の検査に替えた)、`spec/guard.test.sh` が通る。

## 3. 受け入れ(run 3。製品の変更の後)

道具の出力(runner の値。この機械とは別枠):

- 起動確認(boot-bundle): `state` engine-idle・`standIn` false・`error` null・`elapsedMs` 12,299(P1 の 10 秒は未達)。
- 道具の 1 行(DPR 1): `error` null・`engine` engine-idle・`stopped` false・`windowNew` に `Piano roll - riff machine`(334x506、`visible` true、`painted` true)・`stopLines` 11 行(`not a known function entry` は 0 件)・`elapsedMs` 11,134。
- 流れ A〜D(`welcome-flow.mjs`、4 行): A 11,856 ms・B 11,645 ms・C 11,568 ms・D 11,649 ms。4 行とも `error` null・engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。`preBootBytes` 313,484,200(上限 314,368,711 以内。前回の 313,279,997 から、Reverb の 492,544 B の分だけ増えた)。
- 単体テスト(`npm test --prefix app`): `Test Files 75 passed | 1 skipped (76)`、`Tests 1465 passed | 10 skipped (1475)`。

engine の段(抜粋): `list reverb_x64.dll: emitted ok 1630, left out 0 (omit 0), listed 1630`。

## 4. 止まりの直前の 40 行(前回の止まり)

前回の止まり(`fle: indirect call to 0 ...`)の直前の行は、`sytrus-1429-reverb.md` の §2 に在る。今回の run 3 では止まらなかったので、直前の行は無い。run 2(次の止まり 0xcbb7ef0)の `stopLines` の末尾の 5 行は、次の通り(この順)。

```
fruity: == stop: SIGABRT (runtime abort, see runtime message)
fruity: runtime message: first non-background pixel on a drawing surface (decision 0126): written by UpdateLayeredWindowIndirect into the surface of top-level window lid 8; function entries since the program began: 0
fruity: exit code 134 (the codes of harness/wasm/trace_native.c: 134 SIGABRT, ...)
fle: regs R[0]=0x63736d65 R[1]=0x0 ... R[15]=0x0
fle: indirect call to 0xcbb7ef0 is not a known function entry (caller ?, near ?)
```

`fle: regs` の値は、前回の止まりと同じ(`R[0]=0x63736d65`)。レジスタの値は止まりの位置を示さない(前回の記録と同じ見方)。

## 5. 未確認・残り

- 0x180045acc の回る関数から、0x180017ef0 を呼ぶ最後の `call` の場所は、確かめていない(推定)。
- 表の関数を全て seed にしたが、lift の一般の欠落(データの表からだけ参照される関数)は、製品全体の問題として残る。直し(2)は Reverb だけの明示の行で、他の plugin の同じ型の欠落は、それぞれの seeds.tsv で直す。全体の直し(`.rdata`・`.data` の code pointer を全て seed にする)は、lift の持ち主の判断。
- `inputs/plugin-image-sizes.tsv` の Reverb の行は手で足した(道具の入力に Shared の DLL が無い)。
- ローカルの木の作業: `lift/out-app/host/lift/out-idle1/inputs/install/reverb_x64.dll` を、抽出済みの `lift/out-app/host/lift/out-plugins/reverb_x64.dll`(sha256 892423bc… を確認)から複写した。抽出の段は、この作業の木(HEAD の鍵)には Reverb の行が無いので、seed の実物を作らない。次の run の抽出は、plugin.tsv の行から同じ bytes を作る(`extract-fl.mjs --seed` が sha256 を検査する)。
- P1(起動 10 秒)は未達(runner の 12.3 秒)。今回の変更は起動時間を増やしていない(前回の 12,289 ms と同じ程度)。
- `WavpackDLL_x64.dll`・`ilvideo_x64.dll` の `LoadLibraryW 126` は、触っていない(範囲外)。#1395 の ILSvg は、key が bare 名でないため触っていない。
- #1430 の起動条件(この Issue の close)は、受け入れが通ったので満たす。

## 6. この作業(run 4)の受け入れと、止まりの確認(実測)

判定: **受理**(受け入れの各段が 0 で終わった。道具の riff-machine は止まらず、窓が visible・painted で出た)。ただし bundle の段は、抽出の木に Reverb の seed の実物が無いため、その 1 ファイルを抽出済みの `lift/out-app/host/lift/out-plugins/reverb_x64.dll` から複写してから通った(両方の sha256 が 892423bc… であることを確かめた。木は gitignore の対象で、commit には入らない)。plugin.tsv が main に入った後の次の run は、抽出の鍵の変化で seed の実物が作られ、この複写は要らない。

runner の値。この機械とは別枠。nproc 4。道具の JSON の loadavg 1.88・1.52・1.71。作業の開始 21:59:43 UTC。engine の作り直し 22:01:53 → 22:27:17(約 26 分)。bundle 22:27:33 → 22:39:39(約 12 分。seed の複写の後)。

| 段 | 結果 |
|---|---|
| 起動確認(boot-bundle) | state engine-idle・standIn false・error null・elapsedMs 8,112 |
| 道具の 1 行(DPR 1) | error null・engine engine-idle・stopped false・windowNew の `TPRScoreCreatorForm`「Piano roll - riff machine」334×506 が visible・painted・stopLines 11 行(`not a known function entry` は 0 件)・elapsedMs 7,949 |
| 流れ A〜D | A 7,891 ms・B 7,771・C 7,809・D 7,752。4 行とも error null・engine-idle・standIn false・trapped false・hostErrors 0・opened true。B と D は welcomeClosed true。preBootBytes 305,944,758(上限 314,368,711 以内) |
| 単体テスト(npm test) | Test Files 75 passed \| 1 skipped (76)・Tests 1465 passed \| 10 skipped (1475) |
| fe-build の単体テスト(cargo test -p fe-build) | 29 passed |

### 6.1 止まりが消えたことの確認(同じ道具を --log 付きで 1 回走らせた)

- 前回までの止まり: `missing-path: LoadLibraryW 126 "Reverb_x64.dll"`(§3.1 の実測)。今回のログには 0 件。
- 代わりに、ログの 677 行: `vfs seed: C:\Program Files\Image-Line\FL Studio 21\Shared\Reverb_x64.dll <- /data/seed/lift/out-idle1/inputs/install/reverb_x64.dll (492544 bytes ... sha256 892423bc…)`。4566 行: `real module table: name=reverb_x64.dll base=0xcf60000`。
- 残りの `LoadLibraryW 126`: `""`(4499・4511 行。出どころは未確認)、`WavpackDLL_x64.dll`(4527 行)、`ffmpeg\x64\ilvideo_x64.dll`(4533 行。範囲外)。
- `not a known function entry` の行: 0 件。道具の `stopped` は false。

### 6.2 原因の分類と根拠(前回の記録の確認。objdump は Sytrus の DLL に対して取った)

- 分類: **(iii) 解決されていない外部**(実行時の `LoadLibraryW` が NULL を返した)。(i)(ii) には当たらない。
- 読み元(ImageBase 0x400000):
  ```
  8404d3: mov    0x28(%rbx),%rax     ; this+0x28 を読む
  8404e3: mov    (%rax),%rbx
  8404e6: call   *0x10(%rbx)         ; 止まりの呼び出し(戻り番地 8404e9)
  83f4cc: call   0x5bc1a0            ; LoadLibraryW "Reverb_x64.dll" の包み
  83f4dc: je     0x83f537            ; NULL なら、+0x28 を埋めずに戻る
  83f4f6: mov    %rax,0x28(%rbx)     ; 埋める命令(NULL のときは通らない)
  ```
- 戻り番地: guest 0xc9c04e9。Sytrus の base 0xc580000 のとき RVA は **0x4404e9**(0xc9c04e9 − 0xc580000。VMA 0x8404e9 と一致)。Issue 本文の「RVA 0x5404e9 付近(推定)」は誤りだった。
- 止まりの値: 前回は `+0x28 = 0` のまま読まれ、`indirect call to 0`(§3.4)。0xcc の埋めでも、上位 32 ビットの値でもない。
- 対照: 今回は `Reverb_x64.dll` が読め、実モジュールが table に載り、止まりが消えた。`+0x28` の値そのものは、今回は読んでいない(未確認)。

### 6.3 #1395 との比較(記録による)

- #1395 の止まりは `fle_addr_hi32`(`guest address 0x3280983d0 (8-byte load) has bits above 32 set`。`pm-work/issue-drafts/wave1ad/154-language-start.md`)。64 ビットの値を番地として読む型で、NULL ではない。
- この止まりは NULL(0)を呼んだもの。型は別。ただし #1395 の本文は、この機械では読めなかった(`gh` が認証されていない)。本文との照合は未確認。

### 6.4 この作業での製品の変更

- 製品のコードは、前回の差分(§2 の直し)のまま。この作業で変えたのは、前回の差分を取り込む時の衝突の解消だけ(`lift/crates/fe-build/src/plugins.rs` のテスト 3 か所)。Fruity Filter(main)と Reverb(前回)の両方を残した。読み込みの数は 163(Balance・Filter・Sytrus・Reverb と wave の 159)。wave の合計は Filter を除き、Reverb の 524,288 B を足した(748,158,976 + 524,288)。

### 6.5 未確認・残り

- 直前の 40 行: 今回は止まらないので無い(前回の止まりの直前の行は §2 にある)。
- `LoadLibraryW 126 ""` の出どころは未確認。`WavpackDLL_x64.dll`・`ilvideo_x64.dll` は範囲外(#1395 の ILSvg と同じく、この作業では触らない)。
- #1430 の起動条件(この Issue の close)は、受け入れが通ったので満たす。
