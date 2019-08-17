use influxdb::query::{InfluxDbQuery, Timestamp};
use influxdb::client::{InfluxDbClient};
use tokio;

use crate::{Config, CurrentStats};
use super::{BackendClient, ClientError};
use futures::Future;
use log::debug;

pub struct InfluxClient {
    client: InfluxDbClient,
}


impl<'a> BackendClient<'a> for InfluxClient {
    fn new(config: &'a Config) -> Self {
        let client;
        let db_host = config.db_host.as_ref().unwrap();
        let auth = if config.db_user.is_some() && config.db_password.is_some() {
            let client = InfluxDbClient::new(db_host, config.db.as_ref().unwrap()).with_auth(config.db_user, config.db_password);
        } else { 
            let client = InfluxDbClient::new(db_host, config.db.as_ref().unwrap());
        };

        InfluxClient {
            client
        }
    }
    fn send(&self, data: &CurrentStats) -> Result<(), ClientError> {
        let total_measurement = InfluxDbQuery::write_query(Timestamp::NOW, "battery")
            .add_field("filled", data.battery.filled)
            .add_field("battery", data.battery.battery)
            .add_field("pv", data.battery.pv)
            .add_field("withdrawal", data.battery.withdrawal)
            .add_field("feedin", data.battery.feedin)
            .add_field("inverter", data.battery.inverter)
            .add_field("load", data.battery.load)
            .add_field("temperature", data.battery.temperature);
        let inverter_pv1_m = InfluxDbQuery::write_query(Timestamp::NOW, "inverter")
            .add_tag("stat", "üv1")
            .add_field("voltage", data.inverter.pv1.voltage)
            .add_field("current", data.inverter.pv1.current)
            .add_field("power", data.inverter.pv1.power);
        let inv_q1 = self.client.query(&inverter_pv1_m);
        let inverter_pv2_m = InfluxDbQuery::write_query(Timestamp::NOW, "inverter")
            .add_tag("stat", "pv2")
            .add_field("voltage", data.inverter.pv2.voltage)
            .add_field("current", data.inverter.pv2.current)
            .add_field("power", data.inverter.pv2.power);
        let inv_q2 = self.client.query(&inverter_pv2_m);
        let inverter_inv_m = InfluxDbQuery::write_query(Timestamp::NOW, "inverter")
            .add_tag("stat", "inv")
            .add_field("inv.voltage", data.inverter.inv.voltage)
            .add_field("inv.current", data.inverter.inv.current)
            .add_field("inv.power", data.inverter.inv.power);
        let inv_q3 = self.client.query(&inverter_inv_m);

        let f1 = self.client.query(&total_measurement);
        let f2 = inv_q1.join3(inv_q2, inv_q3);
        let f = f1.join(f2);
        let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();
        debug!("Send measurement to InfluxDB");
        rt.block_on(f).map(|_| ()).map_err(|e| {
            ClientError::ConnectionError(e.to_string())
        })
    }
}