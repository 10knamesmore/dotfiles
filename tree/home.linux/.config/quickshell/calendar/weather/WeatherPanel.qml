import "../../services"
import "../../theme"
import QtQuick
import QtQuick.Layouts
import "WeatherFormat.js" as Format

// 当前天气、逐时趋势和七日预报；初次加载与保留旧数据时共用刷新入口。
ColumnLayout {
    id: root

    required property double now
    // 跟随外层面板的展开进度，反向变化时同步退场。
    property real entranceProgress: 1
    readonly property var weather: WeatherService.weather
    readonly property var upcomingHours: weather ? Format.upcomingHours(weather.hourly, now) : []
    readonly property var today: weather ? weather.daily.find((day) => {
        return day.date === Format.dateKey(now);
    }) || null : null
    readonly property bool stale: weather !== null && (now - weather.current.observedAt > 3.6e+06 || now - WeatherService.weatherUpdatedAt > 3.6e+06)
    readonly property color conditionColor: weather && weather.current.code >= 50 ? Colors.sky : weather && weather.current.isDay ? Colors.yellow : Colors.lavender

    function sectionProgress(start) {
        return Math.max(0, Math.min(1, (entranceProgress - start) / (1 - start)));
    }

    spacing: 10

    WeatherSection {
        Layout.fillWidth: true
        visible: root.weather === null
        title: "北京天气"

        Text {
            Layout.fillWidth: true
            Layout.topMargin: 18
            Layout.bottomMargin: 18
            text: WeatherService.weatherStatus === "loading" ? "正在获取天气…" : "暂时无法获取天气，请重试"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            horizontalAlignment: Text.AlignHCenter
        }

    }

    Loader {
        id: weatherLoader

        Layout.fillWidth: true
        active: root.weather !== null
        onLoaded: weatherArrival.restart()

        sourceComponent: ColumnLayout {
            spacing: 10

            WeatherSection {
                id: overview

                readonly property real reveal: root.sectionProgress(0.15)

                Layout.fillWidth: true
                opacity: reveal
                color: Colors.withAlpha(root.conditionColor, 0.05)
                border.color: Colors.withAlpha(root.conditionColor, 0.1)

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        Layout.fillWidth: true
                        text: "󰍎 " + WeatherService.cityName + " · " + (root.stale ? "上次天气" : "现在")
                        color: Colors.subtext1
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small
                    }

                    Text {
                        text: Format.timestamp(root.weather.current.observedAt, root.now) + " 观测"
                        color: Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.caption
                    }

                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    Text {
                        text: Format.temperature(root.weather.current.temp)
                        color: Colors.text
                        font.family: Fonts.family
                        font.pixelSize: Fonts.display1
                        font.weight: Fonts.weightLight
                    }

                    ColumnLayout {
                        spacing: 4

                        Text {
                            text: WeatherService.wmoDesc(root.weather.current.code)
                            color: root.conditionColor
                            font.family: Fonts.family
                            font.pixelSize: Fonts.bodyLarge
                        }

                        Text {
                            text: "体感 " + Format.temperature(root.weather.current.apparentTemp) + (root.today ? " · ↑" + Format.temperature(root.today.maxTemp) + " ↓" + Format.temperature(root.today.minTemp) : "")
                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                        }

                    }

                    Item {
                        Layout.fillWidth: true
                    }

                    Text {
                        text: WeatherService.wmoIcon(root.weather.current.code, root.weather.current.isDay)
                        color: root.conditionColor
                        font.family: Fonts.family
                        font.pixelSize: Fonts.display1
                        Accessible.ignored: true
                    }

                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: Colors.overlay(0.06)
                }

                Text {
                    Layout.fillWidth: true
                    text: "󰖌 " + Format.summary(root.upcomingHours)
                    color: Colors.subtext1
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    wrapMode: Text.Wrap
                }

                transform: Translate {
                    y: (1 - overview.reveal) * 14
                }

            }

            HourlyForecast {
                id: hourlyForecast

                Layout.fillWidth: true
                entranceProgress: root.sectionProgress(0.32)
                opacity: entranceProgress
                hours: root.upcomingHours
                now: root.now

                transform: Translate {
                    y: (1 - hourlyForecast.entranceProgress) * 18
                }

            }

            RowLayout {
                id: forecastDetails

                readonly property real reveal: root.sectionProgress(0.48)

                Layout.fillWidth: true
                spacing: 10
                opacity: reveal

                DailyForecast {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.alignment: Qt.AlignTop
                    days: root.weather.daily
                    now: root.now
                }

                EnvironmentCard {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.alignment: Qt.AlignTop
                    current: root.weather.current
                    currentHour: root.upcomingHours.length ? root.upcomingHours[0] : null
                    today: root.today
                    now: root.now
                }

                transform: Translate {
                    y: (1 - forecastDetails.reveal) * 22
                }

            }

        }

    }

    NumberAnimation {
        id: weatherArrival

        target: weatherLoader
        property: "opacity"
        from: 0
        to: 1
        duration: Tokens.animNormal
        easing.type: Easing.OutCubic
    }

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: statusRow.implicitHeight + 8
        opacity: root.sectionProgress(0.6)
        radius: Tokens.radiusS
        color: WeatherService.weatherStatus === "error" || root.stale ? Colors.withAlpha(Colors.yellow, 0.06) : "transparent"

        RowLayout {
            id: statusRow

            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 4
            spacing: 8

            Text {
                Layout.fillWidth: true
                text: {
                    if (WeatherService.weatherStatus === "loading")
                        return root.weather ? "更新中… · 保留上次数据" : "正在连接 Open-Meteo…";

                    if (WeatherService.weatherStatus === "error")
                        return root.weather ? "更新失败 · 保留上次数据 · " + Format.timestamp(WeatherService.weatherUpdatedAt, root.now) + " 更新" : "天气获取失败";

                    return (root.stale ? "数据已过期 · " : "") + Format.timestamp(WeatherService.weatherUpdatedAt, root.now) + " 更新 · Open-Meteo";
                }
                color: WeatherService.weatherStatus === "error" || root.stale ? Colors.yellow : Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                wrapMode: Text.Wrap
                Accessible.role: Accessible.StaticText
                Accessible.name: text
            }

            WeatherButton {
                text: WeatherService.refreshing ? "更新中" : WeatherService.weatherStatus === "error" ? "重试" : "刷新"
                glyph: "󰑓"
                busy: WeatherService.refreshing
                label: "刷新天气与空气质量"
                enabled: !WeatherService.refreshing
                onClicked: WeatherService.refresh()
            }

        }

    }

}
