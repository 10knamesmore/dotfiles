syntax on
" 使用系统剪贴板
set clipboard=unnamedplus
set mouse=a
set cursorline

" 缩进设置
set autoindent
set shiftwidth=4
set tabstop=4
set softtabstop=4
set expandtab
set wrap

" 搜索
set hlsearch
set ignorecase
set smartcase

" 外观
set termguicolors

" 设置领导键为空格（注意这需要在映射之前设置）
let mapleader=" "
set updatetime=200
set scrolloff=5
set virtualedit=

" 启用相对行号
set relativenumber

" 键映射：普通模式中空格键不触发任何操作
nnoremap <Space> <Nop>

" 移动到行尾与行首
nnoremap E $
vnoremap E $
nnoremap B ^
vnoremap B ^

" 模式切换：插入模式下 jk/kj 退出
inoremap jk <Esc>
inoremap kj <Esc>

" 普通和可视模式下按 q 退出（这里和使用 q 启动录制寄存器冲突，建议小心）
nnoremap q <Esc>
vnoremap q <Esc>

" 光标移动：JK 代替多行移动
nnoremap J 7j
vnoremap J 7j
nnoremap K 7k
vnoremap K 7k

" 插入空行但不进入插入模式
nnoremap <leader>o o<Esc>
nnoremap <leader>O O<Esc>

" 窗口移动：大写 W 切换窗口焦点
nnoremap W <C-w>w

" ============================================================================
" 基础配置 - 无插件的 Vim 配置
" 基于 Neovim 配置简化而来
" ============================================================================

" ============================================================================
" 基本设置
" ============================================================================
set nocompatible              " 不兼容 vi
filetype plugin indent on     " 启用文件类型检测和缩进
syntax enable                 " 启用语法高亮

" 领导键设置
let mapleader = " "
let g:mapleader = " "

" ============================================================================
" 编辑器外观
" ============================================================================
set number                    " 显示行号
set relativenumber            " 显示相对行号
set cursorline                " 高亮当前行
set showmatch                 " 高亮匹配的括号
set showcmd                   " 显示命令
set wildmenu                  " 命令行补全增强
set laststatus=2              " 始终显示状态栏
set ruler                     " 显示光标位置
set scrolloff=5               " 光标上下保留5行

" 颜色和终端
if has('termguicolors')
    set termguicolors         " 真彩色支持
endif

" 显示特殊字符
set list
set listchars=tab:>-,trail:-

" ============================================================================
" 缩进和格式化
" ============================================================================
set autoindent                " 自动缩进
set smartindent               " 智能缩进
set expandtab                 " 用空格代替制表符
set tabstop=4                 " Tab 宽度为 4
set shiftwidth=4              " 缩进宽度为 4
set softtabstop=4             " 软 Tab 宽度为 4
set wrap                      " 自动换行

" ============================================================================
" 搜索设置
" ============================================================================
set hlsearch                  " 高亮搜索结果
set incsearch                 " 增量搜索
set ignorecase                " 搜索时忽略大小写
set smartcase                 " 如果搜索中有大写字母则区分大小写

" ============================================================================
" 系统集成
" ============================================================================
set mouse=a                   " 启用鼠标支持
set clipboard=unnamed         " 使用系统剪贴板
if has('unnamedplus')
    set clipboard=unnamedplus
endif

" 关闭备份和交换文件
set noswapfile
set nobackup
set nowritebackup


" 其他设置
set updatetime=200            " 更新时间
set timeoutlen=500            " 键映射超时时间
set backspace=indent,eol,start " 退格键行为

" ============================================================================
" 键位映射
" ============================================================================

" 禁用空格键默认行为
nnoremap <Space> <Nop>
vnoremap <Space> <Nop>

" 禁用 q 键默认行为
nnoremap q <Nop>
vnoremap q <Nop>

" 行首行尾快捷键
nnoremap E $
vnoremap E $
nnoremap B ^
vnoremap B ^

" 插入模式快速退出
inoremap jk <Esc>
inoremap kj <Esc>

" 清除搜索高亮
nnoremap <Esc> :nohlsearch<CR>

" 快速移动
nnoremap J 7gj
vnoremap J 7gj
nnoremap K 7gk
vnoremap K 7gk

" 更好的上下移动（处理自动换行）
nnoremap <expr> j v:count == 0 ? 'gj' : 'j'
nnoremap <expr> k v:count == 0 ? 'gk' : 'k'
vnoremap <expr> j v:count == 0 ? 'gj' : 'j'
vnoremap <expr> k v:count == 0 ? 'gk' : 'k'

" 跳转历史
nnoremap H <C-o>
nnoremap L <C-i>

" 窗口操作
nnoremap <C-h> <C-w>h
nnoremap <C-j> <C-w>j
nnoremap <C-k> <C-w>k
nnoremap <C-l> <C-w>l
nnoremap W <C-w>w
nnoremap <leader>wd <C-w>c

" 插入空行（不进入插入模式）
nnoremap <leader>o o<Esc>
nnoremap <leader>O O<Esc>

" 标签页切换
nnoremap <Left> :tabprevious<CR>
nnoremap <Right> :tabnext<CR>

" 禁用 < > 键
nnoremap < <Nop>
vnoremap < <Nop>
nnoremap > <Nop>
vnoremap > <Nop>

" 搜索结果居中并展开折叠
nnoremap <expr> n 'Nn'[v:searchforward].'zzzv'
nnoremap <expr> N 'nN'[v:searchforward].'zzzv'

" ============================================================================
" 状态行
" ============================================================================
set statusline=%f               " 文件名
set statusline+=%m              " 修改标志
set statusline+=%r              " 只读标志
set statusline+=%h              " 帮助文件标志
set statusline+=%w              " 预览窗口标志
set statusline+=%=              " 右对齐
set statusline+=%y              " 文件类型
set statusline+=\ [%{&fileencoding?&fileencoding:&encoding}]  " 编码
set statusline+=\ [%{&fileformat}]  " 文件格式
set statusline+=\ %l/%L         " 行号/总行数
set statusline+=\ %c            " 列号
set statusline+=\ %P            " 百分比

" ============================================================================
" 自动命令
" ============================================================================
" 自动跳转到上次编辑位置
autocmd BufReadPost *
    \ if line("'\"") > 1 && line("'\"") <= line("$") |
    \   exe "normal! g`\"" |
    \ endif

" 保存时自动删除行尾空格
autocmd BufWritePre * :%s/\s\+$//e

" ============================================================================
" Netrw 文件浏览器设置
" ============================================================================
let g:netrw_banner = 0          " 隐藏横幅
let g:netrw_liststyle = 3       " 树形视图
let g:netrw_browse_split = 4    " 在前一个窗口打开文件
let g:netrw_altv = 1
let g:netrw_winsize = 25

" 使用 <leader>e 打开文件浏览器
nnoremap <leader>e :Explore<CR>
