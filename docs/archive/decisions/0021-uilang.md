# 0021: kernel32!GetThreadUILanguage の応答

状態: 作業者の判断（依頼者の「判断は作業者側が行え」と、PM の指示の範囲）。実行記録は `docs/investigations/uilang-run.md`。実装は `lift/runtime/fle_uilang.h`、`fle_uilang.c`。応答表への追記は `lift/crates/fe-lift/src/imports_table.rs`（1 行）。

## 決定

### 1. GetThreadUILanguage の応答を作った

- 値: `fle_emulated_locale.lcid & 0xFFFF`（en-US では 0x0409）。`GetUserDefaultUILanguage`、`GetSystemDefaultUILanguage`（`fle_locale.c`）と同じ値。`fle_uilang.c` は値をコードに持たない（テスト `source_holds_no_locale_value_of_its_own` が確かめる）。
- 理由（仕様）: MSDN の GetThreadUILanguage のページ（取得日 2026-10-20、updated_at 2025-07-01）の Return value は、「SetThreadUILanguage または SetThreadPreferredUILanguages で明示的にスレッドへ結び付けた言語。何も結び付けていなければ、ユーザーまたはシステムの UI 言語を示しうる」。この runtime は両方の setter に応答しない（応答表にない）ので、スレッドへ結び付けた言語は存在しえない。よって答えはユーザー／システムの UI 言語になる。その値は 0016 で決めた環境の設定（en-US）から導ける。新しい値を作ってはいない。
- LIP（補助ロケール）の 1400／1000 の規則は、en-US が補助ロケールでないため当たらない（ページの規則と、en-US が補助ロケールでないことからの読み）。
- 「ユーザー／システムの UI 言語」のどちらを返すかは、ページが「can indicate」と書くだけで厳密には定めない（読み）。この環境ではユーザー既定 = システム既定 = 0x0409 なので差は出ない。

### 2. GetThreadPreferredUILanguages、SetThreadPreferredUILanguages には応答を作らなかった

- 0x418e10 の関数は、kernel32.dll から 3 つを GetProcAddress で取る: `GetThreadPreferredUILanguages`（→ `[0x15eaee0]`）、`SetThreadPreferredUILanguages`（→ `[0x15eaee8]`）、`GetThreadUILanguage`（→ `[0x15eaef0]`）。依頼の範囲は `GetThreadUILanguage` だけ（依頼の「停止点の `[0x15eaef0]`」）。他の 2 つは今回の停止点ではなく、次の停止点（0x4193c4）が `[0x15eaee0]` = GetThreadPreferredUILanguages である。
- 判断を保留した理由: GetThreadPreferredUILanguages はバッファを書く API で、言語リストの形式（MUI_LANGUAGE_ID／NAME、MUI_MERGE_* のフラグ別の内容、サイズ問い合わせ、ERROR_INSUFFICIENT_BUFFER）を決める必要がある。ページから規則は読めるが、この呼び出し元が渡す dwFlags（0x38）の意味の読み取りと、テストの設計が別の作業になる。今回の担当範囲に含めず、記録して止める（0x4193c4 の停止）。
- 注意: 一度に 1 つの API だけ応答を足し、次の停止点を観測する方針に従った。

### 3. 他の担当との関係

- `imports_table.rs` へは `GetStartupInfoW` の行の後に 1 行を追記した（編集の直前に読み直した。既存行は変えていない）。応答表は 83 → 84 行（生成物の `fle_resp_count = 84`）。
- `fle_locale.h` の `fle_emulated_locale` を読むだけで、`fle_locale.c` は変えていない。

## 限界

- Windows 実機との照合はしていない（Linux で実行）。
- スレッドごとの UI 言語は持たない。将来 SetThreadPreferredUILanguages／SetThreadUILanguage に応答を足すときは、この関数がその値を返すように直す必要がある（`fle_uilang.c` のコメントにも書いた）。
