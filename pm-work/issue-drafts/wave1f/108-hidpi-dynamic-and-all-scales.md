---
draft: "108"
title: "Fractional Scaling の動的な変化と、全ての拡大率(100%〜300%)・全ての窓の種類の検証"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: ["107"]
labels: ["hidpi", "verify"]
size: L
priority: High
project: signature-studio/3
---

## 目的

#{{107}} で、起動時の DPR に対して Windows と同じ Fractional Scaling を再現する。この Issue は、その結果を全ての拡大率と全ての窓の種類で確かめ、ブラウザのズームや窓の移動による DPR の変化(動的な変化)でも、同じ見え方を保つことを確かめる。

依頼者の要求: 全ての GUI が操作できる大きさで、ぼやけないこと。拡大されてぼやけた画面は棄却する(依頼者の指示)。

## 背景

- #{{107}} の道具 `tools/feat-c1-hidpi/measure-size.mjs` の JSON の形と、判定の基準(guest の大きさ・DPI・canvas と display の一致・縁の遷移の幅・部品の座標・blackShare)を、この Issue でも使う。
- Windows の動的な変化: 拡大率の変更や、別のモニターへの移動で DPI が変わると、DPI 対応のアプリは `WM_DPICHANGED`(提案の矩形つき)を受け取り、窓を提案の大きさに変える。画面の大きさが変わると `WM_DISPLAYCHANGE` が届く。Windows の公式の仕様(DPI の文書)で、この形を確かめる。
- Windows の FL の実際の挙動は、この機械では確かめられない。FL の実行ファイルと runtime の実測で分かる範囲を報告し、確認できない部分は「確認できない」と書く。
- 既存の経路: 画面の大きさの更新は `app/src/main.ts`(`applyScreenParam`・`applyDisplay`・DPR の監視)と `lift/runtime` の `WM_DISPLAYCHANGE` の送り方(`grep -rn WM_DISPLAYCHANGE lift/runtime` で探す)にある。

## 読む物

- #{{107}} の成果物(`tools/feat-c1-hidpi/measure-size.mjs`、`lift/runtime` の DPI の経路、`app/src/ui/screen-fit.ts`・`desktop.ts`)
- `lift/runtime/fle_win.c`(窓の大きさの変更と、`WM_DISPLAYCHANGE`・`WM_DPICHANGED` の送り方)
- `app/src/main.ts`(`applyScreenParam`・`applyDisplay`・DPR の監視)
- `app/src/engine/render-worker.ts`(窓ごとのテクスチャ)、`app/src/input/pointer.ts`(ポインタの変換)

## やること

1. #{{107}} の道具を拡張する。引数は次のとおり。
   - `--dprs 1,1.25,1.5,1.75,2,2.5,3`(静的な行。拡大率ごと)
   - `--windows main,welcome,mixer,playlist,piano-roll,plugin,tooltip,menu`(窓の種類ごと。開けない窓は、その行の `error` に理由を書く)
   - `--dynamic 1,1.7647,2,1`(一回の実行の中で DPR をこの順に変える。`Emulation.setDeviceMetricsOverride` の deviceScaleFactor を使う)
   各行に `"window"`(窓の名前)と `"mode"`("static" か "dynamic")を足す。動的な行の `dpr` は、最後の DPR にする。
