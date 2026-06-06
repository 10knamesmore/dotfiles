import "../state"
import QtQuick
import Quickshell
import Quickshell.Io

// 歌词服务 — 跟随 MediaService.activePlayer，一次 playerctl --format 取多路歌词：
//   mineral:words(逐字 JSON) / xesam:asText(行级原文) / mineral:translation / mineral:romanization
// 渐进降级：有逐字用逐字，无则回退行级 asText；翻译/罗马音按时间轴合并为附加层。
// 解析结果写入 LyricsState.*（bar 模块 / 媒体面板读取）。
Scope {
    id: root

    // 多 key 一次取，自定义 key 缺失时为空串（不报错）；分隔符取歌词内容不会出现的串
    readonly property string _sep: "@@MFS@@"
    readonly property string _fmt: "{{mineral:words}}" + _sep + "{{xesam:asText}}" + _sep + "{{mineral:translation}}" + _sep + "{{mineral:romanization}}"

    readonly property var _lyricsPlayer: MediaService.activePlayer
    property string _lyricsTrackKey: _lyricsPlayer ? (_lyricsPlayer.identity + "|" + _lyricsPlayer.trackTitle) : ""
    property string _lastRaw: "" // 上一次成功的原始多路文本，双缓冲比较

    on_LyricsTrackKeyChanged: {
        if (_lyricsTrackKey && _lyricsPlayer) {
            _resetLyrics();
            LyricsState.lyricsTrackId = _lyricsTrackKey;
            _pollTimer._attempts = 0;
            _pollTimer.restart();
        } else {
            _pollTimer.stop();
            _resetLyrics();
            LyricsState.lyricsTrackId = "";
        }
    }

    function _resetLyrics() {
        LyricsState.lyricsLines = [];
        LyricsState.currentLyricIndex = -1;
        LyricsState.currentLyric = "";
        LyricsState.hasWords = false;
        LyricsState.hasTranslation = false;
        LyricsState.hasRomanization = false;
    }

    // ── 解析：行级 LRC [mm:ss.xx]text → [{time(秒), text}] ──
    function _parseLrc(raw) {
        let result = [];
        for (let line of (raw || "").split("\n")) {
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

    // ── 解析：逐字 JSON → [{start(ms), words:[{start,duration,text}], text}] ──
    function _parseWords(json) {
        if (!json || json.trim().length === 0)
            return [];
        try {
            let arr = JSON.parse(json);
            if (!Array.isArray(arr))
                return [];
            return arr.map(line => {
                let words = (line.words || []).map(w => ({ "start": w.start, "duration": w.duration, "text": w.text }));
                return { "start": line.start, "words": words, "text": words.map(w => w.text).join("") };
            });
        } catch (e) {
            return [];
        }
    }

    // 把附加轨（翻译/罗马音）按时间轴就近合并到 lines 的对应行
    function _mergeAux(lines, aux, field) {
        for (let a of aux) {
            let best = -1, bestDiff = 1e9;
            for (let i = 0; i < lines.length; i++) {
                let d = Math.abs(lines[i].time - a.time);
                if (d < bestDiff) { bestDiff = d; best = i; }
            }
            if (best >= 0 && bestDiff < 0.5)
                lines[best][field] = a.text;
        }
    }

    // 组装统一行模型，返回 { lines, hasWords }
    function _buildLines(wordsJson, asText, translation, romanization) {
        let lines, hasWords;
        let wl = _parseWords(wordsJson);
        if (wl.length > 0) {
            hasWords = true;
            lines = wl.map(w => ({ "time": w.start / 1000, "text": w.text, "words": w.words, "translation": "", "romanization": "" }));
        } else {
            hasWords = false;
            lines = _parseLrc(asText).map(l => ({ "time": l.time, "text": l.text, "words": [], "translation": "", "romanization": "" }));
        }
        _mergeAux(lines, _parseLrc(translation), "translation");
        _mergeAux(lines, _parseLrc(romanization), "romanization");
        return { "lines": lines, "hasWords": hasWords };
    }

    Timer {
        id: _pollTimer
        interval: 500
        repeat: true
        property int _attempts: 0
        readonly property int _maxAttempts: 20 // 约 10 秒，避免无歌词曲目无限轮询
        onTriggered: {
            if (!root._lyricsPlayer || _attempts >= _maxAttempts) {
                stop();
                return;
            }
            _attempts++;
            _pollProc.command = ["playerctl", "-p", root._lyricsPlayer.identity, "metadata", "--format", root._fmt];
            _pollProc.running = true;
        }
    }

    Process {
        id: _pollProc

        stdout: StdioCollector {
            onStreamFinished: {
                let raw = this.text;
                if (!raw || raw.trim().length === 0)
                    return;
                if (raw === root._lastRaw)
                    return; // 内容没变，继续轮询等待数据
                let parts = raw.split(root._sep);
                let built = root._buildLines(parts[0] || "", parts[1] || "", parts[2] || "", parts[3] || "");
                if (built.lines.length === 0)
                    return; // 还没拿到任何歌词，继续轮询
                root._lastRaw = raw;
                LyricsState.lyricsLines = built.lines;
                LyricsState.hasWords = built.hasWords;
                LyricsState.hasTranslation = built.lines.some(l => l.translation && l.translation.length > 0);
                LyricsState.hasRomanization = built.lines.some(l => l.romanization && l.romanization.length > 0);
                LyricsState.currentLyricIndex = -1;
                LyricsState.currentLyric = "";
                _pollTimer.stop();
            }
        }
    }

    // ── 同步当前行 + 当前时间（逐字 wipe 用）──
    function _syncLyric(positionSec) {
        LyricsState.currentTimeMs = positionSec * 1000;
        let lines = LyricsState.lyricsLines;
        if (!lines || lines.length === 0) {
            LyricsState.currentLyricIndex = -1;
            LyricsState.currentLyric = "";
            return;
        }
        let idx = -1;
        for (let i = lines.length - 1; i >= 0; i--) {
            if (positionSec >= lines[i].time) {
                idx = i;
                break;
            }
        }
        if (idx !== LyricsState.currentLyricIndex) {
            LyricsState.currentLyricIndex = idx;
            LyricsState.currentLyric = idx >= 0 ? lines[idx].text : "";
        }
    }

    // 100ms 直接读 position 实时值（getter 总返回当前值，不依赖 positionChanged），
    // 逐字高亮要够细，故比行级歌词更快。
    Timer {
        interval: 100
        running: root._lyricsPlayer !== null && LyricsState.lyricsLines.length > 0
        repeat: true
        onTriggered: {
            if (root._lyricsPlayer)
                root._syncLyric(root._lyricsPlayer.position + LyricsState.lyricsOffset);
        }
    }
}
