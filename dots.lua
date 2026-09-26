-- 具体规则见
-- - cli/crates/dots
-- - cli/crates/dots-core

-- Pi agent 目录保持真实、逐子项链。~/.pi/agent 下混着 auth.json（凭据）、
-- settings.json、sessions/、models-store.json、npm/ 等机器本地物；整目录链会把
-- 它们卷进仓库。settings.json 也留给 Pi 按机器自行维护。
granularity("home/.pi/agent", { mode = "children" })

-- Mineral 的 default.lua 和 lua/meta 由程序生成；只链接仓库中的用户配置文件。
granularity("home/.config/mineral", { mode = "children" })

distribute("skills", {
  src = "tree/home/.agents/skills",
  to = { "~/.codex/skills", "~/.kimi/skills" },
  mode = "children",
})
dots.hook.before_sync({
  name = "update Pi dependencies",
  cwd = dots.repo .. "/pi",
  program = "pnpm",
  args = { "run", "update:pi" },
})

-- Pi 逐个加载 extension；jiti 沿 source realpath 从 pi/node_modules 解析依赖。
-- agent 目录也可能尚不存在，因此首次 sync 就要创建这些链接。
distribute("pi-extensions", {
  src = "pi/src/extensions",
  to = { "~/.pi/agent/extensions" },
  mode = "children",
  required = true,
})
dots.resource.symlink({
  source = dots.repo .. "/pi/src/extensions/subagent-workflow/skills/workflow-authoring",
  target = dots.home .. "/.pi/agent/skills/workflow-authoring",
})

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
dots.resource.cargo_binary({
  source = {
    path = "cli/crates/agent-hooks",
    binary = "agent-hook",
  },
  root = "~/.local",
})

-- 所有主机共享的 crates.io package inventory；`dots install` 逐项交给 Cargo 安装或升级。
dots.resource.cargo_binary({
  source = "uv",
  binaries = { "uv", "uvx" },
})

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
  dots.resource.cargo_binary({ source = package })
end
