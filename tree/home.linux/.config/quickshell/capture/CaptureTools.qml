import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// 随选区摆放的原位标注工具条。复制是主操作，保存始终为独立按钮。
Rectangle {
    id: root
    required property var surface
    width: rows.implicitWidth + 20
    height: rows.implicitHeight + 20
    radius: 10
    color: Colors.base
    border.color: Colors.surface2
    enabled: !CaptureService.exporting
    MouseArea {
        anchors.fill: parent
    }

    ColumnLayout {
        id: rows
        anchors.centerIn: parent
        spacing: 8
        RowLayout {
            spacing: 5
            Repeater {
                model: [
                    {
                        id: "select",
                        label: "选区"
                    },
                    {
                        id: "arrow",
                        label: "箭头"
                    },
                    {
                        id: "rectangle",
                        label: "矩形"
                    },
                    {
                        id: "brush",
                        label: "画笔"
                    },
                    {
                        id: "text",
                        label: "文字"
                    },
                    {
                        id: "pixelate",
                        label: "马赛克"
                    },
                    {
                        id: "redact",
                        label: "遮挡"
                    }
                ]
                CaptureButton {
                    required property var modelData
                    text: modelData.label
                    checkable: true
                    checked: root.surface.tool === modelData.id
                    hint: modelData.id === "redact" ? "敏感内容请使用实色遮挡" : ""
                    onClicked: {
                        root.surface.commitText();
                        root.surface.tool = modelData.id;
                    }
                }
            }
            CaptureButton {
                text: "撤销"
                hint: "Ctrl+Z"
                enabled: root.surface.annotations.length > 0
                onClicked: root.surface.undo()
            }
            CaptureButton {
                text: "重做"
                hint: "Ctrl+Shift+Z"
                enabled: root.surface.redoStack.length > 0
                onClicked: root.surface.redo()
            }
        }
        RowLayout {
            spacing: 7
            Repeater {
                model: ["#f38ba8", "#fab387", "#f9e2af", "#a6e3a1", "#89b4fa", "#cba6f7", "#ffffff", "#000000"]
                Button {
                    id: swatch
                    required property string modelData
                    implicitWidth: 25
                    implicitHeight: 25
                    checkable: true
                    checked: root.surface.ink === modelData
                    Accessible.name: "标注颜色 " + modelData
                    onClicked: root.surface.ink = modelData
                    background: Rectangle {
                        color: swatch.modelData
                        radius: 5
                        border.width: swatch.checked || swatch.activeFocus ? 3 : 1
                        border.color: swatch.checked || swatch.activeFocus ? Colors.lavender : Colors.surface2
                    }
                }
            }
            CaptureButton {
                text: "取色"
                onClicked: {
                    root.surface.commitText();
                    root.surface.tool = "eyedropper";
                }
            }
            CaptureComboBox {
                implicitWidth: 82
                model: ["细线", "中线", "粗线"]
                currentIndex: 1
                Accessible.name: "标注线宽"
                onActivated: index => root.surface.strokeWidth = [2, 4, 8][index]
            }
            Text {
                text: root.surface.selection.width + " × " + root.surface.selection.height
                color: Colors.subtext0
                font.pixelSize: 12
                Layout.fillWidth: true
            }
            CaptureButton {
                text: "保存"
                hint: "Ctrl+S · 保存到图片目录"
                onClicked: root.surface.finish("save")
            }
            CaptureButton {
                text: "复制"
                accent: true
                hint: "Enter · 不保存文件"
                onClicked: root.surface.finish("copy")
            }
        }
    }
}
