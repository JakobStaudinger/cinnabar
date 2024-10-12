use crate::config::AppConfig;

use super::webhook::{Callbacks, TriggerCallback};

#[derive(Clone)]
pub struct RequestState<T: TriggerCallback> {
    pub config: AppConfig,
    pub callbacks: Callbacks<T>,
}
