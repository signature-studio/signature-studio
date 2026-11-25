# #1413 MSVC の switch 表の認識(作業の記録)

実エンジン: `lift/target/release/fe-build engine`(exit 0。hybrid 段 558.0 s)・`fe-build bundle`(exit 0)・`tools/switch-tables/check.mjs`(exit 0。下の 5)・`tools/boot-check/boot-bundle.mjs`(2 回とも engine-idle・standIn false・error null。elapsedMs 14502 と 12458)・`tools/smoke/welcome-flow.mjs`(A〜D の 4 行、すべて合格。elapsedMs 11897〜12146)・`npm test --prefix app`(71 files passed・1413 tests passed・10 skipped)・`tools/p3-audio-probe/measure.mjs`(Synth_dfr - Overcome - Sytrus.flp。`not a known function entry` と `0xc9cf273` は 0 件。止まりは 8 を見る)。値は runner の値。この機械とは別枠(nproc 4)。

## 1. 0x44f273 の形

`objdump -d` の `Sytrus_x64.dll`(ImageBase 0x400000。engine では 0xc580000 に rebase)。RVA 0x44f273 の VA は 0x84f273。関数の先頭は RVA 0x44f130。

```
84f24a  cmp   DWORD PTR [rsp+0x88],0x1e      ; 83 bc 24 88 00 00 00 1e   <- ガード(メモリ上の slot)
84f252  ja    0x851ddb                        ; 0f 87 83 2b 00 00
84f258  movsxd rax,DWORD PTR [rsp+0x88]     ; 48 63 84 24 88 00 00 00   <- 同じ slot を読み直す
84f260  lea   rcx,[rip+0xffffffffffbb0d99]  ; = ImageBase 0x400000
84f267  mov   eax,DWORD PTR [rcx+rax*4+0x4520d0]
84f26e  add   rax,rcx
84f271  jmp   rax
84f273  mov   rax,QWORD PTR [rsp+0x80]     ; 表の 0 番の飛び先(RVA 0x44f273)
```

- 表: RVA 0x4520d0、4 バイトの RVA、N = 0x1e(31 項)、31 個すべて別の値。項 0 が 0x44f273。全ての飛び先は関数(RVA 0x44f130 の pdata)の中の命令の先頭。
- 依頼は「直前に cmp・ja が無い」と書いたが、**ガードは在る**。`cmp` が `ja` の直前に在り、その `cmp` は**レジスタでなくメモリ**(`[rsp+0x88]`)を比べる。index は、ガードの後に同じ slot から `movsxd` で読み直される。ガードは基本ブロックの終わりに在り、`ja` の fall-through が読み直しに続く。ガードの無い形の表ではない。
- この件では「表の範囲から N を決める」段は要らなかった。下の 2 の表で、認識できない表の形の全件にガードが在る(ガードが離れている形もある)。

## 2. 基準値(直す前)

`fe-decode jmp-reg FILE`(`lift/crates/fe-decode/src/survey.rs`)で数えた。`analyze_with_pdata`、FLENGINE の注釈は既存のまま。「表の形」は、`jmp reg` の直前の `add rT,B` と、その前の表の読み(`mov eT,[B+I*4+d]` か `movsxd rT,[B+I*4+d]`)の形で決める。`表でない` は、遅延読み込みの戻り(下の 3)など。

| DLL | 間接の `jmp reg`(到達) | 解決した表 | 認識できない表の形 | 表でない | 誤認識 |
|---|---|---|---|---|---|
| FLEngine_x64.dll | 20 | 3(注釈) | 0 | 17 | 0 |
| Sytrus_x64.dll | 17 | 0 | 3 | 14 | 0 |
| Balance(fruity_balance) | 15 | 0 | 2 | 13 | 0 |
| OTT | 未計測(入力が無い。下の 6) | - | - | - | - |

形ごと(VA は RVA の値に ImageBase を足したもの。`fe-decode jmp-reg` の `site` 行):

- F1 メモリ上のガード: Sytrus `0x84f271`(W。`msvc_rejected`: `N not derived`)。
- F2 表相対 `movsxd`・ガードが直前: Sytrus `0x55f666`(N = 0x22)、Balance `0x54a9b6`(N = 0x22)。`msvc_rejected` にも `jumptab_rejected` にも出ず、無言で `Ok(None)` だった。FLEngine の `0x5e5eb6`・`0x5e9ae8`・`0x5f1499` も同じ形で、`FLENGINE` の注釈で手で解決済み(この 3 件は変えない)。
- F3 表相対 `movsxd`・ガードと idiom の間に `mov` が 2 つ(scheduled): Sytrus `0x563298`(N = 0x1e。r15 は保存・復帰の `mov r12,r15` ... `mov r15,r12` の後で `lea` の値のまま)、Balance `0x54e5e8`(N = 0x1e。同じ形)。
- 表でない(other)の 14(Sytrus)・13(Balance)・17(FLEngine): 下の 3。

