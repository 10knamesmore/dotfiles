import QtQuick
pragma Singleton

// 面板快照独立于顶栏的一秒快照；历史按时间保留最近一分钟。
QtObject {
    property var cpu: ({
        "status": "waiting",
        "usage": 0,
        "frequencyGHz": null,
        "temperatureC": null,
        "processes": []
    })
    property var cpuHistory: []
    property var memory: ({
        "status": "waiting",
        "totalBytes": 0,
        "usedBytes": 0,
        "availableBytes": 0,
        "breakdown": {
            "freeBytes": 0,
            "buffersBytes": 0,
            "cachedBytes": 0,
            "sharedBytes": 0
        },
        "swapTotalBytes": 0,
        "swapUsedBytes": 0,
        "pressureSome10": null,
        "processes": []
    })
    property var memoryHistory: []
    property var network: ({
        "status": "waiting",
        "defaultInterface": "",
        "interfaces": []
    })
    property var networkHistories: ({
    })
    property string networkInterface: ""

    function accept(packet) {
        const snapshot = Object.assign({
        }, packet.snapshot, {
            "status": "ready"
        });
        if (packet.resource === "cpu") {
            cpu = snapshot;
            recordCpu(packet.time, cpu.usage);
        } else if (packet.resource === "memory") {
            memory = snapshot;
            recordMemory(packet.time, memory.usedBytes);
        } else {
            network = snapshot;
            if (!network.interfaces.some((iface) => {
                return iface.name === networkInterface;
            }))
                networkInterface = network.defaultInterface || (network.interfaces.length ? network.interfaces[0].name : "");

            recordNetwork(packet.time, network.interfaces);
        }
    }

    function historyWith(history, time, value) {
        return history.filter((point) => {
            return point.time > time - 60;
        }).concat([{
            "time": time,
            "value": value
        }]);
    }

    function recordCpu(time, usage) {
        cpuHistory = historyWith(cpuHistory, time, usage);
    }

    function recordMemory(time, usedBytes) {
        memoryHistory = historyWith(memoryHistory, time, usedBytes);
    }

    function recordNetwork(time, interfaces) {
        const histories = Object.assign({
        }, networkHistories);
        for (const iface of interfaces) {
            const previous = histories[iface.name] || {
                "down": [],
                "up": []
            };
            histories[iface.name] = {
                "down": historyWith(previous.down, time, iface.downSpeed),
                "up": historyWith(previous.up, time, iface.upSpeed)
            };
        }
        for (const name of Object.keys(histories)) {
            if (histories[name].down.length && histories[name].down[histories[name].down.length - 1].time <= time - 60)
                delete histories[name];

        }
        networkHistories = histories;
    }

    function setStatus(resource, status) {
        if (resource === "cpu")
            cpu = Object.assign({
        }, cpu, {
            "status": status
        });
        else if (resource === "memory")
            memory = Object.assign({
        }, memory, {
            "status": status
        });
        else
            network = Object.assign({
        }, network, {
            "status": status
        });
    }

}
