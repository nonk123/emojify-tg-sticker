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

    let mut row = 0;
    while row < rows {
        let mut blank_row = true;
        for col in 0..cols {
            let pixels = emojis[(row * cols + col) as usize].pixels();
            if pixels.cloned().any(|Rgba([_, _, _, a])| a > 0) {
                blank_row = false;
                break;
            }
        }
        if blank_row {
            for _ in 0..cols {
                emojis.remove((row * cols) as usize);
            }
            rows -= 1;
        } else {
            row += 1;
        }
    }

    Ok(Emojify { cols, emojis })
}
