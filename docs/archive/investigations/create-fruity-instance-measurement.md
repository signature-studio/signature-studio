# CreateFruityInstance ハーネス 第 2 段階（ビルド、自己テスト、基準の再現、実測）

作業日: 2026-10-28。基準: HEAD **0020f8e** の `git archive`（`/tmp/opencode/fr2/src/`、RAM 上）。作業中に別の作業者のコミット 5345e61 / 4103a6c / 5e8b588 で HEAD が進んだ（`git diff --stat 0020f8e HEAD -- lift ':!lift/harness/web'` は空。docs と `lift/harness/web/` の新規だけで、ビルドに入るファイルは無変更。実測）が、測定は 0020f8e の木のまま行った。作業ツリーと git は読み取りだけ（commit なし）。

凡例: **実測** = 今回のコマンドの出力を見た事実／**読み** = コードや逆アセンブルを読んだ上での解釈（その点は実行で確かめていない）／**未確認**。ログは `logs/`、道具は `tools/`、差分は `fruity2.diff`。

## 0. 結論

| 項目 | 結果 |
|---|---|
| FRUITY 無効の基準（a4、6 モジュール + フォント 2 個） | **再現した**。TAG `t2331d00e8ebb`、`init_count=942 of 942`、`completed 895 / 895`、`all_units_done=yes`、`== stop: returned from entry`、終了コード 0、functions 5,306、entries 1,007,942。**native の SHA-256 は `9c76bc6a…` ではなく `9c3244281cd3696c065ebb41ea674b67605c5b8433c8310d0f13719a3ba32f37`**（実測）。原因は d7336ad の `a4.c`（注記の文言とコメント。§2 の対照で絞った）。d7336ad の前の木（e9216b9）を同じ手順でビルドすると `9c76bc6a…a23785` が出て、HEAD の直前の木（228c78a、fruity のコミットの前）は `9c324428…` で、今回の FRUITY 無効のビルドと**ビット単位で同一**（§2）。**新しい基準は `9c3244281cd3696c065ebb41ea674b67605c5b8433c8310d0f13719a3ba32f37`** |
| 自己テスト | **コミットされたまま: 46 ok + 1 FAIL**（コンパイルは警告 0・エラー 0）。FAIL は**自己テストの期待値の誤り**（`+0x34` はパディングではなく `+0x30` のポインタの上位半分）。1 行直して **47/47、0 failure**（§3） |
| FRUITY 有効: `CreateFruityInstance(0xC0, 0xEDE95A, host, &instance)` | **戻らなかった。名前付き停止**: `fle: GetFileAttributesW: empty path is not defined`（ランタイムの仮想ファイルシステム、`lift/runtime/fle_vfs.c:303`）。呼び出しの 4,693 関数入口目（0.04〜0.05 秒、新規の distinct 関数 157）で SIGABRT、ハーネスの終了コード **134**。戻り値（rax）なし、`instance` は 0 のまま。**設計書の予想（最初の停止は WinVerifyTrust、0x1375bce）は外れた**（§4）。独立の 3 回のビルドで同一（§4.4） |
| 停止の原因（読み） | エンジンが `HKCU\Software\Image-Line\Shared\Paths` の値 `Shared data` を読もうとして開けず（仮想レジストリは空）、結果が空文字列のまま `DirectoryExists('')` 相当を呼ぶ（§5）。偽ホストの値が原因ではない |
| 実験変種（WinVerifyTrust を 0 で返す） | **実行していない**。そこへ行く前の停止だから（§8） |

肯定的に見せる点はない。ハーネスは動いたが（ビルド、リンク、事前検査、呼び出し、停止の報告）、エンジンはほとんど進んでいない（呼び出しの先頭の約 4,700 関数入口）。

## 1. 手順と環境

- `git archive 0020f8e lift docs` を `/tmp/opencode/fr2/src/` に展開。`fe-lift`／`fe-pe` を `CARGO_TARGET_DIR=/tmp/opencode/fr2/target CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0 cargo build --release --offline -p fe-lift -p fe-pe` で作った（4 秒）。SHA-256 は V1 の検証で使われたもの（`/tmp/opencode/v1/target-rel/release`）と**同一**（`fe-lift` `3a6ff3c8…467f6`、`fe-pe` `743e2faf…b570b0`。実測）。V1 の分は消す前に `fr2/febin/` へ複写した。
- 許可された削除: `/tmp/opencode/{a5f,c1,wasmv,wasm6,m2inv,a4fix,v1}/` の下（合計約 690 MB）を、パスを確認してから `find <dir> -mindepth 1 -maxdepth 1 -exec rm -rf` で消した。他のディレクトリ（`web`、`fruity`、`audit-*` など）は触っていない。
- 測定の道具: `tools/run6-fruity.sh`（コミット済みのものを**無変更**で使用。`FRUITY_W`、`S`、`FEBIN` を環境変数で渡した）と `tools/wrap/`（`clang` はコミット済みのまま。`python3` は `/usr/bin/python3` を V1 と同じ `/home/ubuntu/.local/bin/python3` に変えた 1 行だけの差。`-g` を外し、コンパイル済みの分割ソースを消す）。OUT の名前はすべて `o*`。
- 使った木は、コミットされた木に**自己テストの 1 行と docs の 1 行だけを足した**もの（`fruity2.diff`）。この 2 ファイルは `a4-multi.sh` の stamp の対象（`harness/*.c`、`harness/*.h` など）に入らない（読み: スクリプトの `stamp()` の `ls` の列挙）ので、ビルドの入力は 0020f8e のコミットと同じ。実測の裏づけ: 自己テストを直した木の FRUITY 無効のビルド（`o1`）と、直していない 228c78a の木のビルドの native が同一（§2）。なお自己テストの修正は `o1` より前、docs の訂正は全ビルドの後に入れた。

