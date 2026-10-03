import "../components"
import "../state"
import "../state/StatsFormat.js" as StatsFormat
import "../theme"
import QtQuick
import QtQuick.Controls

// 独立网络面板；接口选择不重建窗口，每个接口保留自己的上下行历史。
PanelOverlay {
    id: root

    readonly property real downMax: scaleFor(history.down)
    readonly property var history: ResourceStats.networkHistories[ResourceStats.networkInterface] || {
        down: [],
        up: []
    }
    readonly property var iface: network.interfaces.find(item => item.name === ResourceStats.networkInterface) || null
    // 名称集合不变时，不重置正在操作的 ComboBox 模型。
    readonly property string interfaceNames: network.interfaces.map(item => item.name).join("\n")
    readonly property var network: ResourceStats.network
    readonly property real upMax: scaleFor(history.up)

    function scaleFor(points) {
        const peak = points.reduce((maximum, point) => Math.max(maximum, point.value), 1024);
        return Math.pow(2, Math.ceil(Math.log2(peak)));
    }

    panelHeight: content.implicitHeight + Tokens.spaceL * 2
    panelTargetY: 54
    panelWidth: 520
    showing: PanelState.networkStatsOpen

    onCloseRequested: PanelState.networkStatsOpen = false

    Column {
        id: content

        anchors.left: parent.left
        anchors.margins: Tokens.spaceL
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: Tokens.spaceM

        Text {
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            text: root.network.status === "error" ? "网络数据不可用" : "没有网络接口"
            visible: root.network.status === "error" || (root.network.status === "ready" && !root.interfaceNames)
        }
        Column {
            spacing: Tokens.spaceM
            visible: root.network.status !== "error" && root.iface !== null
            width: parent.width

            ComboBox {
                id: interfacePicker

                Accessible.name: "网络接口"
                currentIndex: model.indexOf(ResourceStats.networkInterface)
                font.family: Fonts.family
                font.pixelSize: Fonts.body
                implicitHeight: 32
                model: root.interfaceNames ? root.interfaceNames.split("\n") : []
                palette.base: Colors.surface0
                palette.button: Colors.surface0
                palette.buttonText: Colors.text
                palette.highlight: Colors.surface1
                palette.highlightedText: Colors.teal
                palette.text: Colors.text
                palette.window: Colors.surface0
                palette.windowText: Colors.text
                width: parent.width

                background: Rectangle {
                    border.color: interfacePicker.activeFocus ? Colors.teal : Colors.overlay(Tokens.borderAlpha)
                    border.width: 1
                    color: interfacePicker.hovered ? Colors.surface1 : Colors.surface0
                    radius: Tokens.radiusS
                }

                Keys.onEscapePressed: {
                    if (popup.visible)
                        popup.close();
                    else
                        root.closeRequested();
                }
                onActivated: index => ResourceStats.networkInterface = model[index]
            }
            StatRow {
                label: "↓ 下载  " + (root.iface ? StatsFormat.speed(root.iface.downSpeed) : "")
                value: "0–" + StatsFormat.speed(root.downMax)
                valueColor: Colors.teal
                width: parent.width
            }
            TimeSeriesChart {
                Accessible.name: "下载速率，最近一分钟"
                lineColor: Colors.teal
                maxValue: root.downMax
                plotHeight: 62
                points: root.history.down
                width: parent.width
            }
            StatRow {
                label: "↑ 上传  " + (root.iface ? StatsFormat.speed(root.iface.upSpeed) : "")
                value: "0–" + StatsFormat.speed(root.upMax)
                valueColor: Colors.blue
                width: parent.width
            }
            TimeSeriesChart {
                Accessible.name: "上传速率，最近一分钟"
                lineColor: Colors.blue
                maxValue: root.upMax
                plotHeight: 62
                points: root.history.up
                width: parent.width
            }
            Divider {
                width: parent.width
            }
            StatRow {
                label: "累计下载"
                value: root.iface ? StatsFormat.bytes(root.iface.downTotal) : ""
                width: parent.width
            }
            StatRow {
                label: "累计上传"
                value: root.iface ? StatsFormat.bytes(root.iface.upTotal) : ""
                width: parent.width
            }
            StatRow {
                label: "IPv4"
                value: root.iface ? root.iface.ipv4 : ""
                visible: root.iface !== null && root.iface.ipv4 !== ""
                width: parent.width
            }
            StatRow {
                label: "网关"
                value: root.iface ? root.iface.gateway : ""
                visible: root.iface !== null && root.iface.gateway !== ""
                width: parent.width
            }
        }
    }
}
