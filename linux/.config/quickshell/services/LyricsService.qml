import "../theme"
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris

// 歌词服务 — 跟随当前播放器，轮询 LRC 文本并按播放进度同步当前行。
// 状态全部写入 LyricsState.lyrics*（bar 模块 / 媒体面板读取）。
Scope {
    id: root

    // 选最先 isPlaying 的播放器；否则沿用上一个活跃的；再否则第一个
    property var _lyricsPlayer: {
        let ps = Mpris.players.values;
        for (let i = 0; i < ps.length; i++) {
            if (ps[i].isPlaying) {
                MediaState.lastActivePlayer = ps[i];
                return ps[i];
            }
        }
        if (MediaState.lastActivePlayer && ps.indexOf(MediaState.lastActivePlayer) >= 0)
            return MediaState.lastActivePlayer;
        return ps.length > 0 ? ps[0] : null;
    }
    property string _lyricsTrackKey: _lyricsPlayer ? (_lyricsPlayer.identity + "|" + _lyricsPlayer.trackTitle) : ""

    property string _lastLyricsRaw: "" // 上一次成功的歌词原文，用于双缓冲比较

    on_LyricsTrackKeyChanged: {
        if (_lyricsTrackKey && _lyricsPlayer) {
            LyricsState.lyricsLines = [];
            LyricsState.currentLyricIndex = -1;
            LyricsState.currentLyric = "";
            LyricsState.lyricsTrackId = _lyricsTrackKey;
            // 重置尝试计数，启动轮询等歌词数据
            _lyricsPollTimer._attempts = 0;
            _lyricsPollTimer.restart();
        } else {
            _lyricsPollTimer.stop();
            LyricsState.lyricsLines = [];
            LyricsState.currentLyricIndex = -1;
            LyricsState.currentLyric = "";
            LyricsState.lyricsTrackId = "";
        }
    }

    Timer {
        id: _lyricsPollTimer
        interval: 500
        repeat: true
        property int _attempts: 0
        // 最多轮询 20 次（约 10 秒），避免曲目无歌词时无限调用 playerctl
        readonly property int _maxAttempts: 20
        onTriggered: {
            if (!root._lyricsPlayer || _attempts >= _maxAttempts) {
                stop();
                return;
            }
            _attempts++;
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
                    LyricsState.lyricsLines = root._parseLrc(_buf);
                    LyricsState.currentLyricIndex = -1;
                    LyricsState.currentLyric = "";
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
        let lines = LyricsState.lyricsLines;
        if (!lines || lines.length === 0) {
            LyricsState.currentLyricIndex = -1;
            LyricsState.currentLyric = "";
            return;
        }
        let idx = -1;
        for (let i = lines.length - 1; i >= 0; i--) {
            if (position >= lines[i].time) {
                idx = i;
                break;
            }
        }
        if (idx !== LyricsState.currentLyricIndex) {
            LyricsState.currentLyricIndex = idx;
            LyricsState.currentLyric = idx >= 0 ? lines[idx].text : "";
        }
    }

    Timer {
        interval: 200
        running: root._lyricsPlayer !== null && LyricsState.lyricsLines.length > 0
        repeat: true
        onTriggered: {
            if (root._lyricsPlayer)
                root._syncLyric(root._lyricsPlayer.position + LyricsState.lyricsOffset);
        }
    }
}
