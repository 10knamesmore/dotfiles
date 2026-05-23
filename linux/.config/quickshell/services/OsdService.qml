import "../theme"
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire

// OSD 服务 — 监听 PipeWire 音量/静音变化自动弹音量 OSD；
// 亮度 OSD 与音量 OSD 由全局快捷键主动触发（requestVolumeOsd / requestBrightnessOsd）。
Scope {
    id: root

    property var _sink: Pipewire.defaultAudioSink
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
        volumeProc.running = true;
    }
    function requestBrightnessOsd() {
        brightnessProc.running = true;
    }

    Connections {
        function onVolumesChanged() {
            let vol = Math.round(root._sink.audio.volume * 100);
            if (root._lastVolume >= 0 && vol !== root._lastVolume) {
                root.showVolumeOsd(vol, root._sink.audio.muted);
            }
            root._lastVolume = vol;
        }

        function onMutedChanged() {
            let vol = Math.round(root._sink.audio.volume * 100);
            root.showVolumeOsd(vol, root._sink.audio.muted);
        }

        target: root._sink ? root._sink.audio : null
    }

    Process {
        id: volumeProc

        command: ["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"]

        stdout: SplitParser {
            onRead: data => {
                let m = data.match(/Volume:\s+([\d.]+)(\s+\[MUTED\])?/);
                if (m) {
                    let vol = Math.round(parseFloat(m[1]) * 100);
                    let muted = m[2] !== undefined;
                    root._lastVolume = vol;
                    root.showVolumeOsd(vol, muted);
                }
            }
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