2. 全ての静的な行で、#{{107}} の判定の基準を満たすことを確かめる。部品の座標と blackShare は main の窓で測る。縁の遷移の幅(edgeWidth)は、窓の種類ごとに測る。
3. 動的な変化: DPR を 1 → 1.7647 → 2 → 1 と変える。各変化で、guest の画面と DPI が更新され、engine が `WM_DISPLAYCHANGE`・`WM_DPICHANGED` を受け取ったことを engine のログで確かめる。最後の行(`mode` が dynamic で、`dpr` が 1)の main の部品の座標と edgeWidth が、最初から DPR 1 で起動した静的な行と一致すること。
4. 直す: 動的な変化で崩れる所(窓の大きさ、描画の矩形、ポインタの変換、テクスチャの大きさの更新)。FL が `WM_DPICHANGED` を受けた後にどう窓を配置するかを、実測で確かめる。Windows の仕様と違う所は直す。FL の実際の挙動を、Windows の仕様より優先して真似ることはしない。
5. ポインタ: 各 DPR で、部品の中心を押して、押した部品が正しいことを engine の記録(`WM_LBUTTONDOWN` の宛先)で確かめる。ずれが ±1 画素を超える DPR は直す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1,1.25,1.5,1.75,2,2.5,3 --windows main,welcome,mixer,playlist,piano-roll,plugin,tooltip,menu --dynamic 1,1.7647,2,1 > "$RUNNER_TEMP/fs.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const D=[1,1.25,1.5,1.75,2,2.5,3];const W=["main","welcome","mixer","playlist","piano-roll","plugin","tooltip","menu"];const P=["tempo","song-position","pattern-selector"];const near=(x,y,t)=>Math.abs(x-y)<=Math.max(2,t*Math.abs(y));const S=L.filter((x)=>x.mode==="static");const g=(d,w)=>S.find((x)=>Math.abs(x.dpr-d)<0.0001&&x.window===w);const row=(x)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Math.abs(x.guest.w-Math.round(x.room.w*x.dpr))<=1&&Math.abs(x.guest.h-Math.round(x.room.h*x.dpr))<=1&&Math.abs(x.dpi-Math.round(96*x.dpr))<=1&&Math.abs(x.canvas.w-x.display.w)<=1&&Math.abs(x.canvas.h-x.display.h)<=1&&x.edgeWidth<=2;const a=g(1,"main");const cover=D.every((d)=>W.every((w)=>g(d,w)!==undefined));const main=(x)=>P.every((p)=>near(x.parts[p].x,a.parts[p].x,0.05)&&near(x.parts[p].y,a.parts[p].y,0.05))&&Math.abs(x.blackShare-a.blackShare)<=0.02;const dyn=L.filter((x)=>x.mode==="dynamic");const last=dyn[dyn.length-1];const f=g(1,"main");const dynOk=dyn.length>=1&&dyn.every(row)&&last.dpr===1&&P.every((p)=>near(last.parts[p].x,f.parts[p].x,0.02)&&near(last.parts[p].y,f.parts[p].y,0.02))&&Math.abs(last.edgeWidth-f.edgeWidth)<=0.5;const ok=cover&&S.every(row)&&S.every((x)=>x.window!=="main"||x.dpr===1||main(x))&&a.blackShare<0.5&&dynOk;process.exit(ok?0:1)' "$RUNNER_TEMP/fs.jsonl"
```

判定の基準: 道具の出力の生の値を、上の node -e が読む。(a) 静的な行は、拡大率 7 段 × 窓の種類 8 種の全ての組が在り、全て #{{107}} の行の判定(guest・dpi・canvas と display・edgeWidth 2 以下)を満たす。(b) main の窓の静的な行は、部品の座標が DPR 1 の値と ±5% 以内、blackShare が DPR 1 の値と 2 ポイント以内。DPR 1 の blackShare は 0.5 未満。(c) 動的な行は、全て同じ判定を満たし、最後の行(DPR 1)の部品の座標が ±2%、edgeWidth が DPR 1 の静的な行と ±0.5 画素以内で一致する。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 拡大率と窓の種類ごとの表(edgeWidth と blackShare)、動的な変化の結果(engine のログの `WM_DISPLAYCHANGE`・`WM_DPICHANGED` の行)、ポインタの結果、開けなかった窓と理由、Windows の実際の挙動のうち確認できなかった部分。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 論理の小さい画面を DPR 倍に拡大して表示する実装を、作らない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: #{{107}}(起動時の DPR に対する Windows と同じ Fractional Scaling)。
- #{{109}}(音が途切れる現象)は、この Issue と並行して進めてよい。ただし、物理画素の画面で測る。
- 同じ描画の経路を変える #{{040}} と #{{106}} と衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
