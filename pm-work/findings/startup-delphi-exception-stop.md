# 起動の最中の入力で止まる、Delphi の例外 0xEEDFADE の読み

- 日付: 2026-11-28(UTC)。作業者: PM 側の作業者(読みと起票だけ。製品のビルドと実行はしていない)。
- 読んだ物: `lift/runtime/fle_seh.c`・`fle_thread.c`・`fle_callback.h`・`lift/harness/boot.h`、`binaries/win64/FLEngine_x64.dll`(ImageBase 0x400000。`FLE_EXE_HANDLE` と同じ番地)の `.pdata`・`UNWIND_INFO`・逆アセンブル。
- 確認できなかった物: `.tmp-e2e46/e2e.jsonl`(この作業場所に無い)。下の 1 の記録は、依頼文に書かれた値の引用である。

## 1. 入力(依頼文の記録。原本は確認できない)

- 表示 4k-random(DPR 2.29)。scenario `context-menu-none` の before-idle 領域(起動の最初の idle の前に右クリックを 20 回)。`ok` false。scenario の判定(contextmenu が全て cancel される)は通っていた。
- `engineStops` 1。同じ job の他の表示(DPR 2.39・smoke-dpr1・smoke-dpr2)では起きていない。間欠。
- 停止の文言(`lastFle`。記録の一部は省略):
  `fle: RaiseException: exception code 0xeedfade, flags 0x1, 8 argument(s) raised; no frame-based handler took it (11 frames searched); ended at return address 0 (read at 0x3e958818) after frame 11; arg1 class Exception < TObject; first frames 1-11 [411e4d:r0 411edb:r0 d340a7:r3 dc2f37:r2 dbf6ba:r2 dc2aa2:r2 1034956:r2 114cfb4:r0 92d940:r0 976b4f:r0 52a596:r0]`
