---
draft: "134"
title: "本番の engine の『検証済みの一覧』の門を廃止する: main の deploy は、常に、その engine を本番に出す"
type: Task
milestone: "M4 FL に必要な物の配布"
parent: "012"
blocked_by: []
labels: ["delivery"]
size: S
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の指示(原文): 「main が更新されたら、デプロイされるはず。デプロイを迂回する仕組みがあるとしたら、廃止してください」。

main の deploy は、常に、その engine を本番に出す。デプロイを迂回する仕組みは、残さない。

## 背景

- 事実(実機のビーコンで確定。`pm-work/findings/real-user-beacons-2026-11-24.md` の節 4・5): `fe-build upload`(`lift/crates/fe-build/src/main.rs` の `upload_gated_steps`)は、engine の sha256 が `lift/harness/web/verified.txt` の行に在る時だけ、本番の `manifest.json` を差し替える。無い時は、bundle のファイルは上がるが、manifest は替わらず、手順は落ちない(deploy は success になる)。
- 結果: 約 3 日間、engine の変更が本番に出なかった。実機の記録の版は b440e13a623a のまま。deploy の run は success だった。
- 940e08fd(暫定): deploy の中で、スモークが通った engine の sha を、作業ツリーの `verified.txt` に 1 行足す形に変えた(commit はしない)。これは暫定で、門そのものを廃止する(この Issue)。
- スモーク(`deploy.yml` の boot と welcome-flow の jq の段)が落ちると、deploy は失敗し、本番は替わらない。これは迂回ではなく、deploy の失敗なので、残す。

## 他の迂回の仕組み(既知)

1. `lift/harness/web/verified.txt`: 門の一覧。この Issue で削除する。
2. `lift/harness/web/root.txt`: 空のファイル(0 B)。非空なら、その内容の key を、本番の manifest にコピーする(`upload_gated_steps` の `root`)。本番の manifest を、この build 以外に替える経路。この Issue で削除し、経路を取り除く。
3. `.github/workflows/deploy.yml` の `paths`(`app/**`・`lift/**`・`ci/**`・`.github/workflows/deploy.yml`): これらを触らない push では deploy が走らない。`pm-work/`・`spec/`・`tools/`・`guard/` だけの変更は、製品の内容が変わらないので、迂回ではない(PM の判断)。`inputs/` と `tools/` は、製品の bundle の入力に入らない(`ci/extract-dlls.tsv` の注釈と `lift/crates/fe-build/tests/` の参照だけ)。作業者は、この事実を grep で確かめ、報告に書く。もし入る経路が見つかれば、`paths` に足す。
4. `deploy.yml` の host の鍵(`runs-on/cache` の `hashFiles('ci/**', 'app/scripts/**/*.mjs', 'lift/harness/seeds/vfs-seed-idle1.tsv')`): 鍵に入らない入力が変わると、古い取り出し物が使われる。作業者は、鍵の入力を全て列挙し、足りない物が在れば、報告に書く(鍵に足すのは、この Issue でよい)。
5. `lift/crates/fe-build/src/main.rs` の bundle の段(`if Path::new(OFFLINE_PAGE_SRC).is_file()`): ファイルが無い時、黙って入れない。作業者は、この分岐と、他に黙って省く分岐を列挙して報告する。直す場合は、この Issue では行わず、次の Issue の候補として書く。

## やること

1. `lift/crates/fe-build/src/main.rs`: `upload_gated_steps` と、その読み(`verified.txt`・`root.txt`)を取り除く。`upload()` は、常に、bundle のファイルを上げ(`upload_sync_steps`)、**最後に** `manifest.json` を差し替える(`s3 cp`)。順序は変えない(ファイル → manifest)。`verified` と `root` の語を、コードと注釈から取り除く。
   - 試験 `plans_match_recorded_texts` は、4 つの計画を一度に見る。upload の検査を、名前に `upload` を含む別の試験に分けてよい(engine の既存の失敗は範囲外のまま)。`lift/crates/fe-build/tests/plan/upload.txt` は、今の 3 段(sync 2 件と manifest の `cp`)と一致させる。
