# Sytrus の止まり(#1429): 原因は、Sytrus が読む `Shared\Reverb_x64.dll` がビルドに無いこと

判定: **差し戻し**(受け入れは通らない)。原因は実測で確定した。直すには `Reverb_x64.dll` を実モジュールとして lift する必要があり、その抽出は main の行が要る(§5)。この作業では、製品のコードは変えていない。記録だけを commit する。

実エンジン: `tools/piano-roll-tools/measure.mjs --tool riff-machine --dprs 1 --log`(受け入れと同じ bundle。engine の作り直しから。終了コード 0 の測定)と、起動確認・流れ A〜D・単体テストを、受け入れの順に実行した。

runner の値。この機械とは別枠。nproc 4。riff の JSON の loadavg 1.77・1.47・1.77。作業の開始 16:27:48 UTC。engine の作り直し 16:36:43 → 17:13:25(37 分)。bundle 17:13:32 → 17:28:09(15 分)。

## 1. 受け入れの各段(実測。製品のコードは HEAD のまま)

| 段 | 結果 |
|---|---|
| 起動確認(boot-bundle) | `state` engine-idle・`standIn` false・`error` null・`elapsedMs` 12,289(P1 の 10 秒は未達。runner の値) |
| 道具の 1 行(DPR 1) | `error` null・`windowNew` の `TPRScoreCreatorForm` が visible・painted。**`stopped` true・`engine` exited・`stopLines` に `fle: indirect call to 0 is not a known function entry`**。判定は不合格(判定式の終了コード 1) |
| 流れ A〜D | A 11,671 ms・B 11,681・C 11,616・D 11,635。4 行とも `error` null・engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。`preBootBytes` の最大 313,279,997(上限 314,368,711 以内)。判定式は通った(終了コード 0) |
| 単体テスト(`npm test --prefix app`) | `Test Files 75 passed | 1 skipped (76)`・`Tests 1463 passed | 10 skipped (1473)`(終了コード 0) |

受け入れは、道具の 1 行で落ちる(止まりが残る)。

## 2. 止まりの行と、直前の 40 行(実測。`--log` の出力の行 5372〜5412)

```
missing-path: GetFileAttributesW 2 "C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\Arpeggiator\Default.fsc"
missing-path: GetFileAttributesW 3 ""
missing-path: GetFileAttributesW 3 ""
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Quantization\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Quantization\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\Quantization\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\Quantization\Default.fsc"
missing-path: GetFileAttributesW 2 "C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\Quantization\Default.fsc"
missing-path: GetFileAttributesW 2 "C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\Quantization\Default.fsc"
missing-path: GetFileAttributesW 3 ""
missing-path: GetFileAttributesW 3 ""
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Chopping\Riff machine step 1\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Chopping\Riff machine step 1\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\Chopping\Riff machine step 1\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\Chopping\Riff machine step 1\Default.fsc"
missing-path: FindFirstFileW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Chopping\Riff machine step 1\*.*"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\"
enum-dir: 24 "C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\Chopping\Riff machine step 1\*.*"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Chopping\Riff machine step 2\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Chopping\Riff machine step 2\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\Chopping\Riff machine step 2\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\Chopping\Riff machine step 2\Default.fsc"
missing-path: FindFirstFileW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Chopping\Riff machine step 2\*.*"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\"
enum-dir: 19 "C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\Chopping\Riff machine step 2\*.*"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Arpeggiator\Riff machine\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Arpeggiator\Riff machine\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\Arpeggiator\Riff machine\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\Arpeggiator\Riff machine\Default.fsc"
missing-path: FindFirstFileW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Arpeggiator\Riff machine\*.*"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\"
enum-dir: 40 "C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\Arpeggiator\Riff machine\*.*"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Quantization\Riff machine\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Quantization\Riff machine\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\Quantization\Riff machine\Default.fsc"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\Quantization\Riff machine\Default.fsc"
missing-path: FindFirstFileW 3 "C:\Users\user\Documents\Image-Line\FL Studio\Presets\Scores\Quantization\Riff machine\*.*"
missing-path: GetFileAttributesW 3 "C:\Users\user\Documents\Image-Line\Downloads\Scores\"
enum-dir: 9 "C:\Program Files\Image-Line\FL Studio 21\Data\Patches\Scores\Quantization\Riff machine\*.*"
fle: regs R[0]=0x63736d65 R[1]=0x0 R[2]=0x0 R[3]=0x0 R[4]=0x0 R[5]=0x0 R[6]=0x0 R[7]=0x0 R[8]=0x0 R[9]=0x0 R[10]=0x0 R[11]=0x0 R[12]=0x0 R[13]=0x0 R[14]=0x0 R[15]=0x0
fle: indirect call to 0 is not a known function entry (caller ?, near ?)
```

