--- Advertise this editor and give one live RPC channel exclusive Pi ownership.
--- Editing remains available through Neovim's native API, not bridge-specific wrappers.
local M = {}
local uv = vim.uv
local owner
local record
local registry_path
local write_timer
local owner_timer
local stopped = false

---@class PiConnection
---@field session_id string Pi session identifier
---@field cwd string Pi working directory
---@field pid integer Pi Python worker PID
---@field connected_at number Unix seconds when ownership began
---@field model table|userdata Model {provider, id, name}; vim.NIL when Pi has no selected model

local function log(event, fields)
  local entry = fields or {}
  entry.event = event
  entry.at = os.time()
  local directory = vim.fn.stdpath("state")
  vim.fn.mkdir(directory, "p")
  local fd = uv.fs_open(directory .. "/pi-nvim.log", "a", 384)
  if fd then
    uv.fs_write(fd, vim.json.encode(entry) .. "\n", -1)
    uv.fs_close(fd)
  end
end

--- Log failure metadata only; params, results and server error messages may contain document text.
---@param reason string
---@param fields? table
function M.fail(reason, fields)
  log(reason, fields)
  error("pi-nvim: " .. reason, 0)
end

--- Record lifecycle/request metadata without document content.
---@param event string
---@param fields? table
function M.log(event, fields)
  log(event, fields)
end

local function validate_model(model)
  if
    model ~= vim.NIL
    and (
      type(model) ~= "table"
      or type(model.provider) ~= "string"
      or type(model.id) ~= "string"
      or type(model.name) ~= "string"
    )
  then
    M.fail("invalid_model")
  end
end

local function channel_alive(channel)
  local ok, info = pcall(vim.api.nvim_get_chan_info, channel)
  return ok and info.mode == "rpc" and info.stream == "socket"
end

local function refresh_record()
  record.cwd = vim.fn.getcwd()
  record.current_file = vim.api.nvim_buf_get_name(0)
  record.connection = owner and vim.deepcopy(owner.identity) or vim.NIL
  local roots = {}
  for _, client in ipairs(vim.lsp.get_clients()) do
    if client.root_dir then
      roots[client.root_dir] = true
    end
    for _, folder in ipairs(client.workspace_folders or {}) do
      roots[vim.uri_to_fname(folder.uri)] = true
    end
  end
  record.workspace_roots = vim.tbl_keys(roots)
  table.sort(record.workspace_roots)
  return record
end

local function write_record()
  if stopped then
    return
  end
  local temporary = registry_path .. ".tmp"
  local fd = uv.fs_open(temporary, "w", 384)
  if not fd then
    log("registry_open_failed")
    return
  end
  local payload = vim.json.encode(refresh_record())
  local ok = uv.fs_fchmod(fd, 384)
  local written = ok and uv.fs_write(fd, payload, 0)
  uv.fs_close(fd)
  if written ~= #payload or not uv.fs_rename(temporary, registry_path) then
    uv.fs_unlink(temporary)
    log("registry_write_failed")
  end
end

local function changed()
  vim.cmd.redrawstatus()
  if stopped or not write_timer then
    return
  end
  -- Keep a fixed 200 ms coalescing window; frequent events must not postpone publication forever.
  if not write_timer:is_active() then
    write_timer:start(200, 0, vim.schedule_wrap(write_record))
  end
end

local function cancel_pending(connection)
  local pending = connection.pending
  connection.pending = {}
  for _, request in pairs(pending) do
    if request.request_id then
      local ok = pcall(request.client.cancel_request, request.client, request.request_id)
      if not ok then
        log("lsp_cancel_failed", { client_id = request.client.id })
      end
    end
  end
end

local function release(reason)
  if not owner then
    return
  end
  local previous = owner
  -- Invalidate callbacks before cancellation, including synchronous cancellation callbacks.
  owner = nil
  owner_timer:stop()
  cancel_pending(previous)
  log("disconnected", { channel = previous.channel, reason = reason })
  changed()
  if not stopped then
    vim.api.nvim_exec_autocmds("User", { pattern = "PiConnectionChanged", data = { connected = false } })
  end
end

local function check_connection()
  if owner and not channel_alive(owner.channel) then
    release("channel_closed")
  end
end

--- Require the active owner; sibling LSP code uses the returned per-connection pending requests.
---@param channel integer
---@return table connection
function M.require_owner(channel)
  check_connection()
  if not owner or owner.channel ~= channel then
    M.fail("not_owner", { channel = channel })
  end
  return owner
