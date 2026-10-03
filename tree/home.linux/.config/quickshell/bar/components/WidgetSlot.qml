import "../../theme"
import QtQuick
import QtQuick.Layouts

// widget 槽位：分组项 {group:[...]} → 无背景的组容器；否则 → 单个 WidgetHost。
Loader {
    id: slot

    property var widgetItem: null
    property var barScreen: null
    property var barWindow: null

    readonly property bool isGroup: slot.widgetItem && slot.widgetItem.group !== undefined

    sourceComponent: slot.isGroup ? groupComp : singleComp

    Component {
        id: singleComp

        WidgetHost {
            item: BarLayout.normalize(slot.widgetItem)
            barScreen: slot.barScreen
            barWindow: slot.barWindow
        }
    }

    Component {
        id: groupComp

        Item {
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
