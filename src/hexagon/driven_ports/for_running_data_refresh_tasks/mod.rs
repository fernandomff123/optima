use std::{future::Future, pin::Pin};

pub type DataRefreshTask = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub trait ForRunningDataRefreshTasks: Send + Sync {
    fn run_data_refresh_task(&self, task: DataRefreshTask);
}
