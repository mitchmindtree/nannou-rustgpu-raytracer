use crate::Config;
use fps_ticker::Fps;
use nannou::prelude::*;
use nannou::ui::prelude::*;
use shared::ShaderConstants;

pub const WIN_W: u32 = 280;
pub const WIN_X: i32 = PAD as i32;
pub const PAD: Scalar = 20.0;
const COL_W: Scalar = WIN_W as Scalar - PAD * 2.0;
const LABEL_FONT_SIZE: u32 = 12;
const DEFAULT_WIDGET_H: Scalar = 30.0;

widget_ids! {
    pub struct Ids {
        background,
        title_text,
        scene_fps_text,
        scene_fps_avg_text,
        scene_fps_min_text,
        scene_fps_max_text,
        render_text,
        render_scale_slider,
        rays_per_pixel_slider,
        ray_bounce_limit_slider,
        seed_rng_with_time_button,
        camera_text,
        camera_vfov_slider,
        camera_aperture_slider,
        camera_focus_dist_slider,
    }
}

/// Update the user interface.
pub fn update(
    ref mut ui: UiCell,
    ids: &Ids,
    scene_fps: &Fps,
    config: &mut Config,
    push_constants: &mut ShaderConstants,
) {
    widget::Canvas::new()
        .border(0.0)
        .rgb(0.1, 0.1, 0.1)
        .pad(PAD)
        .set(ids.background, ui);

    // Title

    text("NANNOU RAYTRACER")
        .mid_top_of(ids.background)
        .set(ids.title_text, ui);

    // Scene FPS

    fn fps_to_rgb(fps: f64) -> (f32, f32, f32) {
        let r = clamp(map_range(fps, 0.0, 60.0, 1.0, 0.0), 0.0, 1.0);
        let g = clamp(map_range(fps, 0.0, 60.0, 0.0, 1.0), 0.0, 1.0);
        let b = 0.5;
        (r, g, b)
    }

    widget::Text::new("Scene Performance")
        .mid_left_of(ids.background)
        .down(PAD * 1.5)
        .font_size(16)
        .color(color::WHITE)
        .set(ids.scene_fps_text, ui);

    let label = format!("{:.2} AVG FPS", scene_fps.avg());
    let (r, g, b) = fps_to_rgb(scene_fps.avg());
    widget::Text::new(&label)
        .down(PAD)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.scene_fps_avg_text, ui);

    let label = format!("{:.2} MIN FPS", scene_fps.min());
    let (r, g, b) = fps_to_rgb(scene_fps.min());
    widget::Text::new(&label)
        .down(PAD * 0.5)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.scene_fps_min_text, ui);

    let label = format!("{:.2} MAX FPS", scene_fps.max());
    let (r, g, b) = fps_to_rgb(scene_fps.max());
    widget::Text::new(&label)
        .down(PAD * 0.5)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.scene_fps_max_text, ui);

    // Render

    widget::Text::new("Render Control")
        .mid_left_of(ids.background)
        .down(PAD * 1.5)
        .font_size(16)
        .color(color::WHITE)
        .set(ids.render_text, ui);

    let min = 0.025;
    let max = 1.0;
    let label = format!("Render scale: {:.2}", config.render_scale);
    for scale in slider(config.render_scale, min, max)
        .label(&label)
        .down(PAD)
        .set(ids.render_scale_slider, ui)
    {
        config.render_scale = scale;
    }

    let min = 1.0;
    let max = 100.0;
    let label = format!("Rays per pixel: {}", push_constants.rays_per_pixel);
    for rays in slider(push_constants.rays_per_pixel as f32, min, max)
        .label(&label)
        .mid_left_of(ids.background)
        .skew(2.0)
        .down(PAD * 0.5)
        .set(ids.rays_per_pixel_slider, ui)
    {
        push_constants.rays_per_pixel = rays.round() as u32;
    }

    let min = 1.0;
    let max = 50.0;
    let label = format!("Ray bounce limit: {}", push_constants.ray_bounce_limit);
    for limit in slider(push_constants.ray_bounce_limit as f32, min, max)
        .label(&label)
        .down(PAD * 0.5)
        .set(ids.ray_bounce_limit_slider, ui)
    {
        push_constants.ray_bounce_limit = limit.round() as u32;
    }

    let (label, color) = match config.seed_rng_with_time {
        true => ("ON", ui::color::BLUE),
        false => ("OFF", ui::color::DARK_CHARCOAL),
    };
    let label = format!("Animate Noise: {}", label);
    for _click in button()
        .label(&label)
        .color(color)
        .down(PAD * 0.5)
        .set(ids.seed_rng_with_time_button, ui)
    {
        config.seed_rng_with_time = !config.seed_rng_with_time;
    }

    // Camera

    widget::Text::new("Camera")
        .mid_left_of(ids.background)
        .down(PAD * 1.5)
        .font_size(16)
        .color(color::WHITE)
        .set(ids.camera_text, ui);

    let pi = core::f32::consts::PI;
    let min = pi * 0.16;
    let max = pi - min;
    let label = format!("Field of View: {:.3} radians", push_constants.vfov);
    for vfov in slider(push_constants.vfov, min, max)
        .label(&label)
        .down(PAD)
        .set(ids.camera_vfov_slider, ui)
    {
        push_constants.vfov = vfov;
    }

    let min = 0.0;
    let max = 4.0;
    let label = format!("Aperture: {:.3}", push_constants.aperture);
    for value in slider(push_constants.aperture, min, max)
        .label(&label)
        .down(PAD * 0.5)
        .skew(2.0)
        .set(ids.camera_aperture_slider, ui)
    {
        push_constants.aperture = value;
    }

}

fn text(s: &str) -> widget::Text {
    widget::Text::new(s).color(color::WHITE)
}

fn button() -> widget::Button<'static, widget::button::Flat> {
    widget::Button::new()
        .w_h(COL_W, DEFAULT_WIDGET_H)
        .label_font_size(LABEL_FONT_SIZE)
        .color(color::DARK_CHARCOAL)
        .label_color(color::WHITE)
        .border(0.0)
}

fn slider(val: f32, min: f32, max: f32) -> widget::Slider<'static, f32> {
    widget::Slider::new(val, min, max)
        .w_h(COL_W, DEFAULT_WIDGET_H)
        .label_font_size(LABEL_FONT_SIZE)
        .color(color::DARK_CHARCOAL)
        .label_color(color::WHITE)
        .border(0.0)
}
