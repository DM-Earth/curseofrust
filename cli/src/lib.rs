use std::{cmp::max, ffi::OsStr, net::SocketAddr, process::exit};

use curseofrust::state::{BasicOpts, MultiplayerOpts};

use wrapper::{DifficultyWrapper as Difficulty, SpeedWrapper as Speed, StencilWrapper as Stencil};

mod wrapper;

const DEFAULT_SERVER_PORT: u16 = 19140;
const DEFAULT_CLIENT_PORT: u16 = 19150;

/// Parses the command line arguments.
pub fn parse(
    args: impl IntoIterator<Item = impl Into<std::ffi::OsString>>,
) -> Result<(BasicOpts, MultiplayerOpts), Error> {
    let mut basic_opts = BasicOpts::default();
    let mut multiplayer_opts = MultiplayerOpts::default();

    let args = clap_lex::RawArgs::new(args);
    let mut cursor = args.cursor();
    args.next(&mut cursor); // skip bin
    while let Some(arg) = args.next(&mut cursor) {
        if let Some(mut s) = arg.to_short() {
            while let Some(Ok(flag)) = s.next() {
                macro_rules! parse {
                    ($a:expr, $t:expr, $vt:ty) => {{
                        let v: Result<$vt, _> = args
                            .next(&mut cursor)
                            .ok_or_else(|| Error::MissingValue { arg: $a, ty: $t })
                            .and_then(|a| {
                                a.to_value_os()
                                    .to_string_lossy()
                                    .parse()
                                    .map_err(From::from)
                            });
                        v
                    }};
                    ($a:expr, $t:expr) => {
                        parse!($a, $t, _)
                    };
                }
                match flag {
                    'W' => basic_opts.width = parse!("-W", "integer")?,
                    // Minimum height.
                    'H' => basic_opts.height = max(parse!("-H", "integer")?, 5),
                    'S' => basic_opts.shape = parse!("-S", "shape", Stencil)?.0,
                    'l' => basic_opts.locations = parse!("-l", "integer")?,
                    'i' => basic_opts.inequality = Some(parse!("-i", "integer")?),
                    'q' => basic_opts.conditions = Some(parse!("-q", "integer")?),
                    'r' => basic_opts.keep_random = true,
                    'd' => basic_opts.difficulty = parse!("-d", "difficulty", Difficulty)?.0,
                    's' => basic_opts.speed = parse!("-s", "speed", Speed)?.0,
                    'R' => basic_opts.seed = parse!("-R", "integer")?,
                    'T' => basic_opts.timeline = true,
                    'E' => {
                        basic_opts.clients = parse!("-E", "integer")?;
                        if matches!(multiplayer_opts, MultiplayerOpts::None) {
                            multiplayer_opts = MultiplayerOpts::Server {
                                port: DEFAULT_SERVER_PORT,
                            };
                        }
                    }
                    'e' => {
                        multiplayer_opts = MultiplayerOpts::Server {
                            port: parse!("-e", "integer")?,
                        };
                    }
                    'C' => {
                        let parsed = parse!("-C", "SocketAddr")?;
                        if let MultiplayerOpts::Client { ref mut server, .. } = multiplayer_opts {
                            *server = parsed;
                        } else {
                            multiplayer_opts = MultiplayerOpts::Client {
                                server: parsed,
                                port: DEFAULT_CLIENT_PORT,
                            }
                        }
                    }
                    'c' => {
                        let parsed = parse!("-c", "integer")?;
                        if let MultiplayerOpts::Client { ref mut port, .. } = multiplayer_opts {
                            *port = parsed
                        } else {
                            multiplayer_opts = MultiplayerOpts::Client {
                                server: SocketAddr::from((
                                    std::net::Ipv4Addr::LOCALHOST,
                                    DEFAULT_SERVER_PORT,
                                )),
                                port: parsed,
                            };
                        }
                    }
                    'v' => {
                        println!("curseofrust");
                        exit(0)
                    }
                    'h' => {
                        println!("{HELP_MSG}");
                        exit(0)
                    }
                    f => return Err(Error::UnknownFlag { flag: f }),
                }
            }
        }
    }

    // Fix a weird bug.
    if basic_opts.shape == curseofrust::grid::Stencil::Rect {
        basic_opts.width += 10;
    }

    Ok((basic_opts, multiplayer_opts))
}

