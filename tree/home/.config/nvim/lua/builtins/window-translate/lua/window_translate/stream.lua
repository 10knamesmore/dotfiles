local M = {}

--- 空行结束 SSE 事件，同一事件的 data 字段以换行连接。API 完成状态由 client 检查。
---@param on_data fun(data: string) 同步回调，接收去掉 SSE 字段前缀的完整数据。
---@return fun(chunk: string) 接收任意边界的 stdout 字节块；EOF 的不完整事件不发出。
function M.sse(on_data)
  local pending, fields = "", {}
  return function(chunk)
    pending = pending .. chunk
    while true do
      local newline = pending:find("\n", 1, true)
      if not newline then
        return
      end
      local line = pending:sub(1, newline - 1):gsub("\r$", "")
      pending = pending:sub(newline + 1)
      if line == "" then
        if #fields > 0 then
          on_data(table.concat(fields, "\n"))
          fields = {}
        end
      elseif line:sub(1, 5) == "data:" then
        fields[#fields + 1] = line:sub(6):gsub("^ ", "")
      end
    end
  end
end

---@class WindowTranslate.TranslationUpdate
---@field index integer 本批中的 1-based 位置，由字符串数组顺序确定。
---@field text string 当前块的累计译文，可为未完成的句子。

---@class WindowTranslate.TranslationStream
---@field append fun(chunk: string) 接收模型输出的 JSON 文本分片。
---@field take_changes fun(): WindowTranslate.TranslationUpdate[] 取走变化项，同一块只返回最新文本。
---@field finish fun(): string[] 解码完整响应并核对数量；失败抛错。

--- 只扫描 translations 字符串数组的边界；转义和 Unicode 交由 Neovim JSON 解码器处理。
--- 未闭合字符串临时补引号供流式显示，遇到尚未完整的转义时保留上次结果。
---@param count integer 本批请求的块数。
---@return WindowTranslate.TranslationStream
function M.translations(count)
  local raw, index = "", 0
  local in_array, in_string, escaped = false, false, false
  ---@type integer?
  local start
  ---@type table<integer, string>
  local values, changes = {}, {}

  ---@param text string
  local function publish(text)
    assert(index <= count, "翻译响应的块数超出请求数量")
    if values[index] ~= text then
      values[index], changes[index] = text, text
    end
  end

  return {
    append = function(chunk)
      local first = #raw + 1
      raw = raw .. chunk
      for position = first, #raw do
        local char = raw:sub(position, position)
        if in_string then
          if escaped then
            escaped = false
          elseif char == "\\" then
            escaped = true
          elseif char == '"' then
            in_string = false
            if start then
              publish(vim.json.decode(raw:sub(start, position)))
              start = nil
            end
          end
        elseif char == '"' then
          in_string = true
          if in_array then
            index, start = index + 1, position
          end
        elseif char == "[" then
          in_array = true
        elseif char == "]" then
          in_array = false
        end
      end
      if start and not escaped then
        local ok, text = pcall(vim.json.decode, raw:sub(start) .. '"')
        if ok then
          publish(text)
        end
      end
    end,
    take_changes = function()
      local result = {}
      for position, text in pairs(changes) do
        result[#result + 1] = { index = position, text = text }
      end
      changes = {}
      return result
    end,
    finish = function()
      local translations = vim.json.decode(raw).translations
      assert(type(translations) == "table" and vim.islist(translations), "翻译响应缺少 translations 数组")
      assert(#translations == count, "翻译响应的块数与请求不一致")
      for _, text in ipairs(translations) do
        assert(type(text) == "string", "翻译响应中的每一项必须是字符串")
      end
      return translations
    end,
  }
end

return M
