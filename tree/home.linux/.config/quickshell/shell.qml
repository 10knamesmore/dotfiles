//@ pragma IconTheme breeze-dark
//@ pragma UseQApplication

import "./bar"
import "./bluetooth"
import "./display"
import "./calendar"
import "./clipboard"
import "./computer-control"
import "./launcher"
import "./media"
import "./network"
import "./notifications"
import "./osd"
import "./power"
import "./screen-effects"
import "./services"
import "./settings"
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
        name: "screenEffects"
        description: "Toggle screen effects panel"
        onPressed: {
            PanelState.calendarOpen = false;
            PanelState.mediaOpen = false;
            PanelState.toggleScreenEffects();
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
        name: "settings"
        description: "Toggle quick settings"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleSettings();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "bluetooth"
        description: "Toggle bluetooth panel"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleBluetooth();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "display"
        description: "Toggle display management panel"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleDisplay();
        }
    }

    // ── 全局面板（唯一实例）──
    ScreenEffectsPanel {}

    CalendarPanel {}

    MediaPanel {}

    PowerMenu {}

    OsdPanel {}

    NotificationPanel {
        notifServer: notifService.server
    }

    NotificationToast {
        notifServer: notifService.server
    }

    AppLauncher {}

    QuickSettings {}

    Variants {
        model: Quickshell.screens

        delegate: HotEdge {}
    }

    ClipboardPanel {}

    NetworkPanel {}

    BluetoothPanel {}

    DisplayPanel {}
}
