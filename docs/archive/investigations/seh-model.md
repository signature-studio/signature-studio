# SEH(x64 構造化例外処理)のモデル: FLEngine と同梱 DLL の棚卸し、Delphi の例外機構の読み、モデルの選択肢(決定 0131 の調査。何も決めない)

作業日: 2026-11-01。基準: ブランチ main、HEAD 1d7173f。調査担当(サブエージェント)の成果物。**この文書は何も決めない**(決めるのは PM)。リポジトリの既存ファイルは編集していない。新規は本書と `docs/investigations/seh-model/`(スクリプトと TSV)だけ。

## 0. 結論(5 行)

1. FLEngine の SEH は **Delphi 独自のハンドラ 1 種類(0x4112c0)に集約**される。UNWIND_INFO にハンドラを持つ関数は 10,837(UHANDLER のみ 8,021、EHANDLER+UHANDLER 2,816)。QFC・ILTools も同一のハンドラ本体(452 命令)を 1 つずつ持つ。dsp・Blend2D・freetype は MSVC 系(複数、形で読んだ)。
2. ハンドラは `.pdata` の LSDA にある **scope 表**(`{count; count × {begin, end, f2, f3}}`)を引き、except に当たると **`RtlUnwindEx(TargetFrame, TargetIp=着地, …)` を自分で呼ぶ**(読み。逆アセンブルと紙上の再現で確認、実行はしていない)。着地 VA は LSDA から**静的に全部列挙できる**(FLEngine 2,938。すべて親関数の内側の命令境界。現在の生成 C にはラベルが 0 個)。
3. CreateFruityInstance の直接 call の閉包(6,591 関数)で、Delphi の例外の新規送出の呼び出し点は **192**(`f_411e60` 10 + `f_411e80` 179 + `f_411d20` への直接 3。すべて漏斗 `f_411d20` を通って `RaiseException` に届く。`RaiseException` の直接の呼び出し点 5 は閉包に無い)、except の着地は 711。ただし**壁 16 の鎖自体が閉包に入らない**(vtable 経由)ので、これは下限。
4. 壁 16 は **2 回の送出**(`f_5df610` の `on EOleSysError` が新しい例外を送出し、`f_63ade0` の `on Exception`(着地 0x63ae4e)が受ける)。途中で finally の funclet 2 つが走る。**「握りつぶすだけ」の限定モデル(S2)では壁 16 すら通らない**(1 つ目の着地が「新規送出」型)。
5. 推奨(根拠は §4): G1〜G3 の最短は S3(ITaskbarList の局所対応)。ただし **S1(ゲストの SEH を実行する)は G6 より前に必要**で、生成器の変更は小さい(着地を種にし、着地を持つ関数にラッパー。§3.3 の実験)。2 件目の送出の壁が出たら S1 に切り替える。

## 1. 凡例・手順

- **実測** = 記録・実行結果・PE の値(自作の PE リーダ `pdata_inv.PE` と capstone で読んだもの)の再確認。**読み** = 逆アセンブルや文書の読み(実行していない)。**未確認**。
- 実機の Windows とは照合していない。**FLEngine のコードは一度も例外を送出・受理する経路を実行していない**(壁 16 の停止まで)。例外の機構の読みは、静的な読みと紙上の再現(`sim_scopes.py`)だけ。
- 入力: `binaries/win64/FLEngine_x64.dll`(SHA-256 b2c64da4…9496d、ImageBase 0x400000)と同梱 5 本(`lift/out-idle1/inputs/install/` から `/tmp/opencode/seh1/in/` へ読み取りコピー。SHA-256 は `tsv/run-output.txt` の先頭に記録。dsp 497dd4f7…、Blend2D 66634d24…、QFC afc5a0ed…、freetype 6ea6c106…、ILTools 20409dcb…)。
- Learn で読んだページ: RaiseException(`dwExceptionFlags` は 0 = 継続可、EXCEPTION_NONCONTINUABLE = 継続不可。ハンドラが無ければ既定は ExitProcess)、RtlUnwindEx(引数 6 個、TargetFrame・TargetIp・ReturnValue の意味)、x64 exception handling(UNWIND_INFO、ハンドラの 4 引数、DISPATCHER_CONTEXT のフィールド、unwind の手順)。
- 手順(すべて `docs/investigations/seh-model/`。`run_all.sh INDIR WORKDIR` が全部を通す。出力は `tsv/run-output.txt` に 174 行):
  `pdata_inv.py`(.pdata と UNWIND_INFO の flags、ハンドラ RVA)、`scope_census.py`(scope 表)、`landing_class.py`・`landing_check.py`(着地の位置と命令境界)、`landing_regs.py`・`landing_livein.py`(着地での callee-saved レジスタ)、`landing_kinds.py`(着地の型)、`unwind_ops.py`(unwind コードの数と prologue との突き合わせ)、`raise_sites.py`(SEH 系 import の呼び出し点)、`seh_reach.py`・`raise_classes.py`(CreateFruityInstance からの静的到達)、`sim_scopes.py`(壁 16 の紙上の再現)。呼び出しグラフの索引は `docs/investigations/goal-ladder/fle_index.py` の出力(直接 call/jmp rel32、lea、rip 相対)。
- **到達の 3 段階**(CreateFruityInstance = 0x1375990 から): T1 = 直接 call/jmp のみ(6,591 関数)、T2 = T1 + lea で取られた関数開始(8,953)、RTA = T2 + 「rip 相対で取られた再配置済みポインタ表(VMT など)の先を全部到達とみなす」型の推定(19,183。**緩い推定で、健全な上界でも下限でもない**。独立に書き直した簡易版は 19,164 で、数十の差が出る)。

## 2. 棚卸し(実測)

### 2.1 UNWIND_INFO のハンドラ(`tsv/handler-summary.tsv`、`tsv/handlers.tsv`)

| 画像 | RUNTIME_FUNCTION | flags=0 | EHANDLER のみ | UHANDLER のみ | E+U | 連鎖 | ハンドラ付き | ハンドラ RVA の種類 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| FLEngine | 57,816 | 46,979 | 0 | 8,021 | 2,816 | 0 | 10,837 | **1**(0x4112c0) |
| QFC | 6,347 | 5,305 | 0 | 681 | 361 | 0 | 1,042 | **1**(0x40e590) |
| ILTools | 11,747 | 9,949 | 0 | 1,225 | 573 | 0 | 1,798 | **1**(0x40cc60) |
| dsp_ippv2 | 50,529 | 28,419 | 12 | 26 | 2,359 | 19,713 | 2,397 | 7 |
| Blend2D | 2,493 | 1,444 | 13 | 71 | 315 | 650 | 399 | 4 |
| freetype | 2,429 | 1,195 | 5 | 26 | 21 | 1,182 | 52 | 2 |

