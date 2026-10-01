import QtQuick
pragma Singleton

// OSD 显示状态（音量）
QtObject {
    property bool osdVisible: false
    property int osdValue: 0 // 0-100
    property string osdIcon: ""
}
