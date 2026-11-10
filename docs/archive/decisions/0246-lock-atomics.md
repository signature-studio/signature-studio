# mt4 段階 A: ゲストの LOCK 付き命令と XCHG[mem] を本物のアトミックにする(lifter)— 決定文書の下書き(番号は PM)

状態: **実装・単体テスト・変異テスト・生成 C の差の計数は済み。協調モードの同一性(84/55 行)と再生 CPU の費用は、基準(HEAD 3688a940)と変更版の 2 本をビルド中**(結果が入りしだい本書を更新する)。差分: `lift/out-git/mt4-results/mt4-atomics.diff`(HEAD に `git apply --check` で当たる)。旗・環境変数・opt-in は無い(無条件)。

## 1. 契約(根拠)
- Intel SDM Vol. 2 XCHG: 「メモリオペランドがあるとき、LOCK プレフィックスの有無によらず、交換の間プロセッサのロックのプロトコルが自動的に実施される」。Vol. 3A 9.1: LOCK プレフィックスは、読み・変更・書きを他のプロセッサに対して単一のアトミックな操作にする。
- 現状の lifter は、これらを `fle_ld → 計算 → fle_st` の非アトミックな列にしていた(単一の処理器では意味が同じ)。ゲストのスレッドが別々の Worker(別の処理器)で同時に走ると契約の実装不足になる(FastMM のロック `lock cmpxchg byte [..], ah`、参照カウント `lock xadd`、遅延読み込みの `xchg [mem], reg` など)。

## 2. 変更
| 場所 | 内容 |
|---|---|
| `lift/runtime/fle_rmw.h`(新) | `fle_cas`(比較交換)、`fle_xchg`、`fle_xadd`、`fle_cmpxchg16b`。ゲストのリニアメモリに `__atomic_*`(wasm では共有メモリの i32/i64.atomic.rmw.*)。**旧値を返す**(フラグとレジスタのため)。整列していないオペランド(x86 の split lock。wasm のアトミックは trap)と cmpxchg16b(wasm に 16 バイトのアトミックは無い)は、1 本の大域スピン(`fle_rmw_begin/end`)の下で行う: 同じ経路の相手に対してだけアトミック(制限は 5 に)。`fle_runtime.h` の末尾から読む。 |
| `fe-lift` `int_more.rs` | メモリ形の XADD → `fle_xadd`、CMPXCHG → `fle_cas`(LOCK の有無によらず。上位集合)。レジスタ形は従来のまま。 |
| `fe-lift` `int_more2.rs` | XCHG[mem] → `fle_xchg`。CMPXCHG16B → `fle_cmpxchg16b`。LOCK 付き BTS/BTR/BTC(レジスタ・即値のオフセット)→ 比較交換のループ。LOCK なしは従来どおり読んで書く。 |
| `fe-lift` `lib.rs` `rmw_flags` | LOCK 付きのメモリ宛先の ADD/ADC/SUB/SBB/AND/OR/XOR/INC/DEC/NEG/NOT → `do { o = fle_ld; t = F(o); } while (fle_cas(a, sz, o, t) != o);`(フラグは交換に成功した周のもの)。 |
| `fe-wasmgen` `lower.rs` ほか | 直 wasm(ハイブリッド)は、メモリ形の XADD/CMPXCHG/XCHG と LOCK 付きの全命令を**常に**レーンから外し、その関数は C 経路に残る(決定 0169 の `--atomic-rmw` を無条件にした。旗 `atomic_rmw` と `--atomic-rmw`、死んだ XADD/CMPXCHG の lowering、XCHG のメモリ形を削除。レジスタ形の XCHG は残る)。 |

