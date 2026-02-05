// adapted from
// https://github.com/MirrorX-Desktop/MirrorX/tree/master/mirrorx_core/src/component/desktop/windows
use super::{
    dx_math::{VERTEX_COUNT, VERTEX_STRIDE, VERTICES},
    shader,
};

use crate::capture::{NV12FrameRef, YUVFrame};

use std::os::raw::c_void;

use std::sync::Arc;
use windows::{
    Win32::Graphics::{Direct3D::*, Direct3D11::*, Dxgi::Common::*},
    core::PCSTR,
};

#[derive(Clone)]
pub struct YuvConverter {
    device: Arc<ID3D11Device>,
    device_context: Arc<ID3D11DeviceContext>,

    vertex_shader: ID3D11VertexShader,
    vertex_buffer: ID3D11Buffer,

    pixel_shader_luminance: ID3D11PixelShader,
    pixel_shader_chrominance: ID3D11PixelShader,

    backend_texture: ID3D11Texture2D,

    luminance_render_texture: ID3D11Texture2D,
    luminance_staging_texture: ID3D11Texture2D,
    luminance_viewport: [D3D11_VIEWPORT; 1],
    luminance_rtv: [Option<ID3D11RenderTargetView>; 1],

    chrominance_render_texture: ID3D11Texture2D,
    chrominance_staging_texture: ID3D11Texture2D,
    chrominance_viewport: [D3D11_VIEWPORT; 1],
    chrominance_rtv: [Option<ID3D11RenderTargetView>; 1],

    resolution: (u32, u32),
}

unsafe impl Send for YuvConverter {}

impl YuvConverter {
    pub fn new(
        device: Arc<ID3D11Device>,
        device_context: Arc<ID3D11DeviceContext>,
        resolution: (u32, u32),
    ) -> Result<YuvConverter, anyhow::Error> {
        unsafe {
            let backend_texture = init_backend_resources(&device, resolution)?;

            let (vertex_shader, vertex_buffer, pixel_shader_luminance, pixel_shader_chrominance) =
                init_shaders(&device)?;

            let (
                luminance_render_texture,
                luminance_staging_texture,
                luminance_viewport,
                luminance_rtv,
            ) = init_luminance_resources(&device, resolution)?;

            let (
                chrominance_render_texture,
                chrominance_staging_texture,
                chrominance_viewport,
                chrominance_rtv,
            ) = init_chrominance_resources(&device, resolution)?;

            let sampler_state = init_sampler_state(&device)?;

            device_context.PSSetSamplers(0, Some(&[Some(sampler_state)]));

            device_context.IASetInputLayout(&init_input_layout(&device)?);

            Ok(YuvConverter {
                device,
                device_context,
                vertex_shader,
                vertex_buffer,
                pixel_shader_luminance,
                pixel_shader_chrominance,
                backend_texture,
                luminance_render_texture,
                luminance_staging_texture,
                luminance_viewport: [luminance_viewport],
                luminance_rtv: [Some(luminance_rtv)],
                chrominance_render_texture,
                chrominance_staging_texture,
                chrominance_viewport: [chrominance_viewport],
                chrominance_rtv: [Some(chrominance_rtv)],
                resolution,
            })
        }
    }

    pub fn capture(&mut self, desktop_texture: ID3D11Texture2D) -> Result<YUVFrame, anyhow::Error> {
        let (w, h) = self.resolution;
        self.capture_with_nv12_view(desktop_texture, |nv12| {
            Ok(YUVFrame {
                display_time: 0,
                width: w as i32,
                height: h as i32,
                luminance_bytes: nv12.luminance_bytes.to_vec(),
                luminance_stride: nv12.luminance_stride,
                chrominance_bytes: nv12.chrominance_bytes.to_vec(),
                chrominance_stride: nv12.chrominance_stride,
            })
        })
    }

