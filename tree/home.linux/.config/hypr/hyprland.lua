-- ============================================================
-- Hyprland Lua 主配置。
--
-- 一旦本文件存在，Hyprland 完全接管，忽略所有 .conf
-- hypridle / hyprlock 仍走各自的 .conf（hyprlang）
-- 要切换回 .conf，删除或重命名本文件后重启 Hyprland。
-- ============================================================

local HOME = os.getenv("HOME")
-- dots：仓库根优先环境变量 DOTFILES_DIR，兜底读 ~/.config/dots/root（裸 TTY 启动也不断）
local DOTFILES = os.getenv("DOTFILES_DIR")
if not DOTFILES or DOTFILES == "" then
  local f = io.open(HOME .. "/.config/dots/root", "r")
  if f then
    DOTFILES = (f:read("l") or ""):gsub("%s+$", "")
    f:close()
  end
end
if not DOTFILES or DOTFILES == "" then
  DOTFILES = HOME .. "/dotfiles"
end
local SCRIPTS = DOTFILES .. "/.gen/scripts"
local N = SCRIPTS

local terminal = "kitty"
local fileManager = "dolphin"
local mainMod = "SUPER"

-- ============================================================
-- 显示器
-- ============================================================

-- 优先加载 QuickShell MonitorService 回写的机器本地布局（按当前显示器组合，开机即恢复、无闪烁）。
-- 文件缺失或出错则退回安全默认。完整可视化管理见 QuickShell 控制中心的「显示器」页。
local mlocal = HOME .. "/.local/state/hypr/monitors.local.lua"
local chunk = loadfile(mlocal)
local ok = chunk and pcall(chunk)
if not ok then
  hl.monitor({ output = "eDP-1", mode = "preferred", position = "auto", scale = 1 })
end

-- ============================================================
-- 环境变量
-- ============================================================

hl.env("XCURSOR_SIZE", "24")
-- hl.env("XMODIFIERS", "@im=fcitx")
hl.env("XDG_MENU_PREFIX", "arch-")
hl.env("LIBVA_DRIVER_NAME", "nvidia") -- NVIDIA 硬件加速
hl.env("QT_QPA_PLATFORM", "wayland")
hl.env("QT_STYLE_OVERRIDE", "Breeze")
hl.env("QT_QPA_PLATFORMTHEME", "qt6ct")
hl.env("AQ_DRM_DEVICES", "/dev/dri/card0:/dev/dri/card1")
hl.env("WEBKIT_DISABLE_DMABUF_RENDERER", "1") -- NVIDIA 上 WebKit 渲染

-- ============================================================
-- 主配置
-- ============================================================

