pragma Singleton
import QtQuick

// 顶栏、快捷键与面板共享开合状态。控制中心内部只切换页面，不创建额外窗口。
QtObject {
    property bool calendarOpen: false
    property bool mediaOpen: false
    property bool powerMenuOpen: false
    property bool launcherOpen: false
    property bool controlCenterOpen: false
    property string controlCenterPage: "home"
    readonly property bool anyPanelOpen: calendarOpen || mediaOpen || powerMenuOpen || launcherOpen || controlCenterOpen

    onControlCenterOpenChanged: console.info("[control-center]", controlCenterOpen ? "opened" : "closed", controlCenterPage)
    onControlCenterPageChanged: console.info("[control-center] page", controlCenterPage)

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

    function toggleControlCenter(capsule = null) {
        if (controlCenterOpen)
            controlCenterOpen = false;
        else
            openControlCenter("home", capsule);
    }

    function toggleCalendar() {
        calendarOpen = !calendarOpen;
    }

    function toggleMedia() {
        mediaOpen = !mediaOpen;
    }

    function togglePowerMenu() {
        powerMenuOpen = !powerMenuOpen;
    }

    function toggleLauncher() {
        launcherOpen = !launcherOpen;
    }

    function closeAll() {
        MorphState.reset();
        calendarOpen = false;
        mediaOpen = false;
        powerMenuOpen = false;
        launcherOpen = false;
        controlCenterOpen = false;
    }
}
