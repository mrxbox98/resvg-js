// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


#[cfg(not(target_arch = "wasm32"))]
use napi::bindgen_prelude::{
    AbortSignal, AsyncTask, Buffer, Either, Error as NapiError, Task,
};
#[cfg(not(target_arch = "wasm32"))]
use napi_derive::napi;
use options::JsOptions;
use resvg::{
    tiny_skia::Pixmap,
    usvg::{self},
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{
    prelude::{wasm_bindgen, JsValue},
    JsCast,
};

mod error;
mod fonts;
mod options;

use error::Error;

#[cfg(all(not(target_family = "wasm"), not(debug_assertions),))]
#[cfg(not(all(target_os = "linux", target_arch = "arm")))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Uint8Array | string")]
    pub type IStringOrBuffer;
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg_attr(not(target_arch = "wasm32"), napi)]
#[derive(Debug)]
pub struct BBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg_attr(not(target_arch = "wasm32"), napi)]
pub struct Resvg {
    tree: usvg::Tree,
    js_options: JsOptions,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg_attr(not(target_arch = "wasm32"), napi)]
pub struct RenderedImage {
    pix: Pixmap,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg_attr(not(target_arch = "wasm32"), napi)]
impl RenderedImage {
    // Wasm
    #[cfg(not(target_arch = "wasm32"))]
    #[napi]
    /// Write the image data to Buffer
    pub fn as_png(&self) -> Result<Buffer, NapiError> {
        let buffer = self.pix.encode_png().map_err(Error::from)?;
        Ok(buffer.into())
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(getter)]
    /// Get the PNG width
    pub fn width(&self) -> u32 {
        self.pix.width()
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(getter)]
    /// Get the PNG height
    pub fn height(&self) -> u32 {
        self.pix.height()
    }

