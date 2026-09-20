use std::{
    error::Error,
    fs,
    io::{Error as IoError, ErrorKind},
    path::Path,
    thread,
    time::Duration,
};

use libgpiod::{
    chip::Chip,
    line::{Config as LineConfig, Direction, Settings, Value, ValueMap},
    request::Config as RequestConfig,
};
use serde::Deserialize;

use crate::CONSUMER;

#[derive(Debug, Deserialize)]
struct SequenceConfig {
    #[serde(default = "default_phase_duration_ms")]
    phase_duration_ms: u64,
    #[serde(default)]
    cycles: Option<u64>,
    peripherals: Vec<Peripheral>,
}

#[derive(Debug, Deserialize)]
struct Peripheral {
    id: String,
    pin: Option<u32>,
    io_mode: String,
}

fn default_phase_duration_ms() -> u64 {
    5_000
}

pub fn run(chip_path: &Path, config_path: &Path) -> Result<(), Box<dyn Error>> {
    let config = load_config(config_path)?;
    let outputs = config
        .peripherals
        .iter()
        .filter(|peripheral| peripheral.io_mode == "write")
        .filter_map(|peripheral| peripheral.pin.map(|pin| (peripheral, pin)))
        .collect::<Vec<_>>();
    let inputs = config
        .peripherals
        .iter()
        .filter(|peripheral| peripheral.io_mode == "read")
        .filter_map(|peripheral| peripheral.pin.map(|pin| (peripheral, pin)))
        .collect::<Vec<_>>();

    if outputs.is_empty() {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "the sequence configuration has no write peripherals with a pin",
        )
        .into());
    }

    let mut output_settings = Settings::new()?;
    output_settings.set_direction(Direction::Output)?;
    output_settings.set_output_value(Value::InActive)?;

    let mut input_settings = Settings::new()?;
    input_settings.set_direction(Direction::Input)?;

    let mut line_config = LineConfig::new()?;
    line_config.add_line_settings(
        &outputs.iter().map(|(_, pin)| *pin).collect::<Vec<_>>(),
        output_settings,
    )?;
    if !inputs.is_empty() {
        line_config.add_line_settings(
            &inputs.iter().map(|(_, pin)| *pin).collect::<Vec<_>>(),
            input_settings,
        )?;
    }

    let mut request_config = RequestConfig::new()?;
    request_config.set_consumer(CONSUMER)?;
    let chip = Chip::open(&chip_path)?;
    let mut request = chip.request_lines(Some(&request_config), &line_config)?;
    let phase_duration = Duration::from_millis(config.phase_duration_ms);
    let mut cycle = 1;

    loop {
        println!("\n---------------------------------\ncycle {cycle}\n---------------------------------", );
        for value in [Value::Active, Value::InActive] {
            println!("setting outputs to {}", value_name(value));
            let mut values = ValueMap::new();
            for (_, pin) in &outputs {
                values.insert(*pin, value);
            }
            request.set_values_subset(values)?;

            for (peripheral, pin) in &outputs {
                println!(
                    "cycle {cycle}: {} (GPIO {pin}) -> {}",
                    peripheral.id,
                    value_name(value)
                );
            }
            for (peripheral, pin) in &inputs {
                println!(
                    "cycle {cycle}: {} (GPIO {pin}) reads {}",
                    peripheral.id,
                    value_name(request.value(*pin)?)
                );
            }
            thread::sleep(phase_duration);
        }
        
        if config.cycles.is_some_and(|cycles| cycle >= cycles) {
            break;
        }
        cycle += 1;
        println!("\n---------------------------\ncycle {cycle} completed\n---------------------------");
    }

    Ok(())
}

fn load_config(config_path: &Path) -> Result<SequenceConfig, Box<dyn Error>> {
    let config = serde_json::from_str::<SequenceConfig>(&fs::read_to_string(config_path)?)?;
    if config.phase_duration_ms == 0 {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "phase_duration_ms must be greater than zero",
        )
        .into());
    }
    Ok(config)
}

fn value_name(value: Value) -> &'static str {
    if value == Value::Active {
        "HIGH"
    } else {
        "LOW"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_phase_duration_to_five_seconds() {
        let config = serde_json::from_str::<SequenceConfig>(
            r#"{"peripherals":[{"id":"RELAY_DIMMER","pin":18,"io_mode":"write"}]}"#,
        )
        .unwrap();

        assert_eq!(config.phase_duration_ms, 5_000);
        assert_eq!(config.peripherals[0].pin, Some(18));
    }
}
