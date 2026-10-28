//! The other Spaces (open counts from the mockup sidebar: NWS 6, LUM 9, FBA 11, INT 8) and the
//! untracked archive project.

use super::catalog::Kind::{Bug, Story, Subtask, Task};
use super::catalog::Priority::{High, Highest, Low, Lowest};
use super::catalog::Status::{Blocked, Done, InProgress, InReview, ToDo};
use super::people::{AL, AM, JR, LN, MK, SB, TH};
use super::spec::{CommentSpec, IssueSpec, comment, issue};

#[rustfmt::skip]
pub fn issues() -> Vec<IssueSpec> {
    vec![
        // Northwind support: 6 open, 2 done in scope, 1 old
        issue("NWS-211", Bug, InProgress, "Invoice PDF shows the wrong VAT ID").to(AM).by(MK).prio(High).labels(&["support"]).due("2026-10-29").at("2026-10-21 08:50", "2026-10-27 17:05"),
        issue("NWS-212", Bug, InReview, "Product filter ignores size XL").to(JR).by(MK).labels(&["support"]).reviewer(AM).at("2026-10-19 10:20", "2026-10-26 15:45"),
        issue("NWS-208", Bug, Blocked, "Newsletter sign-up returns 500 in Safari").to(MK).by(MK).prio(High).labels(&["support"]).at("2026-10-14 09:00", "2026-10-22 13:30"),
        issue("NWS-214", Bug, ToDo, "Customer cannot reset password on mobile").to(AM).by(MK).labels(&["support"]).at("2026-10-26 11:15", "2026-10-26 11:15"),
        issue("NWS-215", Task, ToDo, "Export September orders as CSV").to(JR).by(MK).prio(Low).labels(&["client"]).due("2026-11-02").at("2026-10-27 09:40", "2026-10-27 09:40"),
        issue("NWS-216", Task, ToDo, "Update store opening hours for December").to(AM).by(MK).prio(Low).labels(&["client"]).due("2026-11-30").at("2026-10-27 16:00", "2026-10-27 16:00"),
        issue("NWS-201", Bug, Done, "Broken link in the footer imprint").to(JR).by(MK).prio(Low).labels(&["support"]).at("2026-10-05 10:00", "2026-10-08 11:30"),
        issue("NWS-205", Task, Done, "Add the new shipping partner logo").to(AM).by(MK).prio(Low).labels(&["client"]).at("2026-10-15 14:10", "2026-10-23 10:00"),
        issue("NWS-190", Bug, Done, "Typo in the returns policy").to(MK).by(MK).prio(Lowest).labels(&["support"]).at("2026-06-02 09:00", "2026-06-03 10:00"),
        // Lumen iOS app: 9 open, 2 done in scope, 1 old
        issue("LUM-318", Bug, ToDo, "Widget crash on iPad").to(AM).by(SB).prio(Highest).labels(&["ios", "crash"]).sprints(&[38]).at("2026-10-28 09:20", "2026-10-28 09:20").desc("The home screen widget crashes on iPad when the app has not been opened since the update."),
        issue("LUM-301", Story, InProgress, "Dark mode for onboarding screens").to(SB).by(SB).labels(&["ios"]).fix("3.3.0").sprints(&[37, 38]).points(5.0).at("2026-10-01 10:00", "2026-10-27 12:30"),
        issue("LUM-305", Story, InProgress, "Offline queue for journal entries").to(AM).by(SB).prio(High).labels(&["ios"]).fix("3.3.0").sprints(&[38]).points(8.0).reviewer(SB).at("2026-10-06 09:30", "2026-10-27 18:20"),
        issue("LUM-309", Bug, InReview, "Crash when rotating during photo import").to(SB).by(AL).prio(High).labels(&["ios", "crash"]).fix("3.3.0").sprints(&[38]).points(2.0).reviewer(AM).at("2026-10-12 15:00", "2026-10-26 10:40"),
        issue("LUM-314", Task, Blocked, "Migrate analytics to the new SDK").to(SB).by(SB).fix("3.3.0").sprints(&[38]).points(3.0).at("2026-10-14 11:00", "2026-10-21 09:15"),
        issue("LUM-310", Task, ToDo, "Localize App Store screenshots (DE, FR)").to(AL).by(SB).prio(Low).fix("3.3.0").at("2026-10-09 14:00", "2026-10-09 14:00"),
        issue("LUM-312", Story, ToDo, "Haptic feedback on streak milestones").by(SB).prio(Low).fix("4.0").at("2026-10-13 16:20", "2026-10-13 16:20"),
        issue("LUM-315", Bug, ToDo, "VoiceOver labels on the mood picker").to(AL).by(AL).labels(&["ios", "accessibility"]).fix("3.3.0").sprints(&[38]).points(2.0).at("2026-10-20 09:00", "2026-10-22 11:10"),
        issue("LUM-316", Task, ToDo, "Reduce cold start below 1.5 s").to(AM).by(SB).fix("4.0").points(5.0).at("2026-10-22 13:00", "2026-10-22 13:00"),
        issue("LUM-296", Bug, Done, "Reminder notifications late after the clock change").to(SB).by(AL).prio(High).labels(&["ios"]).fix("3.2.0").sprints(&[37]).points(3.0).at("2026-09-29 09:00", "2026-10-14 16:00"),
        issue("LUM-299", Task, Done, "Submit 3.2.0 for App Store review").to(AM).by(SB).fix("3.2.0").sprints(&[37]).points(1.0).at("2026-10-10 10:00", "2026-10-15 09:30"),
        issue("LUM-280", Task, Done, "Upgrade the project to Swift 6").to(SB).by(SB).at("2026-06-10 09:00", "2026-07-15 17:00"),
        // Fernweh booking API: 11 open, 2 done in scope
        issue("FBA-87", Story, InProgress, "Idempotency keys for booking requests").to(AM).by(LN).prio(High).labels(&["backend"]).points(5.0).reviewer(LN).at("2026-10-07 09:00", "2026-10-27 11:00"),
        issue("FBA-99", Subtask, ToDo, "Keep idempotency keys for 24 hours").to(AM).by(AM).parent("FBA-87").at("2026-10-27 11:00", "2026-10-27 11:00"),
        issue("FBA-88", Bug, InReview, "Return 409 for double-booked rooms").to(LN).by(LN).prio(High).labels(&["backend"]).reviewer(AM).at("2026-10-13 14:30", "2026-10-27 15:20"),
        issue("FBA-93", Bug, InProgress, "Timeouts when the channel manager is slow").to(LN).by(AM).prio(Highest).labels(&["backend"]).at("2026-10-20 08:10", "2026-10-28 08:45"),
        issue("FBA-92", Story, Blocked, "Cancellation fees per tariff").to(AM).by(LN).prio(High).labels(&["client"]).at("2026-10-16 10:00", "2026-10-26 16:10"),
        issue("FBA-90", Task, ToDo, "OpenAPI spec for /availability").to(AM).by(LN).labels(&["docs"]).due("2026-11-04").at("2026-10-15 09:00", "2026-10-21 10:00"),
        issue("FBA-91", Story, ToDo, "Rate limit partner API keys").to(LN).by(LN).labels(&["backend"]).at("2026-10-15 09:20", "2026-10-15 09:20"),
        issue("FBA-94", Story, ToDo, "Webhook retries with backoff").to(JR).by(LN).labels(&["backend"]).at("2026-10-19 13:00", "2026-10-19 13:00"),
        issue("FBA-95", Task, ToDo, "Remove the deprecated v1 search endpoint").by(LN).prio(Low).labels(&["backend"]).at("2026-10-19 13:10", "2026-10-19 13:10"),
        issue("FBA-96", Bug, ToDo, "Currency rounding for CHF").to(AM).by(LN).labels(&["backend"]).at("2026-10-23 11:30", "2026-10-24 09:00"),
        issue("FBA-97", Task, ToDo, "Load test the booking flow").to(JR).by(AM).prio(Low).at("2026-10-23 15:00", "2026-10-23 15:00"),
        issue("FBA-80", Task, Done, "Health check endpoint for the load balancer").to(LN).by(LN).labels(&["backend"]).at("2026-09-14 09:00", "2026-09-18 12:00"),
        issue("FBA-84", Task, Done, "Upgrade Postgres to 17").to(AM).by(LN).prio(High).labels(&["backend"]).at("2026-09-28 10:00", "2026-10-09 18:30"),
        // Internal tools: 8 open, 1 done in scope, 1 old
        issue("INT-42", Task, InProgress, "Renew the wildcard certificate for staging").to(TH).by(TH).prio(High).due("2026-11-01").at("2026-10-20 09:00", "2026-10-28 08:30"),
        issue("INT-47", Story, InReview, "Shared password vault per client").to(TH).by(AM).reviewer(AM).at("2026-10-12 10:00", "2026-10-26 14:00"),
        issue("INT-44", Task, Blocked, "Move CI runners to the new build machine").to(TH).by(TH).at("2026-10-08 11:00", "2026-10-19 09:30"),
        issue("INT-41", Task, ToDo, "Time tracking export for October").to(AM).by(AM).due("2026-11-03").at("2026-10-26 08:30", "2026-10-26 08:30"),
        issue("INT-43", Task, ToDo, "Backup restore drill").to(AM).by(TH).at("2026-10-14 15:00", "2026-10-14 15:00"),
        issue("INT-45", Task, ToDo, "Template for client kickoff notes").to(MK).by(AM).prio(Low).at("2026-10-16 10:30", "2026-10-16 10:30"),
        issue("INT-46", Task, ToDo, "Clean up unused DNS records").by(TH).prio(Lowest).at("2026-10-02 12:00", "2026-10-02 12:00"),
        issue("INT-48", Story, ToDo, "Onboarding checklist for freelancers").to(AM).by(AM).prio(Low).at("2026-10-27 17:30", "2026-10-27 17:30"),
        issue("INT-38", Task, Done, "Rotate API keys for the status page").to(TH).by(TH).prio(High).at("2026-10-12 09:00", "2026-10-18 10:15"),
        issue("INT-30", Task, Done, "Archive old chat channels").to(AM).by(TH).prio(Lowest).at("2026-03-02 09:00", "2026-03-20 16:00"),
        // Archive 2024: untracked, done long ago
        issue("ARC-12", Story, Done, "Landing page for the spring campaign").to(MK).by(MK).at("2024-02-05 09:00", "2024-03-28 17:00"),
        issue("ARC-15", Task, Done, "Move the blog to a static site").to(JR).by(JR).at("2024-05-13 10:00", "2024-07-02 15:30"),
        issue("ARC-19", Task, Done, "Holiday shipping banner").to(AM).by(MK).prio(Low).at("2024-11-18 09:00", "2024-12-02 11:00"),
    ]
}

#[rustfmt::skip]
pub fn comments() -> Vec<CommentSpec> {
    vec![
        comment("NWS-211", MK, "2026-10-21 08:52", "The customer sent a screenshot; the VAT ID is the old one from before the move."),
        comment("NWS-211", AM, "2026-10-27 17:05", "Found it: the PDF template caches the company footer. Fix is ready."),
        comment("NWS-208", MK, "2026-10-22 13:30", "Waiting for the mail provider to whitelist the new sender domain."),
        comment("LUM-318", SB, "2026-10-28 09:20", "Five crash reports since this morning, all on iPadOS."),
        comment("LUM-309", AM, "2026-10-26 10:40", "Looks good, one question about the cancel path inline."),
        comment("FBA-92", LN, "2026-10-26 16:10", "The client still has to decide how fees work for non-refundable tariffs."),
        comment("FBA-93", AM, "2026-10-28 08:45", "Seen again overnight; the channel manager took 31 s for one call."),
        comment("INT-42", TH, "2026-10-28 08:30", "New certificate is issued, rolling it out after lunch."),
    ]
}
