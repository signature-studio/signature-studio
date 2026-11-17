# 0023: 私の docs だけの push が、別の worker の未検証の commit(6fb75fbe・0322bfec)を範囲に入れ、deploy の run 43944303574 を起動させた。未検証の engine b440e13a623a が R2 の manifest に載った

- 日付: 2026-11-17
- 誰が: worker(push 1df9827b の作成者。本記録の作者)。取り込まれた commit の作者は別の worker(R5-6a。commit の作者欄は signature-studio agent)。
- 依頼者の発言: なし。main からの指摘もない。run 43944303574 のログの upload 行を読んで、自分で気づいた(2026-11-17)。

## 何が起きたか

- 1df9827b(docs/failures/README.md と 0022 の文のみの commit)を push した。その時の `git pull` で、origin の main にあった 2 commit が入った: `0322bfec`(plain-units.txt に 4 件)と `6fb75fbe`(`ci/extract-dlls.tsv`・`modules.tsv`・`vfs-seed-idle1.tsv`・`lift/harness/web/pack.args` に fruity_delay2 を 1 件ずつ。commit メッセージは「bundle と実測は未了(doing)」)。`git log --oneline e63f7770..1df9827b -- lift app/scripts ci .github/workflows/deploy.yml` の 2 件。
- push の範囲に `lift/**` と `ci/**` の変更が入り、deploy.yml の paths に当たったため、run 43944303574(headSha 1df9827b)が起動した。
- run 43944303574 は success。upload 行: `engine.b440e13a623a.wasm`(165,776,779 B。sha256 b440e13a623a…)、`dll.bce3049ad4d1.flpack`(11 件。88,675,800 B)、`seed.d6d345ed0b46.flpack`、`engine-glue.b20cb7b500e2.js`、`manifest.json` を `s3://fl-web-assets/fl-Oz/manifest.json` に上書き。wrangler-action の deploy も success。
- 先の run 43933717646(e63f7770。engine e5fecd8d93d5、manifest は 9 件の版)は、この run より前に完了していた。この run の manifest で、R2 の `fl-Oz/manifest.json` の参照が e5fe から b440 に移った。

## 原因(その時の判断)

- push の前に、`git log HEAD..origin/main` で取り込む commit を確かめなかった。`git pull` で入った commit が lift/・ci/ を変えていて、deploy を起動する範囲だと気づかなかった。
- 6fb75fbe の commit メッセージが「bundle と実測は未了」と書いていた。その commit を取り込んだ先で deploy されると予測しなかった。
- 6fb75fbe の push 自体では deploy が起動しなかった(run 一覧に 6fb75fbe の headSha が無い)。その理由は未確認。

## 影響

- 未検証の engine(b440e13a623a)と DLL pack(11 件)が、R2 の manifest に載った。公開側(Worker)の実際の参照と、この engine の P1〜P3・Export は未確認。
- 公開物(R2 の fl-Oz/manifest.json)が、基準の e5fe から b440 に変わった。変わる前に main は確かめていない。

## 是正

- 未了。取り消し(e5fe の bundle に戻す)か、取り込みを続けるか(b440 の検証)は main の判断。自分では R2 の manifest を直接書き換えない(AGENTS §4)。
- 本記録は main に報告する(最終応答)。

## 再発防止

- 文書: push の前に `git fetch` と `git log --oneline HEAD..origin/main -- lift app/scripts ci .github/workflows/deploy.yml` を実行し、結果が空でなければ push せず main に報告する。
- 検査(未実施): pre-push で、push の範囲に deploy の paths の変更があるなら、その commit の一覧を表示して止める。案のみで、実装していない。

## 同じ型の探索

- 範囲: 自分の push(1df9827b と 594ef4a7)の範囲。`git log --oneline e63f7770..594ef4a7 -- lift app/scripts ci .github/workflows/deploy.yml` の件数は 2(上の 2 件)。
- 594ef4a7(docs のみ)の run は無い(`gh run list --workflow=deploy.yml` で確かめた。headSha 594ef4a7 の行は 0 件)。
- e63f7770 以降の deploy の run は 2 件: 43933717646(e63f7770。success)と 43944303574(1df9827b。success)。

## 未確認

- 公開側(Worker と R2)の実際の参照。
- b440e13a623a の中身(起動・P1〜P3・Export)。
- 6fb75fbe の push で deploy が起動しなかった理由。
- 追記(2026-11-17。main の判断で取り消さず続ける): deploy.yml は root の manifest.json を書かず、prefix fl-Oz/ だけに書く(決定 0400 の『root の差し替えは deploy.yml に置かない』。evidence/R6-30.md §4 の『root は差し替えていない』)。本番の既定の入口(root)は R6-7 で置いた fl-engine-ci-d5d440fa を指すままで、この run の fl-Oz/ の書き込みの影響を受けない(文書から読んだ。R2 は読んでいない)。
- 追記 2(2026-11-17。main の判断で続ける): 164c8ed4(R5-6c の seed 無しの版)の engine は、Problems で止まる(0x8806f80 = kernel32!RaiseException の thunk を間接 call)。その版は origin に入り、run 43961833150(headSha fa751e1a)が fl-Oz に載せた(文書から。R2 は読んでいない)。修正は e27ad00f(seed 18 件。これだけでは止まる)と 48ccf733(`--seh-handler 0x10490`。止まらない。Soundgoodizer は開けない)。root は触っていない(文書から)。