## 3. テスト(すべて通過。lift の cargo test、HEAD + 本差分)
- 差分テスト `fe-lift/tests/lock_rmw.rs`: ホスト CPU(LOCK 付きの本物の命令)と、lifter が出す C(clang -O0)を、**6 レジスタ・64 バイトのバッファ・定義済みの 6 フラグ**(CF PF AF ZF SF OF。lifted 側が未定義にした値は比べない)で照合。208 ベクトル: add/sub/and/or/xor × 幅 8/16/32/64 × 4 値、adc/sbb × CF=0/1 × 幅 8/32/64 × 2 値、inc/dec/not/neg × 幅、bts/btr/btc(レジスタ・即値のオフセット、32/64 ビット)、xchg/xadd/cmpxchg(一致・不一致、幅 8/16/32/64)、**整列していないオペランド**(interlock の経路)。
- 負荷テスト `fe-lift/tests/lock_rmw_threads.rs`: 4 本の本物のスレッド × 400,000 回で、fe-lift が出すループ(LOCK add の CAS ループ、`fle_xadd`、`fle_xchg` で作ったスピンロック、整列していないオペランド)を回し、合計が厳密に一致する。`fle_xadd` を非アトミックに変えた改変は 4,633,518(期待 4,800,000)で失敗した。
- 既存: `fe-lift` の `int_more`(5)・`int_more2`(10: xchg、cmpxchg16b、bit_rmw)、`fe-wasmgen` の `rmw_forms`(2: 7 形すべてが C 経路に残ることを確かめるテストに作り直した)。
- 変異テスト(lifter を 1 か所ずつ壊して `lock_rmw` が落ちるか): cmpxchg の失敗側の条件の反転、xadd が和を返す、LOCK 付き ALU の CAS を外す、xchg が書かない、bts の CF を 0 に固定 = **5/5 を検出**。

## 4. 生成 C の差(基準 = HEAD の lift、変更版 = 本差分。同じ入力 FLEngine_x64.dll ほか)
7,064,801 行(44 ファイル)中、**629 行だけが変わる**(行の集合の差。lift の分割は行が伸びると境界が動くので、ファイル単位の diff は行の移動も数える): XADD 310、CMPXCHG 137、XCHG[mem] 90、CMPXCHG16B 15、LOCK 付き ADD 38 / INC 20 / OR 12 / AND 7。ほかのニーモニックは 0。この binary に LOCK 付きの SUB・DEC・NEG・NOT・XOR・ADC・SBB・BTS・BTR・BTC は無い。全 629 行が LOCK 付きまたは暗黙の LOCK の命令の行。LOCK なしの BTS/BTR/BTC は従来の出力と同じ文字列(別の形に変えない)。LOCK を含む関数は 440(`lift/harness/seeds/mt4-atomic-functions.tsv`)。表: `gen-c-lines.tsv`。

## 5. 制限
- 16 バイトの比較交換は、同じ interlock を通る相手(他の CMPXCHG16B と整列していない LOCK 命令)に対してだけアトミック。半分の 8 バイトへの通常の書き込みとは排他にならない(ゲストの lock-free リストは CMPXCHG16B だけで更新する前提)。この binary の CMPXCHG16B は 15 か所。
- LOCK なしの通常の load/store の順序: wasm の非アトミックなアクセスは x86 の TSO を保証しない(V8 は実際には並べ替えないが保証ではない)。必要になれば `fle_st` にリリース順序を付ける(別件)。
- ハイブリッド(直 wasm)で C 経路に戻る関数が増える(本命令を含む関数)。数と CPU は未測定。

