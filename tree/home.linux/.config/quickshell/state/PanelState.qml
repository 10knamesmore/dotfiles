pragma Singleton
import QtQuick

// 顶栏、快捷键与面板共享开合状态。控制中心内部只切换页面，不创建额外窗口。
QtObject {
    id: root

    readonly property bool anyPanelOpen: calendarOpen || mediaOpen || powerMenuOpen || launcherOpen || controlCenterOpen || cpuOpen || memoryOpen || networkStatsOpen || aiUsageOpen
    property bool aiUsageOpen: false
    property bool calendarOpen: false
    property bool controlCenterOpen: false
    property string controlCenterPage: "home"
    readonly property int controlCenterTab: tabForControlPage(controlCenterPage)
    property bool cpuOpen: false
    property bool launcherOpen: false
    property bool mediaOpen: false
    property bool memoryOpen: false
    property bool networkStatsOpen: false
    property bool powerMenuOpen: false

    function closeAll() {
        MorphState.reset();
        calendarOpen = false;
        mediaOpen = false;
        powerMenuOpen = false;
        launcherOpen = false;
        controlCenterOpen = false;
        cpuOpen = false;
        memoryOpen = false;
        networkStatsOpen = false;
        aiUsageOpen = false;
    }
    function openControlCenter(page = "home", capsule = null) {
        if (controlCenterOpen) {
            controlCenterPage = page;
            return;
        }
        closeAll();
        controlCenterPage = page;
        if (capsule)
            MorphState.openFrom(capsule, () => controlCenterOpen = true);
        else
            controlCenterOpen = true;
    }
    function tabForControlPage(page) {
        return page === "notifications" ? 1 : page === "clipboard" ? 2 : 0;
    }
    function toggleCalendar() {
        calendarOpen = !calendarOpen;
    }
    function toggleControlCenter(capsule = null) {
        if (controlCenterOpen)
            controlCenterOpen = false;
        else
            openControlCenter("home", capsule);
    }
    function toggleLauncher() {
        launcherOpen = !launcherOpen;
    }
    function toggleMedia() {
        mediaOpen = !mediaOpen;
    }
    function togglePowerMenu() {
        powerMenuOpen = !powerMenuOpen;
    }
    function toggleAiUsage(capsule) {
        if (aiUsageOpen) {
            aiUsageOpen = false;
            return;
        }
        closeAll();
        MorphState.openFrom(capsule, () => aiUsageOpen = true);
    }
    function toggleResourcePanel(resource, capsule) {
        const field = resource === "network" ? "networkStatsOpen" : resource + "Open";
        if (root[field]) {
            root[field] = false;
            return;
        }
        closeAll();
        MorphState.openFrom(capsule, () => root[field] = true);
    }

    onAiUsageOpenChanged: console.info("[ai-usage] panel", aiUsageOpen ? "opened" : "closed")
    onControlCenterOpenChanged: console.info("[control-center]", controlCenterOpen ? "opened" : "closed", controlCenterPage)
    onControlCenterPageChanged: console.info("[control-center] page", controlCenterPage)
}