end

--- Check callback ownership without granting a newly connected channel old requests.
---@param connection table
---@return boolean
function M.is_owner(connection)
  check_connection()
  return owner == connection
end

local function label(text)
  return text:gsub("[%c]", " "):gsub("%%", "%%%%")
end

--- Return the statusline's disconnected, connected, or active-operation label.
---@return string
function M.statusline()
  if not owner then
    return "Pi —"
  end
  local parts = {}
  local model = owner.identity.model
  if model ~= vim.NIL then
    parts[#parts + 1] = label(model.name)
  end
  parts[#parts + 1] = label(owner.identity.session_id:sub(1, 8))
  if owner.operation then
    parts[#parts + 1] = label(owner.operation)
  end
  return (owner.operation and "Pi ◌ " or "Pi ● ") .. table.concat(parts, " · ")
end

local function setup()
  if record then
    return true
  end
  local base
  if uv.os_uname().sysname == "Darwin" then
    base = vim.env.TMPDIR
  else
    base = vim.env.XDG_RUNTIME_DIR
  end
  if not base or base == "" then
    log("runtime_directory_missing")
    vim.notify("Pi connection is unavailable: the runtime directory is not configured.", vim.log.levels.WARN)
    return false
  end
  local directory = base:gsub("/+$", "") .. "/pi-nvim"
  -- Other editors may create the shared directory concurrently; validate the resulting directory.
  uv.fs_mkdir(directory, 448)
  local stat = uv.fs_lstat(directory)
  if not stat or stat.type ~= "directory" or stat.uid ~= uv.getuid() or not uv.fs_chmod(directory, 448) then
    M.fail("registry_directory_not_private")
  end
  local instance_id = string.format("%d-%s", uv.os_getpid(), tostring(uv.hrtime()))
  local socket = vim.v.servername
  if socket == "" then
    socket = vim.fn.serverstart(directory .. "/" .. instance_id .. ".sock")
  end
  if socket:sub(1, 1) ~= "/" then
    M.fail("unix_socket_required")
  end
  record = {
    instance_id = instance_id,
    pid = uv.os_getpid(),
    started_at = os.time(),
    socket = socket,
  }
  registry_path = directory .. "/" .. instance_id .. ".json"
  write_timer = uv.new_timer()
  owner_timer = uv.new_timer()
  local group = vim.api.nvim_create_augroup("PiBridge", { clear = true })
  vim.api.nvim_create_autocmd({ "BufEnter", "BufFilePost", "DirChanged", "LspAttach", "LspDetach" }, {
    group = group,
    callback = changed,
  })
  vim.api.nvim_create_autocmd("LspNotify", {
    group = group,
    callback = function(event)
      if event.data.method == "workspace/didChangeWorkspaceFolders" then
        changed()
      end
    end,
  })
  vim.api.nvim_create_autocmd("VimLeavePre", {
    group = group,
    callback = function()
      stopped = true
      release("editor_exit")
      write_timer:stop()
      write_timer:close()
      owner_timer:close()
      uv.fs_unlink(registry_path)
      log("editor_exit", { instance_id = instance_id })
    end,
  })
  vim.api.nvim_create_user_command("PiConnection", function()
    check_connection()
    local connection = refresh_record().connection
    local message = "Pi: disconnected"
    if connection ~= vim.NIL then
      message = string.format(
        "Pi: connected\nSession: %s\nDirectory: %s\nWorker: %d\nConnected: %s",
        connection.session_id,
        connection.cwd,
        connection.pid,
        os.date("%Y-%m-%d %H:%M:%S", connection.connected_at)
      )
      if connection.model ~= vim.NIL then
        message = message .. "\nModel: " .. connection.model.name
        message = message .. "\nProvider: " .. connection.model.provider
        message = message .. "\nModel ID: " .. connection.model.id
      end
      if owner.operation then
        message = message .. "\nOperation: " .. owner.operation
        message = message .. "\nStarted: " .. os.date("%Y-%m-%d %H:%M:%S", owner.operation_started_at)
      end
    end
    vim.notify(message, vim.log.levels.INFO, { title = "Pi connection" })
  end, { desc = "Show this editor's Pi connection" })
  vim.api.nvim_create_user_command("PiDisconnect", function()
    check_connection()
    local channel = owner and owner.channel
    release("user_disconnect")
    if channel then
      local ok, closed = pcall(vim.fn.chanclose, channel)
      if not ok or closed ~= 1 then
        log("channel_close_failed", { channel = channel })
        vim.notify("Pi was released, but its connection could not be closed.", vim.log.levels.WARN)
        return
      end
    end
    vim.notify("Pi: disconnected", vim.log.levels.INFO, { title = "Pi connection" })
  end, { desc = "Release Pi and close its RPC connection" })
  write_record()
  log("editor_ready", { instance_id = instance_id, socket = socket })
  return true
end

--- Initialize private discovery files, lifecycle hooks, commands and connection monitoring once.
---@return boolean ready False disables the bridge without preventing the editor from starting.
function M.setup()
  local ok, ready = pcall(setup)
  if not ok then
    log("setup_failed")
    vim.notify(
      "Pi connection is unavailable: initialization failed. See pi-nvim.log in Neovim's state directory.",
      vim.log.levels.WARN
    )
    return false
  end
  return ready
end

--- Claim this instance for a socket RPC channel. A live second channel cannot take ownership.
---@param channel integer
---@param identity {session_id: string, cwd: string, pid: integer, model: table|userdata}
---@param expected_instance_id string Discovery instance selected by Pi
---@return table record Current discovery record, including connection identity
function M.connect(channel, identity, expected_instance_id)
  if not record or stopped then
    M.fail("bridge_not_ready")
  end
  if record.instance_id ~= expected_instance_id then
    M.fail("instance_mismatch", { channel = channel })
  end
  check_connection()
  if owner and owner.channel ~= channel then
    M.fail("already_connected", { channel = channel })
  end
  if type(channel) ~= "number" or channel <= 0 or channel % 1 ~= 0 or not channel_alive(channel) then
    M.fail("invalid_channel")
  end
  if
    type(identity) ~= "table"
    or type(identity.session_id) ~= "string"
    or identity.session_id == ""
    or type(identity.cwd) ~= "string"
    or type(identity.pid) ~= "number"
    or identity.pid <= 0
    or identity.pid % 1 ~= 0
  then
    M.fail("invalid_identity", { channel = channel })
  end
  validate_model(identity.model)
  if owner then
    if
      not vim.deep_equal({
        session_id = owner.identity.session_id,
        cwd = owner.identity.cwd,
        pid = owner.identity.pid,
        model = owner.identity.model,
      }, identity)
    then
      M.fail("identity_mismatch", { channel = channel })
    end
    return vim.deepcopy(refresh_record())
  end
  owner = {
    channel = channel,
    identity = {
      session_id = identity.session_id,
      cwd = identity.cwd,
      pid = identity.pid,
      model = vim.deepcopy(identity.model),
      connected_at = os.time(),
    },
    pending = {},
  }
  owner_timer:start(500, 500, vim.schedule_wrap(check_connection))
  log("connected", { channel = channel, pid = identity.pid })
  changed()
  vim.api.nvim_exec_autocmds("User", { pattern = "PiConnectionChanged", data = { connected = true } })
  return vim.deepcopy(refresh_record())
end

--- Publish the selected model to statusline, connection details and the discovery summary.
---@param channel integer
---@param model table|userdata Model {provider, id, name}; vim.NIL when no model is selected
function M.update_model(channel, model)
  local connection = M.require_owner(channel)
  validate_model(model)
  if vim.deep_equal(connection.identity.model, model) then
    return
  end
  connection.identity.model = vim.deepcopy(model)
  log("model_changed", { channel = channel, model = model })
  changed()
end

--- Release ownership and cancel pending LSP requests; the client closes its own socket afterwards.
---@param channel integer
function M.disconnect(channel)
  M.require_owner(channel)
  release("client_disconnect")
end

--- Show a Pi-supplied operation label while the Python client is waiting for work to finish.
---@param channel integer
---@param operation_label string
function M.begin_operation(channel, operation_label)
  local connection = M.require_owner(channel)
  if type(operation_label) ~= "string" or operation_label == "" then
    M.fail("invalid_operation_label")
  end
  connection.operation = operation_label
  connection.operation_started_at = os.time()
  vim.cmd.redrawstatus()
end

--- Restore the connected label without changing discovery metadata.
---@param channel integer
function M.end_operation(channel)
  local connection = M.require_owner(channel)
  connection.operation = nil
  connection.operation_started_at = nil
  vim.cmd.redrawstatus()
end

return M
