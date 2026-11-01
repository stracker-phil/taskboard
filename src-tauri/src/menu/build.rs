//! Building the menu bar from the items: only known commands, enabled as the bus says, checked as the
//! place says, with macOS's standard items around them.

use tauri::menu::{
    AboutMetadata, CheckMenuItemBuilder, IsMenuItem, Menu, MenuItemBuilder, Submenu, SubmenuBuilder,
};
use tauri::{AppHandle, Manager, Wry};

use super::items::{
    APP_MENU, FILE_MENU, GO_MENU, HELP_MENU, ISSUE_MENU, Item, LIST_MENU, MenuDef, QUIT, RELOAD,
    SETTINGS, VIEW, VIEW_MENU,
};
use super::place::{Place, VIEW_KEYS};
use crate::bus::{self, CommandStatus};

/// What every item is built with: the app, which commands exist and run, and the current place.
struct Ctx<'a> {
    app: &'a AppHandle,
    commands: &'a dyn CommandStatus,
    place: &'a Place,
}

type Builder<'a> = SubmenuBuilder<'a, Wry, AppHandle>;

fn build_item(ctx: &Ctx, item: &Item) -> tauri::Result<Option<Box<dyn IsMenuItem<Wry>>>> {
    if !ctx.commands.knows(item.command) {
        return Ok(None);
    }
    let enabled = ctx.commands.is_enabled(item.command);
    let built: Box<dyn IsMenuItem<Wry>> = match ctx.place.checked(item.command) {
        Some(checked) => {
            let mut builder = CheckMenuItemBuilder::with_id(item.command, item.label)
                .checked(checked)
                .enabled(enabled);
            if let Some(accelerator) = item.accelerator {
                builder = builder.accelerator(accelerator);
            }
            Box::new(builder.build(ctx.app)?)
        }
        None => {
            let mut builder = MenuItemBuilder::with_id(item.command, item.label).enabled(enabled);
            if let Some(accelerator) = item.accelerator {
                builder = builder.accelerator(accelerator);
            }
            Box::new(builder.build(ctx.app)?)
        }
    };
    Ok(Some(built))
}

fn with_items<'a>(
    ctx: &Ctx<'a>,
    mut builder: Builder<'a>,
    items: &[Item],
) -> tauri::Result<Builder<'a>> {
    for item in items {
        if let Some(built) = build_item(ctx, item)? {
            builder = builder.item(&*built);
        }
    }
    Ok(builder)
}

/// A menu's groups with a separator between each two; a group without a known command is left out.
fn with_groups<'a>(
    ctx: &Ctx<'a>,
    mut builder: Builder<'a>,
    groups: &[&[Item]],
) -> tauri::Result<Builder<'a>> {
    let shown = groups
        .iter()
        .filter(|items| items.iter().any(|item| ctx.commands.knows(item.command)));
    for (i, items) in shown.enumerate() {
        if i > 0 {
            builder = builder.separator();
        }
        builder = with_items(ctx, builder, items)?;
    }
    Ok(builder)
}

fn submenu<'a>(ctx: &Ctx<'a>, (title, groups): MenuDef) -> tauri::Result<Builder<'a>> {
    with_groups(ctx, SubmenuBuilder::new(ctx.app, title), groups)
}

/// The level's first nine views by name, the shown one checked, each with its ⌘ digit.
fn with_views<'a>(ctx: &Ctx<'a>, mut builder: Builder<'a>) -> tauri::Result<Builder<'a>> {
    if ctx.place.views.is_empty() {
        return Ok(builder);
    }
    builder = builder.separator();
    for (i, (id, name)) in ctx.place.views.iter().take(VIEW_KEYS).enumerate() {
        let item = CheckMenuItemBuilder::with_id(format!("page:view.select.{}", i + 1), name)
            .checked(ctx.place.view_id == Some(*id))
            .accelerator(format!("CmdOrCtrl+{}", i + 1))
            .build(ctx.app)?;
        builder = builder.item(&item);
    }
    Ok(builder)
}

fn app_submenu<'a>(ctx: &Ctx<'a>) -> tauri::Result<Submenu<Wry>> {
    let builder = SubmenuBuilder::new(ctx.app, APP_MENU.0)
        .about(Some(AboutMetadata::default()))
        .separator();
    let builder = with_items(ctx, builder, &[SETTINGS])?
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator();
    with_items(ctx, builder, &[QUIT])?.build()
}

/// The menu bar for `place`; `place::refresh` builds it again when the level's views change.
pub fn build(app: &AppHandle, place: &Place) -> tauri::Result<Menu<Wry>> {
    let registry = app.state::<bus::Commands>();
    let commands = registry.available(app);
    let ctx = Ctx {
        app,
        commands: &commands,
        place,
    };
    let taskboard = app_submenu(&ctx)?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let file = submenu(&ctx, FILE_MENU)?.build()?;
    let go = with_views(&ctx, submenu(&ctx, GO_MENU)?)?.build()?;
    let list = submenu(&ctx, LIST_MENU)?.build()?;
    let issue = submenu(&ctx, ISSUE_MENU)?.build()?;
    // The View menu keeps its items together, above Enter Full Screen.
    let view = with_items(&ctx, SubmenuBuilder::new(app, VIEW_MENU.0), VIEW)?;
    let view = with_items(&ctx, view, &[RELOAD])?
        .separator()
        .fullscreen()
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .bring_all_to_front()
        .build()?;
    let help = submenu(&ctx, HELP_MENU)?.build()?;
    Menu::with_items(
        app,
        &[
            &taskboard, &file, &edit, &go, &list, &issue, &view, &window, &help,
        ],
    )
}
