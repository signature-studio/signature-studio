# 0394: C-2(直接関数の境界の X の往復)は採らない。候補 4 関数の X アクセス 357 件のうち省けるのは 1 件(R4-4k)
分類: (c) 通信・変換の削減(R4-4 の候補 C-2 の分類。結論は採らない)
撤回条件: 次の 2 つが両方示されたら、この決定を見直す。(1) C の入口(`fle_hyb_dispatch`)から直接関数に入る集合を、入口の全経路を読んで閉じた形で列挙できる。(2) 出口の store を呼び元の reload だけに絞っても、C 側の読みが変わらないと示せる。その上で候補 4 関数の X アクセスが静的に 1 割超減ること。R 側(sync_saved_regs・c_stub)は decision 0169 の契約の変更になるので、別の決定にする。
日付: 2026-11-16   ROADMAP: R4-4(候補 C-2)   一覧: なし   コミット: 試作 17c2376(実験ブランチ r4-4k-c2、ローカル。push していない)。この文書の commit は後続。

## 決定
- 決定: C-2 を採らない。根拠: 候補 4 関数の X アクセス 357 件(呼び出し計画の静的な数)のうち、試作(揮発 XMM の must-write)で省けたのは 1 件(f_b2f3650 の 0xb2f373a の xmm1)。残りは、呼び先が読む呼び出し前の store 154 件、呼び出し後の load 84 件、入口の load 35 件、出口の store 74 件(C の入口があるため、呼び元の reload だけでは省けない)、省けない reload のみの store 9 件(呼び先が C 経路で mw = 0)。
- 決定: bundle のビルドと Worker CPU の比較はしない。根拠: 省ける量が 1 件で、差は測定の散らばり(R4-4i の c の範囲 836.1〜850.1 ms/s)より小さいと見込まれる。見込みは未確認。
- 決定: R 側は C-2 の対象にしない。根拠: xabi.rs に R の規則は無い。R の往復は lower.rs の sync_saved_regs と c_stub で、decision 0169 の契約(SEH の unwinder が R[] を読む)による。
- 決定: 試作は main の対象にしない。根拠: 採らないため。試作は実験ブランチ r4-4k-c2 の commit 17c2376 に残す(ローカル)。

## 実測(コマンド・条件・日付・n)
- 計画のダンプ(fe-wasmgen の一時ダンプ XD_DUMP。4f4bb1b と試作の 2 版。hybn0 と同じ引数、番地はダミー): FLE 2,758 関数・14,077 呼び出し、elastique 165 関数・801 呼び出し。n は 1 回(計画は決定的)。2026-11-16。出典: evidence/R4-4k.md §4・§5。
- 数え方: 呼び出し前の store = 計画の flush の bit 数。入口 = entry の bit 数。出口 = 出口数 × own の bit 数。呼び出し後の load = reload の bit 数。
- 候補の X アクセス(4f4bb1b): f_b2f3650 197(store 127・load 53・入口 7・出口 10)、f_b1ce5e0 110(23・23・16・48)、f_b2f4490 50(14・8・12・16)、f_5287c0 0。
- 試作: f_b2f3650 の store 127 → 126(0xb2f373a の xmm1)。他の候補は変わらない。FLE 全体の store は −481 bit、elastique は −15 bit。
- cargo test -p fe-wasmgen: 4f4bb1b と試作で同じ(passed 71・failed 0・ignored 1)。
- hybn0 の wasm(bundle ではない): FLE −6,148 B、elastique −171 B。

## 読み(解釈。実行していない)
- 呼び出し前の store の大半(154/164)は、呼び先が読むので必要。省ける store は、呼び先が全経路で書く揮発 xmm の reload のみの分だけ。
- 出口の store は、直接の呼び元が読まなくても、C の入口(hybrid_glue.c:10 の `fle_hyb_dispatch`。VA で引く。hybrid_glue.c:29 の交差の数え方)から入る場合に必要。
- 揮発に限った理由: fle_seh.c:409 は RaiseException の時点で X[6..15] を文脈に写し、fle_seh.c:366 は着地で戻す。callee-saved の省略は、この文脈を変えうる。
- R 側(callee-saved 8 本の sync、c_stub の ins・outs)は、SEH の unwinder が R[] を読む前提(0169)。narrowing には、C 側の関数が例外を投げるかの分析が要る。

## 未確認
- 実エンジンでの効果、Export の sha256、Worker CPU、P2・P3 の数字。bundle は作っていない。
- f_b2f3650 の計画の 197 と R4-4j の wat の 199 の差 2 件。R 側の計数(420・73・53・63 件)の内訳。
- C 経路の呼び先(mw = 0 で扱った)の X の全経路の性質。fle_dispatch の対象の全体(どの直接関数が C から入るか)は読んでいない。
- elastique の入力(他の作業ディレクトリの複製)が、この作業の gen 段の出力と同じかは照合していない。

## 出典(旧決定番号・ファイル)
- docs/archive/decisions/0164-direct-backend-ott-pilot.md(XMM の境界の規則。xabi.rs)、0169-direct-backend-real-engine.md(sync_saved_regs と SEH の R[])、0179-wasmgen-hybrid-identity.md(xabi の素通りの修正)、0234e-bprime-stage1-tls.md(R/X の TLS 化の案)、0285-typed-indirect-call.md(R[] の scratch)。
- evidence/R4-4.md、R4-4i.md(c の CPU の範囲 836.1〜850.1 ms/s)、R4-4j.md(R/X の計数と候補 C-2)、R4-4k.md(本文と計画の数)。
- lift/crates/fe-wasmgen/src/xabi.rs、lower.rs:1246・1262・1352・5117・5694、lib.rs。lift/runtime/fle_seh.c:366・409。lift/harness/hyb1/hybrid_glue.c:2・10・29。
