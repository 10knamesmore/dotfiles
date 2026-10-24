import "../components"
import "../services"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    required property var request
    readonly property string kind: request?.kind ?? ""
    readonly property bool needsInput: kind === "pin" || kind === "passkey"
    readonly property bool displayOnly: kind === "displayPin" || kind === "displayPasskey"
    readonly property string deviceName: BluetoothService.deviceFor(request?.device)?.name ?? ""
    readonly property bool validInput: kind === "passkey" ? /^\d{1,6}$/.test(codeInput.text) : codeInput.text.length > 0
    implicitHeight: contents.implicitHeight + Tokens.spaceM * 2
    radius: Tokens.radiusM
    color: Colors.withAlpha(Colors.yellow, 0.08)
    border.width: 1
    border.color: Colors.withAlpha(Colors.yellow, 0.3)

    onRequestChanged: {
        codeInput.text = "";
        if (needsInput)
            codeInput.forceActiveFocus();
    }

    ColumnLayout {
        id: contents
        anchors.fill: parent
        anchors.margins: Tokens.spaceM
        spacing: Tokens.spaceS

        Text {
            Layout.fillWidth: true
            text: root.deviceName
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.bold: true
            wrapMode: Text.Wrap
        }
        Text {
            Layout.fillWidth: true
            text: root.kind === "confirmation" ? "两端显示的配对码是否一致？"
                : root.needsInput ? (root.kind === "pin" ? "输入设备的 PIN 码" : "输入设备显示的数字配对码")
                : root.displayOnly ? "在设备上输入此配对码"
                : root.kind === "service" ? "允许设备使用此服务？" : "允许此设备配对？"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            wrapMode: Text.Wrap
        }
        Text {
            visible: !!root.request?.passkey
            text: root.request?.passkey ?? ""
            color: Colors.yellow
            font.family: Fonts.family
            font.pixelSize: Fonts.title
            font.letterSpacing: 4
        }
        Text {
            visible: root.kind === "displayPasskey"
            text: "已输入 " + (root.request?.entered ?? 0) + " 位"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }
        Text {
            visible: root.kind === "service"
            Layout.fillWidth: true
            text: root.request?.uuid ?? ""
            wrapMode: Text.WrapAnywhere
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }
        TextField {
            id: codeInput
            visible: root.needsInput
            Layout.fillWidth: true
            maximumLength: root.kind === "passkey" ? 6 : 16
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            selectByMouse: true
            Accessible.name: root.kind === "pin" ? "PIN 码" : "配对码"
            onAccepted: {
                if (root.validInput)
                    BluetoothService.respond(true, text);
            }
            background: Rectangle {
                color: Colors.surface0
                radius: Tokens.radiusS
                border.width: 1
                border.color: codeInput.activeFocus ? Colors.yellow : Colors.surface2
            }
        }
        RowLayout {
            Layout.fillWidth: true
            Item { Layout.fillWidth: true }
            ActionButton {
                text: root.displayOnly ? "取消配对" : "拒绝"
                onClicked: {
                    if (root.displayOnly)
                        BluetoothService.request("cancel", root.request.device);
                    else
                        BluetoothService.respond(false);
                }
            }
            ActionButton {
                visible: !root.displayOnly
                text: root.kind === "confirmation" ? "一致，配对" : root.needsInput ? "提交" : "允许"
                enabled: !root.needsInput || root.validInput
                emphasized: true
                accent: Colors.yellow
                onClicked: BluetoothService.respond(true, codeInput.text)
            }
        }
    }
}
