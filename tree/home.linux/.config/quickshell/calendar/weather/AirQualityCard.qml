import "../../services"
import "../../theme"
import QtQuick
import QtQuick.Layouts
import "WeatherFormat.js" as Format

// 空气质量使用 US AQI，独立展示采样时间及更新状态。
WeatherSection {
    id: root

    required property double now
    readonly property var reading: WeatherService.airQuality
    readonly property int level: Format.aqiLevel(reading ? reading.usAqi : null)
    readonly property var levelColors: [Colors.green, Colors.yellow, Colors.peach, Colors.red, Colors.mauve, Colors.maroon]
    readonly property color levelColor: level < 0 ? Colors.subtext0 : levelColors[level]

    title: "空气质量"
    note: WeatherService.cityName

    RowLayout {
        Layout.fillWidth: true
        visible: root.reading !== null

        Text {
            Layout.fillWidth: true
            text: "US AQI"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
        }

        Rectangle {
            implicitWidth: categoryText.implicitWidth + 12
            implicitHeight: categoryText.implicitHeight + 8
            radius: Tokens.radiusS
            color: Colors.withAlpha(root.levelColor, 0.12)

            Text {
                id: categoryText

                anchors.centerIn: parent
                text: WeatherService.aqiDesc(root.reading ? root.reading.usAqi : null)
                color: root.levelColor
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
            }

        }

    }

    RowLayout {
        Layout.fillWidth: true
        visible: root.reading !== null

        Text {
            text: Format.number(root.reading ? root.reading.usAqi : null)
            color: root.levelColor
            font.family: Fonts.family
            font.pixelSize: Fonts.display3
            font.weight: Fonts.weightLight
        }

        Item {
            Layout.fillWidth: true
        }

        Text {
            text: "PM2.5 " + Format.number(root.reading ? root.reading.pm25 : null, 1) + " μg/m³"
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
        }

    }

    RowLayout {
        Layout.fillWidth: true
        visible: root.reading !== null
        spacing: 3
        Accessible.ignored: true

        Repeater {
            model: root.levelColors

            delegate: Rectangle {
                required property color modelData
                required property int index

                Layout.fillWidth: true
                height: 4
                radius: 2
                color: modelData
                opacity: index === root.level ? 1 : 0.25
            }

        }

    }

    Text {
        Layout.fillWidth: true
        text: {
            const status = WeatherService.airQualityStatus;
            if (!root.reading)
                return status === "loading" ? "正在获取空气质量…" : "空气质量暂不可用";

            return Format.timestamp(root.reading.observedAt, root.now) + " 数据" + (status === "loading" ? " · 更新中…" : status === "error" ? " · 更新失败，保留上次数据" : root.now - WeatherService.airQualityUpdatedAt > 3.6e+06 || root.now - root.reading.observedAt > 7.2e+06 ? " · 数据已过期" : "");
        }
        color: WeatherService.airQualityStatus === "error" ? Colors.yellow : Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        wrapMode: Text.Wrap
        Accessible.role: Accessible.StaticText
        Accessible.name: text
    }

    WeatherButton {
        Layout.alignment: Qt.AlignRight
        visible: WeatherService.airQualityStatus === "error"
        text: "󰑓 重试"
        label: "重试更新空气质量"
        enabled: !WeatherService.refreshing
        onClicked: WeatherService.refreshIfStale()
    }

}
