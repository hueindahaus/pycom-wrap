pub trait ParseExponentStr {
    fn parse_exponent_str(str: &str) -> Result<Self, String>
    where
        Self: Sized;
}

impl ParseExponentStr for i64 {
    fn parse_exponent_str(value: &str) -> Result<Self, String> {
        let (coefficient_part, sign, exponent_part) = exponent_parts(value)?;
        let coefficient = coefficient_part.parse::<i64>().map_err(|e| e.to_string())?;
        let exponent = exponent_part.parse::<u32>().map_err(|e| e.to_string())?;

        if sign {
            return Ok(coefficient * (10 as i64).pow(exponent));
        } else {
            return Ok(coefficient / (10 as i64).pow(exponent));
        }
    }
}

impl ParseExponentStr for f64 {
    fn parse_exponent_str(value: &str) -> Result<Self, String> {
        let (coefficient_part, sign, exponent_part) = exponent_parts(value)?;
        let coefficient = coefficient_part.parse::<f64>().map_err(|e| e.to_string())?;
        let exponent = exponent_part.parse::<f64>().map_err(|e| e.to_string())?;

        if sign {
            return Ok(coefficient * (10 as f64).powf(exponent));
        } else {
            return Ok(coefficient / (10 as f64).powf(exponent));
        }
    }
}
fn exponent_parts(value: &str) -> Result<(&str, bool, &str), String> {
    let iter = value.chars().collect::<Vec<char>>();
    let windows = iter.windows(2);
    let mut sign = true;
    let mut coefficient_part: &str = "";
    let mut exponent_part: &str = "";

    for (idx, window) in windows.enumerate() {
        match window {
            ['e' | 'E', '+'] => {
                coefficient_part = &value[0..idx];
                exponent_part = &value[idx + 2..];
            }
            ['e' | 'E', '-'] => {
                coefficient_part = &value[0..idx];
                exponent_part = &value[idx + 2..];
                sign = false;
            }
            ['e' | 'E', ..] => {
                coefficient_part = &value[0..idx];
                exponent_part = &value[idx + 1..];
            }
            _ => {}
        }
    }

    if coefficient_part.len() == 0 {
        return Err("Coefficient part is missing".to_string());
    }
    if exponent_part.len() == 0 {
        return Err("Exponent part is missing".to_string());
    }

    return Ok((coefficient_part, sign, exponent_part));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cases() {
        assert_eq!(10000, i64::parse_exponent_str("10e3").unwrap());
        assert_eq!(10000.0, f64::parse_exponent_str("10.e3").unwrap());
        assert_eq!(10000.0, f64::parse_exponent_str("10.0e3").unwrap());
    }
}
