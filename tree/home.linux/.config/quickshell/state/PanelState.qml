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
    property bool networkOpen: false
    property bool bluetoothOpen: false
    property bool displayOpen: false
    readonly property bool anyPanelOpen: screenEffectsOpen || calendarOpen || mediaOpen || notificationOpen || powerMenuOpen || launcherOpen || settingsOpen || clipboardOpen || networkOpen || bluetoothOpen || displayOpen

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

    function toggleNetwork() {
        networkOpen = !networkOpen;
    }

    function toggleBluetooth() {
        bluetoothOpen = !bluetoothOpen;
    }

    function toggleDisplay() {
        displayOpen = !displayOpen;
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
        networkOpen = false;
        bluetoothOpen = false;
        displayOpen = false;
    }

}
