# 0370: About を閉じた後の停止を、bundle ごと・台本ごとに実エンジンで観測する(R5-3c)

分類: 測定(lift の処理を足さない。(a)(b)(c) の対象外)。撤回条件: この観測の結果を、停止の原因の切り分けの次の作業の選択にだけ使う。別の窓・別の道具の結果(`evidence/R5-3.md` の旧の値など)と混ぜて判断しない。

日付: 2026-11-15   依頼: R5-3c(main が付けた名。ROADMAP の行は変えない)   前コミット: `6ea627f`

## 決定
決定: 観測は本書の定義どおりに行う。判断の規則は置かない(観測の記録)。根拠: 結果を見てから基準を決めると、後から基準を動かす余地が生まれるため。次の作業の選択は、結果の後に別の決定文書で、規則を先に書いてから行う。

## 問い
R5-3a(`evidence/R5-3.md`)で、work-r25 の bundle は About タブを Escape で閉じた直後に停止した(3 回中 3 回。message は `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]`)。E13 は 3 回中 0 回。この停止が (1) どの bundle から起きるか、(2) About タブに限るか、を測る。原因は断定しない。

## 台本と道具
- 台本 A(`tools/verify-kit/tools/scn-r53-about-ja.json`): 閉じる → F10 → About タブ → Escape → 所定プロジェクト(`openZipProject`)を開く。
- 台本 B(`tools/verify-kit/tools/scn-r53c-settings-close.json`、新規): 閉じる → F10 → Settings の既定のタブのまま → shot → Escape → listWindows。About タブへは移らず、所定プロジェクトも開かない(対照)。
- 道具: `tools/verify-kit/serve-once-r53-bundle.mjs`(台本 A)と `tools/verify-kit/serve-once-r53c-bundle.mjs`(新規。台本のパスの 1 行だけが違う。`diff` で確認)。
- 台本 B は `scn-lint.mjs` を通す(rc 0)。

## 定義(観測の前に固定)
1. 「停止した」= その run の `report.json` が `stopped: true` かつ `state: exited` かつ `beaconExit.code` が 134。
2. 「同じ停止」= `beaconExit.message` が `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]` と一致。
3. 「止まらなかった」= `stopped: false` かつ `state: engine-idle` かつ `opErrors` が空(台本が最後まで届く)。
4. それ以外(別の message・別の code・台本の失敗)は「別の結果」とし、message と code をそのまま書く。
- この決定は停止の有無だけを対象にする。ロゴ・staff roll・日本語の目視は本書の対象外(`evidence/R5-3.md` の補いの範囲)。

## 組(各 n=3。直列。次の表の上から、1 組ずつ、3 回続けて)

| 順 | 台本 | bundle | label | HEAD(manifest の description) | wasm sha256 頭 16 桁 | seed pack |
|---|---|---|---|---|---|---|
| 1 | A | work-r25 | `r53cwr25` | 4ba7ce1 | 921bb7e2ed1ad326 | 7cd2eaec(228 件) |
| 2 | A | work(base) | `r53cbase` | 818b5b0 | 0286bdf1827f5930 | 7cd2eaec(228 件) |
| 3 | A | work-r21 | `r53c21` | 46e2d1a | 0c9ea9ee7a7d3ebf | 7cd2eaec(228 件) |
| 4 | A | work-r22 | `r53c22` | bd9a3fe | 135bab8618a1a795 | 7cd2eaec(228 件) |
| 5 | A | work-r23 | `r53c23` | ca9ef9d | 04d71ca25954b090 | 7cd2eaec(228 件) |
| 6 | A | work-r24 | `r53c24` | f1635bf | 619954341286f4a8 | 7cd2eaec(228 件) |
| 7 | A | work-r31 | `r53c31` | abbb315 | 5d9c82ddd4b540cc | 7cd2eaec(228 件) |
| 8 | A | E13 | `r53ce13` | d0447902 | 58ad4a6aa720375a | f7726113(225 件※) |
| 9 | B | work-r25 | `r53csetwr25` | 4ba7ce1 | 921bb7e2ed1ad326 | 7cd2eaec(228 件) |
| 10 | B | E13 | `r53csete13` | d0447902 | 58ad4a6aa720375a | f7726113(225 件※) |

