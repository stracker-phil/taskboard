//! Website relaunch (`NW`): every issue the mockup shows, filled up to its sidebar counts
//! (26 open, 12 done in scope: Current sprint 12, Epics 5, Code reviews 2, Release 2.4.0 9, Open bugs 3; the
//! sidebar counts the subtasks like any other issue).

use super::catalog::Kind::{Bug, Epic, Story, Subtask, Task};
use super::catalog::Priority::{High, Highest, Low};
use super::catalog::Status::{Blocked, Done, InProgress, InReview, ToDo};
use super::people::{AL, AM, JR, MK};
use super::spec::{CommentSpec, IssueSpec, comment, issue};

const CHECKOUT: &str = "Rebuild the checkout as a single page with validated addresses, faster totals and fewer \
steps. Scope for 2.4.0 is frozen; follow-ups go into 2.5.0.";

const POSTCODES: &str =
    "Validate the postcode against the selected country before the payment step. Austria and \
Switzerland use 4 digits, Germany uses 5.
- Show inline errors under the field, not in a toast.
- Unknown countries pass without validation.
- Guest checkout must never be blocked by this.
![screenshot-login.png]";

/// Rich text: inline code with `->` and `--`, a strike, a code block, a mention and a status (`demo/adf.rs`).
const STORE_API: &str =
    "Move the cart totals from the legacy endpoint to the Store API. Values pass cart ->totals-> \
invoice, so nothing may round in between.
- Replace `Shop()->cart->get_totals()` with the `/store/v1/cart` response.
- Run the smoke test with `--no-cache --currency=EUR`.
- ~~Keep the session cache for totals~~ (no longer needed).
Status on staging: {status:In review}
```php
$totals = Shop()->cart->get_totals(); // old
```
@{Jonas Reiter} please confirm the rounding rules.";

