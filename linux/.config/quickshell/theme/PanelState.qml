import QtQuick
pragma Singleton

// 全局面板状态单例 — 用于 bar 模块与弹出面板之间的跨组件通信
QtObject {
    // ── Bar ──
    property bool barPinnedVisible: true
    property string barHoverRevealScreen: ""
    readonly property bool barVisible: barPinnedVisible
    // ── 面板 ──
    property bool screenEffectsOpen: false
    property bool calendarOpen: false
    property bool mediaOpen: false
    property bool notificationOpen: false
    property bool powerMenuOpen: false
    property bool launcherOpen: false
    property bool settingsOpen: false
    property bool clipboardOpen: false
    property bool keybindingsOpen: false
    property bool networkOpen: false
    // ── 桌面小组件 ──
    property bool analogClockVisible: true
    property bool pomodoroVisible: true
    property bool visualizerVisible: true
    // ── 音频可视化共享数据（cava 进程单例，多屏共享）──
    property var visualizerBars: []
    // ── 新面板 ──
    property bool notesOpen: false
    property bool journalOpen: false
    property bool aiOpen: false
    // ── 媒体 ──
    property var lastActivePlayer: null
    // ── 歌词 ──
    property var lyricsLines: []      // [{time: seconds, text: "歌词行"}, ...]
    property int currentLyricIndex: -1
    property string currentLyric: ""
    property string lyricsTrackId: "" // 用于检测歌曲切换
    property real lyricsOffset: 0    // 歌词时间偏移（秒），正值=歌词提前，负值=歌词延后
    // ── 勿扰 ──
    property bool dndEnabled: false
    // ── 通知 ──
    property int notificationCount: 0
    // ── OSD ──
    property bool osdVisible: false
    property string osdType: "" // "volume" | "brightness"
    property int osdValue: 0 // 0-100
    property string osdIcon: ""
    readonly property bool anyPanelOpen: screenEffectsOpen || calendarOpen || mediaOpen || notificationOpen || powerMenuOpen || launcherOpen || settingsOpen || clipboardOpen || keybindingsOpen || networkOpen || notesOpen || journalOpen || aiOpen

    signal clearAllNotifications()

    function toggleBar() {
        barPinnedVisible = !barPinnedVisible;
        if (barPinnedVisible)
            barHoverRevealScreen = "";
    }

    function showBarForScreen(screenName) {
        if (!screenName || barPinnedVisible)
            return;

        barHoverRevealScreen = screenName;
    }

    function hideHoverBar() {
        barHoverRevealScreen = "";
    }

    function isBarVisibleForScreen(screenName) {
        return barPinnedVisible || barHoverRevealScreen === screenName;
    }

    function toggleScreenEffects() {
        screenEffectsOpen = !screenEffectsOpen;
    }

    function toggleCalendar() {
        calendarOpen = !calendarOpen;
    }

    function toggleMedia() {
        mediaOpen = !mediaOpen;
    }

    function toggleNotification() {
        notificationOpen = !notificationOpen;
    }

    function togglePowerMenu() {
        powerMenuOpen = !powerMenuOpen;
    }

    function toggleLauncher() {
        launcherOpen = !launcherOpen;
    }

    function toggleSettings() {
        settingsOpen = !settingsOpen;
    }

    function toggleClipboard() {
        clipboardOpen = !clipboardOpen;
    }

    function toggleKeybindings() {
        keybindingsOpen = !keybindingsOpen;
    }

    function toggleNetwork() {
        networkOpen = !networkOpen;
    }

    function toggleNotes() {
        notesOpen = !notesOpen;
    }

    function toggleJournal() {
        journalOpen = !journalOpen;
    }

    function toggleAi() {
        aiOpen = !aiOpen;
    }

    // 关闭所有面板（互斥：打开一个时关闭其他）
    function closeAll() {
        screenEffectsOpen = false;
        calendarOpen = false;
        mediaOpen = false;
        notificationOpen = false;
        powerMenuOpen = false;
        launcherOpen = false;
        settingsOpen = false;
        clipboardOpen = false;
        keybindingsOpen = false;
        networkOpen = false;
        notesOpen = false;
        journalOpen = false;
        aiOpen = false;
    }

}
