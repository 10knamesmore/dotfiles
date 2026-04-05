import "../components"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris
import Quickshell.Wayland

// Quick Settings — 左侧滑出面板
PanelOverlay {
    id: root

    // ── 系统状态 ──
    property bool wifiEnabled: true
    property bool btEnabled: true
    property string wifiName: ""
    property string btDevice: ""
    property int brightnessValue: 100
    property int volumePct: 0
    property bool volumeMuted: false
    property bool nightLightEnabled: false
    property bool caffeineEnabled: false
    // 夜灯状态文件/脚本路径（与 ScreenEffectsPanel 共享）
    property string _home: Quickshell.env("HOME")
    property string _effectsState: _home + "/.cache/hypr/screen-effects.json"
    property string _effectsScript: _home + "/dotfiles/generated/scripts/hypr/screen_effects.sh"

    function refreshStatus() {
        wifiProc.running = true;
        btPowerProc.running = true;
        btDeviceProc.running = true;
        brightnessProc.running = true;
        nightLightReader.running = true;
        caffeineCheckProc.running = true;
    }

    function toggleNightLight() {
        let json;
        if (root.nightLightEnabled)
            json = JSON.stringify({
            "warmth": 0,
            "grain": 0,
            "grain_size": 50,
            "shadow_boost": 40
        });
        else
            json = JSON.stringify({
            "warmth": 60,
            "grain": 85,
            "grain_size": 10,
            "shadow_boost": 40
        });
        nightLightWriter.command = ["sh", "-c", "echo '" + json + "' > " + root._effectsState];
        nightLightWriter.running = true;
        nightLightApplier.command = [root._effectsScript, "apply"];
        nightLightApplier.running = true;
        root.nightLightEnabled = !root.nightLightEnabled;
    }

    function toggleCaffeine() {
        if (root.caffeineEnabled) {
            caffeineStopProc.command = ["sh", "-c", "kill $(cat /tmp/quickshell-caffeine.pid 2>/dev/null) 2>/dev/null; rm -f /tmp/quickshell-caffeine.pid"];
            caffeineStopProc.running = true;
            root.caffeineEnabled = false;
        } else {
            caffeineStartProc.command = ["sh", "-c", "systemd-inhibit --what=idle --who=quickshell --why=caffeine sleep infinity & echo $! > /tmp/quickshell-caffeine.pid"];
            caffeineStartProc.running = true;
            root.caffeineEnabled = true;
        }
    }

    showing: PanelState.settingsOpen
    panelWidth: 340
    panelHeight: root.height - 64
    panelTargetX: 10
    panelTargetY: 54
    closedOffsetX: -360
    closedOffsetY: 0
    onCloseRequested: PanelState.settingsOpen = false
    onShowingChanged: {
        if (showing)
            refreshStatus();

    }

    // ── 进程 ──
    Process {
        id: wifiProc

        command: ["nmcli", "radio", "wifi"]

        stdout: SplitParser {
            onRead: (data) => {
                return root.wifiEnabled = data.trim() === "enabled";
            }
        }

    }

    Process {
        id: wifiNameProc

        command: ["nmcli", "-t", "-f", "active,ssid", "dev", "wifi"]

        stdout: SplitParser {
            onRead: (data) => {
                if (data.startsWith("yes:"))
                    root.wifiName = data.substring(4);

            }
        }

    }

    Process {
        id: btPowerProc

        command: ["bluetoothctl", "show"]

        stdout: SplitParser {
            onRead: (data) => {
                if (data.includes("Powered:"))
                    root.btEnabled = data.includes("yes");

            }
        }

    }

    Process {
        id: btDeviceProc

        command: ["bluetoothctl", "devices", "Connected"]

        stdout: SplitParser {
            onRead: (data) => {
                let parts = data.split(" ");
                if (parts.length >= 3)
                    root.btDevice = parts.slice(2).join(" ");

            }
        }

    }

    Process {
        id: brightnessProc

        command: ["brightnessctl", "-m"]

        stdout: SplitParser {
            onRead: (data) => {
                let parts = data.split(",");
                if (parts.length >= 4)
                    root.brightnessValue = parseInt(parts[3]) || 0;

            }
        }

    }

    Process {
        id: volProc

        command: ["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"]

        stdout: SplitParser {
            onRead: (data) => {
                let m = data.match(/Volume:\s+([\d.]+)(\s+\[MUTED\])?/);
                if (m) {
                    root.volumePct = Math.round(parseFloat(m[1]) * 100);
                    root.volumeMuted = m[2] !== undefined;
                }
            }
        }

    }

    Process {
        id: actionProc
    }

    // 夜灯状态读取
    Process {
        id: nightLightReader

        command: ["cat", root._effectsState]

        stdout: SplitParser {
            onRead: (data) => {
                try {
                    let obj = JSON.parse(data);
                    root.nightLightEnabled = (obj.warmth > 0 || obj.grain > 0);
                } catch (e) {
                }
            }
        }

    }

    // 夜灯写入 + 应用
    Process {
        id: nightLightWriter
    }

    Process {
        id: nightLightApplier
    }

    // 咖啡因 — systemd-inhibit (通过 PID 文件管理)
    Process {
        id: caffeineStartProc
    }

    Process {
        id: caffeineStopProc
    }

    Process {
        id: caffeineCheckProc

        command: ["sh", "-c", "kill -0 $(cat /tmp/quickshell-caffeine.pid 2>/dev/null) 2>/dev/null && echo 1 || echo 0"]

        stdout: SplitParser {
            onRead: (data) => {
                return root.caffeineEnabled = data.trim() === "1";
            }
        }

    }

    Timer {
        running: root.showing
        interval: 40
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            volProc.running = true;
        }
    }

    // ── UI ──
    Flickable {
        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        contentHeight: mainCol.implicitHeight
        clip: true

        ColumnLayout {
            id: mainCol

            // ── 系统信息（可点击展开）──
            property bool infoExpanded: false

            width: parent.width
            spacing: Tokens.spaceM

            // ── 用户头像 ──
            ProfileHeader {
                Layout.fillWidth: true
            }

            Divider {
                Layout.fillWidth: true
            }

            // ── 滑块区 ──
            Text {
                text: "调节"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                font.letterSpacing: 2
                font.weight: Font.Medium
            }

            SettingsSlider {
                Layout.fillWidth: true
                icon: root.brightnessValue > 50 ? "󰃠" : "󰃞"
                value: root.brightnessValue / 100
                label: "亮度 " + root.brightnessValue + "%"
                accentColor: Colors.yellow
                onMoved: (val) => {
                    root.brightnessValue = Math.round(val * 100);
                    actionProc.command = [Quickshell.env("HOME") + "/dotfiles/generated/scripts/hypr/screen_effects.sh", "brightness", String(root.brightnessValue)];
                    actionProc.running = true;
                }
            }

            SettingsSlider {
                Layout.fillWidth: true
                icon: root.volumeMuted ? "" : (root.volumePct < 50 ? "" : "")
                value: root.volumePct / 100
                label: "音量 " + root.volumePct + "%"
                accentColor: Colors.blue
                onMoved: (val) => {
                    actionProc.command = ["wpctl", "set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", Math.round(val * 100) + "%"];
                    actionProc.running = true;
                }
            }

            Divider {
                Layout.fillWidth: true
            }

            // ── 开关区 ──
            Text {
                text: "快捷开关"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                font.letterSpacing: 2
                font.weight: Font.Medium
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 3
                rowSpacing: 8
                columnSpacing: 8

                QuickToggle {
                    icon: root.wifiEnabled ? "󰤨" : "󰤭"
                    label: "WiFi"
                    status: root.wifiName || (root.wifiEnabled ? "已开启" : "已关闭")
                    toggled: root.wifiEnabled
                    onClicked: {
                        actionProc.command = ["nmcli", "radio", "wifi", root.wifiEnabled ? "off" : "on"];
                        actionProc.running = true;
                        root.wifiEnabled = !root.wifiEnabled;
                    }
                    onRightClicked: {
                        PanelState.settingsOpen = false;
                        PanelState.toggleNetwork();
                    }
                }

                QuickToggle {
                    icon: root.btEnabled ? "󰂯" : "󰂲"
                    label: "蓝牙"
                    status: root.btDevice || (root.btEnabled ? "已开启" : "已关闭")
                    toggled: root.btEnabled
                    onClicked: {
                        actionProc.command = ["bluetoothctl", "power", root.btEnabled ? "off" : "on"];
                        actionProc.running = true;
                        root.btEnabled = !root.btEnabled;
                    }
                }

                QuickToggle {
                    icon: PanelState.dndEnabled ? "󰂛" : "󰂚"
                    label: "勿扰"
                    status: PanelState.dndEnabled ? "已开启" : "已关闭"
                    toggled: PanelState.dndEnabled
                    onClicked: PanelState.dndEnabled = !PanelState.dndEnabled
                }

                QuickToggle {
                    icon: root.volumeMuted ? "" : "󰕾"
                    label: "静音"
                    status: root.volumeMuted ? "已静音" : "未静音"
                    toggled: root.volumeMuted
                    onClicked: {
                        actionProc.command = ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"];
                        actionProc.running = true;
                    }
                }

                QuickToggle {
                    icon: root.nightLightEnabled ? "󰛨" : "󰹏"
                    label: "夜灯"
                    status: root.nightLightEnabled ? "已开启" : "已关闭"
                    toggled: root.nightLightEnabled
                    onClicked: root.toggleNightLight()
                }

                QuickToggle {
                    icon: root.caffeineEnabled ? "󰅶" : "󰾪"
                    label: "咖啡因"
                    status: root.caffeineEnabled ? "保持唤醒" : "已关闭"
                    toggled: root.caffeineEnabled
                    onClicked: root.toggleCaffeine()
                }

                QuickToggle {
                    icon: "󰈋"
                    label: "取色器"
                    status: "hyprpicker"
                    toggled: false
                    onClicked: {
                        PanelState.settingsOpen = false;
                        actionProc.command = ["hyprpicker", "-a"];
                        actionProc.running = true;
                    }
                }

            }

            Divider {
                Layout.fillWidth: true
            }

            // ── 媒体卡片 ──
            MediaCard {
                Layout.fillWidth: true
                player: {
                    let ps = Mpris.players.values;
                    for (let i = 0; i < ps.length; i++) {
                        if (ps[i].isPlaying) {
                            PanelState.lastActivePlayer = ps[i];
                            return ps[i];
                        }
                    }
                    if (PanelState.lastActivePlayer && ps.indexOf(PanelState.lastActivePlayer) >= 0)
                        return PanelState.lastActivePlayer;
                    return ps.length > 0 ? ps[0] : null;
                }
            }

            // ── 天气卡片 ──
            WeatherCard {
                Layout.fillWidth: true
            }

            // ── 电池卡片 ──
            BatteryCard {
                Layout.fillWidth: true
            }

            Divider {
                Layout.fillWidth: true
            }

            // ── 截图 ──
            Text {
                text: "工具"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                font.letterSpacing: 2
                font.weight: Font.Medium
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Tokens.spaceS

                ToolButton {
                    icon: "󰹑"
                    label: "区域截图"
                    command: "hyprshot -m region"
                }

                ToolButton {
                    icon: "󰖯"
                    label: "窗口截图"
                    command: "hyprshot -m window"
                }

            }

            // ── 显示器切换 ──
            RowLayout {
                Layout.fillWidth: true
                spacing: Tokens.spaceS

                ToolButton {
                    icon: "󰍹"
                    label: "双屏"
                    command: root._home + "/dotfiles/generated/scripts/hypr/monitor_profile.sh dual"
                    closeOnClick: false
                }

                ToolButton {
                    icon: "󰶐"
                    label: "外接"
                    command: root._home + "/dotfiles/generated/scripts/hypr/monitor_profile.sh external"
                    closeOnClick: false
                }

                ToolButton {
                    icon: "󰌢"
                    label: "笔记本"
                    command: root._home + "/dotfiles/generated/scripts/hypr/monitor_profile.sh laptop"
                    closeOnClick: false
                }

            }

            Divider {
                Layout.fillWidth: true
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: sysInfoCol.implicitHeight + 20
                radius: Tokens.radiusM
                color: sysHover.containsMouse ? Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, Tokens.cardAlpha) : Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, Tokens.cardAlpha)
                border.color: sysHover.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, Tokens.borderHoverAlpha) : Qt.rgba(1, 1, 1, 0.06)
                border.width: 1
                clip: true

                MouseArea {
                    id: sysHover

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: mainCol.infoExpanded = !mainCol.infoExpanded
                }

                ColumnLayout {
                    id: sysInfoCol

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: 10
                    spacing: Tokens.spaceS

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        Text {
                            text: "󰍹"
                            color: Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon
                        }

                        Text {
                            id: uptimeText

                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.small
                            text: "uptime..."

                            Timer {
                                running: root.showing
                                interval: 60000
                                repeat: true
                                triggeredOnStart: true
                                onTriggered: uptimeProc.running = true
                            }

                            Process {
                                id: uptimeProc

                                command: ["sh", "-c", "uptime -p | sed 's/up //'"]

                                stdout: SplitParser {
                                    onRead: (data) => {
                                        return uptimeText.text = data;
                                    }
                                }

                            }

                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        Text {
                            text: mainCol.infoExpanded ? "󰅃" : "󰅀"
                            color: Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon
                        }

                        // 重载按钮（阻止点击穿透到卡片）
                        Rectangle {
                            width: 28
                            height: 28
                            radius: Tokens.radiusFull
                            color: reloadHover.containsMouse ? Colors.surface2 : "transparent"

                            Text {
                                anchors.centerIn: parent
                                text: "󰑓"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.icon
                            }

                            MouseArea {
                                id: reloadHover

                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: (mouse) => {
                                    mouse.accepted = true;
                                    Quickshell.reload(true);
                                }
                            }

                            Behavior on color {
                                ColorAnimation {
                                    duration: 150
                                }

                            }

                        }

                    }

                    // 折叠的系统信息
                    SystemInfo {
                        Layout.fillWidth: true
                        expanded: mainCol.infoExpanded
                    }

                }

                Behavior on color {
                    ColorAnimation {
                        duration: 200
                        easing.type: Easing.OutCubic
                    }

                }

                Behavior on border.color {
                    ColorAnimation {
                        duration: 200
                        easing.type: Easing.OutCubic
                    }

                }

                Behavior on implicitHeight {
                    NumberAnimation {
                        duration: 200
                        easing.type: Easing.OutCubic
                    }

                }

            }

            Divider {
                Layout.fillWidth: true
            }

            // ── 电源操作 ──
            Text {
                text: "电源"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                font.letterSpacing: 2
                font.weight: Font.Medium
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Tokens.spaceS

                PowerButton {
                    icon: "󰌾"
                    label: "锁屏"
                    command: "hyprlock"
                }

                PowerButton {
                    icon: "󰍃"
                    label: "注销"
                    command: "hyprctl dispatch exit"
                }

                PowerButton {
                    icon: "󰤄"
                    label: "挂起"
                    command: "systemctl suspend"
                }

                PowerButton {
                    icon: "󰜉"
                    label: "重启"
                    command: "systemctl reboot"
                }

                PowerButton {
                    icon: "󰐥"
                    label: "关机"
                    command: "systemctl poweroff"
                }

            }

        }

    }

}
