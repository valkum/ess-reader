use structopt::StructOpt;
use confy;
use prettytable::{Table, row, cell, table};
use failure::Error;

use ess_reader::{Error as EssError, Config, CurrentStats, BackendClient};
use fern;
use log;
use log::{warn, debug};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::thread;
use ctrlc;

#[derive(Debug, StructOpt)]
#[structopt(name = "ESS Reader", author = "Rudi Floren", about = "Save stats from your Hansol Technics AIO (ex-Samsung ESS AIO)")]
struct CmdArgs {
    /// Debug mode
    #[structopt(short = "d", long = "debug")]
    debug: bool,
    /// Pass when called from cron or systmed.timed
    #[structopt(long = "cron")]
    cron: bool,
    /// Output only
    #[structopt(long = "print")]
    print: bool,
    /// Output as JSON
    #[structopt(long = "json")]
    json: bool,
    /// IP of the ESS
    #[structopt(long = "ip")]
    ip: Option<String>,
    /// IP or Hostname of influxdb server
    #[structopt(long = "db_host")]
    db_host: Option<String>,
    /// Influxdb database name
    #[structopt(long = "db")]
    db: Option<String>,
    /// User if required
    #[structopt(long = "db_user")]
    db_user: Option<String>,
    /// Password if required
    #[structopt(long = "db_password")]
    db_password: Option<String>,
}



fn main() {
    
    let mut cfg: Config = confy::load("ess-reader").unwrap();

    let args = CmdArgs::from_args();
    if let Some(ref ip) = args.ip {
        cfg.ip = ip.clone();
    }
    let mut logger = fern::Dispatch::new();
    logger = if args.debug {
        logger
        .level(log::LevelFilter::Debug)
        .level_for("hyper", log::LevelFilter::Warn)
        .level_for("reqwest::async_impl::response", log::LevelFilter::Warn)
        .level_for("html5ever", log::LevelFilter::Warn)
        .level_for("tokio_reactor", log::LevelFilter::Warn)
    } else {
        logger.level(log::LevelFilter::Warn)
    };
    let stdout_config  = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.level(),
                record.target(),
                message,
            ))
        })
        .chain(std::io::stdout());

    logger.chain(stdout_config).apply().expect("Failed to start logging...");

    if args.db_host.is_some() {
        cfg.db_host = args.db_host.clone();
    }
    if args.db.is_some() {
        cfg.db = args.db.clone();
    }
    if args.db_user.is_some() {
        cfg.db_user = args.db_user.clone();
    }
    if args.db_password.is_some() {
        cfg.db_password = args.db_password.clone();
    }
   
    if cfg.ip.is_empty() {
        println!("No IP of ESS found");
        return
    }
    debug!("{:?}", cfg);

    if let Err(e) = run(&cfg, &args) {
        println!("Something went wrong \n{}", e.as_fail());
        debug!("{}", e.backtrace());
    }

}

fn run(config: &Config, args: &CmdArgs) -> Result<(), Error> {
    let stats = CurrentStats::get_from(&config.ip)?;
    if args.print && !args.json {
        let prod_table = table!(
            ["", "Voltage", "Current", "Power"],
            ["PV-1", format!("{} V", stats.inverter.pv1.voltage), format!("{} A", stats.inverter.pv1.current), format!("{} W", stats.inverter.pv1.power)],
            ["PV-2", format!("{} V", stats.inverter.pv2.voltage), format!("{} A", stats.inverter.pv2.current), format!("{} W", stats.inverter.pv2.power)],
            ["INV", format!("{} V", stats.inverter.inv.voltage), format!("{} A", stats.inverter.inv.current), format!("{} W", stats.inverter.inv.power)]
        );
        let mut table = Table::new();
        table.add_row(row!["Date", stats.time.to_rfc2822(), ]);
        table.add_row(row!["Load", format!("{} W", stats.battery.load),]);
        table.add_row(row!["Battery Filled", format!("{} %", stats.battery.filled), ]);
        table.add_row(row!["Grid (Withdrawal)", format!("{} W", stats.battery.withdrawal), ]);
        table.add_row(row!["Grid (Feedin)", format!("{} W", stats.battery.feedin), ]);
        table.add_row(row!["PV Production", format!("{} W", stats.battery.pv),]);
        table.add_row(row!["Production", prod_table]);
 
        table.printstd();
        return Ok(())
    }
    if config.db_host.is_some() {
        let client = ess_reader::InfluxClient::new(&config);
        client.send(&stats)?;
    } else {
        warn!("Database information are required for cron mode");
        return Err(EssError::Config("No DB host".to_string()).into())
    }
    if !args.cron {
        debug!("Starting daemon mode");
        let interval = Duration::from_millis(15000);

        let client = ess_reader::InfluxClient::new(&config);

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        }).expect("Error setting Ctrl-C handler");

        while running.load(Ordering::SeqCst) {
            // This loop will run once every second
            let stats = CurrentStats::get_from(&config.ip)?;
            client.send(&stats)?;
            thread::sleep(interval);
        }

    }
    Ok(())
}