§3 の原因の行(LoadLibraryW)は、この 40 行より前(5288 行)にある。直前の 40 行は、その後の `GetFileAttributesW`・`enum-dir` の行。

## 3. 原因の連鎖(実測と、逆アセンブルで確かめた。番地は Sytrus の VMA)

### 3.1 実測(ログ。`riff-log.txt` の 5281〜5288 行)

```
missing-path: GetFileAttributesW 2 "C:\Users\user\Documents\Image-Line\Sytrus\Data\Maps"
missing-path: GetFileAttributesW 2 "C:\Users\user\Documents\Image-Line\Sytrus\Data\Default.dat"
missing-path: GetFileAttributesW 2 "C:\Users\user\Documents\Image-Line\Sytrus\Data\Default.dat"
missing-path: GetFileAttributesW 2 "C:\Program Files\Image-Line\FL Studio 21\Shared\Reverb_x64.dll"
missing-path: GetFileAttributesW 2 "Reverb_x64.dll"
missing-path: GetFileAttributesW 2 "C:\Program Files\Image-Line\FL Studio 21\Shared\Reverb_x64.dll"
missing-path: GetFileAttributesW 2 "C:\Program Files\Image-Line\FL Studio 21\Reverb_x64.dll"
missing-path: LoadLibraryW 126 "Reverb_x64.dll"
```

- `GetFileAttributesW 2 "C:\Program Files\Image-Line\FL Studio 21\Shared\Reverb_x64.dll"`(無い)。
- `GetFileAttributesW 2 "Reverb_x64.dll"`(無い)。
- `GetFileAttributesW 2 "C:\Program Files\Image-Line\FL Studio 21\Reverb_x64.dll"`(無い)。
- **`missing-path: LoadLibraryW 126 "Reverb_x64.dll"`**: 読み込みは失敗し、NULL と ERROR_MOD_NOT_FOUND(126)を返す(`lift/runtime/fle_load.c` の `not_found`)。

### 3.2 Sytrus の中の、+0x28 を埋める経路(逆アセンブル。`objdump -d` の VMA)

`this` は、`0x8402cf`(生成器)が `0x83f710`(基底のコンストラクタ)を呼ぶ。そのコンストラクタが `+0x28 = 0` を書き、続けて `0x83f4b0` を呼ぶ。

```
83f763: movq   $0x0,0x28(%rax)        ; 基底のコンストラクタ: this+0x28 = 0
83f77b: call   0x83f4b0
83f4b9: cmpq   $0x0,0x8(%rbx)         ; [this+8] が 0 のとき続く
83f4c0: lea    0x85(%rip),%rcx        ; 0x83f54c = L"Reverb_x64.dll"
83f4cc: call   0x5bc1a0               ; LoadLibrary の包み(下の 3.3)
83f4d1: mov    %rax,0x8(%rbx)         ; [this+8] = モジュール(NULL のとき 0)
83f4dc: je     0x83f537               ; NULL なら、ここで戻る(+0x28 は 0 のまま)
83f4e1: lea    0x82(%rip),%rdx        ; 0x83f56a = L"CreateReverbInstance"
83f4e8: call   0x421e20               ; GetProcAddress の包み
83f4f4: call   *%rax                  ; 生成関数を呼ぶ
83f4f6: mov    %rax,0x28(%rbx)        ; ここで初めて this+0x28 が埋まる
```

