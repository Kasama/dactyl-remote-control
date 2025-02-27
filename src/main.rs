mod config;
mod i3;
mod keyboard;

use clap::Parser;
use clap_num::maybe_hex;
use log::{debug, error};

use crate::i3::I3Ext;

use self::keyboard::{HidInfo, Keyboard, KeyboardResponse, Operation};

const VENDOR_ID: u16 = 0x4b41; // Kasama
const PRODUCT_ID: u16 = 0x636D; // Dactyl
// const VENDOR_ID: u16 = 0x444D; // Tshort
// const PRODUCT_ID: u16 = 0x3435; // Dactyl Manuform

const USAGE_PAGE: u16 = 0xff60; // QMK default
const USAGE: u16 = 0x61; // QMK default

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct App {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, action = clap::ArgAction::Count)]
    /// Increases log verbosity on each appearance. -vvv will print out trace logs
    verbose: u8,

    #[arg(short, long)]
    /// Suppress all output when set
    quiet: bool,

    #[arg(long, default_value_t = VENDOR_ID, value_parser=maybe_hex::<u16>)]
    /// HID Vendor ID
    vid: u16,
    #[arg(long, default_value_t = PRODUCT_ID, value_parser=maybe_hex::<u16>)]
    /// HID Product ID
    pid: u16,
    #[arg(short = 'p', long, default_value_t = USAGE_PAGE, value_parser=maybe_hex::<u16>)]
    /// HID Usage Page
    usage_page: u16,
    #[arg(short, long, default_value_t = USAGE, value_parser=maybe_hex::<u16>)]
    /// HID Usage
    usage: u16,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    PrintKeyboardLayer,
    KeyboardBootloader,
    WatchI3Focus {
        #[arg(long, default_value = "false")]
        create_config: bool,
        #[arg(short, long)]
        config: Option<String>,
    },
    ChangeKeyboardLayer {
        layer: u8,
    },
    EnableMouseJiggle,
    DisableMouseJiggle,
}

fn print_error<T, E: std::fmt::Debug>(r: Result<T, E>) {
    r.map(|_| ()).unwrap_or_else(|e| error!("Error: {:?}", e));
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), anyhow::Error> {
    let app = App::parse();

    if !app.quiet {
        simple_logger::init_with_level(match app.verbose {
            0 => log::Level::Error,
            1 => log::Level::Info,
            2 => log::Level::Debug,
            3.. => log::Level::Trace,
        })?;
    }

    match app.command {
        Commands::PrintKeyboardLayer => print_error(app.print_keyboard_layer()),
        Commands::KeyboardBootloader => print_error(app.keyboard_bootloader()),
        Commands::WatchI3Focus {
            create_config,
            ref config,
        } => {
            if create_config {
                return Ok(());
            }
            if let Some(config) = config {
                let config = config::I3WatcherConfig::load_config(config)?;
                print_error(app.watch_i3_focus(config).await)
            } else {
                error!("No window names provided")
            }
        }
        Commands::ChangeKeyboardLayer { layer } => print_error(app.change_keyboard_layer(layer)),
        Commands::EnableMouseJiggle => print_error(app.set_mouse_jiggle(true)),
        Commands::DisableMouseJiggle => print_error(app.set_mouse_jiggle(false)),
    };

    Ok(())
}

impl App {
    fn connect_to_keyboard(&self) -> Result<Keyboard, anyhow::Error> {
        Keyboard::new(&HidInfo {
            vendor_id: self.vid,
            product_id: self.pid,
            usage_page: self.usage_page,
            usage: self.usage,
        })
    }

    async fn watch_i3_focus(&self, config: config::I3WatcherConfig) -> Result<(), anyhow::Error> {
        let i3 = tokio_i3ipc::I3::connect().await?;

        i3.subscribe_to_window_focus_events(|prev_ev, window_data| {
            let node = window_data.container;
            debug!("win: current focused node: {:?}", node);

            if let Some(name) = node.name {
                if let Some(entry) = config.matches_window(&name) {
                    debug!("win: matched window: {:?}", entry);
                    let keyboard = self.connect_to_keyboard()?;
                    entry
                        .to_layer
                        .map(|layer| keyboard.send_message(Operation::ChangeLayer(layer)));
                } else if let Some(ev) = prev_ev {
                    if let Some(name) = ev.container.name {
                        if let Some(entry) = config.matches_window(&name) {
                            debug!("win: exited matching window: {:?}", entry);
                            let keyboard = self.connect_to_keyboard()?;
                            entry
                                .base_layer
                                .map(|layer| keyboard.send_message(Operation::ChangeLayer(layer)));
                        }
                    }
                }
            }

            Ok(())
        })
        .await?;

        Ok(())
    }

    fn print_keyboard_layer(&self) -> Result<(), anyhow::Error> {
        let keyboard = self.connect_to_keyboard()?;

        let response = keyboard.send_message(Operation::GetLayer)?;
        let jiggler_response = keyboard.send_message(Operation::GetJiggler)?;

        let jiggler_status = if let KeyboardResponse::JigglerStatus(true) = jiggler_response {
            "z "
        } else {
            ""
        };

        if let KeyboardResponse::CurrentLayer(_, name) = response {
            println!("{}{}", jiggler_status, name);
        }

        Ok(())
    }

    fn set_mouse_jiggle(&self, value: bool) -> Result<(), anyhow::Error> {
        let keyboard = self.connect_to_keyboard()?;
        keyboard.send_message(Operation::SetJiggler(value))?;
        Ok(())
    }

    fn change_keyboard_layer(&self, layer: u8) -> Result<(), anyhow::Error> {
        let keyboard = self.connect_to_keyboard()?;

        let response = keyboard.send_message(Operation::ChangeLayer(layer))?;

        if let KeyboardResponse::CurrentLayerNum(layer) = response {
            println!("Current layer: {}", layer);
        }

        Ok(())
    }

    fn keyboard_bootloader(&self) -> Result<(), anyhow::Error> {
        let keyboard = self.connect_to_keyboard()?;

        let _response = keyboard.send_message(Operation::Bootloader)?;

        Ok(())
    }
}