#[rustfmt::skip]
pub fn issues() -> Vec<IssueSpec> {
    vec![
        // Epics
        issue("NW-1002", Epic, InProgress, "Payments").to(AM).prio(High).fix("2.5.0").color("ghx-label-3").at("2026-08-03 10:00", "2026-10-27 15:10"),
        issue("NW-1003", Epic, InProgress, "Performance").to(MK).color("ghx-label-4").at("2026-08-03 10:05", "2026-10-22 09:40"),
        issue("NW-1004", Epic, ToDo, "Accessibility").to(AL).color("ghx-label-13").at("2026-08-04 11:20", "2026-10-20 14:00"),
        issue("NW-1005", Epic, ToDo, "Emails").to(MK).prio(Low).color("ghx-label-9").at("2026-08-04 11:25", "2026-10-19 10:30"),
        issue("NW-1020", Epic, InProgress, "Checkout").to(JR).by(MK).fix("2.4.0").color("ghx-label-7").desc(CHECKOUT).at("2026-09-01 09:30", "2026-10-28 10:02"),
        // Open
        issue("NW-1042", Story, InProgress, "Checkout: address validation for DACH postcodes").to(AM).prio(High).parent("NW-1020").labels(&["frontend"]).fix("2.4.0").sprints(&[22]).points(5.0).reviewer(MK).due("2026-10-30").blocks("NW-1051").desc(POSTCODES).at("2026-10-12 09:14", "2026-10-28 11:42"),
        issue("NW-1047", Task, InProgress, "Migrate cart totals to the Store API").to(AM).parent("NW-1020").labels(&["backend"]).fix("2.4.0").sprints(&[22]).points(3.0).reviewer(JR).desc(STORE_API).at("2026-10-14 13:20", "2026-10-27 16:45"),
        issue("NW-1049", Story, InProgress, "Express payment buttons on product page").to(AM).prio(Highest).parent("NW-1002").labels(&["frontend"]).fix("2.5.0").sprints(&[22]).points(8.0).at("2026-10-15 10:00", "2026-10-28 08:15"),
        issue("NW-1038", Bug, InReview, "Shipping costs not refreshed after address change").to(AM).by(MK).prio(Highest).parent("NW-1020").labels(&["frontend"]).fix("2.4.0").sprints(&[21, 22]).points(2.0).reviewer(MK).at("2026-10-07 11:05", "2026-10-28 09:28"),
        issue("NW-1044", Task, InReview, "Lazy-load product gallery images").to(MK).prio(Low).parent("NW-1003").labels(&["frontend"]).fix("2.4.0").sprints(&[22]).points(2.0).reviewer(AM).at("2026-10-13 15:00", "2026-10-26 17:20"),
        issue("NW-1046", Task, InReview, "Preload critical fonts on landing pages").to(JR).parent("NW-1003").labels(&["frontend", "web-vitals", "needs-design-review", "fonts", "performance-budget-for-landing-and-product-pages"]).fix("2.4.0").sprints(&[22]).points(2.0).reviewer(AM).at("2026-10-16 10:40", "2026-10-27 11:30"),
        issue("NW-1033", Story, Blocked, "Klarna invoice for B2B customers").to(AM).prio(High).parent("NW-1002").labels(&["client"]).fix("2.5.0").sprints(&[21, 22]).points(5.0).due("2026-10-27").at("2026-10-02 09:00", "2026-10-23 09:05"),
        issue("NW-1035", Task, Blocked, "Tax rules sync with ERP").to(JR).parent("NW-1020").labels(&["backend"]).fix("2.4.0").sprints(&[22]).points(3.0).at("2026-10-05 10:00", "2026-10-23 11:00"),
        issue("NW-1051", Story, ToDo, "Guest checkout: remember shipping country").to(AM).parent("NW-1020").labels(&["frontend"]).fix("2.4.0").sprints(&[23]).points(3.0).at("2026-10-28 09:58", "2026-10-28 10:02"),
        issue("NW-1053", Bug, ToDo, "Focus trap missing in mini-cart drawer").to(AL).by(AL).parent("NW-1004").labels(&["frontend"]).fix("2.5.0").sprints(&[23]).points(1.0).at("2026-10-21 10:10", "2026-10-21 10:10"),
        issue("NW-1050", Bug, ToDo, "Skip link hidden behind sticky header").to(AL).by(AL).prio(High).parent("NW-1004").labels(&["frontend"]).fix("2.4.0").sprints(&[23]).points(2.0).at("2026-10-19 14:25", "2026-10-22 09:10"),
        issue("NW-1055", Task, ToDo, "Order confirmation email: add VAT breakdown").to(AM).prio(Low).parent("NW-1005").labels(&["backend"]).points(2.0).at("2026-10-22 14:30", "2026-10-22 14:30"),
        issue("NW-1056", Task, ToDo, "Newsletter double opt-in email template").to(MK).prio(Low).parent("NW-1005").labels(&["backend"]).at("2026-10-22 14:45", "2026-10-22 14:45"),
        issue("NW-1058", Story, ToDo, "Abandoned cart reminder email").to(JR).parent("NW-1005").fix("2.5.0").points(5.0).at("2026-10-23 10:00", "2026-10-26 09:30"),
        issue("NW-1040", Story, ToDo, "Contrast audit for product cards").to(AL).by(AL).parent("NW-1004").labels(&["frontend"]).fix("2.5.0").points(3.0).at("2026-10-09 13:00", "2026-10-20 14:00"),
        issue("NW-1059", Subtask, ToDo, "Add Liechtenstein postcode rule").to(AM).by(AM).parent("NW-1042").at("2026-10-28 11:50", "2026-10-28 11:50"),
        // Subtasks with other statuses, people and sprints than their parents (ADR 0037)
        issue("NW-1060", Subtask, InReview, "Screen reader hint for the postcode field").to(MK).by(AM).parent("NW-1042").sprints(&[22]).at("2026-10-24 09:20", "2026-10-27 15:30"),
        issue("NW-1061", Subtask, ToDo, "Round totals on the server only").to(JR).by(AM).parent("NW-1047").sprints(&[22]).at("2026-10-24 10:00", "2026-10-26 11:15"),
        issue("NW-1062", Subtask, InProgress, "Drop the legacy totals cache").to(AM).by(AM).parent("NW-1047").at("2026-10-24 10:05", "2026-10-27 17:10"),
        issue("NW-1063", Subtask, Blocked, "Regression test for switching the shipping country").to(JR).by(MK).parent("NW-1038").sprints(&[22]).at("2026-10-25 09:00", "2026-10-27 10:40"),
        issue("NW-1064", Subtask, ToDo, "Apple Pay domain verification file").to(AL).by(AM).prio(High).parent("NW-1049").sprints(&[22]).at("2026-10-25 11:30", "2026-10-26 09:00"),
        // Done within the last 90 days
        issue("NW-1029", Bug, Done, "PayPal button missing for logged-in users").to(AM).prio(High).parent("NW-1002").labels(&["frontend"]).fix("2.3.1").sprints(&[21]).points(3.0).reviewer(JR).at("2026-09-28 14:00", "2026-10-21 16:30"),
        issue("NW-1031", Story, Done, "Address autocomplete for DE").to(AM).parent("NW-1020").labels(&["frontend"]).fix("2.4.0").sprints(&[21, 22]).points(3.0).at("2026-10-02 11:00", "2026-10-27 10:15"),
        issue("NW-1026", Bug, Done, "Coupon field loses focus on mobile").to(AL).by(AL).parent("NW-1020").labels(&["frontend"]).fix("2.4.0").sprints(&[22]).points(1.0).at("2026-09-30 15:30", "2026-10-26 15:00"),
        issue("NW-1024", Task, Done, "Remove legacy checkout templates").to(JR).parent("NW-1020").labels(&["backend"]).fix("2.4.0").sprints(&[21]).points(2.0).at("2026-09-24 09:00", "2026-10-20 11:00"),
        issue("NW-1021", Story, Done, "One-page checkout layout").to(MK).by(MK).parent("NW-1020").labels(&["frontend"]).fix("2.4.0").sprints(&[21]).points(5.0).at("2026-09-21 10:00", "2026-10-16 17:00"),
        issue("NW-1043", Subtask, Done, "Postcode rules for AT, DE and CH").to(AM).by(AM).parent("NW-1042").sprints(&[22]).at("2026-10-23 10:00", "2026-10-26 17:45"),
        issue("NW-1045", Subtask, Done, "Inline error copy for the postcode field").to(MK).by(AM).parent("NW-1042").sprints(&[22]).at("2026-10-23 10:05", "2026-10-27 09:30"),
        issue("NW-1036", Task, Done, "Cache category pages at the edge").to(JR).parent("NW-1003").labels(&["backend"]).fix("2.4.0").sprints(&[22]).points(3.0).at("2026-10-06 10:00", "2026-10-26 12:10"),
        issue("NW-1039", Task, Done, "Alt texts for brand logos").to(AL).by(AL).parent("NW-1004").labels(&["frontend"]).fix("2.4.0").sprints(&[22]).points(1.0).at("2026-10-08 09:45", "2026-10-23 15:40"),
        issue("NW-1041", Task, Done, "Password reset email: new layout").to(MK).parent("NW-1005").labels(&["backend"]).fix("2.4.0").sprints(&[22]).points(2.0).at("2026-10-12 13:00", "2026-10-27 14:20"),
        issue("NW-977", Bug, Done, "PayPal button missing on checkout for logged-in users").to(AM).prio(High).parent("NW-1002").labels(&["frontend"]).at("2026-08-28 10:30", "2026-09-08 15:20"),
        issue("NW-1012", Task, Done, "Legacy coupon import").to(JR).by(MK).labels(&["backend"]).at("2026-09-10 09:00", "2026-10-27 08:40"),
        // Done more than 90 days ago: outside the default sync scope
        issue("NW-912", Task, Done, "Migrate product feed to the new CDN").to(JR).labels(&["backend"]).at("2026-05-04 09:00", "2026-06-18 16:00"),
        issue("NW-934", Story, Done, "Cookie banner copy update").to(MK).by(MK).labels(&["client"]).at("2026-06-01 11:00", "2026-07-10 10:20"),
    ]
}

