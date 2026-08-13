use crate::hexagon::driven_ports::for_running_data_refresh_tasks::{
    DataRefreshTask, ForRunningDataRefreshTasks,
};

#[derive(Debug, Clone, Copy)]
pub struct TokioDataRefreshTaskRunner;

impl ForRunningDataRefreshTasks for TokioDataRefreshTaskRunner {
    fn run_data_refresh_task(&self, task: DataRefreshTask) {
        tokio::spawn(task);
    }
}
