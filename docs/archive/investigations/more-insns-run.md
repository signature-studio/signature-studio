# more-insns 実行結果（0013）

作業者（M1）の記録。決定と仕様の出典は `docs/decisions/0013-more-insns.md`。

## 1. 未対応の上位 20 件（前）

生成 C（`lift/out-more/out/before-more-896.c`、閉包 896 seed、現行の生成器）から集計。`unsupported_total` は 951（gen 報告と一致）。

| 件数 | ニーモニック |
|---|---|
| 431 | Jmp: indirect through IAT slot（前回の報告）。生成 C の集計では Jmp 451（IAT 431 + 解決不能 20） |
| 56 | Xadd |
| 53 | Bt |
| 30 | Cmpxchg |
| 27 | Packuswb |
| 24 | Paddw |
| 23 | Mulps |
| 20 | Maxps |
| 18 | Div |
| 16 | Minps |
| 16 | Pshuflw |
| 14 | Psllw |
| 13 | Shufps |
| 12 | Vmovups |
| 11 | Xchg |
| 10 | Addps |
| 10 | Movd |
| 10 | Packssdw |
| 8 | Movlps |
| 8 | Psrlw |

## 2. 実装した命令と件数

| 命令 | 生成 C の件数（前 → 後） | 仕様の出典 |
|---|---|---|
| Xadd | 56 → 0 | felixcloutier xadd |
| Cmpxchg | 30 → 0 | felixcloutier cmpxchg |
| Bt | 53 → 0 | felixcloutier bt（Bit() 本文は未取得） |
| Div | 18 → 0 | felixcloutier div |
| Packuswb | 27 → 4 | felixcloutier packuswb（残り 4 件は MMX 形、対象外） |
| Paddw | 24 → 4 | felixcloutier paddb:paddw:paddd:paddq（残り 4 件は MMX 形、対象外） |
| Mulps | 23 → 0 | felixcloutier mulps |
| Maxps | 20 → 0 | felixcloutier maxps |
| Minps | 16 → 0 | felixcloutier minps |
| Addps | 10 → 0 | felixcloutier addps |
| Subps | 4 → 0 | felixcloutier subps |

## 3. 仕様を確認できず実装しなかった命令

決定記録の「仕様を確認できず実装しなかった命令」を参照。主なもの: Shufps、Pshuflw、Psllw、Psrlw、Movlps、Movd、Packssdw、Xchg、Bts、Btr、Bsf、Rol、Ror、Vmovups、Xgetbv、Fldcw、Fnstcw など。

## 4. テスト

- `int_more`（`cargo test --offline -p fe-lift --test int_more`）: 5 件すべて通過。ホスト突合は 55 ベクタ（xadd 12、cmpxchg 10、bt 21、div 12）。`divide_error_aborts` は C 側のみの #DE 検査（3 件）。
- `sse_more`（`cargo test --offline -p fe-lift --test sse_more -- --nocapture`）: 7 件すべて通過。ホスト突合は 338 件（addps、subps、mulps、minps、maxps 各 48、packuswb と paddw 各 49）。failures 0。
- `cargo test --offline --workspace --no-fail-fast`: 失敗は 1 件（`verify::verify_version_info_thunk_stays_unsupported`）。内容は後述 4.1。

### 4.1 workspace の失敗（私の変更の範囲外）

- `verify_version_info_thunk_stays_unsupported`: 検査 `!st.handled.contains_key("jmp_imp:VerifyVersionInfoW")` が失敗。`imports_table.rs` に `VerifyVersionInfoW` が登録されている（10:57 更新。私の作業中に外部で変わった）。私の変更は `imports_table.rs` を含まない。
- 作業の途中で一度 `osver.rs` のコンパイル不能（`BTreeMap<&str,_>` に `&String` で索引）が出たが、次の実行では `osver` は 6 件すべて通過した。外部の変更で解消したとみられる（推測）。

## 5. 未対応の件数の前後（ニーモニック別）

- 前: `unsupported_total` 951（`before-more-896.c`）。
- 後: `unsupported_total` 672（`after-more-896-b.c`）。差 279。
- 内訳: 私の変更による差は 273。残り 6 件は Jmp（451 → 445）で、`imports_table.rs` の外部変更（10:57）による。前後の比較はこの 6 件を含むので、純粋な効果は 273 件と読むこと。
- 差分の表は `lift/out-more/cmp.txt`（前後の件数、ニーモニック別）。

注: `lift/out-more/out/after-more-896.c` は 10:59 に古い release バイナリで作った生成物で、`fle_unsupported` は 951 件のまま。後続の正しい after ではないので使わない（rm 禁止のため残置）。正しい after は `after-more-896-b.c`。

## 6. clang とリンク