- **ハンドラの種類(読み)**。FLEngine・QFC・ILTools の 3 本のハンドラ本体は、アドレスと rip 相対の差を除いて **452 命令で同一**(実測。2,380 バイト。QFC と ILTools を FLEngine と命令ごとに比較)。シンボルは無く名前は取れない(文字列は import 名のみ)。**Delphi 独自**と読む根拠は 2 つ: (a) LSDA の形が下の scope 表で、型の比較に Delphi の VMT(クラス名の Pascal 文字列 `[VMT-0x88]`、親 `[VMT-0x78]`)を使う(実測: `EOleSysError` > `EOleError` > `Exception` > `TObject` が読めた)、(b) 例外コード 0x0EEDFADE。
- dsp・Blend2D・freetype のハンドラは MSVC 系と**形で読んだ**(未確認。名前なし): 本体 29 バイトで `r9+0x38`(HandlerData)を読み `eax=1` で返すもの(GS 検査の形。dsp 2,350 関数、Blend2D 23、freetype 21)、523 バイト・438 バイトで **`RtlUnwindEx` を呼ぶもの**(`__C_specific_handler` の形。dsp 38、Blend2D 44、freetype 31)、`r9+8`(ImageBase)を読む 134〜191 バイト(C++ の frame handler の形。Blend2D 331、dsp 4+1)。dsp・Blend2D・freetype は FLEngine の起動経路の主役ではない。
- 連鎖(CHAININFO)は FLEngine・QFC・ILTools で 0。UNWIND_INFO の version は FLEngine・QFC・ILTools で全部 1(dsp と Blend2D は 2 件ずつ別)。

### 2.2 scope 表と着地(FLEngine。`tsv/flengine-landing-kinds.tsv`)

LSDA は `{u32 count; count × {u32 begin, end, f2, f3}}`(RVA)。ハンドラが読む(§2.4)形と一致して全関数で範囲が親関数の内側(`bad_ranges` 0)。

| 項目 | FLEngine | QFC | ILTools |
|---|---:|---:|---:|
| scope の総数 | 16,940 | 1,620 | 2,829 |
| finally(f2 = 0。f3 = funclet = **`.pdata` の関数開始**) | 14,035 | 1,245 | 2,235 |
| 型なし except(f2 = 2。f3 = 着地) | 2,722 | 371 | 590 |
| 型つき except(f2 > 2。`on` 表。項目 = {型のセルの RVA, 着地の RVA}) | 183 scope(着地 216、うち「どの型でも」22) | 4 | 4 |
| f2 = 1 | 0 | 0 | 0 |
| 1 関数あたりの scope(最大) | 66 | 8 | 20 |
| **着地(except 側)の VA の数** | **2,938** | 375 | 594 |

- **着地は静的に全部列挙できる**(実測): 2,938 すべてが親関数の命令境界(開始から線形走査して 2,938 / 2,938)。finally の funclet 14,035 はすべて `.pdata` の関数開始(= 現在の lift で関数入口になる)。**着地 2,938 は現在の生成 C に L_ ラベルが 0 個**(`lift/out-perf1/onat/gen/flengine.c` を読み取りで走査。着地は制御流から到達できず、lift されていない)。
- ハンドラ付きの 10,837 関数は**全部 frame register = rbp、offset 0**(実測。したがって EstablisherFrame = 関数の rbp)。UHANDLER のみ 8,021 = 「finally だけ」の関数、E+U 2,816 = 「except を持つ」関数(個数が `scope_census` の分類と一致)。
- **着地の型**(`landing_kinds.py`。着地から関数内の CFG を辿り、`call` の先を見た読み。2,938): **再送出 2,392(81.4%)**(`f_411ea0` を呼ぶ。cleanup してから伝播)、**握りつぶし 538(18.3%)**(`f_411fc0` / `f_411ef0` で終わり、再送出も新規送出もしない)、**新規送出 8**。型つきの except の型の上位: `Exception` 112、どの型でも 22、`EFCreateError` 15、`EZipMaster` 12、`ERegistryException` 8。
- **着地での callee-saved レジスタ**(`landing_livein.py`): rbx・rsi・rdi・r12〜r15・xmm6〜15 が**着地で live-in になる着地は 0**(2,935 は無し、3 は状態数の上限で不明)。着地は rbp を読む(2,876)。つまり着地に要るのは「rbp = EstablisherFrame と、`rax` = 例外オブジェクト」で、callee-saved の復元は着地側の要件にならない(読み。静的な前向き走査)。
- 保護範囲の中の `call`: 直接 12,273、間接 1,342(非 finally の 2,905 scope = 型なし 2,722 + 型つき 183 の保護範囲の合計。範囲が重なる scope は重複して数える)。

### 2.3 unwind コード(`tsv/unwind-ops.tsv`)

FLEngine: PUSH_NONVOL 87,979、ALLOC_SMALL 47,552、SET_FPREG 28,324、ALLOC_LARGE 3,000、SAVE_XMM128_FAR 1,901、SAVE_XMM128 61。**SAVE_NONVOL(_FAR)・PUSH_MACHFRAME・EPILOG はゼロ**(QFC・ILTools も同じ 6 種)。unwind コードから出した「rsp の下がり幅」と prologue の逆アセンブル(`push`・`sub rsp`・`add rsp,-N`・`lea rsp,[rsp-N]`)の合計が **FLEngine の 57,816 全関数で一致**(QFC 6,347、ILTools 11,747 も全部一致。実測)。ただし**確かめたのは rsp の下がり幅だけ**で、どのレジスタをどこに保存したか、SAVE_XMM128 の offset は突き合わせていない。FP 付きの関数の frame offset は 0 が 28,285、1〜8 が 39。

### 2.4 SEH 系の import と呼び出し点(`tsv/sites-*.tsv`。静的)

| 画像 | RaiseException の呼び出し点(例外コード) | RtlUnwindEx | RtlUnwind | RtlCaptureContext / LookupFunctionEntry / VirtualUnwind | UnhandledExceptionFilter / SetUnhandled… | AddVectoredExceptionHandler |
|---|---|---:|---:|---|---|---:|
| FLEngine | 5(0xc06d0057・7e・7f が各 1 = 遅延 import ヘルパー `f_41d9a0`、0x406d1388 が 2)+ **Delphi の漏斗 `f_411d20` 内の `call [0x15e6030]`**(ポインタ経由) | 1(ハンドラ内 0x4116fe) | import のみ(`f_412160`・`f_41ce10` がアドレスを `[0x15e6038]` に格納。呼び出し 0) | import なし | 1 / import なし | 1(`f_8eb460`) |
| QFC | 4(同じ 3 + 0x406d1388 が 1) | 1 | import のみ | import なし | 1 / なし | 0 |
| ILTools | 3(同じ 3 のみ) | 1 | import のみ | import なし | 1 / なし | 0 |
| dsp | 4(0xe06d7363 が 1、不明 3) | 3 | — | 4 / 5 / 4 | 3 / 3 | 0 |
| Blend2D | 3(0xe06d7363 が 1、不明 2) | 2 | — | 4 / 4 / 4 | 3 / 3 | 0 |
| freetype | 0 | 3 | — | 4 / 1 / 1 | 4 / 4 | 0 |

