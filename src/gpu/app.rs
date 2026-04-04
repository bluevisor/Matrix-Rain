use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowId, Fullscreen},
};

use crate::gpu::atlas::GlyphAtlas;
use crate::gpu::camera::Camera;
use crate::gpu::rain::{self, RainSimulation, GREEN_THEME};
use crate::gpu::renderer::Renderer;

pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Option<Camera>,
    rain: Option<RainSimulation>,
    atlas: Option<GlyphAtlas>,
    chars: Vec<char>,
    is_fullscreen: bool,
    frame_count: u64,
}

impl App {
    pub fn new() -> Self {
        let chars = rain::char_set();
        App {
            window: None,
            renderer: None,
            camera: None,
            rain: None,
            atlas: None,
            chars,
            is_fullscreen: false,
            frame_count: 0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Matrix Rain — GPU")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

        let window = Arc::new(event_loop.create_window(window_attrs).expect("Failed to create window"));
        let size = window.inner_size();

        let mut renderer = Renderer::new(window.clone());

        // Create atlas and filter to only chars that rendered visible pixels
        let atlas = GlyphAtlas::new(&self.chars, 48.0);
        renderer.upload_atlas(&atlas);
        self.chars = atlas.valid_chars.clone();

        let camera = Camera::new(size.width as f32 / size.height as f32);

        let rain = RainSimulation::new(
            80,   // columns
            60,   // rows
            10,   // layers
            0.55, // density
            self.chars.len(),
        );

        self.atlas = Some(atlas);
        self.camera = Some(camera);
        self.rain = Some(rain);
        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                if let Some(camera) = &mut self.camera {
                    if size.height > 0 {
                        camera.set_aspect(size.width as f32 / size.height as f32);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    logical_key, state: ElementState::Pressed, ..
                },
                ..
            } => {
                match logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => {
                        event_loop.exit();
                    }
                    Key::Character("q") | Key::Character("Q") => {
                        event_loop.exit();
                    }
                    Key::Character(",") => {
                        if let Some(camera) = &mut self.camera {
                            camera.zoom_out();
                        }
                    }
                    Key::Character(".") => {
                        if let Some(camera) = &mut self.camera {
                            camera.zoom_in();
                        }
                    }
                    Key::Character("f") | Key::Character("F") => {
                        if let Some(window) = &self.window {
                            self.is_fullscreen = !self.is_fullscreen;
                            if self.is_fullscreen {
                                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                            } else {
                                window.set_fullscreen(None);
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(camera), Some(rain), Some(atlas)) =
                    (&mut self.renderer, &self.camera, &mut self.rain, &self.atlas)
                {
                    rain.update();

                    let instances = rain.generate_instances(
                        &self.chars,
                        atlas,
                        &GREEN_THEME,
                        0.5,   // col_spacing
                        0.5,   // row_height
                        8.0,   // layer_spacing
                        0.0,   // grid_offset_x
                    );

                    self.frame_count += 1;
                    if self.frame_count % 120 == 1 {
                        eprintln!("Frame {}: {} instances, {} streams", self.frame_count, instances.len(), rain.streams.len());
                        if let Some(inst) = instances.first() {
                            eprintln!("  First instance pos: {:?}, color: {:?}", inst.position, inst.color);
                        }
                    }

                    renderer.render(camera, &instances);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
