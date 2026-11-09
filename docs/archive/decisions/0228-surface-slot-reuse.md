# 0228: 面の表(`lift/harness/fruity_surface.c`)の欄を再利用する(File メニューなどの窓が約 30 回でページに出なくなる不具合)

状態: **仮**(2026-11-04)。担当: ci1(原因の特定と差分)、PM(取り込み)。詳細: `docs/investigations/ci1/popup-surface-slots.md`。

## 結論
- **不具合:** ページの面の表は 32 欄で、消えた窓の欄を返さなかった。メニュー・ダイアログ・ツールチップは開くたびに新しい面を使うので、主窓とウィザードの後なら約 29 回目で欄が尽き、以後に開く窓が画面に出ない。エンジンは止まらず、窓表には窓がある。
- **退行ではない:** 本番の bundle(main 15540a0)でも旧 SHIP2 でも、5 回目までは popup が描かれ、40 回の開閉の後は描かれない。
- **FL 固有ではない:** 面の表はハーネスの登録簿で、クラス名・窓の種類での分岐はない。layered の経路でもない(popup の exstyle は 0x10008)。
- **直し:** 新しい view は、まず空き欄(`flags & 1 == 0` の最小の欄)を再利用し、無ければ表を伸ばす。欄が死ぬとき `view_of[k] = NULL`(解放されたアドレスに来た新しい view が死んだ欄に一致して書き込みが全部捨てられる別の欠陥も直る)。
- **試験:** 新規の単体試験 `lift/harness/web/present/test_fruity_surface_slots.c`(20 項目)。元の `fruity_surface.c` は 11 項目が失敗(33 枚目の窓で `no entry for lid 33`)、直した版は 20/20、既存の `test_fruity_surface_layer.c` は 27/27。
- **残る制限(直さない):** 同時に生きている窓が 32 枚を超える場合の 33 枚目は出ない。`LID_SCAN_MAX 4096`: lid が 4095 を超える窓は lid 0xFFFFFFFF で記録される(メニューを約 4,000 回開くセッション)。

## PM の判断と取り込み(2026-11-04)
- HEAD 6f3a6f8 に `ci1-popup.diff`(3 ファイル、`-` 2 行を読んだ)を適用。desk1 の変更とは重ならない(`fruity_surface.c`・`web/present/` に触れない)。desk1 が採用された場合(`?present=D`)、この表は読まれなくなる。
- bundle での 100 回開閉の確認(Chrome 156・Firefox 158)は ci1 が続ける。次の bundle(CD)にこの修正が入る。
