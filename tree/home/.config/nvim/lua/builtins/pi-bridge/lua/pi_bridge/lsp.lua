--- Forward raw LSP traffic through existing Neovim clients without changing editor focus.
--- Every call belongs to the current Pi channel; callbacks cannot outlive that ownership.
local M = {}
local bridge = require("pi_bridge")
local api = vim.api

local function client_by_id(client_id)
  local client = vim.lsp.get_client_by_id(client_id)
  if not client or client:is_stopped() then
    bridge.fail("client_unavailable", { client_id = client_id })
  end
  return client
end

local function loaded_buffer(buffer)
  if type(buffer) ~= "number" or buffer < 0 or buffer % 1 ~= 0 then
    bridge.fail("invalid_buffer")
  end
  buffer = buffer == 0 and api.nvim_get_current_buf() or buffer
  if not api.nvim_buf_is_loaded(buffer) then
    bridge.fail("buffer_not_loaded", { buffer = buffer })
  end
  return buffer
end

local function request_buffer(params, buffer)
  if buffer ~= nil and buffer ~= vim.NIL then
    return loaded_buffer(buffer)
  end
  local document = type(params) == "table" and params.textDocument
  if type(document) == "table" and type(document.uri) == "string" then
    for _, candidate in ipairs(api.nvim_list_bufs()) do
      if api.nvim_buf_is_loaded(candidate) and vim.uri_from_bufnr(candidate) == document.uri then
        return candidate
      end
    end
  end
  -- Client:request/exec_cmd resolve nil and 0 to the current buffer. Neovim preserves -1,
  -- so workspace/unopened-document requests get no current-buffer version or didChange context.
  return -1
end

local function begin_request(channel, token)
  local connection = bridge.require_owner(channel)
  if type(token) ~= "string" or token == "" then
    bridge.fail("invalid_token")
  end
  if connection.pending[token] then
    bridge.fail("token_in_use")
  end
  local request = {}
  connection.pending[token] = request
  local function complete(err, result)
    if not bridge.is_owner(connection) or connection.pending[token] ~= request then
      return
    end
    connection.pending[token] = nil
    request.finished = true
    if err and err ~= vim.NIL then
      bridge.log("lsp_request_failed", { client_id = request.client and request.client.id, code = err.code })
    end
    -- Lua's `or` would erase valid false results; only absent values become RPC null.
    local ok = pcall(vim.rpcnotify, channel, "pi_nvim_lsp", token, {
      result = result == nil and vim.NIL or result,
      error = err == nil and vim.NIL or err,
    })
    if not ok then
      bridge.log("lsp_notification_failed", { channel = channel })
    end
  end
  return connection, request, complete
end

local function request_failed(complete)
  complete({ code = -32603, message = "Pi Neovim request failed" }, nil)
end

local function registered(connection, token, request, request_id)
  request.request_id = request_id
  -- LspRequest autocmds can disconnect Pi while Client:request is still registering the ID.
  if
    request_id
    and not request.finished
    and (not bridge.is_owner(connection) or connection.pending[token] ~= request)
  then
    local ok = pcall(request.client.cancel_request, request.client, request_id)
    if not ok then
      bridge.log("lsp_cancel_failed", { client_id = request.client.id })
    end
  end
end

