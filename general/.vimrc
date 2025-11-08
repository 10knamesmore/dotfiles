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
