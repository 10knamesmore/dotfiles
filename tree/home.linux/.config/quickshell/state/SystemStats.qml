import QtQuick
pragma Singleton

// 顶栏快照固定每秒更新；面板的 100ms 数据保存在 ResourceStats。
QtObject {
    property var cpu: ({
        "usage": 0,
        "cores": []
    })
    property var memory: ({
        "usage": 0,
        "usedBytes": 0,
        "totalBytes": 0
    })
    property var network: ({
        "name": "",
        "downSpeed": 0,
        "upSpeed": 0,
        "downTotal": 0,
        "upTotal": 0
    })
}
