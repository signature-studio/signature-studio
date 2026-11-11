# 0261(mt3y、仮): 『RSP が入口の値でない関数』の実態と、RSP の差分(k)を署名に持たせる扱い(2026-11-06 訂正: 『健全』は C ↔ 直 wasm の境界の callee-saved を除く。0264 §21)

基準 HEAD: 62ca650b(`mt3-rsp-delta.diff` は 62ca650b の `git archive` に `patch -p1 --dry-run` が通る。537 行: `pres.rs`・`lib.rs`・`lower.rs` とテスト `tests/rsp_delta.rs`。fe-decode の差分 `mt3-jt-copy.diff`〔141 行、8 節〕は別のファイルで、互いに独立)。この機械(負荷 5〜20)、FL Studio の実物の DLL(CI の extract 済みの 8 つ: FLEngine・dsp_ippv2・Blend2D・Elastique・qfc・freetype・ILTools・oggio)を `fe-wasmgen --census` に通した結果と cargo のテスト。**実エンジンをブラウザ/node で走らせた測定は 0 件**(opt1 の E4 以降の担当)。

## 冒頭の 3 点
1. **測ったこと(事実)**: 『RSP が入口の値でない』で C 経路に残る関数は、FLEngine 4、**Blend2D 21、dsp_ippv2 2,304、Elastique 333**(qfc 2、freetype 65、ILTools 57、oggio 137)。出口(`ret`・tail jmp)の RSP のずれは、**定数 k(正)の共有エピローグ**(`add rsp,N; pop…; ret` を別の関数が `jmp` で入る。Blend2D 6、dsp 1,873、Elastique 161)、**フレームを持ったままの間接 jmp(ジャンプ表・計算 goto)**(Blend2D 5〔8,551 命令〕、dsp 約 210、Elastique 33)、**変数**(Blend2D 4、dsp 約 94、Elastique 8)。変数の正体は `mov eax,N; call __chkstk; sub rsp,rax`(MSVC の大きなフレームのプローブ)。dsp の f_38e6d90(0.43 s)は **RSP ではなく Cvtps2dq** が理由(指示の動機の一部は別の理由)。直接の `call` で呼ばれる『k ≠ 0 の関数』は 0 件(3 モジュールの全 call 部位を数えた): k ≠ 0 の関数は `jmp` でだけ入られる。
2. **結論(作った物と効果)**: 署名 `Sig` に『戻り時の RSP − 入口の RSP = k』を持たせ(callee の k を caller の追跡と `pending_rsp` に足す、tail jmp は k を足して親の出口と比べる、C 経路から呼ぶ wrapper は R[4] を入口 + k にする)、`mov eax,imm` の定数追跡で `sub rsp, rax` を定数にした。census で **Blend2D +25 関数(+2,708 命令)、dsp +960(+84,943 命令)、Elastique +218(+10,610 命令)**が ok になり、失われた関数は 0。指示にあった関数: **Elastique の f_b034096(421 ms)・f_b04a921(388 ms)は ok(直 wasm 6,463 B・7,325 B)**、Blend2D の **f_4758e40(Mixer ドラッグ 201 ms)も ok(7,848 B)**。**Blend2D の最大の f_4776f60(Mixer ドラッグ 546 ms、320/104 ms)は直らない**: 出口が `jmp rcx`(MSVC のジャンプ表 `lea rdi,[imagebase]; mov ecx,[rdi+rax*4+tab]; add rcx,rdi; jmp rcx`)で、フレームを持ったまま fe-decode が解決しない表へ飛ぶ。fe-decode のジャンプ表を解決すると RSP の理由は消えるが、今度は大きさの上限で C 経路に残る(8 節)。
3. **未確認**(fe-decode のジャンプ表の追跡は 8 節): 実エンジン(84/55/87 行・a4・参照プロジェクト・CPU)での走行は 0 件。k ≠ 0 の関数への**間接 call**(`fle_hyb_dispatch` 経由)は呼び出し側が RSP を戻さないので不健全になりうるが、実物にその形は見つかっていない(上の 0 件)。hybrid の一覧 U に新しく ok になった関数を入れるかは PM の未決(U の関数は変更前の ok だけ)。効果の見積り(Elastique の 2 関数 809 ms × 平均短縮 37.5% ≈ 300 ms、ミキサーの約 2%。Blend2D の Mixer ドラッグは 201 ms × 37.5% ≈ 75 ms)は平均値を当てただけの未測定の値。
   **訂正(2026-11-06、mt3)**: 本差分(0002)のテストは『C 経路から呼ばれた k ≠ 0 の関数が戻った後の R[4]・rax』を見たが、**同じ呼び出しで callee-saved レジスタが pop した値になるか**を見ていなかった。`f_<va>` の wrapper は callee-saved を『入口の値で戻す』ので、k ≠ 0 の継続入口が pop した rbx・rsi… を消す。E7b の実エンジン確認(opt1 x8)の起動停止の原因と読み、`0002b` で直した(0264 §21 以降)。この節と 2・3 節の『0002 は健全』と読める記述は、R[4] と呼び出し規約の k についてだけ正しい。

