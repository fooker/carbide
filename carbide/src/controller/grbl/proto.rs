#![allow(dead_code)]

use std::str::FromStr;

use bytes::Bytes;
use failure::Error;
use lazy_static::lazy_static;
use regex::{Regex, Captures};

use crate::controller;
use crate::position::Position;

#[derive(Debug, Clone)]
pub enum GrblRestoreCommand {
    Settings,
    Parameters,
    All,
}

#[derive(Debug, Clone)]
pub enum GrblSystemCommand {
    Help,
    ViewSettings,
    WriteSetting { code: u8, value: f64 },
    // TODO: Replace code with enum
    ViewParameters,
    ViewParserState,
    ViewBuildInfo,
    ViewStartupBlocks,
    WriteStartupBlock { nr: u8, line: String },
    ToggleCheckMode,
    KillAlarmLock,
    RunHomingCycle,
    RunJoggingMotion(String),
    Restore(GrblRestoreCommand),
    Sleep,
}

impl GrblSystemCommand {
    // TODO: Return bytes here, too
    pub fn into_string(self) -> String {
        return match self {
            GrblSystemCommand::Help => format!("$"),
            GrblSystemCommand::ViewSettings => format!("$$"),
            GrblSystemCommand::WriteSetting { code, value } => format!("${}={}", code, value),
            GrblSystemCommand::ViewParameters => format!("$#"),
            GrblSystemCommand::ViewParserState => format!("$G"),
            GrblSystemCommand::ViewBuildInfo => format!("$I"),
            GrblSystemCommand::ViewStartupBlocks => format!("$N"),
            GrblSystemCommand::WriteStartupBlock { nr, line } => format!("$N{}={}", nr, line),
            GrblSystemCommand::ToggleCheckMode => format!("$C"),
            GrblSystemCommand::KillAlarmLock => format!("$X"),
            GrblSystemCommand::RunHomingCycle => format!("$H"),
            GrblSystemCommand::RunJoggingMotion(line) => format!("$J={}", line),
            GrblSystemCommand::Restore(restore) => match restore {
                GrblRestoreCommand::Settings => format!("$RST=$"),
                GrblRestoreCommand::Parameters => format!("$RST=#"),
                GrblRestoreCommand::All => format!("$RST=*"),
            },
            GrblSystemCommand::Sleep => format!("$SLP"),
        };
    }
}

#[derive(Debug, Clone)]
pub enum GrblFeedOverride {
    Reset,
    Increase10,
    Decrease10,
    Increase1,
    Decrease1,
}

#[derive(Debug, Clone)]
pub enum GrblRapidOverride {
    Full,
    Half,
    Quarter,
}

#[derive(Debug, Clone)]
pub enum GrblSpeedOverride {
    Reset,
    Increase10,
    Decrease10,
    Increase1,
    Decrease1,
}

#[derive(Debug, Clone)]
pub enum GrblRealtimeCommand {
    SoftReset,
    StatusReportQuery,
    CycleStartResume,
    FeedHold,
    SafetyDoor,
    JogCancel,
    FeedOverride(GrblFeedOverride),
    RapidOverride(GrblRapidOverride),
    SpeedOverride(GrblSpeedOverride),
    ToggleSpindleStop,
    ToggleFloodCoolant,
    ToggleMistCoolant,
}

