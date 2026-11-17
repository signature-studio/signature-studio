# 0025: deploy.yml の 100 行・80 字の版で、emsdk の出所・変数名・ビルドの書き方を、文面の条件を満たすだけの形にした(依頼者の指摘 3 点)

- 日付: 2026-11-17
- 誰が: deploy.yml の版を書いた worker(7306a58d・8f2e4ec6・3643d45d など。worker ID は未確認)。記録の作成は本 worker(main の依頼)
- 依頼者の発言: 2026-11-17。発言 ID は未確認(main 経由の原文)。対象は commit 4dd4f8c3 の deploy.yml(blob 86c481fe。3643d45d 以降、HEAD と同じ内容)。
  - 「(48 行) なぜemscripten-core/setup-emsdkではなくmymindstorm/setup-emsdkという表記を採用している？」
  - 「(14〜16 行) 奇怪な変数名、独特で醜い」
  - 「(51〜95 行) マトモなビルドツールチェインを採用しろという指示を全く守っていなくて感動しています。まさかあなたの美的感覚がここまで壊滅的だったとは思いませんでした。」
  - 制約の原文(記録の引用): 2026-11-16 の依頼文は「もし100行に収まらないのだとしたら、マトモなツールチェインを採用せずクソシェルスクリプトを書いてきたあなたが100%悪いです。」(0015 の 5 行、R6-23 の 194 行の引用)。main が伝えた「是正する良い機会ですね」の句は、リポジトリの記録に無い(未確認)。R6-25 の 37 行は同じ依頼を「100 行に収まらないなら、マトモなツールチェインを採らずシェルを書いたあなたが悪い」(msg_1e3971a68001YFwQ1g3aiNwF3f、18:01:34)と要約している。依頼者の原文は handover の 01a に 2026-11-16 の発言が無いため照合できない(未確認)。

## 何が起きたか

### (1) 出所: emsdk を、公式の emscripten-core ではない名前の action で入れた

- 48 行: `- uses: mymindstorm/setup-emsdk@v16`(`version: 6.0.12` の with は 49〜50 行)。
- 初めて入ったのは 8f2e4ec6(2026-11-16 20:21:36 +0900。R6-25 §13 の v4)。`git log --reverse -S 'mymindstorm' --all` の最初の行。
- その直前の版(a4c3fbd9。R6-25 §9、20:12 の指示)は、`git clone -q https://github.com/emscripten-core/emsdk.git`・`git -C $W/emsdk checkout -q $EMSDK_REV`(REV 35ff8a6…)・`python3 $W/emsdk/emsdk.py install 6.0.12` だった。8f2e4ec6 は、この REV 固定を外して action 1 行に替えた。R6-25 §13 の「主な変更」に理由は書かれていない。R6-31 の表(L51–53 の差)は「由来は §9」と記録している。
- 選定の根拠の記録は無い。R6-25 の `gh api` は runner-images の 1 件だけ(§13 の Node の確認)で、setup-emsdk のリポジトリは確かめていない(`git show HEAD:evidence/R6-25.md | grep -n 'gh api'`)。
- GitHub の事実(2026-11-17 に確認):
  - `gh api repos/emscripten-core/setup-emsdk`: 公開。archived=false。説明「Setup Emscripten for use with GitHub actions」。最新リリース v16(2026-04-08 公開)。
  - `curl -sI https://github.com/mymindstorm/setup-emsdk`: HTTP 301、location `https://github.com/emscripten-core/setup-emsdk`。`gh api repos/mymindstorm/setup-emsdk` は emscripten-core/setup-emsdk の内容を返す。つまり mymindstorm の名前は同じリポジトリへの転送。改名か移管かは未確認。
  - `users/mymindstorm`: 個人アカウントは存在する(作成 2017-04-19)。
  - タグ `v16` は両方の名前で同じ(`gh api repos/emscripten-core/setup-emsdk/commits/v16` の sha 4528d102f723…)。
- 固定の仕方: `@v16` のタグ指定。SHA 固定ではない。

### (2) 変数名: 14〜16 行などを 1 文字にした

