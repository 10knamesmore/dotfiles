import "../components"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

// App Launcher — 全屏居中，fuzzy search 启动应用
PanelOverlay {
    id: root

    showing: PanelState.launcherOpen
    panelWidth: 600
    panelHeight: root.height * 0.55
    panelRadius: Tokens.radiusXL
    panelTargetY: root.height * 0.2
    backdropOpacity: Tokens.backdropMedium
    closedOffsetY: -20
    onCloseRequested: PanelState.launcherOpen = false
    onShowingChanged: {
        if (showing) {
            searchInput.text = "";
            query = "";
            selectedIndex = 0;
            ensureAppsLoaded();
            updateFilter();
            focusTimer.start();
        }
    }

    // 搜索状态
    property string query: ""
    property int selectedIndex: 0
    // 缓存全部应用（非 noDisplay），启动时填充一次，预计算 lowercase 字段
    property var allApps: []
    property bool appsLoaded: false
    // 存储排序后的结果引用，delegate 通过 index 索引取
    property var _results: []

    function ensureAppsLoaded() {
        if (appsLoaded)
            return;

        let apps = DesktopEntries.applications.values;
        if (!apps || apps.length === 0)
            return;

        let list = [];
        for (let i = 0; i < apps.length; i++) {
            let app = apps[i];
            if (!app.noDisplay)
                list.push({
                    entry: app,
                    _name: (app.name || "").toLowerCase(),
                    _id: (app.id || "").toLowerCase(),
                    _generic: (app.genericName || "").toLowerCase(),
                    _comment: (app.comment || "").toLowerCase(),
                    _keywords: (app.keywords || []).map(k => k.toLowerCase())
                });
        }
        allApps = list;
        appsLoaded = true;
    }

    // ── Fuzzy Match（接收已 lowercase 的文本，避免重复转换） ──
    function fuzzyScore(ql, lower, origLen) {
        if (ql.length === 0)
            return 1;
        if (lower.length === 0)
            return 0;
        if (lower.startsWith(ql))
            return 1000 + (100 - origLen);

        let qi = 0, score = 0, consecutive = 0;
        for (let i = 0; i < lower.length && qi < ql.length; i++) {
            if (lower[i] === ql[qi]) {
                qi++;
                consecutive++;
                score += consecutive * 10;
                if (i === 0 || lower[i - 1] === ' ' || lower[i - 1] === '-' || lower[i - 1] === '.')
                    score += 50;
            } else {
                consecutive = 0;
            }
        }
        return qi === ql.length ? score : 0;
    }

    function updateFilter() {
        let apps = root.allApps;
        let q = root.query.trim().toLowerCase();
        let results = [];
        for (let i = 0; i < apps.length; i++) {
            let a = apps[i];
            let nameScore = fuzzyScore(q, a._name, a._name.length);
            let idScore = fuzzyScore(q, a._id, a._id.length) * 0.9;
            let genericScore = fuzzyScore(q, a._generic, a._generic.length) * 0.8;
            let commentScore = fuzzyScore(q, a._comment, a._comment.length) * 0.5;
            let keywordScore = 0;
            let kw = a._keywords;
            for (let k = 0; k < kw.length; k++) {
                keywordScore = Math.max(keywordScore, fuzzyScore(q, kw[k], kw[k].length) * 0.6);
            }
            let best = Math.max(nameScore, idScore, genericScore, commentScore, keywordScore);
            if (best > 0)
                results.push({
                    "entry": a.entry,
                    "score": best
                });
        }
        if (q.length > 0)
            results.sort((a, b) => b.score - a.score);
        else
            results.sort((a, b) => (a.entry.name || "").localeCompare(b.entry.name || ""));
        root._results = results;
        root.selectedIndex = Math.min(root.selectedIndex, Math.max(0, results.length - 1));
    }

    function launchSelected() {
        if (_results.length === 0)
            return;

        let entry = _results[selectedIndex].entry;
        launchProc.command = entry.command;
        launchProc.running = true;
        PanelState.launcherOpen = false;
    }

    Timer {
        id: filterDebounce
        interval: 50
        onTriggered: root.updateFilter()
    }

    Timer {
        id: focusTimer
        interval: 50
        onTriggered: searchInput.forceActiveFocus()
    }

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

    ColumnLayout {
        id: col

        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceS

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
                    filterDebounce.restart();
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
            visible: root._results.length > 0
        }

        // ── 结果列表 ──
        ListView {
            id: resultList

            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root._results.length
            spacing: 2
            currentIndex: root.selectedIndex
            onCurrentIndexChanged: {
                if (currentIndex >= 0)
                    positionViewAtIndex(currentIndex, ListView.Contain);
            }

            delegate: Rectangle {
                required property int index
                property var entry: root._results[index] ? root._results[index].entry : null

                width: resultList.width
                height: 44
                radius: Tokens.radiusMS
                color: index === root.selectedIndex ? Colors.surface1 : appHover.containsMouse ? Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, 0.5) : "transparent"

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: Tokens.spaceM

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

                    Text {
                        text: entry ? entry.name || "" : ""
                        color: Colors.text
                        font.family: Fonts.family
                        font.pixelSize: Fonts.icon
                        font.weight: index === root.selectedIndex ? Font.Bold : Font.Normal
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

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
                        duration: Tokens.animFast
                    }
                }
            }
        }

        // 空状态
        Text {
            visible: root.query.length > 0 && root._results.length === 0
            text: "未找到匹配的应用"
            color: Colors.overlay0
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 10
            Layout.bottomMargin: 10
        }
    }
}
