mod i3;
mod keyboard;

use clap::Parser;
use clap_num::maybe_hex;
use log::{debug, error};

use crate::i3::I3Ext;

use self::keyboard::{HidInfo, Keyboard, KeyboardResponse, Operation};

// const VENDOR_ID: u16 = 0x4b41; // Kasama
// const PRODUCT_ID: u16 = 0x504d; // Macro pad
const VENDOR_ID: u16 = 0x444D; // Tshort
const PRODUCT_ID: u16 = 0x3435; // Dactyl Manuform

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
        window_names: Vec<String>,
        #[arg(short, long, default_value = "0")]
        base_layer: u8,
        #[arg(short, long, default_value = "5")]
        change_layer: u8,
    },
    ChangeKeyboardLayer { layer: u8 },
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
        Commands::WatchI3Focus { ref window_names, base_layer, change_layer } => {
            if window_names.is_empty() {
                error!("No window names provided")
            } else {
                print_error(app.watch_i3_focus(window_names, base_layer, change_layer).await)
            }
        }
        Commands::ChangeKeyboardLayer { layer } => print_error(app.change_keyboard_layer(layer)),
    };

    Ok(())
}

fn matches_layer_names(name: String, matches: &[String]) -> bool {
    matches
        .iter()
        .map(|m| m.to_lowercase())
        .any(|m| name.to_lowercase().contains(&m))
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

    async fn watch_i3_focus(&self, window_names: &[String], base_layer: u8, change_layer: u8) -> Result<(), anyhow::Error> {
        let i3 = tokio_i3ipc::I3::connect().await?;

        i3.subscribe_to_window_focus_events(|prev_ev, window_data| {
            let node = window_data.container;
            debug!("win: current focused node: {:?}", node);

            if let Some(name) = node.name {
                if matches_layer_names(name, window_names) {
                    let keyboard = self.connect_to_keyboard()?;
                    keyboard.send_message(Operation::ChangeLayer(change_layer))?;
                } else if let Some(ev) = prev_ev {
                    if let Some(name) = ev.container.name {
                        if matches_layer_names(name, window_names) {
                            let keyboard = self.connect_to_keyboard()?;
                            keyboard.send_message(Operation::ChangeLayer(base_layer))?;
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

        if let KeyboardResponse::CurrentLayer(layer) = response {
            println!("⌨: {}", keyboard::Layers::from(layer).to_string());
        }

        Ok(())
    }

    fn change_keyboard_layer(&self, layer: u8) -> Result<(), anyhow::Error> {
        let keyboard = self.connect_to_keyboard()?;

        let response = keyboard.send_message(Operation::ChangeLayer(layer))?;

        if let KeyboardResponse::CurrentLayer(layer) = response {
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
