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
    // 已证实这块屏吃不下 HDR（由 MonitorService 应用后回读得出），开关置灰
    property bool hdrUnsupported: false

    signal modeEdited(string mode)
    signal scaleEdited(real scale)
    signal transformEdited(int transform)
    signal primaryToggled()
    signal enabledToggled()
    signal hdrToggled()
    signal sdrMaxLuminanceEdited(real v)
    signal sdrBrightnessEdited(real v)
    signal sdrSaturationEdited(real v)

    readonly property var _color: MM.colorOf(monitor)

    // 面板内统一的紧凑滑块。SettingsSlider 是带 icon 的大号样式（22px handle、
    // from/to 写死 0..1），风格和量程都不合，故本面板自持一套。
    component CompactSlider: Slider {
        id: cs

        // background / handle 必须给 implicitWidth/implicitHeight，只给 width/height 不够：
        // Slider 的 implicitHeight = max(implicitBackgroundHeight, implicitHandleHeight + padding)
        // （见 QtQuick/Controls/Basic/Slider.qml），implicit 全为 0 时整个 Slider 塌成 0 高。
        // 那时轨道因为有显式 height 仍然画得出来——看得见、但命中区域为零，拖不动。
        padding: 6

        // 只负责改光标：acceptedButtons: NoButton 让点击/拖拽照常落到 Slider 上，
        // 否则这层会吃掉按下事件，滑块又变成拖不动。同 SettingsSlider 的做法。
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            acceptedButtons: Qt.NoButton
        }

        background: Rectangle {
            x: cs.leftPadding
            y: cs.topPadding + cs.availableHeight / 2 - height / 2
            implicitWidth: 200
            implicitHeight: 4
            width: cs.availableWidth
            height: implicitHeight
            radius: height / 2
            color: Colors.surface1
            Rectangle {
                width: cs.visualPosition * parent.width
                height: parent.height
                radius: parent.radius
                color: cs.enabled ? Colors.blue : Colors.overlay0
            }
        }
        handle: Rectangle {
            x: cs.leftPadding + cs.visualPosition * (cs.availableWidth - width)
            y: cs.topPadding + cs.availableHeight / 2 - height / 2
            implicitWidth: 14
            implicitHeight: 14
            radius: width / 2
            color: cs.enabled ? Colors.blue : Colors.overlay0
            border.width: 2
            border.color: Colors.base
        }
    }

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
    CompactSlider {
        Layout.fillWidth: true
        enabled: root.monitor && root.monitor.enabled
        from: 0.5
        to: 3.0
        stepSize: 0.25
        value: root.monitor ? root.monitor.scale : 1.0
        onMoved: root.scaleEdited(value)
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

    // ── HDR ──
    // Hyprland 不暴露 supportsHDR，硬件不吃时只会把 cm 悄悄退回 srgb，所以
    // hdrUnsupported 是「应用过一次才知道」的后验结果，不是开机就有的能力表。
    Rectangle {
        Layout.fillWidth: true
        Layout.topMargin: Tokens.spaceS
        implicitHeight: hdrCol.implicitHeight + Tokens.spaceS * 2
        radius: Tokens.radiusS
        color: Colors.withAlpha(Colors.surface0, 0.5)

        ColumnLayout {
            id: hdrCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.margins: Tokens.spaceS
            spacing: Tokens.spaceXS

            RowLayout {
                Layout.fillWidth: true
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    Text { text: "HDR"; color: Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.small }
                    Text {
                        visible: root.hdrUnsupported
                        text: "此显示器不支持"
                        color: Colors.overlay0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.xs
                    }
                }
                ToggleSwitch {
                    small: true
                    enabled: root.monitor && root.monitor.enabled && !root.hdrUnsupported
                    opacity: enabled ? 1 : 0.4
                    checked: root.monitor && MM.isHdr(root.monitor)
                    onToggled: root.hdrToggled()
                }
            }

            // SDR 补偿：只在 SDR↔HDR 转换时进着色器（Hyprland SH_FEAT_SDR_MOD），srgb 下隐藏。
            //
            // 白点排在最前面且是主旋钮：HDR 下桌面发暗的根因就是它——SDR 内容默认只映射到
            // 80 nits（sRGB 参考白），而 HDR 屏峰值有几百 nits。下面两个是 PQ 域的微调，
            // 别拿它们当亮度用（拉大会压平对比、抬高黑位）。
            Text {
                visible: root.monitor && MM.isHdr(root.monitor)
                text: "SDR 白点 · " + Math.round(root._color.sdrMaxLuminance) + " nits"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                font.letterSpacing: 1
            }
            CompactSlider {
                Layout.fillWidth: true
                visible: root.monitor && MM.isHdr(root.monitor)
                from: 80
                to: 600
                stepSize: 10
                value: root._color.sdrMaxLuminance
                onMoved: root.sdrMaxLuminanceEdited(value)
            }

            Text {
                visible: root.monitor && MM.isHdr(root.monitor)
                text: "SDR 亮度 · " + Number(root._color.sdrBrightness).toFixed(2) + "×"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                font.letterSpacing: 1
            }
            CompactSlider {
                Layout.fillWidth: true
                visible: root.monitor && MM.isHdr(root.monitor)
                from: 1.0
                to: 2.0
                stepSize: 0.05
                value: root._color.sdrBrightness
                onMoved: root.sdrBrightnessEdited(value)
            }

            Text {
                visible: root.monitor && MM.isHdr(root.monitor)
                text: "SDR 饱和 · " + Number(root._color.sdrSaturation).toFixed(2) + "×"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.xs
                font.letterSpacing: 1
            }
            CompactSlider {
                Layout.fillWidth: true
                visible: root.monitor && MM.isHdr(root.monitor)
                from: 0.5
                to: 1.5
                stepSize: 0.05
                value: root._color.sdrSaturation
                onMoved: root.sdrSaturationEdited(value)
            }
        }
    }
}
