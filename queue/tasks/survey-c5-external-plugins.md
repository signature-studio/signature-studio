# survey-c5-external-plugins: C-5(Serum 2・OTT など多数の外部プラグインを存在させる)の現状を、読みと入力の確認で調べる(測定のみ。製品は変えない)

## 報告の先頭の規則
- 報告の 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。実行していなければ「実エンジン: 実行なし(入力が無い)」のように、理由を書く。
- 数字は、この実行で得た実測だけを書く。各数字に、出典のコマンドと出力の行を付ける。推定は「推定」と書き、実測と分ける。
- 代役(別のプラグイン、テスト用の信号の再生など)で、外部プラグインの測定の代わりにしない。
- 「達成」「動いた」と書かない。判定は「受理」「差し戻し」「未確認」のいずれかにする。

## 規則の要約(この task で守ること)
- 製品(app/・lift/・ci/・tools/・patches/・.github/)を変えない。変更は queue/findings/survey-c5-external-plugins.md だけ。
- Python は使わない。ベンダー名、Cloudflare のアカウント ID・ホスト名・ETag(32 桁の 16 進)、絶対パスを書かない。パスは repo の中の相対パスで書く。
- commit は環境の git の設定のまま行う(git config・--author・GIT_AUTHOR_*・git -c を使わない)。commit に Claude の行を付けない。ブランチを作らない。push はしない(判定の段が push する)。
- .git/config、opencode.jsonc、.opencode を読まない。git の内部命令(update-index・hash-object・update-ref・commit-tree・write-tree・bundle・for-each-ref を含む)を使わない。force は使わない。
- Secrets と Variables は読まない。wrangler と cf の CLI は使わない。R2 の鍵はこの実行には無い。
- sleep、timeout、nohup、disown、末尾の & は使わない。
- 一時ファイルは repo の中の .tmp-* に置き、終わったら消す。リポジトリの外(/tmp を含む)に書かない。

## 目的と根拠
台帳 L-033 の (5): 「Serum 2、OTT、その他多数の外部プラグインが存在しない」。queue/plan.md の C-5 を参照。この task は、(a) 外部プラグインの入力(インストーラ)が runner に在るか、(b) 製品の経路にプラグインを読み込む仕組みが在るか、(c) 既定で有効か(オプトインか)、を読みと確認で書く。入力が無ければ、その時点で実測は止める。

## 読む物(実在を確認済み)
- ci/artifacts.lock(R2 の artifact バケットにあるインストーラの表。6〜8 行が Serum 2・OTT・Vital)
- ci/external-inputs.sha256(外部入力の SHA-256 の表)
- .github/workflows/agent.yml(R2 の鍵を使う段: extract ジョブの「インストーラを取り、抽出する」段だけ。FL のインストーラを取る)
- app/src/plugins/ott-host.ts(先頭 1〜3 行: OTT の harness の wasm を指す。参照先の lift/harness/vst と lift/harness/web/mt1/plugin-worker/build.sh は、リポジトリに無いことを確かめる)
- app/src/plugins/plugin-client.ts、app/src/plugins/plugin-worker.ts、app/src/plugins/driver.ts、app/src/plugins/protocol.ts(プラグインの Worker と SharedArrayBuffer の仕組み)
- app/test/unit/plugins-ott.test.ts(2 行と 15 行: wasm が無ければ skip)
- lift/harness/vst3a/main_vst3a.c(Serum 2 を扱う native の harness の先頭の注釈)
- lift/harness/two/wrp1_vst3.c(VST3 の link。注釈に opt-in とある)
- lift/runtime/fle_modules.h(82 行付近)、lift/runtime/fle_ole.c(171 行付近)、lift/runtime/fle_realmod.c(173 行付近): 「オプトイン」と書かれた箇所
- lift/crates/fe-lift/tests/wall_wrp1.rs(60 行付近: realmod の表に serum2.vst3 がある)

