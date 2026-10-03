//@ pragma IconTheme breeze-dark
//@ pragma UseQApplication

import "./bar"
import "./calendar"
import "./computer-control"
import "./controlcenter"
import "./launcher"
import "./media"
import "./notifications"
import "./osd"
import "./power"
import "./resources"
import "./services"
import "./state"
import QtQuick
import Quickshell
import Quickshell.Hyprland._GlobalShortcuts

ShellRoot {
    id: root

    // ── 后台服务：歌词 / OSD / 通知 ──
    LyricsService {}

    OsdService {
        id: osdService
    }

    NotificationService {
        id: notifService
    }

    SystemStatsService {}

    ResourceMonitorService {}

    MonitorService {}

    ScreenEffectsService {}

    ComputerControlService {}

    // ── 每个显示器生成一个 Bar ──
    Variants {
        model: Quickshell.screens

        delegate: Bar {}
    }

    Variants {
        model: Quickshell.screens

        delegate: BarRevealEdge {}
    }

    // ── 全局快捷键 ──
    GlobalShortcut {
        appid: "quickshell"
        name: "toggleBar"
        description: "Toggle bar visibility"
        onPressed: {
            PanelState.closeAll();
            BarState.toggleBar();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "powerMenu"
        description: "Toggle power menu"
        onPressed: {
            PanelState.closeAll();
            PanelState.togglePowerMenu();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "osdVolume"
        description: "Show volume OSD"
        onPressed: osdService.requestVolumeOsd()
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "launcher"
        description: "Toggle app launcher"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleLauncher();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "controlCenter"
        description: "Toggle control center"
        onPressed: PanelState.toggleControlCenter()
    }

    // ── 全局面板（唯一实例）──
    CalendarPanel {}

    MediaPanel {}

    CpuPanel {}

    MemoryPanel {}

    NetworkPanel {}

    PowerMenu {}

    OsdPanel {}

    ControlCenter {
        notifServer: notifService.server
    }

    NotificationToast {
        notifServer: notifService.server
    }

    AppLauncher {}
}