--- List active clients with native capabilities and buffer IDs as an array, not integer-keyed RPC maps.
---@param channel integer
---@return table[] clients Fields: id, name, root_dir, workspace_folders, offset_encoding, server_capabilities, attached_buffers
function M.clients(channel)
  bridge.require_owner(channel)
  local clients = {}
  for _, client in ipairs(vim.lsp.get_clients()) do
    local buffers = {}
    for buffer, attached in pairs(client.attached_buffers) do
      if attached then
        buffers[#buffers + 1] = buffer
      end
    end
    table.sort(buffers)
    clients[#clients + 1] = {
      id = client.id,
      name = client.name,
      root_dir = client.root_dir or vim.NIL,
      workspace_folders = client.workspace_folders or {},
      offset_encoding = client.offset_encoding,
      server_capabilities = client.server_capabilities,
      attached_buffers = buffers,
    }
  end
  table.sort(clients, function(a, b)
    return a.id < b.id
  end)
  return clients
end

--- Build document position params from an explicit loaded buffer and native editor cursor tuple.
--- A client must be attached and support the method. Ambiguity reports candidate IDs instead of choosing one.
---@param channel integer
---@param method string Raw LSP method
---@param buffer integer 0 explicitly selects the current buffer
---@param position integer[] {1-based row, 0-based byte column}
---@param client_id? integer Select one eligible client explicitly
---@return table selection {client_id, params={textDocument={uri},position={line,character}}}
function M.position_params(channel, method, buffer, position, client_id)
  bridge.require_owner(channel)
  buffer = loaded_buffer(buffer)
  local candidates = {}
  for _, client in ipairs(vim.lsp.get_clients({ bufnr = buffer })) do
    if client:supports_method(method, buffer) then
      candidates[#candidates + 1] = client
    end
  end
  table.sort(candidates, function(a, b)
    return a.id < b.id
  end)
  local client
  for _, candidate in ipairs(candidates) do
    if candidate.id == client_id then
      client = candidate
    end
  end
  if client_id == nil or client_id == vim.NIL then
    client = #candidates == 1 and candidates[1] or nil
  end
  if not client then
    local ids = vim.tbl_map(function(candidate)
      return candidate.id
    end, candidates)
    bridge.fail("position_client_selection_failed; candidate ids: " .. table.concat(ids, ","))
  end
  if
    type(position) ~= "table"
    or #position ~= 2
    or type(position[1]) ~= "number"
    or type(position[2]) ~= "number"
    or position[1] % 1 ~= 0
    or position[2] % 1 ~= 0
    or position[1] < 1
    or position[1] > api.nvim_buf_line_count(buffer)
    or position[2] < 0
  then
    bridge.fail("invalid_position")
  end
  local row, column = position[1] - 1, position[2]
  local line = api.nvim_buf_get_lines(buffer, row, row + 1, true)[1]
  if column > #line then
    bridge.fail("invalid_position")
  end
  return {
    client_id = client.id,
    params = {
      textDocument = { uri = vim.uri_from_bufnr(buffer) },
      position = { line = row, character = vim.str_utfindex(line, client.offset_encoding, column, false) },
    },
  }
end

--- Start a native asynchronous request. Accepted tokens complete via pi_nvim_lsp even on synchronous failure.
--- Params and results are unmodified; no loaded buffer or server capability is required for raw methods.
---@param channel integer
---@param token string Unique among this connection's pending operations
---@param client_id integer Existing client
---@param method string Standard or extended LSP method
---@param params any Native LSP params, including vim.NIL
---@param buffer? integer Explicit context, otherwise derive a loaded document buffer or use -1
---@return boolean accepted
function M.start(channel, token, client_id, method, params, buffer)
  local connection, request, complete = begin_request(channel, token)
  local ok = pcall(function()
    request.client = client_by_id(client_id)
    local success, request_id = request.client:request(method, params, complete, request_buffer(params, buffer))
    registered(connection, token, request, request_id)
    if not success then
      request_failed(complete)
    end
  end)
  if not ok then
    bridge.log("lsp_start_failed", { client_id = client_id })
    request_failed(complete)
  end
  return true
end

--- Cancel a native request and complete its token with RequestCancelled; late replies are ignored.
---@param channel integer
---@param token string
---@return boolean cancelled False when the token has already completed
function M.cancel(channel, token)
  local connection = bridge.require_owner(channel)
  local request = connection.pending[token]
  if not request then
    return false
  end
  connection.pending[token] = nil
  if request.request_id then
    local ok = pcall(request.client.cancel_request, request.client, request.request_id)
    if not ok then
      bridge.log("lsp_cancel_failed", { client_id = request.client.id })
    end
  end
  local ok = pcall(vim.rpcnotify, channel, "pi_nvim_lsp", token, {
    result = vim.NIL,
    error = { code = -32800, message = "Request cancelled" },
  })
  if not ok then
    bridge.log("lsp_notification_failed", { channel = channel })
  end
  return true
end

--- Send an unmodified native LSP notification to an existing client.
---@param channel integer
---@param client_id integer
---@param method string
---@param params any
---@return boolean success
function M.notify(channel, client_id, method, params)
  bridge.require_owner(channel)
  local client = client_by_id(client_id)
  local ok, success = pcall(client.notify, client, method, params)
  if not ok then
    bridge.fail("lsp_notify_failed", { client_id = client_id })
  end
  if not success then
    bridge.log("lsp_notify_failed", { client_id = client_id })
  end
  return success
end

--- Reuse unsaved document contents or load a hidden buffer and attach an existing client.
--- Native BufRead/FileType/LspAttach hooks run; no window is switched and no client is started by the bridge.
---@param channel integer
---@param client_id integer
---@param uri string Document URI
---@return integer buffer Loaded buffer; use native Neovim APIs to unload it later
function M.open_document(channel, client_id, uri)
  bridge.require_owner(channel)
  local client = client_by_id(client_id)
  local buffer = vim.uri_to_bufnr(uri)
  vim.fn.bufload(buffer)
  if not api.nvim_buf_is_loaded(buffer) then
    bridge.fail("document_load_failed")
  end
  if vim.bo[buffer].filetype == "" then
    local filetype = vim.filetype.match({ buf = buffer })
    if filetype then
      vim.bo[buffer].filetype = filetype
    end
  end
  if not vim.lsp.buf_attach_client(buffer, client.id) then
    bridge.fail("document_attach_failed", { client_id = client_id })
  end
  return buffer
end

--- Execute a native client-side command or workspace/executeCommand and complete the same token protocol.
--- Native client-side commands have no completion callback: completion means their function returned.
---@param channel integer
---@param token string
---@param client_id integer
---@param command table Native lsp.Command
---@param buffer? integer Explicit command context; omitted context uses -1, never the current buffer
---@return boolean accepted
function M.execute_command(channel, token, client_id, command, buffer)
  local connection, request, complete = begin_request(channel, token)
  local observer
  local ok = pcall(function()
    local client = client_by_id(client_id)
    request.client = client
    local local_command = client.commands[command.command] or vim.lsp.commands[command.command]
    local provider = client.server_capabilities.executeCommandProvider
    if
      not local_command
      and not vim.list_contains(type(provider) == "table" and provider.commands or {}, command.command)
    then
      complete({ code = -32601, message = "Command is not supported by this client" }, nil)
      return
    end
    -- Client:exec_cmd does not return a request ID. Observe its synchronous native registration
    -- so disconnect/cancel can cancel the server request without replacing Client:request.
    if not local_command then
      observer = api.nvim_create_autocmd("LspRequest", {
        callback = function(event)
          local data = event.data
          if
            data.client_id == client.id
            and data.request.type == "pending"
            and data.request.method == "workspace/executeCommand"
          then
            registered(connection, token, request, data.request_id)
          end
        end,
      })
    end
    client:exec_cmd(command, { bufnr = request_buffer(nil, buffer) }, complete)
    if local_command then
      complete(nil, nil)
    elseif connection.pending[token] == request and not request.request_id then
      request_failed(complete)
    end
  end)
  if observer then
    api.nvim_del_autocmd(observer)
  end
  if not ok then
    bridge.log("lsp_command_failed", { client_id = client_id })
    request_failed(complete)
  end
  return true
end

return M
