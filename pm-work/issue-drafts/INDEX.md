# Issue の下書き(索引)

番号は下書きの番号。作成時に実際の Issue の番号に置き換わる(`#{{NNN}}` → `#123`)。

依頼者の承認まで、GitHub に書き込まない。この下書きは、承認の後に、Milestones・Issues・Sub-Issues・ラベルとして作る。

- 1 ファイル = 1 つの Issue。ファイルの先頭の front matter は、起票の属性(`draft`・`title`・`type`・`milestone`・`parent`・`blocked_by`・`labels`・`size`・`priority`・`project`)。`---` より後が、Issue の本文。
- `type` は Issue の Type(Task・Feature)。`size` は、プロジェクト(`project`: signature-studio/3)の Size(XS・S・M・L・XL)で指定する。`priority` は、組織の Issue field の Priority(Urgent・High・Medium・Low)で指定する。本文とラベルには入れない。
- Size の決め方: 想定の所要が 1 時間以下なら XS、3 時間以下なら S、6 時間以下なら M、10 時間以下なら L、10 時間を超えるなら XL。Epic は、子の最大の Size。
- Priority の決め方: P3 に直結するものは Urgent。P2 のものと、多くの task を塞ぐものは High。P1・プラグイン・性能は Medium。後回しと最終の検証は Low。Epic は、子の最も高い Priority。
- Milestones の一覧と説明: `pm-work/milestones.md`。ラベルの案: `pm-work/issue-drafts/LABELS.md`。依存図: `pm-work/issue-drafts/GRAPH.md`。持続的な起票の計画(第 2 波以降): `pm-work/issue-drafts/WAVES.md`。
- 件数: Epic 11 件(Type: Feature)、task 28 件(Type: Task)(合計 39 件)。
- 初期投入は 39 件。起票は初期投入で終わらせず、WAVES.md の波で続ける。
- 本文の「読む物」は、リポジトリの中で実在するパスだけを書く。旧のパス(`queue/`・`driver/`・`ci/tasks/`)は、作業者に読ませない。

## 一覧

