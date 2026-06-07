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

-- Claude hooks：目录保持真实、逐子项链（同 systemd user/ 的理由）——
-- post_sync 会把机器本地编译的 cc-hook bin 软链进来，不污染仓库。
granularity("home/.claude/hooks", { mode = "children" })

distribute("skills", {
    src = "tree/home/.agents/skills",
    to = { "~/.claude/skills", "~/.codex/skills" },
    mode = "children",
})
distribute("agents", {
    src = "tree/home/.agents/claude/agents",
    to = { "~/.claude/agents" },
    mode = "children",
})
distribute("commands", {
    src = "tree/home/.agents/claude/commands",
    to = { "~/.claude/commands" },
    mode = "children",
})

-- systemd user 单元：sync 时 systemctl --user enable（幂等）。
-- bsu-login.service 无 WantedBy（由 bsu-login.timer 触发），不在此 enable。
systemd_user({ "mihomo.service", "bsu-login.timer", "napcat.service" })

-- 每次 sync 保持 cc-hook（Claude Code hooks 入口）新鲜并复制到 ~/.claude/hooks/。
on({
    post_sync = function()
        local bin = dots.cargo.build(dots.repo .. "/cli", "cc-hook")
        if bin then
            dots.file.install(bin, dots.home .. "/.claude/hooks/cc-hook")
        end
    end,
})

-- per-host：本机变量（供 .inject 引用）。monitors.conf 链接见 B7 抽取后补。
hosts({
    ["wanger-arch-16p"] = function()
        vars({ backlight = "amdgpu_bl1", ddc_index = "1" })
    end,
    -- 腾讯云服务器：只装 shell 基线，dev/ai/js 工具链跳过（组见 packages/toolchains.toml）
    ["VM-0-6-ubuntu"] = function()
        toolchains({ only = { "core" } })
    end,
})
