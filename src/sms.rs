use crate::getatext::Getatext;
use crate::pvacodes::Pvacodes;
use crate::types::{CreateError, SmsRental};
use std::time::Duration;

#[derive(Clone)]
pub enum SmsProvider {
    Getatext(Getatext),
    Pvacodes(Pvacodes),
}

impl SmsProvider {
    pub async fn rent(&self) -> Result<SmsRental, CreateError> {
        match self {
            Self::Getatext(g) => g.rent().await,
            Self::Pvacodes(p) => p.rent().await,
        }
    }

    pub async fn poll_code(
        &self,
        rental: &SmsRental,
        timeout: Duration,
        interval: Duration,
    ) -> anyhow::Result<String> {
        match self {
            Self::Getatext(g) => g.poll_code(rental, timeout, interval).await,
            Self::Pvacodes(p) => p.poll_code(rental, timeout, interval).await,
        }
    }

    pub async fn mark_completed(&self, rental: &SmsRental) {
        match self {
            Self::Getatext(g) => g.mark_completed(rental).await,
            Self::Pvacodes(p) => p.mark_completed(rental).await,
        }
    }

    pub async fn cancel(&self, rental: &SmsRental) {
        match self {
            Self::Getatext(g) => g.cancel(rental).await,
            Self::Pvacodes(p) => p.cancel(rental).await,
        }
    }
}

pub struct RentalGuard {
    state: Option<(SmsRental, SmsProvider)>,
}

impl RentalGuard {
    pub fn new(rental: SmsRental, provider: SmsProvider) -> Self {
        Self {
            state: Some((rental, provider)),
        }
    }

    pub fn rental(&self) -> &SmsRental {
        &self.state.as_ref().unwrap().0
    }

    pub async fn complete(mut self) {
        if let Some((rental, provider)) = self.state.take() {
            provider.mark_completed(&rental).await;
        }
    }
}

impl Drop for RentalGuard {
    fn drop(&mut self) {
        if let Some((rental, provider)) = self.state.take() {
            tokio::spawn(async move {
                provider.cancel(&rental).await;
            });
        }
    }
}
