# 0007: extra seed の API と、seed 0x41d050 の追加

状態: 決定（2026-10-20）。実装と実行の結果は `docs/investigations/extra-seed-run.md` を参照。

## 文脈

0006 の決定 1 のうち seed の追加は、fe-decode に extra seed を渡す API が無かったため、本番の生成器には入れていなかった。その後、表 0x13a9410 の要素 [1] として 0x41d050 を呼ぶ読み手 0x41223e が、実行で確かめられた（`docs/investigations/mode-0x41223e-runtime.md`）。

## 決定

1. fe-decode に `analyze_with_seeds(pe, extra: &[u64])` を追加する。`extra` は既定の seed（entry と export）の後ろに積む。名前は `<seed:VA>`。
2. `analyze(pe)` は `analyze_with_seeds(pe, &[])` を呼ぶ。既定の seed の挙動は変えない。
3. fe-lift に `generate_with_seeds(data, extra, w)` を追加する。`generate` は空の `extra` で呼ぶ。
4. 本番の既定の閉包に 0x41d050 を extra seed として加える（呼び出し側が指定する。既定の seed には入れない）。
5. 0x41e7e0 と 0x41ce10 は加えない。表経由の静的な読み手が見つかっていないため（0006 の決定 2 のまま）。

## 閉包の数字が変わることの受け入れ

extra seed 0x41d050 を加えると、閉包は次のとおり変わる（実測、`docs/investigations/closure-count.md` の追記を参照）。

- 関数: 6,969 → 6,971（+2）
- 命令: 460,479 → 460,492（+13）

これは PM が受け入れた。0006 の「閉包の関数数と命令数は変わらないこと」という前提は、この決定で置き換える。既存の文書の閉包の数字は、新しい数字に更新した（既定の seed だけの場合の数字は、従来どおり併記している）。

## 理由

- 0x41d050 は、閉包の外にある関数のうち、表経由で閉包内の読み手から実際に呼ばれることが、静的にも実行でも確かめられた唯一のものである。
- 既定の seed を変えず、extra として明示的に渡すので、他の呼び出し（既定の閉包を使う箇所）の挙動は変わらない。

## 残る課題

- 本番の実行（0x41d050 を seed に加えた生成 C）は、次の停止点 0x41e7e0 で止まる（dispatch miss）。0x41e7e0 は本決定で seed に加えない（決定 5）。したがって、この決定だけでは実行の完了（フル lift）には届かない。
- 0x41e7e0 を越えるかどうかは、別の決定が要る。
