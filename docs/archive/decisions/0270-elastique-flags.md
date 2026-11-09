# 0270(mt3w、仮): Elastique のミキサースレッドの C 経路 3 関数(f_b1ce5e0・f_b25ce20・f_b2cf0c0)の flags を直 wasm にする — call が返すフラグ、可変シフト、bsr/bsf。および 0270b(フラグを読まれる callee を C 経路に残す検査)

基準: 現在の木(e7b-wip c6b62f2d。E7b の 0001/0002/0004/0005 と 0002b 入り。fe-wasmgen のコードは 8c6b295e 以降 0002b を除いて同じ)。差分: `0270-flags.diff`(本体)と `0270b-on-top-of-0270.diff` / `0270b-alone.diff`(§6)。**実エンジン(ブラウザ・node の FL)の走行は 0 件、依頼者の機械は 0 件**。以下の数は、fe-wasmgen の report(census)・`cargo test`・node 単体の合成試験・host CPU との差分試験の測定だけ。性能(ms)は何も測っていない。

## 冒頭の 3 点
1. **測ったこと(事実)**:
   - 3 関数は、fe-wasmgen の report で**全部 `ok`**(直 wasm にできる)になった: f_b1ce5e0 = 68,546 B、f_b25ce20 = 210,574 B、f_b2cf0c0 = 30,455 B(`--hybrid-only` に 3 関数だけを渡した onez の report)。
   - **前提の訂正**: 依頼の『shr r,cl』は 3 関数のうち **f_b2cf0c0 だけ**。f_b1ce5e0 と f_b25ce20(合計 1,492 ms)は、`call 0xb2c8090`(MSVC の `__security_check_cookie`)の**戻りの ZF** を `je` が読む形。しかも f_b2cf0c0 は `shr rax,cl; jae` の後に **`bsr` の ZF** も読むので、**可変シフトの recipe だけでは census 全体で新しく ok になる関数が 0**。`bsf/bsr`(fe-wasmgen に lowering が無かった)を足して初めて 3 関数が通る。
   - census(8 モジュール、基準 = 現コード、`tables/0270-census-*.txt`): 新たに ok **+120 関数(84,221 命令)、失われた関数 0**(oggio +21、Blend2D +9、Elastique +23、dsp +67、他 0)。原因別 = call のフラグ 62、可変シフト 12、bsr/bsf 39、callee が ok になった連鎖 7。『recipe が無い』が理由の関数は 210 → 103(`tables/0270-no-recipe-before-after.tsv`)。
   - **出荷と同じ構成(各モジュールの `--hybrid-only hyb-U-<m>.txt`、U は変えない)で作った wasm は、qfc・ft・ilt・Blend2D・dsp・fle の 6 つがバイト単位で同一**。oggio は 452,307 → 451,219 B(−1,088)、Elastique は 1,977,198 → 1,975,990 B(−1,208)。違う関数は U の中で 16 本(oggio 4・Elastique 12。署名/elision が変わるのは Elastique の 3 本)で、**全部が callee の署名が変わったこと**(『recipe 無し』の C 経路の callee が、保守的な署名から精密な署名になった)で説明できる(`tables/0270-U-changed-production-like.tsv`)。
   - 試験: PM の木のコピー + 0270 で `cargo test -p fe-wasmgen`: **30 結果・62 通過・失敗 0・無視 1**(基準 27 結果・51 通過 + 新規 3 ファイル 11 件)。+ 0270b(重ねた場合)**31 結果・67 通過・失敗 0・無視 1**。0270b だけ(0270 無し)**28 結果・55 通過・失敗 0・無視 1**。可変シフトは host CPU に同じ機械語を走らせた結果と **112,800 run 一致**、bsf/bsr は **864 run 一致**。故意に壊した版(count 0 の扱い、bsr の定数)は検出される。
2. **結論**: 方式は 3 つで、どれも『外部の契約/命令の意味の実装』(lift 忠実性の方針の (b))で、クラス名・窓・番地による分岐、新しいオプションは無い。(a) `flagout.rs`: 関数が `ret` する時のフラグの既知値の要約(cookie の戻りの ZF=1)。(b) 可変シフト: count&mask==0 でフラグを変えない `weak` 定義 + CF/ZF/SF/PF の recipe。(c) `bsf/bsr` の ZF と lowering。出荷の構成では U の外の C 経路の関数の署名が変わるだけで、**3 関数を hyb リストに入れるかは PM の判断**(§9 に 1 行ずつ。規則 0239 + 手順 3b を回すと Elastique の 4 関数だけが U に増える)。0270b は、U の関数を 1 つも変えない(出荷の構成で wasm がバイト単位で同一)。
3. **未確認**: (i) **実エンジンでの動作**(E8)。(ii) 3 関数が速くなる量(C 経路の自己時間 + 補助 = 1,193 + 299 + 277 ms は 0265 の測定。直 wasm にした時に減る量は**未測定**。mt3 の『平均 37.5% 短縮』を当てれば約 660 ms だが推定)。(iii) 出荷構成で署名が変わる U の 3 関数(§7)は 1 本ずつ手で読んだが、実エンジンの確認が要る。(iv) C 経路の関数が**直 wasm の callee** のフラグを読む形は、wrapper がフラグを運ばないので成り立たない(§6)。8 モジュールの全 call 部位の走査では実例は 0 件(callee は 5 つ、どれも U の外・新しく直になる 120 関数の外)。**この型をビルドで止める検査 0270b を作った**(PM の指示。§6)。止めるのは直接の呼び出しと `jmp` と『フラグをそのまま返す call』で、解決できない間接 call の callee は止められない(限界)。