hl.config({
  -- ---------- 输入 ----------
  input = {
    kb_layout = "us",
    follow_mouse = 1,
    sensitivity = 0,
    repeat_rate = 70,
    repeat_delay = 200,
    scroll_factor = 2.5,
    touchpad = {
      disable_while_typing = true,
      natural_scroll = true,
    },
    touchdevice = {
      enabled = false,
    },
  },

  xwayland = { force_zero_scaling = true },

  cursor = {
    -- no_hardware_cursors = 1,
    min_refresh_rate = 24,
    hide_on_key_press = true,
    inactive_timeout = 30,
    persistent_warps = true,
    enable_hyprcursor = false,
  },

  render = {
    direct_scanout = 1,
    -- cm_auto_hdr 自动处理 fullscreen HDR passthrough。
  },

  ecosystem = { no_donation_nag = true },

  -- ---------- 外观 ----------
  general = {
    gaps_in = 3,
    gaps_out = {
      top = 5,
      left = 10,
      right = 10,
      bottom = 10,
    },
    border_size = 2,
    resize_on_border = true,
    hover_icon_on_border = true,
    allow_tearing = false,
    layout = "scrolling",
    snap = { enabled = true },
  },

  decoration = {
    rounding = 12,
    rounding_power = 2.0,
    active_opacity = 1,
    inactive_opacity = 0.98,
    fullscreen_opacity = 1,
    dim_inactive = true,
    dim_strength = 0.15,
    dim_special = 0.5,
    blur = {
      enabled = true,
      size = 6,
      passes = 2,
      vibrancy = 0.1696,
      special = true,
      input_methods = true,
    },
    -- shadow.color 使用 gradient；角度动画由 shadowangle leaf 驱动。
    shadow = {
      enabled = true,
      range = 15,
      render_power = 2,
    },

    -- 窗口位移运动模糊
    motion_blur = {
      enabled = false,
      samples = 15,
    },
  },

  animations = {
    enabled = true,
    workspace_wraparound = true,
  },

  -- ---------- 布局 ----------
  dwindle = { preserve_split = true },
  master = { new_status = "master" },
  scrolling = { column_width = 0.51 },

  group = {
    groupbar = {
      enabled = true,
      disable_when_only = false,
      stacked = false,
      render_titles = true,
      gradients = true,
      height = 24,
      font_size = 11,
      font_weight_active = 600,
      font_weight_inactive = 400,
      text_padding = 8,
      indicator_height = 0,
      indicator_gap = 0,
      gaps_in = 6,
      gaps_out = 4,
      keep_upper_gap = false,
      gradient_rounding = 6,
      gradient_round_only_edges = false,
    },
  },

  -- ---------- misc / debug ----------
  misc = {
    force_default_wallpaper = -1,
    disable_hyprland_logo = true,
    animate_manual_resizes = true,
    enable_swallow = false,
    swallow_regex = "^(kitty)$",
    swallow_exception_regex = "^(pnpm tauri dev)$",
    allow_session_lock_restore = true,
  },

  debug = {
    vfr = true,
  },
})

-- 壁纸配色由 dots-wallpaper 生成；换壁纸时通过 eval 热更新。
local wallpaperColors = loadfile(HOME .. "/.local/state/dots/theme/hyprland.lua")
if wallpaperColors then
  wallpaperColors()
end

-- ============================================================
-- 动画曲线 + 动画绑定
-- ============================================================

hl.curve("easeOutQuint", { type = "bezier", points = { { 0.23, 1 }, { 0.32, 1 } } })
hl.curve("easeInOutCubic", { type = "bezier", points = { { 0.65, 0.05 }, { 0.36, 1 } } })
hl.curve("linear", { type = "bezier", points = { { 0, 0 }, { 1, 1 } } })
hl.curve("almostLinear", { type = "bezier", points = { { 0.5, 0.5 }, { 0.75, 1 } } })
hl.curve("quick", { type = "bezier", points = { { 0.15, 0 }, { 0.1, 1 } } })
hl.curve("shellDecelerate", { type = "bezier", points = { { 0.2, 0.9 }, { 0.3, 1 } } })
hl.curve("shellStandard", { type = "bezier", points = { { 0.4, 0 }, { 0.2, 1 } } })

-- spring 动画在中途打断时保持速度连续。
-- 无回弹 ⟺ dampening ≥ 临界阻尼 2√(stiffness·mass)（源码 Spring.cpp: GAMMA<OMEGA0 才振荡）
-- 改 stiffness 必须同步改 dampening，否则重新变欠阻尼、又开始过冲
hl.curve("easy", { type = "spring", mass = 1, stiffness = 250, dampening = 30 })
-- snappy：高刚度=固有频率高=到位快，给窗口移动用；临界 2√200≈28.3
hl.curve("snappy", { type = "spring", mass = 1, stiffness = 200, dampening = 28.3 })

hl.animation({ leaf = "global", enabled = true, speed = 10, bezier = "default" })
hl.animation({ leaf = "border", enabled = true, speed = 5.39, bezier = "easeOutQuint" })

-- style 只接受 "loop" | "once"（AnimationManager.cpp::styleValidInConfigVar）。
-- shadowangle 必须使用 once；loop 会让角度永久旋转并导致闲置时持续重绘。
hl.animation({ leaf = "shadowangle", enabled = true, speed = 5, bezier = "easeOutQuint", style = "once" })
hl.animation({ leaf = "fadeShadow", enabled = true, speed = 3, bezier = "almostLinear" })

