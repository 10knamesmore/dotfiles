import "../theme"
import QtQuick
import Quickshell
import Quickshell.Wayland

// 每个显示器展示自己的原生截图；关闭浮层即销毁画布，释放大图与标注内存。
Scope {
    Variants {
        model: CaptureService.snapshot ? CaptureService.snapshot.screens : []
        delegate: PanelWindow {
            id: overlay
            required property var modelData
            screen: Quickshell.screens.find(s => s.name === modelData.name) || null
            visible: CaptureService.visible
            color: "transparent"
            exclusionMode: ExclusionMode.Ignore
            anchors {
                top: true
                bottom: true
                left: true
                right: true
            }
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "quickshell-capture"
            WlrLayershell.keyboardFocus: CaptureService.activeScreen === modelData.name ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.OnDemand
            CaptureSurface {
                anchors.fill: parent
                screenInfo: overlay.modelData
            }
        }
    }

    PanelWindow {
        id: feedback
        screen: Quickshell.screens.find(s => s.name === CaptureService.activeScreen) || Quickshell.screens[0] || null
        visible: CaptureService.notice !== "" && !CaptureService.visible
        color: "transparent"
        exclusionMode: ExclusionMode.Ignore
        anchors.top: true
        margins.top: 68
        implicitWidth: Math.min(720, message.implicitWidth + 40)
        implicitHeight: message.implicitHeight + 24
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "quickshell-capture-notice"
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
        mask: Region {}
        Rectangle {
            anchors.fill: parent
            color: Colors.base
            radius: 10
            border.color: CaptureService.noticeIsError ? Colors.red : Colors.green
            Text {
                id: message
                anchors.centerIn: parent
                width: Math.min(680, implicitWidth)
                text: CaptureService.notice
                color: Colors.text
                font.pixelSize: 14
                wrapMode: Text.Wrap
                Accessible.role: Accessible.StaticText
                Accessible.name: text
            }
        }
    }

    Connections {
        target: Quickshell
        function onScreensChanged() {
            if (CaptureService.visible)
                CaptureService.cancel();
        }
    }
}
