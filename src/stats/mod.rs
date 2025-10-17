use clap::ValueEnum;
use serde::Serialize;

pub mod activity;
pub mod churn;
pub mod authors;
pub mod calendar;
pub mod hotspots;
pub mod branches;
pub mod coupling;
pub mod ownership;

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

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum StatsMetric {
    Activity,
    Churn,
}

#[derive(Copy, Clone, Debug, ValueEnum, Serialize)]
pub enum AuthorMetric {
    Commits,
    Churn,
}

#[derive(Debug, Serialize, Clone)]
pub struct StatsContext {
    pub repo_path: String,
    pub since: Option<String>,
    pub until: Option<String>,
    pub bucket: Bucket,
    pub no_merges: bool,
}
