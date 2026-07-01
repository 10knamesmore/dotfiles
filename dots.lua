-- dots 例外清单（人手编辑，LuaLS 类型补全见 .luarc.json）。
-- 镜像规则覆盖不到的才写这里；预期长期 < 60 行。

-- opencode 在配置目录生成运行时垃圾，逐文件链 + 忽略它们。
-- post：自定义 tool 经 symlink 加载时，Bun 按 realpath 从仓库侧向上找 node_modules，
-- 但它是 ignore 的运行时垃圾、只在 $HOME 侧 → 断链，tool 里 import "@opencode-ai/plugin"
-- 会 Cannot find module，连带整个 opencode server 在 resolveTools 阶段崩、任何模型都 500。
-- 补一条仓库侧 → $HOME 侧的反向软链桥过去（软链自身在上面的 ignore + .gitignore 里）。
granularity("home/.config/opencode", {
    mode = "file",
    ignore = { "node_modules", "package.json", "bun.lock", ".gitignore" },
    post = function()
        dots.run("ln -sfn '" .. dots.home .. "/.config/opencode/node_modules' '"
            .. dots.repo .. "/tree/home/.config/opencode/node_modules'")
    end,
})

-- Claude hooks：目录保持真实、逐子项链——让 post_sync 软链进来的机器本地
-- cc-hook bin 落在真实目录、不污染仓库。
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
-- 未登记的新机跑 `dots bootstrap`（交互终端）会自动在此插入一个别名块（onboard.rs）；
-- 别名 key 对应机器本地 ~/.config/dots/host（真名不入 git）。未命中非致命，仅跳过 per-host。
hosts({
    ["wanger-arch-16p"] = function()
        vars({ backlight = "amdgpu_bl1", ddc_index = "1" })
    end,
    -- 腾讯云服务器：只装 shell 基线，dev/ai/js 工具链跳过（组见 packages/toolchains.toml）
    ["VM-0-6-ubuntu"] = function()
        toolchains({ only = { "core" } })
    end,
    ["unknown"] = function()
    end,
})