- 0x406d1388 はスレッド名の慣用句(`IsDebuggerPresent` が 0 なら送出されない: 決定 0126 の読み)。FLEngine の 2 点のうち 0xb38c20 の側の条件は**未確認**。0xc06d007e/7f/57 は遅延 import の失敗(決定 0099。SEH の問題ではなく環境の応答の欠落)。
- **Delphi の送出の漏斗**(実測): `RaiseException` の IAT thunk は `[0x15e6030]` に格納され(`f_412160` と `f_41ce10` の初期化が `lea` の thunk アドレスを書く)、`f_411d20` だけが `call [0x15e6030]` で呼ぶ。`f_411d20` の呼び元は 3 関数(`f_411e60`、`f_411e80`、`f_411ea0` = 再送出)。`f_411e60`・`f_411e80` を呼ぶ**新規送出の呼び出し点は FLEngine 全体で 1,245**(`f_411e60` 150 + `f_411e80` 1,092 + `f_411d20` への直接 3)、再送出 `f_411ea0` は 2,388。`f_411ea0` 自体も `f_411d20` を呼ぶので、`f_411d20` の呼び元 3 関数は「新規送出 2 + 再送出 1」の関係(読み)。
- `AddVectoredExceptionHandler` の呼び出しは `f_8eb460`(呼び元 `f_8ed980`。T1・T2 の閉包に無い)で、登録されるハンドラ 0x8eb430 は「アクセス違反でアドレスが 0 なら `ExitProcess(0)`」(読み)。

### 2.5 CreateFruityInstance からの静的到達(`tsv/run-output.txt` §6、§7)

| 量 | 全体 | T1(6,591) | T2(8,953) | RTA(19,183) |
|---|---:|---:|---:|---:|
| 漏斗 `f_411d20` の呼び出し点(3 経路) | 3 | 3 | 3 | 3 |
| 新規送出の呼び出し点(`f_411e60` + `f_411e80` + `f_411d20`) | 1,245 | **192** | 220 | 571 |
| 再送出の呼び出し点(`f_411ea0`) | 2,388 | 597 | 713 | 1,266 |
| ハンドラ付きの関数 | 10,837 | 1,901 | 2,537 | — |
| except を持つ関数 / except の着地 | 2,816 / 2,938 | 677 / 711 | 839 / 888 | — |
| finally の scope | 14,035 | 2,489 | 3,431 | — |
| 保護範囲に「送出しうる関数」への直接 call を含む except の scope | — | 674 | 821 | — |

- **`RaiseException` の直接の呼び出し点(上の 5 点)は T1・T2 の閉包に無い**。閉包に入るのは漏斗経由の送出だけ(遅延 import ヘルパー `f_41d9a0` は `.didata` の stub から jmp で入るので静的閉包に入らない: 決定 0099 の実測と一致)。
- **壁 16 の鎖(`f_63ade0` ← `f_63b060` ← `f_11fba20` ← `f_11d58c0` …)は T1・T2 に入らない**(これらの直接の呼び元は閉包の外。lea で取る関数も無く、再配置ポインタの項にも無い。vtable 経由と読む)。RTA には入る(実測と整合)。**静的閉包は下限**。
- 新規送出の呼び出し点の例外クラス(`raise_classes.py`。呼び出しの直前 40 命令以内の `mov r,[rip+cell]` が指す VMT のクラス名。**ヒューリスティック**): T1 の 192 点 49 クラスの上位は `EEncodingError` 27、`EListError` 26、`EJSONParseException` 14、`EParserException` 14、`EInvalidCast` 12、`Exception` 11、`EThread` 7、`EInOutArgumentException` 7。**「環境の欠落」らしい名前**(OS・入出力・レジストリ・COM・MIDI・スレッド・資源。名前の正規表現で数えた目安)は T1 で 44 / 192(23%)、T2 で 49 / 220、RTA で 71 / 571。残りはデータ・論理の誤りの型(リスト範囲、型変換、JSON など)で、通常は誤り経路でだけ送出される(読み)。

## 3. Delphi の例外機構の読み(逆アセンブル。実行していない)

### 3.1 送出(`f_411d20`)— 問 2 (a)

- `RaiseException(0x0EEDFADE, 1, 8, args)`。**flags = 1 = EXCEPTION_NONCONTINUABLE**。args(8 個)は `f_411d20` の局所配列: **[0] = 送出アドレス(引数 rdx)、[1] = 例外オブジェクト(引数 rcx = TObject のインスタンスのポインタ)、[2]〜[7] = 0**。ハンドラが pass 1 で **[7] に EstablisherFrame を書き込む**(NumberParameters > 7 のとき `[ExceptionRecord+0x58] = rdx`。8 個を渡す理由と読む)。ExceptionObject の型は RTTI(VMT)で比較される(§3.2)。
- 送出の前に `[0x15e6040]`(デバッガ通知の関数ポインタ)が 0 でなければそれを呼ぶ。`.bss` で、rip 相対の書き込みは見つからない(読み。初期値 0 なので通常は呼ばれない)。さらに `f_411050` などの通知用 `RaiseException(0x0EEDFADF〜0x0EEDFAE4)` は `[0x13a5058]`(`.data` の初期値 0。rip 相対の書き込みなし)が 1 を超えるときだけ(読み。通常は送出されない)。

```
; f_411d20 の送出 [0x411e30..0x411e47]
mov ecx,0xeedfade ; mov edx,1 ; mov r8d,8 ; lea r9,[rbp+0xc0] ; call [0x15e6030]  ; = RaiseException
; args: [rbp+0xc0]=rsi(address) [rbp+0xc8]=rbx(object) [rbp+0xd0..0xf8]=0
```

### 3.2 ハンドラ 0x4112c0 — 問 2 (b)(c)

