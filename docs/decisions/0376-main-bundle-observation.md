# 0376: 取り込み後の main の HEAD(`9610708`)から作った bundle で、About を閉じた後の停止を観測する(R5-3l)

分類: 測定(lift の処理を足さない。(a)(b)(c) の対象外。ソースは変えず、bundle の作成元の commit を main の HEAD にするだけ)。撤回条件: この観測の結果を、0375 の取り込み(0372 の main への取り込み)を残すか戻すかの選択にだけ使う。別の窓・別の道具・別の bundle の結果と混ぜて判断しない。

日付: 2026-11-15   依頼: R5-3l(main が付けた名。ROADMAP の行は変えない)   前コミット: `9610708`(取り込み後の main の HEAD)   観測対象: `9610708`(main。ブランチは作らない。ワークツリーも作らない)

## 決定
決定: main の HEAD `9610708` から 9 段のビルドで bundle(`~/.cache/signature-studio/work-r53k/bundle/fl-Oz`)を作り、work-r25 と並べて、本書の定義どおりに台本 A で観測する。同時に、取り込みの push(`d2d90a9`・`ab3f818`・`9610708` のどれか)で起動した GitHub Actions の `engine-bundle.yml` の run の結果を、読み取りだけで記録する。判断の規則は置かない(観測の記録)。根拠: 0375 で取り込んだ 0372 が、main の HEAD から作った bundle でも台本 A の停止を止めるかを、観測で確かめるため。次の作業の選択は、結果の後に別の決定文書で、規則を先に書いてから行う。

## 問い
1. 取り込み後の main の HEAD(`9610708`)から作った bundle(work-r53k)が、台本 A で止まらなくなっているか。
2. `engine-bundle.yml` の run(取り込みの push で起動したもの)は成功したか。

背景:
- `git diff --stat a03c903 HEAD -- app ci` は空(`app/`・`ci/` は `a03c903` から変わっていない)。
- `git diff --stat a03c903 HEAD -- lift` は 12 ファイル(0372 の 3 ファイル + テスト 9 ファイル。取り込み commit `d2d90a9`)。
- したがって main の HEAD のソースは work-r53h(`2f1915b`)と同じ(`lift`・`app`・`ci`)と見込まれる。wasm のバイトが work-r53h と同じになるかは、ビルド後に `cmp` で確かめる(手順 4)。同じにならない場合は事実として書く。
- 観測の前例: `evidence/R5-3i.md`(work-r53h は 3/3 止まらず。work-r25 は 3/3 停止)、`evidence/R5-3j.md`(work-r53j(`a03c903`)は 3/3 停止。work-r53h は 3/3 止まらず)。
- main は R5-3i・R5-3j の結果を見ていた(開示)。

## 定義(観測の前に固定。0373・0374 と同じ)
1. 「停止した」= その run の `report.json` が `stopped: true` かつ `state: exited` かつ `beaconExit.code` が 134。
2. 「同じ停止」= `beaconExit.message` が `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]` と一致。
3. 「止まらなかった」= `stopped: false` かつ `state: engine-idle` かつ `opErrors` が空(台本が最後まで届く)。
4. それ以外は「別の結果」とし、`message` と `code` をそのまま書く。`message` に `dwCoInit is not COINIT_APARTMENTTHREADED` を含む停止(0372 の名前つき停止)も「別の結果」として区別して書く。bundle が配れない・manifest の検証で失敗する場合は「配れなかった」とし、原因を書いて止める。

CI の結果の定義: `gh run view <id> --json status,conclusion` の `conclusion` が `success` か `failure` か、その他(`cancelled` など)か。進行中なら「進行中」と書き、完了まで待つ(観測の後に `gh run watch <id> --exit-status`。ビルドの観測と並行しない)。

## 組(各 n=3。直列。台本 A = `tools/verify-kit/tools/scn-r53-about-ja.json`)
- 組 1: work-r53k(main の HEAD `9610708` から作った bundle)。label `r53knew`。
- 組 2(対照): work-r25(`~/.cache/signature-studio/work-r25/bundle/fl-Oz`。0372 なし・R7-6 なし。R5-3c・R5-3i で 3/3 停止を観測済み)。label `r53kold`。
- 順: 組 1・組 2・組 1・組 2・組 1・組 2 を交互に 1 回ずつ。
- work-r53h は前回の観測済みのため走らせない(wasm の比較は手順 4 で行う)。

## 静かさ
0373・0374 と同じ。各回の直前に `node tools/verify-kit/tools/wait-quiet.mjs 8 10`(rc を記録)と `/proc/loadavg` を記録する。`wait-quiet` の 2 秒の swap の入出力が 0 でない・1 分 loadavg が 8 超・I/O some avg10 が 0 でない回は数えず、同じ位置で撮り直す(最大 3 回の試行)。

## 予測(観測の前)
- 組 2(work-r25): 3 回とも同じ停止(R5-3c・R5-3i の再現)。
- 組 1(work-r53k): main の予測は (a) 3/3 止まらず。根拠: work-r53h と同じソース(`lift`・`app`・`ci`)で、R5-3i で 3/3 止まらなかったため。
  - (a) 3/3 止まらず: 取り込み後の main の bundle でも、About を閉じた後の停止は止まる(R5-3i の再現)。
  - (b) 3/3 停止: 同じ停止が続く。main の HEAD と work-r53h の差が、wasm・bundle の作り方のどこかにある(未確認)。
  - (c) 混在: 3 回の内訳をそのまま書く。
  - (d) 別の結果: `message` と `code` をそのまま書く。
- 外れ得る。開示: main は R5-3i・R5-3j の結果を見ていた。

## 参考(旧の観測)
`evidence/R5-3c.md`・`R5-3i.md`・`R5-3j.md` の対照。work-r25 の台本 A の停止 3/3。work-r53h の台本 A の止まらず 3/3(R5-3i)。work-r53j(`a03c903`)の台本 A の停止 3/3(R5-3j)。

## 未確認
- なぜ seed pack に 3 ファイルがある bundle だけがこの経路に入るか。
- `dwCoInit` の実際の値。
- 他の 5 つの名前(`CoCreateInstanceEx` など)の NULL の影響。
- work-r53k と work-r53h の wasm が同じバイトになるかは、ビルド後に `cmp` で確かめるまで未確認。
- CI の `SHA256SUMS` の wasm の sha256 と、ローカルのビルドの wasm の sha256 が一致するかは、CI の結果を読むまで未確認(一致しなくても、ビルド環境の違いで想定内)。

## 結果の追記(2026-11-15、`evidence/R5-3l.md`)

- 組 1(work-r53k、`9610708`。0372 あり・R7-6 あり): 3 回とも止まらなかった(定義 3)。`opErrors` は空。予測の (a) 3/3 止まらず に当たる。
- 組 2(work-r25、`4ba7ce1`。対照): 3 回とも同じ停止(定義 1・2)。`beaconExit.code` 134、`message` は `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]`。予測どおり。
- engine-bundle(run `43687019712`、headSha `9610708`): `conclusion` は `success`(`gh run view` の出力)。wasm の sha256(全桁)は、CI の `SHA256SUMS` とローカルの work-r53k で一致。
- 別の結果(定義 4): 0 件。
- n=3。この機械だけ。判断の規則は置いていない。次の作業の選択は別の決定文書で行う。
