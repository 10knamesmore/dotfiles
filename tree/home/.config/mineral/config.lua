-- Mineral 用户配置。只需写要覆盖的字段,其余回落默认(深合并,数组整体替换)。
-- 编辑器补全 / 类型检查依赖同目录 lua/meta 下的 stub(本文件由 `mineral config init` 生成)。
-- 完整可覆盖字段见 lua/meta/config.lua;各字段默认值见同目录 default.lua(仅参考,程序不读)。
--
-- 本文件同时是脚本:顶层 mineral.* 调用写在 **return 之前**(Lua 的 return 必须是
-- 最后一条语句),daemon 加载时真实执行;return 的表是纯配置数据,里面不放调用。
-- 脚本 API 指南见仓库 docs/scripting.md。

dofile((os.getenv("HOME") or "") .. "/.config/mineral/demo_unplayable_rescue.lua")

mineral.on("track_started", function(args)
  mineral.ui.card({
    title = "Now Play",
    ttl_secs = 6,
    body = {
      {
        { (" "):rep(3) },
        { args.song.title, fg = "accent", bold = true, italic = true, align = "center" },
        { (" "):rep(3) },
      },
      { { args.song.album, align = "center" } },
      { { args.song.artists[1], align = "center" } },
    },
  })
end)

local styles, si = { "bars", "scope", "waterfall", "terrain" }, 1
mineral.action("my.cycle_spectrum", function()
  si = si % #styles + 1
  mineral.config.override({ tui = { spectrum = { style = styles[si] } } })
end)

do
  local cur, state = nil, "stopped"
  mineral.on("track_started", function(a)
    cur = a.song
  end)
  mineral.observe("player.state", function(s)
    state = s or "stopped"
  end)

  local PULSE = { "░", "░", "▒", "▓", "█", "█", "▓", "▒" }
  local N, tick = #PULSE, 0

  local function pulse()
    return PULSE[tick % N + 1]
  end

  local function title()
    if not cur then
      return nil
    end -- 无歌 → nil 撤销覆盖,回落结构化模板
    local name = cur.title .. (cur.artists[1] and (" — " .. cur.artists[1]) or "")
    if state == "playing" then
      return pulse() .. " " .. name
    end
    if state == "paused" then
      return "▶ " .. name
    end
    return nil
  end

  mineral.timer.every(250, function() -- 4fps:8 帧 ≈ 2s 一次呼吸,写入 ≤4/s
    tick = tick + 1
    mineral.ui.window_title(title())
  end)
end

-- mineral.observe("terminal", function(t)
--   if t == nil then
--     return
--   end
--
--   -- 全屏播放态:固定示波器
--   -- if t.fullscreen then
--   --   mineral.config.override("tui.spectrum.style", "scope")
--   --   return
--   -- end
--
--   -- browse 态:越高越复杂(terrain 的层叠纵深吃高度,矮面板层挤,退回条形)
--   if t.rows < 48 then
--     mineral.config.override("tui.spectrum.style", "bars")
--   elseif t.rows < 58 then
--     mineral.config.override("tui.spectrum.terrain.layers", 6)
--     mineral.config.override("tui.spectrum.terrain.amplitude", 0.25)
--     mineral.config.override("tui.spectrum.style", "terrain")
--   else
--     mineral.config.override("tui.spectrum.terrain.layers", 8)
--     mineral.config.override("tui.spectrum.terrain.amplitude", 0.3)
--     mineral.config.override("tui.spectrum.style", "terrain")
--   end
-- end)

---@type mineral.Config
return {
  -- 示例:把初始音量调到 80
  -- audio = { volume = 80 },
  -- stems = {
  --   enabled = true,
  --   engine = {
  --     runtime_path = "/Users/wanger/.cache/uv/archive-v0/ZcBg1mJLvbUuhgaF/lib/python3.12/site-packages/onnxruntime/capi/libonnxruntime.1.27.0.dylib",
  --   },
  -- },

  -- 示例:换主强调色 + 重映射暂停键
  tui = {
    animation = {
      ambient_trail = {
        enter = { delay_ms = 80, ease_ms = 740 },
        exit = {},
      },

      transport = { -- 播放栏操作反馈:停留与过渡时长(毫秒)
        volume_hold_ms = 2000, -- 调节音量
        volume_fade_out_ms = 110, -- 音量标题fade out
        volume_fade_in_ms = 150, -- 音量标题fade in
        mode_hold_ms = 1200, -- 切换模
        controls_hold_ms = 4000, -- 播放控制
        mode_reveal_ms = 220, -- 模式文字
        mode_resize_ms = 180, -- 模式区域
        controls_fade_ms = 220, -- 控制键fade in/out
      },
    },
    -- keys = {
    --   script = { ["my.cycle_spectrum"] = "S" },
    -- },
    -- 主题:每次加载从 pool 随机(见上方 chosen_theme);想固定改成 THEMES.<名字>
    theme = {
      base = "#1a1b26",
      mantle = "#16161e",
      crust = "#13131a",
      surface0 = "#292e42",
      surface1 = "#3b4261",
      overlay = "#565f89",
      subtext = "#a9b1d6",
      text = "#c0caf5",
      accent = "#bb9af7",
      accent_2 = "#7aa2f7",
      red = "#f7768e",
      yellow = "#e0af68",
      green = "#9ece6a",
      peach = "#ff9e64",
    },
    spectrum = {
      style = "terrain",
      waterfall = { push_ms = 42 },
    },
    cover = {
      debounce_ms = 32,
      cache = {
        image = 256 * 1024 * 1024, -- 解码原图 RAM 预算,字节;越界逐出最久未显示者
        protocol = 128 * 1024 * 1024, -- 终端图片成品 RAM 预算,字节;越界逐出最久未渲染者
      },
    },
    cover_transition = {
      style = "slide",
    },
    waveform = {
      contrast = 3,
    },
    behavior = {
      filter_play_scope = "matches",
      kill_spawned_daemon_on_exit = false,
    },
  },
  queue = {
    transforms = {
      -- 队列操作菜单(o)里多一项:按专辑排序,同专辑内按主艺人。
      -- album / artist 缺失的排在最前(空串最小);返回值只认 id,daemon 从原队列取回实体。
      {
        key = "s",
        label = "Sort by album",
        transform = function(queue)
          table.sort(queue, function(a, b)
            local aa, ba = a.album or "", b.album or ""
            if aa ~= ba then
              return aa < ba
            end
            return (a.artists[1] or "") < (b.artists[1] or "")
          end)
          return queue
        end,
      },
    },
  },
  sources = {
    bilibili = {
      curate_playlists = function(lists)
        local keep = {}
        for _, p in ipairs(lists) do
          if p.track_count > 0 and p.name:match("^音乐") then
            keep[#keep + 1] = p
          end
        end
        return keep
      end,
    },
  },
}