- 14〜16 行: `O: lift/out-app`(出力先)・`H: lift/harness`・`F: lift/target/release`。run の中の短い変数は 29〜30 行(`A=`・`U=`)、90〜91 行(`Q=`・`S=`)。
- 初めて入ったのは 7306a58d(2026-11-16 20:30:58 +0900。commit 件名「100 行・80 字の版」)。O がここで入った。H と F は、それより前の R6-25 §10(20:14 の指示。`W`・`H`・`F` の 1 文字)に既にある。
- 変数名を短くした判断の記録は無い(`git grep -n -E '変数名|1 ?〜 ?2 文字|1 文字|短くし|縮め' HEAD -- docs evidence` は、無関係の 1 行だけ)。

### (3) ビルド: 100 行に収めるため、データファイル・xargs・run の統合で書いた

- 51〜62 行: env と `cargo build`・`git clone`・`patch`・`emcmake cmake … -G Ninja`・`cmake --build` を 1 つの run に。
- 63〜82 行: 1 つの run に、オブジェクトの生成(`emcc`)・`fe-lift gen-all`・`fe-wasmgen hybrid-n0`・`emcc @link.rsp`・`wasm-opt`・`fe-wasmgen hybrid`・`sed` を入れた。
- 83〜95 行: 1 つの run に、`pack_seed.py`・`pack-assets.mjs`・`aws s3 sync`・root の gate と cp を入れた。
- 引数はファイルから読む: `xargs -d '\n' -a $H/…args`(65・77・80・81・85 行)、`emcc @$H/…args`(67・69・74・76 行)、`@$H/web/link.rsp @$O/exports.rsp`(78 行)、`$(cat …)`(70・72・73・84・94 行)。合計 15 行(`git show 4dd4f8c3:.github/workflows/deploy.yml | grep -n -E 'xargs|@\$[HO]|\$\(cat '` の出力を数えた)。7306a58d の件名は「引数はデータ(@file と xargs)」。
- main の依頼文の「(45 行)」は、この版では再現できなかった。xargs・mv・mkdir を含む行は、全体で 10 行(`grep -c -E 'xargs|\bmv\b|mkdir'`)、run の 63〜95 行の範囲で 9 行。
- 直接の CMake と Ninja: 60〜62 行で使っている。0401 の 15 行は「B(CMake・Ninja)却下: CMake 自身がスクリプト」と書いている。R6-36b(main が挙げた計画)は HEAD の ROADMAP に無い(`git show HEAD:docs/ROADMAP.md | grep -c R6-36` は 0)。
- 0400 の制約との関係: 公開 action は 3 つ(checkout・setup-node・cloudflare/wrangler-action)。HEAD の deploy.yml の `uses:` は 5 件で、そのうち 2 件(34 行 `runs-on/cache@v5.0.7`・48 行 `mymindstorm/setup-emsdk@v16`)が範囲外。0400 の 20 行の文に、この 2 件の扱いは無い。0404 は runs-on/cache を依頼者の指示(msg_1a2ccc…)に基づいて採った。0404 に 0400 の限定への言及は無い(`grep -n -E 'action|公開|0400|限定' docs/decisions/0404-*.md` は 1 行だけで、R2 のキャッシュの指示の引用)。

## 原因(その時の判断)

- (1) 出所: 版を REV 固定の clone から action に替えたとき、action の名前・リポジトリ・最新・保守状態を確かめなかった。0017(最新の版を調べずに版を書いた)と同じ型。名前がどこから来たかは記録に無い(未確認)。
- (2) 変数名: 100 行・80 字に収めるため、名前を 1〜2 文字にした。読みやすさを確かめず、理由も書かなかった。
- (3) ビルド: 100 行・80 字・スクリプト禁止の 3 条件に合わせるため、データファイル・xargs・run の統合を選んだ。依頼者の「マトモなツールチェイン」の指示は、R6-25 の 37 行で「100 行の根拠」の欄に置かれ、要件として扱われていない。0401 は CMake と Ninja を却下していたが、この版は Blend2D の段で CMake と Ninja を使っている。その食い違いを決定の文に書かなかった。
- 共通の判断: 行数の制限を、中身を減らすのではなく、書式・名前・データ化・統合で満たした(0018 と同じ型)。

## 影響

