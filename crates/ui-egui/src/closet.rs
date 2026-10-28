//! The Closet and the Basement (Window › Closet…): tools, panels and menu items put out of sight
//! without deleting anything. The Closet holds what is used now and then, one click from coming
//! back; the Basement, a tab further away, holds what will probably never be used. A tool put away
//! leaves the Tools panel and its letter key; a panel leaves the Window menu and the dock; a menu
//! item or a whole submenu leaves its menu (the in-window menu bar and the macOS one). Restoring
//! puts each back where it was.
//!
//! Everything is named by a key: `tool:<tool id>` (`tool:pen`), `panel:<id>` (`panel:effects`),
//! `menu:<command id>` (`menu:app.exportEpub`) or `submenu:<Menu/Submenu>` (`submenu:Type/Notes`,
//! the path Edit › Menus uses). [`SHIPPED`] is the sort Marian DesignCraft starts with;
//! `UiState::closet` keeps only what was moved since, so it is saved with the settings.

use std::collections::BTreeSet;

use egui::RichText;
use serde_json::{Value, json};

use crate::DesignApp;
use crate::menus::Item;
use crate::state::UiState;
use crate::theme::Tokens;

/// The Closet's own commands.
pub const IDS: [&str; 4] = ["window.closet", "closet.set", "closet.reset", "closet.list"];

/// Where something lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shelf {
    InUse,
    Closet,
    Basement,
}

impl Shelf {
    const ALL: [Self; 3] = [Self::InUse, Self::Closet, Self::Basement];

    pub fn name(self) -> &'static str {
        match self {
            Self::InUse => "inUse",
            Self::Closet => "closet",
            Self::Basement => "basement",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|shelf| shelf.name() == s)
    }
}

use Shelf::{Basement, Closet};

