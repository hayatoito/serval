use failure::Fail;

pub type Result<T> = std::result::Result<T, failure::Error>;

pub trait Clamp {
    fn clamp(self, lower: Self, upper: Self) -> Self;
}
impl Clamp for f32 {
    fn clamp(self, lower: f32, upper: f32) -> f32 {
        self.max(lower).min(upper)
    }
}

#[derive(Fail, Debug)]
pub enum EngineError {
    #[fail(display = "Parse Error: {}", error)]
    ParseError {
        error: combine::error::StringStreamError,
    },
}

impl From<combine::error::StringStreamError> for EngineError {
    fn from(error: combine::error::StringStreamError) -> Self {
        EngineError::ParseError { error }
    }
}

#[cfg(test)]
macro_rules! assert_parse {
    ($parser:expr, $input:expr, $output:expr, $remaining:expr) => {
        assert_eq!($parser.parse($input), Ok(($output, $remaining)));
    };
    ($parser:expr, $input:expr, $output:expr) => {
        assert_eq!($parser.parse($input), Ok(($output, "")));
    };
}

#[cfg(test)]
macro_rules! assert_parse_fail {
    ($parser:expr, $input:expr) => {
        assert!($parser.parse($input).is_err());
    };
}

macro_rules! def_parser {
    (pub fn $parser:ident() -> $type:ty $body:block) => {
        parser! {
            pub fn $parser[I]()(I) -> $type
            where
                [I: Stream<Item = char>]
                $body
        }
    };
    (fn $parser:ident() -> $type:ty $body:block) => {
        parser! {
            fn $parser[I]()(I) -> $type
            where
                [I: Stream<Item = char>]
                $body
        }
    };
}
