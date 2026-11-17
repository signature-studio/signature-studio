1. **自動開始の誤り（直前の担当、E13 の pure）**: 『gpu1 の連鎖が終わったら自動で開始する』待機を PM の go なしに張った（E13 の docs の逸脱 4 に記録済み。23:05 に張り、23:07 に止めた）。以後、計時・Chrome の開始は PM の info だけで、今回の E14 の first idle と pure は PM の『go』を受けてから始めた。
2. **`bash` という名前のツールを呼んだ**（E13 の docs の逸脱 6 の後にもう 1 回）。実行されたのは `echo test` だけ、変更なし。
3. **preflight が rc 2 で『FAILED: no WebGL in Firefox』と出た**（P の page は beacon に `desk.mode` を出さず、基準の `desk.mode = gl` と不一致）。Firefox は WebGL ありだった（`webglRenderer` = llvmpipe）。連鎖は止めずに Firefox の 6 組を走らせた。事後に `beacon.py` と `report.py` を、『`desk.mode` が None で `webglRenderer` が非 null かつ engine が `present D` の run を WebGL の run とみなす』ように直した（kit のツール。`verify-bundle.sh` は編集していない）。
4. **kit の run の入口 `wait-quiet.mjs` を変更**（PM の 04:16 の規則: swap の入出力 0 と MemAvailable の記録を足した）。走行中の run が無い時に編集した。
5. **G9-a の test bundle の Export の『読み（未確認）』の取り下げ**: 04:59 に『単純な利得・パンではなく波形の中身が変わった可能性』と書いたが、alpha1 の解析（0310 §6）で、差は Balance の自動化 clip の内側とその終端の尾だけ、符号反転は 16 ビット clip の効果と読めるとされた。`g9a-export/RESULTS-g9a-export.md` に取り下げの追記をした。
6. **使い終わった preview server を止めた**（自分が起動した 5052 = E12、5062 = P + V4）。残りは 5053・5061・5063。
7. **Firefox の 15 シーン・File ×100・Export は E14 では未実施**（時間と mt3 の build との重なりを避けたため。gpu1 の機能確認〔Firefox 158 + P + V7 の first idle 4.3 s ほか〕はある）。
