---@meta
--- dots.lua 的类型标注（LuaLS）。
--- 让 nvim/LuaLS 对 dots.lua 提供字段补全、签名提示、类型检查。
--- 此文件不被执行，仅供编辑期类型推导（由 .luarc.json 的 workspace.library 引入）。

---@class GranularitySpec
---@field mode? "dir"|"children"|"file"   # 链接粒度，缺省 "dir"
---@field ignore? string[]                # 下钻/逐文件时跳过的子项名
---@field pre? fun(): boolean?            # 条目链接前执行；返回 false 则整条目跳过
---@field post? fun()                     # 条目链接后执行（被 pre 阻止则不执行）

--- 覆盖某 tree 内路径的链接粒度。
---@param path string                      # 相对 tree 的路径，如 "home/.config/opencode"
---@param spec GranularitySpec
function granularity(path, spec) end

---@class DistributeSpec
---@field src string                       # 唯一真相源（仓库内），如 "tree/home/.agent/skills"
---@field to string[]                      # 落点列表（$HOME 侧，可用 ~）
---@field mode? "dir"|"children"|"file"    # 落点粒度，缺省 "dir"
---@field pre? fun(): boolean?             # 分发前执行；返回 false 则整个分发跳过
---@field post? fun()                      # 分发完成后执行（被 pre 阻止则不执行）

--- 一源多落点分发（接入新工具 = to 加一行 + dots sync）。
---@param name string                       # 分发组标识，仅用于人类可读输出（pre 跳过时的提示），不参与链接判定
---@param spec DistributeSpec
function distribute(name, spec) end

---@class RootSpec
---@field path string                      # 目标根（$HOME 外的绝对/~ 路径）
---@field os? "linux"|"macos"              # 仅该平台生效；nil 为全平台

--- 声明非 $HOME 镜像的额外层（罕见，如 macOS App Support）。
---@param name string                      # 对应 tree/<name>
---@param spec RootSpec
function root(name, spec) end

--- 声明 systemd user 单元（sync 时 systemctl --user enable，幂等）。
---@param units string[]
function systemd_user(units) end

---@class ScriptsSpec
---@field ignore_tree? string[]            # 聚合时递归拍平其文件的子目录名（子目录默认整目录链保树形）

--- scripts 聚合选项。
---@param spec ScriptsSpec
function scripts(spec) end

---@alias HookPhase "pre_sync"|"on_host_activate"|"post_link"|"post_sync"

--- 注册生命周期钩子（effect 阶段执行）。
--- 表形式：phase 做 key，value 是单个函数或函数数组（同 phase 多钩子按数组序执行）。
---@param hooks table<HookPhase, fun()|fun()[]>
function on(hooks) end

--- per-host 块表：hostname → 配置闭包。未命中当前机且非空 → sync 硬报错。
---@param blocks table<string, fun()>
function hosts(blocks) end

--- 设置 per-host 注入变量（仅在 hosts 块/钩子内调用）。
---@param tbl table<string, string>
function vars(tbl) end

--- 声明 per-host 专属链接（仅在 hosts 块内调用）。
---@param src string                       # 仓库内相对路径
---@param target string                    # $HOME 侧目标（可用 ~）
function link(src, target) end

---@class DotsJson
---@field merge fun(path: string, tbl: table)              # 读-改-写 JSON：合并 tbl、保留其余键
---@field set fun(path: string, keypath: string, value: any) # 设某 keypath（如 "hooks.Stop"）
---@field decode fun(text: string): table|nil, string?     # JSON 文本 → Lua 表（null→nil）；坏 JSON 返回 nil + 错误信息

---@class DotsFile
---@field ensure_block fun(path: string, marker: string, content: string) # 文本 managed-block 幂等替换
---@field install fun(src: string, dest: string)      # 原子安装文件：无差异跳写；temp+rename（免 ETXTBSY）、保留权限位

---@class DotsCargo
---@field build fun(dir: string, bin: string): string|nil, string? # release 编译 dir 下的 bin，返回产物绝对路径；失败/dry-run 返回 nil + 原因

---@class DotsRunResult
---@field code integer                     # 退出码（被信号杀死等无退出码 → -1）
---@field stdout string                    # 捕获的标准输出
---@field stderr string                    # 捕获的标准错误
---@field ok boolean                       # code == 0 的便捷判断

---@class Dots
---@field host string                      # 当前主机名（只读）
---@field os "linux"|"macos"               # 当前平台（只读）
---@field home string                      # $HOME（只读）
---@field repo string                      # 仓库根（只读）
---@field json DotsJson                    # JSON 写原语（effect 阶段）
---@field file DotsFile                    # 文本写原语（effect 阶段）
---@field cargo DotsCargo                  # cargo 集成（effect 阶段）
---@field run_once fun(key: string, cmd: string): boolean # 幂等执行一次性命令
---@field run fun(cmd: string): DotsRunResult # 每次 sync 都执行（dry-run 跳过、ok=true）；非零退出留一行告警不致命

---@type Dots
dots = {}
