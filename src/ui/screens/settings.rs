//! Screen stub — replaced by the screen implementation.
use iced::Element;

use crate::ui::app::{Action, Ctx, Route};

pub struct Screen;

#[derive(Debug, Clone)]
pub enum Msg {}

impl Screen {
    pub fn new(_ctx: &mut Ctx, _route: &Route) -> Self {
        Screen
    }
    pub fn update(&mut self, _ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {}
    }
    pub fn view<'a>(&'a self, _ctx: &'a Ctx) -> Element<'a, Msg> {
        crate::ui::widgets::heading("settings", 1)
    }
    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }
}
