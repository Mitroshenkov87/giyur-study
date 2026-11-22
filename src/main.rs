#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn view(_: &()) -> iced::Element<'_, ()> {
    iced::widget::text("Giyur Study").into()
}
fn main() -> iced::Result {
    iced::application(|| (), |_: &mut (), _: ()| {}, view).run()
}
