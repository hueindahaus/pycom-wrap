use std::{collections::HashMap, str::Chars};

use crate::lsp::lexer::{
    lex::TokenSpan,
    text_range::TextRange,
    text_size::TextSize,
    token::{StringKind, Token},
};

use super::lexer::lex::{Lexer, LexicalError};

struct Position {
    line: u32,
    character: u32,
}

struct Range {
    start: Position,
    end: Position,
}

struct TextEdit {
    range: Range,
    new_text: String,
}

struct CommentWrapper {
    max_line_length: u64,
}

impl CommentWrapper {
    fn process(&self, source: &str) -> Result<Vec<TextEdit>, LexicalError> {
        let lexer = Lexer::new(source.chars());
        let mut token_groups: HashMap<TextSize, Vec<&TokenSpan>> = HashMap::new();

        let tokens = lexer
            .map(|w| w)
            .collect::<Result<Vec<TokenSpan>, LexicalError>>()?;

        for (idx, token) in tokens.iter().enumerate() {
            if let (Token::Comment(_), ..) = token {
                // If single line comment, go back and check if there are any other single line
                // comments that this could be grouped with.
                if idx == 0 {
                    token_groups.insert(token.1.start, vec![token]);
                    continue;
                }

                let mut has_encountered_nl = !tokens[..idx]
                    .iter()
                    .any(|e| matches!(e, (Token::Comment(_), ..)));

                for (prior_token_idx, prior_token_span) in tokens[..idx].iter().rev().enumerate() {
                    match prior_token_span {
                        (Token::Indent | Token::Dedent, _) if prior_token_idx != 0 => {
                            continue;
                        }
                        (Token::Newline | Token::NonLogicalNewline, _)
                            if !has_encountered_nl && prior_token_idx != 0 =>
                        {
                            has_encountered_nl = true;
                            continue;
                        }
                        (Token::Comment(_), text_range) => {
                            if let Some(group) = token_groups.get_mut(&text_range.start) {
                                group.push(token);
                                break;
                            } else if prior_token_idx != 0 {
                                // this comment has been grouped with an even earlier comment block
                                has_encountered_nl = false;
                                continue;
                            }
                            unreachable!();
                        }
                        _ => {
                            token_groups.insert(token.1.start, vec![token]);
                            break;
                        }
                    };
                }
            } else if let (
                Token::String {
                    kind: StringKind::String,
                    triple_quoted: true,
                    ..
                },
                ..,
            ) = token
            {
                if idx == 0 {
                    token_groups.insert(token.1.start, vec![token]);
                    continue;
                }

                // Make sure triple quoted string is set to be a comment
                for (previous_token_idx, previous_token) in tokens[..idx].iter().rev().enumerate() {
                    match previous_token {
                        (Token::Dedent | Token::Indent, _) if previous_token_idx != 0 => continue,
                        (Token::Newline, text_range) => {
                            token_groups.insert(text_range.start, vec![previous_token]);
                        }
                        _ => break,
                    }
                    if previous_token_idx == 0 {
                        token_groups.insert(previous_token.1.start, vec![previous_token]);
                    }
                }
            }
        }

        let mut text_edits: Vec<TextEdit> = Vec::with_capacity(token_groups.len());

        let text = source.replace('\t', "    ");
        let chars_vec: Vec<char> = text.chars().collect();
        for (start_offset, tokens) in token_groups.iter() {
            let leading = tokens[0];

            let start_char_index = to_char_index(text.chars(), *start_offset);
            let max_comment_length = self.max_line_length
                - chars_vec[..start_char_index]
                    .iter()
                    .rev()
                    .position(|c| matches!(c, '\n' | '\r'))
                    .map(|w| w + 1)
                    .unwrap_or(0) as u64;

            // TODO Fix case where max_comment_length becomes negative or 0

            match leading {
                (Token::Comment(_), TextRange { start, .. }) => {
                    assert!(tokens.iter().all(|e| matches!(e.0, Token::Comment(_))));
                    let mut acc_text_range = TextRange::empty(*start);
                    let mut acc_text = String::from("");
                    for token in tokens {
                        if let (Token::Comment(s), token_text_range) = token {
                            acc_text_range = acc_text_range.cover(*token_text_range);
                            acc_text += s;
                        };
                    }
                }

                (
                    Token::String {
                        kind: StringKind::String,
                        triple_quoted: true,
                        ..
                    },
                    ..,
                ) => {
                    assert!(tokens.iter().all(|e| matches!(
                        e.0,
                        Token::String {
                            kind: StringKind::String,
                            triple_quoted: true,
                            ..
                        }
                    )))
                }

                _ => unreachable!(),
            }
        }

        panic!();
    }
}

fn format_multi_line_comments(mut str: String, max_comment_length: u32) -> String {
    let lines = str.lines();

    for line in lines {}

    panic!()
}

fn format_single_line_comments(mut str: String, max_comment_length: u32) -> String {
    if str.starts_with('#') {
        str = str[1..].to_string();
    }

    str = str.trim().replace('\n', "").replace('\r', "");

    return str;
}

fn to_char_index<T: Iterator<Item = char>>(chars: T, utf_8_offset: TextSize) -> usize {
    let mut utf_8_sum: TextSize = 0.into();
    let mut num_chars: usize = 0;

    for (idx, c) in chars.enumerate() {
        if utf_8_sum >= utf_8_offset {
            return idx;
        }
        num_chars += 1;

        utf_8_sum += TextSize::from(c);
    }

    return num_chars;
}

#[cfg(test)]
mod tests {
    use super::to_char_index;

    #[test]
    fn test_to_char_offset() {
        let text = "a𐐀b𐐀d";

        // println!("{}", chars.to_string());
        // println!("string length: {}", chars.to_string().len());
        // println!("Chars length: {}", chars.chars().count());
        //
        // let mut self_count = 0;
        // let mut utf_8_sum = 0;
        // for c in chars.chars() {
        //     self_count += 1;
        //     utf_8_sum += c.len_utf8();
        // }
        //
        // println!("{}", self_count);
        // println!("utf_8_sum: {}", utf_8_sum);
        let chars = text.chars();

        let expected_char_offset = 2;
        let actual_char_offset = to_char_index(chars.clone(), 3.into());
        assert_eq!(expected_char_offset, actual_char_offset);

        let expected_char_offset = 4;
        let actual_char_offset = to_char_index(chars.clone(), 7.into());
        assert_eq!(expected_char_offset, actual_char_offset);
    }
}
