import "../modules"
import QtQuick

// id → bar module 工厂。特例 props（direction / barScreen / barWindow）与 flat 在此集中注入。
// 用内联 Component（声明式绑定，宿主属性变化自动透传）而非 Qt.createComponent。
Loader {
    id: host

    property var item: ({
        "id": "",
        "props": {}
    })
    property var barScreen: null
    property var barWindow: null
    property bool flat: false

    sourceComponent: {
        switch (item.id) {
        case "workspaces":
            return cWorkspaces;
        case "scrollstatus":
            return cScrollStatus;
        case "windowtitle":
            return cWindowTitle;
        case "tray":
            return cTray;
        case "netspeed":
            return cNetSpeed;
        case "media":
            return cMedia;
        case "clock":
            return cClock;
        case "cpu":
            return cCpu;
        case "memory":
            return cMemory;
        case "audio":
            return cAudio;
        case "network":
            return cNetwork;
        case "clipboard":
            return cClipboard;
        case "notification":
            return cNotification;
        case "screeneffects":
            return cScreenEffects;
        case "battery":
            return cBattery;
        default:
            return null;
        }
    }

    // 上下文型
    Component {
        id: cWorkspaces
        WorkspacesModule {
            barScreen: host.barScreen
            flat: host.flat
        }
    }
    Component {
        id: cScrollStatus
        ScrollStatusModule {
            barScreen: host.barScreen
            flat: host.flat
        }
    }
    Component {
        id: cTray
        TrayModule {
            barWindow: host.barWindow
            flat: host.flat
        }
    }
    // props 型
    Component {
        id: cNetSpeed
        NetSpeedModule {
            direction: host.item.props && host.item.props.direction ? host.item.props.direction : "up"
            flat: host.flat
        }
    }
    // 普通型
    Component {
        id: cWindowTitle
        WindowTitleModule {
            flat: host.flat
        }
    }
    Component {
        id: cMedia
        MediaModule {
            flat: host.flat
        }
    }
    Component {
        id: cClock
        ClockModule {
            flat: host.flat
        }
    }
    Component {
        id: cCpu
        CpuModule {
            flat: host.flat
        }
    }
    Component {
        id: cMemory
        MemoryModule {
            flat: host.flat
        }
    }
    Component {
        id: cAudio
        AudioModule {
            flat: host.flat
        }
    }
    Component {
        id: cNetwork
        NetworkModule {
            flat: host.flat
        }
    }
    Component {
        id: cClipboard
        ClipboardModule {
            flat: host.flat
        }
    }
    Component {
        id: cNotification
        NotificationModule {
            flat: host.flat
        }
    }
    Component {
        id: cScreenEffects
        ScreenEffectsModule {
            flat: host.flat
        }
    }
    Component {
        id: cBattery
        BatteryModule {
            flat: host.flat
        }
    }
}
