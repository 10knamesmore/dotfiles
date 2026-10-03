pragma ComponentBehavior: Bound

import "../state"
import QtQuick
import Quickshell
import Quickshell.Io

// 只为打开的面板启动一个持续采集进程，不在 100ms 定时器里反复 fork。
Scope {
    id: root

    readonly property string resource: PanelState.cpuOpen ? "cpu" : PanelState.memoryOpen ? "memory" : PanelState.networkStatsOpen ? "network" : ""

    onResourceChanged: console.info("[resource-monitor] active", resource || "none")

    Variants {
        model: root.resource ? [root.resource] : []

        delegate: Scope {
            id: sampler

            required property string modelData

            Component.onCompleted: ResourceStats.setStatus(modelData, "waiting")

            Process {
                command: ["python3", "-u", Quickshell.shellPath("services/resource-monitor/collect.py"), sampler.modelData]
                running: true

                stderr: SplitParser {
                    onRead: line => console.warn(line)
                }
                stdout: SplitParser {
                    onRead: line => {
                        if (root.resource !== sampler.modelData)
                            return;
                        ResourceStats.accept(JSON.parse(line));
                    }
                }

                onExited: exitCode => {
                    if (root.resource === sampler.modelData) {
                        ResourceStats.setStatus(sampler.modelData, "error");
                        console.error("[resource-monitor] collector exited", sampler.modelData, exitCode);
                    }
                }
            }
        }
    }
}
