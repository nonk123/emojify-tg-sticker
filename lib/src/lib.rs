use std::fmt;

use image::{GenericImageView, ImageError, Rgba, RgbaImage};

pub const EMOJI_SIZE: u32 = 100;

#[derive(Debug)]
pub enum EmojifyError {
    ImageError(ImageError),
}

impl std::error::Error for EmojifyError {}

impl fmt::Display for EmojifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImageError(err) => write!(f, "{}", err),
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
) -> EmojifyResult<Emojify> {
    let mut emojis = vec![];

    let mut rows = image.height() / EMOJI_SIZE;
    let cols = image.width() / EMOJI_SIZE;

    for row in 0..rows {
        for col in 0..cols {
            let startx = col * EMOJI_SIZE;
            let starty = row * EMOJI_SIZE;

            let emoji = image
                .try_view(startx, starty, EMOJI_SIZE, EMOJI_SIZE)?
                .to_image();
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