## 1. 3 関数が止まっていた理由(fe-wasmgen の report)
| 関数 | 命令数 | 理由(再掲) | 実態 |
|---|---|---|---|
| f_b1ce5e0 | 1,049 | `flag merge from a definition without recipe: Call Opaque flag 0x8 @ 0xb1cf384` | `0xb1cf367: call 0xb2c8090` の次の `0xb1cf384: je 0xb1cf210` が、`test edi,edi; jne` で call を**飛ばした道**の ZF と call の戻りの ZF を合流して読む |
| f_b25ce20 | 2,275 | `... Call Opaque flag 0x8 @ 0xb25dd4c` | 同じ形 |
| f_b2cf0c0 | 353 | `flag read from a definition without recipe: Shr Opaque flag 0x1 @ 0xb2cf61b` | `shr rax,cl; jae`(CF)。その次(0xb2cf654)に `bsr rcx,r10; jne` の ZF |
census 全体では、Call Opaque の 93 関数は**ほぼ 1 つの慣用句**(callee = Elastique の 0xb2c8090 が 15 部位、dsp の 0x37ca8f0 が 72 部位)、可変 Shr の 13 関数は同じ形のコードの複製(oggio・Blend2D・Elastique・dsp)。

## 2. x86 の `__security_check_cookie` の戻りの ZF と、C 経路がフラグをどう持つか(PM の問い)
- **本体**(Elastique 0xb2c8090、dsp 0x37ca8f0 も同形): `cmp rcx,[cookie]; jne F1; rol rcx,16; test cx,0xffff; jne F2; ret; F2: ror rcx,16; F1: jmp __report_gsfailure(0xb2c8ac0)`。**唯一の `ret` は 2 つの `jne` が成立しなかった後**で、直前の `test cx,0xffff` が 0 だった = **ZF=1、CF=OF=0**。
- 失敗側の `__report_gsfailure` は `IsProcessorFeaturePresent(0x17)` → `int 0x29`、さもなくば `RtlCaptureContext` / `RtlLookupFunctionEntry` / `RtlVirtualUnwind` を呼ぶ関数(0xb2c8cb4)と、`SetUnhandledExceptionFilter` → `UnhandledExceptionFilter` → `GetCurrentProcess` → `jmp [TerminateProcess]` の関数(0xb2c8a8c)に行き、最後は `add rsp,0x38; ret` を持つ(静的な CFG では ret に達する)。
- **契約の性質(正直に)**: ZF=1 での復帰は Windows の公開された契約ではなく、**この DLL に静的にリンクされた CRT の実装の事実**。caller がそれを読むのはコンパイラが合流した結果で、読む部位は Elastique 15 + dsp 72(§6 の走査)。フラグの値は `cmp`/`test` の結果として静的に決まる(実行時の値に依らない)ので、要約は x86 の意味そのもの。
- **失敗側を『ret に達しない道』と数える根拠**: runtime は ExitProcess・TerminateProcess・RaiseException・RtlCaptureContext・RtlLookupFunctionEntry・RtlVirtualUnwind・RtlUnwindEx・RtlPcToFileHeader・UnhandledExceptionFilter・SetUnhandledExceptionFilter(`fle_proc.h`)と ExitThread(`fle_thread.h`)を `__attribute__((noreturn))` と宣言し、`abort()` で止める。`flagout.rs` の `NORETURN_IMPORTS` はこの表で、**単体試験 `noreturn_imports_match_the_runtime_headers` がヘッダと表の一致を検査する**(runtime がどれかを返す実装にしたら試験が落ちる)。Windows の契約ではなく『この runtime ではそこで止まる』という前提で、止まった後のフラグは観測されない。
- **C 経路のフラグ**: R/FL/X は 1 つのスレッドローカルのブロック `fle_ctx` の R +0x000、FL +0x100、X +0x200(`fle_lowregs.h`)。FL は 6 つのバイト(cf pf af zf sf of)で、値 0xff は未定義(読むと止まる)。lifted C は命令ごとに FL に書き、`call` を越えて読む。**直 wasm の関数はフラグのレジスタを持たず**(オペランドを局所に保存して必要な時に再計算)、**`f_<va>` の wrapper はレジスタ(R[])だけを運び、FL に書かない**。よって (1) 直 wasm の caller が callee のフラグを読むには callee の静的な要約が要り(本差分)、(2) C 経路の caller が直 wasm の callee のフラグを読む形は成り立たない(§6)。