-- windows / windowsMove 用高刚度 snappy（快速到位，打断仍平滑）
hl.animation({ leaf = "windows", enabled = true, speed = 8, spring = "snappy" })
hl.animation({ leaf = "windowsMove", enabled = true, speed = 1, spring = "snappy" })
hl.animation({ leaf = "windowsIn", enabled = true, speed = 4.1, bezier = "easeOutQuint", style = "popin 87%" })
hl.animation({ leaf = "windowsOut", enabled = true, speed = 1.49, bezier = "linear", style = "popin 87%" })

hl.animation({ leaf = "layers", enabled = true, speed = 3.81, bezier = "easeOutQuint" })
hl.animation({ leaf = "layersIn", enabled = true, speed = 4, bezier = "shellDecelerate", style = "fade" })
hl.animation({ leaf = "layersOut", enabled = true, speed = 1.5, bezier = "shellStandard", style = "fade" })

hl.animation({ leaf = "fadeIn", enabled = true, speed = 1.73, bezier = "almostLinear" })
hl.animation({ leaf = "fadeOut", enabled = true, speed = 1.46, bezier = "almostLinear" })
hl.animation({ leaf = "fade", enabled = true, speed = 3.03, bezier = "quick" })
hl.animation({ leaf = "fadeLayersIn", enabled = true, speed = 1.79, bezier = "almostLinear" })
hl.animation({ leaf = "fadeLayersOut", enabled = true, speed = 1.39, bezier = "almostLinear" })

-- 负位移比例反转工作区竖向滑动方向
hl.animation({ leaf = "workspaces", enabled = true, speed = 7, spring = "easy", style = "slidevert -100%" })
hl.animation({ leaf = "workspacesIn", enabled = true, speed = 7, spring = "easy", style = "slidevert -100%" })
hl.animation({ leaf = "workspacesOut", enabled = true, speed = 7, spring = "easy", style = "slidevert -100%" })

-- ============================================================
-- 设备配置
-- ============================================================

hl.device({ name = "msft0001:00-04f3:317c-touchpad", enabled = false })

-- ============================================================
-- Window rules
-- ============================================================

hl.window_rule({
  name = "windows-no-blur",
  match = { class = ".*" },
  no_blur = true,
})

hl.window_rule({
  name = "suppress-maximize-events",
  match = { class = ".*" },
  suppress_event = "maximize",
})

hl.window_rule({
  name = "xwayland-dragging-nofocus",
  match = {
    class = "^$",
    title = "^$",
    xwayland = true,
    float = true,
    fullscreen = false,
    pin = false,
  },
  no_focus = true,
})

-- JetBrains: tooltip 不抢焦点（标题为 win.<id>）
hl.window_rule({
  name = "jetbrains-tooltip-noinitialfocus",
  match = { class = "^(.*jetbrains.*)$", title = "^(win.*)$" },
  no_initial_focus = true,
})
hl.window_rule({
  name = "jetbrains-tooltip-nofocus",
  match = { class = "^(.*jetbrains.*)$", title = "^(win.*)$" },
  no_focus = true,
})

-- JetBrains: 拖动 tab（标题为单个空格）
hl.window_rule({
  name = "jetbrains-tabdrag-noinitialfocus",
  match = { class = "^(.*jetbrains.*)$", title = "^\\s$" },
  no_initial_focus = true,
})
hl.window_rule({
  name = "jetbrains-tabdrag-nofocus",
  match = { class = "^(.*jetbrains.*)$", title = "^\\s$" },
  no_focus = true,
})

