# 0424: runtime の契約に無い停止の除去（SwitchToThread の通算 100 万回）と、譲りで時間が進む規則

状態: 作業者（R6-43、worker）が決定。根拠は `evidence/R6-43.md`。撤回条件は末尾。

## 範囲と件数（先頭に書く）

- 範囲: `lift/runtime/` の `.c`・`.h`・`.inc`（202 ファイル、64,687 行。`wc -l` の合計）。`lift/crates/fe-lift/tests/` は、停止の期待を持つテストだけを見た（proc.rs）。
- 停止の行: `grep -rnE 'fle_stop|STOP\(' lift/runtime --include=*.c --include=*.h --include=*.inc` で **1,594 行**（定義・コメントを含む）。このうち上限の語（上限・大きさ・件数・深さ・表・容量など）を含む **364 行** を `evidence/R6-43/classify.awk` で分類（`evidence/R6-43/limits.tsv`）。分類の内訳は 表の容量 T 121、大きさ Z 166、深さ D 16、計数のあふれ C 14、符号化 E 11、その他 O 36。O と D には上限でない行が混ざる（読みで確かめた。`evidence/R6-43.md` §1）。
- `abort()` の呼び出し（コメントを含む）**67 行**（`grep -rn 'abort()'`）。ほとんどは状態の不整合と名前規則。
- 無限ループの候補 `for (;;)` と `while (1)`: **21 行**。計数の上限を持つのは 4 か所（`fle_seh.c` 431・491 の `FRAME_LIMIT`、`fle_input.c` 825 の `guard`、`fle_modules.c` 188 の名前長）。
- 除去: **1 件**（`fle_proc.c` の SwitchToThread の通算 100 万回の停止）。新しく足した処理: **1 件**（譲りで時計を 10 ticks 進める。`fle_sys.c`）。
- 除かなかった上限: 364 行のうち契約に根拠があるもの（i）、表の容量・資源の大きさ（iii）、診断の安全網（iv）、契約に無い上限だが表の動的化が要るもの（ii）。件数は `evidence/R6-43.md` §1 の表。

## 分類

- (a) 契約に無い上限で、外すと実機と同じ振る舞いになるもの: SwitchToThread の通算 100 万回の停止。Learn SwitchToThread は「譲る先が無ければ 0 を返す」とだけ書き、回数の上限は書かない（`fle_proc.c` の proc.rs の注記に引用された文言。Learn 本文は本作業では未取得）。実機では止まらない（`evidence/R4-5e.md` の SwitchToThread の行）。**除いた**。
- (b) 契約の拡充（時間は経過する）: 譲る先が無い譲りは仮想時計を `FLE_YIELD_TICKS`（10 ticks = 1 µs）進める。根拠: 実機の譲りは時間を使う（Learn の記述からは直接言えない。未確認）。前例は決定 0017 の A の 2（時計を読む呼び出しは 10 ticks 進める。「時計の値を待つループを終わらせるため」）。**足した**。
- (i) 契約に根拠がある上限（残す）: `KEY_NAME_MAX` 255（Learn Registry element size limits: キー名 255 文字）、`TREE_DEPTH_MAX` 512（同じ Learn: 木は 512 階層）、`CREATE_LEVELS_MAX` 32（同じ Learn: 一度の呼び出しで 32 階層）。`VALUE_DATA_MAX` 1 MiB は Learn の標準形式の 1 MB に基づく（最新形式は「利用可能な記憶域」で、この停止は標準形式の数。分類は（i）と（ii）の境界、未確認）。
- (iii) 資源の物理・表の大きさ（残す）: 固定の表と handle 領域（T 121 件の多く）、256 MiB のファイル、1 GiB のオブジェクト、2 GiB の bitmap、カウンタの幅（C 14 件。`UINT32_MAX`）。
- (iv) 診断の安全網（残す）: `FRAME_LIMIT` 100000（`fle_seh.c` の停止は 435・496。展開の鎖の循環の検出）、`FLE_CB_MAX_DEPTH` 256（`fle_callback.c` 73。`fle_callback.h` 27 行に backstop と書かれている）。
- (ii) 契約に無い上限（残す。理由つき）: 内部のバッファ（`FLE_DATE_BUF` 1024、`FLE_EXPAND_MAX` 1 MiB、256 文字の名前、600 文字の文字列）、固定の表（`FLE_HOOK_MAX` 64、`LOAD_MODULES_MAX` 64、`MAXC` 640、`CLIP_MAX` 64、`TRUST_MAX_OBJS` 64、INIT_ONCE 64 など）、ネストの上限（フックのネスト 8、DC の状態 1,048,576）。除くには、表と呼び手の buffer 長の規則を可変にする設計変更が要る。単独 commit の範囲を超える。除くと、表が無限に伸びるか、呼び手の buffer を超える時の戻り値の規則が決まらない（Learn の記述は未確認）。件ごとの除去は行わない。次の依頼の候補。
- 対象外（上限ではない）: E 11 件（符号化・code page の未実装）、O 36 件（引数の値の検査、契約の値の列挙、未実装の形）。

