use super::{Gl, WebGL2RenderResourceContext, WebGlFramebuffer};

use crate::{gl_call, Buffer, WebGL2RenderPass};
use bevy::render::{
    pass::{LoadOp, Operations, PassDescriptor, RenderPass, TextureAttachment},
    renderer::{
        BufferId, RenderContext, RenderResourceBinding, RenderResourceBindings,
        RenderResourceContext, TextureId,
    },
    texture::Extent3d,
};
use std::sync::Arc;
use std::unreachable;

pub struct WebGL2RenderContext {
    pub device: Arc<crate::Device>,
    pub render_resource_context: WebGL2RenderResourceContext,
}

impl WebGL2RenderContext {
    pub fn new(device: Arc<crate::Device>, resources: WebGL2RenderResourceContext) -> Self {
        WebGL2RenderContext {
            device,
            render_resource_context: resources,
        }
    }

    /// Consume this context, finalize the current CommandEncoder (if it exists), and take the current WebGL2Resources.
    /// This is intended to be called from a worker thread right before synchronizing with the main thread.
    pub fn finish(&mut self) {}
}

impl RenderContext for WebGL2RenderContext {
    fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: BufferId,
        source_offset: u64,
        destination_buffer: BufferId,
        destination_offset: u64,
        size: u64,
    ) {
        let gl = &self.device.get_context();
        let resources = &self.render_resource_context.resources;
        let buffers = resources.buffers.read();
        let src = buffers.get(&source_buffer).unwrap();
        let dst = buffers.get(&destination_buffer).unwrap();
        match (&src.buffer, &dst.buffer) {
            (Buffer::WebGlBuffer(src_id), Buffer::WebGlBuffer(dst_id)) => {
                gl_call!(gl.bind_buffer(Gl::COPY_READ_BUFFER, Some(&src_id)));
                gl_call!(gl.bind_buffer(Gl::COPY_WRITE_BUFFER, Some(&dst_id)));
                gl_call!(gl.copy_buffer_sub_data_with_i32_and_i32_and_i32(
                    Gl::COPY_READ_BUFFER,
                    Gl::COPY_WRITE_BUFFER,
                    source_offset as i32,
                    destination_offset as i32,
                    size as i32,
                ));
            }
            (Buffer::Data(data), Buffer::WebGlBuffer(dst_id)) => {
                gl_call!(gl.bind_buffer(Gl::COPY_WRITE_BUFFER, Some(dst_id)));
                gl_call!(
                    gl.buffer_sub_data_with_i32_and_u8_array_and_src_offset_and_length(
                        Gl::COPY_WRITE_BUFFER,
                        destination_offset as i32,
                        data,
                        source_offset as u32,
                        size as u32,
                    )
                );
            }
            _ => panic!("copy_buffer_to_buffer: writing to in-memory buffer is not supported"),
        }
    }

    fn copy_buffer_to_texture(
        &mut self,
        source_buffer: BufferId,
        source_offset: u64,
        _source_bytes_per_row: u32,
        destination_texture: TextureId,
        _destination_origin: [u32; 3],
        _destination_mip_level: u32,
        size: Extent3d,
    ) {
        let gl = &self.device.get_context();
        let resources = &self.render_resource_context.resources;
        let textures = resources.textures.read();
        let texture = textures.get(&destination_texture).unwrap();
        let buffers = resources.buffers.read();
        let buffer = buffers.get(&source_buffer).unwrap();

        // TODO
        // let tex_internal_format = match &texture_descriptor.format {
        //     TextureFormat::Rgba8UnormSrgb => Gl::RGBA8_SNORM,
        //     TextureFormat::Rgba8Snorm => Gl::RGBA8_SNORM,
        //     _ => Gl::RGBA,
        // };

        let buffer_id = match &buffer.buffer {
            Buffer::WebGlBuffer(buffer_id) => buffer_id,
            Buffer::Data(_) => panic!("not supported"),
        };

        gl_call!(gl.bind_buffer(Gl::PIXEL_UNPACK_BUFFER, Some(buffer_id)));
        gl_call!(gl.bind_texture(Gl::TEXTURE_2D, Some(&texture)));

        gl_call!(
            gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_f64(
                Gl::TEXTURE_2D,
                0,                       //destination_mip_level as i32,
                Gl::SRGB8_ALPHA8 as i32, // TODO
                size.width as i32,
                size.height as i32,
                0,
                Gl::RGBA,
                Gl::UNSIGNED_BYTE,
                source_offset as f64,
            )
        )
        .expect("tex image");
        gl_call!(gl.generate_mipmap(Gl::TEXTURE_2D));

        // PATCHED: For our specific use case.
        gl_call!(gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as i32));
        gl_call!(gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32));
        gl_call!(gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::REPEAT as i32));
        gl_call!(gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::REPEAT as i32));
        // gl_call!(gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::NEAREST as i32));
        // gl_call!(gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::NEAREST as i32));
        // gl_call!(gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32));
        // gl_call!(gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32));

        // gl_call!(gl.tex_parameteri(
        //     Gl::TEXTURE_2D,
        //     Gl::TEXTURE_MAG_FILTER,
        //     Gl::NEAREST as i32,
        // ));

        // gl_call!(gl.tex_parameteri(
        //     Gl::TEXTURE_2D,
        //     Gl::TEXTURE_MIN_FILTER,
        //     Gl::NEAREST as i32,
        // ));
    }

    fn resources(&self) -> &dyn RenderResourceContext {
        &self.render_resource_context
    }

    fn resources_mut(&mut self) -> &mut dyn RenderResourceContext {
        &mut self.render_resource_context
    }

    fn begin_pass(
        &mut self,
        pass_descriptor: &PassDescriptor,
        render_resource_bindings: &RenderResourceBindings,
        run_pass: &mut dyn Fn(&mut dyn RenderPass),
    ) {
        let mut clear_mask = 0;
        let gl = &self.device.get_context();

        match &pass_descriptor.color_attachments[0].attachment {
            TextureAttachment::Id(texture_id)
                if *texture_id == self.render_resource_context.swap_chain_texture_id.get() =>
            {
                // Normal render to canvas, aka swap chain.
                // Unbind any framebuffer to use the original canvas framebuffer.
                // gl_call!(gl.bind_framebuffer(Gl::FRAMEBUFFER, None));
                gl_call!(gl.bind_framebuffer(Gl::FRAMEBUFFER, Option::<&WebGlFramebuffer>::None));
            }
            TextureAttachment::Input(_) =>
                panic!("Any texture attachment that is mapped using the input slot name should have been replaced with the correct texture attachment by the graph executor"),
            non_swap_chain_attachment => {
                let textures = self.render_resource_context.resources.textures.read();
                let texture_descriptors = self.render_resource_context.resources.texture_descriptors.read();
                let (texture, texture_descriptor) = match non_swap_chain_attachment {
                    TextureAttachment::Id(texture_id) => (
                        textures.get(&texture_id).unwrap(),
                        texture_descriptors.get(&texture_id).unwrap(),
                    ),
                    TextureAttachment::Name(name) => {
                        let texture_id = match render_resource_bindings.get(&name).unwrap() {
                            RenderResourceBinding::Texture(texture_id) => texture_id,
                            _ => panic!("attachment {} does not exist", name),
                        };
                        (
                            textures.get(&texture_id).unwrap(),
                            texture_descriptors.get(&texture_id).unwrap(),
                        )
                    }
                    _ => unreachable!(),
                };
                // TODO: This should only be set depending on the sampler descriptor.
                gl_call!(gl.bind_texture(Gl::TEXTURE_2D, Some(texture)));
                gl_call!(gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as i32));

                let framebuffer = gl_call!(gl.create_framebuffer()).unwrap();
                gl_call!(gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&framebuffer)));
                gl_call!(gl.viewport(
                    0,
                    0,
                    texture_descriptor.size.width as i32,
                    texture_descriptor.size.height as i32,
                ));
                gl_call!(gl.framebuffer_texture_2d(
                    Gl::FRAMEBUFFER,
                    Gl::COLOR_ATTACHMENT0,
                    Gl::TEXTURE_2D,
                    Some(texture),
                    0,
                ));
                let framebuffer_status = gl_call!(gl.check_framebuffer_status(Gl::FRAMEBUFFER));
                assert!(framebuffer_status== Gl::FRAMEBUFFER_COMPLETE, "Framebuffer is not complete yet: {}", framebuffer_status);
            }
        }

        if let LoadOp::Clear(c) = pass_descriptor.color_attachments[0].ops.load {
            gl_call!(gl.clear_color(c.r(), c.g(), c.b(), c.a()));
            clear_mask |= Gl::COLOR_BUFFER_BIT;
        }
        if let Some(d) = &pass_descriptor.depth_stencil_attachment {
            if let Some(Operations {
                load: LoadOp::Clear(_),
                ..
            }) = d.depth_ops
            {
                clear_mask |= Gl::DEPTH_BUFFER_BIT;
            }
        }
        if clear_mask > 0 {
            gl_call!(gl.clear(clear_mask));
        }
        let mut render_pass = WebGL2RenderPass {
            render_context: self,
            pipeline_descriptor: None,
            pipeline: None,
        };
        run_pass(&mut render_pass);
    }
}
