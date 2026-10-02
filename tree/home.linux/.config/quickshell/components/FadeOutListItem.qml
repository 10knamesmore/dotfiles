import "../theme"
import QtQuick

// 列表条目删除时保留画面 240 ms：原地淡出，并收拢自身高度与尾部间距。
// 调用方提供 implicitHeight；ListView.spacing 保持为 0，间距由 itemSpacing 承担。
Item {
    id: root

    property bool animateRemoval: true
    property real itemSpacing: 0
    property alias color: surface.color
    property alias radius: surface.radius
    default property alias content: surface.data
    readonly property bool removing: _removing
    property bool _removing: false
    property real _heightProgress: 1

    // 用于宿主等待全部退场结束后显示空态；提前销毁也会配对发射。
    signal removalStarted()
    signal removalFinished()

    height: (implicitHeight + itemSpacing) * _heightProgress
    enabled: !removing
    ListView.delayRemove: removing
    ListView.onRemove: {
        if (animateRemoval) {
            _removing = true;
            removalStarted();
            removal.start();
        }
    }
    onAnimateRemovalChanged: {
        if (!animateRemoval && removing)
            removal.complete();
    }
    Component.onDestruction: finishRemoval()

    function finishRemoval() {
        if (!_removing)
            return;
        _removing = false;
        removalFinished();
    }

    Rectangle {
        id: surface
        width: root.width
        height: root.implicitHeight
        color: "transparent"
    }

    ParallelAnimation {
        id: removal
        onFinished: root.finishRemoval()

        NumberAnimation {
            target: surface
            property: "opacity"
            to: 0
            duration: 140
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.accelerate
        }

        SequentialAnimation {
            PauseAnimation { duration: 40 }
            NumberAnimation {
                target: root
                property: "_heightProgress"
                to: 0
                duration: 200
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }
        }
    }
}
