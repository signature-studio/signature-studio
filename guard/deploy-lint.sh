#!/usr/bin/env bash
# guard/deploy-lint.sh [FILE] : 依頼者の制約(台帳 L-007)。FILE の既定は .github/workflows/deploy.yml。上限は FILE の名前で決める: deploy.yml は 100 行以内、e2e.yml は 200 行以内(E2E を分けたため。pm-work/workflow-design.md の「19」)。どちらも 1 行 80 字以内、シェルスクリプトを実行しない、ローカル action・再利用ワークフロー・composite action を使わない、Makefile を作らない、workflow_dispatch・schedule・[skip ci]・マスク(add-mask)を置かない。if: は、deploy.yml では e2e の成功の 1 つだけ、e2e.yml ではブランチ・イベントで段を飛ばさない。終了コード 0 = 守っている。
f=${1:-.github/workflows/deploy.yml}; bad=0
chk() { echo "deploy-lint: $1"; bad=1; }
case "$(basename "$f")" in deploy.yml) max=100 ;; e2e.yml) max=200 ;; *) max=0; chk "deploy.yml・e2e.yml 以外の名前で、上限が決まらない($(basename "$f"))" ;; esac
[ "$(wc -l < "$f")" -le "$max" ] || chk "$max 行を超えている"
w=$(awk 'length($0)>80{print NR; exit}' "$f"); [ -z "$w" ] || chk "80 字を超える行がある(行 $w)"
grep -nE '^[[:space:]]*-?[[:space:]]*run:.*(\.sh\b|\bbash\b|\bsh\b[[:space:]]+-c|\bmake\b|\./[A-Za-z])' "$f" >/dev/null && chk "シェルスクリプトか make を実行している"
grep -nE 'uses:[[:space:]]*\./|workflow_call' "$f" >/dev/null && chk "ローカル action か再利用ワークフローを使っている"
[ -z "$(git ls-files -- Makefile makefile GNUmakefile '*/Makefile' 2>/dev/null)" ] || chk "Makefile がある"
grep -nE 'workflow_dispatch|schedule|skip ci|add-mask' "$f" >/dev/null && chk "workflow_dispatch・schedule・[skip ci]・マスク(add-mask)を置いている"
ifs=$(grep -oE '(^|[{,[:space:]-])if:[^,}]*' "$f" | sed -E 's/^[^i]*//; s/[[:space:]]+$//')
while read -r c; do
  [ -n "$c" ] || continue
  case "$(basename "$f")" in
    deploy.yml) [ "$c" = "if: github.event.workflow_run.conclusion == 'success'" ] || chk "deploy.yml の if は、e2e の成功の 1 つだけ(見つかった: $c)" ;;
    e2e.yml) grep -qE 'github[.](event|ref|base_ref|head_ref)|event_name|refs/' <<<"$c" && chk "if: でブランチ・イベントの段を飛ばしている($c)" ;;
  esac
done <<<"$ifs"
[ "$bad" -eq 0 ]
