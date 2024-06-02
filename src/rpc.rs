use crate::constants::{self};
use serde_json;

pub fn encode_message<T>(msg: &T) -> String
where
    T: serde::ser::Serialize,
{
    let json = serde_json::to_string(&msg).expect("Could not serialize message");
    return format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
}

pub fn decode_message<'a, T: serde::de::Deserialize<'a>>(msg: &'a [u8]) -> T {
    let content_start_index = msg
        .windows(4)
        .enumerate()
        .find(|(_, w)| matches!(*w, b"\r\n\r\n"))
        .map(|(i, _)| i)
        .expect("Could not find the \\r\\n\\r\\n separator.");
    let content_bytes = &msg[(content_start_index + 4)..];
    let stringified_msg = std::str::from_utf8(&content_bytes)
        .expect("Could not convert content bytes to str literal.");
    println!("{}", stringified_msg);

    let deserialized =
        serde_json::from_str(stringified_msg).expect("Could not deserialize message");
    return deserialized;
}

pub fn split_fn(data: &[u8]) -> Result<(usize, &[u8], bool), String> {
    if !data.starts_with(constants::CONTENT_LENGTH_LABEL.as_bytes()) {
        return Err(format!(
            "Line did not start with {}",
            constants::CONTENT_LENGTH_LABEL
        ));
    }

    return match cut_data(data) {
        Ok((header, content)) => {
            let content_length = std::str::from_utf8(
                header
                    .strip_prefix(constants::CONTENT_LENGTH_LABEL.as_bytes())
                    .unwrap(),
            )
            .unwrap()
            .parse::<usize>()
            .unwrap();

            if content.len() < content_length {
                return Ok((0, &[], false));
            }

            let total_length =
                constants::CONTENT_LENGTH_LABEL.as_bytes().len() + 4 + content_length;

            return Ok((total_length, &data[..total_length], true));
        }
        Err(message) => Err(message),
    };
}

pub fn cut_data(data: &[u8]) -> Result<(&[u8], &[u8]), String> {
    let delimiter_index_option = data
        .windows(4)
        .enumerate()
        .find(|(_, w)| matches!(*w, constants::JSON_RPC_DELIMITER_BYTES))
        .map(|(i, _)| i);

    return match delimiter_index_option {
        Some(delimiter_index) => Ok((&data[..delimiter_index], &data[delimiter_index + 4..])),
        None => Err("Could not cut data".to_string()),
    };
}

#[cfg(test)]
mod test {
    use super::{decode_message, encode_message};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct EncodingExample {
        testing: bool,
    }

    #[test]
    fn test_encode_message() {
        let expected = "Content-Length: 16\r\n\r\n{\"testing\":true}";
        let example = EncodingExample { testing: true };
        let actual = encode_message(&example);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_decode_message() {
        let expected = EncodingExample { testing: true };
        let message = b"Content-Length: 16\r\n\r\n{\"testing\":true}";
        let actual: EncodingExample = decode_message(message);
        assert_eq!(actual, expected);
    }
}
