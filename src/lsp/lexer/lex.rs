use super::{
    string_parsing::ParseExponentStr,
    text_range::TextRange,
    text_size::TextSize,
    token::{StringKind, Token},
};
use num_bigint::BigInt;
use num_traits::Num;
use serde_json::error;
use std::{cmp::Ordering, panic::Location};
use unic_emoji_char::is_emoji_presentation;
use unic_ucd_ident::{is_xid_continue, is_xid_start};

pub type TokenSpan = (Token, TextRange);
pub type LexResult = Result<TokenSpan, LexicalError>;

#[derive(Debug, PartialEq)]
pub enum LexicalErrorType {
    StringError,
    /// Decoding of a unicode escape sequence in a string literal failed.
    UnicodeError,
    /// The nesting of brackets/braces/parentheses is not balanced.
    NestingError,
    /// The indentation is not consistent.
    IndentationError,
    /// Inconsistent use of tabs and spaces.
    TabError,
    /// Encountered a tab after a space.
    TabsAfterSpaces,
    /// A non-default argument follows a default argument.
    DefaultArgumentError,
    /// A duplicate argument was found in a function definition.
    DuplicateArgumentError(String),
    /// A positional argument follows a keyword argument.
    PositionalArgumentError,
    /// An iterable argument unpacking `*args` follows keyword argument unpacking `**kwargs`.
    UnpackedArgumentError,
    /// A keyword argument was repeated.
    DuplicateKeywordArgumentError(String),
    /// An unrecognized token was encountered.
    UnrecognizedToken {
        tok: char,
    },
    /// An f-string error containing the [`FStringErrorType`].
    FStringError, //(FStringErrorType),
    /// An unexpected character was encountered after a line continuation.
    LineContinuationError,
    /// An unexpected end of file was encountered.
    Eof,
    /// An unexpected error occurred.
    OtherError(String),
}

#[derive(Debug, PartialEq)]
pub struct LexicalError {
    pub error: LexicalErrorType,
    pub location: TextSize,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
struct IndentationLevel {
    tabs: u32,
    spaces: u32,
}

impl IndentationLevel {
    fn reset(&mut self) {
        self.tabs = 0;
        self.spaces = 0;
    }
    fn compare_strict(
        &self,
        other: &IndentationLevel,
        location: TextSize,
    ) -> Result<Ordering, LexicalError> {
        return match self.tabs.cmp(&other.tabs) {
            Ordering::Less => {
                if self.spaces <= other.spaces {
                    return Ok(Ordering::Less);
                }
                Err(LexicalError {
                    error: LexicalErrorType::TabError,
                    location,
                })
            }

            Ordering::Greater => {
                if self.spaces >= other.spaces {
                    return Ok(Ordering::Greater);
                }
                Err(LexicalError {
                    error: LexicalErrorType::TabError,
                    location,
                })
            }

            Ordering::Equal => Ok(self.spaces.cmp(&other.spaces)),
        };
    }
}

#[derive(Debug)]
struct Indentations {
    indent_stack: Vec<IndentationLevel>,
}

impl Indentations {
    fn is_empty(&self) -> bool {
        return self.indent_stack.len() == 1;
    }

    fn push(&mut self, indent: IndentationLevel) {
        self.indent_stack.push(indent)
    }

    fn pop(&mut self) -> Option<IndentationLevel> {
        if self.is_empty() {
            return None;
        }

        return self.indent_stack.pop();
    }

    fn current(&self) -> &IndentationLevel {
        self.indent_stack
            .last()
            .expect("Indentation must have at least one level")
    }
}

impl Default for Indentations {
    fn default() -> Self {
        Self {
            indent_stack: vec![IndentationLevel::default()],
        }
    }
}

pub struct CharReader<T, const N: usize>
where
    T: Iterator<Item = char>,
{
    source: T,
    window: [Option<char>; N],
    cursor: TextSize,
}

impl<T, const N: usize> CharReader<T, N>
where
    T: Iterator<Item = char>,
{
    fn new(source: T) -> Self {
        let mut char_reader = Self {
            source,
            window: [None; N],
            cursor: TextSize::default(),
        };

        // Fill window
        for _ in 0..N {
            char_reader.next();
        }

        return char_reader;
    }
}

impl<T, const N: usize> Iterator for CharReader<T, N>
where
    T: Iterator<Item = char>,
{
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.window.rotate_left(1);
        let next = self.source.next();
        *self.window.last_mut().expect("Will always be populated") = next;
        if let Some(c) = next {
            self.cursor += TextSize::from(c)
        }
        return self.window[0];
    }
}

pub struct Lexer<T>
where
    T: Iterator<Item = char>,
{
    char_reader: CharReader<T, 3>,
    at_begin_of_line: bool,
    nesting: usize,
    indentations: Indentations,
    queue: Vec<TokenSpan>,
}

impl<T> Lexer<T>
where
    T: Iterator<Item = char>,
{
    pub fn new(input: T) -> Self {
        let mut lexer = Lexer {
            char_reader: CharReader::new(input),
            at_begin_of_line: true,
            nesting: 0,
            indentations: Indentations::default(),
            queue: Vec::with_capacity(5),
        };

        if let Some('\u{feff}') = lexer.char_reader.window[0] {
            lexer.next_char();
        }

        return lexer;
    }
}

