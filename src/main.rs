use std::{env, error::Error, fmt, path::PathBuf, process::ExitCode, thread, time::Duration};

use libgpiod::{
    chip::Chip,
    line::{Config as LineConfig, Direction, Settings, Value},
    request::Config as RequestConfig,
};

mod sequence;

const DEFAULT_CHIP: &str = "/dev/gpiochip0";
pub(crate) const CONSUMER: &str = "gpio-cli";

#[derive(Debug, PartialEq)]
enum Command {
    Read {
        pin: u32,
    },
    Write {
        pin: u32,
        value: Value,
        hold_for: Option<Duration>,
    },
    WriteAll {
        value: Value,
        hold_for: Option<Duration>,
    },
    Sequence {
        config_path: PathBuf,
    },
}

#[derive(Debug, PartialEq)]
struct Cli {
    chip: PathBuf,
    command: Command,
}

#[derive(Debug, PartialEq)]
struct CliError(String);

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for CliError {}

fn usage() -> &'static str {
    "Usage:\n  gpio-cli [--chip PATH] read <pin>\n  gpio-cli [--chip PATH] write <pin> <HIGH|LOW|1|0> [--for <duration>]\n  gpio-cli [--chip PATH] write-all <HIGH|LOW|1|0> [--for <duration>]\n  gpio-cli [--chip PATH] sequence <config.json>\n\nDurations: 500ms, 10s, 2m (default unit: seconds)"
}

fn parse_value(raw: &str) -> Result<Value, CliError> {
    match raw.to_ascii_uppercase().as_str() {
        "HIGH" | "1" => Ok(Value::Active),
        "LOW" | "0" => Ok(Value::InActive),
        _ => Err(CliError(format!(
            "invalid value '{raw}'; expected HIGH, LOW, 1, or 0"
        ))),
    }
}

fn parse_pin(raw: &str) -> Result<u32, CliError> {
    raw.parse().map_err(|_| {
        CliError(format!(
            "invalid pin offset '{raw}'; expected an unsigned integer"
        ))
    })
}

fn parse_duration(raw: &str) -> Result<Duration, CliError> {
    let (number, multiplier) = if let Some(number) = raw.strip_suffix("ms") {
        (number, 1)
    } else if let Some(number) = raw.strip_suffix('s') {
        (number, 1_000)
    } else if let Some(number) = raw.strip_suffix('m') {
        (number, 60_000)
    } else {
        (raw, 1_000)
    };

    let amount: u64 = number.parse().map_err(|_| {
        CliError(format!(
            "invalid duration '{raw}'; expected values such as 500ms, 10s, or 2m"
        ))
    })?;
    let milliseconds = amount
        .checked_mul(multiplier)
        .filter(|value| *value > 0)
        .ok_or_else(|| CliError(format!("invalid duration '{raw}'")))?;

    Ok(Duration::from_millis(milliseconds))
}

fn parse_args<I, S>(args: I) -> Result<Cli, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into).peekable();
    let mut chip = PathBuf::from(DEFAULT_CHIP);

    if args.peek().is_some_and(|arg| arg == "--chip") {
        args.next();
        chip = args
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| CliError("--chip requires a device path".into()))?;
    }

    let command = match args.next().as_deref() {
        Some("read") => Command::Read {
            pin: parse_pin(
                &args
                    .next()
                    .ok_or_else(|| CliError("read requires <pin>".into()))?,
            )?,
        },
        Some("write") => {
            let pin = parse_pin(
                &args
                    .next()
                    .ok_or_else(|| CliError("write requires <pin> and <value>".into()))?,
            )?;
            let value = parse_value(
                &args
                    .next()
                    .ok_or_else(|| CliError("write requires <pin> and <value>".into()))?,
            )?;
            let hold_for = match args.next().as_deref() {
                Some("--for") => {
                    Some(parse_duration(&args.next().ok_or_else(|| {
                        CliError("--for requires a duration".into())
                    })?)?)
                }
                Some(extra) => return Err(CliError(format!("unexpected argument '{extra}'"))),
                None => None,
            };
            Command::Write {
                pin,
                value,
                hold_for,
            }
        }
        Some("write-all") => {
            let value = parse_value(
                &args
                    .next()
                    .ok_or_else(|| CliError("write-all requires <value>".into()))?,
            )?;
            let hold_for = match args.next().as_deref() {
                Some("--for") => {
                    Some(parse_duration(&args.next().ok_or_else(|| {
                        CliError("--for requires a duration".into())
                    })?)?)
                }
                Some(extra) => return Err(CliError(format!("unexpected argument '{extra}'"))),
                None => None,
            };
            Command::WriteAll { value, hold_for }
        }
        Some("sequence") => Command::Sequence {
            config_path: PathBuf::from(
                args.next()
                    .ok_or_else(|| CliError("sequence requires <config.json>".into()))?,
            ),
        },
        Some("help" | "--help" | "-h") => return Err(CliError(usage().into())),
        Some(command) => return Err(CliError(format!("unknown command '{command}'"))),
        None => return Err(CliError("missing command".into())),
    };

    if let Some(extra) = args.next() {
        return Err(CliError(format!("unexpected argument '{extra}'")));
    }

    Ok(Cli { chip, command })
}

