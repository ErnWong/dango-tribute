use bevy::{
    prelude::{AppBuilder, EventReader, Events, Local, Plugin, ResMut},
    window::Windows,
};

pub struct FullViewportPlugin;

impl Plugin for FullViewportPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(Events::<ViewportResized>::default())
            .add_startup_system(setup_viewport_resize_system)
            .add_system(viewport_resize_system);
    }
}

fn get_viewport_size() -> (u32, u32) {
    let window = web_sys::window().expect("could not get window");
    let document_element = window
        .document()
        .expect("could not get document")
        .document_element()
        .expect("could not get document element");

    let width = document_element.client_width();
    let height = document_element.client_height();

    (width as u32, height as u32)
}

pub struct ViewportResized {
    pub width: u32,
    pub height: u32,
}

impl From<(u32, u32)> for ViewportResized {
    fn from(size: (u32, u32)) -> Self {
        ViewportResized {
            width: size.0,
            height: size.1,
        }
    }
}

fn setup_viewport_resize_system(
    mut viewport_resized_events: ResMut<'static, Events<ViewportResized>>,
) {
    let window = web_sys::window().expect("could not get window");

    viewport_resized_events.send(get_viewport_size().into());

    gloo_events::EventListener::new(&window, "resize", move |_event| {
        viewport_resized_events.send(get_viewport_size().into());
    })
    .forget();
}

fn viewport_resize_system(
    mut windows: ResMut<Windows>,
    viewport_resized_events: ResMut<Events<ViewportResized>>,
    mut viewport_resized_event_reader: Local<EventReader<ViewportResized>>,
) {
    for event in viewport_resized_event_reader.iter(&viewport_resized_events) {
        if let Some(window) = windows.get_primary_mut() {
            window.set_resolution(event.width, event.height);
        }
    }
}
