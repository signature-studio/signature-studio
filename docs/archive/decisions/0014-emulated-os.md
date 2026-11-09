# 0014: エミュレートする Windows（VerifyVersionInfoW、GetVersion、GetVersionExW）

状態: PM が決定（エミュレート OS の値と方針）。仕様の読みと実装は作業者（M1 作業者）。
この決定は 0012（VerifyVersionInfoW の応答を作らず停止のままにする保留）を解く。0012 のファイルは書き換えていない。0012 が「候補 (a) 固定値」を退けた理由（固定の真偽値は仕様にない主張になる）は、この決定でも守る。応答は固定の真偽値ではなく、1 つの OS 定数群に対して仕様どおりに動く。

## 決定（PM。変更しない）

- lift 環境は「エミュレートする Windows」を持つ。VerifyVersionInfoW、GetVersion、GetVersionExW は、その 1 つの定数群に対して仕様どおりに動く。
- エミュレート OS: Windows 10 22H2。dwMajorVersion=10、dwMinorVersion=0、dwBuildNumber=19045、dwPlatformId=2（VER_PLATFORM_WIN32_NT）、szCSDVersion=""、wServicePackMajor=0、wServicePackMinor=0、wSuiteMask=0、wProductType=1（VER_NT_WORKSTATION）。
- 定数は runtime の 1 か所に置く: `lift/runtime/fle_osver.c` の `fle_emulated_os`（型は `lift/runtime/fle_osver.h` の `fle_osver_t`）。変更が必要になったら、ここだけ直す。テスト `emulated_os_defined_once` が、他のランタイムファイルに 19045 がないことを確かめる。
- 実装は `fle_osver.c`（新規）、`fle_imports.c` に `fle_set_last_error` の追記、対応表（`imports_table.rs`）に 3 件追記。

## 仕様の出典（2026-10-20 取得、Microsoft Learn）

- VerifyVersionInfoW: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-verifyversioninfow
  - 引数: lpVersionInformation（OSVERSIONINFOEXW*。dwOSVersionInfoSize を sizeof(OSVERSIONINFOEX) にしなければならない。dwTypeMask のビットが立っていないメンバーは無視される）、dwTypeMask（0x1 MINOR、0x2 MAJOR、0x4 BUILD、0x8 PLATFORMID、0x10 SPMINOR、0x20 SPMAJOR、0x40 SUITENAME、0x80 PRODUCT_TYPE）、dwlConditionMask。
  - 戻り値: 満たすと非 0。満たさないと 0 で GetLastError は ERROR_OLD_WIN_VERSION。その他の失敗は 0 で別のエラー。
  - Remarks: 通常は指定した全テストが成功したときだけ非 0。major、minor、service pack は階層的に、この順でテストする。「値が等しい間はテストを続け、条件を満たさない値で止まる」。例: 5.1 SP1 以上のテストは 6.0 で成功、5.2 で成功、5.0 SP2 で失敗。major の条件は minor と service pack の条件より優先される（「major が 5 より大きい、かつ minor が 1 以下」はテストできず、minor の greater than に変えられる、とある）。
- VerSetConditionMask: https://learn.microsoft.com/en-us/windows/win32/api/winnt/nf-winnt-versetconditionmask （Condition の値 1 から 5、SUITENAME は 6（AND）と 7（OR））。
- OSVERSIONINFOW: https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-osversioninfow
- OSVERSIONINFOEXW: https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-osversioninfoexw
  - メンバーの型と順序: DWORD dwOSVersionInfoSize、DWORD dwMajorVersion、DWORD dwMinorVersion、DWORD dwBuildNumber、DWORD dwPlatformId、WCHAR szCSDVersion[128]、（EXW のみ）WORD wServicePackMajor、WORD wServicePackMinor、WORD wSuiteMask、BYTE wProductType、BYTE wReserved。
  - オフセット（ページには数値が書かれていない。型から計算した値）: 0、4、8、12、16、20、（EXW）276、278、280、282、283。OSVERSIONINFOW は 276 バイト、OSVERSIONINFOEXW は 284（0x11c）バイト。0x11c は FLEngine 自身が dwOSVersionInfoSize に書く値（f_409b30、0x409b56 の `movl $0x11c,0x24(%rbp)`）と一致する（実測の読み）。パディングの有無は型から導いた（ページには書かれていない）。
