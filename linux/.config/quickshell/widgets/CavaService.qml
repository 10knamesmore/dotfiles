import "../theme"
import QtQuick
import Quickshell
import Quickshell.Io

// cava 单例服务 — 启动 cava 进程，将频谱数据写入 PanelState.visualizerBars
// 在 shell.qml 中实例化一次即可，多个 AudioVisualizer 共享数据
Item {
    id: root

    // ── cava 参数 ──
    // [general]
    property int barCount: 150          // 频谱柱数量
    property int framerate: 120          // 刷新帧率 (fps)
    property int sensitivity: 200       // 自动增益灵敏度
    property bool autosens: true        // 自动灵敏度调节
    property real noiseReduction: 0   // 噪声抑制 0-1
    property int lowerCutoff: 40        // 最低频率 Hz
    property int upperCutoff: 10000     // 最高频率 Hz
    // [output]
    property int maxRange: 100          // ascii 输出最大值
    property string channels: "mono"    // stereo/mono
    property string monoOption: "average" // average/left/right
    property bool reverse: false        // 反转柱子顺序
    // [smoothing]
    property int integral: 0            // 积分平滑度 0-100
    property bool monstercat: false     // Monstercat 风格平滑
    property int waves: 0              // 波浪平滑强度
    property int gravity: 100          // 下落速度

    Component.onCompleted: writeConfig.running = true
    Component.onDestruction: cavaProc.running = false

    // 生成 cava 配置（输出到 stdout）
    Process {
        id: writeConfig
        command: ["sh", "-c", "mkdir -p /tmp/quickshell-cava && cat > /tmp/quickshell-cava/config << CAVAEOF\n"
            + "[general]\n"
            + "bars = " + root.barCount + "\n"
            + "framerate = " + root.framerate + "\n"
            + "sensitivity = " + root.sensitivity + "\n"
            + "autosens = " + (root.autosens ? 1 : 0) + "\n"
            + "noise_reduction = " + root.noiseReduction + "\n"
            + "lower_cutoff_freq = " + root.lowerCutoff + "\n"
            + "upper_cutoff_freq = " + root.upperCutoff + "\n"
            + "\n"
            + "[output]\n"
            + "method = raw\n"
            + "raw_target = /dev/stdout\n"
            + "data_format = ascii\n"
            + "ascii_max_range = " + root.maxRange + "\n"
            + "channels = " + root.channels + "\n"
            + "mono_option = " + root.monoOption + "\n"
            + "reverse = " + (root.reverse ? 1 : 0) + "\n"
            + "\n"
            + "[smoothing]\n"
            + "integral = " + root.integral + "\n"
            + "monstercat = " + (root.monstercat ? 1 : 0) + "\n"
            + "waves = " + root.waves + "\n"
            + "gravity = " + root.gravity + "\n"
            + "CAVAEOF"]
        onExited: cavaProc.running = true
    }

    // cava 直接输出到 stdout，SplitParser 直接读取
    Process {
        id: cavaProc
        command: ["cava", "-p", "/tmp/quickshell-cava/config"]
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
        onTriggered: writeConfig.running = true
    }
}