この連鎖の中で `+0x28` を書くのは、コンストラクタの 0 と、この `mov %rax,0x28(%rbx)` の 2 か所(DLL の他の箇所からの書込みは、全体を確かめていない。未確認)。埋める条件は「`Reverb_x64.dll` が読めて、`CreateReverbInstance` が取れる」こと。

### 3.3 `0x5bc1a0` の中身(IAT の番地から。推定の箇所は括弧内)

- `0x422260` → IAT `0x913ef0` = **SetDllDirectoryW**。
- `0x422280` → IAT `0x914238` = **SetErrorMode**。
- `0x422110` → IAT `0x9141b8` = **LoadLibraryW**。
- `0x448860` は、`SetErrorMode(0x8000)`・`LoadLibraryW`・`SetErrorMode(元)` の順で呼ぶ(推定: 引数の組み立ては `0x5bbe20`。その中で 3 か所の `GetFileAttributesW` を見る。§3.1 の 3 行)。

### 3.4 止まり

止まりの時の `this+0x28` は 0(`fruity: guest registers at the stop` の `rax=0`)。`0x8404d3` の `mov 0x28(%rbx),%rax` が NULL を読み、`0x8404e3` の `mov (%rax),%rbx` が guest の番地 0 の中身(`rbx=0x63736d65`)を読み、`0x8404e6` の `call *0x10(%rbx)` が `0x63736d75` の中身を呼ぶ。そこが 0 なので、`indirect call to 0`。

- 戻り番地 `0xc9c04e9`(`guest stack at rsp`)。Sytrus の像の base は `0xc580000`(`real module table`)、ImageBase は `0x400000`。VMA = guest − `0xc180000` なので、戻り番地の VMA は `0x8404e9`(`call *0x10(%rbx)` の直後)。RVA(ImageBase 基準)は `0x4404e9`。
- 止まりの値が回ごとに違う理由(推定): 値は guest の番地 0 の中身(`[0]`)から決まり、その中身は回ごとに違う(`0xcccc…`・`0x475c…`・`0x5eff…` と見えた理由)。

## 4. 分類(Issue の (i)〜(iii))

- (i) lift が 0xcc の埋めを読んだ: **該当しない**。読んだ値は NULL(0)から来る。
- (ii) データが初期化されていない: **該当しない**(書き手は正しい。書かれないのは、読み込みが失敗したため。コードの誤りではない)。
- (iii) 解決されていない外部(NULL): **該当する**。ただし import ではなく、実行時の `LoadLibraryW` で、ビルドに無い DLL が NULL を返した。

PM の仮説(別のゲストのスレッドが `+0x28` を設定する): **この止まりには当たらない**。`+0x28` を埋める経路(`0x83f4b0`)は、コンストラクタの連鎖の中で同期に走る。その途中の `LoadLibraryW` が失敗したことを、ログが示す(5288 行)。失敗の結果、埋める 1 か所(`0x83f4f6`)が通らないので、0 のまま読まれる。別のスレッドが書く必要は無い。

前回の記録の二重解放・ヒープの書換え・`lock cmpxchg %ah` は、この止まりの説明には要らない。別件として残す(未確認)。

## 5. 直し方(この作業では未実施)

原因: `Shared\Reverb_x64.dll`(インストーラの一覧 `inputs/fl-installer-files.tsv` の 26530 行。492,544 B。SHA-256 `892423bcce3d516eb42d303566e8a15a23a9ae9014fbc589c79afa542019ba40`。依存は kernel32.dll だけ)が、次のどこにも無い。

- ビルドの seed: `lift/harness/seeds/vfs-seed-idle1.tsv` に行が無い(`Reverb` の行は 2 件だが、どちらも Reverb Devices と Reverb send。DLL は無い)。
- 抽出: `ci/extract-dlls.tsv` に行が無い。`lift/plugins/` にも無い。
- 木: `lift/out-app/host` に無い(`find` で確かめた。ランナーのどこにも実物は無い)。

