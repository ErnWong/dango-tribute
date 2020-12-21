#version 450

void main() {
    const vec2 quad_vertices[6] = vec2[6](
        vec2(-1.0, 1.0),
        vec2(-1.0, -1.0),
        vec2(1.0, -1.0),
        vec2(1.0, 1.0),
        vec2(-1.0, 1.0),
        vec2(1.0, -1.0)
    );

    vec2 current_vertex = quad_vertices[gl_VertexIndex];
    gl_Position = vec4(current_vertex, 0.0, 1.0);
}
