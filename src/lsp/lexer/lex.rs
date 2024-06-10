use super::{
    text_range::TextRange,
    text_size::{TextLen, TextSize},
    token::{StringKind, Token},
};
use std::{cmp::Ordering, ops::Index, slice::SliceIndex};
use std::{
    iter::{self, Peekable},
    slice::Windows,
};
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
        return next;
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
            lexer.char_reader.next();
        }

        return lexer;
    }
}

impl<T> Lexer<T>
where
    T: Iterator<Item = char>,
{
    pub fn next_char(&mut self) -> Option<char> {
        let char = self.char_reader.next();

        return match self.window()[..2] {
            [Some('\r'), Some('\n')] => self.char_reader.next(),
            [Some(_), ..] => char,
            _ => None,
        };
    }

    pub fn char_cursor(&self) -> TextSize {
        return self.char_reader.cursor;
    }

    pub fn window(&self) -> &[Option<char>; 3] {
        return &self.char_reader.window;
    }

    pub fn get_identifier_or_keyword_token(&mut self) -> LexResult {
        let start_pos = self.char_cursor();
        let mut name = String::with_capacity(8);

        while let [Some(c1), Some(c2)] = self.window()[..2] {
            name.push(c1);
            if !is_identifier_or_keyword_continuation(c2) {
                break;
            }
            self.next_char();
        }

        let end_pos = self.char_cursor();

        if let Some(token) = Token::try_get_keyword(&name) {
            return Ok((token.clone(), TextRange::new(start_pos, end_pos)));
        } else {
            return Ok((Token::Name { name }, TextRange::new(start_pos, end_pos)));
        }
    }

    pub fn try_get_tagged_string_literal_token(&mut self) -> Option<LexResult> {
        // detect potential string like rb'' r'' f'' u'' r''
        return match self.window()[..3] {
            [Some(c), Some('"' | '\''), ..] => match StringKind::try_from(c) {
                Ok(kind) => Some(self.get_string_token(kind)),
                Err(msg) => Some(Err(LexicalError {
                    error: LexicalErrorType::OtherError(msg),
                    location: self.char_cursor(),
                })),
            },
            [Some(c1), Some(c2), Some('"' | '\'')] => match StringKind::try_from([c1, c2]) {
                Ok(kind) => Some(self.get_string_token(kind)),
                Err(msg) => Some(Err(LexicalError {
                    error: LexicalErrorType::OtherError(msg),
                    location: self.char_cursor(),
                })),
            },
            _ => None,
        };
    }

    pub fn get_string_token(&mut self, kind: StringKind) -> LexResult {
        let start_pos = self.char_cursor();

        for _ in 0..u32::from(kind.prefix_len()) {
            self.next_char();
        }

        let quote_char = self.window()[0].expect("Quote character is expected!");

        let mut string_content = String::with_capacity(5);

        let is_triple_quoted = if [Some(quote_char); 3] == self.window()[..3] {
            self.next_char();
            self.next_char();
            self.next_char();
            true
        } else {
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
                            self.next_char();
                            self.next_char();
                            break;
                        } else {
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

    pub fn populate_results_queue(&mut self) -> Result<(), LexicalError> {
        match self.window()[0] {
            Some(c) if is_identifier_or_keywords_start(c) => {
                if let Some(token) = self.try_get_tagged_string_literal_token() {
                    self.queue.push(token?);
                } else {
                    let token = self.get_identifier_or_keyword_token()?;
                    self.queue.push(token);
                }

                Ok(())
            }
            Some(c) => {
                // TODO
                return Err(LexicalError {
                    error: LexicalErrorType::UnrecognizedToken { tok: c },
                    location: self.char_cursor(),
                });
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

                // TODO insert a trailing newline if required

                // TODO Flush indentation stack to zero

                self.queue
                    .push((Token::EndOfFile, TextRange::empty(self.char_cursor())));
                Ok(())
            }
        }
    }

    fn inner_next(&mut self) -> LexResult {
        while self.queue.is_empty() {
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