/// The sort Marian DesignCraft starts with (Emma's list, 28 October 2026).
pub const SHIPPED: &[(&str, Shelf)] = &[
    // Tools.
    ("tool:page", Basement),
    ("tool:gap", Closet),
    ("tool:contentCollector", Basement),
    ("tool:contentPlacer", Basement),
    ("tool:verticalType", Basement),
    ("tool:verticalTypeOnPath", Basement),
    ("tool:pen", Closet),
    ("tool:addAnchor", Closet),
    ("tool:deleteAnchor", Closet),
    ("tool:convertDirection", Closet),
    ("tool:pencil", Closet),
    ("tool:smooth", Closet),
    ("tool:erase", Closet),
    ("tool:rectangle", Closet),
    ("tool:ellipse", Closet),
    ("tool:polygon", Closet),
    ("tool:scissors", Closet),
    ("tool:freeTransform", Closet),
    ("tool:scale", Closet),
    ("tool:shear", Closet),
    ("tool:gradientSwatch", Closet),
    ("tool:gradientFeather", Closet),
    ("tool:note", Basement),
    ("tool:colorTheme", Closet),
    ("tool:measure", Closet),
    // Panels.
    ("panel:properties", Closet),
    ("panel:table", Closet),
    ("panel:pathfinder", Closet),
    ("panel:info", Closet),
    ("panel:preflight", Closet),
    ("panel:attributes", Closet),
    ("panel:scripts", Closet),
    ("panel:trackChanges", Closet),
    ("panel:separations", Closet),
    ("panel:book", Closet),
    ("panel:effects", Basement),
    ("panel:flexLayout", Basement),
    ("panel:conditions", Basement),
    ("panel:notes", Basement),
    ("panel:library", Basement),
    ("panel:hyperlinks", Basement),
    ("panel:bookmarks", Basement),
    ("panel:articles", Basement),
    ("panel:tags", Basement),
    ("panel:buttons", Basement),
    ("panel:liquid", Basement),
    ("panel:states", Basement),
    ("panel:media", Basement),
    ("panel:transitions", Basement),
    ("panel:dataMerge", Basement),
    // Menu items: File.
    ("menu:app.placeWithOptions", Closet),
    ("menu:app.exportText", Closet),
    ("menu:file.info", Closet),
    ("menu:app.printBooklet", Closet),
    ("menu:app.exportIdml", Closet),
    ("menu:app.exportEpub", Basement),
    ("menu:app.exportFixedEpub", Basement),
    ("menu:app.exportInteractivePdf", Basement),
    ("menu:app.exportHtml", Basement),
    ("menu:app.exportPptx", Basement),
    ("menu:app.exportXml", Basement),
    ("submenu:File/Export", Basement),
    // Edit.
    ("menu:app.placeAndLink", Closet),
    ("menu:app.storyEditor", Closet),
    ("menu:app.colorSettings", Closet),
    ("submenu:Edit/Transparency Blend Space", Closet),
    ("submenu:Edit/Transparency Flattener Presets", Closet),
    // Layout.
    ("menu:toc.generate", Closet),
    ("menu:toc.update", Closet),
    ("submenu:Layout/Index", Closet),
    ("menu:layout.createAlternate", Basement),
    // Type.
    ("menu:footnote.insert", Closet),
    ("menu:app.footnoteOptionsDialog", Closet),
    ("menu:endnote.insert", Closet),
    ("menu:footnote.goToReference", Closet),
    ("submenu:Type/Hyperlinks & Cross-References", Closet),
    ("submenu:Type/Text Variables", Closet),
    ("menu:math.insert", Closet),
    ("menu:font.reload", Closet),
    ("submenu:Type/Notes", Basement),
    ("submenu:Type/Track Changes", Basement),
    ("submenu:Type/Story Direction", Basement),
    // Object.
    ("submenu:Object/Flex Layout", Closet),
    ("menu:object.primaryTextFrame", Closet),
    ("menu:app.qrCode", Closet),
    ("submenu:Object/Clipping Path", Closet),
    ("submenu:Object/Captions", Closet),
    ("menu:app.layerOptions", Basement),
    ("menu:object.exportOptions", Basement),
    // View.
    ("menu:view.overprintPreview", Closet),
    ("submenu:View/Proof Setup", Closet),
    ("menu:view.proofColors", Closet),
    ("submenu:View/Separations Preview", Closet),
    ("menu:view.flattenerPreview", Closet),
    ("submenu:View/Rotate Spread", Closet),
    ("submenu:View/Display Performance", Closet),
    ("submenu:View/Structure", Basement),
    ("submenu:View/Story Editor", Basement),
    // Help.
    ("menu:file.newSample", Closet),
];