プロトタイプは x64 の標準(rcx = ExceptionRecord、rdx = EstablisherFrame、r8 = ContextRecord、r9 = DispatcherContext)。DISPATCHER_CONTEXT の ControlPc(+0)、ImageBase(+8)、TargetIp(+0x20)、ContextRecord(+0x28)、HandlerData(+0x38 = scope 表)、HistoryTable(+0x40)を使う。**`ExceptionFlags & 0x66`(UNWINDING・EXIT・TARGET・COLLIDED)が 0 なら pass 1(探索)、そうでなければ pass 2(巻き戻し)**。

- **pass 1(探索)**: scope 表を表の順に見て、`begin ≤ ControlPc − ImageBase < end` の scope について: f2 = 0(finally)は飛ばす。f2 = 2(型なし except)は着地 = ImageBase + f3。f2 > 2 は `on` 表を引き、各項目で**型のセルの RVA が 0 なら一致、そうでなければ ExceptionInformation[1] のオブジェクトの VMT が「セルの VMT と同じ」「インスタンスサイズとクラス名の Pascal 文字列が同じ」のどちらかなら一致、違えば親(`[VMT-0x78]`)へ**(`f_4111d0`)。コードが 0x0EEDFADE でなければ関数ポインタ `[0x15e6020]`(Delphi の `ExceptObjProc` に当たると読む)で例外オブジェクトを作る(0 なら一致しない。この 2 つのポインタ `[0x15e6020]`・`[0x15e6028]` の書き込みは rip 相対では見つからない: **未確認**)。
- 一致したら: `ExceptionFlags |= 2`、スレッド局所の「例外ノードの表」(TLS + 0x208)にノード {送出アドレス, オブジェクト} を push し、**`RtlUnwindEx(TargetFrame = rdx(EstablisherFrame)、TargetIp = 着地、ExceptionRecord、ReturnValue = ExceptionInformation[1](オブジェクト)、ContextRecord = DC.ContextRecord、HistoryTable = DC.HistoryTable)` を呼ぶ**。呼び出しは `call` のあとに `ExceptionContinueSearch`(1)を返す文があるが、**一致すれば `RtlUnwindEx` は戻らない**。**ハンドラは `ExceptionContinueExecution` も `EXCEPTION_NONCONTINUABLE` の返しも使わず、`jmp` も `RtlRestoreContext` も使わない**(読み。着地への制御移動は `RtlUnwindEx` の内部の文脈復元)。

```
; ハンドラ 0x4112c0 pass 1、型の一致の後 [0x4116ec..0x4116fe]
mov rcx,[rbp+0x148]  ; TargetFrame = EstablisherFrame (rdx)
mov rdx,[rbp+0x88]   ; TargetIp = ImageBase + 着地の RVA
mov r8,[rbp+0x140]   ; ExceptionRecord
mov r9,[rbp+0xb8]    ; ReturnValue = ExceptionInformation[1] (オブジェクト)
mov r10,[rax+0x28] ; mov [rsp+0x20],r10     ; ContextRecord (DC+0x28)
mov rax,[rax+0x40] ; mov [rsp+0x28],rax     ; HistoryTable  (DC+0x40)
call 0x406ff0        ; thunk -> kernel32!RtlUnwindEx
```

- **pass 2(巻き戻し。`RtlUnwindEx` が各フレームのハンドラを UNWINDING つきで呼ぶ)**: `begin ≤ ControlPc < end` の scope について、`TARGET_UNWIND` かつ `TargetIp` がその scope の中なら**そこで戻る**(着地を含む scope とそれより外の finally は走らない)。f2 = 0 は **finally の funclet を `call`**(rcx = `&EXCEPTION_POINTERS{ExceptionRecord, ContextRecord}`、rdx = EstablisherFrame)。funclet の後に **MXCSR(`stmxcsr`)と x87 制御語(`fnstcw`)を ContextRecord に書く**(+0x34、+0x100)。DC.ControlPc は scope の end に更新する。f2 ≥ 2 の scope は、`TARGET_UNWIND` のとき着地を再計算し `DC.TargetIp` と一致すれば 1 を返して終わる。返す値は常に 1(ExceptionContinueSearch)。

```
; ハンドラ 0x4112c0 pass 2、finally の funclet [0x411866..0x4118b0]
lea rcx,[rbp+0xd0]   ; EXCEPTION_POINTERS {ExceptionRecord, ContextRecord}
mov rdx,[rbp+0x148]  ; EstablisherFrame
call [rbp+0x88]      ; funclet = ImageBase + f3
call 0x40a3b0 ; mov [rcx+0x34],eax    ; MxCsr -> ContextRecord
call 0x40a330 ; mov [rcx+0x100],ax    ; x87 制御語 -> ContextRecord
```

### 3.3 try…finally の後始末(UHANDLER)— 問 2 (d)

finally の funclet は pass 2 でハンドラが呼ぶ(上)。**例外が無い通常の経路では funclet は走らず、finally の本体は関数の中に複製されている**(読み。`f_63ade0` の本体の `0x63af73`: `lea rcx,[rbp+0x20]; call f_41a770` と funclet 0x63afb0 の `lea rcx,[rdx+0x20]; call f_41a770` が同じ)。funclet は親の局所を `rdx`(EstablisherFrame、= 親の rbp)で参照する。

### 3.4 壁 16 の紙上の再現(`sim_scopes.py`。実機の観測ではない)

壁 16 の呼び元の鎖(`logs/w20i` の停止と一致)に、実際の scope 表とクラス表を当てた。

| 送出 | 呼び元のフレーム(関数、ControlPc) | pass 1 で受けるもの | pass 2 で走るもの | 着地 |
|---|---|---|---|---|
| 1 回目: `f_5df540` が `EOleSysError`("Class not registered")を送出 | f_411d20、f_411e80、f_5df540、f_5df570、**f_5df610**(0x5df66f)、f_63ade0 | `f_5df610` の `on EOleSysError`、scope [0x5df63c,0x5df672) | 途中のフレームにハンドラ無し | **0x5df673**(`rax` = オブジェクトを局所に保存し、メッセージを作って**新しい例外を再び送出**) |
| 2 回目: `f_5df610` の着地が `EOleSysError` を送出(0x5df6e2) | f_411d20、f_411e80、f_5df610(0x5df6e7)、**f_63ade0**(0x63ae33) | `f_63ade0` の `on Exception`(EOleSysError の親鎖に Exception がある)、scope [0x63ae23,0x63ae4d) | `f_5df610` の finally funclet 0x5df720(`f_411ef0`)と 0x5df730(文字列 2 つの解放)を表の順に | **0x63ae4e** |

