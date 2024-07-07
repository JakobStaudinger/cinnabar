use crate::config::AppConfig;

use super::webhook::Callbacks;

#[derive(Clone)]
pub struct RequestState {
    pub config: AppConfig,
    pub callbacks: Callbacks,
}
