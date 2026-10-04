import "../theme"
import "../bar/components"
import QtQuick

// 顶栏捕获入口；录制时整个计时按钮都是停止热区，保存期间不可再次操作。
BarModule {
    id: root

    implicitWidth: label.implicitWidth + horizontalPadding * 2
    clickable: CaptureService.recordingState !== "saving"
    activeFocusOnTab: true
    accentColor: CaptureService.recordingActive ? Colors.red : Colors.blue
    Accessible.role: Accessible.Button
    Accessible.name: CaptureService.recordingActive ? "停止录屏" : "屏幕捕获"
    Accessible.description: CaptureService.recordingActive ? CaptureService.recordingLabel : "截图、录屏与取色"
    Accessible.onPressAction: activate()
    Keys.onReturnPressed: activate()
    Keys.onSpacePressed: activate()
    onClicked: activate()

    function activate() {
        if (CaptureService.recordingActive)
            CaptureService.stopRecording();
        else
            CaptureService.open("screenshot", "region");
    }

    Text {
        id: label
        anchors.centerIn: parent
        text: CaptureService.recordingActive ? "■  " + CaptureService.recordingLabel : "󰹑"
        color: root.accentColor
        font.family: Fonts.family
        font.pixelSize: CaptureService.recordingActive ? 12 : 18
        Accessible.ignored: true
    }
}
