import QtQuick
pragma Singleton

// 顶栏写入来源几何与真实头部组件；PanelOverlay 暂时接管头部，收起后归还。
// heldItems 只隐藏原胶囊的外壳，保留其布局位置和数据更新；本单例不采集系统数据。
QtObject {
    property var pendingSource: null
    property var heldItems: []

    function isHeld(item) {
        return heldItems.indexOf(item) !== -1;
    }

    function hold(item) {
        heldItems = heldItems.concat([item]);
    }

    function release(item) {
        heldItems = heldItems.filter(held => held !== item);
    }

    function boundsFor(item, barWindow) {
        const origin = item.mapToItem(null, 0, 0);
        const corner = item.mapToItem(null, item.width, item.height);
        return Qt.rect(origin.x + barWindow.margins.left,
            origin.y + barWindow.margins.top, corner.x - origin.x, corner.y - origin.y);
    }

    function openFrom(item, openPanel) {
        const barWindow = item.parent.barWindow;
        item.parent.morphItem = item;
        pendingSource = {
            item: item,
            header: item.capsuleHeader,
            home: item.capsuleHeader.parent,
            barWindow: barWindow,
            screen: barWindow.screen,
            initialDetails: item.detailProgress,
            bounds: boundsFor(item, barWindow)
        };
        console.info("[panel-morph] opening from", barWindow.screen.name, pendingSource.bounds);
        openPanel();
    }

    function takeSource() {
        const source = pendingSource;
        pendingSource = null;
        return source;
    }

    function reset() {
        pendingSource = null;
    }
}
