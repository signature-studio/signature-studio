# 0396: G9 の Fruity Balance と Fruity Filter を実モジュールとして取り込む(RM_MAX 16。Problems 窓の件数は 8 から 6)
分類: (b) API 拡充(LoadLibraryW のフルパスと GetProcAddress("CreatePlugInstance") の解決を、実 DLL のモジュールで満たす。モジュールの表・RM_MAX・カーソルの上限は、その手続きの容量の定数)。追加行の getenv・#ifdef・URL パラメータは 0(§3)。
撤回条件: (1) 所定プロジェクトの再生・Export で `fle: unsupported` か名前つきの停止が出る(止まった VA を API 名に直し、契約を実装する。実装できない間は該当の module を外す版を測る)。(2) Wine の同じ操作の Export と比べて Filter の効果が一致しないと分かったら、lift の欠陥か契約かを分けて再判定する。(3) 取り込んだ wasm の増分が S1 の判定(R5-6)で許容を超えたら、静的結合を見直す(0313 §18.6 の O-2)。
日付: 2026-11-16   ROADMAP: R5-5   一覧: G9   コミット: d935286..8b5ff31 の R5-5 の 6 件(6abb3a4・b7b7b5f・e84a13f・9fbf6fb・4da68cb・8b5ff31)

## 決定
- 決定: 連鎖 `0314v1-lazy-vfs-runtime` → `0314v1-lazy-vfs-host` → `0313-g9a+filter-combined` の lift/・app/ の部分を取り込む。ci/ の 5 ファイル(extract-dlls.tsv・objkeys.sh・stage-bundle.sh・stage-lift.sh・test-hybrid.sh)は取り込まない(ROADMAP R5-5)。根拠: ci/ は CI の段の表で、R5-5 は lift・app の実モジュール化だけを対象にする。
- 決定: 連鎖に `0313f2-cursor-realmod-cap` を足す(6 件のうち e84a13f)。根拠: RM_MAX を 16 にすると 9 本目の Filter(index 9)の DestroyCursor が fle_cursor.c の `off / 0x1000 > 8u` で弾かれ、File→Open で停止する(0313 §21 の実測)。
- 決定: DLL の抽出は、ci/extract-dlls.tsv を変えずに、同じ 2 行を入れた一時の TSV で `ci/lib/extract_fl.py` に渡した(2 DLL と seed 226 件、SHA-256 全一致)。根拠: ci/ を取り込まない方針。後に別 worker が同じ 2 行を a2518b1 で入れた(抽出の内容は同じ SHA)。
- 決定: 取り込み後の直し 3 件(分類は全て (b) の文書上の手続き。挙動は変えない): (i) app/test/unit/lazy-tree.test.ts の型(opts に wrongLength を足す。tsc が通らなかった)、(ii) fle_vfs.c の仮想時計の変数 clock_now・clock_saved を vtime_now・vtime_saved に改名(wall_r.rs の『新しい runtime コードは host を読まない』の語句検査が clock の語に当たった。仮想時計の読みで、ホストの時計は読まない)、(iii) wall_dec1.rs の RM_MAX の期待 8 を 16 に(0315 の該当の 2 箇所だけを当てた)。
- 決定: bundle は commit 済みの HEAD(9135826)の専用ワークツリーから作った。共有のワークツリーの app/scripts/build-engine-bundle.sh に別 worker の未 commit の編集があり、1 回目の build は途中で書き換わり bundle 段が rc 2 で止まった(捨てた。ログは evidence/R5-5.md §5)。

## 1. 検索の範囲と件数(設計の前の確認)
- 読んだ差分: 0314v1-lazy-vfs-runtime 1,223 行・host 1,238 行・0313-g9a+filter-combined 365 行(ci/ 5 ファイル 56 行を含む)・0313f2 65 行。
- 旧決定・旧記録: docs/archive/decisions/ で 0313・0314・G9・Balance・Filter・lazy に一致するファイル 88 件(`grep -rl`)。中心は 0313-g9a-balance-module.md(846 行)。
- 依頼者の発言: ~/work/handover/setup/S16-intent/catalog.md の 84 行中、G9・同梱・プラグイン・plugin・Problems・Balance・Filter・Fruity に一致は 1 行(No. 28、msg_154b82f28001ucbVLduEYx7ziF 00:11:58 の「なぜプラグインが機能していないのですか？」。プラグインの一部を指すと読める)。

## 2. getenv・#ifdef の分類(取り込みの追加行。grep -c の結果)
| 差分 | 追加行の数 | 分類 |
|---|---|---|
| 0314v1-lazy-vfs-runtime | 1(新規 lift/harness/vfs_lazy.h の `#ifndef FLE_HARNESS_VFS_LAZY_H`) | インクルードガード。製品の動作を分けない |
| 0314v1-lazy-vfs-host | 0 | — |
| 0313-g9a+filter-combined(lift・app 部分) | 0 | — |
| 0313f2 | 0 | — |
| 取り込み後の直し(i)〜(iii) | 0 | — |
新しい旗・環境変数・ビルド変数は足していない。

