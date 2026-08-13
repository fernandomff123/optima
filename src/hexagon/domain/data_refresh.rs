use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::hexagon::{PortError, PortResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataRefreshState {
    Running,
    Completed,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataRefreshOrigin {
    Startup,
    Scheduled,
    Retry,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataRefreshFailure {
    pub ticker: String,
    pub operation: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataRefreshRun {
    pub id: String,
    pub origin: DataRefreshOrigin,
    pub state: DataRefreshState,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub target_session: NaiveDate,
    pub items_obtained: u64,
    pub items_persisted: u64,
    pub failure_count: u64,
    pub next_attempt_at: Option<DateTime<Utc>>,
    pub summary: String,
    pub failures: Vec<DataRefreshFailure>,
}

impl DataRefreshRun {
    pub fn running(
        id: String,
        origin: DataRefreshOrigin,
        now: DateTime<Utc>,
        target_session: NaiveDate,
    ) -> Self {
        Self {
            id,
            origin,
            state: DataRefreshState::Running,
            started_at: now,
            finished_at: None,
            target_session,
            items_obtained: 0,
            items_persisted: 0,
            failure_count: 0,
            next_attempt_at: None,
            summary: "Atualização iniciada".to_string(),
            failures: Vec::new(),
        }
    }

    pub fn finish(
        &mut self,
        now: DateTime<Utc>,
        obtained: u64,
        persisted: u64,
        failures: Vec<DataRefreshFailure>,
        next_attempt_at: Option<DateTime<Utc>>,
    ) -> PortResult<()> {
        if self.state != DataRefreshState::Running {
            return Err(PortError::Conflict(
                "only a running refresh can be finished".to_string(),
            ));
        }
        self.items_obtained = obtained;
        self.items_persisted = persisted;
        self.failure_count = failures.len() as u64;
        self.finished_at = Some(now);
        self.next_attempt_at = next_attempt_at;
        self.failures = failures;
        self.state = if self.failure_count == 0 {
            DataRefreshState::Completed
        } else if obtained > 0 || persisted > 0 {
            DataRefreshState::Partial
        } else {
            DataRefreshState::Failed
        };
        self.summary = match self.state {
            DataRefreshState::Completed => "Atualização concluída".to_string(),
            DataRefreshState::Partial => {
                format!("Atualização parcial: {} falha(s)", self.failure_count)
            }
            DataRefreshState::Failed => {
                format!("Atualização falhou: {} falha(s)", self.failure_count)
            }
            DataRefreshState::Running => {
                return Err(PortError::Conflict("invalid terminal state".to_string()));
            }
        };
        Ok(())
    }

    pub fn interrupt(&mut self, now: DateTime<Utc>) -> PortResult<()> {
        if self.state != DataRefreshState::Running {
            return Err(PortError::Conflict(
                "only a running refresh can be interrupted".to_string(),
            ));
        }
        self.state = DataRefreshState::Failed;
        self.finished_at = Some(now);
        self.failure_count += 1;
        self.summary =
            "Atualização falhou: execução interrompida pelo encerramento do processo".to_string();
        self.failures.push(DataRefreshFailure {
            ticker: "system".to_string(),
            operation: "refresh".to_string(),
            error: "process interrupted".to_string(),
        });
        Ok(())
    }
}
