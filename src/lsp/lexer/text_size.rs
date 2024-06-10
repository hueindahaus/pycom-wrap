use std::{
    iter,
    ops::{Add, AddAssign, Sub, SubAssign},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextSize {
    pub(crate) raw: u32,
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

    pub fn checked_add(&self, rhs: TextSize) -> Option<TextSize> {
        self.raw.checked_add(rhs.raw).map(|raw| TextSize { raw })
    }

    pub fn checked_sub(&self, rhs: TextSize) -> Option<TextSize> {
        self.raw.checked_sub(rhs.raw).map(|raw| TextSize { raw })
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

// Operator overloading
macro_rules! ops {
    (impl $Op:ident for TextSize by fn $f:ident = $op:tt) => {
        impl $Op<TextSize> for TextSize {
            type Output = TextSize;
            fn $f(self, other: TextSize) -> TextSize {
                TextSize { raw: self.raw $op other.raw }
            }
        }
        impl $Op<&TextSize> for TextSize {
            type Output = TextSize;
            fn $f(self, other: &TextSize) -> TextSize {
                self $op *other
            }
        }
        impl<T> $Op<T> for &TextSize
        where
            TextSize: $Op<T, Output=TextSize>,
        {
            type Output = TextSize;
            fn $f(self, other: T) -> TextSize {
                *self $op other
            }
        }
    };
}

ops!(impl Add for TextSize by fn add = +);
ops!(impl Sub for TextSize by fn sub = -);

impl<A> AddAssign<A> for TextSize
where
    TextSize: Add<A, Output = TextSize>,
{
    fn add_assign(&mut self, rhs: A) {
        *self = *self + rhs;
    }
}

impl<S> SubAssign<S> for TextSize
where
    TextSize: Sub<S, Output = TextSize>,
{
    fn sub_assign(&mut self, rhs: S) {
        *self = *self - rhs;
    }
}

impl From<u32> for TextSize {
    fn from(raw: u32) -> Self {
        TextSize::new(raw)
    }
}

impl From<TextSize> for u32 {
    fn from(value: TextSize) -> Self {
        value.to_u32()
    }
}

impl TryFrom<usize> for TextSize {
    type Error = std::num::TryFromIntError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(u32::try_from(value)?.into())
    }
}

impl From<TextSize> for usize {
    fn from(value: TextSize) -> Self {
        value.to_usize()
    }
}

impl<A> iter::Sum<A> for TextSize
where
    TextSize: Add<A, Output = TextSize>,
{
    fn sum<I: Iterator<Item = A>>(iter: I) -> TextSize {
        iter.fold(0.into(), Add::add)
    }
}
