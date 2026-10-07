---@type mineral.DaemonConfig
return {
  --- 注册网易云不可播歌曲的 Bilibili 跨源补救。
  ---@param api mineral.DaemonApi
  setup = function(api)
    local setup_rescue = dofile(api.sys.paths.config .. "/unplayable_rescue.lua")
    setup_rescue(api)
  end,
  queue = {
    transforms = {
      {
        name = "Sort by album",
        transform = function(queue)
          -- 专辑名相同时按主艺人排序；缺失的名称排在最前。
          table.sort(queue, function(a, b)
            local aa, ba = a.album or "", b.album or ""
            if aa ~= ba then
              return aa < ba
            end
            return (a.artists[1] or "") < (b.artists[1] or "")
          end)
          return queue
        end,
      },
    },
  },
  sources = {
    bilibili = {
      curate_collections = function(collections)
        local keep = {}
        for _, collection in ipairs(collections) do
          if collection.track_count and collection.track_count > 0 and collection.name:match("^音乐") then
            keep[#keep + 1] = collection
          end
        end
        return keep
      end,
    },
  },
}
