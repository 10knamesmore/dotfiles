pragma Singleton

import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire

// 统一音量服务 — 收口全 shell 的音量读写。
// 读：基于 PipeWire defaultAudioSink 事件驱动（取代旧的 wpctl 轮询）。
// 写：统一封装 wpctl 命令（保留 -l 1.0 上限语义）。
// 不提供图标函数：各消费者图标字形不同（waybar pulseaudio vs nerdfont），自行决定。
Singleton {
    id: root

    readonly property var sink: Pipewire.defaultAudioSink
    readonly property int volume: sink && sink.audio ? Math.round(sink.audio.volume * 100) : 0
    readonly property bool muted: sink && sink.audio ? sink.audio.muted : false

    // PipeWire 节点属性需 tracker 才会实时同步，否则 volume 恒为 0
    PwObjectTracker {
        objects: root.sink ? [root.sink] : []
    }

    // 设为绝对百分比（0~100）
    function setVolume(pct) {
        let v = Math.max(0, Math.min(100, pct)) / 100;
        _setProc.command = ["wpctl", "set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", v.toFixed(2)];
        _setProc.running = true;
    }

    // 相对增减（deltaPct 可正可负，单位 %）
    function step(deltaPct) {
        let sign = deltaPct >= 0 ? (Math.abs(deltaPct) + "%+") : (Math.abs(deltaPct) + "%-");
        _setProc.command = ["wpctl", "set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", sign];
        _setProc.running = true;
    }

    function toggleMute() {
        _muteProc.command = ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"];
        _muteProc.running = true;
    }

    Process {
        id: _setProc
    }

    Process {
        id: _muteProc
    }
}
