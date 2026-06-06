import QtQuick
pragma Singleton

// 全局面板状态单例 — 用于 bar 模块与弹出面板之间的跨组件通信
QtObject {
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
    // ── 新面板 ──
    property bool notesOpen: false
    property bool journalOpen: false
    property bool aiOpen: false
    property bool bluetoothOpen: false
    property bool displayOpen: false
    property bool systemMonitorOpen: false
    property string systemMonitorTab: "cpu"
    readonly property bool anyPanelOpen: screenEffectsOpen || calendarOpen || mediaOpen || notificationOpen || powerMenuOpen || launcherOpen || settingsOpen || clipboardOpen || keybindingsOpen || networkOpen || notesOpen || journalOpen || aiOpen || bluetoothOpen || displayOpen || systemMonitorOpen

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

    function toggleBluetooth() {
        bluetoothOpen = !bluetoothOpen;
    }

    function toggleDisplay() {
        displayOpen = !displayOpen;
    }

    function toggleSystemMonitor() {
        systemMonitorOpen = !systemMonitorOpen;
    }

    // 关闭所有面板（互斥：打开一个时关闭其他）
    function closeAll() {
        MorphState.reset();
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
        bluetoothOpen = false;
        displayOpen = false;
        systemMonitorOpen = false;
    }

}
