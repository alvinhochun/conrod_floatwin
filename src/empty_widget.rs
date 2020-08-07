use conrod_core::{widget, Widget, WidgetCommon};

#[derive(Clone, Copy, Debug, WidgetCommon)]
pub struct EmptyWidget {
    #[conrod(common_builder)]
    pub common: widget::CommonBuilder,
}

impl EmptyWidget {
    pub fn new() -> Self {
        EmptyWidget {
            common: widget::CommonBuilder::default(),
        }
    }
}

impl Widget for EmptyWidget {
    type State = ();
    type Style = ();
    type Event = ();

    fn init_state(&self, _: conrod_core::widget::id::Generator) -> Self::State {}

    fn style(&self) -> Self::Style {}

    fn update(self, _: conrod_core::widget::UpdateArgs<Self>) -> Self::Event {}
}
