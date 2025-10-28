use gpui::{
    AnyView, Context, IntoElement, ParentElement, Render, Styled, Window, div, prelude::FluentBuilder,
    rgb,
};

pub struct InstanceGrid {
    instances: Vec<AnyView>,
}

impl InstanceGrid {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    pub fn add_instance(mut self, instance: AnyView) -> Self {
        self.instances.push(instance);
        self
    }

    pub fn instances(mut self, instances: Vec<AnyView>) -> Self {
        self.instances = instances;
        self
    }
}

impl Default for InstanceGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for InstanceGrid {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let has_instances = !self.instances.is_empty();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .bg(rgb(0x1A1A1F))
            .when(!has_instances, |this| {
                this.items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_xl()
                            .text_color(rgb(0x888888))
                            .child("No instances yet")
                    )
            })
            .when(has_instances, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_4()
                        .children(self.instances.clone())
                )
            })
    }
}
