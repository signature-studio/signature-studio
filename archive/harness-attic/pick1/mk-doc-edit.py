#!/usr/bin/env python3
# pick1 (decision 0242 addendum): the edits of docs/decisions/0242-fl-drawing-paths.md (the merged document) for the cr_plus_more correction; usage: mk-doc-edit.py MERGED.md SEC25.md OUT.md
import sys
src = open(sys.argv[1], encoding='utf-8').read(); sec = open(sys.argv[2], encoding='utf-8').read(); out = sys.argv[3]
def rep(old_start, new, whole_line=True):
    global src
    lines = src.split('\n'); hit = [i for i, l in enumerate(lines) if l.startswith(old_start)]
    assert len(hit) == 1, (old_start, hit)
    lines[hit[0]] = new; src = '\n'.join(lines)
rep('5. **`More plugins...`(チャンネルラックの + の最初の項目)でメイン窓が disabled のまま窓が出ない停止**',
    '5. **`More plugins...`(チャンネルラックの + の最初の項目)でメイン窓が disabled のまま窓が出ない停止は、原因を特定して直した**(§2.5): `+` のポップアップは FL が `ShowWindow(SW_SHOWNOACTIVATE)` で出し、続けて VCL が**既に見えているその窓に** `ShowWindow(SW_SHOWNORMAL)` を呼ぶ。実 Windows(と Wine 10.0 の実走)ではこれは何もしないが、我々の runtime は窓を活性化していた(契約の不一致 b1)。その結果 list 窓 `TPluginListForm` の owner が popup になり、約 820 ms 後の選択フェード終了の `DestroyWindow(popup)` で list 窓が道連れになっていた。**以前の版の『契約の不足は見つからなかった』は誤り**(記録器の行の並びを時系列と読み違えた)。修正は `fle_win.c` の `ShowWindow`(b1)と `DestroyWindow` の所有窓の再帰(b2)。')
# §2.4: the bullet that said no contract was missing
lines = src.split('\n')
for i, l in enumerate(lines):
    if l.startswith('- **契約の不足は見つからなかった**'):
        lines[i] = '- **(訂正済み。§2.5 を見る)** 以前の版はここで『契約の不足は見つからなかった』『popup は `SetFocus(popup)` で活性化された』と書いたが、**どちらも誤り**: 活性化したのは popup への 2 回目の `ShowWindow(SW_SHOWNORMAL)`(既に見えている窓への呼び出しで、実 Windows では何もしない)で、`SetFocus` はその後に来る。実機の FL が避けている仕組みは『popup が活性化されない』ことだった。'
src = '\n'.join(lines)
src = src.replace('、`cr_plus_more` で実 Windows が list 窓を守る仕組み、', '、b1 / b2(§2.5)が実 Windows でも Wine 10.0 と同じか(Wine は参照であって Windows ではない)、', 1)
lines = src.split('\n')
for i, l in enumerate(lines):
    if l.startswith('4. **未解決として引き継ぐ**: (a) `+` > More plugins…'):
        lines[i] = l.replace(l[l.index('(a) `+`'):l.index('(b) ピッカーの 62.5')], '(a) `+` > More plugins… の停止は §2.5 で解決(原因は `ShowWindow` の契約の不一致 b1、直しは b1 / b2)。', 1)
src = '\n'.join(lines)
marker = '\n## 3. '
i = src.index(marker) if marker in src else -1
assert i > 0, 'no section 3'
src = src[:i].rstrip('\n') + '\n\n' + sec.rstrip('\n') + '\n' + src[i:]
open(out, 'w', encoding='utf-8').write(src)
