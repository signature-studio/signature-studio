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

#{{107}} で、起動時の DPR に対して Windows と同じ Fractional Scaling を再現する。この Issue は、その結果を全ての拡大率と全ての窓の種類で確かめ、ブラウザのズームや窓の移動による DPR の変化(動的な変化)で、FL の仕様(拡大率は再起動で適用される)に合った振る舞いを確かめる。

依頼者の要求: 全ての GUI が操作できる大きさで、ぼやけないこと。拡大されてぼやけた画面は棄却する(依頼者の指示)。特定の拡大率の数字を予め用意しない(#{{107}} と同じ。拡大率は、環境の DPR から Windows と同じ計算で導く)。

## 背景

- #{{107}} の道具 `tools/feat-c1-hidpi/measure-size.mjs` の JSON の形と、判定の基準(guest の大きさ・DPI・FL の拡大率・canvas と display の一致・縁の遷移の幅・部品の座標・blackShare・Main GUI Scaling・Debug ログ)を、この Issue でも使う。
- FL の仕様(FL 公式マニュアルの General settings の Display の節の記述。実機では未確認):
  - Main GUI Scaling・Pop-ups・Toolbars・Legacy scaling は、FL の再起動で効く。実行中に変えても適用されない。
  - ネイティブプラグインの GUI は、100%・200%・300%・400% の整数だけ(Legacy scaling)。分数の拡大はできない。
  - Cursors scaling と Thick lines(200% 以上の線の太さ)がある。
- Windows の動的な変化: 拡大率の変更や、別のモニターへの移動で DPI が変わると、DPI 対応のアプリは `WM_DPICHANGED`(提案の矩形つき)を受け取り、窓を提案の大きさに変える。画面の大きさが変わると `WM_DISPLAYCHANGE` が届く。Windows の公式の仕様(DPI の文書)で、この形を確かめる。ただし FL は上の仕様のとおり、拡大率を再起動で適用する。Windows の振る舞いは、FL の仕様より優先しない。
- Windows の FL の実際の挙動は、この機械では確かめられない。FL の実行ファイルと runtime の実測で分かる範囲を報告し、確認できない部分は「確認できない」と書く。
- 既存の経路: 画面の大きさの更新は `app/src/main.ts`(`applyScreenParam`・`applyDisplay`・DPR の監視)と `lift/runtime` の `WM_DISPLAYCHANGE` の送り方(`grep -rn WM_DISPLAYCHANGE lift/runtime` で探す)にある。

## 読む物

- #{{107}} の成果物(`tools/feat-c1-hidpi/measure-size.mjs`、`lift/runtime` の DPI の経路、`app/src/ui/screen-fit.ts`・`desktop.ts`)
- `lift/runtime/fle_win.c`(窓の大きさの変更と、`WM_DISPLAYCHANGE`・`WM_DPICHANGED` の送り方)
- `app/src/main.ts`(`applyScreenParam`・`applyDisplay`・DPR の監視)
- `app/src/engine/render-worker.ts`(窓ごとのテクスチャ)、`app/src/input/pointer.ts`(ポインタの変換)
- `pm-work/findings/hidpi-fl-scaling.md`(#{{107}} の測定の結果。FL の拡大の決め方と、Debug ログの取り方)

## やること

1. #{{107}} の道具を拡張する。引数は次のとおり。
   - `--dprs 1,1.25,1.5,1.75,2,2.5,3`(固定の一覧。DPR の入力としてだけ使い、期待値には使わない)に加えて、`--random 3` で乱数の DPR を 3 つ選ぶ(範囲 1.00〜3.00・0.01 刻み。起動のたびに crypto の乱数で種を選び、その種から 3 つを決める。種と値は `randomDprs` に出す)。静的な行は、固定と乱数を合わせて 10 段 × 窓の種類 8 種 = 80 行。
   - `--windows main,welcome,mixer,playlist,piano-roll,plugin,tooltip,menu`(窓の種類ごと。開けない窓は、その行の `error` に理由を書く)
   - `--dynamic 1,1.7647,2,1`(1 回の実行の中で DPR をこの順に変える。`Emulation.setDeviceMetricsOverride` の deviceScaleFactor を使う)。動的の行は段ごとに 1 行を出し、`dpr` はその段の DPR、`step` は段の番号(1〜4)にする。
   - 各行に `"window"`(窓の名前)と `"mode"`(`"static"` か `"dynamic"`)と `"randomDprs"` を足す。
   - 窓 `plugin` の行には、その窓が出した拡大率(整数の %)を `pluginScalePercent` として出す。
   - 動的の行には、利用者への再起動の案内が出ているかを `restartNotice`(真偽)で出す。案内の要素には `data-restart-notice` の属性を付け、道具がその属性で読む。
   - 全ての行に `debugLog`(#{{107}} の 1 の (g)・(j) と同じ方法で取る FL の Debug ログ)を出す。
2. 静的な行: #{{107}} の判定の基準(guest・dpi・canvas と display・edgeWidth・Main GUI Scaling・debugLog)を全ての行で満たす。拡大率の判定(`flScalingPercent` が round(100 × DPR) の ±3)は main の窓の行で確かめる。部品の座標と blackShare は main の窓で測る。縁の遷移の幅(edgeWidth)は、窓の種類ごとに測る。
3. 動的な変化: DPR を 1 → 1.7647 → 2 → 1 と変える。段ごとに、次を確かめる。
   - guest の画面は round(部屋の CSS × 段の DPR) の物理画素に合わせ直される。canvas と display が ±1 画素で一致し、edgeWidth が 2 以下(ぼやけない)。
   - FL の DPI と拡大率(`dpi`・`flScalingPercent`)は、起動時の値のまま変わらない(FL の仕様どおり、再起動で適用されるため)。実測で確かめ、変わったら、その事実を報告に書く。
   - 段の DPR が起動時(DPR 1)と違う段では、再起動の案内が出ている(`restartNotice` が true)。
   - 最後の段(DPR 1 に戻る)は、主窓の部品の座標が DPR 1 の静的な行と ±2%、edgeWidth が ±0.5 画素以内で一致する。
   - engine のログに、各段の `WM_DISPLAYCHANGE`(と、送ったなら `WM_DPICHANGED`)の行が在る。`WM_DPICHANGED` を送るかどうかは、FL の実測で決め、理由を報告に書く。
4. 直す(動的な変化で崩れる所だけ。`lift/runtime` と `app/src`)。
   - 画面の大きさ・canvas・ポインタの変換・テクスチャの大きさを、段の DPR に合わせて更新する。
   - 拡大率(FL が見る DPI と、FL の UI の拡大率)は、再起動まで変えない。
   - 利用者への案内: ページの UI(既存の表示の仕組み)で、拡大率の変更は FL を再起動すると適用される旨を出す。案内を出すか、再起動を自動で提案するかは作業者が選び、理由を報告に書く。
   - 新しい環境変数・URL パラメータ・ビルド変数を作らない。拡大率の数値を runtime・app・seed に書かない(#{{107}} の制約と同じ)。
5. ポインタ: 各行で、主窓の部品の中心を押して、押した部品が正しいことを engine の記録(`WM_LBUTTONDOWN` の宛先)で確かめる。ずれが ±1 画素を超える DPR は直す。
6. ネイティブプラグインの窓(Fruity Balance など): 窓の拡大が整数の %(100・200・300・400)だけで出ることを、`pluginScalePercent` で確かめる。分数の DPR での出し方(どの整数を選んだか)を、FL の実測で報告に書く。
7. Debug ログ: 全ての行で、FL の Debug ログに、拡大率に関係するエラー・警告が出ていないこと(`debugLog.scalingLines` に error・warn の行が無いこと)を確かめる。出ていれば、その行を報告に書き、原因を直す。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/feat-c1-hidpi/measure-size.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" --dprs 1,1.25,1.5,1.75,2,2.5,3 --windows main,welcome,mixer,playlist,piano-roll,plugin,tooltip,menu --random 3 --dynamic 1,1.7647,2,1 > "$RUNNER_TEMP/fs.jsonl" && node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>JSON.parse(l));const P=["tempo","song-position","pattern-selector"];const W=["main","welcome","mixer","playlist","piano-roll","plugin","tooltip","menu"];const eq=(x,y)=>Math.abs(x-y)<0.0001;const near=(x,y,t)=>Math.abs(x-y)<=Math.max(2,t*Math.abs(y));const S=L.filter((x)=>x.mode==="static");const Dy=L.filter((x)=>x.mode==="dynamic");const R=S[0].randomDprs.values;const F=[1,1.25,1.5,1.75,2,2.5,3].concat(R);const a=S.find((x)=>eq(x.dpr,1)&&x.window==="main");const dbg=(x)=>x.debugLog.captured===true&&x.debugLog.lines>=1&&!x.debugLog.scalingLines.some((l)=>/error|warn/i.test(l));const base=(x)=>x.error===null&&x.state==="engine-idle"&&x.standIn===false&&Math.abs(x.guest.w-Math.round(x.room.w*x.dpr))<=1&&Math.abs(x.guest.h-Math.round(x.room.h*x.dpr))<=1&&Math.abs(x.canvas.w-x.display.w)<=1&&Math.abs(x.canvas.h-x.display.h)<=1&&x.edgeWidth<=2&&x.mainGuiScaling==="System"&&dbg(x);const sRow=(x)=>base(x)&&Math.abs(x.dpi-Math.round(96*x.dpr))<=1&&(x.window!=="plugin"||[100,200,300,400].includes(x.pluginScalePercent))&&(x.window!=="main"||Math.abs(x.flScalingPercent-Math.round(100*x.dpr))<=3);const dRow=(x)=>base(x)&&x.dpi===a.dpi&&Math.abs(x.flScalingPercent-a.flScalingPercent)<=1&&(eq(x.dpr,1)||x.restartNotice===true);const mainOk=(x)=>P.every((p)=>near(x.parts[p].x,a.parts[p].x,0.05)&&near(x.parts[p].y,a.parts[p].y,0.05))&&Math.abs(x.blackShare-a.blackShare)<=0.02;const last=Dy[Dy.length-1];const ok=a!==undefined&&R.length===3&&R.every((d)=>d>=1&&d<=3)&&S.length===80&&F.every((d)=>W.every((w)=>S.some((x)=>eq(x.dpr,d)&&x.window===w)))&&S.every(sRow)&&S.filter((x)=>x.window==="main").every(mainOk)&&a.blackShare<0.5&&Math.abs(a.room.w-1836)<=2&&Math.abs(a.room.h-1056)<=2&&Dy.length===4&&Dy.every(dRow)&&eq(last.dpr,1)&&P.every((p)=>near(last.parts[p].x,a.parts[p].x,0.02)&&near(last.parts[p].y,a.parts[p].y,0.02))&&Math.abs(last.edgeWidth-a.edgeWidth)<=0.5;process.exit(ok?0:1)' "$RUNNER_TEMP/fs.jsonl" && ! git diff -U0 refs/judge/main -- lift/runtime lift/harness app/src | grep -E '^\+[^+]' | grep -qE '\b(125|150|168|169|175|176)\b|1\.75\b|1\.7647|0\.5667' && npm test --prefix app
```

判定の基準: 道具の出力の生の値を、上の node -e が読む。
- (a) 静的な行は 80 行。固定の一覧(1・1.25・1.5・1.75・2・2.5・3)と `randomDprs.values` の 3 つ、計 10 段の全てに、窓の種類 8 種の行が在る。全ての行が、#{{107}} の基準(guest・dpi・canvas と display・edgeWidth・Main GUI Scaling・debugLog。error・warn が無い)を満たす。
- (b) main の窓の静的な行は、`flScalingPercent` が round(100 × DPR) の ±3。部品の座標が DPR 1 の値と ±5% 以内、blackShare が DPR 1 の値と 2 ポイント以内。DPR 1 の blackShare は 0.5 未満。
- (c) plugin の窓の行は、`pluginScalePercent` が 100・200・300・400 のいずれか。
- (d) 動的な行は 4 行(段 1〜4)。全ての段が、guest(段の DPR で計算)・canvas と display・edgeWidth・debugLog の基準を満たす。DPI と拡大率は、起動時(DPR 1)の値のまま変わらない。段の DPR が 1 でない段は、再起動の案内(`restartNotice`)が出ている。
- (e) 最後の段(DPR 1)の部品の座標が、DPR 1 の静的な行と ±2%、edgeWidth が ±0.5 画素以内で一致する。
- 乱数の DPR は、作業者の実行と判定の実行で別になる。固定の値に合わせた実装は通らない。
- ソースの検査(受け入れの最後の段): `lift/runtime`・`lift/harness`・`app/src` の、この変更で足した行に、特定の拡大率・DPI・DPR の値が、拡大率・DPI・DPR の意味で現れないこと。一致が 0 件であること。
- 単体テスト(`npm test --prefix app`)が通ること。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。出力の行を付ける。
- 報告に書く: 拡大率と窓の種類ごとの表(edgeWidth と blackShare)、`randomDprs` の種と値、動的な変化の結果(段ごとの guest・DPI・拡大率と、engine のログの `WM_DISPLAYCHANGE`・`WM_DPICHANGED` の行)、`WM_DPICHANGED` を送ったかどうかと理由、再起動の案内の方式と理由、ポインタの結果、plugin の窓の出した拡大率(整数の丸め方)、Debug ログの error・warn の有無(`scalingLines` の該当行)、開けなかった窓と理由、Windows の実際の挙動のうち確認できなかった部分、ソースの検査の grep の出力。

## 制約

- 製品に新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- 特定の拡大率・DPI・DPR の値を、runtime・app・seed・設定・テストに、定数・表・分岐・seed の値として入れない。拡大率は、実行時の DPR から式で導く。
- 論理の小さい画面を DPR 倍に拡大して表示する実装を、作らない。
- 製品の既定の動作を、測定のためだけに変えない。
- ベンダー名と Cloudflare の識別子を書かない。
- sleep・timeout・nohup・末尾の `&` は使わない。一時ファイルは `.tmp-*` に置き、終わったら消す。

## 他のタスクとの関係

- blocked_by: #{{107}}(起動時の DPR に対する Windows と同じ Fractional Scaling)。
- #{{109}}(音が途切れる現象)は、この Issue と並行して進めてよい。ただし、物理画素の画面で測る。
- 同じ描画の経路を変える #{{040}} と #{{106}} と衝突しうる。衝突したら、作業の最後に `git pull --no-rebase --no-edit` で main を取り込み、衝突を直し、受け入れをもう一度通す。
