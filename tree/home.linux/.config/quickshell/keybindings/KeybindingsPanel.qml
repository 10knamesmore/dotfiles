import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "../theme"
import "../state"
import "../components"

// 快捷键速查面板 — 数据来自 `hyprctl binds`：hyprland.lua 里每条 bind 的 description
// 约定写成「分组 · 标签」，分组标题取 " · " 前那段。
// 绑定由 Lua 闭包实现时 Hyprland 只报 dispatcher __lua，标签只能来自 description。
PanelOverlay {
    id: root

    showing: PanelState.keybindingsOpen
    entrance: PanelOverlay.Slide
    panelWidth: Math.min(700, root.width - 40)
    panelHeight: root.height * 0.8
    backdropOpacity: Tokens.backdropMedium
    onCloseRequested: PanelState.keybindingsOpen = false

    property string searchQuery: ""
    property var _sections: []   // [{title, bindings: [{key, desc}]}]

    ListModel { id: filteredModel }

    onShowingChanged: {
        if (showing) {
            searchQuery = "";
            loadKeybindings();
            focusTimer.start();
        }
    }

    Timer {
        id: focusTimer
        interval: 50
        onTriggered: searchInput.forceActiveFocus()
    }

    function loadKeybindings() {
        bindsProc.running = true;
    }

    Process {
        id: bindsProc
        command: ["hyprctl", "binds", "-j"]
        stdout: StdioCollector {
            onStreamFinished: root.parseBinds(text)
        }
    }

    // 只渲染带 description 的绑定（即 hyprland.lua keybindings 段声明的那些）。
    function parseBinds(json) {
        let raw = [];
        try { raw = JSON.parse(json); } catch (e) { raw = []; }

        let byGroup = new Map();
        for (let b of raw) {
            if (!b.has_description || !b.description)
                continue;
            let parts = b.description.split(" · ");
            let group = parts.length > 1 ? parts[0] : "通用";
            let label = parts.length > 1 ? parts.slice(1).join(" · ") : b.description;
            if (!byGroup.has(group))
                byGroup.set(group, []);
            byGroup.get(group).push({ key: formatKeyCombo(b.modmask, b.key), desc: label });
        }

        let sections = [];
        for (let [title, bindings] of byGroup)
            sections.push({ title: title, bindings: bindings });

        root._sections = sections;
        applyFilter();
    }

    // modmask 位（Hyprland）：1=Shift 4=Ctrl 8=Alt 64=Super
    function formatKeyCombo(modmask, key) {
        let parts = [];
        if (modmask & 64) parts.push("Super");
        if (modmask & 4) parts.push("Ctrl");
        if (modmask & 1) parts.push("Shift");
        if (modmask & 8) parts.push("Alt");

        let keyName = key;
        if (key === "mouse:272") keyName = "LMB";
        else if (key === "mouse:273") keyName = "RMB";
        else if (key === "mouse:274") keyName = "MMB";
        else if (key.startsWith("XF86")) keyName = key.replace("XF86", "").replace(/([A-Z])/g, " $1").trim();
        else if (key === "slash") keyName = "/";
        else if (key === "period") keyName = ".";
        else if (key === "apostrophe") keyName = "'";
        else if (key === "TAB") keyName = "Tab";
        else if (key.length === 1) keyName = key.toUpperCase();

        parts.push(keyName);
        return parts.join(" + ");
    }

    function applyFilter() {
        filteredModel.clear();
        let q = searchQuery.toLowerCase();
        for (let section of _sections) {
            let matchedBindings = [];
            for (let b of section.bindings) {
                if (q.length === 0 || b.key.toLowerCase().includes(q) || b.desc.toLowerCase().includes(q)) {
                    matchedBindings.push(b);
                }
            }
            if (matchedBindings.length > 0) {
                filteredModel.append({
                    sectionTitle: section.title,
                    bindingsJson: JSON.stringify(matchedBindings)
                });
            }
        }
    }

    // ── UI ──
    ColumnLayout {
        anchors.fill: parent; anchors.margins: 20; spacing: Tokens.spaceM

        // 标题 + 搜索
        RowLayout {
            Layout.fillWidth: true; spacing: Tokens.spaceM

            Text {
                text: "󰌌 快捷键速查"
                color: Colors.text
                font.family: Fonts.family; font.pixelSize: Fonts.heading
                font.weight: Font.Bold
            }
            Item { Layout.fillWidth: true }

            // 搜索框
            Rectangle {
                Layout.preferredWidth: 200; height: 32; radius: Tokens.radiusMS
                color: Colors.surface1

                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10; spacing: 6
                    Text { text: ""; color: Colors.overlay1; font.family: Fonts.family; font.pixelSize: Fonts.bodyLarge }
                    TextInput {
                        id: searchInput
                        Layout.fillWidth: true
                        color: Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.body
                        clip: true; selectByMouse: true
                        onTextChanged: { root.searchQuery = text; root.applyFilter() }
                        Keys.onEscapePressed: PanelState.keybindingsOpen = false
                        Text {
                            anchors.fill: parent; text: "搜索快捷键..."
                            color: Colors.overlay0; font: parent.font
                            visible: !parent.text && !parent.activeFocus
                        }
                    }
                }
            }
        }

        Rectangle { Layout.fillWidth: true; height: 1; color: Colors.surface1 }

        // 分组列表
        Flickable {
            Layout.fillWidth: true; Layout.fillHeight: true
            contentHeight: sectionsCol.implicitHeight
            clip: true

            ColumnLayout {
                id: sectionsCol
                width: parent.width
                spacing: Tokens.spaceM

                // 空状态
                Text {
                    visible: filteredModel.count === 0
                    text: "未找到匹配的快捷键"
                    color: Colors.overlay0
                    font.family: Fonts.family; font.pixelSize: Fonts.bodyLarge
                    Layout.alignment: Qt.AlignHCenter
                    Layout.topMargin: 40
                }

                Repeater {
                    model: filteredModel
                    delegate: Rectangle {
                        required property string sectionTitle
                        required property string bindingsJson

                        Layout.fillWidth: true
                        implicitHeight: cardCol.implicitHeight + 20
                        radius: Tokens.radiusMS
                        color: sectionHover.containsMouse ? Colors.surface1 : Colors.surface0
                        border.color: sectionHover.containsMouse ? Colors.withAlpha(Colors.blue, Tokens.borderHoverAlpha) : Colors.overlay(0.04)
                        border.width: 1
                        Behavior on color { ColorAnimation { duration: Tokens.animFast; easing.type: Easing.BezierSpline; easing.bezierCurve: Anim.standard } }
                        Behavior on border.color { ColorAnimation { duration: Tokens.animFast; easing.type: Easing.BezierSpline; easing.bezierCurve: Anim.standard } }

                        MouseArea {
                            id: sectionHover
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.NoButton
                        }

                        ColumnLayout {
                            id: cardCol
                            anchors.left: parent.left; anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 10
                            spacing: 6

                            // 分组标题
                            Text {
                                text: sectionTitle
                                color: Colors.blue
                                font.family: Fonts.family; font.pixelSize: Fonts.body
                                font.weight: Font.Bold
                            }

                            // 绑定列表
                            Repeater {
                                model: {
                                    try { return JSON.parse(bindingsJson); }
                                    catch(e) { return []; }
                                }
                                delegate: RowLayout {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    spacing: Tokens.spaceM

                                    // 按键 badge
                                    Row {
                                        spacing: Tokens.spaceXS
                                        Layout.preferredWidth: 200
                                        Layout.alignment: Qt.AlignTop

                                        Repeater {
                                            model: modelData.key.split(" + ")
                                            delegate: Rectangle {
                                                required property var modelData
                                                width: keyLabel.implicitWidth + 12
                                                height: 22
                                                radius: Tokens.radiusXS
                                                color: Colors.surface1
                                                border.color: Colors.overlay(0.08)
                                                border.width: 1

                                                Text {
                                                    id: keyLabel
                                                    anchors.centerIn: parent
                                                    text: modelData
                                                    color: Colors.text
                                                    font.family: Fonts.family; font.pixelSize: Fonts.caption
                                                    font.weight: Font.DemiBold
                                                }
                                            }
                                        }
                                    }

                                    // 描述
                                    Text {
                                        text: modelData.desc
                                        color: Colors.subtext0
                                        font.family: Fonts.family; font.pixelSize: Fonts.small
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
