import "../components"
import "../theme"
import "../network" as Net
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// 单个显示器的配置卡片
ColumnLayout {
    id: root

    property var monitor: null
    property var resolutions: []     // [{label, value}] 去重的分辨率列表
    property var refreshRates: []    // [{label, value}] 当前分辨率下的刷新率
    property string currentRes: ""
    property string currentRate: ""
    property real currentScale: 1.0
    property int currentTransform: 0
    property int posX: 0
    property int posY: 0
    property bool enabled: true

    signal applyRequested(var config)

    spacing: Tokens.spaceM

    // ── 解析 availableModes ──
    onMonitorChanged: {
        if (!monitor) return;
        currentRes = monitor.width + "x" + monitor.height;
        currentRate = Math.round(monitor.refreshRate * 100) / 100 + "";
        currentScale = monitor.scale || 1.0;
        currentTransform = monitor.transform || 0;
        posX = monitor.x || 0;
        posY = monitor.y || 0;
        enabled = !monitor.disabled;
        _parseAvailableModes();
    }

    function _parseAvailableModes() {
        if (!monitor || !monitor.availableModes) return;

        // 提取所有 分辨率+刷新率
        let resSet = {};
        let allModes = [];
        for (let mode of monitor.availableModes) {
            let m = mode.match(/^(\d+x\d+)@([\d.]+)Hz$/);
            if (m) {
                let res = m[1], rate = m[2];
                resSet[res] = true;
                allModes.push({ "res": res, "rate": rate });
            }
        }

        // 分辨率列表（去重，按面积降序）
        let resList = Object.keys(resSet);
        resList.sort((a, b) => {
            let [aw, ah] = a.split("x").map(Number);
            let [bw, bh] = b.split("x").map(Number);
            return (bw * bh) - (aw * ah);
        });
        root.resolutions = resList.map(r => ({ "label": r, "value": r }));

        // 更新当前分辨率下的刷新率
        _updateRates();
    }

    function _updateRates() {
        if (!monitor || !monitor.availableModes) return;
        let rates = [];
        for (let mode of monitor.availableModes) {
            let m = mode.match(/^(\d+x\d+)@([\d.]+)Hz$/);
            if (m && m[1] === root.currentRes) {
                rates.push(m[2]);
            }
        }
        // 降序排列
        rates.sort((a, b) => parseFloat(b) - parseFloat(a));
        root.refreshRates = rates.map(r => ({ "label": r + "Hz", "value": r }));
    }

    function buildConfig() {
        return {
            "name": monitor.name,
            "res": root.currentRes,
            "rate": root.currentRate,
            "x": root.posX,
            "y": root.posY,
            "scale": root.currentScale,
            "transform": root.currentTransform,
            "enabled": root.enabled
        };
    }

    // ── 显示器名称 ──
    RowLayout {
        Layout.fillWidth: true
        spacing: Tokens.spaceS

        Text {
            text: "󰍹"
            color: Colors.blue
            font.family: Fonts.family
            font.pixelSize: Fonts.title
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1

            Text {
                text: monitor ? monitor.name : ""
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.body
                font.weight: Font.DemiBold
            }

            Text {
                text: monitor ? (monitor.model || monitor.description || "") : ""
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
        }

        // 启用/禁用
        ToggleSwitch {
            checked: root.enabled
            onToggled: root.enabled = !root.enabled
        }
    }

    Divider { Layout.fillWidth: true }

    // ── 分辨率 ──
    Text {
        text: "分辨率"
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        Layout.leftMargin: 4
    }

    // 分辨率选择器（用 Flow 布局适应多分辨率）
    Flow {
        Layout.fillWidth: true
        spacing: 6

        Repeater {
            model: root.resolutions

            delegate: Rectangle {
                required property var modelData
                width: resText.implicitWidth + 16
                height: 30
                radius: Tokens.radiusS
                color: root.currentRes === modelData.value
                    ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15)
                    : (resHover.containsMouse ? Colors.surface1 : Colors.surface0)
                border.color: root.currentRes === modelData.value
                    ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, Tokens.borderHoverAlpha)
                    : "transparent"
                border.width: 1

                Text {
                    id: resText
                    anchors.centerIn: parent
                    text: modelData.label
                    color: root.currentRes === modelData.value ? Colors.blue : Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    Behavior on color { ColorAnimation { duration: 150 } }
                }

                MouseArea {
                    id: resHover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.currentRes = modelData.value;
                        root._updateRates();
                        // 自动选第一个刷新率
                        if (root.refreshRates.length > 0)
                            root.currentRate = root.refreshRates[0].value;
                    }
                }

                Behavior on color { ColorAnimation { duration: 150 } }
            }
        }
    }

    // ── 刷新率 ──
    Text {
        text: "刷新率"
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        Layout.leftMargin: 4
    }

    Flow {
        Layout.fillWidth: true
        spacing: 6

        Repeater {
            model: root.refreshRates

            delegate: Rectangle {
                required property var modelData
                width: rateText.implicitWidth + 16
                height: 30
                radius: Tokens.radiusS
                color: root.currentRate === modelData.value
                    ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15)
                    : (rateHover.containsMouse ? Colors.surface1 : Colors.surface0)
                border.color: root.currentRate === modelData.value
                    ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, Tokens.borderHoverAlpha)
                    : "transparent"
                border.width: 1

                Text {
                    id: rateText
                    anchors.centerIn: parent
                    text: modelData.label
                    color: root.currentRate === modelData.value ? Colors.blue : Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    Behavior on color { ColorAnimation { duration: 150 } }
                }

                MouseArea {
                    id: rateHover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.currentRate = modelData.value
                }

                Behavior on color { ColorAnimation { duration: 150 } }
            }
        }
    }

    // ── 缩放 ──
    Text {
        text: "缩放  " + (Math.round(root.currentScale * 100) / 100)
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        Layout.leftMargin: 4
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: Tokens.spaceS

        Text {
            text: "0.5"
            color: Colors.overlay0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
        }

        Slider {
            id: scaleSlider
            Layout.fillWidth: true
            from: 0.5
            to: 3.0
            stepSize: 0.25
            value: root.currentScale
            onMoved: root.currentScale = Math.round(value * 4) / 4

            background: Rectangle {
                x: scaleSlider.leftPadding
                y: scaleSlider.topPadding + scaleSlider.availableHeight / 2 - height / 2
                width: scaleSlider.availableWidth
                height: 6
                radius: 3
                color: Colors.surface1

                Rectangle {
                    width: scaleSlider.visualPosition * parent.width
                    height: parent.height
                    radius: 3
                    color: Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.6)
                }
            }

            handle: Rectangle {
                x: scaleSlider.leftPadding + scaleSlider.visualPosition * (scaleSlider.availableWidth - width)
                y: scaleSlider.topPadding + scaleSlider.availableHeight / 2 - height / 2
                width: 18
                height: 18
                radius: 9
                color: Colors.blue
                border.color: Qt.rgba(1, 1, 1, 0.2)
                border.width: 1
            }
        }

        Text {
            text: "3.0"
            color: Colors.overlay0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
        }
    }

    // ── 旋转 ──
    Text {
        text: "旋转"
        color: Colors.subtext0
        font.family: Fonts.family
        font.pixelSize: Fonts.caption
        Layout.leftMargin: 4
    }

    Net.OptionRow {
        Layout.fillWidth: true
        compact: true
        model: [
            { "label": "正常", "value": "0" },
            { "label": "90°", "value": "1" },
            { "label": "180°", "value": "2" },
            { "label": "270°", "value": "3" }
        ]
        current: String(root.currentTransform)
        onSelected: (value) => root.currentTransform = parseInt(value)
    }

    // ── 位置 ──
    RowLayout {
        Layout.fillWidth: true
        spacing: Tokens.spaceS

        Net.EditField {
            label: "位置 X"
            placeholder: "0"
            text: String(root.posX)
            Layout.fillWidth: true
            onEdited: (t) => { let v = parseInt(t); if (!isNaN(v)) root.posX = v; }
        }

        Net.EditField {
            label: "位置 Y"
            placeholder: "0"
            text: String(root.posY)
            Layout.fillWidth: true
            onEdited: (t) => { let v = parseInt(t); if (!isNaN(v)) root.posY = v; }
        }
    }

    // ── 应用按钮 ──
    Rectangle {
        Layout.fillWidth: true
        height: 36
        radius: Tokens.radiusS
        color: applyArea.containsMouse
            ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.25)
            : Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15)

        Text {
            anchors.centerIn: parent
            text: "应用"
            color: Colors.blue
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            font.weight: Font.DemiBold
        }

        MouseArea {
            id: applyArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: root.applyRequested(root.buildConfig())
        }

        Behavior on color { ColorAnimation { duration: 150 } }
    }
}
