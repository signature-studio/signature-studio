# Sytrus の止まり(#1429): 原因の特定の途中記録(直していない)

> 3 回目の試行で原因を確定した: `sytrus-1429-reverb.md`。`+0x28` の設定者は `0x83f4b0`(Sytrus の中の、`Reverb_x64.dll` の読み込みと `CreateReverbInstance` の取得)で、止まりは `Shared\Reverb_x64.dll` がビルドに無く `LoadLibraryW` が NULL を返すため。この記録の §2・§5 の「設定者は未特定」は、そこで解けた。二重解放・ヒープの記録は、この止まりの説明には要らない(別件・未確認)。

判定: **差し戻し**(受け入れは通らない。止まりは残っている)。製品のコードは変えていない。一時の診断は、patch(`sytrus-1429-diag.patch`)として残し、作業ツリーからは消した。

runner の値。この機械とは別枠。nproc 4。loadavg は riff 測定の JSON の `loadavg`(1.83 1.43 1.93)。作業の開始 12:56 UTC。

## 0. 受け入れの各段(実測。診断の入った bundle。製品ではない)

| 段 | 結果 |
|---|---|
| 起動確認(boot-bundle) | `state` engine-idle・`standIn` false・`error` null・`elapsedMs` 9,183(流れ B と同じ実行ではない) |
| 道具の 1 行(DPR 1: `--tool riff-machine`) | **不合格**。`error` null・`windowNew` に `TPRScoreCreatorForm` 751,275 の 334x506 が `visible` true・`painted` true。しかし `stopped` true・`engine` exited・`stopLines` に `fle: indirect call to 0 is not a known function entry` |
| 流れ A〜D(`tools/smoke/welcome-flow.mjs`) | A `elapsedMs` 9,308・B 9,183・C 9,428・D 9,225。4 行とも `error` null・engine-idle・`standIn` false・`trapped` false・`hostErrors` 0・`opened` true。B と D は `welcomeClosed` true。`preBootBytes` 313,283,881(上限 314,368,711 以内) |
| 単体テスト(`npm test --prefix app`) | rc 0。`Test Files 75 passed | 1 skipped`・`Tests 1463 passed | 10 skipped` |

受け入れは、道具の 1 行で落ちる(止まりが残る)。流れ A〜D と単体テストは通る(診断の bundle での値)。

## 1. 止まりの行と直前の 40 行(実測)

止まりの行(2 行)と、その直前の 40 行(`--log` の出力。診断の `diag1429:` 行は含まない):

```
fruity: == stop: SIGABRT (runtime abort, see runtime message)
fruity: exit code 134 (...)
fle: regs R[0]=0x63736d65 R[1]=0x0 ... R[15]=0x0
fle: indirect call to 0 is not a known function entry (caller ?, near ?)
```

