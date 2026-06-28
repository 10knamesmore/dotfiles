import "../components"
import "../theme"
import "lib/monitorModel.js" as MM
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// 选中显示器的参数控件（B 布局右栏）。自身不改 draft，只上抛信号给 DisplayPanel。
ColumnLayout {
    id: root

    property var monitor: null   // 选中的 draft 条目

    // 注意：信号名不能撞 Item 既有属性的隐式变更信号（scale→scaleChanged、transform→transformChanged），
    // 否则 QML 报 Duplicate signal name。故用 *Edited 命名。
    signal modeEdited(string mode)
    signal scaleEdited(real scale)
    signal transformEdited(int transform)
    signal primaryToggled()
    signal enabledToggled()

    readonly property var _resGroups: monitor ? MM.modesByResolution(monitor.availableModes) : []
    readonly property string _curRes: monitor ? String(monitor.mode).split("@")[0] : ""
    readonly property string _curRate: monitor ? String(monitor.mode).split("@")[1] : ""
    readonly property var _curRates: {
        for (var i = 0; i < _resGroups.length; i++)
            if (_resGroups[i].res === _curRes)
                return _resGroups[i].rates;
        return [];
    }

    spacing: Tokens.spaceS

    // ── 头部：名称 + 描述 ──
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 2
        Text {
            text: root.monitor ? root.monitor.name : ""
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.bold: true
        }
        Text {
            Layout.fillWidth: true
            visible: root.monitor && root.monitor.description
            text: root.monitor ? root.monitor.description : ""
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
            elide: Text.ElideRight
        }
    }

    // ── 分辨率 ──
    Text { text: "分辨率"; color: Colors.overlay0; font.family: Fonts.family; font.pixelSize: Fonts.xs; font.letterSpacing: 1; Layout.topMargin: Tokens.spaceS }
    Dropdown {
        Layout.fillWidth: true
        enabled: root.monitor && root.monitor.enabled
        model: root._resGroups.map(function (g) { return g.res; })
        currentText: root._curRes
        onActivated: (value) => {
            // 换分辨率时，取该分辨率下最高刷新率
            for (var i = 0; i < root._resGroups.length; i++) {
                if (root._resGroups[i].res === value) {
                    root.modeEdited(value + "@" + root._resGroups[i].rates[0]);
                    return;
                }
            }
        }
    }

    // ── 刷新率 ──
    Text { text: "刷新率"; color: Colors.overlay0; font.family: Fonts.family; font.pixelSize: Fonts.xs; font.letterSpacing: 1; Layout.topMargin: Tokens.spaceXS }
    Dropdown {
        Layout.fillWidth: true
        enabled: root.monitor && root.monitor.enabled
        model: root._curRates.map(function (r) { return r + " Hz"; })
        currentText: root._curRate ? (root._curRate + " Hz") : ""
        onActivated: (value) => root.modeEdited(root._curRes + "@" + value.replace(" Hz", ""))
    }

    // ── 缩放 ──
    Text {
        text: "缩放 · " + (root.monitor ? Number(root.monitor.scale).toFixed(2) : "1.00") + "×"
        color: Colors.overlay0
        font.family: Fonts.family
        font.pixelSize: Fonts.xs
        font.letterSpacing: 1
        Layout.topMargin: Tokens.spaceXS
    }
    Slider {
        id: scaleSlider
        Layout.fillWidth: true
        enabled: root.monitor && root.monitor.enabled
        from: 0.5
        to: 3.0
        stepSize: 0.25
        value: root.monitor ? root.monitor.scale : 1.0
        onMoved: root.scaleEdited(value)

        background: Rectangle {
            x: scaleSlider.leftPadding
            y: scaleSlider.topPadding + scaleSlider.availableHeight / 2 - height / 2
            width: scaleSlider.availableWidth
            height: 4
            radius: 2
            color: Colors.surface1
            Rectangle {
                width: scaleSlider.visualPosition * parent.width
                height: parent.height
                radius: 2
                color: Colors.blue
            }
        }
        handle: Rectangle {
            x: scaleSlider.leftPadding + scaleSlider.visualPosition * (scaleSlider.availableWidth - width)
            y: scaleSlider.topPadding + scaleSlider.availableHeight / 2 - height / 2
            width: 14
            height: 14
            radius: 7
            color: Colors.blue
            border.width: 2
            border.color: Colors.base
        }
    }

    // ── 旋转 ──
    Text { text: "旋转"; color: Colors.overlay0; font.family: Fonts.family; font.pixelSize: Fonts.xs; font.letterSpacing: 1; Layout.topMargin: Tokens.spaceXS }
    RowLayout {
        Layout.fillWidth: true
        spacing: Tokens.spaceXS
        Repeater {
            model: [{ "l": "0°", "v": 0 }, { "l": "90°", "v": 1 }, { "l": "180°", "v": 2 }, { "l": "270°", "v": 3 }]
            delegate: Rectangle {
                Layout.fillWidth: true
                implicitHeight: 28
                radius: Tokens.radiusS
                property bool sel: root.monitor && root.monitor.transform === modelData.v
                color: sel ? Colors.blue : (segArea.containsMouse ? Colors.surface1 : Colors.surface0)
                Text {
                    anchors.centerIn: parent
                    text: modelData.l
                    color: parent.sel ? Colors.base : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                    font.bold: parent.sel
                }
                MouseArea {
                    id: segArea
                    anchors.fill: parent
                    hoverEnabled: true
                    enabled: root.monitor && root.monitor.enabled
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.transformEdited(modelData.v)
                }
                Behavior on color { ColorAnimation { duration: Tokens.animFast } }
            }
        }
    }

    // ── 主屏 / 启用 开关 ──
    RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: Tokens.spaceS
        Text { text: "设为主显示器"; Layout.fillWidth: true; color: Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.small }
        ToggleSwitch {
            small: true
            checked: root.monitor && root.monitor.primary
            onToggled: root.primaryToggled()
        }
    }
    RowLayout {
        Layout.fillWidth: true
        Text { text: "启用此显示器"; Layout.fillWidth: true; color: Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.small }
        ToggleSwitch {
            small: true
            checked: root.monitor && root.monitor.enabled
            onToggled: root.enabledToggled()
        }
    }
}