## 3. 表でない `jmp reg`(遅延読み込みの戻り)

全件が同じ形: `call <stub>`(stub は `sub rsp,0x28; call [rip+slot]; add rsp,0x28; ret`。slot は `.data`)、その後に `movapd xmm0-3`・`add rsp`・`pop r9/r8/rcx/rdx` で引数を戻し、`jmp rax`。`rax` は stub が返した値。表ではないので、この件の範囲の外。FLEngine にも同じ形が 17 件在り、#1400 までの記録では現行の engine はそれで起動している。この 14 件の実行時の扱いは、この作業では確かめていない(未確認)。

## 4. 広げた認識(decision 0430)

`lift/crates/fe-decode/src/jumptab.rs`:

- (a) メモリ上のガード: `cmp dword [M],N; ja; movsxd I,[M] (または mov eI,[M]); ...; mov eT,[B+I*4+T]`。ガードの slot と読み直しの slot が同じ(同じ base・index・scale・disp・segment・size)のときだけ N を取る。違う slot なら従来どおり `N not derived`(試験 `msvc_slot.rs`)。読み直しは、ja と idiom の間の `MSVC_SKIP`(2)の中に入る。
- (b) 表相対の `movsxd rT,[B+I*4]`(disp 0、64 ビット rT): 表は B(lea の値)から始まり、項は B からの符号付き 32 ビットの差。標準の形(`resolve_msvc` の `rel`)と scheduled の形の両方。N はガードから。飛び先は `read_rel_targets` で、関数の pieces・実行可能な section・デコードの検査を受ける(従来の `read_msvc_targets` と同じ)。
- (c) 基底の証明: `mov B,S`(64 ビット同士)の定義は、その命令での S の値として証明する(保存・復帰の形)。(レジスタ, 命令) の組が証明の途中に在るとき、その組の循環を仮定する(下の 9 の (c))。深さは 8 まで。(b) の表の基底は、証明された値をそのまま表の VA にする。標準の形は従来どおり ImageBase。
- 表の飛び先の検査は従来どおり。誤認識は 0(`jmp-reg` の `misrecognized`: 表の飛び先が関数の pieces の外、または閉包が命令の先頭として復号していない)。

依頼の候補 (a)〜(c)(範囲から N を決める)は、この件では採らなかった。理由: 認識できない表の全件がガードを持つ。ガードの無い形の表は、3 つの DLL に 0 件。

## 5. 結果(直した後)

| DLL | 間接の `jmp reg` | 解決した表 | 認識できない表の形 | 表でない | 誤認識 |
|---|---|---|---|---|---|
| FLEngine_x64.dll | 20 | 3 | 0 | 17 | 0 |
| Sytrus_x64.dll | 17 | 3(0x55f666・0x563298・0x84f271) | 0 | 14 | 0 |
| Balance(fruity_balance) | 15 | 2(0x54a9b6・0x54e5e8) | 0 | 13 | 0 |

0x84f271(0x44f271)の表は N = 0x1e、31 個の飛び先。0x84f273 が含まれる(項 0)。

`node tools/switch-tables/check.mjs`(exit 0。判定は絶対の基準。基準の表とは比べない):

```
switch-tables FLEngine: sites 20 resolved_tables 3 unrecognized_table_shape 0 unrecognized_other 17 misrecognized 0 ok
switch-tables Sytrus: sites 17 resolved_tables 3 unrecognized_table_shape 0 unrecognized_other 14 misrecognized 0 ok
switch-tables Balance: sites 15 resolved_tables 2 unrecognized_table_shape 0 unrecognized_other 13 misrecognized 0 ok
switch-tables OTT: not measured (no VST3 binary in lift/out-app/external; not judged)
```

## 6. OTT の扱い

OTT の VST3 は、`plugin-external` のラベルの Issue でだけ R2 から取られる(`.github/workflows/agent.yml`)。この Issue のラベルには無いので、作業ツリーに無い。`tools/switch-tables/check.mjs` は、`lift/out-app/external/ott/*.vst3` が在れば測って表示し(判定はしない)、無ければ `not measured` と表示する。

## 7. 試験

