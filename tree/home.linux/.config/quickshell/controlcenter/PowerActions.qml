import "../theme"
import QtQuick
import QtQuick.Layouts

// 五个电源动作固定在面板底部，不随任务分区或详情页切换。
Item {
    implicitHeight: 68

    Divider {
        anchors.top: parent.top
        width: parent.width
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Tokens.spaceL
        anchors.rightMargin: Tokens.spaceL
        anchors.topMargin: 10
        anchors.bottomMargin: 10
        spacing: Tokens.spaceS

        PowerButton {
            icon: "󰌾"
            label: "锁屏"
            command: "hyprlock"
        }
        PowerButton {
            icon: "󰍃"
            label: "注销"
            command: "hyprctl dispatch 'hl.dsp.exit()'"
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
