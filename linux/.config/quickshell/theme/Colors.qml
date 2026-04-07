import QtQuick
import Quickshell.Io
pragma Singleton

// Catppuccin 动态调色板 — 支持 Mocha/Macchiato/Frappe/Latte 切换
QtObject {
    property string currentFlavor: "mocha"

    property color base: "#1e1e2e"
    property color mantle: "#181825"
    property color crust: "#11111b"
    property color text: "#cdd6f4"
    property color subtext1: "#bac2de"
    property color subtext0: "#a6adc8"
    property color overlay2: "#9399b2"
    property color overlay1: "#7f849c"
    property color overlay0: "#6c7086"
    property color surface2: "#585b70"
    property color surface1: "#45475a"
    property color surface0: "#313244"
    property color blue: "#89b4fa"
    property color lavender: "#b4befe"
    property color sapphire: "#74c7ec"
    property color sky: "#89dceb"
    property color teal: "#94e2d5"
    property color green: "#a6e3a1"
    property color yellow: "#f9e2af"
    property color peach: "#fab387"
    property color maroon: "#eba0ac"
    property color red: "#f38ba8"
    property color mauve: "#cba6f7"
    property color pink: "#f5c2e7"
    property color flamingo: "#f2cdcd"
    property color rosewater: "#f5e0dc"

    readonly property bool isLight: currentFlavor === "latte"

    function setFlavor(name) {
        let palettes = {
            "mocha": {
                base: "#1e1e2e", mantle: "#181825", crust: "#11111b",
                text: "#cdd6f4", subtext1: "#bac2de", subtext0: "#a6adc8",
                overlay2: "#9399b2", overlay1: "#7f849c", overlay0: "#6c7086",
                surface2: "#585b70", surface1: "#45475a", surface0: "#313244",
                blue: "#89b4fa", lavender: "#b4befe", sapphire: "#74c7ec",
                sky: "#89dceb", teal: "#94e2d5", green: "#a6e3a1",
                yellow: "#f9e2af", peach: "#fab387", maroon: "#eba0ac",
                red: "#f38ba8", mauve: "#cba6f7", pink: "#f5c2e7",
                flamingo: "#f2cdcd", rosewater: "#f5e0dc"
            },
            "macchiato": {
                base: "#24273a", mantle: "#1e2030", crust: "#181926",
                text: "#cad3f5", subtext1: "#b8c0e0", subtext0: "#a5adcb",
                overlay2: "#939ab7", overlay1: "#8087a2", overlay0: "#6e738d",
                surface2: "#5b6078", surface1: "#494d64", surface0: "#363a4f",
                blue: "#8aadf4", lavender: "#b7bdf8", sapphire: "#7dc4e4",
                sky: "#91d7e3", teal: "#8bd5ca", green: "#a6da95",
                yellow: "#eed49f", peach: "#f5a97f", maroon: "#ee99a0",
                red: "#ed8796", mauve: "#c6a0f6", pink: "#f5bde6",
                flamingo: "#f0c6c6", rosewater: "#f4dbd6"
            },
            "frappe": {
                base: "#303446", mantle: "#292c3c", crust: "#232634",
                text: "#c6d0f5", subtext1: "#b5bfe2", subtext0: "#a5adce",
                overlay2: "#949cbb", overlay1: "#838ba7", overlay0: "#737994",
                surface2: "#626880", surface1: "#51576d", surface0: "#414559",
                blue: "#8caaee", lavender: "#babbf1", sapphire: "#85c1dc",
                sky: "#99d1db", teal: "#81c8be", green: "#a6d189",
                yellow: "#e5c890", peach: "#ef9f76", maroon: "#ea999c",
                red: "#e78284", mauve: "#ca9ee6", pink: "#f4b8e4",
                flamingo: "#eebebe", rosewater: "#f2d5cf"
            },
            "latte": {
                base: "#eff1f5", mantle: "#e6e9ef", crust: "#dce0e8",
                text: "#4c4f69", subtext1: "#5c5f77", subtext0: "#6c6f85",
                overlay2: "#7c7f93", overlay1: "#8c8fa1", overlay0: "#9ca0b0",
                surface2: "#acb0be", surface1: "#bcc0cc", surface0: "#ccd0da",
                blue: "#1e66f5", lavender: "#7287fd", sapphire: "#209fb5",
                sky: "#04a5e5", teal: "#179299", green: "#40a02b",
                yellow: "#df8e1d", peach: "#fe640b", maroon: "#e64553",
                red: "#d20f39", mauve: "#8839ef", pink: "#ea76cb",
                flamingo: "#dd7878", rosewater: "#dc8a78"
            }
        };
        let p = palettes[name];
        if (!p) return;
        currentFlavor = name;
        base = p.base; mantle = p.mantle; crust = p.crust;
        text = p.text; subtext1 = p.subtext1; subtext0 = p.subtext0;
        overlay2 = p.overlay2; overlay1 = p.overlay1; overlay0 = p.overlay0;
        surface2 = p.surface2; surface1 = p.surface1; surface0 = p.surface0;
        blue = p.blue; lavender = p.lavender; sapphire = p.sapphire;
        sky = p.sky; teal = p.teal; green = p.green;
        yellow = p.yellow; peach = p.peach; maroon = p.maroon;
        red = p.red; mauve = p.mauve; pink = p.pink;
        flamingo = p.flamingo; rosewater = p.rosewater;
        _persistWriter.command = ["sh", "-c", "mkdir -p ~/.cache/quickshell && echo '" + name + "' > ~/.cache/quickshell/theme-flavor"];
        _persistWriter.running = true;
    }

    property var _persistWriter: Process {}

    property var _persistReader: Process {
        command: ["sh", "-c", "cat ~/.cache/quickshell/theme-flavor 2>/dev/null || echo mocha"]
        running: true
        stdout: SplitParser {
            onRead: data => {
                let f = data.trim();
                if (f && f !== Colors.currentFlavor)
                    Colors.setFlavor(f);
            }
        }
    }
}
