import QtQuick
pragma Singleton

// OSD 显示状态（音量 / 亮度）
QtObject {
    property bool osdVisible: false
    property string osdType: "" // "volume" | "brightness"
    property int osdValue: 0 // 0-100
    property string osdIcon: ""
}
