pragma Singleton

import QtQuick

// 顶栏布局：左侧导航信息带，中部独立模块，右侧按功能分组。
QtObject {
    // 模块热区相接；内容用内边距留白，分组不再额外占宽。
    readonly property int spacing: 0
    readonly property int modulePadding: 10

    // widget 项支持：字符串 id / {id, props} / {group: [...]}
    readonly property var leftWidgets: ["navigation", "tray"]
    readonly property var centerWidgets: ["media"]
    readonly property var rightWidgets: [
        {
            "group": ["cpu", "memory", "netspeed"]
        },
        "aiUsage", "capture", "controlCenter", "battery", "clock"]

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
}