直前の 40 行(`GetFileAttributesW`・`FindFirstFileW` の `missing-path` と `enum-dir` の行。止まりの原因の行は含まれない):

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
```

止まりの時の eager な値(`fruity: guest registers at the stop (R[], eager)`。この harness の読み):

```
rax=0 rcx=0 rdx=0x43676480 rbx=0x63736d65
rsp=0x41e7f918 rbp=0x41e7f970 rsi=0x43676480 rdi=0x43675740
r8=0x43675740 r9=0x1a8 r10=0 r11=0 r12=0 r13=0x1a8 r14=0 r15=0
guest stack at rsp: 0xc9c04e9 0x43676480 0x8000001a8 0xbff0000000000000 0x2c3ca22 0x40abbb20 0x41ad0460 0x41e7f960
```

スレッドの表: `threads: 22 (running index 20)`。走っているのは thread 20(id 0x1015・start 0x412ae0 の thunk・param 0x420971a0・rsp 0x41e7f918)。

注: `fle: regs` の行は R[0] に 0x63736d65、R[1..15] に 0 を出す。eager の値(rax=0・rbx=0x63736d65・rsp 0x41e7f918)と合わない。`fle: regs` は別の R[] の写し(Sytrus の module 側の写し)を読んでいる、と読める(推定。未確認)。以下は eager の値と、逆アセンブルを基にする。

## 2. 止まりの位置(実測 + objdump + 生成 C。確かめた)

- 戻り番地 `0xc9c04e9` は、Sytrus の像(base 0xc580000)の中。**番地の換算を直す**: 前の記録(`RVA 0x5404e9`)は base を ImageBase と取り違えていた。正しくは `guest = VMA + 0xc180000`(= base − ImageBase 0x400000)。`0xc9c04b0` の基本ブロックの先頭が VMA `0x8404b0`(関数の先頭)と一致し、`0xc9c04e9` は VMA `0x8404e9` で、その直前が `call *0x10(%rbx)` である(objdump で確かめた)。RVA(ImageBase 基準)は `0x4404e9`。
- 逆アセンブル(`lift/out-app/host/lift/out-plugins/generators.sytrus.sytrus_x64.dll`、VMA):

```
8404b0: push %r13; push %rdi; push %rsi; push %rbx; sub $0x28,%rsp
8404b9: mov %rcx,%rbx                      ; this
8404c5: cmpb $0x0,0x20(%rbx); je 8404d3   ; フラグ
8404cb: mov %rbx,%rcx; call 0x83fad0       ; (フラグが立っているとき)
8404d3: mov 0x28(%rbx),%rax                ; X = this->+0x28
8404e3: mov (%rax),%rbx                    ; X の vtable
8404e6: call *0x10(%rbx)                   ; X->vt[2](...)
8404e9: add $0x28,%rsp
```

- 生成 C(`fe-lift lift` を Sytrus の DLL に掛けた出力。`f_8404b0`)は、この並びを一命令ずつ忠実に写している(確かめた)。
- eager の値: `rax`(= X)= 0。`rbx`(= `[X]`、vtable)= 0x63736d65(guest のアドレス 0 の中身)。`[vtable+0x10]` は 0 で、「indirect call to 0」になる。**X = this->+0x28 が NULL**。
- 呼び出し元: `0x873e5e`(`call 0x8404b0`)。引数は、`rcx = [obj+0x5fe0]`(this)・`r8 = [obj+0x62a0]`・`rdx = rdi`・`r9d = [[rbp+0x48]]`。`obj = [rbp+0x38]`(生成 C の `L_873e12`〜`L_873e5e` で確かめた)。
- `this = obj+0x5fe0` は遅延生成。`0x869874`〜`0x86988c`: `cmpq $0,0x5fe0(%rbx)`、`jne`、`rcx = [0x83f378]`(= 0x83f440 の定数。.text の中にある)、`dl = 1`、`call 0x8402d0`、`mov %rax,0x5fe0(%rbx)`。
- 生成器 `0x8402d0`: `dl` が立っているとき `0x40f670(rcx, dl)` → `call *-0x30(%rax)`(Delphi の class method の slot らしい。推定)。次に基底のコンストラクタ `0x83f710` を呼ぶ。これが `this+0x28 = 0` を書く(`0x83f763`)。その後、浮動小数の初期化、`call *(%rbx)`(vtable の slot 0。`0x8403d0`)。**生成器自身は `+0x28` を書かない**(生成 C の `L_8402d0`〜`L_8404b0` に `R+0x28` の書込みは無い)。よって `+0x28` の設定は、後の仮想呼び出し(slot 0 など)か、別の経路で行われるはず。**設定者は未特定。**

## 3. 解放と二重解放(実測。診断の hook)

診断の hook は `fle_dispatch`(間接呼び出し)で CRT の malloc(0x407a50)・free(0x407e20)を見る。製品の wrapper(`f_409410`・`f_409440`)は間接呼び出しで CRT を呼ぶので、エンジン側の malloc・free はこの hook に入る。**ただし、CRT の内側に直接呼びの経路が 9(malloc)・12(free)ある(`0x4079xx`〜`0x408xxx`)。これは hook に入らない**。よって、「state 0」(表に無いブロック)の行と、一部の DOUBLE・BAD の行は、この計数の欠けによる見かけの可能性が高い。

解放を行うルーチン `f_412e40`(配列の参照を解放する。`R[1]` = 配列の先頭、`R[2]` = 個数。各要素: 0 にして、`p-0x10` のブロックの `+8`(32 bit)が 1 以上なら `xadd -1`、結果が 0 なら free)は、`f_447f40` の末尾(`0x448259`)から、`this+0x40`〜`this+0x58` の 4 個について呼ばれる。これらの slot は Delphi の文字列(長さが `p-4`、参照数が `p-8`)と読める。

REL の行(最初の 120 件。`f_412e40` からの解放を出した。抜粋):

```
REL block 0x40b0dd70 cur 0 refcount-before 0 slot 0x409f6888 owner 0x44825e state 1 seq 126 msize 0x14 mcaller 0x412c50 mseq 125 prev-free cur 0 owner 0x44825e slot 0x409f6888 seq 122 text "1..."
REL block 0x40ae59c0 cur 0 refcount-before 0 slot 0x409f69b0 owner 0x6533d6 state 2 seq 1447 msize 0x70 mcaller 0x409f6960 mseq 1434 prev-free cur 0 owner 0 slot 0x40ae59c0 seq 1436 text "Shared\QuickFontCache_x6..."
```

読み取り:
- `refcount-before 0` は、正常な最後の解放では 0 になる(xadd の後の値)。この値だけでは誤りは判らない。
- **同じブロック・同じ slot・同じ owner が、4 回の間接呼び出しの間に 2 度解放された**(seq 122 と 126)。2 度目の時、その slot はそのブロックを再び持っていた(slot は 0 にされているはず)。つまり、解放されたブロックへのポインタが、同じ owner の処理の中で再び slot に入った(use-after-free の形)。
- `0x40ae59c0`(文字列 "Shared\QuickFontCache_x6…")は、`0x418202` で 1 度解放され(seq 1436。同じ seq 番号の前の計数の行 `BAD free ... last free cur 0 caller 0x418202 seq 1436`)、その後、`f_412e40` の経由で 2 度目の解放(seq 1447)。`0x418202` は参照数を `p-0xc`(32 bit)で見る別の解放ルーチン。**2 つの経路で参照数の位置が違う**。同じブロックを両方が扱う。
- 推定(未確認): CRT の free は、解放したブロックの `+8` に自分の管理の値(1 など)を書く。その後、解放済みのポインタを `f_412e40` が見ると、参照数が 1 に見えて、もう一度解放する。

## 4. 分類(Issue の (i)〜(iii) との対応)

- (i) lift が code の埋め(0xcc)を読んだ: **該当しない**。読んだ値は 0(NULL)。
- (ii) 初期化されていないデータ(0): **該当する可能性が高い**。読み元は `this+0x28`(X)で、値は NULL。ただし、「設定されないまま」か「解放後の再利用で 0 になった」かは未確認。上の 3 の二重解放は、後者を示唆する(未確認)。
- (iii) import の未解決: **該当しない**。
- 別の見方: 前回の試行で見た、ヒープの管理の値・引数ブロックの書換えも、同じ型(解放後の再利用)と読める(推定)。

## 5. 直せていないこと・未確認

- `this+0x28` の設定者(後の仮想呼び出し、またはホストの経路)は未特定。
- 参照の取り違え(どの AddRef/Release が対になっていないか)は未特定。`f_447f40` の slot(`+0x40`〜`+0x58`)に文字列を入れる `f_414890`・`f_414bb0` の参照の扱いを読むのが次の手。
- CRT の直接呼びの経路(`0x4079xx`〜`0x408xxx`)を hook に入れないと、解放の計数は正確にならない。
- `fle: regs` の行と eager の値が合わない原因は未解決。
- `#1395`(ILSvg の `0x3286b83d0`)との比較は、この作業では読めていない(未確認)。
- 製品の修正は無い。受け入れは通っていない。

## 6. 次にやること

1. 直接呼びの経路を hook に入れる(生成 C の `f_407a50();`・`f_407e20();` を間接呼びに替える一時の変更。または、wrapper 経由に統一する一時の変更)。計数を正確にしてから、二重解放の最初の出どころを見る。
2. `f_447f40` の slot に文字列を入れる経路(`f_414890`→`f_413060`、`f_414bb0`)の参照の扱いを読み、`0x418202` の解放(参照数 `p-0xc`)との食い違いを確かめる。
3. `0x8402d0` が呼ぶ後の仮想呼び出し(vt slot 0 `0x8403d0` など)で、`this+0x28` が設定されるかを確かめる。
4. 直せたら、受け入れ(`accept` のブロック)を全部通す。

## 7. 添付

- `sytrus-1429-diag.patch`: 診断(`lift/runtime/fle_thread.c` の末尾。`DIAG-1429`)。製品には入れない。使うときは `git apply` し、終えたら `git checkout -- lift/runtime/fle_thread.c` で消す。