## 6. 結果(測定済み。この機械・他の担当の負荷 11〜14〜33 のもとでの値。client の機械の値ではない)
- 状態: 「実装・テスト・測定が済んだ。PM の取り込みを待つ」。基準 = HEAD 3688a940 の CI 段階スクリプトのビルド(F2xH7、非ハイブリッド、NO_FS=0、MT2_SHARED=1 TLS_REGS=1。a0)、変更版 = 同じ + mt4-atomics.diff(a1)。ビルドの旗は両方とも同じ。
- **協調モード(単一 Worker、node、仮想時計)の同一性**: 起動 16 スナップ(cmp-short)**84 行**、参照プロジェクト(ref1-perf-short)**55 行**、a0・a1 の両方が基準表(F2xH7-cmp-v0v1v2.txt、F2xH7-ref-v0v1v2.txt)と**全行一致**(差 0 行)。表: tables/identity-a0-a1.txt。(パリティの 10 セッション 552 行は未実施。)
- **再生 CPU**(node、参照プロジェクトの script、5 組、各バンドルを別の物理コアに固定、load 11〜14): 全体の user CPU 65.496 → 65.258 秒(**−0.4%、t=−1.6**)、再生区間 2.115 → 2.104 CPU 秒/仮想秒(**−0.5%、t=−2.0**)。**費用は検出されない**(LOCK 付きの命令は熱い経路にない。単一スレッドの `lock cmpxchg` は 1 回約 20 サイクル)。表: tables/cpu-a0-vs-a1.txt。
- **生成 C の差の説明**: 節 4 のとおり 629 行 = 全部が LOCK 付き・暗黙 LOCK の命令。native 側の基準: 差分テストとスレッドテストは、native の clang で `__atomic` ビルトイン(ホスト CPU の本物の LOCK 命令に落ちる)を実行して、ホスト CPU の同じ命令と照合している。
- **テスト**: `cargo test -p fe-lift -p fe-wasmgen --no-fail-fast`(HEAD + 本差分)= **173 個のテストバイナリ、2152 件通過、失敗 0、無視 1**(tables/cargo-test-summary.txt)。注意: `wall_fr18` は並列実行で作業ディレクトリが衝突することがある(「Text file busy」、1 回目の実行で観測。`--test-threads=1` では基準も変更版も 7/7 通過)。本差分とは無関係。
- **ハイブリッドへの波及(対応済み。節 7・8)**: F2xH7 のハイブリッド一覧 L5s の 13,014 関数のうち **161 関数**(fle 64、ilt 28、qfc 27、b2d 23、dsp 11、ft 8)が LOCK 付き・暗黙 LOCK の命令を含み、直 wasm の lowering が拒否するため C 経路に戻る。`build-engine-bundle.sh` の `hybn0` の検査(`chk_lowered.py`: 一覧の関数がすべて emit されること)は、このままでは落ちる。対応: 一覧から外す(表 = hybrid-L5s-functions-with-atomic-instructions.tsv)、または検査が『拒否された関数』を許すようにする。一覧の再生成は PM/opt1 の層 c・d との調整が要る(本差分は一覧を触らない)。直 wasm の CPU の利得はこの 161 関数の分だけ減る(割合は未測定)。