## 1. 実態の調べ方と分類(`tables/`)
- 判定の場所: `pres.rs` の `analyze` が関数の入口を RSP = 0 として RSP の相対値(`St::rsp: Option<i64>`)を追い、`ret` と tail jmp で 0 でないと `unbalanced`。`lib.rs` の `solve` が `unbalanced` の関数を `opaque` にし、`propagate_hybrid` がその呼び出し側を『calls a function that is opaque』にする。
- 診断(ワークツリーにだけ入れた出力。差分には入れない): 各出口の VA と RSP の相対値を出し、`objdump -d` の出口の命令(`ret` / 直接 `jmp` / `jmp reg`)と突き合わせた。表 `rsp-classification.txt`、関数ごとの `rsp-flagged-{b2d,dsp,elastique}.tsv`(dsp は命令数の多い 120)。
- 例(Blend2D): 0x4755fc0(出口 −168)の `push …; sub rsp,…; jmp 0x47563b8`、その先 0x47563b8 は `movaps … ; add rsp,0xa0; pop rbx; ret`(+168)= 親は『フレームを持ったまま共有エピローグへ tail jmp』、子は『他人のフレームを畳む関数』。この組が 7 組(合計の出口の和が 0)。
- 出口の分類(HEAD の flagged の数): 
  | | Blend2D | dsp | Elastique |
  |---|---|---|---|
  | 共有エピローグ(`ret`、k > 0) | 6 | 1,873 | 161 |
  | 親(直接 jmp、k < 0) | 4+1 | 118+6 | 129+1 |
  | `jmp reg`/`jmp [mem]`(フレームつき) | 5(8,551 命令) | 118+78+14 | 32+1 |
  | 変数(`sub rsp, rax` ほか) | 4(609 命令) | 87+3+2+2 | 8 |
  (複数の出口の種類を持つ関数は主な 1 つに入れた。正確な行は `rsp-classification.txt`。)