## 3. 適用の結果
- dry-run(HEAD d935286 に対して): lazy runtime rc 0、lazy host rc 0、combined の 12 ファイル rc 0。app/scripts/build-engine-bundle.sh は hunk 4 が fuzz 1 で当たった(HEAD の PRUNE_LIST 行が lift/data/prune-SM.tsv で、差分の文脈は docs/investigations/prune1/…。変わるのは EXTRA_MODULES 行だけで、適用後の git diff で確認)。vfs-seed-idle1.tsv は offset 3。0313f2 rc 0。
- 適用は 6 件の commit(§0)。ci/ の 5 ファイルは適用していない。

## 4. 実測: テスト
- cargo test(CARGO_TARGET_DIR=~/.cache/signature-studio/cargo-r55、FLENGINE_PE は binaries/)。fe-lift 2,133 通過・失敗 1(getmodulehandleexw_returns_in_every_build、既知)。fe-pe・fe-decode・fe-wasmgen 294 通過・失敗 0・無視 1(simd_int_…、既知)。合計 2,427 通過・既知の失敗 1・無視 1。基準 evidence/R5-3m-cargo.txt は 2,426・1・1。新しい失敗 0。
- 直す前の 2 件の失敗(新しい失敗): rm_max_is_8(RM_MAX 16 の期待)と the_new_runtime_code_reads_nothing_of_the_host(clock の語)。(ii)(iii) で直し、再実行で通過。
- npm test: 66 files passed・1 skipped(1,308 通過・9 スキップ。lazy-tree の 2 件は opt1 の資材が無いための describe.skipIf、残りは既存の plugins-ott 系)。npm run build(型検査と vite build)rc 0。

## 5. 実測: bundle(§0 の決定。evidence/R5-5.md §5)
- WORK=work-r55b、HOST_ROOT=artifacts/host、JOBS=2、B2D は既存の libblend2d.a。07:36:40Z〜08:02:20Z(25 分 40 秒)。最小の MemAvailable 12,576 MB(memguard 32 点)。
- engine の wasm(B): 取り込み後 158,494,081 B、取り込み前(work-r53k、HEAD 9610708)143,118,360 B。増分 15,375,721 B。比の式: 158,494,081 ÷ 143,118,360 = 1.1074(R5-6 の入力)。
- dll pack: 84,943,680 B(前 77,444,112 B、増分 7,499,568 B = 2 DLL の PE 7,499,264 B + 304 B)。seed pack: 17,048,537 B(前 9,548,386 B、増分 7,500,151 B)。.br(q11): wasm 17,017,790 B(前 15,710,596 B)・dll 17,030,390 B(前 15,953,166 B)・seed 6,471,243 B(前 5,230,435 B)。
- SHA256SUMS 9 件 OK(両方)。realmod 表 9 件、パス表 2 件(生成 C で確認)。manifest の説明文の `modules=8` は build-engine-bundle.sh の固定文字列(line 436。数えていない。realmod は 9)。

## 6. 実測: 観測(evidence/R5-5.md §6)
- Problems 窓(missing-plugins-dialog.png を目視): 取り込み前(work-r53k)8 件・8 件・8 件・8 件(うち条件を満たした 3 件は 8)。取り込み後(work-r55b)6 件・6 件・6 件・6 件・6 件(条件を満たした 3 件は 6)。取り込み前の一覧は Balance と Filter を含み、後の一覧はどちらも含まない。他の 6 件(Fruity Wrapper・FLEX・Frequency Shifter・Soundgoodizer・Fruity Parametric EQ 2・Fruity Delay 2)は両方で同じ。
- 再生 10 秒: 止まらず(stopped false、opErrors 空、engine-idle)。ログの増分 0 行(fle: 0 行)。再生中の画(play-10s-playing.png)で 9:04:06 まで進み、メーターが動いている(目視)。
- Export: 取り込み後 22,863,828 B・sha256 94c9366687076a410f48c6fbe1485e8e22f6b528592fcc5ceba2d665d1554090。取り込み前は R5-4 の 381cdb5a…f979(out/r54-r54r53k-chrome-1 で再確認)。値は変わった。正しい音かは判定しない。

## 7. 未確認
- Filter の Export が Wine と一致するか(旧: RMS 5,325.57)は未確認(Wine の比較は 0 件)。0313 §21 の記録では、この FLP の Export は Balance だけの版と同じ sha で、Filter の効果は WAV に出ていない(読み)。
- P1〜P3 は測っていない(P2 の 120 fps・P3 の音切れ・依頼者機は 0 件)。
- Balance・Filter のプラグイン窓(GUI)は開いていない。再生の動作は Insert に載った状態でだけ確認した。
- 0313 の Balance の窓の client が描かれない件(0313 §15)は未確認のまま。

## 8. 出典
- patches/mt3w/0313/impl/0313-g9a+filter-combined.diff・0313f2-cursor-realmod-cap.diff・0314v1-lazy-vfs-{runtime,host}.diff・patches/mt3w/0315/impl/0315-rtb-test-expectations.diff(wall_dec1.rs の 2 箇所)
- docs/archive/decisions/0313-g9a-balance-module.md §10.5・§14・§17・§21・追記 2026-11-07 09:48
- ~/work/handover/setup/S16-intent/catalog.md No. 28
