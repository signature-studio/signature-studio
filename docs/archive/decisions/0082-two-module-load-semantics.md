# 0082: 2 つ目のモジュール（実在モジュール）のロードの意味 — D11 は名前付き停止のまま、D6 は今は決めない、DllMain が FALSE のときの LoadLibraryW ほか

状態: PM の決定（2026-10-23）を、作業者（サブエージェント）が記録した。**一部は仮の決定**（DllMain が FALSE を返したときの last error。下の表と「仮の決定である点」）。**実装はしない**。実装は W6（決定 0078）の完了後に別作業で行う。保留だった決定 0080 の D6 と D11 に答える（D6 は「今は決めない」、D11 は「名前付き停止のまま」）。事実の出典は `docs/investigations/stage1-two-module.md`（以下「段階 1」。節番号はその文書のもの）、`docs/investigations/two-module-design.md`（以下「設計文書」）、`docs/investigations/dsp-ippv2-dynamic-imports.md`（以下「調査」）。この記録はそれらに無い主張を足さない。

## 決定（PM。変更しない）

| # | 項目 | 決定 | 区分 |
|---|---|---|---|
| D11 | `FreeLibrary` で参照が 0 になるときの扱い | **名前付き停止のまま**（仮想モジュールも実在モジュールも）。fail-closed を保つ。段階 1 の試作のスイッチ `S1_UNLOAD_AT_ZERO`（DETACH を呼んで未ロードに戻す）は既定オフのまま、**採用しない** | 決定 |
| D6 | `GetModuleFileNameW` の DLL（実在モジュール）の仮想パス | **今は決めない**。それまで、実在モジュールのハンドルに対する `GetModuleFileNameW` は**名前付き停止**。FLEngine 側の既存の `GetModuleFileNameW` の応答（決定 0018）は**変えない** | 保留 |
| — | DllMain が FALSE を返したときの `LoadLibraryW` | **NULL を返す**。その前に `DllMain(DETACH, NULL)` を呼ぶ（段階 1 の試作と同じ）。last error は **DllMain が設定した値のまま** | **仮**（last error。Learn が規定していないため） |
| — | DllMain の中からの同名 `LoadLibraryW` の参照の数え方 | **決めない**。実 DLL の DllMain がこれを呼ぶことが確認された時点で決める（段階 1 §4.2、§6.4） | 保留 |
| — | 実在モジュール表と `fle_load.c` の `refs[]` の統合 | 実装時の判断に委ねる。ただし**制約**を守る（下） | 制約の記録 |

### D11 の詳細

- 仮想モジュールの参照 0 の `FreeLibrary` は、無改変ランタイムで既に停止する（段階 1 §6.1: 「the reference count would reach zero, which would unload a module the process imports (not modelled)」。実測）。実在モジュールも同じ扱いにする。
- 実在モジュールに対して DETACH を呼んで未ロードに戻す挙動は、**この決定では採らない**。次の経路に到達した時点で、その経路の証拠（呼び出し元と、直後の使い方）を見て、**別の決定**で決める:
  - `dsp_ippv2_x64.dll` の DLL 自身が、参照 0 になる `FreeLibrary` を実際に呼ぶ経路。
  - FLEngine が、参照 0 になる `FreeLibrary` を実際に呼ぶ経路（設計文書 D11 は FLEngine 側の呼び出し元を `f_65c4f0` と書くが、実行時に届くかは未確認）。
- 試作の `S1_UNLOAD_AT_ZERO=1` で測ったのは、3 回目の `FreeLibrary` が 1 を返し、その前に `DllMain(reason 0, NULL)` が実行され、続く `GetModuleHandleW` が 0、次の `LoadLibraryW` が ATTACH を再実行する、という挙動までである（段階 1 §6.3）。イメージのメモリを再初期化するかは未決・未測定のまま（同節）。この記録はそれを採用しない。
- 未ロードの実在モジュールのハンドルに対する `FreeLibrary` は、無改変ランタイムでも試作でも停止する（段階 1 §6.1、§6.5）。この決定はこの点を変えない。

### D6 の詳細