## 3. 方式
### (a) `flagout.rs`(新規 306 行): 関数が ret する時のフラグの既知値
- 前向きの dataflow。フラグ 6 つごとに『0・1・不明』。`test/and/or/xor`(CF=OF=0)、`xor r,r`/`sub r,r`(ZF=PF=1、CF=SF=OF=0)、`clc/stc` は定数、他の書き込みは不明、`call` は callee の要約(`Out::Flags`)で上書き(未知なら全部不明)。
- `jcc` の辺で検査するフラグを絞る(`je/jne`: ZF、`jb/jae`: CF、`js/jns`: SF、`jo/jno`: OF、`jp/jnp`: PF、`ja` の成立と `jbe` の不成立: CF=ZF=0)。既知の値と矛盾する辺は無い辺として落とす。`ud2/int3/int n`、ret に達しない callee への call、noreturn の import への call/jmp は終端(ret に達しない)。`tail jmp` は先の関数の要約。全 `ret` で同じ既知値のフラグだけが要約。
- 要約が `Unknown` の関数(未対応の終端、深さ 4 を超える呼び出し、64 ブロックを超える callee、再帰、1 つも既知でない)は従来どおり(caller は C 経路に残る)。
- `flags.rs`: `Kind::Known(mask, values)`(保存するオペランドが無い定義)と `analyze_with`(`analyze` は `|_| None` を渡す)。`lower.rs`: `flag_expr` が定数を返す、`direct_cond` は Known で false、call の後に合流で読まれるなら `materialize`。`lib.rs`: **最初の flags 解析が `Call Opaque` で失敗した関数だけ**、call 先の要約を作って解析し直す(census の Elastique 全体の user 時間 10.10 s → 10.16 s。差は測定誤差)。
- **局所(locshare)との関係**: Known の定義は局所を 1 つも使わない。合流で読まれる時の `Src::Mat` の局所(フラグごとに 1 つ、関数で共有)に定数を書くだけ。可変シフトの定義は (sa, sb) を locshare の slot から取り、count 用の i32 を定義ごとに 1 つ足す(共有しない)。bsf/bsr は sa 1 つ。
### (b) 可変シフト(`flags.rs` + `lower.rs`)
- `shl/shr/sar r8|16|32|64, cl` を `Kind::ShlV/ShrV/SarV`、**`weak`**(REP の比較と同じ: 以前の定義を殺さない)にした。count&mask==0 では全フラグが変わらず、以前の定義の合流(`Src::Mat`)がそのまま読まれる。CF = 最後に出たビット(`a >> (c-1)` または `a >> (w-c)`)、ZF/SF/PF は結果から。**OF は count==1 の時だけ定義(未実装)**: OF の読み手は C 経路に残る(試験あり)。8/16 bit で、マスク後の count が幅を超える時は `unreachable`(C 経路の `fle_op_shl/shr/sar` の `fle_bad_count()` と同じく止まる。ハードウェアでは CF が未定義)。
- `shift()`: count を `cin_local` に退避(宛先が rcx 自身でもよい)、結果の保存の後、`if (count) { materialize }`。
- **副作用(1 つの既存試験を編集)**: 『可変シフト + jae』だけの関数は、count==0 の時にフラグが関数の入口で生きているので、今は `reads a flag that is live into the function on one path`(opaque、caller は C 経路)になる。以前は『recipe 無し』として caller が stub で lower できた(決定 0252 の `opaque_stub.rs`)。0252 の試験は『recipe の無い定義』の例として `shr eax,cl; jae` を使っていたので、同じ仕組みの別の例(`mul ecx; jae`)に変えた(1 試験の編集、判定は同じ)。census では失われた関数は 0。
### (c) `bsf/bsr`(`flags.rs` + `lower.rs`)
- `Kind::BitScan`(ZF = 入力が 0。**他のフラグは未定義**でモデルしない)。lowering は `clz/ctz`、入力が 0 の時は宛先を書かない(C 経路の `bscan` と同じ)。16/32/64 bit。

## 4. 試験(`lift/crates/fe-wasmgen/tests/`)
| ファイル | 件数 | 内容 |
|---|---|---|
| `flagout.rs`(node) | 5 | cookie の形(`test edi,edi; jne M; call G; M: je L` の合流)が edi の低 32 bit で正しい道へ行く / `xor eax,eax; ret` の 5 つのフラグを setcc で読む / `jae` の戻りの CF を `adc` が入力にする(CF=1 と CF=0)/ ud2 だけの関数への tail jmp / 要約の無い callee(`cmp rcx,rdx; ret`、2 つの ret で ZF が違う)の caller は C 経路に残り理由が `without recipe`。**基準の木(0270 無し)では前の 4 件が落ち、最後の 1 件は通る**ことを確認。 |
| `flags_cpu.rs`(node + host) | 3 | 同じ機械語を mmap で host CPU に走らせて比較。可変シフト = 3 演算 × 4 幅 × 4 読み手(値 / setcc / jcc 鎖 / cmov + adc)× 40 の入力フラグ状態(`cmp/test/add/sub` の 4 定義が合流 × 10 の被演算子の組)× 14 の count(0 1 2 7 8 9 15 16 17 31 32 33 63 64)× 5 値 = **112,800 run 一致**。bsf/bsr = 3 幅 × 2 演算 × 3 読み手 × 12 入力 × 2 の旧宛先 × 2(skip)= **864 run 一致**。OF の読み手は C 経路に残る。生成器 `gen_flags_cpu.py`(GNU as)。 |
| `boundary_regs.rs`(node) | 2 | PM の依頼(§5)。 |
| `src/flagout.rs` の単体試験 | 1 | `NORETURN_IMPORTS` と runtime のヘッダの一致。 |
| `flagread.rs`(0270b、node 不要) | 5 | §6.3。 |
故意に壊した版: (1) 可変シフトの `if (count)` を常に真 → host との差分が出る、(2) bsr の `31 - clz` を `30 - clz` → 差分が出る。どちらも 1 回の `cargo test` で検出。

