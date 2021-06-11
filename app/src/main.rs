use fps_ticker::Fps;
use nannou::prelude::*;
use nannou::ui::prelude::*;
use shared::ShaderConstants;
use spirv_builder::{Capability, MetadataPrintout, SpirvBuilder};
use std::borrow::Cow;
use std::path::PathBuf;

mod gui;
mod shaders {
    #[allow(non_upper_case_globals)]
    pub const main_fs: &str = "main_fs";
    #[allow(non_upper_case_globals)]
    pub const main_vs: &str = "main_vs";
}

fn main() {
    nannou::app(model).update(update).run();
}

struct Model {
    gui_window: window::Id,
    scene_window: window::Id,
    push_constants: ShaderConstants,
    shader_mod: wgpu::ShaderModule,
    graphics: Graphics,
    config: Config,
    scene_fps: Fps,
    ui: Ui,
    ids: gui::Ids,
}

pub struct Config {
    pub render_scale: f32,
    pub seed_rng_with_time: bool,
}

struct Graphics {
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    // The scaled texture to which the raytraced scene is rendered.
    scaled_texture: wgpu::Texture,
    // Reshapes the scene texture to the swap chain image texture.
    texture_reshaper: wgpu::TextureReshaper,
}

const WIN_H: u32 = 640;
const SCENE_WIN_W: u32 = WIN_H * 21 / 9;
const WIN_Y: i32 = gui::PAD as i32;
const SCENE_WIN_X: i32 = gui::WIN_X + gui::WIN_W as i32 + gui::PAD as i32;

impl Default for Config {
    fn default() -> Self {
        Self {
            render_scale: 0.5,
            seed_rng_with_time: true,
        }
    }
}

fn model(app: &App) -> Model {
    let gui_window = app
        .new_window()
        .title("raytracer controls")
        .size(gui::WIN_W, WIN_H)
        .view(view_ui)
        .build()
        .unwrap();

    // We need a window backed by a wgpu device that supports push constants.
    let device_desc = wgpu::DeviceDescriptor {
        label: Some("nannou-raytracer-device"),
        features: wgpu::Features::PUSH_CONSTANTS,
        limits: wgpu::Limits {
            max_push_constant_size: 256,
            ..Default::default()
        },
    };

    let scene_window = app
        .new_window()
        .title("nannou + rust-gpu raytracer")
        .device_descriptor(device_desc)
        .size(SCENE_WIN_W, WIN_H)
        .view(view_scene)
        .build()
        .unwrap();

    let w = app.window(gui_window).expect("UI window closed unexpectedly");
    let scale = w.scale_factor();
    let y = (WIN_Y as f32 * scale) as i32;
    let x = (gui::WIN_X as f32 * scale) as i32;
    w.set_outer_position_pixels(x, y);

    {
        let w = app.window(scene_window)
            .expect("scene window closed unexpectedly");
        let x = (SCENE_WIN_X as f32 * scale) as i32;
        w.set_outer_position_pixels(x, WIN_Y);
    }

    // Initialise UI.
    let mut ui = app
        .new_ui()
        .window(gui_window)
        .build()
        .expect("failed to build `Ui` for GUI window");
    let ids = gui::Ids::new(ui.widget_id_generator());

    // Load the rust-gpu shader.
    let scene_win = app.window(scene_window).unwrap();
    let device = scene_win.swap_chain_device();
    let shader_mod_desc = load_shader_module_desc();
    let shader_mod = device.create_shader_module(&shader_mod_desc);

    let scene_fps = Fps::default();
    let config = Config::default();
    let push_constants = ShaderConstants {
        rays_per_pixel: 2,
        ray_bounce_limit: 8,
        vfov: core::f32::consts::PI * 0.5,
        aperture: 0.0,
        ..Default::default()
    };
    let msaa_samples = scene_win.msaa_samples();
    let format = Frame::TEXTURE_FORMAT;
    let (w_px, h_px) = scene_win.inner_size_pixels();
    let scaled_texture_size = scaled_texture_size([w_px, h_px], config.render_scale);
    let graphics = create_graphics(device, &shader_mod, format, msaa_samples, scaled_texture_size);

    Model {
        gui_window,
        scene_window,
        shader_mod,
        graphics,
        scene_fps,
        config,
        push_constants,
        ui,
        ids,
    }
}

fn update(app: &App, model: &mut Model, _: Update) {
    {
        let ui = model.ui.set_widgets();
        gui::update(
            ui,
            &model.ids,
            &model.scene_fps,
            &mut model.config,
            &mut model.push_constants,
        );
    }

    // Recreate scaled texture and reshaper if scale changed.
    let win = app.window(model.scene_window).unwrap();
    let (win_w_px, win_h_px) = win.inner_size_pixels();
    let scaled_texture_size = scaled_texture_size([win_w_px, win_h_px], model.config.render_scale);
    if scaled_texture_size != model.graphics.scaled_texture.size() {
        let device = win.swap_chain_device();
        let msaa_samples = win.msaa_samples();
        let format = Frame::TEXTURE_FORMAT;
        model.graphics = create_graphics(device, &model.shader_mod, format, msaa_samples, scaled_texture_size);
    }

    let pc = &mut model.push_constants;

    pc.time = app.time;
    pc.rng_seed_offset = if model.config.seed_rng_with_time {
        app.time
    } else {
        0.0
    };

    let [w_px, h_px] = model.graphics.scaled_texture.size();
    pc.view_size_pixels = [w_px, h_px];

    let win = app.window(model.scene_window).unwrap();
    let win_rect = win.rect();
    let m = app.mouse.position();
    let mouse_x = map_range(m.x, win_rect.left(), win_rect.right(), 0.0, w_px as f32);
    let mouse_y = map_range(m.y, win_rect.top(), win_rect.bottom(), 0.0, h_px as f32);
    pc.mouse_pixels = [mouse_x, mouse_y];
}