## 2. 方式(`mt3-rsp-delta.diff`。行数〔追加/削除〕: `pres.rs` +48/−5、`lib.rs` +23/−6、`lower.rs` +6/−6、テスト +266)
1. **`PresResult::delta`**: 出口(`ret` と tail jmp)の RSP の相対値がすべて同じ定数なら k、そうでなければ None。`ret imm16` は imm16 を足す。tail jmp(`Term::Tail(g)`)は『その時点の相対値 + g の k』。`TailImport`/`TailIndirect` は 0 を要求する(従来どおり)。
2. **関数間の k の不動点**(`lib.rs` の `solve`、pres の不動点と同じループ): 全関数 Some(0) から始め、毎回 `analyze` の `delta` で更新、変わらなくなるまで。k の未確定のうちは親が一時的に None になるので、**None も毎回再計算する**(固定すると P が一生 None になる: 最初の版の誤り)。最後に、`delta` が None か、不動点の値と違う関数だけを従来どおり opaque にする。
3. **call の追跡**: `pres.rs` の `step` の `Call` で、直接 call の callee の k を呼び出し側の RSP の相対値に足す(未知の k = None は 0 とみなす: 従来の仮定)。`lower.rs` の `call_to_inner`(直 wasm の callee も C 経路の callee も)で、戻った後 `pending_rsp += sig.rsp`。
4. **wrapper**(`f_<va>`、C 経路から呼ぶ): lifted C の `call` は `R[4] -= 8; …; f(); R[4] += 8` で、`ret` は pop しない(`ret imm16` は `R[4] += imm`)ので、関数は R[4] を入口の値 + k にして戻る。wrapper は従来『入口の RSP』を書き戻していた(k = 0 のとき同じ)。k ≠ 0 なら `+ k`。
5. **定数追跡**(`V::Const`): `mov r32/r64, imm` の結果を定数として持ち、`sub/add rsp, reg` の reg が定数なら RSP を定数だけ動かす(`mov eax,N; call __chkstk; sub rsp,rax`。`__chkstk` は rax を変えないので、callee の pres を通っても Const が残る)。
- 方式の限界: **フレームを持ったままの間接 jmp は解決しない**(ジャンプ表・計算 goto: 行き先の k が分からない)。`ret imm16`(stdcall)の関数が間接 call されると、呼び出し側が RSP を動かさない(上の 3)ので不健全だが、DLL に `ret imm16`(≠ 0)は Blend2D 2・Elastique 29・dsp 44 箇所あり(dsp の大半は命令でないデータの読み違えに見える)、直接 call の callee になっているかは調べていない。
- 以前の `--unbal-stub`(0181)や `--opaque-stub` の不健全さ(Sig を solve の後に入れる、呼び出し側のレジスタを 0 で書く)は**持ち込んでいない**: k は solve の中で他の署名と同時に決まり、呼び出し側は callee の署名で RSP を足すだけで、レジスタを書かない(**これは直 wasm どうしの呼び出し〔`call_to`〕についてだけ正しい。C → 直 wasm の入口の wrapper が callee-saved を『入口の値で戻す』処理は 0002 では見直していなかった。訂正: 0264 §21、`0002b`**)。

## 3. テスト(`tests/rsp_delta.rs`、6 件。`cargo test -p fe-wasmgen`: HEAD 62ca650b + 本差分で 21 結果ok・30 通過・失敗 0・無視 1〔最終版を 22:17 に再実行〕)
1. 共有エピローグ: P(`push rbx; sub rsp,0x20; mov rbx,rdx; mov [rsp+8],rcx; lea rax,[rdx+1]; jmp E`)と E(`mov rcx,[rsp+8]; add rax,rcx; add rsp,0x20; pop rbx; ret`)がどちらも直 wasm になる。node で P: rax = rdx+1+rcx、rbx が slot から戻る、R[4] は入口の値、P が書いた slot の値。E 単独: rax と R[4] = 入口 + 0x28。
2. `ret 16`(stdcall)の callee F を呼ぶ G(2 つの引数を push して call、戻って `mov qword [rsp+8],0x77`)が直 wasm になり、rax = 引数の和、R[4] は入口の値、**呼び出し後の [rsp+8] の書き込み先が入口 − 0x20**(callee の k = 16 を足さないと別の番地になる)。
3. 変数の RSP(`sub rsp,rax; add rsp,rax`)と、出口が食い違う関数は C 経路に残る。
4. `mov eax,0x1020; call CS; sub rsp,rax; …; add rsp,0x1020; ret`(CS は rax を変えない `ret`)が定数のフレームとして直 wasm になり、0x1020 バイトのフレームの slot に書いて読める。
5. `mov rax,rcx; sub rsp,rax; add rsp,rax` は定数でないので C 経路に残る。
6. `test rcx,rcx; jz L; push rbx; sub rsp,0x20; …; jmp E; L: xor eax,eax; ret`: 出口が『入口の値で `ret`』と『フレームを積んで共有エピローグへ `jmp`』の 2 種で、どちらも合計 0 なので釣り合う(P2 と E が直 wasm)。
- 変異(すべて検出): callers が k を足さない、wrapper が k を足さない、tail jmp が callee の k を足さない、call の追跡が k を足さない、`ret imm16` を数えない、定数追跡を外す。(callers が k を足さない変異は、最初の版のテストでは**検出されなかった**。呼び出し後の slot を書く命令を足して検出するようにした。)
- **このテストの穴(2026-11-06 訂正)**: テスト 1 は rbx が slot から戻ることを直 wasm どうし(P → E)でだけ見て、**C 経路の親が `jmp E` で入る場合の E の後の rbx・rsi・rdi・r12〜r15 は検査しなかった**(コメントに『rbx はここでは検査しない』と書いて飛ばした)。`tests/rsp_boundary.rs`〔0264 §21〕がこの境界を見る 4 件で、修正前の 0002 では落ちる(E の後の rbx = 入口の値 0x1111、期待は pop した 0x2222)。
- 他の差分との重ね(`tables/stack-and-conflicts.md`): 0254(wasmgen-small)・0259(x87)・opt1 の WIP の上に本差分を載せた木(E5)で `cargo test -p fe-wasmgen --no-fail-fast` は 25 結果ok・44 通過・失敗 0・無視 1。重ねた版の差分は `mt3-rsp-delta-on-e4.diff`(E4 = 0254 + 0259 + opt1 の WIP の上に dry-run が通る)。