## 2. FRUITY 無効の基準（手順 1。`logs/base-off/`、`logs/ctl-*/`）

| 構成 | TAG | a4 | native の SHA-256 |
|---|---|---|---|
| **0020f8e、FRUITY 無効**（OUT `o1`） | `t2331d00e8ebb` | 942/942、895/895、functions 5,306、entries 1,007,942、終了 0 | **`9c3244281cd3696c065ebb41ea674b67605c5b8433c8310d0f13719a3ba32f37`** |
| 228c78a（fruity のコミットの前）、同じ手順・道具 | 同じ | 同じ | **`9c3244281cd3696c065ebb41ea674b67605c5b8433c8310d0f13719a3ba32f37`**（上と同一） |
| e9216b9（d7336ad の前）、同じ手順・道具 | 同じ | 同じ（`a4: note:` の文言だけ旧） | **`9c76bc6a9270bd99e874cfd2633ca99334ba1a276d2337db235ecf7a25a23785`**（旧基準の値） |

- 実測: e9216b9 と 228c78a の `lift/` の差は `harness/a4.c`（注記の文言とコメント）と `crates/fe-lift/tests/a4_report.rs` だけ（`diff -rq`）。したがって **`9c76bc6a…` → `9c324428…` の変化は d7336ad の `a4.c` による**（読み: 文字列定数の変更が `.rodata` の SHA を変える）。実測で言えるのは「e9216b9 は旧値、228c78a は新値」まで。
- 実測: 228c78a と 0020f8e（FRUITY 無効）の native が**同一**なので、fruity のコミットは既定のビルドの**ビットを変えない**（設計書 J2 の問い。ビット単位で）。`main_two.c` を `-DFLE_FRUITY` なしでコンパイルした `.o` も、228c78a のものと 0020f8e のものが `cmp` で一致（`logs/static/main_two-object-cmp.txt`）。
- 実測: 新旧の native のログの差は、`rsp=`（ASLR）と、VFS 種付けの一覧の木の場所だけ（e9216b9 の分は加えて `a4: note:` の文言）。`a4:` の 7 行は 3 者で同一。
- 実測: 結合表。`merge_tables_n.py` の旧版（228c78a）と新版が `--extra-rows` なしで**同一の common.c**を書き、出力の行も同一。旧版の出力は、実際の OUT `o1` の common.c ともバイト単位で一致。`fruity-rows.txt` を付けると 8 行の差（宣言 3 + 行 3 + 個数の置換 1 行 = 1 削除 + 1 追加。期待どおり）。FRUITY 有効の OUT の common.c は、この「付けた」版とバイト単位で一致（`logs/static/merge_check.txt`）。
- 実測: 環境変数の拒否。`FLE_FRUITY_SECONDS=5`（`FLE_CALL_FRUITY=1` なし）は `a4-multi: FLE_FRUITY_SECONDS is set without FLE_CALL_FRUITY=1`（終了 2）、`FLE_CALL_FRUITY=2` は `must be unset, empty, 0 or 1`（終了 2）。OUT は作られない。
- 実測: **FRUITY=1 のバイナリ**を `FLE_CALL_FRUITY` なし（または 0）で実行すると、`rsp=` を除いて FRUITY 無効のベースラインの native ログ（297 行）と**同一**、終了コード 0（`logs/harness-tests/off.*`、`off0.*`）。
- 実測（時間と RAM）: gen 17 s、build 57 s、`/usr/bin/time -v` の合計 1 分 15 秒、ビルド中の最大の 1 プロセスの RSS 2,739,144 KB。
- `/tmp/opencode` の使用量（`du -sm`。段階の前後で測った値で、連続監視ではない）: 着手時 1,488 MB → 許可された削除の後 851 MB → ビルド完了後の最大 **2,160 MB**。2.5 GB を超えた観測は無い。`/` への書き込みは `lift/out-git/fruity2-results/`（約 0.9 MB）だけ。

## 3. 自己テスト（手順 2。`logs/selftest/`）

- コンパイル: `clang -std=gnu11 -O0 -g -Wall -Wextra -fPIE -I runtime -I harness harness/seeds/fruity_selftest.c` — 警告 0、エラー 0（実測）。
- 実行（コミットされたまま）: `ok` 46 行、`FAIL the 4 padding bytes at +0x34 are zero …`、`1 failure(s)`、終了 1。
- **原因（実測 + 読み）**: ホストオブジェクトの `+0x30` は 8 バイトのポインタで、オブジェクトの大きさ 0x38 ちょうどで終わる。`+0x34〜0x37` はそのポインタの上位 32 ビットで、パディングではない。第 1 段階の設計書 §6 の表と自己テストの 1 件が「パディング（0）」と誤っていた。**ハーネスの実装（`fruity_host.h`）は誤っていない**（`+0x30` を 8 バイトで書いている。上位半分が 0 でないのは、ヒープのアドレスだから）。
- **直した箇所（全部）**: `lift/harness/seeds/fruity_selftest.c` の 1 件の検査を、「`+0x30` の 8 バイトがオブジェクトの大きさ 0x38 で終わり、`+0x34` の 4 バイトが `h.basedir >> 32` に等しい」に置き換え（件数は 47 のまま）。`docs/investigations/create-fruity-instance-harness.md` §6 の表の 1 行を訂正。偽ホストの値と設計は変えていない。
- 再実行: `ok` 47 行、`0 failure(s)`、終了 0（`logs/selftest/selftest-run-fixed.txt`）。ASan／UBSan は、この環境に clang のランタイムライブラリが無くリンクできなかったので**使えなかった**（未確認）。

