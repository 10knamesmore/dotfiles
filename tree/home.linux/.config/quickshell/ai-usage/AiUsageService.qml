pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

// 为所有显示器共享 Codex / DeepSeek 状态；启动及每分钟运行一次只读采集。
// 每次结果覆盖旧状态，失败时不把上次额度伪装成当前数据；不提供手动刷新。
Singleton {
    id: root

    property var codex: ({ "status": "loading" })
    property var deepseek: ({ "status": "loading" })
    readonly property bool refreshing: collector.running

    Process {
        id: collector

        command: ["python3", Quickshell.shellPath("ai-usage/collect.py")]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const result = JSON.parse(text);
                    root.codex = result.codex;
                    root.deepseek = result.deepseek;
                } catch (error) {
                    root.codex = { "status": "unavailable" };
                    root.deepseek = { "status": "unavailable" };
                    console.warn("[ai-usage] invalid collector output");
                }
            }
        }
        stderr: SplitParser {
            onRead: line => console.info(line)
        }
        onExited: exitCode => {
            if (exitCode !== 0) {
                root.codex = { "status": "unavailable" };
                root.deepseek = { "status": "unavailable" };
                console.warn("[ai-usage] collector exited", exitCode);
            }
        }
    }

    Timer {
        interval: 60000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            if (!collector.running) {
                console.info("[ai-usage] scheduled refresh");
                collector.running = true;
            }
        }
    }
}
