use mtl::{MTLFeatureSet, MTLGPUFamily, MTLLanguageVersion};
use objc::{class, msg_send, sel, sel_impl};
use parking_lot::Mutex;

use std::{sync::Arc, thread};

unsafe impl Send for super::Adapter {}
unsafe impl Sync for super::Adapter {}

impl super::Adapter {
    pub(super) fn new(shared: Arc<super::AdapterShared>) -> Self {
        Self { shared }
    }
}

impl crate::Adapter<super::Api> for super::Adapter {
    unsafe fn open(
        &self,
        features: wgt::Features,
    ) -> Result<crate::OpenDevice<super::Api>, crate::DeviceError> {
        let queue = self.shared.device.lock().new_command_queue();
        Ok(crate::OpenDevice {
            device: super::Device {
                shared: Arc::clone(&self.shared),
                features,
            },
            queue: super::Queue {
                raw: Arc::new(Mutex::new(queue)),
            },
        })
    }

    unsafe fn texture_format_capabilities(
        &self,
        format: wgt::TextureFormat,
    ) -> crate::TextureFormatCapabilities {
        use crate::TextureFormatCapabilities as Tfc;
        use wgt::TextureFormat as Tf;

        let pc = &self.shared.private_caps;
        // Affected formats documented at:
        // https://developer.apple.com/documentation/metal/mtlreadwritetexturetier/mtlreadwritetexturetier1?language=objc
        // https://developer.apple.com/documentation/metal/mtlreadwritetexturetier/mtlreadwritetexturetier2?language=objc
        let (read_write_tier1_if, read_write_tier2_if) = match pc.read_write_texture_tier {
            mtl::MTLReadWriteTextureTier::TierNone => (Tfc::empty(), Tfc::empty()),
            mtl::MTLReadWriteTextureTier::Tier1 => (Tfc::STORAGE_READ_WRITE, Tfc::empty()),
            mtl::MTLReadWriteTextureTier::Tier2 => {
                (Tfc::STORAGE_READ_WRITE, Tfc::STORAGE_READ_WRITE)
            }
        };

        let extra = match format {
            Tf::R8Unorm => {
                read_write_tier2_if
                    | Tfc::SAMPLED_LINEAR
                    | Tfc::STORAGE
                    | Tfc::COLOR_ATTACHMENT
                    | Tfc::COLOR_ATTACHMENT_BLEND
            }
            Tf::R8Snorm => {
                Tfc::SAMPLED_LINEAR
                    | Tfc::STORAGE
                    | Tfc::COLOR_ATTACHMENT
                    | Tfc::COLOR_ATTACHMENT_BLEND
            }
            Tf::R8Uint | Tf::R8Sint | Tf::R16Uint | Tf::R16Sint => {
                read_write_tier2_if | Tfc::STORAGE | Tfc::COLOR_ATTACHMENT
            }
            Tf::R16Float => {
                read_write_tier2_if
                    | Tfc::STORAGE
                    | Tfc::COLOR_ATTACHMENT
                    | Tfc::COLOR_ATTACHMENT_BLEND
            }
            Tf::Rg8Unorm | Tf::Rg8Snorm => {
                Tfc::SAMPLED_LINEAR
                    | Tfc::STORAGE
                    | Tfc::COLOR_ATTACHMENT
                    | Tfc::COLOR_ATTACHMENT_BLEND
            }
            Tf::Rg8Uint | Tf::Rg8Sint => Tfc::COLOR_ATTACHMENT,
            Tf::R32Uint | Tf::R32Sint => {
                if pc.format_r32_all {
                    read_write_tier1_if | Tfc::STORAGE | Tfc::COLOR_ATTACHMENT
                } else {
                    Tfc::COLOR_ATTACHMENT
                }
            }
            Tf::R32Float => {
                let mut flags = Tfc::COLOR_ATTACHMENT | Tfc::COLOR_ATTACHMENT_BLEND;
                if pc.format_r32float_all {
                    flags |= read_write_tier1_if | Tfc::STORAGE | Tfc::SAMPLED_LINEAR;
                } else if pc.format_r32float_no_filter {
                    flags |= Tfc::SAMPLED_LINEAR;
                }
                flags
            }
            Tf::Rg16Uint | Tf::Rg16Sint => {
                read_write_tier2_if | Tfc::STORAGE | Tfc::COLOR_ATTACHMENT
            }
            Tf::Rg16Float => {
                read_write_tier2_if
                    | Tfc::SAMPLED_LINEAR
                    | Tfc::STORAGE
                    | Tfc::COLOR_ATTACHMENT
                    | Tfc::COLOR_ATTACHMENT_BLEND
            }
            Tf::Rgba8Unorm => {
                read_write_tier2_if
                    | Tfc::SAMPLED_LINEAR
                    | Tfc::STORAGE
                    | Tfc::COLOR_ATTACHMENT
                    | Tfc::COLOR_ATTACHMENT_BLEND
            }
            Tf::Rgba8UnormSrgb | Tf::Bgra8UnormSrgb => {
                let mut flags =
                    Tfc::SAMPLED_LINEAR | Tfc::COLOR_ATTACHMENT | Tfc::COLOR_ATTACHMENT_BLEND;
                flags.set(Tfc::STORAGE, pc.format_rgba8_srgb_all);
                flags
            }
            Tf::Rgba8Snorm | Tf::Bgra8Unorm => {
                Tfc::SAMPLED_LINEAR
                    | Tfc::STORAGE
                    | Tfc::COLOR_ATTACHMENT
                    | Tfc::COLOR_ATTACHMENT_BLEND
            }
            Tf::Rgba8Uint | Tf::Rgba8Sint => {
                read_write_tier2_if | Tfc::STORAGE | Tfc::COLOR_ATTACHMENT
            }
            Tf::Rgb10a2Unorm => {
                let mut flags =
                    Tfc::SAMPLED_LINEAR | Tfc::COLOR_ATTACHMENT | Tfc::COLOR_ATTACHMENT_BLEND;
                flags.set(Tfc::STORAGE, pc.format_rgb10a2_unorm_all);
                flags
            }
            Tf::Rg11b10Float => {
                let mut flags =
                    Tfc::SAMPLED_LINEAR | Tfc::COLOR_ATTACHMENT | Tfc::COLOR_ATTACHMENT_BLEND;
                flags.set(Tfc::STORAGE, pc.format_rg11b10_all);
                flags
            }
            Tf::Rg32Uint | Tf::Rg32Sint => Tfc::COLOR_ATTACHMENT | Tfc::STORAGE,
            Tf::Rg32Float => {
                let mut flags = Tfc::COLOR_ATTACHMENT | Tfc::COLOR_ATTACHMENT_BLEND;
                if pc.format_rg32float_all {
                    flags |= Tfc::STORAGE | Tfc::SAMPLED_LINEAR;
                } else if pc.format_rg32float_color_blend {
                    flags |= Tfc::SAMPLED_LINEAR;
                }
                flags
            }
            Tf::Rgba16Uint | Tf::Rgba16Sint => {
                read_write_tier2_if | Tfc::STORAGE | Tfc::COLOR_ATTACHMENT
            }
            Tf::Rgba16Float => {
                read_write_tier2_if
                    | Tfc::SAMPLED_LINEAR
                    | Tfc::STORAGE
                    | Tfc::COLOR_ATTACHMENT
                    | Tfc::COLOR_ATTACHMENT_BLEND
            }
            Tf::Rgba32Uint | Tf::Rgba32Sint => {
                if pc.format_rgba32int_color_write {
                    read_write_tier2_if | Tfc::COLOR_ATTACHMENT | Tfc::STORAGE
                } else {
                    Tfc::COLOR_ATTACHMENT
                }
            }
            Tf::Rgba32Float => {
                if pc.format_rgba32float_all {
                    read_write_tier2_if
                        | Tfc::SAMPLED_LINEAR
                        | Tfc::STORAGE
                        | Tfc::COLOR_ATTACHMENT
                        | Tfc::COLOR_ATTACHMENT_BLEND
                } else if pc.format_rgba32float_color_write {
                    read_write_tier2_if | Tfc::COLOR_ATTACHMENT | Tfc::STORAGE
                } else {
                    Tfc::COLOR_ATTACHMENT
                }
            }
            Tf::Depth32Float => {
                if pc.format_depth32float_filter {
                    Tfc::DEPTH_STENCIL_ATTACHMENT | Tfc::SAMPLED_LINEAR
                } else {
                    Tfc::DEPTH_STENCIL_ATTACHMENT
                }
            }
            Tf::Depth24Plus | Tf::Depth24PlusStencil8 => {
                Tfc::DEPTH_STENCIL_ATTACHMENT | Tfc::SAMPLED_LINEAR
            }
            Tf::Rgb9e5Ufloat => Tfc::SAMPLED_LINEAR,
            Tf::Bc1RgbaUnorm
            | Tf::Bc1RgbaUnormSrgb
            | Tf::Bc2RgbaUnorm
            | Tf::Bc2RgbaUnormSrgb
            | Tf::Bc3RgbaUnorm
            | Tf::Bc3RgbaUnormSrgb
            | Tf::Bc4RUnorm
            | Tf::Bc4RSnorm
            | Tf::Bc5RgUnorm
            | Tf::Bc5RgSnorm
            | Tf::Bc6hRgbUfloat
            | Tf::Bc6hRgbSfloat
            | Tf::Bc7RgbaUnorm
            | Tf::Bc7RgbaUnormSrgb => {
                if pc.format_bc {
                    Tfc::SAMPLED_LINEAR
                } else {
                    Tfc::empty()
                }
            }
            Tf::Etc2RgbUnorm
            | Tf::Etc2RgbUnormSrgb
            | Tf::Etc2RgbA1Unorm
            | Tf::Etc2RgbA1UnormSrgb
            | Tf::EacRUnorm
            | Tf::EacRSnorm
            | Tf::EacRgUnorm
            | Tf::EacRgSnorm => {
                if pc.format_eac_etc {
                    Tfc::SAMPLED_LINEAR
                } else {
                    Tfc::empty()
                }
            }
            Tf::Astc4x4RgbaUnorm
            | Tf::Astc4x4RgbaUnormSrgb
            | Tf::Astc5x4RgbaUnorm
            | Tf::Astc5x4RgbaUnormSrgb
            | Tf::Astc5x5RgbaUnorm
            | Tf::Astc5x5RgbaUnormSrgb
            | Tf::Astc6x5RgbaUnorm
            | Tf::Astc6x5RgbaUnormSrgb
            | Tf::Astc6x6RgbaUnorm
            | Tf::Astc6x6RgbaUnormSrgb
            | Tf::Astc8x5RgbaUnorm
            | Tf::Astc8x5RgbaUnormSrgb
            | Tf::Astc8x6RgbaUnorm
            | Tf::Astc8x6RgbaUnormSrgb
            | Tf::Astc10x5RgbaUnorm
            | Tf::Astc10x5RgbaUnormSrgb
            | Tf::Astc10x6RgbaUnorm
            | Tf::Astc10x6RgbaUnormSrgb
            | Tf::Astc8x8RgbaUnorm
            | Tf::Astc8x8RgbaUnormSrgb
            | Tf::Astc10x8RgbaUnorm
            | Tf::Astc10x8RgbaUnormSrgb
            | Tf::Astc10x10RgbaUnorm
            | Tf::Astc10x10RgbaUnormSrgb
            | Tf::Astc12x10RgbaUnorm
            | Tf::Astc12x10RgbaUnormSrgb
            | Tf::Astc12x12RgbaUnorm
            | Tf::Astc12x12RgbaUnormSrgb => {
                if pc.format_astc {
                    Tfc::SAMPLED_LINEAR
                } else {
                    Tfc::empty()
                }
            }
        };

        Tfc::COPY_SRC | Tfc::COPY_DST | Tfc::SAMPLED | extra
    }

