import "../theme"
import QtQuick
import Quickshell
import Quickshell.Io

// cava 单例服务 — 启动 cava 进程，将频谱数据写入 PanelState.visualizerBars
// 在 shell.qml 中实例化一次即可，多个 AudioVisualizer 共享数据
Item {
    id: root

    // ── cava 参数 ──
    property int barCount: 48        // 频谱柱数量，决定横向宽度
    property int framerate: 60       // 刷新帧率 (fps)
    property int sensitivity: 80     // 自动增益灵敏度（越低越不容易打满，建议 20-80）
    property int maxRange: 100       // ascii 输出最大值，柱高按此归一化

    Component.onCompleted: ensureFifo.running = true
    Component.onDestruction: {
        cavaProc.running = false;
        readerProc.running = false;
    }

    // 确保 FIFO 存在
    Process {
        id: ensureFifo
        command: ["sh", "-c", "[ -p /tmp/quickshell-cava.fifo ] || mkfifo /tmp/quickshell-cava.fifo"]
        onExited: writeConfig.running = true
    }

    // 生成 cava 配置
    Process {
        id: writeConfig
        command: ["sh", "-c",
            "mkdir -p /tmp/quickshell-cava && cat > /tmp/quickshell-cava/config << CAVAEOF\n" +
            "[general]\n" +
            "bars = " + root.barCount + "\n" +
            "framerate = " + root.framerate + "\n" +
            "sensitivity = " + root.sensitivity + "\n" +
            "\n" +
            "[output]\n" +
            "method = raw\n" +
            "raw_target = /tmp/quickshell-cava.fifo\n" +
            "data_format = ascii\n" +
            "ascii_max_range = " + root.maxRange + "\n" +
            "CAVAEOF"]
        onExited: {
            cavaProc.running = true;
            startReader.start();
        }
    }

    Timer {
        id: startReader
        interval: 500
        onTriggered: readerProc.running = true
    }

    // cava 进程
    Process {
        id: cavaProc
        command: ["cava", "-p", "/tmp/quickshell-cava/config"]
    }

    // 读取 FIFO 输出
    Process {
        id: readerProc
        command: ["cat", "/tmp/quickshell-cava.fifo"]
        stdout: SplitParser {
            onRead: data => {
                let parts = data.split(";");
                let vals = [];
                for (let i = 0; i < root.barCount; i++) {
                    let v = i < parts.length ? (parseInt(parts[i]) || 0) : 0;
                    vals.push(v);
                }
                PanelState.visualizerBars = vals;
            }
        }
    }

    // cava 意外退出时自动重启
    Connections {
        target: cavaProc
        function onExited() {
            if (PanelState.visualizerVisible)
                restartTimer.start();
        }
    }

    Timer {
        id: restartTimer
        interval: 3000
        onTriggered: ensureFifo.running = true
    }
}
