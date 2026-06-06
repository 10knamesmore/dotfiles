pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Services.Mpris

// 当前活跃媒体播放器 — 收口全 shell 的 player 选取逻辑。
Singleton {
    id: root

    // 上次正在播放的 player（记忆，用于播放暂停后仍显示该播放器）
    property var _lastPlaying: null

    // 当前正在播放者 — 纯计算，无副作用
    readonly property var playingPlayer: {
        let ps = Mpris.players.values;
        for (let i = 0; i < ps.length; i++)
            if (ps[i].isPlaying)
                return ps[i];
        return null;
    }

    // 副作用移出 binding：有人开始播放就记住它
    onPlayingPlayerChanged: if (playingPlayer)
        _lastPlaying = playingPlayer

    // 对外暴露：当前应展示的 player
    readonly property var activePlayer: {
        if (playingPlayer)
            return playingPlayer;
        let ps = Mpris.players.values;
        if (_lastPlaying && ps.indexOf(_lastPlaying) >= 0)
            return _lastPlaying;
        return ps.length > 0 ? ps[0] : null;
    }

    // MPRIS position 默认不响应式（省 CPU），需播放时周期 emit positionChanged()
    // 才能让进度条/时间绑定刷新。收口在此，bar 与面板的进度一处驱动。
    Timer {
        running: root.activePlayer && root.activePlayer.isPlaying
        interval: 1000
        repeat: true
        onTriggered: root.activePlayer.positionChanged()
    }
}
