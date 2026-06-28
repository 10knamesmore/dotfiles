-- ============================================================
-- Hyprland 主配置（0.55+ Lua 入口）
--
-- 一旦本文件存在，Hyprland 完全接管，忽略所有 .conf
-- hypridle / hyprlock 仍走各自的 .conf（hyprlang）
-- 回滚：删除或重命名本文件，重启 Hyprland 即可
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
local SCRIPTS = DOTFILES .. "/.gen/scripts" -- 原 $scripts_dir
local N = SCRIPTS -- 原 $n（原 conf 中未定义，按 SCRIPTS 同义处理）

local terminal = "kitty"
local fileManager = "dolphin"
local mainMod = "SUPER"

-- ============================================================
-- 显示器
-- ============================================================

-- 优先加载 QuickShell MonitorService 回写的机器本地布局（按当前显示器组合，开机即恢复、无闪烁）。
-- 文件缺失或出错则退回安全默认。完整可视化管理见 QuickShell「显示器」面板（侧边栏打开）。
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
hl.env("XDG_MENU_PREFIX", "arch-")
hl.env("LIBVA_DRIVER_NAME", "nvidia") -- NVIDIA 硬件加速
hl.env("QT_QPA_PLATFORM", "wayland")
hl.env("QT_STYLE_OVERRIDE", "Breeze")
hl.env("QT_QPA_PLATFORMTHEME", "kde")
hl.env("AQ_DRM_DEVICES", "/dev/dri/card0:/dev/dri/card1")
hl.env("HYPRSHOT_DIR", "Pictures/Screenshots")
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
        no_hardware_cursors = 1,
        min_refresh_rate = 24,
        hide_on_key_press = true,
        inactive_timeout = 30,
        persistent_warps = true,
        enable_hyprcursor = false,
    },

    render = {
        direct_scanout = 1,
        -- 0.55 起 cm_fs_passthrough 已移除，由 cm_auto_hdr 自动处理
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
        col = {
            active_border = { colors = { "rgba(89b4faee)", "rgba(cba6f7ee)" }, angle = 45 },
            inactive_border = "rgba(45475a40)",
        },
        resize_on_border = true,
        hover_icon_on_border = true,
        allow_tearing = false,
        layout = "scrolling",
        snap = { enabled = true },
    },

    decoration = {
        rounding = 12,
        rounding_power = 2.0,
        active_opacity = 0.98,
        inactive_opacity = 0.85,
        fullscreen_opacity = 1,
        dim_inactive = true,
        dim_strength = 0.15,
        dim_special = 0.5,
        blur = {
            enabled = true,
            size = 8,
            passes = 2,
            vibrancy = 0.1696,
            special = true,
            input_methods = true,
        },
        shadow = {
            enabled = true,
            range = 15,
            render_power = 2,
            color = "rgba(11111bee)",
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
        vfr = true, -- 0.55 起 vfr 从 misc 搬到 debug
    },
})

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

-- 0.55 spring 物理动画——比 bezier 在打断时更自然
-- easy：稳重，过阻尼无弹，给工作区切换用
hl.curve("easy", { type = "spring", mass = 1, stiffness = 71.2633, dampening = 18.0 })
-- snappy：高刚度=固有频率高=到位快；dampening 取临界(2√90≈19)保证不过冲。给窗口移动用
hl.curve("snappy", { type = "spring", mass = 1, stiffness = 90, dampening = 19 })

hl.animation({ leaf = "global", enabled = true, speed = 10, bezier = "default" })
hl.animation({ leaf = "border", enabled = true, speed = 5.39, bezier = "easeOutQuint" })

-- windows / windowsMove 用高刚度 snappy（快速到位，打断仍平滑）
hl.animation({ leaf = "windows",     enabled = true, speed = 8,    spring = "snappy" })
hl.animation({ leaf = "windowsMove", enabled = true, speed = 8,    spring = "snappy" })
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

-- scrolling 布局：工作区水平滑动 + 淡入 + spring，跟手感强且过渡更柔
hl.animation({ leaf = "workspaces",    enabled = true, speed = 7, spring = "easy", style = "slidefade" })
hl.animation({ leaf = "workspacesIn",  enabled = true, speed = 7, spring = "easy", style = "slidefade" })
hl.animation({ leaf = "workspacesOut", enabled = true, speed = 7, spring = "easy", style = "slidefade" })

-- ============================================================
-- 设备配置
-- ============================================================

hl.device({ name = "msft0001:00-04f3:317c-touchpad", enabled = false })

-- ============================================================
-- Window rules
-- ============================================================

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

