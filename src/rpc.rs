use crate::constants::{self};
use crate::scanner::SplitFnResult;
use serde_json;

pub fn encode_message<T>(msg: &T) -> Result<Vec<u8>, String>
where
    T: serde::ser::Serialize,
{
    let cloned_msg = msg;
    match serde_json::to_string(&cloned_msg) {
        Ok(json) => {
            let json_bytes = json.as_bytes();
            let bytes = [
                constants::CONTENT_LENGTH_LABEL_BYTES,
                &json_bytes.len().to_be_bytes(),
                constants::JSON_RPC_DELIMITER_BYTES,
                json_bytes,
            ]
            .concat();
            return Ok(bytes);
        }
        Err(e) => Err(e.to_string()),
    }
    // return format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
}

pub fn decode_message<'a, T: serde::de::Deserialize<'a>>(msg: &'a [u8]) -> Result<T, String> {
    let content_bytes = match msg
        .windows(4)
        .enumerate()
        .find(|(_, w)| matches!(*w, constants::JSON_RPC_DELIMITER_BYTES))
        .map(|(i, _)| i)
    {
        Some(delimiter_index) => {
            &msg[delimiter_index + constants::JSON_RPC_DELIMITER_BYTES.len()..]
        }
        None => return Err("Could not find delimiter when decoding message".to_string()),
    };
    match serde_json::from_slice(content_bytes) {
        Ok(deserialized) => Ok(deserialized),
        Err(err) => Err(err.to_string()),
    }
}

pub fn split_fn(data: &[u8], start_hint: usize) -> Result<SplitFnResult, String> {
    let start_index = match data[start_hint..]
        .windows(constants::CONTENT_LENGTH_LABEL_BYTES.len())
        .enumerate()
        .find(|(_, w)| matches!(*w, constants::CONTENT_LENGTH_LABEL_BYTES))
        .map(|(i, _)| i + start_hint)
    {
        Some(value) => value,
        None => return Ok(SplitFnResult::Searching),
    };

    let delimiter_index = match data[start_index..]
        .windows(constants::JSON_RPC_DELIMITER_BYTES.len())
        .enumerate()
        .find(|(_, w)| matches!(*w, constants::JSON_RPC_DELIMITER_BYTES))
        .map(|(i, _)| i)
    {
        Some(value) => value,
        None => return Ok(SplitFnResult::SearchingEnd { start: start_index }),
    };

    assert!(start_index + constants::CONTENT_LENGTH_LABEL_BYTES.len() < delimiter_index);

    let content_length_res = match std::str::from_utf8(
        &data[start_hint + constants::CONTENT_LENGTH_LABEL_BYTES.len()..delimiter_index],
    ) {
        Ok(content_length_str) => content_length_str.parse::<usize>(),
        Err(_) => return Err("Could not convert content length bytes to str".to_string()),
    };

    let content_length = match content_length_res {
        Ok(content_length) => content_length,
        Err(_) => return Err("Could not parse content length".to_string()),
    };

    let content_start_index = delimiter_index + constants::JSON_RPC_DELIMITER_BYTES.len();

    if data[content_start_index..].len() < content_length {
        return Ok(SplitFnResult::SearchingEnd { start: start_index });
    }

    return Ok(SplitFnResult::Complete {
        start: start_index,
        end: content_start_index + content_length,
    });
}

pub fn cut_data(data: &[u8]) -> Result<(&[u8], &[u8]), String> {
    let delimiter_index_option = data
        .windows(4)
        .enumerate()
        .find(|(_, w)| matches!(*w, constants::JSON_RPC_DELIMITER_BYTES))
        .map(|(i, _)| i);

    return match delimiter_index_option {
        Some(delimiter_index) => Ok((&data[..delimiter_index], &data[delimiter_index + 4..])),
        None => Err(format!(
            "Could not cut data. Got: {}",
            std::str::from_utf8(data).unwrap()
        )),
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
