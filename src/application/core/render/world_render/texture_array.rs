use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Device,
    Extent3d, FilterMode, MipmapFilterMode, Origin3d, Queue, SamplerBindingType, SamplerDescriptor,
    ShaderStages, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::application::core::{
    blocks::{BlockRegistry, TEXTURES_PER_BLOCK, TextureFace},
    paths::GamePaths,
};

const TEXTURE_SIZE: u32 = 16;

pub(super) struct BlockTextureArray {
    pub layout: BindGroupLayout,
    pub bind_group: BindGroup,
}

impl BlockTextureArray {
    pub fn new(
        device: &Device,
        queue: &Queue,
        registry: &BlockRegistry,
        paths: &GamePaths,
    ) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("block texture array"),
            size: Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: registry.len() as u32 * TEXTURES_PER_BLOCK,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for block in registry.iter() {
            for face in [TextureFace::Side, TextureFace::Top, TextureFace::Bottom] {
                let Some(path) = block.textures().path(face) else {
                    continue;
                };
                let asset = paths.asset(path.as_str());
                let image = image::open(&asset)
                    .unwrap_or_else(|error| panic!("failed to load {}: {error}", asset.display()))
                    .into_rgba8();
                assert_eq!(image.dimensions(), (TEXTURE_SIZE, TEXTURE_SIZE));

                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: Origin3d {
                            x: 0,
                            y: 0,
                            z: face.layer(block.id()),
                        },
                        aspect: TextureAspect::All,
                    },
                    image.as_raw(),
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(TEXTURE_SIZE * 4),
                        rows_per_image: Some(TEXTURE_SIZE),
                    },
                    Extent3d {
                        width: TEXTURE_SIZE,
                        height: TEXTURE_SIZE,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("block texture array view"),
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("block nearest sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("block texture bind group layout"),
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
            label: Some("block texture bind group"),
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
