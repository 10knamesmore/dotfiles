# ============================================================
# Claude Code provider 切换
# API key 一律由 ~/.zshrc export（本仓库不含任何 secret）；
# 这里只放 URL/模型名配置 + 切换/恢复逻辑。
# 用法：cc_switch [kimi|ds|ftai|opencode|status]，无参走 fzf 选择
# ============================================================

# ---- provider 非 secret 配置（key 由 ~/.zshrc 提供）----
export OPENCODE_BASE_URL=https://opencode.ai/zen/go
export KIMI_ANTHROPIC_URL=https://api.kimi.com/coding/
export DEEPSEEK_ANTHROPIC_URL=https://api.deepseek.com/anthropic
export FTAI_ANTHROPIC_URL=https://llm-proxy.ftai.chat

export DEEPSEEK_V4_FLASH_MODEL="deepseek-v4-flash"
export KIMI_K3_1M_MODEL="k3"
export KIMI_K3_256K_MODEL="k3-256k"
export KIMI_K2_7_HIGHSPEED_MODEL="kimi-for-coding-highspeed"
export FTAI_GLM_MODEL="glm-5.2-auto"

# 保存上次选择的 provider
export ANTHROPIC_PROVIDER_FILE="${XDG_STATE_HOME:-$HOME/.local/state}/anthropic/provider"

_anthropic_init_state_file() {
    local state_dir

    state_dir="$(dirname -- "$ANTHROPIC_PROVIDER_FILE")" || return 1

    command mkdir -p -- "$state_dir" || {
        echo "无法创建状态目录：$state_dir" >&2
        return 1
    }

    if [[ ! -e "$ANTHROPIC_PROVIDER_FILE" ]]; then
        : >|"$ANTHROPIC_PROVIDER_FILE" || {
            echo "无法创建状态文件：$ANTHROPIC_PROVIDER_FILE" >&2
            return 1
        }
    fi
}

# alias → canonical（kimi/ds/ftai），未知返回 1
_anthropic_normalize_provider() {
    case "$1" in
    kimi | k) echo kimi ;;
    ds | deepseek | d) echo ds ;;
    ftai | proxy | f) echo ftai ;;
    opencode | oc) echo opencode ;;
    *) return 1 ;;
    esac
}

_anthropic_use_opencode() {
    export ANTHROPIC_PROVIDER="opencode"

    export ANTHROPIC_BASE_URL="$OPENCODE_BASE_URL"
    export ANTHROPIC_API_KEY="$OPENCODE_API_KEY"

    export ANTHROPIC_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"

    export ANTHROPIC_DEFAULT_FABLE_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"
    export ANTHROPIC_DEFAULT_OPUS_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"
    export ANTHROPIC_DEFAULT_SONNET_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"
    export ANTHROPIC_DEFAULT_HAIKU_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"

    export CLAUDE_CODE_SUBAGENT_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"
}

_anthropic_use_kimi() {
    export ANTHROPIC_PROVIDER="kimi"

    export ANTHROPIC_BASE_URL="$KIMI_ANTHROPIC_URL"
    export ANTHROPIC_AUTH_TOKEN="$KIMI_CODE_API_KEY"

    export ANTHROPIC_MODEL="$KIMI_K3_256K_MODEL"

    export ANTHROPIC_DEFAULT_FABLE_MODEL="${KIMI_K3_1M_MODEL}[1m]"
    export ANTHROPIC_DEFAULT_OPUS_MODEL="$KIMI_K3_256K_MODEL"
    export ANTHROPIC_DEFAULT_SONNET_MODEL="$KIMI_K3_256K_MODEL"
    export ANTHROPIC_DEFAULT_HAIKU_MODEL="$KIMI_K2_7_HIGHSPEED_MODEL"

    export CLAUDE_CODE_SUBAGENT_MODEL="$KIMI_K3_256K_MODEL"
}

_anthropic_use_deepseek() {
    export ANTHROPIC_PROVIDER="ds"

    export ANTHROPIC_BASE_URL="$DEEPSEEK_ANTHROPIC_URL"
    export ANTHROPIC_AUTH_TOKEN="$DEEPSEEK_API_KEY"

    export ANTHROPIC_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"

    export ANTHROPIC_DEFAULT_FABLE_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"
    export ANTHROPIC_DEFAULT_OPUS_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"
    export ANTHROPIC_DEFAULT_SONNET_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"
    export ANTHROPIC_DEFAULT_HAIKU_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"

    export CLAUDE_CODE_SUBAGENT_MODEL="${DEEPSEEK_V4_FLASH_MODEL}[1m]"
}

_anthropic_use_ftai() {
    export ANTHROPIC_PROVIDER="ftai"

    export ANTHROPIC_BASE_URL="$FTAI_ANTHROPIC_URL"
    export ANTHROPIC_AUTH_TOKEN="$FTAI_API_KEY"

    export ANTHROPIC_MODEL="${FTAI_GLM_MODEL}[1m]"

    export ANTHROPIC_DEFAULT_FABLE_MODEL="${FTAI_GLM_MODEL}[1m]"
    export ANTHROPIC_DEFAULT_OPUS_MODEL="${FTAI_GLM_MODEL}[1m]"
    export ANTHROPIC_DEFAULT_SONNET_MODEL="${FTAI_GLM_MODEL}[1m]"
    export ANTHROPIC_DEFAULT_HAIKU_MODEL="${FTAI_GLM_MODEL}[1m]"

    export CLAUDE_CODE_SUBAGENT_MODEL="${FTAI_GLM_MODEL}[1m]"
}