## 5. C ↔ 直 wasm の境界の検査(PM の 08:55 の依頼 1)
`tests/boundary_regs.rs`: ミキサーの 7 関数の形(rbx・rsi・rdi を push、xmm6・xmm7 をスタックに保存して復元、末尾の cookie 形の合流)の合成関数 F を `f_<va>` の wrapper から呼び、**callee-saved(rbx rbp rsi rdi r12〜r15)に 8 つの印、xmm6〜15(X[])に印、R[4]、フラグ 8 バイト(FL)に印**を置いて、戻りの後に全部を比べる。
- (1) F 直 wasm、cookie 関数 G は C 経路(JS の関数。lifted C と同じく FL に ZF=1 などを書く): 全部戻り、rax = F が通った道(G が呼ばれるのは cookie の道の時だけ)。**G が FL に間違った ZF=0 を書く版でも結果は同じ**(caller は FL を読まず callee の静的な要約を使う、の確認)。
- (2) F も G も直 wasm: 全部戻り、**FL は一切書かれない**(8 バイトが元のまま)。
- (3) 直 wasm の G の wrapper を呼ぶと FL は呼ぶ前のまま(**フラグは wrapper を越えない**)。
- 逆向き(C 経路の caller が直 wasm の callee のフラグを読む)は、(3) の通り成り立たない。その形が実在するかの走査が §6。