    pub fn capture_with_nv12_view<T, F>(
        &mut self,
        desktop_texture: ID3D11Texture2D,
        f: F,
    ) -> Result<T, anyhow::Error>
    where
        F: FnOnce(NV12FrameRef<'_>) -> Result<T, anyhow::Error>,
    {
        unsafe {
            self.device_context
                .CopyResource(&self.backend_texture, &desktop_texture);
            self.draw_lumina_and_chrominance_texture()?;
            self.device_context.CopyResource(
                &self.luminance_staging_texture,
                &self.luminance_render_texture,
            );
            self.device_context.CopyResource(
                &self.chrominance_staging_texture,
                &self.chrominance_render_texture,
            );

            let mut lumina_mapped_resource = std::mem::zeroed();
            self.device_context.Map(
                &self.luminance_staging_texture,
                0,
                D3D11_MAP_READ,
                0,
                Some(&mut lumina_mapped_resource),
            )?;

            let mut chrominance_mapped_resource = std::mem::zeroed();
            self.device_context.Map(
                &self.chrominance_staging_texture,
                0,
                D3D11_MAP_READ,
                0,
                Some(&mut chrominance_mapped_resource),
            )?;

            let luminance_stride = lumina_mapped_resource.RowPitch as usize;
            let chrominance_stride = chrominance_mapped_resource.RowPitch as usize;
            let h = self.resolution.1;

            let lum_slice = std::slice::from_raw_parts(
                lumina_mapped_resource.pData as *const u8,
                (h as usize) * luminance_stride,
            );
            let chr_slice = std::slice::from_raw_parts(
                chrominance_mapped_resource.pData as *const u8,
                (h as usize / 2) * chrominance_stride,
            );

            let view = NV12FrameRef {
                luminance_bytes: lum_slice,
                luminance_stride: luminance_stride as i32,
                chrominance_bytes: chr_slice,
                chrominance_stride: chrominance_stride as i32,
            };

            let result = f(view);

            self.device_context.Unmap(&self.chrominance_staging_texture, 0);
            self.device_context.Unmap(&self.luminance_staging_texture, 0);

            result
        }
    }

    unsafe fn draw_lumina_and_chrominance_texture(&self) -> Result<(), anyhow::Error> {
        unsafe {
            let mut backend_texture_desc = std::mem::zeroed();
            self.backend_texture.GetDesc(&mut backend_texture_desc);

            let shader_resource_view_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: backend_texture_desc.Format,
                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: backend_texture_desc.MipLevels - 1,
                        MipLevels: backend_texture_desc.MipLevels,
                    },
                },
            };

            let mut shader_resource_view = None;
            self.device.CreateShaderResourceView(
                &self.backend_texture,
                Some(&shader_resource_view_desc),
                Some(&mut shader_resource_view),
            )?;

            let shader_resource_view = [shader_resource_view];

            self.device_context
                .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

            let vertex_buffers = [Some(self.vertex_buffer.clone())];
            let strides = [VERTEX_STRIDE];
            let offsets = [0u32];
            self.device_context.IASetVertexBuffers(
                0,
                1,
                Some(vertex_buffers.as_ptr()),
                Some(strides.as_ptr()),
                Some(offsets.as_ptr()),
            );

            self.device_context.VSSetShader(&self.vertex_shader, None);

            // draw lumina plane

            self.device_context
                .OMSetRenderTargets(Some(&self.luminance_rtv), None);

            self.device_context
                .PSSetShaderResources(0, Some(&shader_resource_view));

            self.device_context
                .PSSetShader(&self.pixel_shader_luminance, None);

            self.device_context
                .RSSetViewports(Some(&self.luminance_viewport));

            self.device_context.Draw(VERTEX_COUNT, 0);

            // draw chrominance plane

            self.device_context
                .OMSetRenderTargets(Some(&self.chrominance_rtv), None);

            self.device_context
                .PSSetShaderResources(0, Some(&shader_resource_view));

            self.device_context
                .PSSetShader(&self.pixel_shader_chrominance, None);

            self.device_context
                .RSSetViewports(Some(&self.chrominance_viewport));

            self.device_context.Draw(VERTEX_COUNT, 0);

            Ok(())
        }
    }

}

unsafe fn init_shaders(
    device: &ID3D11Device,
) -> Result<
    (
        ID3D11VertexShader,
        ID3D11Buffer,
        ID3D11PixelShader,
        ID3D11PixelShader,
    ),
    anyhow::Error,
> {
    unsafe {
        let mut vertex_shader = None;
        device.CreateVertexShader(shader::VERTEX_SHADER_BYTES, None, Some(&mut vertex_shader))?;

        let vertex_buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: VERTEX_STRIDE * VERTEX_COUNT,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_FLAG::default().0 as u32,
            MiscFlags: D3D11_RESOURCE_MISC_FLAG::default().0 as u32,
            StructureByteStride: 0,
        };

        let subresource_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: &VERTICES as *const _ as *const c_void,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        let mut vertex_buffer = None;
        device.CreateBuffer(
            &vertex_buffer_desc,
            Some(&subresource_data),
            Some(&mut vertex_buffer),
        )?;

        let mut pixel_shader_luminance = None;
        device.CreatePixelShader(
            shader::PIXEL_SHADER_LUMINANCE_BYTES,
            None,
            Some(&mut pixel_shader_luminance),
        )?;

        let mut pixel_shader_chrominance = None;
        device.CreatePixelShader(
            shader::PIXEL_SHADER_CHROMINANCE_BYTES,
            None,
            Some(&mut pixel_shader_chrominance),
        )?;

        Ok((
            vertex_shader.unwrap(),
            vertex_buffer.unwrap(),
            pixel_shader_luminance.unwrap(),
            pixel_shader_chrominance.unwrap(),
        ))
    }
}

unsafe fn init_luminance_resources(
    device: &ID3D11Device,
    resolution: (u32, u32),
) -> Result<
    (
        ID3D11Texture2D,
        ID3D11Texture2D,
        D3D11_VIEWPORT,
        ID3D11RenderTargetView,
    ),
    anyhow::Error,