#[rustfmt::skip]
pub fn comments() -> Vec<CommentSpec> {
    vec![
        comment("NW-1020", MK, "2026-10-12 09:30", "Attached the updated checkout flow from the client workshop.\n![checkout-flow.png]"),
        comment("NW-1020", JR, "2026-10-26 14:02", "Scope for 2.4.0 is frozen. Anything new goes into 2.5.0."),
        comment("NW-1042", MK, "2026-10-13 10:30", "Design for the error states is ready for review."),
        comment("NW-1042", AM, "2026-10-26 17:48", "Validation done for AT, DE and CH. Tests are green on staging."),
        comment("NW-1042", MK, "2026-10-28 10:05", "Copy for the error states is in the design file, page \u{201c}Checkout\u{201d}."),
        comment("NW-1042", MK, "2026-10-28 11:42", "Client confirmed Liechtenstein should behave like Switzerland. Can you include it in the rules?"),
        comment("NW-1047", JR, "2026-10-27 16:45", "The Store API returns totals in cents; please keep the rounding on the server.\nUse `round_half_up()` and pass `--strict`; ~~client-side rounding~~ is out. @{Alex Morgan} {status:Decided}\n```bash\nshop-cli totals --check --order=1001\n```"),
        comment("NW-1038", MK, "2026-10-08 09:15", "Reproduced on staging: totals stay on the old country until the page reloads."),
        comment("NW-1038", JR, "2026-10-15 11:40", "Same when switching from Germany to Austria."),
        comment("NW-1038", AM, "2026-10-28 09:28", "Fixed by refetching the rates on every address change. Ready for review."),
        comment("NW-1033", AM, "2026-10-20 10:12", "Blocked until the client signs the B2B terms with the payment provider."),
        comment("NW-1033", JR, "2026-10-23 09:05", "The contract is with their legal team; expect news next week."),
        comment("NW-1029", JR, "2026-09-28 14:00", "Reported by the client: logged-in customers don't see the PayPal button."),
        comment("NW-1029", AM, "2026-09-29 10:20", "Looks like a regression of NW-977. The session check hides the wallet buttons."),
        comment("NW-1029", AM, "2026-10-01 16:40", "Fix is on staging."),
        comment("NW-1029", JR, "2026-10-02 09:10", "Confirmed on staging, ships with 2.3.1."),
        comment("NW-1029", MK, "2026-10-21 16:30", "Client confirmed it works in production."),
        comment("NW-977", AM, "2026-09-08 15:20", "Fixed in the hotfix. I tested with three accounts and the checkout totals look right now."),
    ]
}