- 着地 0x63ae4e(`f_63ade0`)の先(読み): グローバルのインターフェース変数 `[0x15f3320]` を `f_41a770` で解放(NULL なので何もしない)→ `f_411fc0`(例外ノードを pop し、例外オブジェクトを解放し、デバッガ通知)→ `jmp 0x63ae62` → `[0x15f3320]` が 0 なので `CoUninitialize` → 関数の末尾で局所の解放 → **`al = 0`(FALSE)で戻る**。呼び元 `f_63b060` は `[0x15f3330]` が 0 なので後続の呼び出し(vtable +0x50)を飛ばす。**ゲストは「TaskbarList 無し」のまま続行する**(読み)。
- 着地の直前に nop が 2 つある(`f_63ade0` の 0x63ae4c、0x63ae4d。scope 0 の end は 0x63ae4d で、着地 0x63ae4e はその外)。**詰め物と読むのは推測**(根拠は無い。ControlPc は戻りアドレスで `begin ≤ pc < end` の半開区間で比べるので、`call` が scope の最後の命令のとき戻りアドレスが範囲の外に出ることへの備えかもしれないが、確かめていない)。**未確認**。
- **idle1 の読みの訂正**: idle1 の wall-log は「except 側 0x63ae4c」と書くが、scope 表の着地は **0x63ae4e**(0x63ae4c は nop)。

```
; f_63ade0 の except の着地 [0x63ae4e..]
lea rcx,[0x15f3320] ; call f_41a770        ; インターフェース変数の解放(NULL なら何もしない)
call f_411fc0                              ; except の終わり: 例外ノードを pop、オブジェクトを解放
jmp 0x63ae62                               ; 以後 CoUninitialize → 末尾 → return al=0
```

実機の Windows では CLSID_TaskbarList はシェルが登録している(一般知識。未測定)ので、実機は**成功の経路**を走る。S1 を採ると、環境の定義(決定 0129 の REGDB_E_CLASSNOTREG)のとおり**失敗の経路**が走る。


## 4. モデルの選択肢と評価

**規模の桁について**: 以下の「行数の桁」は、この調査で読んだ構造(ハンドラが使う構造体、`fle_ole.c` の IMalloc 前例の行数)からの**作業者の見積もり**で、実測ではない。実装していない。日数は書かない。

### 4.1 S1 の部品の読みと小さな実験

**ゲストの rsp と戻りアドレスの連鎖を UNWIND_INFO で辿れるか(読みと実測)**。lifted C のゲストスタックはゲストの線形メモリに実在する(`R[4] -= 8; fle_st(R[4], 8, <戻りアドレス>)`)。したがって各フレームについて「ControlPc = 戻りアドレス、`RUNTIME_FUNCTION` を `.pdata` から二分探索、unwind コードで rsp を戻す」ができる。FLEngine・QFC・ILTools(Delphi の RTL 由来の生成コード)の unwind コードは **PUSH_NONVOL・ALLOC_SMALL/LARGE・SET_FPREG・SAVE_XMM128(_FAR)の 5〜6 種だけ**で、rsp の下がり幅は全 57,816 関数で prologue と一致した(実測、§2.3)。SET_FPREG の関数は frame register が rbp(offset は 0 が 28,285)。ハンドラ付き 10,837 関数は全部 rbp(offset 0)で、EstablisherFrame = そのフレームの rbp。**ただし**: (a) 確かめたのは rsp の下がり幅だけで、SAVE_XMM128 の保存位置や、callee-saved の「どこに保存したか」は未確認。(b) **フレームの連鎖を辿るのに必要なもの**は、各フレームの「rsp の値」(戻りアドレスの位置を出すため)。SET_FPREG の関数は frame register が rbp で、ハンドラ付き 10,837 関数は全部「rbp = EstablisherFrame」。したがって**ハンドラ付きのフレームでは rbp から戻りアドレスの位置が出る**(prologue 後は rbp と固定の割り当てで決まる)と読むが、**未実施**(動的な alloca の後の rsp は unwind コードから出ず、rbp を基準にする必要がある)。ハンドラ無しのフレームは unwind コードで rsp を戻す。(c) **epilog の途中**(x64 の unwind の手順 case a)や prologue の途中で例外が来る場合は、`RaiseException` の呼び出し(`call` の中)からの送出なら無関係。**ハードウェア例外(0 除算、アクセス違反)は別**: 現在の runtime は 0 除算で `abort`(`fle_div_fault`)、不正アドレスは検査つきビルドでだけ名前付き停止で、いずれも SEH に入らない。S1 の範囲は `RaiseException` からの送出に限る、というのが最小(読み)。(d) `fle_call_guest` の入れ子を越えて辿ると、**ホスト(C ランタイム)のフレームがゲストのスタックの上に挟まる**(戻りアドレスは `FLE_CB_RETURN_ADDRESS` = 0xCB000000)。このとき「次のフレーム」はホスト側の再開点で、ゲストのスタックの連鎖はそこで終わる。実機の Windows でコールバックを越える例外がどう扱われるかは**未確認**(一般知識でも言い切れない)。lifted の `fle_call_guest` を越える例外は新しい規則が要る(止めるのが最小)。

**着地への制御移動を C で表す(読みと実験)**。決定 0100 の機構(着地を関数入口にして、呼び元の C フレームに `_setjmp` を置く)は **setjmp の戻りアドレスが 1 つの `call` の直後で、ゲストがその後に longjmp する**ことに合わせた形。except の着地は、**関数の内部の命令境界**で、(i) `.pdata`/LSDA から静的に全部列挙できる(2,938)、(ii) **「どの C フレームに戻るか」はハンドラが TargetFrame として指す**(その関数が C フレームとして生きている。例外の送出は常に `call` の中なので、着地を持つ関数の C フレームは送出の時点で C スタックに乗っている)。決定 0100 との違い: 0100 の戻り先は「setjmp を呼んだ直後の 1 点」で、呼び元に `_setjmp` を置けば足りた。except は**同じ関数が `try` を持つ位置を問わず**、保護範囲のどの `call` からも戻る。

実験(小さく。`fe-lift` を HEAD 1d7173f の git archive から作り(`lift/out-seh1/` に作って終了時に削除)、FLEngine の生成 C を `--pdata --seeds` で 2 通り作り、出力を数えた。**実行はしていない**。`tsv/felift-base.stats.txt`、`tsv/felift-landing-seeds.stats.txt`、`tsv/flengine-landing-seeds.txt`):

| 項目 | 基準(現行の seeds `flengine-seeds-thunks.txt`) | 着地 2,938 を `--seeds` に足した | 差 |
|---|---:|---:|---:|
| functions | 59,388 | 62,326 | +2,938 |
| closure_instructions | 2,528,284 | 2,565,788 | +37,504 |
| c_bytes | 215,806,826 | 224,707,150 | +8,900,324(+4.1%) |
| bad_nodes | 0 | 0 | 0 |
| unsupported_total | 252 | 252 | 0 |
| `L_63ae4e` / `f_63ae4e` | 無し | 有り(`f_63ae4e` が独立の関数。入口で `R[1] = 0x15f3320` から始まり、`jmp 0x63ae62` 以降(`CoUninitialize` と末尾)を**自分の本体に複製**) | |