    unsafe fn surface_capabilities(
        &self,
        surface: &super::Surface,
    ) -> Option<crate::SurfaceCapabilities> {
        let current_extent = if surface.main_thread_id == thread::current().id() {
            Some(surface.dimensions())
        } else {
            log::warn!("Unable to get the current view dimensions on a non-main thread");
            None
        };

        let pc = &self.shared.private_caps;
        Some(crate::SurfaceCapabilities {
            formats: vec![
                wgt::TextureFormat::Bgra8Unorm,
                wgt::TextureFormat::Bgra8UnormSrgb,
                wgt::TextureFormat::Rgba16Float,
            ],
            //Note: this is hardcoded in `CAMetalLayer` documentation
            swap_chain_sizes: if pc.can_set_maximum_drawables_count {
                2..=3
            } else {
                // 3 is the default in `CAMetalLayer` documentation
                // iOS 10.3 was tested to use 3 on iphone5s
                3..=3
            },
            present_modes: if pc.can_set_display_sync {
                vec![wgt::PresentMode::Fifo, wgt::PresentMode::Immediate]
            } else {
                vec![wgt::PresentMode::Fifo]
            },
            composite_alpha_modes: vec![
                crate::CompositeAlphaMode::Opaque,
                crate::CompositeAlphaMode::PreMultiplied,
                crate::CompositeAlphaMode::PostMultiplied,
            ],

            current_extent,
            extents: wgt::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            }..=wgt::Extent3d {
                width: 4096,
                height: 4096,
                depth_or_array_layers: 1,
            },
            usage: crate::TextureUses::COLOR_TARGET, //TODO: expose more
        })
    }
}

