# 0012: kernel32!VerifyVersionInfoW（応答は作らない）

状態: 作業者（M1 作業者）が決定。依頼者（PM）の判断は含まない。
変更: VerifyVersionInfoW については変更なし（`fle_imports.c`、`fle_imports.h`、`imports_table.rs` に VerifyVersionInfoW の応答は入れていない）。同じ作業で kernel32!LocalFree を 1 件追加した（`docs/investigations/import-top-spec.md`）。
テスト: `lift/crates/fe-lift/tests/verify.rs`（新規。VerifyVersionInfoW が停止のまま残ることを確かめる。LocalFree の 2 件も含む）。

## 文脈

0x407370 の `jmp *0x156a2aa(%rip)`（`# 0x1971620`、IAT の名前は `kernel32.dll!VerifyVersionInfoW`、`lift/out-ver/run-ver-896-b.txt` の停止点）。呼び出し元は f_409b30（0x409bab の `call 0x407370`）。r8 に、直前の VerSetConditionMask の戻り値（0x409ba8 の `mov %rax,%r8`）が入る。

## 仕様の出典

- VerifyVersionInfoW: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-verifyversioninfow （2026-10-20 取得）
  - 概要: "Compares a set of operating system version requirements to the corresponding values for the currently running version of the system."
  - Return value: 条件を満たすと非ゼロ。満たさないと 0 で、GetLastError は ERROR_OLD_WIN_VERSION。それ以外の失敗は 0 で、別のエラーコード。
  - Remarks: "The VerifyVersionInfo function retrieves version information about the currently running operating system and compares it to the valid members of the lpVersionInfo structure." 複数の要素は、major、minor、service pack の順に階層的に比べる。
  - Remarks（Windows 10 の項）: アプリのマニフェストで、見える OS のバージョンが変わる（マニフェストがなければ 6.2 として振る舞う）。
- 引数の意味: dwlConditionMask は VerSetConditionMask か VER_SET_CONDITION で、比較する要素ごとに 1 回ずつ作る（同ページ）。
- VerSetConditionMask（ビット配置は公式に書かれていない。0011 参照）。

## 仕様から言えること

1. 戻り値は、実行中の OS の状態に依存する。応答の値は OS の状態を見ないと決まらない。
2. 「FLEngine が要求する OS バージョン」は、このページからは分からない。FLEngine の内部で何を満たしたいかは、この関数の仕様では決まらない。

## 仕様から言えないこと（確認できなかった点）

- FLEngine が、どの OS バージョンを要求するか（呼び出し元の判断は未確認）。
- 0x409bf0 が 0/1 をどう使うか（objdump の読みからの推測。未確認）。
- Windows 10 以降のマニフェストによる振る舞いが、この DLL の実行に効くか（推測。未確認）。

## 候補と判断

(a) 応答の値を固定にする（例: 「条件を満たす」を返す）。
   - 不採用。固定の値は、FLEngine が要求する OS の状態について、仕様にない主張になる。どの値を返しても、仕様からは正当化できない。OS の状態を見る応答を、仕様なしに作らないという制約にも反する。
(b) 応答を作らず、停止のままにする。
   - 採用。停止点は 0x407370 のままになる。件数は数えて、次の作業（OS 条件の判断）を依頼者に渡す。

判断の理由: この関数の戻り値は、仕様の定義どおりに実装するなら「ホストの OS がどのバージョンか」を見る必要がある。このランタイムには、そのための情報源（ホストの OS 状態を見せる仕組み）を決める根拠がない。決めずに固定値を入れると、仕様に書かれていない前提を入れることになる。

参考（推測。実測ではない）: 0x409bd0 は cx=6、dx=1、r8w=1 で f_409b30 を呼ぶ。f_409b30 は major>=6、minor>=1、SP major>=1 を作る（VER_GREATER_EQUAL = 3）。そうなると、判定は「Windows 7 SP1 以上」かもしれない。関数名と引数名は未確認なので、これは読みにすぎない。判断の材料として残す（未決）。

## 結果への影響

- 閉包の再生成（896 seed）と実行で、停止点は 0x407370（VerifyVersionInfoW）のまま。詳細は `docs/investigations/verify-run.md`。
- 対応表は VerifyVersionInfoW に関しては追加なし（LocalFree の 1 件のみ追加。8 件）。
