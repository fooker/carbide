use serde_derive::Deserialize;

mod proto;
mod codes;
mod buffer;
mod state;
mod controller;

#[derive(Debug, Clone, Deserialize)]
pub struct GrblControllerConfig {
    pub path: String,
}

pub use self::controller::GrblController;



