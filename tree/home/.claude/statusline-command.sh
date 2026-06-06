#!/bin/bash
# Claude Code status line — mirrors Starship Catppuccin Mocha prompt style

input=$(cat)

# ---------- Colors (Catppuccin Mocha palette) ----------
RED=$'\033[38;2;243;139;168m'      # red
PEACH=$'\033[38;2;250;179;135m'    # peach
YELLOW=$'\033[38;2;249;226;175m'   # yellow
GREEN=$'\033[38;2;166;227;161m'    # green
LAVENDER=$'\033[38;2;180;190;254m' # lavender
SKY=$'\033[38;2;137;220;235m'      # sky
OVERLAY2=$'\033[38;2;147;153;178m' # overlay2
MAUVE=$'\033[38;2;203;166;247m'    # mauve
RESET=$'\033[0m'

# ---------- Data from Claude Code ----------
cwd=$(echo "$input" | jq -r '.workspace.current_dir // .cwd // empty')
model=$(echo "$input" | jq -r '.model.display_name // empty')
used_pct=$(echo "$input" | jq -r '.context_window.used_percentage // empty')
ctx_size=$(echo "$input" | jq -r '.context_window.context_window_size // empty')
total_in=$(echo "$input" | jq -r '.context_window.total_input_tokens // empty')
cur_in=$(echo "$input" | jq -r '.context_window.current_usage.input_tokens // empty')
cur_out=$(echo "$input" | jq -r '.context_window.current_usage.output_tokens // empty')
cache_write=$(echo "$input" | jq -r '.context_window.current_usage.cache_creation_input_tokens // empty')
cache_read=$(echo "$input" | jq -r '.context_window.current_usage.cache_read_input_tokens // empty')
cost=$(echo "$input" | jq -r '.cost.total_cost_usd // empty')
lines_added=$(echo "$input" | jq -r '.cost.total_lines_added // empty')
lines_removed=$(echo "$input" | jq -r '.cost.total_lines_removed // empty')
fiveh_pct=$(echo "$input" | jq -r '.rate_limits.five_hour.used_percentage // empty')
fiveh_resets=$(echo "$input" | jq -r '.rate_limits.five_hour.resets_at // empty')
week_pct=$(echo "$input" | jq -r '.rate_limits.seven_day.used_percentage // empty')
week_resets=$(echo "$input" | jq -r '.rate_limits.seven_day.resets_at // empty')
duration_ms=$(echo "$input" | jq -r '.cost.total_duration_ms // empty')
api_duration_ms=$(echo "$input" | jq -r '.cost.total_api_duration_ms // empty')

# ---------- Session duration ----------
session_duration=""
if [ -n "$duration_ms" ]; then
    elapsed=$((duration_ms / 1000))
    dur_h=$((elapsed / 3600))
    dur_m=$(((elapsed % 3600) / 60))
    dur_s=$((elapsed % 60))
    if [ "$dur_h" -gt 0 ]; then
        session_duration="${dur_h}h${dur_m}m"
    elif [ "$dur_m" -gt 0 ]; then
        session_duration="${dur_m}m${dur_s}s"
    else
        session_duration="${dur_s}s"
    fi
fi

api_duration=""
if [ -n "$api_duration_ms" ]; then
    api_elapsed=$((api_duration_ms / 1000))
    api_h=$((api_elapsed / 3600))
    api_m=$(((api_elapsed % 3600) / 60))
    api_s=$((api_elapsed % 60))
    if [ "$api_h" -gt 0 ]; then
        api_duration="${api_h}h${api_m}m"
    elif [ "$api_m" -gt 0 ]; then
        api_duration="${api_m}m${api_s}s"
    else
        api_duration="${api_s}s"
    fi
fi

# ---------- User + host ----------
user_part="$(whoami)"
host_part="$(hostname -s)"

# ---------- Shorten cwd (replace $HOME with ~) ----------
if [ -n "$cwd" ]; then
    home_dir="$HOME"
    display_dir="${cwd#"$home_dir"}"
    if [ "$display_dir" != "$cwd" ]; then
        display_dir=" ~/${display_dir}"
    fi
else
    display_dir=" $(pwd)"
fi

