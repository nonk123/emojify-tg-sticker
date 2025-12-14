use std::fmt;

use image::{GenericImageView, ImageBuffer, ImageError, Pixel};

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

pub struct Emojify<P: Pixel> {
    pub cols: u32,
    pub emojis: Vec<ImageBuffer<P, Vec<<P as Pixel>::Subpixel>>>,
}

impl<P: Pixel> Emojify<P> {
    pub fn rows(&self) -> u32 {
        self.emojis.len() as u32 / self.cols
    }
}

pub fn transform<P: Pixel>(
    image: &(impl GenericImageView<Pixel = P> + 'static),
) -> EmojifyResult<Emojify<P>> {
    let mut emojis = vec![];

    let rows = image.height() / EMOJI_SIZE;
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

    Ok(Emojify { cols, emojis })
}
