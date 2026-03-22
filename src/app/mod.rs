pub mod state;
pub mod file_ops;
pub mod audio_ops;
pub mod selection;
pub mod ui_menu;
pub mod ui_tabs;
pub mod ui_table;
pub mod ui_tools;
pub mod ui_waveform;
pub mod ui_status;
pub mod ui_modals;
pub mod recorder;
pub mod ui_recorder;

pub use state::{CopaibaApp, TabState, Preset, ShortcutProfile, CustomShortcuts};