# ---------- Git branch + state + status（符号词汇表对齐 starship 同名段）----------
git_branch=""
git_state_part=""
git_status_part=""
if [ -n "$cwd" ] && command -v git >/dev/null 2>&1; then
    status_out=$(git -C "$cwd" --no-optional-locks status --porcelain=v2 --branch 2>/dev/null)
    branch=""
    ahead=0 behind=0 staged=0 modified=0 deleted=0 renamed=0 untracked=0 conflicted=0
    if [ -n "$status_out" ]; then
        while IFS= read -r line; do
            case "$line" in
            "# branch.head "*)
                branch="${line#\# branch.head }"
                ;;
            "# branch.ab "*)
                ab="${line#\# branch.ab }"
                ahead="${ab%% *}" ahead="${ahead#+}"
                behind="${ab##* }" behind="${behind#-}"
                ;;
            "1 "* | "2 "*)
                x="${line:2:1}" y="${line:3:1}"
                [ "$x" != "." ] && staged=$((staged + 1))
                case "$y" in M | T) modified=$((modified + 1)) ;; D) deleted=$((deleted + 1)) ;; esac
                [ "${line:0:1}" = "2" ] && renamed=$((renamed + 1))
                ;;
            "u "*) conflicted=$((conflicted + 1)) ;;
            "? "*) untracked=$((untracked + 1)) ;;
            esac
        done <<<"$status_out"
        stashed=$(git -C "$cwd" --no-optional-locks stash list 2>/dev/null | wc -l)

        # git_state：rebase/merge/cherry-pick 等进行中的操作（探 .git 标志文件，零额外 git 状态进程）
        git_dir=$(git -C "$cwd" rev-parse --absolute-git-dir 2>/dev/null)
        if [ -n "$git_dir" ]; then
            state="" step="" total=""
            if [ -d "$git_dir/rebase-merge" ]; then
                state="REBASING"
                step=$(cat "$git_dir/rebase-merge/msgnum" 2>/dev/null)
                total=$(cat "$git_dir/rebase-merge/end" 2>/dev/null)
            elif [ -d "$git_dir/rebase-apply" ]; then
                if [ -f "$git_dir/rebase-apply/rebasing" ]; then state="REBASING"; else state="AM"; fi
                step=$(cat "$git_dir/rebase-apply/next" 2>/dev/null)
                total=$(cat "$git_dir/rebase-apply/last" 2>/dev/null)
            elif [ -f "$git_dir/MERGE_HEAD" ]; then
                state="MERGING"
            elif [ -f "$git_dir/CHERRY_PICK_HEAD" ]; then
                state="CHERRY-PICKING"
            elif [ -f "$git_dir/REVERT_HEAD" ]; then
                state="REVERTING"
            elif [ -f "$git_dir/BISECT_LOG" ]; then
                state="BISECTING"
            fi
            if [ -n "$state" ]; then
                if [ -n "$step" ] && [ -n "$total" ]; then
                    git_state_part="${YELLOW}(${state} ${step}/${total})${RESET}"
                else
                    git_state_part="${YELLOW}(${state})${RESET}"
                fi
            fi
            # rebase 中 HEAD 处于 detached：从 rebase 状态文件找回原分支名
            if [ "$branch" = "(detached)" ] && [ -f "$git_dir/rebase-merge/head-name" ]; then
                branch=$(cat "$git_dir/rebase-merge/head-name" 2>/dev/null)
                branch="${branch#refs/heads/}"
            fi
        fi

        # 按 starship $all_status$ahead_behind 的顺序拼：conflicted stashed deleted renamed modified staged untracked + ⇡⇣
        st=""
        [ "$conflicted" -gt 0 ] && st="${st}${RED}👎${conflicted}${RESET} "
        [ "$stashed" -gt 0 ] && st="${st}${MAUVE}&${stashed}${RESET} "
        [ "$deleted" -gt 0 ] && st="${st}${RED}✘${deleted}${RESET} "
        [ "$renamed" -gt 0 ] && st="${st}${OVERLAY2}»${renamed}${RESET} "
        [ "$modified" -gt 0 ] && st="${st}${SKY}!${modified}${RESET} "
        [ "$staged" -gt 0 ] && st="${st}${GREEN}${staged}${RESET} "
        [ "$untracked" -gt 0 ] && st="${st}${OVERLAY2}?${untracked}${RESET} "
        if [ "$ahead" -gt 0 ] && [ "$behind" -gt 0 ]; then
            st="${st}${PEACH}⇕⇡${ahead}⇣${behind}${RESET} "
        elif [ "$ahead" -gt 0 ]; then
            st="${st}${PEACH}⇡${ahead}${RESET} "
        elif [ "$behind" -gt 0 ]; then
            st="${st}${PEACH}⇣${behind}${RESET} "
        fi
        git_status_part="${st% }"
    fi
    if [ -n "$branch" ] && [ "$branch" != "(detached)" ]; then
        # remote URL → https 网页地址（取不到或不认识的形式就回退纯文本）
        remote_url=$(git -C "$cwd" remote get-url origin 2>/dev/null)
        web_url=""
        case "$remote_url" in
        git@*)                                       # git@github.com:owner/repo.git
            stripped="${remote_url#git@}"            # github.com:owner/repo.git
            web_url="https://${stripped/:/\/}"       # 先换冒号再拼 scheme，避免换掉 https:// 的冒号
            ;;
        ssh://git@*)                                 # ssh://git@host/owner/repo.git
            web_url="https://${remote_url#ssh://git@}"
            ;;
        https://* | http://*)
            web_url="$remote_url"
            ;;
        esac
        web_url="${web_url%.git}"
        if [ -n "$web_url" ]; then
            # OSC8 用 BEL 终止符——ST（ESC+反斜杠）的 ANSI-C 写法会触发 tree-sitter-bash 解析 bug
            link_open=$'\033]8;;'"${web_url}/tree/${branch}"$'\a'
            link_close=$'\033]8;;\a'
            git_branch="${YELLOW}  ${link_open}${branch}${link_close}${RESET}"
        else
            git_branch="${YELLOW}  ${branch}${RESET}"
        fi
    fi
