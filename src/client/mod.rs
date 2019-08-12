pub use self::influx::InfluxClient;
mod influx;

use failure::{self, Error, Fail};
use crate::{Config, CurrentStats};

#[derive(Debug, Fail)]
pub enum ClientError {
    #[fail(display = "Connection Error: {}", _0)]
    ConnectionError(String),
}
pub trait BackendClient<'a> {
    fn new(config: &'a Config) -> Self;
    fn send(&self, stats: &CurrentStats) -> Result<(), ClientError>;
}