#[cfg(test)]
thread_local! {
    /// Tests start with everything in use, so the app's own tests see every tool, panel and menu
    /// item; the Closet's tests turn the starting sort on.
    static SORTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the starting sort (and Marian DesignCraft's starting panel layout) applies.
pub(crate) fn sorted() -> bool {
    #[cfg(test)]
    if !SORTED.get() {
        return false;
    }
    true
}

#[cfg(test)]
pub(crate) fn use_starting_sort() {
    SORTED.set(true);
}

/// Where [`SHIPPED`] puts `key` (in use when it doesn't list it).
fn shipped(key: &str) -> Shelf {
    if !sorted() {
        return Shelf::InUse;
    }
    SHIPPED.iter().find(|(k, _)| *k == key).map_or(Shelf::InUse, |(_, s)| *s)
}

/// Where `key` lives now.
pub fn shelf(ui: &UiState, key: &str) -> Shelf {
    ui.closet.get(key).and_then(|s| Shelf::parse(s)).unwrap_or_else(|| shipped(key))
}

/// Is `key` put away (in the Closet or the Basement)?
pub fn hidden(ui: &UiState, key: &str) -> bool {
    shelf(ui, key) != Shelf::InUse
}

pub fn tool_hidden(ui: &UiState, id: &str) -> bool {
    hidden(ui, &format!("tool:{id}"))
}

pub fn panel_hidden(ui: &UiState, id: &str) -> bool {
    hidden(ui, &format!("panel:{id}"))
}

/// Every key put away now.
fn put_away(ui: &UiState) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = if sorted() {
        SHIPPED.iter().filter(|(k, s)| *s != Shelf::InUse && !ui.closet.contains_key(*k)).map(|(k, _)| k.to_string()).collect()
    } else {
        BTreeSet::new()
    };
    out.extend(ui.closet.iter().filter(|(_, s)| Shelf::parse(s).is_some_and(|s| s != Shelf::InUse)).map(|(k, _)| k.clone()));
    out
}

/// The panels put away, by id.
pub fn hidden_panels(ui: &UiState) -> Vec<String> {
    put_away(ui).iter().filter_map(|k| k.strip_prefix("panel:")).map(str::to_owned).collect()
}

// ---------- menus ----------

/// The panel a menu item opens (Window › Panels, Type › Character, Type › Tabs).
fn panel_of(id: &str, params: &Value) -> Option<String> {
    match id {
        "window.panel" => params.get("panel").and_then(Value::as_str).map(str::to_owned),
        "app.tabsPanel" => Some("tabs".into()),
        "app.tablePanel" => Some("table".into()),
        _ => None,
    }
}

/// Each menu of `tree` without what is put away: items, whole submenus and the items that open a
/// put-away panel, with no submenu left empty and no separator left first, last or twice running.
pub fn prune_tree(ui: &UiState, tree: &mut [(&'static str, Vec<Item>)]) {
    let away = put_away(ui);
    if away.is_empty() {
        return;
    }
    for (menu, items) in tree.iter_mut() {
        prune(&away, items, menu);
    }
}

fn prune(away: &BTreeSet<String>, items: &mut Vec<Item>, path: &str) {
    items.retain_mut(|item| match item {
        Item::Sep => true,
        Item::Sub(name, children) => {
            let sub = format!("{path}/{name}");
            if away.contains(&format!("submenu:{sub}")) {
                return false;
            }
            let before = children.len();
            prune(away, children, &sub);
            // A submenu whose every item went away goes too (one built from a list stays).
            !(before > 0 && children.iter().all(|c| matches!(c, Item::Sep)))
        }
        Item::Cmd { id, params, .. } => {
            if let Some(panel) = panel_of(id, params) {
                return !away.contains(&format!("panel:{panel}"));
            }
            !(params.is_null() && away.contains(&format!("menu:{id}")))
        }
    });
    let mut tidy: Vec<Item> = Vec::with_capacity(items.len());
    for item in items.drain(..) {
        if matches!(item, Item::Sep) && tidy.last().is_none_or(|l| matches!(l, Item::Sep)) {
            continue;
        }
        tidy.push(item);
    }
    if matches!(tidy.last(), Some(Item::Sep)) {
        tidy.pop();
    }
    *items = tidy;
}

// ---------- what can be put away ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Tool,
    Panel,
    Menu,
}

impl Kind {
    fn heading(self) -> &'static str {
        match self {
            Self::Tool => "Tools",
            Self::Panel => "Panels",
            Self::Menu => "Menus",
        }
    }
}

/// One thing that can be put away.
#[derive(Clone, Debug)]
pub struct Thing {
    pub key: String,
    pub kind: Kind,
    /// Its English name (a menu item's whole path).
    pub label: String,
}

/// Tools that stay: the Selection tool is what every other tool gives way to.
const KEPT_TOOLS: [&str; 1] = ["selection"];

/// Menu items that stay: the way into the Closet, and the ones a Mac app must have.
const KEPT_COMMANDS: [&str; 6] = ["window.closet", "app.preferences", "app.quit", "help.about", "app.menus", "window.workspace"];

/// Every tool, panel and menu item, in that order (menus in menu-bar order).
pub fn things() -> Vec<Thing> {
    let mut out = Vec::new();
    for tool in designcraft_tools::TOOL_GROUPS.iter().flat_map(|g| g.iter()) {
        if !KEPT_TOOLS.contains(&tool.id) {
            out.push(Thing { key: format!("tool:{}", tool.id), kind: Kind::Tool, label: tool.label.to_owned() });
        }
    }
    let mut panels: Vec<(&str, &str)> = crate::dock::DOCK_TABS
        .iter()
        .chain(crate::dock::ICON_PANELS)
        .map(|(id, label, _)| (*id, *label))
        .chain(crate::dock::FLOATING_PANELS.iter().copied())
        .collect();
    panels.sort_by_key(|(_, label)| label.to_lowercase());
    for (id, label) in panels {
        out.push(Thing { key: format!("panel:{id}"), kind: Kind::Panel, label: label.to_owned() });
    }
    fn walk(items: &[Item], path: &str, shown: &str, out: &mut Vec<Thing>, seen: &mut BTreeSet<String>) {
        for item in items {
            match item {
                Item::Sub(name, children) => {
                    let sub = format!("{path}/{name}");
                    let shown = format!("{shown} › {name}");
                    if seen.insert(format!("submenu:{sub}")) {
                        out.push(Thing { key: format!("submenu:{sub}"), kind: Kind::Menu, label: format!("{shown} ›") });
                    }
                    walk(children, &sub, &shown, out, seen);
                }
                Item::Cmd { label, id, params, .. } => {
                    if !params.is_null() || panel_of(id, params).is_some() || KEPT_COMMANDS.contains(&id.as_str()) {
                        continue;
                    }
                    if seen.insert(format!("menu:{id}")) {
                        out.push(Thing { key: format!("menu:{id}"), kind: Kind::Menu, label: format!("{shown} › {label}") });
                    }
                }
                Item::Sep => {}
            }
        }
    }
    let mut seen = BTreeSet::new();
    for (menu, items) in crate::menus::menu_tree() {
        // The Window menu's panel groups are the panels above; its other items stay.
        if menu == "Window" {
            continue;
        }
        walk(&items, menu, menu, &mut out, &mut seen);
    }
    out
}

// ---------- commands ----------

/// `window.closet {show?}`, `closet.set {key, shelf}`, `closet.reset {all?}` and
/// `closet.list {shelf?}`; `None` for other ids.
pub fn command(app: &mut DesignApp, id: &str, p: &Value) -> Option<Result<Value, String>> {
    Some(match id {
        "window.closet" => {
            let open = p.get("show").and_then(Value::as_bool).unwrap_or(!app.ui.closet_open);
            app.ui.closet_open = open;
            Ok(json!({ "visible": open }))
        }
        "closet.set" => (|| {
            let key =
                p.get("key").and_then(Value::as_str).ok_or("key is required: tool:<id>, panel:<id>, menu:<command id> or submenu:<Menu/Submenu>")?;
            let to = p.get("shelf").and_then(Value::as_str).and_then(Shelf::parse).ok_or("shelf must be inUse, closet or basement")?;
            if !things().iter().any(|t| t.key == key) {
                return Err(format!("nothing called `{key}` can be put away"));
            }
            set(&mut app.ui, key, to);
            tidy_away(app);
            Ok(json!({ "key": key, "shelf": to.name() }))
        })(),
        "closet.reset" => (|| {
            let all = match p.get("all") {
                None | Some(Value::Null) => false,
                Some(v) => v.as_bool().ok_or("all must be a boolean")?,
            };
            app.ui.closet = if all && sorted() {
                SHIPPED.iter().map(|(k, _)| (k.to_string(), Shelf::InUse.name().to_string())).collect()
            } else {
                Default::default()
            };
            tidy_away(app);
            Ok(list(&app.ui, None))
        })(),
        "closet.list" => (|| {
            let shelf = match p.get("shelf").and_then(Value::as_str) {
                None => None,
                Some(s) => Some(Shelf::parse(s).ok_or("shelf must be inUse, closet or basement")?),
            };
            Ok(list(&app.ui, shelf))
        })(),
        _ => return None,
    })
}

/// A hidden tool can't be chosen (its key does nothing): an error naming where it is.
pub fn refuse_tool(ui: &UiState, id: &str, p: &Value) -> Option<Result<Value, String>> {
    if id != "tool.select" {
        return None;
    }
    let tool = p.get("tool").and_then(Value::as_str)?;
    if !tool_hidden(ui, tool) {
        return None;
    }
    let name = designcraft_tools::tool_info(tool).map_or(tool, |t| t.label);
    let shelf = if shelf(ui, &format!("tool:{tool}")) == Shelf::Basement { "Basement" } else { "Closet" };
    Some(Err(format!("{name} is in the {shelf} (Window › Closet…)")))
}

pub fn checked(app: &DesignApp, id: &str) -> Option<bool> {
    (id == "window.closet").then_some(app.ui.closet_open)
}

/// Move `key` to `to`, remembering only what differs from [`SHIPPED`].
fn set(ui: &mut UiState, key: &str, to: Shelf) {
    if to == shipped(key) {
        ui.closet.remove(key);
    } else {
        ui.closet.insert(key.to_string(), to.name().to_string());
    }
}

/// After a move: the current tool, if put away, gives way to the Selection tool, and panels put
/// away close.
fn tidy_away(app: &mut DesignApp) {
    if tool_hidden(&app.ui, app.session.tool_id()) {
        app.select_tool("selection");
    }
    for panel in hidden_panels(&app.ui) {
        if panel == "tabs" {
            app.ui.floating.retain(|(p, _)| p != "tabs");
        } else if crate::panel_docking::showing(app, &panel) {
            // A panel that is already closed has nothing to report.
            let _ = app.run("window.panel.close", json!({ "panel": panel }));
        }
    }
}

/// The keys on `shelf` (every key put away, by shelf, without one).
fn list(ui: &UiState, shelf: Option<Shelf>) -> Value {
    let things = things();
    let on = |s: Shelf| things.iter().filter(|t| self::shelf(ui, &t.key) == s).map(|t| t.key.clone()).collect::<Vec<_>>();
    match shelf {
        Some(s) => json!(on(s)),
        None => json!({ "closet": on(Shelf::Closet), "basement": on(Shelf::Basement) }),
    }
}

// ---------- the window ----------

/// The window's tabs.
const TABS: [&str; 3] = ["Closet", "Basement", "Everything"];

fn tab_id() -> egui::Id {
    egui::Id::unique("closet-tab")
}

fn search_id() -> egui::Id {
    egui::Id::unique("closet-search")
}

fn started_id() -> egui::Id {
    egui::Id::unique("closet-started")
}

/// Draws the Closet window when it is open. On the first frame, closes the panels of a saved
/// workspace that the starting sort put away.
pub fn window(app: &mut DesignApp, ctx: &egui::Context) {
    if !ctx.data(|d| d.get_temp::<bool>(started_id()).unwrap_or(false)) {
        ctx.data_mut(|d| d.insert_temp(started_id(), true));
        tidy_away(app);
    }
    if !app.ui.closet_open {
        return;
    }
    let t = Tokens::get(ctx);
    let lang = app.ui.language.clone();
    let tr = |s: &'static str| crate::i18n::tr(&lang, s).to_owned();
    let mut tab: usize = ctx.data(|d| d.get_temp(tab_id()).unwrap_or(0));
    let mut query: String = ctx.data(|d| d.get_temp(search_id()).unwrap_or_default());
    let mut open = true;
    let mut moves: Vec<(String, Shelf)> = Vec::new();
    let mut reset: Option<bool> = None;
    let things = things();
    let shelves: Vec<Shelf> = things.iter().map(|th| shelf(&app.ui, &th.key)).collect();
    egui::Window::new(tr("Closet"))
        .id(egui::Id::unique("closet-window"))
        .open(&mut open)
        .default_pos(egui::pos2(ctx.content_rect().max.x - 440.0, 80.0))
        .default_width(400.0)
        .resizable(true)
        .collapsible(false)
        .frame(egui::Frame::window(&ctx.global_style()).fill(t.panel))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (i, name) in TABS.iter().enumerate() {
                    if ui.selectable_label(tab == i, RichText::new(tr(name)).size(12.5)).clicked() {
                        tab = i;
                    }
                }
            });
            let blurb = match tab {
                0 => "Put away, one click from coming back.",
                1 => "Things you'll probably never need. Nothing here is deleted.",
                _ => "Everything that can be put away. Pick where each one lives.",
            };
            ui.label(RichText::new(tr(blurb)).size(11.5).color(t.text_dim));
            ui.add_space(4.0);
            ui.add(egui::TextEdit::singleline(&mut query).hint_text(tr("Search")).desired_width(f32::INFINITY));
            ui.add_space(4.0);
            let q = query.trim().to_lowercase();
            let wanted = |s: Shelf| match tab {
                0 => s == Shelf::Closet,
                1 => s == Shelf::Basement,
                _ => true,
            };
            let rows: Vec<(&Thing, Shelf)> = things
                .iter()
                .zip(shelves.iter().copied())
                .filter(|(th, s)| wanted(*s) && (q.is_empty() || th.label.to_lowercase().contains(&q)))
                .collect();
            egui::ScrollArea::vertical().id_salt("closet-rows").auto_shrink([false, true]).max_height(440.0).show(ui, |ui| {
                if rows.is_empty() {
                    ui.add_space(8.0);
                    let empty = match (tab, q.is_empty()) {
                        (0, true) => "The Closet is empty.",
                        (1, true) => "The Basement is empty.",
                        _ => "Nothing matches.",
                    };
                    ui.label(RichText::new(tr(empty)).color(t.text_dim));
                }
                let mut kind = None;
                for (th, now) in &rows {
                    if kind != Some(th.kind) {
                        kind = Some(th.kind);
                        ui.add_space(6.0);
                        ui.label(RichText::new(tr(th.kind.heading())).size(11.0).strong().color(t.text_dim));
                    }
                    ui.horizontal(|ui| {
                        ui.set_min_height(24.0);
                        let buttons: [(&'static str, Shelf); 2] = match now {
                            Shelf::Closet => [("To Basement", Shelf::Basement), ("Restore", Shelf::InUse)],
                            Shelf::Basement => [("To Closet", Shelf::Closet), ("Restore", Shelf::InUse)],
                            Shelf::InUse => [("To Basement", Shelf::Basement), ("To Closet", Shelf::Closet)],
                        };
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Right to left: the far shelf first.
                            for (label, to) in buttons {
                                if ui.small_button(tr(label)).clicked() {
                                    moves.push((th.key.clone(), to));
                                }
                            }
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.add(egui::Label::new(RichText::new(crate::i18n::tr(&lang, &th.label)).color(t.text)).truncate());
                            });
                        });
                    });
                }
            });
            ui.add_space(6.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if crate::widgets::outline_button(ui, &tr("Restore My Starting Sort"), 180.0).clicked() {
                    reset = Some(false);
                }
                if crate::widgets::outline_button(ui, &tr("Bring Everything Back"), 170.0).clicked() {
                    reset = Some(true);
                }
            });
        });
    ctx.data_mut(|d| {
        d.insert_temp(tab_id(), tab);
        d.insert_temp(search_id(), query);
    });
    for (key, to) in moves {
        if let Err(e) = app.run("closet.set", json!({ "key": key, "shelf": to.name() })) {
            app.status(e);
        }
    }
    if let Some(all) = reset
        && let Err(e) = app.run("closet.reset", json!({ "all": all }))
    {
        app.status(e);
    }
    if !open {
        app.ui.closet_open = false;
    }
}

#[cfg(test)]
mod tests;