## 6. 0270b: フラグを読まれる callee を C 経路に残す検査(PM の 09:15 の指示)
### 6.1 問題と規則
- 問題: C 経路の caller(lifted C)は call の後に `FL` を読む。直 wasm の関数の wrapper `f_<va>` は R[] だけを運び `FL` に書かない。したがって『C 経路の caller が、直 wasm の callee が返すフラグを読む』形は成り立たない(合成試験 `boundary_regs.rs`: wrapper の後の FL は呼ぶ前のまま)。0002b と同じ『境界で値が落ちる』型。
- 規則(一般形。関数のアドレス・名前・モジュール名の分岐は無し): **C 経路に残る関数のどこかの direct call の部位で、戻り直後に(フラグを書く前に)状態フラグを読まれる callee は、直 wasm にしない**。さらに (i) その callee が自分の callee の戻りのフラグをそのまま返す形(`call g; ret`)の g、(ii) その callee が `jmp`(`jcc` を含む)で入る別の関数、も同じ扱い(フラグは呼ばれた側から呼んだ側へ渡るので)。
- 『C 経路の caller』= その時点で直 wasm にならない関数(hybrid の選択から外れた物、解析できない物)。**直 wasm にする caller の部位は数えない**: 直 wasm の caller は callee のフラグを 0270 の要約から定数で読む(要約が無ければ lower できず、C 経路の caller になる)。
### 6.2 実装
- `flagread.rs`(新規、119 行): `read_after_call(&Analysis, &c_path) -> BTreeSet<u64>`。判定の根拠 = `tools/flaglive.rs` の走査と同じ(call の直後から fall-through・direct `jmp`・`jcc` の両側を辿り、状態フラグ〔OF SF ZF AF CF PF。DF は除く〕を読む命令が先にあれば『読まれる』、書く・間接 call/jmp なら『読まれない』、ret か別の関数の入口への jmp なら『フラグを渡す』)。
- `lib.rs` に 1 箇所(+16 行: モジュール宣言 1、hook 15): `hybrid_only` を適用した直後、`closed` を決める前。hybrid の時だけ。C 経路の関数の集合が変わらなくなるまで繰り返し(外した callee が新しい C 経路の caller になる)。外した関数の理由は `flags are read after a call of it by a caller on the C path (they do not cross the C wrapper)`。**新オプション・環境変数・旗は無い**。
- 解析できない呼び出し(間接 call で `Resolved::Code` にならない物、import の call)は走査できない: import はランタイムの C 関数でフラグは `FL` に残る(影響なし)、解決できない間接 call は callee が不明で守れない(限界)。
### 6.3 試験(`tests/flagread.rs` 5 件)
(a) C 経路の `call T; je` → T は C 経路に残る。(b) `call T; test eax,eax; je`(先にフラグを書く)→ T は直 wasm のまま。(c) `X: call T; ret` と C 経路の `call X; je` → X も T も残る。(d) `X: jmp T` → X も T も残る。(e) 両方を直 wasm にする(0270 の要約)→ 両方 ok、守りは働かない。(f) 選んだ caller が lower できない形(要約の無いフラグ)→ caller は ok でなく、T は残る。(e) 以外は 0270 無しでも通る(e だけが 0270 に依存し、0270 無しの差分〔alone〕では外してある)。
### 6.4 census(`tables/0270b-census-config.tsv`、`0270b-callee-callers.tsv`)
- **census の構成(全関数が候補)**: 0270 だけの版と守りを足した版の ok を全関数で比べた結果、**守りで C 経路に戻るのは 5 関数だけ**: Elastique 0xb2c8090(cookie)・0xb3aa660・0xb3ae300・0xb3b26e0、dsp 0x37ca8f0(cookie)。**今の U に入っている物は 0、0270 で新しく ok になる 120 関数に入っている物は 0**、他の関数の status の変化も 0。
- **出荷の構成(`--hybrid-only hyb-U-<m>.txt`、U は変えない)**: 8 モジュールすべてで、守りを足した wasm は 0270 だけの wasm とバイト単位で同一(=今の U は 1 関数も影響を受けない)。
- **regen の構成(実行された 15,412 関数を候補、規則 0239 + 手順 3b)**: 守りで C 経路に残るのは 2 関数(Elastique 0xb2c8090・dsp 0x37ca8f0、どちらも cookie)。§9 の 4 関数の追加は守りの有無で同じ。**ただし E7b のコード(0270 無し)で同じ regen を回すと、Elastique 0xb2c8090(cookie、18 run で 41,190 命令、198 B)が『only in regenerated 192』の追加候補に入っており、守りの無いまま取り込むと、C 経路に残る cookie の caller〔`Popcnt` などで lower できない 6 関数〕が戻りの ZF を `FL` から読むのに wrapper が `FL` に書かない型の誤りになる**(今のコミット済みの U には入っていない。`tables/0270-hyb-regen-diff-vs-current.tsv` の `+ elastique 0xb2c8090`)。0270b を入れると regen の結果から外れる。
- **0270 の効果を消さないか**(PM の問い): 消さない。0270 の効果は『cookie を呼ぶ側(f_b1ce5e0 など)が直 wasm になる』ことで、その caller は cookie の ZF を要約の定数で読む。cookie 自身は元々 U に入っていない(C 経路)。守りは cookie を C 経路に**残す**だけで、caller の直 wasm 化には影響しない。5 つの callee は、どれも**C 経路の caller が実在する**(守れない = 直にしてはいけない)ので、C 経路に残るのが正しい:
| callee | 部位 | caller | caller のうち直 wasm 化できる(ok) | C 経路の caller(理由) |
|---|---|---|---|---|
| Elastique 0xb2c8090(cookie) | 15 | 15 | 9 | 6(`mnemonic Popcnt` など) |
| dsp 0x37ca8f0(cookie) | 72 | 72 | 67 | 5(`Movddup`・`Popcnt` など) |
| Elastique 0xb3aa660 / 0xb3ae300 / 0xb3b26e0 | 各 2 | 各 1 | 0 | 各 1(`Call Opaque flag 0x10`: callee が実行時の演算で SF/OF を返す) |
- **cookie 以外の 3 つ(OF/SF)が将来 U に入りうるか**: 18 run で一度も実行されていない(実行された関数の集合に入らない)ので、規則 0239 の入力が変わらない限り U には入らない。入っても、caller 各 1 が C 経路(callee が実行時の演算でフラグを返す。要約が `Unknown`)なので、守りで C 経路に残る。
### 6.5 0270 の機構との切り分け(docs 用の 1 段落)
0270 は**直 wasm の caller** が callee のフラグを読む機構で、callee の `ret` でのフラグの既知値を静的に求め、caller はそれを定数として使う(callee が C 経路でも直 wasm でも同じ。caller は `FL` を読まず、callee も `FL` に書かなくてよい)。0270b は **C 経路の caller** が `FL` を読む場合の境界の検査で、`FL` に書かない wrapper の背後に callee を置かない。両者の対象は別の部位(直 wasm の caller の部位か、C 経路の caller の部位か)で、0270b は直 wasm の caller の部位を数えないので矛盾しない。直 wasm 同士の call にはフラグの運搬は要らない(静的な事実)。C ↔ 直でフラグが落ちるのは、C 経路の caller が直 wasm の callee のフラグを読む時だけで、そこを 0270b が止める。

