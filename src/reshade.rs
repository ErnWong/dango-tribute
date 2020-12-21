use bevy::{
    prelude::*,
    render::{
        pass::{
            LoadOp, Operations, PassDescriptor, RenderPassColorAttachmentDescriptor,
            TextureAttachment,
        },
        pipeline::{PipelineCompiler, PipelineDescriptor},
        render_graph::{
            base::node, Node, RenderGraph, ResourceSlotInfo, ResourceSlots, SystemNode,
            WindowSwapChainNode, WindowTextureNode,
        },
        renderer::{
            BindGroup, RenderContext, RenderResourceBindings, RenderResourceContext,
            RenderResourceType, SamplerId, SharedBuffers,
        },
        shader::{Shader, ShaderStage, ShaderStages},
        texture::{
            AddressMode, FilterMode, SamplerDescriptor, TextureDescriptor, TextureDimension,
            TextureFormat, TextureUsage,
        },
    },
    window::WindowId,
};
use std::borrow::Cow;

use super::window_random_texture_node::WindowRandomTextureNode;

pub struct ReshadePlugin;

impl Plugin for ReshadePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_reshade.system());
    }
}

fn setup_pipeline(
    mut shaders: ResMut<Assets<Shader>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut pipeline_compiler: ResMut<PipelineCompiler>,
    render_resource_context: &dyn RenderResourceContext,
) -> Handle<PipelineDescriptor> {
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(
            ShaderStage::Vertex,
            include_str!("reshade.vert"),
        )),
        fragment: Some(shaders.add(Shader::from_glsl(
            ShaderStage::Fragment,
            include_str!("reshade.frag"),
        ))),
    }));
    pipelines
        .get_mut(&pipeline_handle)
        .unwrap()
        .depth_stencil_state = None;

    let pipeline_specialization = Default::default();

    pipeline_compiler.compile_pipeline(
        render_resource_context,
        &mut pipelines,
        &mut shaders,
        &pipeline_handle,
        &pipeline_specialization,
    );

    pipeline_compiler
        .get_specialized_pipeline(&pipeline_handle, &pipeline_specialization)
        .unwrap()
}

fn inject_into_render_graph(
    mut render_graph: ResMut<RenderGraph>,
    pipeline_handle: Handle<PipelineDescriptor>,
    render_resource_context: &dyn RenderResourceContext,
) {
    render_graph.add_node(
        "reshade_source_texture",
        WindowTextureNode::new(
            WindowId::primary(),
            TextureDescriptor {
                dimension: TextureDimension::D2,
                format: TextureFormat::default(),
                usage: TextureUsage::OUTPUT_ATTACHMENT | TextureUsage::SAMPLED,
                ..Default::default()
            },
        ),
    );
    render_graph.add_node(
        "reshade_random_texture",
        WindowRandomTextureNode::new(
            WindowId::primary(),
            TextureDescriptor {
                dimension: TextureDimension::D2,
                format: TextureFormat::default(),
                usage: TextureUsage::COPY_DST | TextureUsage::SAMPLED,
                ..Default::default()
            },
        ),
    );
    render_graph.add_system_node(
        "reshade",
        ReshadeNode::new(pipeline_handle, render_resource_context),
    );
    render_graph
        .add_slot_edge(
            "reshade_source_texture",
            WindowTextureNode::OUT_TEXTURE,
            node::MAIN_PASS,
            "color_attachment", // TODO: msaa, color_resolve_target
        )
        .unwrap();
    render_graph
        .add_slot_edge(
            "reshade_source_texture",
            WindowTextureNode::OUT_TEXTURE,
            "reshade",
            "source_texture",
        )
        .unwrap();
    render_graph
        .add_slot_edge(
            "reshade_random_texture",
            WindowTextureNode::OUT_TEXTURE,
            "reshade",
            "random_texture",
        )
        .unwrap();
    render_graph
        .add_slot_edge(
            node::PRIMARY_SWAP_CHAIN,
            WindowSwapChainNode::OUT_TEXTURE,
            "reshade",
            "color_attachment", // TODO: msaa, color_resolve_target
        )
        .unwrap();
    render_graph
        .add_node_edge(node::MAIN_PASS, "reshade")
        .unwrap();
}

