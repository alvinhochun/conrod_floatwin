pub mod windowing_area;

mod classic_button;
mod classic_frame;
mod empty_widget;
mod util;

pub use windowing_area::{
    layout::{WinId, WindowingState},
    WindowBuilder, WindowEvent, WindowSetter, WindowingArea, WindowingContext,
};