const RESOURCE_HEAP_SUPPORT: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily1_v3,
    MTLFeatureSet::iOS_GPUFamily2_v3,
    MTLFeatureSet::iOS_GPUFamily3_v2,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v2,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v3,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const ARGUMENT_BUFFER_SUPPORT: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily1_v4,
    MTLFeatureSet::iOS_GPUFamily2_v4,
    MTLFeatureSet::iOS_GPUFamily3_v3,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v3,
    MTLFeatureSet::macOS_GPUFamily1_v3,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const MUTABLE_COMPARISON_SAMPLER_SUPPORT: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::macOS_GPUFamily1_v1,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const SAMPLER_CLAMP_TO_BORDER_SUPPORT: &[MTLFeatureSet] = &[
    MTLFeatureSet::macOS_GPUFamily1_v2,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const ASTC_PIXEL_FORMAT_FEATURES: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily2_v1,
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v1,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
];

const ANY8_UNORM_SRGB_ALL: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily2_v3,
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v2,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
];

const ANY8_SNORM_RESOLVE: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily2_v1,
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v1,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v1,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const RGBA8_SRGB: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily2_v3,
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v2,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
];

const RGB10A2UNORM_ALL: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v1,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const RGB10A2UINT_COLOR_WRITE: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v1,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const RG11B10FLOAT_ALL: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v1,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const RGB9E5FLOAT_ALL: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
];

