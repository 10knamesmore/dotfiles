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

-- Pi agent 目录保持真实、逐子项链。~/.pi/agent 下混着 auth.json（凭据）、
-- settings.json、sessions/、models-store.json、npm/ 等机器本地物；整目录链会把
-- 它们卷进仓库。settings.json 也留给 Pi 按机器自行维护。
granularity("home/.pi/agent", { mode = "children" })

distribute("skills", {
    src = "tree/home/.agents/skills",
    to = { "~/.codex/skills", "~/.kimi/skills" },
    mode = "children",
})
dots.hook.before_sync {
    name = "install Pi dependencies",
    cwd = dots.repo .. "/pi",
    program = "pnpm",
    args = { "install", "--frozen-lockfile" },
}

-- Pi 通过 jiti 直接加载 TypeScript；pnpm dependency 沿 source realpath 从 pi/node_modules 解析。
dots.resource.symlink {
    source = dots.repo .. "/pi/src/distribution",
    target = dots.home .. "/.pi/agent/extensions/pi-distribution",
}
dots.resource.symlink {
    source = dots.repo .. "/pi/src/subagent-workflow",
    target = dots.home .. "/.pi/agent/extensions/subagent-workflow",
}
dots.resource.symlink {
    source = dots.repo .. "/pi/src/remember-last-model.ts",
    target = dots.home .. "/.pi/agent/extensions/remember-last-model.ts",
}
local local_provider = dots.repo .. "/pi/src/local_provider.ts"
dots.resource.symlink {
    source = local_provider,
    target = dots.home .. "/.pi/agent/extensions/local_provider.ts",
    enabled = dots.path.exists(local_provider),
}
dots.resource.symlink {
    source = dots.repo .. "/pi/src/subagent-workflow/skills/workflow-authoring",
    target = dots.home .. "/.pi/agent/skills/workflow-authoring",
}

-- 全局指令源统一放在 .agents；各工具从自己的全局目录加载同一文件。
distribute("agents-md", {
    src = "tree/home/.agents/AGENTS.md",
    to = { "~/.pi/agent/AGENTS.md", "~/.codex/AGENTS.md" },
    mode = "file",
})

-- Hook 定义与规则源都住中立的 .agents/ 命名空间，再分发到各 harness。
distribute("codex-hooks", {
    src = "tree/home/.agents/codex/hooks.json",
    to = { "~/.codex/hooks.json" },
    mode = "file",
})
distribute("agent-hook-rules", {
    src = "tree/home/.agents/hooks/pretool.toml",
    to = {
        "~/.codex/pretool.toml",
        "~/.kimi-code/pretool.toml",
        "~/.pi/agent/pretool.toml",
    },
    mode = "file",
})

-- `dots install` 把声明直接映射为 cargo install --path/--bin/--root。
dots.resource.cargo_binary {
    source = {
        path = "cli/crates/agent-hooks",
        binary = "agent-hook",
    },
    root = "~/.local",
}

-- 所有主机共享的 crates.io package inventory；`dots install` 逐项交给 Cargo 安装或升级。
dots.resource.cargo_binary {
    source = "uv",
    binaries = { "uv", "uvx" },
}

local cargo_binary_packages = {
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
