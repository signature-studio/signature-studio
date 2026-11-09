<!-- worker への依頼の雛形。main はこれを埋め、先頭に prompts/worker-rules.md の全文を貼って worker を起動する(plugin が付与するなら貼らない)。 -->
<!-- 1 依頼 = 1 目的。完了条件が 3 つを超えたら分解する。<…> を全て埋める。空欄を残さない。 -->

## 目的
<1 文。何ができた状態にするか。例: 「`scripts/env.sh` を source した後に `emcc --version` が 6.0.12 を返すことを確かめ、結果を docs/STATE.md の『環境』節に書く」>

## 入力(実ファイル・実測値。推測を事実として書かない)
- 作業木: `~/work/signature-studio`(ブランチ: <名前>)
- 読むファイル: <パス>(読み取りのみの物は「読み取りのみ」と書く)
- 現在の状態: <パスと数字。出典のコマンドを添える>
- 関連する規則: `docs/ops/<該当する文書>.md` の <節>

## やること(条件の一覧。依頼者の指示・ROADMAP の次の一手の条件をそのまま)
1. <条件 1>
2. <条件 2>
3. <条件 3>

## やらないこと
- 上の条件にない条件・状態・分岐・設定・ログ・テストを足さない。足したい物は最終報告の末尾に「提案: …」を 1 行。足さない。
- 新しい URL パラメータ・環境変数・`#ifdef`・ビルド変数・旗を作らない。
- <この依頼に固有の禁止。例: 「既定の manifest を変更しない」「旧作業ツリーを変更しない」>

## 完了条件(コマンドで確かめられる形。各行 = 実行するコマンドと、期待する出力)
```
1. <コマンド>   → <期待する出力・終了コード>
2. <コマンド>   → <期待する出力・終了コード>
3. scripts/scan-forbidden.sh <変更したファイル>   → 終了コード 0
```
- commit する(メッセージは変更の内容を短く。秘密・識別子・禁止語を書かない)。commit したら `git push origin main` で push し、`git ls-remote origin main` で届いたことを確かめる(scan 通過後。リポジトリは PUBLIC で意図的)。

## 報告形式
`prompts/worker-rules.md` の「報告の形式」に従う。追加の項目: <あれば。例: 「測定は n=5 の中央値とマシン・ブラウザを書く」>

## 返信先
- main の sessionID: <$OPENCODE_SESSION_ID の値>(中間の報告・ブロッカーは `/opencode-send-message` で。`delivery: queue` を指定しない)

## 記入例(消して使う)
- 目的: 「`docs/STATE.md` の push の行を実測で更新する」
- 完了条件: `git push --dry-run origin HEAD` の出力の先頭 3 行を報告に貼った → `grep -n '2026-' docs/STATE.md` に今日の日付がある。`scripts/scan-forbidden.sh docs/STATE.md` が 0。
- やらないこと: 他の行を変更しない。push を実行しない(`--dry-run` のみ)。
