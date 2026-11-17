# 0413: emscripten と zig cc（WASI）の比較の調査。製品は変えない

分類: 調査（製品の変更なし。依頼: 依頼者 2026-11-17「なぜemscriptenを採用しているのですか？ ... zig ccでwasiにすれば良いだけでは？」）。
決定: 製品（lift/・app/・.github/・ci/）は変えない。移行の採否は main が決める。根拠: 依頼文は事実の確認の依頼で、製品の変更を求めていない（依頼者は emscripten を選んでいない。§1.2）。
撤回条件: (1) 依頼者が移行を指示したとき（その決定を別の番号で書く）。(2) zig の版が 0.18.0 以降に上がり、evidence/R6-38.md §3 の結果が変わるとき（テストを取り直す）。(3) §0 の件数に誤りが見つかったとき（件数を直して書き直す）。
詳細と出典: evidence/R6-38.md（コマンドと出力。テスト用のソースは evidence/R6-38/probes/）。

## 0 範囲と件数（先頭）

- 記録: emscripten・emcc・emsdk を含む決定文書 140 件（docs/archive/decisions と docs/decisions）。最初の出現は 0043（「最終目的は C → emcc → wasm」）。0001〜0042 は 0 件。zig は 0 件。wasi は 2 件（0173・0177。ツールチェーンの選択ではない）。
- 依頼者の発言: 263 件のうち、emscripten は 1 件（seq 34694。制約かどうかの問い）。emcc・zig・wasi・clang・llvm は 0 件。
- 棚卸し（本番の入力）: runtime の fle_*.c 90 件のうち、emscripten の語を含むものは 12 件、emscripten の API を直接使うものは 8 件。harness の本番の入力 12 件のうち 8 件。生成 C 65 件は 0 件。
- API の件数: EMSCRIPTEN_KEEPALIVE（export）25 関数（runtime 13、harness 12）。EM_JS 10 件（engine の import に 9 件）。emscripten_builtin_* 6 行。emscripten_stack_* 5 行。
- JS の本番: `--js-library` の 3 ファイル 180 行、8 関数（補助 3）。
- リンク: link.rsp の -s 17 件、-Wl 9 件、--js-library 3 件。exports.rsp の --export-if-defined 2,269 件。
- engine.wasm の import: 48 件（env 39、wasi_snapshot_preview1 9）。
- app/src: emscripten・emcc・EM_JS の語を含むファイル 10 件、31 行。
- zig: 0.17.1（公式の一覧の最新の安定版。SHA-256 を照合）。テスト: 重いコンパイル 4 件、小さい probe 5 件。

## 1 依頼への答え（事実）

### 1.1 なぜ emscripten か

- 記録に、emscripten を選んだ理由も、zig・WASI と比べた記録も無い。0043 は目的として「C → emcc → wasm」を書いた。emcc を名指しした基準（A3）は旧の依頼文に由来し、原文はリポジトリに無い（決定 0046 §1）。版（emsdk 6.0.12）は旧プロジェクトの ci/versions.env から引き継いだ。

### 1.2 依頼者の発言

- emscripten に触れる発言は 1 件（seq 34694）。依頼者が emscripten を選ぶ、または名指しで指定した発言は無い。

### 1.3 zig cc + WASI で足りるか（テストした範囲の事実）

- 足りない箇所（テストで確かめた）: (a) setjmp/longjmp は zig 0.17.1 の wasm32-wasi でリンクできない（`__c_longjmp` が未定義。zig の lib に定義 0 件）。(b) OS スレッドは作れない（pthread_create が EAGAIN）。(c) zig cc は本番のリンク引数のうち -s 17 件、--allow-undefined、--export-if-defined、-Map を拒否する（zig wasm-ld を直接呼べば 4 種類とも受理）。(d) emscripten の API（EM_JS 10、EMSCRIPTEN_KEEPALIVE 25、emscripten_stack_* 5 行）と glue（engine-glue.js 15,898 行）を、ホスト（JS）と宣言で置き換える必要がある。
- 通った箇所（テストで確かめた）: C の単位 4 件（fle_sse_more.c、fle_thread.c、fle_thread_wasm.c、生成 C の m6-02.c）はコンパイルが通る。ただし setjmp の #error を外すための -mexception-handling -mllvm -wasm-enable-sjlj と、emscripten.h の宣言だけのスタブが要る。malloc の 16 バイト整列（aligned_alloc・posix_memalign）は実行で確かめた。node の WASI で hello・align・thr を実行した。

## 2 比較（充足・しない・未確認）

| 軸 | emscripten（現行） | zig cc + WASI（0.17.1） | 判定 |
|---|---|---|---|
| C のコンパイル | 充足（本番） | 充足（4 単位。sjlj のフラグとスタブが要る） | 充足（4 単位の範囲） |
| setjmp/longjmp | 充足（SUPPORT_LONGJMP=wasm） | しない（リンクで __c_longjmp 未定義） | しない |
| OS スレッド | 充足（-pthread。guest thread は JSPI で 1 本の wasm スレッド上） | しない（pthread_create が EAGAIN） | しない（engine が OS スレッドを要るかは未確認） |
| JSPI | 充足（-sJSPI） | 未確認（JS の API。ホストで包む。テストしていない） | 未確認 |
| メモリの増加 | 充足 | 受理のみ（--initial-memory・--max-memory。増加の実行は未確認） | 未確認 |
| リンク引数 | 充足（link.rsp） | zig cc 経由はしない。zig wasm-ld 直接は受理（完全なリンクは未確認） | 未確認 |
| リンクマップ（fe-wasmgen の入力） | 充足（-Wl,-Map） | zig cc 経由はしない。zig wasm-ld --Map は受理 | 未確認（形式の互換） |
| JS の glue | 生成物（engine-glue.js 15,898 行） | 自前（未作成。推測: 数百行） | 未確認 |
| 導入 | emsdk 6.0.12（setup-emsdk v16） | zig の tarball 1 つ（57,411,036 B） | 事実 |
| Cargo との接続 | C は Cargo の外（deploy.yml） | cc クレート 1.11.1 は CC の空白分割で zig cc を渡せる。lift に cc 依存は無い | 未確認（実ビルドは未テスト） |
| 出力の同一性 | 現行の鍵 b440e13a623a | 変わる（一般論。未測定） | 未確認 |
| 性能（P1〜P3） | 計測済み（別の記録） | 未測定 | 未確認 |
| 版の将来 | — | 0.17.0 のノートの予告: 0.18.0 で WASI の libc の提供元が変わる | 未確認 |

## 3 未確認（この調査で確かめていない物）

1. JSPI・共有メモリの実行。zig 出力で engine を動かした結果。本番の入力の全部のコンパイルとリンク。
2. setjmp のリンクの回避策。-pthread を外した場合の engine の動作。
3. 出力の同一性と性能（P1〜P3・S1）。
4. zig 0.18.0 の WASI libc の扱い（0.17.0 のノートの予告のみ）。
5. blend2d（CMake・emcmake）の zig 版。
6. 依頼者が emscripten を選んだ理由（記録にも発言にも無い）。