impl GrblRealtimeCommand {
    pub fn to_code(&self) -> Bytes {
        return match self {
            GrblRealtimeCommand::SoftReset => Bytes::from_static(&[0x18]),
            GrblRealtimeCommand::StatusReportQuery => Bytes::from_static(&['?' as u8]),
            GrblRealtimeCommand::CycleStartResume => Bytes::from_static(&['~' as u8]),
            GrblRealtimeCommand::FeedHold => Bytes::from_static(&['!' as u8]),
            GrblRealtimeCommand::SafetyDoor => Bytes::from_static(&[0x84]),
            GrblRealtimeCommand::JogCancel => Bytes::from_static(&[0x85]),
            GrblRealtimeCommand::FeedOverride(value) => match value {
                GrblFeedOverride::Reset => Bytes::from_static(&[0x90]),
                GrblFeedOverride::Increase10 => Bytes::from_static(&[0x91]),
                GrblFeedOverride::Decrease10 => Bytes::from_static(&[0x92]),
                GrblFeedOverride::Increase1 => Bytes::from_static(&[0x93]),
                GrblFeedOverride::Decrease1 => Bytes::from_static(&[0x94]),
            },
            GrblRealtimeCommand::RapidOverride(value) => match value {
                GrblRapidOverride::Full => Bytes::from_static(&[0x95]),
                GrblRapidOverride::Half => Bytes::from_static(&[0x96]),
                GrblRapidOverride::Quarter => Bytes::from_static(&[0x97]),
            },
            GrblRealtimeCommand::SpeedOverride(value) => match value {
                GrblSpeedOverride::Reset => Bytes::from_static(&[0x99]),
                GrblSpeedOverride::Increase10 => Bytes::from_static(&[0x9A]),
                GrblSpeedOverride::Decrease10 => Bytes::from_static(&[0x9B]),
                GrblSpeedOverride::Increase1 => Bytes::from_static(&[0x9C]),
                GrblSpeedOverride::Decrease1 => Bytes::from_static(&[0x9D]),
            },
            GrblRealtimeCommand::ToggleSpindleStop => Bytes::from_static(&[0x9E]),
            GrblRealtimeCommand::ToggleFloodCoolant => Bytes::from_static(&[0xA0]),
            GrblRealtimeCommand::ToggleMistCoolant => Bytes::from_static(&[0xA1]),
        };
    }
}

#[derive(Debug, Clone)]
pub enum GrblLineCommand {
    Line(String),
    System(GrblSystemCommand),
}

