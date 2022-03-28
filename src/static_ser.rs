use smallvec::SmallVec;
use std::mem::MaybeUninit;

use crate::err;
use crate::Result;

macro_rules! impl_int {
    ( $($i: ty),* ) => {
        $(
            impl StaticSerialize for $i {
                const LEN: usize = std::mem::size_of::<Self>();
                #[inline]
                fn serialize(&self, w: &mut impl std::io::Write) -> Result<()> {
                    w.write_all(&Self::to_be_bytes(*self))?;
                    Ok(())
                }
            }
            impl StaticDeserialize for $i {
                #[inline]
                fn deserialize(w: &mut impl std::io::Read) -> Result<Self> {
                    let mut v = [0u8; std::mem::size_of::<Self>()];
                    w.read_exact(&mut v)?;
                    Ok(Self::from_be_bytes(v))
                }
            }
        )*
    }
}

impl_int!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64, usize);

/// serialize with a static size
pub trait StaticSerialize: Sized {
    /// length of the serialized value
    const LEN: usize;
    /// serialize value
    fn serialize(&self, w: &mut impl std::io::Write) -> Result<()>;
}

/// deserialize with a static size
pub trait StaticDeserialize: Sized + StaticSerialize {
    /// deserialize value
    fn deserialize(w: &mut impl std::io::Read) -> Result<Self>;
}

impl StaticSerialize for bool {
    const LEN: usize = 1;

    fn serialize(&self, w: &mut impl std::io::Write) -> Result<()> {
        let b = [*self as u8];
        w.write_all(&b)?;
        Ok(())
    }
}

impl StaticDeserialize for bool {
    fn deserialize(w: &mut impl std::io::Read) -> Result<Self> {
        let mut buf = [0u8];
        w.read_exact(&mut buf)?;
        Ok(buf[0] == 1)
    }
}

impl StaticSerialize for char {
    const LEN: usize = std::mem::size_of::<char>();

    fn serialize(&self, w: &mut impl std::io::Write) -> Result<()> {
        let b = u32::to_be_bytes(*self as u32);
        w.write_all(&b)?;
        Ok(())
    }
}

impl StaticDeserialize for char {
    fn deserialize(w: &mut impl std::io::Read) -> Result<Self> {
        let mut buf = [0u8; std::mem::size_of::<char>()];
        w.read_exact(&mut buf)?;
        let num = u32::from_be_bytes(buf);
        let c = char::from_u32(num).ok_or(err!(format!("invalid char {num:?}")))?;
        Ok(c)
    }
}

impl<T: StaticSerialize, const N: usize> StaticSerialize for [T; N] {
    const LEN: usize = T::LEN * N;

    fn serialize(&self, w: &mut impl std::io::Write) -> Result<()> {
        for val in self {
            val.serialize(w)?
        }
        Ok(())
    }
}

impl<T: StaticDeserialize, const N: usize> StaticDeserialize for [T; N] {
    fn deserialize(w: &mut impl std::io::Read) -> Result<Self> {
        let mut m = unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() };
        for i in 0..N {
            let n = T::deserialize(w)?;
            m[i].write(n);
        }
        Ok(unsafe { (&m as *const _ as *const [T; N]).read() })
    }
}

impl<T: StaticSerialize> StaticSerialize for Option<T> {
    const LEN: usize = T::LEN + u8::LEN;

    fn serialize(&self, w: &mut impl std::io::Write) -> Result<()> {
        let variant = [matches!(self, &Some(_)) as u8];
        let mut v = SmallVec::<[u8; 8]>::with_capacity(T::LEN);
        match self {
            Some(val) => val.serialize(&mut v)?,
            None => v.resize(T::LEN, 0),
        }
        w.write_all(&variant)?;
        w.write_all(&v)?;
        Ok(())
    }
}

impl<T: StaticDeserialize> StaticDeserialize for Option<T> {
    fn deserialize(w: &mut impl std::io::Read) -> Result<Self> {
        if bool::deserialize(w)? {
            Ok(Some(T::deserialize(w)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: StaticSerialize, E: StaticSerialize> StaticSerialize for std::result::Result<T, E> {
    const LEN: usize = {
        const fn max(a: usize, b: usize) -> usize {
            [a, b][(a < b) as usize]
        }

        max(T::LEN, E::LEN) + u8::LEN
    };

    fn serialize(&self, w: &mut impl std::io::Write) -> Result<()> {
        let variant = [matches!(self, &Ok(_)) as u8];
        let mut v = SmallVec::<[u8; 8]>::with_capacity(Self::LEN - u8::LEN);
        match self {
            Ok(val) => val.serialize(&mut v)?,
            Err(e) => e.serialize(&mut v)?,
        }
        w.write_all(&variant)?;
        w.write_all(&v)?;
        Ok(())
    }
}

impl<T: StaticDeserialize, E: StaticDeserialize> StaticDeserialize for std::result::Result<T, E> {
    fn deserialize(w: &mut impl std::io::Read) -> Result<Self> {
        if bool::deserialize(w)? {
            Ok(Ok(T::deserialize(w)?))
        } else {
            Ok(Err(E::deserialize(w)?))
        }
    }
}

impl StaticSerialize for () {
    const LEN: usize = 0;

    fn serialize(&self, _: &mut impl std::io::Write) -> Result<()> {
        Ok(())
    }
}

impl StaticDeserialize for () {
    fn deserialize(_: &mut impl std::io::Read) -> Result<Self> {
        Ok(())
    }
}

#[allow(unused_macros)] // make rust-analyzer calm down
macro_rules! for_tuples {
    ($($t:tt)*) => {};
}

#[impl_trait_for_tuples::impl_for_tuples(1, 16)]
impl StaticSerialize for Tuple {
    for_tuples!( const LEN: usize = #( Tuple::LEN )+*; );
    fn serialize(&self, w: &mut impl std::io::Write) -> Result<()> {
        for_tuples!( #( Tuple.serialize(w)?; )* );
        Ok(())
    }
}

#[impl_trait_for_tuples::impl_for_tuples(1, 16)]
impl StaticDeserialize for Tuple {
    fn deserialize(w: &mut impl std::io::Read) -> Result<Self> {
        Ok((for_tuples!( #(
            Tuple::deserialize(w)?
        ),*)))
    }
}
