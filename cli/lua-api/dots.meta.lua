---@meta
--- dots.lua 的 LuaLS 声明。所有 sync 持续结果必须通过 Resource 或 mapping declaration 表达。

---@class GranularitySpec
---@field mode? "dir"|"children"|"file" 链接粒度，缺省为 "dir"
---@field ignore? string[] 下钻或逐文件时跳过的子项名

--- 覆盖 tree 内某路径的链接粒度。
---@param path string 相对 tree 的路径，如 "home/.config/opencode"
---@param spec GranularitySpec 粒度设置
function granularity(path, spec) end

---@class DistributeSpec
---@field src string 仓库内唯一真相源
---@field to string[] 目标列表，可使用 `~`
---@field mode? "dir"|"children"|"file" 目标粒度，缺省为 "dir"

--- 把一份仓库 source 映射到多个工具目录。
---@param name string 用于诊断的分发组名
---@param spec DistributeSpec 分发设置
function distribute(name, spec) end

---@class RootSpec
---@field path string 额外 tree layer 的绝对目标根或 `~` 路径
---@field os? "linux"|"macos" 仅在指定平台启用

--- 声明非 `$HOME` 的额外 tree layer target root。
---@param name string 对应 `tree/<name>` 的 layer 名
---@param spec RootSpec target root 设置
function root(name, spec) end

---@class ScriptsSpec
---@field ignore_tree? string[] 递归拍平的 scripts 子目录；其他子目录保持树形

--- 配置 scripts 聚合行为。
---@param spec ScriptsSpec 聚合设置
function scripts(spec) end

---@class ResourceSpecBase
---@field enabled? boolean false 表示本次不进入 Desired Set；缺省为 true

---@class SymlinkResourceSpec: ResourceSpecBase
---@field source string 相对仓库根、绝对或 `~` source path
---@field target string 绝对或 `~` target path

---@class CopiedFileResourceSpec: ResourceSpecBase
---@field source string 相对仓库根、绝对或 `~` source file
---@field target string 绝对或 `~` target file

---@class CargoWorkspaceBinarySource
---@field manifest string 相对仓库根、绝对或 `~` Cargo.toml path
---@field binary string Cargo binary target 名称

---@class CargoWorkspaceBinaryResourceSpec: ResourceSpecBase
---@field source CargoWorkspaceBinarySource 仓库内 binary source
---@field target string 绝对或 `~` 安装位置

---@class CargoCratesIoBinaryResourceSpec: ResourceSpecBase
---@field source string crates.io package 名称；全部 bin 安装到 `~/.cargo/bin`

---@alias CargoBinaryResourceSpec CargoWorkspaceBinaryResourceSpec|CargoCratesIoBinaryResourceSpec

---@class ManagedBlockResourceSpec: ResourceSpecBase
---@field target string 绝对或 `~` 文本文件
---@field marker string 文件内稳定且唯一的 block 名称
---@field content string 两条 marker 之间的期望内容

---@class SystemdUserUnitResourceSpec: ResourceSpecBase
---@field unit string systemd user unit 名称

---@class DotsResource
---@field symlink fun(spec: SymlinkResourceSpec) 声明符号链接
---@field copied_file fun(spec: CopiedFileResourceSpec) 声明内容复制文件
---@field cargo_binary fun(spec: CargoBinaryResourceSpec) 声明编译并安装的 Cargo binary
---@field managed_block fun(spec: ManagedBlockResourceSpec) 声明文本 marker block
---@field systemd_user_unit fun(spec: SystemdUserUnitResourceSpec) 声明 enabled 的 systemd user unit

---@class DotsPath
---@field exists fun(path: string): boolean 声明阶段只读判断路径是否存在

---@class Dots
---@field os "linux"|"macos" 当前平台
---@field home string 当前 `$HOME`
---@field repo string 当前 dotfiles 仓库根
---@field resource DotsResource 显式 Resource declaration
---@field path DotsPath 声明阶段只读 path query

---@type Dots
dots = {}
