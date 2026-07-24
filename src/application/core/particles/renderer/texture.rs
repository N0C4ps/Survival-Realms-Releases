use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Device,
    Extent3d, FilterMode, MipmapFilterMode, Origin3d, Queue, SamplerBindingType, SamplerDescriptor,
    ShaderStages, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::application::core::{blocks::BlockRegistry, paths::GamePaths};

use super::super::assets;

pub(super) struct ParticleTextures {
    pub layout: BindGroupLayout,
    pub bind_group: BindGroup,
}

impl ParticleTextures {
    pub fn new(
        device: &Device,
        queue: &Queue,
        registry: &BlockRegistry,
        paths: &GamePaths,
    ) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("particle texture array"),
            size: Extent3d {
                width: assets::TEXTURE_SIZE,
                height: assets::TEXTURE_SIZE,
                depth_or_array_layers: registry.len() as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for block in registry.iter() {
            let Some(path) = assets::texture_path(block.id()) else {
                continue;
            };
            let asset = paths.asset(path);
            let image = image::open(&asset)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", asset.display()))
                .into_rgba8();
            assert_eq!(
                image.dimensions(),
                (assets::TEXTURE_SIZE, assets::TEXTURE_SIZE)
            );
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: u32::from(block.id().value()),
                    },
                    aspect: TextureAspect::All,
                },
                image.as_raw(),
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(assets::TEXTURE_SIZE * 4),
                    rows_per_image: Some(assets::TEXTURE_SIZE),
                },
                Extent3d {
                    width: assets::TEXTURE_SIZE,
                    height: assets::TEXTURE_SIZE,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("particle texture array view"),
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("particle nearest sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("particle texture layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("particle texture bind group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });
        Self { layout, bind_group }
    }
}