_anthropic_apply_provider() {
    local provider
    provider="$(_anthropic_normalize_provider "$1")" || {
        echo "未知 provider：$1" >&2
        return 1
    }

    case "$provider" in
    kimi) _anthropic_use_kimi ;;
    ds) _anthropic_use_deepseek ;;
    ftai) _anthropic_use_ftai ;;
    opencode) _anthropic_use_opencode ;;
    esac
}

# cc_switch          使用 fzf 选择
cc_switch() {
    local provider="${1:-}"

    if [[ "$provider" == "status" ]]; then
        echo "provider : ${ANTHROPIC_PROVIDER:-unknown}"
        echo "base_url : ${ANTHROPIC_BASE_URL:-unset}"
        echo "model    : ${ANTHROPIC_MODEL:-unset}"
        echo "fable    : ${ANTHROPIC_DEFAULT_FABLE_MODEL:-unset}"
        echo "opus     : ${ANTHROPIC_DEFAULT_OPUS_MODEL:-unset}"
        echo "sonnet   : ${ANTHROPIC_DEFAULT_SONNET_MODEL:-unset}"
        echo "haiku    : ${ANTHROPIC_DEFAULT_HAIKU_MODEL:-unset}"
        echo "subagent : ${CLAUDE_CODE_SUBAGENT_MODEL:-unset}"
        echo "state    : ${ANTHROPIC_PROVIDER_FILE:-unset}"
        return 0
    fi

    if [[ -z "$provider" ]]; then
        if ! command -v fzf >/dev/null 2>&1; then
            echo "未找到 fzf，请先安装：brew install fzf" >&2
            return 1
        fi

        # 只列出已配置 API key 的 provider（key 由 ~/.zshrc 提供）
        local providers=()
        [[ -n "$KIMI_CODE_API_KEY" ]] && providers+=("kimi")
        [[ -n "$DEEPSEEK_API_KEY" ]] && providers+=("ds")
        [[ -n "$FTAI_API_KEY" ]] && providers+=("ftai")
        [[ -n "$OPENCODE_API_KEY" ]] && providers+=("opencode")

        if (( ${#providers[@]} == 0 )); then
            echo "未配置任何 provider 的 API key（请在 ~/.zshrc 中 export 对应 key）" >&2
            return 1
        fi

        provider="$(
            printf '%s\n' "${providers[@]}" |
                fzf \
                    --prompt="Claude Code provider > " \
                    --height="40%" \
                    --layout="reverse" \
                    --border
        )" || return 1

        [[ -n "$provider" ]] || return 1
    fi

    provider="$(_anthropic_normalize_provider "$provider")" || {
        echo "用法：cc_switch [kimi|ds|ftai|opencode|status]" >&2
        return 1
    }

    case "$provider" in
    kimi) _provider_key="${KIMI_CODE_API_KEY:-}" ;;
    ds) _provider_key="${DEEPSEEK_API_KEY:-}" ;;
    ftai) _provider_key="${FTAI_API_KEY:-}" ;;
    opencode) _provider_key="${OPENCODE_API_KEY:-}" ;;
    esac
    if [[ -z "$_provider_key" ]]; then
        echo "未配置 $provider 的 API key（请在 ~/.zshrc 中 export 对应 key）" >&2
        return 1
    fi
    unset _provider_key

    _anthropic_apply_provider "$provider" || return 1
    _anthropic_init_state_file || return 1

    printf '%s\n' "$provider" >|"$ANTHROPIC_PROVIDER_FILE" || {
        echo "无法写入状态文件：$ANTHROPIC_PROVIDER_FILE" >&2
        return 1
    }

    echo "已切换到：$ANTHROPIC_PROVIDER"
    echo "Base URL：$ANTHROPIC_BASE_URL"
    echo "Model：$ANTHROPIC_MODEL"
}

# Shell 启动：恢复上次 provider
if _anthropic_init_state_file; then
    if [[ -s "$ANTHROPIC_PROVIDER_FILE" ]]; then
        _saved_anthropic_provider="$(<"$ANTHROPIC_PROVIDER_FILE")"

        if ! _anthropic_apply_provider "$_saved_anthropic_provider" 2>/dev/null; then
            _anthropic_use_kimi
            printf '%s\n' "kimi" >|"$ANTHROPIC_PROVIDER_FILE"
        fi

        unset _saved_anthropic_provider
    else
        _anthropic_use_kimi
        printf '%s\n' "kimi" >|"$ANTHROPIC_PROVIDER_FILE"
    fi
else
    # 状态文件初始化失败时，至少保证当前终端可用
    _anthropic_use_kimi
fi
