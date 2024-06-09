use super::text_size::TextSize;

pub struct TextRange {
    start: TextSize,
    end: TextSize,
}

impl TextRange {
    pub fn new(start: TextSize, end: TextSize) -> TextRange {
        assert!(start <= end);
        return TextRange { start, end };
    }
}
