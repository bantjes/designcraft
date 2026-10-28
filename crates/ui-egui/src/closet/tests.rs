use super::*;
use crate::menus::menu_tree;

fn app() -> DesignApp {
    use_starting_sort();
    DesignApp::new(designcraft_engine::Session::new(), Default::default())
}

fn ids(items: &[Item]) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        match item {
            Item::Cmd { id, .. } => out.push(id.clone()),
            Item::Sub(_, children) => out.extend(ids(children)),
            Item::Sep => {}
        }
    }
    out
}

fn menu(tree: &[(&'static str, Vec<Item>)], title: &str) -> Vec<String> {
    tree.iter().find(|(t, _)| *t == title).map(|(_, items)| ids(items)).unwrap_or_default()
}

#[test]
fn every_key_in_the_starting_sort_names_something_that_exists() {
    use_starting_sort();
    let keys: BTreeSet<String> = things().into_iter().map(|t| t.key).collect();
    for (key, _) in SHIPPED {
        assert!(keys.contains(*key), "{key} is in the starting sort but not in the app");
    }
}

#[test]
fn emmas_sort_puts_the_tools_and_panels_she_named_away() {
    let app = app();
    for tool in ["pen", "pencil", "rectangle", "scissors", "freeTransform", "gradientSwatch", "gradientFeather", "colorTheme", "measure"] {
        assert_eq!(shelf(&app.ui, &format!("tool:{tool}")), Shelf::Closet, "{tool}");
    }
    for tool in ["page", "contentCollector", "note"] {
        assert_eq!(shelf(&app.ui, &format!("tool:{tool}")), Shelf::Basement, "{tool}");
    }
    for tool in ["selection", "directSelection", "rectangleFrame", "ellipseFrame", "polygonFrame", "rotate", "eyedropper", "type"] {
        assert!(!tool_hidden(&app.ui, tool), "{tool} stays in the Tools panel");
    }
    for panel in ["properties", "table", "pathfinder", "info", "preflight", "attributes", "scripts", "trackChanges"] {
        assert_eq!(shelf(&app.ui, &format!("panel:{panel}")), Shelf::Closet, "{panel}");
    }
    for panel in [
        "effects",
        "flexLayout",
        "conditions",
        "notes",
        "library",
        "hyperlinks",
        "bookmarks",
        "articles",
        "tags",
        "buttons",
        "liquid",
        "states",
        "media",
        "transitions",
    ] {
        assert_eq!(shelf(&app.ui, &format!("panel:{panel}")), Shelf::Basement, "{panel}");
    }
    for panel in ["pages", "character", "paragraph", "story", "glyphs", "swatches", "layers"] {
        assert!(!panel_hidden(&app.ui, panel), "{panel} stays");
    }
}

#[test]
fn the_toolbar_leaves_out_put_away_tools_and_keeps_both_arrows_together() {
    let app = app();
    let groups = crate::toolbar::shown_groups(&app);
    let all: Vec<&str> = groups.iter().flatten().map(|t| t.id).collect();
    assert!(!all.contains(&"pen") && !all.contains(&"scissors") && !all.contains(&"page"));
    assert!(all.contains(&"rectangleFrame") && all.contains(&"rotate"));
    let first: Vec<&str> = groups.first().unwrap().iter().map(|t| t.id).collect();
    assert_eq!(first, ["selection", "directSelection"]);
    // An empty group is a separator: never first, last or two in a row.
    assert!(!groups.last().unwrap().is_empty());
    assert!(!groups.windows(2).any(|w| w[0].is_empty() && w[1].is_empty()));
}

#[test]
fn a_put_away_tool_cannot_be_chosen_until_it_is_restored() {
    let mut app = app();
    let err = app.run("tool.select", json!({"tool": "pen"})).unwrap_err();
    assert!(err.contains("Closet"), "{err}");
    app.run("closet.set", json!({"key": "tool:pen", "shelf": "inUse"})).unwrap();
    app.run("tool.select", json!({"tool": "pen"})).unwrap();
    assert_eq!(app.session.tool_id(), "pen");
    // Putting the current tool away hands over to the Selection tool.
    app.run("closet.set", json!({"key": "tool:pen", "shelf": "basement"})).unwrap();
    assert_eq!(app.session.tool_id(), "selection");
    assert_eq!(shelf(&app.ui, "tool:pen"), Shelf::Basement);
}

#[test]
fn menus_lose_put_away_items_and_their_panels() {
    let app = app();
    let mut tree = menu_tree();
    prune_tree(&app.ui, &mut tree);
    let window = menu(&tree, "Window");
    assert!(window.contains(&"window.closet".to_string()));
    let mut opened = Vec::new();
    for (_, items) in &tree {
        collect_panels(items, &mut opened);
    }
    for panel in ["effects", "pathfinder", "trackChanges", "hyperlinks"] {
        assert!(!opened.iter().any(|p| p == panel), "{panel} is still in a menu");
    }
    assert!(opened.iter().any(|p| p == "pages"));
    // No submenu is left empty and no separator doubled.
    fn tidy(items: &[Item]) {
        assert!(!matches!(items.first(), Some(Item::Sep)) && !matches!(items.last(), Some(Item::Sep)));
        assert!(!items.windows(2).any(|w| matches!(w, [Item::Sep, Item::Sep])));
        for item in items {
            if let Item::Sub(name, children) = item {
                assert!(!children.is_empty() || name == "Open Recent", "{name} is empty");
                tidy(children);
            }
        }
    }
    for (_, items) in &tree {
        tidy(items);
    }
}

fn collect_panels(items: &[Item], out: &mut Vec<String>) {
    for item in items {
        match item {
            Item::Cmd { id, params, .. } => out.extend(panel_of(id, params)),
            Item::Sub(_, children) => collect_panels(children, out),
            Item::Sep => {}
        }
    }
}

#[test]
fn bring_everything_back_and_restore_the_starting_sort() {
    let mut app = app();
    app.run("closet.reset", json!({"all": true})).unwrap();
    assert!(hidden_panels(&app.ui).is_empty());
    assert!(!tool_hidden(&app.ui, "pen"));
    let mut tree = menu_tree();
    let before = tree.iter().map(|(_, items)| ids(items).len()).sum::<usize>();
    prune_tree(&app.ui, &mut tree);
    assert_eq!(tree.iter().map(|(_, items)| ids(items).len()).sum::<usize>(), before);
    app.run("closet.reset", json!({})).unwrap();
    assert!(tool_hidden(&app.ui, "pen"));
    assert!(app.ui.closet.is_empty());
}

#[test]
fn closet_commands_reject_bad_params() {
    let mut app = app();
    assert!(app.run("closet.set", json!({"key": "tool:nothing", "shelf": "closet"})).is_err());
    assert!(app.run("closet.set", json!({"key": "tool:pen", "shelf": "attic"})).is_err());
    assert!(app.run("closet.set", json!({})).is_err());
    assert!(app.run("closet.reset", json!({"all": "yes"})).is_err());
    assert!(app.run("closet.list", json!({"shelf": 3})).is_ok());
    let listed = app.run("closet.list", json!({"shelf": "basement"})).unwrap();
    assert!(listed.as_array().unwrap().iter().any(|k| k == "tool:page"));
}

#[test]
fn pages_start_in_a_narrow_column_on_the_left() {
    let app = app();
    let layout = crate::panel_docking::default_layout(&app.ui);
    let Some(craft_ui::docking::Node::Split { first, .. }) = &layout.root else { panic!("no start column: {:?}", layout.root) };
    let craft_ui::docking::Node::Dock { side, columns } = first.as_ref() else { panic!("the left side isn't a dock") };
    assert_eq!(*side, craft_ui::docking::Side::Start);
    assert_eq!(columns.first().unwrap().groups.first().unwrap().panels, ["pages"]);
    assert!(!layout.panels().iter().any(|p| *p == "effects"));
}

#[test]
fn marian_designcraft_starts_in_red_with_frame_edges_hidden() {
    use_starting_sort();
    let ui = UiState::default();
    assert_eq!(ui.brightness, crate::theme::Brightness::MarianRed);
    assert!(!ui.frame_edges);
    let red = crate::theme::Tokens::for_brightness(crate::theme::Brightness::MarianRed);
    // The Tools panel is the darker red, its current tool green.
    let tools = red.marian_toolbar();
    assert!(tools.panel.r() > 2 * tools.panel.g() && tools.well.g() > tools.well.r());
}
