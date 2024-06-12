use super::text_size::TextSize;
use cmp::Ordering;
use std::{
    cmp,
    ops::{Add, AddAssign, Bound, Index, IndexMut, Range, RangeBounds, Sub, SubAssign},
};

#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextRange {
    pub start: TextSize,
    pub end: TextSize,
}

// Associative functions
impl TextRange {
    pub fn new(start: TextSize, end: TextSize) -> TextRange {
        assert!(start <= end);
        return TextRange { start, end };
    }

    pub fn at(offset: TextSize, len: TextSize) -> TextRange {
        return TextRange::new(offset, offset + len);
    }

    pub fn empty(offset: TextSize) -> TextRange {
        return TextRange::new(offset, offset);
    }

    pub fn up_to(end: TextSize) -> TextRange {
        return TextRange::new(0.into(), end);
    }
}

// methods
impl TextRange {
    pub const fn start(self) -> TextSize {
        return self.start;
    }

    pub const fn end(self) -> TextSize {
        return self.end;
    }

    pub const fn is_empty(self) -> bool {
        return self.start().raw == self.end().raw;
    }

    pub const fn len(self) -> TextSize {
        return TextSize {
            raw: self.end().raw - self.start().raw,
        };
    }
}

impl TextRange {
    pub fn contains(self, offset: TextSize) -> bool {
        return self.start() <= offset && offset < self.end();
    }

    pub fn contains_inclusive(self, offset: TextSize) -> bool {
        return self.start() <= offset && offset <= self.end();
    }

    pub fn containts_range(self, other: TextRange) -> bool {
        return self.start() <= other.start() && other.end() <= self.end();
    }

    pub fn get_intersection(self, other: TextRange) -> Option<TextRange> {
        let lo = cmp::max(self.start(), other.start());
        let hi = cmp::min(self.end(), other.end());

        if lo > hi {
            return None;
        }

        return Some(TextRange::new(lo, hi));
    }

    pub fn cover(self, other: TextRange) -> TextRange {
        let lo = cmp::max(self.start(), other.start());
        let hi = cmp::min(self.end(), other.end());

        return TextRange::new(lo, hi);
    }

    pub fn cover_offset(self, offset: TextSize) -> TextRange {
        return self.cover(TextRange::empty(offset));
    }

    pub fn ordering(self, other: TextRange) -> Ordering {
        if self.end() <= other.start() {
            Ordering::Less
        } else if other.end() <= self.start() {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }

    pub fn sub_start(self, amount: TextSize) -> TextRange {
        return TextRange::new(self.start() - amount, self.end());
    }

    pub fn add_start(self, amount: TextSize) -> TextRange {
        return TextRange::new(self.start() + amount, self.end());
    }

    pub fn sub_end(self, amount: TextSize) -> TextRange {
        return TextRange::new(self.start(), self.end() - amount);
    }

    pub fn add_end(self, amount: TextSize) -> TextRange {
        return TextRange::new(self.start(), self.end() + amount);
    }

    pub fn checked_add(self, amount: TextSize) -> Option<TextRange> {
        return Some(TextRange::new(
            self.start().checked_add(amount)?,
            self.end().checked_add(amount)?,
        ));
    }

    pub fn checked_sub(self, amount: TextSize) -> Option<TextRange> {
        return Some(TextRange::new(
            self.start().checked_sub(amount)?,
            self.end.checked_sub(amount)?,
        ));
    }
}

impl RangeBounds<TextSize> for TextRange {
    fn start_bound(&self) -> Bound<&TextSize> {
        Bound::Included(&self.start)
    }

    fn end_bound(&self) -> Bound<&TextSize> {
        Bound::Excluded(&self.end)
    }
}

impl From<Range<TextSize>> for TextRange {
    fn from(r: Range<TextSize>) -> Self {
        TextRange::new(r.start, r.end)
    }
}

impl<T> From<TextRange> for Range<T>
where
    T: From<TextSize>,
{
    fn from(r: TextRange) -> Self {
        r.start().into()..r.end().into()
    }
}

impl Index<TextRange> for str {
    type Output = str;
    fn index(&self, index: TextRange) -> &Self::Output {
        return &self[Range::<usize>::from(index)];
    }
}

impl Index<TextRange> for String {
    type Output = str;
    fn index(&self, index: TextRange) -> &Self::Output {
        return &self[Range::<usize>::from(index)];
    }
}

impl IndexMut<TextRange> for str {
    fn index_mut(&mut self, index: TextRange) -> &mut Self::Output {
        return &mut self[Range::<usize>::from(index)];
    }
}

impl IndexMut<TextRange> for String {
    fn index_mut(&mut self, index: TextRange) -> &mut Self::Output {
        return &mut self[Range::<usize>::from(index)];
    }
}

macro_rules! ops {
    (impl $Op:ident for TextRange by fn $f:ident = $op:tt) => {
        impl $Op<&TextSize> for TextRange {
            type Output = TextRange;
            #[inline]
            fn $f(self, other: &TextSize) -> TextRange {
                self $op *other
            }
        }
        impl<T> $Op<T> for &TextRange
        where
            TextRange: $Op<T, Output=TextRange>,
        {
            type Output = TextRange;
            #[inline]
            fn $f(self, other: T) -> TextRange {
                *self $op other
            }
        }
    };
}

impl Add<TextSize> for TextRange {
    type Output = TextRange;
    fn add(self, offset: TextSize) -> TextRange {
        self.checked_add(offset)
            .expect("TextRange +offset overflowed")
    }
}

impl Sub<TextSize> for TextRange {
    type Output = TextRange;
    fn sub(self, offset: TextSize) -> TextRange {
        self.checked_sub(offset)
            .expect("TextRange -offset overflowed")
    }
}

ops!(impl Add for TextRange by fn add = +);
ops!(impl Sub for TextRange by fn sub = -);

impl<A> AddAssign<A> for TextRange
where
    TextRange: Add<A, Output = TextRange>,
{
    fn add_assign(&mut self, rhs: A) {
        *self = *self + rhs;
    }
}

impl<S> SubAssign<S> for TextRange
where
    TextRange: Sub<S, Output = TextRange>,
{
    fn sub_assign(&mut self, rhs: S) {
        *self = *self - rhs;
    }
}