- 読み: **生成器の変更は「着地を種にする」だけで通る**(`bad_nodes` 0、未対応命令は増えない)。着地の後の尾部(`L_63ae62` 以降)は `f_63ade0` と `f_63ae4e` の**両方に複製**される(命令は到達した最初の関数に属し、他方は同じ命令を自分の本体にも出す。上の +37,504 命令)。複製は C の関数の本体を増やすだけで、動作は変わらない。
- **まだ無いもの**(この実験は含まない。生成器と runtime の変更): (1) 着地を持つ関数(2,816)を、`f_L` を `fle_landing` 型のラッパーで呼ぶ形にする(決定 0100 の `fle_landing_push/try` の拡張。ただし 0100 のラッパーは「その関数が自分の中で呼ぶ 1 点」にだけ置いた)。except は**保護範囲の `call` ごと**に着地へ戻るので、ラッパーは**関数の入口に 1 つ**(`fle_seh_frame` の登録: 関数の C フレームに `_setjmp`、登録 = {EstablisherFrame(rbp)、着地の表への参照})にして、`RaiseException` が `longjmp` する。(2) `RaiseException` の実装(pass 1/2 と `RtlUnwindEx` 相当)。(3) 着地への「rbp = EstablisherFrame、rsp の再計算、`rax` = オブジェクト」の復元(rsp は rbp と frame offset から出る: ハンドラ付き 10,837 関数は全部 rbp 基準)。(4) finally の funclet を呼ぶこと(全 14,035 が `.pdata` の関数開始なので既に関数。`fle_call_guest(funclet, &EXCEPTION_POINTERS, EstablisherFrame, …)` で呼べる)。

**ゲスト側のハンドラを実行する選択肢(S1a)と、ハンドラの働きをホストで再実装する選択肢(S1b)**。S1a: ゲストのハンドラ 0x4112c0 をそのまま `fle_call_guest(handler, ExceptionRecord, EstablisherFrame, ContextRecord, DispatcherContext)` で呼ぶ。ハンドラが読む構造(ExceptionRecord、DISPATCHER_CONTEXT、`ContextRecord` の MxCsr(+0x34)と x87 制御語(+0x100))を作り、**ハンドラが呼ぶ `RtlUnwindEx` を行う**(ホストの `fle_imp_RtlUnwindEx`: 「TargetFrame を持つフレームの着地へ longjmp する」+ 途中のフレームのハンドラを UNWINDING で呼ぶ)。**ゲストのコードが走る(偽の応答が少ない)**。S1b: scope 表の探索をホストで書き、ハンドラは呼ばない。ホストが Delphi の VMT 規約を知ることになる(**ゲストの規則をホストに複製する**)。S1a のほうが実機に近い。S1a のための ContextRecord の中身: ハンドラが読むのは MxCsr/x87 の書き込みと `EXCEPTION_POINTERS` の渡しだけで、**Rip/Rsp/レジスタは読まない**(ハンドラ本体 452 命令を通読した範囲。読み)。`RtlUnwindEx` 側は **ContextRecord にレジスタが無いので、ホストが持つ rbp と rsp を TargetFrame から出す**。

### 4.2 選択肢の表

| 選択肢 | 実機との同一性 | 実装の規模(桁) | fe-lift / fe-decode の変更 | wasm・JSPI での成立性 | 偽りの有無 | リスク |
|---|---|---|---|---|---|---|
| **S0 止まる(現状)** | 壁 16 で止まる | 0 | 無し | 無関係 | 無し(名前付き停止) | G1 以降に進めない |
| **S1 ゲストの SEH を実行する**(RaiseException = pass 1/2 + RtlUnwindEx 相当。S1a = ゲストのハンドラを呼ぶ) | **高い**: ゲストのハンドラ(452 命令)が実機と同じ表を引き、同じ funclet が同じ順で走る。ただし ContextRecord はレジスタを持たず(読み)、実機の OS の例外ディスパッチ(デバッガ通知、VEH、SEH チェーンの検証)は無い | 生成器: 小(着地を種にする、関数入口のラッパー出力)〜中。runtime: **100〜1,000 行の桁**(RaiseException/RtlUnwindEx、EXCEPTION_RECORD/DISPATCHER_CONTEXT/CONTEXT の構築、フレームの連鎖を辿る、ラッパーの登録と longjmp)。テスト: 紙上の再現(`sim_scopes.py`)が土台 | **要る(小)**: 着地 2,938 を種にする(実験: bad_nodes 0、C +4.1%)。関数入口の `fle_landing` 型ラッパー(着地を持つ 2,816 関数)。fe-decode の変更は不要の見込み(`--seeds` で足りる) | **SJLJ=wasm の `try/catch` を使う**(決定 0124、`jspi-real-wasm.md`: SJLJ=emscripten は JSPI と共存できない。SJLJ=wasm では中断の内側の着地が通った)。**全着地の関数が C の setjmp フレームを持つ**ので、wasm の関数が 2,816 個、`try` を持つ(コストは未測定。現在の着地は 1 個)。例外の送出は JSPI の中断と無関係(同期の C 呼び出し) | 無し(ゲストの規則を実行する)。ただし**捨てた C フレームの副作用は戻らない**(決定 0100 の限界。`fle_call_guest` の入れ子カウンタ、ホストのロック。FLEngine の except の途中に `fle_call_guest` のフレームがある場合に効く。未測定) | 中: (1) 着地の本体は lift されていないので、lift した初回は未実行の命令(既存の未対応・間接呼びの壁)が出る。(2) 「送出が C のどのフレームに戻るか」の対応が崩れると名前付き停止(安全側)。(3) スレッド(決定 0122)ごとの landing 表: `fle_thread.c` が `fle_landing_top` を切り替えているので、同じ要領で例外ノード表(TLS + 0x208 に相当)が要る(ゲストのメモリなので自然に切り替わる)。(4) G6 以降の例外(ファイルの失敗、操作の誤り)が**実機と同じ規則で**通る |
| **S2 Delphi の限定モデル**(except が「握りつぶして続行」の形だけを fe-lift が変換) | 部分的 | 小〜中 | **要る(中)**: 握りつぶしの着地の呼び出し点の変換 | S1 と同じ(`try/catch`) | 無し | **壁 16 すら通らない**(§3.4: 1 つ目の着地は「新規送出」型、2 つ目は握りつぶし)。FLEngine の着地のうち握りつぶし型は 538(18.3%)、再送出型 2,392(81.4%)、新規送出 8。**finally(再送出型の大半)を無視すると資源が戻らない**。実質、S1 の部品をほぼ全部要し、得られる範囲は小さい |
| **S3 `RaiseException` を呼ばせない**(環境を作る。壁 16 は ITaskbarList の最小実装で CoCreateInstance を成功させる) | **低〜中**: 実機は ITaskbarList が成功する経路(一般知識。未測定)で、S3 はそれに寄せる。ただし ITaskbarList の vtable をホストが持つので**偽の COM オブジェクト**を作ることになる | 壁 16 の 1 件: 小〜中(決定 0129: 約 150〜300 行。ホストの vtable、HrInit/AddTab/…/SetActiveAlt、QueryInterface の `ITaskbarList3` 側は E_NOINTERFACE にするか実装、`fle_ole.c` の IMalloc の前例あり) | 無し | 無関係(例外が無い) | **ある**(ホストが偽の `ITaskbarList` を返す。実機はシェルが返す) | **1 件ごとの局所対応**: 起動の経路で `RaiseException` に届く点が環境欠落型の 44 / 192(T1)で、他の経路が必ず出るとは言えない。**S3 で通しても SEH は後で必要になる可能性が高い**(G6 以降の失敗の経路、§4.3) |