2. `lift/harness/web/verified.txt` と `lift/harness/web/root.txt` を、`git rm` で削除する。
3. `.github/workflows/deploy.yml`: 「検証済みにする」の段を取り除く。「本番の manifest を確かめる」の段は残す(deploy が、確かに本番の manifest を替えたことの確認)。他の段は変えない。
4. `spec/workflow.test.sh`: deploy の段の順の検査から `verify` の種類を外す。「検証済みにする」の検査(`(s)` の節)を、廃止の検査(`verified` の段が無い・manifest の確認の段が在る)に書き換える。`pm-work/workflow-design.md` の 14 節は、門を廃止した事実と理由を書く(静かに本番に出なかったこと)。
5. 迂回の仕組みを、上の既知の 5 件を含めて全て列挙する。grep: `git grep -nE 'verified|root\.txt|gate|skip|allowlist|pinned' -- .github/workflows/deploy.yml lift/crates/fe-build app/worker app/src/bundle`。列挙した物のうち迂回に当たる物はこの Issue で取り除き、当たらない物は理由を報告に書く。

## 受け入れ

```accept
cargo test --locked --manifest-path lift/Cargo.toml -p fe-build upload && bash spec/workflow.test.sh && ! git grep -qF 'verified.txt' -- lift .github guard spec && ! git grep -qF 'root.txt' -- lift .github guard spec && bash guard/deploy-lint.sh
```

判定の基準:
- `cargo test` の upload の試験が通る。`spec/workflow.test.sh` が通る。
- `lift`・`.github`・`guard`・`spec` の追跡ファイルに、`verified.txt` と `root.txt` の語が無い(`git grep` が 0 件)。
- `guard/deploy-lint.sh` が通る。
- 報告に、迂回の列挙の結果(上の 5 件の扱い)を書く。

この Issue の push で、deploy が起動する。作業者は、deploy の run の完了を待たない。報告には、次を書く: push した commit の SHA、deploy が起動したこと。run の結論(success か)と、「本番の manifest を確かめる」の段の結果は、作業者には確認できないので、「確認できない」と書く。

## 注意

- この Issue の後、main の deploy は、スモーク(起動・Welcome の流れ)が通った engine を、常に本番に出す。スモークが落ちれば、deploy は失敗し、本番は替わらない(今のまま)。
- `plans_match_recorded_texts` の既存の失敗(`lift/crates/fe-build/tests/plan/engine.txt` の rt 91 と 90。f4cffe71 以来)は、この Issue の範囲外。
- `#1374`(fe-build の blend2d の取得)が、同じ `lift/crates/fe-build/src/main.rs` を変える。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。

## 読む物

- `lift/crates/fe-build/src/main.rs`(`upload_gated_steps`・`upload`・`upload_sync_steps`・テストの `plans_match_recorded_texts`)
- `lift/crates/fe-build/tests/plan/upload.txt`
- `.github/workflows/deploy.yml`(「検証済みにする」「本番の manifest を確かめる」)
- `spec/workflow.test.sh`(deploy の検査。`(s)` の節)
- `pm-work/workflow-design.md` の 14 節、`pm-work/findings/real-user-beacons-2026-11-24.md` の節 4・5

## 制約

- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- deploy の段に、迂回のための分岐(オプトイン・許可リスト・固定)を足さない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。
- Python は製品に使わない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通らなくても、作業ツリーを commit し、報告を書いて終える(ジョブの上限は 5 時間 50 分)。
- engine のビルドは要らない。`cargo test -p fe-build` は fe-build だけを作る。初回のコンパイルは数分かかるので、シェルツールの `timeout` 引数に大きい値(例 3600000)を渡して待つ。
- 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。
