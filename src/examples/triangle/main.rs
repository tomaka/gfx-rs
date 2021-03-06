#![feature(phase)]
#![crate_name = "triangle"]

extern crate gfx;
#[phase(plugin)]
extern crate gfx_macros;
extern crate glfw;
extern crate native;

use gfx::{Device, DeviceHelper};
use glfw::Context;

#[vertex_format]
struct Vertex {
    pos: [f32, ..2],
    color: [f32, ..3],
}

static VERTEX_SRC: gfx::ShaderSource = shaders! {
GLSL_120: b"
    #version 120
    attribute vec2 pos;
    attribute vec3 color;
    varying vec4 v_Color;
    void main() {
        v_Color = vec4(color, 1.0);
        gl_Position = vec4(pos, 0.0, 1.0);
    }
"
GLSL_150: b"
    #version 150 core
    in vec2 pos;
    in vec3 color;
    out vec4 v_Color;
    void main() {
        v_Color = vec4(color, 1.0);
        gl_Position = vec4(pos, 0.0, 1.0);
    }
"
};

static FRAGMENT_SRC: gfx::ShaderSource = shaders! {
GLSL_120: b"
    #version 120
    varying vec4 v_Color;
    void main() {
        gl_FragColor = v_Color;
    }
"
GLSL_150: b"
    #version 150 core
    in vec4 v_Color;
    out vec4 o_Color;
    void main() {
        o_Color = v_Color;
    }
"
};

// We need to run on the main thread for GLFW, so ensure we are using the `native` runtime. This is
// technically not needed, since this is the default, but it's not guaranteed.
#[start]
fn start(argc: int, argv: *const *const u8) -> int {
     native::start(argc, argv, main)
}

fn main() {
    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    glfw.window_hint(glfw::ContextVersion(3, 2));
    glfw.window_hint(glfw::OpenglForwardCompat(true));
    glfw.window_hint(glfw::OpenglProfile(glfw::OpenGlCoreProfile));

    let (window, events) = glfw
        .create_window(640, 480, "Triangle example.", glfw::Windowed)
        .expect("Failed to create GLFW window.");

    window.make_current();
    glfw.set_error_callback(glfw::FAIL_ON_ERRORS);
    window.set_key_polling(true); // so we can quit when Esc is pressed
    let (w, h) = window.get_framebuffer_size();
    let frame = gfx::Frame::new(w as u16, h as u16);

    let mut device = gfx::GlDevice::new(|s| glfw.get_proc_address(s));
    let mut list = device.create_draw_list();

    let state = gfx::DrawState::new();
    let vertex_data = vec![
        Vertex { pos: [ -0.5, -0.5 ], color: [1.0, 0.0, 0.0] },
        Vertex { pos: [ 0.5, -0.5 ], color: [0.0, 1.0, 0.0]  },
        Vertex { pos: [ 0.0, 0.5 ], color: [0.0, 0.0, 1.0]  }
    ];
    let mesh = device.create_mesh(vertex_data);
    let program: gfx::shade::EmptyProgram = device.link_program(
        VERTEX_SRC.clone(), FRAGMENT_SRC.clone()).unwrap();

    list.clear(
        gfx::ClearData {
            color: Some(gfx::Color([0.3, 0.3, 0.3, 1.0])),
            depth: None,
            stencil: None,
        },
        &frame
    );
    list.draw(&mesh, mesh.get_slice(), &frame, &program, &state)
        .unwrap();

    while !window.should_close() {
        glfw.poll_events();
        // quit when Esc is pressed.
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::KeyEvent(glfw::KeyEscape, _, glfw::Press, _) =>
                    window.set_should_close(true),
                _ => {},
            }
        }
        device.submit(list.as_slice());
        window.swap_buffers();
    }
}
