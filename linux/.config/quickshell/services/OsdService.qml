import "../state"
import QtQuick
import Quickshell
import Quickshell.Io

// OSD 服务 — 监听 AudioService 音量/静音变化自动弹音量 OSD；
// 亮度 OSD 与音量 OSD 由全局快捷键主动触发（requestVolumeOsd / requestBrightnessOsd）。
Scope {
    id: root

    property real _lastVolume: -1

    function volumeIcon(vol, muted) {
        if (muted)
            return "󰝟";
        if (vol <= 0)
            return "󰕿";
        if (vol < 50)
            return "󰖀";
        return "󰕾";
    }

    function showVolumeOsd(vol, muted) {
        OsdState.osdType = "volume";
        OsdState.osdValue = vol;
        OsdState.osdIcon = volumeIcon(vol, muted);
        OsdState.osdVisible = true;
    }

    // 供全局快捷键调用
    function requestVolumeOsd() {
        root._lastVolume = AudioService.volume;
        root.showVolumeOsd(AudioService.volume, AudioService.muted);
    }
    function requestBrightnessOsd() {
        brightnessProc.running = true;
    }

    Connections {
        target: AudioService

        function onVolumeChanged() {
            let vol = AudioService.volume;
            if (root._lastVolume >= 0 && vol !== root._lastVolume)
                root.showVolumeOsd(vol, AudioService.muted);
            root._lastVolume = vol;
        }

        function onMutedChanged() {
            root.showVolumeOsd(AudioService.volume, AudioService.muted);
        }
    }

    // ── 亮度检测 ──
    Process {
        id: brightnessProc

        command: ["brightnessctl", "-m"]

        stdout: SplitParser {
            onRead: data => {
                let parts = data.split(",");
                if (parts.length >= 4) {
                    let pct = parseInt(parts[3]) || 0;
                    OsdState.osdType = "brightness";
                    OsdState.osdValue = pct;
                    OsdState.osdIcon = pct > 50 ? "󰃠" : "󰃞";
                    OsdState.osdVisible = true;
                }
            }
        }
    }
}