| 番号 | タイトル | Type | Milestone | Priority | Size | 親 | blocked_by |
|---|---|---|---|---|---|---|---|
| 001 | E 描画と音声の分離(P3) | Feature | M1 描画と音声の分離(P3) | Urgent | L | - | - |
| 002 | C-2 の描画の分離が、音声への圧迫を減らしたかを、本番相当の版で実測する | Task | M1 描画と音声の分離(P3) | Urgent | M | 001 | 003 |
| 003 | P3 の音声の実測の道具を作る(AudioContext を開始し、underrun を数える) | Task | M1 描画と音声の分離(P3) | Urgent | M | 001 | - |
| 004 | インストーラの demo プロジェクトで P3(Song Mode の underrun)を測る | Task | M1 描画と音声の分離(P3) | Urgent | S | 001 | 003, 040 |
| 005 | E HiDPI と P2 の測り方(画面がぼやけない。UI の 120 fps) | Feature | M2 HiDPI と P2 の測定(P2) | High | M | - | - |
| 006 | HiDPI guest の画面を物理画素にし、DPI を guest に伝える(画面をぼやけさせない) | Task | M2 HiDPI と P2 の測定(P2) | High | M | 005 | - |
| 007 | P2 の測定の道具を作る(フレーム間隔と入力の遅延を JSON で出す) | Task | M2 HiDPI と P2 の測定(P2) | High | M | 005 | - |
| 008 | HiDPI と描画の分離の後で、描画の所要と P2 を実測する | Task | M2 HiDPI と P2 の測定(P2) | High | S | 005 | 006, 002, 007 |
| 009 | E トップバーのコンテキストメニュー(Tempo・Song Position・Pattern Selector) | Feature | M3 トップバーのコンテキストメニュー(P2) | High | M | - | - |
| 010 | 右クリックでメニューを画面に出し、項目の選択を engine に返す | Task | M3 トップバーのコンテキストメニュー(P2) | High | M | 009 | - |
| 011 | Tempo・Song Position・Pattern Selector のメニューで項目を選ぶと、部品の値が変わる | Task | M3 トップバーのコンテキストメニュー(P2) | High | M | 009 | 010 |
| 012 | E FL Studio に必要な物の配布(一覧、起動前と起動後の区分、言語・テーマ・テンプレート) | Feature | M4 FL に必要な物の配布 | High | L | - | - |
| 013 | インストーラ全件の一覧を作り、作業者が読めるように commit する | Task | M4 FL に必要な物の配布 | High | M | 012 | - |
| 014 | 起動前に取る物と、起動後にバックグラウンドで取る物を分ける(lazy-load の仕組み) | Task | M4 FL に必要な物の配布 | High | L | 012 | - |
| 015 | Fruity Balance を起動後の取得に移し、1 プラグイン 1 wasm で届ける(最初の 1 件) | Task | M4 FL に必要な物の配布 | High | M | 012 | 014 |
| 016 | 起動時にホストに無かったファイルを、engine のログに記録する | Task | M4 FL に必要な物の配布 | Medium | M | 012 | - |
| 017 | 言語・テーマ・プロジェクトテンプレートを、インストーラの全件と揃える | Task | M4 FL に必要な物の配布 | Medium | M | 012 | 013, 016 |
| 018 | E 内蔵のエフェクトとインストゥルメント(DLL で 100 以上。OpenGL の VFX 系は後で入れる) | Feature | M5 内蔵のエフェクトとインストゥルメント | High | L | - | - |
| 019 | Fruity Filter と Fruity Delay 2 を、起動後の取得で届ける(1 プラグイン 1 wasm) | Task | M5 内蔵のエフェクトとインストゥルメント | Medium | M | 018 | 015 |
| 020 | 内蔵のエフェクトとインストゥルメントの DLL を、抽出の表に揃える(一覧と SHA-256 が一致) | Task | M5 内蔵のエフェクトとインストゥルメント | Medium | M | 018 | 013, 023 |
| 021 | E 内蔵 Effects の残り(約 87 件。束ねた Sub-Issue は WAVES.md の第 2 波で起票) | Feature | M5 内蔵のエフェクトとインストゥルメント | Medium | M | 018 | 020 |
| 022 | E 内蔵インストゥルメント(Generators。約 51 件。束ねた Sub-Issue は WAVES.md の第 2 波で起票) | Feature | M5 内蔵のエフェクトとインストゥルメント | Medium | M | 018 | 020 |
| 023 | OpenGL を使う VFX 系の DLL を一覧から判定し、後回しの一覧を作る | Task | M5 内蔵のエフェクトとインストゥルメント | High | S | 018 | 013 |
| 024 | E 外部プラグイン(Serum 2・OTT・Vital を、既定の経路で読み込み、音声を処理する) | Feature | M6 外部プラグイン | High | L | - | - |
| 025 | 外部プラグインのインストーラ(3 件)を、FL のインストーラと同じ経路で作業の段から読めるようにする | Task | M6 外部プラグイン | High | M | 024 | - |
| 026 | VST3 の link を既定で有効にする(オプトインを外す) | Task | M6 外部プラグイン | High | M | 024 | - |
| 027 | Serum 2 を既定の経路で読み込み、1 ブロックの音声を処理する | Task | M6 外部プラグイン | Medium | L | 024 | 040, 025, 026, 015 |
| 028 | OTT を既定の経路で読み込み、1 ブロックの音声を処理する(harness の source を作る) | Task | M6 外部プラグイン | Medium | L | 024 | 040, 025, 026, 015 |
| 029 | Vital を既定の経路で読み込み、1 ブロックの音声を処理する | Task | M6 外部プラグイン | Medium | M | 024 | 040, 025, 026, 015 |
| 030 | E エンジンの性能(P3 の再生中に実行される関数を、直接 wasm にする) | Feature | M7 エンジンの性能(P3 のホットパス) | Medium | L | - | - |
| 031 | P3 の再生中の命令の内訳を採り、直接 wasm でない関数の割合を測る | Task | M7 エンジンの性能(P3 のホットパス) | Medium | M | 030 | 003, 004 |
| 032 | P3 の再生中に命令数の多い未直接の関数を、一括で直接 wasm にする | Task | M7 エンジンの性能(P3 のホットパス) | Medium | L | 030 | 031 |
| 033 | E 起動を 10 秒以内にする(P1)。単一の wasm は 1GB 以下にする(サイズ) | Feature | M8 起動 10 秒以内とサイズ(P1) | Medium | L | - | - |
| 034 | 起動の内訳を実測し、P1 の測り方を確定する(取得・コンパイル・インスタンス化・起動) | Task | M8 起動 10 秒以内とサイズ(P1) | Medium | S | 033 | - |
| 035 | 起動の中央値を 10 秒以内にする(起動前に取る量と、wasm のコンパイルを見直す) | Task | M8 起動 10 秒以内とサイズ(P1) | Medium | L | 033 | 034, 014 |
| 037 | E 完成の検証(6 課題と P1・P2・P3 の最終の実測) | Feature | M9 完成の検証 | Low | M | - | - |
| 038 | 完成の検証: 6 課題と P1・P2・P3 の最終の実測を、同じ道具で行う | Task | M9 完成の検証 | Low | M | 037 | 001, 005, 009, 012, 018, 021, 022, 024, 030, 033, 039 |
| 039 | OpenGL を使う VFX 系の内蔵プラグインを、全群の後に 1 プラグイン 1 wasm で入れる | Task | M5 内蔵のエフェクトとインストゥルメント | Low | L | 018 | 021, 022, 023 |
| 040 | 描画(guest の UI 描画を含む)が音声を圧迫する経路を取り除く | Task | M1 描画と音声の分離(P3) | Urgent | L | 001 | 002 |

## 検査の結果

- 数の内訳: task 28 件のうち、ブロッカーが無く即時に起動するのは 10 件(003・006・007・010・013・014・016・025・026・034)。残りの 18 件は、ブロッカーの解除を待つ。
- Epic 021・022 は、WAVES.md の第 2 波で、束ねた Sub-Issue を起票する(起票待ち)。Size は、子が未起票のため M と仮定する。
- 同じファイルを変えうる task は、blocked_by にせず、各 Issue の「他のタスクとの関係」に書いた。作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
