use failure::Error;
use futures::Future;
use futures::Stream;
use futures::sync::oneshot;
use serde::de::DeserializeOwned;

use crate::position::Position;
use crate::server;

pub mod grbl;

#[derive(Debug, Clone)]
pub enum Response {
    Ok,
    Error(String),
}

pub type Canceled = oneshot::Canceled;

pub trait Sender {
    fn send_line(&self, line: &str) -> Box<Future<Item=Response, Error=Canceled> + Send>;
}

pub trait Controller: Send {
    // TODO: Allow to get raw stream of controller output for terminal

    fn description(&self) -> (server::ControllerType, &str);

    fn sender(&self) -> Box<Sender + Send>;

    fn state(&self) -> Box<Stream<Item=State, Error=()> + Send>;
}

#[derive(Debug, Clone)]
pub enum HoldStatus {
    Complete,
    InProgress,
}

#[derive(Debug, Clone)]
pub enum DoorStatus {
    Closed,
    Open,
    Holding,
    Resuming,
}

#[derive(Debug, Clone)]
pub enum MachineStatus {
    Idle,
    Run,
    Hold(HoldStatus),
    Jog,
    Alarm,
    Door(DoorStatus),
    Check,
    Home,
    Sleep,
}

#[derive(Debug, Clone)]
pub struct State {
    pub status: MachineStatus,

    pub machine_position: Position,
    pub work_position: Position,
}