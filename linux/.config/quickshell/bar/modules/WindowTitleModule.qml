import "../../theme"
import "../components"
import QtQuick
import Quickshell.Hyprland._Ipc

// 活动窗口标题，带图标替换（对应 Waybar hyprland/window rewrites）
BarModule {
    id: root

    property string rawTitle: Hyprland.activeToplevel ? Hyprland.activeToplevel.title : ""
    // 最多显示 50 个字符
    property string displayTitle: {
        let t = rawTitle;
        return t.length > 50 ? t.substring(0, 47) + "…" : t;
    }

    accentColor: Colors.mauve
    implicitWidth: Math.min(titleText.implicitWidth + 32, 400)
    clip: true

    Text {
        id: titleText

        anchors.centerIn: parent
        width: Math.min(implicitWidth, root.width - 32)
        text: root.displayTitle
        color: Colors.text
        font.family: Fonts.family
        font.pixelSize: Fonts.bodyLarge
        font.weight: Font.Medium
        font.italic: true
        elide: Text.ElideRight
    }

}
