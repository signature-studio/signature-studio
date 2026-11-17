# 0373: CoInitializeEx の応答(0372。ブランチ 2f1915b)を足した bundle で、About を閉じた後の停止を実エンジンで観測する(R5-3i)

分類: 測定(lift の処理を足さない。(a)(b)(c) の対象外。0372 の差分はブランチ `r5-3h-coinitializeex` にあり、main には入れていない)。撤回条件: この観測の結果を、0372 の差分を main に入れるかの選択と、次の停止の解析の選択にだけ使う。別の窓・別の道具・別の bundle の結果と混ぜて判断しない。

日付: 2026-11-15   依頼: R5-3i(main が付けた名。ROADMAP の行は変えない)   前コミット: `6f28a7b`(0372 を main に置いた commit)   実装: ブランチ `r5-3h-coinitializeex` の `2f1915b`

## 決定
決定: ブランチ `2f1915b` から 9 段のビルドで bundle(`~/.cache/signature-studio/work-r53h/bundle/fl-Oz`)を作り、本書の定義どおりに台本 A で観測する。対照として work-r25 を同じ台本で走らせる。判断の規則は置かない(観測の記録)。根拠: 結果を見てから基準を決めると、後から基準を動かす余地が生まれるため。次の作業の選択(0372 を main に入れるか、次の停止の解析)は、結果の後に別の決定文書で、規則を先に書いてから行う。

## 問い
0372 の `ole32!CoInitializeEx` の応答を足した bundle で、台本 A(About を開いて Escape で閉じる)が止まらなくなるか。
- 背景: `evidence/R5-3c.md`・`R5-3d.md`(0370・0371)で、台本 A の停止は work-r25 など 7 bundle で 3 回中 3 回、About を閉じた直後に `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]` で起きた。`evidence/R5-3e.md`〜`R5-3g.md` の静的な読みでは、停止位置 `0x1331c80` は `call [rax]` で、`rax` の指す cell は `GetProcAddress("ole32.dll", "CoInitializeEx")` の戻り値を入れる。現在の応答表に `CoInitializeEx` の行が無く、`fle_imp_GetProcAddress` は NULL を返す。原因は断定しない。

## 定義(観測の前に固定。0370 の定義 1〜4 と同じ)
1. 「停止した」= その run の `report.json` が `stopped: true` かつ `state: exited` かつ `beaconExit.code` が 134。
2. 「同じ停止」= `beaconExit.message` が `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]` と一致。
3. 「止まらなかった」= `stopped: false` かつ `state: engine-idle` かつ `opErrors` が空(台本が最後まで届く)。
4. それ以外は「別の結果」とし、`message` と `code` をそのまま書く。特に **`fle_imp_CoInitializeEx` の名前つき停止**(`message` に `dwCoInit is not COINIT_APARTMENTTHREADED` を含む)は「別の結果(0372 の名前つき停止)」として区別して書く。bundle が配れない・manifest の検証で失敗する場合は「配れなかった」とし、原因を書いて止める。

## 組(各 n=3。直列。台本 A = `tools/verify-kit/tools/scn-r53-about-ja.json`)
- 組 1: work-r53h(ブランチ `2f1915b` から作った bundle)。label `r53hnew`。
- 組 2(対照): work-r25(`~/.cache/signature-studio/work-r25/bundle/fl-Oz`)。label `r53hold`。
- 順: 組 1・組 2・組 1・組 2・組 1・組 2 を交互に 1 回ずつ。

## 静かさ
各回の直前に `node tools/verify-kit/tools/wait-quiet.mjs 8 10`(rc を記録)と `/proc/loadavg` を記録する。基準は 0367・0370 と同じ(`wait-quiet` の 2 秒の swap の入出力が 0 でない・1 分 loadavg が 8 超・I/O some avg10 が 0 でない回は数えず、同じ位置で撮り直す。最大 3 回の試行)。この観測の結果は「停止の有無」なので、静かさへの敏感さは低いと見込むが、これは読みで、未確認。

## 予測(観測の前)
- 組 2(work-r25): 3 回とも同じ停止(R5-3c の再現)。
- 組 1(work-r53h): (i) 止まらなくなる、(ii) 同じ停止が続く、(iii) 別の停止(0372 の名前つき停止を含む)、の 3 通りのうちどれとも言わない。main は FL が `CoInitializeEx` に渡す `dwCoInit` の値を知らない。
- 観測の前の読み: 0372 が停止位置の原因を直したなら (i) か (iii)。(ii) なら、応答表に足した行が使われていない(`GetProcAddress` の引数・DLL 名の大文字小文字・`jmp_imp` 名など)を疑う。

開示: main は R5-3c〜R5-3h の結果を見ていた。

## 参考(旧の観測)
`evidence/R5-3c.md`・`R5-3d.md` の work-r25 台本 A の停止 3/3。

## 未確認
- なぜ seed pack に 3 ファイルがある bundle だけがこの経路に入るか(0372 で止まらなくなっても、seed との関係は説明されない)。
- FL が渡す `dwCoInit` の値。
- 他の 5 つの名前(`CoCreateInstanceEx` など)の NULL の影響。

## 結果の追記(2026-11-15、`evidence/R5-3i.md`)

- 組 1(work-r53h、ブランチ `2f1915b`): 3 回とも止まらなかった(定義 3)。`stopped: false`・`state: engine-idle`・`opErrors: []`。
- 組 2(work-r25、対照): 3 回とも同じ停止(定義 1・2)。`beaconExit.code` 134、`message` は `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]`。
- 別の結果(定義 4、0372 の名前つき停止を含む): 0 件。
- n=3。この機械だけ。画像は目視。
- 組 1 と組 2 は、ソースの版が `4ba7ce1` と `2f1915b` で違う(109 件・498 files)。0372 だけの効果かは、この観測では切り分けていない(未確認)。
- 判断の規則は置いていない。次の作業の選択(0372 を main に入れるか、次の停止の解析)は、別の決定文書で行う。
