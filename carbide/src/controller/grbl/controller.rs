use std::time::Duration;

use bytes::Bytes;
use failure::Error;
use futures::Async;
use futures::Future;
use futures::future;
use futures::IntoFuture;
use futures::Sink;
use futures::Stream;
use futures::sync::mpsc;
use futures::sync::oneshot;
use tokio::codec::BytesCodec;
use tokio::codec::FramedRead;
use tokio::codec::FramedWrite;
use tokio::codec::LinesCodec;
use tokio::io::AsyncRead;
use tokio::sync::watch;
use tokio::timer::Interval;
use tokio_serial as serial;

use crate::controller;
use crate::utils::stream::broadcast::Broadcast;

use super::buffer;
use super::GrblControllerConfig;
use super::proto;
use super::codes;
use super::state::State;
use crate::server;

pub struct GrblController {
    description: String,

    // Sender used to send line commands to the controller
    lines: mpsc::UnboundedSender<(proto::GrblLineCommand, oneshot::Sender<proto::GrblResponse>)>,

    // Sender used to send realtime commands
    realtime: mpsc::UnboundedSender<proto::GrblRealtimeCommand>,

    // State changes of the controller
    state: watch::Receiver<controller::State>,
}

impl GrblController {
    const PORT_SETTINGS: serial::SerialPortSettings = serial::SerialPortSettings {
        baud_rate: 115200,
        data_bits: serial::DataBits::Eight,
        flow_control: serial::FlowControl::None,
        parity: serial::Parity::None,
        stop_bits: serial::StopBits::One,
        timeout: Duration::from_millis(1),
    };

    // GRBL docs recommend 5Hz
    const STATUS_INTERVAL: Duration = Duration::from_millis(1000 / 5);

    pub fn new(config: &GrblControllerConfig) -> Result<(Self, impl Future<Item=(), Error=Error>), Error> {
        // Create channel for sending commands
        let (line_sender, line_receiver) = mpsc::unbounded();
        let (realtime_sender, realtime_receiver) = mpsc::unbounded();

        // Process line commands through streamer to avoid buffer underflow
        let line_receiver = line_receiver
            .map(|cmd: (proto::GrblLineCommand, oneshot::Sender<proto::GrblResponse>)| (format!("{}\n", cmd.0.into_string()), cmd.1));
        let (line_receiver, line_tracker) = buffer::sender(line_receiver);

        // Send status queries request commands to controller every now and then
        let status_poller = Interval::new_interval(Self::STATUS_INTERVAL)
            .map(|_| proto::GrblRealtimeCommand::StatusReportQuery)
            .forward(realtime_sender.clone()
                .sink_map_err(|_| tokio::timer::Error::shutdown()))
            .map(|_| ())
            .map_err(Error::from);

        // Intermix lines with realtime commands
        let receiver = Stream::select(
            line_receiver.map(|line| Bytes::from(line)),
            realtime_receiver.map(|cmd| Bytes::from(cmd.to_code())),
        );

        // Open serial port
        let mut port = serial::Serial::from_path(&config.path, &Self::PORT_SETTINGS)?;
        port.set_exclusive(false)?;

        let (reader, writer) = port.split();

        log::info!("GRBL: Port opened");

        let writer = FramedWrite::new(writer, BytesCodec::new());

        // Wake up controller by sending some newlines
        // let writer = writer.send(Bytes::from_static(b"\r\n\r\n"));
        // TODO: Send some newlines and sleep a second to wake up controller

        // Write commands to controller
        let writer = receiver
            .map_err(|_| unreachable!()) // FIXME
            .inspect(|cmd| log::trace!("GRBL > {:?}", cmd))
            .fold(writer, |writer, cmd| {
                // Flush after each send
                return writer.send(cmd)
                    .and_then(Sink::flush)
                    .map_err(Error::from);
            })
            .map(|_| ());

        // Handle incoming messages from controller
        let reader = FramedRead::new(reader, LinesCodec::new())
            .map_err(Error::from)
            .inspect(|msg| log::trace!("GRBL < {:?}", msg))
            .and_then(|msg| proto::GrblMessage::parse(&msg).into_future())
            .inspect(|msg| log::trace!("GRBL << {:?}", msg));

        let mut reader = Broadcast::new(reader);

        // Handle response messages
        let response_handler = reader.receive()
            .filter_map(|msg| match msg {
                proto::GrblMessage::Response(response) => Some(response),
                _ => None,
            })
            .forward(line_tracker)
            .map_err(|_| unreachable!())
            .map(|_| ());

        // Handle state updates
        let (state, state_watch) = State::new();
        let state_handler = reader.receive()
            .forward(state)
            .map_err(|_| unreachable!())
            .map(|_| ());

        let driver = future::join_all::<Vec<Box<Future<Item=(), Error=Error> + Send>>>(vec![
            Box::new(reader),
            Box::new(writer),
            Box::new(status_poller),
            Box::new(response_handler),
            Box::new(state_handler),
        ]).map(|_| ());

        return Ok((Self {
            description: format!("GRBL: {}", &config.path),
            lines: line_sender,
            realtime: realtime_sender,
            state: state_watch,
        }, driver));
    }
}

impl controller::Controller for GrblController {
    fn description(&self) -> (server::ControllerType, &str) {
        return (server::ControllerType::Grbl, &self.description);
    }

    fn sender(&self) -> Box<controller::Sender + Send> {
        return Box::new(GrblSender(self.lines.clone()));
    }

    fn state(&self) -> Box<Stream<Item=controller::State, Error=()> + Send> {
        return Box::new(self.state.clone()
            .map_err(|_| unreachable!()));
    }
}

struct GrblSender(mpsc::UnboundedSender<(proto::GrblLineCommand, oneshot::Sender<proto::GrblResponse>)>);

impl controller::Sender for GrblSender {
    fn send_line(&self, line: &str) -> Box<Future<Item=controller::Response, Error=controller::Canceled> + Send> {
        let (sender, receiver) = oneshot::channel();

        self.0.unbounded_send((proto::GrblLineCommand::Line(line.to_owned()), sender))
            .unwrap();

        return Box::new(receiver.map(|response| {
            return match response {
                proto::GrblResponse::Ok => controller::Response::Ok,
                proto::GrblResponse::Error(code) => controller::Response::Error(codes::ERROR_CODES.get(&code).unwrap().to_string()),
            };
        }));
    }
}
