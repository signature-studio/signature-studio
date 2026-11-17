# 0417: 上流 Blend2D の CMake(emcmake・cmake --build)をやめ、em++ と emar で fe-build が直接ビルドする(CMake は採らない)
分類: (a) 除去(上流の CMake の configure と ninja の build を外す)。新しく入るのは、ninja -t commands の行から写した em++ の 24 フラグ(値は同じ)と、159 件の一覧(データ)と、一覧が src の .cpp の集合と一致するかの確かめ(CheckSources)。
撤回条件: (1) 最終の engine.wasm の sha256 が lift/harness/web/verified.txt の b440e13a623a と一致しないとき(最初に違う .o か libblend2d.a を cmp で探し、本決定を見直す)。(2) 依頼者が em++ と emar の直接の起動を禁じたとき。(3) 上流の .cpp の集合が一覧と合わなくなり CheckSources が止まったとき(一覧を作り直す。ninja の順は失われるので、順も確かめる)。
日付: 2026-11-17   ROADMAP: R6-36c(doing)   一覧: なし   証跡: `evidence/R6-36c.md`   コード: lift/crates/fe-build/src/main.rs(branch r36b の 12e19230 と 5586dcae)、データ: lift/harness/b2d1/blend2d-sources.txt
番号の理由: origin/main の `docs/decisions/` の最後は 0416(2026-11-17 の git ls-tree)。0417 が最初の空き。
改訂の経緯: 0412 §5-1 は、上流 Blend2D の CMake を fe-build から呼ぶことを、依頼者の CMake 却下(原文「流石にCMakeなんて醜いものは採用しないですよね？」。main 経由。発言 ID は未確認)に当たるかどうか未確認のまま残していた。本決定は、当たる側に倒して CMake を外す(0412 §5-1 の撤回条件 (2) に当たる)。

## 0 範囲と件数(設計の前の確認)

| 範囲 | 確認 | 件数と結果 |
|---|---|---|
| CMake の build の全行 | ビルドの出力の `ninja -C lift/out-app/b2d/build -t commands`(evidence/R6-36c.md §1) | 160 行。compile 159 行(em++)と archive 1 行(`cmake -E rm`・`emar qc`・`emranlib`) |
| 生成物 | build dir の一覧 | CMakeCache・CMakeFiles・build.ninja・cmake_install.cmake・libblend2d.a。生成ヘッダは無い(compile に -I が無い) |
| ソースの集合 | ビルドの出力を `find lift/out-app/b2d-src/src -name '*.cpp'` と ninja の入力を diff | 159 件。diff 0 行(除外は無い。JIT の .cpp も含む) |
| アーカイブの順 | ninja の emar 行の順と、パスの昇順を比べた | 違う(`pixelops/funcs.cpp` が `interpolation_sse2` の後など。上流の CMakeLists の並び)。順は一覧に持つ |
| フラグ | em++ 行の -MD・-MT・-MF・-o・-c を除いた列 | 24 トークン。順も含めて同じ値を fe-build に写した(空白の正規化後に diff 0) |

## 1 決定

1. fe-build は、emcmake と cmake と ninja を呼ばない。em++ で 159 件を `-c <src> -o lift/out-app/b2d/obj/<相対パス>.o` で作り、`emar qc lift/out-app/b2d/build/libblend2d.a <objects>` と `emranlib` で archive を作る(いずれも emsdk の PATH の道具。CI の setup-emsdk が PATH に置く)。
2. 順は一覧(lift/harness/b2d1/blend2d-sources.txt。ninja の順)に従う。archive の部材は basename の `x.cpp.o` で、CMake の版と同じ。
3. 一覧が src の .cpp の集合と一致しないと、CheckSources が止める(ビルドは続けない)。
4. 上流の git clone・checkout・patch(b2d3-compop.patch)は変えない。

分類の理由: CMake の configure と build を外す(a)。em++ の 24 フラグは、CMake が出した値をそのまま写した(新しい処理ではない)。

## 2 検査(実測。出力は evidence/R6-36c.md)

- `engine --plan` の差分: 変わったのは blend2d の段だけ(差分の hunk は 1 つ。CMake の 2 行が消え、check・mkdir・159 件の em++・emar の行が増えた)。他の段は diff 0。
- 5 件の .o(api-globals・api-nocxx・array・array_test・bitarray)を、plan と同じ em++ の行で作り、CMake 版の .o と cmp でバイト一致。
- 全体の sha256(フルビルド)は evidence/R6-36c.md §C。b440e13a623a と一致。

## 3 未確認

- 159 件の全 .o と libblend2d.a の内容の一致(5 件だけ cmp した。libblend2d.a の member の順は一覧で保証し、中身は未確認)。
- engine.wasm の sha256 は b440e13a623a と一致した(2026-11-17 10:03〜10:15 UTC の engine の段。証跡 evidence/R6-36c.md §C)。
- 0412 §5-1 の CMake の扱いは、本決定で「当たる側に倒した」。CMake を許す判断が main から出た場合は、撤回条件に当たるかどうかを main が見る。