fn request_line(
    chip_path: &PathBuf,
    pin: u32,
    direction: Direction,
    output_value: Option<Value>,
) -> Result<libgpiod::request::Request, Box<dyn Error>> {
    let mut settings = Settings::new()?;
    settings.set_direction(direction)?;
    if let Some(value) = output_value {
        settings.set_output_value(value)?;
    }

    let mut line_config = LineConfig::new()?;
    line_config.add_line_settings(&[pin], settings)?;

    let mut request_config = RequestConfig::new()?;
    request_config.set_consumer(CONSUMER)?;

    let chip = Chip::open(chip_path)?;
    Ok(chip.request_lines(Some(&request_config), &line_config)?)
}

fn request_all_lines(
    chip_path: &PathBuf,
    value: Value,
) -> Result<(libgpiod::request::Request, usize), Box<dyn Error>> {
    let chip = Chip::open(chip_path)?;
    let offsets = (0..chip.info()?.num_lines() as u32).collect::<Vec<_>>();

    let mut settings = Settings::new()?;
    settings.set_direction(Direction::Output)?;
    settings.set_output_value(value)?;

    let mut line_config = LineConfig::new()?;
    line_config.add_line_settings(&offsets, settings)?;

    let mut request_config = RequestConfig::new()?;
    request_config.set_consumer(CONSUMER)?;

    let line_count = offsets.len();
    Ok((
        chip.request_lines(Some(&request_config), &line_config)?,
        line_count,
    ))
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    match cli.command {
        Command::Read { pin } => {
            let request = request_line(&cli.chip, pin, Direction::Input, None)?;
            let value = request.value(pin)?;
            println!(
                "{}",
                if value == Value::Active {
                    "HIGH"
                } else {
                    "LOW"
                }
            );
        }
        Command::Write {
            pin,
            value,
            hold_for,
        } => {
            let _request = request_line(&cli.chip, pin, Direction::Output, Some(value))?;
            println!(
                "{}",
                if value == Value::Active {
                    "HIGH"
                } else {
                    "LOW"
                }
            );
            if let Some(duration) = hold_for {
                thread::sleep(duration);
            }
        }
        Command::WriteAll { value, hold_for } => {
            let (_request, line_count) = request_all_lines(&cli.chip, value)?;
            println!(
                "set {line_count} lines {}",
                if value == Value::Active {
                    "HIGH"
                } else {
                    "LOW"
                }
            );
            if let Some(duration) = hold_for {
                thread::sleep(duration);
            }
        }
        Command::Sequence { config_path } => sequence::run(&cli.chip, &config_path)?,
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = match parse_args(env::args().skip(1)) {
        Ok(cli) => cli,
        Err(error) => {
            eprintln!("{error}\n\n{}", usage());
            return ExitCode::from(2);
        }
    };

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gpio-cli: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_read_with_default_chip() {
        assert_eq!(
            parse_args(["read", "17"]).unwrap(),
            Cli {
                chip: PathBuf::from(DEFAULT_CHIP),
                command: Command::Read { pin: 17 },
            }
        );
    }

    #[test]
    fn parses_write_values_case_insensitively() {
        assert_eq!(
            parse_args(["--chip", "/dev/gpiochip4", "write", "23", "high"]).unwrap(),
            Cli {
                chip: PathBuf::from("/dev/gpiochip4"),
                command: Command::Write {
                    pin: 23,
                    value: Value::Active,
                    hold_for: None,
                },
            }
        );
        assert_eq!(parse_value("0").unwrap(), Value::InActive);
    }

    #[test]
    fn rejects_invalid_value() {
        assert!(parse_args(["write", "23", "on"]).is_err());
    }

    #[test]
    fn parses_timed_write() {
        assert_eq!(
            parse_args(["write", "17", "HIGH", "--for", "500ms"]).unwrap(),
            Cli {
                chip: PathBuf::from(DEFAULT_CHIP),
                command: Command::Write {
                    pin: 17,
                    value: Value::Active,
                    hold_for: Some(Duration::from_millis(500)),
                },
            }
        );
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
        assert!(parse_duration("0s").is_err());
    }

    #[test]
    fn parses_timed_write_all() {
        assert_eq!(
            parse_args(["write-all", "LOW", "--for", "10s"]).unwrap(),
            Cli {
                chip: PathBuf::from(DEFAULT_CHIP),
                command: Command::WriteAll {
                    value: Value::InActive,
                    hold_for: Some(Duration::from_secs(10)),
                },
            }
        );
    }
}
