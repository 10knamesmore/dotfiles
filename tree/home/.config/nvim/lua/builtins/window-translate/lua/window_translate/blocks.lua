--- 源位置由本地解析确定。整个 buffer 版本使用同一组块，滚动和窗口尺寸不参与切分。
local M = {}
local api = vim.api

---@class WindowTranslate.Block
---@field id integer 当前 buffer 版本内的顺序编号，不发送给模型。
---@field start_line integer 1-based，包含起止行。
---@field end_line integer
---@field source string 去掉注释外壳后的原文，保留 Markdown 标记、换行和内部缩进。
---@field text string 当前译文，流式阶段可不完整。
---@field complete boolean 只有成功完成的请求才可复用为缓存。
---@field error? string 失败块不自动重试；再次手动开启翻译时清除。

---@class WindowTranslate.Region
---@field lines string[]
---@field rows integer[] 文本每一行对应的原 buffer 行号。
---@field join? string 连续独立行注释的缩进与标记；块注释和尾随注释不合并。

--- 首行已由语法节点裁掉源缩进，其余行去掉共同缩进，保留内部列表和代码的相对缩进。
---@param lines string[]
local function dedent(lines)
  local margin
  for index = 2, #lines do
    if lines[index]:find("%S") then
      local indent = #lines[index]:match("^%s*")
      margin = margin and math.min(margin, indent) or indent
    end
  end
  for index = 2, #lines do
    lines[index] = lines[index]:sub((margin or 0) + 1)
  end
  lines[1] = lines[1]:gsub("^%s+", "")
end