-- HDR 屏上 Chrome 整窗发暗的解药。
-- Chrome 会给自己的 surface 声明 BT2020+PQ 且 set_luminances(0, 1000, 203)——不管显示器
-- 实际峰值多少都报 1000 nits。DP-3 峰值只有 417，于是 Hyprland 判定需要色调映射
-- （needsTonemap = srcMax >= dstMax*1.01），把 1000 压进 417，连参考白 203 的 SDR 界面
-- 一起压暗。Firefox 从不调 set_image_description，走普通 SDR 路径，所以不受影响——
-- 「Chrome 比 Firefox 暗」就是这么来的，不是显示器或 Hyprland 配错了。
-- tonemap: 0=关 1=默认 2=clamp 3=limited。关掉的代价是 >417 nits 的高光被硬裁剪。
hl.window_rule({
  name = "chrome-no-tonemap",
  match = { class = "^(google-chrome)$" },
  tonemap = 0,
})

hl.window_rule({
  name = "game-fullscreen-passthrough",
  match = { class = "^(steam_app_\\d+|gamescope)" },
  no_blur = true,
  no_anim = true,
  no_shadow = true,
  no_dim = true,
  opaque = true, -- 必须：active_opacity 0.98 会直接掐死 direct scanout
  immediate = true, -- 撕裂换低延迟，不想要就删这行
})

-- `proton run xxx.exe` 或非 Steam 快捷方式启动的游戏，class 是 steam_proton，
-- 拿不到上面那条规则。放任它被平铺会和游戏自己要求的窗口尺寸互相拉扯，整屏闪；
-- 强制全屏则会黑屏（Kirikiri + Wine 的 present 链在窗口被重置后断掉），所以只浮动。
hl.window_rule({
  name = "proton-game-fullscreen",
  match = { class = "^(steam_proton|limelight_lj\\.exe|limelight_lj_ai\\.exe)$" },
  float = true,
  no_blur = true,
  no_anim = true,
  no_shadow = true,
  no_dim = true,
  opaque = true,
})

-- ============================================================
-- Layer rules
-- ============================================================

-- 面板通过 BackgroundEffect 提交局部模糊区域，材质与形变由 QML 驱动。
-- 接管边框和断开按钮必须从首帧起位于固定位置；光标动画由 QML 驱动。
hl.layer_rule({
  name = "computer_control",
  match = { namespace = "^pi-computer-control$" },
  no_anim = true,
})

hl.layer_rule({
  name = "quickshell_capture",
  match = { namespace = "^quickshell-capture(-notice)?$" },
  no_anim = true,
})

hl.layer_rule({
  name = "quickshell_panels",
  match = { namespace = "^quickshell-panel$" },
  no_anim = true,
  ignore_alpha = 0.1,
})

hl.layer_rule({
  name = "quickshell_toasts",
  match = { namespace = "^quickshell-(toast|osd)$" },
  no_anim = true,
  blur = true,
  ignore_alpha = 0.1,
})

hl.layer_rule({
  name = "quickshell_bar_blur",
  match = { namespace = "^quickshell-bar$" },
  blur = true,
  ignore_alpha = 0.1,
})

hl.layer_rule({
  name = "quickshell_blur",
  match = { namespace = "^quickshell$" },
  blur = true,
  ignore_alpha = 0.1,
})

-- ============================================================
-- Keybindings
--
-- 统一走本地 bind()：description 以「分组 · 标签」写入 Hyprland 的 bind 记录。
-- ============================================================

local function bind(keys, desc, dispatcher, opts)
  opts = opts or {}
  opts.description = desc
  hl.bind(keys, dispatcher, opts)
end

