# 0374: 0372 の親(`a03c903`。R7-6 あり・0372 なし)から作った bundle で、About を閉じた後の停止を観測する(R5-3j)

分類: 測定(lift の処理を足さない。(a)(b)(c) の対象外。ソースは変えず、bundle の作成元の commit を変えるだけ)。撤回条件: この観測の結果を、0372 を main に入れるかの選択にだけ使う。別の窓・別の道具・別の bundle の結果と混ぜて判断しない。

日付: 2026-11-15   依頼: R5-3j(main が付けた名。ROADMAP の行は変えない)   前コミット: `110df5e`(R5-3i)   観測対象: `a03c903`(ブランチ `r5-3j-parent` に置く。`r5-3h-coinitializeex`(`2f1915b`)は変えない)

## 決定
決定: `a03c903` から 9 段のビルドで bundle(`~/.cache/signature-studio/work-r53j/bundle/fl-Oz`)を作り、work-r53h と並べて、本書の定義どおりに台本 A で観測する。判断の規則は置かない(観測の記録)。根拠: 0373 で work-r53h が 3/3 止まらなかった効果が、0372 によるか、R7-6 によるかを切り分けるため。次の作業の選択(0372 を main に入れるか、次の停止の解析)は、結果の後に別の決定文書で、規則を先に書いてから行う。

## 問い
work-r53h(0372 あり・R7-6 あり)が台本 A で止まらなかった(`evidence/R5-3i.md` で 3/3)のは、0372 によるか、R7-6 によるか。
- 背景: `git diff --stat 4ba7ce1 a03c903 -- lift app ci` は `lift/runtime/fle_appfile.c`・`fle_proc.c`・`fle_proc.h` の 3 ファイル(R7-6。決定 0364、commit `33b676a`)。`git diff --stat a03c903 r5-3h-coinitializeex -- lift app ci` は 12 ファイル。うちテスト以外は `lift/runtime/fle_ole.c`・`fle_ole.h`・`lift/crates/fe-lift/src/imports_table.rs` の 3 ファイル(0372)で、残り 9 ファイルはテスト(`lift/crates/fe-lift/tests/`)。したがって `a03c903` のソースは「0372 なし、R7-6 あり」。

## 定義(観測の前に固定。0373 と同じ)
1. 「停止した」= その run の `report.json` が `stopped: true` かつ `state: exited` かつ `beaconExit.code` が 134。
2. 「同じ停止」= `beaconExit.message` が `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]` と一致。
3. 「止まらなかった」= `stopped: false` かつ `state: engine-idle` かつ `opErrors` が空(台本が最後まで届く)。
4. それ以外は「別の結果」とし、`message` と `code` をそのまま書く。`message` に `dwCoInit is not COINIT_APARTMENTTHREADED` を含む停止(0372 の名前つき停止)も「別の結果」として区別して書く。bundle が配れない・manifest の検証で失敗する場合は「配れなかった」とし、原因を書いて止める。

## 組(各 n=3。直列。台本 A = `tools/verify-kit/tools/scn-r53-about-ja.json`)
- 組 1: work-r53j(`a03c903` から作った bundle。0372 なし・R7-6 あり)。label `r53jold`。
- 組 2(対照): work-r53h(`~/.cache/signature-studio/work-r53h/bundle/fl-Oz`。0372 あり・R7-6 あり。R5-3i で 3/3 止まらず)。label `r53jnew`。
- 順: 組 1・組 2・組 1・組 2・組 1・組 2 を交互に 1 回ずつ。
- work-r25 の対照(0372 なし・R7-6 なし)は、R5-3c・R5-3i で 3/3 停止を観測済みのため、今回は走らせない(出典は `evidence/R5-3c.md`・`evidence/R5-3i.md`)。

## 静かさ
0373 と同じ。各回の直前に `node tools/verify-kit/tools/wait-quiet.mjs 8 10`(rc を記録)と `/proc/loadavg` を記録する。`wait-quiet` の 2 秒の swap の入出力が 0 でない・1 分 loadavg が 8 超・I/O some avg10 が 0 でない回は数えず、同じ位置で撮り直す(最大 3 回の試行)。

## 予測(観測の前)
- 組 2(work-r53h): 3 回とも止まらない(R5-3i の再現)。
- 組 1(work-r53j): main の予測は (a) 3/3 停止。根拠: R7-6 は到達しない枝の除去で、About の経路と無関係と読めるため(0364)。
  - (a) 3/3 停止なら、止まらなくなった効果は 0372 による(R7-6 だけでは止まらない)。
  - (b) 3/3 止まらなければ、R7-6 または別の差による(テスト以外の `lift`・`app`・`ci` の差は上の 6 ファイル(R7-6 の 3 + 0372 の 3)だけ。それ以外の差は、この観測では切り分けていない)。
  - (c) 混在なら、3 回の内訳をそのまま書く。
- 外れ得る。開示: main は R5-3i の結果と 0364 の読みを見ていた。

## 参考(旧の観測)
`evidence/R5-3c.md`・`R5-3i.md` の work-r25 台本 A の停止 3/3。work-r53h 台本 A の止まらず 3/3(R5-3i)。

## 未確認
- なぜ seed pack に 3 ファイルがある bundle だけがこの経路に入るか。
- `dwCoInit` の実際の値。
- `a03c903` と `4ba7ce1` の bundle の差が、R7-6 の 3 ファイルだけによるかは、wasm の比較と `manifest.json` の sha256 で確かめる(手順 5)。

## 結果の追記(2026-11-15、`evidence/R5-3j.md`)

- 組 1(work-r53j、`a03c903`。0372 なし・R7-6 あり): 3 回とも停止(定義 1・2)。`beaconExit.code` 134、`message` は `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]`。予測の (a) 3/3 停止に当たる。
- 組 2(work-r53h、`2f1915b`。0372 あり・R7-6 あり、対照): 3 回とも止まらなかった(定義 3)。`opErrors` は空。R5-3i の再現。
- 別の結果(定義 4、0372 の名前つき停止を含む): 0 件。
- n=3。この機械だけ。画像は観測の対象外(この回では PNG を開いていない)。
- wasm: work-r53j は work-r25 と `cmp` で同一(rc=0)。work-r53h とは異なる(rc=1)。
- 判断の規則は置いていない。次の作業の選択(0372 を main に入れるか、次の停止の解析)は別の決定文書で行う。
