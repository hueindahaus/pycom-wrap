use std::ops;

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextSize {
    raw: u32,
}

impl TextSize {
    pub fn new(raw: u32) -> TextSize {
        return TextSize { raw };
    }

    pub fn from<T: TextLen>(text: T) -> TextSize {
        return text.text_len();
    }

    pub fn to_u32(&self) -> u32 {
        return self.raw;
    }

    pub fn to_usize(&self) -> usize {
        return self.raw as usize;
    }
}

pub trait TextLen: Copy {
    fn text_len(self) -> TextSize;
}

impl TextLen for &'_ str {
    fn text_len(self) -> TextSize {
        return TextSize {
            raw: self.len().try_into().unwrap(),
        };
    }
}

impl TextLen for &'_ String {
    fn text_len(self) -> TextSize {
        return self.as_str().text_len();
    }
}

impl TextLen for char {
    fn text_len(self) -> TextSize {
        return TextSize {
            raw: self.len_utf8() as u32,
        };
    }
}
