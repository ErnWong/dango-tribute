use bevy::{
    app::prelude::{EventReader, Events},
    ecs::{Resources, World},
    render::{
        render_graph::{Node, ResourceSlotInfo, ResourceSlots},
        renderer::{BufferInfo, BufferUsage, RenderContext, RenderResourceId, RenderResourceType},
        texture::TextureDescriptor,
    },
    window::{WindowCreated, WindowId, WindowResized, Windows},
};
use rand::RngCore;
use std::borrow::Cow;

// TODO: Remove code duplication from WindowTextureNode.
pub struct WindowRandomTextureNode {
    window_id: WindowId,
    descriptor: TextureDescriptor,
    window_created_event_reader: EventReader<WindowCreated>,
    window_resized_event_reader: EventReader<WindowResized>,
}

impl WindowRandomTextureNode {
    pub const OUT_TEXTURE: &'static str = "texture";

    pub fn new(window_id: WindowId, descriptor: TextureDescriptor) -> Self {
        WindowRandomTextureNode {
            window_id,
            descriptor,
            window_created_event_reader: Default::default(),
            window_resized_event_reader: Default::default(),
        }
    }
}

impl Node for WindowRandomTextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(WindowRandomTextureNode::OUT_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        const WINDOW_TEXTURE: usize = 0;
        let window_created_events = resources.get::<Events<WindowCreated>>().unwrap();
        let window_resized_events = resources.get::<Events<WindowResized>>().unwrap();
        let windows = resources.get::<Windows>().unwrap();

        let window = windows
            .get(self.window_id)
            .expect("Received window resized event for non-existent window.");

        if self
            .window_created_event_reader
            .find_latest(&window_created_events, |e| e.id == window.id())
            .is_some()
            || self
                .window_resized_event_reader
                .find_latest(&window_resized_events, |e| e.id == window.id())
                .is_some()
        {
            let render_resource_context = render_context.resources_mut();
            if let Some(RenderResourceId::Texture(old_texture)) = output.get(WINDOW_TEXTURE) {
                render_resource_context.remove_texture(old_texture);
            }

            self.descriptor.size.width = window.physical_width();
            self.descriptor.size.height = window.physical_height();
            let texture_resource = render_resource_context.create_texture(self.descriptor);
            let format_size = self.descriptor.format.pixel_size();
            let aligned_width = render_resource_context
                .get_aligned_texture_size(self.descriptor.size.width as usize);
            let mut data = vec![
                0;
                format_size
                    * aligned_width
                    * self.descriptor.size.height as usize
                    * self.descriptor.size.depth as usize
            ];
            rand::thread_rng().fill_bytes(&mut data);
            let random_buffer = render_resource_context.create_buffer_with_data(
                BufferInfo {
                    buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                    mapped_at_creation: true,
                    ..Default::default()
                },
                &data,
            );
            render_context.copy_buffer_to_texture(
                random_buffer,
                0,
                (format_size * aligned_width) as u32,
                texture_resource,
                [0, 0, 0],
                0,
                self.descriptor.size,
            );
            render_context.resources().remove_buffer(random_buffer);
            output.set(WINDOW_TEXTURE, RenderResourceId::Texture(texture_resource));
        }
    }
}