    // napi-rs
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(js_name = asPng)]
    /// Write the image data to Uint8Array
    pub fn as_png(&self) -> Result<js_sys::Uint8Array, js_sys::Error> {
        let buffer = self.pix.encode_png().map_err(Error::from)?;
        Ok(buffer.as_slice().into())
    }

    /// Get the RGBA pixels of the image
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(getter)]
    pub fn pixels(&self) -> js_sys::Uint8Array {
        self.pix.data().into()
    }

    /// Get the RGBA pixels of the image
    #[cfg(not(target_arch = "wasm32"))]
    #[napi(getter)]
    pub fn pixels(&self) -> Buffer {
        self.pix.data().into()
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[napi(getter)]
    /// Get the PNG width
    pub fn width(&self) -> u32 {
        self.pix.width()
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[napi(getter)]
    /// Get the PNG height
    pub fn height(&self) -> u32 {
        self.pix.height()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[napi]
impl Resvg {
    #[napi(constructor)]
    pub fn new(svg: Either<String, Buffer>, options: Option<String>) -> Result<Resvg, NapiError> {
        Resvg::new_inner(&svg, options)
    }

    fn new_inner(
        svg: &Either<String, Buffer>,
        options: Option<String>,
    ) -> Result<Resvg, NapiError> {
        let js_options: JsOptions = options
            .and_then(|o| serde_json::from_str(o.as_str()).ok())
            .unwrap_or_default();
        let _ = env_logger::builder()
            .filter_level(js_options.log_level) 
            .try_init();

        let opts = js_options.to_usvg_options();
        // Parse the SVG string into a tree.
        let tree = match svg {
            Either::A(a) => usvg::Tree::from_str(a.as_str(), &opts),
            Either::B(b) => usvg::Tree::from_data(b.as_ref(), &opts),
        }
        .map_err(|e| napi::Error::from_reason(format!("{e}")))?;

        // tree.convert_text(&fontdb);
        // Drop `opts` (which may borrow from `js_options`) before we move
        // `js_options` into the returned `Resvg`. This avoids the "cannot
        // move out of `js_options` because it is borrowed" error.
    drop(opts);
        Ok(Resvg { tree, js_options })
    }

    #[napi]
    /// Renders an SVG in Node.js
    pub fn render(&self) -> Result<RenderedImage, NapiError> {
        Ok(self.render_inner()?)
    }

    #[napi]
    /// Output usvg-simplified SVG string
    pub fn to_string(&self) -> String {
        self.tree.to_string(&usvg::WriteOptions::default())
    }

    /// Get the SVG width
    #[napi(getter)]
    pub fn width(&self) -> f32 {
        self.tree.size().width().round()
    }

    /// Get the SVG height
    #[napi(getter)]
    pub fn height(&self) -> f32 {
        self.tree.size().height().round()
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Resvg {
    #[wasm_bindgen(constructor)]
    pub fn new(
        svg: IStringOrBuffer,
        options: Option<String>,
        custom_font_buffers: Option<js_sys::Array>,
    ) -> Result<Resvg, js_sys::Error> {
        let js_options: JsOptions = options
            .and_then(|o| serde_json::from_str(o.as_str()).ok())
            .unwrap_or_default();

        let mut opts = js_options.to_usvg_options();

        // Create a font database and load wasm-supplied fonts into it,
        // then replace the options' fontdb with the populated one.
        let mut fontdb = resvg::usvg::fontdb::Database::new();
        crate::fonts::load_wasm_fonts(&js_options.font, custom_font_buffers, &mut fontdb)?;
        opts.fontdb = std::sync::Arc::new(fontdb);

        let mut tree = if js_sys::Uint8Array::instanceof(&svg) {
            let uintarray = js_sys::Uint8Array::unchecked_from_js_ref(&svg);
            let svg_buffer = uintarray.to_vec();
            usvg::Tree::from_data(&svg_buffer, &opts).map_err(Error::from)
        } else if let Some(s) = svg.as_string() {
            usvg::Tree::from_str(s.as_str(), &opts).map_err(Error::from)
        } else {
            Err(Error::InvalidInput)
        }?;
        // Drop `opts` (which may borrow from `js_options`) before we move
        // `js_options` into the returned `Resvg` to avoid borrow/move conflicts.
        drop(opts);
        Ok(Resvg { tree, js_options })
    }

    /// Get the SVG width
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> f32 {
        self.tree.size().width().round()
    }

    /// Get the SVG height
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> f32 {
        self.tree.size().height().round()
    }

    /// Renders an SVG in Wasm
    pub fn render(&self) -> Result<RenderedImage, js_sys::Error> {
        Ok(self.render_inner()?)
    }
    /// Output usvg-simplified SVG string
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.tree.to_string(&usvg::WriteOptions::default())
    }
}

impl Resvg {
    fn render_inner(&self) -> Result<RenderedImage, Error> {
        let (width, height, transform) = self.js_options.fit_to.fit_to(self.tree.size())?;
        let mut pixmap = self.js_options.create_pixmap(width, height)?;
        // Render the tree
        let _image = resvg::render(&self.tree, transform, &mut pixmap.as_mut());

        // Crop the SVG
        let crop_rect = resvg::tiny_skia::IntRect::from_ltrb(
            self.js_options.crop.left,
            self.js_options.crop.top,
            self.js_options.crop.right.unwrap_or(width as i32),
            self.js_options.crop.bottom.unwrap_or(height as i32),
        );

        if let Some(crop_rect) = crop_rect {
            pixmap = pixmap.clone_rect(crop_rect).unwrap_or(pixmap);
        }

        Ok(RenderedImage { pix: pixmap })
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct AsyncRenderer {
    options: Option<String>,
    svg: Either<String, Buffer>,
}

#[cfg(not(target_arch = "wasm32"))]
#[napi]
impl Task for AsyncRenderer {
    type Output = RenderedImage;
    type JsValue = RenderedImage;

    fn compute(&mut self) -> Result<Self::Output, NapiError> {
        let resvg = Resvg::new_inner(&self.svg, self.options.clone())?;
        resvg.render()
    }

    fn resolve(
        &mut self,
        _env: napi::Env,
        result: Self::Output,
    ) -> Result<Self::JsValue, NapiError> {
        Ok(result)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[napi]
pub fn render_async(
    svg: Either<String, Buffer>,
    options: Option<String>,
    signal: Option<AbortSignal>,
) -> AsyncTask<AsyncRenderer> {
    match signal {
        Some(s) => AsyncTask::with_signal(AsyncRenderer { options, svg }, s),
        None => AsyncTask::new(AsyncRenderer { options, svg }),
    }
}

// Detects the file type by magic number.
// Currently resvg only supports the following types of files.
pub enum MimeType {
    Png,
    Jpeg,
    Gif,
}

impl MimeType {
    pub fn parse(buffer: &[u8]) -> Result<Self, Error> {
        if buffer.len() < 4 {
            return Err(Error::UnsupportedImage);
        }
        Ok(match &buffer[0..4] {
            [0x89, 0x50, 0x4E, 0x47] => MimeType::Png,
            [0xFF, 0xD8, 0xFF, _] => MimeType::Jpeg,
            [0x47, 0x49, 0x46, _] => MimeType::Gif,
            _ => return Err(Error::UnsupportedImage),
        })
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            MimeType::Png => "image/png",
            MimeType::Gif => "image/gif",
            _ => "image/jpeg",
        }
    }
}