## 7. 署名が変わる関数(PM の依頼 2)
### 7.1 census と出荷の構成の違い
`tables/0270-U-changed.tsv` は census(全関数を候補にして `called_from_c` が空)の構成で、U ∩ 『既に ok で行が変わる』= 28 関数(署名 8)。**この構成は実際のビルドとは違う**(出荷は `--hybrid-only hyb-U-<m>.txt`)。出荷と同じ構成の差(`tables/0270-U-changed-production-like.tsv`、`tools/prod_cmp.sh`)では **U の中で行が変わるのは 16 関数**。以下はこちら。
### 7.2 16 関数
| 種類 | 関数 | 変化 | 原因(callee) |
|---|---|---|---|
| 大きさだけ(署名同じ) | oggio 0xc085ba0 −96 B、0xc099d70 −648 B、0xc0a7ea0 −172 B、0xc0a8d50 −172 B | 呼び出しの並べ方 | callee 0xc058a30(0xc085ba0)、0xc0a9410・0xc0a9760(0xc0a7ea0・0xc0a8d50)= 『recipe 無し』の C 経路 → 精密な署名の C 経路。0xc099d70 は U の 0xc0a7ea0・0xc0a8d50 を呼ぶ(その大きさが変わる) |
| 大きさだけ | Elastique 0xb2a09a0 −71、0xb2a2320 −143、0xb2c17b0 −440、0xb2cebc0 −214、0xb2f0610 +1、0xb2f07d0 −35、0xb2f3650 −583、0xb2f5760 −36、0xb2e6b30 +1 | 同上 | callee 0xb2cf0c0(= f_b2cf0c0)・0xb2d7da0・0xb2d7ef0 など。0xb2f0610 の +1 B は callee 0xb2cf0c0 の署名の変化、0xb2e6b30 の +1 B は直接の callee の変化が無く、関数番号の LEB128 の幅と読む |
| **署名の変化** | Elastique **0xb1baec0**(2 命令) | sig_in 0x0f07 → 0x2f86 | tail の先 0xb1ce5e0(= f_b1ce5e0)の署名が精密になった |
| **署名の変化** | Elastique **0xb005900**(109 命令) | elided 0x30c8 → 0x10c8(r13 が elide されなくなる) | 上の 0xb1baec0 の入力に r13 が入った |
| **署名の変化** | Elastique **0xb2c51e0**(8 命令) | sig_in 0x3bce → 0xfbee(rbp・r14・r15 が入力に増える)、+210 B | jump table(0xb1bad10 の switch)の先の thunk が精密な署名になった |
### 7.3 署名が変わる 3 本を 1 本ずつ読んだ結果
1. **0xb1baec0**: `mov rax,[rip+slot]; jmp rax`(slot の静的な内容が 0xb1ce5e0 に解決された thunk、決定 0169)。署名は先の関数の入力。f_b1ce5e0 は冒頭で rbx・rsi・rdi・r12・r14・r15・rbp を push(save として入力から除かれる)、`mov r9d→ebx`、`mov r8→r12`、`mov rdx→r14`、`mov rcx→rbp`、`mov [rsp+0x310],r13`(r13 の読み)、rax は `mov rax,[rip+..]` で最初に書かれる。= 入力 rcx・rdx・rdi・r8・r9・r13 は上のとおり読まれ、**rax が外れたのは正しい**(以前は C 経路の stub の保守的な ABI の値)。r11 は `mov r11b,1`(下位 1 バイトだけを書く)ので入力、r10 が入力である道は読み切れていない(署名の判定を信じている)。wrapper は同じ solve から作る署名で R[] を読む。
2. **0xb005900**: 直接の caller は無い(wrapper 経由)。elision は『その callee-saved を callee が使わないので push/pop を省ける』の判定で、callee(0xb1baec0)の入力に r13 が入ったので r13 は省けなくなった = **より保守的な側**の変化。
3. **0xb2c51e0**: `mov rax,r8; mov r8,rdx; mov rdx,rcx; mov rcx,rax; jmp 0xb1bad10`(引数を回して switch の thunk へ)。入力は switch の先の署名の和集合で、先の thunk の 1 つ(0xb1bb180)が精密な署名(sig_in 0xffa6、sig_out 0xdf27)になり、rbp・r14・r15 の push が入力として数えられるようになった = **入力の増加(上位集合)なので安全側**。+210 B はその引数の受け渡し。
- 『呼ぶ側が正しく追従するか』: 直 wasm の caller は同じ solve(1 つの `compile`)の署名を使い、C 経路の caller は wrapper `f_<va>` が同じ署名で R[] を読む。署名を食い違わせる経路は無い(境界の試験 §5)。
### 7.4 注意
上の 16 本は『署名が変わる』ことの機械的な説明と手読みで、意味の検証ではない。署名の変化は 0002b で痛い目を見た型なので、**実エンジンの E8 で確認するのが前提**。

## 8. 失われる物・副作用
- 失われた関数は 0(census 8 モジュール)。既に ok だった 654 関数で report の行が変わる(census 構成。oggio 162・Blend2D 41・Elastique 69・dsp 382。±1〜4 B の関数番号の LEB128 の幅と、callee の署名が精密になることによる)。このうち 240 本は、新しく ok になった関数との直接の呼び出し関係(caller・callee の推移閉包)が無く(`tables/0270-changed-rows.txt`)、1 本ずつは見ていない(`called_from_c` の変化や間接の呼び出しが原因と読む)。**出荷構成では 16 本に減り、全部が callee の署名の変化で説明できた**(上)。
- 生成する C は変わらない(fe-wasmgen だけの差分)。
- 可変シフトと bsf/bsr は、今 ok で U に入っている関数には現れない(無かったから ok でなかった)ので、U の関数の中身は変わらない。

