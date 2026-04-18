use simplelog::{CombinedLogger, ConfigBuilder, LevelFilter, WriteLogger};
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::Once;
use tauri::Manager;

static INIT_LOGGER: Once = Once::new();

pub fn init_logging(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let mut init_error: Option<String> = None;

    INIT_LOGGER.call_once(|| {
        let mut log_dir = match app_handle.path().app_data_dir() {
            Ok(path) => path,
            Err(err) => {
                init_error = Some(format!("Failed to resolve app_data_dir for logger: {}", err));
                return;
            }
        };

        if let Err(err) = fs::create_dir_all(&log_dir) {
            init_error = Some(format!("Failed to create log directory: {}", err));
            return;
        }

        log_dir.push("filesorter.log");
        let log_file = match File::options().create(true).append(true).open(&log_dir) {
            Ok(file) => file,
            Err(err) => {
                init_error = Some(format!("Failed to open log file {}: {}", log_dir.display(), err));
                return;
            }
        };

        let config = ConfigBuilder::new().set_time_format_rfc3339().build();
        if let Err(err) = CombinedLogger::init(vec![WriteLogger::new(LevelFilter::Info, config, log_file)]) {
            init_error = Some(format!("Failed to initialize logger: {}", err));
        }
    });

    if let Some(err) = init_error {
        return Err(err);
    }

    Ok(())
}

pub fn log_path(app_handle: &tauri::AppHandle) -> Option<PathBuf> {
    let mut path = app_handle.path().app_data_dir().ok()?;
    path.push("filesorter.log");
    Some(path)
}
