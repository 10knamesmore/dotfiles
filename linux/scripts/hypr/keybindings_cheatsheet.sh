#!/usr/bin/env bash
set -euo pipefail

# 快捷键速查：解析 keybindings.conf 并在浮动窗口中显示
# 如果窗口已存在则 focus，否则新建

CLASS="keybindings-cheatsheet"
CONFIG="$HOME/.config/hypr/keybindings.conf"

# 已有窗口则 focus
existing="$(hyprctl -j clients 2>/dev/null | jq -r ".[] | select(.class == \"$CLASS\") | .address" | head -1)"
if [[ -n "$existing" ]]; then
  hyprctl dispatch focuswindow "address:$existing" >/dev/null
  exit 0
fi

# 解析并格式化 keybindings.conf
format_keybindings() {
  local section=""

  # 颜色定义
  local C_SECTION='\033[1;35m'  # 紫色粗体 — 分类标题
  local C_KEY='\033[1;36m'      # 青色粗体 — 快捷键
  local C_DESC='\033[0;37m'     # 白色 — 功能描述
  local C_DIM='\033[2m'         # 暗色 — 分隔线
  local C_RESET='\033[0m'

  echo -e "${C_SECTION}  Hyprland 快捷键速查${C_RESET}"
  echo -e "${C_DIM}  ──────────────────────────────────────────${C_RESET}"
  echo ""

  while IFS= read -r line; do
    # 空行
    [[ -z "${line// /}" ]] && continue

    # 注释行 → 分类标题
    if [[ "$line" =~ ^[[:space:]]*#[[:space:]]*(.*) ]]; then
      local comment="${BASH_REMATCH[1]}"
      # 跳过无意义的注释（变量定义、URL、空注释等）
      [[ "$comment" =~ ^(\$|请参|bind|menu|http|=) ]] && continue
      [[ -z "${comment// /}" ]] && continue
      echo -e "\n${C_SECTION}  $comment${C_RESET}"
      echo -e "${C_DIM}  ────────────────────────────────────${C_RESET}"
      continue
    fi

    # 变量定义行 → 跳过
    [[ "$line" =~ ^\$ ]] && continue

    # bind 行 → 解析快捷键和功能
    if [[ "$line" =~ ^[[:space:]]*(bind[a-z]*)[[:space:]]*=[[:space:]]*(.*) ]]; then
      local parts="${BASH_REMATCH[2]}"

      # 按逗号分割：修饰键, 键名, 动作, 参数...
      IFS=',' read -ra fields <<< "$parts"
      [[ ${#fields[@]} -lt 3 ]] && continue

      local mod="${fields[0]// /}"
      local key="${fields[1]// /}"
      local action="${fields[2]// /}"
      local arg=""
      [[ ${#fields[@]} -ge 4 ]] && arg="${fields[3]}"
      arg="${arg## }"  # 去前导空格

      # 构建快捷键显示名
      local keyname=""
      mod="${mod//\$mainMod/Super}"
      mod="${mod//SHIFT/Shift}"
      mod="${mod//CONTROL/Ctrl}"
      [[ -n "$mod" ]] && keyname="$mod + "
      # 特殊键名映射
      case "$key" in
        mouse:272) key="鼠标左键" ;;
        mouse:273) key="鼠标右键" ;;
        period)    key="." ;;
        slash)     key="/" ;;
        TAB)       key="Tab" ;;
        XF86Audio*|XF86Mon*) keyname=""; key="$key" ;;
      esac
      keyname="${keyname}${key}"

      # 构建功能描述
      local desc=""
      case "$action" in
        exec)
          # 从命令中提取有意义的描述
          case "$arg" in
            *toggle_fullscreen*)  desc="切换全屏" ;;
            *layout_dispatch*shift*) desc="移动窗口" ;;
            *layout_dispatch*ctrl*)  desc="调整窗口大小" ;;
            *opacity_toggle*)     desc="切换窗口透明度" ;;
            *quick_note*)         desc="Quick Note 浮窗" ;;
            *focus_mode*)         desc="切换专注模式" ;;
            *screen_record_toggle*region*) desc="选区录屏" ;;
            *screen_record_toggle*) desc="切换录屏" ;;
            *workspace_save*)     desc="保存工作区布局" ;;
            *workspace_restore*)  desc="恢复工作区布局" ;;
            *monitor_profile*)    desc="显示器模式切换" ;;
            *keybindings_cheatsheet*) desc="快捷键速查 (本窗口)" ;;
            *killall*waybar*)     desc="切换 Waybar 显示" ;;
            *launch_yazi*)        desc="打开文件管理器 (Yazi)" ;;
            *hyprshot*region*)    desc="截图 (选区)" ;;
            *hyprshot*window*)    desc="截图 (窗口)" ;;
            *wlogout*)            desc="注销菜单" ;;
            *hyprlock*)           desc="锁屏" ;;
            *wpctl*volume*+*)     desc="音量 +" ;;
            *wpctl*volume*-*)     desc="音量 -" ;;
            *wpctl*mute*SINK*toggle*) desc="静音切换" ;;
            *wpctl*mute*SOURCE*)  desc="麦克风静音" ;;
            *brightnessctl*+*)    desc="亮度 +" ;;
            *brightnessctl*-*)    desc="亮度 -" ;;
            *playerctl*next*)     desc="下一曲" ;;
            *playerctl*prev*)     desc="上一曲" ;;
            *playerctl*play*)     desc="播放/暂停" ;;
            *\$terminal*)         desc="打开终端" ;;
            *\$menu*|*fuzzel*)    desc="应用启动器" ;;
            *)                    desc="$arg" ;;
          esac
          ;;
        killactive*)           desc="关闭窗口" ;;
        togglefloating*)       desc="切换浮动" ;;
        movefocus*)            desc="移动焦点 ($arg)" ;;
        workspace*)            desc="切换到工作区 $arg" ;;
        movetoworkspace*)      desc="移动窗口到工作区 $arg" ;;
        movetoworkspacesilent*) desc="静默移动到工作区 $arg" ;;
        togglespecialworkspace*) desc="切换特殊工作区" ;;
        movewindow*)           desc="拖动移动窗口" ;;
        resizewindow*)         desc="拖动调整大小" ;;
        togglegroup*)          desc="切换窗口分组" ;;
        changegroupactive*)    desc="切换组内窗口 ($arg)" ;;
        layoutmsg*)            desc="布局: $arg" ;;
        global*)               desc="$arg" ;;
        *)                     desc="$action $arg" ;;
      esac

      printf "  ${C_KEY}%-28s${C_RESET} ${C_DESC}%s${C_RESET}\n" "$keyname" "$desc"
    fi
  done < "$CONFIG"

  echo ""
  echo -e "${C_DIM}  按 q 退出${C_RESET}"
}

# 在浮动 kitty 窗口中显示
kitty --class "$CLASS" -o initial_window_width=700 -o initial_window_height=800 \
  bash -c "$(declare -f format_keybindings); CONFIG='$CONFIG' format_keybindings | less -R" &
