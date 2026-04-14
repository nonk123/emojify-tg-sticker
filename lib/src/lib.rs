use std::fmt;

use image::{GenericImageView, ImageError, Rgba, RgbaImage, imageops};

pub use image;

pub const EMOJI_TILE_SIZE: u32 = 100;

#[derive(Debug)]
pub enum EmojifyError {
    ImageError(ImageError),
    TilingExceedsTelegramLimit,
}

impl std::error::Error for EmojifyError {}

impl fmt::Display for EmojifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImageError(err) => write!(f, "{}", err),
            Self::TilingExceedsTelegramLimit => write!(
                f,
                "emojify produced over 50 tiles, which breaks Telegram's emoji-pack limits"
            ),
        }
    }
}

impl From<ImageError> for EmojifyError {
    fn from(value: ImageError) -> Self {
        Self::ImageError(value)
    }
}

pub type EmojifyResult<T> = Result<T, EmojifyError>;

pub struct Emojify {
    pub cols: u32,
    pub emojis: Vec<RgbaImage>,
}

impl Emojify {
    pub fn rows(&self) -> u32 {
        self.emojis.len() as u32 / self.cols
    }
}

pub fn transform(
    image: &(impl GenericImageView<Pixel = Rgba<u8>> + 'static),
    tile_size: u32,
) -> EmojifyResult<Emojify> {
    let mut emojis = vec![];

    let mut rows = image.height() / tile_size;
    let cols = image.width() / tile_size;

    if rows * cols < 1 || rows * cols > 50 {
        return Err(EmojifyError::TilingExceedsTelegramLimit);
    }

    for row in 0..rows {
        for col in 0..cols {
            let emoji = image
                .try_view(col * tile_size, row * tile_size, tile_size, tile_size)?
                .to_image();
            let emoji = imageops::resize(
                &emoji,
                EMOJI_TILE_SIZE,
                EMOJI_TILE_SIZE,
                imageops::FilterType::Triangle,
            );
            emojis.push(emoji);
        }
    }

    'search: while rows > 0 {
        for col in 0..cols {
            let pixels = emojis[((rows - 1) * cols + col) as usize].pixels();
            if pixels.cloned().any(|Rgba([_, _, _, a])| a > 0) {
                break 'search;
            }
        }
        for _ in 0..cols {
            emojis.remove(((rows - 1) * cols) as usize);
        }
        rows -= 1;
    }

    Ok(Emojify { cols, emojis })
}
