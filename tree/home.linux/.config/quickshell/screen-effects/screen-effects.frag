// Hyprland screen shader 主体 —— 护眼色温 + 胶片颗粒。
//
// 本文件不含占位符，是一段合法 GLSL。参数（TEMP_SCALE / GRAIN / GRAIN_FREQ /
// SHADOW_BOOST）由 lib/shaderGen.js 生成的 const 块提供，拼在本文件之前，
// 产物落 ~/.cache/hypr/screen-effects.glsl。所以单独把本文件送 glslangValidator
// 会报 undeclared identifier —— 那是语义错误，语法/结构检查与高亮仍然可用。
// 注意本文件（含注释）不要出现连续两个下划线 —— 校验产物有无占位符残留时会 grep
// 这个模式，注释里出现就会自己命中自己，让那条检查失去意义。
//
// 参数为什么不用 uniform：Hyprland 的 shader uniform 是写死的枚举
// （Hyprland/src/render/Shader.hpp 的 eShaderUniform），没有自定义 uniform 通道，
// hyprctl 也没有设 uniform 的接口。自定义 screen shader 的参数只能编译进源码。
//
// 自定义 screen shader 默认配 tex300.vert，必须使用 GLSL ES 3.00 的 in/texture/fragColor 语法。
// 只有以 `#version 320 es` 开头才会配 tex320.vert。版本不一致时 glLinkProgram 会失败，而且
// hyprctl 那侧照样返回 ok，错误只落在 Hyprland 日志和屏幕错误浮层里。

// `precision highp float;` 不在这里 —— 它必须先于生成的 const 块（GLSL ES 要求
// 默认精度限定符先于任何用到该类型的声明），所以住在 shaderGen.js 的头里。
in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;
uniform sampler2D tex;

// ── 胶片颗粒噪声 ──
float hash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float filmGrain(vec2 uv, float intensity, float freq) {
    // 细颗粒：高频像素级噪声（银盐基底）
    float fine = hash(uv * freq) * 2.0 - 1.0;
    // 中颗粒：团簇感（银盐聚集），频率为细颗粒的 ~0.4x
    float cluster = hash(floor(uv * freq * 0.38) + fract(uv * freq) * 0.3) * 2.0 - 1.0;
    // 低频明暗起伏（胶片乳剂层厚度不均匀），频率为细颗粒的 ~0.1x
    float undulation = hash(floor(uv * freq * 0.1)) * 2.0 - 1.0;

    float grain = fine * 0.55 + cluster * 0.35 + undulation * 0.10;
    return grain * intensity;
}

void main() {
    vec4 color = texture(tex, v_texcoord);

    // ── 护眼色温：白点缩放 ──
    // TEMP_SCALE 由黑体辐射拟合算出（同 hyprsunset 的 CTM 对角矩阵，数学等价），
    // 三个分量恒 <= 1.0。别在这里补什么「亮度补偿」把分量推过 1 —— 亮部会撞上
    // 函数末尾的 clamp 被截平，且单通道被截会连带色相偏移。
    color.rgb *= TEMP_SCALE;

    // ── 胶片颗粒 ──
    // GRAIN 是编译期常量，颗粒关闭时整个分支会被编译器消除。
    if (GRAIN > 0.0) {
        float luma = dot(color.rgb, vec3(0.299, 0.587, 0.114));
        // 暗部增强：亮部压颗粒、暗部加颗粒，模拟银盐在低曝光区更显眼
        float shadow = 1.0 - smoothstep(0.0, 0.5, luma);
        float strength = GRAIN * 0.12 * (1.0 - SHADOW_BOOST * 0.4 + shadow * SHADOW_BOOST * 0.8);

        color.rgb += filmGrain(v_texcoord, strength, GRAIN_FREQ);
    }

    color.rgb = clamp(color.rgb, 0.0, 1.0);
    fragColor = color;
}