## 7. ハイブリッド(直 wasm)への波及と、除外が大きい場合の次の作業(設計)
- **除外**: F2xH7 の一覧 L5s(13,014 関数)から 161 関数を外す(データの変更。`mt4-atomics.diff` に含む: `hyb-L5s-*.txt`、`hyb-L5-*.txt`〔F2xH8 用〕、`hyb-sse2-F2x-*.txt`〔F2xH5 用。35 関数〕、`ci/test-hybrid.sh` の個数 13,014 → 12,853 / 13,133 → 12,972、`build-engine-bundle.sh` の説明文の個数)。外した関数の表 = `lift/harness/seeds/mt4-atomic-functions.tsv`(LOCK を含む 440 関数の全部。VA、箇所数、ニーモニック別。理由は『LOCK 付きまたは暗黙 LOCK の命令を含むため C 経路』)。一覧への適用は `lift/harness/hyb/filter-hybrid-list.py`、表の再生成は `lift/harness/hyb/list-atomic-functions.py WSRC`(lift の生成 C から。別のモジュール構成〔Elastique、oggio〕の lift には、その lift から作り直す)。ship6 の hyb-U(3,879 関数)のうち、F2xH7 の lift の表に当たるのは 65 関数(b2d 9、dsp 2、fle 32、ft 3、ilt 10、qfc 9。elastique 139 と oggio 43 は F2xH7 の lift に無いモジュールなので**未判定**。F2xH6 の lift で表を作り直して適用する): `mt4-hyb-U-filter.tsv`。
- **直 wasm の利得がどれだけ減るか**: hyb5 の count ビルド(実行回数)は走らせていない(重い別ビルドが要る)。代わりに、除外後の一覧で F2xH7 のハイブリッドを実際にビルドした版(hy1)と、除外前の HEAD のハイブリッド(hy0)を、同じ node の協調モードの参照プロジェクトの script で 5 組比べる(結果は下の 8)。
- **除外が大きい場合の案(設計。実装は段階 B の後)**: fe-wasmgen が LOCK 付き命令を直 wasm のまま扱う。第 1 版 = **C の helper を呼ぶ**: 直 wasm は C 部の関数を import できる(`--c-imports`)ので、cmpxchg は `old = fle_cas(a, n, acc, src)` を呼び、フラグは既存の CMP の lowering で `acc - old` から作り、`acc != old` なら accumulator に old を入れる。xadd は `fle_xadd`、xchg は `fle_xchg`、LOCK 付きの ALU は wasm の loop で `old = load; new = op(old); if fle_cas(a,n,old,new) != old repeat`(フラグは最後の周)。関数は直 wasm のまま、呼び出し 1 回(wasm → wasm、マージ後)が増えるだけ。第 2 版 = 呼び出しをやめて `i32/i64.atomic.rmw.cmpxchg / .add / .sub / .and / .or / .xor / .xchg` を直接出す(flaglive でフラグが死んでいれば add/sub/and/or は fetch 形一発)。整列していないオペランドは wasm のアトミックが trap するので、第 2 版でも `addr & (n-1)` を見て helper に落とす。cmpxchg16b は C 経路のまま。規模 = lower.rs の約 250 行(4 族 × 幅 4 通り。既存の alu / flags の出力を使う)+ 差分テスト(旧 `rmw_forms.rs` の node テストをアトミック版に直して戻す)で約 1 日。着手の判断は 8 の CPU の差による。

## 8. ハイブリッド(直 wasm)の最終コードでの検証(測定済み。この機械・負荷 15〜25 のもとでの値)
基準 hy0 = main 1a6383d2 のハイブリッド(F2xH7、NO_FS=0、MT2_SHARED=1 TLS_REGS=1。CI 段階スクリプトの `hybn0` の検査・リンク・bundle まで通した実物)、変更版 hy1 = hy0 + `mt4-atomics.diff`(一覧から LOCK 付きの 161 関数を除外した 12,853 関数。`ci/test-hybrid.sh` の個数もこの数に直してある。`test-hybrid: all checks hold`、`hybn0` の検査通過)。
- **同一性**(node、協調モード、仮想時計): 起動 16 スナップ(cmp-short)**84 行**、参照プロジェクト(ref1-perf-short)**55 行**。hy0 と hy1 は**全行一致**(差 0 行)。表: tables/g3-hy0-cmp.txt、g3-hy1-cmp.txt、g3-hy0-ref.txt、g3-hy1-ref.txt。
- **除外で直 wasm の利得がどれだけ減るか**(参照プロジェクトの script を 5 組、各バンドルを別の物理コアに固定、交互、load 15〜25。表: tables/cpu-hy0-vs-hy1.txt、tables/cpuround-hy.txt): 全体の user CPU 44.332 → 45.136 秒(**+1.8%、t=10.8**)、開く区間 0.928 → 0.952 CPU 秒/仮想秒(+2.5%、t=6.0)、アイドル 0.315 → 0.322(+2.1%、t=2.0)、**再生区間 1.299 → 1.307(+0.6%、t=1.4。有意でない)**。5 組のどれでも hy1 の方が 0.6〜1.0 秒(1.5〜2.3%)大きい。
- **直 wasm の利得との比**: 非ハイブリッド(a0、同じ script)の user CPU 65.496 秒に対し hy0 は 44.332 秒(−32%)。除外の費用 0.80 秒は、その利得 21.2 秒の**約 4%**。再生区間では非ハイブリッド 2.115 → hy0 1.299(−39%)に対し除外の費用 +0.008 は利得の**約 1%**。(a0 と hy0 は別の時刻の測定で負荷が違う。割合は目安。)
- **判断**: 除外の費用は小さい(再生区間では検出されない)。**直 wasm のアトミックの実装(節 7 の設計)は、今は着手しない**。ship6 の hyb-U を取り込むとき、その一覧から LOCK 付きの関数を外す(`mt4-hyb-U-filter.tsv`、`filter-hybrid-list.py`)。Elastique・oggio の 182 関数は F2xH6 の lift で表を作り直して判定する。


