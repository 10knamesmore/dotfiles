#!/usr/bin/env zsh
###############################################################################
# dotfiles 安装脚本
#
# 功能：
#   1. 检测操作系统类型（macos 或 linux）。
#   2. 备份已有配置文件到 dotfiles/backup/时间戳/ 目录。
#   3. 将 dotfiles 目录下的文件以符号链接方式安装到用户主目录，保持目录结构一致。
#   4. 支持通用配置和系统特定配置（general、macos、linux）。
#
# 用法：
#   bash install.sh
###############################################################################

if [[ -z "$ZSH_VERSION" ]]; then
    echo "请使用 zsh 来运行此脚本."
    exit 1
fi

setopt localoptions nullglob dotglob

set -e

# install.sh 的绝对路径
SCRIPT_PATH="${0:A}"

# dotfiles 项目路径
DOTFILES_DIR="$(dirname "$SCRIPT_PATH")"

# 备份路径
BACKUP_DIR="$DOTFILES_DIR/backup"

# 时间戳
TIMESTAMP="$(date +%Y-%m-%dT%H:%M:%S)"

# generated 目录路径
GENERATE_DIR="$DOTFILES_DIR/generated"

# scripts 路径 
SCRIPTS_DIR="$GENERATE_DIR/scripts"

mkdir -p "$GENERATE_DIR"

declare -A templates
templates[ZSH_CUSTOM_TEMPLATE]="$DOTFILES_DIR/static/omz_custom"
templates[DOT_TEMPLATE]="cd $DOTFILES_DIR"
templates[SCRIPTS_DIR_TEMPLATE]="$SCRIPTS_DIR"

info() {
    echo -e "\033[0;32m[info]\033[0m: $1"
}

warn() {
    echo -e "\033[1;33m[warn]\033[0m: $1"
}

error() {
    echo -e "\033[0;31m[error]\033[0m: $1"
}

error_exit() {
    error "$1"
    exit 1
}

detect_os() {
    local uname
    uname="$(uname)"
    if [[ "$uname" == "Darwin" ]]; then
        echo "macos"
    elif [[ "$uname" == "Linux" ]]; then
        echo "linux"
    else
        echo "other"
    fi
}

#######################################
# 备份目标(文件或目录)或移除符号链接
#
# 如果目标不存在，则不进行任何操作。
#
# 如果是其他不支持的类型，则报错退出。
#
# 总之只要返回了, "$0"就一定是空的, 可以安全创建符号链接。
#
# arguments:
# "$1": 目标绝对路径
#######################################
backup_file() {
    local target="$1"

    # 如果目标不存在且不是符号链接，则直接返回
    if [[ ! -e "$target" && ! -L "$target" ]]; then
        return
    fi

    # 如果是符号链接（无论是否有效），直接删除
    if [[ -L "$target" ]]; then
        rm "$target"
        return
    fi

    # 如果是普通文件或目录，进行备份
    if [[ -f "$target" || -d "$target" ]]; then
        local backup_path
        # 判断父目录是否是 .config
        local parent_dir
        parent_dir=$(basename "$(dirname "$target")")

        if [[ "$parent_dir" == ".config" ]]; then
            backup_path="$BACKUP_DIR/$TIMESTAMP/.config/$(basename "$target")"
        else
            backup_path="$BACKUP_DIR/$TIMESTAMP/$(basename "$target")"
        fi

        mkdir -p "$(dirname "$backup_path")"

        if mv "$target" "$backup_path"; then
            info "备份成功: $target -> $backup_path"
        else
            error_exit "备份失败: $target"
        fi
        return
    fi

    # 不支持的类型
    error_exit "不支持的类型! 请检查: $target"
}

#######################################
# 备份,创建符号链接
# arguments:
#   "$1": source 绝对路径
#   "$2": target 绝对路径
#######################################
create_symlink() {
    local source="$1"
    local target="$2"

    mkdir -p "$(dirname "$target")"
    backup_file "$target"
    ln -s "$source" "$target" || error_exit "创建符号链接失败: $target -> $source"

    info "  linked: $target -> $source"
}

