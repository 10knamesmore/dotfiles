import "../state"
import QtQuick
import Quickshell

// 读取当前播放器的 MPRIS 歌词元数据，合并原文、逐字、翻译与罗马音到 LyricsState。
// 播放时按帧同步歌词时间；暂停、跳转和偏移变化立即同步，不另起进程查询歌词。
Scope {
    id: root

    readonly property var player: MediaService.activePlayer
    property string lastLyrics: ""

    function refreshLyrics() {
        const metadata = player ? player.metadata : {};
        const raw = [metadata["mineral:words"] || "", metadata["xesam:asText"] || "",
            metadata["mineral:translation"] || "", metadata["mineral:romanization"] || ""];
        const serialized = JSON.stringify(raw);
        if (LyricsState.lyricsTrackId === MediaService.activeTrackKey && lastLyrics === serialized)
            return;

        lastLyrics = serialized;
        const built = _buildLines(raw[0], raw[1], raw[2], raw[3]);
        LyricsState.lyricsTrackId = MediaService.activeTrackKey;
        LyricsState.lyricsLines = built.lines;
        LyricsState.hasWords = built.hasWords;
        LyricsState.hasTranslation = built.lines.some(line => line.translation.length > 0);
        LyricsState.hasRomanization = built.lines.some(line => line.romanization.length > 0);
        LyricsState.currentLyricIndex = -1;
        LyricsState.currentLyric = "";
        syncPosition();
        console.info("[lyrics] metadata updated:", MediaService.activeTrackKey,
            "lines:", built.lines.length, "word timing:", built.hasWords);
    }

    function syncPosition() {
        _syncLyric(player ? player.position + LyricsState.lyricsOffset : 0);
    }

    Component.onCompleted: refreshLyrics()

    Connections {
        target: MediaService
        function onActiveTrackKeyChanged() { Qt.callLater(root.refreshLyrics); }
    }

    Connections {
        target: root.player
        function onMetadataChanged() { Qt.callLater(root.refreshLyrics); }
        function onPositionChanged() { root.syncPosition(); }
        function onIsPlayingChanged() { root.syncPosition(); }
    }

    Connections {
        target: LyricsState
        function onLyricsOffsetChanged() { root.syncPosition(); }
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

    // position getter 在本地计算播放位置；逐字扫亮直接跟随渲染帧，不用定时器插值。
    FrameAnimation {
        running: root.player !== null && root.player.isPlaying && LyricsState.lyricsLines.length > 0
        onTriggered: root.syncPosition()
    }
}
