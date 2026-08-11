# ============================================================
# Claude Code provider 切换
#
# provider/model 全部是数据：registry.json（仓库，无 secret）
# + local.json（machine-local overlay，jq 深合并，local 优先）。
# 切换 = 渲染 active.json（${VAR} 在此展开，key 由 ~/.zshrc export），
# claude/cc 经 --settings 注入；环境变量只活在 CC 进程内，shell 零污染。
#
# 渲染时对「注册表全集 − 当前 profile」的变量补 ""（CC 视 "" 为 unset），
# 上一 provider 独有的变量（如 AUTH_TOKEN/API_KEY 交叉）不会残留。
#
# 用法：cc_switch [status]，无参走 fzf 选择
# ============================================================

_ccs_registry="$HOME/.config/cc-switch/registry.json"
_ccs_dir="${XDG_STATE_HOME:-$HOME/.local/state}/cc-switch"
_ccs_local="$_ccs_dir/local.json"
_ccs_active="$_ccs_dir/active.json"
_ccs_profile="$_ccs_dir/profile"

# 合并后的注册表（registry + local overlay，深合并）
_ccs_merged() {
    if [[ -f "$_ccs_local" ]]; then
        jq -s '.[0] * .[1]' "$_ccs_registry" "$_ccs_local"
    else
        jq '.' "$_ccs_registry"
    fi
}

# 全部可选 combo：provider 一行，provider/model 各一行
_ccs_combos() {
    _ccs_merged | jq -r '
        .providers | to_entries[] | .key as $n
        | $n, ($n + "/" + ((.value.models // {}) | to_entries[] | .key))
    '
}

# combo 引用的 key 变量里为空的列表（${VAR} 展开后为空即缺 key）
_ccs_missing_keys() {
    local combo="$1" provider model env_tsv line k v
    provider="${combo%%/*}"
    [[ "$combo" == */* ]] && model="${combo#*/}" || model=""
    env_tsv="$(_ccs_merged | jq -r --arg p "$provider" --arg m "$model" '
        .providers[$p] as $p
        | (($p.env // {}) * (($p.models // {})[$m].env // {}))
        | to_entries[] | .key + "\t" + .value')" || return 1
    while IFS=$'\t' read -r k v; do
        [[ "$v" == *'${'* ]] || continue
        v="${(e)v}"
        [[ -z "$v" ]] && echo "$k"
    done <<<"$env_tsv"
}

# 渲染 active.json：<combo-id>；成功返回 0
_ccs_render() {
    local combo="$1" provider model
    provider="${combo%%/*}"
    [[ "$combo" == */* ]] && model="${combo#*/}" || model=""

    local merged
    merged="$(_ccs_merged)" || return 1

    jq -e --arg p "$provider" --arg m "$model" '
        .providers[$p] as $p
        | $p and (if $m == "" then true else $p.models[$m] end)' \
        <<<"$merged" >/dev/null || {
        echo "未知 provider/model：$combo" >&2
        return 1
    }

    local missing
    missing="$(_ccs_missing_keys "$provider${model:+/$model}")"
    if [[ -n "$missing" ]]; then
        echo "缺少 API key（在 ~/.zshrc export 对应变量）：" >&2
        echo "$missing" | sed 's/^/  /' >&2
        return 1
    fi

    # 当前 profile 的 env（展开 ${VAR}）
    local -A wanted
    local k v
    while IFS=$'\t' read -r k v; do
        wanted[$k]="${(e)v}"
    done < <(jq -r --arg p "$provider" --arg m "$model" '
        .providers[$p] as $p
        | (($p.env // {}) * (($p.models // {})[$m].env // {}))
        | to_entries[] | .key + "\t" + .value' <<<"$merged")

    # 注册表全集中、当前 profile 没有的变量补 ""，清掉上一 provider 残留
    local union_key
    while read -r union_key; do
        [[ -n "${wanted[$union_key]+x}" ]] || wanted[$union_key]=""
    done < <(jq -r '
        [ .providers[] as $p
          | (($p.env // {}), (($p.models // {}) | to_entries[] | .value.env // {}))
          | keys[] ] | unique[]' <<<"$merged")

    # 展开完毕的 model 同时写顶层 model 键，压住 user settings 的 model（--settings 优先级更高）
    local -a jqargs
    for k v in "${(@kv)wanted}"; do
        jqargs+=(--arg "$k" "$v")
    done

    command mkdir -p -- "$_ccs_dir" || return 1
    local tmp="$_ccs_active.tmp.$$"
    jq -n "${jqargs[@]}" --arg model "${wanted[ANTHROPIC_MODEL]:-}" \
        '{env: $ARGS.named} + (if $model == "" then {} else {model: $model} end)' \
        >|"$tmp" || {
        rm -f -- "$tmp"
        return 1
    }
    chmod 600 "$tmp"
    command mv -f -- "$tmp" "$_ccs_active" || return 1

    printf '%s\n' "$provider${model:+/$model}" >|"$_ccs_profile" || return 1
}

cc_switch() {
    if [[ "${1:-}" == "status" ]]; then
        echo "profile  : $(<"$_ccs_profile" 2>/dev/null || echo none)"
        echo "active   : $_ccs_active"
        echo "overlay  : $([[ -f "$_ccs_local" ]] && echo "$_ccs_local" || echo none)"
        if [[ -f "$_ccs_active" ]]; then
            jq -r '.model as $m | "model    : \($m // "unset")",
                   (.env | to_entries[] | "\(.key) = \(.value)")' "$_ccs_active"
        else
            echo "（尚未渲染，任意 cc_switch 选择后生成）"
        fi
        return 0
    fi

    if [[ $# -gt 0 ]]; then
        echo "用法：cc_switch [status]（切换走无参 fzf）" >&2
        return 1
    fi

    if ! command -v fzf >/dev/null 2>&1; then
        echo "未找到 fzf" >&2
        return 1
    fi
    # 只列出 key 齐全的 combo
    local -a combos
    local c
    while read -r c; do
        [[ -z "$(_ccs_missing_keys "$c")" ]] && combos+=("$c")
    done < <(_ccs_combos)
    if (( ${#combos[@]} == 0 )); then
        echo "没有任何 key 齐全的 provider（key 在 ~/.zshrc export）" >&2
        return 1
    fi
    local target
    target="$(
        printf '%s\n' "${combos[@]}" |
            fzf --prompt="Claude Code provider > " --height="40%" --layout="reverse" --border
    )" || return 1
    [[ -n "$target" ]] || return 1

    _ccs_render "$target" || return 1
    echo "已切换到：$(<"$_ccs_profile")（重启 claude 生效）"
}

# 强制走 active profile；--settings 优先级高于 user settings
claude() {
    if [[ -f "$_ccs_active" ]]; then
        command claude --settings "$_ccs_active" "$@"
    else
        command claude "$@"
    fi
}

# 首次使用（active.json 不存在）时按默认 provider 渲染一次
if [[ ! -f "$_ccs_active" && -f "$_ccs_registry" && -n "$KIMI_CODE_API_KEY" ]]; then
    _ccs_render kimi >/dev/null 2>&1
fi
