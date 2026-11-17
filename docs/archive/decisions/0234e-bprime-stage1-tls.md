# 0234e(mt3): B′ の第 1 段 — ゲストの文脈(R / FL / X)の `_Thread_local` 化(opt-in `TLS_REGS=1`)の実装と測定

状態: **仮の決定**。依頼 = PM(決定 0234 の続き、第 3 報への返信 (1))。**この段の範囲は R・FL・X の 3 つ**(LOWREGS=2 の固定アドレスに置かれている物で、直 wasm が絶対アドレスを埋め込んでいる物)。残りの文脈(x87、`fle_mxcsr`、`fle_landing_top`、SEH、コールバックの深さ、TEB・TLS のポインタ、進捗カウンタ、MX 窓)は**この段では動かしていない**(1 Worker の協調モードでは今のまま値渡しで保存・復元され、直 wasm はそれらの C 側のアドレス〔リンクマップ〕を使い続ける)。区分: 実測=このセッションの出力を見た事実(この機械)、読み、未確認。

## 1. 変更(`mt3-tls.diff`)

| ファイル | 変更 |
|---|---|
| `lift/runtime/fle_lowregs.h` | `FLE_TLS_REGS` のとき `R`・`FL` を `fle_ctx`(スレッドローカルのブロック。R +0x000、FL +0x100、X +0x200、大きさ 0x300。LOWREGS の固定アドレスと同じ相対配置)のマクロにする(既定は不変) |
| `lift/runtime/fle_sse.h` | 同じく `X` |
| `lift/runtime/fle_ctx_tls.c`(新規) | `fle_ctx` の定義。フラグ 6 バイトは `.tdata` の初期値 0xff(FLE_UNDEF)。`FLE_TLS_REGS` が無いと空 |
| `lift/runtime/fle_host.c` | LOWREGS の constructor によるフラグの初期化を `FLE_TLS_REGS` では外す(TLS の初期イメージが代わる。**スレッド(インスタンス)ごとに初期化される**) |
| `lift/harness/b2d1/b2d_swap.cpp` | Blend2D のアダプタの R・X のマクロ(C++ の `thread_local`) |
| `app/scripts/build-engine-bundle.sh` | `TLS_REGS=1`(`LOWREGS=2` と `MT2_SHARED=1` が必要): `-DFLE_TLS_REGS`、リンクに `-Wl,--export=__tls_base`、hybrid の段でリンクマップから `fle_ctx` の TLS オフセットを読んで fe-wasmgen に `--tls-ctx` |
| `lift/crates/fe-wasmgen/src/{lib,main,wasm,lower}.rs` | `--tls-ctx OFFSET`(opt-in): レジスタページの全アクセス(17 か所の R と 2 か所の X)が `global.get <import c.__tls_base>` + memarg オフセットになる。既定の出力はバイト同一 |

**fe-lift・fe-pe・fe-decode は 1 行も変えていない**(生成 C のテキストは `R`・`FL`・`X` の名前のまま。`lowregs.py` も変更なし: LOWREGS=2 が定義をすでに `#ifndef FLE_LOWREGS` で包んでおり、`FLE_TLS_REGS` はマクロの中身だけを替える)。= 生成 C の既定の sha256(cd9107ce…)は変わらない(読み。FLEngine の生成の再実行は PM が走らせる)。

## 2. ビルドとテスト(実エンジン、HEAD 2a26d5a6 + この差分。F2xH7、`NO_FS=0`、COMPRESS=none、混成 13,014 関数、node 24 の JSPI・仮想時計。この機械、load 8〜48)

3 つの bundle を同じ木から作った: **v0** = 既定(`MT2_SHARED` なし)、**v1** = `MT2_SHARED=1`(共有メモリ)、**v2** = `MT2_SHARED=1 TLS_REGS=1`(共有メモリ + R/FL/X が TLS)。

### 2.1 fe-wasmgen(cargo)
`cargo test --release -p fe-wasmgen -p fe-decode`: **37 results / 188 passed / 0 failed**(前は 36 / 187。新しい `tests/tls_ctx.rs` 1 本: A を既定の形と `tls_ctx` の形で lower し、C 経路を JS が演じて node で走らせ、どちらもホスト CPU の答えと一致する。tls の形は `c.__tls_base` のグローバル import を持ち、既定の形は持たない)。**既定の出力はバイト同一**(ILTools の混成 1,342 関数の直 wasm、1,248,347 B を `cmp`)。`--tls-ctx 0x0` の直 wasm は `wasm-opt` の検証を通り、1,210,190 B(既定より −3.1%)。

### 2.2 同一性(g3 の行。`tables/tls-stage1-identity.txt`)
| テスト | v0 / v1 / v2 |
|---|---|
| 起動 16 スナップ(`cmp-short.g3`) | **84 行、全行一致**(v0 = v1 = v2) |
| 参照プロジェクト(`ref1-perf-short.g3`、nested の seed) | **55 行、全行一致** |
| parity の 10 セッション(`mk-scripts.py` の wasm スクリプト) | **552 行、10 セッションとも全行一致** |

