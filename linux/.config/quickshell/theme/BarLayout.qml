pragma Singleton

import QtQuick
import Quickshell.Io

// Bar 顶栏样式预设 — 数据驱动 + 运行时切换 + 持久化。
// 切换只改 currentName，current 计算属性自动跟随，所有 BarLayout.current.* 绑定重算。
QtObject {
    id: root

    property string currentName: "pills"

    // 三段 widget 项支持：字符串 id / {id, props} / {group:[...]}（分组留第二阶段）
    readonly property var presets: [
        {
            "name": "pills",
            "label": "满栏药丸",
            "floating": false,
            "spacing": 6,
            "sideMargin": 2,
            "topMargin": 6,
            "barBackground": false,
            "barAlpha": 0.0,
            "squareCorners": false,
            "moduleFlat": false,
            "leftWidgets": ["workspaces", "scrollstatus", "windowtitle", "tray"],
            "centerWidgets": [{
                "id": "netspeed",
                "props": {
                    "direction": "up"
                }
            }, "media", "clock", {
                "id": "netspeed",
                "props": {
                    "direction": "down"
                }
            }],
            "rightWidgets": ["cpu", "memory", "audio", "network", "clipboard", "notification", "screeneffects", "battery"]
        },
        {
            "name": "floating",
            "label": "悬浮扁平",
            "floating": true,
            "spacing": 10,
            "sideMargin": 12,
            "topMargin": 8,
            "barBackground": true,
            "barAlpha": 0.55,
            "squareCorners": false,
            "moduleFlat": true,
            "leftWidgets": ["workspaces", "scrollstatus", "windowtitle", "tray"],
            "centerWidgets": [{
                "id": "netspeed",
                "props": {
                    "direction": "up"
                }
            }, "media", "clock", {
                "id": "netspeed",
                "props": {
                    "direction": "down"
                }
            }],
            "rightWidgets": ["cpu", "memory", "audio", "network", "clipboard", "notification", "screeneffects", "battery"]
        },
        {
            "name": "grouped",
            "label": "分组悬浮",
            "floating": false,
            "spacing": 8,
            "sideMargin": 6,
            "topMargin": 6,
            "barBackground": false,
            "barAlpha": 0.0,
            "squareCorners": false,
            "moduleFlat": true,
            "leftWidgets": ["workspaces", "scrollstatus", "windowtitle", "tray"],
            "centerWidgets": [{
                "id": "netspeed",
                "props": {
                    "direction": "up"
                }
            }, "media", "clock", {
                "id": "netspeed",
                "props": {
                    "direction": "down"
                }
            }],
            "rightWidgets": [{
                "group": ["cpu", "memory"]
            }, {
                "group": ["audio", "network"]
            }, {
                "group": ["clipboard", "notification", "screeneffects"]
            }, "battery"]
        }
    ]

    readonly property var current: {
        for (let i = 0; i < presets.length; i++) {
            if (presets[i].name === currentName)
                return presets[i];
        }
        return presets[0];
    }

    // 归一化 widget 项 → {id, props}
    function normalize(item) {
        if (typeof item === "string")
            return {
                "id": item,
                "props": {}
            };
        return {
            "id": item.id,
            "props": item.props || {}
        };
    }

    function setLayout(name) {
        currentName = name;
        _persistWriter.command = ["sh", "-c", "mkdir -p ~/.cache/quickshell && echo '" + name + "' > ~/.cache/quickshell/bar-layout"];
        _persistWriter.running = true;
    }

    property var _persistWriter: Process {}

    property var _persistReader: Process {
        command: ["sh", "-c", "cat ~/.cache/quickshell/bar-layout 2>/dev/null || echo pills"]
        running: true
        stdout: SplitParser {
            onRead: data => {
                let n = data.trim();
                if (n && n !== root.currentName)
                    root.setLayout(n);
            }
        }
    }
}
