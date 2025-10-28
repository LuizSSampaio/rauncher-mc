use std::rc::Rc;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Render, Rgba, Styled,
    Window, div, prelude::FluentBuilder, rgb,
};
use gpui_component::{
    Disableable, IconName, StyledExt,
    button::{Button, ButtonVariants},
};

use super::status::InstanceStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModLoader {
    Vanilla,
    Forge,
    Fabric,
    Quilt,
}

impl ModLoader {
    pub fn as_str(&self) -> &str {
        match self {
            ModLoader::Vanilla => "Vanilla",
            ModLoader::Forge => "Forge",
            ModLoader::Fabric => "Fabric",
            ModLoader::Quilt => "Quilt",
        }
    }
}

#[derive(Clone)]
pub struct Instance {
    id: u64,
    name: String,
    version: String,
    modloader: ModLoader,
    last_played: Option<String>,
    play_time: f32,
    icon_path: Option<String>,
    status: InstanceStatus,
    on_play: Option<Rc<dyn Fn()>>,
    on_settings: Option<Rc<dyn Fn()>>,
    on_delete: Option<Rc<dyn Fn()>>,
}

impl Instance {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            version: "0.0.0".to_string(),
            modloader: ModLoader::Vanilla,
            last_played: None,
            play_time: 0.0,
            icon_path: None,
            status: InstanceStatus::default(),
            on_play: None,
            on_settings: None,
            on_delete: None,
        }
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn modloader(mut self, modloader: ModLoader) -> Self {
        self.modloader = modloader;
        self
    }

    pub fn last_played(mut self, last_played: impl Into<String>) -> Self {
        self.last_played = Some(last_played.into());
        self
    }

    pub fn play_time(mut self, hours: f32) -> Self {
        self.play_time = hours;
        self
    }

    pub fn icon_path(mut self, path: impl Into<String>) -> Self {
        self.icon_path = Some(path.into());
        self
    }

    pub fn status(mut self, status: InstanceStatus) -> Self {
        self.status = status;
        self
    }

    pub fn on_play<F>(mut self, callback: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.on_play = Some(Rc::new(callback));
        self
    }

    pub fn on_settings<F>(mut self, callback: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.on_settings = Some(Rc::new(callback));
        self
    }

    pub fn on_delete<F>(mut self, callback: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.on_delete = Some(Rc::new(callback));
        self
    }

    fn render_status_badge(&self) -> AnyElement {
        let color = match self.status {
            InstanceStatus::Ready => Rgba {
                r: 0.13,
                g: 0.77,
                b: 0.37,
                a: 1.0,
            }, // Green
            InstanceStatus::Running => Rgba {
                r: 0.23,
                g: 0.51,
                b: 0.96,
                a: 1.0,
            }, // Blue
            InstanceStatus::Installing => Rgba {
                r: 0.92,
                g: 0.70,
                b: 0.03,
                a: 1.0,
            }, // Yellow
            InstanceStatus::Error => Rgba {
                r: 0.94,
                g: 0.27,
                b: 0.27,
                a: 1.0,
            }, // Red
        };

        div()
            .px_2()
            .py_0p5()
            .rounded_md()
            .text_xs()
            .font_semibold()
            .bg(color)
            .text_color(rgb(0xFFFFFF))
            .child(self.status.label().to_string())
            .into_any_element()
    }

    fn render_metadata(&self) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .text_sm()
            .text_color(rgb(0x999999))
            .child(div().child(format!("Version: {}", self.version)))
            .child(div().child(format!("Modloader: {}", self.modloader.as_str())))
            .when_some(self.last_played.clone(), |this, last_played| {
                this.child(div().child(format!("Last played: {}", last_played)))
            })
            .child(div().child(format!("Play time: {:.1}h", self.play_time)))
            .into_any_element()
    }

    fn render_icon(&self) -> AnyElement {
        div()
            .w_full()
            .h(gpui::px(120.0))
            .rounded_t_lg()
            .bg(rgb(0x33343F))
            .flex()
            .items_center()
            .justify_center()
            .child(div().text_2xl().child("⛏"))
            .into_any_element()
    }

    fn render_actions(&self) -> AnyElement {
        let play_label = match self.status {
            InstanceStatus::Running => "Stop",
            InstanceStatus::Installing => "Installing...",
            _ => "Play",
        };

        let play_icon = match self.status {
            InstanceStatus::Running => IconName::CircleX,
            _ => IconName::ArrowRight,
        };

        div()
            .flex()
            .gap_2()
            .w_full()
            .child({
                let mut button = Button::new(play_label).icon(play_icon).primary().w_full();

                if let Some(callback) = self.on_play.clone() {
                    button = button.on_click(move |_, _, _| callback());
                }

                if self.status == InstanceStatus::Installing {
                    button = button.disabled(true);
                }

                button
            })
            .child({
                let mut button = Button::new("").icon(IconName::Settings);

                if let Some(callback) = self.on_settings.clone() {
                    button = button.on_click(move |_, _, _| callback());
                }

                button
            })
            .child({
                let mut button = Button::new("").icon(IconName::CircleX);

                if let Some(callback) = self.on_delete.clone() {
                    button = button.on_click(move |_, _, _| callback());
                }

                button
            })
            .into_any_element()
    }
}

impl Render for Instance {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(gpui::px(280.0))
            .rounded_lg()
            .border_1()
            .border_color(rgb(0x4D4D59))
            .bg(rgb(0x26262E))
            .overflow_hidden()
            .shadow_lg()
            .hover(|this| this.border_color(rgb(0x666673)).shadow_xl())
            .child(self.render_icon())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(rgb(0xF2F2FA))
                                    .child(self.name.clone()),
                            )
                            .child(self.render_status_badge()),
                    )
                    .child(self.render_metadata())
                    .child(self.render_actions()),
            )
    }
}