impl<T> Lexer<T>
where
    T: Iterator<Item = char>,
{
    pub fn current_char(&mut self) -> Option<char> {
        return self.window()[0];
    }

    pub fn next_char(&mut self) -> Option<char> {
        let char = self.char_reader.next();

        return match self.window()[..2] {
            [Some('\r'), Some('\n')] => self.char_reader.next(),
            [Some(_), ..] => char,
            _ => None,
        };
    }

    pub fn jump_forward_n_chars(&mut self, num: u32) -> TextSize {
        for _ in 0..num {
            self.next_char();
        }

        return self.char_cursor();
    }

    pub fn char_cursor(&self) -> TextSize {
        return self.char_reader.cursor;
    }

    pub fn window(&self) -> &[Option<char>; 3] {
        return &self.char_reader.window;
    }

    pub fn lex_identifier_or_keyword(&mut self) -> LexResult {
        let start_pos = self.char_cursor();
        let mut name = String::with_capacity(8);

        while let [Some(c1), Some(c2)] = self.window()[..2] {
            name.push(c1);
            self.next_char();
            if !is_identifier_or_keyword_continuation(c2) {
                break;
            }
        }

        let end_pos = self.char_cursor();

        if let Some(token) = Token::try_get_keyword(&name) {
            return Ok((token.clone(), TextRange::new(start_pos, end_pos)));
        }
        return Ok((Token::Name { name }, TextRange::new(start_pos, end_pos)));
    }

    pub fn try_lex_tagged_string(&mut self) -> Option<LexResult> {
        // detect potential string like rb'' r'' f'' u'' r''
        return match self.window()[..3] {
            [Some(c), Some('"' | '\''), ..] => match StringKind::try_from(c) {
                Ok(kind) => Some(self.lex_string(kind)),
                Err(msg) => Some(Err(LexicalError {
                    error: LexicalErrorType::OtherError(msg),
                    location: self.char_cursor(),
                })),
            },
            [Some(c1), Some(c2), Some('"' | '\'')] => match StringKind::try_from([c1, c2]) {
                Ok(kind) => Some(self.lex_string(kind)),
                Err(msg) => Some(Err(LexicalError {
                    error: LexicalErrorType::OtherError(msg),
                    location: self.char_cursor(),
                })),
            },
            _ => None,
        };
    }

    pub fn lex_string(&mut self, kind: StringKind) -> LexResult {
        let start_pos = self.char_cursor();

        self.jump_forward_n_chars(kind.prefix_len().into());

        let quote_char = self.window()[0].expect("Quote character is expected!");

        let mut string_content = String::with_capacity(5);

        let is_triple_quoted = if [Some(quote_char); 3] == self.window()[..3] {
            self.jump_forward_n_chars(3);
            true
        } else {
            self.jump_forward_n_chars(1);
            false
        };

        loop {
            match self.window()[0] {
                Some(c) => {
                    if c == '\\' {
                        if let Some(next_c) = self.next_char() {
                            string_content.push('\\');
                            string_content.push(next_c);
                            continue;
                        }
                    }

                    if c == '\n' && !is_triple_quoted {
                        return Err(LexicalError {
                            error: LexicalErrorType::OtherError(
                                "EOL while scanning string literal".to_owned(),
                            ),
                            location: self.char_cursor(),
                        });
                    }

                    if c == quote_char {
                        if is_triple_quoted {
                            self.jump_forward_n_chars(3);
                            break;
                        } else {
                            self.jump_forward_n_chars(1);
                            break;
                        }
                    }
                    string_content.push(c);
                }
                None => {
                    return Err(LexicalError {
                        error: if is_triple_quoted {
                            LexicalErrorType::Eof
                        } else {
                            LexicalErrorType::StringError
                        },
                        location: self.char_cursor(),
                    })
                }
            }
        }
        let end_pos = self.char_cursor();
        let token = Token::String {
            value: string_content,
            kind,
            triple_quoted: is_triple_quoted,
        };
        Ok((token, TextRange::new(start_pos, end_pos)))
    }

    pub fn lex_next(&mut self) -> LexResult {
        return match self.window()[..3] {
            [Some('0'..='9'), ..] => Ok(self.lex_number()?),
            [Some('#'), ..] => Ok(self.lex_single_line_comment()?),
            [Some('"' | '\''), ..] => Ok(self.lex_string(StringKind::String)?),
            [Some('='), Some('='), ..] => Ok((
                Token::EqEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('='), ..] => Ok((
                Token::Equal,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),

            [Some('+'), Some('='), ..] => Ok((
                Token::PlusEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('+'), ..] => Ok((
                Token::Plus,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('*'), Some('*'), Some('=')] => Ok((
                Token::DoubleStarEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(3)),
            )),
            [Some('*'), Some('*'), ..] => Ok((
                Token::DoubleStar,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('*'), Some('='), ..] => Ok((
                Token::StarEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('*'), ..] => Ok((
                Token::Star,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('/'), Some('/'), Some('=')] => Ok((
                Token::DoubleSlashEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(3)),
            )),
            [Some('/'), Some('/'), ..] => Ok((
                Token::DoubleSlash,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('/'), Some('='), ..] => Ok((
                Token::SlashEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('/'), ..] => Ok((
                Token::Slash,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('%'), Some('='), ..] => Ok((
                Token::PercentEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('%'), ..] => Ok((
                Token::Percent,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('|'), Some('='), ..] => Ok((
                Token::VbarEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('|'), ..] => Ok((
                Token::Vbar,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('^'), Some('='), ..] => Ok((
                Token::CircumflexEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('^'), ..] => Ok((
                Token::CircumFlex,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('&'), Some('='), ..] => Ok((
                Token::AmperEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('&'), ..] => Ok((
                Token::Amper,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('-'), Some('='), ..] => Ok((
                Token::MinusEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('-'), Some('>'), ..] => Ok((
                Token::Rarrow,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('-'), ..] => Ok((
                Token::Minus,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('@'), Some('='), ..] => Ok((
                Token::AtEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('@'), ..] => Ok((
                Token::At,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('!'), Some('='), ..] => Ok((
                Token::NotEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('!'), ..] => Err(LexicalError {
                error: LexicalErrorType::UnrecognizedToken { tok: '!' },
                location: self.char_cursor(),
            }),
            [Some('~'), ..] => Ok((
                Token::Tilde,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('('), ..] => {
                self.nesting += 1;
                Ok((
                    Token::Lpar,
                    TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
                ))
            }
            [Some(')'), ..] => {
                if self.nesting == 0 {
                    return Err(LexicalError {
                        error: LexicalErrorType::NestingError,
                        location: self.char_cursor(),
                    });
                }
                self.nesting -= 1;
                Ok((
                    Token::Rpar,
                    TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
                ))
            }
            [Some('['), ..] => {
                self.nesting += 1;
                Ok((
                    Token::Lsqb,
                    TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
                ))
            }
            [Some(']'), ..] => {
                if self.nesting == 0 {
                    return Err(LexicalError {
                        error: LexicalErrorType::NestingError,
                        location: self.char_cursor(),
                    });
                }
                self.nesting -= 1;
                Ok((
                    Token::Rsqb,
                    TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
                ))
            }
            [Some('{'), ..] => {
                self.nesting += 1;
                Ok((
                    Token::Lbrace,
                    TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
                ))
            }
            [Some('}'), ..] => {
                if self.nesting == 0 {
                    return Err(LexicalError {
                        error: LexicalErrorType::NestingError,
                        location: self.char_cursor(),
                    });
                }
                self.nesting -= 1;
                Ok((
                    Token::Rbrace,
                    TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
                ))
            }
            [Some(':'), Some('='), ..] => Ok((
                Token::ColonEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some(':'), ..] => Ok((
                Token::Colon,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some(';'), ..] => Ok((
                Token::Semi,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('<'), Some('<'), Some('=')] => Ok((
                Token::LeftShiftEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(3)),
            )),
            [Some('<'), Some('<'), ..] => Ok((
                Token::LeftShift,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('<'), Some('='), ..] => Ok((
                Token::LessEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('<'), ..] => Ok((
                Token::Less,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),

            [Some('>'), Some('>'), Some('=')] => Ok((
                Token::RightShiftEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(3)),
            )),
            [Some('>'), Some('>'), ..] => Ok((
                Token::RightShift,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('>'), Some('='), ..] => Ok((
                Token::GreaterEqual,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(2)),
            )),
            [Some('>'), ..] => Ok((
                Token::Greater,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some(','), ..] => Ok((
                Token::Comma,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('.'), Some('.'), Some('.')] => Ok((
                Token::Ellipsis,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(3)),
            )),
            [Some('.'), ..] => Ok((
                Token::Dot,
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some('\n' | '\r'), ..] => {
                if self.nesting == 0 {
                    self.at_begin_of_line = true;
                    Ok((
                        Token::Newline,
                        TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
                    ))
                } else {
                    Ok((
                        Token::NonLogicalNewline,
                        TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
                    ))
                }
            }
            [Some(' ' | '\t' | '\x0c'), ..] => {
                let start_pos = self.char_cursor();
                while let Some(' ' | '\t' | '\x0c') = self.window()[0] {
                    self.next_char();
                }
                let end_pos = self.char_cursor();
                Ok((Token::WhiteSpace, TextRange::new(start_pos, end_pos)))
            }
            [Some('\\'), Some('\n' | '\r'), ..] => Err(LexicalError {
                error: LexicalErrorType::LineContinuationError,
                location: self.char_cursor(),
            }),
            [Some('\\'), None, ..] => Err(LexicalError {
                error: LexicalErrorType::Eof,
                location: self.char_cursor(),
            }),
            [Some(c), ..] if is_emoji_presentation(c) => Ok((
                Token::Name {
                    name: c.to_string(),
                },
                TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
            )),
            [Some(c), ..] => Err(LexicalError {
                error: LexicalErrorType::UnrecognizedToken { tok: c },
                location: self.char_cursor(),
            }),
            _ => unreachable!("Unexpected character flow"),
        };
    }

    pub fn lex_single_line_comment(&mut self) -> LexResult {
        assert!(self.window()[0].unwrap() == '#');
        let start_pos = self.char_cursor();
        self.jump_forward_n_chars(1);
        let mut value = String::new();
        loop {
            match self.window()[0] {
                Some('\n' | '\r') | None => {
                    let end_pos = self.char_cursor();
                    return Ok((Token::Comment(value), TextRange::new(start_pos, end_pos)));
                }

                Some(c) => {
                    value.push(c);
                    self.jump_forward_n_chars(1);
                }
            }
        }
    }

    pub fn lex_number(&mut self) -> LexResult {
        match self.window()[..2] {
            [Some('0'), Some('x' | 'X')] => self.lex_number_radix(16),
            [Some('0'), Some('o' | 'O')] => self.lex_number_radix(8),
            [Some('0'), Some('b' | 'B')] => self.lex_number_radix(2),
            _ => {
                let start_pos = self.char_cursor();

                let is_start_zero = self.window()[0] == Some('0');
                let mut is_float = false;
                let mut is_scientific_notation = false;

                let mut value_text = self.radix_run(10)?;

                // Handle float
                match self.window()[..2] {
                    [Some('.'), Some('_')] => {
                        return Err(LexicalError {
                            error: LexicalErrorType::OtherError("Invalid underscore".to_owned()),
                            location: self.char_cursor(),
                        })
                    }
                    [Some('.'), Some(c)] if is_digit_of_radix(c, 10) => {
                        is_float = true;
                        value_text.push(self.current_char().unwrap());
                        self.jump_forward_n_chars(1);
                        value_text.push_str(&self.radix_run(10)?)
                    }
                    [Some('.'), ..] => {
                        is_float = true;
                        value_text.push(self.current_char().unwrap());
                        self.jump_forward_n_chars(1);
                    }
                    _ => {}
                };

                // Handle exponent
                match self.window()[..2] {
                    [Some('e' | 'E'), None] => {
                        return Err(LexicalError {
                            error: LexicalErrorType::OtherError("Invalid underscore".to_owned()),
                            location: self.char_cursor(),
                        });
                    }
                    [Some('e' | 'E'), Some('+' | '-')] => {
                        is_float = true;
                        is_scientific_notation = true;
                        value_text.push(self.current_char().unwrap());
                        let bar = self.next_char().unwrap().to_ascii_lowercase();
                        value_text.push(bar);
                        self.jump_forward_n_chars(1);

                        match self.window()[0] {
                            None => return Err(LexicalError {
                                error: LexicalErrorType::OtherError(
                                    "exponential numeric literal must be followed by an integer"
                                        .to_owned(),
                                ),
                                location: self.char_cursor(),
                            }),
                            Some(c) => {
                                if !is_digit_of_radix(c, 10) {
                                    return Err(LexicalError{
                                        error: LexicalErrorType::OtherError("exponential numeric literal must be followed by an integer".to_owned()),
                                        location: self.char_cursor()
                                    });
                                }
                                value_text.push_str(&self.radix_run(10)?);
                            }
                        }
                    }
                    [Some('e' | 'E'), Some(c)] => {
                        is_float = true;
                        is_scientific_notation = true;
                        if !is_digit_of_radix(c, 10) {
                            return Err(LexicalError {
                                error: LexicalErrorType::OtherError(
                                    "exponential numeric literal must be followed by an integer"
                                        .to_owned(),
                                ),
                                location: self.char_cursor() + TextSize::new(2),
                            });
                        }
                        let e_char = self.current_char().unwrap().to_ascii_lowercase();
                        value_text.push(e_char);
                        self.jump_forward_n_chars(1);
                        value_text.push_str(&self.radix_run(10)?);
                    }

                    _ => {}
                }

                if let Some('j' | 'J') = self.window()[0] {
                    let imag =
                        f64::from_str_radix(&value_text, 10).map_err(|err| LexicalError {
                            error: LexicalErrorType::OtherError(format!(
                                "Could not parse float: {}",
                                err.to_string()
                            )),
                            location: self.char_cursor(),
                        })?;
                    return Ok((
                        Token::Complex { real: 0.0, imag },
                        TextRange::new(start_pos, self.jump_forward_n_chars(1)),
                    ));
                }
                if is_float {
                    let value = match is_scientific_notation {
                        false => {
                            f64::from_str_radix(&value_text, 10).map_err(|err| LexicalError {
                                error: LexicalErrorType::OtherError(format!(
                                    "Could not parse float: {}",
                                    err.to_string()
                                )),
                                location: self.char_cursor(),
                            })?
                        }

                        true => f64::parse_exponent_str(&value_text).map_err(|e| LexicalError {
                            error: LexicalErrorType::OtherError(format!(
                                "Could not parse float {}",
                                e
                            )),
                            location: self.char_cursor(),
                        })?,
                    };
                    return Ok((
                        Token::Float { value },
                        TextRange::new(start_pos, self.char_cursor()),
                    ));
                }

                // If we reach here, we have an integer
                if is_start_zero && value_text.len() > 1 {
                    return Err(LexicalError {
                        error: LexicalErrorType::OtherError(
                            "An integer can't have a leading 0".to_owned(),
                        ),
                        location: self.char_cursor(),
                    });
                }

                let value = i64::from_str_radix(&value_text, 10).map_err(|err| LexicalError {
                    error: LexicalErrorType::OtherError(format!(
                        "Could not parse integer: {}",
                        err.to_string()
                    )),
                    location: self.char_cursor(),
                })?;
                return Ok((
                    Token::Int { value },
                    TextRange::new(start_pos, self.char_cursor()),
                ));
            }
        }
    }

    fn lex_number_radix(&mut self, radix: u32) -> LexResult {
        let start_pos = self.char_cursor();
        // Jump over Ox or Oo or Ob
        self.jump_forward_n_chars(2);
        let value_text = self.radix_run(radix)?;
        let value = i64::from_str_radix(&value_text, radix).map_err(|err| LexicalError {
            error: LexicalErrorType::OtherError(err.to_string()),
            location: self.char_cursor(),
        })?;

        return Ok((
            Token::Int { value },
            TextRange::new(start_pos, self.char_cursor()),
        ));
    }

    fn radix_run(&mut self, radix: u32) -> Result<String, LexicalError> {
        let mut value_text = String::new();

        loop {
            match self.window()[..2] {
                [Some(c1), Some(c2)] if is_digit_of_radix(c1, radix) => {
                    value_text.push(c1);
                    self.jump_forward_n_chars(1);

                    if !is_digit_of_radix(c2, radix) && c2 != '_' {
                        break;
                    }
                }

                [Some(c1), None] if is_digit_of_radix(c1, radix) => {
                    value_text.push(c1);
                    self.jump_forward_n_chars(1);
                    break;
                }

                [Some('_'), Some(c2)] => {
                    if !is_digit_of_radix(c2, radix) {
                        return Err(LexicalError {
                            error: LexicalErrorType::OtherError(
                                "Numeric can't end with _".to_owned(),
                            ),
                            location: self.char_cursor(),
                        });
                    }
                    self.jump_forward_n_chars(1);
                }

                [Some('_'), None] => {
                    return Err(LexicalError {
                        error: LexicalErrorType::OtherError("Numeric can't end with _".to_owned()),
                        location: self.char_cursor(),
                    })
                }

                _ => break,
            }
        }

        return Ok(value_text);
    }

    pub fn populate_results_queue(&mut self) -> Result<(), LexicalError> {
        match self.window()[0] {
            Some(c) if is_identifier_or_keywords_start(c) => {
                if let Some(token) = self.try_lex_tagged_string() {
                    self.queue.push(token?);
                } else {
                    let token = self.lex_identifier_or_keyword()?;
                    self.queue.push(token);
                }

                Ok(())
            }
            Some(_c) => {
                let token = self.lex_next()?;

                // Just ignore whitespace
                if token.0 != Token::WhiteSpace {
                    self.queue.push(token);
                }

                Ok(())
            }
            // End of file
            None => {
                // Return Error if nesting is not exhausted at EoF
                if self.nesting > 0 {
                    return Err(LexicalError {
                        error: LexicalErrorType::Eof,
                        location: self.char_cursor(),
                    });
                }

                // Next, insert a trailing newline, if required.
                if !self.at_begin_of_line {
                    self.at_begin_of_line = true;
                    self.queue
                        .push((Token::Newline, TextRange::empty(self.char_cursor())));
                }

                // Next, flush the indentation stack to zero.
                while !self.indentations.is_empty() {
                    self.indentations.pop();
                    self.queue
                        .push((Token::Dedent, TextRange::empty(self.char_cursor())));
                }
                self.queue
                    .push((Token::EndOfFile, TextRange::empty(self.char_cursor())));
                Ok(())
            }
        }
    }

    fn handle_indentations(&mut self) -> Result<(), LexicalError> {
        let mut new_indentation_level = IndentationLevel::default();

        loop {
            match self.window()[0] {
                Some(' ') => {
                    self.next_char();
                    new_indentation_level.spaces += 1;
                }
                Some('\t') => {
                    if new_indentation_level.spaces != 0 {
                        return Err(LexicalError {
                            error: LexicalErrorType::TabsAfterSpaces,
                            location: self.char_cursor(),
                        });
                    }
                    self.next_char();
                    new_indentation_level.tabs += 1;
                }
                Some('#') => {
                    self.lex_single_line_comment();
                    new_indentation_level.reset();
                }
                Some('\x0c') => {
                    self.next_char();
                    new_indentation_level.reset();
                }
                Some('\n' | '\r') => {
                    new_indentation_level.reset();
                    let spanned = (
                        Token::NonLogicalNewline,
                        TextRange::new(self.char_cursor(), self.jump_forward_n_chars(1)),
                    );
                    self.queue.push(spanned)
                }
                None => {
                    new_indentation_level.reset();
                    break;
                }
                _ => {
                    self.at_begin_of_line = false;
                    break;
                }
            }
        }

        let previous_indentation = self.indentations.current();
        let ordering =
            new_indentation_level.compare_strict(previous_indentation, self.char_cursor())?;
        match ordering {
            Ordering::Equal => {
                // Do nothing
            }
            Ordering::Greater => {
                self.indentations.push(new_indentation_level);
                self.queue.push((
                    Token::Indent,
                    TextRange::new(
                        self.char_cursor()
                            - TextSize::new(new_indentation_level.spaces)
                            - TextSize::new(new_indentation_level.tabs),
                        self.char_cursor(),
                    ),
                ));
            }
            Ordering::Less => {
                // One or more dedentations
                // Pop off other levels until equal indentation
                loop {
                    let previous_indentation = self.indentations.current();
                    let ordering = new_indentation_level
                        .compare_strict(previous_indentation, self.char_cursor())?;
                    match ordering {
                        Ordering::Less => {
                            self.indentations.pop();
                            self.queue
                                .push((Token::Dedent, TextRange::empty(self.char_cursor())));
                        }
                        Ordering::Equal => {
                            // Arrived at proper indentation level
                            break;
                        }
                        Ordering::Greater => {
                            return Err(LexicalError {
                                error: LexicalErrorType::IndentationError,
                                location: self.char_cursor(),
                            });
                        }
                    }
                }
            }
        };

        return Ok(());
    }

    fn inner_next(&mut self) -> LexResult {
        while self.queue.is_empty() {
            if self.at_begin_of_line {
                self.handle_indentations()?;
            }
            self.populate_results_queue()?
        }

        return Ok(self.queue.remove(0));
    }
}

impl<T> Iterator for Lexer<T>
where
    T: Iterator<Item = char>,
{
    type Item = LexResult;
    fn next(&mut self) -> Option<Self::Item> {
        let token = self.inner_next();

        match token {
            Ok((Token::EndOfFile, _)) => None,
            r => Some(r),
        }
    }
}

pub fn is_identifier_or_keywords_start(c: char) -> bool {
    // Checks if the character c is a valid starting character as described
    // in https://docs.python.org/3/reference/lexical_analysis.html#identifiers
    return match c {
        'a'..='z' | 'A'..='Z' | '_' => true,
        _ => is_xid_start(c),
    };
}

pub fn is_identifier_or_keyword_continuation(c: char) -> bool {
    // Checks if the character c is a valid continuation character as described
    // in https://docs.python.org/3/reference/lexical_analysis.html#identifiers
    return match c {
        'a'..='z' | 'A'..='Z' | '_' | '0'..='9' => true,
        _ => is_xid_continue(c),
    };
}

pub fn is_digit_of_radix(c: char, radix: u32) -> bool {
    match radix {
        2 => matches!(c, '0'..='1'),
        8 => matches!(c, '0'..='8'),
        10 => matches!(c, '0'..='9'),
        16 => matches!(c, '0'..='9' | 'a'..='f' | 'A'..='F'),
        other => unimplemented!("Radix not implemented {}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOWS_EOL: &str = "\r\n";
    const MAC_EOL: &str = "\r";
    const UNIX_EOL: &str = "\n";

    pub fn lex_source(source: &str) -> Vec<Token> {
        let lexer = Lexer::new(source.chars());
        lexer.map(|x| x.unwrap().0).collect()
    }

    fn str_tok(s: &str) -> Token {
        Token::String {
            value: s.to_owned(),
            kind: StringKind::String,
            triple_quoted: false,
        }
    }

    fn raw_str_tok(s: &str) -> Token {
        Token::String {
            value: s.to_owned(),
            kind: StringKind::RawString,
            triple_quoted: false,
        }
    }

    #[test]
    fn test_numbers() {
        let source = "0x2f 0o12 0b1101 0 123 123_45_67_890 0.2 1e+2 2.1e3 2j 2.2j";
        let tokens = lex_source(source);
        for token in tokens.iter() {
            println!("{}", token.to_string());
        }
        assert_eq!(
            tokens,
            vec![
                Token::Int { value: (47) },
                Token::Int { value: (10) },
                Token::Int { value: (13) },
                Token::Int { value: (0) },
                Token::Int { value: (123) },
                Token::Int {
                    value: (1234567890)
                },
                Token::Float { value: 0.2 },
                Token::Float { value: 100.0 },
                Token::Float { value: 2100.0 },
                Token::Complex {
                    real: 0.0,
                    imag: 2.0,
                },
                Token::Complex {
                    real: 0.0,
                    imag: 2.2,
                },
                Token::Newline,
            ]
        );
    }

    macro_rules! test_line_comment {
        ($($name:ident: $eol:expr,)*) => {
            $(
            #[test]
            #[cfg(feature = "full-lexer")]
            fn $name() {
                let source = format!(r"99232  # {}", $eol);
                let tokens = lex_source(&source);
                assert_eq!(tokens, vec![Token::Int { value: 99232 }, Token::Comment(format!("# {}", $eol)), Token::Newline]);
            }
            )*
        }
    }

    test_line_comment! {
        test_line_comment_long: " foo",
        test_line_comment_whitespace: "  ",
        test_line_comment_single_whitespace: " ",
        test_line_comment_empty: "",
    }

    macro_rules! test_comment_until_eol {
        ($($name:ident: $eol:expr,)*) => {
            $(
            #[test]
            #[cfg(feature = "full-lexer")]
            fn $name() {
                let source = format!("123  # Foo{}456", $eol);
                let tokens = lex_source(&source);
                assert_eq!(
                    tokens,
                    vec![
                        Token::Int { value: 123 },
                        Token::Comment("# Foo".to_string()),
                        Token::Newline,
                        Token::Int { value: 456 },
                        Token::Newline,
                    ]
                )
            }
            )*
        }
    }

    test_comment_until_eol! {
        test_comment_until_windows_eol: WINDOWS_EOL,
        test_comment_until_mac_eol: MAC_EOL,
        test_comment_until_unix_eol: UNIX_EOL,
    }

    // #[test]
    // fn test_assignment() {
    //     let source = r"a_variable = 99 + 2-0";
    //     let tokens = lex_source(source);
    //
    //     assert_eq!(
    //         tokens,
    //         vec![
    //             Token::Name {
    //                 name: String::from("a_variable"),
    //             },
    //             Token::Equal,
    //             Token::Int { value: 99 },
    //             Token::Plus,
    //             Token::Int { value: 2 },
    //             Token::Minus,
    //             Token::Int { value: 0 },
    //             Token::Newline,
    //         ]
    //     );
    // }

    //     macro_rules! test_indentation_with_eol {
    //         ($($name:ident: $eol:expr,)*) => {
    //             $(
    //             #[test]
    //             #[cfg(feature = "full-lexer")]
    //             fn $name() {
    //                 let source = format!("def foo():{}   return 99{}{}", $eol, $eol, $eol);
    //                 let tokens = lex_source(&source);
    //                 assert_eq!(
    //                     tokens,
    //                     vec![
    //                         Token::Def,
    //                         Token::Name {
    //                             name: String::from("foo"),
    //                         },
    //                         Token::Lpar,
    //                         Token::Rpar,
    //                         Token::Colon,
    //                         Token::Newline,
    //                         Token::Indent,
    //                         Token::Return,
    //                         Token::Int { value: 99 },
    //                         Token::Newline,
    //                         Token::NonLogicalNewline,
    //                         Token::Dedent,
    //                     ]
    //                 );
    //             }
    //             )*
    //         };
    //     }
    //
    //     test_indentation_with_eol! {
    //         test_indentation_windows_eol: WINDOWS_EOL,
    //         test_indentation_mac_eol: MAC_EOL,
    //         test_indentation_unix_eol: UNIX_EOL,
    //     }
    //
    //     macro_rules! test_double_dedent_with_eol {
    //         ($($name:ident: $eol:expr,)*) => {
    //         $(
    //             #[test]
    //             #[cfg(feature = "full-lexer")]
    //             fn $name() {
    //                 let source = format!("def foo():{} if x:{}{}  return 99{}{}", $eol, $eol, $eol, $eol, $eol);
    //                 let tokens = lex_source(&source);
    //                 assert_eq!(
    //                     tokens,
    //                     vec![
    //                         Token::Def,
    //                         Token::Name {
    //                             name: String::from("foo"),
    //                         },
    //                         Token::Lpar,
    //                         Token::Rpar,
    //                         Token::Colon,
    //                         Token::Newline,
    //                         Token::Indent,
    //                         Token::If,
    //                         Token::Name {
    //                             name: String::from("x"),
    //                         },
    //                         Token::Colon,
    //                         Token::Newline,
    //                         Token::NonLogicalNewline,
    //                         Token::Indent,
    //                         Token::Return,
    //                         Token::Int { value: 99 },
    //                         Token::Newline,
    //                         Token::NonLogicalNewline,
    //                         Token::Dedent,
    //                         Token::Dedent,
    //                     ]
    //                 );
    //             }
    //         )*
    //         }
    //     }
    //
    //     macro_rules! test_double_dedent_with_tabs {
    //         ($($name:ident: $eol:expr,)*) => {
    //         $(
    //             #[test]
    //             #[cfg(feature = "full-lexer")]
    //             fn $name() {
    //                 let source = format!("def foo():{}\tif x:{}{}\t return 99{}{}", $eol, $eol, $eol, $eol, $eol);
    //                 let tokens = lex_source(&source);
    //                 assert_eq!(
    //                     tokens,
    //                     vec![
    //                         Token::Def,
    //                         Token::Name {
    //                             name: String::from("foo"),
    //                         },
    //                         Token::Lpar,
    //                         Token::Rpar,
    //                         Token::Colon,
    //                         Token::Newline,
    //                         Token::Indent,
    //                         Token::If,
    //                         Token::Name {
    //                             name: String::from("x"),
    //                         },
    //                         Token::Colon,
    //                         Token::Newline,
    //                         Token::NonLogicalNewline,
    //                         Token::Indent,
    //                         Token::Return,
    //                         Token::Int { value: 99 },
    //                         Token::Newline,
    //                         Token::NonLogicalNewline,
    //                         Token::Dedent,
    //                         Token::Dedent,
    //                     ]
    //                 );
    //             }
    //         )*
    //         }
    //     }
    //
    //     test_double_dedent_with_eol! {
    //         test_double_dedent_windows_eol: WINDOWS_EOL,
    //         test_double_dedent_mac_eol: MAC_EOL,
    //         test_double_dedent_unix_eol: UNIX_EOL,
    //     }
    //
    //     test_double_dedent_with_tabs! {
    //         test_double_dedent_tabs_windows_eol: WINDOWS_EOL,
    //         test_double_dedent_tabs_mac_eol: MAC_EOL,
    //         test_double_dedent_tabs_unix_eol: UNIX_EOL,
    //     }
    //
    //     macro_rules! test_newline_in_brackets {
    //         ($($name:ident: $eol:expr,)*) => {
    //         $(
    //             #[test]
    //             #[cfg(feature = "full-lexer")]
    //             fn $name() {
    //                 let source = r"x = [
    //
    //     1,2
    // ,(3,
    // 4,
    // ), {
    // 5,
    // 6,\
    // 7}]
    // ".replace("\n", $eol);
    //                 let tokens = lex_source(&source);
    //                 assert_eq!(
    //                     tokens,
    //                     vec![
    //                         Token::Name {
    //                             name: String::from("x"),
    //                         },
    //                         Token::Equal,
    //                         Token::Lsqb,
    //                         Token::NonLogicalNewline,
    //                         Token::NonLogicalNewline,
    //                         Token::Int { value: 1 },
    //                         Token::Comma,
    //                         Token::Int { value: 2 },
    //                         Token::NonLogicalNewline,
    //                         Token::Comma,
    //                         Token::Lpar,
    //                         Token::Int { value: 3 },
    //                         Token::Comma,
    //                         Token::NonLogicalNewline,
    //                         Token::Int { value: 4 },
    //                         Token::Comma,
    //                         Token::NonLogicalNewline,
    //                         Token::Rpar,
    //                         Token::Comma,
    //                         Token::Lbrace,
    //                         Token::NonLogicalNewline,
    //                         Token::Int { value: 5 },
    //                         Token::Comma,
    //                         Token::NonLogicalNewline,
    //                         Token::Int { value: 6 },
    //                         Token::Comma,
    //                         // Continuation here - no NonLogicalNewline.
    //                         Token::Int { value: 7 },
    //                         Token::Rbrace,
    //                         Token::Rsqb,
    //                         Token::Newline,
    //                     ]
    //                 );
    //             }
    //         )*
    //         };
    //     }
    //
    //     test_newline_in_brackets! {
    //         test_newline_in_brackets_windows_eol: WINDOWS_EOL,
    //         test_newline_in_brackets_mac_eol: MAC_EOL,
    //         test_newline_in_brackets_unix_eol: UNIX_EOL,
    //     }
    //
    //     #[test]
    //     fn test_non_logical_newline_in_string_continuation() {
    //         let source = r"(
    //     'a'
    //     'b'
    //
    //     'c' \
    //     'd'
    // )";
    //         let tokens = lex_source(source);
    //         assert_eq!(
    //             tokens,
    //             vec![
    //                 Token::Lpar,
    //                 Token::NonLogicalNewline,
    //                 str_tok("a"),
    //                 Token::NonLogicalNewline,
    //                 str_tok("b"),
    //                 Token::NonLogicalNewline,
    //                 Token::NonLogicalNewline,
    //                 str_tok("c"),
    //                 str_tok("d"),
    //                 Token::NonLogicalNewline,
    //                 Token::Rpar,
    //                 Token::Newline,
    //             ]
    //         );
    //     }
    //
    //     #[test]
    //     fn test_logical_newline_line_comment() {
    //         let source = "#Hello\n#World\n";
    //         let tokens = lex_source(source);
    //         assert_eq!(
    //             tokens,
    //             vec![
    //                 Token::Comment("#Hello".to_owned()),
    //                 Token::NonLogicalNewline,
    //                 Token::Comment("#World".to_owned()),
    //                 Token::NonLogicalNewline,
    //             ]
    //         );
    //     }
    //
    //     #[test]
    //     fn test_operators() {
    //         let source = "//////=/ /";
    //         let tokens = lex_source(source);
    //         assert_eq!(
    //             tokens,
    //             vec![
    //                 Token::DoubleSlash,
    //                 Token::DoubleSlash,
    //                 Token::DoubleSlashEqual,
    //                 Token::Slash,
    //                 Token::Slash,
    //                 Token::Newline,
    //             ]
    //         );
    //     }
    //
    //     #[test]
    //     fn test_string() {
    //         let source = r#""double" 'single' 'can\'t' "\\\"" '\t\r\n' '\g' r'raw\'' '\420' '\200\0a'"#;
    //         let tokens = lex_source(source);
    //         assert_eq!(
    //             tokens,
    //             vec![
    //                 str_tok("double"),
    //                 str_tok("single"),
    //                 str_tok(r"can\'t"),
    //                 str_tok(r#"\\\""#),
    //                 str_tok(r"\t\r\n"),
    //                 str_tok(r"\g"),
    //                 raw_str_tok(r"raw\'"),
    //                 str_tok(r"\420"),
    //                 str_tok(r"\200\0a"),
    //                 Token::Newline,
    //             ]
    //         );
    //     }
    //
    //     macro_rules! test_string_continuation {
    //         ($($name:ident: $eol:expr,)*) => {
    //         $(
    //             #[test]
    //             fn $name() {
    //                 let source = format!("\"abc\\{}def\"", $eol);
    //                 let tokens = lex_source(&source);
    //                 assert_eq!(
    //                     tokens,
    //                     vec![
    //                         str_tok("abc\\\ndef"),
    //                         Token::Newline,
    //                     ]
    //                 )
    //             }
    //         )*
    //         }
    //     }
    //
    //     test_string_continuation! {
    //         test_string_continuation_windows_eol: WINDOWS_EOL,
    //         test_string_continuation_mac_eol: MAC_EOL,
    //         test_string_continuation_unix_eol: UNIX_EOL,
    //     }
    //
    //     #[test]
    //     fn test_escape_unicode_name() {
    //         let source = r#""\N{EN SPACE}""#;
    //         let tokens = lex_source(source);
    //         assert_eq!(tokens, vec![str_tok(r"\N{EN SPACE}"), Token::Newline])
    //     }
    //
    //     macro_rules! test_triple_quoted {
    //         ($($name:ident: $eol:expr,)*) => {
    //         $(
    //             #[test]
    //             fn $name() {
    //                 let source = format!("\"\"\"{0} test string{0} \"\"\"", $eol);
    //                 let tokens = lex_source(&source);
    //                 assert_eq!(
    //                     tokens,
    //                     vec![
    //                         Token::String {
    //                             value: "\n test string\n ".to_owned(),
    //                             kind: StringKind::String,
    //                             triple_quoted: true,
    //                         },
    //                         Token::Newline,
    //                     ]
    //                 )
    //             }
    //         )*
    //         }
    //     }
    //
    //     test_triple_quoted! {
    //         test_triple_quoted_windows_eol: WINDOWS_EOL,
    //         test_triple_quoted_mac_eol: MAC_EOL,
    //         test_triple_quoted_unix_eol: UNIX_EOL,
    //     }
}
