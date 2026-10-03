import "../modules"
import "../navigation"
import "../../state"
import "../../ai-usage"
import QtQuick

// id → bar module 工厂。barScreen / barWindow 在此集中注入。
// 用内联 Component（声明式绑定，宿主属性变化自动透传）而非 Qt.createComponent。
Loader {
    id: host

    property var item: ({
            "id": "",
            "props": {}
        })
    property var barScreen: null
    property var barWindow: null
    property Item morphItem: null
    opacity: MorphState.isHeld(host.morphItem) ? 0 : 1

    sourceComponent: {
        switch (item.id) {
        case "navigation":
            return cNavigation;
        case "tray":
            return cTray;
        case "netspeed":
            return cNetSpeed;
        case "media":
            return cMedia;
        case "clock":
            return cClock;
        case "aiUsage":
            return cAiUsage;
        case "cpu":
            return cCpu;
        case "memory":
            return cMemory;
        case "controlCenter":
            return cControlCenter;
        case "battery":
            return cBattery;
        default:
            return null;
        }
    }

    // 上下文型
    Component {
        id: cNavigation
        NavigationModule {
            barScreen: host.barScreen
        }
    }
    Component {
        id: cTray
        TrayModule {
            barWindow: host.barWindow
        }
    }
    Component {
        id: cNetSpeed
        NetSpeedModule {
        }
    }
    // 普通型
    Component {
        id: cMedia
        MediaModule {
        }
    }
    Component {
        id: cClock
        ClockModule {
        }
    }
    Component {
        id: cAiUsage
        AiUsageModule {
        }
    }
    Component {
        id: cCpu
        CpuModule {
        }
    }
    Component {
        id: cMemory
        MemoryModule {
        }
    }
    Component {
        id: cControlCenter
        ControlCenterModule {
        }
    }
    Component {
        id: cBattery
        BatteryModule {
        }
    }
}