## 4. FRUITY 有効の実測（手順 3。`logs/fruity-run1/`〜`run3/`、`logs/gdb/`、`logs/harness-tests/`）

### 4.1 ビルド

`FLE_CALL_FRUITY=1 bash tools/run6-fruity.sh <OUT>`。TAG `t7eb3fac2ab02`（fruity の鍵が入るので既定の TAG と別）。`FRUITY=1` と `FRUITY_ROWS` が働き、FLEngine の生成 C の SHA 検査（`e047beeb…`、註釈あり）は `(expected)` で通った（実測。**FLEngine の生成 C は不変**）。ビルドのログ（`build.txt`）は `built …` の 1 行だけで、警告・エラーは出ていない（`--wrap=__cyg_profile_func_enter` のリンクも通った）。native の SHA-256 は **`8a54acc6c15e7cedd6af26d29d885cddc2cbe0df670c5924f5272ec31f4b9374`**。

### 4.2 呼び出しと停止点（`logs/fruity-run1/native.txt`、`a4-multi.stdout`）

```
fruity: CreateFruityInstance: export ordinal 3, va 0x1375990 (PE export table of the image at 0x400000)
fruity: host object at 0x…870 (0x38 bytes): vtable=… [slot0=0x7ffb00000010 slot1=0x7ffb00000020 slot2=0x7ffb00000030] version=1 flags=0 +0x10=0
fruity:   +0x18 name … "FL Studio"  +0x20 … ""  +0x28 … ""  +0x30 base directory … "C:\Program Files\Image-Line\FL Studio 21\" (buffer 0x8000 bytes)
fruity: before the call: functions executed (distinct) 5306, entries 1007942, shadow depth 12
fruity: limits: 20000000 lifted entries, 60 s wall
fruity: calling CreateFruityInstance(rcx=0xc0, rdx=0xede95a, r8=host …, r9=&instance …) with rsp=…
fruity: == stop: SIGABRT (runtime abort, see runtime message)
fruity: runtime message: fle: GetFileAttributesW: empty path is not defined
fruity: the stop is in the lifted thunk f_4234f0 (innermost lifted function) = `jmp [IAT slot]` of import kernel32.dll!GetFileAttributesW
fruity: CreateFruityInstance at 0x1375990; elapsed 0.05 s; lifted entries since the call began: 4693; new distinct functions: 157
```

| 項目 | 値（実測） |
|---|---|
| 呼び出し引数 | rcx=0xC0、rdx=0xEDE95A、r8=偽ホストオブジェクト、r9=`&instance`（0 で初期化） |
| 打ち切りの種類 | **名前付き停止**（ランタイムの `fle_stop`、`abort()`。ハーネスの上限（回数 20,000,000／実時間 60 秒）ではない） |
| メッセージ全文 | `fle: GetFileAttributesW: empty path is not defined`（`fle_vfs.c` の `parse_path`、`len == 0`） |
| ゲストの停止位置 | import thunk `f_4234f0`（`kernel32!GetFileAttributesW`）、戻り先 `0x43bb6e`（`f_43bb50` の中）。引数 rcx=`0x413ffc`（イメージ内の番地で、先頭の 2 バイトが 0 = 空の UTF-16 文字列） |
| C スタック（gdb `bt`、`FLE_FRUITY_TRAP=1` 併用） | `fle_stop` ← `parse_path` ← `fle_imp_GetFileAttributesW` ← `f_4234f0` ← `f_43bb50` ← `f_653130` ← `f_653100` ← `f_9ddfc0` ← `f_1263f60` ← `f_1264050` ← `f_12652c0` ← `f_1265390` ← `f_1265690` ← **`f_1375990`（CreateFruityInstance）** ← `fle_dispatch` ← `fle_call_guest` ← `fruity_run` ← `main` |
| シャドウコールスタック | 深さ 23（DllMain の longjmp が残した古い 12 + この呼び出しの 11）。`CreateFruityInstance`(guest-return 0xcb000000 = fle_call_guest の戻り番地) → `f_1265690`(0x1375aa1) → `f_1265390` → `f_12652c0` → `f_1264050` → `f_1263f60` → `f_9ddfc0`(0x1263fc1) → `f_653100`(0x9de086) → `f_653130`(0x65311e) → `f_43bb50`(0x653161) → thunk(0x43bb6e)。`CreateFruityInstance` から `f_1265690` を呼ぶのは 0x1375a9c（設計書の静的な読みの `1375a9c call 0x1265690` と一致） |
| 戻り値（rax）／`instance` | **なし**（戻らない）／0 のまま |
| 終了コード | ハーネス **134**（`a4-multi.sh` 自体は 0） |
| functions／entries | DllMain 完走時 5,306／1,007,942 → 停止時 **5,463／1,012,635**（呼び出しの増分は +157／+4,693） |
| 時間 | 呼び出しの区間 0.04〜0.05 秒。プロセス全体（DllMain を含む）0.78 秒（`/usr/bin/time -v`） |
| 最大 RSS（native の実行） | **181,904 KB**（約 178 MiB） |
| ホストオブジェクトへの呼び出し | slot0 が 1 回（`id=3 index=1 value=0` → slot2 へ転送 → **1** を返した）、slot1 は 0 回 |