## 4. census(`tables/census-before-after.txt`)
| モジュール | ok(前→後) | 増えた関数 | 増えた命令 | wasm の増分(census 全関数) | flagged(前→後) |
|---|---|---|---|---|---|
| Blend2D | 597 → 622 | +25 | +2,708 | +140 KB | 21 → 8 |
| dsp_ippv2 | 14,640 → 15,600 | +960 | +84,943 | +2.47 MB | 2,304 → 1,296 |
| Elastique | 3,550 → 3,768 | +218 | +10,610 | +380 KB | 333 → 147 |
| FLEngine | 58,555 → 58,555 | 0 | 0 | 0 | 4 → 4 |
| QuickFontCache | 6,596 → 6,596 | 0 | 0 | 0 | 2 → 2 |
| FreeType | 1,647 → 1,650 | +3 | +62 | +1.2 KB | 65 → 62 |
| ILTools | 12,448 → 12,476 | +28 | +137 | +3.3 KB | 57 → 46 |
| oggio | 2,173 → 2,224 | +57 | +5,556 | +198 KB | 137 → 84(**失われた関数 6**) |
- 失われた関数は dsp・Blend2D・Elastique・FLEngine・qfc・freetype・ILTools で 0、**oggio で 6**(下)。『増えた関数』は flagged だけでなく、その呼び出し側(『calls a function that is opaque』だった物)も含む。hybrid の一覧 U に入るのは PM の選択で、U の既存の関数は変わらない(U は変更前の ok だけ)。
- **oggio の失われた 6 関数**(0xc01a69a・0xc01a6b4・0xc01a707・0xc01a721・0xc01a738・0xc01a790。いずれも `hyb-U-oggio.txt` にない): HEAD では ok、本差分では C 経路。出口が 2 種で食い違う(`jmp rcx` の `TailIndirect` で RSP の相対値 0、もう 1 つは『相対値 0 のまま共有エピローグ(`add rsp,N; …; ret`、k = 264)へ tail jmp』で 264)。HEAD は tail jmp の callee の k を足さずに両方を 0 とみなしていたので、wrapper が R[4] を入口の値に戻すのは後者の道で 264 ずれていた(**HEAD が不健全だった**と私は読む。後者の道が実際に走るかは未確認)。新しい版は食い違いを検出して C 経路に残す。新しい版のバグではなく、HEAD の ok の取り消し。8 つの census の中で唯一の『失われた関数』。
- 残る flagged: Blend2D 8(f_4776f60 を含む、ジャンプ表)、dsp 1,296(主にフレームつきの `jmp reg`/計算 goto と、それを持つ親)、Elastique 147。

