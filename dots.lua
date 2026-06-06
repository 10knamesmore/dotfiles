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

-- ~/.claude 本体保持真实目录，逐项链；排除 CC 运行时产物与 *.local.json。
granularity("home/.claude", {
  mode = "children",
  ignore = { "projects", "todos", "shell-snapshots", "statsig", "history.jsonl", "settings.local.json" },
})

-- skills 一源多落点：主落点 ~/.claude/skills 走镜像；额外分发到 codex/copilot（逐 skill 链）。
distribute("skills", {
  src = "tree/home/.claude/skills",
  to = { "~/.codex/skills", "~/.copilot/skills" },
  mode = "children",
})

-- systemd user 单元：sync 时 systemctl --user enable（幂等）。
-- bsu-login.service 无 WantedBy（由 bsu-login.timer 触发），不在此 enable。
systemd_user { "mihomo.service", "bsu-login.timer", "napcat.service" }

-- hypr/ 子目录保持树形（键位用 $scripts_dir/hypr/xxx.sh 引用）。
scripts { keep_tree = { "hypr" } }

-- per-host：本机变量（供 .inject 引用）。monitors.conf 链接见 B7 抽取后补。
hosts {
  ["wanger-arch-16p"] = function()
    vars { backlight = "amdgpu_bl1", ddc_index = "1" }
  end,
}
