use core::cmp;

#[derive(Clone)]
pub struct Img {
    pub welcome: image::DynamicImage,
}

impl Img {
    const WELCOME: &'static [u8] = include_bytes!("../assets/img/welcome.png");

    #[inline]
    pub fn new() -> Self {
        Self {
            welcome: image::load_from_memory_with_format(Self::WELCOME, image::ImageFormat::Png).expect("Load welcome img"),
        }
    }
}

#[derive(Clone)]
pub struct Font {
    pub welcome: rusttype::Font<'static>
}

impl Font {
    const WELCOME: &'static [u8] = include_bytes!("../assets/font/welcome.ttf");

    #[inline]
    pub fn new() -> Self {
        Self {
            welcome: rusttype::Font::try_from_bytes(Self::WELCOME).expect("Load welcome font"),
        }
    }
}

#[derive(Clone)]
pub struct Assets {
    pub img: Img,
    pub font: Font,
}

impl Assets {
    #[inline]
    pub fn new() -> Self {
        Self {
            img: Img::new(),
            font: Font::new(),
        }
    }

    pub fn gen_welcome(&self, name: &str) -> Option<image::DynamicImage> {
        const LETTER_SIZE: f32 = 28.0;
        const TEXT_BOX_X_MAX: u32 = 900;
        const TEXT_BOX_X: u32 = 460;
        //Modify Y position accordingly to font
        const TEXT_BOX_Y: u32 = 364;

        //~18px per letter so we need to crop a bit
        //Discord allows up to 32 letters for nickname
        //but smaller font doesn't look so good
        //To fit 32 we'd need ~14px letter
        //TODO: it works well for latin letters
        //but current font may not support all of UTF-8
        const LETTER_PX_X: u32 = 18;

        const TEXT_BOX_X_SIZE: u32 = TEXT_BOX_X_MAX - TEXT_BOX_X;
        const MAX_LETTER_NUM: usize = TEXT_BOX_X_SIZE as usize / LETTER_PX_X as usize;

        let name = name.trim_matches(|ch| self.font.welcome.glyph(ch).id().0 == 0);
        let name = name.replace(|ch| self.font.welcome.glyph(ch).id().0 == 0, " ");
        let name = name.as_str();
        if name.is_empty() {
            return None;
        }

        let mut name_len = 0;
        //Japanese/Chinese full width may take 4 bytes
        //while actual length of character would take space
        //of two latin characters
        for ch in name.chars() {
            name_len += cmp::min(ch.len_utf8(), 2);
        }

        let name = if name_len > MAX_LETTER_NUM {
            //Safeguard against hitting middle of Unicode character
            let mut max = MAX_LETTER_NUM;
            loop {
                if let Some(result) = name.get(..max) {
                    break result
                } else {
                    max -= 1;
                }
            }
        } else {
            name
        };

        let name_len_px = cmp::min(TEXT_BOX_X_SIZE, name_len as u32 * LETTER_PX_X);
        let shift_x = (TEXT_BOX_X_SIZE - name_len_px) / 2;
        let scale = rusttype::Scale { x: LETTER_SIZE * 1.25, y: LETTER_SIZE * 1.25 };

        let mut img = self.img.welcome.clone();
        imageproc::drawing::draw_text_mut(&mut img, image::Rgba([238, 183, 149, 255]),
                                          TEXT_BOX_X + shift_x, TEXT_BOX_Y,
                                          scale, &self.font.welcome, name);

        Some(img)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_font_nihingo() {
        let text = "おしざーおしざー";

        let assets = Assets::new();
        let welcome = assets.gen_welcome(text).expect("Welcome img");
        let mut image_buffer = Vec::new();
        welcome.write_to(&mut image_buffer, image::ImageOutputFormat::Png).expect("Write buffer");
        std::fs::write("test1.png", image_buffer.as_slice()).expect("Write file");
    }

    #[test]
    fn verify_font_eigo() {
        let text = "Chitanda Eru";

        let assets = Assets::new();
        let welcome = assets.gen_welcome(text).expect("Welcome img");
        let mut image_buffer = Vec::new();
        welcome.write_to(&mut image_buffer, image::ImageOutputFormat::Png).expect("Write buffer");
        std::fs::write("test2.png", image_buffer.as_slice()).expect("Write file");
    }

    #[test]
    fn verify_font_emoji() {
        let text = "✦ღGlitter Gal Lilacღ✦";

        let assets = Assets::new();
        let welcome = assets.gen_welcome(text).expect("Welcome img");
        let mut image_buffer = Vec::new();
        welcome.write_to(&mut image_buffer, image::ImageOutputFormat::Png).expect("Write buffer");
        std::fs::write("test3.png", image_buffer.as_slice()).expect("Write file");
    }

}
