-- 异步调用 DeepSeek FIM completions 接口。
-- 用 curl(经 vim.system)发请求，回调返回补全文本，并给出取消句柄。
local M = {}

--- 发起一次 FIM 补全请求。
--- @param cfg deepseek_fim.Config
--- @param prompt string 光标前文本
--- @param suffix string 光标后文本
--- @param on_result fun(text: string|nil, err: string|nil, usage: table|nil) 完成回调（可能在 fast-event，调用方需自行 schedule）
--- @return fun()|nil cancel 取消句柄：kill 在途 curl；无法发起时为 nil
function M.request(cfg, prompt, suffix, on_result)
  local key = vim.env[cfg.api_key_env]
  if not key or key == "" then
    on_result(nil, "missing api key: " .. cfg.api_key_env)
    return nil
  end

  local body = vim.json.encode({
    model = cfg.model,
    prompt = prompt,
    suffix = suffix,
    max_tokens = cfg.max_tokens,
    temperature = cfg.temperature,
    stop = cfg.stop,
    stream = false,
  })

  local args = {
    "curl",
    "-sS",
    "--fail-with-body", -- HTTP 错误码也把响应体带回来，便于诊断
    "-X",
    "POST",
    cfg.base_url .. "/completions",
    "-H",
    "Content-Type: application/json",
    "-H",
    "Authorization: Bearer " .. key,
    "--max-time",
    tostring(math.ceil((cfg.timeout_ms or 5000) / 1000)),
    "-d",
    body,
  }

  local ok, proc = pcall(vim.system, args, { text = true }, function(res)
    if res.code ~= 0 then
      on_result(nil, "curl exit " .. tostring(res.code) .. ": " .. (res.stderr or res.stdout or ""))
      return
    end
    local decoded_ok, decoded = pcall(vim.json.decode, res.stdout)
    if not decoded_ok or type(decoded) ~= "table" then
      on_result(nil, "json decode failed")
      return
    end
    local choice = decoded.choices and decoded.choices[1]
    local text = choice and choice.text
    if type(text) ~= "string" or text == "" then
      on_result(nil, "empty completion")
      return
    end
    on_result(text, nil, decoded.usage)
  end)

  if not ok then
    on_result(nil, "failed to spawn curl: " .. tostring(proc))
    return nil
  end

  return function()
    pcall(function()
      proc:kill(15) -- SIGTERM 掉在途 curl
    end)
  end
end

return M