### 4.3 段階別: G1〜G3 に必要な範囲と、G6 以降に必要な範囲

**G1〜G3(`CreateFruityInstance` → Run → メッセージループ。到達済みの経路は壁 16 の手前まで実測)**:
- 実測: 壁 16 まで来た経路(入口 141,732,210)は、CreateFruityInstance 開始から**1 回の送出**(2 段、同じ壁)を起こしただけ。それまでに起動の経路でゲストが例外を**受理した**ことは idle1 の記録に無い(`RaiseException` の止まりは 1 回)。
- 読み: 起動の完走(G1〜G3)で SEH が**必要か**は、壁 16 の先に「環境欠落型の送出」がさらにあるかによる。静的閉包(T1 192、T2 220、RTA 571)で環境欠落型は 44 / 49 / 71 だが、**閉包に壁 16 の鎖すら入らない下限**で、実行時の到達数は未測定。
- S3 で壁 16 を通す: 1 件の実装で通る。次の壁が「環境の欠落による送出」なら S3 を繰り返す(1 件ごとの局所対応)。次の壁が「誤り経路での送出」なら、その誤りを環境側で消すか S1。
- S1 で壁 16 を通す: 壁 16 と同じ範囲(2 回の送出、finally funclet 2 つ)が通れば、**以後の環境欠落型の送出もすべて同じ機構で通る**(CoCreateInstance の失敗、ファイル I/O の失敗、レジストリの失敗など)。ただし**各々の失敗の後でゲストが続行できるかは別**(壁 16 のように続行できる保証は無い)。

**G6 以降(ユーザーの操作、ファイルの読み込みの失敗など。アプリが動いている間)**:
- 読み: 実機の FL Studio はユーザーの操作・ファイルの読み込みの失敗で Delphi の例外を**ふつうに送出し、except が受ける**(Delphi の標準的な作り: `try…except` で `ShowException`、`try…finally` での解放)。FLEngine の except の着地 2,938(再送出 2,392 を含む)と finally 14,035 は、**通常の運用で実行される前提で作られている**(読み。実行していない)。したがって **S3 は G6 以降を覆えない**(失敗の種類は環境の欠落に限らない: 壊れたファイル、範囲外、ユーザーの中断(`EAbort`)など。`raise_classes` の上位は `EListError`、`EEncodingError`、`EInvalidCast`、`EJSONParseException`、`EParserException` など論理の誤りの型)。
- **S2(握りつぶし型のみ)は G6 以降も足りない**(再送出型 81% の cleanup が走らない)。
- **S1 だけが G6 以降を覆える**。
- 言い換え: **S3 は壁ごとの回避、S1 は後で必ず要る前提の基盤**。壁 16 の時点では、どちらが短いかは「S3 の 1 件(150〜300 行)」対「S1 の初回(100〜1,000 行の桁、生成器の小変更)」。**S1 は一度作れば以降の壁(例外型)を個別に直す必要が無い**。S3 は回数に比例する。

**(参考)ブラウザ(wasm・JSPI)での補足(読み)**: S1 の `RaiseException` は**同期の C コード**で、JSPI の中断(`Suspending` の import)を挟まない。着地への longjmp は SJLJ=wasm の `try/catch`(決定 0124: JSPI + SJLJ=wasm の組み合わせで setjmp 着地の内側の中断が通った。3 つの `longjmp`(QFC・ILTools・FLEngine の終了)は a4 の一致から通ったと読む)。S1 が増やすのは **着地を持つ関数の数(1 → 2,816)の setjmp フレーム**で、wasm の `try` ブロックを持つ関数が増える。実行時間と wasm のサイズへの影響は**未測定**。決定 0100 の `FLE_LANDING_NOJUMP`(wasm の既定)では着地に当たると停止するので、**S1 は wasm でも `SJLJ=wasm`(`FLE_LANDING_FORCE`)を前提にする**(決定 0124 の `THREADS=1` ビルドは既に `SJLJ=wasm` を要求)。

## 5. 推奨(決めない。根拠つき)

**段階導入案**:

1. **まず S3 の局所対応で壁 16 を通す**(CLSID_TaskbarList を成功させる最小の `ITaskbarList`、または他の局所対応)。根拠: G1〜G3 の最短経路は 1 件の局所対応。S1 は壁 16 の 1 件を通すだけに 100〜1,000 行の桁と生成器の変更を要する(実験は生成器の部分が小さいことだけを示した)。
2. **S3 の後の次の壁で `RaiseException` が出るたびに分類する**。切り替えの基準(次のどれかに当たったら **S1 に切り替える**):
   - (a) 次の `RaiseException` の壁が**環境の欠落ではない**(誤り経路での送出、ユーザーの操作で起こる例外型)。
   - (b) 同じ起動で **2 件目の送出の壁**が出た(S3 の局所対応が回数に比例して増える兆し)。
   - (c) G6(入力の往復)に着手する時点。ユーザーの操作の失敗の経路は S3 で覆えない。**S1 は遅くとも G6 より前に要る**(読み)。
   - (d) idle1 の「文字描画 API が 1 回も呼ばれない」(決定 0129 の発見 2)の原因が、静かに握りつぶされている例外(`try…except` が受けて続行している箇所。例えば現在は止まらず通っている except)に由来すると分かったとき。**静かに受理される例外は S0 のときは止まるので、いま見えていないだけで、S3 で壁 16 を通したあとに増えうる**。
