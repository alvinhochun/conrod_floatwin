use conrod_core::{
    widget, widget_ids, Borderable, Colorable, Labelable, Positionable, Sizeable, Widget,
};
use conrod_floatwin::windowing_area::{
    layout::{WinId, WindowingState},
    WindowBuilder, WindowingArea, WindowingContext,
};
use glium::Surface;

mod support;

fn main() {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    // Build the window.
    let mut events_loop = glium::glutin::EventsLoop::new();
    let window = glium::glutin::WindowBuilder::new()
        .with_title("conrod_floatwin demo")
        .with_dimensions((WIDTH, HEIGHT).into());
    let context = glium::glutin::ContextBuilder::new()
        .with_vsync(true)
        .with_multisampling(4);
    let display = glium::Display::new(window, context, &events_loop).unwrap();
    let display = support::GliumDisplayWinitWrapper(display);

    let mut current_hidpi_factor = display.0.gl_window().get_hidpi_factor();

    // construct our `Ui`.
    let mut ui = conrod_core::UiBuilder::new([WIDTH as f64, HEIGHT as f64]).build();

    // Add a `Font` to the `Ui`'s `font::Map` from file.
    let font_path =
        "D:/dev/experiments/rust-fiddle/conrod/assets/fonts/NotoSans/NotoSans-Regular.ttf";
    ui.fonts.insert_from_file(font_path).unwrap();

    // A type used for converting `conrod_core::render::Primitives` into `Command`s that can be used
    // for drawing to the glium `Surface`.
    let mut renderer = conrod_glium::Renderer::new(&display.0).unwrap();

    // The image map describing each of our widget->image mappings (in our case, none).
    let image_map = conrod_core::image::Map::<glium::texture::Texture2d>::new();

    // Instantiate the generated list of widget identifiers.
    let ids = &mut Ids::new(ui.widget_id_generator());

    // Instantiate the windowing state.
    let mut win_state = WindowingState::new();
    let win_ids = WinIds {
        test1: win_state.next_id(),
        test2: win_state.next_id(),
        test_array: Vec::new(),
    };

    let mut ui_state = UiState {
        enable_debug: false,
        win_state,
        win_ids,
        array_win_count: 0,
    };

    // Poll events from the window.
    let mut event_loop = support::EventLoop::new();
    'main: loop {
        // Handle all events.
        for event in event_loop.next(&mut events_loop) {
            // Use the `winit` backend feature to convert the winit event to a conrod one.
            if let Some(event) = support::convert_event(event.clone(), &display) {
                ui.handle_event(event);
                event_loop.needs_update();
            }

            match event {
                glium::glutin::Event::WindowEvent { event, .. } => match event {
                    // Break from the loop upon `Escape`.
                    glium::glutin::WindowEvent::CloseRequested
                    | glium::glutin::WindowEvent::KeyboardInput {
                        input:
                            glium::glutin::KeyboardInput {
                                virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => break 'main,
                    glium::glutin::WindowEvent::HiDpiFactorChanged(hidpi_factor) => {
                        current_hidpi_factor = hidpi_factor;
                    }
                    // Toggle fullscreen on `F11`.
                    glium::glutin::WindowEvent::KeyboardInput {
                        input:
                            glium::glutin::KeyboardInput {
                                virtual_keycode: Some(glium::glutin::VirtualKeyCode::F11),
                                state: glium::glutin::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => match display.0.gl_window().window().get_fullscreen() {
                        Some(_) => display.0.gl_window().window().set_fullscreen(None),
                        None => display.0.gl_window().window().set_fullscreen(Some(
                            display.0.gl_window().window().get_current_monitor(),
                        )),
                    },
                    glium::glutin::WindowEvent::KeyboardInput {
                        input:
                            glium::glutin::KeyboardInput {
                                virtual_keycode: Some(glium::glutin::VirtualKeyCode::F12),
                                state: glium::glutin::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => ui_state.enable_debug = !ui_state.enable_debug,
                    _ => (),
                },
                _ => (),
            }
        }

        // Instantiate all widgets in the GUI.
        set_widgets(
            ui.set_widgets(),
            ids,
            current_hidpi_factor,
            &mut ui_state,
        );

        // Get the underlying winit window and update the mouse cursor as set by conrod.
        display
            .0
            .gl_window()
            .window()
            .set_cursor(support::convert_mouse_cursor(ui.mouse_cursor()));

        // Render the `Ui` and then display it on the screen.
        if let Some(primitives) = ui.draw_if_changed() {
            renderer.fill(&display.0, primitives, &image_map);
            let mut target = display.0.draw();
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            renderer.draw(&display.0, &mut target, &image_map).unwrap();
            target.finish().unwrap();
        }
    }
}

widget_ids! {
    struct Ids {
        backdrop,
        windowing_area,
        text,
        button,
    }
}

struct WinIds {
    test1: WinId,
    test2: WinId,
    test_array: Vec<WinId>,
}

struct UiState {
    enable_debug: bool,
    win_state: WindowingState,
    win_ids: WinIds,
    // show_test1: bool,
    array_win_count: usize,
}

fn set_widgets(
    ref mut ui: conrod_core::UiCell,
    ids: &mut Ids,
    hidpi_factor: f64,
    state: &mut UiState,
) {
    if state.win_ids.test_array.len() < state.array_win_count {
        let win_state = &mut state.win_state;
        state
            .win_ids
            .test_array
            .resize_with(state.array_win_count, || win_state.next_id());
    }
    widget::Rectangle::fill(ui.window_dim())
        .color(conrod_core::color::BLUE)
        .middle()
        .set(ids.backdrop, ui);
    let mut win_ctx: WindowingContext = WindowingArea::new(&mut state.win_state, hidpi_factor)
        .with_debug(state.enable_debug)
        .set(ids.windowing_area, ui);
    let builder = WindowBuilder::new()
        .title("Test1")
        .initial_position([100.0, 100.0])
        .initial_size([150.0, 100.0]);
    if let (_, Some(win)) = win_ctx.make_window(builder, state.win_ids.test1, ui) {
        let c = widget::Canvas::new()
            .border(0.0)
            .color(conrod_core::color::LIGHT_YELLOW)
            .scroll_kids();
        let (container_id, _) = win.set(c, ui);
        widget::Text::new("Hello World!")
            .color(conrod_core::color::RED)
            .font_size(32)
            .parent(container_id)
            .set(ids.text, ui);
    }
    let mut add_win = 0;
    let builder = WindowBuilder::new()
        .title("Test2")
        .initial_position([150.0, 150.0])
        .initial_size([200.0, 200.0]);
    if let (_, Some(win)) = win_ctx.make_window(builder, state.win_ids.test2, ui) {
        let c = widget::Canvas::new()
            .border(0.0)
            .color(conrod_core::color::LIGHT_BLUE)
            .scroll_kids();
        let (container_id, _) = win.set(c, ui);
        let clicks = widget::Button::new()
            .label("Click me")
            .w_h(100.0, 50.0)
            .middle_of(container_id)
            .parent(container_id)
            .set(ids.button, ui);
        for _ in clicks {
            println!("Clicked me!");
            add_win += 1;
        }
    }
    for (i, &win_id) in state.win_ids.test_array.iter().enumerate() {
        let title = format!("Test multi - {}", i);
        let builder = WindowBuilder::new()
            .title(&title)
            .initial_size([100.0, 100.0]);
        if let (_, Some(win)) = win_ctx.make_window(builder, win_id, ui) {
            let c = widget::Canvas::new()
                .border(0.0)
                .color(conrod_core::color::LIGHT_CHARCOAL)
                .scroll_kids();
            let (_container_id, _) = win.set(c, ui);
        }
    }
    state.array_win_count += add_win;
}
