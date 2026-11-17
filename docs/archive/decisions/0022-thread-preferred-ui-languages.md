# 0022: kernel32!GetThreadPreferredUILanguages の応答

状態: 作業者の判断（依頼者の「判断は作業者側が行え」と、PM の指示の範囲）。実行記録は `docs/investigations/null0-run.md`。実装は `lift/runtime/fle_uipref.h`、`fle_uipref.c`。応答表への追記は `lift/crates/fe-lift/src/imports_table.rs`（1 行）。テストは `lift/crates/fe-lift/tests/null0.rs`（6 件）。

## 決定

### 1. GetThreadPreferredUILanguages に応答を作った

- 値を作ったのではなく、ページの規則と、0016 で決めた環境の設定（en-US、LCID 0x0409）から導いた。
- 規則（MS のページ、取得 2026-10-20）:
  - 形式: MUI_LANGUAGE_ID(0x4) と MUI_LANGUAGE_NAME(0x8) は排他、既定は NAME。ID 形式は 16 進 4 桁、0x なし（en-US は "0409"、en は "0009"）。
  - フィルタ: 既定は MUI_MERGE_USER_FALLBACK(0x20)。スレッド優先言語、プロセス優先言語、重複しないユーザー優先言語、なければ（リストにない場合）システム既定 UI 言語、の順。MUI_MERGE_SYSTEM_FALLBACK(0x10) は 0x20 とだけ組み合わせられ、組み合わせ（= MUI_UI_FALLBACK）ではフォールバック言語と中立言語がリストに入る。
  - バッファ NULL かつ `*pcch == 0` はサイズ問い合わせ（2 つの NUL を含む）。成功時 `*pcch` は取得したバッファのサイズ。
  - 失敗: ERROR_INSUFFICIENT_BUFFER（バッファ不足、または NULL の誤指定）。他の理由で失敗したときの `*pul`、`*pcch` は未定義。
- この環境では、スレッド優先言語とプロセス優先言語は存在しない（Set 系に応答しない）。ユーザー／システムの言語は 1 つ（emulated locale）。よって、リストは 0x20 で `[name]`、0x30 で `[name, 中立形]`。
  - 0x38（MUI_LANGUAGE_NAME | MUI_UI_FALLBACK。停止点 0x4193c4 の呼び出し元の値）の結果: `"en-US\0en\0\0"`、数 2、サイズ 10 文字。
- 名前は `fle_imp_GetLocaleInfoW`（`fle_locale.c`）経由で得る（LOCALE_SNAME、LOCALE_SISO639LANGNAME）。ID 形式は `lcid & 0xFFFF` と `lcid & 0x3FF`。`fle_uipref.c` は言語の値をコードに持たない（テストが確かめる）。
- 失敗時（バッファ不足）は ERROR_INSUFFICIENT_BUFFER を設定し、`*pul`、`*pcch` に触れない（ページが未定義とするので、触れないことを選んだ）。成功時は last error に触れない。

### 2. 読みであって、ページの規則ではない点（推測として明記）

- 「中立形」: ページ「User Interface Language Management」は、言語の後にその中立形（"fr-FR, fr"）を置き、重複を除くと書く。en-US の中立形を "en"（ID 形式で "0009"）とするのは、GetThreadPreferredUILanguages のページの例（"0409"、"0009"）と整合する読みで、全ロケールに通用する規則としては確認していない。ID 形式の中立形は主言語 ID（下位 10 ビット）とした。
- MUI_MERGE_SYSTEM_FALLBACK を付けた場合に「ユーザー言語と同じ言語は 1 つだけ入る」（システム既定 UI 言語が同じ en-US なので追加されない）ことは、「重複は最初の 1 つだけ」の文からの読み。
- 既定フィルタ 0x20 の場合に中立形を入れないことは、ページがフォールバックと中立言語を 0x30 の効果として書くことからの読み。実 Windows との照合はしていない。

### 3. 停止（実装しない入力）にしたもの

次はページが結果を定めないので、`fle_stop` で止める: 0x4、0x8、0x10、0x20 以外のビット（MUI_THREAD_LANGUAGES(0x40) を含む）、MUI_LANGUAGE_ID と MUI_LANGUAGE_NAME の同時指定、0x10 だけ（0x20 なし）、`pulNumLanguages` または `pcchLanguagesBuffer` が NULL。

### 4. SetThreadPreferredUILanguages には応答を作らなかった

- 0x418e10 の関数は kernel32.dll から Get、Set、`GetThreadUILanguage` を GetProcAddress で取る。Set の `[0x15eaee8]` は応答表になく NULL のまま。Set の呼び出し点は f_419400 内の 0x41948a、0x4194f6、0x419535（objdump の読み）。0x41948a は `GetThreadUILanguage` の戻り値が引数 di と違うときに通る経路（0x419422 の `je 0x419490` を通らない場合）、0x4194f6 と 0x419535 は r13 が非 0（0x4193a0 が非 NULL を返した）のときの経路。今回の実行でこれらに到達したかは**確かめていない**（関数入口ログは擬似 VA のスタブ呼び出しを記録しないので、ログに現れないことは根拠にならない。probe も入れていない）。次の停止点（Bsf）より前に Set が呼ばれていれば、`fle_dispatch_miss`（宛先 0）で止まっていたはずだが、実際の停止は別の種類（`fle_unsupported`）だった。この点から、停止点までに Set の呼び出しには到達していない、と読める（推測）。
- 応答を作るなら、スレッドの言語リストを状態として持ち、Get が返すリストを変える必要がある。形式（ID／NAME）、5 言語の上限、フィルタフラグ（MUI_CONSOLE_FILTER ほか）の効果、`GetThreadUILanguage`（0021）の戻り値との整合を決めなければならず、今回の停止点では要らない。判断を保留し、Set は応答表に加えない（テスト `set_thread_preferred_ui_languages_is_still_not_answered` が確かめる）。
- 注意: Set に応答を足すとき、Get の結果（このファイルの「スレッド優先言語は存在しない」前提）と `GetThreadUILanguage`（0021）を直す必要がある。

## 限界

- Windows 実機との照合はしていない（Linux で実行）。
- 言語リストの内容は emulated locale（en-US）にだけ決まる。他のロケールでは中立形の読みが当たらない可能性がある（上の推測）。
- 単一スレッドの前提。スレッドごとの状態は持たない。
