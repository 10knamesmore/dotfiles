return {
  "fei6409/log-highlight.nvim",
  event = { "BufEnter *.log", "BufReadPost *.log" },
  -- 选项名是单数：上游 defaults 为 extension/filename/pattern/keyword
  -- （log-highlight.lua:5-16），setup 只做 tbl_deep_extend，写成复数会被当成
  -- 无关新键静默吞掉、整段配置零生效。
  opts = {
    extension = { "log" },
    pattern = {
      ".*%.log",
    },
    keyword = {
      error = { "ERROR", "FATAL", "PANIC", "Error" },
      warning = { "WARN", "WARNING", "CAUTION" },
      info = { "INFO", "NOTE", "IMPORTANT" },
      debug = { "DEBUG", "TRACE", "VERBOSE" },
      pass = { "PASS", "SUCCESS", "OK" },
    },
  },
}