-- 应用启动
bind(mainMod .. " + Q", "应用启动 · 终端", hl.dsp.exec_cmd(terminal))
bind(mainMod .. " + D", "窗口 · 关闭", hl.dsp.window.close())
bind(mainMod .. " + F", "窗口 · 切换全屏", hl.dsp.exec_cmd(SCRIPTS .. "/hypr/toggle_fullscreen.sh"))
bind(mainMod .. " + T", "面板 · 切换状态栏", hl.dsp.global("quickshell:toggleBar"))
bind(mainMod .. " + E", "应用启动 · 文件管理器", hl.dsp.exec_cmd(terminal .. " launch_yazi.sh"))
-- ALT+V 对齐 macOS 的浮动快捷键；SUPER+V 留给 xremap 做粘贴。
bind("ALT + V", "窗口 · 切换浮动", hl.dsp.window.float({ action = "toggle" }))
bind(mainMod .. " + R", "应用启动 · 应用启动器", hl.dsp.global("quickshell:launcher"))
bind(mainMod .. " + P", "捕获 · 区域截图", hl.dsp.global("quickshell:captureScreenshot"))
bind(mainMod .. " + SHIFT + P", "捕获 · 窗口截图", hl.dsp.global("quickshell:captureWindow"))
bind(mainMod .. " + ALT + P", "捕获 · 取色", hl.dsp.global("quickshell:captureColor"))
bind(mainMod .. " + CTRL + P", "捕获 · 录屏", hl.dsp.global("quickshell:captureRecord"))
bind(mainMod .. " + CTRL + SHIFT + P", "捕获 · 停止录屏", hl.dsp.global("quickshell:captureStopRecording"))
bind(mainMod .. " + SHIFT + T", "面板 · 控制中心", hl.dsp.global("quickshell:controlCenter"))
bind(mainMod .. " + SHIFT + Q", "会话 · 锁屏", hl.dsp.exec_cmd("hyprlock"))

-- 焦点切换（hjkl）：repeating，按住超过 input.repeat_delay(200ms) 后按 input.repeat_rate 连续移动
bind(mainMod .. " + h", "焦点切换 · ←", hl.dsp.focus({ direction = "left" }), { repeating = true })
bind(mainMod .. " + l", "焦点切换 · →", hl.dsp.focus({ direction = "right" }), { repeating = true })
bind(mainMod .. " + k", "焦点切换 · ↑", hl.dsp.focus({ direction = "up" }), { repeating = true })
bind(mainMod .. " + j", "焦点切换 · ↓", hl.dsp.focus({ direction = "down" }), { repeating = true })

local windowActions = dofile(HOME .. "/.config/hypr/window-actions.lua")
for _, direction in ipairs({
  { key = "H", name = "left", arrow = "←" },
  { key = "J", name = "down", arrow = "↓" },
  { key = "K", name = "up", arrow = "↑" },
  { key = "L", name = "right", arrow = "→" },
}) do
  bind(mainMod .. " + SHIFT + " .. direction.key, "移动窗口 · " .. direction.arrow, function()
    return windowActions.move(direction.name)
  end)
  bind(mainMod .. " + CONTROL + " .. direction.key, "调整大小 · " .. direction.arrow, function()
    return windowActions.resize(direction.name)
  end, { repeating = true })
end

-- 工作区切换 / 移动窗口 / silent 移动
for i = 1, 10 do
  local key = i % 10 -- 10 → 键 "0"
  bind(mainMod .. " + " .. key, "工作区 · 切到 " .. i, hl.dsp.focus({ workspace = i }))
  bind(mainMod .. " + SHIFT + " .. key, "工作区 · 移到 " .. i, hl.dsp.window.move({ workspace = tostring(i) }))
  bind(
    mainMod .. " + CONTROL + " .. key,
    "工作区 · 静默移到 " .. i,
    hl.dsp.window.move({ workspace = tostring(i), silent = true })
  )
end

-- 特殊工作区
bind(mainMod .. " + M", "暂存区 · 切换", hl.dsp.workspace.toggle_special("special"))
bind(mainMod .. " + SHIFT + M", "暂存区 · 移入", hl.dsp.window.move({ workspace = "special" }))

-- 窗口分组
bind(mainMod .. " + s", "分组 · 切换分组", hl.dsp.group.toggle())
bind(mainMod .. " + n", "分组 · 下一个标签", hl.dsp.group.next())
bind(mainMod .. " + CTRL + S", "分组 · 堆叠最近窗口", function()
  local current = hl.get_active_window()
  local recent = hl.get_last_window()
  if not current or not recent or (current.group and current.group == recent.group) then
    return
  end

  if not current.group then
    hl.dispatch(hl.dsp.group.toggle({ window = current }))
  end
  -- add 会合并来源窗口的整组；先移出，只堆叠最近聚焦的那个窗口。
  if recent.group then
    recent.group:remove(recent)
  end
  current.group:add(recent)
  hl.dispatch(hl.dsp.focus({ window = current }))
end)

