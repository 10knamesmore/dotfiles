import "../bar/components"
import "../state"
import "../theme"
import "UsageFormat.js" as UsageFormat
import QtQuick

// 常驻 Codex 周剩余额度、DS 与 OpenAI 余额，点击展开详情。
BarModule {
    id: root

    readonly property var codex: AiUsageService.codex
    readonly property var deepseek: AiUsageService.deepseek
    readonly property var openai: AiUsageService.openai
    readonly property string openaiBalanceText: openai.status === "ok" ? "$" + openai.balanceUsd.toFixed(2) : UsageFormat.statusText(openai.status)
    readonly property string weeklyRemainingText: codex.status === "ok" ? Math.round(100 - codex.usedPercent) + "%" : UsageFormat.statusText(codex.status)
    readonly property string balanceText: {
        if (deepseek.status !== "ok")
            return UsageFormat.statusText(deepseek.status);
        return deepseek.balances.map(balance => {
            const symbol = balance.currency === "CNY" ? "¥" : balance.currency === "USD" ? "$" : balance.currency + " ";
            return symbol + balance.total.toFixed(2);
        }).join(" / ");
    }

    accentColor: Colors.lavender
    implicitWidth: summary.implicitWidth + horizontalPadding * 2
    activeFocusOnTab: true
    Accessible.role: Accessible.Button
    Accessible.name: "AI 额度与余额"
    Accessible.description: "Codex 本周剩余 " + weeklyRemainingText + "；DS 余额 " + balanceText + "；OpenAI 余额 " + openaiBalanceText
    Accessible.checkable: true
    Accessible.checked: PanelState.aiUsageOpen
    Accessible.onPressAction: PanelState.toggleAiUsage(root)
    Keys.onReturnPressed: PanelState.toggleAiUsage(root)
    Keys.onSpacePressed: PanelState.toggleAiUsage(root)
    onClicked: PanelState.toggleAiUsage(root)

    Row {
        id: summary

        anchors.verticalCenter: parent.verticalCenter
        spacing: 8

        Image {
            anchors.verticalCenter: parent.verticalCenter
            source: "icons/codex.svg"
            width: Fonts.iconLarge
            height: Fonts.iconLarge
            sourceSize.width: width
            sourceSize.height: height
            Accessible.ignored: true
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.weeklyRemainingText
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.DemiBold
        }
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: 1
            height: 12
            color: Colors.surface2
        }
        Image {
            anchors.verticalCenter: parent.verticalCenter
            source: "icons/deepseek.svg"
            width: Fonts.iconLarge
            height: Fonts.iconLarge
            sourceSize.width: width
            sourceSize.height: height
            Accessible.ignored: true
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.balanceText
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.DemiBold
            Accessible.name: "DeepSeek 余额 " + root.balanceText
        }
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: 1
            height: 12
            color: Colors.surface2
        }
        Image {
            anchors.verticalCenter: parent.verticalCenter
            source: "icons/openai.svg"
            width: Fonts.iconLarge
            height: Fonts.iconLarge
            sourceSize.width: width
            sourceSize.height: height
            Accessible.ignored: true
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.openaiBalanceText
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.DemiBold
            Accessible.name: "OpenAI 余额 " + root.openaiBalanceText
        }
    }
}
