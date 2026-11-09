# PM の注記: Serum 2 のサイズ

`docs/investigations/reference-project.md` と `tables/plugins.tsv` が書く Serum 2 の「ini 上のサイズ 39,121,408 B」は、`FL_STUDIO_.Plugins.ini` の `Generators\VST3\Serum 2` の記録(2024-11-28 の走査、旧版)である。同じ ini の `Effects\VST3\Serum 2 FX` の記録(2026-07-07 の走査)は 19,236,864 B で、`Install_Xfer_Serum2_2.1.5.exe`(NSIS-3 Unicode、7z で一覧・抽出。実行なし)から取り出した `Serum2.vst3` の実サイズと一致する(VST3 調査担当 vst1 の report 1、2026-11-03)。**現行 2.1.5 の本体は 19.2 MB**で、39 MB は旧版の記録が残っていたもの(推測)。性能・サイズの見積りには 19,236,864 B を使う。
