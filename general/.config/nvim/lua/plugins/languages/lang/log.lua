return {
    "fei6409/log-highlight.nvim",
    event = { "BufEnter *.log", "BufReadPost *.log" },
    opts = {
        extensions = { "log" },
        patterns = {
            ".*%.log",
        },
        keywords = {
            error = { "ERROR", "FATAL", "PANIC", "Error" },
            warning = { "WARN", "WARNING", "CAUTION" },
            info = { "INFO", "NOTE", "IMPORTANT" },
            debug = { "DEBUG", "TRACE", "VERBOSE" },
            pass = { "PASS", "SUCCESS", "OK" },
        },
    },
}
