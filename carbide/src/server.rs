use std::sync::Arc;
use std::sync::Mutex;

use failure::Error;
use futures::Future;
use futures::sink::Sink;
use futures::stream::Stream;
use serde_derive::Serialize;
use warp::{self, Filter, Rejection, Reply};

use crate::config::ServerConfig;
use crate::controller;
use crate::position::Position;

#[derive(Debug, Clone, Serialize)]
pub enum ControllerType {
    Grbl,
}

#[derive(Debug, Clone, Serialize)]
pub struct Info {
    version: String,
    controller: ControllerType,
    description: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum MachineStatus {
    Idle,
    Run,
    HoldComplete,
    HoldInProgress,
    Jog,
    Alarm,
    DoorClosed,
    DoorOpen,
    DoorHolding,
    DoorResuming,
    Check,
    Home,
    Sleep,
}

impl From<controller::MachineStatus> for MachineStatus {
    fn from(status: controller::MachineStatus) -> Self {
        return match status {
            controller::MachineStatus::Idle => MachineStatus::Idle,
            controller::MachineStatus::Run => MachineStatus::Run,
            controller::MachineStatus::Hold(status) => match status {
                controller::HoldStatus::InProgress => MachineStatus::HoldInProgress,
                controller::HoldStatus::Complete => MachineStatus::HoldComplete,
            },
            controller::MachineStatus::Jog => MachineStatus::Jog,
            controller::MachineStatus::Alarm => MachineStatus::Alarm,
            controller::MachineStatus::Door(status) => match status {
                controller::DoorStatus::Closed => MachineStatus::DoorClosed,
                controller::DoorStatus::Open => MachineStatus::DoorOpen,
                controller::DoorStatus::Holding => MachineStatus::DoorHolding,
                controller::DoorStatus::Resuming => MachineStatus::DoorResuming,
            },
            controller::MachineStatus::Check => MachineStatus::Check,
            controller::MachineStatus::Home => MachineStatus::Home,
            controller::MachineStatus::Sleep => MachineStatus::Sleep,
        };
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ControllerState {
    pub status: MachineStatus,

    pub machine_position: (f64, f64, f64),
    pub work_position: (f64, f64, f64),
}

impl From<controller::State> for ControllerState {
    fn from(state: controller::State) -> Self {
        return ControllerState {
            status: state.status.into(),
            machine_position: state.machine_position.into(),
            work_position: state.work_position.into(),
        };
    }
}

fn info(controller: Arc<Mutex<controller::Controller>>) -> impl warp::Reply {
    let controller = controller.lock().unwrap();

    let description = controller.description();

    return warp::reply::json(&Info {
        version: format!("{} - {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        controller: description.0, // FIXME
        description: description.1.to_owned(),
    });
}

fn state(controller: Arc<Mutex<controller::Controller>>, ws: warp::ws::Ws2) -> impl warp::Reply {
    return ws.on_upgrade(move |socket| {
        let (sink, stream) = socket.split();

        let state = controller.lock().unwrap().state()
            .map(|state| {
                let state: ControllerState = state.into();
                let state = serde_json::to_string(&state).unwrap();

                return warp::ws::Message::text(state);
            });

        return sink
            .sink_map_err(|err| log::warn!("Socket closed: {}", err) )
            .send_all(state)
            .map(|_| ());
    });
}

pub fn serve(config: &ServerConfig,
             controller: Arc<Mutex<controller::Controller>>) -> impl Future<Item=(), Error=()> {
    let controller = warp::any().map(move || controller.clone());

    let info = warp::get2()
        .and(warp::path("info"))
        .and(controller.clone())
        .map(info);

    let state = warp::path("state")
        .and(controller.clone())
        .and(warp::ws2())
        .map(state);

    let api = warp::path("api")
        .and(info.or(state))
        .with(warp::log("carbide::server::api"));

    let fs = warp::fs::dir("web")
        .with(warp::log("carbide::server::fs"));

    let routes = api.or(fs)
        .with(warp::log("carbide::server"));

    return warp::serve(routes)
        .bind((config.host, config.port));
}