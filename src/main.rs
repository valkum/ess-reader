use structopt::StructOpt;
use confy;
use prettytable::{Table, row, cell, table};
use failure::Error;

use ess_reader::{Config, CurrentStats, BackendClient};



#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct CmdArgs {
    /// Activate debug mode
    #[structopt(short = "d", long = "debug")]
    debug: bool,
    // Output as JSON
    #[structopt(long = "json")]
    json: bool,
    // IP of the ESS
    #[structopt(long = "ip")]
    ip: Option<String>,
    // Send to this influxdb host
    #[structopt(long = "db_host")]
    db_host: Option<String>,
    #[structopt(long = "db")]
    db: Option<String>,
    #[structopt(long = "db_user")]
    db_user: Option<String>,
    #[structopt(long = "db_password")]
    db_password: Option<String>,
}



fn main() {
    let mut cfg: Config = confy::load("ess_reader").unwrap();

    let args = CmdArgs::from_args();
    if let Some(ref ip) = args.ip {
        cfg.ip = ip.clone();
    }
    cfg.db_host = args.db_host.clone();
    cfg.db = args.db.clone();
    cfg.db_user = args.db_user.clone();
    cfg.db_password = args.db_password.clone();
   
   if cfg.ip.is_empty() {
       println!("No IP of ESS found");
       return
   }

    if let Err(e) = run(&cfg, &args) {
        println!("Something went wrong {}", e.as_fail());
        print!("{}", e.backtrace());
    }

}

fn run(config: &Config, args: &CmdArgs) -> Result<(), Error> {
    let stats = CurrentStats::get_from(&config.ip)?;
    if args.debug {
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
    }

    if config.db_host.is_some() {
        let client = ess_reader::InfluxClient::new(&config);
        client.send(&stats)?;
    }
    Ok(())
}

