use reqwest;
use select::document::Document;
use select::node::Node;
use select::predicate::{Name, Text, Predicate};


use serde::{Serialize, Deserialize};
use failure::{Error as fError, Fail};
use chrono::{DateTime, Utc};
use log::debug;

pub use self::client::*;
mod client;

type Result<T> = std::result::Result<T, fError>;

#[derive(Serialize, Deserialize, Debug)]
pub  struct Config {
    pub ip: String,
    pub db: Option<String>,
    pub db_user: Option<String>,
    pub db_password: Option<String>,
    pub db_host: Option<String>,
    // Todo allow different databases
    // pub db_driver: , 
}
impl Default for  Config {
    fn default() -> Self { 
        Self { 
            ip: "".into(),
            db: None,
            db_user: None,
            db_password: None,
            db_host: None,
        } 
    }
}

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Connection Error: {}", _0)]
    Connection(#[fail(cause)] reqwest::Error),
    #[fail(display = "Config Error: {}", _0)]
    Config(String),
    #[fail(display = "{}", _0)]
    Parse(#[fail(cause)] std::io::Error),
    #[fail(display = "Float Parsing Error: {}", _0)]
    FloatParse(#[fail(cause)] std::num::ParseFloatError),
}

#[derive(Debug, Clone)]
pub struct CurrentStats {
    pub time: DateTime<Utc>,
    pub battery: EmsStats,
    pub inverter: InvStats,
}

impl Default for CurrentStats {
    fn default() -> Self {
        CurrentStats{
            time: Utc::now(),
            battery: EmsStats::default(),
            inverter: InvStats::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EmsStats {
    // Percentage loaded
    pub filled: f32,
    // Battery power in Watts
    pub battery: f32,
    // PV power in Watts
    pub pv: f32,
    // Grid withdrawal in Watts
    pub withdrawal: f32,
    // Grid feedin in Watts
    pub feedin: f32,
    // Inverter power in Watts
    pub inverter: f32,
    // Load in Watts
    pub load: f32,
    // Battery temperature
    pub temperature: f32,
}

enum InvColParseState {
    None,
    V,
    A,
    W,
}

#[derive(Debug, Clone, Default)]
pub struct PowerStats {
    // Current in Ampere
    pub current: f32,
    // Voltage in Volts
    pub voltage: f32,
    // Power in Watts
    pub power: f32,
}

impl PowerStats {
    fn from_row(row: Node) -> Result<PowerStats> {
        let mut col_state = InvColParseState::None;
        let mut stats = PowerStats::default();
        for td in row.find(Name("td")) {
            col_state = match td.text().as_str() {
                "V[V]:" => InvColParseState::V,
                "I[A]:" => InvColParseState::A,
                "P[W]:" => InvColParseState::W,
                _ => {
                    match col_state {
                        InvColParseState::V => {stats.voltage = td.text().trim().parse::<f32>()?}
                        InvColParseState::A => {stats.current = td.text().trim().parse::<f32>()?}
                        InvColParseState::W => {stats.power = td.text().trim().parse::<f32>()?}
                        InvColParseState::None => {}
                    }
                    InvColParseState::None
                }
            }
        }
        Ok(stats)
    }
}

#[derive(Debug, Clone, Default)]
pub struct InvStats {
    // IN: PV1
    pub pv1: PowerStats,
    // IN: PV2
    pub pv2: PowerStats,
    // OUT: Inverter
    pub inv: PowerStats,
}



enum EmsParseState {
    None,
    GRID_P,
    LOAD_P,
    PV_P,
    INV_P,
    BT_SOC,
    BT_P,
    Temp
}


impl CurrentStats {

    pub fn get_from(ip: &String) -> Result<CurrentStats> {
        debug!("Get stats from ess");
        let url = format!("http://{}:21710/f0", ip);
        let resp = match reqwest::get(&url) {
            Ok(resp) => resp,
            Err(err) => {
                debug!("Failed to get stats from {}", url);
                return Err(err.into())
            }
        };


        let document = Document::from_read(resp)?;

        let mut stats = CurrentStats::default();
        stats.time = Utc::now();

        debug!("Parse stats");
        stats.battery = Self::parse_ems(&document)?;
        stats.inverter = Self::parse_inv(&document)?;
        Ok(stats)
    }

    fn parse_ems(document: &Document) -> Result<EmsStats> {
        let mut ems_table = None;
        let mut stats = EmsStats::default();
        // finding table of interest
        for table in document.find(Name("table")) {
            if table.find(Name("tr").descendant(Text("EMS Control MODE"))).next().is_some() {
                ems_table = Some(table);
            }
        }
        let mut state = EmsParseState::None;
        for td in ems_table.expect("Could not get EMS Control MODE table").find(Name("td")) {
            state = match td.text().as_str() {
                "GRID_P" => EmsParseState::GRID_P,
                "LOAD_P" => EmsParseState::LOAD_P,
                "PV_P" => EmsParseState::PV_P,
                "INV_P" => EmsParseState::INV_P,
                "BT_P" => EmsParseState::BT_P,
                "BT_SOC" => EmsParseState::BT_SOC,
                "Temp" => EmsParseState::Temp,
                _ => {
                    match state {
                        EmsParseState::GRID_P => {
                            let grid : f32 = td.text().trim().parse::<f32>()?;
                            if grid >= 0.0 {
                                stats.withdrawal = grid;
                            } else {
                                stats.feedin = grid * -1.0;
                            }
                        }   
                        EmsParseState::LOAD_P => {stats.load = td.text().trim().parse::<f32>()?}
                        EmsParseState::PV_P => {stats.pv = td.text().trim().parse::<f32>()?}
                        EmsParseState::INV_P => {stats.inverter = td.text().trim().parse::<f32>()?}
                        EmsParseState::BT_SOC => {stats.filled = td.text().trim().parse::<f32>()?}
                        EmsParseState::BT_P => {stats.battery = td.text().trim().parse::<f32>()?}
                        EmsParseState::Temp => {stats.temperature = td.text().trim().parse::<f32>()?}
                        EmsParseState::None => {}
                    }
                    EmsParseState::None
                }
            }
        }
        Ok(stats)
    }
    fn parse_inv(document: &Document) -> Result<InvStats> {
        let mut pcs_table = None;
        let mut stats = InvStats::default();
        // finding table of interest
        for table in document.find(Name("table")) {
            if table.find(Name("tr").descendant(Text("PCS Sensing Data"))).next().is_some() {
                pcs_table = Some(table);
            }
        }
        
        for tr in pcs_table.expect("Could not get PCS Sensing Data table").find(Name("tr")) {
            match tr.find(Name("td")).next().unwrap().text().as_str() {
                "PV-1" => {
                    stats.pv1 = PowerStats::from_row(tr)?
                }
                "PV-2" => {
                    stats.pv2 = PowerStats::from_row(tr)?
                }
                "INV" => {
                    stats.inv = PowerStats::from_row(tr)?
                }
                _ => {}
            }
        }
        Ok(stats)
    }
}