fn view_ui(app: &App, model: &Model, frame: Frame) {
    frame.clear(BLACK);
    model
        .ui
        .draw_to_frame(app, &frame)
        .expect("failed to draw `Ui` to `Frame`");
}

fn view_scene(_app: &App, model: &Model, frame: Frame) {
    frame.clear(BLACK);

    // Encode the commands for rendering to the scaled texture.
    let mut encoder = frame.command_encoder();
    {
        let texture_view = model.graphics.scaled_texture.view().build();
        let mut render_pass = wgpu::RenderPassBuilder::new()
            .color_attachment(&texture_view, |color| color.load_op(wgpu::LoadOp::Load))
            .begin(&mut encoder);
        render_pass.set_pipeline(&model.graphics.pipeline);
        let pc_bytes = unsafe { any_as_u8_slice(&model.push_constants) };
        render_pass.set_push_constants(wgpu::ShaderStage::all(), 0, pc_bytes);
        let vertex_range = 0..3;
        let instance_range = 0..1;
        render_pass.draw(vertex_range, instance_range);
    }

    // Draw the scaled texture to the frame.
    model
        .graphics
        .texture_reshaper
        .encode_render_pass(frame.texture_view(), &mut *encoder);

    model.scene_fps.tick();
}

fn scaled_texture_size(win_size_px: [u32; 2], scale: f32) -> [u32; 2] {
    let [w, h] = win_size_px;
    [(w as f32 * scale) as u32, (h as f32 * scale) as u32]
}

fn create_graphics(
    device: &wgpu::Device,
    shader_mod: &wgpu::ShaderModule,
    dst_format: wgpu::TextureFormat,
    sample_count: u32,
    scaled_texture_size: [u32; 2],
) -> Graphics {
    let scaled_texture_sample_count = 1;
    let scaled_texture_format = wgpu::TextureFormat::Rgba16Float;

    // Create our custom texture.
    let scaled_texture = wgpu::TextureBuilder::new()
        .size(scaled_texture_size)
        // Our texture will be used as the RENDER_ATTACHMENT for our `Draw` render pass.
        // It will also be SAMPLED by the `TextureCapturer` and `TextureResizer`.
        .usage(wgpu::TextureUsage::RENDER_ATTACHMENT | wgpu::TextureUsage::SAMPLED)
        // Use nannou's default multisampling sample count.
        .sample_count(scaled_texture_sample_count)
        // Use a spacious 16-bit linear sRGBA format suitable for high quality drawing.
        .format(scaled_texture_format)
        // Build it!
        .build(device);

    // Create the texture reshaper.
    let texture_view = scaled_texture.view().build();
    let texture_sample_type = scaled_texture.sample_type();
    let texture_reshaper = wgpu::TextureReshaper::new(
        device,
        &texture_view,
        scaled_texture_sample_count,
        texture_sample_type,
        sample_count,
        dst_format,
    );

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("nannou-raytracer-pipeline-layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[wgpu::PushConstantRange {
            stages: wgpu::ShaderStage::all(),
            range: 0..std::mem::size_of::<ShaderConstants>() as u32,
        }],
    });

    let pipeline = wgpu::RenderPipelineBuilder::from_layout(&pipeline_layout, &shader_mod)
        .fragment_shader(&shader_mod)
        .vertex_entry_point(shaders::main_vs)
        .fragment_entry_point(shaders::main_fs)
        .color_format(scaled_texture_format)
        .color_blend(wgpu::BlendComponent::OVER)
        .alpha_blend(wgpu::BlendComponent::REPLACE)
        .sample_count(scaled_texture_sample_count)
        .build(device);

    Graphics {
        pipeline_layout,
        pipeline,
        scaled_texture,
        texture_reshaper,
    }
}

fn load_shader_module_desc() -> wgpu::ShaderModuleDescriptor<'static> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let crate_path = [manifest_dir, "..", "shader"]
        .iter()
        .copied()
        .collect::<PathBuf>();
    let compile_result = SpirvBuilder::new(crate_path, "spirv-unknown-vulkan1.1")
        .print_metadata(MetadataPrintout::None)
        // Seems to be needed to handle conditions within functions?
        // Error was confusing but adding this worked.
        .capability(Capability::Int8)
        .build()
        .unwrap();
    let module_path = compile_result.module.unwrap_single();
    let data = std::fs::read(module_path).unwrap();
    let spirv = wgpu::util::make_spirv(&data);
    let spirv = match spirv {
        wgpu::ShaderSource::Wgsl(cow) => wgpu::ShaderSource::Wgsl(Cow::Owned(cow.into_owned())),
        wgpu::ShaderSource::SpirV(cow) => {
            wgpu::ShaderSource::SpirV(Cow::Owned(cow.into_owned()))
        }
    };
    wgpu::ShaderModuleDescriptor {
        label: Some("nannou-raytracer-shader"),
        source: spirv,
        flags: wgpu::ShaderFlags::default(),
    }
}

// NOTE: Super unsafe for general use, OK for this case.
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts((p as *const T) as *const u8, std::mem::size_of::<T>())
}
