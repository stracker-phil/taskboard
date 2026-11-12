//! The Jira source: every call the sync makes goes through the Jira client and its gateway (ADR 0025).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::source::{Answer, IssuePages, Source, SourceKind};
use serde_json::Value;

use crate::error::AppResult;
use crate::jira::dto::{
    BulkFetch, Changelog, Comment, Field, Myself, Project, RemoteLink, SearchPage, Status,
    Transition, Version,
};
use crate::jira::{JiraClient, SearchPages, SearchRequest};

pub struct JiraSource {
    client: Arc<JiraClient>,
}

impl JiraSource {
    pub fn new(client: Arc<JiraClient>) -> Self {
        Self { client }
    }
}

struct JiraPages<'a>(SearchPages<'a>);

impl IssuePages for JiraPages<'_> {
    fn next_page(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Option<AppResult<SearchPage>>> + Send + '_>> {
        Box::pin(self.0.next_page())
    }

    fn without_expand(&mut self, part: &str) -> bool {
        self.0.without_expand(part)
    }
}

impl Source for JiraSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Jira
    }

    fn fields(&self) -> Answer<'_, Vec<Field>> {
        Box::pin(self.client.fields())
    }

    fn statuses(&self) -> Answer<'_, Vec<Status>> {
        Box::pin(self.client.statuses())
    }

    fn spaces(&self) -> Answer<'_, Vec<Project>> {
        Box::pin(self.client.projects())
    }

    fn versions<'a>(&'a self, space_key: &'a str) -> Answer<'a, Vec<Version>> {
        Box::pin(self.client.versions(space_key))
    }

    fn myself(&self) -> Answer<'_, Myself> {
        Box::pin(self.client.myself())
    }

    fn count<'a>(&'a self, query: &'a str) -> Answer<'a, u64> {
        Box::pin(self.client.approximate_count(query))
    }

    fn issues(&self, request: SearchRequest) -> Box<dyn IssuePages + '_> {
        Box::new(JiraPages(self.client.search_jql(request)))
    }

    fn fetch<'a>(&'a self, ids: &'a [String], fields: &'a [String]) -> Answer<'a, BulkFetch> {
        Box::pin(self.client.bulk_fetch(ids, fields))
    }

    fn comments<'a>(&'a self, issue_key: &'a str) -> Answer<'a, Vec<Comment>> {
        Box::pin(self.client.comments(issue_key))
    }

    fn remote_links<'a>(&'a self, issue_id: &'a str) -> Answer<'a, Vec<RemoteLink>> {
        Box::pin(self.client.remote_links(issue_id))
    }

    fn changelog<'a>(&'a self, issue_id: &'a str) -> Answer<'a, Vec<Changelog>> {
        Box::pin(self.client.changelog(issue_id))
    }

    fn attachment_content<'a>(&'a self, attachment_id: &'a str) -> Answer<'a, Vec<u8>> {
        Box::pin(self.client.attachment_content(attachment_id))
    }

    fn add_comment<'a>(&'a self, issue_id: &'a str, body: Value) -> Answer<'a, Comment> {
        Box::pin(self.client.add_comment(issue_id, body))
    }

    fn transitions<'a>(&'a self, issue_id: &'a str) -> Answer<'a, Vec<Transition>> {
        Box::pin(self.client.transitions(issue_id))
    }

    fn transition<'a>(&'a self, issue_id: &'a str, transition_id: &'a str) -> Answer<'a, ()> {
        Box::pin(self.client.transition(issue_id, transition_id))
    }

    fn priorities<'a>(&'a self, issue_id: &'a str) -> Answer<'a, Vec<String>> {
        Box::pin(self.client.priorities(issue_id))
    }

    fn add_remote_link<'a>(
        &'a self,
        issue_id: &'a str,
        url: &'a str,
        title: &'a str,
    ) -> Answer<'a, String> {
        Box::pin(self.client.add_remote_link(issue_id, url, title))
    }

    fn set_priority<'a>(&'a self, issue_id: &'a str, name: &'a str) -> Answer<'a, ()> {
        Box::pin(self.client.set_priority(issue_id, name))
    }

    fn set_title<'a>(&'a self, issue_id: &'a str, title: &'a str) -> Answer<'a, ()> {
        Box::pin(self.client.set_title(issue_id, title))
    }

    fn assign<'a>(&'a self, issue_id: &'a str, account_id: Option<&'a str>) -> Answer<'a, ()> {
        Box::pin(self.client.assign(issue_id, account_id))
    }
}
