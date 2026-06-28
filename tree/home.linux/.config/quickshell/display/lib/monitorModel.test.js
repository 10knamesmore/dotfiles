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
