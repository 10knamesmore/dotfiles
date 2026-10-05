import "../../services"
import "../../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "WeatherFormat.js" as Format

// 七日预报的温度条共享一套温标；逐日展开显示雨量、紫外线与日出日落。
WeatherSection {
    id: root

    required property var days
    required property double now
    property string expandedDate: ""
    readonly property var minimums: days.map((day) => {
        return day.minTemp;
    }).filter((value) => {
        return value !== null;
    })
    readonly property var maximums: days.map((day) => {
        return day.maxTemp;
    }).filter((value) => {
        return value !== null;
    })
    readonly property real minTemperature: minimums.length ? Math.min.apply(Math, minimums) : 0
    readonly property real temperatureRange: Math.max(1, (maximums.length ? Math.max.apply(Math, maximums) : 1) - minTemperature)

    title: "7 天预报"
    note: "点选查看详情"
    Accessible.role: Accessible.Grouping
    Accessible.name: title

    RowLayout {
        Layout.fillWidth: true
        spacing: 6

        Text {
            Layout.preferredWidth: 34
            text: "日期"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
        }

        Item {
            Layout.preferredWidth: 20
        }

        Text {
            Layout.preferredWidth: 32
            text: "降雨"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
        }

        Text {
            Layout.fillWidth: true
            text: "温度范围"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
        }

        Text {
            text: "°C"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
        }

    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 2

        Repeater {
            model: root.days

            delegate: ColumnLayout {
                id: dayRow

                required property var modelData
                readonly property bool expanded: root.expandedDate === modelData.date

                Layout.fillWidth: true
                spacing: 4

                Button {
                    id: dayButton

                    Layout.fillWidth: true
                    implicitHeight: 32
                    padding: 0
                    hoverEnabled: true
                    checkable: true
                    checked: dayRow.expanded
                    Accessible.name: Format.dayLabel(dayRow.modelData.date, root.now) + " " + dayRow.modelData.date + " 的天气预报"
                    Accessible.description: WeatherService.wmoDesc(dayRow.modelData.code) + "，" + Format.temperature(dayRow.modelData.minTemp) + " 至 " + Format.temperature(dayRow.modelData.maxTemp) + "，降雨概率 " + Format.number(dayRow.modelData.precipProb) + "%；" + (dayRow.expanded ? "已展开" : "已收起")
                    onClicked: root.expandedDate = dayRow.expanded ? "" : dayRow.modelData.date

                    HoverHandler {
                        enabled: dayButton.enabled
                        cursorShape: Qt.PointingHandCursor
                    }

                    background: Rectangle {
                        radius: Tokens.radiusS
                        color: dayButton.hovered || dayRow.expanded || dayButton.visualFocus ? Colors.overlay(0.05) : "transparent"
                        border.width: dayButton.visualFocus ? 1 : 0
                        border.color: Colors.mauve

                        Behavior on color {
                            ColorAnimation {
                                duration: Tokens.animFast
                            }

                        }

                    }

                    contentItem: RowLayout {
                        spacing: 6
                        Accessible.ignored: true

                        Text {
                            Layout.preferredWidth: 34
                            text: Format.dayLabel(dayRow.modelData.date, root.now)
                            color: dayRow.modelData.date === Format.dateKey(root.now) ? Colors.mauve : Colors.text
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }

                        Text {
                            Layout.preferredWidth: 20
                            text: WeatherService.wmoIcon(dayRow.modelData.code)
                            color: dayRow.modelData.code >= 50 ? Colors.sky : Colors.yellow
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Text {
                            Layout.preferredWidth: 32
                            text: Format.number(dayRow.modelData.precipProb) + "%"
                            color: dayRow.modelData.precipProb > 0 ? Colors.sky : Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.xs
                        }

                        Text {
                            Layout.preferredWidth: 25
                            text: Format.temperature(dayRow.modelData.minTemp)
                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                            horizontalAlignment: Text.AlignRight
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.minimumWidth: 30
                            height: 4
                            radius: 2
                            color: Colors.overlay(0.06)

                            Rectangle {
                                visible: dayRow.modelData.minTemp !== null && dayRow.modelData.maxTemp !== null
                                x: parent.width * (dayRow.modelData.minTemp - root.minTemperature) / root.temperatureRange
                                width: Math.max(2, parent.width * (dayRow.modelData.maxTemp - dayRow.modelData.minTemp) / root.temperatureRange)
                                height: parent.height
                                radius: 2

                                gradient: Gradient {
                                    orientation: Gradient.Horizontal

                                    GradientStop {
                                        position: 0
                                        color: Colors.lavender
                                    }

                                    GradientStop {
                                        position: 1
                                        color: Colors.peach
                                    }

                                }

                            }

                        }

                        Text {
                            Layout.preferredWidth: 25
                            text: Format.temperature(dayRow.modelData.maxTemp)
                            color: Colors.peach
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                            horizontalAlignment: Text.AlignRight
                        }

                    }

                }

                Rectangle {
                    Layout.fillWidth: true
                    visible: implicitHeight > 0
                    implicitHeight: dayRow.expanded ? details.implicitHeight + 16 : 0
                    opacity: dayRow.expanded ? 1 : 0
                    clip: true
                    radius: Tokens.radiusS
                    color: Colors.withAlpha(Colors.mauve, 0.06)

                    ColumnLayout {
                        id: details

                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 8
                        spacing: 5

                        Text {
                            Layout.fillWidth: true
                            text: dayRow.modelData.date.slice(5) + " · " + WeatherService.wmoDesc(dayRow.modelData.code) + " · 降水 " + Format.number(dayRow.modelData.precipitation, 1) + " mm"
                            color: Colors.subtext1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                            wrapMode: Text.Wrap
                        }

                        Text {
                            Layout.fillWidth: true
                            text: "日出 " + Format.timeOfDay(dayRow.modelData.sunrise) + " · 日落 " + Format.timeOfDay(dayRow.modelData.sunset)
                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                            wrapMode: Text.Wrap
                        }

                        Text {
                            text: "最高紫外线 " + Format.number(dayRow.modelData.uvMax, 1) + " · " + Format.uvLevel(dayRow.modelData.uvMax)
                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }

                    }

                    Behavior on implicitHeight {
                        NumberAnimation {
                            duration: Tokens.animNormal
                            easing.type: Easing.OutCubic
                        }

                    }

                    Behavior on opacity {
                        NumberAnimation {
                            duration: Tokens.animFast
                        }

                    }

                }

            }

        }

    }

}
