use crate::Level;
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use types::config::EnvironmentConfig;

pub(crate) fn open_config_file<P: AsRef<Path>>(path: P) -> Result<EnvironmentConfig> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;
    let reader = BufReader::new(&file);
    let env: EnvironmentConfig = match serde_json::from_reader(reader) {
        Ok(env) => env,
        Err(_) => {
            let default_config = EnvironmentConfig::default();
            let mut writer = BufWriter::new(&file);
            serde_json::to_writer(&mut writer, &default_config)?;
            writer.flush()?;
            file.sync_all()?;
            default_config
        }
    };
    Ok(env)
}

pub(crate) fn default_db_opts() -> rocksdb::Options {
    let mut opts = rocksdb::Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts.set_atomic_flush(true);

    // TODO: tune
    //opts.increase_parallelism(num_cpus::get() as i32);
    opts.set_allow_mmap_writes(true);
    opts.set_allow_mmap_reads(true);

    opts.set_max_log_file_size(1_000_000);
    opts.set_recycle_log_file_num(5);
    opts.set_keep_log_file_num(5);
    opts
}
