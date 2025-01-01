//! Metal renderer implementation.

use objc2::{rc::Retained, runtime::ProtocolObject, ClassType};
use objc2_foundation::{CGRect, CGSize, MainThreadMarker, NSUInteger};
use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue,
    MTLCreateSystemDefaultDevice, MTLDevice, MTLDrawable, MTLTexture,
};
use objc2_metal_kit::{MTKTextureLoader, MTKView};
use objc2_quartz_core::CAMetalDrawable;

use crate::{app::CorApp, util::app_from_objc};

use super::Texture;

const SLICE: NSUInteger = 0;
const MIPMAP_LEVEL: NSUInteger = 1;

/// Helper macro for creating texture objects.
macro_rules! texture {
    ($($name:ident => $file:literal)*) => {
        thread_local!{
            /// `All the Textures` instance.
            static TEXTURE: TextureCollection = {
                let loader = DEVICE.with(|device| unsafe {
                    MTKTextureLoader::initWithDevice(MTKTextureLoader::alloc(), device)
                });
                unsafe {
                    TextureCollection {
                        $(
                            $name: loader
                                .newTextureWithData_options_error(
                                    &objc2_foundation::NSData::with_bytes(include_bytes!($file)),
                                    None,
                                )
                                .expect(concat!("can't load texture ", $file)),
                        )*
                    }
                }
            };
        }
    };
}

/// All the Textures
struct TextureCollection {
    /// Contains all possible characters of all colors.
    typ: Retained<ProtocolObject<dyn MTLTexture>>,
    /// The line between two text sections.
    ui: Retained<ProtocolObject<dyn MTLTexture>>,
    /// Main game resources.
    tile: Retained<ProtocolObject<dyn MTLTexture>>,
}

thread_local! {
    /// Metal device representing a GPU the game uses for rendering.
    static DEVICE: Retained<ProtocolObject<dyn MTLDevice>> =
        unsafe { Retained::retain(MTLCreateSystemDefaultDevice()) }.expect("can't create MTLDevice");

    /// Texture loader to load game image resources.
    static TEXTURE_LOADER: Retained<MTKTextureLoader> = DEVICE
        .with(|device| unsafe { MTKTextureLoader::initWithDevice(MTKTextureLoader::alloc(), device) });
}
texture! {
    typ => "../../../images/type.gif"
    ui => "../../../images/ui.gif"
    tile => "../../../images/tileset.gif"
}

pub fn draw_raw(
    texture: Texture,
    dest: cacao::core_graphics::display::CGPoint,
    rect: cacao::core_graphics::display::CGRect,
) {
    todo!()
}

pub struct Renderer {
    pub view: Retained<MTKView>,
    pub size: CGSize,
    pub command_queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    pub command_buffer: Option<Retained<ProtocolObject<dyn MTLCommandBuffer>>>,
    pub command_encoder: Option<Retained<ProtocolObject<dyn MTLBlitCommandEncoder>>>,
    pub current_drawable: Option<Retained<ProtocolObject<dyn CAMetalDrawable>>>,
}

impl Renderer {
    /// # Safety
    /// Must be called on a main thread.
    pub fn new() -> Self {
        let view = DEVICE.with(|device| unsafe {
            MTKView::initWithFrame_device(
                MainThreadMarker::alloc(
                    MainThreadMarker::new()
                        .expect("called Metal Renderer::new() on a non-main thread"),
                ),
                CGRect::ZERO,
                Some(device),
            )
        });
        unsafe {
            view.setPaused(true);
            view.setEnableSetNeedsDisplay(true);
            view.setAutoResizeDrawable(false);
        }
        Self {
            view,
            size: CGSize::ZERO,
            command_queue: DEVICE.with(|dev| {
                dev.newCommandQueue()
                    .expect("error creating MTLCommandQueue")
            }),
            command_buffer: None,
            command_encoder: None,
            current_drawable: None,
        }
    }

    #[inline]
    pub fn set_view_needs_display(&self, needs_display: bool) {
        unsafe { self.view.setNeedsDisplay(needs_display) }
    }

    #[inline]
    pub fn set_content_window<T>(&self, window: &cacao::appkit::window::Window<T>) {
        /// Type system stretch.
        #[repr(transparent)]
        struct Encodable<T>(pub T);
        unsafe impl cacao::objc::RefEncode for Encodable<*mut MTKView> {
            const ENCODING_REF: cacao::objc::Encoding = cacao::objc::Encoding::Object;
        }

        let ptr = Encodable(Retained::<MTKView>::into_raw(self.view.retain()));
        unsafe {
            let _: () = cacao::objc::msg_send![&window.objc, setContentView:&ptr];
            let _obj = Retained::<MTKView>::from_raw(ptr.0)
                .expect("Metal Renderer::set_content_window() failure");
        }
    }

    pub fn init_renderer(&mut self, screen_size: cacao::core_graphics::display::CGSize) {
        let size = CGSize {
            width: screen_size.width,
            height: screen_size.height,
        };
        unsafe {
            self.view.setDrawableSize(size);
        }
        self.size = size;
    }

    pub fn finalize_renderer(&mut self) {
        self.size = CGSize::ZERO;
        unsafe {
            self.view.releaseDrawables();
        }
    }

    pub fn init_frame(&mut self) {
        // Check validity.
        unsafe {
            let _ = self
                .view
                .currentRenderPassDescriptor()
                .expect("error getting currentRenderPassDescriptor");
        }
        let command_buffer = self
            .command_queue
            .commandBuffer()
            .expect("error creating MTLCommandBuffer");
        self.command_encoder = Some(
            command_buffer
                .blitCommandEncoder()
                .expect("error creating MTLBlitCommandEncoder"),
        );
        self.current_drawable = Some(unsafe {
            self.view
                .currentDrawable()
                .expect("error getting currentDrawable")
        });
        self.command_buffer = Some(command_buffer);
    }

    pub fn finalize_frame(&mut self) {
        self.command_encoder
            .take()
            .expect("called Metal Renderer::finalize_frame() but COMMAND_ENCODER is None")
            .endEncoding();
        let command_buffer = self
            .command_buffer
            .take()
            .expect("called Metal Renderer::finalize_frame() but command_buffer is None");
        command_buffer.waitUntilScheduled();
        self.current_drawable
            .take()
            .expect("called Metal Renderer::finalize_frame() but current_drawable is None")
            .present();
        command_buffer.commit();
    }
}
