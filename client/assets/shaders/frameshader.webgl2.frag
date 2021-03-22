#version 300 es
precision highp float;

out vec4 o_color;
uniform sampler2D i_source; // set = 0, binding = 0
uniform sampler2D i_random; // set = 0, binding = 1
layout(std140) uniform Resolution { // set = 1, binding = 0
    vec2 i_resolution;
};
layout(std140) uniform Time { // set = 1, binding = 1
    float i_time_sec;
};
layout(std140) uniform TimeDelta { // set = 1, binding = 2
    float i_time_delta_sec;
};
layout(std140) uniform Mouse { // set = 1, binding = 3
    vec4 i_mouse;
};

vec4 read_texture(sampler2D the_texture, vec2 position) {
    return texture(the_texture, position);
}

// The rest can be copied to the wgpu version.
// The webgl2 version shall be the source of truth.

vec2 pos2uv(vec2 pos) {
    return pos / i_resolution;
}

vec4 rand(vec2 pos, float page) {
    const vec2 TIME_OFFSET_DIRECTION = vec2(1234.567, 123.4567);
    vec2 time_based_offset = i_time_sec * TIME_OFFSET_DIRECTION;

    const vec2 PAGE_OFFSET_DIRECTION = vec2(1234.567, 123.4567);
    vec2 page_based_offset = page * PAGE_OFFSET_DIRECTION;

    vec2 uv = pos2uv(pos + time_based_offset + page_based_offset);
    return read_texture(i_random, uv);
}

vec4 smoothed_rand(vec2 pos, float smoothing, float page) {
    vec2 subpixel_pos = pos / smoothing;

    return rand(subpixel_pos, page);
}

vec3 background(vec2 pos) {
    if (pos.y < i_resolution.y * 0.5) {
        return vec3(0.3, 0.3, 0.3);
    } else {
        return mix(vec3(0.3, 0.7, 1.0), vec3(0.15, 0.35, 0.5), pos.y / i_resolution.y);
    }
}

vec3 source(vec2 pos) {
    vec2 uv = pos2uv(pos);
    vec4 texture_colour = read_texture(i_source, uv).rgba;
    return mix(background(pos), texture_colour.rgb, texture_colour.a);
}

float source_brightness(vec2 pos) {
    return length(source(pos)) / sqrt(3.0);
}

float source_darkness(vec2 pos) {
    return 1.0 - source_brightness(pos);
}

vec2 wobbly_pos(float page, float amplitude) {
    return gl_FragCoord.xy + smoothed_rand(gl_FragCoord.xy, 10.0, page).xy * amplitude;
}

vec2 scribbly_pos(float page, float amplitude) {
    return gl_FragCoord.xy + smoothed_rand(gl_FragCoord.xy, 3.0, page).xy * amplitude;
}

vec2 gradient(vec2 pos) {
    const float delta = 0.3;
    return vec2(
        source_brightness(pos) - source_brightness(pos - vec2(delta, 0.0)),
        source_brightness(pos) - source_brightness(pos - vec2(0.0, delta))
    );
}

float vignette_mask() {
    const float SQUARENESS = 2.7;
    vec2 r = gl_FragCoord.xy / i_resolution * 2.0 - 1.0;
    float t = pow(pow(abs(r.x), SQUARENESS) + pow(abs(r.y), SQUARENESS), 1.0 / SQUARENESS);
    return 1.0 - smoothstep(0.6, 1.2, t);
}

vec4 sketch_outline(float page, float amplitude) {
    if (source_darkness(wobbly_pos(page, amplitude)) < 0.8) {
        return vec4(0.0);
    }
    float strength = mix(vignette_mask(), 1.0, 0.3);
    return vec4(0.0, 0.0, 0.0, 1.0);
}