## 9. hyb リストへ入れる案(PM の依頼 3。規則 0239 + 手順 3b〔B=150 ms〕を、`tools/` の regen を E7b のコード〔REF2〕と本差分〔NEW2〕で**同じ引数**で回して比べた)
入力: 実行された 15,412 関数(E-all.tsv.gz)、`--atomic mt4-atomic-functions*.tsv`、`--startup-ms startup-big-ms.tsv --startup-budget-ms 150`、`--current lift/harness/seeds`。REF2 = lowerable 14,987・切り口 4,143・startup-big 98(手順 3b で 4 を再許可)→ 4,045。**NEW2(0270)= lowerable 14,994(+7)・切り口 4,146(+3)・startup-big 99(+1)→ 4,047**。REF2 に対する追加は **Elastique の 4 関数だけ**(`tables/0270-hyb-U-additions-elastique.txt`)。fle で 2 本(0x4781f0・0xe936b0)が切り口の移動で外れるが、既存の約束(『追加のみ』、規則の注記)どおり反映しない。守りの有無(0270b)で追加は変わらない。
| 関数 | 本体〔raw〕/ -Oz の概算 | 18 run の命令(start+par-boot / ref1-perf / ref1-ui / 合計) | 判定 | S1 の増分 | first idle(悲観 0.53 ms/KB) | C 経路の費用(0265〔ミキサースレッド、45 s〕) |
|---|---|---|---|---|---|---|
| Elastique 0xb1ce5e0 | 68,488 B / 61,982 B | 0 / 444.5 M / 158.4 M / 602.9 M | startup-big でない(起動で動かない)。切り口内 → **U に入る** | +68,488 B | 35 ms | 1,193 ms(自己 438 + SSE 541 + ld/st 213) |
| Elastique 0xb25ce20 | 210,782 B / 190,758 B | 0 / 36.9 M / 8.8 M / 45.8 M | 同上 → **入る** | +210,782 B | 109 ms | 299 ms |
| Elastique 0xb2cf0c0 | 30,509 B / 27,611 B | 0 / 61.8 M / 2.3 M / 64.1 M | 同上 → **入る** | +30,509 B | 16 ms | 277 ms |
| Elastique 0xb337800 | 423 B | 0 / 12.8 M / 2.0 M / 14.8 M | 同上 → **入る** | +423 B | 0 ms | 0265 の上位 20 に無い(< 36 ms) |
| dsp 0x329fb00 | 183,830 B(census)/ 166,364 B | **708,952 + 285,122** / 1.50 M / 1.77 M / 20.4 M | **startup-big**(≥ 20,000 B で start・par-boot で動く)→ 手順 3 で落ちる。手順 3b は『C 経路 ms』の入力(startup-big-ms.tsv)に無いので再許可されない → **入らない** | (0) | (95 ms) | 不明(startup-big-ms.tsv に無い。上位 20 に無い) |
| Blend2D 0x47a9eb0 | 2,089 B | 83 / 83 / 83 / 1,494 | 実行が 18 run 合計 1,494 命令で、99.99% の切り口の外 → **入らない** | (0) | — | — |
| oggio 0xc058a30 | 428 B | 0 / 406 / 406 / 812 | 同上(合計 812 命令)→ **入らない** | (0) | — | — |
- 追加の 4 関数の S1 の増分は合計 **310,202 B**(raw)。first idle の悲観の概算(0.53 ms/KB)は合計 **160 ms**(node の 1 関数ずつの実測の 1/9〜1/20 を当てれば約 8〜18 ms)。**手順 3b の予算 B=150 ms は startup-big の再許可だけにかかる**ので、この 4 関数は予算の外だが、E7b の 149 ms(悲観)に足すと悲観の合計は約 309 ms になる。予算で縛りたいなら、追加を 0xb1ce5e0 + 0xb2cf0c0 + 0xb337800(51 ms)に絞る案もある(0xb25ce20 だけで 109 ms)。
- 期待する利得: 3 関数の C 経路の費用の合計 1,769 ms(45 s のプロファイルのミキサースレッド 15.1 s の 11.7%)。mt3 の『平均 37.5% 短縮』を当てれば約 660 ms だが**推定で未測定**。
- hyb リストのファイルは変えていない(`lift/harness/seeds/hyb-U-elastique.txt` に追加する 4 行 = `tables/0270-hyb-U-additions-elastique.txt`)。取り込むなら regen を回して追加分だけを足す(E8 の候補)。

## 10. 差分と取り込み
- `0270-flags.diff`(1,820 行、10 ファイル: 新規 `src/flagout.rs`、`tests/{flagout,flags_cpu,boundary_regs}.rs`、`tests/flags_cpu_code/mod.rs`、`tests/gen_flags_cpu.py`; 既存 4 ファイルの hook `src/{lib,flags,lower}.rs`・`tests/opaque_stub.rs`、`-` は 25 行)。
- `0270b-on-top-of-0270.diff`(327 行、3 ファイル: 新規 `src/flagread.rs`・`tests/flagread.rs`、`src/lib.rs` に +16 行。`-` は 0 行)と `0270b-alone.diff`(318 行、同じ 3 ファイル、0270 無しの木に当たる。0270 に依存する試験 (e) だけ外してある)。
- いずれも現在の木(コピー)に `patch -p1 --dry-run` が offset/fuzz なしで通る: 0270 だけ、0270b だけ、0270 → 0270b の順(0270 を実際に当てた木に 0270b)。cargo test の結果は冒頭の 3 点。
- E7b の次のビルド(E8)の候補。**E7b には載せない**(PM の判断済み)。hyb リストは変えていない。

