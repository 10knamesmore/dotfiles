import QtQuick
pragma Singleton

// 面板展开动画源（鼠标点击位置）— bar 模块点击时写入，PanelOverlay 读取做 morph 起点
QtObject {
    property real morphSourceX: -1
    property real morphSourceY: -1

    function reset() {
        morphSourceX = -1;
        morphSourceY = -1;
    }
}
