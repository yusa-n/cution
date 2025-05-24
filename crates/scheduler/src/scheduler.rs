use anyhow::Result;
use tokio_cron_scheduler::{JobScheduler, Job};
use tracing::{info, error};
use time::OffsetDateTime;
use std::sync::Arc;

pub struct DailyScheduler {
    scheduler: JobScheduler,
}

impl DailyScheduler {
    pub async fn new() -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        
        Ok(Self {
            scheduler,
        })
    }

    pub async fn add_daily_job<F, Fut>(&mut self, hour: u32, minute: u32, job_fn: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let cron_expression = format!("0 {} {} * * *", minute, hour);
        info!("Scheduling daily job with cron: {}", cron_expression);

        let job_fn = Arc::new(job_fn);
        let job = Job::new_async(&cron_expression, move |_uuid, _l| {
            let job_fn = job_fn.clone();
            Box::pin(async move {
                info!("Executing scheduled job at {}", OffsetDateTime::now_utc());
                match job_fn().await {
                    Ok(()) => info!("Scheduled job completed successfully"),
                    Err(e) => error!("Scheduled job failed: {}", e),
                }
            })
        })?;

        self.scheduler.add(job).await?;
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting scheduler...");
        self.scheduler.start().await?;
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down scheduler...");
        self.scheduler.shutdown().await?;
        Ok(())
    }

    pub async fn run_forever(&self) -> Result<()> {
        self.start().await?;
        
        // Keep the scheduler running
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}