hl.window_rule({
    name = "keybindings-cheatsheet",
    match = { class = "^(keybindings-cheatsheet)$" },
    float = true,
    size = "45% 70%",
    center = true,
    animation = "popin",
})

hl.window_rule({
    name = "quick-note-float",
    match = { class = "^(quick-note)$" },
    float = true,
    size = "50% 60%",
    center = true,
    animation = "popin",
})

-- ============================================================
-- Layer rules
-- ============================================================

hl.layer_rule({
    name = "quickshell_blur",
    match = { namespace = "quickshell" },
    blur = true,
    ignore_alpha = 0.1,
})

-- ============================================================
-- Keybindings
-- ============================================================

-- 应用启动
hl.bind(mainMod .. " + Q", hl.dsp.exec_cmd(terminal))
hl.bind(mainMod .. " + D", hl.dsp.window.close())
hl.bind(mainMod .. " + F", hl.dsp.exec_cmd(SCRIPTS .. "/hypr/toggle_fullscreen.sh"))
hl.bind(mainMod .. " + T", hl.dsp.global("quickshell:toggleBar"))
hl.bind(mainMod .. " + E", hl.dsp.exec_cmd(terminal .. " launch_yazi.sh"))
-- float 挪到 ALT+V，对齐 macOS 的 `alt - v`，并腾出 SUPER+V 给 keyd 做粘贴
hl.bind("ALT + V", hl.dsp.window.float({ action = "toggle" }))
hl.bind(mainMod .. " + R", hl.dsp.global("quickshell:launcher"))
hl.bind(mainMod .. " + S", hl.dsp.layout("togglesplit"))
hl.bind(mainMod .. " + P", hl.dsp.exec_cmd("hyprshot -m region"))
hl.bind(mainMod .. " + SHIFT + P", hl.dsp.exec_cmd("hyprshot -m window"))
hl.bind(mainMod .. " + SHIFT + T", hl.dsp.global("quickshell:settings"))
hl.bind(mainMod .. " + SHIFT + Q", hl.dsp.exec_cmd("hyprlock"))

-- 焦点切换（hjkl）
hl.bind(mainMod .. " + h", hl.dsp.focus({ direction = "left" }))
hl.bind(mainMod .. " + l", hl.dsp.focus({ direction = "right" }))
hl.bind(mainMod .. " + k", hl.dsp.focus({ direction = "up" }))
hl.bind(mainMod .. " + j", hl.dsp.focus({ direction = "down" }))

-- 移动窗口（按当前布局自动分流脚本）
hl.bind(mainMod .. " + SHIFT + H", hl.dsp.exec_cmd(SCRIPTS .. "/hypr/layout_dispatch.sh shift h"))
hl.bind(mainMod .. " + SHIFT + J", hl.dsp.exec_cmd(SCRIPTS .. "/hypr/layout_dispatch.sh shift j"))
hl.bind(mainMod .. " + SHIFT + K", hl.dsp.exec_cmd(SCRIPTS .. "/hypr/layout_dispatch.sh shift k"))
hl.bind(mainMod .. " + SHIFT + L", hl.dsp.exec_cmd(SCRIPTS .. "/hypr/layout_dispatch.sh shift l"))

-- 调整尺寸（binde 等价：repeating = true）
hl.bind(
    mainMod .. " + CONTROL + H",
    hl.dsp.exec_cmd(SCRIPTS .. "/hypr/layout_dispatch.sh ctrl h"),
    { repeating = true }
)
hl.bind(
    mainMod .. " + CONTROL + J",
    hl.dsp.exec_cmd(SCRIPTS .. "/hypr/layout_dispatch.sh ctrl j"),
    { repeating = true }
)
hl.bind(
    mainMod .. " + CONTROL + K",
    hl.dsp.exec_cmd(SCRIPTS .. "/hypr/layout_dispatch.sh ctrl k"),
    { repeating = true }
)
hl.bind(
    mainMod .. " + CONTROL + L",
    hl.dsp.exec_cmd(SCRIPTS .. "/hypr/layout_dispatch.sh ctrl l"),
    { repeating = true }
)

-- 工作区切换 / 移动窗口 / silent 移动
for i = 1, 10 do
    local key = i % 10 -- 10 → 键 "0"
    hl.bind(mainMod .. " + " .. key,           hl.dsp.focus({ workspace = i }))
    hl.bind(mainMod .. " + SHIFT + " .. key,   hl.dsp.window.move({ workspace = tostring(i) }))
    hl.bind(mainMod .. " + CONTROL + " .. key, hl.dsp.window.move({ workspace = tostring(i), silent = true }))
