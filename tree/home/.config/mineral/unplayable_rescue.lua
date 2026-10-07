-- 网易云无可播资源时，异步搜索 Bilibili 并替换音频流，保留原歌曲身份。
-- 即时播放受 script.hook_timeout_ms 限制；预取使用整个预取窗口。
-- 由 daemon.lua 的 setup(api) 加载本文件，再调用返回的注册函数。

-- 搜索关键词:标题去掉「cover:」「翻自:」后缀,拼「标题 - 艺人1 / 艺人2」(最多两位)。
local function keyword_for(song)
  local name = (song.title or "")
    :gsub("（%s*[Cc][Oo][Vv][Ee][Rr][:：%s][^）]*）", "")
    :gsub("%(%s*[Cc][Oo][Vv][Ee][Rr][:：%s][^%)]*%)", "")
    :gsub("（%s*翻自[:：%s][^）]*）", "")
    :gsub("%(%s*翻自[:：%s][^%)]*%)", "")
  local artists = {}
  for i, artist in ipairs(song.artists or {}) do
    if i > 2 then
      break
    end
    artists[#artists + 1] = artist
  end
  if #artists > 0 then
    return name .. " - " .. table.concat(artists, " / ")
  end
  return name
end

-- 选替身:全部命中里取时长差最小、且差 < 5s 的;都对不上返回 nil(宁可不救)。
local function pick(hits, duration_ms)
  if not duration_ms or duration_ms <= 0 then
    return nil
  end
  local best, best_diff = nil, 5000
  for _, hit in ipairs(hits) do
    if hit.duration_ms and hit.duration_ms > 0 then
      local diff = math.abs(hit.duration_ms - duration_ms)
      if diff < best_diff then
        best, best_diff = hit, diff
      end
    end
  end
  return best
end

--- 为不可播的网易云歌曲注册跨源补救。
---@param api mineral.DaemonApi
return function(api)
  api.hook("before_stream", function(ctx)
    if not ctx.unplayable then
      return nil
    end -- 有可播 URL:不掺和
    if ctx.song.source ~= "netease" then
      return nil
    end -- 只救 netease 的歌

    api.library.search(keyword_for(ctx.song), { source = "bilibili", limit = 10 }, function(hits, err)
      if err or not hits or #hits == 0 then
        ctx.resolve(nil) -- 没找到:放行,维持原失败语义
        return
      end
      local best = pick(hits, ctx.song.duration_ms)
      if not best then
        ctx.resolve(nil) -- 无时长贴近的命中:放弃补救,别拿错内容顶
        return
      end
      api.library.song_url(best.id, function(remote, url_err)
        if url_err or not remote then
          ctx.resolve(nil)
          return
        end
        api.log.info("rescue: " .. ctx.song.title .. " <- " .. best.id)
        -- bitrate_bps / format 是替身流的展示元信息,透传后 transport 才显示真值
        ctx.resolve({
          url = remote.url,
          headers = remote.headers,
          layout = remote.layout,
          bitrate_bps = remote.bitrate_bps,
          format = remote.format,
        })
      end)
    end)
    return api.DEFER
  end)
end
