// monitorModel 纯逻辑单测。纯 JS、无 QML 依赖，用 node 直接跑：
//   node --test monitorModel.test.js
const test = require("node:test");
const assert = require("node:assert/strict");
const M = require("./monitorModel.js");

// ── 稳定标识 ──────────────────────────────────────────────
test("stableId 优先用 description（含 EDID 厂商/型号/序列）", () => {
    assert.equal(M.stableId({ name: "DP-3", description: "Dell U2720Q ABC123", make: "Dell" }), "Dell U2720Q ABC123");
});

test("stableId 无 description 时退回 make|model|serial", () => {
    assert.equal(M.stableId({ name: "DP-3", description: "", make: "Dell", model: "U2720Q", serial: "ABC123" }), "Dell|U2720Q|ABC123");
});

test("stableId 全空时兜底用 name", () => {
    assert.equal(M.stableId({ name: "eDP-1", description: "" }), "eDP-1");
});

// ── 组合签名 ──────────────────────────────────────────────
test("signature 与显示器顺序无关（排序后拼接）", () => {
    const a = [{ name: "eDP-1", description: "BOE x" }, { name: "DP-3", description: "Dell y" }];
    const b = [{ name: "DP-3", description: "Dell y" }, { name: "eDP-1", description: "BOE x" }];
    assert.equal(M.signature(a), M.signature(b));
});

test("signature 区分不同组合", () => {
    const one = [{ name: "eDP-1", description: "BOE x" }];
    const two = [{ name: "eDP-1", description: "BOE x" }, { name: "DP-3", description: "Dell y" }];
    assert.notEqual(M.signature(one), M.signature(two));
});

// ── monitor 串序列化 ──────────────────────────────────────
test("serializeMonitorString 启用：NAME,MODE,XxY,SCALE", () => {
    assert.equal(
        M.serializeMonitorString({ name: "eDP-1", enabled: true, mode: "2560x1600@60", x: 640, y: 2160, scale: 1, transform: 0 }),
        "eDP-1,2560x1600@60,640x2160,1"
    );
});

test("serializeMonitorString 带 transform", () => {
    assert.equal(
        M.serializeMonitorString({ name: "DP-3", enabled: true, mode: "3840x2160@60", x: 0, y: 0, scale: 1.5, transform: 1 }),
        "DP-3,3840x2160@60,0x0,1.5,transform,1"
    );
});

test("serializeMonitorString 禁用：NAME,disable", () => {
    assert.equal(M.serializeMonitorString({ name: "eDP-1", enabled: false }), "eDP-1,disable");
});

// ── 开机 lua 行 ───────────────────────────────────────────
test("monitorLuaLine 启用", () => {
    assert.equal(
        M.monitorLuaLine({ name: "eDP-1", enabled: true, mode: "2560x1600@60", x: 640, y: 2160, scale: 1, transform: 0 }),
        'hl.monitor({ output = "eDP-1", mode = "2560x1600@60", position = "640x2160", scale = 1 })'
    );
});

test("monitorLuaLine 带 transform", () => {
    assert.equal(
        M.monitorLuaLine({ name: "DP-3", enabled: true, mode: "3840x2160@60", x: 0, y: 0, scale: 1, transform: 3 }),
        'hl.monitor({ output = "DP-3", mode = "3840x2160@60", position = "0x0", scale = 1, transform = 3 })'
    );
});

test("monitorLuaLine 禁用", () => {
    assert.equal(M.monitorLuaLine({ name: "eDP-1", enabled: false }), 'hl.monitor({ output = "eDP-1", disabled = true })');
});