- GetVersion: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getversion
  - 戻り値は DWORD。下位ワードに major（下位バイト）と minor（上位バイト）。上位ワードにプラットフォーム情報。Examples のコードは `dwVersion < 0x80000000` のとき上位ワードをビルド番号として読む。
  - ページの本文に「high-order bit is zero, the next 7 bits represent the build number, and the low-order byte is 5」という文もあるが、Examples のコードと整合しない。実装は Examples のコードの読み方（上位ワード = ビルド番号、bit 31 = 0）に従った。本文の文は使っていない。
  - 値: build 19045 = 0x4A65、minor 0、major 10 から 0x4A65000A。
- GetVersionExW: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getversionexw
  - 引数は OSVERSIONINFOW または OSVERSIONINFOEXW。呼ぶ前に dwOSVersionInfoSize を、渡す構造体に合わせて設定する。成功で非 0。失敗で 0（GetLastError で拡張エラー）。dwOSVersionInfoSize が不正なときに失敗する。

## 実装した規則と、仕様にない点

- 階層比較（major、minor、SP major、SP minor、実際にテストされたものだけ）: この順に見て、値が等しい間は条件を評価して続け、値が違う最初のメンバーで、その条件の成否が結果になって止まる。Remarks の 3 つの例（6.0、5.2、5.0 SP2）で確かめた（テスト `hierarchy_examples_from_remarks`）。
- BUILD、PLATFORMID、SUITENAME、PRODUCT_TYPE は、それぞれ単独で比べる。SUITENAME の AND は、指定した全ビットが現在にあること、OR は少なくとも 1 つあること（VerSetConditionMask のページの表）。全テストが成功したときだけ非 0。
- 失敗時: 戻り値 0、`fle_last_error` を 1150（ERROR_OLD_WIN_VERSION）にする。成功時は last error に触れない（ページに書かれていないため）。
- 仕様のページで確認できなかった点（黙って成功にせず、`abort` で止める。stderr に理由を出す）:
  - dwTypeMask が 0、または 0xFF の外のビットがある。
  - lpVersionInformation が NULL。
  - dwOSVersionInfoSize が sizeof(OSVERSIONINFOEXW)（284）以外（ページは「しなければならない」と書くが、違うときの戻り値は書かない）。
  - 比較の条件が 1 から 5 の外（SUITENAME は 6、7 以外）。VER_SUITENAME で wSuiteMask が 0。
  - 階層のメンバー（実際にテストされるもの）のうち、隣り合う 2 つの条件が違い、上位の条件が VER_EQUAL ではない（例: major の GREATER と minor の LESS_EQUAL）。Remarks は「major の条件が優先され、要求を変える」と書くが、変えた後のテストの定義は書かない。上位が VER_EQUAL の場合は、下位の条件は上位が等しいときだけ評価されるので定義できる（Remarks の例の規則）。
- 実装しない点（仕様が結果を書かない）: dwTypeMask の MAJORVERSION の説明は「major をテストするなら minor と SP major、SP minor も必ずテストしなければならない」と書く。テストしなかったときの結果をページは書かず、「ビットが立っていないメンバーは無視」とも書くため、テストされないメンバーは飛ばす。FLEngine の呼び出し（dwTypeMask 0x23）は SP minor を含まない。
- 条件マスクのビット配置（3 bit × 3k）は公式ページにない。VerSetConditionMask（0011）と同じ配置で読む。0011 のとおり、Wine と mingw-w64 に基づく（Windows 本体との一致は未確認）。
- GetVersionExW の成功時: 共通の 4 DWORD と szCSDVersion（全 256 バイトを 0）を書く。EXW のときは SP major、SP minor、suite、product type も書く。dwOSVersionInfoSize と wReserved は書き換えない。dwOSVersionInfoSize が 276、284 以外のとき、0 を返し last error を 87（ERROR_INVALID_PARAMETER）にする（コード 87 は作業者の選択。ページは名前を書かない）。NULL は定義がないので `abort`。
- GetVersion の値が DWORD に収まらない OS（major、minor が 255 より大きい、build が 0x7FFF より大きい）は `abort`。10.0.19045 では起きない。

## FL Studio の要件の確認

