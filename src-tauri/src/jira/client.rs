//! `JiraClient`: the typed Jira operations (`api.rs`) over the gateway, which owns every request.

use std::sync::Arc;

use super::gateway::Gateway;
use super::in_flight::InFlight;
use super::limiter::Limiter;
use super::site::Site;
use crate::secrets::SecretStore;

pub struct JiraClient {
    gateway: Gateway,
}

impl JiraClient {
    /// `limiter` and `in_flight` are shared by every client, so a reconnect can't double the request rate or
    /// lose the requests still running.
    pub fn new(
        site: Site,
        secrets: Arc<dyn SecretStore>,
        limiter: Arc<Limiter>,
        in_flight: Arc<InFlight>,
    ) -> Self {
        Self {
            gateway: Gateway::new(site, secrets, limiter, in_flight),
        }
    }

    pub fn site(&self) -> &Site {
        self.gateway.site()
    }

    pub(super) fn gateway(&self) -> &Gateway {
        &self.gateway
    }
}