fn setup_reshade(
    // asset_server: ResMut<AssetServer>,
    shaders: ResMut<Assets<Shader>>,
    pipelines: ResMut<Assets<PipelineDescriptor>>,
    render_graph: ResMut<RenderGraph>,
    pipeline_compiler: ResMut<PipelineCompiler>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
) {
    let pipeline_handle = setup_pipeline(
        shaders,
        pipelines,
        pipeline_compiler,
        &**render_resource_context,
    );
    inject_into_render_graph(render_graph, pipeline_handle, &**render_resource_context);
}

pub struct ReshadeNode {
    pipeline_handle: Handle<PipelineDescriptor>,
    sampler: SamplerId,
    pass_descriptor: PassDescriptor,
}

impl ReshadeNode {
    // TODO: remove hardcoded indices (should match input() return value).
    pub const IN_SOURCE_COLOR_TEXTURE: &'static str = "source_texture";
    pub const IN_SOURCE_COLOR_TEXTURE_INDEX: usize = 0;
    pub const IN_SOURCE_RANDOM_TEXTURE: &'static str = "random_texture";
    pub const IN_SOURCE_RANDOM_TEXTURE_INDEX: usize = 1;
    pub const IN_TARGET_COLOR_TEXTURE: &'static str = "color_attachment";
    pub const IN_TARGET_COLOR_TEXTURE_INDEX: usize = 2;

