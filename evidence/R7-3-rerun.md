実エンジン: 実行なし(文書の grep だけ)
結果: 完了条件 1 の grep 2 行、(iii) 本文として移っている行 0 件(直した件数 0。docs/ は変更していない)
※ 完了ではない: P2 の 120 fps は未確認、P3 は音切れが残る、O9 は未特定、依頼者機の値は 0 件。この記録は文書の食い違いの確認だけ
変更: evidence/R7-3-rerun.md(新規。a2147e1)。evidence/R7-3.md は a5f2d5a で誤って上書きし、a2147e1 で元の版(9f1c8c8)に戻した(元の版との差分は空)。docs/ は変更していない。基準 HEAD d2bd504(main)。

## 1 食い違いの正しい姿(出典つき)

計画 §5.3 は `docs/` に無い。`grep -rn '計画' docs --exclude-dir=archive` と `expire_grep` の `5\.3` で確認し、計画 §5.3 に当たる文書は見つからなかった。出典は `docs/ops/fidelity-and-performance.md` §5 と `docs/ops/honesty.md` §6。

| 食い違い | 正しい姿(表の記述) | 出典 |
|---|---|---|
| a 「帯 STAND-IN」 | 旧 app の代役表示の帯は「旧ワークツリーの実装。新リポジトリにあるとは書かない。持ち込むか作り直すかは別の担当で、持ち込まれるまで §4 の検査で代替」。代役を見せる必要が出たときの規則は 3 点(案内の先頭行・物自体・明示パス)で、既定の入口には置かない | `docs/ops/honesty.md:73`、`docs/ops/honesty.md` §2 の 2 |
| b 「約 40 秒・320 MB」 | 「履歴の値。移さない。新しい基準は P1(10 秒以内)で、値は再測定する」 | `docs/ops/honesty.md:71` |
| c `?debug=1` | 新 docs に記述が無い。§5 にも `honesty.md` §6 にも行が無い。近い規則は点検表 2 行目「FL の観測(診断・ログ・計測)だけか → 製品の経路から切り離す(製品の中にスイッチを置かない)」と、AGENTS §6 の「新しい URL パラメータを作らない」 | `docs/ops/fidelity-and-performance.md:33`、`AGENTS.md` §6 |
| d 決定 0222 の逆スイッチ | 「決定 0222 の差分は『すべて逆スイッチのマクロつき』で、オプション禁止と衝突」。規則は「新しい (b) の拡充に逆スイッチ(#ifdef・マクロ)を付けない。比較は前後のコミットで」 | `docs/ops/fidelity-and-performance.md:70`(§5 の行)、同 §6 |

## 2 完了条件 1 の grep(G1)の分類

コマンド: `grep -rniF -e '約 40 秒' -e '320 MB' -e '?debug=1' docs --include='*.md' --exclude-dir=archive`。rc 0、2 行。

| 行 | 分類 | 理由 |
|---|---|---|
| docs/ROADMAP.md:63 | (i') 作業計画の依頼文 | R7-3 の依頼そのもの。旧記述を事実として書いていない。(iii) には数えない |
| docs/ops/honesty.md:71 | (i) と (ii) | 「履歴の値。移さない」。docs/ops/ の既存の説明行 |

集計: (i) 1、(i') 1、(ii) 0(honesty.md:71 は (i) と (ii) の両方)、(iii) 0。

字義との差: 完了条件の字義は「各行が『移さない』の説明だけ」。docs/ROADMAP.md:63 は説明ではなく依頼文なので、字義どおりには満たさない。(iii) = 0 は満たす(§6 の 3)。

## 3 「帯 STAND-IN」(G2 と G5)

- `grep -rniF 'STAND-IN' docs --include='*.md' --exclude-dir=archive` → 1 行。docs/ROADMAP.md:63(i')。
- `grep -rn '帯' docs --include='*.md' --exclude-dir=archive` → 2 行。docs/ROADMAP.md:63(i')、docs/ops/honesty.md:73(i)「旧ワークツリーの実装。新リポジトリにあるとは書かない」。

(iii) = 0。現行の仕組みとして書かれた「帯 STAND-IN」は 0 件。

## 4 「逆スイッチ」「0222」(G3 と G4)

G3: `grep -rn '逆スイッチ' docs --include='*.md' --exclude-dir=archive` → 6 行。

| 行 | 分類 | 理由 |
|---|---|---|
| docs/core/pitfalls.md:33 | (i) | 規則「逆スイッチ(#ifdef・マクロ)を付けない」 |
| docs/core/win32-contract.md:27 | (i) | 「決定 0222 の差分は逆スイッチのマクロつき(今は付けない)」 |
| docs/decisions/0353-l01-l04-and-sweep1-stubs.md:10 | (i) | 「逆スイッチ・旗は作っていない」(L01 の記述) |
| docs/ROADMAP.md:22 | (i') 作業計画 | R1-1 の作業規則「逆スイッチ・旗を付けない」 |
| docs/ROADMAP.md:63 | (i') 作業計画 | R7-3 の依頼文 |
| docs/ops/fidelity-and-performance.md:70 | (i) | 食い違い d の行。「付けない」 |

G4: `grep -rn '0222' docs --include='*.md' --exclude-dir=archive` → 5 行。

| 行 | 分類 | 理由 |
|---|---|---|
| docs/core/pitfalls.md:9 | 対象外 | F-1 の原因(0222 の (b))と解消済みの記述。逆スイッチに触れていないので、食い違い d ではない |
| docs/core/win32-contract.md:27 | (i) | G3 と同じ行 |
| docs/core/win32-contract.md:44 | 対象外 | 出典の一覧 |
| docs/ops/fidelity-and-performance.md:70 | (i) | G3 と同じ行 |
| docs/ROADMAP.md:63 | (i') | G3 と同じ行 |

(iii) = 0。「逆スイッチを付ける」方針として書かれた行は 0 件。G3 の全 6 行は「付けない」「作っていない」「移さない」のどれか。

## 5 完了条件 1 の再実行(直しなし)

同じ grep を再実行: rc 0、2 行(§2 と同じ 2 行)。(iii) = 0。

参考(対象外): `grep -rniE '40 ?秒|320 ?MB|debug' docs --include='*.md' --exclude-dir=archive` → 5 行。docs/core/reference-project.md:39(曲の再生の長さ。起動時間ではない)、docs/ops/environment.md:79 と docs/ops/launch.md:13(`opencode debug agents` のコマンド名)、残り 2 行は §2 の 2 行。

## 6 記録の欠け・依頼・提案(main へ返す。この worker は範囲外の文書を変えていない)

1. 「?debug=1」は、新 docs に記述が無く、「移さない」の行もどこにも無い(§5 にも honesty §6 にも無い)。移っていない(0 件)ことは確認できたが、記録が欠けている。提案: `docs/ops/fidelity-and-performance.md` §5 に 1 行足す(別の依頼にする)。
2. 計画 §5.3 は見つからない。出典の表は、d は `fidelity-and-performance.md` §5(70 行)、a・b は `honesty.md` §6(71 行・73 行)。fidelity §5 の他の 6 行は、食い違いの 4 件とは別の項目(getenv・補足の一覧・点検・分類行・方針・docs/policies)。
3. 完了条件 1 の字義: docs/ROADMAP.md:63 が grep の 1 行に入る。字義を「docs/ROADMAP.md の依頼文を除く」と直すかは main が決める。ROADMAP は変更していない。
4. 直し 0 件のため、docs/ の commit は無い。commit は evidence/R7-3.md のみ。

## 7 検査

- `scripts/scan-forbidden.sh evidence/R7-3.md` → `scan-forbidden: 0 findings in 1 file(s)`、rc 0
- `scripts/preflight.sh` → rc 0。consistency 行は、作業の途中では `WARN 3 finding(s)`(R2-3 の未 commit の `docs/decisions/0361-virtual-pointer-contextmenu.md` が `evidence/R2-3.md` を参照していた。私の範囲外)。直後の `scripts/check-consistency.sh` は `consistency: 0 finding(s)`、rc 0 で、preflight の再実行で OK になる(commit の前の出力は、この行の判定に使う)
- 禁止の語の検査(語の一覧はこの報告に書かない。`scripts/scan-forbidden.sh` が見る語を含む語の一覧で `grep -n -E` を実行): 出力 0 行、rc 1(該当なし)。最初の版は 1 行に禁止語そのものを書いていたため scan が 2 件を止めた。その行を、語を書かない形に直した

## 8 付録: 生の出力(`grep` の出力をそのまま貼る)

完了条件 1(G1)の出力の全行:

```
docs/ROADMAP.md:63:| R7-3 | todo | R1-1 | 整備 | 旧文書の食い違いの訂正を `docs/` に反映する(計画 §5.3 の表) | `docs/ops/fidelity-and-performance.md` §5、計画 §5.3 | 食い違い(旧 `progress-integrity` の「帯 STAND-IN」、「約 40 秒・320 MB」、`?debug=1`、決定 0222 の逆スイッチ)が、新 `docs/` に**移っていない**ことを `grep` で確認 | `grep -rniF -e '約 40 秒' -e '320 MB' -e '?debug=1' docs --include='*.md' --exclude-dir=archive` の各行が「移さない」の説明だけ(`docs/ops/` の既存の説明行を除く。本文として移っている行 = 0) | 計画 §5.3 |
docs/ops/honesty.md:71:| 「最初の idle まで約 40 秒、wasm は約 320 MB」を妥当の目安とする | 履歴の値。移さない。新しい基準は P1(10 秒以内)で、値は再測定する |
```

`grep -rniF 'STAND-IN' docs --include='*.md' --exclude-dir=archive` の全行(1 行): docs/ROADMAP.md:63(R7-3 の依頼文。(i'))

`grep -rn '逆スイッチ' ...` の全行(6 行): §4 の G3 の表のとおり。
`grep -rn '0222' ...` の全行(5 行): §4 の G4 の表のとおり。
`grep -rn '帯' ...` の全行(2 行): §3 のとおり。
`grep -rnE '40 ?秒|320 ?MB|debug' ...` の全行(5 行): §5 の参考のとおり。

§2 の `(i')` は依頼文(ROADMAP 63)、`(i)` は「移さない」の説明、`(iii)` は 0 件。

## 追記 2026-11-14(reviewer 指摘の直し)

直す前の 4 行目: `変更: evidence/R7-3.md のみ。基準 HEAD d2bd504(main)。ワークツリーに R2-3 の未 commit 変更が残っているが、触れていない。`
