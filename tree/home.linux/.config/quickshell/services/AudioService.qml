pragma Singleton

import Quickshell
import Quickshell.Services.Pipewire

Singleton {
    id: root

    readonly property bool ready: Pipewire.ready
    readonly property var devices: Pipewire.nodes.values.filter(node => node.audio && !node.isStream)
    readonly property var outputs: devices.filter(node => node.isSink)
    readonly property var inputs: devices.filter(node => !node.isSink)
    readonly property var sink: Pipewire.defaultAudioSink
    readonly property var source: Pipewire.defaultAudioSource
    readonly property int volume: sink?.audio ? Math.round(sink.audio.volume * 100) : 0
    readonly property bool muted: sink?.audio?.muted ?? false

    // 节点被 tracker 持有时，音量和设备属性才会订阅 PipeWire 更新。
    PwObjectTracker {
        objects: root.devices
    }

    function setDeviceVolume(node, percent) {
        node.audio.volume = Math.max(0, Math.min(100, percent)) / 100;
    }

    function setVolume(percent) {
        if (sink)
            setDeviceVolume(sink, percent);
    }

    function step(deltaPercent) {
        setVolume(volume + deltaPercent);
    }

    function toggleMute() {
        if (sink)
            sink.audio.muted = !sink.audio.muted;
    }

    function selectDevice(node) {
        if (node.isSink)
            Pipewire.preferredDefaultAudioSink = node;
        else
            Pipewire.preferredDefaultAudioSource = node;
    }

    function bluetoothOutput(address) {
        return outputs.find(node => node.properties["api.bluez5.address"] === address) ?? null;
    }

    function deviceFormat(node) {
        const properties = node.properties;
        return [properties["api.bluez5.profile"], properties["api.bluez5.codec"]]
            .filter(value => !!value).join(" · ").toUpperCase();
    }
}