fi

# ---------- Context window ----------
# token 数人话化：1234567 → 1.2M，350000 → 350k
fmt_tokens() {
    if [ "$1" -ge 1000000 ]; then
        m_int=$(($1 / 1000000))
        m_dec=$((($1 % 1000000) / 100000))
        if [ "$m_dec" -eq 0 ]; then printf '%dM' "$m_int"; else printf '%d.%dM' "$m_int" "$m_dec"; fi
    elif [ "$1" -ge 1000 ]; then
        printf '%dk' "$(($1 / 1000))"
    else
        printf '%d' "$1"
    fi
}

ctx_part=""
if [ -n "$total_in" ] && [ -n "$ctx_size" ] && [ "$ctx_size" -gt 0 ]; then
    if [ -n "$used_pct" ]; then
        used_int=$(printf '%.0f' "$used_pct")
    else
        used_int=$((total_in * 100 / ctx_size))
    fi
    # 250k+ 进入长上下文计价档兼质量衰减拐点 → 至少黄色警戒
    if [ "$used_int" -ge 70 ]; then
        ctx_color="$RED" ctx_emoji="🥵"
    elif [ "$used_int" -ge 50 ] || [ "$total_in" -ge 250000 ]; then
        ctx_color="$YELLOW" ctx_emoji="😢"
    else
        ctx_color="$GREEN" ctx_emoji="😎"
    fi
    ctx_part="${ctx_emoji} ${ctx_color}$(fmt_tokens "$total_in")/$(fmt_tokens "$ctx_size")${RESET}"
fi

# ---------- Helper: remaining time string ----------
# usage: time_left_str <resets_at_epoch>  → sets $time_left
time_left_str() {
    now=$(date +%s)
    secs=$(($1 - now))
    if [ "$secs" -le 0 ]; then
        time_left=""
    else
        h=$((secs / 3600))
        m=$(((secs % 3600) / 60))
        time_left="${OVERLAY2}(${h}h${m}m)${RESET}"
    fi
}

# ---------- Rate limit color ----------
rate_color() {
    if [ "$1" -ge 80 ]; then
        echo "$RED"
    elif [ "$1" -ge 50 ]; then
        echo "$YELLOW"
    else
        echo "$GREEN"
    fi
}

# ---------- 5-hour rate limit ----------
fiveh_part=""
if [ -n "$fiveh_pct" ]; then
    fiveh_int=$(printf '%.0f' "$fiveh_pct")
    col=$(rate_color "$fiveh_int")
    fiveh_part="${col}5h:${fiveh_int}%${RESET}"
    if [ -n "$fiveh_resets" ]; then
        time_left_str "$fiveh_resets"
        fiveh_part="${fiveh_part}${time_left}"
    fi
fi

# ---------- Weekly rate limit ----------
week_part=""
if [ -n "$week_pct" ]; then
    week_int=$(printf '%.0f' "$week_pct")
    col=$(rate_color "$week_int")
    week_part="${col}168h:${week_int}%${RESET}"
    if [ -n "$week_resets" ]; then
        time_left_str "$week_resets"
        week_part="${week_part}${time_left}"
    fi