    pub fn new(
        pipeline_handle: Handle<PipelineDescriptor>,
        render_resource_context: &dyn RenderResourceContext,
    ) -> ReshadeNode {
        ReshadeNode {
            pipeline_handle,
            sampler: render_resource_context.create_sampler(&SamplerDescriptor {
                // Allow random texture to repeat past the boundary.
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                address_mode_w: AddressMode::Repeat,
                // Allow smoothness in random texture.
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Linear,
                ..Default::default()
            }),
            pass_descriptor: PassDescriptor {
                color_attachments: vec![RenderPassColorAttachmentDescriptor {
                    attachment: TextureAttachment::Input("color_attachment".to_string()),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
                sample_count: 1,
            },
        }
    }
}

impl SystemNode for ReshadeNode {
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System<In = (), Out = ()>> {
        let system = reshade_node_system.system();
        commands.insert_resource(InfoBindGroup {
            bind_group: BindGroup::build().finish(),
        });
        commands.insert_local_resource(
            system.id(),
            ReshadeNodeState {
                sampler: Some(self.sampler),
            },
        );
        Box::new(system)
    }
}

#[derive(Debug, Default)]
struct ReshadeNodeState {
    sampler: Option<SamplerId>,
}

struct InfoBindGroup {
    bind_group: BindGroup,
}

fn reshade_node_system(
    state: Local<ReshadeNodeState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut shared_buffers: ResMut<SharedBuffers>,
    mut info_bind_group: ResMut<InfoBindGroup>,
    time: Res<Time>,
    mouse_button: Res<Input<MouseButton>>,
    windows: Res<Windows>,
) {
    let window = windows.get_primary().unwrap();
    let resolution = Vec2::new(
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    let resolution_buffer = shared_buffers
        .get_uniform_buffer(&**render_resource_context, &resolution)
        .unwrap();
    let time_delta_buffer = shared_buffers
        .get_uniform_buffer(&**render_resource_context, &time.delta_seconds())
        .unwrap();
    let time_buffer = shared_buffers
        .get_uniform_buffer(
            &**render_resource_context,
            &(time.seconds_since_startup() as f32),
        )
        .unwrap();
    let (cursor_x, cursor_y) = window.cursor_position().unwrap_or(Vec2::zero()).into();
    let mouse = Vec4::new(
        cursor_x,
        cursor_y,
        mouse_button.pressed(MouseButton::Left) as i32 as f32,
        mouse_button.pressed(MouseButton::Right) as i32 as f32,
    );
    let mouse_buffer = shared_buffers
        .get_uniform_buffer(&**render_resource_context, &mouse)
        .unwrap();
    info_bind_group.bind_group = BindGroup::build()
        .add_sampler(0, state.sampler.unwrap())
        .add_binding(1, resolution_buffer)
        .add_binding(2, time_buffer)
        .add_binding(3, time_delta_buffer)
        .add_binding(4, mouse_buffer)
        .finish();
}

impl Node for ReshadeNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        static INPUT: &[ResourceSlotInfo] = &[
            ResourceSlotInfo {
                name: Cow::Borrowed(ReshadeNode::IN_SOURCE_COLOR_TEXTURE),
                resource_type: RenderResourceType::Texture,
            },
            ResourceSlotInfo {
                name: Cow::Borrowed(ReshadeNode::IN_SOURCE_RANDOM_TEXTURE),
                resource_type: RenderResourceType::Texture,
            },
            ResourceSlotInfo {
                name: Cow::Borrowed(ReshadeNode::IN_TARGET_COLOR_TEXTURE),
                resource_type: RenderResourceType::Texture,
            },
        ];
        INPUT
    }

    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let source_texture = input
            .get(ReshadeNode::IN_SOURCE_COLOR_TEXTURE_INDEX)
            .unwrap()
            .get_texture()
            .unwrap();
        let random_texture = input
            .get(ReshadeNode::IN_SOURCE_RANDOM_TEXTURE_INDEX)
            .unwrap()
            .get_texture()
            .unwrap();

        let pipeline_descriptors = resources.get::<Assets<PipelineDescriptor>>().unwrap();
        let pipeline_descriptor = pipeline_descriptors.get(&self.pipeline_handle).unwrap();
        let layout = pipeline_descriptor.get_layout().unwrap();

        // TODO: enumify this
        const IMAGE_BIND_GROUP_INDEX: u32 = 0;
        const INFO_BIND_GROUP_INDEX: u32 = 1;

        let image_bind_group_descriptor = layout.get_bind_group(IMAGE_BIND_GROUP_INDEX).unwrap();
        let image_bind_group = BindGroup::build()
            .add_texture(0, source_texture)
            .add_texture(1, random_texture)
            .finish();
        render_context
            .resources()
            .create_bind_group(image_bind_group_descriptor.id, &image_bind_group);

        let info_bind_group_descriptor = layout.get_bind_group(INFO_BIND_GROUP_INDEX).unwrap();
        let info_bind_group = &resources.get::<InfoBindGroup>().unwrap().bind_group;
        render_context
            .resources()
            .create_bind_group(info_bind_group_descriptor.id, info_bind_group);

        // Update pass descriptor to reflect current texture Ids from the input slots.
        self.pass_descriptor.color_attachments[0].attachment = TextureAttachment::Id(
            input
                .get(ReshadeNode::IN_TARGET_COLOR_TEXTURE_INDEX)
                .unwrap()
                .get_texture()
                .unwrap(),
        );

        let render_resource_bindings = resources.get::<RenderResourceBindings>().unwrap();

        render_context.begin_pass(
            &self.pass_descriptor,
            &render_resource_bindings,
            &mut |render_pass| {
                render_pass.set_pipeline(&self.pipeline_handle);
                render_pass.set_bind_group(
                    IMAGE_BIND_GROUP_INDEX,
                    image_bind_group_descriptor.id,
                    image_bind_group.id,
                    None,
                );
                render_pass.set_bind_group(
                    INFO_BIND_GROUP_INDEX,
                    info_bind_group_descriptor.id,
                    info_bind_group.id,
                    None,
                );
                render_pass.draw(0..6, 0..1);
            },
        );
    }
}