- bundle の場所: work-r25・base・r21〜r24・r31 は `~/.cache/signature-studio/<名前>/bundle/fl-Oz`(base は `work`)。E13 は `~/work/signature-studio/artifacts/bundles/fl-engine-ci-0363b050`。
- seed pack の sha 頭は各 manifest の `seedPack.sha256` で確かめた(`expire_grep`)。件数は `evidence/R2-7c.md` と main の記述による値で、この観測では数えていない(※ 未確認)。
- 同じ道具で、label は `r53-<label>-chrome-<N>`(N は既存の最大 + 1)で出力する(`tools/verify-kit/out/`)。

## 静かさ
- 各回の直前に `node tools/verify-kit/tools/wait-quiet.mjs 8 10` を実行し、rc を記録する。道具の実行後に `/proc/loadavg` を記録する。
- 基準は 0367 と同じ(swap の入出力が 0 でない・1 分 loadavg が 8 超・I/O some avg10 が 0 でない回は数えず、同じ位置で撮り直す。最大 3 回の試行)。rc が 1 の回は道具を起動せず、同じ位置で撮り直す。
- 未確認: この観測の結果(停止の有無)が静かさに左右されるか。本書は静かさに敏感だと仮定していない。

## 予測(観測の前に書く)
- 台本 A の work-r25: 3 回とも同じ停止(R5-3a と同じ)。
- 台本 A の work(base): 3 回とも同じ停止(seed pack が 228 件で、3 ファイルがある)。
- 台本 A の work-r21・r22・r23・r24・r31: seed pack が 228 件なので、3 回とも同じ停止。
- 台本 A の E13: 止まらない(R5-3a で 3 回中 0 回)。
- 台本 B の work-r25: About タブに行かないので、止まらない。
- 台本 B の E13: 予測は書かない(未予測)。
- 開示: main は R5-3a の結果と seed pack の件数(E13 225・work-r25 228)を見ていた。予測は外れ得る。

## 参考(旧の別の観測。判定に使わない)
- `evidence/R5-3.md` の work-r25 の停止 3/3、E13 の 0/3(同じ道具・同じ台本 A)。

## 未確認(観測の前の時点)
- 停止の原因(API・ゲストの関数・seed 資材との関係)。
- 台本 B で About を経由しない場合の停止の有無。
- 静かさの基準が停止の有無に効くか。
- 実機との比較は 0 件(H6)。

## 出典
- `evidence/R5-3.md`(R5-3a の結果と目視の補い)。
- `docs/decisions/0369-seed-assets-about-ja.md`(書式と seed の定義)。
- `docs/decisions/0367-p3-e13-vs-r25.md`(静かさの基準)。
- 各 bundle の `manifest.json`(`bundle.description`・`engine.wasm.sha256`・`seedPack.sha256`)。

## 結果の追記(2026-11-15、`evidence/R5-3c.md`)

- 定義の適用(組ごと。n=3):
  - 台本 A: work-r25 停止 3/3、work(base) 停止 3/3、work-r21 停止 3/3、work-r22 停止 3/3、work-r23 停止 3/3、work-r24 停止 3/3、work-r31 停止 3/3、E13 止まらなかった 3/3。
  - 台本 B: work-r25 止まらなかった 3/3、E13 止まらなかった 3/3。
- 停止の message と code は、停止した 24 回すべてで定義 2 の文言と 134。別の結果は 0 件。
- 台本 B の work-r25 の 1 回目は、誤って台本 A の道具で走った(記録に明記。組の数には入れない)。
- 予測との比較: 当たり 5 組(台本 A の 7 bundle を 5 組で数えた行と、台本 A の E13、台本 B の work-r25)。外れ 0。台本 B の E13 は未予測。
- 「n=3。この機械だけ(headless Chrome)。実機との比較は 0 件(H6)」。
- 判断の規則は置いていない。次の作業の選択は別の決定文書で行う。
- 上の予測との比較の訂正: 予測のある 9 組(台本 A の 8 組と、台本 B の work-r25)はすべて当たり。外れ 0。台本 B の E13 は未予測(数えない)。上の「当たり 5 組」は誤り。
- 上の「停止した 24 回」は誤り。台本 A の組 1〜7 の停止は 21 回。誤った道具で走った 1 回を足して 22 回。message と code は、この 22 回すべてで定義 2 と一致(134)。詳細は `evidence/R5-3c.md` の「数えの訂正」。