- 生成 C のコンパイル（`clang -O0 -finstrument-functions`）: 成功（`lift/out-more/build.sh`）。
- リンク: 最初の試行は `fle_imp_GetVersion`、`fle_imp_GetVersionExW`、`fle_imp_VerifyVersionInfoW` が未定義で失敗。原因は `fle_osver.o` をリンクに入れていなかったこと。`build.sh` に `fle_osver.c` を加えて成功（`bin/flengine-after-more-896-b`）。
- 対照の before（同じ build.sh）も成功（`bin/flengine-before-more-896`）。

## 7. 実行の停止点

同じ harness（`harness/main.c`）、同じ DLL（`binaries/win64/FLEngine_x64.dll`）、同じ entry（既定の DllMain 相当）。

| | 停止 VA | 関数 | abort の種類 | 関数実行数（distinct） | 関数の入り数（calls） | 呼び出し深さ |
|---|---|---|---|---|---|---|
| 前回（out-verify、10:25） | 0x407370 | f_407370（VerifyVersionInfoW の IAT jmp） | fle_unsupported（SIGABRT） | 29 | 45 | 9 |
| 対照 before（out-more、同じ build） | 0x407370 | f_407370 | fle_unsupported（SIGABRT） | 29 | 45 | 9 |
| after（out-more、11:01） | 0x407060 | f_407060（IAT jmp） | fle_unsupported（SIGABRT） | 31 | 47 | 8 |

- 前回と対照 before は一致（0x407370）。
- after の停止点は 0x407060。命令は `jmp QWORD PTR [rip+0x156a302]`（IAT スロット 0x1971368 の間接 jmp）。呼び出し元は `f_4099c0` の `0x4099e4 call 0x407060`（`entry 46`）。`0x4099c0` は `entry 45`、その前の `entry 44` は `0x407370` で、after では `0x407370` を返して実行が続いた。
- 注意: 時系列では 0x407370 より後だが、VA は 0x407370 ではない。報告条件「0x407370 より前に変わったら報告」には該当しない（後ろに進んだ）が、停止 VA は変わった。
- entry 0～44 は before と after で完全に一致。差は、`entry 44`（`0x407370`）の後に `0x4099c0` → `0x407060` が続くこと。
- 差の原因: after の生成 C では `f_407370` が `fle_imp_VerifyVersionInfoW()` の呼び出し。before は `fle_unsupported`。`imports_table.rs` の `VerifyVersionInfoW` 登録は、before の生成（10:41）の後（10:57）に入っている。実測: 2 つの生成 C の差分と、entry 0～44 の一致。
- 私の振り分けの効果は、この経路では分離できていない。before と after は、lib（私の振り分け）と imports_table（外部）の両方が違うため、交絡している。私の振り分けが Jmp の経路を変えないことはコードから言えるが、対照実験（振り分けだけを外した同じ imports_table での生成）は、既存ファイルの編集を伴うため実施していない。
- 私の振り分け（`int_more`、`sse_more`）が `f_4099c0`、`f_409bf0`、`f_409bd0`、`f_409b30` の C を変えていないことは、C の差分で確認（SAME）。

注: 前回の報告（0x407370 が停止点）は、今回の before と一致する。停止点を 0x407370 より先へ進めるには、`imports_table.rs` の外部変更と、Jmp の IAT 経由（431 件）の扱いが要る。

## 8. 失敗・拒否された操作

- 拒否はなし（権限拒否は出ていない）。
- 失敗: (a) `after-more-896.c` の生成が `create_new` で既存ファイルに当たり失敗（前の生成物があったため。上書きはせず、新しい名前 `after-more-896-b.c` で再生成）。(b) リンクの未定義参照（`fle_osver.o` の不足。build.sh を直して解消）。(c) `cargo test --workspace` の `verify` 1 件の失敗（4.1 参照）。(d) 一時的な `osver.rs` のコンパイル不能（外部変更とみられる）。
- 規則違反の疑い: なし。`rm` は使っていない。既存ファイルの上書きはしていない（`create_new` の失敗は上書きではない）。git は使っていない。`/tmp` は使っていない（`TMPDIR` は `out-more/tmp`）。

## 残りの作業量

- 未対応は 672 件（閉包 896 seed）。上位: Jmp 445（IAT 経由の未解決、`imports_table.rs` の範囲）、Pshuflw 16、Psllw 14、Shufps 13、Vmovups 12、Xchg 11、Movd 10、Packssdw 10、Movlps 8、Psrlw 8。
- 停止点の先へ進むには、Jmp の IAT 経由（IAT の未登録分）と、次に現れる未対応の命令の仕様確認が要る。
- 仕様の未取得: Bt の Bit() 本文（SDM Vol. 1 の 3-11）。Intel SDM 本体は未取得で、すべて felixcloutier の転載。
