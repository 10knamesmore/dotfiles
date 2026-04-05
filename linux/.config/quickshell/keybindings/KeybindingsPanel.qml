import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "../theme"
import "../components"

// 快捷键速查面板 — 解析 keybindings.conf，分组卡片，可搜索
PanelOverlay {
    id: root

    showing: PanelState.keybindingsOpen
    panelWidth: Math.min(700, root.width - 40)
    panelHeight: root.height * 0.8
    backdropOpacity: Tokens.backdropMedium
    onCloseRequested: PanelState.keybindingsOpen = false

    property string searchQuery: ""
    property var _lines: []
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
        root._lines = [];
        confProc.running = true;
    }

    Process {
        id: confProc
        command: ["cat", Quickshell.env("HOME") + "/.config/hypr/keybindings.conf"]
        onStarted: root._lines = []
        stdout: SplitParser {
            onRead: data => root._lines.push(data)
        }
        onExited: root.parseAndFilter()
    }

    function parseAndFilter() {
        // 解析变量
        let vars = {};
        for (let line of _lines) {
            let vm = line.match(/^\$(\w+)\s*=\s*(.+)/);
            if (vm) vars["$" + vm[1]] = vm[2].trim();
        }

        // 解析分组和绑定
        let sections = [];
        let currentSection = { title: "通用", bindings: [] };

        for (let line of _lines) {
            line = line.trim();
            if (!line || line.match(/^\$\w+\s*=/)) continue;

            // 检测分组标题
            let sectionMatch = line.match(/^#{2,}\s*(.+?)\s*#{2,}$/);
            if (sectionMatch) {
                if (currentSection.bindings.length > 0)
                    sections.push(currentSection);
                currentSection = { title: sectionMatch[1].trim(), bindings: [] };
                continue;
            }
            // 单行注释作为子标题
            if (line.startsWith("#") && !line.startsWith("##")) {
                let comment = line.replace(/^#+\s*/, "").trim();
                if (comment.length > 0 && comment.length < 50 && !comment.startsWith("请参") && !comment.startsWith("exec")) {
                    // 如果当前分组已有绑定，开始新分组
                    if (currentSection.bindings.length > 0) {
                        sections.push(currentSection);
                        currentSection = { title: comment, bindings: [] };
                    } else {
                        currentSection.title = comment;
                    }
                }
                continue;
            }

            // 解析 bind 行
            let bindMatch = line.match(/^bind[eml]*\s*=\s*(.+)/);
            if (!bindMatch) continue;

            let parts = bindMatch[1].split(",").map(s => s.trim());
            if (parts.length < 3) continue;

            let mods = parts[0];
            let key = parts[1];
            let action = parts[2];
            let args = parts.slice(3).join(", ").trim();

            // 变量替换
            for (let [k, v] of Object.entries(vars)) {
                mods = mods.replace(k, v);
                args = args.replace(k, v);
            }

            // 格式化按键组合
            let keyCombo = formatKeyCombo(mods, key);

            // 格式化描述
            let desc = formatDescription(action, args);

            currentSection.bindings.push({ key: keyCombo, desc: desc });
        }
        if (currentSection.bindings.length > 0)
            sections.push(currentSection);

        root._sections = sections;
        applyFilter();
    }

    function formatKeyCombo(mods, key) {
        let parts = [];
        let m = mods.toUpperCase();
        if (m.includes("SUPER")) parts.push("Super");
        if (m.includes("CONTROL") || m.includes("CTRL")) parts.push("Ctrl");
        if (m.includes("SHIFT")) parts.push("Shift");
        if (m.includes("ALT")) parts.push("Alt");

        // 格式化特殊键名
        let keyName = key;
        if (key === "mouse:272") keyName = "LMB";
        else if (key === "mouse:273") keyName = "RMB";
        else if (key.startsWith("XF86")) keyName = key.replace("XF86", "").replace(/([A-Z])/g, " $1").trim();
        else if (key === "slash") keyName = "/";
        else if (key === "period") keyName = ".";
        else if (key === "TAB") keyName = "Tab";
        else keyName = key.toUpperCase();

        parts.push(keyName);
        return parts.join(" + ");
    }

    function formatDescription(action, args) {
        // 映射常见 dispatcher 到中文
        let map = {
            "killactive": "关闭窗口",
            "togglefloating": "切换浮动",
            "togglegroup": "切换分组",
            "changegroupactive": args === "f" ? "下一个标签" : "上一个标签",
            "movefocus": "切换焦点 " + ({l:"←",r:"→",u:"↑",d:"↓"}[args] || args),
            "workspace": "切换到工作区 " + args,
            "movetoworkspace": args === "special" ? "移到暂存区" : "移到工作区 " + args,
            "movetoworkspacesilent": "静默移到工作区 " + args,
            "togglespecialworkspace": "切换暂存区",
            "fullscreen": "全屏",
            "movewindow": "移动窗口",
            "resizewindow": "调整窗口大小",
            "layoutmsg": args === "togglesplit" ? "切换布局方向" : args,
        };

        if (map[action]) return map[action];

        // exec 命令解析
        if (action === "exec") {
            if (args.includes("hyprshot -m region")) return "区域截图";
            if (args.includes("hyprshot -m window")) return "窗口截图";
            if (args.includes("screen_record_toggle") && args.includes("region")) return "区域录屏";
            if (args.includes("screen_record_toggle")) return "录屏";
            if (args.includes("hyprlock")) return "锁屏";
            if (args.includes("toggle_fullscreen")) return "切换全屏";
            if (args.includes("toggleBar")) return "切换状态栏";
            if (args.includes("launcher")) return "应用启动器";
            if (args.includes("settings")) return "快捷设置";
            if (args.includes("keybindings")) return "快捷键速查";
            if (args.includes("launch_yazi")) return "文件管理器";
            if (args.includes("opacity_toggle")) return "透明度切换";
            if (args.includes("quick_note")) return "快速笔记";
            if (args.includes("focus_mode")) return "专注模式";
            if (args.includes("workspace_save")) return "保存工作区";
            if (args.includes("workspace_restore")) return "恢复工作区";
            if (args.includes("monitor_profile")) return "显示器切换";
            if (args.includes("layout_dispatch")) {
                let dir = args.includes(" h") ? "←" : args.includes(" l") ? "→" : args.includes(" k") ? "↑" : "↓";
                if (args.includes("shift")) return "移动窗口 " + dir;
                if (args.includes("ctrl")) return "调整大小 " + dir;
            }
            if (args.includes("wpctl set-volume") && args.includes("+")) return "音量+";
            if (args.includes("wpctl set-volume") && args.includes("-")) return "音量-";
            if (args.includes("wpctl set-mute") && args.includes("SINK")) return "静音切换";
            if (args.includes("wpctl set-mute") && args.includes("SOURCE")) return "麦克风静音";
            if (args.includes("brightnessctl") && args.includes("+")) return "亮度+";
            if (args.includes("brightnessctl") && args.includes("-")) return "亮度-";
            if (args.includes("playerctl next")) return "下一首";
            if (args.includes("playerctl play-pause")) return "播放/暂停";
            if (args.includes("playerctl previous")) return "上一首";
            if (args.includes("$terminal")) return "终端";
            // 通用 fallback
            let short = args.replace(/.*\//, "").replace(/\.sh$/, "");
            return short.length > 40 ? short.substring(0, 37) + "..." : short;
        }

        return action + (args ? " " + args : "");
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
                        border.color: sectionHover.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, Tokens.borderHoverAlpha) : Qt.rgba(1, 1, 1, 0.04)
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
                                                border.color: Qt.rgba(1, 1, 1, 0.08)
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
