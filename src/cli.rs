use arg::Args;

use core::str::FromStr;

pub type TokenStr = str_buf::StrBuf<[u8; 59]>;

#[derive(Debug)]
#[repr(transparent)]
pub struct Token(pub TokenStr);

impl FromStr for Token {
    type Err = ();

    #[inline]
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if TokenStr::capacity() != text.len() {
            Err(())
        } else {
            Ok(Token(TokenStr::from_str(text)))
        }
    }
}

#[derive(Args, Debug)]
///Find files utility
pub struct Cli {
    #[arg(long, short, required)]
    ///Discord token to use. Must be a string of 59 characters.
    pub token: Token,
    #[arg(long, short, default_value = "'.'")]
    ///Command prefix. Default is '.'.
    pub prefix: char,
}

impl Cli {
    #[inline]
    pub fn new<'a, T: IntoIterator<Item = &'a str>>(args: T) -> Result<Self, u8> {
        let args = args.into_iter();

        Cli::from_args(args).map_err(|err| match err.is_help() {
            true => {
                println!("{}", Cli::HELP);
                0
            },
            false => {
                eprintln!("{}", err);
                1
            },
        })
    }
}