const BGR10A2_ALL: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily1_v4,
    MTLFeatureSet::iOS_GPUFamily2_v4,
    MTLFeatureSet::iOS_GPUFamily3_v3,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v3,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v3,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const BASE_INSTANCE_SUPPORT: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v1,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const BASE_VERTEX_INSTANCE_SUPPORT: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v1,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const TEXTURE_CUBE_ARRAY_SUPPORT: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v2,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v1,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const DUAL_SOURCE_BLEND_SUPPORT: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily1_v4,
    MTLFeatureSet::iOS_GPUFamily2_v4,
    MTLFeatureSet::iOS_GPUFamily3_v3,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v3,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v2,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const LAYERED_RENDERING_SUPPORT: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::macOS_GPUFamily1_v1,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const FUNCTION_SPECIALIZATION_SUPPORT: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily1_v3,
    MTLFeatureSet::iOS_GPUFamily2_v3,
    MTLFeatureSet::iOS_GPUFamily3_v2,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v2,
    MTLFeatureSet::macOS_GPUFamily1_v2,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

const DEPTH_CLIP_MODE: &[MTLFeatureSet] = &[
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v3,
    MTLFeatureSet::macOS_GPUFamily1_v1,
    MTLFeatureSet::macOS_GPUFamily2_v1,
];

impl super::PrivateCapabilities {
    fn version_at_least(major: u32, minor: u32, needed_major: u32, needed_minor: u32) -> bool {
        major > needed_major || (major == needed_major && minor >= needed_minor)
    }

    fn supports_any(raw: &mtl::DeviceRef, features_sets: &[MTLFeatureSet]) -> bool {
        features_sets
            .iter()
            .cloned()
            .any(|x| raw.supports_feature_set(x))
    }