= 成功条件 (a) を満たした(協調モード、1 Worker)。fe-lift・fe-pe・fe-decode は HEAD と同一(`diff -r`)なので、(b) 生成 C の既定の sha は変わらない(PM が走らせる)。

### 2.3 wasm の大きさ(`NO_FS=0` の bundle)
v0 133,901,932 B → v1 133,921,818 B(+19.9 KB)→ **v2 128,139,479 B(v0 より −5.76 MB、−4.3%)**。S1 の線(F2xH7 は 7.26 MB 下)の下に十分収まる(TLS のオフセットは LEB128 が短く、R/X のアクセス 1 つあたり約 1 バイト減る。読み)。

### 2.4 1 スレッドの CPU(`tables/tls-stage1-cpu-rounds.txt`。`tools3/cpuround.py`)
同じ ref1-perf-short.g3(開く、idle 30 仮想秒、再生 10 仮想秒)を 3 つの bundle で同時に(各 1 つの論理 CPU に固定、物理コアが別、毎周回す)、6 周。load 8〜12。
| | v0(既定) | v1(共有) | v2(共有 + TLS、毎アクセスで `global.get`) |
|---|---|---|---|
| run 全体の user CPU | 38.93 s | 38.84 s(−0.2%、t=−0.9) | 40.99 s(**+5.3%**、t=9.2) |
| 再生の CPU 秒/仮想秒 | 1.169 | 1.169(−0.1%、t=−0.2) | 1.208(**+3.3%**、t=3.6) |
| 開く | 0.864 | 0.875(+1.3%) | 0.890(+3.0%) |
| idle | 0.280 | 0.273(−2.7%) | 0.286(+2.2%) |

v2 は v1 に対して毎周 +1.3〜+7.8%(再生)で一貫。= **成功条件 (c)『再生で既定より +5% 以内』は +3.3% で満たす。run 全体(開く・idle 込み)は +5.3% でわずかに超える**。micro(§0234d)で TLS が固定アドレスより遅くなかったのに、エンジンでは +3〜5% になった。原因の読み: 直 wasm の全アクセスが `global.get $__tls_base`(インスタンスの可変グローバル)を毎回読む。V8 は呼び出しをまたいでこの値を保持できないので、アクセスのたびにグローバルの読み出しが入る。

### 2.5 基底を関数ごとに 1 回ローカルへ取る(v3)と、4 bundle 同時の第 2 の測定(`tables/tls-stage1-cpu-rounds-v3.txt`)
fe-wasmgen の `lower()` の先頭で `__tls_base` を 1 つの i32 ローカルへ取り、その関数のレジスタページのアクセスはその `local.get` を基底にする(`__tls_base` はインスタンスの生存中変わらない)。混成のラッパー(`w_<va>`)も同様。ILTools の直 wasm の `global.get $__tls_base` は 54,509 → 2,684 に減り(プロローグとアダプタだけ)、大きさは 1,210,190 B(既定 −3.1%)。既定の出力はバイト同一(`cmp`)。cmp 84 行・参照 55 行は v0 と全行一致(parity は未実行)。

4 bundle(v0 既定、v1 共有、v2 共有 + TLS、v3 共有 + TLS + 基底のローカル)を同時に、各 1 論理 CPU(別の物理コア)に固定して 6 周、load 13〜20(他担当のビルドと重なった):
| | v0 | v1 | v2 | v3 |
|---|---|---|---|---|
| run 全体の user CPU | 41.79 s | +0.4%(t=1.5) | **+5.6%**(t=30.5) | **+5.0%**(t=14.6) |
| 再生の CPU 秒/仮想秒 | 1.247 | +1.1%(t=1.9) | **+4.5%**(t=6.1) | **+4.4%**(t=4.2) |
| 開く | 0.970 | +1.0% | +2.4% | +1.5% |
| idle | 0.288 | +0.7% | +3.5% | +3.1% |

= **v3 は v2 と変わらない**(再生 +4.4% 対 +4.5%)。費用は `global.get` の読み出しの数ではなく、基底 + オフセットのアドレッシングそのもの(V8 は絶対アドレスの定数を即値で扱える。読み)。micro(§0234d)の『TLS は固定アドレスより遅くない』は、固定アドレス版の micro がコンパイラのエイリアス解析で不利だったために出た結果で、代表的でなかった(撤回する)。

### 2.6 結論
成功条件 (a) 同一性: 満たす(84 / 55 / 552 行、v0 = v1 = v2)。(b) 生成 C の sha: fe-lift・fe-pe・fe-decode 不変(PM が走らせる)。(c) 1 スレッドの CPU: **再生の窓で +3.3%(load 8〜12)/ +4.5%(load 13〜20)= 5% 以内。run 全体は +5.0〜5.6% で境目**。大きさ: −4.3%(S1 の線の下)。未決の点: (1) TLS の +3〜4.5% を許すか、(2) 許さないなら R/X の一部だけを TLS にする案(直 wasm が触る頻度の低い物から。FL と X の XMM 退避 / 復帰は C 経路との境界だけ)。

## 3. 未確認
Chrome / Firefox の実ページでの TLS 版(NO_FS=1・host files)、実スレッド(第 2 段)、残りの 13 変数、全体テスト・a4・生成 C の sha の再実行(PM)。
