-- dots 例外清单（人手编辑，LuaLS 类型补全见 .luarc.json）。
-- 镜像规则覆盖不到的才写这里；预期长期 < 60 行。

-- opencode 在配置目录生成运行时垃圾，逐文件链 + 忽略它们。
-- 自定义 tool 经 symlink 加载时，Bun 按 realpath 从仓库侧向上找 node_modules，
-- 但它是 ignore 的运行时垃圾、只在 $HOME 侧 → 断链，tool 里 import "@opencode-ai/plugin"
-- 会 Cannot find module，连带整个 opencode server 在 resolveTools 阶段崩、任何模型都 500。
-- 补一条仓库侧 → $HOME 侧的反向软链桥过去（软链自身在上面的 ignore + .gitignore 里）。
granularity("home/.config/opencode", {
    mode = "file",
    ignore = { "node_modules", "package.json", "bun.lock", ".gitignore" },
})
dots.resource.symlink {
    source = dots.home .. "/.config/opencode/node_modules",
    target = dots.repo .. "/tree/home/.config/opencode/node_modules",
}

-- Claude hooks：目录保持真实、逐子项链——让 cargo binary 落在机器本地目录，
-- cc-hook bin 落在真实目录、不污染仓库。
granularity("home/.claude/hooks", { mode = "children" })

-- pi agent：同上，目录保持真实、逐子项链。~/.pi/agent 下混着 auth.json（凭据）、
-- sessions/、models-store.json、npm/ 等机器本地物，整目录链会把它们卷进仓库。
-- 注意 pi 用 writeFileSync 原地写 settings.json（跟随软链、不 temp+rename），
-- 所以链进来的 settings.json 在 /settings 改动后会直接回流仓库——这是要的行为，
-- 代价是 pi 自写的 lastChangelogVersion 会跟着进 diff。
granularity("home/.pi/agent", { mode = "children" })

distribute("skills", {
    src = "tree/home/.agents/skills",
    to = { "~/.claude/skills", "~/.codex/skills", "~/.kimi/skills" },
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

-- pi extension：源住 pi-ext/（仓库内独立 TS 工程，含 package.json/tsconfig/
-- node_modules，全为编辑期 LSP 服务），只把 src/ 下的成品链进去。pi 运行时由
-- 自己的 loader 内建提供 @earendil-works/pi-* 与 typebox，不读 node_modules，
-- 故工程文件整个不必落 $HOME。tree/ 因此保持纯 $HOME 镜像、不掺开发工作区。
-- 注意 pi 只认 extensions 下的 `*.ts` 与 `*/index.ts` 两种形态。
-- extension 若 import 外部依赖（非 pi 内建提供的那几个），jiti **不 resolve
-- symlink**，只从软链所在的 ~/.pi/agent/extensions/ 逐级向上找 node_modules，
-- 够不着仓库侧的 pi-ext/node_modules → Cannot find module。补一条 $HOME 侧 →
-- 仓库侧的桥。注意与上面 opencode 那条方向相反：Bun 按 realpath 找、jiti 按软链
-- 路径找，所以两个工具要往相反方向搭桥。
-- 未跑 pnpm install 时 node_modules 不存在，静默跳过（纯编辑期产物，不该报警）。
distribute("pi-extensions", {
    src = "pi-ext/src",
    to = { "~/.pi/agent/extensions" },
    mode = "children",
})
local pi_node_modules = dots.repo .. "/pi-ext/node_modules"
dots.resource.symlink {
    source = pi_node_modules,
    target = dots.home .. "/.pi/agent/node_modules",
    enabled = dots.path.exists(pi_node_modules),
}

-- 全局 agent 指令的唯一真相源。Claude Code 只认 ~/.claude/CLAUDE.md，
-- 那份改成 `@~/.agents/AGENTS.md` 一行 import（官方推荐的 AGENTS.md 接法）；
-- pi 只认 ~/.pi/agent/ 下的 AGENTS.md，够不着 ~/.agents/，只能链过去。
-- 接新工具 = to 加一行。
distribute("agents-md", {
    src = "tree/home/.agents/AGENTS.md",
    to = { "~/.pi/agent/AGENTS.md", "~/.codex/AGENTS.md" },
    mode = "file",
})

-- Codex hooks 与共享规则各自住真实目录；hook 定义留在中立的 .agents/ 命名空间，
-- `pretool.toml` 是所有 harness 的规则真相源。
distribute("codex-hooks", {
    src = "tree/home/.agents/codex/hooks.json",
    to = { "~/.codex/hooks.json" },
    mode = "file",
})
distribute("agent-hook-rules", {
    src = "tree/home/.claude/hooks/pretool.toml",
    to = { "~/.codex/pretool.toml", "~/.kimi-code/pretool.toml" },
    mode = "file",
})

-- 每次 plan 先编译，再按 artifact 内容收敛稳定安装位置。
dots.resource.cargo_binary {
    source = {
        manifest = "cli/Cargo.toml",
        binary = "cc-hook",
    },
    target = "~/.claude/hooks/cc-hook",
}
dots.resource.cargo_binary {
    source = {
        manifest = "cli/Cargo.toml",
        binary = "agent-hook",
    },
    target = "~/.local/bin/agent-hook",
}

-- 所有主机共享的 crates.io binary inventory。Cargo 实际生成的全部 bin 由 dots
-- 自动收敛到 ~/.cargo/bin，不在这里重复维护 package 到 binary 名称的映射。
local cargo_binary_packages = {
    "uv",
    "starship",
    "zoxide",
    "du-dust",
    "ripgrep",
    "fd-find",
    "bat",
    "eza",
    "git-delta",
    "cargo-nextest",
    "cargo-insta",
    "cargo-watch",
    "samply",
    "ast-grep",
    "prek",
    "cargo-update",
    "cargo-cache",
}
for _, package in ipairs(cargo_binary_packages) do
    dots.resource.cargo_binary { source = package }
end