## 5. 指示にあった関数(`tables/gpu-top40-join.tsv`)
| 関数 | モジュール | 自己時間(gpu2 の表) | HEAD | 本差分 |
|---|---|---|---|---|
| f_b034096 | Elastique | 421 ms(ミキサー側) | calls a function that is opaque | **ok(6,463 B)** |
| f_b04a921 | Elastique | 388 ms | calls a function that is opaque | **ok(7,325 B)** |
| f_4758e40 | Blend2D | 23 / 201 / 27 ms(ミキサー側 / Mixer ドラッグ / Settings) | RSP(`__chkstk` の `sub rsp, rax`) | **ok(7,848 B)** |
| f_4776f60 | Blend2D | 320 / 546 / 104 ms | RSP(`jmp rcx` のジャンプ表) | **直らない** |
| f_38e6d90 | dsp | 429 ms | **Cvtps2dq**(RSP ではない) | 変わらない(opt1 の E2 の担当) |
- Blend2D の『RSP 17 関数』の Mixer ドラッグ 754 ms のうち、f_4776f60 と f_4758e40 で約 747 ms。本差分で直るのは 201 ms 分で、**Mixer ドラッグ(P2)の主要部分の f_4776f60(546 ms)は本差分では直らない。ジャンプ表を解決しても(8 節)大きさの上限で C 経路に残る**。
- 見積り(未測定、平均の直 wasm の短縮 37.5% を当てた上限): Elastique の 2 関数 809 ms → 約 300 ms(ミキサーの約 2%)。Blend2D の f_4758e40 は Mixer ドラッグの約 75 ms。

## 6. 衝突(後から取り込む側が rebase する所。`tables/stack-and-conflicts.md`)
- `Sig` に欄 `rsp` を足した: **opt1 の WIP の `nocfg_sig()` の構造体リテラル `Sig { inr, ins, out }` が `rsp: 0` を要る**(1 行)。
- `pres::analyze` の引数に `rsp_of` を足した: 呼び出しは `lib.rs` の 4 か所(`solve`)。opt1 の WIP は `solve` の引数に `nocfg` を足していて、同じ周辺の行が衝突する(『pres の不動点の前後』『`sigs` の作成』の 2 か所)。
- `lower.rs` は `Sig` の定義に欄、`call_to_inner` の 2 か所に 1 行ずつ、`Term::Ret` の『ret with a pop count』の検査の削除。
- 0254(wasmgen-small)と 0259(x87)とは行が離れていて、`patch` は offset だけで通る。

## 7. 未確認・限界(正直に)
- 実エンジンでの走行 0 件(E4 以降)。census と cargo のテスト(host の x86 との比較ではなく、手で組んだ機械語を node で走らせて、スタックの番地・値・R[4] を確かめた)だけ。lifted C の呼び出し規約(`call` が `R[4] += 8`、`ret` は pop しない)との一致は、`fe-lift/src/lib.rs` を読んで確認した。
- 診断出力(RSPDBG)はワークツリーの fe-wasmgen にだけ入れ、差分には入れていない。
- k ≠ 0 の関数への間接 call は未対応(上の 2 の限界)。dsp の大きな『フレームつき間接 jmp』の群(約 210 関数)は直らない(8 節)。
- 以前の作業(0259 の x87 と同じく)で、`pgrep -f` を自分の `bash -c` に当てて 15 分待った件、sleep と末尾 `&` を試みて拒否された件、PM の木の `lift/` を作業ディレクトリと取り違えて `cargo test` を 1 回走らせた件(次の自己申告)がある。
- 0261 の間の自己申告(追加): 存在しない `bash` というツールを 1 回呼んだ(`echo no` だけ。ハーネスが shell に振り替えた)。PM の木で `cargo test` を走らせた件は、`lift/Cargo.lock` の mtime が私の実行の終わりに近く、私が書き換えていないとは言い切れない(PM に報告済み)。最初の版の『None を固定する』誤りは census で 33 関数(Blend2D)の消失として見つかった。変異 M1 は最初検出されなかった。共有の古い `ct5` の target で x87 のテストが偶然に compile error になった。

