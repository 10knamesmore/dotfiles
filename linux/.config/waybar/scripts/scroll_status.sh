#!/usr/bin/env bash
# Hyprland scrolling layout status for Waybar

active=$(hyprctl -j activewindow 2>/dev/null)

# 无活动窗口
if [ -z "$active" ] || [ "$(echo "$active" | jq 'length')" -eq 0 ]; then
    printf '{"text":"—","tooltip":"无活动窗口"}\n'
    exit 0
fi

# 浮动窗口
is_floating=$(echo "$active" | jq '.floating')
if [ "$is_floating" = "true" ]; then
    printf '{"text":"  ","tooltip":"浮动窗口"}\n'
    exit 0
fi

# 全屏窗口
is_fullscreen=$(echo "$active" | jq '.fullscreen')
if [ "$is_fullscreen" -ne 0 ] 2>/dev/null; then
    printf '{"text":"  ","tooltip":"全屏模式"}\n'
    exit 0
fi

ws_id=$(echo "$active" | jq '.workspace.id')
win_x=$(echo "$active" | jq '.at[0]')
win_y=$(echo "$active" | jq '.at[1]')

clients=$(hyprctl -j clients 2>/dev/null)

result=$(jq -cn \
    --argjson ws "$ws_id" \
    --argjson wx "$win_x" \
    --argjson wy "$win_y" \
    --argjson clients "$clients" '
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
            # ── focused 列：上/中/下三态 ──
            if $cnt == 1 then "█"
            else
                ([range($cnt) | select($wins[.].at[1] == $wy)] | .[0] // 0) as $fi |
                if   $fi == 0        then "▀"
                elif $fi == ($cnt-1) then "▄"
                else                      "▬"
                end
            end
        else
            # ── 非 focused 列：单窗口空心，多窗口内含点 ──
            if $cnt == 1 then "▢" else "▣" end
        end
    ) | join(" ")) as $col_map |

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
