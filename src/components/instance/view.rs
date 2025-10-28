use gpui::{ParentElement, Render, Styled, div};
use gpui_component::{
    IconName,
    button::{Button, ButtonVariants},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instance {
    id: u64,
    name: String,
}

impl Instance {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }
}

impl Render for Instance {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(div().text_center().child(self.name.clone()))
            .child(
                Button::new("Play")
                    .icon(IconName::ArrowLeft)
                    .primary()
                    .on_click(|_, _, _| println!("Play")),
            )
    }
}
