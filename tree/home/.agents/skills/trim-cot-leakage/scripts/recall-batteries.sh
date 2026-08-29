#!/usr/bin/env bash
# trim-cot-leakage recall batteries 的一键版本（通用 runner，不含 probe 数据）。
#
# 用法: recall-batteries.sh [scope] [额外排除 glob...]
#   scope 默认当前目录；额外排除 glob 自动加 `!` 前缀，排在 inclusion glob 之后。
#   已内置排除 .git 和本 skill 目录（例子故意含泄漏 wording，会自命中）。
#
# probe 数据在 references/recall-batteries.tsv；新增或调整 probe 改 TSV，不改本脚本。
# 每个 hit 都需要语义判断；误报边界见 references/recall-batteries.md。
set -u

scope="${1:-.}"
if (($#)); then shift; fi

data="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/../references/recall-batteries.tsv"
if [[ ! -r "$data" ]]; then
  printf 'probe data not readable: %s\n' "$data" >&2
  exit 2
fi

exclusions=(
  --glob '!.git/**'
  --glob '!**/trim-cot-leakage/**'
)
for extra in "$@"; do
  exclusions+=(--glob "!${extra#!}")
done

failed=0

# TSV 四列: label, flags, pattern, glob。逐行读、不经 eval，pattern 里的特殊字符原样传给 rg。
# 不用 IFS=$'\t' read：tab 是 IFS 空白，连续 tab 会塌缩、空的 flags 列会丢，字段整体左移。
while IFS= read -r line; do
  if [[ -z "$line" || "$line" == \#* ]]; then continue; fi
  label="${line%%$'\t'*}"; rest="${line#*$'\t'}"
  flags="${rest%%$'\t'*}"; rest="${rest#*$'\t'}"
  if [[ "$rest" == *$'\t'* ]]; then
    pattern="${rest%%$'\t'*}"; glob="${rest#*$'\t'}"
  else
    pattern="$rest"; glob=""
  fi
  args=(-n --hidden)
  if [[ "$flags" == *i* ]]; then args+=(-i); fi
  if [[ -n "$glob" ]]; then args+=(--glob "$glob"); fi
  printf '\n== %s ==\n' "$label"
  rg "${args[@]}" "$pattern" "${exclusions[@]}" "$scope"
  status=$?
  if ((status == 1)); then
    printf '(no hits)\n'
  elif ((status > 1)); then
    printf 'probe failed: rg exit %d\n' "$status" >&2
    failed=1
  fi
done < "$data"

printf '\n'
if ((failed)); then
  printf 'some probes failed; see stderr\n' >&2
  exit 1
fi
printf 'done. every hit needs semantic judgment; zero hits prove nothing without a known-positive check.\n'
