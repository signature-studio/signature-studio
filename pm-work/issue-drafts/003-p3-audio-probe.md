---
draft: "003"
title: "P3 の音声の実測の道具を作る(AudioContext を開始し、underrun を数える)"
type: task
milestone: "M1 描画と音声の分離(P3)"
parent: "001"
blocked_by: []
labels: ["task", "audio", "verify", "priority-p3"]
size: 4
needs_owner: false
---

## 目的

台帳 L-033 の (2) と、P3(Song Mode で音が途切れない)の判定のための道具を作る。道具は、製品の既定の動作を変えずに、AudioContext を開始して、再生の quantum と underrun を数える。所定のプロジェクトを使う実測(子 004)の前に、道具が使えることを確かめる。

## 規模

規模: 大(想定 4 時間。ビルドを約 5 回回す規模)

## 背景

- 旧の測定(pm-work/findings/survey-c2-worker-split.md の手順 6)では、AudioContext が `suspended` のまま(status `needs-gesture`)で、AudioWorklet が再生の quantum を 1 つも数えていなかった(`quantaWhilePlaying` 0)。このため、旧の `underrunQuanta` 0 は測定ではなかった。
- 音声の状態は `window.__app.audio.detail()` で読める。項目は `app/src/audio/page-audio.ts` の 42〜43 行(`quantaWhilePlaying`・`underrunQuanta`・`underrunQuantaSinceFirstIdle`・`engineGapFrames`)と、249 行(取得)。
- AudioContext を、ユーザーの操作と同じ経路で開始させる方法は未確認。推定: CDP の `Input.dispatchMouseEvent` で、ページのボタン押下と同じ入力を送る(pm-work/findings/survey-c2-worker-split.md の「次の手順」2 の推定)。
- P3 の所定のプロジェクトは runner に無い(子 004 で扱う)。この Issue は、既定のプロジェクト(起動直後の状態)で、道具が音声の状態を読めることを確かめる。

## 読む物

- `app/src/audio/page-audio.ts`(42〜43 行: 音声の状態の型。249 行: 取得)、`app/src/audio/soft-sink.ts`(1〜6 行の注釈: AudioWorklet が動かない間の読み位置)
- `lift/runtime/fle_audio_transport.c`(54 行: 音声の書き込み `fle_js_publish`。108 行: 音声の待ち `fle_js_wait`)
- `tools/boot-check/boot-bundle.mjs`、`tools/boot-check/cdp.mjs`(起動と CDP の手順)
- `tools/verify-kit/tools/r635-run.sh`(既存の P3 の 1 回の列。出力の形の参考)
- `pm-work/findings/survey-c2-worker-split.md`(手順 6 の判定)

## やること

1. 道具 `tools/p3-audio-probe/measure.mjs` を作る。引数は bundle の絶対パス(必須)と、プロジェクトのファイル(任意。子 004 で使う)。stdout には JSON の 1 行だけを出す。ログは stderr に出す。
2. 道具は、engine-idle の後に、ページの操作と同じ入力(CDP の Input イベント)で AudioContext を開始させる。開始の方法と、開始できたかの確認を、報告に書く。
3. 開始の後、約 10 秒、既定のプロジェクトを再生して(再生の開始は道具が行う)、音声の状態を読む。読んだ項目を JSON に入れる: `ctxState`・`status`・`quantaWhilePlaying`・`underrunQuanta`・`engineGapFrames`・`playingSeconds`。
4. 道具の JSON には、`error`(null または文字列)と `standIn`(起動の値)を入れる。
5. 変更の前後の音声の状態を、報告に貼る。道具が AudioContext を開始できない場合は、「未確認」と書いて理由を書く。

## 受け入れ

```accept
npm run build --prefix app && cargo build --release --locked --manifest-path lift/Cargo.toml && rm -rf lift/out-app/b2d-src lift/out-app/gen && lift/target/release/fe-build engine && lift/target/release/fe-build bundle && node tools/boot-check/boot-bundle.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/boot.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.exit(r.error==null&&r.state==="engine-idle"&&r.standIn===false?0:1)' "$RUNNER_TEMP/boot.json" && node tools/p3-audio-probe/measure.mjs "$PWD/lift/out-app/bundle/raw/fl-Oz" > "$RUNNER_TEMP/p3-probe.json" && node -e 'const r=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8").trim());const ok=r.error===null&&r.ctxState==="running"&&r.quantaWhilePlaying>0&&typeof r.underrunQuanta==="number"&&typeof r.engineGapFrames==="number";process.exit(ok?0:1)' "$RUNNER_TEMP/p3-probe.json"
```

判定の基準: 道具の出力の生の値を、上の node -e が読む。AudioContext が `running` になり、再生の quantum が 1 以上数えられ、`underrunQuanta` と `engineGapFrames` が数値で出ている。

## 報告の規則

- 1 行目は「実エンジン: <何を実行し、どの出力を観測したか>」で始める。
- 測定の値には「runner の値。この機械とは別枠」と書き、nproc と loadavg を添える。
- 耳の確認は行わない。「音が途切れない」とは書かない。
- 実測と推定を分ける。「達成」と書かず、「受理」「差し戻し」「未確認」のいずれかにする。
- 出力が取れない物は「確認できない」と書く(台帳 L-036)。

## 制約

- 製品に新しい環境変数・#ifdef・URL パラメータ・ビルド変数を足さない(L-002)。道具の引数は製品の設定ではない。
- 製品の既定の動作を、測定のためだけに変えない。測定の道具の中だけで操作を行う。
- 依頼者の個人のプロジェクトを、リポジトリに入れない。
- Python を製品に使わない(L-008)。道具は JavaScript で書く。
- ベンダー名と Cloudflare の識別子を書かない(L-005・L-006)。
- sleep・timeout・nohup・末尾の `&` は使わない。待ちは JS のタイマーで行う。一時ファイルは `.tmp-*` に置く(L-022)。

## 他のタスクとの関係

- 004(所定のプロジェクトでの underrun の実測)は、この道具を使う。
- 031(P3 の再生中の命令数)は、この道具の開始の方法を使う。
- 002 の受理の判定とは独立。同じ道具を、描画の分離の前後で使える。