### 4.3 呼び出しの区間でエンジンが呼んだ API（gdb で読むだけ。`logs/phase/`、`tools/fr2apis.py`）

`fle_imp_*` の呼び出しは**142 回、21 種**（停止時の `GetFileAttributesW` を含む）。すべて応答表に行がある API。上位: `SysAllocStringLen` 22、`SysFreeString` 22、`InitializeCriticalSection` 21、`TlsGetValue` 14、`MultiByteToWideChar` 12、`WideCharToMultiByte` 11、`RegOpenKeyExW` 11、`CompareStringW` 11。主なもの（順序どおり）:

| # | API（引数） | 読み |
|--:|---|---|
| 14〜19 | `GetClassInfoW`／`CreateWindowExW`（クラス `TPUtilWindow`）／`DefWindowProcW` ×3／`SetWindowLongPtrW` | Delphi の補助ウィンドウ（仮想ウィンドウ）を作った |
| 106〜108 | `RegOpenKeyExW` HKCU `Software\Image-Line\Shared\General` → `RegQueryValueExW` `FL Cloud` → `RegCloseKey` | |
| 109 | `GetComputerNameExW` | |
| 110〜115 | `RegOpenKeyExW` HKCU と HKLM の `Software\Image-Line\Registrations\Token`（アクセス 0x20019／0x20009／0x1 の 3 通りずつ） | 登録情報の読み取り。仮想レジストリに無い |
| 116 | `RegCreateKeyExW` HKCU `Software\Image-Line\FL Studio 26` | 仮想レジストリに**キーを作った** |
| 117 | `RegOpenKeyExW` 作ったキーの下 `DSTITF` | |
| 137 | `MultiByteToWideChar`（CP_UTF8、41 バイト = **偽ホストの基底ディレクトリ `C:\Program Files\Image-Line\FL Studio 21\`**） | エンジンが偽ホストの `+0x30` を読んだ |
| 138〜140 | `RegOpenKeyExW` HKCU `Software\Image-Line\Shared\Paths`（0x20019／0x20009／0x1） | §5 |
| 141 | `GetFileAttributesW`（空の文字列）→ **停止** | |

呼ばれた API の名前はすべて既に応答があるもの（未応答 import の停止は 0 件）。文字列の中身（エンジンの UI 文字列）は引用していない。

### 4.4 再現（独立の OUT、一から）

OUT `o2`、`o3`、`o4` の 3 回のビルド（それぞれ新しい OUT。`o4` は後で gdb の測定用）: native の SHA-256 は 3 回とも `8a54acc6…b9374` で**同一**。`o2` と `o3` のネイティブのログ（462 行）は、ASLR のアドレスと時刻の正規化の後で 1 か所だけ違う: `rbx=0x53df1901` 対 `0xc55f1901`（`ebx` に入った `rdx` の下位 32 ビット = ASLR のアドレス由来）。停止点、メッセージ、スタック、増分（+4,693／+157）は同一。

### 4.5 ハーネスの機能の検査（`logs/harness-tests/`。いずれもエンジンの応答は変えていない）

| 検査 | 結果（実測） |
|---|---|
| `FLE_FRUITY_MAX_CALLS=1000` | 1,000 入口で停止、終了コード **3**、レポートあり |
| `FLE_FRUITY_STOP_AT=RegOpenKeyExW` | 最初の `RegOpenKeyExW` の thunk 入口（呼び出しの 2,174 入口目）で名前付き停止、終了コード **6** |
| `FLE_FRUITY_STOP_AT=kernel32.dll!GetFileAttributesW` | 本来の停止点の**手前**（応答が走る前）の thunk で名前付き停止、終了コード 6 |
| `FLE_FRUITY_STOP_AT=WinVerifyTrust` ／ 存在しない名前 | 到達せず本来の停止（134） |
| 実時間の上限 | 実時間 60 秒の上限そのものは**未実行**（呼び出しが 0.05 秒で止まるため）。gdb で `fle_call_guest` の入口に `SIGALRM` を注入して、ハンドラの経路（理由 `limit: wall time 60 s …`、終了コード **4**）を確かめた（注入であって時間経過ではない） |
| 不正な環境変数 | `FLE_FRUITY_MAX_CALLS=abc`、`SECONDS=0`、`TRAP=x`、`FLE_CALL_FRUITY=2`、`FLE_CALL_FRUITY` なしの `FLE_FRUITY_MAX_CALLS` — すべて終了 **2**、メッセージあり、ゲストは動かない |
| `FLE_FRUITY_TRAP=1` + gdb | 停止の報告の後に SIGTRAP（SIGABRT は gdb が先に止まる）。`bt` で上の C スタックが取れた |
| 戻りの経路（**合成**） | 停止したので戻りの経路は実エンジンでは動かなかった。gdb で `fle_call_guest` を `return` させ rax を設定して（`0`、`0xfffffffe`、`7`、`2` + `instance` を偽ホストに向ける）、`fruity: rax=… instance=…`、`RETURNED`、FL64.exe の分岐の記録（0／2 → vtable[0]、−2 → MessageBoxA、他 → なし）、`instance->vtable=… slot0=…` が正しく出ることだけを確かめた。エンジンが戻った結果ではない |
| 偽ホスト | 自己テストの 47 件 + 実行で slot0 → slot2 の転送が動いた（上の 4.2） |

## 5. 停止の原因の分類（読み。逆アセンブルと実行時の呼び出しの記録から）

- **分類**: 「ランタイムの名前付き停止」。`GetFileAttributesW` は応答表に行があるが、`parse_path` が空のパスで `fle_stop`（決定 0018 の方針「仕様が定める範囲を超える入力は abort で止める」。空のパスは決定の命名規則の一覧には書かれておらず、コードの `parse_path` が同じ扱いで止める）。未応答の import ではない。
- **空のパスの出どころ（読み）**:
  1. `f_653100`（ゲッター）が `f_652e60` を呼ぶ。`f_652e60` はレジストリを開くラッパーを呼んで `HKCU` の `Software\Image-Line\Shared\Paths`（0x652f88 の UTF-16）を開き（0x5af3d0 でルートキーを設定、0x5af7c0 で開く、と読む）、開けたら値名 `Shared data`（0x652fd8 の UTF-16）を読む。開けなければ結果は空のまま。**実測**（gdb で応答関数の戻りの `R[0]` を読んだ。`logs/gdb/reg-returns.txt`）: `Software\Image-Line\Shared\Paths` の 3 回の `RegOpenKeyExW`（アクセス 0x20019／0x20009／0x1 の順に再試行）はすべて **2（ERROR_FILE_NOT_FOUND）** を返し、その後に `RegQueryValueExW` は呼ばれていない。仮想レジストリは空で始まる（決定 0018）。同じ区間で `Shared\General` の open は 0（成功。ゲスト自身が作ったキーと読む）、`Registrations\Token`（HKCU／HKLM）は 2、`FL Studio 26` の `RegCreateKeyExW` は 0、その下の `DSTITF` の open は 2。
  2. 結果が空（`[rbx] == 0`）なら `f_653130` が走る: ローカル文字列を 0 にして、`f_43bb50`（`DirectoryExists` 相当。`GetFileAttributesW` を呼び、`-1` なら偽、0x400（リパースポイント）、0x10（ディレクトリ）を見る）を**空文字列で**呼ぶ → これが停止。
  3. 戻り値の扱い: `f_43bb50` は戻り値 `-1`（INVALID_FILE_ATTRIBUTES）を普通に偽として扱う（`cmp edi,-1; je`）。つまりエンジンはこの呼び出しが失敗するのを前提にしている（読み）。
- **偽ホストの値が原因ではない**。エンジンは偽ホストを読んだ後（§6）でこのレジストリを読んでいる。
- この停止を過ぎた場合の続き（読み）: `f_653130` は、ディレクトリが存在しなければ `f_63b880(&path, 5, 1)`、それでも存在しなければ `f_63b880(&path, 0x1a, 1)` を呼ぶ（その先にさらに `f_6525b0`、`f_43c830`、`f_652500` の呼び出しがある）。`f_63b880` は `shfolder!SHGetFolderPathW(0, csidl, 0, 0, buf)` を呼ぶ（csidl 0x5 = CSIDL_PERSONAL、0x1a = CSIDL_APPDATA と読む）。**ランタイムの `fle_imp_SHGetFolderPathW` は CSIDL_FONTS(0x14) と CSIDL_APPDATA(0x1a) だけを答え、他は `csidl 0x… is not answered` で名前付き停止する**（`fle_env.c`）。したがって、空のパスを偽（−1）で通すと、次の壁は **`SHGetFolderPathW(CSIDL_PERSONAL)` の名前付き停止**になる可能性が高い（読み。ランタイムのソースとエンジンの呼び出しの引数から。実行していない）。レジストリの値を置く選択をした場合は、この分岐に入らない可能性がある。

## 6. エンジンが偽ホストをどう使ったか（実測。gdb のハードウェア読み取りウォッチポイント、`logs/gdb/hostwatch-*.txt`）

停止までの呼び出し（4,693 入口）で、ゲストが**読んだ**偽ホストのフィールド:

| フィールド | 読んだか | 読んだ関数 |
|---|---|---|
| `+0x00` vtable | はい | `f_1262140`（vtable を読み、slot0 を `call [vtable]`） |
| `+0x08`／`+0x0c`（HostVersion／Flags、8 バイトのワード） | はい（ウォッチポイントの検出 4 回。`memcpy` の読み取り単位で、命令の数ではない） | `f_1375990`（CreateFruityInstance 自身） |
| `+0x10` | **いいえ** | — |
| `+0x18`（`"FL Studio"`） | **いいえ** | — |
| `+0x20`、`+0x28`（空文字） | **いいえ** | — |
| `+0x30`（基底ディレクトリ） | はい | `f_1262140` |

`f_1262140` の読み（逆アセンブル）: グローバルのホストポインタが非 NULL なら `host->vtable[0](host, 3, 1, 0)` を呼び、戻り値が **1 なら** `host->+0x30` の文字列（空でなければ）を UTF-8 → UTF-16 に変換して、アプリのディレクトリとして使う。そうでなければ `GetModuleFileNameW(HInstance)` のディレクトリ部にフォールバックする。つまり **`Dispatch(3, 1, 0)` は「ホストが基底ディレクトリを持つか」の問い合わせ**（`id=3` が基底ディレクトリ、`index=1`）と読める（FL64.exe のスロット 2 の `id == 3 && index <= 1 → 1` と整合）。`id != 3` の呼び出しは、停止までに 0 回（設計書の「`id` が 3 以外で何が呼ばれたか」の問いは未解決のまま）。

## 7. 次の壁の一覧（上位 20）

停止点が WinVerifyTrust ではなかったので、依頼の「署名後の予想」の表は**そのままでは作れない**。実測の壁 1 と、読み（静的）の候補を分けて並べる。**読みの行は「壁 1 を何らかの方法で通した場合」の条件つきで、順序は実行順の予想ではなくコード上の位置**（設計書の静的な到達可能性は、応答表に行が**ない** import だけを数えていた。今回の停止は、行があって**名前付きで止まる**応答だった。この種の壁は静的な数え方に出ない）。

| # | 壁 | 区分 | 根拠 |
|--:|---|---|---|
| 1 | `kernel32!GetFileAttributesW(L"")` の名前付き停止 | **実測** | §4 |
| 2 | `shfolder!SHGetFolderPathW(CSIDL_PERSONAL = 0x5)` の名前付き停止（`csidl 0x5 is not answered`） | 読み（1 を「失敗（−1）」で通した場合。ランタイムのソース + エンジンの引数） | §5 |
| 3 | その先の `SHGetFolderPathW(CSIDL_APPDATA = 0x1a)`（答えは有り）の結果ディレクトリの存在確認（`GetFileAttributesW`／仮想ファイルシステムの状態次第） | 読み | `f_653130` |
| 4 | `f_6525b0`（最後のフォールバック）が呼ぶ `GetModuleFileNameW`、`LoadStringW`、`Reg*`（行は有る） | 読み（到達するかは未確認） | `reach` の結果（`logs/next-walls/reach-after-stop-f653130.txt`） |
| 5 | `advapi32!RegDeleteKeyExW`（行なし） | 読み（設計書の静的な読み。`0x1375a33` の `f_13752f0` の下。今回は呼ばれなかった） | 設計書の `reach-before-first-WinVerifyTrust` |
| 6 | `wintrust!WinVerifyTrust`（自己検査、0x1375bce 付近） | 読み（設計書の予想。**到達していない**） | 設計書 §3 |
| 7〜9 | `crypt32!CryptQueryObject`、`CryptMsgGetParam`、`CertCloseStore`／`CryptMsgClose` | 読み（静的な下限、呼び出し位置順） | `logs/next-walls/next-walls-static-order.txt` |
| 10〜12 | `CertFindCertificateInStore`、`CertGetNameStringW`、`CertFreeCertificateContext`（署名者名 `Image Line NV` の比較） | 読み | 同上 |
| 13〜17 | user32: `DestroyWindow`、`GetWindowLongPtrW`、`PostMessageW`、`UnregisterClassW`、`IsWindowVisible` | 読み | 同上 |
| 18〜20 | user32: `EnableWindow`、`EnumThreadWindows`、`GetActiveWindow`（以下 `GetCapture`、`GetLastActivePopup`、`GetWindowRect`、`MessageBoxW`、`SendMessageW`、`SetWindowPos` …） | 読み | 同上 |

その後（設計書の 113 個の続き）: メッセージループ（`PeekMessageW`、`TranslateMessage`、`DispatchMessageW`、`WaitMessage`、`MsgWaitForMultipleObjects`）、`CreateThread`／`ResumeThread`、`DirectSoundCreate`、`CoCreateInstance`、`ShellExecute*`、`InternetGetConnectedState` など（設計書 `unanswered-reachable-after-first-WinVerifyTrust.txt`）。静的な下限なので実際の数は不明。

## 8. 実験変種（WinVerifyTrust を 0 で返す）

**実行しなかった**。依頼は「停止点が WinVerifyTrust のとき」の参考値のためで、今回の停止は WinVerifyTrust の手前（壁 1）にある。WinVerifyTrust に届くには、`GetFileAttributesW("")` に何かを答えるか、仮想レジストリに `Shared data` を置く必要があり、どちらも**環境の主張（PM の判断）**で、「停止点は分類するだけで直さない」という依頼の範囲外と読んだ。実行すればよい次の実験の候補（実行していない。判断は PM）: (a) 壁 1 の応答を「失敗（INVALID_FILE_ATTRIBUTES、エラー番号は PM が選ぶ）」にした変種、(b) レジストリに `HKCU\Software\Image-Line\Shared\Paths\Shared data` を置いた変種。どちらも壁 2 以降の確認につながる。

## 9. 設計の判断点 J1〜J13 のうち、測定で答えが出たもの

| ID | 測定で言えること | まだ要るもの |
|---|---|---|
| J1（スタブの受け方 D） | **動いた**。事前検査が 3 行を見つけ、ゲストが `call [vtable]` で slot0 を呼び、slot2 へ転送され 1 を返した（実測）。FLEngine の生成 C は不変（SHA 検査が通った）、common.c の差は 8 行 | A、B との比較は不要になった（D で足りるかぎり） |
| J2（SHA の解釈） | **答えが出た**: 既定のビルド（FRUITY 無効）の native は、fruity のコミットの前後で**ビット単位で同一**（228c78a と 0020f8e）。旧基準 `9c76bc6a…` からの変化は d7336ad の `a4.c`。FRUITY 有効のビルドは別の TAG・別のバイナリ（`8a54acc6…`） | 解釈（ビット単位か結果か）の採否は PM。**基準の値の更新**: `lift/` のスクリプトに native の SHA の期待値は入っていない（grep 0 件）が、docs 以下の 10 ファイル（`progress.md`、決定 0113、検証・追跡の記録、`a4-six-module-trace/tools/mkreport.py` の文面など。0020f8e の木での grep）に旧値 `9c76bc6a…` の記述がある。書き換えはしていない |
| J3（基底ディレクトリの値） | エンジンは `+0x30` を**読む**（実測）。`Dispatch(3,1,0)` が 1 のとき、アプリのディレクトリとして使う（読み） | その値が後で何に使われるか（停止より後）は未測定。値 `C:\Program Files\Image-Line\FL Studio 21\` が正しいかは実機の照合が要る |
| J4（擬似 VA） | 衝突なし。事前検査も実行時の転送も通った | 値の妥当性は選択のまま |
| J5（署名検査） | **未到達** | — |
| J6（署名後の 113 個） | **未到達**。ただし「静的な到達可能性（行がない import だけを数える）」は、**名前付きで止まる応答の壁を数えない**ことが実測で分かった（今回の壁 1 と、読みの壁 2） | 壁の洗い出し（census 的な方法）の要否。census は「ユニットを放棄して続ける」ので、`CreateFruityInstance` のような単一の呼び出しには使えない（読み） |
| J7（ブロッキングのメッセージループ） | **未到達** | — |
| J8（`fle_call_guest`） | 呼び出しは働いた。**戻りの検査（`R[4]`、戻り番地、被保存レジスタ）は走っていない**（停止したため） | 戻る呼び出しでの検査 |
| J9（DllMain 完走後の状態から始める） | 呼び出しの先頭の 4,693 入口は異常なく進んだ（実測）。実機と同じ状態かは未照合 | 実機との照合 |
| J10（任意機能） | `stop_named`、`describe_value`、`FLE_FRUITY_TRAP`、停止レポートは動いた。`decode_wvt`（WinVerifyTrust の引数の解読）は**未実行** | `decode_wvt` の確認 |
| J11（報告の分離） | 問題なし。`a4:` の 7 行は 1 回だけ、`fruity:` の行は別。`a4-survey.sh` の要約は DllMain の報告のまま（`harness exit: 134` は fruity の停止） | — |
| J12（ホストを DllMain の後に作る） | DllMain はホストなしで完走する（基準）。CFI は DllMain の後に作ったホストで進んだ。ホストを先に作った場合との差は未測定 | — |
| J13（H1 の予想） | **外れた**（§4、§7） | — |
| 新: `Dispatch` の `id` | 停止までは `id=3 index=1 value=0` の 1 回だけ（§6） | `id` が 0、1、4 のものは未観測 |

## 10. 直した誤り（全部）

1. `lift/harness/seeds/fruity_selftest.c`: `+0x34` の「パディングが 0」の検査（誤った期待）を、`+0x30` のポインタが 0x38 で終わること・`+0x34` が上位半分であることの検査に置換。
2. `docs/investigations/create-fruity-instance-harness.md` §6 の表の `+0x34〜0x37` の行を訂正。

上の 2 つ以外のソースは変えていない（`diff -rq` で 2 ファイルだけ。`crates/`、`runtime/`、`harness/` の C は無変更）。`fruity2.diff` は 0020f8e の `git archive` に `patch -p1` で当たり、当てた木と作業木は `diff -r` で一致する（実測）。

## 11. `cargo test` への影響（手順 5。全体テストは走らせていない）

- 実測（grep）: `crates/` のテストが参照する `harness/` のファイルは `image.c`、`a4.c`、`a4-survey.sh`、`wasm/trace_native.c`、`wasm/build.sh`、`wasm/split_c.py`、`census/`、`hooks/`、`seeds/ft-seeds-4b00000.txt` など。**fruity のコミットで変わったファイル（`main_two.c`、`build_two_native.sh`、`gen_two.sh`、`merge_tables_n.py`、`a4-multi.sh`）と新規ファイル（`fruity*.c`／`.h`、`fruity-rows.txt`、`seeds/fruity_selftest.c`）を参照するテストは 0 件**（`a4-multi`、`main_two`、`merge_tables_n`、`gen_two`、`build_two_native`、`fruity` の grep が 0 件）。`crates/` と `runtime/` は無変更（`diff -rq`）。ただし `a4_report.rs` は `-I harness` で `a4.c` をコンパイルするので、`harness/` に新しいヘッダ（`fruity.h`、`fruity_host.h`）が増えたことは入力に入る（名前の衝突は無い。読み）。テストを実行していないので、影響が無いことは**未確認**。

## 12. 限界・未確認

- **実機 Windows との照合は 0 件**。ブラウザ・wasm は未測定。
- エンジンは呼び出しの先頭の約 4,700 関数入口までしか進んでいない。署名検査、メッセージループ、`instance` の値、`CreateFruityInstance` が戻るかは何も測っていない。FL64.exe が戻り値を見て `instance->vtable[0]` を呼ぶ部分も、実エンジンでは動かしていない（合成の確認のみ）。
- 停止の原因（§5）の「`Shared data` が読めず空になった」は、逆アセンブルと、`RegOpenKeyExW` の戻り値 2（実測）からの読み（`f_652e60` の戻り値の使われ方を実行で確かめてはいない）。
- 壁 2 以降（§7）は読み。静的な到達可能性は直接 call だけの下限（設計書と同じ限界）。
- 自己テストは ASan／UBSan で走らせられなかった。
- 実時間の上限（60 秒）の実際の経過での動作は未確認（シグナル注入のみ）。
- 3 回のビルドが同一なのは同じ機械・同じ clang（21.1.8）・同じ `fe-lift` での結果。別の機械は未確認。
- `/tmp/opencode` の使用量は段階の前後でしか測っていない（連続監視ではない）ので、ビルド中の瞬間的なピークは未確認。

## 13. 規則違反の自己申告

- AGENTS.md の「作業はすべて task agent を通じて」: 私は task ツールを持たないサブエージェントなので、調査・ビルド・測定・文書をすべて自分で行った。
- 存在しないツール名 `bash` を 1 回呼んで失敗した（副作用なし）。
- シェルの組み込み `time` を cargo のビルドの 1 回に使った（コマンドとしての `timeout`／`sleep`／`nohup` は書いていない。`/usr/bin/time -v` は計測に使った）。シェルツールの `timeout` パラメータ（ミリ秒）は長いビルドに使った。バックグラウンドの実行はしていない。
- python は、ほとんどの呼び出しで `PYTHONDONTWRITEBYTECODE=1` を設定して実行し、モジュールを読み込む自作のスクリプトには `sys.dont_write_bytecode = True` を書いた。ただし標準入力から流した数回の短いスクリプト（ファイルの文言の置換、表の整形、JSON の変換）は、そのコマンドで環境変数を設定していたか確認していない（読み込むモジュールは標準ライブラリだけで、ファイルは作らない）。`__pycache__` は `/tmp/opencode/fr2` にも成果物にも 0 個（`find` で確認）。site-packages の側は確認していない。標準ライブラリと衝突する名前のファイルは作っていない（`fr2disasm.py` など）。
- 削除は、許可された `/tmp/opencode/{a5f,c1,wasmv,wasm6,m2inv,a4fix,v1}/` の中身と、自分の `/tmp/opencode/fr2/` の中の OUT（`o1`〜`o4`、`oc-*`、`ox` は作られていない）、作業用の一時ディレクトリだけ。他の作業者のディレクトリ（`web`、`fruity`、`audit-*` など）は触っていない。`rm` のパスはすべて事前に確認した。
- gdb を native に付けた。読み取りとブレークポイント、ハードウェア読み取りウォッチポイントだけの測定のほか、**戻りの経路の合成検査**で `return`、レジスタの設定、`instance_slot` のメモリへの書き込み（使い捨てのプロセス内。成果物や状態には影響しない）と、`SIGALRM` の注入をした。いずれも §4.5 に明記した。
- ホストの実ファイル・レジストリをゲストに見せていない（種付けは `vfs-seed-fonts.tsv` の 2 ファイルだけ）。依頼者の資格情報・ブラウザには触れていない。値の選択（応答の値）は変えていない。WinVerifyTrust には何も答えていない。
- 成果物のログのうち、`bytes[N]=<hex>` の行（ゲストのメモリの先頭 32 バイトのプレビュー。今回の停止では `rcx` の指すイメージ内のコードのバイト列を含む）は、公開監査の指摘を考えて**コミット用の複写では 16 桁以上の 16 進列を `<hex omitted from the committed copy>` に置き換えた**（原本は RAM 上 `/tmp/opencode/fr2/logs/`）。`phase-apis.tsv` は、エンジンの UI 文字列を含む呼び出しの文字列欄を空にした（API 名と引数は残した）。
- 作業中に HEAD が 5345e61 → 5e8b588 に進んだ（私の操作ではない）。測定は 0020f8e の `git archive` で行った。`lift/out-git/` は `.gitignore`（`lift/out*/`）の対象で、`git status` には出ない。

## 14. 成果物（`lift/out-git/fruity2-results/`、約 0.9 MB）

- `fruity2-report.md`（本書）、`fruity2.diff`（2 ファイル、`patch -p1`）。
- `logs/`: `base-off/`、`ctl-e92/`、`ctl-228/`（基準と対照）、`selftest/`、`static/`（結合表と環境変数の拒否）、`fruity-run1/`〜`run3/`（FRUITY 有効の 3 回）、`harness-tests/`、`gdb/`（C スタック、ホストのフィールドの読み取り）、`phase/`（呼び出しの区間の API の一覧と回数）、`next-walls/`。
- `tools/`: `run6-fruity.sh`（無変更）、`run-direct.sh`、`wrap/`（`python3` は 1 行変更）、`fr2apis.py`、`fr2hostwatch.py`、`fr2regret.py`（gdb の読み取り測定）、`fr2disasm.py`、`reach2.py`。
