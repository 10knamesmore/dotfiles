import "../state"
import "../screen-effects/lib/shaderGen.js" as SG
import QtQuick
import Quickshell
import Quickshell.Io

// 屏幕效果常驻服务（仿 MonitorService：非单例 Scope，在 shell.qml 实例化一次）。
// 负责：读写状态 JSON、生成 GLSL、热加载进 Hyprland、转发背光调节。
// 数据写入 ScreenEffectsState 供 UI 读取。
//
// 为什么整条链路在 QML 而不是 shell 脚本：shader 生成的唯一消费者就是这个面板，
// 过去却绕成「QML → spawn sh 写 JSON → spawn bash → jq/awk/sed → hyprctl」，
// 每次 apply 约 12 个进程。收进来后只剩 hyprctl 一次 spawn，JSON 是原生的、
// 数字格式化不经 locale、写盘是原子的。scripts/linux/hypr/screen_effects.sh
// 现在只剩 brightness 一个子命令（brightnessctl + ddcutil 封装）。
Scope {
    id: root

    readonly property string _home: Quickshell.env("HOME")
    // 状态落 ~/.local/state 而非 ~/.cache：预设选择要跨重启保留，是配置不是缓存，
    // cache 随时可能被清理。与 MonitorService 的 monitors.local.lua 同目录。
    readonly property string _statePath: _home + "/.local/state/hypr/screen-effects.json"
    // 生成的 GLSL 是纯派生物，留在 cache 合适。
    readonly property string _shaderPath: _home + "/.cache/hypr/screen-effects.glsl"
    readonly property string _bodyPath: _home + "/.config/quickshell/screen-effects/screen-effects.frag"
    readonly property string _brightnessScript: _home + "/dotfiles/.gen/scripts/hypr/screen_effects.sh"

    // toggle 关闭时的备份值，随状态一起持久化 —— 否则重启后第一次 toggle 只能
    // 回到硬编码默认值，把用户调好的参数丢了。
    property var _bak: ({ warmth: 60, grain: 0, grainSize: 50, shadowBoost: 40 })

    function _current() {
        return {
            warmth: ScreenEffectsState.warmth,
            grain: ScreenEffectsState.grain,
            grainSize: ScreenEffectsState.grainSize,
            shadowBoost: ScreenEffectsState.shadowBoost
        };
    }

    function _writeState(p) {
        ScreenEffectsState.warmth = p.warmth;
        ScreenEffectsState.grain = p.grain;
        ScreenEffectsState.grainSize = p.grainSize;
        ScreenEffectsState.shadowBoost = p.shadowBoost;
    }

    // ── 应用：生成 GLSL → 写盘 → 热加载 ──
    function _apply(p) {
        root._writeState(p);
        stateWriter.setText(JSON.stringify({
            warmth: p.warmth,
            grain: p.grain,
            grain_size: p.grainSize,
            shadow_boost: p.shadowBoost,
            bak: root._bak
        }, null, 2));

        if (!SG.needsShader(p)) {
            root._setShader("");
            return;
        }

        var body = bodyFile.text();
        if (!body) {
            console.warn("ScreenEffectsService: 读不到 shader 主体", root._bodyPath);
            return;
        }
        // setText 是异步的（FileView.blockWrites 默认 false），所以 hyprctl 必须挂在
        // onSaved 上 —— 紧跟在 setText 后面调会让 Hyprland 读到上一版文件。
        shaderWriter.setText(SG.buildShader(p, body));
    }

    // 写 decoration:screen_shader。空串 = 卸载（渲染侧对空串和哨兵 [[EMPTY]] 都走
    // 清除分支，见 Hyprland/src/render/OpenGL.cpp 的 applyScreenShader）。
    //
    // 必须用 hyprctl eval 跑 lua，不能用 hyprctl keyword：Hyprland 的 lua 配置模式
    // （non-legacy parser）下 keyword 被禁用，只回一句 "keyword can't work with
    // non-legacy parsers. Use eval."，且退出码仍是 0 —— 静默失效。同 MonitorService。
    //
    // 路径没变时重设也会重新读盘：hl.config 解析成功后无条件 scheduleRefresh，
    // 所以重写同名文件再 eval 一次就能热更新，不需要先清空再设置。
    function _setShader(path) {
        applyProc.command = ["hyprctl", "eval", 'hl.config({ decoration = { screen_shader = "' + path + '" } })'];
        applyProc.running = true;
    }

    // ── 初始加载 ──
    function _load() {
        var raw = stateFile.text();
        if (!raw)
            return; // 首次运行：文件不存在，用 State 里的默认值
        try {
            var o = JSON.parse(raw);
            root._writeState({
                warmth: o.warmth || 0,
                grain: o.grain || 0,
                grainSize: o.grain_size !== undefined ? o.grain_size : 50,
                shadowBoost: o.shadow_boost !== undefined ? o.shadow_boost : 40
            });
            if (o.bak)
                root._bak = o.bak;
        } catch (e) {
            console.warn("ScreenEffectsService: 状态文件解析失败，用默认值", e);
        }
    }

    // ── UI 意图回调 ──
    Connections {
        target: ScreenEffectsState

        function onApplyRequested(warmth, grain, grainSize, shadowBoost) {
            root._apply({ warmth: warmth, grain: grain, grainSize: grainSize, shadowBoost: shadowBoost });
        }

        function onToggleRequested() {
            if (ScreenEffectsState.effectsActive) {
                root._bak = root._current();
                root._apply({ warmth: 0, grain: 0, grainSize: ScreenEffectsState.grainSize, shadowBoost: ScreenEffectsState.shadowBoost });
            } else {
                root._apply(root._bak);
            }
        }

        function onBrightnessRequested(value) {
            ScreenEffectsState.brightness = value;
            brightnessSetter.command = [root._brightnessScript, "brightness", String(value)];
            brightnessSetter.running = true;
        }

        function onRefreshRequested() {
            brightnessReader.command = ["brightnessctl", "-m"];
            brightnessReader.running = true;
        }
    }

    // ── 文件 ──
    // 读：同步取一次即可（本服务是唯一写入者，无需 watchChanges 回环监听）。
    FileView {
        id: stateFile
        path: root._statePath
        blockAllReads: true
        printErrors: false // 首次运行文件不存在，属正常
    }
    FileView {
        id: bodyFile
        path: root._bodyPath
        blockAllReads: true
        printErrors: false
    }

    // 写：atomicWrites 走「临时文件 + rename」，Hyprland 不会读到写了一半的 shader。
    FileView {
        id: stateWriter
        path: root._statePath
        atomicWrites: true
        printErrors: false
    }
    FileView {
        id: shaderWriter
        path: root._shaderPath
        atomicWrites: true
        printErrors: false
        onSaved: root._setShader(root._shaderPath)
        onSaveFailed: err => console.warn("ScreenEffectsService: shader 写入失败", err)
    }

    // ── 进程 ──
    Process {
        id: mkdirProc
        // 必须等 mkdir 退出再 apply：FileView 往不存在的目录写会直接失败，
        // 而 Process.running = true 只是排队、不阻塞。
        onExited: {
            root._load();
            // 开机恢复上次效果。过去由 hyprland.lua 的 exec_cmd 调脚本负责，现在归这里 ——
            // quickshell 本来就是 exec_once 拉起的，少一个进程和一份重复逻辑。
            root._apply(root._current());
        }
    }
    Process {
        id: brightnessSetter
    }
    Process {
        id: brightnessReader
        // brightnessctl -m 输出：device,class,current,percent,max —— 取第 4 段的百分比
        stdout: SplitParser {
            onRead: data => {
                let parts = data.split(",");
                if (parts.length >= 4)
                    ScreenEffectsState.brightness = parseInt(parts[3]) || 100;
            }
        }
    }
    Process {
        id: applyProc
        // hyprctl eval 成功打 "ok"、失败打 "error: ..."，退出码均为 0，故按 stdout 判错。
        // 注意 "ok" 只代表配置项写入成功 —— GLSL 的编译/链接发生在下一帧渲染，失败信息
        // 只落在 Hyprland 日志和屏幕错误浮层，这里感知不到。
        stdout: StdioCollector {
            onStreamFinished: {
                if (text.indexOf("error") >= 0 || text.indexOf("Error") >= 0)
                    console.warn("ScreenEffectsService: hyprctl eval 失败:", text.trim());
            }
        }
    }

    Component.onCompleted: {
        // 目录就绪后由 mkdirProc.onExited 接着做加载与开机应用。
        mkdirProc.command = ["mkdir", "-p", root._home + "/.local/state/hypr", root._home + "/.cache/hypr"];
        mkdirProc.running = true;
    }
}
