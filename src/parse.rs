use crate::error::ParseError;

pub fn read<T>(data: &str, data_type: &str) -> Result<Vec<T>, ParseError>
where
    T: From<u8> + From<u16> + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    if data
        .chars()
        .all(|c| c.is_ascii_hexdigit() || c.is_whitespace())
        && data.split_whitespace().all(|s| {
            s.len() == 2
                && !((s.len() > 1 && s.starts_with("0") && s.chars().all(|c| c.is_ascii_digit()))
                    && (data.contains("e+") || data.contains("e-")))
        })
    {
        return read_hex(data, data_type);
    }
    read_decimal(data)
}

fn read_hex<T>(data: &str, data_type: &str) -> Result<Vec<T>, ParseError>
where
    T: From<u8> + From<u16> + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    let stride = match data_type {
        "uchar" => 1,
        "ushort" | "half" => 2,
        "float" => 4,
        _ => return Err(ParseError::new(&format!("Unknown data type {}", data_type))),
    };

    let bytes = data
        .split_whitespace()
        .map(|s| {
            u8::from_str_radix(s, 16)
                .map_err(|e| ParseError::new(&format!("Failed to decode hex: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    bytes
        .chunks(stride)
        .map(|chunk| match (stride, data_type) {
            (1, "uchar") => Ok(T::from(chunk[0])),
            (2, "ushort") => {
                let arr = <[u8; 2]>::try_from(chunk)
                    .map_err(|_| ParseError::new("Invalid length for u16"))?;
                Ok(T::from(u16::from_be_bytes(arr)))
            }
            (2, "half") => {
                let arr = <[u8; 2]>::try_from(chunk)
                    .map_err(|_| ParseError::new("Invalid length for u16"))?;

                let f = half_to_f32(u16::from_be_bytes(arr));
                f.to_string()
                    .parse::<T>()
                    .map_err(|_| ParseError::new("Failed to convert half to target type"))
            }
            (4, "float") => {
                let arr = <[u8; 4]>::try_from(chunk)
                    .map_err(|_| ParseError::new("Invalid length for f32"))?;
                let f = f32::from_be_bytes(arr);
                f.to_string()
                    .parse::<T>()
                    .map_err(|_| ParseError::new("Failed to convert f32 to target type"))
            }
            _ => unreachable!(),
        })
        .collect()
}

fn read_decimal<T>(data: &str) -> Result<Vec<T>, ParseError>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    data.split_whitespace()
        .map(|s| {
            s.parse::<T>()
                .map_err(|e| ParseError::new(&format!("Failed to decode decimal data: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()
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
