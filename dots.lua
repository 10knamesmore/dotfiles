-- dots 例外清单（人手编辑，LuaLS 类型补全见 .luarc.json）。
-- 镜像规则覆盖不到的才写这里；预期长期 < 60 行。

-- opencode 在配置目录生成运行时垃圾，逐文件链 + 忽略它们。
granularity("home/.config/opencode", {
    mode = "file",
    ignore = { "node_modules", "package.json", "bun.lock", ".gitignore" },
})

-- systemd user：systemd 容器下钻，user/ 逐文件链（保持 user/ 是真实目录，
-- 这样 systemctl --user enable 写的 *.wants/ 软链落在真实目录、不污染仓库）。
granularity("home.linux/.config/systemd", { mode = "children" })
granularity("home.linux/.config/systemd/user", {
    mode = "file",
    ignore = { "default.target.wants", "timers.target.wants" },
})

distribute("skills", {
    src = "tree/home/.agent/skills",
    to = { "~/.claude/skills", "~/.codex/skills" },
    mode = "children",
})
distribute("agents", {
    src = "tree/home/.agent/claude/agents",
    to = { "~/.claude/agents" },
    mode = "children",
})

-- systemd user 单元：sync 时 systemctl --user enable（幂等）。
-- bsu-login.service 无 WantedBy（由 bsu-login.timer 触发），不在此 enable。
systemd_user({ "mihomo.service", "bsu-login.timer", "napcat.service" })

-- per-host：本机变量（供 .inject 引用）。monitors.conf 链接见 B7 抽取后补。
hosts({
    ["wanger-arch-16p"] = function()
        vars({ backlight = "amdgpu_bl1", ddc_index = "1" })
    end,
})
