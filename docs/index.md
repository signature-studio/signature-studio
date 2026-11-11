# docs/ 索引

入口。迷ったら `STATE.md` → `ROADMAP.md` の順に読む。規約は `../AGENTS.md`(毎ターン読むのはこれだけ)。

| パス | 用途 |
|---|---|
| `STATE.md` | 現在の状態(事実の表。各行に 区分・出典・日付)。サブエージェントが実測のたびに更新する |
| `ROADMAP.md` | ロードマップと「次の一手」。メインセッションが表の上から機械的に、サブエージェントへの依頼を作る |
| `ops/` | 運用規約(権限・環境・役割と手順・報告・誠実性・忠実性と性能・危険操作・辞書・教訓・起動) |
| `ops/glossary.md` | 用語集。依頼者が使う語と、使う言葉の決まり(この表にない言葉を作らない) |
| `ops/models.md` | AI の利用先(Cloudflare の AI ゲートウェイ経由と Amazon Bedrock)の切り替え。依頼者の操作 |
| `ops/successor-risks.md` | 後任者が陥る型の表(依頼者の発言が指した失敗を、後任者の言葉で。起動直後に読む) |
| `ops/intent.md` | 依頼者の指示の読み方(字義どおりに読まない。依頼者への確認はしない) |
| `ops/requests.md` | Issue の規則(依頼者に頼むのは責任範囲外の物だけ。いつ出すか・必須項目・出し方) |
| `ops/authority.md` | 責任範囲と能力表(何を自分でやり、何を Issue で頼むか) |
| `ops/environment.md` | この機械の能力の事実表(道具・資源・GitHub の状態。値は確認コマンドで取り直す) |
| `ops/launch.md` | 起動前の確認項目と起動手順(依頼者が端末から実行する) |
| `ops/dangerous-ops.md` | 危険操作の規則(取り消せない操作、認証情報、期限つきの予告) |
| `ops/dictionary.md` | 依頼者の発言の型と応答の対応 |
| `ops/honesty.md` | 進捗の誠実性(代役・モック・推測を進捗にしない。完了の宣言の条件) |
| `ops/fidelity-and-performance.md` | lift の忠実性・性能・オプションの規則 |
| `ops/reporting.md` | 依頼者への報告の形式 |
| `ops/roles-and-loop.md` | 役割(main・worker・reviewer)と作業の回し方 |
| `requests/` | Issue(`NNNN-<slug>.md`)と雛形 `TEMPLATE.md`。依頼の一覧と状態 |
| `core/` | 技術文書 7 ページ(各 100 行以内。旧資料の要点。区分つき) |
| `decisions/` | 新しい決定(0346〜。1 本 1 ページ)。書き方は `decisions/README.md` |
| `archive/` | 旧プロジェクトの原文。読み取り専用。進捗の根拠にしない(`archive/README.md`) |

`core/` の 7 ページ:
- `pipeline.md` — lift パイプラインの構造と各段の入出力、ビルド手順
- `win32-contract.md` — Win32 互換層の契約と、監査結果(何が足りないか)
- `reference-project.md` — 基準プロジェクト `serina1-serum2.zip` の構成と要求
- `performance.md` — P1〜P3・S1/S2 の定義、基準値、測り方、実測の要約
- `ladder.md` — G 段階 G0〜G11 の定義と、旧自己判定
- `pitfalls.md` — 前任の技術上の失敗(F-1〜F-14)と設計判断
- `inputs.md` — 外部入力と取得手順(照合用の値つき)

運用上の失敗の記録は `ops/lessons.md`。旧文書の引き方は `archive/README.md`。
