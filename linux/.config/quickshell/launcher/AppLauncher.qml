import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// App Launcher — 全屏居中，fuzzy search 启动应用
PanelWindow {
    id: root

    // 双阶段可见性
    property bool showing: PanelState.launcherOpen
    property bool animating: _opacityAnim.running || _scaleAnim.running
    // 搜索状态
    property string query: ""
    property int selectedIndex: 0
    // 缓存全部应用（非 noDisplay），启动时填充一次
    property var allApps: []
    property bool appsLoaded: false
    // 存储排序后的结果引用，delegate 通过 appIndex 索引取
    property var _results: []

    function ensureAppsLoaded() {
        if (appsLoaded)
            return ;

        let apps = DesktopEntries.applications.values;
        if (!apps || apps.length === 0)
            return ;

        let list = [];
        for (let i = 0; i < apps.length; i++) {
            if (!apps[i].noDisplay)
                list.push(apps[i]);

        }
        allApps = list;
        appsLoaded = true;
    }

    // ── Fuzzy Match ──
    function fuzzyScore(q, text) {
        if (q.length === 0)
            return 1;

        let lower = text.toLowerCase();
        let ql = q.toLowerCase();
        let qi = 0, score = 0, consecutive = 0;
        // 完全前缀匹配加大分
        if (lower.startsWith(ql))
            return 1000 + (100 - text.length);

        for (let i = 0; i < lower.length && qi < ql.length; i++) {
            if (lower[i] === ql[qi]) {
                qi++;
                consecutive++;
                score += consecutive * 10;
                // 词首字母加分
                if (i === 0 || text[i - 1] === ' ' || text[i - 1] === '-' || text[i - 1] === '.')
                    score += 50;

            } else {
                consecutive = 0;
            }
        }
        return qi === ql.length ? score : 0;
    }

    function updateFilter() {
        let apps = root.allApps;
        let q = root.query.trim();
        let results = [];
        for (let i = 0; i < apps.length; i++) {
            let app = apps[i];
            let idScore = fuzzyScore(q, app.id || "") * 0.9;
            let nameScore = fuzzyScore(q, app.name || "");
            let genericScore = fuzzyScore(q, app.genericName || "") * 0.8;
            let commentScore = fuzzyScore(q, app.comment || "") * 0.5;
            let keywordScore = 0;
            let kw = app.keywords || [];
            for (let k = 0; k < kw.length; k++) {
                keywordScore = Math.max(keywordScore, fuzzyScore(q, kw[k]) * 0.6);
            }
            let best = Math.max(idScore, nameScore, genericScore, commentScore, keywordScore);
            if (best > 0)
                results.push({
                    "entry": app,
                    "score": best
                });

        }
        if (q.length > 0)
            results.sort((a, b) => {
                return b.score - a.score;
            });
        else
            results.sort((a, b) => {
                return (a.entry.name || "").localeCompare(b.entry.name || "");
            });
        resultModel.clear();
        for (let r = 0; r < results.length; r++) {
            resultModel.append({
                "appIndex": r
            });
        }
        root._results = results;
        root.selectedIndex = Math.min(root.selectedIndex, Math.max(0, results.length - 1));
    }

    function launchSelected() {
        if (_results.length === 0)
            return ;

        let entry = _results[selectedIndex].entry;
        launchProc.command = entry.command;
        launchProc.running = true;
        PanelState.launcherOpen = false;
    }

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    visible: showing || animating
    focusable: root.showing
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    onShowingChanged: {
        if (showing) {
            query = "";
            selectedIndex = 0;
            ensureAppsLoaded();
            updateFilter();
            focusTimer.start();
        }
    }

    ListModel {
        id: resultModel
    }

    Timer {
        id: focusTimer

        interval: 50
        onTriggered: searchInput.forceActiveFocus()
    }

    // 如果首次加载失败（DesktopEntries 还没 ready），监听变化
    Connections {
        function onObjectInsertedPost() {
            if (root.showing && !root.appsLoaded) {
                root.ensureAppsLoaded();
                root.updateFilter();
            }
        }

        target: DesktopEntries.applications
    }

    Process {
        id: launchProc
    }

    // ── 半透明遮罩 ──
    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root.showing ? 0.4 : 0

        Behavior on opacity {
            NumberAnimation {
                duration: 200
                easing.type: Easing.OutCubic
            }

        }

    }

    MouseArea {
        anchors.fill: parent
        onClicked: PanelState.launcherOpen = false
    }

    // ── 居中搜索面板 ──
    Rectangle {
        id: panel

        width: 600
        height: root.height * 0.55
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: parent.height * 0.2
        radius: 20
        color: Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, 0.85)
        border.color: Qt.rgba(1, 1, 1, 0.08)
        border.width: 1
        opacity: root.showing ? 1 : 0
        scale: root.showing ? 1 : 0.95

        MouseArea {
            anchors.fill: parent
            onClicked: (mouse) => {
                return mouse.accepted = true;
            }
        }

        ColumnLayout {
            id: col

            anchors.fill: parent
            anchors.margins: 16
            spacing: 8

            // ── 搜索栏 ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Text {
                    text: ""
                    color: Colors.overlay1
                    font.family: Fonts.family
                    font.pixelSize: Fonts.iconLarge
                }

                TextInput {
                    id: searchInput

                    Layout.fillWidth: true
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.heading
                    clip: true
                    selectByMouse: true
                    onTextChanged: {
                        root.query = text;
                        root.selectedIndex = 0;
                        root.updateFilter();
                    }
                    Keys.onUpPressed: {
                        root.selectedIndex = Math.max(0, root.selectedIndex - 1);
                        resultList.positionViewAtIndex(root.selectedIndex, ListView.Contain);
                    }
                    Keys.onDownPressed: {
                        root.selectedIndex = Math.min(root._results.length - 1, root.selectedIndex + 1);
                        resultList.positionViewAtIndex(root.selectedIndex, ListView.Contain);
                    }
                    Keys.onReturnPressed: root.launchSelected()
                    Keys.onEnterPressed: root.launchSelected()
                    Keys.onEscapePressed: PanelState.launcherOpen = false

                    // 占位文字
                    Text {
                        anchors.fill: parent
                        text: "搜索应用..."
                        color: Colors.overlay0
                        font: parent.font
                        visible: !parent.text && !parent.activeFocus
                    }

                }

            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Colors.surface1
                visible: root.filteredApps.length > 0
            }

            // ── 结果列表（可滚动）──
            ListView {
                id: resultList

                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: resultModel
                spacing: 2
                currentIndex: root.selectedIndex
                onCurrentIndexChanged: {
                    if (currentIndex >= 0)
                        positionViewAtIndex(currentIndex, ListView.Contain);

                }

                delegate: Rectangle {
                    required property int index
                    required property int appIndex
                    property var entry: root._results[appIndex] ? root._results[appIndex].entry : null

                    width: resultList.width
                    height: 44
                    radius: 10
                    color: index === root.selectedIndex ? Colors.surface1 : appHover.containsMouse ? Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, 0.5) : "transparent"

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        spacing: 12

                        // 应用图标
                        Image {
                            source: {
                                if (!entry)
                                    return "";

                                let icon = entry.icon || "";
                                if (icon === "")
                                    return "";

                                return "image://icon/" + icon;
                            }
                            Layout.preferredWidth: 28
                            Layout.preferredHeight: 28
                            sourceSize.width: 28
                            sourceSize.height: 28
                            visible: status === Image.Ready
                        }

                        // 应用名
                        Text {
                            text: entry ? entry.name || "" : ""
                            color: Colors.text
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon
                            font.weight: index === root.selectedIndex ? Font.Bold : Font.Normal
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }

                        // 描述
                        Text {
                            text: entry ? (entry.comment || entry.genericName || "") : ""
                            color: Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.small
                            elide: Text.ElideRight
                            Layout.maximumWidth: 200
                        }

                    }

                    MouseArea {
                        id: appHover

                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.selectedIndex = index;
                            root.launchSelected();
                        }
                    }

                    Behavior on color {
                        ColorAnimation {
                            duration: 100
                        }

                    }

                }

            }

            // 空状态
            Text {
                visible: root.query.length > 0 && resultModel.count === 0
                text: "未找到匹配的应用"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.bodyLarge
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 10
                Layout.bottomMargin: 10
            }

        }

        Behavior on opacity {
            NumberAnimation {
                id: _opacityAnim

                duration: 200
                easing.type: Easing.OutCubic
            }

        }

        Behavior on scale {
            NumberAnimation {
                id: _scaleAnim

                duration: 200
                easing.type: Easing.OutCubic
            }

        }

    }

}
