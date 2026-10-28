//! `JiraClient`: the typed Jira operations (`api.rs`) over the gateway, which owns every request.

use std::sync::Arc;

use super::gateway::Gateway;
use super::limiter::Limiter;
use super::site::Site;
use crate::secrets::SecretStore;

pub struct JiraClient {
    gateway: Gateway,
}

impl JiraClient {
    /// `limiter` is shared by every client, so a reconnect can't double the request rate.
    pub fn new(site: Site, secrets: Arc<dyn SecretStore>, limiter: Arc<Limiter>) -> Self {
        Self {
            gateway: Gateway::new(site, secrets, limiter),
        }
    }

    pub fn site(&self) -> &Site {
        self.gateway.site()
    }

    pub(super) fn gateway(&self) -> &Gateway {
        &self.gateway
    }
}