test("buildLocalLua 含多行且带生成头注释", () => {
    const lua = M.buildLocalLua([
        { name: "DP-3", enabled: true, mode: "3840x2160@60", x: 0, y: 0, scale: 1, transform: 0 },
        { name: "eDP-1", enabled: false }
    ]);
    assert.match(lua, /自动生成/);
    assert.match(lua, /hl\.monitor\(\{ output = "DP-3"/);
    assert.match(lua, /hl\.monitor\(\{ output = "eDP-1", disabled = true \}\)/);
});

// ── 存储迁移/健壮性 ──────────────────────────────────────
test("migrateStore 解析坏 JSON 返回空 store", () => {
    assert.deepEqual(M.migrateStore("{ not json"), { version: 1, profiles: {} });
});

test("migrateStore 缺字段补全", () => {
    assert.deepEqual(M.migrateStore('{"version":1}'), { version: 1, profiles: {} });
});

test("migrateStore 保留合法 profiles", () => {
    const raw = '{"version":1,"profiles":{"sig":{"primary":"x","monitors":{}}}}';
    assert.equal(M.migrateStore(raw).profiles.sig.primary, "x");
});

// ── 存档↔布局映射（按稳定 id，apply 时映射回当前 name）──
test("layoutFromProfile 用当前 name 还原存档（稳定 id 可能换了接口名）", () => {
    const profile = {
        primary: "Dell U2720Q ABC",
        monitors: {
            "Dell U2720Q ABC": { name: "DP-3", enabled: true, mode: "3840x2160@60", x: 0, y: 0, scale: 1, transform: 0 }
        }
    };
    // 同一台 Dell 现在挂在 DP-5
    const current = [{ name: "DP-5", description: "Dell U2720Q ABC" }];
    const layouts = M.layoutFromProfile(profile, current);
    assert.equal(layouts.length, 1);
    assert.equal(layouts[0].name, "DP-5"); // 映射到当前 name
    assert.equal(layouts[0].mode, "3840x2160@60");
});

test("profileFromLayouts 按稳定 id 存档", () => {
    const layouts = [{ name: "DP-3", enabled: true, mode: "3840x2160@60", x: 0, y: 0, scale: 1, transform: 0 }];
    const current = [{ name: "DP-3", description: "Dell U2720Q ABC" }];
    const profile = M.profileFromLayouts(layouts, current, "DP-3");
    assert.ok(profile.monitors["Dell U2720Q ABC"]);
    assert.equal(profile.primary, "Dell U2720Q ABC"); // primary 也存稳定 id
});

// ── 色彩 / HDR ────────────────────────────────────────────
const BASE = { name: "DP-3", enabled: true, mode: "3840x2160@60", x: 0, y: 0, scale: 1, transform: 0 };

test("colorOf 缺 color 子对象时给 srgb + Hyprland 原生默认（兼容老存档）", () => {
    const def = { cm: "srgb", sdrMaxLuminance: 80, sdrBrightness: 1, sdrSaturation: 1 };
    assert.deepEqual(M.colorOf({ name: "DP-3" }), def);
    assert.deepEqual(M.colorOf(undefined), def);
});

test("colorOf 部分字段缺失时逐项兜底", () => {
    assert.deepEqual(M.colorOf({ color: { cm: "hdr" } }), { cm: "hdr", sdrMaxLuminance: 80, sdrBrightness: 1, sdrSaturation: 1 });
});

test("monitorLuaLine 输出 sdr_max_luminance（下划线命名，与 sdrbrightness 不同）", () => {
    // SDR 白点：决定 SDR 内容映射到的绝对 nits 上界，是「HDR 下桌面发暗」的真正旋钮
    assert.equal(
        M.monitorLuaLine({ ...BASE, color: { cm: "hdr", sdrMaxLuminance: 250 } }),
        'hl.monitor({ output = "DP-3", mode = "3840x2160@60", position = "0x0", scale = 1, bitdepth = 10, cm = "hdr", sdr_max_luminance = 250 })'
    );
});

test("monitorLuaLine 白点等于默认 80 时不输出", () => {
    assert.equal(
        M.monitorLuaLine({ ...BASE, color: { cm: "hdr", sdrMaxLuminance: 80 } }),
        'hl.monitor({ output = "DP-3", mode = "3840x2160@60", position = "0x0", scale = 1, bitdepth = 10, cm = "hdr" })'
    );
});

// ipc→color 的唯一转换入口。曾经 MonitorService 和 layoutFromIpc 各写一份，
// 加 sdrMaxLuminance 时只改了一份，面板读到的白点永远是默认 80。
test("colorFromIpc 读全四个色彩字段", () => {
    assert.deepEqual(
        M.colorFromIpc({ colorManagementPreset: "hdr", sdrMaxLuminance: 250, sdrBrightness: 1.1, sdrSaturation: 1.2 }),
        { cm: "hdr", sdrMaxLuminance: 250, sdrBrightness: 1.1, sdrSaturation: 1.2 }
    );
});

test("colorFromIpc 空 ipc 回落默认", () => {
    assert.deepEqual(M.colorFromIpc({}), { cm: "srgb", sdrMaxLuminance: 80, sdrBrightness: 1, sdrSaturation: 1 });
});

test("layoutFromIpc 与 colorFromIpc 结果一致（同源，不能各写一份）", () => {
    const ipc = {
        name: "DP-3", width: 3840, height: 2160, refreshRate: 60, x: 0, y: 0, scale: 1.5,
        colorManagementPreset: "hdr", sdrMaxLuminance: 600, sdrBrightness: 1.1, sdrSaturation: 1.5
    };
    assert.deepEqual(M.layoutFromIpc(ipc).color, M.colorFromIpc(ipc));
});

test("layoutFromIpc 读回 sdrMaxLuminance", () => {
    const l = M.layoutFromIpc({
        name: "DP-3", width: 3840, height: 2160, refreshRate: 60, x: 0, y: 0, scale: 1.5,
        colorManagementPreset: "hdr", sdrMaxLuminance: 250, sdrBrightness: 1, sdrSaturation: 1
    });
    assert.equal(l.color.sdrMaxLuminance, 250);
});

test("isHdr 判定", () => {
    assert.equal(M.isHdr({ color: { cm: "hdr" } }), true);
    assert.equal(M.isHdr({ color: { cm: "srgb" } }), false);
    assert.equal(M.isHdr({}), false);
});

test("monitorLuaLine 默认 srgb 不输出 cm（保持行干净）", () => {
    assert.equal(
        M.monitorLuaLine(BASE),
        'hl.monitor({ output = "DP-3", mode = "3840x2160@60", position = "0x0", scale = 1 })'
    );
});

test("monitorLuaLine 开 HDR 输出 cm + bitdepth", () => {
    // HDR 必须显式带 bitdepth=10：8bit 下 PQ 曲线会有明显 banding
    assert.equal(
        M.monitorLuaLine({ ...BASE, color: { cm: "hdr" } }),
        'hl.monitor({ output = "DP-3", mode = "3840x2160@60", position = "0x0", scale = 1, bitdepth = 10, cm = "hdr" })'
    );
});

test("monitorLuaLine 输出 sdr 补偿（lua 字段名全小写）", () => {
    assert.equal(
        M.monitorLuaLine({ ...BASE, color: { cm: "hdr", sdrBrightness: 1.2, sdrSaturation: 1.05 } }),
        'hl.monitor({ output = "DP-3", mode = "3840x2160@60", position = "0x0", scale = 1, bitdepth = 10, cm = "hdr", sdrbrightness = 1.2, sdrsaturation = 1.05 })'
    );
});

test("monitorLuaLine srgb 下不输出 sdr 补偿（着色器只在 SDR↔HDR 转换时用它）", () => {
    assert.equal(
        M.monitorLuaLine({ ...BASE, color: { cm: "srgb", sdrBrightness: 1.2, sdrSaturation: 1.05 } }),
        'hl.monitor({ output = "DP-3", mode = "3840x2160@60", position = "0x0", scale = 1 })'
    );
});

test("serializeMonitorString 纳入 cm，否则 _differsFromCurrent 漏判 HDR 变化", () => {
    assert.equal(M.serializeMonitorString(BASE), "DP-3,3840x2160@60,0x0,1");
    assert.equal(
        M.serializeMonitorString({ ...BASE, color: { cm: "hdr", sdrMaxLuminance: 250, sdrBrightness: 1.2, sdrSaturation: 1 } }),
        "DP-3,3840x2160@60,0x0,1,cm,hdr,sdrmax,250,sdrbrightness,1.2,sdrsaturation,1"
    );
});

test("serializeMonitorString 只改白点也要判出差异", () => {
    const a = M.serializeMonitorString({ ...BASE, color: { cm: "hdr", sdrMaxLuminance: 80 } });
    const b = M.serializeMonitorString({ ...BASE, color: { cm: "hdr", sdrMaxLuminance: 250 } });
    assert.notEqual(a, b);
});

test("layoutFromIpc 从 colorManagementPreset 读回色彩状态", () => {
    const l = M.layoutFromIpc({
        name: "DP-3", width: 3840, height: 2160, refreshRate: 60, x: 0, y: 0, scale: 1.5,
        colorManagementPreset: "hdr", sdrMaxLuminance: 250, sdrBrightness: 1.2, sdrSaturation: 1.05
    });
    assert.deepEqual(l.color, { cm: "hdr", sdrMaxLuminance: 250, sdrBrightness: 1.2, sdrSaturation: 1.05 });
});

test("layoutFromIpc 缺色彩字段时回落 srgb（旧 Hyprland / 禁用 cm）", () => {
    const l = M.layoutFromIpc({ name: "eDP-1", width: 2560, height: 1600, refreshRate: 60, x: 0, y: 0, scale: 1 });
    assert.deepEqual(l.color, { cm: "srgb", sdrMaxLuminance: 80, sdrBrightness: 1, sdrSaturation: 1 });
});

test("hdrRejected：请求 hdr 但回读仍是 srgb → 判定该屏不支持", () => {
    assert.equal(M.hdrRejected({ color: { cm: "hdr" } }, { colorManagementPreset: "srgb" }), true);
    assert.equal(M.hdrRejected({ color: { cm: "hdr" } }, { colorManagementPreset: "hdr" }), false);
    assert.equal(M.hdrRejected({ color: { cm: "srgb" } }, { colorManagementPreset: "srgb" }), false);
});
