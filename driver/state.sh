#!/usr/bin/env bash
# driver/state.sh(source して使う): state は、作業が読む木(製品・判定・駆動・型・作業者の決まり)の hash。
# 落ちた時の state を Issue に残し、fill が今の state と比べる。main の commit が変わっても、この木が同じなら同じ結果になるので起動しない。
# 台帳・待ち行列の状態(queue/threshold)の変更では変わらない。
state() {
  for p in app lift tools patches ci driver guard .github queue/kinds queue/inputs AGENTS.md; do
    printf '%s %s\n' "$p" "$(git rev-parse -q --verify "HEAD:$p" || echo -)"
  done | git hash-object --stdin
}