    pub fn new(device: &mtl::Device) -> Self {
        #[repr(C)]
        #[derive(Clone, Copy, Debug)]
        #[allow(clippy::upper_case_acronyms)]
        struct NSOperatingSystemVersion {
            major: usize,
            minor: usize,
            patch: usize,
        }

        let version: NSOperatingSystemVersion = unsafe {
            let process_info: *mut objc::runtime::Object =
                msg_send![class!(NSProcessInfo), processInfo];
            msg_send![process_info, operatingSystemVersion]
        };

        let major = version.major as u32;
        let minor = version.minor as u32;
        let os_is_mac = device.supports_feature_set(MTLFeatureSet::macOS_GPUFamily1_v1);
        let family_check = if os_is_mac {
            Self::version_at_least(major, minor, 10, 15)
        } else {
            Self::version_at_least(major, minor, 13, 0)
        };

        let mut sample_count_mask: u8 = 1 | 4; // 1 and 4 samples are supported on all devices
        if device.supports_texture_sample_count(2) {
            sample_count_mask |= 2;
        }
        if device.supports_texture_sample_count(8) {
            sample_count_mask |= 8;
        }

        Self {
            family_check,
            msl_version: if os_is_mac {
                if Self::version_at_least(major, minor, 10, 15) {
                    MTLLanguageVersion::V2_2
                } else if Self::version_at_least(major, minor, 10, 14) {
                    MTLLanguageVersion::V2_1
                } else if Self::version_at_least(major, minor, 10, 13) {
                    MTLLanguageVersion::V2_0
                } else if Self::version_at_least(major, minor, 10, 12) {
                    MTLLanguageVersion::V1_2
                } else if Self::version_at_least(major, minor, 10, 11) {
                    MTLLanguageVersion::V1_1
                } else {
                    MTLLanguageVersion::V1_0
                }
            } else if Self::version_at_least(major, minor, 13, 0) {
                MTLLanguageVersion::V2_2
            } else if Self::version_at_least(major, minor, 12, 0) {
                MTLLanguageVersion::V2_1
            } else if Self::version_at_least(major, minor, 11, 0) {
                MTLLanguageVersion::V2_0
            } else if Self::version_at_least(major, minor, 10, 0) {
                MTLLanguageVersion::V1_2
            } else if Self::version_at_least(major, minor, 9, 0) {
                MTLLanguageVersion::V1_1
            } else {
                MTLLanguageVersion::V1_0
            },
            exposed_queues: 1,
            read_write_texture_tier: if os_is_mac {
                if Self::version_at_least(major, minor, 10, 13) {
                    device.read_write_texture_support()
                } else {
                    mtl::MTLReadWriteTextureTier::TierNone
                }
            } else if Self::version_at_least(major, minor, 11, 0) {
                device.read_write_texture_support()
            } else {
                mtl::MTLReadWriteTextureTier::TierNone
            },
            resource_heaps: Self::supports_any(device, RESOURCE_HEAP_SUPPORT),
            argument_buffers: Self::supports_any(device, ARGUMENT_BUFFER_SUPPORT),
            shared_textures: !os_is_mac,
            mutable_comparison_samplers: Self::supports_any(
                device,
                MUTABLE_COMPARISON_SAMPLER_SUPPORT,
            ),
            sampler_clamp_to_border: Self::supports_any(device, SAMPLER_CLAMP_TO_BORDER_SUPPORT),
            sampler_lod_average: {
                // TODO: Clarify minimum macOS version with Apple (43707452)
                let need_version = if os_is_mac { (10, 13) } else { (9, 0) };
                Self::version_at_least(major, minor, need_version.0, need_version.1)
            },
            base_instance: Self::supports_any(device, BASE_INSTANCE_SUPPORT),
            base_vertex_instance_drawing: Self::supports_any(device, BASE_VERTEX_INSTANCE_SUPPORT),
            dual_source_blending: Self::supports_any(device, DUAL_SOURCE_BLEND_SUPPORT),
            low_power: !os_is_mac || device.is_low_power(),
            headless: os_is_mac && device.is_headless(),
            layered_rendering: Self::supports_any(device, LAYERED_RENDERING_SUPPORT),
            function_specialization: Self::supports_any(device, FUNCTION_SPECIALIZATION_SUPPORT),
            depth_clip_mode: Self::supports_any(device, DEPTH_CLIP_MODE),
            texture_cube_array: Self::supports_any(device, TEXTURE_CUBE_ARRAY_SUPPORT),
            format_depth24_stencil8: os_is_mac && device.d24_s8_supported(),
            format_depth32_stencil8_filter: os_is_mac,
            format_depth32_stencil8_none: !os_is_mac,
            format_min_srgb_channels: if os_is_mac { 4 } else { 1 },
            format_b5: !os_is_mac,
            format_bc: os_is_mac,
            format_eac_etc: !os_is_mac,
            format_astc: Self::supports_any(device, ASTC_PIXEL_FORMAT_FEATURES),
            format_any8_unorm_srgb_all: Self::supports_any(device, ANY8_UNORM_SRGB_ALL),
            format_any8_unorm_srgb_no_write: !Self::supports_any(device, ANY8_UNORM_SRGB_ALL)
                && !os_is_mac,
            format_any8_snorm_all: Self::supports_any(device, ANY8_SNORM_RESOLVE),
            format_r16_norm_all: os_is_mac,
            format_r32_all: !Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ),
            format_r32_no_write: Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ),
            format_r32float_no_write_no_filter: Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ) && !os_is_mac,
            format_r32float_no_filter: !Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ) && !os_is_mac,
            format_r32float_all: os_is_mac,
            format_rgba8_srgb_all: Self::supports_any(device, RGBA8_SRGB),
            format_rgba8_srgb_no_write: !Self::supports_any(device, RGBA8_SRGB),
            format_rgb10a2_unorm_all: Self::supports_any(device, RGB10A2UNORM_ALL),
            format_rgb10a2_unorm_no_write: !Self::supports_any(device, RGB10A2UNORM_ALL),
            format_rgb10a2_uint_color: !Self::supports_any(device, RGB10A2UINT_COLOR_WRITE),
            format_rgb10a2_uint_color_write: Self::supports_any(device, RGB10A2UINT_COLOR_WRITE),
            format_rg11b10_all: Self::supports_any(device, RG11B10FLOAT_ALL),
            format_rg11b10_no_write: !Self::supports_any(device, RG11B10FLOAT_ALL),
            format_rgb9e5_all: Self::supports_any(device, RGB9E5FLOAT_ALL),
            format_rgb9e5_no_write: !Self::supports_any(device, RGB9E5FLOAT_ALL) && !os_is_mac,
            format_rgb9e5_filter_only: os_is_mac,
            format_rg32_color: Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ),
            format_rg32_color_write: !Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ),
            format_rg32float_all: os_is_mac,
            format_rg32float_color_blend: Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ),
            format_rg32float_no_filter: !os_is_mac
                && !Self::supports_any(
                    device,
                    &[
                        MTLFeatureSet::iOS_GPUFamily1_v1,
                        MTLFeatureSet::iOS_GPUFamily2_v1,
                    ],
                ),
            format_rgba32int_color: Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ),
            format_rgba32int_color_write: !Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ),
            format_rgba32float_color: Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ),
            format_rgba32float_color_write: !Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v1,
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                ],
            ) && !os_is_mac,
            format_rgba32float_all: os_is_mac,
            format_depth16unorm: device.supports_feature_set(MTLFeatureSet::macOS_GPUFamily1_v2),
            format_depth32float_filter: device
                .supports_feature_set(MTLFeatureSet::macOS_GPUFamily1_v1),
            format_depth32float_none: !device
                .supports_feature_set(MTLFeatureSet::macOS_GPUFamily1_v1),
            format_bgr10a2_all: Self::supports_any(device, BGR10A2_ALL),
            format_bgr10a2_no_write: !device
                .supports_feature_set(MTLFeatureSet::macOS_GPUFamily1_v3),
            max_buffers_per_stage: 31,
            max_textures_per_stage: if os_is_mac { 128 } else { 31 },
            max_samplers_per_stage: 16,
            buffer_alignment: if os_is_mac { 256 } else { 64 },
            max_buffer_size: if device.supports_feature_set(MTLFeatureSet::macOS_GPUFamily1_v2) {
                1 << 30 // 1GB on macOS 1.2 and up
            } else {
                1 << 28 // 256MB otherwise
            },
            max_texture_size: if Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily3_v1,
                    MTLFeatureSet::tvOS_GPUFamily2_v1,
                    MTLFeatureSet::macOS_GPUFamily1_v1,
                ],
            ) {
                16384
            } else if Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily1_v2,
                    MTLFeatureSet::iOS_GPUFamily2_v2,
                    MTLFeatureSet::tvOS_GPUFamily1_v1,
                ],
            ) {
                8192
            } else {
                4096
            },
            max_texture_3d_size: 2048,
            max_texture_layers: 2048,
            max_fragment_input_components: if os_is_mac { 128 } else { 60 },
            max_color_render_targets: if Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily2_v1,
                    MTLFeatureSet::iOS_GPUFamily3_v1,
                    MTLFeatureSet::iOS_GPUFamily4_v1,
                    MTLFeatureSet::iOS_GPUFamily5_v1,
                    MTLFeatureSet::tvOS_GPUFamily1_v1,
                    MTLFeatureSet::tvOS_GPUFamily2_v1,
                    MTLFeatureSet::macOS_GPUFamily1_v1,
                    MTLFeatureSet::macOS_GPUFamily2_v1,
                ],
            ) {
                8
            } else {
                4
            },
            max_total_threadgroup_memory: if Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily4_v2,
                    MTLFeatureSet::iOS_GPUFamily5_v1,
                ],
            ) {
                64 << 10
            } else if Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily4_v1,
                    MTLFeatureSet::macOS_GPUFamily1_v2,
                    MTLFeatureSet::macOS_GPUFamily2_v1,
                ],
            ) {
                32 << 10
            } else {
                16 << 10
            },
            sample_count_mask,
            supports_debug_markers: Self::supports_any(
                device,
                &[
                    MTLFeatureSet::macOS_GPUFamily1_v2,
                    MTLFeatureSet::macOS_GPUFamily2_v1,
                    MTLFeatureSet::iOS_GPUFamily1_v3,
                    MTLFeatureSet::iOS_GPUFamily2_v3,
                    MTLFeatureSet::iOS_GPUFamily3_v2,
                    MTLFeatureSet::iOS_GPUFamily4_v1,
                    MTLFeatureSet::iOS_GPUFamily5_v1,
                    MTLFeatureSet::tvOS_GPUFamily1_v2,
                    MTLFeatureSet::tvOS_GPUFamily2_v1,
                ],
            ),
            supports_binary_archives: family_check
                && (device.supports_family(MTLGPUFamily::Apple3)
                    || device.supports_family(MTLGPUFamily::Mac1)),
            supports_capture_manager: if os_is_mac {
                Self::version_at_least(major, minor, 10, 13)
            } else {
                Self::version_at_least(major, minor, 11, 0)
            },
            can_set_maximum_drawables_count: os_is_mac
                || Self::version_at_least(major, minor, 11, 2),
            can_set_display_sync: os_is_mac && Self::version_at_least(major, minor, 10, 13),
            can_set_next_drawable_timeout: if os_is_mac {
                Self::version_at_least(major, minor, 10, 13)
            } else {
                Self::version_at_least(major, minor, 11, 0)
            },
            supports_arrays_of_textures: Self::supports_any(
                device,
                &[
                    MTLFeatureSet::iOS_GPUFamily3_v2,
                    MTLFeatureSet::iOS_GPUFamily4_v1,
                    MTLFeatureSet::iOS_GPUFamily5_v1,
                    MTLFeatureSet::tvOS_GPUFamily2_v1,
                    MTLFeatureSet::macOS_GPUFamily1_v3,
                    MTLFeatureSet::macOS_GPUFamily2_v1,
                ],
            ),
            supports_arrays_of_textures_write: family_check
                && (device.supports_family(MTLGPUFamily::Apple6)
                    || device.supports_family(MTLGPUFamily::Mac1)
                    || device.supports_family(MTLGPUFamily::Mac2)
                    || device.supports_family(MTLGPUFamily::MacCatalyst1)
                    || device.supports_family(MTLGPUFamily::MacCatalyst2)),
            supports_mutability: if os_is_mac {
                Self::version_at_least(major, minor, 10, 13)
            } else {
                Self::version_at_least(major, minor, 11, 0)
            },
        }
    }

    pub fn features(&self) -> wgt::Features {
        use wgt::Features as F;

        let mut features = F::empty()
            | F::DEPTH_CLAMPING
            | F::TEXTURE_COMPRESSION_BC
            | F::MAPPABLE_PRIMARY_BUFFERS
            | F::VERTEX_WRITABLE_STORAGE
            | F::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | F::POLYGON_MODE_LINE
            | F::CLEAR_COMMANDS;

        features.set(
            F::TEXTURE_BINDING_ARRAY
                | F::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
                | F::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING,
            self.msl_version >= MTLLanguageVersion::V2_0 && self.supports_arrays_of_textures,
        );
        //// XXX: this is technically not true, as read-only storage images can be used in arrays
        //// on precisely the same conditions that sampled textures can. But texel fetch from a
        //// sampled texture is a thing; should we bother introducing another feature flag?
        if self.msl_version >= MTLLanguageVersion::V2_2
            && self.supports_arrays_of_textures
            && self.supports_arrays_of_textures_write
        {
            features.insert(F::STORAGE_RESOURCE_BINDING_ARRAY);
        }
        features.set(
            F::ADDRESS_MODE_CLAMP_TO_BORDER,
            self.sampler_clamp_to_border,
        );

        features
    }

    pub fn capabilities(&self) -> crate::Capabilities {
        let mut downlevel = wgt::DownlevelCapabilities::default();
        downlevel.flags.set(
            wgt::DownlevelFlags::CUBE_ARRAY_TEXTURES,
            self.texture_cube_array,
        );
        //TODO: separate the mutable comparisons from immutable ones
        downlevel.flags.set(
            wgt::DownlevelFlags::COMPARISON_SAMPLERS,
            self.mutable_comparison_samplers,
        );
        downlevel
            .flags
            .set(wgt::DownlevelFlags::ANISOTROPIC_FILTERING, true);

        let base = wgt::Limits::default();
        crate::Capabilities {
            limits: wgt::Limits {
                max_texture_dimension_1d: self.max_texture_size as u32,
                max_texture_dimension_2d: self.max_texture_size as u32,
                max_texture_dimension_3d: self.max_texture_3d_size as u32,
                max_texture_array_layers: self.max_texture_layers as u32,
                max_bind_groups: 8,
                max_dynamic_uniform_buffers_per_pipeline_layout: base
                    .max_dynamic_uniform_buffers_per_pipeline_layout,
                max_dynamic_storage_buffers_per_pipeline_layout: base
                    .max_dynamic_storage_buffers_per_pipeline_layout,
                max_sampled_textures_per_shader_stage: base.max_sampled_textures_per_shader_stage,
                max_samplers_per_shader_stage: self.max_samplers_per_stage,
                max_storage_buffers_per_shader_stage: base.max_storage_buffers_per_shader_stage,
                max_storage_textures_per_shader_stage: base.max_storage_textures_per_shader_stage,
                max_uniform_buffers_per_shader_stage: 12,
                max_uniform_buffer_binding_size: self.max_buffer_size.min(!0u32 as u64) as u32,
                max_storage_buffer_binding_size: self.max_buffer_size.min(!0u32 as u64) as u32,
                max_vertex_buffers: base.max_vertex_buffers,
                max_vertex_attributes: base.max_vertex_attributes,
                max_vertex_buffer_array_stride: base.max_vertex_buffer_array_stride,
                max_push_constant_size: 0x1000,
                min_uniform_buffer_offset_alignment: self.buffer_alignment as u32,
                min_storage_buffer_offset_alignment: self.buffer_alignment as u32,
            },
            alignments: crate::Alignments {
                buffer_copy_offset: wgt::BufferSize::new(self.buffer_alignment).unwrap(),
                buffer_copy_pitch: wgt::BufferSize::new(4).unwrap(),
            },
            downlevel,
        }
    }

    pub fn map_format(&self, format: wgt::TextureFormat) -> mtl::MTLPixelFormat {
        use mtl::MTLPixelFormat::*;
        use wgt::TextureFormat as Tf;

        match format {
            Tf::R8Unorm => R8Unorm,
            Tf::R8Snorm => R8Snorm,
            Tf::R8Uint => R8Uint,
            Tf::R8Sint => R8Sint,
            Tf::R16Uint => R16Uint,
            Tf::R16Sint => R16Sint,
            Tf::R16Float => R16Float,
            Tf::Rg8Unorm => RG8Unorm,
            Tf::Rg8Snorm => RG8Snorm,
            Tf::Rg8Uint => RG8Uint,
            Tf::Rg8Sint => RG8Sint,
            Tf::R32Uint => R32Uint,
            Tf::R32Sint => R32Sint,
            Tf::R32Float => R32Float,
            Tf::Rg16Uint => RG16Uint,
            Tf::Rg16Sint => RG16Sint,
            Tf::Rg16Float => RG16Float,
            Tf::Rgba8Unorm => RGBA8Unorm,
            Tf::Rgba8UnormSrgb => RGBA8Unorm_sRGB,
            Tf::Bgra8UnormSrgb => BGRA8Unorm_sRGB,
            Tf::Rgba8Snorm => RGBA8Snorm,
            Tf::Bgra8Unorm => BGRA8Unorm,
            Tf::Rgba8Uint => RGBA8Uint,
            Tf::Rgba8Sint => RGBA8Sint,
            Tf::Rgb10a2Unorm => RGB10A2Unorm,
            Tf::Rg11b10Float => RG11B10Float,
            Tf::Rg32Uint => RG32Uint,
            Tf::Rg32Sint => RG32Sint,
            Tf::Rg32Float => RG32Float,
            Tf::Rgba16Uint => RGBA16Uint,
            Tf::Rgba16Sint => RGBA16Sint,
            Tf::Rgba16Float => RGBA16Float,
            Tf::Rgba32Uint => RGBA32Uint,
            Tf::Rgba32Sint => RGBA32Sint,
            Tf::Rgba32Float => RGBA32Float,
            Tf::Depth32Float => Depth32Float,
            Tf::Depth24Plus => {
                if self.format_depth24_stencil8 {
                    Depth24Unorm_Stencil8
                } else {
                    Depth32Float
                }
            }
            Tf::Depth24PlusStencil8 => {
                if self.format_depth24_stencil8 {
                    Depth24Unorm_Stencil8
                } else {
                    Depth32Float_Stencil8
                }
            }
            Tf::Rgb9e5Ufloat => RGB9E5Float,
            Tf::Bc1RgbaUnorm => BC1_RGBA,
            Tf::Bc1RgbaUnormSrgb => BC1_RGBA_sRGB,
            Tf::Bc2RgbaUnorm => BC2_RGBA,
            Tf::Bc2RgbaUnormSrgb => BC2_RGBA_sRGB,
            Tf::Bc3RgbaUnorm => BC3_RGBA,
            Tf::Bc3RgbaUnormSrgb => BC3_RGBA_sRGB,
            Tf::Bc4RUnorm => BC4_RUnorm,
            Tf::Bc4RSnorm => BC4_RSnorm,
            Tf::Bc5RgUnorm => BC5_RGUnorm,
            Tf::Bc5RgSnorm => BC5_RGSnorm,
            Tf::Bc6hRgbSfloat => BC6H_RGBFloat,
            Tf::Bc6hRgbUfloat => BC6H_RGBUfloat,
            Tf::Bc7RgbaUnorm => BC7_RGBAUnorm,
            Tf::Bc7RgbaUnormSrgb => BC7_RGBAUnorm_sRGB,
            Tf::Etc2RgbUnorm => ETC2_RGB8,
            Tf::Etc2RgbUnormSrgb => ETC2_RGB8_sRGB,
            Tf::Etc2RgbA1Unorm => ETC2_RGB8A1,
            Tf::Etc2RgbA1UnormSrgb => ETC2_RGB8A1_sRGB,
            Tf::EacRUnorm => EAC_R11Unorm,
            Tf::EacRSnorm => EAC_R11Snorm,
            Tf::EacRgUnorm => EAC_RG11Unorm,
            Tf::EacRgSnorm => EAC_RG11Snorm,
            Tf::Astc4x4RgbaUnorm => ASTC_4x4_LDR,
            Tf::Astc4x4RgbaUnormSrgb => ASTC_4x4_sRGB,
            Tf::Astc5x4RgbaUnorm => ASTC_5x4_LDR,
            Tf::Astc5x4RgbaUnormSrgb => ASTC_5x4_sRGB,
            Tf::Astc5x5RgbaUnorm => ASTC_5x5_LDR,
            Tf::Astc5x5RgbaUnormSrgb => ASTC_5x5_sRGB,
            Tf::Astc6x5RgbaUnorm => ASTC_6x5_LDR,
            Tf::Astc6x5RgbaUnormSrgb => ASTC_6x5_sRGB,
            Tf::Astc6x6RgbaUnorm => ASTC_6x6_LDR,
            Tf::Astc6x6RgbaUnormSrgb => ASTC_6x6_sRGB,
            Tf::Astc8x5RgbaUnorm => ASTC_8x5_LDR,
            Tf::Astc8x5RgbaUnormSrgb => ASTC_8x5_sRGB,
            Tf::Astc8x6RgbaUnorm => ASTC_8x6_LDR,
            Tf::Astc8x6RgbaUnormSrgb => ASTC_8x6_sRGB,
            Tf::Astc10x5RgbaUnorm => ASTC_8x8_LDR,
            Tf::Astc10x5RgbaUnormSrgb => ASTC_8x8_sRGB,
            Tf::Astc10x6RgbaUnorm => ASTC_10x5_LDR,
            Tf::Astc10x6RgbaUnormSrgb => ASTC_10x5_sRGB,
            Tf::Astc8x8RgbaUnorm => ASTC_10x6_LDR,
            Tf::Astc8x8RgbaUnormSrgb => ASTC_10x6_sRGB,
            Tf::Astc10x8RgbaUnorm => ASTC_10x8_LDR,
            Tf::Astc10x8RgbaUnormSrgb => ASTC_10x8_sRGB,
            Tf::Astc10x10RgbaUnorm => ASTC_10x10_LDR,
            Tf::Astc10x10RgbaUnormSrgb => ASTC_10x10_sRGB,
            Tf::Astc12x10RgbaUnorm => ASTC_12x10_LDR,
            Tf::Astc12x10RgbaUnormSrgb => ASTC_12x10_sRGB,
            Tf::Astc12x12RgbaUnorm => ASTC_12x12_LDR,
            Tf::Astc12x12RgbaUnormSrgb => ASTC_12x12_sRGB,
        }
    }
}

impl super::PrivateDisabilities {
    pub fn new(device: &mtl::Device) -> Self {
        let is_intel = device.name().starts_with("Intel");
        Self {
            broken_viewport_near_depth: is_intel
                && !device.supports_feature_set(MTLFeatureSet::macOS_GPUFamily1_v4),
            broken_layered_clear_image: is_intel,
        }
    }
}
