import "../components"
import "../state"
import "../theme"
import "UsageFormat.js" as UsageFormat
import QtQuick
import Quickshell

// 从额度模块展开的详情面板；倒计时由本地时钟每分钟更新，不触发额外请求。
PanelOverlay {
    id: root

    readonly property var codex: AiUsageService.codex
    readonly property var deepseek: AiUsageService.deepseek

    showing: PanelState.aiUsageOpen
    panelWidth: 400
    panelHeight: content.implicitHeight + Tokens.spaceL * 2
    onCloseRequested: PanelState.aiUsageOpen = false

    SystemClock {
        id: clock

        enabled: root.showing
        precision: SystemClock.Minutes
    }

    Column {
        id: content

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceM
        Accessible.role: Accessible.Grouping
        Accessible.name: "Codex week 额度详情"

        Text {
            width: parent.width
            text: root.codex.status === "ok" ? UsageFormat.resetIn(root.codex.resetAt, clock.date.getTime()) : UsageFormat.statusDetails("codex", root.codex.status)
            color: root.codex.status === "ok" ? Colors.lavender : Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.heading
            font.weight: Font.DemiBold
            wrapMode: Text.WordWrap
        }
        Text {
            width: parent.width
            visible: root.codex.status === "ok"
            text: root.codex.status === "ok" ? "重置于 " + Qt.formatDateTime(new Date(root.codex.resetAt * 1000), "MM/dd ddd HH:mm") : ""
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.body
        }
        Text {
            width: parent.width
            visible: root.deepseek.status !== "ok"
            text: "DS：" + UsageFormat.statusDetails("deepseek", root.deepseek.status)
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            wrapMode: Text.WordWrap
        }
    }
}
