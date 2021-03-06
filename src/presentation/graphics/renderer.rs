extern crate glium;
extern crate glium_sdl2;
extern crate sdl2;
extern crate image;

use crate::constants::presentation::*;
use std::path::Path;
use glium_sdl2::SDL2Facade;
use sdl2::{Sdl, EventPump};

pub fn create_window() -> (Sdl, SDL2Facade, EventPump) {
    use glium_sdl2::DisplayBuild;
    // initialize SDL library
    let sdl_context = sdl2::init().unwrap();
    // initialize video subsystem
    let video_subsystem = sdl_context.video().unwrap();
    // OpenGL context getters and setters
    let gl_attr = video_subsystem.gl_attr();

    // setup OpenGL profile
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core); // setting type of GL context
    // Set the context into debug mode
    gl_attr.set_context_flags().debug().set();

    // setup OpenGL profile
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core); // setting type of GL context
    // Set the context into debug mode
    gl_attr.set_context_flags().debug().set();

    {
        // OpenGL version switcher for platform compatibility
        let major = 3;
        let minor = 3;
        gl_attr.set_context_version(major, minor); // specifying OpenGL version
    }

    // creating window
    // available functionality: https://nukep.github.io/rust-sdl2/sdl2/video/struct.WindowBuilder.html#method.resizable
    let window = video_subsystem
        .window(WINDOW_TITLE, WINDOW_W, WINDOW_H)
        .resizable()
        .build_glium()
        .unwrap();

    // force vsync
    video_subsystem.gl_set_swap_interval(1);

    let event_pump = sdl_context.event_pump().unwrap();

    (sdl_context, window, event_pump)
}

pub fn load_texture(window: &glium_sdl2::SDL2Facade, path: &str) -> glium::texture::texture2d::Texture2d {
    let image = image::open(Path::new(path)).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    let texture = glium::texture::Texture2d::new(window, image).unwrap();
    (texture)
}

// use sdl2::ttf;
// pub fn create_font() {
    // let ttf_context = ttf::init().unwrap();
    // let _font = ttf_context.load_font("assets/consola.ttf", 50).unwrap();
// }
