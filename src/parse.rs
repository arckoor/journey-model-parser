use crate::error::ParseError;

pub trait FromBytes<const STRIDE: usize>: Sized {
    fn from_bytes(bytes: &[u8; STRIDE]) -> Self;
}

macro_rules! impl_from_bytes {
    ($ty:ty, $stride:literal, $expr:expr) => {
        impl FromBytes<$stride> for $ty {
            fn from_bytes(bytes: &[u8; $stride]) -> Self {
                $expr(*bytes)
            }
        }
    };
}

impl_from_bytes!(u32, 1, |b: [u8; 1]| b[0] as u32);
impl_from_bytes!(u32, 2, |b: [u8; 2]| u16::from_be_bytes(b) as u32);
impl_from_bytes!(u32, 4, |b: [u8; 4]| u32::from_be_bytes(b));
impl_from_bytes!(f32, 2, |b: [u8; 2]| half_to_f32(u16::from_be_bytes(b)));
impl_from_bytes!(f32, 4, |b: [u8; 4]| f32::from_be_bytes(b));

pub fn read<T, const STRIDE: usize>(data: &str) -> Result<Vec<T>, ParseError>
where
    T: FromBytes<STRIDE>,
{
    let bytes = data
        .split_whitespace()
        .map(|s| {
            u8::from_str_radix(s, 16)
                .map_err(|e| ParseError::new(&format!("Failed to decode hex: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if bytes.len() % STRIDE != 0 {
        return Err(ParseError::new("Data length is not a multiple of stride"));
    }

    Ok(bytes
        .chunks_exact(STRIDE)
        .map(|chunk| {
            T::from_bytes(<&[u8; STRIDE]>::try_from(chunk).expect("Chunk must be correct size"))
        })
        .collect())
}

fn half_to_f32(half: u16) -> f32 {
    let sign = (half >> 15) & 0x1;
    let exponent = (half >> 10) & 0x1f;
    let mantissa = half & 0x3ff;

    if exponent == 0 && mantissa == 0 {
        return 0.0;
    }

    let exp = (exponent as i32) - 15 + 127;
    let bits = ((sign as u32) << 31) | ((exp as u32) << 23) | ((mantissa as u32) << 13);

    f32::from_bits(bits)
}
