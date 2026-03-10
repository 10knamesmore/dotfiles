#!/usr/bin/env bash
# Hyprland scrolling layout status for Waybar
# 每个 monitor 独立显示：利用 $WAYBAR_OUTPUT_NAME 获取该 bar 所在 monitor 的 workspace

# ── 配置 ────────────────────────────────────────────────
# 特殊状态
ICON_EMPTY="—"           # 无活动窗口
ICON_FLOAT="  "        # 浮动窗口
ICON_FULLSCREEN="  "   # 全屏窗口

# 列字符：focused 列（当前焦点所在列）
COL_FOCUSED_SINGLE="█"  # 该列只有 1 个窗口
COL_FOCUSED_TOP="▀"     # 焦点在列顶部
COL_FOCUSED_MID="▬"     # 焦点在列中间
COL_FOCUSED_BOT="▄"     # 焦点在列底部

# 列字符：非 focused 列
COL_OTHER_SINGLE="▢"    # 该列只有 1 个窗口
COL_OTHER_MULTI="▣"     # 该列有多个窗口

# 列之间的分隔符
COL_SEP=" "
# ────────────────────────────────────────────────────────

clients=$(hyprctl -j clients 2>/dev/null)

# 确定目标 workspace ID
if [ -n "$WAYBAR_OUTPUT_NAME" ]; then
    ws_id=$(hyprctl -j monitors 2>/dev/null | jq --arg out "$WAYBAR_OUTPUT_NAME" \
        '.[] | select(.name == $out) | .activeWorkspace.id')
fi

if [ -n "$ws_id" ]; then
    # 从该 workspace 中找 focusHistoryID 最小（最近聚焦）的非浮动窗口
    active=$(echo "$clients" | jq --argjson ws "$ws_id" \
        '[.[] | select(.workspace.id == $ws and .floating == false and .mapped == true)]
        | sort_by(.focusHistoryID) | .[0]')
else
    # 回退：无 WAYBAR_OUTPUT_NAME 时使用全局 activewindow（向后兼容）
    active=$(hyprctl -j activewindow 2>/dev/null)
fi

# 无活动窗口
if [ -z "$active" ] || [ "$(echo "$active" | jq 'length')" -eq 0 ]; then
    printf '{"text":"%s","tooltip":"无活动窗口"}\n' "$ICON_EMPTY"
    exit 0
fi

# 浮动窗口
is_floating=$(echo "$active" | jq '.floating')
if [ "$is_floating" = "true" ]; then
    printf '{"text":"%s","tooltip":"浮动窗口"}\n' "$ICON_FLOAT"
    exit 0
fi

# 全屏窗口
is_fullscreen=$(echo "$active" | jq '.fullscreen')
if [ "$is_fullscreen" -ne 0 ] 2>/dev/null; then
    printf '{"text":"%s","tooltip":"全屏模式"}\n' "$ICON_FULLSCREEN"
    exit 0
fi

ws_id=$(echo "$active" | jq '.workspace.id')
win_x=$(echo "$active" | jq '.at[0]')
win_y=$(echo "$active" | jq '.at[1]')

result=$(jq -cn \
    --argjson ws  "$ws_id" \
    --argjson wx  "$win_x" \
    --argjson wy  "$win_y" \
    --argjson clients "$clients" \
    --arg c_foc_single "$COL_FOCUSED_SINGLE" \
    --arg c_foc_top    "$COL_FOCUSED_TOP" \
    --arg c_foc_mid    "$COL_FOCUSED_MID" \
    --arg c_foc_bot    "$COL_FOCUSED_BOT" \
    --arg c_oth_single "$COL_OTHER_SINGLE" \
    --arg c_oth_multi  "$COL_OTHER_MULTI" \
    --arg sep          "$COL_SEP" '
    # 按列分组：非浮动、已映射，按 X 坐标分组后排序
    ($clients | [.[] | select(
        .workspace.id == $ws and
        .floating == false and
        .mapped == true
    )] | group_by(.at[0]) | sort_by(.[0].at[0])) as $col_groups |

    ($col_groups | length) as $total |

    # 当前列索引
    ([range($total) | select($col_groups[.][0].at[0] == $wx)] | .[0] // 0) as $cur_idx |
    ($cur_idx + 1) as $cur |

    # 构建列图
    ($col_groups | to_entries | map(
        .key as $col_idx |
        (.value | sort_by(.at[1])) as $wins |
        ($wins | length) as $cnt |

        if $col_idx == $cur_idx then
            if $cnt == 1 then $c_foc_single
            else
                ([range($cnt) | select($wins[.].at[1] == $wy)] | .[0] // 0) as $fi |
                if   $fi == 0        then $c_foc_top
                elif $fi == ($cnt-1) then $c_foc_bot
                else                      $c_foc_mid
                end
            end
        else
            if $cnt == 1 then $c_oth_single else $c_oth_multi end
        end
    ) | join($sep)) as $col_map |

    # tooltip 详情
    ($col_groups | to_entries | map(
        (.key + 1) as $n |
        (.value | length) as $cnt |
        (if .key == $cur_idx then "▶ 列\($n): \($cnt) 窗口 ◀" else "  列\($n): \($cnt) 窗口" end)
    ) | join("\n")) as $detail |

    {
      text:    "\($col_map) \($cur)/\($total)",
      tooltip: "Scrolling Layout\n\($detail)\n滚轮调整列宽"
    }
')

printf '%s\n' "$result"
