//! Rectangle, Ellipse and Polygon Frames are drawn without a border (no stroke).
use designcraft_doc::ItemId;
use designcraft_engine::Session;
use serde_json::json;

#[test]
fn frames_drawn_with_the_frame_tools_have_no_stroke() {
    let mut s = Session::new();
    s.execute("file.new", &json!({})).unwrap();
    for shape in ["rectangle", "ellipse", "polygon"] {
        let created = s.execute("frame.create", &json!({"rect": [36, 36, 200, 200], "shape": shape, "content": "graphic"})).unwrap();
        let id = ItemId(created["id"].as_u64().unwrap());
        let stroke = &s.doc().unwrap().doc.item(id).unwrap().stroke;
        assert_eq!(stroke.swatch, designcraft_color::swatch::NONE, "{shape} frame");
    }
}
