use influxdb::query::{InfluxDbQuery, Timestamp};
use influxdb::client::{InfluxDbAuthentication, InfluxDbClient};
use tokio;

use crate::{Config, CurrentStats};
use super::{BackendClient, ClientError};
use futures::Future;

pub struct InfluxClient {
    client: InfluxDbClient,
}


impl<'a> BackendClient<'a> for InfluxClient {
    fn new(config: &'a Config) -> Self {
        
        let auth = InfluxDbAuthentication::new(
            config.db_user.as_ref().unwrap(),
            config.db_password.as_ref().unwrap()
        );
        let db_host = config.db_host.as_ref().unwrap();

        let client =  InfluxDbClient::new(db_host, config.db.as_ref().unwrap(), Some(auth));;
        InfluxClient {
            client
        }
    }
    fn send(&self, data: &CurrentStats) -> Result<(), ClientError> {
        let measurement = InfluxDbQuery::write_query(Timestamp::NOW, "battery")
            .add_field("filled", data.battery.filled)
            .add_field("battery", data.battery.battery)
            .add_field("pv", data.battery.pv)
            .add_field("withdrawal", data.battery.withdrawal)
            .add_field("feedin", data.battery.feedin)
            .add_field("inverter", data.battery.inverter)
            .add_field("load", data.battery.load)
            .add_field("temperature", data.battery.temperature);
        let measurement2 = InfluxDbQuery::write_query(Timestamp::NOW, "inverter")
            .add_field("pv1_voltage", data.inverter.pv1.voltage)
            .add_field("pv1.current", data.inverter.pv1.current)
            .add_field("pv1.power", data.inverter.pv1.power)
            .add_field("pv2.voltage", data.inverter.pv2.voltage)
            .add_field("pv2.current", data.inverter.pv2.current)
            .add_field("pv2.power", data.inverter.pv2.power)
            .add_field("inv.voltage", data.inverter.inv.voltage)
            .add_field("inv.current", data.inverter.inv.current)
            .add_field("inv.power", data.inverter.inv.power);

        let f1 = self.client.query(&measurement);
        let f2 = self.client.query(&measurement2);
        let f = f1.join(f2);
        let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();
        
        rt.block_on(f).map(|_| ()).map_err(|e| {
            ClientError::ConnectionError(e.to_string())
        })
    }
}