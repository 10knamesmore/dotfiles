import "../state"
import QtQuick
import Quickshell

// OSD 服务 — 监听 AudioService 音量/静音变化自动弹音量 OSD；
// 音量 OSD 由全局快捷键主动触发（requestVolumeOsd）。
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
        OsdState.osdValue = vol;
        OsdState.osdIcon = volumeIcon(vol, muted);
        OsdState.osdVisible = true;
    }

    // 供全局快捷键调用
    function requestVolumeOsd() {
        root._lastVolume = AudioService.volume;
        root.showVolumeOsd(AudioService.volume, AudioService.muted);
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
}
