# O19(依頼者の 12:4x の追加報告)の再現 scenario — Mixer 表示・マウス移動と再生(ci1、仮の番号 0334。**scenario と検査だけ。計時の run は 0**)

## 0. 実際に実行した物(先頭)
Chrome・Wine・clang・cargo は 0。python3 による scenario の生成(`scn-s3a-solo.json`/`scn-s3a-song.json` の頭を流用)、`scn-lint.mjs`(ドライバ `sdriveg2-lat.mjs` の op 表との照合。8 scenario・5,316 ops・0 problems)、`node --check`、`bash -n`、60 s の窓の待ち時間の合計の算術。**実エンジンへの実行 0 = どの条件で何が起きるかは未測定**。

## 1. 依頼者の観察と、ここで作ったもの
依頼者(12:4x): 『限られたトラックのみの Solo ならリアルタイム再生がギリギリ成立するが、Mixer ウインドウが表示されている、マウスを動かしている、などの条件で破綻する = オーディオ処理と GUI 処理が相互に影響している』。

## 2. scenario(`tools/scn-o19.json`、8 本 = {song, solo12} × 4 条件)
| 条件 | 名前 | 操作(project を開いた後。すべて同じ前処理) | 60 s の窓の中 |
|---|---|---|---|
| ① 基準(Mixer を隠す) | `o19-PROJ-hidden` | F9 を 1 回(Mixer を隠す。**参照 project は開いた後 Mixer が表示されている**ので、『表示なし』が対比になる)→ 2 s 待つ | 何もしない(2 s ごとに audioSnap) |
| ② Mixer 表示 | `o19-PROJ-shown` | 何もしない(Mixer は表示のまま) | 何もしない |
| ③ Mixer 上でマウス往復 | `o19-PROJ-mixmove` | 同上(表示のまま) | ポインタを Mixer の fader の高さ(guest 座標 y=800、x=420〜850)で往復。**100 ms 間隔で 20 回の `move`(= 2 s で 1 往路)× 30 ブロック = 600 move** |
| ④ Playlist 上でマウス往復 | `o19-PROJ-plmove` | 同上 | Playlist の grid(y=500、x=1150〜1880)で往復。同じ 600 move |
共通の流れ: `closeWizard` → `openZipProject`(solo12 は `dir: proj-solo12`)→ 10 s 待つ → (hidden のみ F9)→ `audioMark start` → **Space で再生開始 → 10 s 待って `audioSnap s10`(再生開始の落ち着き)→ `cpuStart` と `audioMark play`(60 s の窓の始め)→ 2 s ごとに `audioSnap w1..w30`(30 個 = 60 s)→ `cpuStop`・`audioDump`・Space で停止**。60 s の窓の中の待ち時間の合計は全 8 本とも 60,000 ms(算術)。`play` の印から 60 s で、underrun(後)・lag cuts・engine gap・present/s・Worker 別 CPU(`profs`)が `tools/rta-p3-table.py` に近い形で出せる(窓を 300 s でなく 60 s にするので Unlock の最後 2,400 件は全部入る)。
座標の根拠: Mixer の fader(window は guest x 390〜870、y 548〜1042)と Playlist の grid(x 1040〜1890)は、`s3a-r12-rta-song/scenario-s3a-song.png`(RT-A の run の絵、1920×1080 の guest)を目で読んだ値で、**実エンジンで当たるかは未確認**(dry は browser が要る)。

## 3. 実行(`tools/run-o19.sh PROJ COND VARIANT`、VARIANT = e13 | rta | rtb)
1 本ずつ(`--only o19-PROJ-COND`)、出力 `out/o19-PROJ-COND-ROUND-VARIANT/`。gate は他の Chrome 0・Wine 0・swap 入出力 0・MemAvailable ≥ 12 GB・開始 load ≤ 8・I/O 圧 0(`run-rta.sh` と同じ)。**計時なので PM が窓を知らせるまで始めない**(gpu1 の非計時の Chrome が動いている間は不可)。順: E13 と RT-A N=8 を交互に n = 2 から。1 本 約 2.5 分(project を開く 10〜30 s + 10 s + 60 s + 後処理)なので、8 条件 × 2 条件(E13/RT-A)× n=2 = 32 本で約 80〜90 分。まず solo12 の 4 条件を E13 と RT-A で 1 本ずつ(8 本、約 20 分)。
## 4. 限界
- Chrome の CDP の `Input.dispatchMouseEvent`(`mouseMoved`)は実マウスではなく、依頼者の『マウスを動かしている』と同じ負荷とは限らない(ブラウザ側の処理は同じだが、OS のポインタの移動・描画の経路は無い)。
- 60 s の窓のみ。依頼者の言う『ギリギリ』『破綻』の定義(音切れの回数)は数にしない。結果は underrun 量子数・lag cuts・engine gap で表す。
- Mixer 表示の既定の状態を F9 で切り替えるので、F9 が効くことが前提(scn-f9.json で 0247 の時に確認済み。この scenario では未確認)。