vec4 sketch_hatch(float page, vec2 hatch_direction, float contrast, float thickness, float variance) {
    // Stretch random texture along the given direction.
    vec2 projected_pos = hatch_direction
        * dot(gl_FragCoord.xy, hatch_direction)
        / dot(hatch_direction, hatch_direction);
    vec2 stretched_pos = gl_FragCoord.xy - projected_pos * (1.0 - 1.0 / length(hatch_direction));
    float stretched_rand = rand(stretched_pos * 2.0, page).a;

    // Weight the random texture with the source's darker regions.
    float weighted_rand = stretched_rand + contrast * source_darkness(wobbly_pos(2.0, 3.0));

    // Extract the hatch using smoothstep.
    float hatch = 1.0 - smoothstep(thickness, thickness + variance, weighted_rand);

    // Mask out the white paper.
    float mask = 0.0;
    const int BLUR_DISTANCE = 10;
    vec2 blur_direction = hatch_direction / length(hatch_direction);
    for (int i = 0; i < BLUR_DISTANCE; i++) {
        vec2 sample_offset = blur_direction * float(i - BLUR_DISTANCE / 2);
        vec2 sample_position = scribbly_pos(page, 6.0) + sample_offset;
        mask += step(0.001, source_darkness(sample_position)) / float(BLUR_DISTANCE);
    }
    hatch *= mask;

    // Hatch less near the screen edge.
    hatch *= mix(vignette_mask(), 1.0, 0.4);

    // Colorize.
    return vec4(source(wobbly_pos(page, 3.0)), hatch);
}

vec4 smudge_fills(float page) {
    vec3 color = source(wobbly_pos(1.0, 3.0));
    float strength = 0.2 * smoothstep(0.0, 0.5, smoothed_rand(gl_FragCoord.xy, 2.0, page).r);
    strength *= vignette_mask();
    return vec4(color, strength);
}

vec4 paper_grain() {
    return vec4(0.9, 0.9, 0.9, 1.0)
        - 0.03 * smoothed_rand(gl_FragCoord.xy, 5.0, 0.0).x
        - 0.10 * smoothed_rand(gl_FragCoord.xy, 2.0, 0.0).x
        + 0.06 * smoothed_rand(gl_FragCoord.xy + vec2(-1.0), 2.0, 0.0).x;
}

vec4 paper_dents() {
    float dent_alpha = smoothstep(0.8, 1.0, rand(gl_FragCoord.xy, 1.0).x);
    return vec4(paper_grain().rgb, dent_alpha);
}

void draw(vec4 colour) {
    o_color.rgb = colour.rgb * colour.a + o_color.rgb * (1.0 - colour.a);
}

vec2 polar(float angle_degrees, float magnitude) {
    float angle_radians = radians(-angle_degrees);
    return magnitude * vec2(cos(angle_radians), sin(angle_radians));
}

vec4 alpha(float value) {
    return vec4(1.0, 1.0, 1.0, value);
}

void main() {
    // Start with white, then mix as we go.
    o_color = vec4(1.0);
    draw(paper_grain());
    draw(smudge_fills(0.0));
    draw(smudge_fills(1.0));
    draw(smudge_fills(2.0));
    draw(smudge_fills(3.0));
    draw(alpha(0.5) * sketch_hatch(0.0, polar(-30.0, 10.0 * 10.0), 0.0, 0.3, 0.5));
    draw(alpha(0.3) * sketch_hatch(1.0, polar(-32.0, 10.0 * 10.0), 0.0, 0.3, 0.5));
    draw(alpha(0.2) * sketch_hatch(2.0, polar(-40.0, 10.0 * 10.0), 0.0, 0.3, 0.5));
    draw(alpha(0.3) * sketch_hatch(3.0, polar(-50.0, 10.0 * 10.0), 0.0, 0.3, 0.5));
    draw(alpha(0.5) * paper_dents());
    o_color.rgb *= mix(vignette_mask(), 1.0, 0.5);
    o_color.rgb = sqrt(o_color.rgb);

    if (false && gl_FragCoord.x < i_mouse.x) {
        o_color.rgb = source(gl_FragCoord.xy);
    }
    if (false && gl_FragCoord.y < i_mouse.y) {
        o_color.rgb = rand(gl_FragCoord.xy, 0.0).rrr;
    }
}
