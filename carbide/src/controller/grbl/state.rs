use futures::Async;
use futures::AsyncSink;
use futures::Sink;
use tokio::sync::watch;

use crate::controller;
use crate::position::Position;

use super::codes;
use super::proto;

#[derive(Debug, Copy, Clone)]
enum Unit {
    Millimeter,
    Inch,
}

impl Unit {
    pub fn metricize(&self, pos: Position) -> Position {
        return match self {
            Unit::Millimeter => pos,
            Unit::Inch => pos * 25.4,
        };
    }
}

#[derive(Debug)]
pub struct State {
    unit: Unit,

    wco: Position,

    sender: watch::Sender<controller::State>,
}

impl State {
    pub fn new() -> (Self, watch::Receiver<controller::State>) {
        let (sender, receiver) = watch::channel(controller::State {
            status: controller::MachineStatus::Idle,
            machine_position: Position::zero(),
            work_position: Position::zero(),
        });

        return (Self {
            unit: Unit::Millimeter,
            wco: Position::zero(),
            sender,
        }, receiver);
    }

    fn handle(&mut self, msg: proto::GrblMessage) {
        match msg {
            proto::GrblMessage::Setting {code, value} => {
                // Remember configured unit
                if code == codes::SETTING_CODE_REPORT_IN_INCHES {
                    self.unit = if (value as usize) == 0 { Unit::Millimeter } else { Unit::Inch };
                }
            }

            proto::GrblMessage::StatusReport(status) => {
                if let Some(wco) = status.wco {
                    self.wco = self.unit.metricize(wco);
                }

                let (mpos, wpos) = match status.position {
                    proto::GrblPositionStatus::MachinePosition(mpos) => {
                        (mpos, mpos - self.wco)
                    }
                    proto::GrblPositionStatus::WorkPosition(wpos) => {
                        (wpos + self.wco, wpos)
                    }
                };

                let state = controller::State {
                    status: status.machine_state.into(),
                    machine_position: mpos,
                    work_position: wpos,
                };

                self.sender.broadcast(state)
                    .expect("Failed to broadcast state");
            }

            _ => {}
        };
    }
}

impl Sink for State {
    type SinkItem = proto::GrblMessage;
    type SinkError = ();

    fn start_send(&mut self, item: Self::SinkItem) -> Result<AsyncSink<Self::SinkItem>, Self::SinkError> {
        self.handle(item);
        return Ok(AsyncSink::Ready);
    }

    fn poll_complete(&mut self) -> Result<Async<()>, Self::SinkError> {
        return Ok(Async::Ready(()));
    }

    fn close(&mut self) -> Result<Async<()>, Self::SinkError> {
        return Ok(Async::Ready(()));
    }
}