> {
    unsafe { init_plane_resources(device, resolution.0, resolution.1, DXGI_FORMAT_R8_UNORM) }
}

unsafe fn init_chrominance_resources(
    device: &ID3D11Device,
    resolution: (u32, u32),
) -> Result<
    (
        ID3D11Texture2D,
        ID3D11Texture2D,
        D3D11_VIEWPORT,
        ID3D11RenderTargetView,
    ),
    anyhow::Error,
> {
    unsafe {
        init_plane_resources(
            device,
            resolution.0 / 2,
            resolution.1 / 2,
            DXGI_FORMAT_R8G8_UNORM,
        )
    }
}

unsafe fn init_backend_resources(
    device: &ID3D11Device,
    resolution: (u32, u32),
) -> Result<ID3D11Texture2D, anyhow::Error> {
    unsafe {
        let mut texture_desc: D3D11_TEXTURE2D_DESC = std::mem::zeroed();
        texture_desc.Width = resolution.0;
        texture_desc.Height = resolution.1;
        texture_desc.MipLevels = 1;
        texture_desc.ArraySize = 1;
        texture_desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
        texture_desc.SampleDesc.Count = 1;
        texture_desc.SampleDesc.Quality = 0;
        texture_desc.Usage = D3D11_USAGE_DEFAULT;
        texture_desc.BindFlags = (D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE).0 as u32;

        let mut texture = None;
        device.CreateTexture2D(&texture_desc, None, Some(&mut texture))?;
        Ok(texture.unwrap())
    }
}

unsafe fn init_plane_resources(
    device: &ID3D11Device,
    width: u32,
    height: u32,
    format: DXGI_FORMAT,
) -> Result<
    (
        ID3D11Texture2D,
        ID3D11Texture2D,
        D3D11_VIEWPORT,
        ID3D11RenderTargetView,
    ),
    anyhow::Error,
> {
    unsafe {
        let mut texture_desc: D3D11_TEXTURE2D_DESC = std::mem::zeroed();
        texture_desc.Width = width;
        texture_desc.Height = height;
        texture_desc.MipLevels = 1;
        texture_desc.ArraySize = 1;
        texture_desc.Format = format;
        texture_desc.SampleDesc.Count = 1;
        texture_desc.SampleDesc.Quality = 0;
        texture_desc.Usage = D3D11_USAGE_DEFAULT;
        texture_desc.BindFlags = D3D11_BIND_RENDER_TARGET.0 as u32;

        let mut render_texture = None;
        device.CreateTexture2D(&texture_desc, None, Some(&mut render_texture))?;
        let render_texture = render_texture.unwrap();

        texture_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        texture_desc.Usage = D3D11_USAGE_STAGING;
        texture_desc.BindFlags = D3D11_BIND_FLAG::default().0 as u32;

        let mut staging_texture = None;
        device.CreateTexture2D(&texture_desc, None, Some(&mut staging_texture))?;

        let viewport = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: width as f32,
            Height: height as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };

        let mut rtv = None;
        device.CreateRenderTargetView(&render_texture, None, Some(&mut rtv))?;

        Ok((
            render_texture,
            staging_texture.unwrap(),
            viewport,
            rtv.unwrap(),
        ))
    }
}

unsafe fn init_sampler_state(device: &ID3D11Device) -> Result<ID3D11SamplerState, anyhow::Error> {
    unsafe {
        let mut sampler_desc: D3D11_SAMPLER_DESC = std::mem::zeroed();
        sampler_desc.Filter = D3D11_FILTER_MIN_MAG_MIP_LINEAR;
        sampler_desc.AddressU = D3D11_TEXTURE_ADDRESS_CLAMP;
        sampler_desc.AddressV = D3D11_TEXTURE_ADDRESS_CLAMP;
        sampler_desc.AddressW = D3D11_TEXTURE_ADDRESS_CLAMP;
        sampler_desc.ComparisonFunc = D3D11_COMPARISON_NEVER;
        sampler_desc.MinLOD = 0f32;
        sampler_desc.MaxLOD = D3D11_FLOAT32_MAX;

        let mut sampler_state = None;
        device.CreateSamplerState(&sampler_desc, Some(&mut sampler_state))?;

        Ok(sampler_state.unwrap())
    }
}

unsafe fn init_input_layout(device: &ID3D11Device) -> Result<ID3D11InputLayout, anyhow::Error> {
    unsafe {
        let input_element_desc_array = [
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(c"POSITION".as_ptr() as *const u8),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32B32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 0,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(c"TEXCOORD".as_ptr() as *const u8),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 12,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];

        let mut input_layout = None;

        device.CreateInputLayout(
            &input_element_desc_array,
            shader::VERTEX_SHADER_BYTES,
            Some(&mut input_layout),
        )?;

        Ok(input_layout.unwrap())
    }
}
