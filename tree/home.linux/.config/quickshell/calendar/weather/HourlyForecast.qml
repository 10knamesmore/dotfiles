import "../../services"
import "../../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "WeatherFormat.js" as Format

// 温度曲线与降雨概率共用逐时时间轴；按钮提供每个数据点的读数和键盘入口。
WeatherSection {
    id: root

    required property var hours
    required property double now
    property real entranceProgress: 1
    property int offset: 0
    property int selectedIndex: 0
    property int nextOffset: 0
    readonly property var shownHours: hours.slice(offset, offset + 8)
    readonly property var selectedHour: shownHours[selectedIndex] || null
    readonly property var temperatures: hours.map((hour) => {
        return hour.temp;
    }).filter((value) => {
        return value !== null;
    })
    readonly property real minTemperature: temperatures.length ? Math.min.apply(Math, temperatures) - 2 : 0
    readonly property real maxTemperature: temperatures.length ? Math.max.apply(Math, temperatures) + 2 : 4
    readonly property color rainColor: Colors.sky

    function changePage(step) {
        nextOffset = offset + step;
        pageTransition.restart();
    }

    function temperatureY(value) {
        return 82 - (value - minTemperature) / (maxTemperature - minTemperature) * 54;
    }

    function hourDescription(hour) {
        return Format.timestamp(hour.time, now) + "，" + WeatherService.wmoDesc(hour.code) + "，温度 " + Format.temperature(hour.temp) + "，降雨概率 " + Format.number(hour.precipProb) + "%，降水量 " + Format.number(hour.precipitation, 1) + " 毫米";
    }

    title: "未来 24 小时"
    note: shownHours.length ? Format.timestamp(shownHours[0].time, now) + "—" + Format.timeOfDay(shownHours[shownHours.length - 1].time) : ""
    Accessible.role: Accessible.Grouping
    Accessible.name: "逐时天气预报"
    onHoursChanged: offset = Math.min(offset, Math.max(0, Math.floor((hours.length - 1) / 8) * 8))
    onShownHoursChanged: {
        selectedIndex = 0;
        plot.requestPaint();
    }
    onMinTemperatureChanged: plot.requestPaint()
    onMaxTemperatureChanged: plot.requestPaint()
    onRainColorChanged: plot.requestPaint()
    onEntranceProgressChanged: plot.requestPaint()
    actions: [
        WeatherButton {
            text: "󰅁"
            label: "前 8 小时"
            iconOnly: true
            enabled: root.offset > 0 && !pageTransition.running
            onClicked: root.changePage(-8)
        },
        WeatherButton {
            text: "󰅂"
            label: "后 8 小时"
            iconOnly: true
            enabled: root.offset + 8 < root.hours.length && !pageTransition.running
            onClicked: root.changePage(8)
        }
    ]

    SequentialAnimation {
        id: pageTransition

        NumberAnimation {
            target: hourlyVisual
            property: "opacity"
            to: 0
            duration: 70
        }

        ScriptAction {
            script: root.offset = root.nextOffset
        }

        NumberAnimation {
            target: hourlyVisual
            property: "opacity"
            to: 1
            duration: 170
            easing.type: Easing.OutCubic
        }

    }

    Item {
        id: hourlyVisual

        Layout.fillWidth: true
        implicitHeight: root.shownHours.length ? 148 : 48

        Text {
            anchors.centerIn: parent
            visible: !root.shownHours.length
            text: "逐时预报已过期，请刷新"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }

        Canvas {
            id: plot

            anchors.left: parent.left
            anchors.right: parent.right
            height: 104
            antialiasing: true
            Accessible.ignored: true
            onWidthChanged: requestPaint()
            onVisibleChanged: {
                if (visible)
                    requestPaint();

            }
            onPaint: {
                const ctx = getContext("2d");
                ctx.clearRect(0, 0, width, height);
                const points = root.shownHours;
                if (!points.length)
                    return ;

                const x = (index) => {
                    return (index + 0.5) * width / points.length;
                };
                ctx.save();
                ctx.beginPath();
                ctx.rect(0, 0, width * root.entranceProgress, height);
                ctx.clip();
                ctx.strokeStyle = Colors.overlay(0.06);
                ctx.lineWidth = 1;
                ctx.setLineDash([3, 4]);
                for (const y of [32, 64, 96]) {
                    ctx.beginPath();
                    ctx.moveTo(0, y);
                    ctx.lineTo(width, y);
                    ctx.stroke();
                }
                ctx.setLineDash([]);
                for (let i = 0; i < points.length; i++) {
                    if (points[i].precipProb === null)
                        continue;

                    const barHeight = points[i].precipProb / 100 * 42;
                    ctx.fillStyle = Colors.withAlpha(root.rainColor, 0.25);
                    ctx.fillRect(x(i) - 9, 98 - Math.max(1, barHeight), 18, Math.max(1, barHeight));
                }
                ctx.strokeStyle = Colors.peach;
                ctx.lineWidth = 2;
                ctx.lineJoin = "round";
                ctx.beginPath();
                let connected = false;
                for (let i = 0; i < points.length; i++) {
                    if (points[i].temp === null) {
                        connected = false;
                        continue;
                    }
                    const y = root.temperatureY(points[i].temp);
                    if (connected)
                        ctx.lineTo(x(i), y);
                    else
                        ctx.moveTo(x(i), y);
                    connected = true;
                }
                ctx.stroke();
                for (let i = 0; i < points.length; i++) {
                    if (points[i].temp === null)
                        continue;

                    ctx.beginPath();
                    ctx.arc(x(i), root.temperatureY(points[i].temp), 2.5, 0, Math.PI * 2);
                    ctx.fillStyle = Colors.surface0;
                    ctx.fill();
                    ctx.stroke();
                }
                ctx.restore();
            }
        }

        Row {
            anchors.fill: parent

            Repeater {
                model: root.shownHours

                delegate: Button {
                    id: hourButton

                    required property var modelData
                    required property int index

                    width: parent.width / root.shownHours.length
                    height: parent.height
                    padding: 0
                    hoverEnabled: true
                    Accessible.name: root.hourDescription(modelData)
                    Accessible.description: "查看此时刻的天气详情"
                    Accessible.checkable: true
                    Accessible.checked: root.selectedIndex === index
                    onClicked: root.selectedIndex = index
                    onHoveredChanged: {
                        if (hovered)
                            root.selectedIndex = index;

                    }
                    onActiveFocusChanged: {
                        if (activeFocus)
                            root.selectedIndex = index;

                    }

                    HoverHandler {
                        enabled: hourButton.enabled
                        cursorShape: Qt.PointingHandCursor
                    }

                    background: Rectangle {
                        radius: Tokens.radiusS
                        color: hourButton.hovered || hourButton.visualFocus ? Colors.overlay(0.04) : "transparent"
                        border.width: hourButton.visualFocus ? 1 : 0
                        border.color: Colors.mauve

                        Behavior on color {
                            ColorAnimation {
                                duration: Tokens.animFast
                            }

                        }

                    }

                    contentItem: Item {
                        Accessible.ignored: true

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            y: hourButton.modelData.temp === null ? 45 : root.temperatureY(hourButton.modelData.temp) - implicitHeight - 6
                            text: Format.temperature(hourButton.modelData.temp)
                            color: Colors.text
                            font.family: Fonts.family
                            font.pixelSize: Fonts.small
                        }

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            y: 109
                            text: Format.timeOfDay(hourButton.modelData.time)
                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            y: 129
                            text: Format.number(hourButton.modelData.precipProb) + "%"
                            color: hourButton.modelData.precipProb > 0 ? root.rainColor : Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.xs
                        }

                    }

                }

            }

        }

    }

    RowLayout {
        spacing: 6

        Rectangle {
            width: 12
            height: 2
            color: Colors.peach
            Accessible.ignored: true
        }

        Text {
            text: "温度"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
        }

        Rectangle {
            Layout.leftMargin: 8
            width: 12
            height: 5
            color: Colors.withAlpha(root.rainColor, 0.5)
            Accessible.ignored: true
        }

        Text {
            text: "降雨概率"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
        }

    }

    Text {
        Layout.fillWidth: true
        text: root.selectedHour ? Format.timestamp(root.selectedHour.time, root.now) + " · " + WeatherService.wmoDesc(root.selectedHour.code) + " · 降水 " + Format.number(root.selectedHour.precipitation, 1) + " mm · 风 " + Format.metersPerSecond(root.selectedHour.windSpeed) + " m/s" : "暂无逐时数据"
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        wrapMode: Text.Wrap
        Accessible.role: Accessible.StaticText
        Accessible.name: text
    }

}
