# 0375: ole32!CoInitializeEx の応答(0372)を main に取り込む
分類: (b 外部 API の契約の不足を埋める)
撤回条件: 取り込み後の main の HEAD から作った bundle で台本 A(About を閉じる)を観測し、停止が同じ位置(`indirect call to 0 [guest return 0x1331c94]`)で続く、または別の停止が出る、または `cargo test` に既知の 1 件以外の新しい失敗が出る、または `engine-bundle.yml` が失敗する、のどれかなら、この取り込みを元に戻し、0372 の差分を外す。
日付: 2026-11-15   ROADMAP: R5-3   一覧: `evidence/R5-3i.md`・`evidence/R5-3j.md`・`evidence/R5-3k-cargo.txt`   取り込みの元: ブランチ `r5-3h-coinitializeex` の `2f1915b`(`a03c903` の上の 1 commit)

## 決定
決定: 0372 の差分のうち `lift/` の 12 ファイルを main に入れる。取り込みは `git cherry-pick -n 2f1915b` で行い、`docs/decisions/0372-coinitializeex-response.md` は main の版を残す(ブランチ版は `6f28a7b` と同一で、main の版には末尾に R5-3i の注が 1 段落ある)。
根拠: 分類 (b)(0372 のとおり)。取り込みの根拠にした観測は、0372 の有無だけが違う 2 つの bundle の結果が分かれたこと(各 n=3。台本 A。この機械。headless Chrome)。
- work-r53j(`a03c903`。0372 なし・R7-6 あり): 3/3 停止。`stopped: true`・`beaconExit.code` 134・`message` = `fle: indirect call to 0 is not a known function entry [guest return 0x1331c94]`(出典: `evidence/R5-3j.md` §7 の表)。
- work-r53h(`2f1915b`。0372 あり・R7-6 あり): 3/3 止まらなかった。`stopped: false`・`state: engine-idle`・`opErrors: []`(出典: `evidence/R5-3i.md` §8・`evidence/R5-3j.md` §7 の表)。
- 参考: work-r25(`4ba7ce1`。0372 なし・R7-6 なし)も 3/3 停止(出典: `evidence/R5-3c.md`・`evidence/R5-3i.md`)。
限界: n=3。条件 A(About を閉じる)だけ。この機械だけ。work-r53h の bundle はブランチ上で作った。この観測は、0372 の有無だけが違う組の結果が分かれたという事実までで、0372 がその原因だとは断定していない(`evidence/R5-3j.md` §7 の「切り分けの結論は書かない」を引き継ぐ)。
分類の理由: 0372 は Win32 API(`ole32!CoInitializeEx`)の応答を足すだけで、FL の挙動を補う・代行する処理は含まない(`docs/ops/fidelity-and-performance.md` §6)。

## 事実
- 取り込み前: `git diff --stat a03c903 HEAD -- lift app ci` は空(出力なし・終了 0)。main の HEAD は `1832b7e`。
- `git show --stat 2f1915b` の 13 件のうち、`docs/decisions/0372-coinitializeex-response.md` の 1 件は取り込まず、残りの `lift/` の 12 件を取り込んだ。`lift/` の 12 件は `lift/runtime/fle_ole.c`(+13)・`fle_ole.h`(+2)・`lift/crates/fe-lift/src/imports_table.rs`(+2)と、テスト 9 件(`oleaut_ole.rs`・`rows_0372/mod.rs`(新規)・`wall_fr4`・`wall_fr5`・`wall_fr13`・`wall_fr15`・`wall_fr16`・`wall_fr19`・`wall_fr20`)。
- 取り込みの衝突: `docs/decisions/0372-coinitializeex-response.md` の 1 件だけ(add/add)。`lift/` に衝突は無かった(`git cherry-pick -n` の出力)。0372 の文書は、`git show HEAD:docs/decisions/0372-coinitializeex-response.md` の内容(main の版)に戻した。取り込み後 `git diff HEAD -- docs/decisions/0372-coinitializeex-response.md` は空。
- 取り込み後(commit 前)の `cargo test --workspace --no-fail-fast`(`lift/`。`CARGO_TARGET_DIR` は `~/.cache/signature-studio/target-r5-3h`): `test result:` の行は 219 件。通過 2426・失敗 1・無視 1。失敗は `getmodulehandleexw_returns_in_every_build`(既知の 1 件。0353 の読み)。内訳と行は `evidence/R5-3k-cargo.txt`。
- `evidence/R5-3h-cargo.txt` は失敗の 1 行だけの抜粋で、合計を足せない(本書の時点で書き換えていない)。

## 未確認
- なぜ seed pack に 3 ファイルがある bundle だけがこの経路に入るか(0373・0374 の未確認を引き継ぐ)。
- FL が `CoInitializeEx` に渡す `dwCoInit` の値。
- 他の 5 つの名前(`CoCreateInstanceEx`・`CoAddRefServerProcess`・`CoReleaseServerProcess`・`CoResumeClassObjects`・`CoSuspendClassObjects`)の NULL の影響。
- 取り込み後の main の HEAD から作った bundle での観測(本取り込みの時点では未実施)。
- `engine-bundle.yml` の結果(本取り込みの push で run が起動したが、結果は本書の時点で未確認)。
- main の 0372 の文書の末尾の注(R5-3i)は「`rows_0372/mod.rs` は main に無い」と書くが、取り込み後は main に入るため、その注の記述は取り込み後の状態と合わない。0372 の文書は本取り込みでは書き換えていない。
