# Issue 0003: 公開リポジトリの履歴にあるホスト名の取り扱い

## 依頼 ID・日付
0003 / 2026-11-13

## 対象
- 公開リポジトリ `signature-studio/signature-studio` の git 履歴。
- 対象の commit: `<commit>`(`app/wrangler.jsonc`。件名「lift: import source from the previous tree」。2026-11-10)。SHA と件名は `evidence/R-history-routes.md` §1。
- ホスト名の形に当たる文字列が、現在のツリー(main)の 15 ファイルにもある(`evidence/R-history-routes.md` §3。ファイル名のみ。値は書かない。実在のホストか、一般語かは未確認)。
- 依頼者にしてほしい操作: 下の (a)・(b) のどちらかを依頼者が選んで実行する。後任(エージェント)は実行しない。返してほしい値: なし。

## 合理的理由
履歴の書き換え(filter-repo・force push など)と、リポジトリの公開範囲の変更は依頼者の権限(`AGENTS.md` §8、`docs/ops/authority.md` §4)。後任は実行しない。値をどう扱うか(書き換える・そのままにする)は依頼者の権限にある。

## 影響範囲
- (a) 書き換えない: 履歴の値は残る。現在のツリー・公開範囲・Secrets・Variables・Actions の設定は変わらない。前任の記録では、公開ホストは Access の背後にあり、認証なしの要求は Access のログインへ転送される(`docs/archive/decisions/0217-ci-deploy.md`)。この記録は 2026-11-04 付近のもので、今日の実測ではない(`evidence/R6-2a.md` は「未確認」)。
- (b) 書き換える: 対象の commit 以降のすべての commit の SHA が変わる。他の clone・fork・Actions の参照は古い履歴を指すようになる。書き換えだけでは現在のツリーの値は消えない(現在のツリーは別のコミットで除去する必要がある)。取り消し: 書き換え前の全体のバックアップ(`git clone --mirror` など)からのみ戻せる。実行前にバックアップを取る。

## 自分で試した記録
```
git log --all --format='%h %ad %s' --date=short -G'<routes の値の形>' -- app/wrangler.jsonc ci/ docs/   → 2 件(14268c2・a70e4ef)
git log --all --format='%h %ad %s' --date=short -S'custom_domain' -- app/wrangler.jsonc   → 1 件(14268c2)
git log --all --format='%h' -- app/wrangler.jsonc | wc -l   → 1
git show <commit>:app/wrangler.jsonc | grep -cE '"pattern"\s*:\s*"[A-Za-z0-9.-]+\.[A-Za-z]{2,}"'   → 1(プレースホルダの語を含まない: 0)
```
値は表示していない(件数のみ)。

## 完了の確認コマンド
(b) を選んだ場合。`app/wrangler.jsonc` の、コメントでない値の行のうち、プレースホルダの語を含まない物を全 commit で数える。
```
git rev-list --all | while read c; do git show "$c:app/wrangler.jsonc" 2>/dev/null | grep -E '"pattern"\s*:\s*"[A-Za-z0-9.-]+\.[A-Za-z]{2,}"' | grep -vE '^\s*//' | grep -vcE 'example|your-domain'; done | grep -vc '^0$'   → 0
```
この確認は `app/wrangler.jsonc` だけを見る。他の 14 ファイル(§3)は対象外で、別に決める必要がある。(a) を選んだ場合は、その旨と理由を Issue の末尾に書いて閉じる。

## 依頼なしで続ける作業
ROADMAP R6-3・R4-3・R5 系を、ホスト名を使わないローカルの実測と GitHub Actions の結果で続ける。R6-2 の公開ホストの応答は未確認のまま進め、報告にその旨を書く。

## 確認の記録
(依頼の後に追記: 日付・コマンドの出力 1 行)

GitHub の issue: https://github.com/signature-studio/signature-studio/issues/2(2026-11-13)