# 遍历 source_dir 目录下的所有文件和文件夹
# 并将文件以符号链接方式安装到用户主目录下，目录结构保持一致。
process_directory() {
    local source_dir="$1"
    local target_base="$HOME"

    if [[ ! -d "$source_dir" ]]; then
        warn "directory not found: $source_dir"
        return
    fi

    # 设置 glob 允许空匹配
    setopt localoptions nullglob dotglob

    for source_path in "$source_dir"/* "$source_dir"/.[!.]* "$source_dir"/..?*; do
        [[ ! -e "$source_path" ]] && continue

        local source_entry="$(basename "$source_path")"
        # 忽略 DS_Store
        [[ "$source_entry" == ".DS_Store" ]] && continue

        local target_path="$target_base/$source_entry"

        if [[ "$source_entry" == ".config" && -d "$source_path" ]]; then
            for sub_source_path in "$source_path"/* "$source_path"/.[!.]* "$source_path"/..?*; do
                [[ ! -e "$sub_source_path" ]] && continue

                local sub_entry="$(basename "$sub_source_path")"
                # 忽略 DS_Store
                [[ "$sub_entry" == ".DS_Store" ]] && continue

                local sub_target_path="$target_path/$sub_entry"

                create_symlink "$sub_source_path" "$sub_target_path"
            done
        elif [[ "$source_entry" = "scripts" && -d "$source_dir" ]]; then
            # 复制脚本目录的一切, 包括目录到generated/scripts
            for script_path in "$source_path"/*; do
                # [[ ! -f "$script_path" ]]  && continue

                local script_entry="$(basename "$script_path")"
                # 忽略 DS_Store
                [[ "$script_entry" == ".DS_Store" ]] && continue

                local script_target_path="$SCRIPTS_DIR/$script_entry"

                create_symlink "$script_path" "$script_target_path"
            done
        else
            create_symlink "$source_path" "$target_path"
        fi
    done
}

declare -A rendered_templates

#######################################
# 替换模版文件中的变量
# globals:
#   template_path 模板替换全局变量
# arguments:
#   "$1" .template 的绝对路径 如 ~/dotfiles/general/.zshrc.template, basename应当唯一
#   "$2" 在哪个路径(k)建软链接到模版渲染后的结果 如 ~/.zshrc
#######################################
render_template() {
    local template_path="$1"
    local output_path="$2"

    local generated_path
    generated_path="$GENERATE_DIR/$(basename "$template_path" .template)"

    # ~/dotfiles/general/.zshrc.template -> ~/dotfiles/generated/.zshrc
    cp "$template_path" "$generated_path"

    for key in ${(k)templates}; do
        local value="${templates[$key]}"
        if grep -q "$key" "$generated_path"; then
            if [[ "$OS" == "macos" ]]; then
                sed -i "" "s|$key|$value|g" "$generated_path"
            else
                sed -i "s|$key|$value|g" "$generated_path"
            fi
            rendered_templates["$key"]="$value"
        fi
    done

    info "  渲染模板: $template_path -> $generated_path"

    rm "$output_path"
    ln -s "$generated_path" "$output_path"
}

#######################################
# 安装后的处理 (模版替换等)
#######################################
post_process() {
    info "进行安装时处理"
    # 此时$home/.zshrc.template 已经是链接 到 template 文件
    local zshrc_template="$DOTFILES_DIR/general/.zshrc.template"
    rm "$HOME/.zshrc.template"
    render_template "$zshrc_template" "$HOME/.zshrc"

    for key in ${(k)rendered_templates}; do
        info "  渲染模板变量: $key -> ${rendered_templates[$key]}"
    done
}

main() {
    info "开始安装 dotfiles..."
    export OS="$(detect_os)"
    info "当前系统 os: $OS"
    [[ "$OS" == "other" ]] && error_exit "unsupported operating system"

    mkdir -p "$BACKUP_DIR"

    info "安装 general dotfiles..."
    process_directory "$DOTFILES_DIR/general"

    if [[ "$OS" == "macos" ]]; then
        info "安装 macos-specific dotfiles..."
        process_directory "$DOTFILES_DIR/macos"
    elif [[ "$OS" == "linux" ]]; then
        info "安装 linux-specific dotfiles..."
        process_directory "$DOTFILES_DIR/linux"
    fi

    post_process

    info "安装完成!"
    info "备份在: $BACKUP_DIR"
}

main "$@"