## 決定

1. `fle_imp_SwitchToThread`（`fle_proc.c`）の通算 100 万回の停止を外す。根拠: Learn SwitchToThread は回数の上限を書かず、R4-5e は実機に無い停止と書いた。
2. 譲る先が無い SwitchToThread は `fle_sys_clock_yield()` で仮想時計を `FLE_YIELD_TICKS`（10）進める（分類 b）。根拠: 外すだけだと、譲りで時計が進まない（`schedule()` は譲る先が無いと時計を進めずに 0 を返す。`fle_thread.c` 682〜731 行）ため、他のスレッドの期限まで待つ譲りの loop が終わらなくなる。決定 0017 A の 2 と同じ理由。
3. 停止の期待（`lift/crates/fe-lift/tests/proc.rs`）は、「1,000,001 回の譲りで停止しない・時計が 10,000,010 ticks 進む」に直した（停止の期待は、契約に無い上限を正として書いたもの）。

## 除かなかった物と残る危険

- 譲りのうち Sleep(0) は、回数の上限が無く、時計も進めない（`fle_thread.c` の `fle_thread_sleep(ms == 0)` は時計を進めない）。譲る先の無い空回りは、停止にならず無限ループになる（従来からの型。本決定は触れない）。main の判断に回す。
- 譲る先の無い無限の空回り（待っている flag を誰も変えない loop）は、停止にならず、固まる。実機と同じ振る舞い。停止の語（診断）は失われる。
- 試して退けた案: (1) 上限を残す（契約に無い停止のまま）。(2) 上限を「全員が期限無しで待つときだけ」に絞る（条件を証明できない。空回りの loop 自身が時計を読めば終わり得る）。(3) 上限の回数で stderr に記録だけ出す（診断は残るが、挙動の変更は増える。未採用）。

## 撤回条件

- 測定（P2・P3 の再生、または依頼者機）で、譲りの空回りによる固まりが観測されたとき。その場合、(2) か (3) を main が決める。
- 譲りの時間（10 ticks）が、実機の譲りの時間と合わないと示されたとき（時計の値の差は Export の決定性に効く。`evidence/R6-43.md` §4 の sha256 の結果を見る）。

## 出典

- Learn（本作業で取得したのは Registry element size limits のみ: キー名 255、木 512 階層、一度の呼び出しで 32 階層、値は標準形式 1 MB）。SwitchToThread と Sleep の文言は、コードの注記に引用された物（`fle_proc.c`、`fle_thread.c` の schedule の注記）。Learn 本文は未取得。
- 決定 0017（時計の規則、A の 2）、0122（協調スレッド、`schedule()`）、0420（R4-5e の CPU 内訳と SwitchToThread の停止の記録）。
- `evidence/R6-43.md`（全探索の表、テスト、エンジンの測定の状態）、`evidence/R6-43/classify.awk`、`evidence/R6-43/limits.tsv`。