- 依頼者の指摘(3 点)。依頼者の時間。
- ビルドの再現性: REV 35ff8a6 の固定が、8f2e4ec6 から外れた。R6-31 の表(L51–53)は「差」と記録している。出力への影響は未確認(再ビルドの比較は行っていない)。
- 範囲外の action が 2 件(34・48 行)。

## 是正

- 本記録(0025)。是正の commit と検証は、この記録の作成時点で無い。
- 作り直し・正規のビルドシステムへの移行は、main の計画(R6-36b、CMake と Ninja。HEAD の ROADMAP に無い)。依頼者の『Makefile など却下』との関係は未確認。
- 公式の名前への変更: 2170029d(2026-11-17 14:25:09 +0900。R6-36a。決定 0412。別の worker が作成)で、48 行を `emscripten-core/setup-emsdk@v16` に直した(`git show origin/main:.github/workflows/deploy.yml | sed -n 48p` で確認)。変数名は直していない(0412 の理由: 100 行・80 字と両立しないため)。本記録の作成時(HEAD 4dd4f8c3)の状態は上の (1) の通り。

## 再発防止

- 文書: 版の action の名前は、`gh api repos/OWNER/REPO` で存在・アーカイブ・転送を確かめてから書く(0017 と同じ規則。未実施)。
- 検査の案(未実施): deploy.yml の `uses:` を、0400 の 3 件の一覧と照合する。`xargs`・`@$`・`$(cat` を含む行の数を記録する。変数名の 1 文字化を検査する(これは判断の材料であり、禁止の規則ではない)。

## 同じ型の探索

- 範囲と方法(2026-11-17。HEAD = 4dd4f8c3 の内容を `git show` と `git grep` で読んだ):
  - 記録: docs/failures/0015〜0024 と README の表。docs/decisions/0400〜0410 の「置かない」「外す」「削除」の行と、100 行・80 字・行数を含む行(`git grep -n -E '100 ?行|80 ?字|行数' HEAD -- docs/decisions`)。evidence/R6-25.md §9〜§14。
  - deploy.yml: 4dd4f8c3 の 100 行(`wc -l`)。80 字超 0 行(Python の len)。
- 件数の数え方: 「型」は、同じ理由で同じ種類の機能・書き方を外した単位。件数は型の数。

### A. 100 行・80 字・スクリプト禁止を理由に、趣旨から外れた書き方・機能を選んだ(記録に理由がある): 15 型

| No | 出典 | 内容 | 現在(4dd4f8c3) |
|---|---|---|---|
| A1 | 0400 行 32 | 抽出の層・emsdk・rust・cargo fetch を「置かない」(100 行・スクリプト禁止) | 直書きで戻した(23〜56 行)。ただし C1 を含む |
| A2 | 0400 行 33 | Blend2D・9 段 bundle・段キャッシュ・bundle の検査を「置かない」(456 行の shell) | Blend2D と 9 段は戻した(57〜82 行)。段キャッシュと bundle の検査は無い |
| A3 | 0400 行 34 | R2 への 5 件の載せを「置かない」(bundle が無いため) | 87〜89 行で `aws s3 sync`。1 件ずつの読み直しと prefix の確かめは無い |
| A4 | 0400 行 35 | root の差し替えを「置かない」 | 92〜95 行で戻した(0410) |
| A5 | 0410 §1 ② | 参照先 5 件の存在確認を「置かない」(100 行) | 未 |
| A6 | 0410 §1 ④ | 退避(put・get と sha)を「置かない」(100 行・スクリプト禁止) | 未 |
| A7 | 0410 §1 ⑥ | 書き込み後の確認を「置かない」(100 行) | 未 |
| A8 | 0410 §1 ⑦ | 失敗時の戻しを「置かない」(④ と同じ) | 未(3643d45d の root.txt による戻しは、失敗時の戻しとは別) |
| A9 | 0018 | deploy.yml の 2〜6 行を { } [ ] の 1 行に詰めた(5 行。1e148a5) | 是正済み(450b2a7)。HEAD に 0 件 |
| A10 | 0018 | run に && で 2 コマンドをつないだ(1e148a5 の 17 行) | 現在 4 行(28・43・75・92)。2 コマンドずつ。0018 の基準(3 つ以上)では 0 件 |
| A11 | 0018 | agent.yml の 1 行の配列(branches・needs。2 件) | 是正済み(0e30892) |
| A12 | 0018 | agent.yml の heredoc 内の 1 行の JSON(5 件) | 未(agent.yml の 147〜151 行) |
| A13 | 0016 | 配信の設定を実行時に作る段(20〜57 行)を直書きで、行数に収めた | 除去で是正(0403。依頼者の指示) |
| A14 | 4dd4f8c3・7306a58d | 引数をファイルから読む(xargs・@file・$(cat)。15 行) | 未 |
| A15 | 8f2e4ec6 §13 | run のブロックへの統合(63〜82 行と 83〜95 行の 2 ブロック) | 未 |