## 8. ジャンプ表の追跡(PM の指示、fe-decode)。差分 `mt3-jt-copy.diff`(141 行: `jumptab.rs` +16/−3、テスト `tests/msvc_switch.rs` +29/−2、新規の `examples/jt_census.rs` +55。62ca650b の `git archive` に `patch -p1 --dry-run` が通る〔offset 0〕。E4 の木にも通る)
**冒頭の 3 点**
1. **測ったこと(事実。実エンジンの走行は 0 件)**: Blend2D の f_4776f60 の `jmp rcx`(0x477fe11)は、CI の fe-lift(out-ci1/c6 の実行ファイル)でも fe-decode の閉包でも解決されていない。理由は `msvc_rejected` の『RAX is written at 0x477fdfe (`Mov`) … not a sign extension』= `cmp edi,5; ja; mov eax,edi; lea rdi,[ImageBase]; mov ecx,[rdi+rax*4+T]` の **32 ビットの `mov eax,edi`**(比べた edi の写し)を、`sext_source` が『検査した値を保つ段』と認めない(`cdqe` と `movsxd` だけ。決定 0109 の『狭く保つ』)。同じ関数のもう 1 つの表(0x47802d9)は添字が比べた物そのものなので HEAD で解決済み。
2. **結論**: 32 ビットの `mov eD,eS` を段として足せば解ける(`jumptab.rs` +16/−3)。8 モジュールの全 `jmp` で、**新たに解決する表は 2 つだけ**(Blend2D 0x477fe11〔6 個〕と Elastique 0xb2bc5d5〔f_b2bc5b0、9 個〕)。他の 6 モジュール(FLEngine・dsp・qfc・freetype・ILTools・oggio)は表の数・復号した命令数・関数数が前後で一致。**ただし Mixer ドラッグの f_4776f60(546 ms)は直らない**: 表を解決すると RSP の理由は消えるが(命令 8,235 → 8,446)、直 wasm が『over the size limit (hybrid): 1,162,969 wasm bytes, 892 locals』(決定 0185 の上限)で C 経路に残る。得られるのは Elastique の f_b2bc5b0(14 → 84 命令、直 wasm 1,386 B、小さい関数)だけ。残りの『フレームつきの間接 jmp』の大半(dsp の 183 関数、Elastique の 31)は表の形が違い(下)、この 1 行では解けない。
3. **未確認**: 実エンジンでの走行(0 件)。解決した表の行き先が実行時に踏まれるか。C 経路の生成物(Blend2D で +365 命令、Elastique で +70)の CI での再生成と、その後の走行。f_4776f60 の大きさの上限を下げる/ケースを分ける案(別の作業、未着手)。

