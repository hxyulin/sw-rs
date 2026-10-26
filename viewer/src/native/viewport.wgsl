// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
struct Camera {
    center: vec4f,
    right: vec4f,
    up: vec4f,
    back: vec4f,
    options: vec4f,
}
@group(0) @binding(0) var<uniform> camera: Camera;
struct Vertex {
    @builtin(position) position: vec4f,
    @location(0) normal: vec3f,
    @location(1) color: vec3f,
}
@vertex fn vertex(@location(0) position: vec3f, @location(1) normal: vec3f,
    @location(2) palette_color: vec3f, @location(3) material: vec3f) -> Vertex {
    let p = position - camera.center.xyz;
    var out: Vertex;
    out.position = vec4f(dot(p, camera.right.xyz) + camera.right.w,
        dot(p, camera.up.xyz) + camera.up.w,
        0.5 - dot(p, camera.back.xyz) / (camera.center.w * 4.), 1.);
    out.normal = normal;
    out.color = mix(palette_color, material, camera.options.x);
    return out;
}
@fragment fn fragment(in: Vertex, @builtin(front_facing) front: bool) -> @location(0) vec4f {
    let normal = normalize(in.normal) * select(-1., 1., front);
    let light = 0.32 + 0.55 * max(dot(normal, normalize(vec3f(0.4, 0.8, 0.6))), 0.)
        + 0.13 * max(dot(normal, normalize(vec3f(-0.6, 0.2, -0.3))), 0.);
    return vec4f(pow(clamp(in.color, vec3f(0.), vec3f(1.)), vec3f(2.2)) * light, 1.);
}