3. S1 を作るときの順序(読みの提案): (i) 紙上の再現(`sim_scopes.py`)を、壁 16 の 2 回の送出で**実行できる形**の最小核にする(pass 1/2、`RtlUnwindEx` 相当、着地への longjmp)。(ii) 着地を種にし(実験: bad_nodes 0)、着地を持つ関数にラッパー。(iii) 壁 16 を **S3 ではなく S1 で通した場合と、S3 で通した場合で、ゲストの状態を比較**できる(S1 は「TaskbarList 無し」の経路、S3 は「有り」の経路で違う: 実機は有り)。(iv) wasm は `SJLJ=wasm` で。

**根拠の要点**:
- G1〜G3 の最短: S3(1 件の局所)。S1 は初回の設計・実装が重い。
- G6 以降の作り直し: S3 だけで進むと壁ごとの偽の応答が増え、**S1 を後で入れると S3 の偽は残る**(実機はシェルが ITaskbarList を返す。S1 の経路は「失敗」で、S3 の「成功」と違う)。S1 を先に入れると、S3 のような環境の偽が要らない件が増える(CoCreateInstance の失敗を許す設計の経路すべて)。ただし S1 だけでは「失敗の後で続行できない経路」は通らない。
- 実機との同一性: S1 > S3(S1 はゲストの規則を実行する。S3 は偽の COM を返す)。
- 偽りの観点: S3 は偽の応答を足す。S1 は足さない(環境の定義は決定 0129 の REGDB_E_CLASSNOTREG のまま)。

**この推奨が崩れる条件**: (1) 壁 16 の先ですぐに別の `RaiseException` の壁が連続して出る(S1 に直行が安い)。(2) S3 の ITaskbarList が実は小さくない(`ITaskbarList3` の QueryInterface、後続の vtable +0x50 など。`f_63b060` は `[0x15f3330]` のオブジェクトの vtable +0x50 を呼ぶ)。(3) S1 の runtime が 100 行台で済む(その場合は S1 を先にする)。

## 6. 限界・未確認

- **実行していない**: ハンドラ 0x4112c0 の挙動(2 パスの分岐、`RtlUnwindEx` への引数、funclet の呼び出し)は逆アセンブルと紙上の再現(`sim_scopes.py`)の読みで、**実機の OS のディスパッチ(`KiUserExceptionDispatcher`、`RtlDispatchException`)との照合は 0 件**。`RtlUnwindEx` の内部(各フレームのハンドラを UNWINDING で呼ぶ順、TargetFrame の扱い、`ContextRecord` の更新、`rax` への ReturnValue の設定)は Learn の 1 ページ(引数の説明だけ)と x64 exception handling のページ(手順)からの読みで、**これも未実測**。
- ハンドラの `[0x15e6020]`・`[0x15e6028]`(Delphi の `ExceptObjProc` に当たると読む。コードが 0x0EEDFADE でないときの例外オブジェクトの変換)と `[0x15e6040]`(デバッガ通知)の書き込み元は rip 相対では見つからなかった。**実行時の初期化の経路は未確認**。
- 名前は取れていない(Delphi のシンボルは無い。`f_411d20` を `RaiseExcept` と呼ぶのは依頼文の読みで、**PE の文字列には Delphi の関数名の断片が無い**。実測: `RaiseException`・`AddVectoredExceptionHandler` の import 名と、`dbkFCallWrapperAddr`・`__dbk_fcall_wrapper`(Delphi の RTL の export 名。決定 0100)だけ)。
- **dsp・Blend2D・freetype のハンドラは形で読んだだけ**(未確認)。C++ の `__CxxFrameHandler`、`__C_specific_handler` の名前は取れていない。`RtlCaptureContext`/`RtlLookupFunctionEntry`/`RtlVirtualUnwind` を呼ぶ点は dsp・Blend2D・freetype の `__report_gsfailure`/`__scrt_fastfail` 型の致命エラー経路と読む(決定 0078。**FLEngine・QFC・ILTools には無い**)。**起動の経路でこれらが呼ばれるかは未測定**。
- 静的到達は **下限**(vtable 経由の呼び出しは T1・T2 の辺にならない。壁 16 の鎖が T1・T2 に入らないことで確認)。RTA は緩い推定。**実行時の到達数(起動の完走までの送出の回数)は未測定**。
- 例外クラスの名前は、呼び出しの直前 40 命令以内の `mov reg,[rip+cell]` の VMT から取ったヒューリスティック。環境欠落型の数え方は名前の正規表現(目安)。T1 の 192 点のうち名前が取れないもの 5。
- unwind コードの突き合わせは **rsp の下がり幅だけ**(callee-saved の保存位置、XMM の保存位置は未確認)。着地での callee-saved の live-in は静的な前向き走査(状態数の上限で 3 着地が不明)。
- 生成 C の着地ラベルの有無は `lift/out-perf1/onat/gen/flengine.c`(別の作業の生成物、読み取りのみ)で見た(idle1 の `obase-on/gen/flengine.c` は作業中に消えていた)。着地を種にした実験は HEAD 1d7173f の `fe-lift` + `flengine-seeds-thunks.txt` で、idle1 の差分(seeds ほか)は含まない。**その C をコンパイルも実行もしていない**。
- 実験(§4.1)は **生成器が通ること**だけを示す。ラッパー、`RaiseException`、longjmp、wasm のビルドは何も作っていない。
- SJLJ=wasm の setjmp 着地が 2,816 関数に増えたときの wasm のサイズ・速度・V8 のコンパイル時間への影響は未測定。ゲストのスレッド(決定 0122)ごとの例外ノード表の切り替えは読みだけ(ゲストメモリの TLS + 0x208 なので自然に切り替わる、と読む)。
- 実機 Windows との照合、実機の `CoCreateInstance(CLSID_TaskbarList)` が成功する観測は 0 件(一般知識)。
- 「`fle_call_guest` を越える例外」「ゲストのコールバック(WndProc など)の中の例外」の扱いは未調査(S1 の範囲外の判断が要る)。

## 7. 成果物

- 本書: `docs/investigations/seh-model.md`。
- `docs/investigations/seh-model/`: スクリプト 12(`pdata_inv.py`、`scope_census.py`、`landing_class.py`、`landing_check.py`、`landing_regs.py`、`landing_livein.py`、`landing_kinds.py`、`unwind_ops.py`、`raise_sites.py`、`seh_reach.py`、`raise_classes.py`、`sim_scopes.py`)と一括実行の `run_all.sh`、`README-line.txt`、`tsv/`(棚卸しの TSV、`run-output.txt`、fe-lift の統計 2、着地の種ファイル)。
