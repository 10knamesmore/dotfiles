// monitorModel — 显示器管理的纯逻辑核心。
//
// 设计要点：本文件不依赖 QML/Quickshell，全是纯函数，可被 node 直接跑单测
// （见 monitorModel.test.js），也能被 QML 以 `.import` 方式引用。文件末尾的
// module.exports 守卫只在 node 下生效，QML 引擎里 typeof module === "undefined"。
//
// 「布局(layout)」对象统一形状：
//   { name, enabled, mode:"WxH@R", x, y, scale, transform, mirror }
// 「ipc」对象 = hyprctl monitors -j 的单条（即 HyprlandMonitor.lastIpcObject）。
//
// 注意：不加 `.pragma library` —— 那会让 node 的 require 解析失败。纯函数库无状态，
// QML 以 `import "lib/monitorModel.js" as MM` 引用即可（各引用方各持一份副本无妨）。

// 显示器的稳定标识：同一块物理屏跨接口/重启都不变。
// 优先 description（Hyprland 已把 EDID 厂商/型号/序列拼进去），其次 make|model|serial，
// 最后才兜底 name —— name(如 DP-3) 会随接口变动，不能作记忆键。
function stableId(ipc) {
    var desc = (ipc.description || "").trim();
    if (desc !== "")
        return desc;
    var make = ipc.make || "", model = ipc.model || "", serial = ipc.serial || "";
    if (make !== "" || model !== "" || serial !== "")
        return make + "|" + model + "|" + serial;
    return ipc.name || "";
}

// 组合签名：在场显示器稳定标识集合排序后拼接，与插入顺序无关。
function signature(ipcList) {
    return ipcList.map(stableId).sort().join("||");
}

// hyprctl keyword monitor 的配置串。
function serializeMonitorString(layout) {
    if (!layout.enabled)
        return layout.name + ",disable";
    var s = layout.name + "," + layout.mode + "," + layout.x + "x" + layout.y + "," + layout.scale;
    if (layout.transform && layout.transform > 0)
        s += ",transform," + layout.transform;
    if (layout.mirror)
        s += ",mirror," + layout.mirror;
    return s;
}

// 开机用的 lua 行（hyprland.lua dofile 加载）。复用仓库既有的 hl.monitor({...}) 写法。
function monitorLuaLine(layout) {
    if (!layout.enabled)
        return 'hl.monitor({ output = "' + layout.name + '", disabled = true })';
    var s = 'hl.monitor({ output = "' + layout.name + '", mode = "' + layout.mode
        + '", position = "' + layout.x + "x" + layout.y + '", scale = ' + layout.scale;
    if (layout.transform && layout.transform > 0)
        s += ", transform = " + layout.transform;
    if (layout.mirror)
        s += ', mirror = "' + layout.mirror + '"';
    return s + " })";
}

// 整个 monitors.local.lua 文件内容。
function buildLocalLua(layouts) {
    var head = "-- 自动生成：由 QuickShell MonitorService 在每次应用显示器配置后回写。\n"
        + "-- 手改会被覆盖；hyprland.lua 开机 dofile 本文件恢复上次布局（无闪烁）。\n";
    return head + layouts.map(monitorLuaLine).join("\n") + "\n";
}

// ── 可用模式解析（供 UI 下拉）──
// availableModes 形如 ["3840x2160@60.00000Hz", "2560x1600@59.99Hz", ...]
// 返回 [{ res:"3840x2160", rates:[60, 30] }]，分辨率按像素面积降序、刷新率降序。
function modesByResolution(availableModes) {
    var byRes = {};
    (availableModes || []).forEach(function (m) {
        var mt = m.match(/^(\d+)x(\d+)@([\d.]+)/);
        if (!mt)
            return;
        var res = mt[1] + "x" + mt[2];
        var rate = Math.round(parseFloat(mt[3]));
        if (!byRes[res])
            byRes[res] = { res: res, w: parseInt(mt[1]), h: parseInt(mt[2]), rates: {} };
        byRes[res].rates[rate] = true;
    });
    return Object.keys(byRes).map(function (k) {
        var e = byRes[k];
        return { res: e.res, rates: Object.keys(e.rates).map(Number).sort(function (a, b) { return b - a; }), _area: e.w * e.h };
    }).sort(function (a, b) { return b._area - a._area; });
}

// 由 ipc 的当前状态构造 mode 串（刷新率取整，hyprland keyword 会就近匹配）。
function formatMode(width, height, rate) {
    return width + "x" + height + "@" + Math.round(rate);
}

// 由 ipc 当前状态构造一条布局（用于快照/默认）。
function layoutFromIpc(ipc) {
    return {
        name: ipc.name,
        enabled: !(ipc.disabled),
        mode: formatMode(ipc.width, ipc.height, ipc.refreshRate),
        x: ipc.x,
        y: ipc.y,
        scale: ipc.scale,
        transform: ipc.transform || 0,
        mirror: null
    };
}

// ── 存储 ──
function emptyStore() {
    return { version: 1, profiles: {} };
}

// 解析 + 健壮化。坏 JSON / 缺字段 → 回退到空 store（调用方负责备份损坏文件）。
function migrateStore(raw) {
    var obj;
    try {
        obj = JSON.parse(raw);
    } catch (e) {
        return emptyStore();
    }
    if (!obj || typeof obj !== "object")
        return emptyStore();
    return { version: 1, profiles: (obj.profiles && typeof obj.profiles === "object") ? obj.profiles : {} };
}

function getProfile(store, sig) {
    return (store && store.profiles) ? store.profiles[sig] : undefined;
}

function putProfile(store, sig, profile) {
    var next = { version: 1, profiles: {} };
    var src = (store && store.profiles) ? store.profiles : {};
    Object.keys(src).forEach(function (k) { next.profiles[k] = src[k]; });
    next.profiles[sig] = profile;
    return next;
}

// 存档(按稳定 id) → 可应用的布局列表(用当前 name)。
// 同一物理屏即便换了接口名，也能凭稳定 id 还原到现在的 name。
function layoutFromProfile(profile, currentIpcList) {
    var out = [];
    currentIpcList.forEach(function (ipc) {
        var sid = stableId(ipc);
        var saved = profile.monitors[sid];
        if (saved) {
            var l = {};
            Object.keys(saved).forEach(function (k) { l[k] = saved[k]; });
            l.name = ipc.name; // 映射回当前接口名
            out.push(l);
        }
    });
    return out;
}

// 布局列表 + 当前 ipc + 主屏 name → 存档(按稳定 id)。
function profileFromLayouts(layouts, currentIpcList, primaryName) {
    var nameToSid = {};
    currentIpcList.forEach(function (ipc) { nameToSid[ipc.name] = stableId(ipc); });
    var monitors = {};
    layouts.forEach(function (l) {
        var sid = nameToSid[l.name];
        if (sid)
            monitors[sid] = l;
    });
    return { primary: nameToSid[primaryName] || null, monitors: monitors };
}

if (typeof module !== "undefined" && module.exports) {
    module.exports = {
        stableId: stableId,
        signature: signature,
        serializeMonitorString: serializeMonitorString,
        monitorLuaLine: monitorLuaLine,
        buildLocalLua: buildLocalLua,
        modesByResolution: modesByResolution,
        formatMode: formatMode,
        layoutFromIpc: layoutFromIpc,
        emptyStore: emptyStore,
        migrateStore: migrateStore,
        getProfile: getProfile,
        putProfile: putProfile,
        layoutFromProfile: layoutFromProfile,
        profileFromLayouts: profileFromLayouts
    };
}