impl GrblLineCommand {
    // TODO: Return bytes here, too (rename `to_code`)
    pub fn into_string(self) -> String {
        return match self {
            GrblLineCommand::Line(line) => line,
            GrblLineCommand::System(command) => command.into_string(),
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrblResponse {
    Ok,
    Error(u8), // TODO: Replace code with enum
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrblMachineHoldStatus {
    Complete,
    InProgress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrblMachineDoorStatus {
    Closed,
    Open,
    Holding,
    Resuming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrblMachineState {
    Idle,
    Run,
    Hold(GrblMachineHoldStatus),
    Jog,
    Alarm,
    Door(GrblMachineDoorStatus),
    Check,
    Home,
    Sleep,
}

impl Into<controller::MachineStatus> for GrblMachineState {
    fn into(self) -> controller::MachineStatus {
        return match self {
            GrblMachineState::Idle => controller::MachineStatus::Idle,
            GrblMachineState::Run => controller::MachineStatus::Run,
            GrblMachineState::Hold(status) => match status {
                GrblMachineHoldStatus::InProgress => controller::MachineStatus::Hold(controller::HoldStatus::InProgress),
                GrblMachineHoldStatus::Complete => controller::MachineStatus::Hold(controller::HoldStatus::Complete),
            },
            GrblMachineState::Jog => controller::MachineStatus::Jog,
            GrblMachineState::Alarm => controller::MachineStatus::Alarm,
            GrblMachineState::Door(status) => match status {
                GrblMachineDoorStatus::Closed => controller::MachineStatus::Door(controller::DoorStatus::Closed),
                GrblMachineDoorStatus::Open => controller::MachineStatus::Door(controller::DoorStatus::Open),
                GrblMachineDoorStatus::Holding => controller::MachineStatus::Door(controller::DoorStatus::Holding),
                GrblMachineDoorStatus::Resuming => controller::MachineStatus::Door(controller::DoorStatus::Resuming),
            },
            GrblMachineState::Check => controller::MachineStatus::Check,
            GrblMachineState::Home => controller::MachineStatus::Home,
            GrblMachineState::Sleep => controller::MachineStatus::Sleep,
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GrblPositionStatus {
    MachinePosition(Position),
    WorkPosition(Position),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrblBufferStatus {
    pub planner: u8,
    pub rx: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrblInputPinsStatus {
    pub x_limit: bool,
    pub y_limit: bool,
    pub z_limit: bool,
    pub probe: bool,
    pub door: bool,
    pub hold: bool,
    pub soft_reset: bool,
    pub cycle_start: bool,
}

impl Default for GrblInputPinsStatus {
    fn default() -> Self {
        return Self {
            x_limit: false,
            y_limit: false,
            z_limit: false,
            probe: false,
            door: false,
            hold: false,
            soft_reset: false,
            cycle_start: false,
        };
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrblOverrridesStatus {
    pub feed: f64,
    pub rapids: f64,
    pub speed: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrblSpindleStatus {
    Off,
    CW,
    CCW,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrblAccessoryStatus {
    pub spindle: GrblSpindleStatus,
    pub flood_coolant: bool,
    pub mist_coolant: bool,
}

impl Default for GrblAccessoryStatus {
    fn default() -> Self {
        return Self {
            spindle: GrblSpindleStatus::Off,
            flood_coolant: false,
            mist_coolant: false,
        };
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrblStatusReport {
    pub machine_state: GrblMachineState,
    pub position: GrblPositionStatus,
    pub wco: Option<Position>,
    pub buffer: Option<GrblBufferStatus>,
    pub line: Option<usize>,
    pub feed: Option<f64>,
    pub speed: Option<f64>,
    pub input_pins: Option<GrblInputPinsStatus>,
    pub overrides: Option<GrblOverrridesStatus>,
    pub accessory: Option<GrblAccessoryStatus>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrblMessage {
    Response(GrblResponse),
    Alarm(u8),
    // TODO: Replace code with enum
    Setting { code: u8, value: f64 },
    // TODO: Replace code with enum
    StartupLine { nr: u8, line: String },
    Feedback(String),
    ParserState(String),
    Help(String),
    Parameter(String),
    Version { version: String, note: String },
    BuildOptions(String),
    StatusReport(GrblStatusReport),
    Other(String),
}

impl GrblMessage {
    pub fn parse(line: &str) -> Result<Self, Error> {
        lazy_static! {
                // ok
                static ref RE_RESPONSE_OK: Regex = Regex::new(r"^ok$").unwrap();

                // error:x
                static ref RE_RESPONSE_ERROR: Regex = Regex::new(r"^error:(.+)$").unwrap();

                // ALARM:x
                static ref RE_ALARM: Regex = Regex::new(r"^ALARM:(.+)$").unwrap();

                // $x=val
                static ref RE_SETTING: Regex = Regex::new(r"^\$(\d+)=(.+)$").unwrap();

                // $Nx=line
                static ref RE_STARTUP_LINE: Regex = Regex::new(r"^\$N(\d+)=(.*)$").unwrap();

                // [MSG:line]
                static ref RE_FEEDBACK: Regex = Regex::new(r"^\[MSG:(.*)\]$").unwrap();

                // [GC:G0 G54 G17 G21 G90 G94 M0 M5 M9 T0 S0.0 F0.0]
                static ref RE_PARSER_STATE: Regex = Regex::new(r"^\[GC:(.*)\]$").unwrap();

                // [HLP:line]
                static ref RE_HELP: Regex = Regex::new(r"^\[HLP:(.*)\]$").unwrap();

                // [G54:0.000,0.000,0.000]
                // [G55:0.000,0.000,0.000]
                // [G56:0.000,0.000,0.000]
                // [G57:0.000,0.000,0.000]
                // [G58:0.000,0.000,0.000]
                // [G59:0.000,0.000,0.000]
                // [G28:0.000,0.000,0.000]
                // [G30:0.000,0.000,0.000]
                // [G92:0.000,0.000,0.000]
                // [TLO:0.000]
                // [PRB:0.000,0.000,0.000:0]
                static ref RE_PARAMETER: Regex = Regex::new(r"^\[(G54|G55|G56|G57|G58|G59|G28|G30|G92|TLO|PRB):(.*)\]$").unwrap();

                // [VER:ver:note]
                static ref RE_VERSION: Regex = Regex::new(r"^\[VER:(.*):(.*)\]$").unwrap();

                // [OPT:codes]
                static ref RE_BUILD_OPTIONS: Regex = Regex::new(r"^\[OPT:(.*)\]$").unwrap();

                // <Idle|MPos:3.000,2.000,0.000|FS:0,0>
                // <Hold:0|MPos:5.000,2.000,0.000|FS:0,0>
                // <Idle|MPos:5.000,2.000,0.000|FS:0,0|Ov:100,100,100>
                // <Idle|MPos:5.000,2.000,0.000|FS:0,0|WCO:0.000,0.000,0.000>
                // <Run|MPos:23.036,1.620,0.000|FS:500,0>
                static ref RE_STATUS_REPORT: Regex = Regex::new(r"^<(.*)>$").unwrap();
            }

        if let Some(captures) = RE_RESPONSE_OK.captures(line) {
            return Self::parse_response_ok(captures);
        }

        if let Some(captures) = RE_RESPONSE_ERROR.captures(line) {
            return Self::parse_response_error(captures);
        }

        if let Some(captures) = RE_ALARM.captures(line) {
            return Self::parse_alarm(captures);
        }

        if let Some(captures) = RE_SETTING.captures(line) {
            return Self::parse_setting(captures);
        }

        if let Some(captures) = RE_STARTUP_LINE.captures(line) {
            return Self::parse_startup_line(captures);
        }

        if let Some(captures) = RE_FEEDBACK.captures(line) {
            return Self::parse_feedback(captures);
        }

        if let Some(captures) = RE_PARSER_STATE.captures(line) {
            return Self::parse_parser_state(captures);
        }

        if let Some(captures) = RE_HELP.captures(line) {
            return Self::parse_help(captures);
        }

        if let Some(captures) = RE_PARAMETER.captures(line) {
            return Self::parse_parameter(captures);
        }

        if let Some(captures) = RE_VERSION.captures(line) {
            return Self::parse_version(captures);
        }

        if let Some(captures) = RE_BUILD_OPTIONS.captures(line) {
            return Self::parse_build_options(captures);
        }

        if let Some(captures) = RE_STATUS_REPORT.captures(line) {
            return Self::parse_status_report(captures);
        }

        return Ok(GrblMessage::Other(line.to_owned()));
    }

    fn parse_response_ok(_: Captures) -> Result<Self, Error> {
        return Ok(GrblMessage::Response(GrblResponse::Ok));
    }

    fn parse_response_error(captures: Captures) -> Result<Self, Error> {
        let code = captures.get(1).unwrap().as_str().parse()?;

        return Ok(GrblMessage::Response(GrblResponse::Error(code)));
    }

    fn parse_alarm(captures: Captures) -> Result<Self, Error> {
        let code = captures.get(1).unwrap().as_str().parse()?;

        return Ok(GrblMessage::Alarm(code));
    }

    fn parse_setting(captures: Captures) -> Result<Self, Error> {
        let code = captures.get(1).unwrap().as_str().parse()?;
        let value = captures.get(2).unwrap().as_str().parse()?;

        return Ok(GrblMessage::Setting { code, value });
    }

    fn parse_startup_line(captures: Captures) -> Result<Self, Error> {
        let nr = captures.get(1).unwrap().as_str().parse()?;
        let line = captures.get(2).unwrap().as_str().parse()?;

        return Ok(GrblMessage::StartupLine { nr, line });
    }

    fn parse_feedback(captures: Captures) -> Result<Self, Error> {
        let line = captures.get(1).unwrap().as_str().to_owned();

        return Ok(GrblMessage::Feedback(line));
    }

    fn parse_parser_state(captures: Captures) -> Result<Self, Error> {
        // TODO: Parse to more detail
        let line = captures.get(1).unwrap().as_str().to_owned();

        return Ok(GrblMessage::ParserState(line));
    }

    fn parse_help(captures: Captures) -> Result<Self, Error> {
        let line = captures.get(1).unwrap().as_str().to_owned();

        return Ok(GrblMessage::Help(line));
    }

    fn parse_parameter(captures: Captures) -> Result<Self, Error> {
        // TODO: Parse to more detail
        let line = captures.get(1).unwrap().as_str().to_owned();

        return Ok(GrblMessage::Parameter(line));
    }

    fn parse_version(captures: Captures) -> Result<Self, Error> {
        let version = captures.get(1).unwrap().as_str().to_owned();
        let note = captures.get(2).unwrap().as_str().to_owned();

        return Ok(GrblMessage::Version { version, note });
    }

    fn parse_build_options(captures: Captures) -> Result<Self, Error> {
        // TODO: Parse to more detail
        let line = captures.get(1).unwrap().as_str().to_owned();

        return Ok(GrblMessage::BuildOptions(line));
    }

    fn parse_status_report(captures: Captures) -> Result<Self, Error> {
        // Split status into items
        let mut items = captures.get(1).unwrap().as_str()
            .split('|')
            .into_iter();

        // First item is always the machine state
        let machine_state = match items.next().unwrap() {
            "Idle" => GrblMachineState::Idle,
            "Run" => GrblMachineState::Run,
            "Hold:0" => GrblMachineState::Hold(GrblMachineHoldStatus::Complete),
            "Hold:1" => GrblMachineState::Hold(GrblMachineHoldStatus::InProgress),
            "Jog" => GrblMachineState::Jog,
            "Alarm" => GrblMachineState::Alarm,
            "Door:0" => GrblMachineState::Door(GrblMachineDoorStatus::Closed),
            "Door:1" => GrblMachineState::Door(GrblMachineDoorStatus::Open),
            "Door:2" => GrblMachineState::Door(GrblMachineDoorStatus::Holding),
            "Door:3" => GrblMachineState::Door(GrblMachineDoorStatus::Resuming),
            "Check" => GrblMachineState::Check,
            "Home" => GrblMachineState::Home,
            "Sleep" => GrblMachineState::Sleep,
            _ => unreachable!(),
        };

        // Parse all remaining items as `key:val`
        let mut items = items.map(|item| {
            let i = item.find(':').unwrap();

            let key = &item[..i];
            let val = &item[(i + 1)..];

            return (key, val);
        });

        // Second item is guaranteed to be always position
        let position = match items.next().unwrap() {
            ("MPos", val) => GrblPositionStatus::MachinePosition(Self::parse_position(val)),
            ("WPos", val) => GrblPositionStatus::WorkPosition(Self::parse_position(val)),
            _ => unreachable!(),
        };

        // Parse remaining items in arbitrary order
        let mut report = GrblStatusReport {
            machine_state,
            position,
            wco: None,
            buffer: None,
            line: None,
            feed: None,
            speed: None,
            input_pins: None,
            overrides: None,
            accessory: None,
        };

        for item in items {
            match item {
                ("WCO", val) => {
                    report.wco = Some(Self::parse_position(val));
                }
                ("Bf", val) => {
                    let parts = Self::parse_parts(val);
                    report.buffer = Some(GrblBufferStatus {
                        planner: parts[0],
                        rx: parts[1],
                    });
                }
                ("Ln", val) => {
                    let parts = Self::parse_parts(val);
                    report.line = Some(parts[0]);
                }
                ("F", val) => {
                    let parts = Self::parse_parts(val);
                    report.feed = Some(parts[0]);
                }
                ("FS", val) => {
                    let parts = Self::parse_parts(val);
                    report.feed = Some(parts[0]);
                    report.speed = Some(parts[1]);
                }
                ("Pn", val) => {
                    let input_pins = val.chars()
                        .fold(GrblInputPinsStatus::default(), |mut pins, char| {
                            match char {
                                'X' => pins.x_limit = true,
                                'Y' => pins.y_limit = true,
                                'Z' => pins.z_limit = true,
                                'P' => pins.probe = true,
                                'D' => pins.door = true,
                                'H' => pins.hold = true,
                                'R' => pins.soft_reset = true,
                                'S' => pins.cycle_start = true,
                                _ => unreachable!(),
                            }

                            return pins;
                        });

                    report.input_pins = Some(input_pins);
                }
                ("Ov", val) => {
                    let parts = Self::parse_parts(val);
                    report.overrides = Some(GrblOverrridesStatus {
                        feed: parts[0],
                        rapids: parts[1],
                        speed: parts[2],
                    });
                }
                ("A", val) => {
                    let accessory = val.chars()
                        .fold(GrblAccessoryStatus::default(), |mut accessory, char| {
                            match char {
                                'S' => accessory.spindle = GrblSpindleStatus::CW,
                                'C' => accessory.spindle = GrblSpindleStatus::CCW,
                                'F' => accessory.flood_coolant = true,
                                'M' => accessory.mist_coolant = true,
                                _ => unreachable!(),
                            }

                            return accessory;
                        });

                    report.accessory = Some(accessory);
                }
                _ => unreachable!(),
            }
        }

        return Ok(GrblMessage::StatusReport(report));
    }

    fn parse_parts<T>(val: &str) -> Vec<T>
        where T: FromStr,
              T::Err: std::fmt::Debug {
        return val.split(',').into_iter()
            .map(|part| T::from_str(part).unwrap())
            .collect();
    }

    fn parse_position(val: &str) -> Position {
        let parts = Self::parse_parts(val);

        assert_eq!(parts.len(), 3);

        return Position {
            x: parts[0],
            y: parts[1],
            z: parts[2],
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ok() {
        assert_eq!(GrblMessage::parse("ok").unwrap(),
                   GrblMessage::Response(GrblResponse::Ok));
    }

    #[test]
    fn test_parse_error() {
        assert_eq!(GrblMessage::parse("error:0").unwrap(),
                   GrblMessage::Response(GrblResponse::Error(0)));
        assert_eq!(GrblMessage::parse("error:255").unwrap(),
                   GrblMessage::Response(GrblResponse::Error(255)));
    }

    #[test]
    fn test_parse_alarm() {
        assert_eq!(GrblMessage::parse("ALARM:0").unwrap(),
                   GrblMessage::Alarm(0));
        assert_eq!(GrblMessage::parse("ALARM:255").unwrap(),
                   GrblMessage::Alarm(255));
    }

    #[test]
    fn test_parse_setting() {
        assert_eq!(GrblMessage::parse("$13=0").unwrap(),
                   GrblMessage::Setting { code: 13, value: 0.0 });
        assert_eq!(GrblMessage::parse("$100=250.0").unwrap(),
                   GrblMessage::Setting { code: 100, value: 250.0 });
        assert_eq!(GrblMessage::parse("$12=0.002").unwrap(),
                   GrblMessage::Setting { code: 12, value: 0.002 });
        assert_eq!(GrblMessage::parse("$30=1000").unwrap(),
                   GrblMessage::Setting { code: 30, value: 1000.0 });
    }

    #[test]
    fn test_parse_startup_line() {
        assert_eq!(GrblMessage::parse("$N0=G54").unwrap(),
                   GrblMessage::StartupLine { nr: 0, line: "G54".to_owned() });
        assert_eq!(GrblMessage::parse("$N1=").unwrap(),
                   GrblMessage::StartupLine { nr: 1, line: "".to_owned() });
    }

    #[test]
    fn test_parse_feedback() {
        assert_eq!(GrblMessage::parse("[MSG:Reset to continue]").unwrap(),
                   GrblMessage::Feedback("Reset to continue".to_owned()));
        assert_eq!(GrblMessage::parse("[MSG:'$H'|'$X' to unlock]").unwrap(),
                   GrblMessage::Feedback("'$H'|'$X' to unlock".to_owned()));
    }

    #[test]
    fn test_parse_parser_state() {
        assert_eq!(GrblMessage::parse("[GC:G0 G54 G17 G21 G90 G94 M5 M9 T0 F0.0 S0]").unwrap(),
                   GrblMessage::ParserState("G0 G54 G17 G21 G90 G94 M5 M9 T0 F0.0 S0".to_owned()));
    }

    #[test]
    fn test_parse_help() {
        assert_eq!(GrblMessage::parse("[HLP:$$ $# $G $I $N $x=val $Nx=line $J=line $C $X $H ~ ! ? ctrl-x]").unwrap(),
                   GrblMessage::Help("$$ $# $G $I $N $x=val $Nx=line $J=line $C $X $H ~ ! ? ctrl-x".to_owned()));
    }

    #[test]
    fn test_parse_parameter() {
        // [G54:0.000,0.000,0.000]
        // [G55:0.000,0.000,0.000]
        // [G56:0.000,0.000,0.000]
        // [G57:0.000,0.000,0.000]
        // [G58:0.000,0.000,0.000]
        // [G59:0.000,0.000,0.000]
        // [G28:0.000,0.000,0.000]
        // [G30:0.000,0.000,0.000]
        // [G92:0.000,0.000,0.000]
        // [TLO:0.000]
        // [PRB:0.000,0.000,0.000:0]
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(GrblMessage::parse("[VER:1.1d.20161014:]").unwrap(),
                   GrblMessage::Version { version: "1.1d.20161014".to_owned(), note: "".to_owned() });
        assert_eq!(GrblMessage::parse("[VER:1.1d.20161014:carbide rocks]").unwrap(),
                   GrblMessage::Version { version: "1.1d.20161014".to_owned(), note: "carbide rocks".to_owned() });
    }

    #[test]
    fn test_parse_build_options() {
        assert_eq!(GrblMessage::parse("[OPT:VL,15,128]").unwrap(),
                   GrblMessage::BuildOptions("VL,15,128".to_owned()));
    }

    #[test]
    fn test_parse_status_report() {
        assert_eq!(GrblMessage::parse("<Idle|MPos:3.000,2.000,0.000|FS:0,0>").unwrap(),
                   GrblMessage::StatusReport(GrblStatusReport {
                       machine_state: GrblMachineState::Idle,
                       position: GrblPositionStatus::MachinePosition(Position::from((3.0, 2.0, 0.0))),
                       wco: None,
                       buffer: None,
                       line: None,
                       feed: Some(0.0),
                       speed: Some(0.0),
                       input_pins: None,
                       overrides: None,
                       accessory: None,
                   }));
        assert_eq!(GrblMessage::parse("<Hold:0|MPos:5.000,2.000,0.000|FS:0,0>").unwrap(),
                   GrblMessage::StatusReport(GrblStatusReport {
                       machine_state: GrblMachineState::Hold(GrblMachineHoldStatus::Complete),
                       position: GrblPositionStatus::MachinePosition(Position::from((5.0, 2.0, 0.0))),
                       wco: None,
                       buffer: None,
                       line: None,
                       feed: Some(0.0),
                       speed: Some(0.0),
                       input_pins: None,
                       overrides: None,
                       accessory: None,
                   }));
        assert_eq!(GrblMessage::parse("<Idle|WPos:5.000,2.000,0.000|FS:0,0|Ov:100,100,100>").unwrap(),
                   GrblMessage::StatusReport(GrblStatusReport {
                       machine_state: GrblMachineState::Idle,
                       position: GrblPositionStatus::WorkPosition(Position::from((5.0, 2.0, 0.0))),
                       wco: None,
                       buffer: None,
                       line: None,
                       feed: Some(0.0),
                       speed: Some(0.0),
                       input_pins: None,
                       overrides: Some(GrblOverrridesStatus {
                           feed: 100.0,
                           rapids: 100.0,
                           speed: 100.0,
                       }),
                       accessory: None,
                   }));
        assert_eq!(GrblMessage::parse("<Idle|MPos:5.000,2.000,0.000|FS:0,0|WCO:0.000,0.000,0.000>").unwrap(),
                   GrblMessage::StatusReport(GrblStatusReport {
                       machine_state: GrblMachineState::Idle,
                       position: GrblPositionStatus::MachinePosition(Position::from((5.0, 2.0, 0.0))),
                       wco: Some(Position::from((0.0, 0.0, 0.0))),
                       buffer: None,
                       line: None,
                       feed: Some(0.0),
                       speed: Some(0.0),
                       input_pins: None,
                       overrides: None,
                       accessory: None,
                   }));
        assert_eq!(GrblMessage::parse("<Run|MPos:23.036,1.620,0.000|F:500>").unwrap(),
                   GrblMessage::StatusReport(GrblStatusReport {
                       machine_state: GrblMachineState::Run,
                       position: GrblPositionStatus::MachinePosition(Position::from((23.036, 1.620, 0.0))),
                       wco: None,
                       buffer: None,
                       line: None,
                       feed: Some(500.0),
                       speed: None,
                       input_pins: None,
                       overrides: None,
                       accessory: None,
                   }));
        assert_eq!(GrblMessage::parse("<Run|MPos:5.000,2.000,0.000|Ln:99999|Bf:15,128>").unwrap(),
                   GrblMessage::StatusReport(GrblStatusReport {
                       machine_state: GrblMachineState::Run,
                       position: GrblPositionStatus::MachinePosition(Position::from((5.0, 2.0, 0.0))),
                       wco: None,
                       buffer: Some(GrblBufferStatus {
                           planner: 15,
                           rx: 128,
                       }),
                       line: Some(99999),
                       feed: None,
                       speed: None,
                       input_pins: None,
                       overrides: None,
                       accessory: None,
                   }));
        assert_eq!(GrblMessage::parse("<Idle|MPos:5.000,2.000,0.000|Pn:XYZR>").unwrap(),
                   GrblMessage::StatusReport(GrblStatusReport {
                       machine_state: GrblMachineState::Idle,
                       position: GrblPositionStatus::MachinePosition(Position::from((5.0, 2.0, 0.0))),
                       wco: None,
                       buffer: None,
                       line: None,
                       feed: None,
                       speed: None,
                       input_pins: Some(GrblInputPinsStatus {
                           x_limit: true,
                           y_limit: true,
                           z_limit: true,
                           probe: false,
                           door: false,
                           hold: false,
                           soft_reset: true,
                           cycle_start: false
                       }),
                       overrides: None,
                       accessory: None,
                   }));
        assert_eq!(GrblMessage::parse("<Idle|MPos:5.000,2.000,0.000|A:SMF>").unwrap(),
                   GrblMessage::StatusReport(GrblStatusReport {
                       machine_state: GrblMachineState::Idle,
                       position: GrblPositionStatus::MachinePosition(Position::from((5.0, 2.0, 0.0))),
                       wco: None,
                       buffer: None,
                       line: None,
                       feed: None,
                       speed: None,
                       input_pins: None,
                       overrides: None,
                       accessory: Some(GrblAccessoryStatus {
                           spindle: GrblSpindleStatus::CW,
                           flood_coolant: true,
                           mist_coolant: true,
                       }),
                   }));
    }
}