**(a) 表の形**: 0x477fe11 の表は RVA 0x8072c(VA 0x478072c)、ImageBase からの 4 バイトの符号つきオフセットが 6 個(`cmp edi,5; ja 0x47801cc` から N = 5 が見える)。.text の中にあり、直後に同じ関数のもう 1 つの表(0x80744、6 個)が続き、その後は 0xcc の詰め物(clang 流)。行き先は 0x47801d3・0x477fe13・0x477fe74・0x477ffda・0x478016b・0x4780189(すべて f_4776f60 の中)。Elastique の 0xb2bc5d5: `cmp edx,8; ja 0xb2bc68f; mov eax,edx; lea rcx,[ImageBase]; mov edx,[rcx+rax*4+0x2bc698]; add rdx,rcx; jmp rdx`、9 個。どちらも HEAD が解決済みの形と同じで、違いは添字レジスタを比べた後に 32 ビットで写している点だけ。
**(b) fe-lift の C 経路**: fe-lift と fe-decode は同じ `analyze_with_options` の閉包を使う(`fe-lift/src/lib.rs` が `msvc_rejected` を出す)。CI の fe-lift を Blend2D に走らせた(0.19 秒)結果は `jumptabs 9`・`msvc_rejected 10`(0x477fe11 を含む)で、生成された C は `L_477fe11: { fle_dispatch(R[1]); return; }`。表の行き先 6 つのうち **5 つ(0x477fe13・0x477fe74・0x477ffda・0x478016b・0x4780189)のラベルが C に存在しない**(閉包が達しない)= 実行時にそこへ来れば `fle_dispatch_miss` で止まる。**C 経路も解決していない**ので、fe-wasmgen が再利用できる既存の解決は無い(同じ fe-decode の閉包を直すことになる)。この経路が実行時に踏まれたかは未確認。
**(c) census**: `tables/jt-census-before-after.txt`(8 モジュール前後)・`jt-wasmgen-census-diff.txt`(変わった行だけ。Blend2D は 2 行〔f_4776f60 と、その callee の f_4776030 の署名の入力マスク〕、Elastique は 1 行)。Blend2D の ok は 622 のまま、Elastique の ok は 3,768 → 3,769。
- 『フレームつきの間接 jmp』の残り(`jt-class-of-flagged.txt`、`jt-shapes-in-flagged.txt`、`jt-sites-<module>.tsv`。本差分の rsp-delta の後に RSP の理由で残る関数のうち、未解決の `jmp` を持つ物): **Blend2D 4 関数**(1 つが本節の copy 型の f_4776f60〔大きさの上限で C のまま〕、2 つが『ja の flags が `cmp idx,imm` で書かれていない』〔0x4770ea7 と 0x48aefcc。`cmp` と `ja` の間に別の flag 書き込みが挟まる〕、1 つが f_4774590〔`jne` の辺と `lea` による添字の書き換え〕。HEAD の分類の 5 のうち f_4758e40 は本差分の定数追跡で ok になっている)。**Elastique 約 34 関数**(31 が『MSVC の形でない』: 18 が dsp と同じ 16 ビットのオフセット表、6 が `call; jmp reg`・`jne; jmp reg`、6 が `mulpd/movsd/movapd; jmp reg`、1 が `jmp [reg+disp]`; ほかに本節の copy 型 1〔解決する〕、『ja の flags』型 1、『別の閉包関数が中で始まる』型 1)。**dsp 約 188 関数**(183 が『MSVC の形でない』: **116 が 16 ビットのオフセット表**〔`and r8d,7; … cmp r8d,1; jb; dec r8d; lea r11,[ImageBase]; movzx eax,word[r11+r8*2+T]; lea rax,[r11+rax+K]; jmp rax`。上限は `and` の mask で `cmp/ja` ではない〕、**61 が 8 バイトのポインタ表**〔`lea r10,[ImageBase]; jmp qword[r10+r12*8+T]`、`cmp r8d,edi; ja` と上限が即値でない〕、残り 6 が別の形)。
- 結論(c): copy 型で ok になるのは Blend2D 0(f_4776f60 は RSP の理由が消えるが大きさの上限で C のまま)、dsp 0、Elastique 1(f_b2bc5b0)。『Blend2D 5・dsp 約 210・Elastique 33』の残りには、**mask で上限が決まる 16 ビット表**(dsp 116 + Elastique 18)と **ポインタ表**(dsp 61)という新しい認識器が要る(別の決定、未着手。前者は `and` の mask と `cmp/jb; dec` から N を導く必要があり、後者は再配置から項目数を読む決定 0079/0102 の拡張になる)。
**(d) 健全性**
- 添字が範囲外: `cmp edi,5; ja default` の fall-through は edi ≤ 5(符号なし)。`mov eax,edi` は上位 32 ビットを 0 にするので rax = edi ≤ 5 で、表は 6 個。範囲外の値は決定 0089 のとおり表の読みが行き先の集合から外れて `fle_jumptab_miss`(この機構は変えていない)。`sext_source` の使われ方(`derive_scheduled_guard` が idiom の先頭から前向きに『検査した値を持つレジスタ』cur を追い、idiom へ入る**すべての辺**〔分岐の行き先と fall-through〕の `cmp cur,imm` を読む)の中で、32 ビットの `mov eD,eS` は『D = zext(eS) なので、eS または rS の比較(値 ≤ N < 4096)は D の値と同じ』という段にした。**64 ビットの `mov rD,rS` は認めない**(32 ビットの比較の後は rS の上位が未保証)。copy の後に cur の書き換え(`lea`・`sub`・64 ビット copy)があれば従来どおり拒否、copy の元レジスタが分岐と copy の間で書き換えられても拒否、`cmp eax` で比べて `mov eax,edx` で書き換えた型も拒否(cur が edx へ移り、`cmp eax` は edx の比較でないため。拒否の理由の文言が変わった)。表の読み取り・行き先の検査・基底の証明・`verify` の検査(`jumptab_rejected` は 8 モジュールとも変化なし)は変えていない。
- RSP が全ケースで同じ: 表を解決した後のケースは関数の中のブロックで、fe-wasmgen 自身の RSP の追跡(pres)が全出口を比べる。Elastique の f_b2bc5b0 は pres を通って ok(`push rbx; sub rsp,0x20` の枠、出口は釣り合う)。f_4776f60 は pres を通って lowering の大きさの検査まで進んだ(大きさの検査は pres の後)ので、全出口の RSP は一致している。
- テスト: fe-decode の新しい `scheduled_a_32bit_copy_of_the_compared_register_is_a_step_mt3y_0261`(Blend2D の形を 4 通りの組み合わせで解決〔2 段、gap の nop・cdqe、`jbe` の taken〕、offset・64 ビット copy・元の書き換え・無関係の copy は拒否)。**既存の 2 つのテストの期待を変えた**(拒否のままだが理由の文言が『is not a sign extension』→『are last written by』になった: `mov eax,edx` の copy 型)。変異: copy の段を外すと新旧の 2 テストが失敗、64 ビット copy も通す変異で 2 テストが失敗(検出)。`cargo test -p fe-decode` は 16 結果ok・141 通過・失敗 0。`cargo test -p fe-wasmgen`(copy の段を含む版)は 21 結果ok・30 通過・失敗 0。
- Elastique の f_b2bc5b0 だけを直 wasm にしたモジュール(`--hybrid-only`、7,515 B)は `wasm-validate --enable-all` が通る。**node での実行は 0 件**(この関数は小さく、ミキサーの主要部分でない)。
- 影響: fe-decode が変わるので、**Blend2D と Elastique の C の生成物が変わる**(Blend2D の復号が 48,362 → 48,727 命令、Elastique が 798,890 → 798,960。FLEngine・dsp・qfc・freetype・ILTools・oggio は前後で同じ)。CI のキャッシュ(m2・m5〜m7 の C)の再生成が要る。採否は PM の未決(性能への効果は 0 に近く、C 経路の欠けているケースを埋める正しさの改善と見るのが妥当と私は考える。未測定)。