- `message` は証拠の行(`first non-background pixel ... guest return 0x411e4d`)で、停止の理由ではない。
- 同じ戻り番地 0x411e4d・同じ文言が、実機の記録(#1410 の元の下書き。`pm-work/findings/real-user-stops-2026-11-25.md` の 3.5)にもある。

## 2. 文言の出所(コード)

- 文言: `lift/runtime/fle_seh.c` の `fle_seh_raise`(527 行から)。探索は 541–609 行、文言は 610 行。`record_append`(269–296 行)が、末尾の `ended at return address 0 (read at …)` と `first frames` を付ける。
- 探索の 1 枠: `unwind_one`(319–440 行)が、`.pdata` の `RUNTIME_FUNCTION` を探し、`UNWIND_INFO` の code を順に戻し、戻り番地を読む。枠の表が無い時だけ葉として扱う(338–346 行)。
- ハンドラ: `handler_va` が在る枠は `fle_call_guest(handler, rec, establisher, ctx, dc)` を呼ぶ(595 行)。戻り値 1(ContinueSearch)なら次の枠。0 や他の値は、名前つきの停止になる。この停止の文言は「取られなかった」まで続いたので、呼ばれた枠の戻り値は 1 だった(実測から)。
- 終わり: 戻り番地が 0 になると `break`(608 行)。`bytes below the stack base` の文言は、`base > slot`(TEB の StackBase より下の読み)の時だけ出る(276 行)。停止の文言に出ていないので、`base <= slot` か `base == 0` のどちらか。文言からは区別できない。

## 3. 11 枠の読み(`binaries/win64/FLEngine_x64.dll`。RVA = VA - 0x400000)

検証の方法: 各枠の戻り番地の直前に `call` 命令があるか、その call の先が、内側の枠の関数の先頭と一致するか。`.tmp-seh1` の作業用の読み(終了後に消した)で確かめた。

| 枠 | 戻り番地(VA) | 関数の先頭(VA) | 直前の call(確認) | UNWIND flags | ハンドラ | 読み |
|---|---|---|---|---|---|---|
| 1 | 0x411e4d | 0x411d20 | `call [0x15e6030]`(RaiseException の番地の入った slot) | 0 | なし | 生の raise 補助。record: code 0xEEDFADE・flags 1・8 個。args[0] = rdx、args[1] = rcx(例外オブジェクト) |
| 2 | 0x411edb | 0x411ea0 | `call 0x411d20`(確認) | 0 | なし | 再 raise(`raise;`)。スレッドの状態(`[rax+0x208]`)から、現在の例外のオブジェクトを取り、raise 補助を r8b=1 で呼ぶ |
| 3 | 0xd340a7 | 0xd33f00 | `call 0x411ea0`(0xd340a2。E8 を直接確認) | 3(EH+UH) | 0x4112c0。表 3 件 | try..except。再 raise の呼び出しは、catch-all の範囲の外(下の表) |
| 4 | 0xdc2f37 | 0xdc2ef0 | `call 0xd33f00`(確認) | 2(UH) | 0x4112c0 | try..finally(検索では呼ばれない) |
| 5 | 0xdbf6ba | 0xdbf640 | `call 0xdc2ef0`(確認) | 2(UH) | 0x4112c0 | 同上 |
| 6 | 0xdc2aa2 | 0xdc2a60 | `call 0xdbf640`(確認) | 2(UH) | 0x4112c0 | 同上 |
| 7 | 0x1034956 | 0x10348b0 | `call 0xdc2a60`(確認) | 2(UH) | 0x4112c0 | 同上 |
| 8 | 0x114cfb4 | 0x114cf40 | `call 0x10348b0`(確認) | 0 | なし | |
| 9 | 0x92d940 | 0x92d710 | `call *%rsi`(間接。先は確認できない) | 0 | なし | 再入の旗(`Self+0x7c`・`+0x7d`)を見る形(読み。意味は確認できない) |
| 10 | 0x976b4f | 0x976b30 | `call 0x92d710`(確認) | 0 | なし | `Message.Msg == 0x401` の時だけ枠 9 を呼ぶ |
| 11 | 0x52a596 | 0x52a570 | `call *(%r11)`(間接。先は確認できない) | 0 | なし | `TMessage`(`rdx` = `&Msg`。Msg は +0x20、WParam +0x28、LParam +0x30、Result +0x38)を作り、`rcx = [r11+8]` で `[r11]` を呼ぶ thunk。戻り番地 0 は、この枠の上(根)に在る |

枠 3 の except の表(関数 0xd33f00。RVA の範囲)。

| 項目 | 範囲 | a | 読み(逆アセンブルから) | landing(b) |
|---|---|---|---|---|
| [0] | [0x933f66, 0x934036) | 0 | 検索で取らない(a == 0 は飛ばす) | 0x9340c0 |
| [1] | [0x933f65, 0x93406c) | 2 | 全ての例外を取る(on 節の検査が無い)。landing = b | 0x93406c |
| [2] | [0x93406d, 0x9340a8) | 0 | 検索で取らない。再 raise の呼び出し(0x9340a2、戻り 0x9340a7)は、この中 | 0x9340f0 |

枠 3 の except 本体は 0x93406c から。仮想呼び出し `call *-0x20(%rbx)` と `call 0x410340` のあとに、再 raise(0x9340a2)が来る。

ハンドラ 0x4112c0(Delphi の共通の例外ハンドラ)の検索の経路(逆アセンブルの読み。意味の確定は、名前が無いので確認できない):
- `ExceptionRecord.flags & 0x66 == 0` の時に、表を上から調べる。範囲の外は次へ。a == 0 は飛ばす。a が 1・2 なら取る(2 の landing は b)。a > 2 は on 節の表を、クラスで調べる(VMT の一致、次に名前、次に親の VMT を `VMT-0x78` で辿る)。
- 取ったら、`RtlUnwindEx`(0x406ff0。ランタイムでは `fle_seh_unwind_ex`)を呼んで landing へ行く。取らなければ 1 を返す。

## 4. 読みの結論

実測(この作業の読みで確かめたこと):
- 11 枠の戻り番地は、全て、直前の call の戻り先と合う(枠 9 と 11 は間接 call で、先は確かめられない)。`unwind_one` の結果は、この chain では実際の呼び出しの並びと一致する。
- 枠 3 の表で、再 raise の呼び出し(0x9340a2)は catch-all の範囲 [1] の外、a == 0 の範囲 [2] の中。したがって枠 3 は取らない。停止の文言(取られなかった)と一致する。
- 枠 4〜11 に、検索で呼ばれるハンドラ(UNW_EHANDLER)は無い。枠 4〜7 は try..finally(UH)だけ。
- 戻り番地 0x411e4d は、枠 1 の raise 補助(0x411d20)の中で RaiseException を呼んだ直後の番地。`real-user-stops-2026-11-25.md` の 3.5 で推定だった発生元は、この読みで確かめられた(raise 補助の中)。

#1410(下書き 166)の仮説との関係:
- (a) 境界: この chain は host の境界(0xCB000000)を跨がない。戻り番地 0 で終わる。支持しない。
- (b) 枠の表の読み: 11 枠の戻り番地と call が合い、各枠のハンドラの有無も、コードの読みと合う。この 11 枠については支持しない。
- (c) 引数: 引数 2 番目(args[1])のクラスは Exception で、raise 補助の引数の組み立てと一致する。支持しない。
- 新しい読み(d): この停止は、最初の例外を枠 3 の catch-all が取り、その except 本体の `raise;` で同じオブジェクトを投げ直した、2 回目の dispatch である。停止の記録は 2 回目だけを示し、最初の例外 X(種類・メッセージ・発生元)は記録に無い。推定だが強い(再 raise は直前に取ったオブジェクトを投げ直す)。

未確認(記録と読みからは決められない):
- X の発生元と、メッセージ。X の種類は、arg1 のクラスが Exception(基底)なので、`raise Exception.Create(...)` 系と推定するが確認できない。
- X が起きる経路。枠 3 の try 本体の呼び出し(0xd33f77・0xd33f8e・0xd33f9a・0xd33fba・0xd33feb・0xd33ffb・0xd3400b・0xd3401c・0xd34030・0xd3403f・0xd34048・0xd3405d)のどれか、またはその先。
- 枠 11 の上(根)の意味。戻り番地 0 がスタックの底か(文言に `bytes below` が無いので、底か、TEB の基準が 0 か)、確認できない。`fle_call_guest` は戻り番地 0xCB000000 を積むので(`fle_callback.c` 89 行)、この根は fle_call_guest からの呼び出しではない。どこから呼ばれたかは確認できない。
- 0x401 を送る経路(枠 10 の分岐)の送り手。確認できない。
- Windows の同じ経路に、この先の try..except(VCL の窓の手続きの枠など)があるか。確認できない。

## 5. 次にすること(起票: #1542 の「やること」1〜4 に写した)

1. 最初の例外 X を記録に残す(既定の動作。新しい環境変数・`#ifdef` は足さない)。`fle_seh_unwind_ex` の landing の分岐(`_longjmp` の直前)で、取った例外の code・arg1 のクラス・Exception のメッセージ(obj+8 の UnicodeString。読めた時だけ)・pass 2 の枠の一覧を残す。停止の文言に、その記録を入れる。
2. 再現の頻度を実測する(直す前に、乱数の DPR で 10 回以上)。
3. 記録から X の発生元を特定し、起動の最中の入力でだけ起きる理由を読む。
4. 原因を直す。例外を握りつぶして停止を消すのは直しにしない。

## 6. 道具

- `.tmp-seh1/unwind.mjs`(`.pdata` と UNWIND_INFO の表示)、`.tmp-seh1/scope.mjs`(scope 表とクラス名)、`.tmp-seh1/callers.mjs`(直接の call の検索)、`.tmp-seh1/precall.mjs`(戻り番地の直前の call の検証)。終わったので消した(`.tmp-fn` も消した)。
- 関数名の artifact(`func-names`)は、`f_<VA>` の形の名前だけで、Delphi の名前は無い。