#[derive(Debug)]
pub enum Error {
    MissingValue {
        arg: &'static str,
        ty: &'static str,
    },
    InvalidIntValueFmt(std::num::ParseIntError),
    InvalidIpAddrValueFmt(std::net::AddrParseError),
    NonUnicodeValue {
        content: Box<OsStr>,
    },
    UnknownFlag {
        flag: char,
    },
    UnknownVariant {
        ty: &'static str,
        variants: &'static [&'static str],
        value: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingValue { arg, ty } => {
                write!(
                    f,
                    "missing value for argument '{arg}', expected type '{ty}'"
                )
            }
            Error::InvalidIntValueFmt(err) => write!(f, "invalid integer formatting: {err}"),
            Error::InvalidIpAddrValueFmt(err) => write!(f, "invalid IP-address formatting: {err}"),
            Error::NonUnicodeValue { content } => {
                write!(f, "non-unicode value: {content:?}")
            }
            Error::UnknownFlag { flag } => write!(f, "unknown flag: {flag}"),
            Error::UnknownVariant {
                ty,
                variants,
                value,
            } => write!(
                f,
                "unknown variant '{value}' for type '{ty}', expected one of: {variants:?}",
            ),
        }
    }
}

impl<'a> From<&'a OsStr> for Error {
    #[inline]
    fn from(value: &'a OsStr) -> Self {
        Error::NonUnicodeValue {
            content: value.into(),
        }
    }
}

impl From<std::num::ParseIntError> for Error {
    #[inline]
    fn from(value: std::num::ParseIntError) -> Self {
        Error::InvalidIntValueFmt(value)
    }
}

impl From<std::net::AddrParseError> for Error {
    #[inline]
    fn from(value: std::net::AddrParseError) -> Self {
        Error::InvalidIpAddrValueFmt(value)
    }
}

impl std::error::Error for Error {}

/// The help message for the program.
pub const HELP_MSG: &str = // Pad
    r#"                                __
   ____                        /  ]  ________             __
  / __ \_ _ ___ ___ ___    __ _| |_  |  ___  \__  __ ___ _| |__
_/ /  \/ | |X _/ __/ __\  /   \   /  | |___| | | |  / __/_  __/
\ X    | | | | |__ | __X  | X || |   | X_  __/ |_|  X__ | | X
 \ \__/\ __X_| \___/___/  \___/| |   | | \ \_ X__ /___ /  \__/
  \____/                       |/    |_\  \__/

  Made by DM Earth in 2024.

  Command line arguments:

-W width
  Map width (default is 21)

-H height
  Map height (default is 21)

-S [rhombus|rect|hex]
  Map shape (rectangle is default). Max number of countries N=4 for rhombus and rectangle, and N=6 for the hexagon.

-l [2|3| ... N]
  Sets L, the number of countries (default is N).

-i [0|1|2|3|4]
  Inequality between the countries (0 is the lowest, 4 in the highest).

-q [1|2| ... L]
  Choose player's location by its quality (1 = the best available on the map, L = the worst). Only in the singleplayer mode.

-r
  Absolutely random initial conditions, overrides options -l, -i, and -q.

-d [ee|e|n|h|hh]
  Difficulty level (AI) from the easiest to the hardest (default is normal).

-s [p|sss|ss|s|n|f|ff|fff]
  Game speed from the slowest to the fastest (default is normal).

-R seed
  Specify a random seed (unsigned integer) for map generation.

-T
  Show the timeline.

-E [1|2| ... L]
  Start a server for not more than L clients.

-e port
  Server's port (19140 is default).

-C IP
  Start a client and connect to the provided server's IP-address.

-c port
  Clients's port (19150 is default).

-v
  Display the version number

-h
  Display this help
"#;
