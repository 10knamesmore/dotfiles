import QtQuick
pragma Singleton

// 媒体共享状态 — cava 频谱数据（cava 单例写入，多屏共享）。
// 当前活跃 player 选取已收口到 services/MediaService.qml 的 activePlayer。
QtObject {
    property var visualizerBars: []
}
