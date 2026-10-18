pragma ComponentBehavior: Bound

import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Flickable {
    id: root

    required property bool active
    property string kernel: ""
    property int packageCount: 0
    property string wmVersion: ""
    property string nvimVersion: ""
    property string nodeVersion: ""
    property string pythonVersion: ""
    property string rustVersion: ""
    property var disks: []

    implicitHeight: cards.implicitHeight
    contentHeight: cards.implicitHeight
    contentWidth: width
    clip: true

    onActiveChanged: {
        if (active)
            packageReader.running = true;
    }

    function mountLabel(mountPoint) {
        const home = Quickshell.env("HOME").replace(/\/$/, "");
        if (mountPoint === home)
            return "~";
        if (mountPoint.startsWith(home + "/"))
            return "~" + mountPoint.slice(home.length);
        return mountPoint;
    }

    Timer {
        running: root.active
        interval: 5000
        repeat: true
        triggeredOnStart: true
        onTriggered: diskReader.running = true
    }

    Process {
        id: packageReader

        command: ["sh", "-c", [
            "norm(){ \"$@\" | grep -oE '[0-9]+(\\.[0-9]+){1,2}' | head -n1; }",
            "echo \"KERNEL:$(uname -r)\"",
            "echo \"PKG:$(pacman -Q | wc -l)\"",
            "echo \"WM:$(hyprctl version -j | python3 -c 'import sys,json; print(json.load(sys.stdin)[\"version\"])')\"",
            "echo \"NVIM:$(norm nvim --version)\"",
            "echo \"NODE:$(norm node --version)\"",
            "echo \"PYTHON:$(norm python3 --version)\"",
            "echo \"RUST:$(norm rustc --version)\""
        ].join(" && ")]

        stdout: SplitParser {
            onRead: data => {
                if (data.startsWith("KERNEL:")) {
                    root.kernel = data.substring(7).trim();
                } else if (data.startsWith("PKG:")) {
                    root.packageCount = parseInt(data.substring(4));
                } else if (data.startsWith("WM:")) {
                    root.wmVersion = data.substring(3).trim();
                } else if (data.startsWith("NVIM:")) {
                    root.nvimVersion = data.substring(5).trim();
                } else if (data.startsWith("NODE:")) {
                    root.nodeVersion = data.substring(5).trim();
                } else if (data.startsWith("PYTHON:")) {
                    root.pythonVersion = data.substring(7).trim();
                } else if (data.startsWith("RUST:")) {
                    root.rustVersion = data.substring(5).trim();
                }
            }
        }
    }

    Process {
        id: diskReader

        property var collectedDisks: []

        command: ["sh", "-c", "LC_ALL=C df -hT -x tmpfs -x devtmpfs -x efivarfs | awk 'NR > 1 && $7 != \"/efi\" {printf \"%s:%s:%s:%s:%s\\n\", $7,$4,$3,$6,$2}'"]
        onStarted: collectedDisks = []
        onExited: root.disks = collectedDisks

        stdout: SplitParser {
            onRead: data => {
                const fields = data.split(":");
                diskReader.collectedDisks.push({
                    mountPoint: fields[0],
                    used: fields[1],
                    total: fields[2],
                    usagePercent: parseInt(fields[3]),
                    fileSystem: fields[4]
                });
            }
        }
    }

    ColumnLayout {
        id: cards

        width: root.width
        spacing: Tokens.spaceS

        InfoCard {
            Layout.fillWidth: true
            Accessible.role: Accessible.Grouping
            Accessible.name: "系统包信息"

            contentItem: ColumnLayout {
                spacing: Tokens.spaceXS

                Text {
                    text: "系统包信息"
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.caption
                    Layout.bottomMargin: Tokens.spaceXS
                }

                InfoRow {
                    icon: "󰌘"
                    label: "内核"
                    value: root.kernel
                }

                InfoRow {
                    icon: "󰏗"
                    label: "pacman"
                    value: root.packageCount + " 个包"
                }

                InfoRow {
                    icon: "󰖯"
                    label: "Hyprland"
                    value: root.wmVersion
                }

                InfoRow {
                    icon: ""
                    label: "Neovim"
                    value: root.nvimVersion
                }

                InfoRow {
                    icon: "󰎙"
                    label: "Node"
                    value: root.nodeVersion
                }

                InfoRow {
                    icon: "󰌠"
                    label: "Python"
                    value: root.pythonVersion
                }

                InfoRow {
                    icon: "󱘗"
                    label: "Rust"
                    value: root.rustVersion
                }
            }
        }

        InfoCard {
            Layout.fillWidth: true
            Accessible.role: Accessible.Grouping
            Accessible.name: "磁盘"

            contentItem: ColumnLayout {
                spacing: Tokens.spaceM

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        Layout.fillWidth: true
                        text: "磁盘"
                        color: Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.caption
                    }

                    Text {
                        text: "已用 / 总量"
                        color: Colors.overlay1
                        font.family: Fonts.family
                        font.pixelSize: Fonts.xs
                    }
                }

                Repeater {
                    model: root.disks

                    delegate: ColumnLayout {
                        id: disk

                        required property var modelData
                        readonly property string label: root.mountLabel(modelData.mountPoint)
                        readonly property color usageColor: modelData.usagePercent > 90 ? Colors.red : modelData.usagePercent > 80 ? Colors.yellow : Colors.lavender

                        Layout.fillWidth: true
                        spacing: 6

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Tokens.spaceS

                            Text {
                                Layout.fillWidth: true
                                text: disk.label
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                                elide: Text.ElideMiddle
                            }

                            Text {
                                text: disk.modelData.fileSystem
                                color: Colors.overlay1
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                            }

                            Text {
                                text: disk.modelData.used + " / " + disk.modelData.total + " · " + disk.modelData.usagePercent + "%"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                            }
                        }

                        ProgressBar {
                            id: usageBar

                            Layout.fillWidth: true
                            implicitHeight: 5
                            from: 0
                            to: 100
                            value: disk.modelData.usagePercent
                            padding: 0
                            Accessible.name: disk.label + " 磁盘占用"
                            Accessible.description: "已用 " + disk.modelData.used + "，总量 " + disk.modelData.total

                            background: Rectangle {
                                radius: Tokens.radiusFull
                                color: Colors.surface2
                            }

                            contentItem: Item {
                                Rectangle {
                                    width: usageBar.visualPosition * parent.width
                                    height: parent.height
                                    radius: Tokens.radiusFull
                                    color: disk.usageColor
                                }
                            }
                        }
                    }
                }
            }
        }

        PerformanceMode {
            Layout.fillWidth: true
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            active: root.active
        }
    }
}