---@param node TSNode
---@param buffer integer
---@param comment boolean
---@return WindowTranslate.Region
local function region(node, buffer, comment)
  local row, col = node:start()
  local text = vim.treesitter.get_node_text(node, buffer)
  local marker
  if comment then
    if text:match("^/%*") then
      text = text:gsub("^/%*+!?", ""):gsub("%*/$", "")
      text = text:gsub("\n[ \t]*%* ?", "\n")
    elseif text:match("^<!%-%-") then
      text = text:gsub("^<!%-%-", ""):gsub("%-%->$", "")
    elseif text:match("^%-%-%[=*%[") then
      text = text:gsub("^%-%-%[=*%[", ""):gsub("%]=*%]$", "")
    elseif text:match("^{%-") then
      text = text:gsub("^{%-", ""):gsub("%-}$", "")
    else
      marker = text:match("^([/#;%%%-]+!?)") or text:match('^(")')
      if marker then
        text = text:sub(#marker + 1):gsub("^ ", "")
      end
    end
  end
  local lines, rows = vim.split(text, "\n", { plain = true }), {}
  for index = 1, #lines do
    rows[index] = row + index
  end
  if not comment then
    dedent(lines)
  elseif not marker then
    local margin
    for _, line in ipairs(lines) do
      if line:find("%S") then
        local indent = #line:match("^%s*")
        margin = margin and math.min(margin, indent) or indent
      end
    end
    for index, line in ipairs(lines) do
      lines[index] = line:sub((margin or 0) + 1)
    end
  end
  local before = api.nvim_buf_get_lines(buffer, row, row + 1, false)[1]:sub(1, col)
  local join = marker and #lines == 1 and before:match("^%s*$") and (before .. marker) or nil
  return { lines = lines, rows = rows, join = join }
end

--- Python 的首个语句字符串才是 docstring；赋值字符串、返回值、f-string 都不提取。
---@param node TSNode
---@param buffer integer
---@return TSNode?
local function docstring(node, buffer)
  local statement = node:parent()
  if not statement or statement:type() ~= "expression_statement" then
    return
  end
  local body = statement:parent()
  local owner = body and body:parent()
  if
    not body
    or not (
      body:type() == "module"
      or (
        body:type() == "block"
        and owner
        and vim.tbl_contains({ "function_definition", "class_definition" }, owner:type())
      )
    )
  then
    return
  end
  for child in body:iter_children() do
    if child:named() and child:type() ~= "comment" then
      if child:id() ~= statement:id() then
        return
      end
      break
    end
  end
  if not vim.treesitter.get_node_text(node, buffer):match("^[rRuU]*[\"']") then
    return
  end
  for child in node:iter_children() do
    if child:type() == "string_content" then
      return child
    end
  end
end

---@param buffer integer
---@return WindowTranslate.Region[]
local function comments(buffer)
  local parser = assert(vim.treesitter.get_parser(buffer))
  local regions = {}
  local function visit(node)
    local kind = node:type()
    local content = parser:lang() == "python" and kind == "string" and docstring(node, buffer) or nil
    if kind:find("comment", 1, true) or content then
      local current = region(content or node, buffer, not content)
      local previous = regions[#regions]
      if
        previous
        and current.join
        and previous.join == current.join
        and previous.rows[#previous.rows] + 1 == current.rows[1]
      then
        vim.list_extend(previous.lines, current.lines)
        vim.list_extend(previous.rows, current.rows)
      else
        regions[#regions + 1] = current
      end
      return
    end
    for child in node:iter_children() do
      if child:named() then
        visit(child)
      end
    end
  end
  visit(parser:parse()[1]:root())
  return regions
end

--- 将 help 的标题、参数条目和代码边界转换为 Markdown；插入的分段空行也保留源行映射。
---@param buffer integer
---@return WindowTranslate.Region
local function help_region(buffer)
  local lines = api.nvim_buf_get_lines(buffer, 0, -1, false)
  local parser = assert(vim.treesitter.get_parser(buffer, "vimdoc"))
  local headings, leading_tags, parameters, sections = {}, {}, {}, {}
  local function visit(node)
    local kind = node:type()
    local sr, sc, er, ec = node:range()
    if kind == "tag" and lines[sr + 1]:sub(1, sc):match("^%s*$") then
      leading_tags[sr + 1] = ec
    elseif kind == "argument" and lines[sr + 1]:sub(1, sc):match("^%s*$") then
      parameters[sr + 1] = true
    elseif kind == "delimiter" and vim.tbl_contains({ "h1", "h2" }, node:parent():type()) then
      sections[sr + 1] = true
    end
    if
      kind == "codeblock"
      or kind == "tag"
      or kind == "modeline"
      or kind == "delimiter"
      or (kind == "<" and node:parent():type() == "block")
    then
      for row = sr, er do
        local line = lines[row + 1]
        if line then
          local first, last = row == sr and sc or 0, row == er and ec or #line
          lines[row + 1] = line:sub(1, first) .. string.rep(" ", last - first) .. line:sub(last + 1)
        end
      end
      return
    end
    if kind == "heading" then
      headings[sr + 1] = true
    end
    for child in node:iter_children() do
      if child:named() or child:type() == "<" then
        visit(child)
      end
    end
  end
  visit(parser:parse()[1]:root())
  local result = { lines = {}, rows = {} }
  for row, line in ipairs(lines) do
    if leading_tags[row] then
      line = line:sub(leading_tags[row] + 1):gsub("^[ \t]+", "")
    end
    if headings[row] then
      line = vim.trim(line)
      if line ~= "" and not line:match("^#+%s") then
        line = "# " .. line
      end
    end
    if parameters[row] or sections[row] then
      result.lines[#result.lines + 1] = ""
      result.rows[#result.rows + 1] = row
    end
    result.lines[#result.lines + 1] = sections[row] and "---" or line
    result.rows[#result.rows + 1] = row
    if sections[row] then
      result.lines[#result.lines + 1] = ""
      result.rows[#result.rows + 1] = row
    end
  end
  return result
end

--- 列表只按第一层条目切分，不再遍历条目内部；紧邻列表的引导段落并入第一项。
---@param region WindowTranslate.Region
---@param blocks WindowTranslate.Block[]
---@param indented_prose? boolean help 的真实代码已由 vimdoc 移除，整段缩进文本需要去掉公共排版缩进。
local function paragraphs(region, blocks, indented_prose)
  local text = table.concat(region.lines, "\n")
  local parser = vim.treesitter.get_string_parser(text, "markdown")

  --- Markdown 节点可能包含下一行的缩进，源范围以最后一个正文字符为准。
  ---@param node TSNode
  ---@param lead? TSNode
  ---@return integer first 0-based，转换后文本中的行号。
  ---@return integer last
  ---@return string source
  local function extract(node, lead)
    local first, col = node:start()
    local source = vim.treesitter.get_node_text(node, text):gsub("%s+$", "")
    local _, newlines = source:gsub("\n", "")
    local last = first + newlines
    if node:type() ~= "pipe_table_cell" then
      source = region.lines[first + 1]:sub(1, col) .. source
    end
    if lead then
      first, col = lead:start()
      local heading = region.lines[first + 1]:sub(1, col) .. vim.treesitter.get_node_text(lead, text):gsub("%s+$", "")
      source = heading .. "\n" .. source
    end
    local lines, margin = vim.split(source, "\n", { plain = true }), nil
    for _, line in ipairs(lines) do
      if line:find("%S") then
        local indent = #line:match("^[ \t]*")
        margin = margin and math.min(margin, indent) or indent
      end
    end
    for index, line in ipairs(lines) do
      lines[index] = line:sub((margin or 0) + 1)
    end
    return first, last, table.concat(lines, "\n")
  end

  ---@param list TSNode
  ---@return TSNode?
  local function introduction(list)
    local previous = list:prev_named_sibling()
    if previous and previous:type() == "paragraph" then
      local _, last = extract(previous)
      if last + 1 == list:start() then
        return previous
      end
    end
  end

  ---@param node TSNode
  ---@param lead? TSNode
  local function append(node, lead)
    local first, last, source = extract(node, lead)
    if source:find("%S") then
      blocks[#blocks + 1] = {
        id = #blocks + 1,
        start_line = region.rows[first + 1],
        end_line = region.rows[last + 1],
        source = source,
        text = "",
        complete = false,
      }
    end
  end

  local function visit(node)
    local kind = node:type()
    if kind == "indented_code_block" and indented_prose then
      local first, _, last, end_col = node:range()
      local prose = { lines = {}, rows = {} }
      for row = first + 1, last + (end_col > 0 and 1 or 0) do
        prose.lines[#prose.lines + 1] = region.lines[row]:gsub("^[ \t]+$", "")
        prose.rows[#prose.rows + 1] = region.rows[row]
      end
      -- 整段处理，保留跨空行的列表缩进；空白行不参与公共缩进计算。
      local normalized = vim.text.indent(0, table.concat(prose.lines, "\n"), { expandtab = 4 })
      prose.lines = vim.split(normalized, "\n", { plain = true })
      paragraphs(prose, blocks)
      return
    end
    if
      vim.tbl_contains({
        "fenced_code_block",
        "indented_code_block",
        "html_block",
        "minus_metadata",
        "plus_metadata",
        "link_reference_definition",
      }, kind)
    then
      return
    end
    if kind == "list" then
      local lead = introduction(node)
      for child in node:iter_children() do
        if child:type() == "list_item" then
          append(child, lead)
          lead = nil
        end
      end
      return
    end
    if vim.tbl_contains({ "paragraph", "atx_heading", "setext_heading", "pipe_table_cell" }, kind) then
      local following = node:next_named_sibling()
      if kind == "paragraph" and following and following:type() == "list" and introduction(following) then
        return
      end
      append(node)
      return
    end
    for child in node:iter_children() do
      if child:named() then
        visit(child)
      end
    end
  end
  visit(parser:parse()[1]:root())
end

--- 每个 buffer 版本解析一次，返回按源位置排序的块和逐行索引；同一行允许多个表格单元格。
---@param buffer integer
---@return WindowTranslate.Block[]
---@return table<integer, WindowTranslate.Block[]>
function M.read(buffer)
  local ft = vim.bo[buffer].filetype
  local regions
  if ft == "help" then
    regions = { help_region(buffer) }
  elseif ft == "" or ft == "text" or ft:match("^markdown") then
    local lines, rows = api.nvim_buf_get_lines(buffer, 0, -1, false), {}
    for row = 1, #lines do
      rows[row] = row
    end
    regions = { { lines = lines, rows = rows } }
  else
    regions = comments(buffer)
  end
  local blocks, by_line = {}, {}
  for _, value in ipairs(regions) do
    paragraphs(value, blocks, ft == "help")
  end
  for _, block in ipairs(blocks) do
    for row = block.start_line, block.end_line do
      by_line[row] = by_line[row] or {}
      table.insert(by_line[row], block)
    end
  end
  return blocks, by_line
end

return M
