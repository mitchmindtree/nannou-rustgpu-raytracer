#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]

use shared::{color, Camera, Dielectric, Lambertian, MaterialInfo, MaterialKind, Materials, Metal, Rng, ShaderConstants, Sphere};
use spirv_std::glam::{vec2, vec3, vec4, Vec2, Vec4};

// Note: This cfg is incorrect on its surface, it really should be "are we compiling with std", but
// we tie #[no_std] above to the same condition, so it's fine.
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;
#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(frag_coord)]
    in_frag_coord: Vec4,
    #[spirv(push_constant)]
    constants: &ShaderConstants,
    output: &mut Vec4,
) {
    // Calc uv coords (i.e. left 0.0, right 1.0, bottom 0.0, top 1.0);
    let frag_coord = vec2(in_frag_coord.x, in_frag_coord.y);
    let [w_px, h_px] = constants.view_size_pixels;

    let time = constants.time;
    let vfov = constants.vfov;
    let aperture = constants.aperture;
    let aspect = w_px as f32 / h_px as f32;
    let from = vec3((time * 0.77).cos() * 0.125 + 0.125, 1.0 + time.sin() * 0.125 + 0.125, 0.0);
    let to = vec3(0.0, 1.0, -3.0);
    let vup = vec3(0.0, 1.0, 0.0);
    let focus_dist = (from - to).length() - 0.25; // subtract a little to get sphere surface.
    let cam = Camera::new(from, to, vup, vfov, aspect, aperture, focus_dist);

    let seed = frag_coord + Vec2::splat(constants.rng_seed_offset);
    let mut rng = Rng { seed };

    let materials = Materials {
        lambertian: [
            Lambertian::new(vec3(0.8 + (constants.time * 1.7).sin() + 0.5, 0.3, 0.3)),
            Lambertian::new(vec3(1.0, 0.1, 0.1)),
            Lambertian::new(vec3(0.1, 1.0, 0.1)),
            Lambertian::new(vec3(0.9, 0.9, 0.9)),
            Lambertian::new(vec3(5.0, 5.0, 5.0)),
        ],
        metal: [
            Metal::new(vec3(0.8, 0.6, 0.2), 0.0),
            Metal::new(vec3(0.8, 0.6, 0.2), 0.05),
            Metal::new(vec3(0.2, 0.6, 0.8), 0.05),
            Metal::new(vec3(0.9, 0.9, 0.9), 0.5),
        ],
        dielectric: [
            Dielectric::new(1.5)
        ],
    };

    let world = [
        Sphere {
            center: to,
            radius: 0.5,
            material: MaterialInfo {
                kind: MaterialKind::Metal,
                index: 0,
            },
        },
        Sphere {
            center: vec3(to.x - 1.0, 0.0, to.z + 1.0),
            radius: 0.5,
            material: MaterialInfo {
                kind: MaterialKind::Dielectric,
                index: 0,
            }
        },
        Sphere {
            center: vec3(1.0, 0.0, -1.0 + (constants.time * 1.32).cos()),
            radius: 0.5,
            material: MaterialInfo {
                kind: MaterialKind::Metal,
                index: 2,
            },
        },

        // Light
        Sphere {
            center: to + vec3(
                        (constants.time * 0.67).sin(),
                        (constants.time * 0.33).cos(),
                        (constants.time * 0.57).cos(),
                    ),
            radius: 0.1,
            material: MaterialInfo {
                kind: MaterialKind::Lambertian,
                index: 3,
            }
        },

        // Floor
        Sphere {
            center: vec3(0.0, -1000.5, -1.0),
            radius: 1000.0,
            material: MaterialInfo {
                kind: MaterialKind::Lambertian,
                index: 3,
            },
        },

        // Left wall.
        Sphere {
            center: vec3(-22.0, 0.0, -1.0),
            radius: 20.0,
            material: MaterialInfo {
                kind: MaterialKind::Lambertian,
                index: 1,
            },
        },

        // Right wall.
        Sphere {
            center: vec3(22.0, 0.0, -1.0),
            radius: 20.0,
            material: MaterialInfo {
                kind: MaterialKind::Lambertian,
                index: 2,
            },
        },

        // Back wall.
        Sphere {
            center: vec3(0.0, 0.0, -24.0),
            radius: 20.0,
            material: MaterialInfo {
                kind: MaterialKind::Lambertian,
                index: 3,
            },
        },
    ];

    // Cast some rays and average their result.
    let mut col = vec3(0.0, 0.0, 0.0);
    for _ in 0..constants.rays_per_pixel {
        let uv = vec2(
            (frag_coord.x + rng.gen()) / w_px as f32,
            ((h_px as f32 - frag_coord.y) + rng.gen()) / h_px as f32,
        );
        let ray = cam.ray(&mut rng, uv);
        col += color(constants.ray_bounce_limit, &mut rng, ray, world, &materials);
    }
    col /= constants.rays_per_pixel as f32;

    // Write the result.
    *output = vec4(col.x, col.y, col.z, 1.0);
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)]
    vert_idx: i32,
    #[spirv(position)]
    builtin_pos: &mut Vec4,
) {
    // Create a "full screen triangle" by mapping the vertex index.
    // ported from https://www.saschawillems.de/blog/2016/08/13/vulkan-tutorial-on-rendering-a-fullscreen-quad-without-buffers/
    let uv = vec2(((vert_idx << 1) & 2) as f32, (vert_idx & 2) as f32);
    let pos = 2.0 * uv - Vec2::ONE;
    *builtin_pos = pos.extend(0.0).extend(1.0);
}