-- 自定义脚本
bind(mainMod .. " + O", "脚本 · 透明度切换", hl.dsp.exec_cmd(SCRIPTS .. "/hypr/opacity_toggle.sh"))

-- 鼠标拖动 / 调整大小（bindm 等价：mouse = true）
bind(mainMod .. " + mouse:272", "鼠标 · 拖动窗口", hl.dsp.window.drag(), { mouse = true })
bind(mainMod .. " + mouse:273", "鼠标 · 调整窗口大小", hl.dsp.window.resize(), { mouse = true })

-- 多媒体按键（bindel 等价：locked + repeating）
-- 多媒体键先执行 wpctl，再通过 Lua dispatcher 通知 QuickShell OSD。
bind("XF86AudioRaiseVolume", "多媒体 · 音量+", function()
  hl.exec_cmd("wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%+")
  hl.dispatch(hl.dsp.global("quickshell:osdVolume"))
end, { locked = true, repeating = true })

bind("XF86AudioLowerVolume", "多媒体 · 音量-", function()
  hl.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-")
  hl.dispatch(hl.dsp.global("quickshell:osdVolume"))
end, { locked = true, repeating = true })

bind("XF86AudioMute", "多媒体 · 静音切换", function()
  hl.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle")
  hl.dispatch(hl.dsp.global("quickshell:osdVolume"))
end, { locked = true, repeating = true })

bind(
  "XF86AudioMicMute",
  "多媒体 · 麦克风静音",
  hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"),
  { locked = true, repeating = true }
)

-- 播放控制（bindl 等价：locked，锁屏时仍可用）
bind("XF86AudioNext", "播放 · 下一首", hl.dsp.exec_cmd("playerctl next"), { locked = true })
bind("XF86AudioPause", "播放 · 播放/暂停", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
bind("XF86AudioPlay", "播放 · 播放/暂停", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
bind("XF86AudioPrev", "播放 · 上一首", hl.dsp.exec_cmd("playerctl previous"), { locked = true })

-- ============================================================
-- Autostart（exec-once 等价）
-- ============================================================

-- 幂等守卫：同名进程已在跑就跳过。hyprland.start 每次 compositor 启动都触发——
-- 对「无单实例锁」的守护（hypridle/wl-paste），若不守卫，Hyprland 每崩溃/
-- 重启一次就再拉起一份，堆叠出孤儿进程。
-- exec 通过 /bin/sh 解释 `||`；guard 缺省取命令首 token 作进程名。
local function exec_once(cmd, guard)
  hl.exec_cmd("pidof -q " .. (guard or cmd:match("^%S+")) .. " || " .. cmd)
end

hl.on("hyprland.start", function()
  hl.exec_cmd(SCRIPTS .. "/dots-wallpaper")
  -- hl.exec_cmd("fcitx5 -d") -- 自带单实例锁，无需守卫
  exec_once("hypridle")
  exec_once("xremap --watch=config,device ~/.config/xremap/config.yml")
  exec_once("wl-paste --watch cliphist store", "wl-paste")
  exec_once("quickshell") -- 屏幕效果 shader 也由它恢复（ScreenEffectsService）
  hl.exec_cmd('"' .. HOME .. '/.cargo/bin/rime-wayland" serve') -- 数据目录锁保证单实例
  exec_once("systemctl --user start hyprpolkitagent", "hyprpolkitagent") -- polkit 认证代理，GUI 提权（pkexec）弹密码框
  hl.exec_cmd("clash-verge") -- tauri 单实例
  exec_once("google-chrome-stable", "chrome") -- Chrome 进程名是 chrome
end)