- 今は仮想パスの値を決めない。設計文書 D6 が候補に挙げた `Shared\` 以下のパスは、**採っていない**（創作の値になるため）。
- 調査が確認している DLL 内の `GetModuleFileNameW` の呼び出し点は、`0x17d637b`（`hModule` = NULL。唯一の呼び元 `0x17d1dc9` が `xor ecx,ecx`。ATTACH の直線上にあるのは読み。調査 §2.5）の 1 点である。`hModule` が NULL の呼び出しは FLEngine 側の既存の応答（決定 0018）の範囲にあり、**この決定は変えない**。
- DLL が**自身のハンドル（基底）**を渡して `GetModuleFileNameW` を呼ぶ地点は、調査では確認されていない。`0x17d7768` は `FreeLibrary` の呼び出し点（調査 §2.2 の呼び出し点の表の 10。呼ばれ方は読み切っていない）で、`GetModuleFileNameW` とは別に確認が要る。そのような地点に実行が届いたら、呼び出し側の使い方（返ったパスをどう使うか）を読んでから、別の決定で値を決める。
- 現在の `fle_vfs.c` の `fle_imp_GetModuleFileNameW` は、`hModule` が NULL でも `FLE_EXE_HANDLE`（0x400000）でもなければ停止する（ワークツリーのソースを読んだ。実行して確かめていない）。実在モジュールのハンドル（基底。決定 0080 D5）はこれに当たるので、この決定の「名前付き停止」は現行の挙動と合うと読める。実装の作業で、実在モジュールの基底が 0x400000 でないこと、停止の文言が実在モジュールを指すことを確認する。

### DllMain が FALSE を返したとき

- `LoadLibraryW` は NULL を返す。`LoadLibraryW` が戻る前に `DllMain(hinstDLL = 基底, DLL_PROCESS_DETACH, NULL)` を呼ぶ。段階 1 の試作（`S1_DETACH_ON_FAIL=1`）と同じ。段階 1 §4.1 は、これが Learn の DllMain の記述（失敗時にシステムが直ちに PROCESS_DETACH で呼ぶ）と合うと書く。
- last error は、DllMain が設定した値のまま（ランタイムは触らない）。**仮**: Learn は `LoadLibrary` の失敗時の last error を明示していない（段階 1 §4.1）。
- 試作で測った範囲（段階 1 §4.1）: DllMain が常に FALSE のとき、2 回目の `LoadLibraryW` も ATTACH から再実行され、NULL になる。最初の ATTACH だけ FALSE のとき、1 回目 NULL、2 回目 成功（その後に DETACH なし）。これらは試作の測定で、この決定が個別に定めるものではない。
- 測っていない: DETACH を呼ばない形（`S1_DETACH_ON_FAIL=0`）。`LoadLibraryExW`、`LoadLibraryA` での DllMain の失敗（段階 1 の失敗系の測定は `LoadLibraryW` のみ）。この決定は `LoadLibraryW` について述べる。

### 実在モジュール表と `refs[]` の統合の制約

統合するか、別の表のままにするかは実装の作業で決める。ただし次を**変えない**:

- 仮想モジュールのハンドル値（決定 0015 の式 `0x7FF800000000 + 0x10000 × N`、「追加は末尾に限り、並べ替えない」。決定 0081 の N = 27 以降の 4 名を含む）。
- `GetModuleHandleW(NULL)` = `FLE_EXE_HANDLE`（0x400000）の既存の挙動。
- 実在モジュールのハンドルは**基底**（決定 0080 D5。DllMain の `hinstDLL` と同じ値）。

## 選択肢と却下

段階 1 §8 の 1（D11）の 3 つの選択肢のうち:

| | 内容 | 結論 |
|---|---|---|
| (a) | 参照 0 で名前付き停止（仮想モジュールと同じ扱い） | **採る** |
| (b) | 参照 0 で DETACH を呼び、未ロードに戻す（試作 `S1_UNLOAD_AT_ZERO=1`） | 今は採らない。到達する経路の証拠を見て別の決定で決める |
| (c) | ロード済みのまま残す | 採らない（PM の決定は (a)。fail-closed を保つため。(c) は参照 0 でも `FreeLibrary` を成功させる形で、停止しない） |

## 理由（PM の決定に付された範囲）

- fail-closed を保つ。ゲストが参照 0 の `FreeLibrary` を実際に呼ぶ経路は、どちらのモジュールからも**まだ確認されていない**。経路が無いまま「DETACH して未ロードに戻す」を実装すると、証拠の無い挙動（DETACH の呼び出しとメモリの再初期化の有無）を環境の振る舞いとして固定することになる。
- D6 も同じ理由。値は創作になるので、DLL が自身のパスを取る地点と、その使い方を見てから決める。

## 仮の決定である点

- **実機 Windows は未確認**（ホストは Linux）。この記録のどの挙動も、Windows 本体との照合はしていない。
- 段階 1 は**合成 PE と native のみ**。実 `dsp_ippv2_x64.dll`、FLEngine と組み合わせた実行、wasm は未確認（段階 1 §9）。段階 1 の数値と挙動は合成 PE のもので、実 DLL のものではない。
- DllMain が FALSE を返したときの last error は、Learn の規定が無いための**仮**。DETACH を呼ぶ点は、段階 1 が Learn の記述と合うと書くが、Learn の原文はこの作業では読み直していない（段階 1 の記述による）。
- `LoadLibraryW` が DllMain を呼ぶ時点の `fle_call_guest` の深さなどの試作の測定（段階 1 §4）は、合成 PE の範囲である。

## 再訪・撤回条件

- **D11**: DLL 自身または FLEngine が、参照 0 になる `FreeLibrary` を実際に呼ぶ経路に到達したとき。呼び出し元、直後の使い方（`GetModuleHandleW` で再確認するか、再ロードするか、など）を見て、別の決定で扱いを決める（実在モジュールのみ DETACH して未ロードに戻すか、イメージの再初期化をするかを含む）。
- **D6**: DLL が実行経路で自身のハンドルを渡して `GetModuleFileNameW` を呼ぶ地点に到達したとき。呼び出し側の使い方を読んで、別の決定で値を決める。
- **DllMain が FALSE のときの last error**: Learn が `LoadLibrary` の失敗時の last error を書いたとき、または実機の測定が DllMain の設定値と違う値を示したとき。実 DLL の DllMain が FALSE を返す経路に届き、ゲストが `GetLastError` を読むとき。
- **DllMain 内の同名 `LoadLibraryW`**: 実 DLL の DllMain がこれを呼ぶことが確認されたとき。
- PM が方針を変えたとき。

## 実装（この決定ではしない）

- W6（決定 0078）の完了後に、別作業で行う。
- 試作は `lift/out-stage1/PATCH-PROPOSAL.md` と `patch-rt-s1.diff`（`rt-base` と `rt-s1` の差。段階 1 §7.1）。対象は `fle_load.c`、`fle_modules.c`、新規 `fle_realmod.c`、`fle_realmod.h`。`fle_modules.c`、`fle_modules.h` は W6 と重なる（決定 0081 の表の末尾への追加とも重なる）ので、**そのまま当てず、当て直しが必要**。
- 試作のうち `S1_UNLOAD_AT_ZERO` は、この決定により採用しない。
- 既存テストの固定数（決定 0081 の実装の節に書いたもの）の扱いは、0081 の実装と合わせて行う。
- この決定の作業では `lift/` 配下を一切編集していない。

## 限界

- 根拠は段階 1 の**合成 PE・native** の測定と、調査・設計文書の読みに限る。実 DLL、wasm、実機 Windows は未確認。
- 段階 1 が未確認と書く点はそのまま残る: `GetProcAddress` の序数指定とフォワーダ、複数の実在モジュール（`RM_MAX` 4）、基底 0x400000 との衝突、FLEngine の実イメージとの重なり、`S1_DETACH_ON_FAIL=0`（段階 1 §9）。
- DLL 自身が参照 0 の `FreeLibrary` を呼ぶ経路は、調査 §2.2 の呼び出し点の表の 7〜10 に `FreeLibrary` があるが、どれが実行に届くかは確認されていない（10 の `0x17d7768` は「呼ばれ方は読み切っていない」）。決定 0081 の API セット名の仮想モジュールに対する `FreeLibrary` の到達も同様に未確認。
