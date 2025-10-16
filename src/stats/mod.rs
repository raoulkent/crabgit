use clap::ValueEnum;
use serde::Serialize;

pub mod activity;

#[derive(Copy, Clone, Debug, Serialize, ValueEnum)]
pub enum Bucket {
    Day,
    Week,
    Month,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Chart,
}

#[derive(Debug, Serialize, Clone)]
pub struct StatsContext {
    pub repo_path: String,
    pub since: Option<String>,
    pub until: Option<String>,
    pub bucket: Bucket,
}
