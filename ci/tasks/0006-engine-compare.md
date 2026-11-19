# 0006: 候補のエンジン(46a72ee9)と参照(b440)を、同じ起動の測定で比べる(R6-46。測定のみ)

## 報告の先頭の規則
- 報告の 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。実行していなければ「実エンジン: 実行なし(理由)」と書く。
- 測定の値は「runner の値。この機械とは別枠」と書く。runner の種類(meta.txt の runner・nproc・loadavg)を添える。
- 代役・再生・事前に作った bundle を使わない。数字は、この実行で得た実測だけを書く。各数字に、出典の出力の行を付ける。
- 「同一」「一致」は、sha256 の値が出力の行で一致したときだけ書く。判定は「受理」「差し戻し」「未確認」のいずれかにする。

## 先頭の注意(記述の所在と、候補の木)
- 正式なエンジン名の記述は、このワークツリーの docs と evidence と scripts のどこにも無い(確認済み: docs・evidence・scripts を grep して 0 件。R6-46 の記録ファイル evidence/R6-46.md も無い)。したがって、この下書きでは、エンジンを sha256 の値で呼ぶ。
- 候補: sha256 46a72ee9638744f0299cf0a7b2d70e97ac591bb98cb4898a34cf85240c105da5。木は commit 1c485231(R6-43 の木。main の転送による。ワークツリーの記録では確認していない)。emsdk は ci/versions.env の EMSDK_VERSION。
- 参照: lift/harness/web/verified.txt の値 b440e13a623aa949ccce21db53566658425743bb2b21263bcf773d278396e043(確認済み)。この値の木が何か、repo の記録では確認できない(未確認)。
- 候補の木 1c485231 が、この clone の履歴から届くかを、最初に確かめる(届かなければ「実エンジン: 実行なし(候補の木が届かない)」と報告して止まる)。

## 目的と根拠
R6-46 の計画(候補 46a72ee9 と参照 b440 の比較)を、runner で行う。比べる項目は、0003 と同じ起動の測定(P1)と、About・再生 10 秒・Problems の窓・Export の WAV の sha256 とする。道具は、0003 の boot-bundle.mjs と、tools/verify-kit の既存の道具。

## 読む物(実在を確認済み)
- lift/harness/web/verified.txt(参照の値)
- tools/boot-check/boot-bundle.mjs(起動の測定。引数は bundle ディレクトリの絶対パス。JSON の 1 行を出す)
- tools/verify-kit/tools/r635-run.sh(R6-35 の比較の 1 回の列の道具。Export と WAV の保存を含む)
- lift/crates/fe-build/src/main.rs(子コマンド engine と bundle。bundle の出力先は lift/out-app/bundle/raw/fl-Oz)
- ci/versions.env(EMSDK_VERSION。grep で読む)

## やること
1. 候補の木が届くかを確かめる(先頭の注意)。届かなければ止めて報告する。
2. 候補の木を作る。git archive 1c485231 を一時の場所に展開し、そこに抽出物(lift/out-app/host)を置いて、npm ci --prefix app、cargo build --release --locked --manifest-path lift/Cargo.toml、lift/target/release/fe-build engine と bundle を実行する。engine の sha256 を出力の行で書く。値が 46a72ee9 と一致しなければ、その値を書いて止める(比較はしない)。
3. 参照を作る。現在の木(HEAD)で、同じ手順でビルドし、engine の sha256 を書く。b440 と一致するかを、出力の行で書く(一致しなければ「HEAD の木は b440 と違う」と書く)。
4. 起動を測る(P1)。両方の bundle に対して、node tools/boot-check/boot-bundle.mjs の絶対パスを渡し、各 n=3 で走らせる。timelineIdle、state、engineKind、error、standIn を、JSON の行のまま残す。standIn が null でない回は「代役」と書き、数字に入れない。
5. Export の WAV の sha256 を、tools/verify-kit/tools/r635-run.sh の使い方に従って両方で取る。sha256 の値を出力の行で書く。
6. 判定する。両方の起動が error 無しで first idle に達し、Export の sha256 が一致したときだけ「受理」と書く。違えば「差し戻し」と書き、違った項目と値の行を書く。道具の一部が無ければ「未確認」と書く。

## 出力
- ci/agents/out/0006-engine-compare/findings.md に、手順 1 から 6 の結果を日本語で書く。
- ci/agents/out/0006-engine-compare/boot-runs.jsonl に、起動の JSON の行を、両方の bundle について、そのまま保存する。
- 最終の応答の 1 行目は「実エンジン: …」で始める。

## 禁止と注意
- 変更は作らない。ci/agents/out/0006-engine-compare/ の外のファイルを変えない。lift/harness/web/verified.txt とルートの bundle は変えない。commit も push もしない。
- .git/config、opencode.jsonc、.opencode を読まない。git の内部命令(update-index・hash-object・update-ref・commit-tree・write-tree・bundle・for-each-ref・refs を含む命令)を使わない。git archive と git show は読みだけに使ってよい。force は使わない。
- Secrets と Variables は読まない。wrangler と cf は使わない。R2 の鍵はこの実行には無い。
- ホストの絶対パス、アカウント ID、鍵、URL のホスト名を書かない。パスは変数名か repo 内の相対パスで書く。
- sleep、timeout、nohup、末尾の & は使わない。
