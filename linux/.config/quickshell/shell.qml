//@ pragma IconTheme breeze-dark
//@ pragma UseQApplication

import "./bar"
import "./calendar"
import "./clipboard"
import "./keybindings"
import "./launcher"
import "./media"
import "./network"
import "./notifications"
import "./osd"
import "./power"
import "./screen-effects"
import "./settings"
import "./theme"
import QtQuick
import Quickshell
import Quickshell.Hyprland._GlobalShortcuts
import Quickshell.Io
import Quickshell.Services.Notifications
import Quickshell.Services.Pipewire

ShellRoot {
    id: root

    // ── OSD: 音量变化检测（Pipewire 响应式）──
    property var _sink: Pipewire.defaultAudioSink
    property real _lastVolume: -1

    // ── 每个显示器生成一个 Bar ──
    Variants {
        model: Quickshell.screens

        delegate: Bar {}
    }

    // ── 全局快捷键 ──
    GlobalShortcut {
        appid: "quickshell"
        name: "toggleBar"
        description: "Toggle bar visibility"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleBar();
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
        onPressed: brightnessProc.running = true
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

    Connections {
        function onVolumesChanged() {
            let vol = Math.round(root._sink.audio.volume * 100);
            if (root._lastVolume >= 0 && vol !== root._lastVolume) {
                PanelState.osdType = "volume";
                PanelState.osdValue = vol;
                PanelState.osdIcon = root._sink.audio.muted ? "" : (vol < 50 ? "" : "");
                PanelState.osdVisible = true;
            }
            root._lastVolume = vol;
        }

        function onMutedChanged() {
            let vol = Math.round(root._sink.audio.volume * 100);
            PanelState.osdType = "volume";
            PanelState.osdValue = vol;
            PanelState.osdIcon = root._sink.audio.muted ? "" : "";
            PanelState.osdVisible = true;
        }

        target: root._sink ? root._sink.audio : null
    }

    // ── OSD: 亮度检测 ──
    Process {
        id: brightnessProc

        command: ["brightnessctl", "-m"]

        stdout: SplitParser {
            onRead: data => {
                let parts = data.split(",");
                if (parts.length >= 4) {
                    let pct = parseInt(parts[3]) || 0;
                    PanelState.osdType = "brightness";
                    PanelState.osdValue = pct;
                    PanelState.osdIcon = pct > 50 ? "󰃠" : "󰃞";
                    PanelState.osdVisible = true;
                }
            }
        }
    }

    // ── 通知服务 ──
    NotificationServer {
        id: notifServer

        keepOnReload: true
        bodySupported: true
        bodyMarkupSupported: true
        imageSupported: true
        actionsSupported: true
        persistenceSupported: true
        onNotification: notification => {
            notification.tracked = true;
        }
    }

    Connections {
        function onObjectInsertedPost() {
            PanelState.notificationCount = notifServer.trackedNotifications.count ?? 0;
        }

        function onObjectRemovedPost() {
            PanelState.notificationCount = notifServer.trackedNotifications.count ?? 0;
        }

        target: notifServer.trackedNotifications
    }

    Connections {
        function onClearAllNotifications() {
            let notifs = notifServer.trackedNotifications;
            for (let i = notifs.count - 1; i >= 0; i--) {
                notifs.values[i].dismiss();
            }
        }

        target: PanelState
    }

    // ── 全局面板（唯一实例）──
    ScreenEffectsPanel {}

    CalendarPanel {}

    MediaPanel {}

    PowerMenu {}

    OsdPanel {}

    NotificationPanel {
        notifServer: notifServer
    }

    NotificationToast {
        notifServer: notifServer
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
}
