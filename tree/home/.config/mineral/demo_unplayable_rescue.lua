-- demo:netease 无版权曲跨源补救(before_stream + DEFER + bilibili)
--
-- 机制:netease 取链失败(无版权 404 / 只有试听片段)时,daemon 以 unplayable 口
-- fire `before_stream`(ctx.url == nil)。本脚本返回 mineral.DEFER 后异步搜 bilibili,
-- 按「标题 - 艺人」关键词 + 时长贴近度挑替身,取流后把 {url, headers, layout} 经
-- ctx.resolve 顶入——歌的身份仍是 netease,只有音频流换了源。
-- 手动点播走 immediate 口(预算 = hook_timeout_ms,来不及就超时放行);
-- gapless 自动续播走 prefetch 口(有整个预取窗口的预算,补上后依然无缝)。
--
-- 挂载:在 ~/.config/mineral/config.lua 顶层(return 之前)加一行:
--   dofile((os.getenv("HOME") or "") .. "/.config/mineral/demo_unplayable_rescue.lua")

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

mineral.hook("before_stream", function(ctx)
  if not ctx.unplayable then
    return nil
  end -- 有可播 URL:不掺和
  if ctx.song.source ~= "netease" then
    return nil
  end -- 只救 netease 的歌

  mineral.library.search(keyword_for(ctx.song), { source = "bilibili", limit = 10 }, function(hits, err)
    if err or not hits or #hits == 0 then
      ctx.resolve(nil) -- 没找到:放行,维持原失败语义
      return
    end
    local best = pick(hits, ctx.song.duration_ms)
    if not best then
      ctx.resolve(nil) -- 无时长贴近的命中:放弃补救,别拿错内容顶
      return
    end
    mineral.library.song_url(best.id, function(remote, url_err)
      if url_err or not remote then
        ctx.resolve(nil)
        return
      end
      mineral.log.info("rescue: " .. (ctx.song.title or "?") .. " <- " .. best.id)
      -- card 是单 opts table(body 必填);同 id 顶替,连续补救不堆叠。
      mineral.ui.card({
        title = "source substituted",
        id = "rescue" .. ctx.song.id,
        ttl_secs = 4,
        body = {
          { "track     ", { ctx.song.title or "?", fg = "text", bold = true } },
          { "stand-in  ", { best.title or best.id, fg = "accent" } },
        },
      })
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
  return mineral.DEFER
end)