fi

# ---------- Cost ----------
cost_part=""
if [ -n "$cost" ]; then
    cost_fmt=$(printf '%.4f' "$cost")
    cost_part="${OVERLAY2}\$${cost_fmt}${RESET}"
fi

# ---------- Lines changed ----------
lines_part=""
if [ -n "$lines_added" ] || [ -n "$lines_removed" ]; then
    la="${lines_added:-0}"
    lr="${lines_removed:-0}"
    lines_part="${GREEN}+${la}${RESET}${OVERLAY2}/${RESET}${RED}-${lr}${RESET}"
fi

sep() { printf '%s' " ${OVERLAY2}|${RESET} "; }

# ---------- Line 1: identity（user@host + 目录 + 分支）----------
printf '%s' "${OVERLAY2}[${RESET}"
printf '%s' "${PEACH}${user_part}${RESET}${OVERLAY2}@${RESET}${RED}${host_part}${RESET}"
printf '%s' "${OVERLAY2}]${RESET} "
printf '%s' "${PEACH}${display_dir}${RESET}"

if [ -n "$git_branch" ]; then
    printf ' %s' "$git_branch"
fi

# starship 同款顺序：$git_branch$git_state$git_status
if [ -n "$git_state_part" ]; then
    printf ' %s' "$git_state_part"
fi

if [ -n "$git_status_part" ]; then
    printf ' %s' "$git_status_part"
fi

printf '\n'

# ---------- Line 2: model + 用量 ----------
line2=""

if [ -n "$model" ]; then
    printf '%s' "${SKY}${model}${RESET}"
    line2="1"
fi

if [ -n "$ctx_part" ]; then
    [ -n "$line2" ] && sep
    printf '%s' "$ctx_part"
    line2="1"
fi

if [ -n "$fiveh_part" ]; then
    [ -n "$line2" ] && sep
    printf '%s' "$fiveh_part"
    line2="1"
fi

if [ -n "$week_part" ]; then
    [ -n "$line2" ] && sep
    printf '%s' "$week_part"
    line2="1"
fi

if [ -n "$cost_part" ]; then
    [ -n "$line2" ] && sep
    printf '%s' "$cost_part"
    line2="1"
fi

if [ -n "$lines_part" ]; then
    [ -n "$line2" ] && printf ' '
    printf '%s' "$lines_part"
    line2="1"
fi

[ -n "$line2" ] && printf '\n'

# ---------- Line 3: token details ----------
line3=""

# current context usage
if [ -n "$cur_in" ] || [ -n "$cur_out" ]; then
    [ -n "$line3" ] && sep
    ci="${cur_in:-0}"
    co="${cur_out:-0}"
    printf '%s' "${OVERLAY2}now${RESET} ${PEACH}in:${ci}${RESET} ${SKY}out:${co}${RESET}"
    line3="1"
fi

# cache（含命中率 read/(input+write+read)，命中省 ~10x 配额）
if [ -n "$cache_write" ] || [ -n "$cache_read" ]; then
    [ -n "$line3" ] && sep
    cw="${cache_write:-0}"
    cr="${cache_read:-0}"
    printf '%s' "${OVERLAY2}cache${RESET} ${YELLOW}write:${cw}${RESET} ${GREEN}read:${cr}${RESET}"
    hit_denom=$((${cur_in:-0} + cw + cr))
    if [ "$hit_denom" -gt 0 ]; then
        hit=$((cr * 100 / hit_denom))
        if [ "$hit" -ge 90 ]; then
            hit_color="$GREEN"
        elif [ "$hit" -ge 50 ]; then
            hit_color="$YELLOW"
        else
            hit_color="$RED"
        fi
        printf '%s' " ${hit_color}hit:${hit}%${RESET}"
    fi
    line3="1"
fi

# session duration
if [ -n "$session_duration" ]; then
    [ -n "$line3" ] && sep
    printf '%s' "${OVERLAY2}duration${RESET} ${LAVENDER}${session_duration}${RESET}"
    if [ -n "$api_duration" ]; then
        printf '%s' "${OVERLAY2}(api ${RESET}${SKY}${api_duration}${RESET}${OVERLAY2})${RESET}"
    fi
    line3="1"
fi

[ -n "$line3" ] && printf '\n'

exit 0
