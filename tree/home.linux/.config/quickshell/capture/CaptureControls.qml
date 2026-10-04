import "../theme"
import QtQuick
import QtQuick.Layouts

// 捕获模式、共享来源与录制前的音频选项；Portal 请求没有模式切换入口。
Rectangle {
    id: root
    required property var surface
    readonly property bool recordingOptionsVisible: CaptureService.mode === "record-setup" || CaptureService.mode === "record-wait"
    width: contents.implicitWidth + 24
    height: contents.implicitHeight + 24
    radius: 12
    color: Colors.base
    border.color: Colors.surface2
    MouseArea {
        anchors.fill: parent
    }

    ColumnLayout {
        id: contents
        anchors.centerIn: parent
        spacing: 10
        RowLayout {
            spacing: 7
            Text {
                text: CaptureService.mode === "portal" ? "选择共享内容" : "屏幕捕获"
                color: Colors.text
                font.pixelSize: 15
                font.bold: true
                Layout.rightMargin: 8
            }
            Repeater {
                model: CaptureService.mode === "portal" ? [] : [
                    {
                        label: "截图",
                        mode: "screenshot"
                    },
                    {
                        label: "录屏",
                        mode: "record-setup"
                    },
                    {
                        label: "取色",
                        mode: "color"
                    }
                ]
                CaptureButton {
                    required property var modelData
                    text: modelData.label
                    checkable: true
                    checked: CaptureService.mode === modelData.mode || (modelData.mode === "record-setup" && CaptureService.mode === "record-wait")
                    enabled: !CaptureService.exporting && CaptureService.mode !== "record-wait"
                    onClicked: CaptureService.open(modelData.mode, "region")
                }
            }
            Item {
                Layout.fillWidth: true
                implicitWidth: 12
            }
            Text {
                text: root.surface.screenInfo.name
                color: Colors.subtext0
                font.pixelSize: 12
            }
            CaptureButton {
                text: "取消"
                hint: "Esc"
                enabled: !CaptureService.exporting
                onClicked: CaptureService.cancel()
            }
        }

        RowLayout {
            visible: CaptureService.mode === "screenshot" || CaptureService.mode === "portal"
            spacing: 7
            Repeater {
                model: [
                    {
                        id: "region",
                        label: "区域"
                    },
                    {
                        id: "window",
                        label: "窗口"
                    },
                    {
                        id: "output",
                        label: "显示器"
                    }
                ]
                CaptureButton {
                    required property var modelData
                    text: modelData.label
                    checkable: true
                    checked: CaptureService.target === modelData.id
                    enabled: !CaptureService.exporting
                    onClicked: {
                        CaptureService.setTarget(modelData.id);
                        if (modelData.id === "output")
                            root.surface.selectOutput();
                    }
                }
            }
            Text {
                text: CaptureService.target === "region" ? "拖动框选 · 边缘调整 · 内部移动" : CaptureService.target === "window" ? "点击窗口" : "点击要捕获的显示器"
                color: Colors.subtext0
                font.pixelSize: 12
            }
        }

        RowLayout {
            visible: CaptureService.mode === "portal"
            spacing: 10
            CaptureComboBox {
                id: windows
                visible: CaptureService.target === "window"
                implicitWidth: 430
                model: root.surface.windowChoices
                textRole: "title"
                currentIndex: -1
                displayText: root.surface.selectedWindowId === "" ? "选择窗口（包括其他工作区）" : currentText
                Accessible.name: "共享窗口"
                onActivated: index => root.surface.choosePortalWindow(index)
                Connections {
                    target: root.surface
                    function onSelectedWindowIdChanged() {
                        windows.currentIndex = root.surface.windowChoices.findIndex(w => w.id === root.surface.selectedWindowId);
                    }
                }
            }
            CaptureButton {
                text: CaptureService.exporting ? "正在确认…" : "确认共享"
                accent: true
                enabled: !CaptureService.exporting && (CaptureService.target === "window" ? root.surface.selectedWindowId !== "" : root.surface.hasSelection)
                onClicked: root.surface.finish("copy")
            }
            Text {
                visible: CaptureService.target !== "window"
                text: "只共享所选范围"
                color: Colors.subtext0
                font.pixelSize: 12
            }
        }

        RowLayout {
            visible: root.recordingOptionsVisible
            enabled: CaptureService.mode !== "record-wait"
            CaptureCheckBox {
                id: systemAudio
                text: "系统声音"
                checked: false
            }
            CaptureCheckBox {
                id: microphone
                text: "麦克风"
                checked: false
            }
            CaptureCheckBox {
                id: cursor
                text: "鼠标指针"
                checked: true
            }
        }

        RowLayout {
            visible: root.recordingOptionsVisible
            CaptureButton {
                text: CaptureService.mode === "record-wait" ? "正在打开选择器…" : "选择录制范围"
                accent: true
                enabled: CaptureService.mode === "record-setup"
                onClicked: {
                    console.info("[capture] recording options confirmed");
                    CaptureService.prepareRecording(systemAudio.checked, microphone.checked, cursor.checked);
                }
            }
            Text {
                text: "H.264 · 60 FPS · MP4"
                color: Colors.subtext0
                font.pixelSize: 12
            }
        }

        RowLayout {
            visible: CaptureService.mode === "color"
            Text {
                text: "移动查看像素 · 点击复制颜色"
                color: Colors.text
                font.pixelSize: 13
            }
            CaptureComboBox {
                model: ["HEX", "RGB"]
                currentIndex: root.surface.rgbFormat ? 1 : 0
                Accessible.name: "颜色复制格式"
                onActivated: index => root.surface.rgbFormat = index === 1
            }
            Text {
                text: root.surface.rgbFormat ? root.surface.sampledRgb : root.surface.sampledColor
                color: Colors.text
                font.pixelSize: 13
            }
        }

        Text {
            visible: CaptureService.errorText !== ""
            text: CaptureService.errorText
            color: Colors.red
            font.pixelSize: 13
            Layout.maximumWidth: 620
            wrapMode: Text.Wrap
            Accessible.role: Accessible.StaticText
            Accessible.name: text
        }
    }
}
