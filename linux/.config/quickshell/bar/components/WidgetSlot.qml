import "../../theme"
import QtQuick
import QtQuick.Layouts

// widget 槽位：分组项 {group:[...]} → 组容器药丸（深凹槽，组内 flat 模块）；否则 → 单个 WidgetHost。
Loader {
    id: slot

    property var widgetItem: null
    property var barScreen: null
    property var barWindow: null
    property bool flat: false

    readonly property bool isGroup: slot.widgetItem && slot.widgetItem.group !== undefined

    sourceComponent: slot.isGroup ? groupComp : singleComp

    Component {
        id: singleComp

        WidgetHost {
            item: BarLayout.normalize(slot.widgetItem)
            flat: slot.flat
            barScreen: slot.barScreen
            barWindow: slot.barWindow
        }
    }

    Component {
        id: groupComp

        Rectangle {
            radius: Tokens.radiusL
            color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, 0.6)
            implicitWidth: groupRow.implicitWidth + 12
            implicitHeight: 36

            RowLayout {
                id: groupRow

                anchors.centerIn: parent
                spacing: 3

                Repeater {
                    model: slot.widgetItem.group

                    delegate: WidgetHost {
                        required property var modelData

                        item: BarLayout.normalize(modelData)
                        flat: true
                        barScreen: slot.barScreen
                        barWindow: slot.barWindow
                        Layout.preferredWidth: implicitWidth
                        Layout.preferredHeight: implicitHeight
                    }
                }
            }
        }
    }
}