## PM の判断と取り込み(2026-11-05)

取り込んだ基準: HEAD 561d1003 に `mt4-atomics.diff`(32 ファイル、`patch -p1` で fuzz・offset・reject なし)。`-` 行のうち fe-wasmgen の lower.rs(xadd/cmpxchg のメモリ形の lowering と atomic_rmw 旗の削除)、lib.rs、int_more2.rs(メモリ形の非アトミックな『読んで書く』の置換)、test-hybrid.sh の個数の更新は読んだ。残りはハイブリッド一覧の関数名の除外(データ)と rmw_forms.rs のテストの更新。PM が測った値(この機械、`lift/out-pmchk-mt4/`): FLEngine の既定の生成 C の sha256 **`cd9107ce…` → `0130eeee89d3e40c7b2aea3f2b661bf799cdfc84d486241e096cb18cca5c71c6`(215,811,678 → 215,805,703 B)**(LOCK 付き命令と暗黙 LOCK の 629 行だけが変わる=mt3 の計数)、全体テスト 201 results / 2,400 passed(初回の失敗 11 件は `/tmp` の `Disk quota exceeded` による環境要因で、`TMPDIR` をワークツリーに向けた再実行で noreturn_audit 8/8・mx_rc_wire 1/1・simd_int_forms 2/2 通過)/ 0 failed / 1 ignored、a4 FRUITY 無効 942/942・895/895、`ci/test-hybrid.sh` と `ci/test-needs.sh`(256 通り)rc=0。

1. **採る(旗なし、無条件)。** x86 の LOCK プレフィックスと XCHG[mem] の暗黙の LOCK は ISA の契約で、lifter の『非アトミックな読んで書く』は契約の実装不足だった。本物のスレッド(1 スレッド 1 Worker)で FastMM のスピンロックなどが正しく動くための前提(決定 0234 の段階 A)。
2. 費用: 協調モードの再生 CPU −0.4〜−0.5%(検出されない)。ハイブリッドから LOCK 付き 161 関数(L5s 13,014 → 12,853)を除外する費用は user CPU +1.8%、再生区間 +0.6%(非有意)=直 wasm の利得の約 4%。**直 wasm が LOCK 付き命令を扱う作業は着手しない**(mt3 の判断を承認)。
3. 同一性: 協調モードの起動 16 スナップ 84 行、参照プロジェクト 55 行は、基準・変更版・ハイブリッドの hy0/hy1 の全てで基準表と全行一致。
4. ship6 の hyb-U(3,879 関数)のうち F2xH7 の表に当たる 65 関数は `mt4-hyb-U-filter.tsv` で除外する(opt1 の層 c が使う)。elastique 139 と oggio 43 は、F2xH7 の lift に無いモジュールなので未判定(F2xH6 系の lift で表を作り直して同じスクリプトで適用)。
5. 制限: 16 バイトの比較交換は同じ interlock を通る相手にだけアトミック。LOCK なしの通常アクセスの順序(x86 の TSO)は wasm の非アトミックアクセスでは保証されない(別件)。