## 先頭の注意(入力)
- 外部プラグインのインストーラ(Serum 2・OTT・Vital)は、R2 の artifact バケットにある(ci/artifacts.lock)。作業ジョブは、このバケットの鍵を持たない(.github/workflows/agent.yml の R2_ARTIFACT_* は extract ジョブの段にだけあり、取るのは FL のインストーラだけ)。
- したがって、この runner では外部プラグインの入力が無い。手順 2 の確認で「入力が無い」と書き、実測(手順 3)は行わず、「実エンジン: 実行なし(入力が無い)」と報告する。読みと、手順 1 と 4 は行う。

## やること
1. 入力を確かめる。`grep -n 'installers/' ci/artifacts.lock | cut -c1-60` の結果(鍵の名前の行だけ)を貼る。`grep -n 'R2_ARTIFACT' .github/workflows/agent.yml` の結果を貼り、鍵を使う段の名前を書く。runner の中に、プラグインのインストーラの file が無いことを `find . -path ./node_modules -prune -o \( -iname '*serum*' -o -iname '*OTT*' -o -iname 'Vital*' \) -print | grep -v '^./.git/' | head -20` で確かめる(結果を貼る。.tmp-* の中の結果は無視する)。
2. 結論を書く: 入力が runner に無い、という事実を、上の出力の行で示す。
3. 実測は行わない(入力が無いため。ビルドも、プラグインの読み込みも実行しない)。読みは続ける(手順 4 と 5)。
4. 製品の経路を読む(実行しない)。次の 3 つを、行番号つきで書く。
   - `grep -rn "plugin-client\|plugins/driver\|plugins/ott\|plugins/protocol\|plugin-worker" app/src app/worker app/shared --include=*.ts` の結果。製品の画面と engine の Worker から、プラグインの仕組みが呼ばれているか(テストと、プラグインの内部からの呼び出しだけか)。
   - `git ls-files lift/harness/vst lift/harness/web/mt1 2>/dev/null | head`、`test -f lift/harness/vst/main_vst.c && echo yes || echo no`、`test -f lift/harness/web/mt1/plugin-worker/build.sh && echo yes || echo no` の結果。OTT の harness の source と build がリポジトリに在るか。
   - `grep -rn -i "opt-in\|OPT-IN\|opt in" lift/runtime lift/harness/two lift/crates --include=*.c --include=*.h --include=*.rs | cut -c1-200 | head -20` の結果。既定で無効のものが、いくつ在るか。
5. Serum 2 の経路を読む(実行しない): lift/harness/vst3a/main_vst3a.c の注釈と、lift/harness/two/wrp1_vst3.c の注釈(何を link するか、既定で有効か)。
6. 後始末: 一時ファイルを消す。`git status --short` で残りが無いことを確かめる。

## 出力
- queue/findings/survey-c5-external-plugins.md に、次の見出しで日本語で書く。
  - `## 実測`: 手順 1 と 2 の結果(入力が無いことの出典の行つき)。手順 4 の件数。
  - `## コードの読み`: 手順 4 と 5 の結果。製品の経路に、プラグインを読み込む仕組みが在るかの結論(パスと行番号つき)。OTT の参照先が無いこと。
  - `## 未確認`: 外部プラグインの実測(入力が無い)、既定で有効にするための仕組みの有無(読みだけでは分からない部分)
  - `## 次の手順の案`: (a) 作業ジョブが入力を取る経路を、extract ジョブと同じ形で足すか、監査・受付が決める(鍵の渡し方は最小にする)。(b) オプトインの link を既定で有効にする(台帳 L-002)。(c) プラグインの処理は、C-2 の音声の経路の後に入れる。推定と明記する。
- commit する(push はしない)。
- 最終の応答の 1 行目は「実エンジン: …」で始める。

accept: test -s queue/findings/survey-c5-external-plugins.md && grep -q '^## 実測' queue/findings/survey-c5-external-plugins.md && grep -q '^## 未確認' queue/findings/survey-c5-external-plugins.md && grep -q '^## 次の手順の案' queue/findings/survey-c5-external-plugins.md
