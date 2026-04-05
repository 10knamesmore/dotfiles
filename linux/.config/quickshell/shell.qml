//@ pragma IconTheme breeze-dark
//@ pragma UseQApplication

import "./ai"
import "./bar"
import "./calendar"
import "./clipboard"
import "./journal"
import "./keybindings"
import "./launcher"
import "./media"
import "./network"
import "./notes"
import "./notifications"
import "./osd"
import "./power"
import "./screen-effects"
import "./settings"
import "./theme"
import "./widgets"
import QtQuick
import Quickshell
import Quickshell.Hyprland._GlobalShortcuts
import Quickshell.Io
import Quickshell.Services.Mpris
import Quickshell.Services.Notifications
import Quickshell.Services.Pipewire

ShellRoot {
    id: root

    // ── OSD: 音量读取与图标选择 ──
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
        PanelState.osdType = "volume";
        PanelState.osdValue = vol;
        PanelState.osdIcon = volumeIcon(vol, muted);
        PanelState.osdVisible = true;
    }

    // ── 歌词服务 ──
    property var _lyricsPlayer: {
        let ps = Mpris.players.values;
        for (let i = 0; i < ps.length; i++) {
            if (ps[i].isPlaying) {
                PanelState.lastActivePlayer = ps[i];
                return ps[i];
            }
        }
        if (PanelState.lastActivePlayer && ps.indexOf(PanelState.lastActivePlayer) >= 0)
            return PanelState.lastActivePlayer;
        return ps.length > 0 ? ps[0] : null;
    }
    property string _lyricsTrackKey: _lyricsPlayer ? (_lyricsPlayer.identity + "|" + _lyricsPlayer.trackTitle) : ""

    property string _lastLyricsRaw: "" // 上一次成功的歌词原文，用于双缓冲比较

    on_LyricsTrackKeyChanged: {
        if (_lyricsTrackKey && _lyricsPlayer) {
            PanelState.lyricsLines = [];
            PanelState.currentLyricIndex = -1;
            PanelState.currentLyric = "";
            PanelState.lyricsTrackId = _lyricsTrackKey;
            // 启动轮询，等歌词数据更新
            _lyricsPollTimer.restart();
        } else {
            _lyricsPollTimer.stop();
            PanelState.lyricsLines = [];
            PanelState.currentLyricIndex = -1;
            PanelState.currentLyric = "";
            PanelState.lyricsTrackId = "";
        }
    }

    Timer {
        id: _lyricsPollTimer
        interval: 500
        repeat: true
        onTriggered: {
            if (!root._lyricsPlayer) {
                stop();
                return;
            }
            _lyricsPollProc._buf = "";
            _lyricsPollProc.command = ["playerctl", "-p", root._lyricsPlayer.identity, "metadata", "xesam:asText"];
            _lyricsPollProc.running = true;
        }
    }

    Process {
        id: _lyricsPollProc
        property string _buf: ""

        stdout: SplitParser {
            onRead: data => {
                _lyricsPollProc._buf += data + "\n";
            }
        }
        onRunningChanged: {
            if (!running && _buf.length > 0) {
                if (_buf !== root._lastLyricsRaw) {
                    // 歌词内容变了，更新并停止轮询
                    root._lastLyricsRaw = _buf;
                    PanelState.lyricsLines = root._parseLrc(_buf);
                    PanelState.currentLyricIndex = -1;
                    PanelState.currentLyric = "";
                    _lyricsPollTimer.stop();
                }
                _buf = "";
            }
        }
    }

    function _parseLrc(raw) {
        let lines = raw.split("\n");
        let result = [];
        for (let line of lines) {
            let m = line.match(/^\[(\d{2}):(\d{2})\.(\d{2,3})\](.*)$/);
            if (m) {
                let time = parseInt(m[1]) * 60 + parseInt(m[2]) + parseInt(m[3]) / (m[3].length === 3 ? 1000 : 100);
                let text = m[4].trim();
                if (text.length > 0)
                    result.push({ "time": time, "text": text });
            }
        }
        result.sort((a, b) => a.time - b.time);
        return result;
    }

    function _syncLyric(position) {
        let lines = PanelState.lyricsLines;
        if (!lines || lines.length === 0) {
            PanelState.currentLyricIndex = -1;
            PanelState.currentLyric = "";
            return;
        }
        let idx = -1;
        for (let i = lines.length - 1; i >= 0; i--) {
            if (position >= lines[i].time) {
                idx = i;
                break;
            }
        }
        if (idx !== PanelState.currentLyricIndex) {
            PanelState.currentLyricIndex = idx;
            PanelState.currentLyric = idx >= 0 ? lines[idx].text : "";
        }
    }


    Timer {
        interval: 200
        running: root._lyricsPlayer !== null && PanelState.lyricsLines.length > 0
        repeat: true
        onTriggered: {
            if (root._lyricsPlayer)
                root._syncLyric(root._lyricsPlayer.position + PanelState.lyricsOffset);
        }
    }

    // ── 每个显示器生成一个 Bar ──
    Variants {
        model: Quickshell.screens

        delegate: Bar {}
    }

    Variants {
        model: Quickshell.screens

        delegate: BarRevealEdge {}
    }

    // ── 全局快捷键 ──
    GlobalShortcut {
        appid: "quickshell"
        name: "toggleBar"
        description: "Toggle bar visibility"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleBar();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "screenEffects"
        description: "Toggle screen effects panel"
        onPressed: {
            PanelState.calendarOpen = false;
            PanelState.mediaOpen = false;
            PanelState.toggleScreenEffects();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "powerMenu"
        description: "Toggle power menu"
        onPressed: {
            PanelState.closeAll();
            PanelState.togglePowerMenu();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "osdBrightness"
        description: "Show brightness OSD"
        onPressed: brightnessProc.running = true
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "osdVolume"
        description: "Show volume OSD"
        onPressed: volumeProc.running = true
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "launcher"
        description: "Toggle app launcher"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleLauncher();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "settings"
        description: "Toggle quick settings"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleSettings();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "keybindings"
        description: "Toggle keybindings cheat sheet"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleKeybindings();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "notes"
        description: "Toggle notes panel"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleNotes();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "journal"
        description: "Toggle journal log panel"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleJournal();
        }
    }

    GlobalShortcut {
        appid: "quickshell"
        name: "ai"
        description: "Toggle AI assistant panel"
        onPressed: {
            PanelState.closeAll();
            PanelState.toggleAi();
        }
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

    // ── OSD: 亮度检测 ──
    Process {
        id: brightnessProc

        command: ["brightnessctl", "-m"]

        stdout: SplitParser {
            onRead: data => {
                let parts = data.split(",");
                if (parts.length >= 4) {
                    let pct = parseInt(parts[3]) || 0;
                    PanelState.osdType = "brightness";
                    PanelState.osdValue = pct;
                    PanelState.osdIcon = pct > 50 ? "󰃠" : "󰃞";
                    PanelState.osdVisible = true;
                }
            }
        }
    }

    // ── 通知服务 ──
    NotificationServer {
        id: notifServer

        keepOnReload: true
        bodySupported: true
        bodyMarkupSupported: true
        imageSupported: true
        actionsSupported: true
        persistenceSupported: true
        onNotification: notification => {
            notification.tracked = true;
        }
    }

    Connections {
        function onObjectInsertedPost() {
            PanelState.notificationCount = notifServer.trackedNotifications.values.length;
        }

        function onObjectRemovedPost() {
            PanelState.notificationCount = notifServer.trackedNotifications.values.length;
        }

        target: notifServer.trackedNotifications
    }

    Connections {
        function onClearAllNotifications() {
            let vals = notifServer.trackedNotifications.values;
            for (let i = vals.length - 1; i >= 0; i--) {
                vals[i].dismiss();
            }
        }

        target: PanelState
    }

    // ── 全局面板（唯一实例）──
    ScreenEffectsPanel {}

    CalendarPanel {}

    MediaPanel {}

    PowerMenu {}

    OsdPanel {}

    NotificationPanel {
        notifServer: notifServer
    }

    NotificationToast {
        notifServer: notifServer
    }

    AppLauncher {}

    QuickSettings {}

    Variants {
        model: Quickshell.screens

        delegate: HotEdge {}
    }

    ClipboardPanel {}

    KeybindingsPanel {}

    NetworkPanel {}

    // ── 新增面板 ──
    JournalPanel {}

    NotesPanel {}

    AiPanel {}

    // ── 桌面浮动组件（每屏一个）──
    Variants {
        model: Quickshell.screens
        delegate: AnalogClock {}
    }

    Variants {
        model: Quickshell.screens
        delegate: PomodoroTimer {}
    }

    // ── 音频频谱（cava 单例 + 每屏渲染）──
    CavaService {}

    Variants {
        model: Quickshell.screens
        delegate: AudioVisualizer {}
    }
}
