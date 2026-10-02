pragma Singleton
import QtQuick

// OSD 显示状态（音量）
QtObject {
    property bool osdVisible: false
    property int osdValue: 0 // 0-100
    property string osdIcon: ""
}
