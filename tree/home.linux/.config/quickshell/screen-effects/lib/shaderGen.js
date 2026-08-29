// shaderGen — 屏幕效果 shader 的参数计算与源码拼装（纯逻辑核心）。
//
// 设计要点：本文件不依赖 QML/Quickshell，全是纯函数，可被 node 直接跑单测
// （见 shaderGen.test.js），也能被 QML 以 `.import` 方式引用。文件末尾的
// module.exports 守卫只在 node 下生效，QML 引擎里 typeof module === "undefined"。
//
// 不加 `.pragma library` —— 那会让 node 的 require 解析失败（同 display/lib/monitorModel.js）。
//
// 产物形状：`#version` + const 声明块 + screen-effects.frag 原文。参数以编译期
// 常量注入，而不是 uniform —— Hyprland 的 shader uniform 是写死的枚举
// （Hyprland/src/render/Shader.hpp 的 eShaderUniform），既无自定义 uniform 通道、
// hyprctl 也没有设 uniform 的接口，所以自定义 screen shader 的参数只能编译进源码。

// ── 色温 ──────────────────────────────────────────────────
// warmth 0-100 映射到的色温区间。6500K = D65 中性白，2500K 是常见护眼下限。
var KELVIN_NEUTRAL = 6500;
var KELVIN_WARMEST = 2500;

function clamp01(x) {
    return x < 0 ? 0 : (x > 1 ? 1 : x);
}

// warmth(0-100) → 黑体辐射白点的 RGB 缩放系数 [r, g, b]。
//
// 算法同 hyprsunset（Tanner Helland 拟合，见其 src/Hyprsunset.cpp 的 matrixForKelvin），
// 两者数学等价：hyprsunset 把系数做成 CTM 对角矩阵下发，这里编译进 shader 做逐通道乘。
//
// 关键不变量：返回值三个分量恒 <= 1.0。降色温只能衰减短波通道；任何通道增益超过 1.0
// 都会在 shader 末尾 clamp 高光，使白色发脏并产生色相偏移。
//
// 因映射上限 6500K < 6600K，恒落在 Tanner Helland 的 temp<=66 分支（该分支 R 恒为满值），
// 故这里不实现高色温分支。若将来把 KELVIN_NEUTRAL 提到 6600 以上，必须补上。
function kelvinScale(warmth) {
    var w = Math.max(0, Math.min(100, warmth));
    var kelvin = KELVIN_NEUTRAL - (KELVIN_NEUTRAL - KELVIN_WARMEST) * (w / 100);
    var t = kelvin / 100;

    var g = 99.4708025861 * Math.log(t) - 161.1195681661;
    var b = t <= 19 ? 0 : Math.log(t - 10) * 138.5177312231 - 305.0447927307;

    return [1.0, clamp01(g / 255), clamp01(b / 255)];
}

// ── 颗粒 ──────────────────────────────────────────────────
// grainSize(0-100) → 噪声采样频率。0 = 极细银盐(6000x)，100 = 粗颗粒(800x)。
function grainFreq(grainSize) {
    var gs = Math.max(0, Math.min(100, grainSize)) / 100;
    return 6000 - (6000 - 800) * gs;
}

// ── 源码拼装 ──────────────────────────────────────────────
// params: { warmth, grain, grainSize, shadowBoost }，四项均为 0-100 整数。
// body: screen-effects.frag 的原文。
//
// `#version` 必须先于任何非注释代码，所以它由本函数提供、不能留在 body 里 ——
// 这也顺带保证了 body 是一个不含占位符的合法 GLSL 片段。
//
// `precision highp float;` 同样住在头里而不是 body 里：GLSL ES 要求默认精度限定符
// 先于任何用到该类型的声明，而下面的 const 块就用了 float/vec3。放 body 里会报
// "type requires declaration of default precision qualifier"。
function buildShader(params, body) {
    var temp = kelvinScale(params.warmth || 0);
    var f = function (x) {
        return x.toFixed(4);
    };

    return "#version 300 es\n"
        + "// 由 QuickShell ScreenEffectsService 生成，改这里没用 —— 主体见\n"
        + "// ~/.config/quickshell/screen-effects/screen-effects.frag，参数见面板。\n"
        + "precision highp float;\n"
        + "\n"
        + "const vec3  TEMP_SCALE   = vec3(" + f(temp[0]) + ", " + f(temp[1]) + ", " + f(temp[2]) + ");\n"
        + "const float GRAIN        = " + f((params.grain || 0) / 100) + ";\n"
        + "const float GRAIN_FREQ   = " + f(grainFreq(params.grainSize || 0)) + ";\n"
        + "const float SHADOW_BOOST = " + f((params.shadowBoost || 0) / 100) + ";\n"
        + "\n"
        + body;
}

// 效果是否需要 shader。两项都为 0 时应当直接清空 decoration:screen_shader，
// 而不是加载一个恒等变换的 shader —— 后者仍会走完整的 final shader 渲染路径。
function needsShader(params) {
    return (params.warmth || 0) > 0 || (params.grain || 0) > 0;
}

if (typeof module !== "undefined" && module.exports) {
    module.exports = {
        KELVIN_NEUTRAL: KELVIN_NEUTRAL,
        KELVIN_WARMEST: KELVIN_WARMEST,
        kelvinScale: kelvinScale,
        grainFreq: grainFreq,
        buildShader: buildShader,
        needsShader: needsShader
    };
}