end

-- 特殊工作区
hl.bind(mainMod .. " + M", hl.dsp.workspace.toggle_special("special"))
hl.bind(mainMod .. " + SHIFT + M", hl.dsp.window.move({ workspace = "special" }))

-- 窗口分组
hl.bind(mainMod .. " + G",           hl.dsp.group.toggle())
hl.bind(mainMod .. " + TAB",         hl.dsp.group.next())
hl.bind(mainMod .. " + SHIFT + TAB", hl.dsp.group.next({ reverse = true }))

-- QuickShell 全局触发
hl.bind(mainMod .. " + slash",              hl.dsp.global("quickshell:keybindings"))
hl.bind(mainMod .. " + SHIFT + A",          hl.dsp.global("quickshell:ai"))
hl.bind(mainMod .. " + N",                  hl.dsp.global("quickshell:notes"))
hl.bind(mainMod .. " + SHIFT + apostrophe", hl.dsp.global("quickshell:journal"))

-- 自定义脚本
hl.bind(mainMod .. " + O", hl.dsp.exec_cmd(SCRIPTS .. "/hypr/opacity_toggle.sh"))
hl.bind(mainMod .. " + SHIFT + S", hl.dsp.exec_cmd(SCRIPTS .. "/hypr/workspace_save.sh"))
hl.bind(mainMod .. " + CONTROL + SHIFT + S", hl.dsp.exec_cmd(SCRIPTS .. "/hypr/workspace_restore.sh"))

-- 鼠标拖动 / 调整大小（bindm 等价：mouse = true）
hl.bind(mainMod .. " + mouse:272", hl.dsp.window.drag(), { mouse = true })
hl.bind(mainMod .. " + mouse:273", hl.dsp.window.resize(), { mouse = true })

-- 多媒体按键（bindel 等价：locked + repeating）
-- 0.55 起 `hyprctl dispatch global xxx` 旧语法失效，改用 lua function：
-- 先 exec_cmd 跑 shell 命令，再 dispatch global dispatcher。
hl.bind("XF86AudioRaiseVolume", function()
    hl.exec_cmd("wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%+")
    hl.dispatch(hl.dsp.global("quickshell:osdVolume"))
end, { locked = true, repeating = true })

hl.bind("XF86AudioLowerVolume", function()
    hl.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-")
    hl.dispatch(hl.dsp.global("quickshell:osdVolume"))
end, { locked = true, repeating = true })

hl.bind("XF86AudioMute", function()
    hl.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle")
    hl.dispatch(hl.dsp.global("quickshell:osdVolume"))
end, { locked = true, repeating = true })

hl.bind("XF86AudioMicMute",
    hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"),
    { locked = true, repeating = true })

hl.bind("XF86MonBrightnessUp", function()
    hl.exec_cmd("brightnessctl s 10%+")
    hl.dispatch(hl.dsp.global("quickshell:osdBrightness"))
end, { locked = true, repeating = true })

hl.bind("XF86MonBrightnessDown", function()
    hl.exec_cmd("brightnessctl s 10%-")
    hl.dispatch(hl.dsp.global("quickshell:osdBrightness"))
end, { locked = true, repeating = true })

-- 播放控制（bindl 等价：locked，锁屏时仍可用）
hl.bind("XF86AudioNext", hl.dsp.exec_cmd("playerctl next"), { locked = true })
hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPlay", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPrev", hl.dsp.exec_cmd("playerctl previous"), { locked = true })

-- ============================================================
-- Autostart（exec-once 等价）
-- ============================================================

hl.on("hyprland.start", function()
    hl.exec_cmd("swaybg -i " .. HOME .. "/Pictures/wallpapers/disco_elysium_wallpaper.png")
    hl.exec_cmd("fcitx5 -d")
    hl.exec_cmd("hypridle")
    hl.exec_cmd("wl-paste --watch cliphist store")
    -- macOS 式 Super+C/V/X/A/Z（按窗口分流，终端走 Ctrl+Shift+C）
    -- --device 显式指定键盘（KM 一体接收器不被 xremap 自动识别为键盘）；换键盘需改设备名
    hl.exec_cmd("xremap --device 'Wireless Link-KM' " .. HOME .. "/.config/xremap/config.yml")
    hl.exec_cmd("quickshell")
    hl.exec_cmd(N .. "/hypr/screen_effects.sh apply")
    hl.exec_cmd("clash-verge")
    hl.exec_cmd("google-chrome-stable")
end)
