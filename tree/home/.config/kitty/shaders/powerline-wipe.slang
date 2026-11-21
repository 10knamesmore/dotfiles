#language slang 2026
import kitty_custom_shader_types;

static const int PASS = 0;

// Four off-screen texels store the previous and current tab's pixel bounds.
// Use two bytes per coordinate even on GPUs with 8-bit render targets.
float4 pack_edge(float x) {
    return float4(floor(x / 256.f), fmod(x, 256.f), 0.f, 255.f) / 255.f;
}

float2 read_span(KittyTexture texture, float2 size, int slot) {
    float2 uv = float2(slot + 0.5f, floor(size.y / 2.f) + 0.5f) / size;
    float2 lo = round(texture.Sample(uv).rg * 255.f);
    float2 hi = round(texture.Sample(uv + float2(1.f / size.x, 0.f)).rg * 255.f);
    return float2(dot(lo, float2(256.f, 1.f)), dot(hi, float2(256.f, 1.f)));
}

// Both configured themes use a bright active tab on a dark strip. At the
// outside edge of the Powerline caps the title doesn't interrupt its fill.
float2 active_span(KittyTexture frame, float2 size, float y) {
    float2 best = 0.f;
    float best_luma = 0.f;
    float3 run_color = -1.f;
    int start = 0;
    for (int x = 0; x <= int(size.x); ++x) {
        float3 pixel = x == int(size.x) ? float3(-1.f) :
            frame.Sample(float2((x + 0.5f) / size.x, y)).rgb;
        if (any(pixel != run_color)) {
            float luma = dot(run_color, float3(0.2126f, 0.7152f, 0.0722f));
            if (x - start >= 4 && luma > best_luma) {
                best = float2(start, x);
                best_luma = luma;
            }
            start = x;
            run_color = pixel;
        }
    }
    return best;
}

float reveal(float x, float2 span, float cap, bool rightward, float progress) {
    span += float2(-cap, cap);
    float edge = lerp(rightward ? span.x : span.y, rightward ? span.y : span.x, progress);
    return saturate(0.5f + (rightward ? edge - x : x - edge));
}

public float4 fragment_main(float4 color, KittyTextures t, KittyCustomShaderData d) {
    if (PASS == 3) return color;
    float2 size = float2(d.viewport_size_pixels);
    int2 pixel = int2(t.pos * size);
    bool header = pixel.y == int(size.y) / 2 && pixel.x < 4;
    float bottom = d.central_area.y, top = d.central_area.y + d.central_area.w;
    bool below = bottom >= 1.f - top;
    float height = round((below ? bottom : 1.f - top) * size.y);
    bool in_bar = height > 0.f && (below ? t.pos.y < bottom : t.pos.y >= top);

    if (PASS == 0) {
        if (!header || height <= 0.f) return float4(0.f, 0.f, 0.f, 1.f);
        float y = below ? 0.5f / size.y : 1.f - 0.5f / size.y;
        float2 target = active_span(t.backbuffer, size, y);
        float2 last = read_span(t.persist, size, 2);
        float2 origin = all(last == target) ? read_span(t.persist, size, 0) : last;
        float2 span = pixel.x < 2 ? origin : target;
        return pack_edge((pixel.x & 1) == 0 ? span.x : span.y);
    }
    if (PASS == 2) {
        if (header) return t.a.Sample(t.pos);
        return in_bar ? color : float4(0.f, 0.f, 0.f, 1.f);
    }

    if (!in_bar) return color;
    float2 origin = read_span(t.a, size, 0), target = read_span(t.a, size, 2);
    if (origin.y <= origin.x || target.y <= target.x || all(origin == target)) return color;
    float x = t.pos.x * size.x;
    bool rightward = target.x > origin.x;
    float2 left = rightward ? origin : target, right = rightward ? target : origin;
    float cap = height * 0.5f;
    float gap = right.x - left.y;
    float progress = t.animation_progress;
    float4 previous = t.persist.Sample(t.pos);
    if (gap >= 0.f && gap < cap && x >= left.y && x <= left.y + cap) {
        // The cap and its background both carry the active fill between the
        // arrival of the incoming edge and the departure of the outgoing one.
        // The two native frames supply the anti-aliased curve at either end.
        float y = below ? 0.5f / size.y : 1.f - 0.5f / size.y;
        float4 active = t.backbuffer.Sample(float2((target.x + target.y) * 0.5f / size.x, y));
        float incoming = reveal(x, target, cap, rightward, progress);
        float outgoing = reveal(x, origin, cap, rightward, progress);
        return lerp(lerp(previous, active, incoming), color, outgoing);
    }
    // Split at the facing edges. The midpoint between tab centers can fall
    // inside the longer title and expose part of it before its wipe arrives.
    float divide = gap < cap ? left.y : (left.y + right.x) * 0.5f;
    float2 span = x < divide ? left : right;
    if (x < span.x - cap || x > span.y + cap) return color;
    float revealed = reveal(x, span, cap, rightward, progress);
    return lerp(previous, color, revealed);
}