- `lift/crates/fe-decode/tests/switch_tables.rs`: Sytrus 0x44f271・0x55f666・0x563298、Balance 0x54a9b6・0x54e5e8、FLEngine(注釈の 3 件は同じ)。DLL は `SYTRUS_PE`・`BALANCE_PE`・`FLENGINE_PE` か、リポジトリの取り出し先から読む(無ければ skip)。
- `lift/crates/fe-decode/tests/msvc_slot.rs`: 合成の画像で (a) の正の例と負の例。
- `lift/crates/fe-decode/tests/msvc_switch.rs`: 既存の 72 件が通る(途中で、`MsvcForm` に `rel` を足した 1 箇所の初期化を直した)。
- `cargo test -p fe-decode`: 全て通る(switch_tables 4・msvc_slot 2・msvc_switch 72 を含む)。
- `cargo test -p fe-lift --test msvc_switch --test jumptab --test ldst_switch --test seh_rt`: 通る(25・5・4・17)。
- `cargo test -p fe-lift` の全体: 失敗する 446 件は、直す前(HEAD)と同じ集合(`diff` で同一)。原因は環境: `auth117/mk_signed_pe.py` が無い(追跡されていない)、fe-lift の試験の link に `fle_clip_host_write` の定義が無い(ブラウザ側の定義は `app/src/plugins/ott-host.ts` に在る)、RCPPS の host 比較(この CPU)。この件では直さない。

## 8. デモ(Synth_dfr - Overcome - Sytrus.flp)

2 回目(#1414 が入った後)の `measure.mjs`:

- `error`: `timeout 180000 ms: the title of synth (the title is "Song info")`。`end.state` は `exited`、`end.title` は `Song info`。
- `end.tail` の最後の行: `[err] fle: more than 32768 live critical sections`。`lift/runtime/fle_sys.c:192` の `fle_cs_init`: 生きている critical section が `FLE_CS_MAX_LIVE`(`FLE_CS_SLOTS / 2`)に達した時に止まる。ゲストが `DeleteCriticalSection` で閉じない section が増えている。原因は未確認。
- それ以前の行は全て `missing-path: GetFileAttributesW`(フォントと Sytrus の DLL の不在)。`CreateFileW: empty path`(1 回目の止まり)は出ない(#1414 の後)。
- `imageReads` に `generators.sytrus.sytrus_x64.c580000.dll`(atMs 20594)と `.wasm`(atMs 20687)が requested。
- 所見: 認識できない表による止まりは無くなり、実行は `Song info` の後の critical section の上限まで進む。デモは開いていない(timeout)。この止まりは、この Issue の範囲の外(次の課題)。

## 9. 未確認・仮定・次の課題

- (c) 基底の証明の仮定(受け入れには入れない。PM が読む): 保存・復帰の循環(ループの中で `mov r12,r15` ... `mov r15,r12` が戻る形)は、(レジスタ, 命令) の組が証明の途中に既に在るとき、その組が成り立つと仮定して読む。論拠: 実行の最初に証明された値と違う値が現れる時点を取る。その値は、その時点の直前の定義から来る。定義が `lea` なら値は合う。定義が証明された複写なら、複写の元の値は、より前の実行の中の値で、最初の違いより前に在るので、仮定した主張に合う。よって最初の違いは無い。これは帰納の形の論拠で、機械による検証ではない。保存・復帰の複写は、F3 の 2 件の r15(Sytrus 0x563298・Balance 0x54e5e8。2 節)で現れる。循環の仮定が、この 2 件の証明で実際に使われたかは確かめていない。
- 表でない `jmp reg`(Sytrus 14・Balance 13・FLEngine 17)は、判定の道具で件数を出すだけ。実行時の扱いは確かめていない(未確認)。
- OTT は入力が無い(未計測)。
- decision 0430 は、この repo に決定ログが無いので登録していない。番号は、コードの注記と、この文書だけで使う。前回の試行で使った 0110 は、VFS の別件で使用済みだった。
- 起動の時間(P1 は 10 秒以内): 2 回の boot-check は elapsedMs 14502・12458。前回の run の 8218 とは違う。原因は未確認(runner の負荷、または #1414 の後の配信の影響の可能性)。この run では P1 は満たしていない。流れ A〜D の elapsedMs は 11897〜12146 で、受け入れの上限 12500 の内に在る。
- 次の課題(この Issue の範囲の外): 8 の `fle: more than 32768 live critical sections`。ゲストの critical section の解放(`DeleteCriticalSection`)か、上限の設計のどちらかを確かめる。

## 基準値(直す前。記録のみ。`tools/switch-tables/check.mjs` は読まない)

```tsv
dll	sites	resolved_tables	unrecognized_table_shape	unrecognized_other	misrecognized
FLEngine	20	3	0	17	0
Sytrus	17	0	3	14	0
Balance	15	0	2	13	0
OTT	-	-	-	-	-
```
