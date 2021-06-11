//! Code shared between shader and app.

#![no_std]

use spirv_std::{
    glam::{vec2, vec3, Vec2, Vec3},
    num_traits::Float,
};

/// Types that may be hit by a ray.
pub trait Hit {
    /// Whether or not the Ray hits the object along with the associated hit data.
    fn hit(self, r: &Ray, t_min: f32, t_max: f32, data: &mut HitData) -> bool;
}

/// Used to describe the surface of different materials.
pub trait Material {
    /// Produce a scattered ray (or say it absorved the incident ray).
    ///
    /// If scattered, describes how much the ray should be attenuated.
    fn scatter(
        self,
        r_in: &Ray,
        hit: &HitData,
        rng: &mut Rng,
        attenuation: &mut Vec3,
        r_out: &mut Ray,
    ) -> bool;
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct HitData {
    pub t: f32,
    pub p: Vec3,
    pub normal: Vec3,
    pub material: MaterialInfo,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub enum MaterialKind {
    Lambertian,
    Metal,
    Dielectric,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct MaterialInfo {
    pub kind: MaterialKind,
    pub index: usize,
}

/// All materials in the world.
// TODO: Not a portable way of storing materials for a world... Need ADTs or trait objects.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Materials<const NL: usize, const NM: usize, const ND: usize> {
    pub lambertian: [Lambertian; NL],
    pub metal: [Metal; NM],
    pub dielectric: [Dielectric; ND],
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Lambertian {
    pub albedo: Vec3,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Dielectric {
    // TODO: This should just be a float, but for some reason results in this error if it isn't a
    // Vec3.
    //
    // error: Cannot cast between pointer types
    //    --> shared/./src/lib.rs:207:6
    //     |
    // 207 |     }
    //     |      ^
    //     |
    //     = note: from: *struct Dielectric { ref_idx: f32 }
    //     = note: to: *u32
    pub ref_idx: Vec3,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Metal {
    pub albedo: Vec3,
    pub fuzz: f32,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct ShaderConstants {
    pub view_size_pixels: [u32; 2],
    pub mouse_pixels: [f32; 2],
    pub time: f32,
    pub rng_seed_offset: f32,

    // Rendering
    pub rays_per_pixel: u32,
    pub ray_bounce_limit: u32,

    // Camera
    pub vfov: f32,
    pub aperture: f32,
    //pub focus_dist: f32,

    // TODO: This would be awesome for automatically improving scene quality when the camera
    // reaches a resting state.
    // pub time_since_camera_move: f32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Ray {
    pub a: Vec3,
    pub b: Vec3,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: MaterialInfo,
}

#[derive(Clone, Default)]
pub struct Rng {
    pub seed: Vec2,
}

#[derive(Clone)]
pub struct Camera {
    pub origin: Vec3,
    pub lower_left_corner: Vec3,
    pub horizontal: Vec3,
    pub vertical: Vec3,

    pub u: Vec3,
    pub v: Vec3,
    pub w: Vec3,
    pub lens_radius: f32,
}

impl Camera {
    // pub fn new(window_size_px: Vec2) -> Self {
    //     let w_px = window_size_px.x;
    //     let h_px = window_size_px.y;
    //     let min_side_px = w_px.min(h_px);

    /// Vertical field of view in radians and aspect ratio.
    pub fn new(
        from: Vec3,
        to: Vec3,
        vup: Vec3,
        vfov: f32,
        aspect: f32,
        aperture: f32,
        focus_dist: f32,
    ) -> Self {
        let lens_radius = aperture * 0.5;

        let half_h = (vfov * 0.5).tan();
        let half_w = aspect * half_h;

        let origin = from;
        let w = unit_vector(from - to);
        let u = unit_vector(vup.cross(w));
        let v = w.cross(u);

        let lower_left_corner = origin
            - half_w * focus_dist * u
            - half_h * focus_dist * v
            - focus_dist * w;
        let horizontal = u * 2.0 * half_w * focus_dist;
        let vertical = v * 2.0 * half_h * focus_dist;

        Self {
            lower_left_corner,
            origin,
            horizontal,
            vertical,
            u,
            v,
            w,
            lens_radius,
        }
    }

    pub fn ray(&self, rng: &mut Rng, uv: Vec2) -> Ray {
        let rd = self.lens_radius * random_in_unit_disk(rng);
        let offset = self.u * rd.x + self.v * rd.y;
        Ray {
            a: self.origin + offset,
            b: self.lower_left_corner + uv.x * self.horizontal + uv.y * self.vertical - self.origin - offset,
        }
    }
}

impl Ray {
    pub fn new(a: Vec3, b: Vec3) -> Self {
        Ray { a, b }
    }

    pub fn origin(&self) -> Vec3 {
        self.a
    }

    pub fn direction(&self) -> Vec3 {
        self.b
    }

    pub fn point_at_parameter(&self, t: f32) -> Vec3 {
        self.a + self.b * t
    }
}

impl Rng {
    pub fn gen_signed(&mut self) -> f32 {
        let res = (self.seed.dot(vec2(12.9898, 78.233)).sin() * 43758.5453).fract();
        self.seed = vec2(
            (self.seed.x + res + 17.825) % 3718.0,
            (self.seed.y + res + 72.7859) % 1739.0,
        );
        res
    }

    pub fn gen(&mut self) -> f32 {
        self.gen_signed() * 0.5 + 0.5
    }
}

impl Lambertian {
    pub fn new(albedo: Vec3) -> Self {
        Self { albedo }
    }

    pub fn scatter_ray(
        &self,
        hit: &HitData,
        rng: &mut Rng,
        attenuation: &mut Vec3,
        r_out: &mut Ray,
    ) {
        let target = hit.p + hit.normal + random_in_unit_sphere(rng);
        *r_out = Ray::new(hit.p, target - hit.p);
        *attenuation = self.albedo;
    }
}

impl Metal {
    pub fn new(albedo: Vec3, fuzz: f32) -> Self {
        Self { albedo, fuzz }
    }
}

impl Dielectric {
    pub fn new(ref_idx: f32) -> Self {
        Self { ref_idx: Vec3::splat(ref_idx) }
    }
}

impl<T: Copy + Hit, const N: usize> Hit for [T; N] {
    fn hit(self, r: &Ray, t_min: f32, t_max: f32, hit: &mut HitData) -> bool {
        let mut did_hit = false;
        let mut closest_t = t_max;
        let mut temp_hit = HitData::default();
        for i in 0..N {
            if self[i].hit(r, t_min, closest_t, &mut temp_hit) {
                did_hit = true;
                closest_t = temp_hit.t;
                *hit = temp_hit;
            }
        }
        did_hit
    }
}

impl Hit for Sphere {
    fn hit(self, r: &Ray, t_min: f32, t_max: f32, hit: &mut HitData) -> bool {
        (&self).hit(r, t_min, t_max, hit)
    }
}

impl<'a> Hit for &'a Sphere {
    fn hit(self, r: &Ray, t_min: f32, t_max: f32, hit: &mut HitData) -> bool {
        let Sphere { center, radius, material } = *self;
        let origin = r.origin();
        let direction = r.direction();
        let oc = origin - center;
        let a = direction.dot(direction);
        let b = oc.dot(direction);
        let c = oc.dot(oc) - radius * radius;
        let discriminant = b * b - a * c;
        if discriminant > 0.0 {
            let mut temp = (-b - (b * b - a * c).sqrt()) / a;
            if temp < t_max && temp > t_min {
                hit.t = temp;
                hit.p = r.point_at_parameter(hit.t);
                hit.normal = (hit.p - center) / radius;
                hit.material = material;
                return true;
            }
            temp = (-b + (b * b - a * c).sqrt()) / a;
            if temp < t_max && temp > t_min {
                hit.t = temp;
                hit.p = r.point_at_parameter(hit.t);
                hit.normal = (hit.p - center) / radius;
                hit.material = material;
                return true;
            }
        }
        false
    }
}

impl Material for Lambertian {
    fn scatter(
        self,
        _: &Ray,
        hit: &HitData,
        rng: &mut Rng,
        attenuation: &mut Vec3,
        ray_out: &mut Ray,
    ) -> bool {
        self.scatter_ray(hit, rng, attenuation, ray_out);
        true
    }
}

impl Material for Metal {
    fn scatter(
        self,
        ray_in: &Ray,
        hit: &HitData,
        rng: &mut Rng,
        attenuation: &mut Vec3,
        ray_out: &mut Ray,
    ) -> bool {
        let reflected = reflect(unit_vector(ray_in.direction()), hit.normal);
        *ray_out = Ray::new(hit.p, reflected + self.fuzz * random_in_unit_sphere(rng));
        *attenuation = self.albedo;
        ray_out.direction().dot(hit.normal) > 0.0
    }
}

impl Material for Dielectric {
    fn scatter(
        self,
        ray_in: &Ray,
        hit: &HitData,
        rng: &mut Rng,
        attenuation: &mut Vec3,
        ray_out: &mut Ray,
    ) -> bool {
        let ray_in_dir = ray_in.direction();
        let reflected = reflect(ray_in_dir, hit.normal);
        *attenuation = Vec3::ONE;
        let ray_in_dir_dot_normal = ray_in_dir.dot(hit.normal);
        let (outward_normal, ni_over_nt, cos) = if ray_in_dir_dot_normal > 0.0 {
            let cos = self.ref_idx.x * ray_in_dir_dot_normal / ray_in_dir.length();
            (-hit.normal, self.ref_idx.x, cos)
        } else {
            let cos = -ray_in_dir_dot_normal / ray_in_dir.length();
            (hit.normal, 1.0 / self.ref_idx.x, cos)
        };
        let mut refracted = Vec3::ZERO;
        let reflect_prob = if refract(ray_in.direction(), outward_normal, ni_over_nt, &mut refracted) {
            schlick(cos, self.ref_idx.x)
        } else {
            1.0
        };
        if rng.gen() < reflect_prob {
            *ray_out = Ray::new(hit.p, reflected);
        } else {
            *ray_out = Ray::new(hit.p, refracted);
        }
        true
    }
}

impl<'a, const NL: usize, const NM: usize, const ND: usize> Material for &'a Materials<NL, NM, ND> {
    fn scatter(
        self,
        ray_in: &Ray,
        hit: &HitData,
        rng: &mut Rng,
        attenuation: &mut Vec3,
        ray_out: &mut Ray,
    ) -> bool {
        match hit.material.kind {
            MaterialKind::Lambertian => {
                self.lambertian[hit.material.index].scatter(ray_in, hit, rng, attenuation, ray_out)
            }
            MaterialKind::Metal => {
                self.metal[hit.material.index].scatter(ray_in, hit, rng, attenuation, ray_out)
            }
            MaterialKind::Dielectric => {
                self.dielectric[hit.material.index].scatter(ray_in, hit, rng, attenuation, ray_out)
            }
        }
    }
}

impl Default for MaterialInfo {
    fn default() -> Self {
        let kind = Default::default();
        let index = 0;
        MaterialInfo { kind, index }
    }
}

impl Default for MaterialKind {
    fn default() -> Self {
        MaterialKind::Lambertian
    }
}

pub fn unit_vector(v: Vec3) -> Vec3 {
    v / v.length()
}

fn random_in_unit_sphere(rng: &mut Rng) -> Vec3 {
    let mut p;
    loop {
        let rv = vec3(rng.gen(), rng.gen(), rng.gen());
        p = 2.0 * rv - Vec3::ONE;
        let p_len2 = p.length_squared();
        if p_len2 < 1.0 {
            break p;
        }
    }
}

fn random_in_unit_disk(rng: &mut Rng) -> Vec3 {
    let mut p;
    loop {
        let rv = vec3(rng.gen(), rng.gen(), 0.0);
        p = 2.0 * rv - vec3(1.0, 1.0, 0.0);
        if p.dot(p) < 1.0 {
            break p;
        }
    }
}

pub fn reflect(v: Vec3, n: Vec3) -> Vec3 {
    v - 2.0 * v.dot(n) * n
}

pub fn refract(v: Vec3, n: Vec3, ni_over_nt: f32, refracted: &mut Vec3) -> bool {
    let uv = unit_vector(v);
    let dt = uv.dot(n);
    let discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);
    if discriminant > 0.0 {
        *refracted = ni_over_nt * (uv - n * dt) - n * discriminant.sqrt();
        true
    } else {
        false
    }
}

pub fn schlick(cos: f32, ref_idx: f32) -> f32 {
    let mut r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    r0 = r0 * r0;
    r0 + (1.0 - r0) * (1.0 - cos).powf(5.0)
}

pub fn color(
    ray_bounce_limit: u32,
    rng: &mut Rng,
    mut ray: Ray,
    world: impl Copy + Hit,
    materials: impl Copy + Material,
) -> Vec3 {
    let mut hit = HitData::default();
    let mut scattered = Ray::new(Vec3::ZERO, Vec3::ONE); // placeholder to initialise.
    let mut attenuation = Vec3::default();

    let min_f = 0.001;
    let max_f = core::f32::MAX;
    let mut color = Vec3::ONE;
    let mut bounces = 0;
    while world.hit(&ray, min_f, max_f, &mut hit) {
        if bounces < ray_bounce_limit && materials.scatter(&ray, &hit, rng, &mut attenuation, &mut scattered) {
            color *= attenuation;
            ray = scattered;
        } else {
            color = Vec3::ZERO;
            break;
        }
        bounces += 1;
    }

    let sky = color_sky(&ray);
    sky * color
}

fn color_sky(ray: &Ray) -> Vec3 {
    let unit_direction = unit_vector(ray.direction()) * 2.0;
    let t = 0.5 * (unit_direction.y + 1.0);
    (1.0 - t) * vec3(1.0, 1.0, 1.0) + t * vec3(0.5, 0.7, 1.0)
}
