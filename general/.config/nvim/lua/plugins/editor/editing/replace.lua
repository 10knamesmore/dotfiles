-- Search and replace using ripgrep
return {
    "chrisgrieser/nvim-rip-substitute",
    opts = {
        popupWin = {
            title = " search & replace",
            border = "rounded", -- `vim.o.winborder` on nvim 0.11, otherwise "rounded"
            matchCountHlGroup = "Keyword",
            noMatchHlGroup = "ErrorMsg",
            position = "top", ---@type "top"|"bottom"
            hideSearchReplaceLabels = false,
            hideKeymapHints = true,
            disableCompletions = true, -- such as from blink.cmp
        },
        prefill = {
            normal = "cursorWord", ---@type "cursorWord"|false
            visual = "selection", ---@type "selection"|false -- (does not work with ex-command – see README)
            startInReplaceLineIfPrefill = true,
            alsoPrefillReplaceLine = false,
        },
        keymaps = { -- normal mode (if not stated otherwise)
            abort = "<Esc>",
            confirmAndSubstituteInBuffer = "<CR>",
            insertModeConfirmAndSubstituteInBuffer = "<C-CR>",
            prevSubstitutionInHistory = "<Up>",
            nextSubstitutionInHistory = "<Down>",
            toggleFixedStrings = "<C-f>", -- ripgrep's `--fixed-strings`
            toggleIgnoreCase = "<C-c>", -- ripgrep's `--ignore-case`
            openAtRegex101 = "R",
            showHelp = "?",
        },
        regexOptions = {
            startWithFixedStrings = false,
            startWithIgnoreCase = true,
            pcre2 = true, -- enables lookarounds and backreferences, but slightly slower
            autoBraceSimpleCaptureGroups = true, -- disable if using named capture groups (see README for details)
        },
        editingBehavior = {
            -- Typing `()` in the `search` line, automatically adds `$n` to the `replace` line.
            autoCaptureGroups = true,
        },
        notification = {
            onSuccess = true,
            icon = "",
        },
    },
    keys = {
        {
            "R",
            function()
                require("rip-substitute").sub()
            end,
            mode = { "n", "x" },
            desc = "Rip Substitute",
        },
    },
}