- 依頼の「image-line.com の FL Studio のシステム要件ページ」は、次の公式ページで確認した（2026-10-20 取得）。
  - https://support.image-line.com/action/knowledgebase?ans=82 （「What are the system requirements of FL Studio (Mac and Windows)?」）: Windows は「Windows 10 or 11」、4GB の空きディスク、4GB の RAM、Intel と AMD の CPU（ARM は動くがテスト対象外）。
  - https://www.image-line.com/fl-studio/download : FL Studio 26.1.7.5654 の Windows について「Windows 10, 11, or newer」。FAQ に「Windows 10 or 11, a 2 GHz processor with SSE2 support, 4 GB RAM, and 4 GB of free disk space」。
  - https://support.image-line.com/api.php?call=get_version_info の応答（検索結果の抜粋として確認。直接は取得していない）: Windows の system_requirements は「Windows 10 or later」。
- 試して 404 だったページ: https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/basics_systemrequirements.htm 。オンラインマニュアル側の要件ページは確認できていない。
- 結論: 公式の動作要件は「Windows 10 以降」。エミュレート OS（Windows 10 22H2）はこの要件を満たす。Windows 10 の特定のビルド（22H2 = 19045）を要件が名指しするかは、確認したページには書かれていない（未確認）。
- この `FLEngine_x64.dll` が FL Studio のどのバージョンのものかは確認していない（未確認）。要件ページが指すバージョンと、この DLL の要件は一致しない可能性がある。
- 推測（実測ではない）: f_409b30 の呼び出し（0x409bd0 から 6、1、1）は、major 6、minor 1、SP major 1 以上（Windows 7 SP1 以上）を見ている。10.0 SP0 は major が 10 > 6 なのでこの条件を満たす（実測: 実行がこの結果で先へ進んだ。osver-run.md 参照）。DLL の要件が公式の要件と同じかは、この呼び出しだけからは言えない。値は変えない。

## 限界

- ホストの EXE マニフェストによる挙動は未モデル化。公式ページによれば、Windows 8.1 または Windows 10 のマニフェストがないアプリでは、GetVersion、GetVersionEx、VerifyVersionInfo は Windows 8（6.2）として振る舞う。この環境は、ゲストがどのマニフェストを持つかを見ず、常に 10.0.19045 として答える。ゲストの EXE（ホスト側）が違うマニフェストを持つ場合に結果が違い得ることは、モデル化していない。なお f_409b30 の条件（6.1 SP1 以上）は 6.2 でも満たされるので、今の呼び出しの結果は変わらない（規則からの推論。6.2 での実行はしていない）。
- 互換モード（アプリの互換性設定）の影響も未モデル化。
- szCSDVersion は空、wSuiteMask は 0 とした（PM の決定どおり）。実際の Windows 10 は wSuiteMask に VER_SUITE_SINGLEUSERTS などを返し得る（Wine は 0x100 を使う。作業者の記憶ではなくこの回の取得結果: Wine の dlls/ntdll/version.c の WIN10 の行が VER_SUITE_SINGLEUSERTS）。PM の定数は 0 で、変えていない。SUITENAME を使う呼び出しが現れたときに再検討が必要になり得る。
- 条件マスクのビット配置は Wine と mingw-w64 由来で、Windows 本体との一致は未確認（0011 と同じ限界）。
- ホストが Linux のため、実際の Windows の戻り値との照合はできていない。テストの期待値は、仕様の規則を Rust の式で書いたもので、Windows の実測ではない。

## 0012 との関係

0012 は VerifyVersionInfoW の応答を作らず停止のままにした。理由は、FLEngine が要求する OS の状態を仕様から決められず、固定値は仕様にない主張になるためだった。この決定は、OS の状態そのものを 1 つの定数群として持ち、API が仕様どおりにそれを読むことで、その保留を解く。0012 の「固定値は不採用」は、この決定でも変わらない。

0012 に結び付いたテスト `lift/crates/fe-lift/tests/verify.rs` の `verify_version_info_thunk_stays_unsupported` は、この決定により成り立たなくなる（VerifyVersionInfoW が対応表に入ったため、`jmp_imp:VerifyVersionInfoW` が出る）。このファイルは本作業の担当外のため編集していない。更新が必要（osver-run.md に詳細）。
