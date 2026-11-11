# 0285(仮): 型付きの間接呼び出し — 直 wasm の caller が直 wasm の callee を、C の `fle_hyb_dispatch` と page-ABI の wrapper を通らずに `call_indirect` で呼ぶ(設計。実装は PM の GO の後。mt3w)

状態: **仮の決定(設計のみ。コードの変更 0 行、実エンジンの走行 0 件)**。作業者 mt3w、2026-11-06 JST。基準: e9-wip `b0fe2c11`(= main c74b9194 + E9)+ 0286(E10 の部品)。区分: **実測**=このセッションで出力を見た値(出どころを書く)/**読み**=コードを読んだ推論/**見積り**=式に仮定を入れた値/**未確認**。lift の忠実性の方針(`docs/policies/lift-fidelity.md`)、no-options(`docs/policies/no-options.md`)、進捗の誠実性(`docs/policies/progress-integrity.md`)に従う。

## 0. 冒頭の三点(正直さ)

1. **実エンジンが実行されたか**: この設計のために実エンジンは 0 回実行した。使った実測は、(a) alpha13 の決定 0276(E5 の本番 wasm のブラウザ V8 プロファイル、再生 30〜50 s・71 ブロック)の数、(b) 私の 0286 の regen(b0fe2c11 の hyb-U リスト 4,046 関数)の fe-wasmgen の出力 `NEW4/<module>.tsv`、(c) コード(`lower.rs`・`fle_dispatch.h`・`fle_thread.c`・`hybrid_glue.c`)の読み。代役・再生・推測の画面や音は一つも使っていない。
2. **この設計が主張すること**: 型付きの間接呼びにした後の『状態』は、**すでに許されている直 wasm→直 wasm の直接呼び出し(callee へ型付きで渡す)と同じ**になり、それに `fle_dispatch` が今していること(landing の確認・進捗の加算と slice・hook・未知の target は名前を付けて停止)を、同じ順序で足すだけ。新しい分岐・特別扱い・オプションは足さない(§8)。
3. **未確認(最大の三つ)**: (i) 動的な呼びのうち、同じ module の『適合する』target へ行く割合 φ(下の式の鍵。既存の境界カウンタ `fle_hyb_cnt` が実装後にそのまま φ を出す〔§7〕が、今は 0 件)、(ii) 新経路の 1 呼びあたりの費用 c_new(V8 の `call_indirect` + adapter + body。測定は未実施)、(iii) 適合の述語を『hyb-U の 4,046 関数』に当てた本当の数(標準の adapter の述語は `real_in` と `pres` を使い、私が数えたのは別物の `sig_in/sig_out` の下限・上限だけ。§2)。**§6 の利得は見積り**で、3 つの未確認が決める。

## 1. 現行の 1 呼びあたりの仕事(読み。`lower.rs:1406` `call_indirect`、`1350–1402` `c_stub`/`c_stub_inner`、`fle_dispatch.h`、`hybrid_glue.c:10,14`)

直 wasm の関数の `call r/m64`(`c_mode`)は次をする。

| # | どこで | 何を | 数 |
|---|---|---|---|
| 1 | caller の wasm | target を `tt` に保存。xmm の plan があれば `xflush`(xmm0〜3 → X[])| 0〜4 store |
| 2 | caller の wasm | 戻りアドレス(`va+len`)をゲストの stack の `[rsp-8]` に書き、`rsp-8` を R[4] に書く | 2 store |
| 3 | caller の wasm | rcx・rdx・r8・r9・rax を R[] へ | 5 store |
| 4 | caller の wasm | `sync_saved_regs`: rbx・rbp・rsi・rdi・r12〜r15 のうち、この関数で local になっている物を R[] へ(callee が C の経路でも見えるように)| 最大 8 store |
| 5 | wasm→C の import | `c.fle_hyb_dispatch(tt)`(別 module への import 呼び)| 1 呼び |
| 6 | C(`fle_dispatch`)| `fle_landing_top` の確認 → direct-mapped cache(0163)の索引・2 load・比較 → `++fle_progress_count >= fle_progress_limit` → `fle_dispatch_hook` の確認 → `fn()`(C の関数ポインタ = C 側の表の `call_indirect`)| 約 6 load/store + 1 間接呼び |
| 7 | C の stub → wasm export | `f_<va>`(page-ABI、`void`)。**R[] から約 10 個を load して型付きの body を呼び、約 9 個を R[] へ store して戻る** | 約 19 メモリ操作 + 1 呼び |
| 8 | caller の wasm | `outs = [rax]`: R[0] から rax を load。xmm があれば `xreload` | 1 load(+ 0〜4)|

**実測(決定 0276、E5、再生 30〜50 s、71 ブロック、V8 プロファイラ付き)**: 小さな adapter(命令 ≤ 200)の自己時間 **259 ms = 3.6 ms/ブロック(ミキサーの 4.6%)**。うち `f_b2c16b0` + `f_b2c16c0`(`lea eax,[rcx*2+1]; ret` / `lea eax,[rcx+rcx]; ret` で、x86 で 2 命令)が **225 ms**(= 小さな adapter の 87%)。`fle_hyb_dispatch` **87 ms = 1.23 ms/ブロック(1.6%)**。この 2 つで 約 4.83 ms/ブロック。1 ブロックの間接呼びは **1.085 M 回**(0274 の仮想時計の数。実際に動いた回数の実測ではなくモデルの入力の数)。⇒ **測れている部分だけで 約 4.45 ns/呼び**(= 4.83 ms ÷ 1.085 M)。上の表の #1〜#4・#8(caller 側の spill/reload。caller の自己時間に入っていて 0276 は分離していない)は**含まれていない**。

## 2. レジスタ/フラグの契約と、型の数

**間接呼びの契約は Win64**(xabi の前提。標準の adapter〔`lib.rs:975–1033`〕と同じ): 入力 rsp・rcx・rdx・r8・r9・rax(rax は部分書き込みの merge 用)、出力 rax、callee-saved(rbx・rbp・rsi・rdi・r12〜r15)と xmm6〜15 は callee が保存、フラグは戻りで未定義。

**一様な型を 1 つ使う**: `(rsp, rcx, rdx, r8, r9, rax, tt) → (done: i32, rax: i64)`。理由(読み): 間接呼びの site には callee が静的に無いので、`call_indirect` の型は site ごとに決められない。callee 固有の最小 signature〔`sig_in/sig_out/elided`〕で呼ぶには callee を知っている必要がある。最小 signature の個数(**実測、`NEW4/*.tsv` の hyb-U 4,046 関数**): 異なる `(sig_in, sig_out)` は fle 564・dsp 79・b2d 44・qfc 97・ft 145・ilt 55・oggio 39・elastique 121。wasm の型の上限(1,000,000)には届かないが、それは問題ではない。site が知らないことが問題である。

**適合の述語は既存の標準 adapter のものをそのまま使う**(`lib.rs:984`): `real_in ⊆ {rcx, rdx, r8, r9, rax}` かつ `pres ⊇ CALLEE_SAVED`。**c_mode では今は評価していない(`filter(|_| !c_mode)`)。** Phase 1 では**加えて**『callee が xmm の register を読みも書きもしない』を条件にする(理由: 古い経路は戻りで `xreload` するので、callee が xmm0〜5 を書いた後の値を caller が見る。新経路でそれを caller の local のままにすると、ABI 上は読んではならない値だが、値が変わりうる。条件に入れれば新旧が同一になる)。条件を満たさない target は §4 の slow path(= 今の経路)へ行く。

適合する関数の数(**実測は下限と上限だけ。述語そのものの数は未確認**。`tools/sigcensus.py`): `sig_in/sig_out` が厳密に適合(sig_in ⊆ {rcx, rdx, r8, r9, rsp} かつ sig_out ⊆ {rax, rsp})= **610 / 4,046**(fle 366・dsp 30・b2d 14・qfc 140・ft 15・ilt 13・oggio 2・elastique 30)、sig_in だけで見る(rax の読みを許す。出力は adapter が捨てる)と **1,296 / 4,046**。ただし `sig_in` に r10・r11 が 2,400 関数以上で入っている(読み: 最小 signature は『callee の下にいる呼びが読む』も合算した live-in で、述語が使う `real_in` より広い)ので、**述語の真の数は 1,296 より多い可能性が高い(未確認)**。数え方の追加は『計測だけの診断』(§7 の工程 1: c_mode でも述語を評価して `indirect_fit`・`nonconf_reasons` を report に出す。生成物は変えない)。

**フラグ**: 間接呼びの後のフラグは未定義(Win64)。census の『callee の ret が立てたフラグを caller が読む』は直接呼びの話で、0270 + 0270b で lowerable になった/ならなかったの規則は**そのまま**。adapter は rax 以外を返さない。

## 3. 表の作り方(決定的)

1. module ごとに、適合する hyb-U 関数を**VA の昇順**に並べる(N 個)。各関数に**標準の adapter**(`lib.rs:1000–1033` と同じ生成。rax 以外の結果は捨てる)を 1 つ作り、**先頭に key の確認**(`tt == この関数の VA` でなければ `(0, 0)` を返す)を足す。型は §2 の一様な型。
2. **funcref の表(wasm の `table`、module の内部)**: 大きさ M = 2^k ≥ 8N(fle 2,927 〔述語を通る数はさらに少ない〕→ 32,768 以下)。索引 `h(v) = ((v − text_lo) · 0x9E3779B1 mod 2^32) >> (32 − k)`(Knuth の定数、**固定**)。同じ索引に落ちた関数 b 個(期待値: N·(N−1)/2M ≈ N/16 組 = fle で約 180 組。**実測は実装時**)は、**同じ key の確認を順に行う『束ね関数』**(`if tt==A → A の adapter、elif tt==B → …、else (0,0)`)を 1 つ作って入れる。空の slot は共有の `miss`(`(0, 0)`)。
3. **データが無い**: 表の中身だけ。線形メモリに hash を置かないので、(a) config に新しいアドレスを足さない、(b) 共有メモリ(`mem_import`)への書き込みが無い、(c) C の側に追加の表が要らない。
4. **決定性**: 並びは VA の昇順、定数は固定、束ね関数の中の順も VA の昇順、時刻・乱数・host のアドレスは使わない。同じ入力から同じ wasm のバイトが出る(実装時に regen を 2 回走らせて sha256 を比べる: 0286 の regen の手順で既にある)。
5. **他の module の target**(例: FLEngine から DLL の関数への関数ポインタ)は表に無い。`miss` から slow path へ行く。

## 4. caller 側(`call_indirect` の `c_mode` の枝)

`call r/m64` の site は次の順に動く。**順序は `fle_dispatch` と同一**(landing → 検索 → 進捗 → hook → 呼び)。

```
caller (site):
   tt = target
   if (load64(&fle_landing_top) != 0 || load64(&fle_dispatch_hook) != 0) → slow
   if (tt >> 32) != 0 → slow                         ; pseudo-VA など
   [rsp-8] = va+len                                  ; ゲストの stack に戻りアドレス(今と同じ)
   (done, v) = call_indirect T(rsp-8, rcx, rdx, r8, r9, rax, tt)[ h(tt) ]
   if !done → slow                                   ; 検索の miss(表に無い)
   rax = v

adapter_<va>(rsp, rcx, rdx, r8, r9, rax, tt):         ; 検索が hit した後・body を呼ぶ前に進捗(fle_dispatch の順序)
   if tt != VA → return (0, 0)
   count += 1                                        ; i64 load/store (progress_addr)
   if count >= limit → call c.fle_hyb_slice          ; 既存の import(hybrid_glue.c:14)
   return (1, body(...))
```

進捗の加算と slice は『検索が hit した後、callee を呼ぶ前』に行う(`fle_dispatch` の順序。slice は thread を切り替えうる)ので、**adapter の key 確認の直後**に置く(上)。miss のときは加算しない(slow path の `fle_hyb_dispatch` が加算する。二重に数えない)。`++count`、limit の比較、`fle_hyb_slice` の import はすべて**既にある仕組み**(demoted な直接呼びの進捗、`lower.rs:5201`、決定 0179)。新しく要るのは `fle_progress_limit`・`fle_landing_top`・`fle_dispatch_hook` の 3 つのアドレスを config で渡すこと(`conf.hybrid_progress`、`lib.rs:66` と同じ経路)。**その経路の実装〔hybrid_progress が harness からどう渡されるか〕はまだ読んでいない(未確認)**。

- **slow path は今の経路そのもの**(§1 の表の #1〜#8)。コードは site ごとに残る(+ 約 100 B)。**結果として site のコードは増える**: 実装後の wasm 増加は site の数 × 約 40 B(新しい枝)+ adapter の数 × 約 40 B。**`indirect_calls` は report に出ている**(Elastique: hybrid の 165 関数に 350 site〔0286 の記録〕)。サイズの総量は §6 の『first idle』の項で扱う。
- **`jmp r/m64`(`Term::TailIndirect`、`lower.rs:5174`)は Phase 1 の対象外**(今の経路のまま)。理由: demoted な tail site は『count −= 1 してから dispatch が +1』(`5201`)を持ち、新経路で同じ偽装を再現するには別の確認が要る。Phase 2 に回す。
- **import の site**(`import_sites`)・**C 経路の ext への呼び**(`c_imp`)は対象外。

## 5. 同一性の根拠(読み)

型付きの間接呼びで、callee に見える入力は**古い経路と同じ値**(rsp-8、rcx、rdx、r8、r9、rax)。古い経路で page-ABI の wrapper が R[] から load していた約 10 個のうち、callee が読む物(`real_in`)は述語により {rcx, rdx, r8, r9, rax} と rsp に限られる。残りは callee が読まない。

| 観測できる物 | 古い経路 | 新経路 | 同じか |
|---|---|---|---|
| 戻り値 rax | 戻りで R[0] から load | adapter の戻り | 同じ |
| rcx・rdx・r8〜r11(volatile)| caller は reload しない(`outs=[rax]`)→ caller の local の旧値 | 同じ(local のまま)| 同じ |
| callee-saved(rbx…r15)| callee が保存(`pres`)。caller の local は不変 | 同じ | 同じ |
| xmm0〜5 | `xreload` で callee の最後の値 | 述語(callee は xmm を触らない)により不変 | 同じ(述語に入れたので)|
| xmm6〜15 | 保存 | 保存 | 同じ |
| フラグ | 未定義 | 未定義 | 同じ |
| ゲストの stack(戻りアドレス)| `[rsp-8]` に書く | 同じ(§4 は書く)| 同じ |
| 進捗の count・slice の位置 | `++count` の後、hit の後・呼びの前 | 同じ(adapter の key 確認の後)| 同じ(**wall_fr20 が検査**)|
| landing / hook | `fle_dispatch` の先頭 | 条件を見たら slow path | 同じ |
| 未知の target(停止)| `fle_dispatch_miss`(名前を付けて停止、thunk の確認)| slow path へ → 同じ | 同じ |
| **R[]、X[] の中身(callee の戻り後)**| callee の最後の値 | 古いまま | **異なる**。ただし R[] は caller が次の c_stub の前に必ず書き直す scratch で、thread の切り替えは R[] 全体を保存/復元する(`fle_thread.c:202,220`)。`fle_progress_slice`(784–803)は R[4] を読まない(読み)。他に R[] を読む物がないことは **opt1 の確認項目**に入れる |

0270b との整合: 0270b の guard は『直接呼びの callee が立てたフラグを caller が読む』の場合だけ関係する。間接呼びのフラグは未定義(§2)で、adapter は rax しか返さないので、census の lowerable/not の判定は**変わらない**。0286 の 4 関数は、適合すれば adapter を持つ(特別扱いなし)。

0002b の教訓(C→wasm の wrapper が callee-saved を壊した): 新経路には C の wrapper がない。ただし**同じ形のテスト(`boundary_regs`: callee-saved・xmm6〜15・フラグ・R[4])を新経路にも走らせる**(§7 の工程 3)。

## 6. 利得の式と見積り(**見積り**。実装前は実測が無い)

測れている部分: 4.83 ms/ブロック(= 小さな adapter 3.60 + dispatch 1.23。§1)、N = 1.085 M/ブロック。未測定: caller 側の spill/reload(§1 の #1〜#4、#8)s ns/呼び、新経路の費用 c_new ns/呼び、適合する割合 φ。

**G [ms/ブロック] = N × φ × (4.45 + s − c_new)** ← 新経路にしたときの減り。4.45 は測れている部分の ns/呼び。

『E7b の ミキサー 70.65 ms/ブロック〔= 78.5 × 0.90、0276 の f〕に対する %』:

| φ | s(spill)| c_new = 1 ns | 2 ns | 3 ns |
|---|---|---|---|---|
| 0.9 | 0 | 4.8% | 3.4% | 2.0% |
| 0.9 | 2 | 7.5% | 6.2% | 4.8% |
| 0.9 | 4 | 10.3% | 8.9% | 7.5% |
| 0.7 | 0 | 3.7% | 2.6% | 1.6% |
| 0.7 | 2 | 5.9% | 4.8% | 3.7% |
| 0.7 | 4 | 8.0% | 6.9% | 5.9% |

読み: 0276 の『4.8〜5.8%』(η = 0.70〜0.85)は、この式で c_new が約 0.7〜1.3 ns・s = 0 の行に相当する。V8 の `call_indirect`(型の確認・範囲確認)+ adapter → body の 2 つの wasm の呼びは、1 ns に収まらない可能性が高い(**未測定**)ので、**s = 0 の行(= 保守的)の 2〜4% を下限と見る**。s は、古い経路の caller 側の store 約 15 個 + load 1 個(§1 の #1〜#4、#8)が消える分で、ブロックの 1.085 M × 数 ns になりうるが、**一つも測っていない**。alpha1 の 0282 の『UI 側の直接呼びの境界 −0.025〜0.03 コア』は別の経路の数で、ここには足さない。

**サイズと first idle への費用**(P1: 起動 10 秒以内。見積り): adapter は標準の形で関数 1 つあたり約 30〜40 B(読み。`adapter_bytes` で実測できる)、4,046 × 約 40 B ≒ 160 KB、site 側 +約 40 B × site 数。0276 の『約 190 KB で first idle のモデル +109 ms』(opt1 のモデルの数。1 KB あたり 0.57 ms)を当てると、**200 KB で +約 115 ms(見積り)**。P1 の予算の中に入るかは、実装後の `first idle` の実測で決める(opt1 の確認項目)。利得が first idle の費用を上回らなければ、**この変更は採らない**(GO の後でも、実測で撤回できる)。

## 7. 実装の工程と、opt1 が実エンジンで見る物

実装(GO の後)の工程:

1. **診断だけ**: c_mode でも標準の adapter の述語を評価し、`indirect_fit`・`indirect_nonfit`・`nonconf_reasons`・xmm の条件を report に出す(生成物は変えない)。hyb-U 4,046 関数の『述語の真の数』が出る。
2. 表 + adapter + 束ね関数の生成(§3)と `call r/m64` の枝(§4)。`TailIndirect` は触らない。
3. テスト: (a) fe-wasmgen の単体テスト — 表の索引・束ね関数・miss・landing・hook の各枝、(b) `boundary_regs` の形を新経路で(callee-saved・xmm6〜15・フラグ・R[4])、(c) `wall_fr20`(進捗の count と slice の位置)、(d) 生成の決定性(regen 2 回の sha256)、(e) `test.sh full` と a4(新旧の一致: 942/942、895/895、5,310、1,009,737、started_but_not_entered 0)。
4. 診断: 既存の境界カウンタ `fle_hyb_cnt`(`hybrid_glue.c`)で、実装前後の `fle_hyb_dispatch` の回数を比べる ⇒ **φ = 1 − (後 ÷ 前)**。

**opt1 が実エンジン(node とブラウザ)で見る物**:
- 同じ入力(場面 b、ミキサー)で **progress / slices / snap の clock_ms の行が古い経路と一致する**こと(`g3-progress`)。一致しなければ、加算か slice の位置が違う。
- **Export の sha が変わる**のは当然(wasm が違う)。変わるのは wasm だけで、**出力(音、Export、画面)は同じ**であること。
- V8 プロファイルで、`fle_hyb_dispatch` と小さな adapter(`f_b2c16b0/c0` ほか)の自己時間が減ること、新しい adapter / 束ね関数の自己時間が増えすぎないこと(c_new の実測)。
- first idle、起動(P1)、node の行(g3 など)、UI 120 fps(P2)、音の途切れ(P3)が悪化しないこと。
- 『R[] の中身が古いまま』(§5 の最後の行)で壊れる物がないこと。

## 8. 方針との適合(必須の記載)

- **lift の忠実性**: 追加する処理は、(c) 通信/変換の削減 — wasm↔C の境界の往復と R[] への spill/reload を減らす。`fle_dispatch` の契約(landing・進捗・hook・未知の停止)は**変えず、同じ順序で実装する**。FL の挙動を補う・代行する処理、class/窓の種類/アドレスによる分岐は**無い**。
- **no-options**: 環境変数・フラグ・オプション・『切り替え』は**無い**(c_mode の hybrid の生成は常にこの経路。表が空の module では分岐が生成されない)。新旧の比較は『コミットの前後』で行い、実行時の切り替えは作らない。
- **進捗の誠実性**: 見積りの表は見積りと書いた。φ・c_new・s は未測定と書いた。

## 9. 検討して採らなかった案

| 案 | 理由 |
|---|---|
| site ごとの型付き `call_indirect`(callee 固有の最小 signature)| 間接呼びの site は callee を知らない(§2)。 |
| 全 VA の表(標準の adapter の方式: `target − text_lo` を索引にする)| FLEngine の text は約 14 MB。1 byte 単位の funcref の表は数十〜100 MB。 |
| 線形メモリに hash(key, slot)を置く | config にアドレスが 1 つ増え、共有メモリの予約が要る。funcref の表 + 束ね関数ならデータが無い(§3)。 |
| C 側の direct-mapped cache(0163)を wasm から読む | C の関数ポインタ(C 側の表の索引)を指す。境界が残る。 |
| site ごとの inline cache(プロファイルで最頻の target を 1〜3 個、直接呼びに)| 最も速いが、site の VA と target の VA を表に持つ(アドレスによる分岐)。プロファイルの入力も無い。**Phase 2 として別の決定にする**(未決)。 |
| すべての body を一様な型にする | 直接呼びが遅くなる。 |

## 10. 私の読みで、まだ確かめていない物(未確認の一覧)

1. `hybrid_progress` が harness からどう渡されるか(同じ経路で 3 つのアドレスを増やせるか)。
2. `xplans`(xmm の plan)と `xsite` の詳細: 述語の『xmm を触らない』の判定に使う情報が c_mode の per-function で取れるか。
3. 適合の述語の真の数(§2)、φ、c_new、s(§6)。
4. `schedule_ex` 全体が R[] を読まないこと(`fle_progress_slice` のみ読んだ)。
5. 束ね関数の数と最大の長さ(実装時に report に出す)。
6. site 数(FLEngine)と、wasm の増加の総量。

## PM の決定と取り込み(2026-11-06 11:55、docs のみ。コードの差分は無い)

取り込み: mt3 の設計(最終は 12:45 まで。変われば更新)を置いた。コードの変更は 0 行、実エンジンの走行は 0 件(利得は式による見積り)。
判断:
1. **実装の GO はまだ出さない。工程 1(診断だけ)を GO する**(mt3 に 11:50 に連絡、第 1 報 13:15、最終 13:45、out `lift/out-git/mt3w-results/0285/phase1/`)。利得は G = N·φ·(4.45 + s − c_new) で、保守的な行(s = 0)が E7b のミキサーの 2〜4%。φ(動的な間接呼びのうち同じ module の適合 target へ行く割合)・s(caller 側の spill/reload)・c_new(新経路の 1 呼びあたりの費用)が全て未測定で、adapter が約 200 KB(first idle +約 115 ms の見積り)と釣り合うか分からないため。
2. 工程 1 の中身: (a) c_mode でも標準 adapter の述語(real_in ⊆ {rcx,rdx,r8,r9,rax}、callee-saved を保存)+『xmm を触らない』を評価して report に出す診断(wasm は変えない)で 4,046 関数中の適合数を module 別に、(b) native N1 の使い捨て木で動的な間接呼びの割合(同 module の適合 target / 適合しない U / C 経路 / 未知)を参照プロジェクトの再生 30〜60 s で、ミキサーと UI を別に、(c) s の見積り(典型の caller の wasm を読んで削れる store/load の数)、(d) GO/NO-GO の推奨と first idle の費用の根拠。
3. 方針との適合(mt3 §8): 新しい分岐・フラグ・環境変数・クラス/アドレス分岐なし。fle_dispatch の契約を変えず同じ順序(landing → 検索 → ++count と slice → 呼び)で実装する、通信/変換の削減。site ごとの inline cache(最頻 target の直接呼び)はアドレス分岐になるので採らない(Phase 2 としても別の決定)。TailIndirect(jmp)と import は Phase 1 の対象外。
4. 撤回条件: 利得が first idle の費用を上回らなければ採らない(GO の後でも実測で撤回する)。UI 側の利得(alpha1 0282 の −0.025〜0.03 コア)は同じ実装の別の見積りで、ここには足さない。
5. 未確認(mt3 §10): hybrid_progress の渡り方、xplans の詳細、述語の真の数、φ・c_new・s、schedule_ex が R[] を読まないこと、束ね関数の数と site 数。
