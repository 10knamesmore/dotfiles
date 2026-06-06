import "../../theme"
import "../../state"
import QtQuick
import QtQuick.Layouts
import Quickshell

// 时钟��块 — 渐变背景（blue→sapphire→sky），独立组件不继承 BarModule
Rectangle {
    id: root

    property bool showDate: false
    property bool hovered: clockHover.containsMouse
    property bool flat: false

    implicitWidth: col.implicitWidth + 36
    implicitHeight: 36

    Behavior on implicitWidth {
        NumberAnimation {
            duration: Tokens.animSlow
            easing.type: Easing.OutCubic
        }
    }
    radius: 20

    // Hover 渐变叠加 sapphire → sky → teal
    Rectangle {
        anchors.fill: parent
        radius: parent.radius
        opacity: root.hovered ? 1 : 0
        visible: !root.flat

        gradient: Gradient {
            orientation: Gradient.Horizontal

            GradientStop {
                position: 0
                color: Colors.sapphire
            }

            GradientStop {
                position: 0.5
                color: Colors.sky
            }

            GradientStop {
                position: 1
                color: Colors.teal
            }
        }

        Behavior on opacity {
            NumberAnimation {
                duration: 300
                easing.type: Easing.OutCubic
            }
        }
    }

    // 柔和蓝色阴影
    SoftShadow {
        anchors.fill: parent
        radius: root.radius
        shadowColor: Colors.blue
        strength: root.hovered ? 0.35 : 0.25
        visible: !root.flat
    }

    SystemClock {
        id: clock

        precision: SystemClock.Seconds
    }

    ColumnLayout {
        id: col

        anchors.centerIn: parent
        spacing: 0

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: Qt.formatTime(clock.date, "HH:mm:ss") + " "
            color: root.flat ? Colors.text : Colors.base
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.ExtraBold
            visible: !root.showDate && !root.hovered
        }

        // hover 时显示时间 + 日期
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: Qt.formatTime(clock.date, "HH:mm") + "  " + Qt.formatDate(clock.date, "MM/dd ddd")
            color: root.flat ? Colors.text : Colors.base
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.ExtraBold
            visible: !root.showDate && root.hovered
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: Qt.formatDate(clock.date, "dddd, MMMM d, yyyy") + " 󰃰"
            color: root.flat ? Colors.text : Colors.base
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.ExtraBold
            visible: root.showDate
        }
    }

    MouseArea {
        id: clockHover

        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        cursorShape: Qt.PointingHandCursor
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton) {
                root.showDate = !root.showDate;
            } else {
                PanelState.closeAll();
                let pos = root.mapToItem(null, mouse.x, mouse.y);
                MorphState.morphSourceX = pos.x + 2;
                MorphState.morphSourceY = pos.y + 6;
                PanelState.toggleCalendar();
            }
        }
    }

    // 默认渐变 blue → sapphire → sky（flat 模式回退淡背景）
    color: root.flat ? Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, root.hovered ? 0.85 : 0.5) : "transparent"
    gradient: root.flat ? null : clockGrad

    property Gradient clockGrad: Gradient {
        orientation: Gradient.Horizontal

        GradientStop {
            position: 0
            color: Colors.blue
        }

        GradientStop {
            position: 0.5
            color: Colors.sapphire
        }

        GradientStop {
            position: 1
            color: Colors.sky
        }
    }
}
