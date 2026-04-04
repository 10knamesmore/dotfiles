import QtQuick
import Quickshell.Hyprland._Ipc
import "../components"
import "../../theme"

// 活动窗口标题，带图标替换（对应 Waybar hyprland/window rewrites）
BarModule {
    id: root
    accentColor: Colors.mauve
    backgroundColor: Colors.mantle
    implicitWidth: Math.min(titleText.implicitWidth + 32, 400)

    property string rawTitle: Hyprland.activeToplevel ? Hyprland.activeToplevel.title : ""

    // 最多显示 50 个字符
    property string displayTitle: {
        let t = rawTitle;
        return t.length > 50 ? t.substring(0, 47) + "…" : t;
    }

    clip: true

    Text {
        id: titleText
        anchors.centerIn: parent
        width: Math.min(implicitWidth, root.width - 32)
        text: root.displayTitle
        color: Colors.lavender
        font.family: "Hack Nerd Font"
        font.pixelSize: 13
        font.weight: Font.Bold
        font.italic: true
        elide: Text.ElideRight
    }
}
