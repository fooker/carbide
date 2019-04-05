#![feature(never_type)]

use std::sync::Arc;
use std::sync::Mutex;

use failure::Error;
use futures::future;
use futures::Future;
use futures::Stream;
use log::debug;
use tokio;

use crate::config::Config;
use crate::controller::Controller;
use std::io::BufReader;

mod controller;
mod server;
mod config;
mod position;
mod utils;

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let config = Config::load("config.yaml")?;
    debug!("config loaded: {:?}", config);

    let mut runtime = tokio::runtime::Runtime::new()?;

    let (controller, driver) = match config.controller {
        config::ControllerConfig::GRBL(config) => controller::grbl::GrblController::new(&config)?,
    };

    let driver = driver.map_err(|err| panic!(err));

    let controller = Arc::new(Mutex::new(controller));

    let sender = controller.lock().unwrap().sender();
    let stdin = BufReader::new(tokio::io::stdin());
    let stdin = tokio::io::lines(stdin)
        .map_err(|err| panic!(err))
        .inspect(|line| println!("$ '{}'", line) )
        .for_each(move |line| {
            return sender.send_line(&line)
                .map(|response| {
                    println!(": {:?}", response);
                })
                .map_err(|err| panic!(err));
        });
    runtime.spawn(stdin);

    let server = server::serve(&config.server, controller.clone());
    runtime.spawn(server);

    runtime.block_on_all(driver);

    return Ok(());
}
