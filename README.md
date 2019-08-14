ESS Reader (ess_reader)
-----------------------

ESS Reader reads various stats from the system page of Hansol Technics AIO (ex-Samsung ESS AIO) energy storage
and stores them in a database.

This Repository and rust crate consists of the ess_reader library and the ess-reader binary.

Currently only influxdb is supported. May support other ESS some day.
### Usage
```
$ ess-reader.exe [FLAGS] [OPTIONS]
$ ess-reader [FLAGS] [OPTIONS]

FLAGS:
        --cron       Pass when called from cron or systmed.timer
    -d, --debug      Debug mode
    -h, --help       Prints help information
        --json       Output as JSON
        --print      Output only
    -V, --version    Prints version information

OPTIONS:
        --db <db>                      Influxdb database name
        --db_host <db_host>            IP or Hostname of influxdb server
        --db_password <db_password>    Password if required
        --db_user <db_user>            User if required
        --ip <ip>                      IP of the ESS
```
### Installation
ESS Reader can be installed in various way.

Download a prebuilt binary from [GitHub][https://github.com/valkum/ess_reader/releases]


If you are a **Rust programmer**, ess_reader can be installed with `cargo`.
```
$ cargo install ess_reader
```
### Building
ESS Reader is written in Rust, so you'll need the rust compiler to build it.
If you are new rustup is a good way to install and manage a rust installation.

```
$ git clone https:/github.com/valkum/ess_reader
$ cd ess_reader
$ cargo build --release
$ ./target/release/ess_reader --version
```

### Configuration
A configuration file is created at the following paths. 
A [example config][extras/ess_reader.toml] can be found in the extras directory.
* MacOS: /Users/<User>/Library/Preferences/rs.ess_reader.ess_reader/ess_reader.toml
* Windows: %APPDATA%/ess_reader/ess_reader/ess_reader.toml
* Linux: /home/<User>/.config/ess_reader/ess_reader.toml

### Running with Cron or Systemd
Make sure that your configuration is correct.
You can create a simple cronjob calling ess-reader --cron or use the [supplied][extras] systemd.timer configs.
