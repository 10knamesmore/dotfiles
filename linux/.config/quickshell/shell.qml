//@ pragma IconTheme breeze-dark
//@ pragma UseQApplication

import "./ai"
import "./bar"
import "./bluetooth"
import "./display"
import "./calendar"
import "./clipboard"
import "./journal"
import "./keybindings"
import "./launcher"
import "./media"
import "./network"
import "./notes"
import "./notifications"
import "./osd"
import "./power"
import "./screen-effects"
import "./services"
import "./settings"
import "./systemmonitor"
import "./state"
import "./widgets"
import QtQuick
import Quickshell
import Quickshell.Hyprland._GlobalShortcuts

ShellRoot {
    id: root

    // ── 后台服务（歌词 / OSD / 通知，逻辑已从 root 抽离到 services/）──
    LyricsService {}

    OsdService {
        id: osdService
    }

    NotificationService {
        id: notifService
    }

    SystemStatsService {}

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
        name: "osdBrightness"
        description: "Show brightness OSD"
        onPressed: osdService.requestBrightnessOsd()
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
        name: "keybindings"
        description: "Toggle keybindings cheat sheet"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleKeybindings();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "notes"
        description: "Toggle notes panel"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleNotes();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "journal"
        description: "Toggle journal log panel"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleJournal();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "ai"
        description: "Toggle AI assistant panel"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleAi();
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

    KeybindingsPanel {}

    NetworkPanel {}

    BluetoothPanel {}

    DisplayPanel {}

    SystemMonitorPanel {}

    // ── 新增面板 ──
    JournalPanel {}

    NotesPanel {}

    AiPanel {}

    // ── 桌面浮动组件 ──
    // 保留音频频谱（常驻最前端可见）；其余 widget 多被窗口遮挡用不到，已禁用以省 CPU。
    CavaService {}

    Variants {
        model: Quickshell.screens
        delegate: AudioVisualizer {}
    }

    // 已禁用（用不到，约省 7% CPU）。如需恢复，取消对应 Variants 注释即可。
    /*
    Variants { model: Quickshell.screens; delegate: AnalogClock {} }
    Variants { model: Quickshell.screens; delegate: PomodoroTimer {} }
    Variants { model: Quickshell.screens; delegate: WeatherWidget {} }
    Variants { model: Quickshell.screens; delegate: NowPlaying {} }
    Variants { model: Quickshell.screens; delegate: SystemMonitor {} }
    */
}
