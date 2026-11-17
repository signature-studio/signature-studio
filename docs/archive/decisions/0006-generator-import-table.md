# 0006: 生成器の import 対応表（jmp [IAT] の応答の接続）

状態: 作業者（PM）の決定を実装した。ただし決定 1 のうち seed の追加は、本番の生成器には入れていない（理由は「決定 1」を参照）。

## 文脈

`jmp [IAT]` の thunk は、今まで全部 `fle_unsupported` になっていた。応答の関数（`lift/runtime/fle_imports.c`）は作ってあったが、生成器からは呼ばれていなかった。手作業の sed（`lift/out-tls2/build.sh` など）で置き換えて計測していた。今回、その置き換えを生成器の中で正式に行う。

## 決定

1. 閉包の seed は、今は広げない。ただし、表経由で閉包内の間接 call から実際に読まれることが静的に確かめられた関数（0x41d050 の 1 件）は、seed に加える。
2. 0x41e7e0 と 0x41ce10 は、表経由の静的な読み手が見つかっていないため、加えない。
3. 本番の生成器（`lift/crates/fe-lift`）への応答の接続は、手作業の sed をやめ、生成器の中で行う。対象は、既に runtime に実装済みの応答: `fle_imp_GetCurrentThreadId`、`fle_imp_TlsAlloc`、`fle_imp_LocalAlloc`、`fle_imp_TlsSetValue`、`fle_imp_TlsGetValue`、`fle_imp_GetLastError`（`fle_mxcsr` は stmxcsr と ldmxcsr の生成器に既にあるので、この対応表には入れていない）。
4. 応答の接続は、import の thunk（`jmp [IAT]`）が、対応する応答の関数を呼ぶ形にする。対象は IAT の名前で判定する（`kernel32.dll` の該当 API のみ）。IAT の名前の解決には、既存の fe-decode の `Resolved::Import`（fe-pe の import 表から作られる `dll!name`）を使う。
5. 他の import は、今までどおり `fle_unsupported` のまま。応答は作らない。

## 実装

- 対応表: `lift/crates/fe-lift/src/imports_table.rs`（1 か所だけ定義）。
- 振り分け: `lift/crates/fe-lift/src/lib.rs` の `Mnemonic::Jmp`（near でない分岐）。`Resolved::Import` で表に当たった場合だけ `{ void f(void); f(); return; }` を生成する。当たらない場合は、今までどおり `indirect::jmp`（`fle_unsupported`）に流れる。他の分岐は変えていない。
- テスト: `lift/crates/fe-lift/tests/imports_table.rs`（新規）。合成 PE で、表の TlsAlloc は応答を呼ぶこと、表にない kernel32!TlsFree と k.dll!Fn は `fle_unsupported` のままであることを確かめる。

## 決定 1 の扱い（本番には入れていない）

- seed の追加には、fe-decode に extra seed を渡す API が要る（`analyze_with_seeds`）。fe-decode は今回の担当外のため、本番の生成器には入れていない。
- 本番の閉包に 0x41d050 を加えると、閉包の関数数が 6,969 → 6,971、命令数が 460,479 → 460,492 になる（実測）。依頼の「閉包の関数数と命令数は変わらないこと」と両立しない。この衝突は未決。
- 計測は `lift/out-final/seed-measure/`（複製）で行った。結果は `docs/investigations/generator-import-table-run.md` を参照。

## 決定 2 の理由

0x41e7e0 と 0x41ce10 は、閉包の外の内部関数。表経由の静的な読み手（呼び出し元の間接 call の値が、この関数を指すと静的に言えること）が確かめられていない。静的な根拠が無いものは seed に加えない。

## 決定 3・4 の理由

応答の仕様が確認されたものだけを生成器に入れる（各 API の出典と判断は `docs/decisions/0001`〜`0004`、`docs/investigations/imports-responses.md`）。IAT の名前で判定するため、アドレスの直書きに頼らない。

## 未対応のまま残したもの

- kernel32 の他の API（例: TlsFree）と、他の DLL の import（例: `k.dll!Fn`）。応答は作っていない。
- 閉包の外の関数（0x41e7e0、0x41ce10）。