## PM の決定と取り込み(2026-11-05 22:50、docs のみ)

取り込み: mt3 の最終報告(22:36)の文書・表・道具を `docs/investigations/mt3y/` に、差分 3 本(`mt3-rsp-delta.diff`、`mt3-rsp-delta-on-e4.diff`、`mt3-jt-copy.diff`)を `docs/investigations/mt3y/patches/` に置いた。**製品の木には当てていない**(本決定の差分は 1 行も入らない)。測定は mt3 の census と cargo のテスト(実エンジンの走行は 0 件)で、PM は再現していない。

判断:
1. **`mt3-rsp-delta*.diff` は今回の cold build に入れない。** 理由: (i) 実エンジンの 84/55/87 行・Export・再生が未確認、(ii) 直るのは Elastique の 2 関数(f_b034096 421 ms、f_b04a921 388 ms、ミキサーの約 2%)と Blend2D の f_4758e40(Mixer ドラッグ 201 ms)で、本命の Blend2D f_4776f60(同 546 ms)は直らない、(iii) 新しく ok になる関数は hyb リストに無く、載せるにはもう一度リスト変更(= 全面ビルド)が要る。
2. **`mt3-jt-copy.diff` は採らない。** 解決する表は Blend2D 0x477fe11 と Elastique 0xb2bc5d5 の 2 つだけで、f_4776f60 は hybrid の大きさの上限(1,162,969 wasm バイト、局所 892、決定 0185)で C 経路のまま。性能に効かず、既存テスト 2 件の文言が変わり、Blend2D/Elastique の生成 C が変わる(CI のキャッシュを動かす)ので、採るなら正しさの改善として別決定。
3. **発見の記録**: oggio の 6 関数(0xc01a69a/6b4/707/721/738/790)で HEAD の fe-wasmgen は tail-jmp の callee の k=264 を無視して ok にしていた(rsp-delta ありでは C 経路になる)。いずれも hyb-U-oggio.txt に無く、製品の出力には現れていない。次の担当(0264)で扱う。
4. 未確認: 実エンジンでの動作、indirect call が k≠0 の関数に届く場合(見つからず)、Firefox、依頼者の機械。
