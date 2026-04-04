pragma Singleton
import QtQuick

// 全局面板状态单例 — 用于 bar 模块与弹出面板之间的跨组件通信
QtObject {
    // ── Bar ──
    property bool barVisible: true
    function toggleBar() { barVisible = !barVisible }

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

    function toggleScreenEffects() { screenEffectsOpen = !screenEffectsOpen }
    function toggleCalendar()      { calendarOpen = !calendarOpen }
    function toggleMedia()         { mediaOpen = !mediaOpen }
    function toggleNotification()  { notificationOpen = !notificationOpen }
    function togglePowerMenu()     { powerMenuOpen = !powerMenuOpen }
    function toggleLauncher()      { launcherOpen = !launcherOpen }
    function toggleSettings()      { settingsOpen = !settingsOpen }
    function toggleClipboard()     { clipboardOpen = !clipboardOpen }
    function toggleKeybindings()   { keybindingsOpen = !keybindingsOpen }
    function toggleNetwork()       { networkOpen = !networkOpen }

    // 关闭所有面板（互斥：打开一个时关闭其他）
    function closeAll() {
        screenEffectsOpen = false
        calendarOpen = false
        mediaOpen = false
        notificationOpen = false
        powerMenuOpen = false
        launcherOpen = false
        settingsOpen = false
        clipboardOpen = false
        keybindingsOpen = false
        networkOpen = false
    }

    // ── 勿扰 ──
    property bool dndEnabled: false

    // ── 通知 ──
    property int notificationCount: 0
    signal clearAllNotifications()

    // ── OSD ──
    property bool osdVisible: false
    property string osdType: ""      // "volume" | "brightness"
    property int osdValue: 0         // 0-100
    property string osdIcon: ""
}
