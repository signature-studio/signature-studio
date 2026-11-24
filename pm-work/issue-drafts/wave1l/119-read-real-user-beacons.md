---
draft: "119"
title: "本番の実機のビーコンを読む: 版・DPR・DPI(DPR から求める)・途切れ・起動時間を、直近の記録から表にする"
type: Task
milestone: "M2 HiDPI と P2 の測定(P2)"
parent: "005"
blocked_by: []
labels: ["perf-beacons", "verify", "hidpi"]
size: M
priority: Urgent
project: signature-studio/3
---

## 目的

依頼者の観察: 本番(依頼者の実機)で、FL の UI がまだ小さい(DPR 1.7647)。起動して 10 秒後の途切れは 343(依頼者の画面の値)。

runner の測定(#{{113}})では、headless Chrome で DPR 1.7647 のとき、FL は DPI 169 を受けて 175% の UI を出した。本番の実機の見え方は、これと食い違っている。この Issue は、依頼者の実機の記録(ビーコン)から、食い違いの原因を絞り込む材料を作る。

目標は、直近の実機の記録を、版・DPR・DPI・画面の大きさ・途切れ・起動時間の表にして、「古い版を実行していた」か「新しい版で DPI 169 を受けているのに FL が拡大しない」かを判定することである。

## 背景

- 本番(`app/wrangler.jsonc`)は、前段のアクセス制御の後ろにあり、公開のホスト名が無い。runner からは開けない。そのため、実機の測定は、ページが送るビーコン(`app/src/perf/beacon.ts`)で届く。ページは `POST /__perf` に送り、Worker が R2 のバケット `fl-web-cache` の `perf/<UTC の日付>/<sid>-<seq>.json` に保存する。
- 保存される 1 件は、1 行の JSON `{"t": 受け取った時刻(ISO), "remote": 経路, "perf": ビーコン本体}`。`perf` が版 3 の `BeaconPayload`(`app/src/perf/beacon.ts`)。`t` は保存の時刻で、ビーコン本体には時刻が無い。
- `perf.bundle.version` は、engine の `web/engine.wasm` の sha256 の先頭 12 桁(`lift/crates/fe-build/src/main.rs` の `pack_step_for`)。engine が変わらなければ変わらず、app だけを変えても変わらない。
- `perf.guestScreen` には、guest の画面(`width` × `height`。デバイス画素)・部屋(`roomWidth` × `roomHeight`)・`dpr` がある。DPI の項目 `dpi` は、今の版では常に `null`(`app/src/ui/screen-fit.ts`)。ページは DPI を告げず、engine が DPR から `MulDiv(96, round(dpr × 100), 100)` で DPI を求める。そのため、表の DPI は DPR から求めた値を書く(古い記録に `dpi` の数が在れば、それも並べる)。
- 窓の大きさは、今のビーコンには無い。#{{120}} が足す。この Issue は、窓の大きさが記録に無いことを報告に書く。
- ラベル `perf-beacons` の作業では、extract が直近 3 日(UTC の今日と前の 2 日)の `perf/` の `*.json` を、最大 1000 件・合計 100 MB で取り、run ごとの鍵のキャッシュで作業の段に渡す。作業の段は `lift/out-app/host-extra/perf/` に置く(commit はされず、公開の artifact にも置かれない)。`.github/workflows/agent.yml` の 4 段。
- 既存の道具: `app/scripts/perf-log-summary.mjs --dir <ディレクトリ>` は、保存の形のディレクトリを読み、P1・P2 の数と、機械の種類ごとの表を出す。版ごとの表と、DPR・DPI の表は無いので、この Issue が足す。
- 依頼者の実機の記録は、個人を特定しうる値(`sid`・IP・パス・名前)を含みうる。報告・findings・Issue のコメントには、集計と結論だけを書く。
- 食い違いの候補(推定。未確認): (a) 古い版を実行している(ブラウザや OPFS に保存された複製)。(b) 新しい版で、DPR から求めた DPI が 169 相当なのに、実機の FL が 175% に拡げない(実機でだけ起きる)。`pm-work/findings/hidpi-fl-scaling.md` の表と突き合わせる。

## 読む物

- `app/src/perf/beacon.ts`(`BeaconPayload`。`sid`・`bundle`・`engine`・`startup`・`input.eventToRaf`・`audio.underruns`・`guestScreen`・`browser`)
- `app/src/ui/screen-fit.ts`(`GuestScreenInfo`。DPI は DPR から求める注釈)
- `app/src/perf/browser-info.ts`(`browser.name`・`version`)
- `app/src/perf/display.ts`(画面の計測。道具の中では読むだけ)
- `app/shared/perf-summary.mjs`・`app/scripts/perf-log-summary.mjs`(既存の集計と、`--dir` の読み方)
- `app/worker/perf-summary.ts`・`app/shared/perf.ts`(保存の形。`beaconProblems` の検査)
- `lift/crates/fe-build/src/main.rs`(`pack_step_for`。版の値の出どころ)
- `pm-work/findings/hidpi-fl-scaling.md`(FL の拡大率の実測の表)
- `.github/workflows/agent.yml`(`perf-beacons` の 4 段)

## やること

0. 作業ツリーの確認。`.prior-work.md` が在れば、最初に読む。`lift/out-app/host-extra/perf/` が在ることを確かめ、件数と期間(日付の範囲)を報告に書く。0 件なら、そのことを書き、表は 0 行で終えてよい。
1. 道具 `tools/perf-beacons/summary.mjs` を書く(新規。node。外部のライブラリを使わない)。`lift/out-app/host-extra/perf/` の下の `*.json` を全て読む(壊れた 1 件は、件数だけを数えて飛ばす)。版(`perf.bundle.version`。無ければ「(無し)」)ごとに、次の 1 行の表を出す。
   - 記録の数、セッションの数(`perf.sid` の種類の数。値は出さない)、最初と最後の時刻(`t`。UTC)
   - ブラウザ(`perf.browser.name`・`version` の組。件数の多い順)
   - DPR(`perf.guestScreen.dpr` の種類と件数)と、DPI(DPR から `MulDiv(96, round(dpr × 100), 100)` で求めた値。`perf.guestScreen.dpi` が数なら、その値も並べる)
   - guest の画面(`width` × `height`)と部屋(`roomWidth` × `roomHeight`)
   - 起動時間(`perf.startup.firstIdleSeconds` の中央値と最大)
   - 途切れ(`perf.audio.underruns` の最後の値の最大)
   - 入力(`perf.input.eventToRaf.p95Ms` の中央値)
   - `perf.engine.state` と `perf.engine.exit.reason`(件数)
   表は標準出力に Markdown で出す。`sid`・`remote`・ユーザーエージェントの全文・IP は出さない。
2. 直近の実機の記録(今日の日付。DPR が 1.7 台のセッション)について、次を表にする: (a) 版(`bundle.version`)と、その記録の時刻、(b) DPI(DPR から求めた値)が 169 相当か 96 相当か、(c) guest の画面が物理画素か(`width` が、部屋の幅 × DPR に近いか)。
   依頼者の「UI が小さい」が、(a) 古い版を実行していた、(b) 新しい版で、DPI 169 相当を engine が受けているのに FL が拡大しない(実機でだけ)、のどちらかを判定する。判定できなければ「未確認」と書き、判定に要る記録(欠けている項目)を書く。版と deploy の対応は、版の時刻と、`deploy.yml` の run の時刻(完了の時刻)で突き合わせる。
3. 結果を `pm-work/findings/real-user-beacons-<UTC の日付>.md` に書く(集計と結論だけ。個人を特定できる値は書かない)。runner の測定(#{{113}})と実機の記録の違い(版・DPR・DPI・画面の大きさ)を 1 表にする。
4. 作業の最後に、受け入れが通らなくても、作業ツリーを commit する。

## 受け入れ

```accept
node tools/perf-beacons/summary.mjs lift/out-app/host-extra/perf > /dev/null && node -e 'const fs=require("fs"),p=require("path");const ids=[];const walk=(d)=>{for(const e of fs.readdirSync(d,{withFileTypes:true})){const q=p.join(d,e.name);if(e.isDirectory())walk(q);else if(e.name.endsWith(".json")){try{const s=JSON.parse(fs.readFileSync(q,"utf8")).perf?.sid;if(typeof s==="string"&&s.length>=8)ids.push(s);}catch{}}}};walk("lift/out-app/host-extra/perf");const f=fs.readdirSync("pm-work/findings").filter((x)=>/^real-user-beacons-.*[.]md$/.test(x));if(f.length===0)process.exit(1);for(const x of f){const t=fs.readFileSync(p.join("pm-work/findings",x),"utf8");if(/@|C:\\/.test(t)||ids.some((s)=>t.includes(s)))process.exit(1);}' && bash guard/scan.sh pm-work/findings/real-user-beacons-*.md
```

判定の基準: (1) `tools/perf-beacons/summary.mjs` が 0 で終わり、版ごとの表を出す(0 件でも、0 行の表で通る)。(2) `pm-work/findings/real-user-beacons-*.md` が在り、`guard/scan.sh` が 0 件。(3) その findings に、ビーコンの `sid`(8 文字以上)の値・`@`・`C:\` が含まれない(受け入れの後半の確かめ)。

## 報告の規則

- 1 行目は「実機の記録: <期間・件数・版の数>」で始める。
- 実測と推定を分ける。「原因」と書かず、「特定」「推定」「未確認」のいずれかを使う。出力の行を付ける。
- 報告に書く: (1) 件数と期間、(2) 版ごとの表(集計の値)、(3) 直近の実機の記録の (a)(b)(c)、(4) 判定(どちらか。判定できなければ「未確認」と、足りない項目)、(5) 窓の大きさが記録に無いことの確認。
- ビーコンの中身(`sid`・IP・パス・名前)を、報告・コメント・findings に書かない。

## 制約

- 製品(`app/`・`lift/`・`ci/`)は変えない。この Issue は道具と findings を足すだけ。
- 製品に、利用者向けの新しい環境変数・`#ifdef`・URL パラメータ・ビルド変数を足さない。
- ベンダー名と Cloudflare の識別子を書かない。
- `.github/`・`inputs/`・`guard/`・`spec/` は変えない(読むだけ)。
- Python は使わない(道具は node で書く)。
- 一時ファイルは `.tmp-*` に置き、終わったら消す。sleep・timeout・nohup・末尾の `&` は使わない。

## 作業の進め方

- 最初に `date -u` で開始の時刻を記録する。4 時間 30 分を過ぎたら、受け入れが通っていなくても、作業ツリーを commit し、報告を書いて終える。
- ビルドは要らない(読みと集計だけ)。道具の出力が大きいときは `.tmp-*` に書いて読み、終わったら消す。

## 他のタスクとの関係

- blocked_by: なし。
- #{{113}}(#1353。runner の測定。この Issue の比べる基準)。
- #{{120}}(同じ Epic。ビーコンに FL の窓の大きさを足す)。120 が入る前の記録には、窓の大きさは無い。120 を待たずに、この Issue は動く。