## 11. 未確認・限界・自己申告
- 実エンジン 0 件。3 関数が速くなる量は未測定。
- 要約の前提は『runtime の noreturn の import は返らない』。表とヘッダの一致は試験で守るが、止まる import が将来返す実装になれば前提が崩れる(試験が先に落ちる)。
- OF を読む可変シフトの後、bsf/bsr の ZF 以外のフラグ、callee が実行時の演算でフラグを返す形(§6 の 3 つ)は直にならない。
- 『Call Opaque』の文字列で再解析を起動する判定は、エラー文の書き換えに弱い(文言を変えたら census が `Call Opaque` を数えなくなる)。別の持ち方(エラーの種類を型にする)は別差分。
- 0270b は解決できない間接 call の callee のフラグの読みを見つけられない(callee が不明)。import の call はランタイムの C 関数なので `FL` に残る(影響なし)。
- 0270b が止めるのは『C 経路の caller が読む』形で、0270 の機構が使う『直 wasm の caller が要約から読む』形は止めない(§6.5)。
- **自己申告**: (a) 0261/0264 の『rsp-delta は健全』の誤り(0264 の続き §21〜26)。(b) 再起動前に `timeout` と、再起動後に `sleep` を各 1 回呼んだ(いずれも拒否された)。(c) `bash` という名前の存在しないツールの呼び出しが累計 4 回。(d) 画面に大きな表(654 行)を 2 回出した(出力の整理の不手際。測定には無関係)。

## PM の判断と取り込み(2026-11-06 09:35、docs のみ。差分は製品の木に当てていない)

取り込み: mt3 の 0270 最終(09:22〜09:50)の文書・表・道具を `docs/investigations/mt3w/` に、差分 3 本(`0270-flags.diff`、`0270b-on-top-of-0270.diff`、`0270b-alone.diff`)を `docs/investigations/mt3w/patches/` に置いた。差分は e7b-wip(E7b の積み)に対し `patch --batch -p1 --dry-run` が offset・fuzz なしで通る(0270 単独、0270b 単独、0270 → 0270b。PM が git archive の写しで確認)。測定は mt3 の census・cargo・node 単体の合成試験・host CPU との差分試験で、**実エンジン(ブラウザ/node の FL)の走行は 0 件、自己時間の変化の測定は 0 件、依頼者の機械は 0 件**。PM は cargo test・test.sh full・a4 をこの差分では**まだ走らせていない**。

判断:
1. **0270 と 0270b は E7b の push に入れない。次の冷間ビルド(E8)の候補にする。** E7b は実エンジンの確認(opt1 の x13: node 84/55/87 行が E6 と差 0、Export sha256 `381cdb5a…`)が済んだ形で出す。0270 は直 wasm の呼び出し規約・フラグの扱いの意味に触れる変更で、同じビルドに重ねると、x8 のときのように停止の切り分けができなくなる。
2. **0270b(フラグを返す callee を C 経路から呼ばれる形で直 wasm にしない守り)は、0270 の有無によらず、hyb リストを再生成する前に取る。** mt3 の実例: E7b のコードで規則 0239 + 手順 3b を回すと、Elastique 0xb2c8090(`__security_check_cookie`)が追加候補に入り、C 経路の呼び出し側が返りの ZF を読むのに wrapper は FL を書かないので、黙って誤る。コミット済みの U には無いので今の出荷物は影響を受けないが、再生成のたびに踏みうる穴。
3. **hyb への追加(Elastique 4 関数: 0xb1ce5e0・0xb25ce20・0xb2cf0c0・0xb337800、S1 +310,202 B、first idle の悲観見積り +160 ms)は E7b の first idle の実測(低 load、I/O 圧力 <1)を見てから E8 で決める。** E7b 自体の hyb 追加 18 関数の first idle への効きが先に測れる。
4. E8 の完了条件: (a) C ↔ 直 wasm の境界の全レジスタの試験(callee-saved・xmm6〜15・R[4]・フラグ。mt3 の `boundary_regs.rs` が 7 つの実行される関数の形で通っている)、(b) 出荷構成で行が変わる 16 関数(oggio 4、Elastique 12。署名の変更が 3 関数)の中身を実エンジンで確かめる、(c) opt1 の node 84/55/87 行(E7b の行との差を種類と原因で)・Export の sha256・30 s 再生・f_b1ce5e0/f_b25ce20/f_b2cf0c0 の自己時間(0265 の値 1,193/299/277 ms に対する差)。『約 660 ms』は 0265 の 1,769 ms に 37.5% を当てた見積りで、実測ではない。
5. 未確認(mt3 の §11): 実エンジンの走行全て、実際の速度の改善、変わった 3 つの署名・16 の行の意味、census で変わる 654 関数のうち 240 を 1 つずつ読んでいないこと、0270b が解決できない間接 callee を見られないこと、OF を読む可変シフト・bsf/bsr の ZF 以外の扱い。
