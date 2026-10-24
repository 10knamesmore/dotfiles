-- 具体规则见
-- - cli/dots/crates/dots
-- - cli/dots/crates/dots-core

-- Pi agent 目录保持真实、逐子项链。~/.pi/agent 下混着 auth.json（凭据）、
-- settings.json、sessions/、models-store.json、npm/ 等机器本地物；整目录链会把
-- 它们卷进仓库。settings.json 也留给 Pi 按机器自行维护。
granularity("home/.pi/agent", { mode = "children" })

-- Mineral 的 default.lua 和 lua/meta 由程序生成；只链接仓库中的用户配置文件。
granularity("home/.config/mineral", { mode = "children" })

-- Zed 只管理配置文件；conversations、prompts、themes 等本地内容不纳入仓库。
granularity("home/.config/zed", { mode = "children" })

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
-- 安装 Python SDK 与本地原生扩展。
dots.hook.before_sync({
  name = "prepare Pi Python environment",
  cwd = dots.repo .. "/pi/src/extensions/python",
  program = "uv",
  args = { "sync", "--locked" },
})
-- Pi 启动只读清单；包版本在下次 sync 后更新。
dots.hook.before_sync({
  name = "write Pi Python environment manifest",
  cwd = dots.repo .. "/pi/src/extensions/python",
  program = "uv",
  args = { "run", "--no-sync", "--", "python", "environment_manifest.py" },
})
dots.resource.directory({
  target = "~/.local/state/kitty",
  mode = "0700",
})
dots.resource.symlink({
  source = "pi/src/extensions/python/packages/terminal-use/src/terminal_use/bridge.py",
  target = "~/.local/share/terminal-use/bridge.py",
})

-- Pi 逐个加载 extension；jiti 沿 source realpath 从 pi/node_modules 解析依赖。
-- agent 目录也可能尚不存在，因此首次 sync 就要创建这些链接。
distribute("pi-extensions", {
  src = "pi/src/extensions",
  to = { "~/.pi/agent/extensions" },
  mode = "children",
  required = true,
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

-- Agent 专用 PATH 垫片（如 rm 递归转回收站）；Pi 原生将 ~/.pi/agent/bin 置于 PATH 首位。
distribute("agent-bin", {
  src = "tree/home/.agents/bin",
  to = { "~/.pi/agent/bin" },
  mode = "children",
  required = true,
})

-- 系统包按平台执行；不适用的包管理器清单直接跳过。
dots.install.pacman({
  "kitty",
  "shader-slang",
  "starship",
  "zoxide",
  "ripgrep",
  "fd",
  "bat",
  "eza",
  "git-delta",
  "uv",
  "trash-cli",
})

local macos_cargo_packages = {
  "starship",
  "zoxide",
  "ripgrep",
  "fd-find",
  "bat",
  "eza",
  "git-delta",
  "uv",
}
for _, package in ipairs(macos_cargo_packages) do
  dots.install.cargo({
    source = package,
    platforms = { "macos" },
  })
end

dots.install.cargo({
  source = {
    path = "cli/programs/agent-hooks",
    binary = "agent-hook",
  },
  root = "~/.local",
})

dots.install.cargo({
  source = {
    path = "cli/programs/window-borders",
    binary = "window-borders",
  },
  root = "~/.local",
  platforms = { "macos" },
})

dots.install.cargo({
  source = {
    path = "cli/programs/desktop-backend",
    binary = "desktop-backend",
  },
  root = "~/.local",
  platforms = { "linux" },
})

local cargo_packages = {
  "matugen",
  "du-dust",
  "cargo-nextest",
  "cargo-insta",
  "cargo-watch",
  "samply",
  "ast-grep",
  "prek",
  "cargo-update",
  "cargo-cache",
}
for _, package in ipairs(cargo_packages) do
  dots.install.cargo({ source = package })
end
