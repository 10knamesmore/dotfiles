import QtQuick
pragma Singleton

// 媒体共享状态 — 当前活跃播放器引用 + cava 频谱数据（cava 单例写入，多屏共享）
QtObject {
    property var lastActivePlayer: null
    property var visualizerBars: []
}