直すには、この DLL を実モジュールとして lift する。Sytrus(#1400)や Balance と同じ形で、`lift/plugins/reverb/plugin.tsv` に 1 行を足す。ただし、この作業ジョブでは直せない:

1. 抽出の木は、ジョブの最初の HEAD の鍵で R2 から取る(`agent.yml` の `extract` ジョブ。作業の段は `fail-on-cache-miss: true`)。この作業の HEAD には行が無いので、木に DLL は入らない。
2. 抽出の既定は `--plugins lift/plugins`(`app/scripts/extract-fl.mjs`)なので、plugin 行が main に入れば、次の `extract` ジョブが鍵の変化で DLL を取り出す。作業ジョブ自身は、インストーラを取れない(R2 の artifact の資格は作業ジョブに無い)。
3. judge は受け入れが通ったときだけ push する。受け入れは DLL が無いと通らない。よって、この行は main に入らない。**監査・受付の段で main に入れる**のが要る。

次の手(順番どおり):

1. 監査の段が `lift/plugins/reverb/plugin.tsv` を main に入れる。行の案(下の「案」)。
2. `extract` ジョブが再抽出し、engine が Reverb を実モジュールとして lift する。`fe-build engine` のログで、hook・omit・args を確かめる。
3. `/retry`。受け入れを通し、Sytrus が `CreateReverbInstance` を得るかを見る。得ても、次の止まりが出る可能性があり、未確認。

案(未確認の値は、lift の結果で決める):

```
key	guest	role	sha256	size	hook	args	omit
reverb_x64.dll	Shared\Reverb_x64.dll	本体	892423bcce3d516eb42d303566e8a15a23a9ae9014fbc589c79afa542019ba40	492544	none	--pdata	
```

- `key` を `reverb_x64.dll` にする理由: `load_by_name`(`fle_load.c`)は、名前を `reverb_x64.dll` に正規化してから `fle_realmod_find`(キーと完全一致)で探す。キーを `shared.reverb_x64.dll` のように付けると、名前の照合では届かない(ILSvg の流儀とは違う)。
- `hook` `none`: 依存が kernel32.dll だけ(Fruity の hook は要らない見込み。未確認)。
- `args` `--pdata`: MSVC x64 の既定(Sytrus・Balance と同じ。未確認)。
- `omit`: 空(setjmp の有無は、lift の結果で確かめる。未確認)。
- 注意: 実物を VFS に置くと、`GetFileAttributesW` は見つけるようになる。その場合の読み込みは `load_by_name` の中で、realmod の照合が `search_volume` より先に来るので、実モジュールが当たる。逆に、実モジュールが無いまま実物だけを VFS に置くと、「ファイルはあるがモジュールではない」で名指しの停止になる(`stop_file_not_a_module`)。

## 6. 同じ型の候補(未確認。この作業では直していない)

- #1395 の ILSvg: `tools/persist/measure.mjs` の `STOP_LINE` は `missing-path: LoadLibraryW ... "ILSvg_x64.dll"` を止まりとして数える(`missing-ilsvg`)。ILSvg も Shared の DLL で、ビルドに無い。同じ型の可能性が高い(推定)。#1395 の本文は、この作業では読んでいない。
- このログの他の `LoadLibraryW 126`: `WavpackDLL_x64.dll`(1 回)、`ffmpeg\x64\ilvideo_x64.dll`(1 回)。どちらも Shared の DLL で、ビルドに無い。NULL を使う経路があれば、同じ型の止まりの候補(未確認)。
- `imm32.dll`(`LoadLibraryW 126`)は、`fle_load_absent` の決めどおりで、この止まりとは無関係。

## 7. 前回の記録との関係

- `pm-work/findings/sytrus-1429.md` の §2 の「`+0x28` の設定者は未特定」: **この記録で確定した**(`0x83f4b0`。§3.2)。
- 同じく §5 の「`+0x28` の設定者(後の仮想呼び出し、またはホストの経路)は未特定」: 確定(ホストの経路ではなく、同じコンストラクタの連鎖)。
- `sytrus-1429-diag.patch`(診断の差分): 今回の原因の確認には使わなかった。作業ツリーには載せていない(製品に入れていない)。

## 8. 添付(この記録の根拠。`.tmp-1429/run1/` は作業の後に消した)

- `riff-log.txt`(engine の全ログ。5412 行)、`riff.jsonl`(道具の 1 行)、`boot.json`、`flow.jsonl`(流れ A〜D)、`npm-test.log`。
