import QtQuick
pragma Singleton

// 歌词状态 — LyricsService 写入，bar 歌词模块 / 媒体面板读取。
// 行模型 lyricsLines 每行：
//   { time: 秒, text: "整行文本",
//     words: [{ start: ms, duration: ms, text: "字" }, ...],  // 逐字，空数组=无逐字
//     translation: "译文行" | "", romanization: "罗马音行" | "" }
QtObject {
    property var lyricsLines: []
    property int currentLyricIndex: -1
    property string currentLyric: ""
    property string lyricsTrackId: "" // 用于检测歌曲切换
    property real lyricsOffset: 0    // 歌词时间偏移（秒），正值=歌词提前，负值=歌词延后

    // ── 逐字增强 ──
    property bool hasWords: false      // 当前曲是否含逐字数据（mineral:words）
    property real currentTimeMs: 0     // 当前播放位置（毫秒，含 offset），逐字 wipe 用

    // ── 翻译/罗马音轨 ──
    property bool hasTranslation: false   // 当前曲是否含翻译轨（决定 toggle 显隐）
    property bool hasRomanization: false  // 当前曲是否含罗马音轨
    property bool showTranslation: true   // 用户开关：是否显示翻译
    property bool showRomanization: true  // 用户开关：是否显示罗马音
}
