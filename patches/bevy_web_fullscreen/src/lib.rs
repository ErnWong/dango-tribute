use bevy::{prelude::*, window::Windows};
use gloo_events::EventListener;
use std::{cell::RefCell, rc::Rc};

pub struct FullViewportPlugin;

impl Plugin for FullViewportPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_thread_local_resource(ViewportResizedEvents::default())
            .add_thread_local_resource(LocalReader::default())
            .add_system(viewport_resize_system.system());
    }
}

fn get_viewport_size() -> (f32, f32) {
    let window = web_sys::window().expect("could not get window");
    let document_element = window
        .document()
        .expect("could not get document")
        .document_element()
        .expect("could not get document element");

    let width = document_element.client_width();
    let height = document_element.client_height();

    (width as f32, height as f32)
}

pub struct ViewportResized {
    pub width: f32,
    pub height: f32,
}

pub struct ViewportResizedEvents {
    events: Rc<RefCell<Events<ViewportResized>>>,
    event_listener: EventListener,
}

#[derive(Default)]
pub struct LocalReader {
    pub event_reader: EventReader<ViewportResized>,
}

impl Default for ViewportResizedEvents {
    fn default() -> Self {
        let window = web_sys::window().expect("could not get window");
        let events: Rc<RefCell<Events<ViewportResized>>> = Rc::new(Default::default());
        events.borrow_mut().send(get_viewport_size().into());
        let events_cloned = events.clone();
        let event_listener = EventListener::new(&window, "resize", move |_event| {
            events_cloned.borrow_mut().send(get_viewport_size().into());
        });
        Self {
            events,
            event_listener,
        }
    }
}

fn viewport_resize_system(_world: &mut World, resources: &mut Resources) {
    let viewport_resized_events = resources
        .get_thread_local::<ViewportResizedEvents>()
        .unwrap();
    let mut local_reader = resources.get_thread_local_mut::<LocalReader>().unwrap();
    let mut windows = resources.get_mut::<Windows>().unwrap();
    let events = viewport_resized_events.events.borrow();
    for event in local_reader.event_reader.iter(&events) {
        if let Some(window) = windows.get_primary_mut() {
            window.set_resolution(event.width, event.height);
        }
    }
}
