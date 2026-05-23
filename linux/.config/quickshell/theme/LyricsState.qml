import QtQuick
pragma Singleton

// 歌词状态 — LyricsService 写入，bar 歌词模块 / 媒体面板读取
QtObject {
    property var lyricsLines: []      // [{time: seconds, text: "歌词行"}, ...]
    property int currentLyricIndex: -1
    property string currentLyric: ""
    property string lyricsTrackId: "" // 用于检测歌曲切换
    property real lyricsOffset: 0    // 歌词时间偏移（秒），正值=歌词提前，负值=歌词延后
}