- A の件数: 15 型(是正済み・除去 4 型: A9・A11・A13・A4。一部: A2・A3。未: A5〜A8・A10・A12・A14・A15 の 8 型。A1 は戻したが C1 を含む)。

### B. 100 行の版で書かれたが、記録の原因は別(読み落とし・理由の記録なし): 5 型

| No | 出典 | 内容 | 記録の原因 | 現在 |
|---|---|---|---|---|
| B1 | 0021 | Blend2D の段に b2d3-compop.patch を当てなかった(7306a58d〜a9115cba) | 旧経路の引数を読まなかった(記録の原因) | 是正済み(59 行で patch を当てる) |
| B2 | 0022 | pack.args の 4 件(--description・--stop-note 2 件・--brotli-quality)を落とした | 旧の呼び出しを 1 件ずつ比べなかった(記録の原因) | 是正済み(e63f7770)。0022 の記録に従う |
| B3 | 8f2e4ec6 §13 | emsdk の REV 固定の clone(a4c3fbd9 §9)を mymindstorm の action(REV 固定なし)に替えた | 理由の記録なし | 表記は 2170029d で直した(48 行)。REV 固定は未 |
| B4 | 7306a58d(O)・R6-25 §10(H・F) | 変数名を 1 文字にした | 理由の記録なし | 未(14〜16・29〜30・90〜91 行) |
| B5 | 8f2e4ec6 §13 | Blend2D の tar の SHA-256 照合の行を外した(main の決定) | 記録は「main の決定」のみ。100 行との関係は未確認 | 未(57〜62 行は commit 固定の clone) |

### C. 明文の制約に直接触れる行(同じ版にある): 5 件

| No | 行 | 内容 | 制約の文 |
|---|---|---|---|
| C1 | 46〜47 | `python3 9590e599^:ci/lib/extract_fl.py $O/in.exe …` | 0400 の 20 行「ci/lib の .py の実行を含む」 |
| C2 | 84 | `python3 $H/web/final0129/pack_seed.py …` | 0400 の文は .py を名指ししない(趣旨は同じ。未確認) |
| C3 | 85 | `node app/scripts/pack-assets.mjs …` | 0400 の文は .mjs を名指ししない(趣旨は同じ。未確認) |
| C4 | 48 | `mymindstorm/setup-emsdk@v16`(2170029d で `emscripten-core/setup-emsdk@v16` に変更。範囲外の action であることは同じ) | 0400 の 20 行「公開 action は … だけ」 |
| C5 | 34 | `runs-on/cache@v5.0.7` | 同上。0404 の依頼者の指示に基づく採用。0400 の限定との関係は 0404 に無い |

- 合計: 一覧は A 15 型 + B 5 型 = 20 型。直接の違反候補は C 5 件。

## 未確認

- 依頼者の原文(2026-11-17 の 3 件)の発言 ID。main 経由の文だけを読んだ。2026-11-16 の「100%悪い」の句と、「是正する良い機会ですね」は、記録の引用のみ。
- 8f2e4ec6 の作成者の判断(action の選定の根拠)。
- setup-emsdk の転送の種類(改名か移管か)。
- 変数名・run の統合の判断。記録に無い。
- R6-36b(main の計画)の内容。HEAD の ROADMAP に無い。
- 0401 の CMake・Ninja の却下と、4dd4f8c3 の 60〜62 行の関係。
- 4dd4f8c3 の emsdk(action)と REV 35ff8a6 の差が、出力(engine の鍵)に影響するか。R6-31 の表は差を記録しているが、影響の比較は行っていない。
