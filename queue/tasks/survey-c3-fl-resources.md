# survey-c3-fl-resources: C-3(言語パック・テーマ・プロジェクトテンプレートなど、FL Studio 自体が必要とする物)の現状を、この runner で数える(測定のみ。製品は変えない)

## 報告の先頭の規則
- 報告の 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。実行していなければ「実エンジン: 実行なし(理由)」と書く。
- 数字は、この実行で得た実測だけを書く。各数字に、出典のコマンドと出力の行を付ける。推定は「推定」と書き、実測と分ける。
- 「達成」「揃った」と書かない。判定は「受理」「差し戻し」「未確認」のいずれかにする。

## 規則の要約(この task で守ること)
- 製品(app/・lift/・ci/・tools/・patches/・.github/)を変えない。変更は queue/findings/survey-c3-fl-resources.md だけ。
- Python は使わない。ベンダー名、Cloudflare のアカウント ID・ホスト名・ETag(32 桁の 16 進)、絶対パスを書かない。パスは repo の中の相対パスで書く。
- commit は環境の git の設定のまま行う(git config・--author・GIT_AUTHOR_*・git -c を使わない)。commit に Claude の行を付けない。ブランチを作らない。push はしない(判定の段が push する)。
- .git/config、opencode.jsonc、.opencode を読まない。git の内部命令(update-index・hash-object・update-ref・commit-tree・write-tree・bundle・for-each-ref を含む)を使わない。force は使わない。
- Secrets と Variables は読まない。wrangler と cf の CLI は使わない。R2 の鍵はこの実行には無い。
- sleep、timeout、nohup、disown、末尾の & は使わない。
- 一時ファイルは repo の中の .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。

## 目的と根拠
台帳 L-033 の (3): 「言語パック、テーマ、プロジェクトテンプレートなど FL Studio 自体が必要とする諸々は全く揃っていない」。queue/plan.md の C-3 を参照。この task は、その現状を、seed の表とこの runner の抽出の木の突き合わせで数える。インストーラの全件の一覧は runner に無いので、そのことも記録する。

## 読む物(実在を確認済み)
- lift/harness/seeds/vfs-seed-idle1.tsv(247 行。1 行は「guest のパス TAB 抽出先 TAB SHA-256」。抽出先が「-」の行はディレクトリ)
- ci/extract-dlls.tsv(DLL の表)
- app/scripts/extract-fl.mjs(25 行: rows の読み。72〜91 行: seed の行の処理。抽出先は HOST_ROOT の下に置き、86 行で SHA-256 を確かめる)
- .github/workflows/agent.yml(extract ジョブ: インストーラを取って抽出する段。run ジョブ: 抽出の木を lift/out-app/host に置く段。R2 の鍵が使われる段)

## 先頭の注意(入力)
- 作業ジョブは、抽出の木(lift/out-app/host)だけを持つ。インストーラ(FL Studio の exe)は、この runner には無い(extract ジョブだけが取って抽出し、キャッシュする。R2 の鍵も extract ジョブにだけある)。したがって、インストーラの全件の一覧は、この task では得られない。その旨を未確認として書く。
- 抽出の木が無い場合(lift/out-app/host が無い)は、ここで止め、「実エンジン: 実行なし(抽出の木が無い)」と報告する。

## やること
1. 版と抽出物を確かめる。`find lift/out-app/host -type f | wc -l` の件数を書く。`test -f lift/out-app/host/binaries/win64/FLEngine_x64.dll` の結果を書く。
2. seed の表の行数を数える: `awk -F'\t' 'NF>=3' lift/harness/seeds/vfs-seed-idle1.tsv | wc -l`(ディレクトリの行を含む)と、`awk -F'\t' 'NF>=3 && $2=="-"' lift/harness/seeds/vfs-seed-idle1.tsv | wc -l`(ディレクトリの行)の件数を書く。
3. 抽出の木との突き合わせ(ファイルの行だけ)。`awk -F'\t' 'NF>=3 && $2!="-" {print $2}' lift/harness/seeds/vfs-seed-idle1.tsv` で抽出先を取り、各行について `test -f "lift/out-app/host/$r"` を行う。無かった件数と、無かったパスの最初の 10 件を書く(パスは抽出先の相対パス)。
4. 分類ごとに件数を数える(seed の guest パスの語で。`C:\Program Files\Image-Line\FL Studio 21\` の後ろの部分で数える)。次の分類を書く。
   - 言語パック: `System\Languages\` の下のファイル
   - テーマ(スキン): `Artwork\Skins\` の下のファイル
   - プロジェクトテンプレート: `Data\Templates\` の下のファイル(ディレクトリの行の数も書く)
   - デモのプロジェクト: `Data\Demo projects\` の下のファイル
   - 既定のプロジェクト: `Data\System\` の下の .flp
   - 補足の表: `Data\Patches\`、`System\Config\`、`Shared\Artwork\Fonts\` の件数
5. エンジンの起動時に、見つからないファイルの記録を読む。queue/kinds/lower.md の accept と同じ段で engine と bundle を作り(npm ci --prefix app、npm run build --prefix app、cargo build --release --locked --manifest-path lift/Cargo.toml、lift/target/release/fe-build engine、lift/target/release/fe-build bundle)、tools/boot-check/boot-bundle.mjs の main を一時ファイル tools/boot-check/.tmp-survey-c3.mjs に写して起動し、engine-idle になった後に `window.__app.engineLog()` を取り、その中で「見つからない」「not found」「ENOENT」などの行を grep で数える。行の形は実際の出力で確かめ、使った語を書く。ビルドの段が失敗したら、その段の名前と終了コードを書き、この手順は「未確認」とする。
6. 後始末: `rm -f tools/boot-check/.tmp-survey-c3.mjs` を行い、残った一時ファイルが無いことを `git status --short` で確かめる。

## 出力
- queue/findings/survey-c3-fl-resources.md に、次の見出しで日本語で書く。
  - `## 実測`: 手順 1〜5 の結果(出力の行つき。分類ごとの件数の表)
  - `## コードの読み`: 抽出の仕組み(extract-fl.mjs と agent.yml の段)の要点。パスと行番号つき
  - `## 未確認`: インストーラの全件の一覧が runner に無いこと、起動時の記録を取れなかった場合はその理由、言語・テーマ・テンプレートの「必要な全件」が分からないこと
  - `## 次の手順の案`: インストーラの全件の一覧を作る道具を、extract ジョブに足す案(ci/ の変更になるので、報告に書く)。seed の表を全件に広げる案。推定と明記する。
- commit する(push はしない)。
- 最終の応答の 1 行目は「実エンジン: …」で始める。

accept: test -s queue/findings/survey-c3-fl-resources.md && grep -q '^## 実測' queue/findings/survey-c3-fl-resources.md && grep -q '^## 未確認' queue/findings/survey-c3-fl-resources.md && grep -q '^## 次の手順の案' queue/findings/survey-c3-fl-resources.md
