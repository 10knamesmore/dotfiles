// shaderGen 纯逻辑单测。纯 JS、无 QML 依赖，用 node 直接跑：
//   node --test shaderGen.test.js
const test = require("node:test");
const assert = require("node:assert/strict");
const G = require("./shaderGen.js");

const near = (a, b, eps = 1e-3) => assert.ok(Math.abs(a - b) < eps, `${a} !~= ${b}`);

// ── 色温：关键不变量 ──────────────────────────────────────
test("R 分量恒为 1.0 —— 色温只衰减不增益，>1 会撞 clamp 丢亮部细节", () => {
    for (let w = 0; w <= 100; w += 5)
        assert.equal(G.kelvinScale(w)[0], 1.0);
});

test("所有分量恒 <= 1.0（白点缩放的对角元约束）", () => {
    for (let w = 0; w <= 100; w += 1)
        G.kelvinScale(w).forEach(c => assert.ok(c <= 1.0, `warmth=${w} 出现 ${c} > 1`));
});

test("warmth 越大 G/B 越低，且 B 衰减快于 G（黑体曲线形状）", () => {
    let prev = G.kelvinScale(0);
    for (let w = 5; w <= 100; w += 5) {
        const cur = G.kelvinScale(w);
        assert.ok(cur[1] < prev[1], `warmth=${w} 的 G 没有下降`);
        assert.ok(cur[2] < prev[2], `warmth=${w} 的 B 没有下降`);
        assert.ok(1 - cur[2] > 1 - cur[1], `warmth=${w} 的 B 衰减未快于 G`);
        prev = cur;
    }
});

test("warmth=0 落在 6500K（D65 中性），接近但不等于纯白", () => {
    const [r, g, b] = G.kelvinScale(0);
    assert.equal(r, 1.0);
    near(g, 0.997);
    near(b, 0.981);
});

test("warmth=100 落在 2500K，蓝色大幅衰减", () => {
    const [r, g, b] = G.kelvinScale(100);
    assert.equal(r, 1.0);
    near(g, 0.624);
    near(b, 0.275);
});

test("warmth=60（阅读预设）落在 4100K", () => {
    const [, g, b] = G.kelvinScale(60);
    near(g, 0.817);
    near(b, 0.669);
});

test("warmth 超出 0-100 被夹住，不外推出非法系数", () => {
    assert.deepEqual(G.kelvinScale(-50), G.kelvinScale(0));
    assert.deepEqual(G.kelvinScale(999), G.kelvinScale(100));
});

// ── 颗粒 ──────────────────────────────────────────────────
test("grainFreq 端点：0=极细 6000x，100=粗颗粒 800x", () => {
    near(G.grainFreq(0), 6000);
    near(G.grainFreq(100), 800);
    near(G.grainFreq(50), 3400);
});

// ── 源码拼装 ──────────────────────────────────────────────
const BODY = "precision highp float;\nin vec2 v_texcoord;\nvoid main() { fragColor = vec4(TEMP_SCALE, 1.0); }\n";
const PARAMS = { warmth: 60, grain: 85, grainSize: 10, shadowBoost: 40 };

test("#version 300 es 必须是首行（GLSL 要求先于任何非注释代码）", () => {
    assert.equal(G.buildShader(PARAMS, BODY).split("\n")[0], "#version 300 es");
});

test("precision 声明必须先于 const 块（GLSL ES 的默认精度限定符顺序要求）", () => {
    const src = G.buildShader(PARAMS, BODY);
    assert.ok(src.indexOf("precision highp float;") < src.indexOf("const vec3"),
        "precision 落在 const 之后会报 'type requires declaration of default precision qualifier'");
});

test("产物不含任何 __PLACEHOLDER__ 残留", () => {
    assert.ok(!/__[A-Z_]+__/.test(G.buildShader(PARAMS, BODY)));
});

test("四个常量齐备，且 body 原文被完整带上", () => {
    const src = G.buildShader(PARAMS, BODY);
    ["TEMP_SCALE", "GRAIN", "GRAIN_FREQ", "SHADOW_BOOST"].forEach(name =>
        assert.ok(src.includes("const") && new RegExp(`\\b${name}\\s*=`).test(src), `缺 ${name}`));
    assert.ok(src.includes(BODY));
});

test("数字用定点格式，不会出现科学计数法（GLSL 不认 1e-4）", () => {
    const src = G.buildShader({ warmth: 100, grain: 0, grainSize: 0, shadowBoost: 0 }, BODY);
    assert.ok(!/[eE][-+]\d/.test(src));
});

test("needsShader：色温和颗粒都为 0 时不该加载 shader", () => {
    assert.equal(G.needsShader({ warmth: 0, grain: 0 }), false);
    assert.equal(G.needsShader({ warmth: 60, grain: 0 }), true);
    assert.equal(G.needsShader({ warmth: 0, grain: 20 }), true);
    assert.equal(G.needsShader({}), false);
});
